use super::*;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};

static TEMP_COUNTER: AtomicU64 = AtomicU64::new(0);

fn temp_dir(label: &str) -> PathBuf {
    let id = TEMP_COUNTER.fetch_add(1, Ordering::SeqCst);
    std::env::temp_dir().join(format!(
        "vh-sandbox-test-{}-{label}-{id}",
        std::process::id()
    ))
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
    assert!(ArtifactSpec::new(".vh-sandbox-io/stdout.raw").is_err());
    assert!(sh_spec("true")
        .declare_input_bytes(".vh-sandbox-io/llm/req-0", b"x")
        .is_err());
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
fn deadline_kills_and_reaps_an_ordinary_hung_child() {
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
#[cfg(unix)]
fn child_cannot_redirect_output_collection_by_replacing_capture_paths() {
    let root = temp_dir("capture-path-replacement");
    let spec = sh_spec(
        "rm -f .vh-sandbox-io/stdout.raw .vh-sandbox-io/stderr.raw; \
         mkfifo .vh-sandbox-io/stdout.raw; \
         ln -s /dev/zero .vh-sandbox-io/stderr.raw; \
         printf safe; printf err >&2",
    )
    .with_budget(SandboxBudget::new(Duration::from_secs(2), 1024).expect("valid capture budget"));
    let record = run_once(&spec, &root).unwrap();
    assert_eq!(record.termination, TerminationOutcome::Exited(0));
    assert_eq!(record.stdout.byte_len, 4);
    assert_eq!(record.stdout.digest, fnv_hex(b"safe"));
    assert_eq!(record.stderr.byte_len, 3);
    assert_eq!(record.stderr.digest, fnv_hex(b"err"));
}

#[test]
fn executable_identity_resolves_only_absolute_paths() {
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

    for relative in ["./tool", "dir/tool"] {
        assert_eq!(
            resolve_executable_identity(relative),
            ExecutableIdentity::Unresolved {
                argv0: relative.into()
            },
            "relative argv0 is interpreted under the child workspace/platform and cannot be resolved from the controller cwd"
        );
    }
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

/// C5: a cassette can only run under a spec that BINDS its identity —
/// an unbound or stale binding is an error before anything executes.
#[test]
fn cassette_run_requires_identity_binding() {
    let mut cassette = crate::cassette_v2::CassetteV2::default();
    cassette.push(
        crate::cassette_v2::LlmRequestV2 {
            provider: "fixture".into(),
            model: "echo-2".into(),
            ..Default::default()
        },
        crate::cassette_v2::TapeEntry::Timeout,
    );
    // Never created: the binding check fails before any filesystem work.
    let workspace = std::env::temp_dir().join("vh-c5-bind-fixture");
    // Unbound spec.
    let unbound = SandboxSpec::new(vec!["/usr/bin/python3".into(), "x.py".into()]).unwrap();
    assert!(crate::run_once_with_cassette(&unbound, &workspace, &cassette).is_err());
    // Bound to a DIFFERENT tape.
    let stale = SandboxSpec::new(vec!["/usr/bin/python3".into(), "x.py".into()])
        .unwrap()
        .with_cassette_identity("vh-cassette-v2:sha256:0000");
    assert!(crate::run_once_with_cassette(&stale, &workspace, &cassette).is_err());
}

#[test]
fn programmatic_cassette_size_is_rejected_before_workspace_or_execution() {
    let mut cassette = CassetteV2::default();
    cassette.push(
        LlmRequestV2::default(),
        TapeEntry::Success {
            status: 200,
            body: vec![b'x'; crate::MAX_CASSETTE_BYTES as usize],
        },
    );
    let root = temp_dir("programmatic-cassette-oversize");
    let spec = sh_spec("printf escaped > escaped").with_cassette_identity(cassette.identity());
    assert!(matches!(
        crate::run_once_with_cassette(&spec, &root, &cassette),
        Err(SandboxError::Oversized { .. })
    ));
    assert!(!root.exists());
}

#[test]
fn post_exit_drain_classifies_a_burst_extra_request() {
    let request = LlmRequestV2::default();
    let mut cassette = CassetteV2::default();
    cassette.push(request.clone(), TapeEntry::Timeout);
    let root = temp_dir("post-exit-extra-request");
    let spec = sh_spec(
        "cat > request; \
         cp request .vh-sandbox-io/llm/req-0.tmp; \
         mv .vh-sandbox-io/llm/req-0.tmp .vh-sandbox-io/llm/req-0; \
         cp request .vh-sandbox-io/llm/req-1.tmp; \
         mv .vh-sandbox-io/llm/req-1.tmp .vh-sandbox-io/llm/req-1",
    )
    .with_stdin(request.canonical_bytes())
    .with_cassette_identity(cassette.identity());
    let record = crate::run_once_with_cassette(&spec, &root, &cassette).unwrap();
    let transport = record.transport.unwrap();
    assert_eq!(transport.served, vec![request.digest()]);
    assert!(
        transport
            .taint
            .as_deref()
            .is_some_and(|reason| reason.contains("beyond the recorded tape")),
        "extra request was silently ignored: {transport:?}"
    );
}

#[test]
fn transport_taint_does_not_waive_completed_process_artifact_postconditions() {
    let cassette = CassetteV2::default();
    let request = LlmRequestV2::default();
    let root = temp_dir("tainted-missing-artifact");
    let spec = sh_spec(
        "cat > .vh-sandbox-io/llm/req-0.tmp; \
         mv .vh-sandbox-io/llm/req-0.tmp .vh-sandbox-io/llm/req-0",
    )
    .with_stdin(request.canonical_bytes())
    .with_cassette_identity(cassette.identity())
    .declare_artifact("out.txt")
    .unwrap();
    assert!(matches!(
        crate::run_once_with_cassette(&spec, &root, &cassette),
        Err(SandboxError::ArtifactBoundary { .. })
    ));
}

/// Item 5/9: the broker's child-request read carries the same published
/// byte bound as cassette files — an oversized frame is a typed,
/// bounded taint, never an unbounded read or a raw parse dump.
#[test]
fn broker_rejects_oversized_child_request_before_parsing() {
    let mut cassette = CassetteV2::default();
    cassette.push(LlmRequestV2::default(), TapeEntry::Timeout);
    let root = temp_dir("broker-oversize");
    // 1 MiB + 1: the published maximum plus one byte. The frame is
    // published with the protocol's atomic temp+rename so the broker
    // either sees nothing or the complete oversize frame.
    let spec = sh_spec(
        "head -c 1048577 /dev/zero > .vh-sandbox-io/llm/req-0.tmp; \
         mv .vh-sandbox-io/llm/req-0.tmp .vh-sandbox-io/llm/req-0; sleep 0.3",
    )
    .with_cassette_identity(cassette.identity());
    let record = crate::run_once_with_cassette(&spec, &root, &cassette).unwrap();
    let taint = record
        .transport
        .as_ref()
        .unwrap()
        .taint
        .as_ref()
        .unwrap()
        .clone();
    assert!(
        taint.contains("exceeds"),
        "oversize must be a typed taint, got: {taint}"
    );
    assert!(taint.len() <= 256, "taint must stay bounded: {taint}");
}

#[test]
fn broker_rejects_semantically_equivalent_noncanonical_request_bytes() {
    let request = LlmRequestV2::default();
    let mut cassette = CassetteV2::default();
    cassette.push(request.clone(), TapeEntry::Timeout);
    let root = temp_dir("broker-noncanonical");
    let noncanonical = String::from_utf8(request.canonical_bytes())
        .unwrap()
        .replacen("messages 0\n", "messages 00\n", 1)
        .into_bytes();
    let spec = sh_spec(
        "cat > .vh-sandbox-io/llm/req-0.tmp; \
         mv .vh-sandbox-io/llm/req-0.tmp .vh-sandbox-io/llm/req-0; sleep 0.2",
    )
    .with_stdin(noncanonical)
    .with_cassette_identity(cassette.identity());
    let record = crate::run_once_with_cassette(&spec, &root, &cassette).unwrap();
    let transport = record.transport.unwrap();
    assert!(transport.served.is_empty());
    assert_eq!(transport.taint.as_deref(), Some("noncanonical request 0"));
}

#[test]
fn broker_services_one_frame_per_deadline_tick_and_stops_after_taint() {
    let request = LlmRequestV2::default();
    let mut cassette = CassetteV2::default();
    cassette.push(request.clone(), TapeEntry::Timeout);
    let root = temp_dir("broker-flood");
    std::fs::create_dir_all(&root).unwrap();
    for n in 0..128 {
        std::fs::write(root.join(format!("req-{n}")), request.canonical_bytes()).unwrap();
    }
    let mut broker = crate::BrokerState::new(root.clone(), &cassette);
    broker.service();
    assert_eq!(
        broker.next_seq, 1,
        "one service call consumed a request flood"
    );
    assert!(broker.taint.is_none());
    broker.service();
    assert_eq!(broker.next_seq, 2);
    assert!(broker.taint.is_some(), "out-of-tape request must taint");
    broker.service();
    assert_eq!(
        broker.next_seq, 2,
        "tainted broker kept walking attacker paths"
    );
}

#[test]
#[cfg(unix)]
fn broker_response_publication_refuses_preplanted_temp_and_final_paths() {
    let request = LlmRequestV2::default();
    let mut cassette = CassetteV2::default();
    cassette.push(request.clone(), TapeEntry::Timeout);

    let temp_case = temp_dir("broker-planted-temp");
    std::fs::create_dir_all(&temp_case).unwrap();
    std::fs::write(temp_case.join("req-0"), request.canonical_bytes()).unwrap();
    let sentinel = temp_case.join("sentinel");
    std::fs::write(&sentinel, b"preserve").unwrap();
    std::os::unix::fs::symlink(&sentinel, temp_case.join("resp-0.tmp")).unwrap();
    let mut broker = crate::BrokerState::new(temp_case.clone(), &cassette);
    broker.service();
    assert_eq!(std::fs::read(&sentinel).unwrap(), b"preserve");
    assert!(broker.served.is_empty());
    assert!(broker
        .taint
        .as_deref()
        .unwrap()
        .contains("temp-create-refused"));

    let final_case = temp_dir("broker-planted-final");
    std::fs::create_dir_all(&final_case).unwrap();
    std::fs::write(final_case.join("req-0"), request.canonical_bytes()).unwrap();
    std::fs::write(final_case.join("resp-0"), b"preserve").unwrap();
    let mut broker = crate::BrokerState::new(final_case.clone(), &cassette);
    broker.service();
    assert_eq!(
        std::fs::read(final_case.join("resp-0")).unwrap(),
        b"preserve"
    );
    assert!(broker.served.is_empty());
    assert!(broker
        .taint
        .as_deref()
        .unwrap()
        .contains("final-exists-or-link-failed"));
}

/// Item 5: the published cassette/request byte bound is enforced before
/// parsing and before allocation — exact boundary admitted, max + 1
/// rejected, including a sparse file whose logical size overflows.
#[test]
fn bounded_read_enforces_exact_boundary_and_rejects_sparse_oversize() {
    let root = temp_dir("bounded-read");
    std::fs::create_dir_all(&root).unwrap();

    let exact = root.join("exact.bin");
    std::fs::write(&exact, vec![7u8; crate::MAX_CASSETTE_BYTES as usize]).unwrap();
    let got = crate::read_bounded_file(&exact, crate::MAX_CASSETTE_BYTES).unwrap();
    assert_eq!(got.len() as u64, crate::MAX_CASSETTE_BYTES);

    let over = root.join("over.bin");
    std::fs::write(&over, vec![7u8; crate::MAX_CASSETTE_BYTES as usize + 1]).unwrap();
    assert!(matches!(
        crate::read_bounded_file(&over, crate::MAX_CASSETTE_BYTES),
        Err(SandboxError::Oversized { .. })
    ));

    // Sparse: logical size max + 1 with no backing blocks — rejected
    // from metadata without any max-sized allocation.
    let sparse = root.join("sparse.bin");
    let f = std::fs::File::create(&sparse).unwrap();
    f.set_len(crate::MAX_CASSETTE_BYTES + 1).unwrap();
    drop(f);
    assert!(matches!(
        crate::read_bounded_file(&sparse, crate::MAX_CASSETTE_BYTES),
        Err(SandboxError::Oversized { .. })
    ));
}

#[cfg(unix)]
#[test]
fn bounded_read_refuses_links_parent_links_and_special_files() {
    let root = temp_dir("bounded-special");
    std::fs::create_dir_all(&root).unwrap();
    let regular = root.join("regular");
    std::fs::write(&regular, b"ok").unwrap();

    let link = root.join("link");
    std::os::unix::fs::symlink(&regular, &link).unwrap();
    assert!(matches!(
        crate::read_bounded_file(&link, 32),
        Err(SandboxError::BoundaryFile("symlink" | "open-refused"))
    ));

    let real_parent = root.join("real-parent");
    std::fs::create_dir(&real_parent).unwrap();
    std::fs::write(real_parent.join("value"), b"ok").unwrap();
    let linked_parent = root.join("linked-parent");
    std::os::unix::fs::symlink(&real_parent, &linked_parent).unwrap();
    assert!(matches!(
        crate::read_bounded_file(&linked_parent.join("value"), 32),
        Err(SandboxError::BoundaryFile("symlink"))
    ));

    assert!(matches!(
        crate::read_bounded_file(Path::new("/dev/null"), 32),
        Err(SandboxError::BoundaryFile("non-regular-file"))
    ));
    assert!(matches!(
        crate::read_bounded_file(&root, 32),
        Err(SandboxError::BoundaryFile("non-regular-file"))
    ));
}

#[test]
#[cfg(unix)]
fn declared_inputs_and_executable_observation_refuse_special_or_oversized_files() {
    let root = temp_dir("bounded-declared-input");
    std::fs::create_dir_all(&root).unwrap();
    let fifo = root.join("input.fifo");
    let status = std::process::Command::new("mkfifo")
        .arg(&fifo)
        .status()
        .unwrap();
    assert!(status.success());
    assert!(matches!(
        sh_spec("true").declare_input_file(&fifo),
        Err(SandboxError::BoundaryFile(
            "non-regular-file" | "open-refused"
        ))
    ));

    let sparse = root.join("oversized-input");
    let file = std::fs::File::create(&sparse).unwrap();
    file.set_len(crate::MAX_INPUT_FILE_BYTES + 1).unwrap();
    assert!(matches!(
        sh_spec("true").declare_input_file(&sparse),
        Err(SandboxError::Oversized { .. })
    ));

    let spec = SandboxSpec::new(vec![fifo.display().to_string()]).unwrap();
    let record = run_once(&spec, &root.join("exec")).unwrap();
    assert!(matches!(
        record.executable,
        ExecutableIdentity::Unresolved { .. }
    ));
    assert!(matches!(
        record.termination,
        TerminationOutcome::SpawnFailed { .. }
    ));
}

#[test]
#[cfg(unix)]
fn preplanted_private_io_namespace_is_refused_without_following_links() {
    let root = temp_dir("preplanted-io");
    let io = root.join(".vh-sandbox-io");
    std::fs::create_dir_all(&io).unwrap();
    let sentinel = root.join("sentinel");
    std::fs::write(&sentinel, b"preserve").unwrap();
    std::os::unix::fs::symlink(&sentinel, io.join("stdin.raw")).unwrap();
    assert!(matches!(
        run_once(&sh_spec("true"), &root),
        Err(SandboxError::BoundaryFile("io-directory-not-exclusive"))
    ));
    assert_eq!(std::fs::read(&sentinel).unwrap(), b"preserve");
}

#[test]
#[cfg(unix)]
fn workspace_parent_symlink_is_refused_before_any_directory_is_created_through_it() {
    let root = temp_dir("workspace-parent-link");
    let outside = root.join("outside");
    std::fs::create_dir_all(&outside).unwrap();
    let linked_parent = root.join("linked-parent");
    std::os::unix::fs::symlink(&outside, &linked_parent).unwrap();
    assert!(matches!(
        run_once(&sh_spec("true"), &linked_parent.join("new-workspace")),
        Err(SandboxError::BoundaryFile("symlink"))
    ));
    assert!(
        !outside.join("new-workspace").exists(),
        "workspace creation followed a rejected parent symlink"
    );
}

#[test]
fn logical_inputs_reject_duplicates_collisions_and_staged_byte_changes() {
    let duplicate = sh_spec("true")
        .declare_input_bytes("script.sh", b"true")
        .unwrap()
        .declare_input_bytes("script.sh", b"false");
    assert!(duplicate.is_err());

    let collision = sh_spec("true")
        .declare_artifact("script.sh")
        .unwrap()
        .declare_input_bytes("script.sh", b"true");
    assert!(collision.is_err());

    let root = temp_dir("logical-mismatch");
    std::fs::create_dir_all(&root).unwrap();
    std::fs::write(root.join("script.sh"), b"printf sentinel > escaped").unwrap();
    let spec = SandboxSpec::new(vec!["/bin/sh".into(), "script.sh".into()])
        .unwrap()
        .declare_input_bytes("script.sh", b"true")
        .unwrap();
    assert!(matches!(
        run_once(&spec, &root),
        Err(SandboxError::InputMismatch {
            category: "digest-mismatch",
            ..
        })
    ));
    assert!(
        !root.join("escaped").exists(),
        "changed source must not execute"
    );
}

#[cfg(unix)]
#[test]
fn child_artifact_symlink_is_refused_at_sandbox_collection() {
    let root = temp_dir("artifact-link");
    let spec = sh_spec("ln -s /dev/zero out.txt")
        .declare_artifact("out.txt")
        .unwrap();
    assert!(matches!(
        run_once(&spec, &root),
        Err(SandboxError::ArtifactBoundary {
            category: "symlink" | "open-refused",
            ..
        })
    ));
}

#[test]
fn run_boundary_revalidates_public_spec_and_budget_fields() {
    let root = temp_dir("mutated-public-spec");

    let mut empty_argv = sh_spec("true");
    empty_argv.argv.clear();
    assert!(matches!(
        run_once(&empty_argv, &root),
        Err(SandboxError::InvalidSpec(_))
    ));

    let mut escaped_artifact = sh_spec("true");
    escaped_artifact.artifacts.push(ArtifactSpec {
        path: "../escaped".into(),
    });
    assert!(matches!(
        run_once(&escaped_artifact, &root),
        Err(SandboxError::InvalidSpec(_))
    ));

    let mut escaped_logical = sh_spec("true");
    escaped_logical
        .input_logical_files
        .insert("../source".into(), "0".repeat(64));
    assert!(matches!(
        run_once(&escaped_logical, &root),
        Err(SandboxError::InvalidSpec(_))
    ));

    let mut unbounded = sh_spec("true");
    unbounded.budget = SandboxBudget {
        deadline: MAX_SANDBOX_DEADLINE + Duration::from_secs(1),
        max_output_bytes: MAX_CAPTURE_BYTES + 1,
    };
    assert!(matches!(
        run_once(&unbounded, &root),
        Err(SandboxError::InvalidSpec(_))
    ));
}

#[test]
fn physical_input_is_reobserved_before_workspace_write_or_spawn() {
    let host = temp_dir("physical-input-host");
    std::fs::create_dir_all(&host).unwrap();
    let input = host.join("controller-input");
    std::fs::write(&input, b"first").unwrap();
    let spec = sh_spec("true").declare_input_file(&input).unwrap();
    std::fs::write(&input, b"changed").unwrap();
    let workspace = host.join("workspace");
    assert!(matches!(
        run_once(&spec, &workspace),
        Err(SandboxError::InputMismatch {
            category: "digest-mismatch",
            ..
        })
    ));
    assert!(
        !workspace.exists(),
        "input mismatch must be rejected before workspace publication"
    );
}

#[test]
fn published_budget_and_stdin_hard_bounds_are_enforced() {
    assert!(SandboxBudget::new(MAX_SANDBOX_DEADLINE, MAX_CAPTURE_BYTES).is_ok());
    assert!(SandboxBudget::new(
        MAX_SANDBOX_DEADLINE + Duration::from_nanos(1),
        MAX_CAPTURE_BYTES,
    )
    .is_err());
    assert!(SandboxBudget::new(MAX_SANDBOX_DEADLINE, MAX_CAPTURE_BYTES + 1).is_err());

    let root = temp_dir("oversized-stdin");
    let spec = sh_spec("true").with_stdin(vec![0; MAX_STDIN_BYTES + 1]);
    assert!(matches!(
        run_once(&spec, &root),
        Err(SandboxError::InvalidSpec(_))
    ));
    assert!(!root.exists());
}

#[test]
fn broker_taints_a_future_request_when_the_expected_sequence_is_absent() {
    let root = temp_dir("future-request");
    std::fs::create_dir_all(&root).unwrap();
    std::fs::write(root.join("req-1"), b"future").unwrap();
    let cassette = crate::cassette_v2::CassetteV2::default();
    let mut broker = BrokerState::new(root, &cassette);
    broker.service();
    assert!(
        broker
            .taint
            .as_deref()
            .is_some_and(|reason| reason.contains("out-of-sequence")),
        "future request must taint instead of remaining invisible: {:?}",
        broker.taint
    );
}
