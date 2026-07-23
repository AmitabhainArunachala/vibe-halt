use super::*;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};

static TEMP_COUNTER: AtomicU64 = AtomicU64::new(0);

fn temp_dir(label: &str) -> PathBuf {
    let id = TEMP_COUNTER.fetch_add(1, Ordering::SeqCst);
    std::env::temp_dir().join(format!("vh-sandbox-test-{label}-{id}"))
}

fn sh_spec(script: &str) -> SandboxSpec {
    SandboxSpec::new(vec!["/bin/sh".into(), "-c".into(), script.into()]).unwrap()
}

#[test]
fn spec_identity_is_env_order_independent_and_pins_defaults() {
    let a = SandboxSpec::new(vec!["python3".into(), "fixture.py".into()])
        .unwrap()
        .allow_env("VH_MODE", "clean")
        .unwrap();
    let b = SandboxSpec::new(vec!["python3".into(), "fixture.py".into()])
        .unwrap()
        .allow_env("VH_MODE", "clean")
        .unwrap();
    assert_eq!(a.env.get("PYTHONHASHSEED"), Some(&"0".to_string()));
    assert_eq!(a.env.get("TZ"), Some(&"UTC".to_string()));
    assert_eq!(a.env.get("LC_ALL"), Some(&"C".to_string()));
    assert_eq!(a.identity(), b.identity());
}

#[test]
fn artifact_paths_fail_closed_on_escape() {
    assert!(ArtifactSpec::new("out.txt").is_ok());
    assert!(ArtifactSpec::new("../secret").is_err());
    assert!(ArtifactSpec::new("/tmp/secret").is_err());
}

#[test]
fn cassette_replay_is_exact_digest_or_miss() {
    let req = LlmRequest {
        provider: "fixture".into(),
        model: "echo".into(),
        messages: vec!["hi".into()],
        params: BTreeMap::from([("temperature".into(), "0".into())]),
    };
    let mut cassette = Cassette::default();
    cassette.insert(
        &req,
        CassetteEntry {
            response: b"hello".to_vec(),
            boundary_telemetry: BTreeMap::from([("captured_by".into(), "fixture".into())]),
        },
    );
    assert_eq!(cassette.replay(&req).unwrap(), b"hello".to_vec());
    let mut miss = req.clone();
    miss.messages.push("extra".into());
    assert_eq!(cassette.replay(&miss).unwrap_err().digest, miss.digest());
}

#[test]
fn subprocess_run_records_digests_and_artifacts_without_wall_time_identity() {
    let root = temp_dir("run");
    let a = root.join("a");
    let b = root.join("b");
    let spec = sh_spec("printf stable; printf artifact > out.txt")
        .declare_artifact("out.txt")
        .unwrap();
    let first = run_once(&spec, &a).unwrap();
    let second = run_once(&spec, &b).unwrap();
    assert_eq!(first.termination, TerminationOutcome::Exited(0));
    assert_eq!(first.stdout.digest, second.stdout.digest);
    assert_eq!(first.artifacts, second.artifacts);
    assert_eq!(first.identity(), second.identity());
    assert_eq!(first.evidence_grade(), EvidenceGrade::D2);
}

#[test]
fn run_twice_reports_divergence_rate() {
    let root = temp_dir("twice");
    let spec = sh_spec("printf stable");
    let campaign = run_twice(&spec, &root.join("one"), &root.join("two")).unwrap();
    let report = campaign.divergence_report();
    assert_eq!(report.diverged, 0);
    assert_eq!(report.sample, 1);
    assert_eq!(report.rate(), 0.0);
    assert!(campaign.verdict_line().contains("tier=Tier-2 d-grade=D2"));
    assert!(campaign
        .verdict_line()
        .contains("evidence=run-twice agreement"));
}

#[test]
fn divergence_report_carries_raw_counts_over_a_declared_suite() {
    // Two identical pairs, one diverging pair: replaces the earlier
    // single-pair 0.0/1.0 special case with real numerator/denominator
    // evidence over a multi-pair suite.
    let report = DivergenceReport::from_identity_pairs([("a", "a"), ("b", "b"), ("c", "d")]);
    assert_eq!(report.diverged, 1);
    assert_eq!(report.sample, 3);
    assert!((report.rate() - (1.0 / 3.0)).abs() < 1e-9);
    // Same multiset, different order -> different sample identity: order
    // is part of the declared suite, not incidental.
    let reordered = DivergenceReport::from_identity_pairs([("c", "d"), ("a", "a"), ("b", "b")]);
    assert_ne!(report.sample_identity, reordered.sample_identity);
}

#[test]
fn empty_declared_suite_is_zero_over_zero_not_a_fabricated_clean_rate() {
    let report = DivergenceReport::from_identity_pairs(std::iter::empty());
    assert_eq!(report.sample, 0);
    assert_eq!(report.diverged, 0);
    assert_eq!(report.rate(), 0.0);
}

#[test]
fn no_caller_input_can_mint_a_d1_run_record() {
    // SandboxSpec has no field or method that can assert a channel
    // closed (compile-time fact: it carries argv/stdin/env/artifacts/
    // input_files/budget/cassette+supervisor identity only). The runner
    // always produces a fully-open receipt in this package.
    let root = temp_dir("no-d1-mint");
    let spec = sh_spec("true");
    let record = run_once(&spec, &root).unwrap();
    assert_eq!(record.evidence_grade(), EvidenceGrade::D2);
    assert_eq!(
        record.capability.open_channels().len(),
        CapabilityChannel::ALL.len()
    );
    assert!(!record.capability.is_d1());
}

#[test]
fn capability_channel_inventory_is_pinned_and_exhaustive() {
    assert_eq!(CapabilityChannel::ALL.len(), 29);
    // Every channel string is unique: no two distinct channels can be
    // confused in a rendered receipt.
    let mut names: Vec<&str> = CapabilityChannel::ALL.iter().map(|c| c.as_str()).collect();
    let before = names.len();
    names.sort_unstable();
    names.dedup();
    assert_eq!(names.len(), before, "channel names must be unique");
}

#[test]
fn capability_receipt_is_d1_only_when_every_channel_is_closed() {
    // Exercises the boolean logic directly; there is no production path
    // that can reach an all-closed receipt in this package (see
    // `no_caller_input_can_mint_a_d1_run_record`).
    let mut receipt = CapabilityReceipt::all_open("test fixture");
    assert!(!receipt.is_d1());
    for &channel in CapabilityChannel::ALL.iter() {
        receipt.set_status_for_test(
            channel,
            ChannelStatus::Closed {
                evidence: "test fixture: synchronously verified".into(),
            },
        );
    }
    assert!(receipt.is_d1());
    assert_eq!(receipt.evidence_grade(), EvidenceGrade::D1);
    assert!(receipt.open_channels().is_empty());
}

#[test]
fn different_signals_never_collapse_to_the_same_identity() {
    let killed = TerminationOutcome::Signaled {
        signal: 9,
        core_dumped: None,
    };
    let terminated = TerminationOutcome::Signaled {
        signal: 15,
        core_dumped: None,
    };
    assert_ne!(killed.as_identity_str(), terminated.as_identity_str());
}

#[test]
fn exact_signal_never_collapses_with_unknown() {
    let signaled = TerminationOutcome::Signaled {
        signal: 9,
        core_dumped: None,
    };
    let unknown = TerminationOutcome::Unknown {
        reason: "signaled:9:core_dumped=unknown".into(),
    };
    // Even though the payload strings could otherwise coincide, the tag
    // prefix keeps the two variants from ever sharing an identity.
    assert_ne!(signaled.as_identity_str(), unknown.as_identity_str());
}

#[test]
fn real_subprocess_signals_are_recovered_exactly_and_distinctly() {
    let root = temp_dir("signals");
    let sigkill = sh_spec("kill -9 $$");
    let sigterm = sh_spec("kill -15 $$");
    let killed = run_once(&sigkill, &root.join("kill9")).unwrap();
    let termed = run_once(&sigterm, &root.join("kill15")).unwrap();
    assert_eq!(
        killed.termination,
        TerminationOutcome::Signaled {
            signal: 9,
            core_dumped: killed_core_dumped(&killed.termination),
        }
    );
    assert_eq!(
        termed.termination,
        TerminationOutcome::Signaled {
            signal: 15,
            core_dumped: killed_core_dumped(&termed.termination),
        }
    );
    assert_ne!(killed.identity(), termed.identity());
    assert_eq!(killed.process_tree, ProcessTreeState::DirectChildReaped);
}

fn killed_core_dumped(outcome: &TerminationOutcome) -> Option<bool> {
    match outcome {
        TerminationOutcome::Signaled { core_dumped, .. } => *core_dumped,
        _ => panic!("expected a Signaled outcome"),
    }
}

#[test]
fn spawn_failure_is_typed_not_a_hard_error_and_skips_declared_artifacts() {
    let root = temp_dir("spawn-fail");
    let spec = SandboxSpec::new(vec!["/definitely/not/a/real/executable-xyz".into()])
        .unwrap()
        .declare_artifact("out.txt")
        .unwrap();
    let record = run_once(&spec, &root).unwrap();
    assert!(matches!(
        record.termination,
        TerminationOutcome::SpawnFailed { .. }
    ));
    assert_eq!(record.process_tree, ProcessTreeState::NoChildProcess);
    assert!(record.artifacts.is_empty());
    assert_eq!(record.evidence_grade(), EvidenceGrade::D2);
}

#[test]
fn no_unbounded_wait_a_hung_child_is_killed_and_reaped_at_the_deadline() {
    let root = temp_dir("timeout");
    let spec = sh_spec("sleep 60").with_budget(
        SandboxBudget::new(
            Duration::from_millis(150),
            SandboxBudget::DEFAULT_MAX_OUTPUT_BYTES,
        )
        .unwrap(),
    );
    let record = run_once(&spec, &root).unwrap();
    assert_eq!(record.termination, TerminationOutcome::TimedOut);
    assert_eq!(record.process_tree, ProcessTreeState::DirectChildReaped);
    assert!(
        record.wall_time < Duration::from_secs(5),
        "expected the controller to kill the hung child near its deadline, wall_time={:?}",
        record.wall_time
    );
    // Declared artifacts are never expected from a killed run.
    assert!(record.artifacts.is_empty());
}

#[test]
fn deadline_still_fires_when_child_never_reads_large_stdin() {
    let root = temp_dir("timeout-large-stdin");
    // Four MiB is intentionally well above ordinary anonymous-pipe
    // capacity. The pre-fix controller called `ChildStdin::write_all`
    // before entering its deadline loop, so this child could backpressure
    // that write forever. The prepared-file handoff has no live writer to
    // block: the deadline must remain observable and reap the child.
    let spec = sh_spec("sleep 60")
        .with_stdin(vec![b'x'; 4 * 1024 * 1024])
        .with_budget(
            SandboxBudget::new(
                Duration::from_millis(150),
                SandboxBudget::DEFAULT_MAX_OUTPUT_BYTES,
            )
            .unwrap(),
        );
    let record = run_once(&spec, &root).unwrap();
    assert_eq!(record.termination, TerminationOutcome::TimedOut);
    assert_eq!(record.process_tree, ProcessTreeState::DirectChildReaped);
    assert!(
        record.wall_time < Duration::from_secs(5),
        "large unread stdin bypassed the controller deadline: {:?}",
        record.wall_time
    );
}

#[test]
fn bounded_output_truncates_and_flags_but_never_hides_the_true_length() {
    let root = temp_dir("bounded-output");
    let cap = 1024usize;
    let spec = sh_spec("yes x | head -c 200000")
        .with_budget(SandboxBudget::new(Duration::from_secs(10), cap).unwrap());
    let record = run_once(&spec, &root).unwrap();
    assert_eq!(record.termination, TerminationOutcome::Exited(0));
    assert!(record.stdout.truncated);
    assert_eq!(record.stdout.byte_len, 200_000);
    assert!(record.stdout.digest != fnv_hex(&[]));
}

#[test]
fn executable_identity_is_resolved_for_absolute_paths_and_open_for_bare_names() {
    // Direct filesystem access is not part of this test file's
    // determinism-denylist exemption (only environment-variable reads
    // and the process-counter atomic are), so independent verification
    // goes through two separate `run_once` resolutions of the same
    // absolute path rather than a direct read here: both must agree,
    // deterministically, on `/bin/sh`'s digest.
    let root = temp_dir("executable-identity");
    let absolute = sh_spec("true");
    let first = run_once(&absolute, &root.join("abs1")).unwrap();
    let second = run_once(&absolute, &root.join("abs2")).unwrap();
    match (&first.executable, &second.executable) {
        (
            ExecutableIdentity::Resolved {
                path: p1,
                digest: d1,
            },
            ExecutableIdentity::Resolved {
                path: p2,
                digest: d2,
            },
        ) => {
            assert_eq!(p1, "/bin/sh");
            assert_eq!(p1, p2);
            assert_eq!(d1, d2);
            assert!(!d1.is_empty());
        }
        other => panic!("expected both runs Resolved, got {other:?}"),
    }

    let bare = SandboxSpec::new(vec!["true".into()]).unwrap();
    let unresolved = run_once(&bare, &root.join("bare")).unwrap();
    assert_eq!(
        unresolved.executable,
        ExecutableIdentity::Unresolved {
            argv0: "true".into()
        }
    );
}

#[test]
#[cfg(unix)]
fn executable_identity_binds_pre_spawn_bytes_when_child_replaces_its_path() {
    let root = temp_dir("executable-replacement");
    let script_path = root.join("runner.sh");

    // Produce the executable through the already-admitted subprocess
    // boundary rather than adding direct filesystem I/O to this test
    // module. The script proves it executed the original body by printing
    // "original", then replaces its own path before exit.
    let setup = sh_spec(
        "printf '%s\\n' '#!/bin/sh' 'printf replaced > \"$0\"' \
         'printf original' > runner.sh; chmod +x runner.sh",
    )
    .declare_artifact("runner.sh")
    .unwrap();
    let setup_record = run_once(&setup, &root).unwrap();
    let original_digest = setup_record
        .artifacts
        .get("runner.sh")
        .expect("setup produced the executable")
        .clone();

    let spec = SandboxSpec::new(vec![script_path.display().to_string()]).unwrap();
    let record = run_once(&spec, &root.join("execution")).unwrap();
    assert_eq!(record.termination, TerminationOutcome::Exited(0));
    assert_eq!(record.stdout.digest, fnv_hex(b"original"));
    match &record.executable {
        ExecutableIdentity::Resolved { path, digest } => {
            assert_eq!(path, &script_path.display().to_string());
            assert_eq!(
                digest, &original_digest,
                "the run identity must bind the bytes observed for launch, not replacement bytes read after exit"
            );
        }
        other => panic!("expected pre-spawn Resolved identity, got {other:?}"),
    }

    // Independently prove the executable path now carries different bytes.
    let inspect = sh_spec("true").declare_artifact("runner.sh").unwrap();
    let replaced_record = run_once(&inspect, &root).unwrap();
    assert_ne!(
        replaced_record.artifacts.get("runner.sh"),
        Some(&original_digest),
        "adversarial fixture did not replace its executable path"
    );
}

#[test]
fn input_files_bind_content_into_spec_identity() {
    // Same filesystem-exemption constraint as above: the fixture file is
    // written and rewritten through a subprocess run (already-permitted
    // boundary I/O in `lib.rs`), never through a direct read/write call
    // in this test file.
    let root = temp_dir("input-files");
    let write_v1 = sh_spec("printf '%s' v1 > fixture.py")
        .declare_artifact("fixture.py")
        .unwrap();
    assert_eq!(
        run_once(&write_v1, &root).unwrap().termination,
        TerminationOutcome::Exited(0)
    );
    let script_path = root.join("fixture.py");
    let a = sh_spec("true").declare_input_file(&script_path).unwrap();

    let write_v2 = sh_spec("printf '%s' v2 > fixture.py")
        .declare_artifact("fixture.py")
        .unwrap();
    assert_eq!(
        run_once(&write_v2, &root).unwrap().termination,
        TerminationOutcome::Exited(0)
    );
    let b = sh_spec("true").declare_input_file(&script_path).unwrap();

    assert_ne!(a.identity(), b.identity());
    assert_ne!(
        a.input_files.get(&script_path.display().to_string()),
        b.input_files.get(&script_path.display().to_string())
    );
}

#[test]
fn cassette_and_supervisor_identity_default_to_none_and_are_bound_when_present() {
    let bare = sh_spec("true");
    assert_eq!(bare.cassette_identity, None);
    assert_eq!(bare.supervisor_identity, None);

    let with_cassette = sh_spec("true").with_cassette_identity("cassette-abc");
    assert_ne!(bare.identity(), with_cassette.identity());

    let with_supervisor = sh_spec("true").with_supervisor_identity("helper-xyz");
    assert_ne!(bare.identity(), with_supervisor.identity());
}

#[test]
fn sandbox_budget_rejects_zero_deadline_and_zero_output_cap() {
    assert!(SandboxBudget::new(Duration::from_secs(0), 1024).is_err());
    assert!(SandboxBudget::new(Duration::from_secs(1), 0).is_err());
    assert!(SandboxBudget::new(Duration::from_secs(1), 1024).is_ok());
}
