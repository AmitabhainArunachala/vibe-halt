//! The divergence detector is CI gate #1. These tests prove both directions:
//! a deterministic workload replays bit-identically, and a workload with
//! smuggled-in nondeterminism is caught, not silently blessed.

use std::sync::atomic::{AtomicU64, Ordering};

use vh_gremlin::FaultPlan;
use vh_multiverse::{
    run_multiverse, run_universe, run_universe_with_fault_plan, FaultPlanDiscipline,
    MultiverseConfig, PropertyContract, RunOutcome, UniverseCount, UniverseCtx, Verdict, Workload,
};

fn count(n: u64) -> UniverseCount {
    UniverseCount::try_from(n).unwrap()
}

/// A small deterministic workload: draws ops and a fault plan from named
/// streams, records everything.
struct DeterministicDemo;

impl Workload for DeterministicDemo {
    fn name(&self) -> &str {
        "deterministic-demo"
    }

    fn run(&self, ctx: &mut UniverseCtx) -> RunOutcome {
        let mut ops = ctx.stream("ops");
        let mut gremlin = ctx.stream("gremlin");
        let plan = ctx.fault_plan_or(|| FaultPlan::generate(&mut gremlin, 1_000_000, 4));
        let mut cursor = 0;
        for i in 0..50u64 {
            let now = i * 20_000;
            ctx.advance_to(now);
            let (next, due) = plan.due(cursor, now);
            cursor = next;
            for inj in due {
                let label = inj.fault.label().to_string();
                ctx.record("fault", &label);
            }
            let v = ops.next_below(1000);
            ctx.record("op", &format!("i={i} v={v}"));
        }
        RunOutcome::Completed
    }
}

/// A workload that leaks process-global state into the trace: the second
/// run of the same universe sees different counter values. This is exactly
/// the class of bug the detector exists to catch.
static LEAKY_COUNTER: AtomicU64 = AtomicU64::new(0);

struct NondeterministicDemo;

impl Workload for NondeterministicDemo {
    fn name(&self) -> &str {
        "nondeterministic-demo"
    }

    fn run(&self, ctx: &mut UniverseCtx) -> RunOutcome {
        let leaked = LEAKY_COUNTER.fetch_add(1, Ordering::SeqCst);
        ctx.record("leak", &format!("counter={leaked}"));
        RunOutcome::Completed
    }
}

#[test]
fn same_universe_replays_bit_identically() {
    let w = DeterministicDemo;
    for universe_id in 0..10 {
        let a = run_universe(0xD1CE, universe_id, &w);
        let b = run_universe(0xD1CE, universe_id, &w);
        assert_eq!(
            a.trace_hash(),
            b.trace_hash(),
            "universe {universe_id} diverged"
        );
        assert!(!a.trace_hash().is_empty());
    }
}

#[test]
fn multiverse_replays_across_many_runs() {
    let w = DeterministicDemo;
    let cfg = MultiverseConfig {
        root_seed: 42,
        universes: count(25),
        check_divergence: true,
    };
    let report = run_multiverse(&cfg, &w);
    assert!(
        report.divergent_universes().is_empty(),
        "divergent universes: {:?}",
        report.divergent_universes()
    );

    // The whole-report fingerprint must also be stable across invocations.
    let hashes: Vec<String> = report
        .results()
        .iter()
        .map(|r| r.trace_hash().to_string())
        .collect();
    let report2 = run_multiverse(&cfg, &w);
    let hashes2: Vec<String> = report2
        .results()
        .iter()
        .map(|r| r.trace_hash().to_string())
        .collect();
    assert_eq!(hashes, hashes2);
}

/// The shrinker's oracle surface: an override plan replaces the
/// workload-generated one through the identical code path, deterministically.
#[test]
fn fault_plan_override_replays_deterministically() {
    let w = DeterministicDemo;
    let baseline = run_universe(0xD1CE, 3, &w);
    assert_eq!(
        baseline.lifecycle().fault_plan(),
        &FaultPlanDiscipline::SelfGenerated { retrievals: 1 }
    );

    // Empty plan: no faults fire; the run must differ from the baseline
    // (whose generated plan injects 4 faults) but replay identically.
    let empty = FaultPlan::default();
    let a = run_universe_with_fault_plan(0xD1CE, 3, &w, empty.clone());
    let b = run_universe_with_fault_plan(0xD1CE, 3, &w, empty);
    assert_eq!(a, b, "override replay must be bit-identical");
    assert_ne!(
        a.trace_hash(),
        baseline.trace_hash(),
        "empty override must actually suppress the generated faults"
    );
    assert_eq!(
        a.lifecycle().fault_plan(),
        &FaultPlanDiscipline::OverrideRetrieved
    );
    assert!(a.lifecycle().is_valid_completion());

    // Overriding with the plan the workload would have generated anyway
    // must reproduce the baseline's observables exactly — proving override
    // and generated paths are the same path. (Lifecycles differ by
    // provenance — SelfGenerated vs OverrideRetrieved — which is honest:
    // they ARE different modes; the workload-visible path is identical.)
    let mut gremlin = vh_core::SeedTree::new(0xD1CE).stream(3, "gremlin");
    let generated = FaultPlan::generate(&mut gremlin, 1_000_000, 4);
    let c = run_universe_with_fault_plan(0xD1CE, 3, &w, generated);
    assert_eq!(c.trace_hash(), baseline.trace_hash());
    assert_eq!(c.trace_events(), baseline.trace_events());
    assert_eq!(c.always_checks(), baseline.always_checks());
    assert_eq!(c.always_failures(), baseline.always_failures());
    assert_eq!(c.sometimes(), baseline.sometimes());
}

/// Negative regression (hardening-loop-2 BLOCKER): a workload whose plan
/// GENERATOR shares a PRNG stream with draws it makes later. The pre-repair
/// `fault_plan_or` skipped the generator in override mode, so replaying the
/// workload's own generated plan consumed fewer stream words and the
/// "identical path" claim was false (reproduced: baseline and replay trace
/// hashes differed). The generator is now always evaluated.
struct SharedStreamDemo;

impl Workload for SharedStreamDemo {
    fn name(&self) -> &str {
        "shared-stream-demo"
    }

    fn run(&self, ctx: &mut UniverseCtx) -> RunOutcome {
        let mut g = ctx.stream("gremlin");
        let plan = ctx.fault_plan_or(|| FaultPlan::generate(&mut g, 1_000, 2));
        let _ = plan;
        // Reuse the SAME stream after generation — the adversarial part.
        let v = g.next_below(1000);
        ctx.record("post", &v.to_string());
        RunOutcome::Completed
    }
}

#[test]
fn override_preserves_generator_stream_effects() {
    let w = SharedStreamDemo;
    let baseline = run_universe(7, 0, &w);
    // Derive the exact plan the workload generates, then replay it as the
    // override: the observable run must be identical.
    let mut g = vh_core::SeedTree::new(7).stream(0, "gremlin");
    let generated = FaultPlan::generate(&mut g, 1_000, 2);
    let replay = run_universe_with_fault_plan(7, 0, &w, generated);
    assert_eq!(
        replay.trace_hash(),
        baseline.trace_hash(),
        "override replay must consume generator draws identically"
    );
    assert_eq!(replay.trace_events(), baseline.trace_events());
    assert_eq!(
        replay.lifecycle().fault_plan(),
        &FaultPlanDiscipline::OverrideRetrieved
    );
}

/// A workload that never asks for its fault plan: under an override that
/// is a broken replay (the supplied plan was ignored), and it must fail
/// closed instead of masquerading as a valid run (hardening-loop-2
/// BLOCKER).
struct IgnoresPlanDemo;

impl Workload for IgnoresPlanDemo {
    fn name(&self) -> &str {
        "ignores-plan-demo"
    }

    fn run(&self, ctx: &mut UniverseCtx) -> RunOutcome {
        ctx.record("op", "no plan consulted");
        RunOutcome::Completed
    }
}

#[test]
fn ignored_override_is_never_a_valid_completion() {
    let w = IgnoresPlanDemo;
    let result = run_universe_with_fault_plan(1, 0, &w, FaultPlan::default());
    assert_eq!(
        result.lifecycle().fault_plan(),
        &FaultPlanDiscipline::OverrideNeverRetrieved
    );
    assert!(!result.lifecycle().is_valid_completion());
    // Without an override the same workload is fine — not every workload
    // uses fault plans.
    let plain = run_universe(1, 0, &w);
    assert_eq!(
        plain.lifecycle().fault_plan(),
        &FaultPlanDiscipline::SelfGenerated { retrievals: 0 }
    );
    assert!(plain.lifecycle().is_valid_completion());
}

/// A workload that consumes its plan twice: ambiguous replay, fails closed.
struct DoubleConsumeDemo;

impl Workload for DoubleConsumeDemo {
    fn name(&self) -> &str {
        "double-consume-demo"
    }

    fn run(&self, ctx: &mut UniverseCtx) -> RunOutcome {
        let mut g = ctx.stream("gremlin");
        let _ = ctx.fault_plan_or(|| FaultPlan::generate(&mut g, 1_000, 1));
        let mut g2 = ctx.stream("gremlin2");
        let _ = ctx.fault_plan_or(|| FaultPlan::generate(&mut g2, 1_000, 1));
        RunOutcome::Completed
    }
}

#[test]
fn overconsumed_override_is_never_a_valid_completion() {
    let w = DoubleConsumeDemo;
    let result = run_universe_with_fault_plan(1, 0, &w, FaultPlan::default());
    assert_eq!(
        result.lifecycle().fault_plan(),
        &FaultPlanDiscipline::OverrideRetrievedMultiply { retrievals: 2 }
    );
    assert!(!result.lifecycle().is_valid_completion());
}

/// Negative regressions (hardening-loop-2 BLOCKER): a workload that does
/// not affirmatively complete can never reach CLEAN, whatever its finding
/// ledger looks like. Pre-repair, `Workload::run` returned nothing and an
/// empty ledger was certified.
struct ErroringDemo(RunOutcome);

impl Workload for ErroringDemo {
    fn name(&self) -> &str {
        "erroring-demo"
    }

    fn run(&self, _ctx: &mut UniverseCtx) -> RunOutcome {
        self.0.clone()
    }
}

#[test]
fn execution_error_outcome_is_never_clean() {
    let w = ErroringDemo(RunOutcome::ExecutionError("simulated".into()));
    let cfg = MultiverseConfig {
        root_seed: 1,
        universes: count(2),
        check_divergence: true,
    };
    let report = run_multiverse(&cfg, &w);
    assert_eq!(report.invalid_universes(), vec![0, 1]);
    assert!(
        !report.is_clean(),
        "an erroring workload must never be CLEAN"
    );
}

#[test]
fn invalid_assumption_outcome_is_never_clean() {
    let w = ErroringDemo(RunOutcome::InvalidAssumption("precondition failed".into()));
    let cfg = MultiverseConfig {
        root_seed: 1,
        universes: count(1),
        check_divergence: true,
    };
    let report = run_multiverse(&cfg, &w);
    assert_eq!(report.invalid_universes(), vec![0]);
    assert!(!report.is_clean());
}

/// Typed count boundary (hardening-loop-2 GAP): zero and absurd campaign
/// sizes are typed configuration errors, not runtime aborts.
#[test]
fn universe_count_rejects_zero_and_absurd_sizes() {
    assert!(UniverseCount::try_from(0).is_err());
    assert!(UniverseCount::try_from(u64::MAX).is_err());
    assert!(UniverseCount::try_from(UniverseCount::MAX + 1).is_err());
    assert_eq!(
        UniverseCount::try_from(UniverseCount::MAX).unwrap().get(),
        UniverseCount::MAX
    );
    assert_eq!(UniverseCount::try_from(1).unwrap().get(), 1);
}

#[test]
fn different_seeds_produce_different_multiverses() {
    let w = DeterministicDemo;
    let a = run_universe(1, 0, &w);
    let b = run_universe(2, 0, &w);
    assert_ne!(a.trace_hash(), b.trace_hash());
}

/// Flips its always-verdict between replays WITHOUT recording the leaked
/// value into the trace: trace hashes stay identical, only the property
/// verdict changes. Regression for the PR #1 review BLOCKER — the detector
/// must compare full observable results, not trace hashes alone.
static VERDICT_FLIPPER: AtomicU64 = AtomicU64::new(0);

struct VerdictFlipDemo;

impl Workload for VerdictFlipDemo {
    fn name(&self) -> &str {
        "verdict-flip-demo"
    }

    fn run(&self, ctx: &mut UniverseCtx) -> RunOutcome {
        let n = VERDICT_FLIPPER.fetch_add(1, Ordering::SeqCst);
        ctx.record("op", "constant"); // identical trace on every replay
        ctx.always("stable_verdict", n.is_multiple_of(2), || {
            format!("flip #{n}")
        });
        RunOutcome::Completed
    }
}

#[test]
fn detector_flags_verdict_flips_with_identical_traces() {
    let w = VerdictFlipDemo;
    let cfg = MultiverseConfig {
        root_seed: 42,
        universes: count(3),
        check_divergence: true,
    };
    let report = run_multiverse(&cfg, &w);
    // Each universe's replay pair sees (even, odd) counter values, so the
    // verdict flips within every pair while trace hashes stay equal.
    assert_eq!(
        report.divergent_universes().len(),
        3,
        "verdict flips with identical traces must be flagged divergent"
    );
}

#[test]
fn detector_flags_nondeterminism_instead_of_blessing_it() {
    let w = NondeterministicDemo;
    let cfg = MultiverseConfig {
        root_seed: 42,
        universes: count(5),
        check_divergence: true,
    };
    let report = run_multiverse(&cfg, &w);
    assert_eq!(
        report.divergent_universes().len(),
        5,
        "every universe of the leaky workload must be flagged divergent"
    );
    assert!(!report.is_clean());
}

/// Skips a PASSING invariant on every second replay while keeping the trace
/// identical. The assertion transcript must make this divergent — a passing
/// check that stops being evaluated is a change in observable behavior
/// (PR #1 hardening-loop BLOCKER).
static CHECK_SKIPPER: AtomicU64 = AtomicU64::new(0);

struct SkippedCheckDemo;

impl Workload for SkippedCheckDemo {
    fn name(&self) -> &str {
        "skipped-check-demo"
    }

    fn run(&self, ctx: &mut UniverseCtx) -> RunOutcome {
        let n = CHECK_SKIPPER.fetch_add(1, Ordering::SeqCst);
        ctx.record("op", "constant"); // identical trace on every replay
        if n.is_multiple_of(2) {
            ctx.always("present_sometimes", true, || unreachable!());
        }
        RunOutcome::Completed
    }
}

#[test]
fn detector_flags_skipped_passing_invariants() {
    let w = SkippedCheckDemo;
    let cfg = MultiverseConfig {
        root_seed: 42,
        universes: count(3),
        check_divergence: true,
    };
    let report = run_multiverse(&cfg, &w);
    assert_eq!(
        report.divergent_universes().len(),
        3,
        "skipping a passing invariant with an identical trace must be flagged"
    );
}

/// Reorders two PASSING checks between replays while trace hash AND event
/// count AND the set of executed checks stay identical — only the ORDER of
/// the passing-check transcript differs. Tier-1 identity includes the
/// ordered transcript (hardening-loop-3 GAP; docs/specs/TRACE_FORMAT_V0.md
/// § Observable identity), so this must be divergent.
static ORDER_FLIPPER: AtomicU64 = AtomicU64::new(0);

struct ReorderedChecksDemo;

impl Workload for ReorderedChecksDemo {
    fn name(&self) -> &str {
        "reordered-checks-demo"
    }

    fn run(&self, ctx: &mut UniverseCtx) -> RunOutcome {
        let n = ORDER_FLIPPER.fetch_add(1, Ordering::SeqCst);
        ctx.record("op", "constant"); // identical trace on every replay
        if n.is_multiple_of(2) {
            ctx.always("p", true, || unreachable!());
            ctx.always("q", true, || unreachable!());
        } else {
            ctx.always("q", true, || unreachable!());
            ctx.always("p", true, || unreachable!());
        }
        RunOutcome::Completed
    }
}

/// Hardening-loop-4 GAP 5, reproduced: a no-op workload returning
/// `Completed` with no properties used to reach CLEAN through an empty
/// finding ledger. With the runner-owned property contract, an EMPTY
/// contract campaign is UNCHECKED — never CLEAN.
struct NoOpDemo;

impl Workload for NoOpDemo {
    fn name(&self) -> &str {
        "no-op-demo"
    }

    fn run(&self, ctx: &mut UniverseCtx) -> RunOutcome {
        ctx.record("op", "nothing asserted");
        RunOutcome::Completed
    }
}

#[test]
fn no_op_completed_workload_with_no_contract_is_never_clean() {
    let w = NoOpDemo;
    let cfg = MultiverseConfig {
        root_seed: 1,
        universes: count(3),
        check_divergence: true,
    };
    let report = run_multiverse(&cfg, &w);
    assert!(report.divergent_universes().is_empty());
    assert!(report.contract().is_empty());
    assert_eq!(
        report.verdict(),
        Verdict::Unchecked,
        "an empty property contract asserted nothing and must be UNCHECKED, never CLEAN"
    );
    assert!(!report.is_clean());
}

/// A workload that DECLARES a contract and then fails to honor it: the
/// runner, not the workload, detects the breach per universe.
struct ContractBreakingDemo;

impl Workload for ContractBreakingDemo {
    fn name(&self) -> &str {
        "contract-breaking-demo"
    }

    fn run(&self, ctx: &mut UniverseCtx) -> RunOutcome {
        ctx.record("op", "contract never honored");
        RunOutcome::Completed
    }

    fn property_contract(&self) -> PropertyContract {
        PropertyContract::new(&["durability"], &["crash_seen"])
    }
}

#[test]
fn unmet_property_contract_is_a_finding() {
    let w = ContractBreakingDemo;
    let cfg = MultiverseConfig {
        root_seed: 1,
        universes: count(2),
        check_divergence: true,
    };
    let report = run_multiverse(&cfg, &w);
    assert_eq!(
        report.contract_violations().len(),
        4,
        "both universes must report both unmet requirements: {:?}",
        report.contract_violations()
    );
    assert_eq!(report.verdict(), Verdict::Findings);
    assert!(!report.is_clean());
}

/// The fault-plan digest binds the replay input's identity into the
/// observable result (hardening-loop-4 GAP 5): different plans yield
/// different digests, identical plans yield identical digests, and a
/// workload that never retrieves a plan carries none.
#[test]
fn fault_plan_digest_binds_replay_input_identity() {
    let w = DeterministicDemo;
    let empty = run_universe_with_fault_plan(0xD1CE, 3, &w, FaultPlan::default());
    let mut gremlin = vh_core::SeedTree::new(0xD1CE).stream(3, "gremlin");
    let generated = FaultPlan::generate(&mut gremlin, 1_000_000, 4);
    let full = run_universe_with_fault_plan(0xD1CE, 3, &w, generated.clone());
    let full_again = run_universe_with_fault_plan(0xD1CE, 3, &w, generated);
    assert!(empty.fault_plan_digest().is_some());
    assert_ne!(
        empty.fault_plan_digest(),
        full.fault_plan_digest(),
        "different replay inputs must carry different digests"
    );
    assert_eq!(full.fault_plan_digest(), full_again.fault_plan_digest());

    // The baseline (self-generated) run retrieved the same plan content,
    // so its digest matches the override replay of that plan.
    let baseline = run_universe(0xD1CE, 3, &w);
    assert_eq!(baseline.fault_plan_digest(), full.fault_plan_digest());

    // No retrieval → no digest.
    let none = run_universe(1, 0, &IgnoresPlanDemo);
    assert_eq!(none.fault_plan_digest(), None);
}

/// Hardening-loop-4 BLOCKER 2, reproduced: `GLOBAL.fetch_add(1) / 2`
/// yields the pair-local values A,A then B,B, so ADJACENT pairing
/// reported systematic nondeterminism as divergence-free. Non-adjacent
/// two-pass pairing separates the executions and must flag every
/// universe.
static PAIR_LOCAL_COUNTER: AtomicU64 = AtomicU64::new(0);

struct PairLocalCounterDemo;

impl Workload for PairLocalCounterDemo {
    fn name(&self) -> &str {
        "pair-local-counter-demo"
    }

    fn run(&self, ctx: &mut UniverseCtx) -> RunOutcome {
        let v = PAIR_LOCAL_COUNTER.fetch_add(1, Ordering::SeqCst) / 2;
        ctx.record("leak", &format!("half={v}"));
        RunOutcome::Completed
    }
}

#[test]
fn pair_local_counter_nondeterminism_is_caught_by_nonadjacent_replay() {
    let w = PairLocalCounterDemo;
    let cfg = MultiverseConfig {
        root_seed: 42,
        universes: count(4),
        check_divergence: true,
    };
    let report = run_multiverse(&cfg, &w);
    assert_eq!(
        report.divergent_universes().len(),
        4,
        "the adjacent-pair-blessed counter workload must be flagged in every universe"
    );
    assert!(!report.is_clean());
}

/// The honest limit that keeps the evidence named "agreement", never
/// "proof" (hardening-loop-4 BLOCKER 2): a workload keyed to the full
/// execution schedule — counter modulo the per-pass invocation count —
/// produces identical observations in both passes and is blessed by ANY
/// finite fixed-schedule replay sample. This regression pins the
/// limitation so the naming cannot quietly re-inflate; refuting this
/// class needs controlled-effect closure (the D0 boundary), not more
/// samples.
static SCHEDULE_KEYED_COUNTER: AtomicU64 = AtomicU64::new(0);

struct ScheduleKeyedDemo;

impl Workload for ScheduleKeyedDemo {
    fn name(&self) -> &str {
        "schedule-keyed-demo"
    }

    fn run(&self, ctx: &mut UniverseCtx) -> RunOutcome {
        // 4 universes per pass: invocation i and i+4 collapse to the same
        // recorded value, so the two passes agree observation-for-
        // observation despite the process-global state.
        let v = SCHEDULE_KEYED_COUNTER.fetch_add(1, Ordering::SeqCst) % 4;
        ctx.record("leak", &format!("keyed={v}"));
        RunOutcome::Completed
    }
}

#[test]
fn schedule_keyed_nondeterminism_still_evades_sampled_replay_agreement() {
    let w = ScheduleKeyedDemo;
    let cfg = MultiverseConfig {
        root_seed: 42,
        universes: count(4),
        check_divergence: true,
    };
    let report = run_multiverse(&cfg, &w);
    assert!(
        report.divergent_universes().is_empty(),
        "this workload is CONSTRUCTED to evade the sampled falsifier; if it \
         is now caught, the pairing schedule changed — update the \
         construction AND re-verify the ReplayEvidence naming stays honest"
    );
}

#[test]
fn detector_flags_reordered_passing_check_transcripts() {
    let w = ReorderedChecksDemo;
    let cfg = MultiverseConfig {
        root_seed: 42,
        universes: count(3),
        check_divergence: true,
    };
    let report = run_multiverse(&cfg, &w);
    // Same trace hash, same event count, same executed checks — different
    // transcript ORDER within every replay pair.
    assert_eq!(
        report.divergent_universes().len(),
        3,
        "reordered passing-check transcripts with identical traces must be flagged"
    );
}

/// The observation view is the compile-time schema ratchet (PR #2
/// interface request 5021566209): it must agree with the getter surface
/// field for field, and because both the kernel implementation and this
/// destructuring use no `..`, a new result field cannot ship without
/// extending the view and this test.
#[test]
fn observation_view_matches_the_getter_surface_exhaustively() {
    let w = DeterministicDemo;
    let r = run_universe(0xD1CE, 3, &w);
    let vh_multiverse::UniverseObservation {
        universe_id,
        trace_hash,
        trace_events,
        always_checks,
        always_failures,
        sometimes,
        lifecycle,
        fault_plan_digest,
    } = r.observation();
    assert_eq!(universe_id, r.universe_id());
    assert_eq!(trace_hash, r.trace_hash());
    assert_eq!(trace_events, r.trace_events());
    assert_eq!(always_checks, r.always_checks());
    assert_eq!(always_failures, r.always_failures());
    assert_eq!(sometimes, r.sometimes());
    assert_eq!(lifecycle, r.lifecycle());
    assert_eq!(fault_plan_digest, r.fault_plan_digest());
}
