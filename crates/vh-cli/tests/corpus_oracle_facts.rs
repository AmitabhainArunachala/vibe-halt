//! C2a — corpus oracle fail-closed facts (Codex audit B.1 oracle-semantics
//! half, B.2, B.3, criterion-3 evidence integrity).
//!
//! Two things this file proves that unit tests inside `workloads::corpus`
//! and `workloads::disk` cannot: (1) the fault-free/crash-free control
//! doctrine (PLAYBOOK.md "Anti-gaming rules") still holds for every
//! oracle this package fixed, on the workload's OWN real fault-plan
//! generation, not a hand-built `EndState`; (2) the manifestation and
//! typed-invalid counts are exactly reproducible across repeated runs
//! (the C2a kill/stop clause: a nondeterministic count is UNCHECKED, not
//! a target defect to paper over).
//!
//! This file does not redefine the historical `corpus.md` numbers.
//! The counts asserted here are a regression guard on THIS package's
//! oracle logic and support the current summary pins in `scripts/gate.sh`.

use std::collections::BTreeSet;

use vh_cli::workloads::by_name;
use vh_multiverse::{run_multiverse, MultiverseConfig, RunOutcome, UniverseCount, Verdict};

const SEED: u64 = 0xD1CE;

fn count(n: u64) -> UniverseCount {
    UniverseCount::try_from(n).unwrap()
}

/// The canonical zero-injection fault-plan digest: `corpus-same-timestamp-race`
/// always runs `FaultPlan::new(Vec::new())` (VB-006 is pure-schedule, no
/// faults at all), so its universe-0 digest is the fixed empty-plan
/// identity every other workload's zero-fault universes share (fault-plan
/// digests are content digests over the injection list; an empty list is
/// seed-independent).
fn empty_fault_plan_digest() -> String {
    let w = by_name("corpus-same-timestamp-race").expect("workload exists");
    let r = vh_multiverse::run_universe(SEED, 0, w.as_ref());
    r.fault_plan_digest()
        .expect("VB-006 retrieves a fault plan")
        .to_string()
}

/// Fault-free/crash-free universes must PASS (PLAYBOOK.md "Anti-gaming
/// rules": "crash-free / fault-free universes must PASS"). This is the
/// positive control the C2a acceptance criteria require alongside the
/// fail-closed fixes: proving the no-opportunity classification rejects
/// SILENCE as invalid, not FAULT-FREE EXECUTION — the two must remain
/// distinguishable.
fn assert_fault_free_universes_pass(workload_name: &str) {
    let w = by_name(workload_name).expect("workload exists");
    let empty_digest = empty_fault_plan_digest();
    let report = run_multiverse(
        &MultiverseConfig {
            root_seed: SEED,
            universes: count(100),
            check_divergence: false,
        },
        w.as_ref(),
    );
    let failing: std::collections::BTreeSet<u64> =
        report.failing_universes().iter().copied().collect();
    let fault_free: Vec<u64> = report
        .results()
        .iter()
        .enumerate()
        .filter(|(_, r)| r.fault_plan_digest() == Some(empty_digest.as_str()))
        .map(|(u, _)| u as u64)
        .collect();
    assert!(
        !fault_free.is_empty(),
        "{workload_name}: no fault-free universe found in {} universes at seed 0x{SEED:x} — \
         the control is not exercised, strengthen the sample or confirm the palette floor",
        report.results().len()
    );
    let violating: Vec<u64> = fault_free
        .iter()
        .copied()
        .filter(|u| failing.contains(u))
        .collect();
    assert!(
        violating.is_empty(),
        "{workload_name}: fault-free universe(s) {violating:?} FAILED — no-opportunity \
         classification must reject silence, not legitimate no-fault execution \
         (vacuous-failure doctrine)"
    );
}

#[test]
fn resume_replay_fault_free_universes_pass() {
    assert_fault_free_universes_pass("corpus-resume-replay");
}

#[test]
fn blind_stream_append_fault_free_universes_pass() {
    assert_fault_free_universes_pass("corpus-blind-stream-append");
}

/// Regression pin + determinism proof for every oracle this package
/// touched: exact failing-universe count at seed 0xD1CE / 100 universes,
/// measured twice in this process to rule out the kill/stop clause
/// (nondeterministic count -> UNCHECKED, not a tolerance to widen).
///
/// Before -> after deltas (measured against `origin/main` before this
/// package's changes, same seed/budget, reported honestly per the C2
/// standing law "never silently absorb a count change"):
///   corpus-lost-update             29 -> 29  (unchanged: counter/requested
///                                   are unconditionally declared; only the
///                                   adversarial malformed-state path moved)
///   corpus-dirty-read              83 manifestations + 13 typed
///                                   InvalidAssumption no-opportunity
///                                   universes + 4 clean controls
///   corpus-crash-toctou            21 manifestations + 17 typed
///                                   InvalidAssumption no-opportunity
///                                   universes + 62 clean controls
///   corpus-fsync-lie               21 -> 21  (unchanged: every measured
///                                   universe acknowledges at least one
///                                   record)
///   corpus-unvalidated-checkpoint  96 -> 96  (unchanged: the independent
///                                   fact check is mathematically
///                                   equivalent to the removed
///                                   workload-precomputed boolean on real
///                                   executions; only the adversarial
///                                   lying-workload and zero-ack paths
///                                   moved)
///   corpus-resume-replay           70 -> 70  (unchanged: every step
///                                   always applies at least once by
///                                   workload construction)
///   corpus-blind-stream-append     58 -> 58  (unchanged: assembled/expected
///                                   are unconditionally declared)
///   demo-disk (200 universes)      CLEAN -> CLEAN (unchanged)
///   demo-disk-buggy                87 -> 87  (unchanged)
#[test]
fn touched_oracle_recall_counts_are_pinned_and_deterministic() {
    let cases: &[(&str, u64, usize)] = &[
        ("corpus-lost-update", 100, 29),
        ("corpus-dirty-read", 100, 83),
        ("corpus-crash-toctou", 100, 21),
        ("corpus-fsync-lie", 100, 21),
        ("corpus-unvalidated-checkpoint", 100, 96),
        ("corpus-resume-replay", 100, 70),
        ("corpus-blind-stream-append", 100, 58),
    ];
    for &(name, universes, expected) in cases {
        let w = by_name(name).expect("workload exists");
        let cfg = MultiverseConfig {
            root_seed: SEED,
            universes: count(universes),
            check_divergence: false,
        };
        let first = run_multiverse(&cfg, w.as_ref()).failing_universes().len();
        let second = run_multiverse(&cfg, w.as_ref()).failing_universes().len();
        assert_eq!(
            first, second,
            "{name}: failing-universe count is nondeterministic across repeated runs at the \
             same seed ({first} vs {second}) — per the C2a kill/stop clause this becomes an \
             UNCHECKED claim, not a target defect"
        );
        assert_eq!(
            first, expected,
            "{name}: failing-universe count drifted from the C2a-measured pin ({expected}) to \
             {first} — a real count change must be measured, explained, and reported, never \
             silently absorbed"
        );
    }
}

/// P1 classification repair: actual oracle manifestations and
/// no-opportunity coverage findings are disjoint typed outcomes. Both
/// exact universe sets are replayed twice at the pinned seed; the
/// remaining universes are clean controls. Invalid assumptions remain a
/// fail-closed FINDINGS verdict and carry an exact stable reason.
#[test]
fn vb003_vb004_manifestation_invalid_and_clean_sets_are_exact_and_deterministic() {
    struct ClassificationCase {
        name: &'static str,
        expected_manifestations: &'static [u64],
        expected_invalid: &'static [u64],
        clean_count: usize,
        invalid_reason: &'static str,
    }

    const DR_MANIFESTATIONS: &[u64] = &[
        0, 1, 2, 3, 4, 5, 6, 8, 9, 10, 11, 12, 14, 15, 16, 17, 18, 19, 20, 21, 22, 24, 25, 26, 27,
        28, 29, 30, 31, 33, 34, 35, 37, 40, 42, 43, 44, 46, 48, 49, 51, 52, 53, 54, 55, 56, 57, 58,
        59, 60, 61, 62, 64, 65, 67, 69, 70, 71, 72, 74, 75, 76, 77, 78, 79, 80, 81, 83, 84, 85, 86,
        87, 88, 89, 91, 92, 93, 94, 95, 96, 97, 98, 99,
    ];
    const DR_INVALID: &[u64] = &[7, 13, 32, 36, 38, 41, 47, 50, 63, 66, 68, 73, 90];
    const TT_MANIFESTATIONS: &[u64] = &[
        9, 12, 16, 39, 40, 41, 46, 53, 54, 55, 58, 59, 61, 67, 77, 82, 88, 90, 91, 93, 96,
    ];
    const TT_INVALID: &[u64] = &[
        0, 2, 4, 7, 19, 29, 34, 35, 36, 44, 47, 52, 69, 70, 85, 89, 99,
    ];
    let cases = [
        ClassificationCase {
            name: "corpus-dirty-read",
            expected_manifestations: DR_MANIFESTATIONS,
            expected_invalid: DR_INVALID,
            clean_count: 4,
            invalid_reason:
                "no record was ever published; published_implies_durable had no opportunity to judge",
        },
        ClassificationCase {
            name: "corpus-crash-toctou",
            expected_manifestations: TT_MANIFESTATIONS,
            expected_invalid: TT_INVALID,
            clean_count: 62,
            invalid_reason:
                "no check-then-act action completed; act_epoch_matches_check had no opportunity to judge",
        },
    ];

    for ClassificationCase {
        name,
        expected_manifestations,
        expected_invalid,
        clean_count,
        invalid_reason,
    } in cases
    {
        let w = by_name(name).expect("workload exists");
        let cfg = MultiverseConfig {
            root_seed: SEED,
            universes: count(100),
            check_divergence: true,
        };
        let first = run_multiverse(&cfg, w.as_ref());
        let second = run_multiverse(&cfg, w.as_ref());
        let first_manifestations: BTreeSet<u64> = first.failing_universes().into_iter().collect();
        let second_manifestations: BTreeSet<u64> = second.failing_universes().into_iter().collect();
        let first_invalid: BTreeSet<u64> = first.invalid_universes().into_iter().collect();
        let second_invalid: BTreeSet<u64> = second.invalid_universes().into_iter().collect();
        let expected_manifestations: BTreeSet<u64> =
            expected_manifestations.iter().copied().collect();
        let expected_invalid: BTreeSet<u64> = expected_invalid.iter().copied().collect();

        assert_eq!(
            first_manifestations, second_manifestations,
            "{name}: manifestation universe set changed across exact replays"
        );
        assert_eq!(
            first_invalid, second_invalid,
            "{name}: invalid-assumption universe set changed across exact replays"
        );
        assert_eq!(
            first_manifestations, expected_manifestations,
            "{name}: manifestation universe set drifted"
        );
        assert_eq!(
            first_invalid, expected_invalid,
            "{name}: invalid-assumption universe set drifted"
        );
        assert!(
            first_manifestations.is_disjoint(&first_invalid),
            "{name}: one universe cannot be both a bug manifestation and no-opportunity coverage finding"
        );

        let clean: BTreeSet<u64> = (0..100)
            .filter(|u| !first_manifestations.contains(u) && !first_invalid.contains(u))
            .collect();
        assert_eq!(clean.len(), clean_count, "{name}");
        assert_eq!(
            first_manifestations.len() + first_invalid.len() + clean_count,
            100,
            "{name}: split must account for every universe exactly once"
        );
        assert_eq!(
            first.verdict(),
            Verdict::Findings,
            "{name}: typed invalid assumptions must remain fail-closed FINDINGS"
        );
        assert!(first.divergent_universes().is_empty(), "{name}");
        assert!(first.contract_violations().is_empty(), "{name}");
        for &u in &first_invalid {
            assert_eq!(
                first.results()[u as usize].lifecycle().outcome(),
                &RunOutcome::InvalidAssumption(invalid_reason.to_string()),
                "{name}: invalid-assumption reason drifted in universe {u}"
            );
            assert!(
                first.results()[u as usize].always_failures().is_empty(),
                "{name}: invalid universe {u} must not also inflate oracle manifestation recall"
            );
        }
    }
}

/// Unaffected oracles (already fail-closed per the audit's B.2 table)
/// stay byte-identical controls: this package must not have touched their
/// logic or shifted their recall.
#[test]
fn untouched_oracle_recall_counts_are_unchanged() {
    let cases: &[(&str, usize)] = &[
        ("corpus-retry-double-apply", 76),
        ("corpus-stale-redispatch", 91),
        ("corpus-transient-fatal-abort", 79),
    ];
    for &(name, expected) in cases {
        let w = by_name(name).expect("workload exists");
        let report = run_multiverse(
            &MultiverseConfig {
                root_seed: SEED,
                universes: count(100),
                check_divergence: false,
            },
            w.as_ref(),
        );
        assert_eq!(
            report.failing_universes().len(),
            expected,
            "{name}: recall drifted even though this package did not touch its oracle"
        );
    }
}
