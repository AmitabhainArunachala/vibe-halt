//! C2a — corpus oracle fail-closed facts (Codex audit B.1 oracle-semantics
//! half, B.2, B.3, criterion-3 evidence integrity).
//!
//! Two things this file proves that unit tests inside `workloads::corpus`
//! and `workloads::disk` cannot: (1) the fault-free/crash-free control
//! doctrine (PLAYBOOK.md "Anti-gaming rules") still holds for every
//! oracle this package fixed, on the workload's OWN real fault-plan
//! generation, not a hand-built `EndState`; (2) the fixed recall counts
//! are exactly reproducible across repeated runs (the C2a kill/stop
//! clause: a nondeterministic count is UNCHECKED, not a target defect to
//! paper over).
//!
//! This file does not pin corpus.md numbers or touch `scripts/gate.sh` —
//! recall pinning is `vibe-bug-corpus-2026-07` (K1) and gate integration
//! is C2b; both are out of this package's scope. The counts asserted here
//! are a regression guard on THIS package's oracle logic.

use vh_cli::workloads::by_name;
use vh_multiverse::{run_multiverse, MultiverseConfig, UniverseCount};

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
/// fail-closed fixes: proving the required-progress and independent-fact
/// checks reject SILENCE, not FAULT-FREE EXECUTION — the two must remain
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
        "{workload_name}: fault-free universe(s) {violating:?} FAILED — the required-progress \
         fix must reject silence, not legitimate no-fault execution (vacuous-failure doctrine)"
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
///   corpus-dirty-read              83 -> 96  (+13: required-progress now
///                                   catches universes where every op timer
///                                   was consumed by crashes before the
///                                   first publish point — previously a
///                                   silent vacuous pass)
///   corpus-crash-toctou            21 -> 38  (+17: required-progress now
///                                   catches universes where the volatile
///                                   session token never survived to a
///                                   check — previously a silent vacuous
///                                   pass)
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
        ("corpus-dirty-read", 100, 96),
        ("corpus-crash-toctou", 100, 38),
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
