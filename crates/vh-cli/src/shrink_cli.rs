//! Boundary-side shrink wiring (convergence C5, audit R1 / Track-2 W5):
//! minimize a failing universe's fault plan through vh-shrink's PUBLIC
//! API only — `crates/vh-shrink/**` stays verifier-track-owned and
//! unedited. Pure module: no filesystem, no clock, no printing; `main.rs`
//! owns presentation and exit codes.
//!
//! The oracle is the exact-fingerprint kind vh-shrink demands: a
//! candidate plan passes only if two override replays agree with each
//! other AND reproduce the baseline's exact always-failure set
//! (name + detail) with a valid completion — never "any failure"
//! (cause switching is the documented shrink hazard).

use crate::workloads;
use vh_gremlin::FaultPlan;
use vh_multiverse::{run_universe, run_universe_with_fault_plan, UniverseResult};
use vh_shrink::try_shrink;

/// Everything the boundary prints about a completed minimization,
/// including the provenance-binding fields `ShrinkReport` deliberately
/// does not carry (PR #2's honest open contract — bound here at the CLI
/// boundary until the C4 bundle store lands and takes over).
pub struct ShrinkOutcome {
    pub workload: String,
    pub seed: u64,
    pub universe: u64,
    pub baseline_trace_hash: String,
    pub baseline_plan_digest: Option<String>,
    /// The exact fingerprint the oracle matched: `(name, detail)` per
    /// always-failure, in recorded order.
    pub baseline_failures: Vec<(String, String)>,
    pub original_injections: usize,
    pub minimized_injections: usize,
    pub oracle_calls: usize,
    pub distinct_candidates: usize,
    /// The 1-minimal plan itself, for independent replay verification.
    pub minimized_plan: FaultPlan,
}

fn fingerprint(result: &UniverseResult) -> Vec<(String, String)> {
    result
        .always_failures()
        .iter()
        .map(|f| (f.name.clone(), f.detail.clone()))
        .collect()
}

/// Minimize the fault plan of one failing universe. Errors are typed
/// strings for the boundary to print; they never bless anything.
pub fn shrink_universe(
    workload_name: &str,
    seed: u64,
    universe: u64,
) -> Result<ShrinkOutcome, String> {
    let (capturing, cell) = workloads::by_name_capturing(workload_name).ok_or_else(|| {
        format!(
            "shrink does not support workload {workload_name:?} yet: its fault plan is \
             retrieved inside the sim runtime and the workload never holds it \
             (capture is wired for demo/demo-buggy; see the convergence ledger)"
        )
    })?;

    // Baseline pair: one execution proves nothing (single-replay law).
    let a = run_universe(seed, universe, capturing.as_ref());
    let b = run_universe(seed, universe, capturing.as_ref());
    if !a.observably_equal(&b) {
        return Err(format!(
            "universe {universe} diverges under replay; nothing can be shrunk"
        ));
    }
    let plan = cell
        .borrow_mut()
        .take()
        .ok_or("workload retrieved no fault plan")?;
    if !a.lifecycle().is_valid_completion() {
        return Err(format!(
            "universe {universe} did not validly complete ({:?}); only completed \
             always-failure findings are shrinkable",
            a.lifecycle().outcome()
        ));
    }
    let baseline = fingerprint(&a);
    if baseline.is_empty() {
        return Err(format!(
            "universe {universe} has no always-failure finding to shrink"
        ));
    }

    // Fresh non-capturing workload per oracle call; the override path
    // (run_universe_with_fault_plan) never invokes the generator, so the
    // capture machinery is irrelevant there anyway.
    let oracle_workload = workloads::by_name(workload_name)
        .ok_or_else(|| format!("unknown workload {workload_name:?}"))?;
    let oracle = |candidate: &FaultPlan| -> bool {
        let x = run_universe_with_fault_plan(
            seed,
            universe,
            oracle_workload.as_ref(),
            candidate.clone(),
        );
        let y = run_universe_with_fault_plan(
            seed,
            universe,
            oracle_workload.as_ref(),
            candidate.clone(),
        );
        x.observably_equal(&y) && x.lifecycle().is_valid_completion() && fingerprint(&x) == baseline
    };

    let report = try_shrink(plan, oracle).map_err(|failure| {
        format!(
            "shrink failed after {} oracle call(s): {:?}",
            failure.oracle_calls(),
            failure.cause()
        )
    })?;

    Ok(ShrinkOutcome {
        workload: workload_name.to_string(),
        seed,
        universe,
        baseline_trace_hash: a.trace_hash().to_string(),
        baseline_plan_digest: a.fault_plan_digest().map(str::to_string),
        baseline_failures: baseline,
        original_injections: report.original_injections(),
        minimized_injections: report.minimized_injections(),
        oracle_calls: report.oracle_calls(),
        distinct_candidates: report.distinct_candidates(),
        minimized_plan: report.into_plan(),
    })
}
