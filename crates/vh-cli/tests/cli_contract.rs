//! CLI exit-truthfulness contract (PR #1 hardening-loop-2 BLOCKER).
//!
//! These tests spawn the real `vh` binary and pin exact exit codes plus
//! machine-readable verdict lines, so the process contract the gates rely
//! on is frozen in the test suite, not only in Makefile/CI shell. This
//! file is a declared scanner boundary file: it spawns processes.

use std::process::Command;

fn vh(args: &[&str]) -> (i32, String, String) {
    let out = Command::new(env!("CARGO_BIN_EXE_vh"))
        .args(args)
        .output()
        .expect("spawn vh");
    (
        out.status.code().expect("exit code"),
        String::from_utf8_lossy(&out.stdout).into_owned(),
        String::from_utf8_lossy(&out.stderr).into_owned(),
    )
}

/// Pre-repair: `--universe` exited 0 on a finding-free single execution,
/// blessing an unchecked run as success (reproduced with demo-nondet,
/// whose nondeterminism a single execution cannot see).
#[test]
fn single_universe_replay_is_unchecked_exit_3() {
    let (code, stdout, _) = vh(&["run", "--workload", "demo-nondet", "--universe", "0"]);
    assert_eq!(
        code, 3,
        "finding-free single replay must exit 3, not 0:\n{stdout}"
    );
    assert!(
        stdout.contains("replay verdict: UNCHECKED"),
        "missing machine-readable UNCHECKED verdict:\n{stdout}"
    );
}

/// A failing single replay still reports findings with exit 1.
#[test]
fn single_universe_replay_with_findings_exits_1() {
    // Find a failing universe programmatically (same contract as demo.rs).
    let w = vh_cli::workloads::by_name("demo-buggy").unwrap();
    let report = vh_multiverse::run_multiverse(
        &vh_multiverse::MultiverseConfig {
            root_seed: 0xD1CE,
            universes: vh_multiverse::UniverseCount::try_from(100).unwrap(),
            check_divergence: false,
        },
        w.as_ref(),
    );
    let victim = report.failing_universes()[0].to_string();
    let (code, stdout, _) = vh(&[
        "run",
        "--workload",
        "demo-buggy",
        "--seed",
        "0xD1CE",
        "--universe",
        &victim,
    ]);
    assert_eq!(code, 1, "failing replay must exit 1:\n{stdout}");
    assert!(stdout.contains("replay verdict: FINDINGS"), "{stdout}");
    assert!(stdout.contains("ALWAYS-FAIL durability"), "{stdout}");
}

/// Pre-repair: `--universes 0 --universe 0` exited 0 because the single-
/// universe path ran before campaign-size validation. Conflicting modes
/// are now rejected outright.
#[test]
fn conflicting_universe_flags_are_rejected() {
    let (code, _, stderr) = vh(&[
        "run",
        "--workload",
        "demo",
        "--universes",
        "0",
        "--universe",
        "0",
    ]);
    assert_eq!(
        code, 2,
        "conflicting flags must be a usage error:\n{stderr}"
    );
    assert!(
        stderr.contains("--universes conflicts with --universe"),
        "{stderr}"
    );

    let (code, _, _) = vh(&[
        "run",
        "--workload",
        "demo",
        "--universes",
        "5",
        "--universe",
        "0",
    ]);
    assert_eq!(
        code, 2,
        "nonzero --universes with --universe must also be rejected"
    );
}

#[test]
fn zero_universes_rejected_with_typed_diagnostic() {
    let (code, _, stderr) = vh(&["run", "--workload", "demo", "--universes", "0"]);
    assert_eq!(code, 2);
    assert!(
        stderr.contains("--universes must be nonzero — zero work is never certified"),
        "{stderr}"
    );
}

/// Pre-repair: u64::MAX universes aborted with exit 101 through
/// `Vec::with_capacity` (hardening-loop-2 GAP). Now a typed rejection.
#[test]
fn absurd_universe_count_rejected_with_typed_diagnostic() {
    let (code, _, stderr) = vh(&[
        "run",
        "--workload",
        "demo",
        "--universes",
        "18446744073709551615",
    ]);
    assert_eq!(
        code, 2,
        "resource-bound rejection must be exit 2, not a 101 abort"
    );
    assert!(stderr.contains("exceeds the v0 resource bound"), "{stderr}");
}

#[test]
fn no_divergence_check_is_unchecked_exit_3() {
    // 100 universes so the crash sometimes-properties are reached and the
    // run is genuinely finding-free — leaving UNCHECKED as the only
    // truthful verdict.
    let (code, stdout, _) = vh(&[
        "run",
        "--workload",
        "demo",
        "--seed",
        "0xD1CE",
        "--universes",
        "100",
        "--no-divergence-check",
    ]);
    assert_eq!(code, 3, "{stdout}");
    assert!(stdout.contains("verdict: UNCHECKED"), "{stdout}");
    assert!(
        stdout.contains("single execution (no replay agreement — divergence check disabled)"),
        "the evidence line must state that no replay agreement was sampled:\n{stdout}"
    );
}

/// The clean campaign path stays exit 0 with the checked-tier evidence line.
#[test]
fn clean_campaign_exits_0_with_checked_evidence() {
    let (code, stdout, _) = vh(&[
        "run",
        "--workload",
        "demo",
        "--seed",
        "0xD1CE",
        "--universes",
        "5",
    ]);
    assert_eq!(code, 0, "{stdout}");
    assert!(stdout.contains("verdict: CLEAN"), "{stdout}");
    assert!(
        stdout.contains("pairwise replay agreement (sampled falsifier"),
        "the evidence line must name the sampled falsifier, not a tier proof:\n{stdout}"
    );
}
