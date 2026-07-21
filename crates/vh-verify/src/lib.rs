#![forbid(unsafe_code)]

//! Independent Tier-1 replay probes built only on public kernel APIs.

use std::error::Error;
use std::fmt;

use vh_multiverse::{
    run_universe, FaultPlanDiscipline, RunOutcome, UniverseCtx, UniverseResult, Workload,
};
use vh_props::{AlwaysCheck, AlwaysFailure};

pub const REFERENCE_ROOT_SEED: u64 = 0xD1CE;
pub const REFERENCE_UNIVERSE: u64 = 0;
pub const REFERENCE_TRACE_HASH: &str = "eafa30e8a7a6c82939ea3f755bc866ab";
pub const REFERENCE_TRACE_EVENTS: usize = 33;
pub const REFERENCE_TRACE_FORMAT: &str = "v0";
pub const REFERENCE_WORKLOAD: &str = "vh-verify-reference";
pub const REFERENCE_ALWAYS_PROPERTY: &str = "reference_draw_count_is_32";
pub const REFERENCE_COMPLETED_PROPERTY: &str = "reference_completed_all_draws";
pub const OBSERVABLE_FINGERPRINT_SCHEMA: &str = "vh-verify-observable-v2";
pub const REFERENCE_OBSERVABLE_FINGERPRINT: &str = "39f727ed5d8a949c1f5a1243bd6d1d10";
pub const SOAK_RECEIPT_SCHEMA: &str = "vh-verify-soak-v1";

const FNV128_OFFSET: u128 = 0x6c62_272e_07bb_0142_62b8_2175_6295_c58d;
const FNV128_PRIME: u128 = 0x0000_0000_0100_0000_0000_0000_0000_013B;

struct ReferenceWorkload;

impl Workload for ReferenceWorkload {
    fn name(&self) -> &str {
        REFERENCE_WORKLOAD
    }

    fn run(&self, ctx: &mut UniverseCtx) -> RunOutcome {
        ctx.declare_sometimes(REFERENCE_COMPLETED_PROPERTY);
        let mut operations = ctx.stream("verify.operations");
        let mut timing = ctx.stream("verify.timing");
        let mut draws = 0u64;

        for index in 0..32u64 {
            let at_nanos = index * 10_000 + timing.next_below(1_000);
            ctx.advance_to(at_nanos);
            let value = operations.next_u64();
            ctx.record("verify.draw", &format!("index={index};value={value:016x}"));
            draws += 1;
        }
        ctx.record("verify.final", &format!("draws={draws}"));
        ctx.always(REFERENCE_ALWAYS_PROPERTY, draws == 32, || {
            format!("reference workload produced {draws} draws, expected 32")
        });
        ctx.sometimes(REFERENCE_COMPLETED_PROPERTY);
        RunOutcome::Completed
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReplaySoak {
    runs: usize,
    root_seed: u64,
    universe_id: u64,
    workload: &'static str,
    trace_format: &'static str,
    trace_hash: String,
    trace_events: usize,
    observable_schema: &'static str,
    observable_fingerprint: String,
}

impl ReplaySoak {
    pub fn runs(&self) -> usize {
        self.runs
    }

    pub fn root_seed(&self) -> u64 {
        self.root_seed
    }

    pub fn universe_id(&self) -> u64 {
        self.universe_id
    }

    pub fn workload(&self) -> &'static str {
        self.workload
    }

    pub fn trace_format(&self) -> &'static str {
        self.trace_format
    }

    pub fn trace_hash(&self) -> &str {
        &self.trace_hash
    }

    pub fn trace_events(&self) -> usize {
        self.trace_events
    }

    pub fn observable_schema(&self) -> &'static str {
        self.observable_schema
    }

    pub fn observable_fingerprint(&self) -> &str {
        &self.observable_fingerprint
    }
}

#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReplayPanicStage {
    UniverseRun,
    VerifierBoundary,
    CliBoundary,
}

impl ReplayPanicStage {
    pub const fn token(self) -> &'static str {
        match self {
            Self::UniverseRun => "universe-run",
            Self::VerifierBoundary => "verifier-boundary",
            Self::CliBoundary => "cli-boundary",
        }
    }
}

#[non_exhaustive]
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ReplaySoakError {
    ZeroRuns,
    Panicked {
        requested_runs: usize,
        stage: ReplayPanicStage,
        /// Zero-based replay ordinal when the panic occurred inside a universe
        /// run; `None` identifies another verifier boundary stage.
        run: Option<usize>,
    },
    Diverged {
        requested_runs: usize,
        run: usize,
        expected: Box<UniverseResult>,
        actual: Box<UniverseResult>,
    },
    BaselineDrift {
        requested_runs: usize,
        /// Fingerprint of the independently constructed verifier baseline.
        expected_fingerprint: String,
        actual: Box<UniverseResult>,
    },
    ObservableFingerprintDrift {
        requested_runs: usize,
        expected: &'static str,
        actual: String,
    },
    WorkloadIdentityDrift {
        requested_runs: usize,
        expected: &'static str,
        actual: String,
    },
}

impl ReplaySoakError {
    pub const fn code(&self) -> &'static str {
        match self {
            Self::ZeroRuns => "zero-runs",
            Self::Panicked { .. } => "panic",
            Self::Diverged { .. } => "replay-diverged",
            Self::BaselineDrift { .. } => "baseline-drift",
            Self::ObservableFingerprintDrift { .. } => "observable-fingerprint-drift",
            Self::WorkloadIdentityDrift { .. } => "workload-identity-drift",
        }
    }

    pub const fn requested_runs(&self) -> usize {
        match self {
            Self::ZeroRuns => 0,
            Self::Panicked { requested_runs, .. }
            | Self::Diverged { requested_runs, .. }
            | Self::BaselineDrift { requested_runs, .. }
            | Self::ObservableFingerprintDrift { requested_runs, .. }
            | Self::WorkloadIdentityDrift { requested_runs, .. } => *requested_runs,
        }
    }
}

impl fmt::Display for ReplaySoakError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ZeroRuns => formatter.write_str("replay soak requires at least one run"),
            Self::Panicked { .. } => formatter.write_str("Tier-1 replay soak panicked"),
            Self::Diverged { run, .. } => {
                write!(formatter, "Tier-1 observable divergence at replay {run}")
            }
            Self::BaselineDrift { .. } => {
                formatter.write_str("Tier-1 reference result drifted from its frozen baseline")
            }
            Self::ObservableFingerprintDrift { .. } => formatter.write_str(
                "Tier-1 reference observable fingerprint drifted from its frozen baseline",
            ),
            Self::WorkloadIdentityDrift { .. } => {
                formatter.write_str("Tier-1 reference workload identity drifted")
            }
        }
    }
}

impl Error for ReplaySoakError {}

/// Verifier-owned projection of every Tier-1 universe observable exposed by
/// the current kernel API.
///
/// This is intentionally not a constructible kernel result: unit tests may
/// mutate this plain verifier model to probe the independent framing logic,
/// while comparisons of kernel results can only read runner-owned evidence
/// through immutable public getters.
#[derive(Debug, Clone, PartialEq, Eq)]
struct ObservableSnapshot {
    universe_id: u64,
    trace_hash: String,
    trace_events: usize,
    always_checks: Vec<(String, bool)>,
    always_failures: Vec<(String, String)>,
    sometimes: Vec<(String, bool)>,
    outcome: OutcomeSnapshot,
    fault_plan: FaultPlanSnapshot,
}

/// Explicit verifier vocabulary for workload completion. Matching the public
/// kernel enum without a wildcard is the compile-time schema ratchet: adding a
/// public variant forces this independent encoder to make a compatibility
/// decision.
#[derive(Debug, Clone, PartialEq, Eq)]
enum OutcomeSnapshot {
    Completed,
    InvalidAssumption(String),
    ExecutionError(String),
}

/// Explicit verifier vocabulary for runner-owned fault-plan discipline. As
/// above, exhaustive matching over the public enum prevents silent variant
/// omission without reopening the kernel's private evidence fields.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum FaultPlanSnapshot {
    SelfGenerated { consumptions: u64 },
    OverrideConsumed,
    OverrideIgnored,
    OverrideOverconsumed { consumptions: u64 },
}

fn outcome_snapshot(outcome: &RunOutcome) -> OutcomeSnapshot {
    match outcome {
        RunOutcome::Completed => OutcomeSnapshot::Completed,
        RunOutcome::InvalidAssumption(detail) => OutcomeSnapshot::InvalidAssumption(detail.clone()),
        RunOutcome::ExecutionError(detail) => OutcomeSnapshot::ExecutionError(detail.clone()),
    }
}

fn fault_plan_snapshot(discipline: &FaultPlanDiscipline) -> FaultPlanSnapshot {
    // Track-1 loop-4 rename mapping (retrieval-honest kernel vocabulary ->
    // this crate's frozen snapshot framing, values unchanged so every
    // recorded fingerprint stays valid). Renaming the SNAPSHOT side to the
    // retrieval vocabulary is a verifier-track schema decision deferred to
    // its owner: it would change frozen encodings and must be re-derived
    // independently.
    match discipline {
        FaultPlanDiscipline::SelfGenerated { retrievals } => FaultPlanSnapshot::SelfGenerated {
            consumptions: *retrievals,
        },
        FaultPlanDiscipline::OverrideRetrieved => FaultPlanSnapshot::OverrideConsumed,
        FaultPlanDiscipline::OverrideNeverRetrieved => FaultPlanSnapshot::OverrideIgnored,
        FaultPlanDiscipline::OverrideRetrievedMultiply { retrievals } => {
            FaultPlanSnapshot::OverrideOverconsumed {
                consumptions: *retrievals,
            }
        }
    }
}

/// Independent observable projection through the kernel's immutable public
/// evidence API. Public leaf structs/enums are still destructured or matched
/// exhaustively where Rust permits it; the kernel's private aggregate fields
/// deliberately cannot be forged or mutated downstream.
fn observable_snapshot(result: &UniverseResult) -> ObservableSnapshot {
    let always_checks = result
        .always_checks()
        .iter()
        .map(|check| {
            let AlwaysCheck { name, passed } = check;
            (name.clone(), *passed)
        })
        .collect();
    let always_failures = result
        .always_failures()
        .iter()
        .map(|failure| {
            let AlwaysFailure { name, detail } = failure;
            (name.clone(), detail.clone())
        })
        .collect();
    let sometimes = result
        .sometimes()
        .iter()
        .map(|(name, reached)| (name.clone(), *reached))
        .collect();

    ObservableSnapshot {
        universe_id: result.universe_id(),
        trace_hash: result.trace_hash().to_string(),
        trace_events: result.trace_events(),
        always_checks,
        always_failures,
        sometimes,
        outcome: outcome_snapshot(result.lifecycle().outcome()),
        fault_plan: fault_plan_snapshot(result.lifecycle().fault_plan()),
    }
}

fn fingerprint_absorb(mut state: u128, bytes: &[u8]) -> u128 {
    for byte in bytes {
        state ^= u128::from(*byte);
        state = state.wrapping_mul(FNV128_PRIME);
    }
    state
}

fn fingerprint_absorb_bytes(state: u128, bytes: &[u8]) -> u128 {
    let length = u64::try_from(bytes.len()).expect("observable field length must fit u64");
    let state = fingerprint_absorb(state, &length.to_le_bytes());
    fingerprint_absorb(state, bytes)
}

/// Compatibility fingerprint over a verifier-owned, unambiguously framed
/// observable snapshot. FNV-1a-128 is deliberately non-cryptographic; frozen
/// full structural comparisons remain the collision backstop.
fn snapshot_fingerprint(snapshot: &ObservableSnapshot) -> String {
    let mut state =
        fingerprint_absorb_bytes(FNV128_OFFSET, OBSERVABLE_FINGERPRINT_SCHEMA.as_bytes());
    state = fingerprint_absorb(state, &snapshot.universe_id.to_le_bytes());
    state = fingerprint_absorb_bytes(state, snapshot.trace_hash.as_bytes());
    state = fingerprint_absorb(
        state,
        &u64::try_from(snapshot.trace_events)
            .expect("trace event count must fit u64")
            .to_le_bytes(),
    );
    state = fingerprint_absorb(
        state,
        &u64::try_from(snapshot.always_checks.len())
            .expect("always-check count must fit u64")
            .to_le_bytes(),
    );
    for (name, passed) in &snapshot.always_checks {
        state = fingerprint_absorb_bytes(state, name.as_bytes());
        state = fingerprint_absorb(state, &[u8::from(*passed)]);
    }
    state = fingerprint_absorb(
        state,
        &u64::try_from(snapshot.always_failures.len())
            .expect("always-failure count must fit u64")
            .to_le_bytes(),
    );
    for (name, detail) in &snapshot.always_failures {
        state = fingerprint_absorb_bytes(state, name.as_bytes());
        state = fingerprint_absorb_bytes(state, detail.as_bytes());
    }
    state = fingerprint_absorb(
        state,
        &u64::try_from(snapshot.sometimes.len())
            .expect("sometimes count must fit u64")
            .to_le_bytes(),
    );
    for (name, reached) in &snapshot.sometimes {
        state = fingerprint_absorb_bytes(state, name.as_bytes());
        state = fingerprint_absorb(state, &[u8::from(*reached)]);
    }

    // Canonical lifecycle tags are stable ASCII tokens, never Rust Debug
    // output. Payload-bearing variants length-prefix their UTF-8 detail.
    match &snapshot.outcome {
        OutcomeSnapshot::Completed => {
            state = fingerprint_absorb_bytes(state, b"run-outcome.completed");
        }
        OutcomeSnapshot::InvalidAssumption(detail) => {
            state = fingerprint_absorb_bytes(state, b"run-outcome.invalid-assumption");
            state = fingerprint_absorb_bytes(state, detail.as_bytes());
        }
        OutcomeSnapshot::ExecutionError(detail) => {
            state = fingerprint_absorb_bytes(state, b"run-outcome.execution-error");
            state = fingerprint_absorb_bytes(state, detail.as_bytes());
        }
    }
    match snapshot.fault_plan {
        FaultPlanSnapshot::SelfGenerated { consumptions } => {
            state = fingerprint_absorb_bytes(state, b"fault-plan.self-generated");
            state = fingerprint_absorb(state, &consumptions.to_le_bytes());
        }
        FaultPlanSnapshot::OverrideConsumed => {
            state = fingerprint_absorb_bytes(state, b"fault-plan.override-consumed");
        }
        FaultPlanSnapshot::OverrideIgnored => {
            state = fingerprint_absorb_bytes(state, b"fault-plan.override-ignored");
        }
        FaultPlanSnapshot::OverrideOverconsumed { consumptions } => {
            state = fingerprint_absorb_bytes(state, b"fault-plan.override-overconsumed");
            state = fingerprint_absorb(state, &consumptions.to_le_bytes());
        }
    }

    format!("{state:032x}")
}

fn observable_fingerprint(result: &UniverseResult) -> String {
    snapshot_fingerprint(&observable_snapshot(result))
}

fn observably_equal_independent(left: &UniverseResult, right: &UniverseResult) -> bool {
    observable_snapshot(left) == observable_snapshot(right)
}

/// Require both runner-owned whole-result identity and the verifier's
/// independently framed projection. The kernel comparison automatically
/// includes a future private `UniverseResult` field through struct equality;
/// the independent comparison prevents the verifier from merely trusting the
/// implementation it is meant to audit.
fn observably_equal_dual(left: &UniverseResult, right: &UniverseResult) -> bool {
    left.observably_equal(right) && observably_equal_independent(left, right)
}

fn require_observable_identity<F>(
    runs: usize,
    mut run_once: F,
) -> Result<UniverseResult, ReplaySoakError>
where
    F: FnMut() -> UniverseResult,
{
    if runs == 0 {
        return Err(ReplaySoakError::ZeroRuns);
    }

    let first = catch_universe_run(runs, 0, &mut run_once)?;

    for run in 1..runs {
        let replay = catch_universe_run(runs, run, &mut run_once)?;
        if !observably_equal_dual(&replay, &first) {
            return Err(ReplaySoakError::Diverged {
                requested_runs: runs,
                run,
                expected: Box::new(first),
                actual: Box::new(replay),
            });
        }
    }

    Ok(first)
}

fn catch_universe_run<F>(
    requested_runs: usize,
    run: usize,
    run_once: &mut F,
) -> Result<UniverseResult, ReplaySoakError>
where
    F: FnMut() -> UniverseResult,
{
    match std::panic::catch_unwind(std::panic::AssertUnwindSafe(run_once)) {
        Ok(result) => Ok(result),
        Err(_) => Err(ReplaySoakError::Panicked {
            requested_runs,
            stage: ReplayPanicStage::UniverseRun,
            run: Some(run),
        }),
    }
}

fn expected_reference_snapshot() -> ObservableSnapshot {
    ObservableSnapshot {
        universe_id: 0,
        trace_hash: "eafa30e8a7a6c82939ea3f755bc866ab".to_string(),
        trace_events: 33,
        always_checks: vec![("reference_draw_count_is_32".to_string(), true)],
        always_failures: Vec::new(),
        sometimes: vec![("reference_completed_all_draws".to_string(), true)],
        outcome: OutcomeSnapshot::Completed,
        fault_plan: FaultPlanSnapshot::SelfGenerated { consumptions: 0 },
    }
}

/// Sequentially replay one fixed reference universe. Every run must satisfy
/// both runner-owned whole-result equality and the verifier's independently
/// framed projection; that current projection must also match a frozen
/// baseline. Rust unwind panics are converted to typed evidence; process
/// aborts and `panic=abort` remain outside this in-process boundary.
pub fn replay_soak(runs: usize) -> Result<ReplaySoak, ReplaySoakError> {
    match std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| replay_soak_inner(runs))) {
        Ok(result) => result,
        Err(_) => Err(ReplaySoakError::Panicked {
            requested_runs: runs,
            stage: ReplayPanicStage::VerifierBoundary,
            run: None,
        }),
    }
}

fn replay_soak_inner(runs: usize) -> Result<ReplaySoak, ReplaySoakError> {
    let workload = ReferenceWorkload;
    let workload_name = workload.name();
    if workload_name != REFERENCE_WORKLOAD {
        return Err(ReplaySoakError::WorkloadIdentityDrift {
            requested_runs: runs,
            expected: REFERENCE_WORKLOAD,
            actual: workload_name.to_string(),
        });
    }
    let first = require_observable_identity(runs, || {
        run_universe(REFERENCE_ROOT_SEED, REFERENCE_UNIVERSE, &workload)
    })?;
    let workload_name_after = workload.name();
    if workload_name_after != REFERENCE_WORKLOAD {
        return Err(ReplaySoakError::WorkloadIdentityDrift {
            requested_runs: runs,
            expected: REFERENCE_WORKLOAD,
            actual: workload_name_after.to_string(),
        });
    }
    let expected = expected_reference_snapshot();
    if observable_snapshot(&first) != expected {
        return Err(ReplaySoakError::BaselineDrift {
            requested_runs: runs,
            expected_fingerprint: snapshot_fingerprint(&expected),
            actual: Box::new(first),
        });
    }
    let fingerprint = observable_fingerprint(&first);
    if fingerprint != REFERENCE_OBSERVABLE_FINGERPRINT {
        return Err(ReplaySoakError::ObservableFingerprintDrift {
            requested_runs: runs,
            expected: REFERENCE_OBSERVABLE_FINGERPRINT,
            actual: fingerprint,
        });
    }

    Ok(ReplaySoak {
        runs,
        root_seed: REFERENCE_ROOT_SEED,
        universe_id: REFERENCE_UNIVERSE,
        workload: REFERENCE_WORKLOAD,
        trace_format: REFERENCE_TRACE_FORMAT,
        trace_hash: first.trace_hash().to_string(),
        trace_events: first.trace_events(),
        observable_schema: OBSERVABLE_FINGERPRINT_SCHEMA,
        observable_fingerprint: fingerprint,
    })
}

fn receipt_token(value: &str) -> String {
    const HEX: &[u8; 16] = b"0123456789ABCDEF";
    let mut encoded = String::with_capacity(value.len().saturating_mul(3));
    for byte in value.bytes() {
        if byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.') {
            encoded.push(char::from(byte));
        } else {
            encoded.push('%');
            encoded.push(char::from(HEX[usize::from(byte >> 4)]));
            encoded.push(char::from(HEX[usize::from(byte & 0x0f)]));
        }
    }
    encoded
}

/// One machine-readable Tier-1 soak receipt. `universes_per_hour` is boundary
/// telemetry only and is never used as replay input or evidence identity.
pub fn format_receipt(report: &ReplaySoak, universes_per_hour: u128) -> String {
    format!(
        "soak: receipt-schema={} verdict=PASS determinism-tier=Tier-1 evidence-grade=D0 root-seed=0x{:016x} universe={} workload={} trace-format={} runs={} hash={} events={} observable-schema={} observable-fingerprint={} upH={universes_per_hour} upH-scope=boundary-telemetry",
        SOAK_RECEIPT_SCHEMA,
        report.root_seed,
        report.universe_id,
        report.workload,
        report.trace_format,
        report.runs,
        report.trace_hash,
        report.trace_events,
        report.observable_schema,
        report.observable_fingerprint
    )
}

/// Stable machine-readable failure receipt. Free-form diagnostic detail is
/// intentionally excluded so tokenization remains parseable across failures.
/// Variable text tokens use bytewise UTF-8 percent encoding: ASCII letters,
/// digits, `.`, `_`, and `-` remain literal; every other byte is `%HH` with
/// uppercase hexadecimal digits.
pub fn format_error_receipt(error: &ReplaySoakError) -> String {
    let prefix = format!(
        "soak: receipt-schema={} verdict=ERROR determinism-tier=Tier-1 evidence-grade=D0 root-seed=0x{:016x} universe={} workload={} trace-format={} observable-schema={} requested-runs={} error-code={}",
        SOAK_RECEIPT_SCHEMA,
        REFERENCE_ROOT_SEED,
        REFERENCE_UNIVERSE,
        REFERENCE_WORKLOAD,
        REFERENCE_TRACE_FORMAT,
        OBSERVABLE_FINGERPRINT_SCHEMA,
        error.requested_runs(),
        error.code()
    );
    match error {
        ReplaySoakError::ZeroRuns => prefix,
        ReplaySoakError::Panicked { stage, run, .. } => format!(
            "{prefix} panic-stage={} panic-run={}",
            stage.token(),
            run.map_or_else(|| "boundary".to_string(), |run| run.to_string())
        ),
        ReplaySoakError::Diverged {
            run,
            expected,
            actual,
            ..
        } => format!(
            "{prefix} divergence-run={run} expected-observable-fingerprint={} actual-observable-fingerprint={}",
            observable_fingerprint(expected),
            observable_fingerprint(actual)
        ),
        ReplaySoakError::BaselineDrift {
            expected_fingerprint,
            actual,
            ..
        } => format!(
            "{prefix} expected-observable-fingerprint={} actual-observable-fingerprint={}",
            receipt_token(expected_fingerprint),
            observable_fingerprint(actual)
        ),
        ReplaySoakError::ObservableFingerprintDrift {
            expected, actual, ..
        } => format!(
            "{prefix} expected-observable-fingerprint={} actual-observable-fingerprint={}",
            receipt_token(expected),
            receipt_token(actual)
        ),
        ReplaySoakError::WorkloadIdentityDrift {
            expected, actual, ..
        } => format!(
            "{prefix} expected-workload={} actual-workload={}",
            receipt_token(expected),
            receipt_token(actual)
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use vh_multiverse::run_universe_with_fault_plan;

    fn assert_snapshot_mutant_rejected(baseline: &ObservableSnapshot, mutant: &ObservableSnapshot) {
        assert_ne!(baseline, mutant);
        assert_ne!(snapshot_fingerprint(baseline), snapshot_fingerprint(mutant));
    }

    struct SometimesWorkload {
        hit: bool,
    }

    impl Workload for SometimesWorkload {
        fn name(&self) -> &str {
            "sometimes-observable-fixture"
        }

        fn run(&self, ctx: &mut UniverseCtx) -> RunOutcome {
            ctx.record("same", "trace");
            ctx.declare_sometimes("reached");
            if self.hit {
                ctx.sometimes("reached");
            }
            RunOutcome::Completed
        }
    }

    fn result_with_sometimes(hit: bool) -> UniverseResult {
        run_universe(1, 1, &SometimesWorkload { hit })
    }

    #[test]
    fn observable_comparison_detects_property_only_drift() {
        let mut results = [result_with_sometimes(false), result_with_sometimes(true)].into_iter();
        let error = require_observable_identity(2, || results.next().expect("two results"))
            .expect_err("property-only drift must diverge");

        assert!(matches!(&error, ReplaySoakError::Diverged { run: 1, .. }));
        assert_eq!(
            format_error_receipt(&error),
            "soak: receipt-schema=vh-verify-soak-v1 verdict=ERROR determinism-tier=Tier-1 evidence-grade=D0 root-seed=0x000000000000d1ce universe=0 workload=vh-verify-reference trace-format=v0 observable-schema=vh-verify-observable-v2 requested-runs=2 error-code=replay-diverged divergence-run=1 expected-observable-fingerprint=689b59cc59f29fd8228e4c2c0ffd5282 actual-observable-fingerprint=21d32d1fc33746e207f86ea3496d04d1"
        );
    }

    #[test]
    fn library_panic_boundary_records_the_zero_based_replay_ordinal() {
        let mut calls = 0usize;
        let error = require_observable_identity(3, || {
            calls += 1;
            if calls == 2 {
                panic!("library panic-boundary fixture");
            }
            result_with_sometimes(false)
        })
        .expect_err("the second replay invocation must fail closed");

        assert_eq!(calls, 2);
        assert_eq!(
            error,
            ReplaySoakError::Panicked {
                requested_runs: 3,
                stage: ReplayPanicStage::UniverseRun,
                run: Some(1),
            }
        );
        assert_eq!(
            format_error_receipt(&error),
            "soak: receipt-schema=vh-verify-soak-v1 verdict=ERROR determinism-tier=Tier-1 evidence-grade=D0 root-seed=0x000000000000d1ce universe=0 workload=vh-verify-reference trace-format=v0 observable-schema=vh-verify-observable-v2 requested-runs=3 error-code=panic panic-stage=universe-run panic-run=1"
        );
    }

    #[test]
    fn every_structured_drift_receipt_has_a_frozen_shape() {
        let expected = result_with_sometimes(false);
        let actual = result_with_sometimes(true);
        let baseline_drift = ReplaySoakError::BaselineDrift {
            requested_runs: 3,
            expected_fingerprint: observable_fingerprint(&expected),
            actual: Box::new(actual),
        };
        assert_eq!(
            format_error_receipt(&baseline_drift),
            "soak: receipt-schema=vh-verify-soak-v1 verdict=ERROR determinism-tier=Tier-1 evidence-grade=D0 root-seed=0x000000000000d1ce universe=0 workload=vh-verify-reference trace-format=v0 observable-schema=vh-verify-observable-v2 requested-runs=3 error-code=baseline-drift expected-observable-fingerprint=689b59cc59f29fd8228e4c2c0ffd5282 actual-observable-fingerprint=21d32d1fc33746e207f86ea3496d04d1"
        );

        let fingerprint_drift = ReplaySoakError::ObservableFingerprintDrift {
            requested_runs: 9,
            expected: REFERENCE_OBSERVABLE_FINGERPRINT,
            actual: "00000000000000000000000000000000".to_string(),
        };
        assert_eq!(
            format_error_receipt(&fingerprint_drift),
            "soak: receipt-schema=vh-verify-soak-v1 verdict=ERROR determinism-tier=Tier-1 evidence-grade=D0 root-seed=0x000000000000d1ce universe=0 workload=vh-verify-reference trace-format=v0 observable-schema=vh-verify-observable-v2 requested-runs=9 error-code=observable-fingerprint-drift expected-observable-fingerprint=39f727ed5d8a949c1f5a1243bd6d1d10 actual-observable-fingerprint=00000000000000000000000000000000"
        );

        let encoded_fingerprint_drift = ReplaySoakError::ObservableFingerprintDrift {
            requested_runs: 1,
            expected: "bad expected",
            actual: "bad\nactual=forged".to_string(),
        };
        assert_eq!(
            format_error_receipt(&encoded_fingerprint_drift),
            "soak: receipt-schema=vh-verify-soak-v1 verdict=ERROR determinism-tier=Tier-1 evidence-grade=D0 root-seed=0x000000000000d1ce universe=0 workload=vh-verify-reference trace-format=v0 observable-schema=vh-verify-observable-v2 requested-runs=1 error-code=observable-fingerprint-drift expected-observable-fingerprint=bad%20expected actual-observable-fingerprint=bad%0Aactual%3Dforged"
        );

        let workload_drift = ReplaySoakError::WorkloadIdentityDrift {
            requested_runs: 4,
            expected: REFERENCE_WORKLOAD,
            actual: "renamed workload".to_string(),
        };
        assert_eq!(
            format_error_receipt(&workload_drift),
            "soak: receipt-schema=vh-verify-soak-v1 verdict=ERROR determinism-tier=Tier-1 evidence-grade=D0 root-seed=0x000000000000d1ce universe=0 workload=vh-verify-reference trace-format=v0 observable-schema=vh-verify-observable-v2 requested-runs=4 error-code=workload-identity-drift expected-workload=vh-verify-reference actual-workload=renamed%20workload"
        );
        assert_eq!(ReferenceWorkload.name(), REFERENCE_WORKLOAD);
    }

    #[test]
    fn verifier_owned_model_rejects_every_observable_mutant() {
        let baseline = expected_reference_snapshot();
        let mut mutants = Vec::new();

        let mut changed = baseline.clone();
        changed.universe_id = changed.universe_id.wrapping_add(1);
        mutants.push(changed);

        let mut changed = baseline.clone();
        changed.trace_hash.push('0');
        mutants.push(changed);

        let mut changed = baseline.clone();
        changed.trace_events = changed.trace_events.saturating_add(1);
        mutants.push(changed);

        let mut changed = baseline.clone();
        changed.always_checks.clear();
        mutants.push(changed);

        let mut changed = baseline.clone();
        changed.always_checks[0].0.push_str("-mutant");
        mutants.push(changed);

        let mut changed = baseline.clone();
        changed.always_checks[0].1 = false;
        mutants.push(changed);

        let mut changed = baseline.clone();
        changed
            .always_failures
            .push(("failure".to_string(), "detail".to_string()));
        mutants.push(changed);

        let mut failure_baseline = baseline.clone();
        failure_baseline
            .always_failures
            .push(("failure".to_string(), "detail".to_string()));
        let mut changed = failure_baseline.clone();
        changed.always_failures[0].0.push_str("-mutant");
        assert_snapshot_mutant_rejected(&failure_baseline, &changed);
        let mut changed = failure_baseline.clone();
        changed.always_failures[0].1.push_str("-mutant");
        assert_snapshot_mutant_rejected(&failure_baseline, &changed);

        let mut changed = baseline.clone();
        changed.sometimes.clear();
        mutants.push(changed);

        let mut changed = baseline.clone();
        changed.sometimes[0].0.push_str("-mutant");
        mutants.push(changed);

        let mut changed = baseline.clone();
        changed.sometimes[0].1 = false;
        mutants.push(changed);

        for outcome in [
            OutcomeSnapshot::InvalidAssumption("missing precondition".to_string()),
            OutcomeSnapshot::ExecutionError("execution failed".to_string()),
        ] {
            let mut changed = baseline.clone();
            changed.outcome = outcome;
            mutants.push(changed);
        }

        for fault_plan in [
            FaultPlanSnapshot::SelfGenerated { consumptions: 1 },
            FaultPlanSnapshot::OverrideConsumed,
            FaultPlanSnapshot::OverrideIgnored,
            FaultPlanSnapshot::OverrideOverconsumed { consumptions: 2 },
        ] {
            let mut changed = baseline.clone();
            changed.fault_plan = fault_plan;
            mutants.push(changed);
        }

        for mutant in mutants {
            assert_snapshot_mutant_rejected(&baseline, &mutant);
        }
    }

    #[derive(Clone, Copy)]
    enum FixtureOutcome {
        Completed,
        InvalidAssumption(&'static str),
        ExecutionError(&'static str),
    }

    struct LifecycleWorkload {
        outcome: FixtureOutcome,
        fault_plan_consumptions: u64,
    }

    impl Workload for LifecycleWorkload {
        fn name(&self) -> &str {
            "lifecycle-observable-fixture"
        }

        fn run(&self, ctx: &mut UniverseCtx) -> RunOutcome {
            for _ in 0..self.fault_plan_consumptions {
                let _ = ctx.fault_plan_or(Default::default);
            }
            match self.outcome {
                FixtureOutcome::Completed => RunOutcome::Completed,
                FixtureOutcome::InvalidAssumption(detail) => {
                    RunOutcome::InvalidAssumption(detail.to_string())
                }
                FixtureOutcome::ExecutionError(detail) => {
                    RunOutcome::ExecutionError(detail.to_string())
                }
            }
        }
    }

    fn lifecycle_result(outcome: FixtureOutcome, consumptions: u64) -> UniverseResult {
        run_universe(
            7,
            0,
            &LifecycleWorkload {
                outcome,
                fault_plan_consumptions: consumptions,
            },
        )
    }

    fn lifecycle_result_with_override(consumptions: u64) -> UniverseResult {
        run_universe_with_fault_plan(
            7,
            0,
            &LifecycleWorkload {
                outcome: FixtureOutcome::Completed,
                fault_plan_consumptions: consumptions,
            },
            Default::default(),
        )
    }

    fn assert_authentic_results_differ(left: &UniverseResult, right: &UniverseResult) {
        assert!(!left.observably_equal(right));
        assert!(!observably_equal_independent(left, right));
        assert!(!observably_equal_dual(left, right));
        assert_ne!(observable_fingerprint(left), observable_fingerprint(right));
    }

    struct TraceWorkload {
        payload: &'static str,
        extra_event: bool,
    }

    impl Workload for TraceWorkload {
        fn name(&self) -> &str {
            "trace-observable-fixture"
        }

        fn run(&self, ctx: &mut UniverseCtx) -> RunOutcome {
            ctx.record("fixture", self.payload);
            if self.extra_event {
                ctx.record("fixture-extra", "same");
            }
            RunOutcome::Completed
        }
    }

    struct FailureWorkload {
        name: &'static str,
        detail: &'static str,
    }

    impl Workload for FailureWorkload {
        fn name(&self) -> &str {
            "failure-observable-fixture"
        }

        fn run(&self, ctx: &mut UniverseCtx) -> RunOutcome {
            ctx.always(self.name, false, || self.detail.to_string());
            RunOutcome::Completed
        }
    }

    #[test]
    fn authentic_runner_results_cover_non_lifecycle_observable_drift() {
        let trace_a = run_universe(
            7,
            0,
            &TraceWorkload {
                payload: "a",
                extra_event: false,
            },
        );
        let trace_b = run_universe(
            7,
            0,
            &TraceWorkload {
                payload: "b",
                extra_event: false,
            },
        );
        let trace_extra = run_universe(
            7,
            0,
            &TraceWorkload {
                payload: "a",
                extra_event: true,
            },
        );
        let other_universe = run_universe(
            7,
            1,
            &TraceWorkload {
                payload: "a",
                extra_event: false,
            },
        );
        let failure = run_universe(
            7,
            0,
            &FailureWorkload {
                name: "failure",
                detail: "detail",
            },
        );
        let failure_name = run_universe(
            7,
            0,
            &FailureWorkload {
                name: "failure-mutant",
                detail: "detail",
            },
        );
        let failure_detail = run_universe(
            7,
            0,
            &FailureWorkload {
                name: "failure",
                detail: "detail-mutant",
            },
        );
        let sometimes_unreached = result_with_sometimes(false);
        let sometimes_reached = result_with_sometimes(true);

        assert_authentic_results_differ(&trace_a, &trace_b);
        assert_authentic_results_differ(&trace_a, &trace_extra);
        assert_authentic_results_differ(&trace_a, &other_universe);
        assert_authentic_results_differ(&failure, &failure_name);
        assert_authentic_results_differ(&failure, &failure_detail);
        assert_authentic_results_differ(&sometimes_unreached, &sometimes_reached);
    }

    #[test]
    fn authentic_runner_lifecycle_variants_are_part_of_independent_identity() {
        let completed = lifecycle_result(FixtureOutcome::Completed, 0);
        let generated_once = lifecycle_result(FixtureOutcome::Completed, 1);
        let invalid = lifecycle_result(FixtureOutcome::InvalidAssumption("missing"), 0);
        let invalid_detail = lifecycle_result(FixtureOutcome::InvalidAssumption("different"), 0);
        let execution_error = lifecycle_result(FixtureOutcome::ExecutionError("failed"), 0);
        let override_consumed = lifecycle_result_with_override(1);
        let override_ignored = lifecycle_result_with_override(0);
        let override_overconsumed = lifecycle_result_with_override(2);

        assert_authentic_results_differ(&completed, &generated_once);
        assert_authentic_results_differ(&completed, &invalid);
        assert_authentic_results_differ(&invalid, &invalid_detail);
        assert_authentic_results_differ(&invalid, &execution_error);
        assert_authentic_results_differ(&override_consumed, &override_ignored);
        assert_authentic_results_differ(&override_consumed, &override_overconsumed);
        assert_eq!(
            observable_snapshot(&override_consumed).fault_plan,
            FaultPlanSnapshot::OverrideConsumed
        );
        assert_eq!(
            observable_snapshot(&override_ignored).fault_plan,
            FaultPlanSnapshot::OverrideIgnored
        );
        assert_eq!(
            observable_snapshot(&override_overconsumed).fault_plan,
            FaultPlanSnapshot::OverrideOverconsumed { consumptions: 2 }
        );
    }

    #[test]
    fn frozen_fingerprint_is_independent_of_runtime_property_constants() {
        let expected = expected_reference_snapshot();
        assert_eq!(
            snapshot_fingerprint(&expected),
            REFERENCE_OBSERVABLE_FINGERPRINT
        );
        assert_eq!(REFERENCE_ALWAYS_PROPERTY, "reference_draw_count_is_32");
        assert_eq!(
            REFERENCE_COMPLETED_PROPERTY,
            "reference_completed_all_draws"
        );
    }

    #[test]
    fn observable_fingerprint_schema_has_a_full_frozen_vector() {
        let fixture = ObservableSnapshot {
            universe_id: 0x0123_4567_89ab_cdef,
            trace_hash: "trace/雪\0".to_string(),
            trace_events: 0x0102_0304,
            always_checks: vec![
                ("check-alpha".to_string(), true),
                ("check-beta\0".to_string(), false),
            ],
            always_failures: vec![
                ("failure-one".to_string(), "detail/A".to_string()),
                ("failure-two雪".to_string(), "detail/B\0x".to_string()),
            ],
            sometimes: vec![
                ("a-unreached".to_string(), false),
                ("z-reached".to_string(), true),
            ],
            outcome: OutcomeSnapshot::ExecutionError("fixture-error\0雪".to_string()),
            fault_plan: FaultPlanSnapshot::OverrideOverconsumed { consumptions: 3 },
        };

        assert_eq!(
            snapshot_fingerprint(&fixture),
            "74caf5f7b8813583ea60c759d51f62ca"
        );
    }
}
