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
    assert_eq!(first.exit_code, Some(0));
    assert_eq!(first.stdout_digest, second.stdout_digest);
    assert_eq!(first.artifacts, second.artifacts);
    assert_eq!(first.identity(), second.identity());
    assert_eq!(first.evidence_grade(), EvidenceGrade::D2);
}

#[test]
fn run_twice_reports_divergence_rate() {
    let root = temp_dir("twice");
    let spec = sh_spec("printf stable");
    let campaign = run_twice(&spec, &root.join("one"), &root.join("two")).unwrap();
    assert_eq!(campaign.divergence_rate(), 0.0);
    assert!(campaign.verdict_line().contains("tier=Tier-2 d-grade=D2"));
    assert!(campaign
        .verdict_line()
        .contains("evidence=run-twice agreement"));
}

#[test]
fn empty_honesty_ledger_is_rejected_for_subprocess_mvp() {
    let root = temp_dir("honesty");
    let mut spec = sh_spec("true");
    spec.unmanaged_channels.clear();
    let err = run_once(&spec, &root).unwrap_err().to_string();
    assert!(err.contains("honesty ledger cannot be empty"));
}
