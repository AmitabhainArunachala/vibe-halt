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
        stderr.contains("--universes must be nonzero \\u{2014} zero work is never certified"),
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
fn over_bound_source_commit_is_rejected_before_execution() {
    let source_commit = "x".repeat(4097);
    let (code, stdout, stderr) = vh(&[
        "run",
        "--workload",
        "demo",
        "--universes",
        "1",
        "--source-commit",
        &source_commit,
    ]);
    assert_eq!(code, 2, "over-bound CLI input must be a usage error");
    assert!(
        stdout.is_empty(),
        "execution started before refusal: {stdout}"
    );
    assert!(
        stderr.contains("bounded printable request profile"),
        "{stderr}"
    );
}

#[test]
fn invisible_format_controls_are_refused_and_reflected_unicode_is_escaped() {
    let (code, stdout, stderr) = vh(&[
        "run",
        "--workload",
        "demo",
        "--universes",
        "1",
        "--source-commit",
        "right-to-left\u{202e}",
    ]);
    assert_eq!(code, 2);
    assert!(
        stdout.is_empty(),
        "execution started before refusal: {stdout}"
    );
    assert!(!stderr.contains('\u{202e}'), "{stderr:?}");
    assert!(
        stderr.contains("bounded printable request profile"),
        "{stderr}"
    );

    let (code, stdout, stderr) = vh(&["sandbox-demo", "--mode", "caf\u{e9}"]);
    assert_eq!(code, 2);
    assert!(stdout.is_empty());
    assert!(!stderr.contains('\u{e9}'), "{stderr:?}");
    assert!(stderr.contains(r"caf\u{e9}"), "{stderr:?}");
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

fn verify_run(out: &std::path::Path) -> (i32, String, String) {
    vh(&[
        "verify-run",
        "--out",
        out.to_str().unwrap(),
        "--engine",
        env!("CARGO_BIN_EXE_vh"),
    ])
}

fn rewrite_run_digest(out: &std::path::Path, mutate: impl FnOnce(String) -> String) {
    let path = out.join("run.ndjson");
    let text = std::fs::read_to_string(&path).unwrap();
    let mut lines: Vec<&str> = text.split('\n').collect();
    assert_eq!(lines.pop(), Some(""));
    let digest_line = lines.pop().expect("digest line");
    assert!(digest_line.contains("\"record\":\"digest\""));
    let body = mutate(lines.join("\n") + "\n");
    let digest = vh_digest::sha256_hex(body.as_bytes());
    let digest_line = vh_cli::receipts::render_line(&[
        ("record", vh_cli::receipts::Val::S("digest".into())),
        ("alg", vh_cli::receipts::Val::S("sha256".into())),
        ("value", vh_cli::receipts::Val::S(digest)),
    ]);
    std::fs::write(path, format!("{body}{digest_line}\n")).unwrap();
}

fn write_forged_manifest_only(out: &std::path::Path, universes: u64) {
    std::fs::create_dir(out).unwrap();
    let profile = if cfg!(debug_assertions) {
        "debug"
    } else {
        "release"
    };
    let manifest = vh_cli::receipts::render_line(&[
        ("record", vh_cli::receipts::Val::S("manifest".into())),
        (
            "schema",
            vh_cli::receipts::Val::S(vh_cli::receipts_v2::RUN_RECEIPTS_SCHEMA_V2.into()),
        ),
        ("workload", vh_cli::receipts::Val::S("demo".into())),
        ("seed", vh_cli::receipts::Val::S("0xd1ce".into())),
        ("universes", vh_cli::receipts::Val::N(universes)),
        ("palette", vh_cli::receipts::Val::S("v0".into())),
        ("schedule_policy", vh_cli::receipts::Val::S("fifo".into())),
        ("divergence_check", vh_cli::receipts::Val::B(true)),
        ("verdict", vh_cli::receipts::Val::S("CLEAN".into())),
        ("findings", vh_cli::receipts::Val::N(0)),
        ("divergent", vh_cli::receipts::Val::N(0)),
        ("sometimes_unreached", vh_cli::receipts::Val::N(0)),
        (
            "cli_version",
            vh_cli::receipts::Val::S(env!("CARGO_PKG_VERSION").into()),
        ),
        ("build_profile", vh_cli::receipts::Val::S(profile.into())),
        (
            "target_os",
            vh_cli::receipts::Val::S(std::env::consts::OS.into()),
        ),
        (
            "target_arch",
            vh_cli::receipts::Val::S(std::env::consts::ARCH.into()),
        ),
        ("declared_source_commit", vh_cli::receipts::Val::Null),
    ]);
    let body = format!("{manifest}\n");
    let digest = vh_digest::sha256_hex(body.as_bytes());
    let digest_line = vh_cli::receipts::render_line(&[
        ("record", vh_cli::receipts::Val::S("digest".into())),
        ("alg", vh_cli::receipts::Val::S("sha256".into())),
        ("value", vh_cli::receipts::Val::S(digest)),
    ]);
    std::fs::write(out.join("run.ndjson"), format!("{body}{digest_line}\n")).unwrap();
}

#[test]
fn verify_run_v2_freshly_reproduces_clean_findings_and_unchecked() {
    let tmp = unique_tmp("verify-run-shapes");
    let cases = [
        ("clean", "demo", false, 0, "CLEAN", 0, true),
        ("findings", "demo-buggy", false, 1, "FINDINGS", 1, true),
        ("unchecked", "demo", true, 3, "UNCHECKED", 3, false),
    ];
    for (label, workload, unchecked, run_code, verdict, outcome_code, verified) in cases {
        let out = tmp.join(label);
        let mut args = vec![
            "run",
            "--workload",
            workload,
            "--universes",
            "4",
            "--out",
            out.to_str().unwrap(),
        ];
        if unchecked {
            args.push("--no-divergence-check");
        }
        let (actual_run_code, _, run_stderr) = vh(&args);
        assert_eq!(actual_run_code, run_code, "{run_stderr}");
        let (code, stdout, stderr) = verify_run(&out);
        assert_eq!(code, 0, "{stderr}");
        assert!(stdout.contains("\"schema\":\"vh-verify-run-v2\""));
        assert!(stdout.contains("\"authentic\":true"), "{stdout}");
        assert!(
            stdout.contains(&format!("\"verified\":{verified}")),
            "{stdout}"
        );
        assert!(
            stdout.contains(&format!("\"verdict\":\"{verdict}\"")),
            "{stdout}"
        );
        assert!(
            stdout.contains(&format!("\"outcome_exit_code\":{outcome_code}")),
            "{stdout}"
        );
        assert!(stdout.contains("\"engine_request_digest\":"), "{stdout}");
    }
    let _ = std::fs::remove_dir_all(tmp);
}

#[test]
fn verify_run_rejects_digest_valid_claim_forgery_before_promotion() {
    let tmp = unique_tmp("verify-run-forgery");
    let out = tmp.join("receipt");
    write_forged_manifest_only(&out, 1);
    let (code, stdout, _) = verify_run(&out);
    assert_eq!(
        code, 1,
        "old claim-only verifier accepted a two-line forgery"
    );
    assert!(stdout.contains("\"authentic\":false"), "{stdout}");
    assert!(stdout.contains("\"verdict\":\"ERROR\""), "{stdout}");
    let _ = std::fs::remove_dir_all(tmp);
}

#[test]
fn verify_run_rejects_redigested_verdict_path_tree_and_bundle_tampering() {
    let tmp = unique_tmp("verify-run-adversarial");

    let verdict_out = tmp.join("verdict");
    assert_eq!(
        vh(&[
            "run",
            "--workload",
            "demo",
            "--universes",
            "4",
            "--out",
            verdict_out.to_str().unwrap(),
        ])
        .0,
        0
    );
    rewrite_run_digest(&verdict_out, |body| {
        body.replacen("\"verdict\":\"CLEAN\"", "\"verdict\":\"FINDINGS\"", 1)
    });
    assert_eq!(verify_run(&verdict_out).0, 1);

    let orphan_out = tmp.join("orphan");
    assert_eq!(
        vh(&[
            "run",
            "--workload",
            "demo",
            "--universes",
            "4",
            "--out",
            orphan_out.to_str().unwrap(),
        ])
        .0,
        0
    );
    std::fs::write(orphan_out.join("orphan"), b"must reject").unwrap();
    assert_eq!(verify_run(&orphan_out).0, 1);

    let path_out = tmp.join("path");
    assert_eq!(
        vh(&[
            "run",
            "--workload",
            "demo-buggy",
            "--universes",
            "4",
            "--out",
            path_out.to_str().unwrap(),
        ])
        .0,
        1
    );
    rewrite_run_digest(&path_out, |body| {
        body.replacen("\"path\":\"findings/", "\"path\":\"../", 1)
    });
    assert_eq!(verify_run(&path_out).0, 1);

    let bundle_out = tmp.join("bundle");
    assert_eq!(
        vh(&[
            "run",
            "--workload",
            "demo-buggy",
            "--universes",
            "4",
            "--out",
            bundle_out.to_str().unwrap(),
        ])
        .0,
        1
    );
    let bundle = dir_snapshot(&bundle_out)
        .into_iter()
        .find(|(path, _)| path.ends_with("finding.ndjson"))
        .unwrap()
        .0;
    std::fs::write(bundle_out.join(bundle), b"{}\n").unwrap();
    assert_eq!(verify_run(&bundle_out).0, 1);

    let _ = std::fs::remove_dir_all(tmp);
}

#[test]
fn verify_run_rejects_wrong_engine_and_oversized_work_claim_without_replay() {
    let tmp = unique_tmp("verify-run-bounds");
    let out = tmp.join("receipt");
    write_forged_manifest_only(&out, 10_001);
    let (code, stdout, _) = verify_run(&out);
    assert_eq!(code, 1);
    assert!(stdout.contains("work bound"), "{stdout}");

    let fake_engine = tmp.join("fake-engine");
    std::fs::write(&fake_engine, b"not the executing image").unwrap();
    let (code, stdout, _) = vh(&[
        "verify-run",
        "--out",
        out.to_str().unwrap(),
        "--engine",
        fake_engine.to_str().unwrap(),
    ]);
    assert_eq!(code, 2);
    assert!(stdout.is_empty());
    let _ = std::fs::remove_dir_all(tmp);
}

#[test]
fn demo_admission_executes_and_freshly_verifies_the_fixed_pair() {
    let tmp = unique_tmp("demo-admission-positive");
    let out = tmp.join("evidence");
    let (code, stdout, stderr) = vh(&[
        "demo-admission",
        "--out",
        out.to_str().expect("UTF-8 test path"),
    ]);

    assert_eq!(
        code, 1,
        "the seeded faulty target must HALT:\n{stdout}\n{stderr}"
    );
    assert!(stderr.is_empty(), "{stderr}");
    assert!(
        stdout.starts_with("vh-real-execution-receipt-digest-v1\n"),
        "{stdout}"
    );
    assert!(stdout.contains("admission-kind 9:CONFIRMED\n"), "{stdout}");
    assert!(stdout.contains("fixed-control-miss true\n"), "{stdout}");
    assert!(
        stdout.contains("confirmation-authority 17:RUST_FRESH_REPLAY\n"),
        "{stdout}"
    );
    assert!(
        stdout.contains("treatment-outcome 8:FINDINGS\n"),
        "{stdout}"
    );
    assert!(
        stdout.contains("fixed-control-outcome 5:CLEAN\n"),
        "{stdout}"
    );
    assert!(
        stdout.contains("treatment-budget-universes 4\n"),
        "{stdout}"
    );
    assert!(
        stdout.contains("fixed-control-budget-universes 4\n"),
        "{stdout}"
    );
    let receipt = std::fs::read(out.join("admission.receipt")).unwrap();
    assert_eq!(receipt, stdout.as_bytes());
    assert_eq!(vh_digest::sha256_hex(&receipt).len(), 64);
    assert!(out.join("treatment/run.ndjson").is_file());
    assert!(out.join("fixed-control/run.ndjson").is_file());

    let before = receipt;
    let (rerun_code, rerun_stdout, rerun_stderr) = vh(&[
        "demo-admission",
        "--out",
        out.to_str().expect("UTF-8 test path"),
    ]);
    assert_eq!(rerun_code, 2);
    assert!(rerun_stdout.is_empty());
    assert!(rerun_stderr.contains("failed closed"), "{rerun_stderr}");
    assert_eq!(
        std::fs::read(out.join("admission.receipt")).unwrap(),
        before
    );
    let _ = std::fs::remove_dir_all(tmp);
}

#[test]
fn demo_admission_refuses_an_occupied_root_before_any_arm_runs() {
    let tmp = unique_tmp("demo-admission-occupied");
    let out = tmp.join("evidence");
    std::fs::create_dir(&out).unwrap();
    std::fs::write(out.join("owner-marker"), b"preserve").unwrap();

    let (code, stdout, stderr) = vh(&[
        "demo-admission",
        "--out",
        out.to_str().expect("UTF-8 test path"),
    ]);

    assert_eq!(code, 2);
    assert!(stdout.is_empty());
    assert!(stderr.contains("failed closed"), "{stderr}");
    assert_eq!(
        std::fs::read(out.join("owner-marker")).unwrap(),
        b"preserve"
    );
    assert!(!out.join("treatment").exists());
    assert!(!out.join("fixed-control").exists());
    assert!(!out.join("admission.receipt").exists());
    let _ = std::fs::remove_dir_all(tmp);
}

#[test]
fn eval_validate_maps_readable_structural_failures_to_invalid_exit_1() {
    let tmp = unique_tmp("eval-structural-invalid");
    let path = tmp.join("manifest-only.ndjson");
    std::fs::write(
        &path,
        b"{\"record\":\"manifest\",\"schema\":\"vibe-halt.holdout-manifest.v1\",\"name\":\"x\"}\n",
    )
    .unwrap();

    let (code, stdout, stderr) = vh(&[
        "eval-validate",
        "--dossier",
        path.to_str().expect("UTF-8 test path"),
    ]);

    assert_eq!(code, 1, "{stdout}\n{stderr}");
    assert!(stdout.contains("verdict: INVALID"), "{stdout}");
    assert!(stderr.contains("no dossier records found"), "{stderr}");
    let _ = std::fs::remove_dir_all(tmp);
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
            stdout.contains("receipts: ") && stdout.contains("vh-run-receipts-v2"),
            "missing v2 receipts summary line:\n{stdout}"
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

    // Tamper: flip the recorded trace hash — the v2 content digest
    // fails closed before any semantic comparison.
    let bundle = vh_cli::receipts_v2::FindingBundleV2::parse(&bundle_a).unwrap();
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

/// A decision-taped v2 bundle is not replayable by today's FIFO/untaped
/// replay path. Refuse the incompatible request before running or creating
/// the output directory instead of emitting evidence the same binary rejects.
#[test]
fn record_tape_conflicts_with_out_before_any_write() {
    let tmp = unique_tmp("taped-out");
    let out = tmp.join("must-not-exist");
    let (code, stdout, stderr) = vh(&[
        "run",
        "--workload",
        "demo-net-buggy",
        "--seed",
        "0xD1CE",
        "--universes",
        "100",
        "--record-tape",
        "--out",
        out.to_str().unwrap(),
    ]);
    assert_eq!(
        code, 2,
        "incompatible evidence modes must be a usage error, not a run verdict:\n\
         stdout:\n{stdout}\nstderr:\n{stderr}"
    );
    assert!(
        stderr.contains("--record-tape conflicts with --out"),
        "missing typed conflict diagnostic:\n{stderr}"
    );
    assert!(
        !out.exists(),
        "conflict must be rejected before creating the output directory"
    );
    let _ = std::fs::remove_dir_all(&tmp);
}

// ---- dirty --out refusal (C3-honesty; PR #19 thread PRRT_kwDOTdlCIM6S0Hr9) ----

/// Recursive (relative-path, bytes) snapshot, sorted, for byte-identity
/// proofs across a refused write.
fn dir_snapshot(root: &std::path::Path) -> Vec<(String, Vec<u8>)> {
    fn walk(root: &std::path::Path, dir: &std::path::Path, out: &mut Vec<(String, Vec<u8>)>) {
        for entry in std::fs::read_dir(dir).expect("read_dir") {
            let path = entry.expect("dir entry").path();
            if path.is_dir() {
                walk(root, &path, out);
            } else {
                let rel = path
                    .strip_prefix(root)
                    .expect("under root")
                    .to_string_lossy()
                    .into_owned();
                out.push((rel, std::fs::read(&path).expect("read file")));
            }
        }
    }
    let mut out = Vec::new();
    walk(root, root, &mut out);
    out.sort();
    out
}

/// PR #19's exact stale-finding mechanism, pinned: run once into DIR
/// (manifest + finding bundles written), run again into the SAME dir
/// with a different seed (different trace hashes, so different finding
/// ids). Pre-repair the second run overwrote `run.ndjson` in place and
/// the first run's `findings/<id>/` bundles survived as orphans the
/// fresh manifest no longer listed. The second run must refuse (exit 2)
/// before writing anything, leaving the first run's receipts
/// byte-identical.
#[test]
fn rerun_into_same_out_dir_refuses_instead_of_orphaning() {
    let tmp = unique_tmp("dirty-rerun");
    let out = tmp.join("receipts");
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
    assert_eq!(code, 1, "first run must exit 1 with findings:\n{stdout}");
    let before = dir_snapshot(&out);
    let paths: Vec<&str> = before.iter().map(|(p, _)| p.as_str()).collect();
    assert!(
        paths.iter().any(|p| p.starts_with("findings/")),
        "first run must write at least one finding bundle: {paths:?}"
    );

    let (code, _, stderr) = vh(&[
        "run",
        "--workload",
        "demo-buggy",
        "--seed",
        "0xBEEF",
        "--universes",
        "100",
        "--out",
        out.to_str().unwrap(),
    ]);
    assert_eq!(
        code, 2,
        "rerun into a non-empty --out must refuse with exit 2, never overwrite:\n{stderr}"
    );
    assert!(
        stderr.contains("is not empty"),
        "missing typed refusal diagnostic:\n{stderr}"
    );
    assert_eq!(
        dir_snapshot(&out),
        before,
        "refusal must leave every existing byte untouched"
    );
    let _ = std::fs::remove_dir_all(&tmp);
}

/// Refusal is fail-closed and write-free for arbitrary operator
/// directories, not only prior receipt dirs: pre-existing user files are
/// byte-identical after the refusal, no `run.ndjson` appears, and an
/// --out that is a plain FILE is an error (exit 2), never a write.
#[test]
fn out_refuses_non_empty_directory_before_any_write() {
    let tmp = unique_tmp("dirty-out");
    let out = tmp.join("keep");
    std::fs::create_dir_all(out.join("findings").join("u9-stale00cafe")).expect("mk stale");
    std::fs::write(out.join("precious.txt"), b"operator bytes\n").expect("write precious");
    std::fs::write(
        out.join("findings")
            .join("u9-stale00cafe")
            .join("finding.ndjson"),
        b"stale bundle\n",
    )
    .expect("write stale");
    let before = dir_snapshot(&out);

    let (code, _, stderr) = vh(&[
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
    assert_eq!(
        code, 2,
        "non-empty --out must refuse with exit 2, never the run verdict:\n{stderr}"
    );
    assert!(
        stderr.contains("is not empty"),
        "missing typed refusal diagnostic:\n{stderr}"
    );
    assert!(
        !out.join("run.ndjson").exists(),
        "refusal must not write a manifest"
    );
    assert_eq!(
        dir_snapshot(&out),
        before,
        "refusal must not touch existing files"
    );

    let file_out = tmp.join("not-a-dir");
    std::fs::write(&file_out, b"do not replace\n").expect("write file");
    let (code, _, stderr) = vh(&[
        "run",
        "--workload",
        "demo-buggy",
        "--seed",
        "0xD1CE",
        "--universes",
        "100",
        "--out",
        file_out.to_str().unwrap(),
    ]);
    assert_eq!(code, 2, "--out at a plain file must fail closed:\n{stderr}");
    assert_eq!(
        std::fs::read(&file_out).expect("reread"),
        b"do not replace\n",
        "the file at --out must be untouched"
    );
    let _ = std::fs::remove_dir_all(&tmp);
}

/// A caller-created EMPTY directory is accepted — refusal is about
/// non-empty contents, not prior existence.
#[test]
fn out_accepts_existing_empty_directory() {
    let tmp = unique_tmp("empty-out");
    let out = tmp.join("empty");
    std::fs::create_dir_all(&out).expect("mk empty");
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
    assert_eq!(code, 1, "empty existing --out must be accepted:\n{stdout}");
    assert!(
        out.join("run.ndjson").exists(),
        "receipts must be written into the empty directory"
    );
    let _ = std::fs::remove_dir_all(&tmp);
}

// ---- boundary-side shrink wiring (convergence C5, audit R1) ----

/// The charter's C5 acceptance, pinned: `vh run --workload demo-buggy
/// --seed 0xD1CE --universes 100 --shrink` exits 1 and prints a shrunk
/// plan with STRICTLY fewer injections whose replay reproduces the SAME
/// oracle violation (exact fingerprint — the oracle inside shrink_cli
/// matches name+detail, never any-failure).
#[test]
fn run_shrink_minimizes_first_failing_universe_strictly() {
    let (code, stdout, _) = vh(&[
        "run",
        "--workload",
        "demo-buggy",
        "--seed",
        "0xD1CE",
        "--universes",
        "100",
        "--shrink",
    ]);
    assert_eq!(code, 1, "--shrink must not change the FINDINGS exit code");
    let line = stdout
        .lines()
        .find(|l| l.starts_with("  shrink: MINIMIZED"))
        .unwrap_or_else(|| panic!("missing anchored MINIMIZED line:\n{stdout}"));
    // "  shrink: MINIMIZED N -> M injection(s) ..."
    let mut nums = line
        .split_whitespace()
        .filter_map(|w| w.parse::<usize>().ok());
    let original = nums.next().expect("original count");
    let minimized = nums.next().expect("minimized count");
    assert!(
        minimized < original,
        "shrink must remove at least one injection ({original} -> {minimized}):\n{stdout}"
    );
    assert!(
        stdout.contains("  shrink-binding: workload=demo-buggy seed=0xd1ce universe="),
        "missing provenance binding line:\n{stdout}"
    );
}

/// Standalone minimization replays to the same violation: shrink one
/// universe, then independently verify the minimized plan through the
/// public replay hook — same exact failure detail as the baseline.
#[test]
fn standalone_shrink_result_reproduces_the_exact_baseline_violation() {
    let outcome = vh_cli::shrink_cli::shrink_universe("demo-buggy", 0xD1CE, 2)
        .expect("universe 2 is a known failing universe");
    assert!(outcome.minimized_injections < outcome.original_injections);
    // Independent replay of the minimized plan through the public hook:
    // the SAME oracle violation, exact detail — not any-failure.
    let w = vh_cli::workloads::by_name("demo-buggy").unwrap();
    let replayed = vh_multiverse::run_universe_with_fault_plan(
        0xD1CE,
        2,
        w.as_ref(),
        outcome.minimized_plan.clone(),
    );
    let replayed_failures: Vec<(String, String)> = replayed
        .always_failures()
        .iter()
        .map(|f| (f.name.clone(), f.detail.clone()))
        .collect();
    assert_eq!(
        replayed_failures, outcome.baseline_failures,
        "minimized plan switched cause — exact fingerprint law violated"
    );
    // And removing the last kept injection must lose the violation
    // (1-minimality is a claim, so check its negative once).
    assert!(!outcome.minimized_plan.injections().is_empty());
    let without_last = vh_gremlin::FaultPlan::new(
        outcome.minimized_plan.injections()[..outcome.minimized_plan.injections().len() - 1]
            .to_vec(),
    );
    let weaker = vh_multiverse::run_universe_with_fault_plan(0xD1CE, 2, w.as_ref(), without_last);
    let weaker_failures: Vec<(String, String)> = weaker
        .always_failures()
        .iter()
        .map(|f| (f.name.clone(), f.detail.clone()))
        .collect();
    assert_ne!(
        weaker_failures, outcome.baseline_failures,
        "dropping a kept injection should not still reproduce the exact violation"
    );
}

#[test]
fn shrink_exit_contract_is_typed() {
    // Clean universe: nothing to shrink — exit 1, anchored UNAVAILABLE.
    let (code, stdout, _) = vh(&[
        "shrink",
        "--workload",
        "demo",
        "--seed",
        "0xD1CE",
        "--universe",
        "0",
    ]);
    assert_eq!(code, 1);
    assert!(stdout.contains("shrink: UNAVAILABLE"));
    // Unsupported workload: usage-class error, exit 2.
    let (code, _, stderr) = vh(&[
        "shrink",
        "--workload",
        "corpus-lost-update",
        "--seed",
        "0xD1CE",
        "--universe",
        "1",
    ]);
    assert_eq!(code, 2);
    assert!(stderr.contains("does not support workload"));
    // Missing --universe: usage error.
    let (code, _, _) = vh(&["shrink", "--workload", "demo-buggy"]);
    assert_eq!(code, 2);
    // --shrink conflicts with --universe and with non-v0 palettes.
    let (code, _, _) = vh(&[
        "run",
        "--workload",
        "demo-buggy",
        "--universe",
        "2",
        "--shrink",
    ]);
    assert_eq!(code, 2);
    let (code, _, _) = vh(&[
        "run",
        "--workload",
        "demo-buggy",
        "--palette",
        "swarm",
        "--shrink",
    ]);
    assert_eq!(code, 2);
}

// ---- decision tape (convergence C1, W2/RFC-003) ----

/// The W2 acceptance: two PROCESSES, same seed, same universe -> same
/// tape digest; the tape is additive (separate stream + line) and the
/// legacy demo path never grows one.
#[test]
fn decision_tape_digest_is_identical_across_processes() {
    let args = [
        "run",
        "--workload",
        "demo-net",
        "--seed",
        "0xD1CE",
        "--universe",
        "3",
        "--record-tape",
    ];
    let (_, out_a, _) = vh(&args);
    let (_, out_b, _) = vh(&args);
    let tape_line = |out: &str| -> String {
        out.lines()
            .find(|l| l.starts_with("  decision tape: "))
            .unwrap_or_else(|| panic!("missing decision tape line:\n{out}"))
            .to_string()
    };
    let a = tape_line(&out_a);
    let b = tape_line(&out_b);
    assert_eq!(a, b, "two processes must agree on the tape digest");
    assert!(
        a.contains("(vh-decision-tape-v1)"),
        "tape line must carry its schema: {a}"
    );
    // The digest is a real 32-hex digest, not a placeholder.
    let digest = a
        .trim_start_matches("  decision tape: ")
        .split_whitespace()
        .next()
        .unwrap();
    assert_eq!(digest.len(), 32, "expected 32-hex digest, got {digest:?}");
    assert!(digest.chars().all(|c| c.is_ascii_hexdigit()));
}

/// Leak test half 1: the LEGACY (non-runtime) demo path must never
/// print a tape line — a tape there would mean the frozen demo
/// universe silently migrated onto the sim runtime.
#[test]
fn legacy_demo_universe_has_no_decision_tape() {
    let (_, out, _) = vh(&[
        "run",
        "--workload",
        "demo",
        "--seed",
        "0xD1CE",
        "--universe",
        "0",
        "--record-tape",
    ]);
    assert!(
        !out.contains("decision tape:"),
        "legacy demo must not grow a tape:\n{out}"
    );
    assert!(out.contains("hash 9ce6199f133f4d3c9dd0da0075e352d2 events 45"));
}

/// Leak test half 2: recording the tape must not perturb the execution
/// trace — a runtime universe's trace hash with tape recording live is
/// compared against the whole-observable replay agreement (two in-process
/// runs), and the tape digest differs between different universes while
/// the same universe's digest is stable.
#[test]
fn decision_tape_is_additive_and_universe_specific() {
    use vh_gremlin::FaultPalette;
    let w = vh_cli::workloads::by_name("demo-net").unwrap();
    let rec =
        |u| vh_multiverse::run_universe_recorded(0xD1CE, u, w.as_ref(), FaultPalette::V0, true);
    let a = rec(3);
    let b = rec(3);
    assert!(a.observably_equal(&b));
    assert_eq!(a.decision_tape_digest(), b.decision_tape_digest());
    assert!(a.decision_tape_digest().is_some());
    let other = rec(4);
    assert_ne!(
        a.decision_tape_digest(),
        other.decision_tape_digest(),
        "different universes make different scheduling decisions"
    );
    // The default (un-recorded) path stays the C1-kill-criterion
    // fallback: no tape, and every OTHER observable identical to the
    // recorded run — the tape is purely additive.
    let plain = vh_multiverse::run_universe(0xD1CE, 3, w.as_ref());
    assert!(plain.decision_tape_digest().is_none());
    assert_eq!(plain.trace_hash(), a.trace_hash());
    assert_eq!(plain.trace_events(), a.trace_events());
    // Legacy path: no runtime, no tape, flag or not.
    let demo = vh_cli::workloads::by_name("demo").unwrap();
    let legacy =
        vh_multiverse::run_universe_recorded(0xD1CE, 0, demo.as_ref(), FaultPalette::V0, true);
    assert!(legacy.decision_tape_digest().is_none());
}

// ---- schedule strategies + VB-006 (convergence C2, W3) ----

/// The C2 acceptance pair: VB-006 is INVISIBLE to FIFO v0 (red-on-v0,
/// in-process 1000-universe check; 10k pinned in the receipt) and PCT
/// d=3 finds it within 100 universes at the pinned seed.
#[test]
fn vb006_invisible_to_fifo_and_found_by_pct() {
    use vh_gremlin::FaultPalette;
    use vh_multiverse::SchedulePolicy;
    let w = vh_cli::workloads::by_name("corpus-same-timestamp-race").unwrap();
    let cfg = vh_multiverse::MultiverseConfig {
        root_seed: 0xD1CE,
        universes: vh_multiverse::UniverseCount::try_from(1000).unwrap(),
        check_divergence: false,
    };
    let fifo = vh_multiverse::run_multiverse(&cfg, w.as_ref());
    assert!(
        fifo.failing_universes().is_empty(),
        "VB-006 must be invisible to FIFO v0"
    );
    let cfg100 = vh_multiverse::MultiverseConfig {
        root_seed: 0xD1CE,
        universes: vh_multiverse::UniverseCount::try_from(100).unwrap(),
        check_divergence: true,
    };
    let pct = vh_multiverse::run_multiverse_scheduled(
        &cfg100,
        w.as_ref(),
        FaultPalette::V0,
        true,
        SchedulePolicy::Pct { depth: 3 },
    );
    assert!(
        !pct.failing_universes().is_empty(),
        "PCT d=3 must find VB-006 within 100 universes"
    );
    assert!(
        pct.divergent_universes().is_empty(),
        "PCT must replay deterministically"
    );
}

/// Exploratory schedules replay byte-identically: same (seed, universe,
/// policy) -> same observable result INCLUDING the tape digest.
#[test]
fn scheduled_universe_replays_byte_identically_with_tape() {
    use vh_gremlin::FaultPalette;
    use vh_multiverse::SchedulePolicy;
    let w = vh_cli::workloads::by_name("corpus-same-timestamp-race").unwrap();
    let run = || {
        vh_multiverse::run_universe_scheduled(
            0xD1CE,
            0,
            w.as_ref(),
            FaultPalette::V0,
            true,
            SchedulePolicy::Pct { depth: 3 },
        )
    };
    let a = run();
    let b = run();
    assert!(a.observably_equal(&b));
    assert!(a.decision_tape_digest().is_some());
    assert_eq!(a.decision_tape_digest(), b.decision_tape_digest());
    // And the uniform comparator is likewise deterministic.
    let u = || {
        vh_multiverse::run_universe_scheduled(
            0xD1CE,
            0,
            w.as_ref(),
            FaultPalette::V0,
            true,
            SchedulePolicy::UniformTiebreak,
        )
    };
    assert!(u().observably_equal(&u()));
}

/// The schedule flag's typed edges: unknown value, and the fail-closed
/// conflicts with the policy-less replay paths (--out, --shrink).
#[test]
fn schedule_flag_contract_is_typed() {
    let (code, _, stderr) = vh(&["run", "--schedule", "chaotic"]);
    assert_eq!(code, 2);
    assert!(stderr.contains("unknown schedule"));
    let (code, _, stderr) = vh(&[
        "run",
        "--workload",
        "demo-buggy",
        "--schedule",
        "pct:3",
        "--out",
        "/tmp/never",
    ]);
    assert_eq!(code, 2);
    assert!(stderr.contains("conflicts with --shrink and --out"));
    let (code, _, _) = vh(&[
        "run",
        "--workload",
        "demo-buggy",
        "--schedule",
        "uniform",
        "--shrink",
    ]);
    assert_eq!(code, 2);
}

/// A non-FIFO finding's printed repro must carry the schedule flag and
/// actually reproduce (C2): a flagless repro replays under the FIFO
/// default, where VB-006 is invisible by construction — the
/// one-command-repro law would break silently.
#[test]
fn pct_repro_line_carries_schedule_and_reproduces() {
    let (code, stdout, _) = vh(&[
        "run",
        "--workload",
        "corpus-same-timestamp-race",
        "--seed",
        "0xD1CE",
        "--universes",
        "100",
        "--schedule",
        "pct:3",
    ]);
    assert_eq!(code, 1, "PCT d=3 must find VB-006:\n{stdout}");
    let repro = stdout
        .lines()
        .find(|l| l.trim_start().starts_with("repro: vh run "))
        .expect("a printed repro line");
    assert!(
        repro.contains("--schedule pct:3"),
        "repro must carry the schedule policy (FIFO replay hides VB-006): {repro}"
    );
    let args: Vec<&str> = repro
        .trim_start()
        .trim_start_matches("repro: vh ")
        .split_whitespace()
        .collect();
    let (rcode, rout, _) = vh(&args);
    assert_eq!(
        rcode, 1,
        "printed repro must reproduce the finding:\n{rout}"
    );
    assert!(
        rout.contains("replay verdict: FINDINGS"),
        "repro must end in FINDINGS:\n{rout}"
    );
}

// ---- v2 shrink lineage + v1 compatibility (post-audit C3, audit D.3) ----

/// Rebuild a v2 bundle's trailing content digest after an intentional
/// body edit, so lineage tests probe SEMANTIC verification rather than
/// (only) the digest check.
fn rebuild_v2_digest(text: &str) -> String {
    let body_end = text.rfind("{\"record\":\"digest\"").expect("digest record");
    let body = &text[..body_end];
    format!(
        "{body}{{\"record\":\"digest\",\"alg\":\"sha256\",\"value\":\"{}\"}}\n",
        vh_digest::sha256_hex(body.as_bytes())
    )
}

/// `--shrink --out` persists the minimized plan INSIDE the shrunk
/// universe's bundle; replay CONSUMES that plan and verifies its digest,
/// observation identity, and failure fingerprint. Adversarial edits —
/// a different "minimized" plan (the original-plan-presented-as-minimized
/// class), a changed minimized failure detail, and a lineage spliced onto
/// a different baseline — all fail closed.
#[test]
fn shrink_lineage_is_persisted_consumed_and_tamper_evident() {
    let tmp = unique_tmp("lineage");
    let out = tmp.join("out");
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
        "--shrink",
        "--source-commit",
        "cafe0001",
    ]);
    assert_eq!(code, 1, "demo-buggy must still exit 1:\n{stdout}");
    assert!(stdout.contains("shrink: MINIMIZED"), "{stdout}");

    // The shrunk universe is the FIRST failing one; find its bundle by
    // looking for the shrink record.
    let findings_dir = out.join("findings");
    let mut lineage_bundle = None;
    for entry in std::fs::read_dir(&findings_dir).unwrap() {
        let p = entry.unwrap().path().join("finding.ndjson");
        let text = std::fs::read_to_string(&p).unwrap();
        if text.contains("{\"record\":\"shrink\"") {
            lineage_bundle = Some((p, text));
            break;
        }
    }
    let (path, text) = lineage_bundle.expect("one bundle must carry the shrink lineage");
    let parsed = vh_cli::receipts_v2::FindingBundleV2::parse(&text).unwrap();
    let lineage = parsed.lineage.as_ref().expect("lineage present");
    assert!(
        lineage.minimized_plan.len() as u64 <= lineage.original_injections,
        "minimized plan cannot exceed the original"
    );
    assert_eq!(
        parsed.provenance.declared_source_commit.as_deref(),
        Some("cafe0001")
    );

    // Positive: replay consumes the minimized plan.
    let (code, stdout, _) = vh(&["replay-bundle", path.to_str().unwrap()]);
    assert_eq!(code, 0, "{stdout}");
    assert!(
        stdout.contains("lineage-minimized") && stdout.contains("consumed+verified"),
        "replay must report minimized-plan consumption:\n{stdout}"
    );

    // Adversarial 1: present a DIFFERENT plan as "minimized" (the
    // original-plan-repro-as-minimized-evidence class). Append one more
    // injection, fix the declared count, rebuild the digest.
    let with_extra = text.replacen(
        "{\"record\":\"minimized_failure\"",
        "{\"record\":\"injection\",\"at_nanos\":999999,\"fault\":\"fsync_lie\"}\n{\"record\":\"minimized_failure\"",
        1,
    );
    let declared = format!("\"minimized_injections\":{}", lineage.minimized_plan.len());
    let fixed = with_extra.replacen(
        &declared,
        &format!(
            "\"minimized_injections\":{}",
            lineage.minimized_plan.len() + 1
        ),
        1,
    );
    let forged = tmp.join("forged-plan.ndjson");
    std::fs::write(&forged, rebuild_v2_digest(&fixed)).unwrap();
    let (code, stdout, _) = vh(&["replay-bundle", forged.to_str().unwrap()]);
    assert_eq!(code, 1, "forged minimized plan must MISMATCH:\n{stdout}");
    assert!(
        stdout.contains("minimized plan digest") || stdout.contains("MISMATCH"),
        "{stdout}"
    );

    // Adversarial 2: change a minimized failure detail.
    let cooked = text.replacen("missing after crash", "missing after cr4sh", 1);
    assert_ne!(cooked, text, "edit must apply");
    let cooked_path = tmp.join("cooked-detail.ndjson");
    std::fs::write(&cooked_path, rebuild_v2_digest(&cooked)).unwrap();
    let (code, stdout, _) = vh(&["replay-bundle", cooked_path.to_str().unwrap()]);
    assert_eq!(code, 1, "cooked failure detail must MISMATCH:\n{stdout}");

    // Adversarial 3: splice the lineage onto a different baseline.
    let spliced = text.replacen(
        &format!("\"original_digest\":\"{}\"", lineage.original_digest),
        "\"original_digest\":\"ffffffffffffffffffffffffffffffff\"",
        1,
    );
    let spliced_path = tmp.join("spliced.ndjson");
    std::fs::write(&spliced_path, rebuild_v2_digest(&spliced)).unwrap();
    let (code, _, stderr) = vh(&["replay-bundle", spliced_path.to_str().unwrap()]);
    assert_eq!(
        code, 2,
        "spliced baseline must be rejected at parse:\n{stderr}"
    );
    assert!(stderr.contains("spliced"), "{stderr}");

    // Attacker-controlled v2 fields are self-digested, not trusted. Their
    // replay diagnostics must remain bounded, printable ASCII and incapable
    // of adding terminal lines/state even when semantic verification fails.
    let hostile_value = format!("line\n\x1b[31m\u{202e}{}", "x".repeat(4096));

    let mut hostile_observation = parsed.clone();
    hostile_observation.observation_sha256 = hostile_value.clone();
    hostile_observation.finding_id = vh_cli::receipts_v2::finding_id_v2(
        hostile_observation.universe,
        &hostile_observation.observation_sha256,
    );
    let hostile_observation_path = tmp.join("hostile-observation.ndjson");
    std::fs::write(&hostile_observation_path, hostile_observation.to_ndjson()).unwrap();
    let (code, stdout, stderr) = vh(&["replay-bundle", hostile_observation_path.to_str().unwrap()]);
    assert_eq!(code, 1, "{stdout}{stderr}");
    assert_eq!(
        stdout.lines().count(),
        2,
        "hostile field split output: {stdout:?}"
    );
    assert!(!stdout.contains('\x1b') && !stdout.contains('\u{202e}'));
    assert!(stdout.lines().all(|line| line.len() < 512));

    for (field, mutate) in [("workload", true), ("schedule_policy", false)] {
        let mut hostile = parsed.clone();
        if mutate {
            hostile.workload = hostile_value.clone();
        } else {
            hostile.schedule_policy = hostile_value.clone();
        }
        let hostile_path = tmp.join(format!("hostile-{field}.ndjson"));
        std::fs::write(&hostile_path, hostile.to_ndjson()).unwrap();
        let (code, stdout, stderr) = vh(&["replay-bundle", hostile_path.to_str().unwrap()]);
        assert_eq!(code, 2, "field={field}: {stdout}{stderr}");
        assert!(stdout.is_empty(), "field={field}: {stdout:?}");
        assert_eq!(
            stderr.lines().count(),
            1,
            "field={field} split output: {stderr:?}"
        );
        assert!(!stderr.contains('\x1b') && !stderr.contains('\u{202e}'));
        assert!(stderr.lines().all(|line| line.len() < 512));
        assert!(
            stderr.contains("...[truncated]"),
            "field={field}: {stderr:?}"
        );
    }

    let _ = std::fs::remove_dir_all(&tmp);
}

/// v1 bundles remain replayable within their explicit limitation — the
/// reader labels them FIFO-only self-consistent replay. The v1 content
/// is built from a live in-process run, so the test never hardcodes
/// identity values.
#[test]
fn v1_bundles_stay_replayable_with_the_limitation_label() {
    let tmp = unique_tmp("v1compat");
    let workload = vh_cli::workloads::by_name("demo-buggy").unwrap();
    // Find a failing universe deterministically.
    let (universe, result) = (0..100)
        .map(|u| (u, vh_multiverse::run_universe(0xD1CE, u, workload.as_ref())))
        .find(|(_, r)| !r.always_failures().is_empty())
        .expect("demo-buggy fails somewhere in 100 universes");
    let v1 = vh_cli::receipts::FindingBundle {
        finding_id: format!("u{universe}-legacy"),
        workload: "demo-buggy".into(),
        seed: 0xD1CE,
        palette: "v0".into(),
        universe,
        trace_hash: result.trace_hash().to_string(),
        trace_events: result.trace_events() as u64,
        fault_plan_digest: result.fault_plan_digest().map(str::to_string),
        failures: result
            .always_failures()
            .iter()
            .map(|f| (f.name.clone(), f.detail.clone()))
            .collect(),
        contract_violations: workload.property_contract().violations(&result),
        invalid_completion: None,
    };
    let path = tmp.join("v1.ndjson");
    std::fs::write(&path, v1.to_ndjson()).unwrap();
    let (code, stdout, _) = vh(&["replay-bundle", path.to_str().unwrap()]);
    assert_eq!(code, 0, "{stdout}");
    assert!(
        stdout.contains("REPRODUCED") && stdout.contains("FIFO-only self-consistent replay"),
        "v1 replay must carry its limitation label:\n{stdout}"
    );

    // Legacy v1 does not bind/recompute its finding id. Even a reproducible
    // attacker-authored bundle must not inject terminal controls or extra
    // output lines through that untrusted label.
    let mut hostile = v1;
    hostile.finding_id = "hostile\n\u{1b}[31m-label".into();
    let hostile_path = tmp.join("v1-hostile-label.ndjson");
    std::fs::write(&hostile_path, hostile.to_ndjson()).unwrap();
    let (code, stdout, stderr) = vh(&["replay-bundle", hostile_path.to_str().unwrap()]);
    assert_eq!(code, 0, "{stdout}{stderr}");
    assert!(!stdout.contains('\u{1b}') && !stderr.contains('\u{1b}'));
    assert_eq!(
        stdout.lines().count(),
        1,
        "untrusted label split output: {stdout:?}"
    );
    assert!(stdout.contains(r"hostile\n\u{1b}[31m-label"), "{stdout:?}");
    let _ = std::fs::remove_dir_all(&tmp);
}

// ---- cooperative hardening (PR #57 / issue #61 findings 5, 6, 9) ----

/// Item 6: an existing regular file as --out is refused BEFORE any
/// cassette load or child launch, and its bytes are preserved.
#[test]
fn cooperative_out_regular_file_is_refused_and_preserved() {
    let tmp = unique_tmp("coop-out-file");
    let out = tmp.join("out");
    std::fs::write(&out, b"precious").unwrap();
    let (code, stdout, stderr) = vh(&["cooperative", "--out", out.to_str().unwrap()]);
    assert_eq!(code, 2, "existing file --out must be refused:\n{stderr}");
    assert!(
        stdout.is_empty(),
        "refusal must happen before any child execution:\n{stdout}"
    );
    assert_eq!(std::fs::read(&out).unwrap(), b"precious");
}

/// Item 6: a symlink --out is refused before anything executes.
#[test]
#[cfg(unix)]
fn cooperative_out_symlink_is_refused() {
    let tmp = unique_tmp("coop-out-symlink");
    let target = tmp.join("target");
    std::fs::create_dir_all(&target).unwrap();
    let link = tmp.join("link");
    std::os::unix::fs::symlink(&target, &link).unwrap();
    let (code, stdout, stderr) = vh(&["cooperative", "--out", link.to_str().unwrap()]);
    assert_eq!(code, 2, "symlink --out must be refused:\n{stderr}");
    assert!(
        stdout.is_empty(),
        "refusal must happen before any child execution:\n{stdout}"
    );
    assert!(!target.join("outcome.ndjson").exists());
}

#[test]
#[cfg(unix)]
fn cooperative_out_parent_symlink_is_refused() {
    let tmp = unique_tmp("coop-out-parent-symlink");
    let real_parent = tmp.join("real-parent");
    std::fs::create_dir(&real_parent).unwrap();
    let linked_parent = tmp.join("linked-parent");
    std::os::unix::fs::symlink(&real_parent, &linked_parent).unwrap();
    let out = linked_parent.join("out");
    let (code, stdout, stderr) = vh(&["cooperative", "--out", out.to_str().unwrap()]);
    assert_eq!(code, 2, "parent symlink must be refused:\n{stderr}");
    assert!(stdout.is_empty());
    assert!(!real_parent.join("out").exists());
}

/// Item 6: a non-empty --out directory is refused before execution and
/// every pre-existing byte is preserved.
#[test]
fn cooperative_out_non_empty_dir_is_refused_and_preserved() {
    let tmp = unique_tmp("coop-out-nonempty");
    let out = tmp.join("out");
    std::fs::create_dir_all(&out).unwrap();
    std::fs::write(out.join("keep.txt"), b"keep").unwrap();
    let (code, stdout, stderr) = vh(&["cooperative", "--out", out.to_str().unwrap()]);
    assert_eq!(code, 2, "non-empty --out must be refused:\n{stderr}");
    assert!(
        stdout.is_empty(),
        "refusal must happen before any child execution:\n{stdout}"
    );
    assert_eq!(std::fs::read(out.join("keep.txt")).unwrap(), b"keep");
    assert!(!out.join("outcome.ndjson").exists());
}

#[test]
#[cfg(unix)]
fn cooperative_out_group_or_other_writable_directory_is_refused() {
    use std::os::unix::fs::PermissionsExt;

    let tmp = unique_tmp("coop-out-shared-mode");
    let out = tmp.join("out");
    std::fs::create_dir_all(&out).unwrap();
    std::fs::set_permissions(&out, std::fs::Permissions::from_mode(0o777)).unwrap();

    let (code, stdout, stderr) = vh(&["cooperative", "--out", out.to_str().unwrap()]);
    assert_eq!(code, 2, "shared --out must be refused: {stderr}");
    assert!(
        stdout.is_empty(),
        "refusal must precede execution: {stdout}"
    );
    assert!(stderr.contains("group/other-writable"), "{stderr}");
    assert_eq!(std::fs::read_dir(&out).unwrap().count(), 0);
}

#[cfg(unix)]
#[test]
fn cooperative_out_group_or_other_writable_non_sticky_parent_is_refused() {
    use std::os::unix::fs::PermissionsExt;

    let shared = unique_tmp("coop-shared-parent");
    std::fs::set_permissions(&shared, std::fs::Permissions::from_mode(0o777)).unwrap();
    let out = shared.join("out");
    let (code, stdout, stderr) = vh(&["cooperative", "--out", out.to_str().unwrap()]);
    std::fs::set_permissions(&shared, std::fs::Permissions::from_mode(0o700)).unwrap();
    std::fs::remove_dir(&shared).unwrap();
    assert_eq!(code, 2);
    assert!(
        stdout.is_empty(),
        "refusal must precede execution: {stdout}"
    );
    assert!(
        stderr.contains("group/other-writable non-sticky parent"),
        "{stderr}"
    );
    assert!(
        !out.exists(),
        "refused parent must not receive an output root"
    );
}

#[cfg(unix)]
#[test]
fn cooperative_out_non_root_owned_shared_sticky_parent_is_refused() {
    use std::os::unix::fs::{MetadataExt, PermissionsExt};

    let shared = unique_tmp("coop-shared-sticky-parent");
    if std::fs::metadata(&shared).unwrap().uid() == 0 {
        // Root-owned sticky parents are the explicitly trusted Unix case.
        return;
    }
    std::fs::set_permissions(&shared, std::fs::Permissions::from_mode(0o1777)).unwrap();
    let out = shared.join("out");
    let (code, stdout, stderr) = vh(&["cooperative", "--out", out.to_str().unwrap()]);
    std::fs::set_permissions(&shared, std::fs::Permissions::from_mode(0o700)).unwrap();
    std::fs::remove_dir(&shared).unwrap();
    assert_eq!(code, 2);
    assert!(
        stdout.is_empty(),
        "refusal must precede execution: {stdout}"
    );
    assert!(
        stderr.contains("non-root-owned shared sticky parent"),
        "{stderr}"
    );
    assert!(
        !out.exists(),
        "refused parent must not receive an output root"
    );
}

#[cfg(unix)]
#[test]
fn cooperative_unsafe_ambient_temp_root_is_refused_without_execution_or_residue() {
    use std::os::unix::fs::{MetadataExt, PermissionsExt};

    let shared = unique_tmp("coop-unsafe-ambient-temp");
    let (receipt_code, receipt) = run_cooperative_receipt("coop-safe-before-unsafe-temp", None);
    assert_eq!(receipt_code, 0);
    let mut modes = vec![0o777];
    if std::fs::metadata(&shared).unwrap().uid() != 0 {
        modes.push(0o1777);
    }
    for mode in modes {
        std::fs::set_permissions(&shared, std::fs::Permissions::from_mode(mode)).unwrap();
        let output = Command::new(env!("CARGO_BIN_EXE_vh"))
            .arg("cooperative")
            .env("TMPDIR", &shared)
            .output()
            .unwrap();
        assert_eq!(output.status.code(), Some(2), "mode={mode:o}");
        assert!(
            output.stdout.is_empty(),
            "mode={mode:o}: child output escaped"
        );
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(
            stderr.contains("ambient temp root owner chain is unsafe"),
            "{stderr}"
        );
        assert_eq!(
            std::fs::read_dir(&shared).unwrap().count(),
            0,
            "mode={mode:o}: refusal left temp-path residue"
        );

        let verification = Command::new(env!("CARGO_BIN_EXE_vh"))
            .args([
                "verify-cooperative",
                "--receipt",
                receipt.to_str().unwrap(),
                "--expected-workload",
                "cooperative-echo",
                "--expect-default-cassette",
            ])
            .env("TMPDIR", &shared)
            .output()
            .unwrap();
        assert_ne!(verification.status.code(), Some(0), "mode={mode:o}");
        let verify_stdout = String::from_utf8_lossy(&verification.stdout);
        assert!(
            verify_stdout.contains("\"verified\":false"),
            "{verify_stdout}"
        );
        assert_eq!(
            std::fs::read_dir(&shared).unwrap().count(),
            0,
            "mode={mode:o}: reverify left temp-path residue"
        );
    }
    std::fs::set_permissions(&shared, std::fs::Permissions::from_mode(0o700)).unwrap();
}

/// Item 6 ordering: --out is validated BEFORE the cassette is read.
#[test]
fn cooperative_out_refusal_precedes_cassette_load() {
    let tmp = unique_tmp("coop-out-order");
    let out = tmp.join("out");
    std::fs::create_dir_all(&out).unwrap();
    std::fs::write(out.join("keep.txt"), b"keep").unwrap();
    let missing = tmp.join("missing.vhc");
    let (code, _, stderr) = vh(&[
        "cooperative",
        "--cassette",
        missing.to_str().unwrap(),
        "--out",
        out.to_str().unwrap(),
    ]);
    assert_eq!(code, 2);
    assert!(
        stderr.contains("not empty"),
        "output refusal must precede cassette load:\n{stderr}"
    );
}

/// Item 5: an oversized cassette (published maximum + 1, sparse logical
/// size) is rejected from its size before any parsing or allocation.
#[test]
fn cooperative_oversized_cassette_rejected_before_parsing() {
    let tmp = unique_tmp("coop-cassette-oversize");
    let cassette = tmp.join("big.vhc");
    let f = std::fs::File::create(&cassette).unwrap();
    f.set_len(1_048_577).unwrap(); // published 1 MiB maximum + 1
    drop(f);
    let (code, _, stderr) = vh(&["cooperative", "--cassette", cassette.to_str().unwrap()]);
    assert_eq!(code, 2);
    assert!(
        stderr.contains("exceeds"),
        "oversize must be a typed size refusal, not a parse error:\n{stderr}"
    );
}

#[test]
fn cooperative_noncanonical_cassette_encoding_is_refused() {
    let tmp = unique_tmp("coop-cassette-noncanonical");
    let cassette = timeout_cassette_file(&tmp);
    let canonical = std::fs::read(&cassette).unwrap();
    let noncanonical = String::from_utf8(canonical).unwrap().replacen(
        "vh-cassette-v2 1\n",
        "vh-cassette-v2 01\n",
        1,
    );
    std::fs::write(&cassette, noncanonical).unwrap();
    let (code, stdout, stderr) = vh(&["cooperative", "--cassette", cassette.to_str().unwrap()]);
    assert_eq!(code, 2);
    assert!(stdout.is_empty());
    assert!(stderr.contains("noncanonical-encoding"), "{stderr}");
}

#[cfg(unix)]
#[test]
fn cooperative_cassette_special_files_are_refused_without_hanging() {
    let (code, stdout, stderr) = vh(&["cooperative", "--cassette", "/dev/null"]);
    assert_eq!(code, 2);
    assert!(stdout.is_empty());
    assert!(stderr.contains("non-regular-file"), "{stderr}");

    let tmp = unique_tmp("coop-cassette-fifo");
    let fifo = tmp.join("cassette.fifo");
    let mkfifo = Command::new("/usr/bin/mkfifo")
        .arg(&fifo)
        .status()
        .expect("create FIFO fixture");
    assert!(mkfifo.success());
    let timeout = ["/usr/bin/timeout", "/opt/homebrew/bin/gtimeout"]
        .into_iter()
        .find(|candidate| std::path::Path::new(candidate).is_file());
    if let Some(timeout) = timeout {
        let output = Command::new(timeout)
            .args([
                "2",
                env!("CARGO_BIN_EXE_vh"),
                "cooperative",
                "--cassette",
                fifo.to_str().unwrap(),
            ])
            .output()
            .expect("run FIFO refusal under an independent deadline");
        assert_eq!(
            output.status.code(),
            Some(2),
            "FIFO must be typed refusal, not timeout 124:\n{}",
            String::from_utf8_lossy(&output.stderr)
        );
    }
}

/// Item 9: a malformed cassette carrying attacker content exposes only a
/// stable bounded category — never the attacker's bytes.
#[test]
fn cooperative_malformed_cassette_redacts_attacker_content() {
    let tmp = unique_tmp("coop-cassette-redact");
    let cassette = tmp.join("bad.vhc");
    let sentinel = "S3CR3T-SENTINEL-PR57";
    std::fs::write(&cassette, format!("{sentinel} garbage head\n")).unwrap();
    let (code, stdout, stderr) = vh(&["cooperative", "--cassette", cassette.to_str().unwrap()]);
    assert_eq!(code, 2);
    assert!(
        !stderr.contains(sentinel),
        "attacker content leaked to stderr:\n{stderr}"
    );
    assert!(
        !stdout.contains(sentinel),
        "attacker content leaked to stdout:\n{stdout}"
    );
    assert!(
        stderr.len() <= 320,
        "diagnostic must stay bounded:\n{stderr}"
    );
}

// ---- item 2: invocation-isolated cooperative workspaces ----

fn field_value(line: &str, key: &str) -> String {
    let needle = format!("\"{key}\":\"");
    let start = line
        .find(&needle)
        .unwrap_or_else(|| panic!("missing {key}: {line}"))
        + needle.len();
    let rest = &line[start..];
    let end = rest
        .find('"')
        .unwrap_or_else(|| panic!("unterminated {key}: {line}"));
    rest[..end].to_string()
}

fn gated_cooperative(
    ready: &std::path::Path,
    gate: &std::path::Path,
    out: &std::path::Path,
) -> std::process::Child {
    Command::new("/bin/sh")
        .args([
            "-c",
            "touch \"$1\"; while [ ! -e \"$2\" ]; do sleep 0.01; done; exec \"$3\" cooperative --out \"$4\"",
            "vh-gate",
            ready.to_str().unwrap(),
            gate.to_str().unwrap(),
            env!("CARGO_BIN_EXE_vh"),
            out.to_str().unwrap(),
        ])
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .expect("spawn gated cooperative")
}

fn release_when_both_ready(
    ready_a: &std::path::Path,
    ready_b: &std::path::Path,
    gate: &std::path::Path,
) {
    let status = Command::new("/bin/sh")
        .args([
            "-c",
            "i=0; while [ $i -lt 500 ]; do [ -e \"$1\" ] && [ -e \"$2\" ] && exit 0; i=$((i+1)); sleep 0.01; done; exit 1",
            "vh-ready",
            ready_a.to_str().unwrap(),
            ready_b.to_str().unwrap(),
        ])
        .status()
        .expect("wait for both gated children");
    assert!(
        status.success(),
        "both children must reach the admission barrier"
    );
    std::fs::write(gate, b"go").unwrap();
}

/// Two simultaneous invocations with the same (builtin) cassette and
/// different output roots must BOTH succeed, with identical deterministic
/// evidence identity — no shared root may bind or race them together.
#[test]
fn cooperative_concurrent_invocations_isolated_with_identical_identity() {
    let tmp = unique_tmp("coop-iso");
    let out_a = tmp.join("A");
    let out_b = tmp.join("B");
    let gate = tmp.join("go");
    let ready_a = tmp.join("ready-a");
    let ready_b = tmp.join("ready-b");
    let p1 = gated_cooperative(&ready_a, &gate, &out_a);
    let p2 = gated_cooperative(&ready_b, &gate, &out_b);
    release_when_both_ready(&ready_a, &ready_b, &gate);
    let o1 = p1.wait_with_output().expect("wait first");
    let o2 = p2.wait_with_output().expect("wait second");
    assert_eq!(
        o1.status.code(),
        Some(0),
        "first invocation must succeed:\n{}",
        String::from_utf8_lossy(&o1.stderr)
    );
    assert_eq!(
        o2.status.code(),
        Some(0),
        "second invocation must succeed:\n{}",
        String::from_utf8_lossy(&o2.stderr)
    );
    let line_a = std::fs::read_to_string(out_a.join("outcome.ndjson")).unwrap();
    let line_b = std::fs::read_to_string(out_b.join("outcome.ndjson")).unwrap();
    assert_eq!(
        field_value(&line_a, "evidence_digest"),
        field_value(&line_b, "evidence_digest"),
        "evidence identity must not bind the invocation's unique staging path"
    );
    assert_eq!(
        field_value(&line_a, "result_digest"),
        field_value(&line_b, "result_digest"),
    );
}

/// Competing use of the SAME output root admits at most one invocation;
/// the loser is refused (exit 2) and no caller's data is deleted.
#[test]
fn cooperative_competing_output_root_admits_at_most_one() {
    let tmp = unique_tmp("coop-compete");
    let out = tmp.join("O");
    std::fs::create_dir_all(&out).unwrap();
    let sentinel = tmp.join("sentinel");
    std::fs::write(&sentinel, b"preserve").unwrap();
    let gate = tmp.join("go");
    let ready_a = tmp.join("ready-a");
    let ready_b = tmp.join("ready-b");
    let p1 = gated_cooperative(&ready_a, &gate, &out);
    let p2 = gated_cooperative(&ready_b, &gate, &out);
    release_when_both_ready(&ready_a, &ready_b, &gate);
    let o1 = p1.wait_with_output().expect("wait first");
    let o2 = p2.wait_with_output().expect("wait second");
    let codes = [o1.status.code(), o2.status.code()];
    let wins = codes.iter().filter(|c| **c == Some(0)).count();
    assert_eq!(
        wins, 1,
        "exactly one invocation may win an output root: {codes:?}"
    );
    for o in [&o1, &o2] {
        if o.status.code() != Some(0) {
            assert_eq!(
                o.status.code(),
                Some(2),
                "the loser must be a typed refusal:\n{}",
                String::from_utf8_lossy(&o.stderr)
            );
        }
    }
    assert!(
        out.join("outcome.ndjson").exists(),
        "the winner's receipt must survive the loser's refusal"
    );
    assert_eq!(std::fs::read(&sentinel).unwrap(), b"preserve");
}

// ---- item 3: deterministic child-failure semantics (CLI level) ----

fn timeout_cassette_file(dir: &std::path::Path) -> std::path::PathBuf {
    let mut cassette = vh_sandbox::CassetteV2::default();
    cassette.push(
        vh_sandbox::LlmRequestV2 {
            provider: "fixture".into(),
            model: "cooperative-echo".into(),
            messages: vec![("user".into(), "hello".into())],
            tools: Vec::new(),
            tool_choice: None,
            structured_output: None,
            params: std::collections::BTreeMap::from([("temperature".into(), "0".into())]),
        },
        vh_sandbox::TapeEntry::Timeout,
    );
    let path = dir.join("timeout.vhc");
    std::fs::write(&path, cassette.file_bytes()).unwrap();
    path
}

/// A fully consumed, untainted, identically reproduced cassette Timeout
/// is FINDINGS only via the declared oracle: exit 1, verified=true,
/// findings_count=1, exact stable finding identity.
#[test]
fn cooperative_cassette_timeout_is_verified_finding_with_stable_identity() {
    let tmp = unique_tmp("coop-timeout");
    let cassette = timeout_cassette_file(&tmp);
    let (code, stdout, _) = vh(&["cooperative", "--cassette", cassette.to_str().unwrap()]);
    assert_eq!(code, 1, "timeout oracle finding must exit 1:\n{stdout}");
    let line = stdout.lines().last().expect("outcome line");
    assert_eq!(field_value(line, "verdict"), "FINDINGS");
    assert!(line.contains("\"verified\":true"), "{line}");
    assert!(line.contains("\"findings_count\":1"), "{line}");
    assert_eq!(
        field_value(line, "oracle"),
        "cooperative-llm-call-completed"
    );
    assert_eq!(
        field_value(line, "finding_identity"),
        "cooperative-llm-call-completed:timeout"
    );
}

// ---- item 4: persisted cooperative receipt + strict reverification ----

fn run_cooperative_receipt(
    label: &str,
    cassette: Option<&std::path::Path>,
) -> (i32, std::path::PathBuf) {
    let tmp = unique_tmp(label);
    let out = tmp.join("O");
    let mut args = vec!["cooperative", "--out", out.to_str().unwrap()];
    let cassette_str;
    if let Some(c) = cassette {
        cassette_str = c.to_str().unwrap().to_string();
        args.push("--cassette");
        args.push(&cassette_str);
    }
    let (code, stdout, stderr) = vh(&args);
    let receipt = out.join("cooperative.receipt");
    if code == 0 || code == 1 {
        assert!(
            receipt.exists(),
            "receipt must be persisted:\n{stdout}\n{stderr}"
        );
    }
    (code, receipt)
}

fn verify_receipt(path: &std::path::Path) -> (i32, String, String) {
    vh(&["verify-cooperative", "--receipt", path.to_str().unwrap()])
}

/// Rewrite the digest line so a tampered body is internally consistent.
fn redigest(body_with_digest: &[u8]) -> Vec<u8> {
    let text = body_with_digest;
    let marker = b"digest sha256:";
    let pos = text
        .windows(marker.len())
        .rposition(|w| w == marker)
        .expect("digest line");
    let body = &text[..pos];
    let mut out = body.to_vec();
    out.extend_from_slice(marker);
    out.extend_from_slice(vh_digest::sha256_hex(body).as_bytes());
    out.push(b'\n');
    out
}

fn framed_payload_range(bytes: &[u8], tag: &str) -> std::ops::Range<usize> {
    let marker = format!("{tag} ");
    let field = bytes
        .windows(marker.len())
        .enumerate()
        .find(|(index, window)| {
            *window == marker.as_bytes() && (*index == 0 || bytes[*index - 1] == b'\n')
        })
        .map(|(index, _)| index)
        .unwrap_or_else(|| panic!("missing framed field {tag}"));
    let length_start = field + marker.len();
    let colon = bytes[length_start..]
        .iter()
        .position(|byte| *byte == b':')
        .map(|offset| length_start + offset)
        .expect("framed colon");
    let length: usize = std::str::from_utf8(&bytes[length_start..colon])
        .unwrap()
        .parse()
        .unwrap();
    colon + 1..colon + 1 + length
}

fn replace_framed_payload(bytes: &[u8], tag: &str, replacement: &[u8]) -> Vec<u8> {
    let range = framed_payload_range(bytes, tag);
    let marker = format!("{tag} ");
    let field_start = bytes[..range.start]
        .windows(marker.len())
        .rposition(|window| window == marker.as_bytes())
        .expect("framed field start");
    let mut out = bytes[..field_start].to_vec();
    out.extend_from_slice(format!("{tag} {}:", replacement.len()).as_bytes());
    out.extend_from_slice(replacement);
    out.extend_from_slice(&bytes[range.end..]);
    out
}

fn receipt_line_value<'a>(text: &'a str, tag: &str) -> &'a str {
    text.lines()
        .find_map(|line| line.strip_prefix(&format!("{tag} ")))
        .unwrap_or_else(|| panic!("missing receipt line {tag}"))
}

fn cooperative_engine_request_digest_for_test(
    workload: &str,
    cassette_identity: &str,
    child_source_digest: &str,
) -> String {
    let mut bytes = b"vh-cooperative-engine-request-v1\n".to_vec();
    for (tag, value) in [
        ("workload", workload),
        ("cassette-identity", cassette_identity),
        ("child-source-digest", child_source_digest),
    ] {
        bytes.extend_from_slice(format!("{tag} {}:{value}\n", value.len()).as_bytes());
    }
    vh_digest::sha256_hex(&bytes)
}

/// Replace the first line starting with `tag ` (or a framed field
/// `tag <len>:`) with `new_line` (without trailing newline), then
/// redigest so the tamper is internally consistent.
fn replace_line_and_redigest(original: &[u8], tag: &str, new_line: &str) -> Vec<u8> {
    let mut lines: Vec<&[u8]> = original.split(|b| *b == b'\n').collect();
    // split leaves a trailing empty slice after the final newline
    if lines.last() == Some(&b"".as_slice()) {
        lines.pop();
    }
    let prefix = format!("{tag} ");
    let mut replaced = false;
    let mut out: Vec<u8> = Vec::new();
    for line in &lines {
        if !replaced && line.starts_with(prefix.as_bytes()) {
            out.extend_from_slice(new_line.as_bytes());
            replaced = true;
        } else {
            out.extend_from_slice(line);
        }
        out.push(b'\n');
    }
    assert!(replaced, "tag {tag} not found");
    redigest(&out)
}

#[test]
fn cooperative_receipt_clean_reverifies() {
    let (code, receipt) = run_cooperative_receipt("coop-rcpt-clean", None);
    assert_eq!(code, 0);
    let (vcode, vout, verr) = verify_receipt(&receipt);
    assert_eq!(vcode, 0, "clean receipt must reverify:\n{vout}\n{verr}");
    let line = vout.lines().last().expect("verify record");
    assert_eq!(field_value(line, "record"), "cooperative-verify");
    assert!(line.contains("\"verified\":true"), "{line}");
    assert!(line.contains("\"authentic\":true"), "{line}");
    assert_eq!(field_value(line, "verdict"), "CLEAN");
    assert!(line.contains("\"outcome_exit_code\":0"), "{line}");
    assert!(line.contains("\"exit_code\":0"), "{line}");
    assert_eq!(field_value(line, "workload"), "cooperative-echo");
    assert_eq!(field_value(line, "engine_request_digest").len(), 64);
    let file_bytes = std::fs::read(&receipt).unwrap();
    assert_eq!(
        field_value(line, "receipt_sha256"),
        vh_digest::sha256_hex(&file_bytes)
    );
    let engine_bytes = std::fs::read(env!("CARGO_BIN_EXE_vh")).unwrap();
    assert_eq!(
        field_value(line, "engine_sha256"),
        vh_digest::sha256_hex(&engine_bytes)
    );
}

#[test]
fn cooperative_receipt_timeout_finding_reverifies() {
    let tmp = unique_tmp("coop-rcpt-timeout-cassette");
    let cassette = timeout_cassette_file(&tmp);
    let (code, receipt) = run_cooperative_receipt("coop-rcpt-timeout", Some(&cassette));
    assert_eq!(code, 1, "timeout finding exits 1");
    let (vcode, vout, verr) = verify_receipt(&receipt);
    assert_eq!(vcode, 0, "finding receipt must reverify:\n{vout}\n{verr}");
    let line = vout.lines().last().unwrap();
    assert!(line.contains("\"verified\":true"), "{line}");
    assert_eq!(field_value(line, "verdict"), "FINDINGS");
    assert!(line.contains("\"findings_count\":1"), "{line}");
    assert!(line.contains("\"outcome_exit_code\":1"), "{line}");
    assert!(
        line.contains("\"exit_code\":0"),
        "verifier authenticity status stays distinct from outcome status: {line}"
    );
    assert_eq!(
        field_value(line, "finding_identity"),
        "cooperative-llm-call-completed:timeout"
    );
}

#[test]
fn cooperative_receipt_expected_request_binding_rejects_substitution() {
    let (code, receipt) = run_cooperative_receipt("coop-rcpt-context", None);
    assert_eq!(code, 0);
    let tmp = unique_tmp("coop-rcpt-context-timeout");
    let timeout = timeout_cassette_file(&tmp);
    let (vcode, stdout, stderr) = vh(&[
        "verify-cooperative",
        "--receipt",
        receipt.to_str().unwrap(),
        "--expected-workload",
        "cooperative-echo",
        "--expected-cassette",
        timeout.to_str().unwrap(),
    ]);
    assert_eq!(
        vcode, 1,
        "substituted context must fail:\n{stdout}\n{stderr}"
    );
    assert!(stdout.contains("expected-cassette-mismatch"), "{stdout}");

    let (vcode, stdout, stderr) = vh(&[
        "verify-cooperative",
        "--receipt",
        receipt.to_str().unwrap(),
        "--expected-workload",
        "cooperative-echo",
        "--expect-default-cassette",
    ]);
    assert_eq!(vcode, 0, "matching context must pass:\n{stdout}\n{stderr}");
}

// ---- issue #90: negotiated cooperative-v2 red contract ----

const ISSUE90_MANIFEST_SCHEMA: &str = "vh-protocol-manifest-v1";
const ISSUE90_OPERATION: &str = "cooperative-target-v1";
const ISSUE90_FEATURES: [&str; 3] = [
    "cooperative-cassette-v2",
    "fresh-replay-v1",
    "observed-child-source-sha256-v1",
];
const ISSUE90_CHILD_SHA256: &str =
    "abbbaf8284752607e8a80324c87e39302848c4fca50a5ad034ca40562a38d60a";

fn issue90_v2_args(out: &std::path::Path) -> Vec<String> {
    let (manifest_code, manifest, manifest_error) = vh(&["protocol-manifest"]);
    assert_eq!(
        manifest_code, 0,
        "same-engine manifest query failed:\n{manifest}\n{manifest_error}"
    );
    let manifest_id = framed_record_value(&manifest, "manifest-id");
    let mut args = vec![
        "cooperative-v2".to_string(),
        "--protocol-schema".to_string(),
        ISSUE90_MANIFEST_SCHEMA.to_string(),
        "--manifest-id".to_string(),
        manifest_id,
        "--operation".to_string(),
        ISSUE90_OPERATION.to_string(),
    ];
    for feature in ISSUE90_FEATURES {
        args.push("--require-feature".to_string());
        args.push(feature.to_string());
    }
    args.extend([
        "--requested-target-revision".to_string(),
        format!("sha256:{ISSUE90_CHILD_SHA256}"),
        "--out".to_string(),
        out.display().to_string(),
    ]);
    args
}

fn issue90_verify_v2_args(receipt: &std::path::Path) -> Vec<String> {
    let (manifest_code, manifest, manifest_error) = vh(&["protocol-manifest"]);
    assert_eq!(
        manifest_code, 0,
        "same-engine manifest query failed:\n{manifest}\n{manifest_error}"
    );
    let manifest_id = framed_record_value(&manifest, "manifest-id");
    let mut args = vec![
        "verify-cooperative-v2".to_string(),
        "--receipt".to_string(),
        receipt.display().to_string(),
        "--expected-operation".to_string(),
        ISSUE90_OPERATION.to_string(),
    ];
    for feature in ISSUE90_FEATURES {
        args.push("--expected-feature".to_string());
        args.push(feature.to_string());
    }
    args.extend([
        "--expected-requested-target-revision".to_string(),
        format!("sha256:{ISSUE90_CHILD_SHA256}"),
        "--expected-protocol-schema".to_string(),
        ISSUE90_MANIFEST_SCHEMA.to_string(),
        "--expected-manifest-id".to_string(),
        manifest_id,
        "--expect-default-cassette".to_string(),
        "--expected-request-schema".to_string(),
        "vh-cooperative-request-v2".to_string(),
        "--expected-outcome-schema".to_string(),
        "vh-cooperative-outcome-v2".to_string(),
        "--expected-receipt-schema".to_string(),
        "vh-cooperative-receipt-v2".to_string(),
        "--expected-verifier-schema".to_string(),
        "vh-cooperative-verify-v2".to_string(),
        "--expected-observation-subject".to_string(),
        "cooperative-child-source-v1".to_string(),
        "--expected-revision-algorithm".to_string(),
        "sha256".to_string(),
        "--expected-revision-policy".to_string(),
        "bound-required".to_string(),
        "--expected-execution-binding".to_string(),
        "staged-d2".to_string(),
        "--expected-observation-to-exec-channel".to_string(),
        "open".to_string(),
    ]);
    args
}

fn issue90_v2_receipt(label: &str) -> std::path::PathBuf {
    let out = unique_tmp(label).join("O");
    let args = issue90_v2_args(&out);
    let (code, stdout, stderr) = vh_owned(&args);
    assert_eq!(code, 0, "v2 fixture failed:\n{stdout}\n{stderr}");
    let receipt = out.join("cooperative.receipt");
    assert!(receipt.is_file(), "v2 fixture did not publish a receipt");
    receipt
}

fn assert_issue90_verify_failure(receipt: &std::path::Path) {
    let args = issue90_verify_v2_args(receipt);
    let (code, stdout, stderr) = vh_owned(&args);
    assert_eq!(code, 1, "tamper must fail:\n{stdout}\n{stderr}");
    assert!(
        stderr.is_empty(),
        "machine failure must not depend on stderr: {stderr}"
    );
    assert!(
        stdout.starts_with("vh-cooperative-verify-failure-v1\n"),
        "tamper must emit the closed verification-failure record:\n{stdout}"
    );
    assert!(stdout.contains("executions 0\n"), "{stdout}");
    assert!(stdout.contains("authentic false\n"), "{stdout}");
    assert!(stdout.contains("verified false\n"), "{stdout}");
    assert!(!stdout.contains("CLEAN"), "{stdout}");
}

fn framed_record_value(record: &str, tag: &str) -> String {
    let prefix = format!("{tag} ");
    let line = record
        .lines()
        .find(|line| line.starts_with(&prefix))
        .unwrap_or_else(|| panic!("missing {tag} in record:\n{record}"));
    let framed = &line[prefix.len()..];
    let (length, value) = framed
        .split_once(':')
        .unwrap_or_else(|| panic!("malformed {tag} frame: {line}"));
    let expected: usize = length.parse().expect("canonical frame length");
    assert_eq!(value.len(), expected, "wrong {tag} frame length");
    value.to_string()
}

fn vh_owned(args: &[String]) -> (i32, String, String) {
    let borrowed: Vec<&str> = args.iter().map(String::as_str).collect();
    vh(&borrowed)
}

fn issue90_set_features(args: &mut Vec<String>, features: &[String]) {
    let start = args
        .iter()
        .position(|argument| argument == "--require-feature")
        .expect("issue #90 request has mandatory features");
    let end = args
        .iter()
        .position(|argument| argument == "--requested-target-revision")
        .expect("issue #90 request has a revision requirement");
    let replacement = features
        .iter()
        .flat_map(|feature| ["--require-feature".to_string(), feature.clone()])
        .collect::<Vec<_>>();
    args.splice(start..end, replacement);
}

fn assert_issue90_preexecution_refusal(
    args: &[String],
    out: &std::path::Path,
    expected_reason: &str,
) {
    let (code, stdout, stderr) = vh_owned(args);
    assert_eq!(
        code, 4,
        "negotiation refusal must use exit 4:\n{stdout}\n{stderr}"
    );
    assert!(
        stdout.starts_with("vh-engine-negotiation-refusal-v1\n"),
        "refusal must be a strict Rust protocol record:\n{stdout}"
    );
    assert_eq!(
        framed_record_value(&stdout, "reason"),
        expected_reason,
        "wrong typed refusal:\n{stdout}"
    );
    assert!(stdout.contains("executions 0\n"), "{stdout}");
    assert!(
        !stdout.contains("vh-cooperative-outcome-v2") && !stdout.contains("CLEAN"),
        "a refusal must not publish a checked outcome:\n{stdout}"
    );
    assert!(
        !out.exists(),
        "negotiation crossed the output/staging boundary before refusal: {}",
        out.display()
    );
    assert!(
        !out.join("cooperative.receipt").exists(),
        "negotiation refusal published a receipt"
    );
}

#[test]
fn negotiated_manifest_is_published_by_the_exact_engine() {
    let (code, stdout, stderr) = vh(&["protocol-manifest"]);
    assert_eq!(code, 0, "manifest query must succeed:\n{stdout}\n{stderr}");
    assert!(stdout.starts_with(ISSUE90_MANIFEST_SCHEMA), "{stdout}");
    assert!(stdout.contains(ISSUE90_OPERATION), "{stdout}");
    for feature in ISSUE90_FEATURES {
        assert!(stdout.contains(feature), "missing {feature}:\n{stdout}");
    }
}

#[test]
fn negotiated_unsupported_operation_is_typed_and_executes_nothing() {
    let out = unique_tmp("issue90-unsupported-operation").join("O");
    let mut args = issue90_v2_args(&out);
    let operation = args.iter().position(|arg| arg == "--operation").unwrap() + 1;
    args[operation] = "unsupported-target-v1".to_string();
    let (code, stdout, stderr) = vh_owned(&args);
    assert_eq!(
        code, 4,
        "engine refusal has a distinct status:\n{stdout}\n{stderr}"
    );
    assert!(
        stdout.starts_with("vh-engine-negotiation-refusal-v1"),
        "{stdout}"
    );
    assert!(stdout.contains("unsupported-operation"), "{stdout}");
    assert!(stdout.contains("executions 0"), "{stdout}");
    assert!(!out.join("cooperative.receipt").exists());
    assert!(!stdout.contains("CLEAN"));
}

#[test]
fn negotiated_requested_revision_mismatch_refuses_before_execution() {
    let out = unique_tmp("issue90-revision-mismatch").join("O");
    let mut args = issue90_v2_args(&out);
    let revision = args
        .iter()
        .position(|arg| arg == "--requested-target-revision")
        .unwrap()
        + 1;
    args[revision] = format!("sha256:{}", "0".repeat(64));
    let (code, stdout, stderr) = vh_owned(&args);
    assert_eq!(
        code, 4,
        "revision mismatch must be an engine refusal:\n{stdout}\n{stderr}"
    );
    assert!(stdout.contains("requested-revision-mismatch"), "{stdout}");
    assert!(stdout.contains("executions 0"), "{stdout}");
    assert!(!out.join("cooperative.receipt").exists());
}

#[test]
fn negotiated_bound_operation_refuses_unknown_revision_before_execution() {
    let out = unique_tmp("issue90-revision-unknown").join("O");
    let mut args = issue90_v2_args(&out);
    let revision = args
        .iter()
        .position(|argument| argument == "--requested-target-revision")
        .unwrap()
        + 1;
    args[revision] = "unknown".to_string();
    assert_issue90_preexecution_refusal(&args, &out, "requested-revision-mismatch");
}

#[test]
fn negotiated_v2_refuses_occupied_output_roots_before_sandbox_attempt() {
    for occupied_name in ["marker", "cooperative.receipt"] {
        let out = unique_tmp(&format!("issue90-v2-occupied-{occupied_name}")).join("out");
        std::fs::create_dir(&out).unwrap();
        let occupied = out.join(occupied_name);
        std::fs::write(&occupied, b"caller-owned").unwrap();
        let args = issue90_v2_args(&out);

        let (code, stdout, stderr) = vh_owned(&args);

        assert_eq!(
            code, 2,
            "occupied root must fail locally: {stdout}\n{stderr}"
        );
        assert!(
            stdout.is_empty(),
            "no machine success/refusal record: {stdout}"
        );
        assert!(
            stderr.contains("not empty"),
            "typed local boundary error: {stderr}"
        );
        assert_eq!(std::fs::read(&occupied).unwrap(), b"caller-owned");
        assert!(!out.join("workspace").exists());
    }
}

#[test]
fn negotiated_feature_sets_fail_closed_before_child_execution() {
    let cases = [
        ("malformed", vec!["fresh_replay-v1".to_string()]),
        (
            "duplicate",
            vec![
                ISSUE90_FEATURES[0].to_string(),
                ISSUE90_FEATURES[0].to_string(),
                ISSUE90_FEATURES[1].to_string(),
                ISSUE90_FEATURES[2].to_string(),
            ],
        ),
        (
            "unsorted",
            ISSUE90_FEATURES
                .iter()
                .rev()
                .map(|feature| feature.to_string())
                .collect(),
        ),
        (
            "oversized",
            (0..17)
                .map(|index| format!("feature-{index:02}-v1"))
                .collect(),
        ),
        ("noncanonical", vec!["Fresh-replay-v1".to_string()]),
    ];

    for (label, features) in cases {
        let out = unique_tmp(&format!("issue90-features-{label}")).join("O");
        let mut args = issue90_v2_args(&out);
        issue90_set_features(&mut args, &features);
        assert_issue90_preexecution_refusal(&args, &out, "invalid-feature-set");
    }
}

#[test]
fn negotiated_stale_manifest_and_mutated_closure_are_revalidated_at_execution() {
    let stale_out = unique_tmp("issue90-stale-manifest").join("O");
    let mut stale_args = issue90_v2_args(&stale_out);
    let manifest_index = stale_args
        .iter()
        .position(|argument| argument == "--manifest-id")
        .unwrap()
        + 1;
    let mut stale_id = stale_args[manifest_index].clone().into_bytes();
    stale_id[0] = if stale_id[0] == b'0' { b'1' } else { b'0' };
    stale_args[manifest_index] = String::from_utf8(stale_id).unwrap();
    assert_issue90_preexecution_refusal(&stale_args, &stale_out, "protocol-manifest-mismatch");

    let mutated_out = unique_tmp("issue90-mutated-feature-closure").join("O");
    let mut mutated_args = issue90_v2_args(&mutated_out);
    let mut mutated_features = ISSUE90_FEATURES
        .iter()
        .map(|feature| feature.to_string())
        .collect::<Vec<_>>();
    mutated_features.push("unsupported-negotiated-feature-v1".to_string());
    mutated_features.sort();
    issue90_set_features(&mut mutated_args, &mutated_features);
    assert_issue90_preexecution_refusal(&mutated_args, &mutated_out, "unsupported-feature");
}

#[test]
fn negotiated_verifier_rejects_legacy_receipt_before_replay() {
    let (code, receipt) = run_cooperative_receipt("issue90-v1-is-legacy", None);
    assert_eq!(code, 0);
    let (vcode, stdout, stderr) = vh(&[
        "verify-cooperative-v2",
        "--receipt",
        receipt.to_str().unwrap(),
        "--expected-operation",
        ISSUE90_OPERATION,
    ]);
    assert_eq!(
        vcode, 4,
        "v1 must not acquire v2 meaning:\n{stdout}\n{stderr}"
    );
    assert!(stdout.contains("unsupported-receipt-schema"), "{stdout}");
    assert!(stdout.contains("executions 0"), "{stdout}");
}

#[test]
fn negotiated_v2_receipt_structural_tamper_is_public_typed_and_zero_execution() {
    let seed_receipt = issue90_v2_receipt("issue90-v2-structural-seed");
    let original = std::fs::read(&seed_receipt).unwrap();
    let text = String::from_utf8(original.clone()).unwrap();
    let protocol_line = text
        .lines()
        .find(|line| line.starts_with("protocol-schema "))
        .unwrap();
    let manifest_line = text
        .lines()
        .find(|line| line.starts_with("manifest-id "))
        .unwrap();
    let claim_line = text
        .lines()
        .find(|line| line.starts_with("claimed-observed-revision "))
        .unwrap();
    let mutations = [
        (
            "missing",
            text.replacen(&format!("{claim_line}\n"), "", 1)
                .into_bytes(),
        ),
        (
            "duplicate",
            text.replacen(
                &format!("{protocol_line}\n"),
                &format!("{protocol_line}\n{protocol_line}\n"),
                1,
            )
            .into_bytes(),
        ),
        (
            "unknown",
            text.replacen(
                "vh-cooperative-receipt-v2\n",
                "vh-cooperative-receipt-v2\nunknown 1:x\n",
                1,
            )
            .into_bytes(),
        ),
        (
            "reordered",
            text.replacen(
                &format!("{protocol_line}\n{manifest_line}\n"),
                &format!("{manifest_line}\n{protocol_line}\n"),
                1,
            )
            .into_bytes(),
        ),
        ("truncated", original[..original.len() - 1].to_vec()),
        (
            "noncanonical-length",
            text.replacen("protocol-schema 23:", "protocol-schema 023:", 1)
                .into_bytes(),
        ),
        ("trailing", [original.as_slice(), b"trailing\n"].concat()),
        (
            "mutated-claim",
            text.replacen(
                claim_line,
                &format!("claimed-observed-revision 64:{}", "0".repeat(64)),
                1,
            )
            .into_bytes(),
        ),
    ];

    for (label, bytes) in mutations {
        let receipt = issue90_v2_receipt(&format!("issue90-v2-structural-{label}"));
        std::fs::write(&receipt, bytes).unwrap();
        assert_issue90_verify_failure(&receipt);
    }

    let oversized = issue90_v2_receipt("issue90-v2-structural-oversized");
    std::fs::write(&oversized, vec![b'x'; (4 << 20) + 1]).unwrap();
    let args = issue90_verify_v2_args(&oversized);
    let (code, stdout, stderr) = vh_owned(&args);
    assert_eq!(code, 1, "oversized receipt must fail:\n{stdout}\n{stderr}");
    assert!(stderr.is_empty(), "{stderr}");
    assert!(stdout.starts_with("vh-cooperative-verify-failure-v1\n"));
    assert_eq!(framed_record_value(&stdout, "reason"), "malformed-receipt");
    assert_eq!(
        framed_record_value(&stdout, "receipt-sha256"),
        "unavailable",
        "an unread oversized receipt has no observed bounded SHA-256"
    );
    assert!(stdout.contains("executions 0\n"), "{stdout}");
    assert!(stdout.contains("authentic false\n"), "{stdout}");
    assert!(stdout.contains("verified false\n"), "{stdout}");
}

#[test]
fn negotiated_v2_alternate_expected_request_is_public_typed_and_zero_execution() {
    let receipt = issue90_v2_receipt("issue90-v2-alternate-request");
    let mut args = issue90_verify_v2_args(&receipt);
    let revision = args
        .iter()
        .position(|argument| argument == "--expected-requested-target-revision")
        .unwrap()
        + 1;
    args[revision] = format!("sha256:{}", "0".repeat(64));
    let (code, stdout, stderr) = vh_owned(&args);
    assert_eq!(code, 1, "alternate request must fail:\n{stdout}\n{stderr}");
    assert!(stderr.is_empty(), "{stderr}");
    assert!(
        stdout.starts_with("vh-cooperative-verify-failure-v1\n"),
        "{stdout}"
    );
    assert_eq!(
        framed_record_value(&stdout, "reason"),
        "expected-request-mismatch"
    );
    assert!(stdout.contains("executions 0\n"), "{stdout}");
    assert!(stdout.contains("authentic false\n"), "{stdout}");
    assert!(stdout.contains("verified false\n"), "{stdout}");
}

#[test]
fn cooperative_receipt_deletion_fails_closed() {
    let (code, receipt) = run_cooperative_receipt("coop-rcpt-delete", None);
    assert_eq!(code, 0);
    std::fs::remove_file(&receipt).unwrap();
    let (vcode, _, _) = verify_receipt(&receipt);
    assert_eq!(vcode, 2, "missing receipt is a usage-class failure");
}

#[test]
#[cfg(unix)]
fn cooperative_receipt_symlink_rejected() {
    let (code, receipt) = run_cooperative_receipt("coop-rcpt-link", None);
    assert_eq!(code, 0);
    let link_dir = receipt
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .join("link-holder");
    std::fs::create_dir(&link_dir).unwrap();
    let link = link_dir.join("cooperative.receipt");
    std::os::unix::fs::symlink(&receipt, &link).unwrap();
    let (vcode, _, _) = verify_receipt(&link);
    assert_eq!(vcode, 2, "symlink receipt path must be refused");
}

#[test]
#[cfg(unix)]
fn cooperative_receipt_parent_symlink_and_directory_are_rejected() {
    let (code, receipt) = run_cooperative_receipt("coop-rcpt-parent-link", None);
    assert_eq!(code, 0);
    let real_parent = receipt.parent().unwrap();
    let linked_parent = real_parent.parent().unwrap().join("linked-receipt-parent");
    std::os::unix::fs::symlink(real_parent, &linked_parent).unwrap();
    let linked_receipt = linked_parent.join("cooperative.receipt");
    let (vcode, _, _) = verify_receipt(&linked_receipt);
    assert_eq!(vcode, 2, "parent symlink receipt path must be refused");

    let tmp = unique_tmp("coop-rcpt-directory");
    let directory = tmp.join("cooperative.receipt");
    std::fs::create_dir(&directory).unwrap();
    let (vcode, _, stderr) = verify_receipt(&directory);
    assert_eq!(
        vcode, 2,
        "receipt directory is not a regular file: {stderr}"
    );
}

#[cfg(unix)]
#[test]
fn cooperative_receipt_fifo_is_refused_without_hanging() {
    let tmp = unique_tmp("coop-rcpt-fifo");
    let fifo = tmp.join("cooperative.receipt");
    let mkfifo = Command::new("/usr/bin/mkfifo")
        .arg(&fifo)
        .status()
        .expect("create receipt FIFO");
    assert!(mkfifo.success());
    let timeout = ["/usr/bin/timeout", "/opt/homebrew/bin/gtimeout"]
        .into_iter()
        .find(|candidate| std::path::Path::new(candidate).is_file());
    if let Some(timeout) = timeout {
        let output = Command::new(timeout)
            .args([
                "2",
                env!("CARGO_BIN_EXE_vh"),
                "verify-cooperative",
                "--receipt",
                fifo.to_str().unwrap(),
            ])
            .output()
            .expect("run receipt FIFO refusal under deadline");
        assert_eq!(output.status.code(), Some(2));
        assert!(String::from_utf8_lossy(&output.stderr).contains("non-regular-file"));
    }
}

#[test]
fn cooperative_receipt_truncation_fails() {
    let (code, receipt) = run_cooperative_receipt("coop-rcpt-trunc", None);
    assert_eq!(code, 0);
    let bytes = std::fs::read(&receipt).unwrap();
    std::fs::write(&receipt, &bytes[..bytes.len() / 2]).unwrap();
    let (vcode, _, _) = verify_receipt(&receipt);
    assert_eq!(vcode, 2, "truncated receipt must fail closed");
}

#[test]
fn cooperative_receipt_trailing_data_fails() {
    let (code, receipt) = run_cooperative_receipt("coop-rcpt-trail", None);
    assert_eq!(code, 0);
    let mut bytes = std::fs::read(&receipt).unwrap();
    bytes.extend_from_slice(b"extra\n");
    std::fs::write(&receipt, &bytes).unwrap();
    let (vcode, _, _) = verify_receipt(&receipt);
    assert_eq!(vcode, 2, "trailing data must fail closed");
}

#[test]
fn cooperative_receipt_body_digest_mismatch_fails_before_replay() {
    let (code, receipt) = run_cooperative_receipt("coop-rcpt-body-digest", None);
    assert_eq!(code, 0);
    let bytes = std::fs::read(&receipt).unwrap();
    let text = String::from_utf8(bytes).unwrap();
    let digest_line = text
        .lines()
        .find(|line| line.starts_with("digest sha256:"))
        .unwrap();
    let replacement = format!("digest sha256:{}", "0".repeat(64));
    let tampered = text.replacen(digest_line, &replacement, 1);
    std::fs::write(&receipt, tampered).unwrap();
    let (vcode, stdout, _) = verify_receipt(&receipt);
    assert_eq!(vcode, 1);
    assert!(stdout.contains("body-digest-mismatch"), "{stdout}");
}

#[test]
fn cooperative_receipt_noncanonical_numbers_fail_structurally() {
    for (label, old, new) in [
        ("exit", "exit-code 0\n", "exit-code 00\n"),
        ("frame", "errors 2:[]\n", "errors 02:[]\n"),
    ] {
        let (code, receipt) =
            run_cooperative_receipt(&format!("coop-rcpt-noncanonical-{label}"), None);
        assert_eq!(code, 0);
        let bytes = std::fs::read(&receipt).unwrap();
        let text = String::from_utf8(bytes).unwrap();
        let mutated = text.replacen(old, new, 1);
        assert_ne!(mutated, text);
        std::fs::write(&receipt, redigest(mutated.as_bytes())).unwrap();
        let (vcode, stdout, _) = verify_receipt(&receipt);
        assert_eq!(vcode, 2, "alternate number spelling must fail: {stdout}");
        assert!(stdout.is_empty());
    }
}

#[test]
fn cooperative_receipt_duplicate_field_fails() {
    let (code, receipt) = run_cooperative_receipt("coop-rcpt-dup", None);
    assert_eq!(code, 0);
    let bytes = std::fs::read(&receipt).unwrap();
    let text = String::from_utf8_lossy(&bytes).into_owned();
    let dup = text.replacen("verdict CLEAN\n", "verdict CLEAN\nverdict CLEAN\n", 1);
    std::fs::write(&receipt, redigest(dup.as_bytes())).unwrap();
    let (vcode, _, _) = verify_receipt(&receipt);
    assert_eq!(vcode, 2, "duplicate field must fail closed");
}

#[test]
fn cooperative_receipt_reordered_fields_fail() {
    let (code, receipt) = run_cooperative_receipt("coop-rcpt-reorder", None);
    assert_eq!(code, 0);
    let bytes = std::fs::read(&receipt).unwrap();
    let text = String::from_utf8_lossy(&bytes).into_owned();
    let swapped = text.replacen(
        "oracle cooperative-llm-call-completed\noracle-evaluation completed\n",
        "oracle-evaluation completed\noracle cooperative-llm-call-completed\n",
        1,
    );
    assert_ne!(swapped, text, "fixture must contain the oracle lines");
    std::fs::write(&receipt, redigest(swapped.as_bytes())).unwrap();
    let (vcode, _, _) = verify_receipt(&receipt);
    assert_eq!(vcode, 2, "reordered fields must fail closed");
}

#[test]
fn cooperative_receipt_unknown_field_fails() {
    let (code, receipt) = run_cooperative_receipt("coop-rcpt-unknown", None);
    assert_eq!(code, 0);
    let bytes = std::fs::read(&receipt).unwrap();
    let text = String::from_utf8_lossy(&bytes).into_owned();
    let injected = text.replacen("verdict CLEAN\n", "evil 1\nverdict CLEAN\n", 1);
    std::fs::write(&receipt, redigest(injected.as_bytes())).unwrap();
    let (vcode, _, _) = verify_receipt(&receipt);
    assert_eq!(vcode, 2, "unknown field must fail closed");
}

#[test]
fn cooperative_receipt_blank_interior_line_fails() {
    let (code, receipt) = run_cooperative_receipt("coop-rcpt-blank", None);
    assert_eq!(code, 0);
    let bytes = std::fs::read(&receipt).unwrap();
    let text = String::from_utf8_lossy(&bytes).into_owned();
    let blanked = text.replacen("verdict CLEAN\n", "\nverdict CLEAN\n", 1);
    std::fs::write(&receipt, redigest(blanked.as_bytes())).unwrap();
    let (vcode, _, _) = verify_receipt(&receipt);
    assert_eq!(vcode, 2, "blank interior line must fail closed");
}

#[test]
fn cooperative_receipt_cassette_tamper_fails() {
    let (code, receipt) = run_cooperative_receipt("coop-rcpt-casstam", None);
    assert_eq!(code, 0);
    let mut bytes = std::fs::read(&receipt).unwrap();
    // Flip a byte inside the framed cassette payload (after the header).
    let marker = b"cassette ";
    let pos = bytes
        .windows(marker.len())
        .position(|w| w == marker)
        .expect("cassette field");
    let payload = pos + marker.len() + 40; // past the length prefix
    bytes[payload] ^= 0x01;
    std::fs::write(&receipt, redigest(&bytes)).unwrap();
    let (vcode, vout, _) = verify_receipt(&receipt);
    assert_eq!(
        vcode, 1,
        "cassette tamper must fail as inauthentic:\n{vout}"
    );
    assert!(vout.contains("\"authentic\":false"), "{vout}");
}

#[test]
fn cooperative_receipt_noncanonical_equivalent_cassette_fails_before_replay() {
    let (code, receipt) = run_cooperative_receipt("coop-rcpt-cassette-spelling", None);
    assert_eq!(code, 0);
    let bytes = std::fs::read(&receipt).unwrap();
    let range = framed_payload_range(&bytes, "cassette");
    let canonical = std::str::from_utf8(&bytes[range]).unwrap();
    let alternate = canonical.replacen("vh-cassette-v2 1\n", "vh-cassette-v2 01\n", 1);
    assert_ne!(alternate, canonical);
    let replaced = replace_framed_payload(&bytes, "cassette", alternate.as_bytes());
    std::fs::write(&receipt, redigest(&replaced)).unwrap();
    let (vcode, stdout, _) = verify_receipt(&receipt);
    assert_eq!(vcode, 1);
    assert!(stdout.contains("cassette-noncanonical"), "{stdout}");
    assert!(!stdout.contains("cassette-identity-mismatch"), "{stdout}");
}

#[test]
fn cooperative_receipt_identity_tamper_fails() {
    let (code, receipt) = run_cooperative_receipt("coop-rcpt-idtam", None);
    assert_eq!(code, 0);
    let bytes = std::fs::read(&receipt).unwrap();
    let text = String::from_utf8_lossy(&bytes).into_owned();
    let first_line = text
        .lines()
        .find(|l| l.starts_with("first-identity "))
        .unwrap()
        .to_string();
    // Fabricate: force the first hex char to differ.
    let mut chars: Vec<char> = first_line.chars().collect();
    let idx = "first-identity ".len();
    chars[idx] = if chars[idx] == 'a' { 'b' } else { 'a' };
    let fabricated: String = chars.into_iter().collect();
    let tampered = text.replacen(&first_line, &fabricated, 1);
    std::fs::write(&receipt, redigest(tampered.as_bytes())).unwrap();
    let (vcode, vout, _) = verify_receipt(&receipt);
    assert_eq!(
        vcode, 1,
        "identity tamper must fail as inauthentic:\n{vout}"
    );
}

#[test]
fn cooperative_receipt_artifact_tamper_fails() {
    let (code, receipt) = run_cooperative_receipt("coop-rcpt-arttam", None);
    assert_eq!(code, 0);
    let mut bytes = std::fs::read(&receipt).unwrap();
    let range = framed_payload_range(&bytes, "first-artifact");
    bytes[range.start] ^= 0x01;
    let digest = vh_sandbox::fnv_hex(&bytes[range.clone()]);
    let text = String::from_utf8(bytes).unwrap();
    let old_digest = text
        .lines()
        .find(|line| line.starts_with("first-artifact-digest "))
        .unwrap();
    let updated = text.replacen(old_digest, &format!("first-artifact-digest {digest}"), 1);
    std::fs::write(&receipt, redigest(updated.as_bytes())).unwrap();
    let (vcode, vout, _) = verify_receipt(&receipt);
    assert_eq!(
        vcode, 1,
        "artifact tamper must fail as inauthentic:\n{vout}"
    );
    assert!(vout.contains("replay-first-artifact-mismatch"), "{vout}");
    assert!(!vout.contains("first-artifact-digest-mismatch"), "{vout}");
}

#[test]
fn cooperative_receipt_engine_mismatch_fails() {
    let (code, receipt) = run_cooperative_receipt("coop-rcpt-engine", None);
    assert_eq!(code, 0);
    let bytes = std::fs::read(&receipt).unwrap();
    let text = String::from_utf8_lossy(&bytes).into_owned();
    let engine_line = text
        .lines()
        .find(|l| l.starts_with("engine-sha256 "))
        .unwrap()
        .to_string();
    let forged = format!("engine-sha256 {}", "0".repeat(64));
    let tampered = text.replacen(&engine_line, &forged, 1);
    std::fs::write(&receipt, redigest(tampered.as_bytes())).unwrap();
    let (vcode, vout, _) = verify_receipt(&receipt);
    assert_eq!(
        vcode, 1,
        "engine mismatch must fail as inauthentic:\n{vout}"
    );
}

#[test]
fn cooperative_receipt_engine_request_digest_tamper_fails_before_replay() {
    let (code, receipt) = run_cooperative_receipt("coop-rcpt-request-digest", None);
    assert_eq!(code, 0);
    let bytes = std::fs::read(&receipt).unwrap();
    let tampered = replace_line_and_redigest(
        &bytes,
        "engine-request-digest",
        &format!("engine-request-digest {}", "0".repeat(64)),
    );
    std::fs::write(&receipt, tampered).unwrap();
    let (vcode, stdout, _) = verify_receipt(&receipt);
    assert_eq!(vcode, 1);
    assert!(
        stdout.contains("engine-request-digest-mismatch"),
        "{stdout}"
    );
}

#[test]
fn cooperative_receipt_workload_tamper_fails_before_execution() {
    let (code, receipt) = run_cooperative_receipt("coop-rcpt-wltam", None);
    assert_eq!(code, 0);
    let bytes = std::fs::read(&receipt).unwrap();
    let text = String::from_utf8(bytes.clone()).unwrap();
    let request_digest = cooperative_engine_request_digest_for_test(
        "cooperative-evil",
        receipt_line_value(&text, "cassette-identity"),
        receipt_line_value(&text, "child-source-digest"),
    );
    let changed_workload =
        replace_line_and_redigest(&bytes, "workload", "workload cooperative-evil");
    let tampered = replace_line_and_redigest(
        &changed_workload,
        "engine-request-digest",
        &format!("engine-request-digest {request_digest}"),
    );
    std::fs::write(&receipt, tampered).unwrap();
    let (vcode, vout, _) = verify_receipt(&receipt);
    assert_eq!(vcode, 1, "unknown workload must fail closed:\n{vout}");
    assert!(vout.contains("unknown-workload"), "{vout}");
    assert!(!vout.contains("engine-request-digest-mismatch"), "{vout}");
}

#[test]
fn cooperative_receipt_forged_clean_contradiction_fails() {
    // A forged receipt whose content digest is valid but whose claimed
    // CLEAN verdict contradicts the fresh replay.
    let tmp = unique_tmp("coop-rcpt-forge-cassette");
    let cassette = timeout_cassette_file(&tmp);
    let (code, receipt) = run_cooperative_receipt("coop-rcpt-forge", Some(&cassette));
    assert_eq!(code, 1);
    let bytes = std::fs::read(&receipt).unwrap();
    let text = String::from_utf8_lossy(&bytes).into_owned();
    let forged = text
        .replacen("verdict FINDINGS\n", "verdict CLEAN\n", 1)
        .replacen(
            "oracle-evaluation not-completed:timeout\n",
            "oracle-evaluation completed\n",
            1,
        )
        .replacen(
            "finding-identity cooperative-llm-call-completed:timeout\n",
            "finding-identity none\n",
            1,
        )
        .replacen("findings-count 1\n", "findings-count 0\n", 1)
        .replacen("exit-code 1\n", "exit-code 0\n", 1);
    assert_ne!(forged, text);
    std::fs::write(&receipt, redigest(forged.as_bytes())).unwrap();
    let (vcode, vout, _) = verify_receipt(&receipt);
    assert_eq!(
        vcode, 1,
        "a CLEAN claim contradicting fresh replay must fail closed:\n{vout}"
    );
    assert!(vout.contains("\"authentic\":false"), "{vout}");
}

#[test]
fn cooperative_receipt_fabricated_identities_fail_replay_consistency() {
    // Internally re-digested, valid-shaped receipt with fabricated run
    // identities: this proves replay consistency, not provenance/authorship.
    // The body digest is consistent, but the identities contradict replay.
    let (code, receipt) = run_cooperative_receipt("coop-rcpt-handwritten", None);
    assert_eq!(code, 0);
    let bytes = std::fs::read(&receipt).unwrap();
    let fabricated_a = format!("first-identity {}", "a".repeat(32));
    let fabricated_b = format!("second-identity {}", "a".repeat(32));
    let step1 = replace_line_and_redigest(&bytes, "first-identity", &fabricated_a);
    let step2 = replace_line_and_redigest(&step1, "second-identity", &fabricated_b);
    std::fs::write(&receipt, step2).unwrap();
    let (vcode, vout, _) = verify_receipt(&receipt);
    assert_eq!(
        vcode, 1,
        "fabricated identities must fail against fresh replay:\n{vout}"
    );
}

#[test]
fn cooperative_receipt_forged_source_never_executes() {
    // Forged-source receipt with a recomputed content digest and a
    // sentinel side effect: rejection must come BEFORE any child launch.
    let sentinel_root = unique_tmp("coop-rcpt-source-sentinel");
    let sentinel = sentinel_root.join("must-not-exist");
    let (code, receipt) = run_cooperative_receipt("coop-rcpt-forgesrc", None);
    assert_eq!(code, 0);
    let bytes = std::fs::read(&receipt).unwrap();
    let forged_source =
        format!("open({:?},'w').write('x')\n", sentinel.to_string_lossy()).into_bytes();
    let text = String::from_utf8_lossy(&bytes).into_owned();
    // The child-source field is framed; replace it using its declared
    // length (the payload itself contains newlines).
    let marker = "child-source ";
    let start = text.find(marker).expect("child-source field");
    let after_tag = start + marker.len();
    let colon = text[after_tag..].find(':').map(|i| after_tag + i).unwrap();
    let declared: usize = text[after_tag..colon].parse().unwrap();
    let field_end = colon + 1 + declared; // index of the field's trailing newline
    let mut mutated = String::new();
    mutated.push_str(&text[..start]);
    mutated.push_str(&format!("child-source {}:", forged_source.len()));
    mutated.push_str(&String::from_utf8_lossy(&forged_source));
    mutated.push_str(&text[field_end..]);
    // Recompute the source digest line for the forged bytes.
    let digest_line = format!(
        "child-source-digest sha256:{}",
        vh_digest::sha256_hex(&forged_source)
    );
    let old_digest_line = mutated
        .lines()
        .find(|l| l.starts_with("child-source-digest "))
        .unwrap()
        .to_string();
    let mutated = mutated.replacen(&old_digest_line, &digest_line, 1);
    let source_digest = digest_line.strip_prefix("child-source-digest ").unwrap();
    let request_digest = cooperative_engine_request_digest_for_test(
        receipt_line_value(&mutated, "workload"),
        receipt_line_value(&mutated, "cassette-identity"),
        source_digest,
    );
    let old_request_digest = mutated
        .lines()
        .find(|line| line.starts_with("engine-request-digest "))
        .unwrap()
        .to_string();
    let mutated = mutated.replacen(
        &old_request_digest,
        &format!("engine-request-digest {request_digest}"),
        1,
    );
    std::fs::write(&receipt, redigest(mutated.as_bytes())).unwrap();
    let (vcode, vout, _) = verify_receipt(&receipt);
    assert_eq!(vcode, 1, "forged source must be rejected:\n{vout}");
    assert!(vout.contains("source-mismatch"), "{vout}");
    assert!(!vout.contains("engine-request-digest-mismatch"), "{vout}");
    assert!(
        !sentinel.exists(),
        "the forged child source must never have executed"
    );
}
