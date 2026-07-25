//! V1 — verifier-owned FULL-observation projection and cross-OS artifact
//! (post-audit controller §5A; audit A.4 and the independent-verifier
//! half of A.1/A.2).
//!
//! The legacy `vh-verify-observable-v2` snapshot predates C1: it does not
//! project `fault_plan_digest`, `runtime_evidence`, `schedule_policy`,
//! `decision_tape_digest`, or the two C1 canonical identities. This
//! module adds a NEW schema beside it (nothing frozen moves):
//! `vh-verify-full-observation-v1` — an independent, exhaustive
//! projection of `vh_multiverse::UniverseObservation`, exercised on a
//! verifier-owned SimRuntime reference workload rather than the shallow
//! legacy fixture.
//!
//! The soak replays each artifact universe many times (1,000 Tier-1
//! replays total across the set — the ratified criterion-1 count) and
//! requires every replay's full projection to equal the first. The
//! rendered artifact is NORMALIZED: it contains semantic observation
//! only, no OS/toolchain/wall-time provenance, so the CI matrix can
//! byte-compare it across Ubuntu, macOS, and Windows in an aggregate
//! job. Provenance is captured by the workflow as a SEPARATE artifact
//! and is expected to differ.
//!
//! Independence law: every field below is re-derived through immutable
//! public getters and destructured/matched without wildcards, so a new
//! public observable or enum variant stops THIS crate compiling until
//! the verifier decides how it is projected — the compile-time ratchet,
//! held independently of the kernel's own canonical encoding. Identity
//! bytes are re-encoded as verifier-owned hex, not copied digests.

use std::fmt::Write as _;

use vh_multiverse::{
    run_universe, InjectionOutcome, RunOutcome, RuntimeEvidence, SchedulePolicy, StepEvent,
    UniverseCtx, UniverseObservation, Workload,
};
use vh_props::{AlwaysCheck, AlwaysFailure};

/// Schema tag of the normalized artifact. Bump on ANY rendering change.
pub const FULL_OBSERVATION_SCHEMA: &str = "vh-verify-full-observation-v1";
/// The verifier-owned SimRuntime reference workload's stable name.
pub const SIM_REFERENCE_WORKLOAD: &str = "vh-verify-sim-reference";
/// Root seed shared with the rest of the battery.
pub const SIM_REFERENCE_ROOT_SEED: u64 = 0xD1CE;
/// Universes rendered into the artifact.
pub const SIM_REFERENCE_UNIVERSES: u64 = 8;
/// Replays per universe; times universes = the ratified 1,000 Tier-1
/// replays of criterion 1.
pub const SIM_REFERENCE_REPLAYS_PER_UNIVERSE: usize = 125;

const ROUNDS: u64 = 5;
const ROUND_SPACING_NANOS: u64 = 100_000;
const HORIZON_NANOS: u64 = ROUNDS * ROUND_SPACING_NANOS;

const CLIENT: u32 = 0;
const SERVER: u32 = 1;

/// Verifier-owned SimRuntime reference: each round the client pings the
/// server; the server persists the round to disk (write + flush) and
/// acks. Faults come from the workload's own deterministic stream —
/// partitions, delays, duplicates, reorders, disk write failures, and
/// fsync lies — so fault plans, runtime lifecycle evidence, and varied
/// verdicts all appear across the artifact set. No retry tuning is
/// attempted: universes where a fault defeats a round FAIL their oracle,
/// and that failing observation is exactly as replay-stable as a clean
/// one. The soak asserts replay identity, never cleanliness.
struct SimReferenceWorkload;

fn sim_reference_oracle(end: &vh_multiverse::EndState) -> Result<(), String> {
    let mut missing = Vec::new();
    for round in 0..ROUNDS {
        if end.get(&format!("acked:{round}")).map(String::as_str) != Some("true") {
            missing.push(round.to_string());
        }
    }
    if missing.is_empty() {
        Ok(())
    } else {
        Err(format!(
            "round(s) {} were never acknowledged (fault window defeated the single attempt)",
            missing.join(",")
        ))
    }
}

impl Workload for SimReferenceWorkload {
    fn name(&self) -> &str {
        SIM_REFERENCE_WORKLOAD
    }

    fn property_contract(&self) -> vh_multiverse::PropertyContract {
        vh_multiverse::PropertyContract::new(&[], &["sim_ref_fault_window"])
            .with_oracles(&["sim_ref_all_acked"])
    }

    fn end_state_oracles(&self) -> Vec<vh_multiverse::EndStateOracle> {
        vec![vh_multiverse::EndStateOracle {
            name: "sim_ref_all_acked",
            check: sim_reference_oracle,
        }]
    }

    fn run(&self, ctx: &mut UniverseCtx) -> RunOutcome {
        ctx.declare_sometimes("sim_ref_fault_window");
        let mut gremlin = ctx.stream("verify.sim.gremlin");
        let universe_seed = ctx.universe_seed();
        let fault_palette = ctx.fault_palette();
        let mut rt = ctx.runtime(|| {
            use vh_gremlin::{FaultInjection, FaultKind, FaultPlan, PaletteChooser};
            let count = 1 + gremlin.next_below(3); // 1..=3
            let chooser = PaletteChooser::new(fault_palette, universe_seed, 6);
            let injections = (0..count)
                .map(|_| {
                    let at_nanos = gremlin.next_below(HORIZON_NANOS);
                    let fault = match chooser.choose(&mut gremlin) {
                        0 => FaultKind::NetworkPartition {
                            duration_nanos: 20_000 + gremlin.next_below(60_000),
                        },
                        1 => FaultKind::NetworkDelay {
                            delay_nanos: 5_000 + gremlin.next_below(40_000),
                        },
                        2 => FaultKind::NetworkDuplicate,
                        3 => FaultKind::NetworkReorder,
                        4 => FaultKind::DiskWriteFail,
                        _ => FaultKind::FsyncLie,
                    };
                    FaultInjection { at_nanos, fault }
                })
                .collect();
            FaultPlan::new(injections)
        });

        let mut acked = [false; ROUNDS as usize];
        let mut window = false;
        for round in 0..ROUNDS {
            rt.set_timer(round * ROUND_SPACING_NANOS, round);
        }
        while let Some(ev) = rt.step() {
            match ev {
                StepEvent::Timer { token } => {
                    rt.send(CLIENT, SERVER, &format!("ping:{token}"));
                }
                StepEvent::Delivered {
                    to, payload, note, ..
                } => {
                    if note.delayed || note.duplicate || note.reordered {
                        window = true;
                    }
                    if to == SERVER {
                        if let Some(round) = payload.strip_prefix("ping:") {
                            // Server: persist, then ack. A failed write
                            // or a lying fsync marks the fault window;
                            // durability truth is the oracle's business.
                            if rt.disk_write(SERVER, &format!("round:{round}")).is_err() {
                                window = true;
                            }
                            if rt.disk_flush(SERVER).is_err() {
                                window = true;
                            }
                            rt.send(SERVER, CLIENT, &format!("pong:{round}"));
                        }
                    } else if let Some(round) = payload.strip_prefix("pong:") {
                        let idx: u64 = round.parse().expect("deterministic payload");
                        if (idx as usize) < acked.len() {
                            acked[idx as usize] = true;
                        }
                    }
                }
                StepEvent::Crashed => {
                    // Defensive: this palette generates no CrashRestart,
                    // but a crash IS a fault window if one ever appears.
                    window = true;
                }
            }
        }
        if rt.drops() > 0 {
            window = true;
        }
        if window {
            rt.sometimes("sim_ref_fault_window");
        }
        for (round, was_acked) in acked.iter().enumerate() {
            rt.declare_end(&format!("acked:{round}"), &was_acked.to_string());
        }
        rt.finish();
        RunOutcome::Completed
    }
}

/// The verifier-owned reference workload as a public probe surface for
/// the crate's own tests and future battery extensions.
pub fn sim_reference_workload() -> impl Workload {
    SimReferenceWorkload
}

/// The verifier's exhaustive projection of one universe's public
/// observation. Plain owned data: unit tests may mutate it to probe the
/// comparison logic; kernel results only enter through
/// [`full_snapshot`], which reads immutable getters.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FullSnapshot {
    pub universe_id: u64,
    pub trace_hash: String,
    pub trace_events: usize,
    pub always_checks: Vec<(String, bool)>,
    pub always_failures: Vec<(String, String)>,
    pub sometimes: Vec<(String, bool)>,
    pub outcome: String,
    pub fault_plan_discipline: String,
    pub fault_plan_digest: Option<String>,
    pub runtime_injections: Option<Vec<String>>,
    pub schedule_policy: String,
    pub decision_tape_digest: Option<String>,
    pub end_state_schema: String,
    pub end_state_algorithm: String,
    pub end_state_hex: String,
    pub observation_schema: String,
    pub observation_algorithm: String,
    pub observation_hex: String,
}

fn hex(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut out = String::with_capacity(bytes.len() * 2);
    for &b in bytes {
        out.push(HEX[(b >> 4) as usize] as char);
        out.push(HEX[(b & 0x0f) as usize] as char);
    }
    out
}

fn injection_line(outcome: &InjectionOutcome) -> String {
    fn stage(v: Option<u64>) -> String {
        v.map_or_else(|| "absent".to_string(), |t| t.to_string())
    }
    format!(
        "at={} fault={} offered={} armed={} injected={} manifested={} recovered={}",
        outcome.at_nanos(),
        outcome.fault(),
        stage(outcome.offered_at()),
        stage(outcome.armed_at()),
        stage(outcome.injected_at()),
        stage(outcome.manifested_at()),
        stage(outcome.recovered_at()),
    )
}

fn runtime_lines(evidence: &RuntimeEvidence) -> Vec<String> {
    evidence.injections().iter().map(injection_line).collect()
}

/// Project the COMPLETE observation. The destructuring carries no `..`:
/// a new public observable fails compilation here until the verifier
/// decides its projection.
pub fn full_snapshot(result: &vh_multiverse::UniverseResult) -> FullSnapshot {
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
        schedule_policy,
        decision_tape_digest,
        end_state_identity,
        complete_observation_identity,
    } = result.observation();
    let outcome = match lifecycle.outcome() {
        RunOutcome::Completed => "completed".to_string(),
        RunOutcome::InvalidAssumption(detail) => format!("invalid-assumption:{detail}"),
        RunOutcome::ExecutionError(detail) => format!("execution-error:{detail}"),
    };
    let fault_plan_discipline = match lifecycle.fault_plan() {
        vh_multiverse::FaultPlanDiscipline::SelfGenerated { retrievals } => {
            format!("self-generated:{retrievals}")
        }
        vh_multiverse::FaultPlanDiscipline::OverrideRetrieved => "override-retrieved".to_string(),
        vh_multiverse::FaultPlanDiscipline::OverrideNeverRetrieved => {
            "override-never-retrieved".to_string()
        }
        vh_multiverse::FaultPlanDiscipline::OverrideRetrievedMultiply { retrievals } => {
            format!("override-retrieved-multiply:{retrievals}")
        }
    };
    let schedule = match schedule_policy {
        SchedulePolicy::Fifo => "fifo".to_string(),
        SchedulePolicy::Pct { depth } => format!("pct:{depth}"),
        SchedulePolicy::UniformTiebreak => "uniform".to_string(),
    };
    FullSnapshot {
        universe_id,
        trace_hash: trace_hash.to_string(),
        trace_events,
        always_checks: always_checks
            .iter()
            .map(|check| {
                let AlwaysCheck { name, passed } = check;
                (name.clone(), *passed)
            })
            .collect(),
        always_failures: always_failures
            .iter()
            .map(|failure| {
                let AlwaysFailure { name, detail } = failure;
                (name.clone(), detail.clone())
            })
            .collect(),
        sometimes: sometimes
            .iter()
            .map(|(name, reached)| (name.clone(), *reached))
            .collect(),
        outcome,
        fault_plan_discipline,
        fault_plan_digest: fault_plan_digest.map(str::to_string),
        runtime_injections: runtime_evidence.map(runtime_lines),
        schedule_policy: schedule,
        decision_tape_digest: decision_tape_digest.map(str::to_string),
        end_state_schema: end_state_identity.schema().to_string(),
        end_state_algorithm: end_state_identity.algorithm().to_string(),
        end_state_hex: hex(end_state_identity.canonical_bytes()),
        observation_schema: complete_observation_identity.schema().to_string(),
        observation_algorithm: complete_observation_identity.algorithm().to_string(),
        observation_hex: hex(complete_observation_identity.canonical_bytes()),
    }
}

/// Render one snapshot as normalized artifact lines. Deterministic,
/// provenance-free text: field per line, `escape`d free-text values.
fn render_snapshot(out: &mut String, s: &FullSnapshot) {
    fn escape(v: &str) -> String {
        v.chars()
            .flat_map(|c| match c {
                '\n' => "\\n".chars().collect::<Vec<_>>(),
                '\\' => "\\\\".chars().collect(),
                c => vec![c],
            })
            .collect()
    }
    let _ = writeln!(out, "universe {}", s.universe_id);
    let _ = writeln!(out, "  trace-hash {}", s.trace_hash);
    let _ = writeln!(out, "  trace-events {}", s.trace_events);
    for (name, passed) in &s.always_checks {
        let _ = writeln!(out, "  check {} {}", escape(name), passed);
    }
    for (name, detail) in &s.always_failures {
        let _ = writeln!(out, "  failure {} {}", escape(name), escape(detail));
    }
    for (name, reached) in &s.sometimes {
        let _ = writeln!(out, "  sometimes {} {}", escape(name), reached);
    }
    let _ = writeln!(out, "  outcome {}", escape(&s.outcome));
    let _ = writeln!(out, "  fault-plan-discipline {}", s.fault_plan_discipline);
    let _ = writeln!(
        out,
        "  fault-plan-digest {}",
        s.fault_plan_digest.as_deref().unwrap_or("absent")
    );
    match &s.runtime_injections {
        None => {
            let _ = writeln!(out, "  runtime-evidence absent");
        }
        Some(lines) => {
            let _ = writeln!(out, "  runtime-evidence {}", lines.len());
            for line in lines {
                let _ = writeln!(out, "    injection {}", escape(line));
            }
        }
    }
    let _ = writeln!(out, "  schedule-policy {}", s.schedule_policy);
    let _ = writeln!(
        out,
        "  decision-tape-digest {}",
        s.decision_tape_digest.as_deref().unwrap_or("absent")
    );
    let _ = writeln!(
        out,
        "  end-state-identity {} {} {}",
        s.end_state_schema, s.end_state_algorithm, s.end_state_hex
    );
    let _ = writeln!(
        out,
        "  complete-observation-identity {} {} {}",
        s.observation_schema, s.observation_algorithm, s.observation_hex
    );
}

/// Typed soak failure: which universe/replay broke identity.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FullObservationError {
    ZeroWork,
    Diverged { universe: u64, replay: usize },
}

impl std::fmt::Display for FullObservationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ZeroWork => f.write_str("full-observation soak requires nonzero work"),
            Self::Diverged { universe, replay } => write!(
                f,
                "full-observation divergence: universe {universe} replay {replay} disagreed with replay 0"
            ),
        }
    }
}

/// Run the full-observation soak and render the normalized artifact.
/// Every universe in `0..universes` is replayed `replays` times; every
/// replay's FULL projection must equal replay 0's. Total Tier-1 replays
/// = `universes * replays`.
pub fn full_observation_artifact(
    universes: u64,
    replays: usize,
) -> Result<String, FullObservationError> {
    if universes == 0 || replays == 0 {
        return Err(FullObservationError::ZeroWork);
    }
    let workload = SimReferenceWorkload;
    let mut out = String::new();
    let _ = writeln!(
        out,
        "schema {FULL_OBSERVATION_SCHEMA} workload {SIM_REFERENCE_WORKLOAD} root-seed 0x{SIM_REFERENCE_ROOT_SEED:x} universes {universes} replays-per-universe {replays}"
    );
    for universe in 0..universes {
        let first = full_snapshot(&run_universe(SIM_REFERENCE_ROOT_SEED, universe, &workload));
        for replay in 1..replays {
            let again = full_snapshot(&run_universe(SIM_REFERENCE_ROOT_SEED, universe, &workload));
            if again != first {
                return Err(FullObservationError::Diverged { universe, replay });
            }
        }
        render_snapshot(&mut out, &first);
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn artifact_is_deterministic_and_schema_tagged() {
        let a = full_observation_artifact(4, 3).expect("soak passes");
        let b = full_observation_artifact(4, 3).expect("soak passes");
        assert_eq!(a, b, "normalized artifact must be byte-deterministic");
        assert!(a.starts_with(&format!("schema {FULL_OBSERVATION_SCHEMA} ")));
        // The artifact must actually exercise the post-C1 schema.
        assert!(a.contains("  fault-plan-digest "), "{a}");
        assert!(a.contains("  end-state-identity vh-end-state-observation-v1 "));
        assert!(a.contains("  complete-observation-identity vh-complete-observation-v1 "));
        assert!(a.contains("  schedule-policy fifo"));
        assert!(a.contains("  runtime-evidence "), "{a}");
    }

    #[test]
    fn faults_actually_manifest_somewhere_in_the_artifact_set() {
        let a = full_observation_artifact(SIM_REFERENCE_UNIVERSES, 1).expect("soak passes");
        // At least one universe must reach the fault window and at least
        // one injection lifecycle must appear — otherwise the reference
        // is as shallow as the legacy fixture and proves nothing new.
        assert!(
            a.contains("sometimes sim_ref_fault_window true"),
            "no universe reached the fault window:\n{a}"
        );
        assert!(
            a.contains("    injection at="),
            "no injection evidence:\n{a}"
        );
    }

    #[test]
    fn zero_work_fails_closed() {
        assert_eq!(
            full_observation_artifact(0, 5).unwrap_err(),
            FullObservationError::ZeroWork
        );
        assert_eq!(
            full_observation_artifact(5, 0).unwrap_err(),
            FullObservationError::ZeroWork
        );
    }

    /// Mutating any single projected field must break snapshot equality —
    /// the comparison is the whole projection, not a subset.
    #[test]
    fn every_projected_field_participates_in_equality() {
        let base = full_snapshot(&run_universe(
            SIM_REFERENCE_ROOT_SEED,
            0,
            &SimReferenceWorkload,
        ));
        let mut mutations: Vec<FullSnapshot> = Vec::new();
        {
            let mut m = base.clone();
            m.trace_hash = "0".repeat(32);
            mutations.push(m);
        }
        {
            let mut m = base.clone();
            m.trace_events += 1;
            mutations.push(m);
        }
        {
            let mut m = base.clone();
            m.always_checks.push(("forged".into(), true));
            mutations.push(m);
        }
        {
            let mut m = base.clone();
            m.sometimes.push(("forged".into(), false));
            mutations.push(m);
        }
        {
            let mut m = base.clone();
            m.outcome = "execution-error:forged".into();
            mutations.push(m);
        }
        {
            let mut m = base.clone();
            m.fault_plan_digest = Some("f".repeat(32));
            mutations.push(m);
        }
        {
            let mut m = base.clone();
            m.schedule_policy = "pct:3".into();
            mutations.push(m);
        }
        {
            let mut m = base.clone();
            m.end_state_hex.push_str("00");
            mutations.push(m);
        }
        {
            let mut m = base.clone();
            m.observation_hex.push_str("00");
            mutations.push(m);
        }
        for (i, m) in mutations.iter().enumerate() {
            assert_ne!(*m, base, "mutation {i} did not change equality");
        }
    }
}
