//! End-to-end demo contract: the correct workload is clean, the seeded
//! durability bug is FOUND with a reproducible failing universe, and the
//! finding replays bit-identically.

use vh_cli::workloads::by_name;
use vh_multiverse::{run_multiverse, run_universe, MultiverseConfig, UniverseCount};

const SEED: u64 = 0xD1CE;

fn count(n: u64) -> UniverseCount {
    UniverseCount::try_from(n).unwrap()
}

/// The explicit pre-landing verification the night plan demands for the
/// oracle re-expression: oracles read state and record NO trace events,
/// so the frozen demo trace identity must be byte-identical to the
/// pre-oracle recording. If this moves, the re-expression touched the
/// trace and must not land.
#[test]
fn demo_trace_identity_survives_the_oracle_reexpression() {
    let w = by_name("demo").unwrap();
    let r = run_universe(SEED, 0, w.as_ref());
    assert_eq!(r.trace_hash(), "9ce6199f133f4d3c9dd0da0075e352d2");
    assert_eq!(r.trace_events(), 45);
    // The transcript now carries exactly one runner-judged oracle entry.
    assert_eq!(r.always_checks().len(), 1);
    assert_eq!(r.always_checks()[0].name, "oracle:durability");
    assert!(r.always_checks()[0].passed);
}

#[test]
fn correct_demo_is_clean() {
    let w = by_name("demo").unwrap();
    let report = run_multiverse(
        &MultiverseConfig {
            root_seed: SEED,
            universes: count(100),
            check_divergence: true,
        },
        w.as_ref(),
    );
    assert!(
        report.failing_universes().is_empty(),
        "correct workload must not violate durability: {:?}",
        report.failing_universes()
    );
    assert!(report.divergent_universes().is_empty());
    assert!(report.invalid_universes().is_empty());
    // The fault space must actually be exercised, or the pass is vacuous.
    assert!(
        report.merged().sometimes["crash_injected"],
        "no universe ever crashed — gremlins are not firing"
    );
    assert!(report.merged().sometimes["crash_with_dirty_wal"]);
    assert!(report.is_clean());
}

#[test]
fn buggy_demo_is_caught_with_reproducible_universe() {
    let w = by_name("demo-buggy").unwrap();
    let report = run_multiverse(
        &MultiverseConfig {
            root_seed: SEED,
            universes: count(100),
            check_divergence: true,
        },
        w.as_ref(),
    );
    let failing = report.failing_universes();
    assert!(
        !failing.is_empty(),
        "the ack-before-flush bug must be found within 100 universes"
    );
    assert!(!report.is_clean());

    // The repro contract: re-running a failing universe alone reproduces
    // the same trace hash and the same failure.
    let victim = failing[0];
    let solo = run_universe(SEED, victim, w.as_ref());
    let original = &report.results()[victim as usize];
    assert_eq!(solo.trace_hash(), original.trace_hash());
    assert_eq!(solo.always_failures(), original.always_failures());
    assert!(solo
        .always_failures()
        .iter()
        .all(|f| f.name == "oracle:durability"));

    // The demo's claim is that CRASHES expose the bug: after the final
    // clean flush, every failing universe must have crashed with a dirty
    // WAL. A failure without that would mean the oracle fires on
    // crash-free runs again (PR #1 review GAP regression).
    for &u in &failing {
        let r = &report.results()[u as usize];
        assert!(
            r.sometimes()["crash_with_dirty_wal"],
            "universe {u} failed durability without a dirty-WAL crash"
        );
    }
}
