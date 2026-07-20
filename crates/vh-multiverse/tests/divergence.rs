//! The divergence detector is CI gate #1. These tests prove both directions:
//! a deterministic workload replays bit-identically, and a workload with
//! smuggled-in nondeterminism is caught, not silently blessed.

use std::sync::atomic::{AtomicU64, Ordering};

use vh_gremlin::FaultPlan;
use vh_multiverse::{
    run_multiverse, run_universe, run_universe_with_fault_plan, MultiverseConfig, UniverseCtx,
    Workload,
};

/// A small deterministic workload: draws ops and a fault plan from named
/// streams, records everything.
struct DeterministicDemo;

impl Workload for DeterministicDemo {
    fn name(&self) -> &str {
        "deterministic-demo"
    }

    fn run(&self, ctx: &mut UniverseCtx) {
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

    fn run(&self, ctx: &mut UniverseCtx) {
        let leaked = LEAKY_COUNTER.fetch_add(1, Ordering::SeqCst);
        ctx.record("leak", &format!("counter={leaked}"));
    }
}

#[test]
fn same_universe_replays_bit_identically() {
    let w = DeterministicDemo;
    for universe_id in 0..10 {
        let a = run_universe(0xD1CE, universe_id, &w);
        let b = run_universe(0xD1CE, universe_id, &w);
        assert_eq!(
            a.trace_hash, b.trace_hash,
            "universe {universe_id} diverged"
        );
        assert!(!a.trace_hash.is_empty());
    }
}

#[test]
fn multiverse_replays_across_many_runs() {
    let w = DeterministicDemo;
    let cfg = MultiverseConfig {
        root_seed: 42,
        universes: 25,
        check_divergence: true,
    };
    let report = run_multiverse(&cfg, &w);
    assert!(
        report.divergent_universes.is_empty(),
        "divergent universes: {:?}",
        report.divergent_universes
    );

    // The whole-report fingerprint must also be stable across invocations.
    let hashes: Vec<String> = report
        .results
        .iter()
        .map(|r| r.trace_hash.clone())
        .collect();
    let report2 = run_multiverse(&cfg, &w);
    let hashes2: Vec<String> = report2
        .results
        .iter()
        .map(|r| r.trace_hash.clone())
        .collect();
    assert_eq!(hashes, hashes2);
}

/// The shrinker's oracle surface: an override plan replaces the
/// workload-generated one through the identical code path, deterministically.
#[test]
fn fault_plan_override_replays_deterministically() {
    let w = DeterministicDemo;
    let baseline = run_universe(0xD1CE, 3, &w);

    // Empty plan: no faults fire; the run must differ from the baseline
    // (whose generated plan injects 4 faults) but replay identically.
    let empty = FaultPlan::default();
    let a = run_universe_with_fault_plan(0xD1CE, 3, &w, empty.clone());
    let b = run_universe_with_fault_plan(0xD1CE, 3, &w, empty);
    assert_eq!(a, b, "override replay must be bit-identical");
    assert_ne!(
        a.trace_hash, baseline.trace_hash,
        "empty override must actually suppress the generated faults"
    );

    // Overriding with the plan the workload would have generated anyway
    // must reproduce the baseline exactly — proving override and generated
    // paths are the same path.
    let mut gremlin = vh_core::SeedTree::new(0xD1CE).stream(3, "gremlin");
    let generated = FaultPlan::generate(&mut gremlin, 1_000_000, 4);
    let c = run_universe_with_fault_plan(0xD1CE, 3, &w, generated);
    assert_eq!(c, baseline);
}

#[test]
fn different_seeds_produce_different_multiverses() {
    let w = DeterministicDemo;
    let a = run_universe(1, 0, &w);
    let b = run_universe(2, 0, &w);
    assert_ne!(a.trace_hash, b.trace_hash);
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

    fn run(&self, ctx: &mut UniverseCtx) {
        let n = VERDICT_FLIPPER.fetch_add(1, Ordering::SeqCst);
        ctx.record("op", "constant"); // identical trace on every replay
        ctx.props
            .always("stable_verdict", n % 2 == 0, || format!("flip #{n}"));
    }
}

#[test]
fn detector_flags_verdict_flips_with_identical_traces() {
    let w = VerdictFlipDemo;
    let cfg = MultiverseConfig {
        root_seed: 42,
        universes: 3,
        check_divergence: true,
    };
    let report = run_multiverse(&cfg, &w);
    // Each universe's replay pair sees (even, odd) counter values, so the
    // verdict flips within every pair while trace hashes stay equal.
    assert_eq!(
        report.divergent_universes.len(),
        3,
        "verdict flips with identical traces must be flagged divergent"
    );
}

#[test]
fn detector_flags_nondeterminism_instead_of_blessing_it() {
    let w = NondeterministicDemo;
    let cfg = MultiverseConfig {
        root_seed: 42,
        universes: 5,
        check_divergence: true,
    };
    let report = run_multiverse(&cfg, &w);
    assert_eq!(
        report.divergent_universes.len(),
        5,
        "every universe of the leaky workload must be flagged divergent"
    );
    assert!(!report.is_clean());
}
