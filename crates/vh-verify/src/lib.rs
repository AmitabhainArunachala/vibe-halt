#![forbid(unsafe_code)]

//! Independent Tier-1 replay probes built only on public kernel APIs.
//!
//! The verifier never imports a Track-1 observable fingerprint. It projects
//! [`vh_multiverse::UniverseResult::observation`] into verifier-owned plain
//! data, exhaustively ratchets every public field and enum variant, and applies
//! its own versioned framing.

use std::error::Error;
use std::fmt;

use vh_multiverse::{
    run_universe, FaultPlanDiscipline, PropertyContract, RunOutcome, UniverseCtx,
    UniverseObservation, UniverseResult, Workload,
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
pub const OBSERVABLE_FINGERPRINT_SCHEMA: &str = "vh-verify-observable-v3";
pub const REFERENCE_OBSERVABLE_FINGERPRINT: &str = "bf78c94b6f72ae77ad0a00a86e36c2e9";
pub const SOAK_RECEIPT_SCHEMA: &str = "vh-verify-soak-v1";

const FNV128_OFFSET: u128 = 0x6c62_272e_07bb_0142_62b8_2175_6295_c58d;
const FNV128_PRIME: u128 = 0x0000_0000_0100_0000_0000_0000_0000_013B;

struct ReferenceWorkload;

impl Workload for ReferenceWorkload {
    fn name(&self) -> &str {
        REFERENCE_WORKLOAD
    }

    fn property_contract(&self) -> PropertyContract {
        PropertyContract::new(
            &[REFERENCE_ALWAYS_PROPERTY],
            &[REFERENCE_COMPLETED_PROPERTY],
        )
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

/// Verifier-owned projection of every public universe observable in schema v3.
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
    fault_plan_digest: Option<String>,
    runtime_evidence: Option<Vec<InjectionSnapshot>>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum OutcomeSnapshot {
    Completed,
    InvalidAssumption(String),
    ExecutionError(String),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum FaultPlanSnapshot {
    SelfGenerated { retrievals: u64 },
    OverrideRetrieved,
    OverrideNeverRetrieved,
    OverrideRetrievedMultiply { retrievals: u64 },
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct InjectionSnapshot {
    at_nanos: u64,
    fault: String,
    offered_at: Option<u64>,
    armed_at: Option<u64>,
    injected_at: Option<u64>,
    manifested_at: Option<u64>,
    recovered_at: Option<u64>,
}

fn outcome_snapshot(outcome: &RunOutcome) -> OutcomeSnapshot {
    match outcome {
        RunOutcome::Completed => OutcomeSnapshot::Completed,
        RunOutcome::InvalidAssumption(detail) => OutcomeSnapshot::InvalidAssumption(detail.clone()),
        RunOutcome::ExecutionError(detail) => OutcomeSnapshot::ExecutionError(detail.clone()),
    }
}

fn fault_plan_snapshot(discipline: &FaultPlanDiscipline) -> FaultPlanSnapshot {
    match discipline {
        FaultPlanDiscipline::SelfGenerated { retrievals } => FaultPlanSnapshot::SelfGenerated {
            retrievals: *retrievals,
        },
        FaultPlanDiscipline::OverrideRetrieved => FaultPlanSnapshot::OverrideRetrieved,
        FaultPlanDiscipline::OverrideNeverRetrieved => FaultPlanSnapshot::OverrideNeverRetrieved,
        FaultPlanDiscipline::OverrideRetrievedMultiply { retrievals } => {
            FaultPlanSnapshot::OverrideRetrievedMultiply {
                retrievals: *retrievals,
            }
        }
    }
}

/// Project only through the granted read-only observation view. The pattern is
/// intentionally `..`-free: adding a public observable field makes this crate
/// fail compilation until its verifier-owned schema is deliberately revised.
fn observable_snapshot(result: &UniverseResult) -> ObservableSnapshot {
    let UniverseObservation {
        universe_id,
        trace_hash,
        trace_events,
        always_checks,
        always_failures,
        sometimes,
        lifecycle,
        fault_plan_digest,
        runtime_evidence,
    } = result.observation();

    let always_checks = always_checks
        .iter()
        .map(|check| {
            let AlwaysCheck { name, passed } = check;
            (name.clone(), *passed)
        })
        .collect();
    let always_failures = always_failures
        .iter()
        .map(|failure| {
            let AlwaysFailure { name, detail } = failure;
            (name.clone(), detail.clone())
        })
        .collect();
    let sometimes = sometimes
        .iter()
        .map(|(name, reached)| (name.clone(), *reached))
        .collect();
    let runtime_evidence = runtime_evidence.map(|evidence| {
        evidence
            .injections()
            .iter()
            .map(|injection| InjectionSnapshot {
                at_nanos: injection.at_nanos(),
                fault: injection.fault().to_string(),
                offered_at: injection.offered_at(),
                armed_at: injection.armed_at(),
                injected_at: injection.injected_at(),
                manifested_at: injection.manifested_at(),
                recovered_at: injection.recovered_at(),
            })
            .collect()
    });

    ObservableSnapshot {
        universe_id,
        trace_hash: trace_hash.to_string(),
        trace_events,
        always_checks,
        always_failures,
        sometimes,
        outcome: outcome_snapshot(lifecycle.outcome()),
        fault_plan: fault_plan_snapshot(lifecycle.fault_plan()),
        fault_plan_digest: fault_plan_digest.map(str::to_string),
        runtime_evidence,
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

fn fingerprint_absorb_count(state: u128, count: usize, label: &str) -> u128 {
    fingerprint_absorb(
        state,
        &u64::try_from(count)
            .unwrap_or_else(|_| panic!("{label} must fit u64"))
            .to_le_bytes(),
    )
}

fn fingerprint_absorb_option_u64(mut state: u128, value: Option<u64>) -> u128 {
    match value {
        None => fingerprint_absorb(state, &[0]),
        Some(value) => {
            state = fingerprint_absorb(state, &[1]);
            fingerprint_absorb(state, &value.to_le_bytes())
        }
    }
}

fn snapshot_fingerprint(snapshot: &ObservableSnapshot) -> String {
    let mut state =
        fingerprint_absorb_bytes(FNV128_OFFSET, OBSERVABLE_FINGERPRINT_SCHEMA.as_bytes());
    state = fingerprint_absorb(state, &snapshot.universe_id.to_le_bytes());
    state = fingerprint_absorb_bytes(state, snapshot.trace_hash.as_bytes());
    state = fingerprint_absorb_count(state, snapshot.trace_events, "trace event count");

    state = fingerprint_absorb_count(state, snapshot.always_checks.len(), "always-check count");
    for (name, passed) in &snapshot.always_checks {
        state = fingerprint_absorb_bytes(state, name.as_bytes());
        state = fingerprint_absorb(state, &[u8::from(*passed)]);
    }

    state = fingerprint_absorb_count(
        state,
        snapshot.always_failures.len(),
        "always-failure count",
    );
    for (name, detail) in &snapshot.always_failures {
        state = fingerprint_absorb_bytes(state, name.as_bytes());
        state = fingerprint_absorb_bytes(state, detail.as_bytes());
    }

    state = fingerprint_absorb_count(state, snapshot.sometimes.len(), "sometimes count");
    for (name, reached) in &snapshot.sometimes {
        state = fingerprint_absorb_bytes(state, name.as_bytes());
        state = fingerprint_absorb(state, &[u8::from(*reached)]);
    }

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
        FaultPlanSnapshot::SelfGenerated { retrievals } => {
            state = fingerprint_absorb_bytes(state, b"fault-plan.self-generated");
            state = fingerprint_absorb(state, &retrievals.to_le_bytes());
        }
        FaultPlanSnapshot::OverrideRetrieved => {
            state = fingerprint_absorb_bytes(state, b"fault-plan.override-retrieved");
        }
        FaultPlanSnapshot::OverrideNeverRetrieved => {
            state = fingerprint_absorb_bytes(state, b"fault-plan.override-never-retrieved");
        }
        FaultPlanSnapshot::OverrideRetrievedMultiply { retrievals } => {
            state = fingerprint_absorb_bytes(state, b"fault-plan.override-retrieved-multiply");
            state = fingerprint_absorb(state, &retrievals.to_le_bytes());
        }
    }

    match &snapshot.fault_plan_digest {
        None => {
            state = fingerprint_absorb(state, &[0]);
        }
        Some(digest) => {
            state = fingerprint_absorb(state, &[1]);
            state = fingerprint_absorb_bytes(state, digest.as_bytes());
        }
    }

    match &snapshot.runtime_evidence {
        None => {
            state = fingerprint_absorb(state, &[0]);
        }
        Some(injections) => {
            state = fingerprint_absorb(state, &[1]);
            state = fingerprint_absorb_count(state, injections.len(), "runtime injection count");
            for injection in injections {
                state = fingerprint_absorb(state, &injection.at_nanos.to_le_bytes());
                state = fingerprint_absorb_bytes(state, injection.fault.as_bytes());
                state = fingerprint_absorb_option_u64(state, injection.offered_at);
                state = fingerprint_absorb_option_u64(state, injection.armed_at);
                state = fingerprint_absorb_option_u64(state, injection.injected_at);
                state = fingerprint_absorb_option_u64(state, injection.manifested_at);
                state = fingerprint_absorb_option_u64(state, injection.recovered_at);
            }
        }
    }

    format!("{state:032x}")
}

/// Verifier-owned complete-observable fingerprint. This is a Tier-1
/// compatibility identity, not a cryptographic integrity proof.
pub fn observable_fingerprint(result: &UniverseResult) -> String {
    snapshot_fingerprint(&observable_snapshot(result))
}

/// Compare the complete verifier projection independently of Track-1's own
/// whole-result equality implementation.
pub fn observably_equal_independent(left: &UniverseResult, right: &UniverseResult) -> bool {
    observable_snapshot(left) == observable_snapshot(right)
}

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
        universe_id: REFERENCE_UNIVERSE,
        trace_hash: REFERENCE_TRACE_HASH.to_string(),
        trace_events: REFERENCE_TRACE_EVENTS,
        always_checks: vec![(REFERENCE_ALWAYS_PROPERTY.to_string(), true)],
        always_failures: Vec::new(),
        sometimes: vec![(REFERENCE_COMPLETED_PROPERTY.to_string(), true)],
        outcome: OutcomeSnapshot::Completed,
        fault_plan: FaultPlanSnapshot::SelfGenerated { retrievals: 0 },
        fault_plan_digest: None,
        runtime_evidence: None,
    }
}

/// Sequentially replay one fixed reference universe. Every run must satisfy
/// runner-owned whole-result equality and the independent verifier projection;
/// the projection must also match the frozen v3 baseline.
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

/// One machine-readable Tier-1 soak receipt. Throughput is boundary telemetry
/// only and never enters replay identity.
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
/// excluded; variable tokens are percent encoded.
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
    use std::collections::BTreeMap;

    use super::*;

    fn assert_snapshot_mutant_rejected(
        baseline: &ObservableSnapshot,
        mutant: &ObservableSnapshot,
    ) {
        assert_ne!(baseline, mutant);
        assert_ne!(snapshot_fingerprint(baseline), snapshot_fingerprint(mutant));
    }

    #[test]
    fn independently_derived_reference_v3_fingerprint_is_frozen() {
        let snapshot = expected_reference_snapshot();
        assert_eq!(snapshot_fingerprint(&snapshot), REFERENCE_OBSERVABLE_FINGERPRINT);
    }

    #[test]
    fn v3_framing_covers_digest_runtime_and_every_lifecycle_stage() {
        let baseline = expected_reference_snapshot();

        let mut digest = baseline.clone();
        digest.fault_plan_digest = Some("abc".to_string());
        assert_snapshot_mutant_rejected(&baseline, &digest);

        let mut empty_runtime = baseline.clone();
        empty_runtime.runtime_evidence = Some(Vec::new());
        assert_snapshot_mutant_rejected(&baseline, &empty_runtime);

        let injection = InjectionSnapshot {
            at_nanos: 7,
            fault: "network_delay:0".to_string(),
            offered_at: Some(7),
            armed_at: Some(7),
            injected_at: Some(8),
            manifested_at: Some(1_008),
            recovered_at: Some(1_008),
        };
        let mut runtime = baseline.clone();
        runtime.runtime_evidence = Some(vec![injection.clone()]);
        assert_snapshot_mutant_rejected(&baseline, &runtime);

        for mutate in 0..7 {
            let mut changed = runtime.clone();
            let item = &mut changed
                .runtime_evidence
                .as_mut()
                .expect("runtime evidence")
                [0];
            match mutate {
                0 => item.at_nanos += 1,
                1 => item.fault.push('x'),
                2 => item.offered_at = None,
                3 => item.armed_at = None,
                4 => item.injected_at = None,
                5 => item.manifested_at = None,
                6 => item.recovered_at = None,
                _ => unreachable!(),
            }
            assert_snapshot_mutant_rejected(&runtime, &changed);
        }
    }

    #[test]
    fn oracle_transcript_entries_are_observable() {
        let baseline = expected_reference_snapshot();
        let mut oracle = baseline.clone();
        oracle
            .always_checks
            .push(("oracle:durability".to_string(), true));
        assert_snapshot_mutant_rejected(&baseline, &oracle);
    }

    #[test]
    fn btree_iteration_is_preserved_in_projection_order() {
        let mut map = BTreeMap::new();
        map.insert("z".to_string(), true);
        map.insert("a".to_string(), false);
        let projected: Vec<(String, bool)> = map.into_iter().collect();
        assert_eq!(
            projected,
            vec![("a".to_string(), false), ("z".to_string(), true)]
        );
    }
}
