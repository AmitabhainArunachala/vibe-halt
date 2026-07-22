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
    assert!(stdout.contains("ALWAYS-FAIL oracle:durability"), "{stdout}");
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

#[test]
fn palette_flag_accepts_v0_and_swarm_but_keeps_v0_default() {
    let (default_code, default_stdout, _) = vh(&[
        "run",
        "--workload",
        "demo",
        "--seed",
        "0xD1CE",
        "--universes",
        "5",
    ]);
    let (v0_code, v0_stdout, _) = vh(&[
        "run",
        "--workload",
        "demo",
        "--seed",
        "0xD1CE",
        "--universes",
        "5",
        "--palette",
        "v0",
    ]);
    let (swarm_code, swarm_stdout, _) = vh(&[
        "run",
        "--workload",
        "demo",
        "--seed",
        "0xD1CE",
        "--universes",
        "5",
        "--palette",
        "swarm",
    ]);
    assert_eq!(default_code, 0, "{default_stdout}");
    assert_eq!(v0_code, 0, "{v0_stdout}");
    assert_eq!(
        default_stdout, v0_stdout,
        "explicit --palette v0 must be bit-identical to the default"
    );
    assert_eq!(swarm_code, 0, "{swarm_stdout}");
    assert!(swarm_stdout.contains("palette=swarm"), "{swarm_stdout}");
}

#[test]
fn unknown_palette_is_usage_error() {
    let (code, _, stderr) = vh(&["run", "--palette", "magic"]);
    assert_eq!(code, 2);
    assert!(
        stderr.contains("unknown palette \"magic\"; expected v0 or swarm"),
        "{stderr}"
    );
}

// ---- evidence store + replay bundles (convergence C4, audit R4) ----

fn unique_tmp(label: &str) -> std::path::PathBuf {
    let dir = std::env::temp_dir().join(format!("vh-c4-{label}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("create tmp");
    dir
}

/// The full C4 acceptance in one flow: receipts are byte-deterministic
/// across two runs; a finding bundle copied OUT of the out-dir replays
/// standalone after the out-dirs are deleted (exit 0, anchored
/// REPRODUCED); a tampered bundle fails closed (exit 1, anchored
/// MISMATCH); an unreadable path is a usage error (exit 2).
#[test]
fn run_out_receipts_are_deterministic_and_bundles_replay_standalone() {
    let tmp = unique_tmp("roundtrip");
    let a = tmp.join("A");
    let b = tmp.join("B");
    for out in [&a, &b] {
        let (code, stdout, _) = vh(&[
            "run",
            "--workload",
            "demo-buggy",
            "--seed",
            "0xD1CE",
            "--universes",
            "100",
            "--out",
            out.to_str().unwrap(),
        ]);
        assert_eq!(code, 1, "demo-buggy must still exit 1 with --out");
        assert!(
            stdout.contains("receipts: ") && stdout.contains("vh-run-receipts-v1"),
            "missing receipts summary line:\n{stdout}"
        );
    }
    let run_a = std::fs::read_to_string(a.join("run.ndjson")).unwrap();
    let run_b = std::fs::read_to_string(b.join("run.ndjson")).unwrap();
    assert_eq!(run_a, run_b, "run.ndjson must be byte-deterministic");

    // Find the first bundle through the receipt index itself.
    let rel_path = run_a
        .lines()
        .filter_map(|l| vh_cli::receipts::parse_line(l).ok())
        .find_map(|fields| {
            let rec = fields.iter().find(|(k, _)| k == "record")?.1.as_str()?;
            if rec != "finding" {
                return None;
            }
            fields
                .iter()
                .find(|(k, _)| k == "path")?
                .1
                .as_str()
                .map(str::to_string)
        })
        .expect("demo-buggy run must index at least one finding bundle");
    let bundle_a = std::fs::read_to_string(a.join(&rel_path)).unwrap();
    let bundle_b = std::fs::read_to_string(b.join(&rel_path)).unwrap();
    assert_eq!(bundle_a, bundle_b, "bundles must be byte-deterministic");

    // Standalone: copy the bundle out, delete BOTH out-dirs entirely.
    let standalone = tmp.join("standalone.ndjson");
    std::fs::write(&standalone, &bundle_a).unwrap();
    std::fs::remove_dir_all(&a).unwrap();
    std::fs::remove_dir_all(&b).unwrap();

    let (code, stdout, _) = vh(&["replay-bundle", standalone.to_str().unwrap()]);
    assert_eq!(code, 0, "standalone replay must exit 0:\n{stdout}");
    assert!(
        stdout.contains("replay-bundle: REPRODUCED"),
        "missing anchored REPRODUCED verdict:\n{stdout}"
    );

    // Tamper: flip the recorded trace hash — fail closed.
    let bundle = vh_cli::receipts::FindingBundle::parse(&bundle_a).unwrap();
    let tampered_text = bundle_a.replace(&bundle.trace_hash, "00000000000000000000000000000000");
    let tampered = tmp.join("tampered.ndjson");
    std::fs::write(&tampered, tampered_text).unwrap();
    let (code, stdout, _) = vh(&["replay-bundle", tampered.to_str().unwrap()]);
    assert_eq!(code, 1, "tampered bundle must exit 1:\n{stdout}");
    assert!(
        stdout.contains("replay-bundle: MISMATCH"),
        "missing anchored MISMATCH verdict:\n{stdout}"
    );

    // Unreadable path: usage error, never a verdict.
    let (code, _, stderr) = vh(&["replay-bundle", tmp.join("nope").to_str().unwrap()]);
    assert_eq!(code, 2, "unreadable bundle must exit 2:\n{stderr}");

    let _ = std::fs::remove_dir_all(&tmp);
}

/// --out is a campaign receipt writer; the single-universe repro path
/// must reject it rather than silently write a one-universe "campaign".
#[test]
fn out_conflicts_with_single_universe_replay() {
    let (code, _, stderr) = vh(&[
        "run",
        "--workload",
        "demo",
        "--universe",
        "0",
        "--out",
        "/tmp/never-written",
    ]);
    assert_eq!(code, 2, "--out with --universe must be a usage error");
    assert!(
        stderr.contains("--out conflicts with --universe"),
        "missing typed diagnostic:\n{stderr}"
    );
}
