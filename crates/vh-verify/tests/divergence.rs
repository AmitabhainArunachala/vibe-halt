#![forbid(unsafe_code)]

use std::sync::atomic::{AtomicU64, Ordering};

use vh_multiverse::{
    run_multiverse, run_universe, MultiverseConfig, RunOutcome, UniverseCount, UniverseCtx,
    UniverseResult, Workload,
};

struct PropertyOnlyNondeterminism {
    calls: AtomicU64,
}

impl Workload for PropertyOnlyNondeterminism {
    fn name(&self) -> &str {
        "property-only-nondeterminism"
    }

    fn run(&self, ctx: &mut UniverseCtx) -> RunOutcome {
        let leaked = self.calls.fetch_add(1, Ordering::SeqCst);
        ctx.always("counter_is_even", leaked.is_multiple_of(2), || {
            format!("counter={leaked}")
        });
        RunOutcome::Completed
    }
}

struct SkippedPassingInvariant {
    calls: AtomicU64,
}

fn assert_property_only_drift(first: &UniverseResult, second: &UniverseResult) {
    assert_eq!(first.universe_id(), second.universe_id());
    assert_eq!(first.trace_hash(), second.trace_hash());
    assert_eq!(first.trace_events(), second.trace_events());
    assert!(
        first.always_checks() != second.always_checks()
            || first.always_failures() != second.always_failures()
            || first.sometimes() != second.sometimes(),
        "the fixture must drift only in the assertion transcript"
    );
}

impl Workload for SkippedPassingInvariant {
    fn name(&self) -> &str {
        "skipped-passing-invariant"
    }

    fn run(&self, ctx: &mut UniverseCtx) -> RunOutcome {
        let invocation = self.calls.fetch_add(1, Ordering::SeqCst);
        ctx.record("identical", "trace");
        if invocation.is_multiple_of(2) {
            ctx.always("passing_check_was_invoked", true, || {
                unreachable!("passing invariant detail must remain lazy")
            });
        }
        RunOutcome::Completed
    }
}

#[test]
fn property_only_nondeterminism_must_be_reported() {
    let preflight = PropertyOnlyNondeterminism {
        calls: AtomicU64::new(0),
    };
    let first = run_universe(1, 0, &preflight);
    let second = run_universe(1, 0, &preflight);
    assert_eq!(preflight.calls.load(Ordering::SeqCst), 2);
    assert_property_only_drift(&first, &second);

    let workload = PropertyOnlyNondeterminism {
        calls: AtomicU64::new(0),
    };
    let report = run_multiverse(
        &MultiverseConfig {
            root_seed: 1,
            universes: UniverseCount::try_from(1).expect("one universe is valid and bounded"),
            check_divergence: true,
        },
        &workload,
    );

    assert_eq!(workload.calls.load(Ordering::SeqCst), 2);
    assert_eq!(report.divergent_universes(), &[0]);
    assert!(!report.is_clean());
}

#[test]
fn skipping_a_passing_invariant_must_be_reported() {
    let preflight = SkippedPassingInvariant {
        calls: AtomicU64::new(0),
    };
    let first = run_universe(1, 0, &preflight);
    let second = run_universe(1, 0, &preflight);
    assert_eq!(preflight.calls.load(Ordering::SeqCst), 2);
    assert_property_only_drift(&first, &second);

    let workload = SkippedPassingInvariant {
        calls: AtomicU64::new(0),
    };
    let report = run_multiverse(
        &MultiverseConfig {
            root_seed: 1,
            universes: UniverseCount::try_from(1).expect("one universe is valid and bounded"),
            check_divergence: true,
        },
        &workload,
    );

    assert_eq!(workload.calls.load(Ordering::SeqCst), 2);
    let first = &report.results()[0];
    assert!(first.always_failures().is_empty());
    assert_eq!(first.always_checks().len(), 1);
    assert_eq!(report.divergent_universes(), &[0]);
    assert!(!report.is_clean());
}
