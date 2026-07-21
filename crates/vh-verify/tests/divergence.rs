#![forbid(unsafe_code)]

use std::cell::Cell;

use vh_multiverse::{
    run_multiverse, run_universe, MultiverseConfig, PropertyContract, RunOutcome, UniverseCount,
    UniverseCtx, UniverseResult, Workload,
};

struct PropertyOnlyNondeterminism {
    calls: Cell<u64>,
}

impl Workload for PropertyOnlyNondeterminism {
    fn name(&self) -> &str {
        "property-only-nondeterminism"
    }

    fn property_contract(&self) -> PropertyContract {
        PropertyContract::new(&["counter_is_even"], &[])
    }

    fn run(&self, ctx: &mut UniverseCtx) -> RunOutcome {
        let leaked = self.calls.get();
        self.calls.set(leaked + 1);
        ctx.always("counter_is_even", leaked.is_multiple_of(2), || {
            format!("counter={leaked}")
        });
        RunOutcome::Completed
    }
}

struct SkippedPassingInvariant {
    calls: Cell<u64>,
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

    fn property_contract(&self) -> PropertyContract {
        PropertyContract::new(&["passing_check_was_invoked"], &[])
    }

    fn run(&self, ctx: &mut UniverseCtx) -> RunOutcome {
        let invocation = self.calls.get();
        self.calls.set(invocation + 1);
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
fn property_only_nondeterminism_must_be_reported_across_non_adjacent_passes() {
    let preflight = PropertyOnlyNondeterminism {
        calls: Cell::new(0),
    };
    let first = run_universe(1, 0, &preflight);
    let second = run_universe(1, 0, &preflight);
    assert_eq!(preflight.calls.get(), 2);
    assert_property_only_drift(&first, &second);

    let workload = PropertyOnlyNondeterminism {
        calls: Cell::new(0),
    };
    let report = run_multiverse(
        &MultiverseConfig {
            root_seed: 1,
            universes: UniverseCount::try_from(2).expect("two universes are valid and bounded"),
            check_divergence: true,
        },
        &workload,
    );

    // Pass 1 runs universes 0 and 1; pass 2 then replays 0 and 1. The
    // non-adjacent schedule makes both pairings disagree for this fixture.
    assert_eq!(workload.calls.get(), 4);
    assert_eq!(report.divergent_universes(), &[0, 1]);
    assert!(!report.is_clean());
}

#[test]
fn skipping_a_passing_invariant_must_be_reported() {
    let preflight = SkippedPassingInvariant {
        calls: Cell::new(0),
    };
    let first = run_universe(1, 0, &preflight);
    let second = run_universe(1, 0, &preflight);
    assert_eq!(preflight.calls.get(), 2);
    assert_property_only_drift(&first, &second);

    let workload = SkippedPassingInvariant {
        calls: Cell::new(0),
    };
    let report = run_multiverse(
        &MultiverseConfig {
            root_seed: 1,
            universes: UniverseCount::try_from(1).expect("one universe is valid and bounded"),
            check_divergence: true,
        },
        &workload,
    );

    assert_eq!(workload.calls.get(), 2);
    let first = &report.results()[0];
    assert!(first.always_failures().is_empty());
    assert_eq!(first.always_checks().len(), 1);
    assert_eq!(report.divergent_universes(), &[0]);
    assert!(!report.is_clean());
}
