//! The divergence detector is CI gate #1. These tests prove both directions:
//! a deterministic workload replays bit-identically, and a workload with
//! smuggled-in nondeterminism is caught, not silently blessed.

use std::sync::atomic::{AtomicU64, Ordering};

use vh_gremlin::FaultPlan;
use vh_multiverse::{run_multiverse, run_universe, MultiverseConfig, UniverseCtx, Workload};

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
        let plan = FaultPlan::generate(&mut gremlin, 1_000_000, 4);
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

#[test]
fn different_seeds_produce_different_multiverses() {
    let w = DeterministicDemo;
    let a = run_universe(1, 0, &w);
    let b = run_universe(2, 0, &w);
    assert_ne!(a.trace_hash, b.trace_hash);
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
