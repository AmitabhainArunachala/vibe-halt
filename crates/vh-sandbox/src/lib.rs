//! vh-sandbox — Tier-2/D1 subprocess sandbox MVP.
//!
//! This crate is deliberately a **boundary crate**: it owns subprocess
//! execution, environment scrubbing, artifact reads, and LLM cassette replay.
//! It is not part of the Tier-1 deterministic kernel. Its identities are
//! deterministic renderings of specs and observations; wall time and host I/O
//! are boundary telemetry only and never enter identity digests.

#![forbid(unsafe_code)]

use std::collections::BTreeMap;
use std::path::{Component, Path};
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

use vh_trace::Trace;

pub const SANDBOX_SPEC_SCHEMA: &str = "vh-sandbox-spec-v1";
pub const CASSETTE_SCHEMA: &str = "vh-cassette-v1";
pub const RUN_RECORD_SCHEMA: &str = "vh-sandbox-run-v1";

/// Channels the MVP does not control. This ledger is part of every run
/// record so callers cannot silently promote opaque subprocess execution to
/// D1. D1 is reserved for future instrumented targets whose effect channels
/// are controlled and replayed exactly; this MVP is normally D2-honest.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum HonestyChannel {
    RealNetwork,
    RealClockSyscalls,
    FilesystemOutsideWorkspace,
    ThreadScheduling,
    OsProcessScheduler,
}

impl HonestyChannel {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::RealNetwork => "real_network",
            Self::RealClockSyscalls => "real_clock_syscalls",
            Self::FilesystemOutsideWorkspace => "filesystem_outside_workspace",
            Self::ThreadScheduling => "thread_scheduling",
            Self::OsProcessScheduler => "os_process_scheduler",
        }
    }
}

/// Tier-2 evidence grade. The MVP can honestly report D2 for arbitrary
/// subprocesses; D1 requires an empty unmanaged-channel ledger.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EvidenceGrade {
    D1,
    D2,
}

impl EvidenceGrade {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::D1 => "D1",
            Self::D2 => "D2",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ArtifactSpec {
    pub path: String,
}

impl ArtifactSpec {
    pub fn new(path: impl Into<String>) -> Result<Self, SandboxError> {
        let path = path.into();
        validate_relative_path(&path)?;
        Ok(Self { path })
    }
}

/// Explicit subprocess universe spec. Environment is allowlist-only; pinned
/// defaults are applied by [`SandboxSpec::new`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SandboxSpec {
    pub argv: Vec<String>,
    pub stdin: Vec<u8>,
    pub env: BTreeMap<String, String>,
    pub artifacts: Vec<ArtifactSpec>,
    pub unmanaged_channels: Vec<HonestyChannel>,
}

impl SandboxSpec {
    pub fn new(argv: Vec<String>) -> Result<Self, SandboxError> {
        if argv.is_empty() || argv.iter().any(|s| s.is_empty()) {
            return Err(SandboxError::InvalidSpec(
                "argv must contain at least one non-empty element".into(),
            ));
        }
        let mut env = BTreeMap::new();
        env.insert("LC_ALL".to_string(), "C".to_string());
        env.insert("PYTHONHASHSEED".to_string(), "0".to_string());
        env.insert("TZ".to_string(), "UTC".to_string());
        Ok(Self {
            argv,
            stdin: Vec::new(),
            env,
            artifacts: Vec::new(),
            unmanaged_channels: default_unmanaged_channels(),
        })
    }

    pub fn with_stdin(mut self, stdin: impl Into<Vec<u8>>) -> Self {
        self.stdin = stdin.into();
        self
    }

    pub fn allow_env(
        mut self,
        key: impl Into<String>,
        value: impl Into<String>,
    ) -> Result<Self, SandboxError> {
        let key = key.into();
        if key.is_empty() || key.contains('=') || key.contains('\0') {
            return Err(SandboxError::InvalidSpec(format!(
                "invalid environment key {key:?}"
            )));
        }
        self.env.insert(key, value.into());
        Ok(self)
    }

    pub fn declare_artifact(mut self, path: impl Into<String>) -> Result<Self, SandboxError> {
        self.artifacts.push(ArtifactSpec::new(path)?);
        self.artifacts.sort_by(|a, b| a.path.cmp(&b.path));
        self.artifacts.dedup_by(|a, b| a.path == b.path);
        Ok(self)
    }

    pub fn identity(&self) -> String {
        let mut t = Trace::new();
        t.record(0, "schema", SANDBOX_SPEC_SCHEMA);
        for (i, arg) in self.argv.iter().enumerate() {
            t.record(i as u64, "argv", arg);
        }
        t.record(0, "stdin", &fnv_hex(&self.stdin));
        for (k, v) in &self.env {
            t.record(0, "env", &format!("{k}={v}"));
        }
        for artifact in &self.artifacts {
            t.record(0, "artifact", &artifact.path);
        }
        for channel in &self.unmanaged_channels {
            t.record(0, "honesty", channel.as_str());
        }
        t.hash_hex()
    }

    pub fn evidence_grade(&self) -> Result<EvidenceGrade, SandboxError> {
        if self.unmanaged_channels.is_empty() {
            return Ok(EvidenceGrade::D1);
        }
        Ok(EvidenceGrade::D2)
    }
}

fn default_unmanaged_channels() -> Vec<HonestyChannel> {
    vec![
        HonestyChannel::RealNetwork,
        HonestyChannel::RealClockSyscalls,
        HonestyChannel::FilesystemOutsideWorkspace,
        HonestyChannel::ThreadScheduling,
        HonestyChannel::OsProcessScheduler,
    ]
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct LlmRequest {
    pub provider: String,
    pub model: String,
    pub messages: Vec<String>,
    pub params: BTreeMap<String, String>,
}

impl LlmRequest {
    pub fn digest(&self) -> String {
        let mut t = Trace::new();
        t.record(0, "schema", "vh-llm-request-v1");
        t.record(0, "provider", &self.provider);
        t.record(0, "model", &self.model);
        for (i, msg) in self.messages.iter().enumerate() {
            t.record(i as u64, "message", msg);
        }
        for (k, v) in &self.params {
            t.record(0, "param", &format!("{k}={v}"));
        }
        t.hash_hex()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CassetteEntry {
    pub response: Vec<u8>,
    pub boundary_telemetry: BTreeMap<String, String>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Cassette {
    entries: BTreeMap<String, CassetteEntry>,
}

impl Cassette {
    pub fn insert(&mut self, request: &LlmRequest, entry: CassetteEntry) {
        self.entries.insert(request.digest(), entry);
    }

    pub fn replay(&self, request: &LlmRequest) -> Result<Vec<u8>, CassetteMiss> {
        let digest = request.digest();
        self.entries
            .get(&digest)
            .map(|entry| entry.response.clone())
            .ok_or(CassetteMiss { digest })
    }

    pub fn identity(&self) -> String {
        let mut t = Trace::new();
        t.record(0, "schema", CASSETTE_SCHEMA);
        for (digest, entry) in &self.entries {
            t.record(0, "request", digest);
            t.record(0, "response", &fnv_hex(&entry.response));
            for (k, v) in &entry.boundary_telemetry {
                t.record(0, "telemetry", &format!("{digest}:{k}={v}"));
            }
        }
        t.hash_hex()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CassetteMiss {
    pub digest: String,
}

/// Complete public observation of one subprocess run. `wall_time` is boundary
/// telemetry and is intentionally excluded from [`RunRecord::identity`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RunRecord {
    pub spec_identity: String,
    pub exit_code: Option<i32>,
    pub stdout_digest: String,
    pub stderr_digest: String,
    pub artifacts: BTreeMap<String, String>,
    pub unmanaged_channels: Vec<HonestyChannel>,
    pub wall_time: Duration,
}

impl RunRecord {
    pub fn evidence_grade(&self) -> EvidenceGrade {
        if self.unmanaged_channels.is_empty() {
            EvidenceGrade::D1
        } else {
            EvidenceGrade::D2
        }
    }

    pub fn identity(&self) -> String {
        let mut t = Trace::new();
        t.record(0, "schema", RUN_RECORD_SCHEMA);
        t.record(0, "spec", &self.spec_identity);
        t.record(
            0,
            "exit",
            &self
                .exit_code
                .map_or_else(|| "signal-or-unknown".to_string(), |c| c.to_string()),
        );
        t.record(0, "stdout", &self.stdout_digest);
        t.record(0, "stderr", &self.stderr_digest);
        for (path, digest) in &self.artifacts {
            t.record(0, "artifact", &format!("{path}={digest}"));
        }
        for channel in &self.unmanaged_channels {
            t.record(0, "honesty", channel.as_str());
        }
        t.hash_hex()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SandboxCampaign {
    pub first: RunRecord,
    pub second: RunRecord,
}

impl SandboxCampaign {
    pub fn divergence_rate(&self) -> f64 {
        if self.first.identity() == self.second.identity() {
            0.0
        } else {
            1.0
        }
    }

    pub fn verdict_line(&self) -> String {
        let grade = if self.first.evidence_grade() == EvidenceGrade::D1
            && self.second.evidence_grade() == EvidenceGrade::D1
        {
            EvidenceGrade::D1
        } else {
            EvidenceGrade::D2
        };
        format!(
            "tier=Tier-2 d-grade={} divergence-rate={:.3} evidence=run-twice agreement (sampled falsifier — not proof)",
            grade.as_str(),
            self.divergence_rate()
        )
    }
}

pub fn run_twice(
    spec: &SandboxSpec,
    workspace_a: &Path,
    workspace_b: &Path,
) -> Result<SandboxCampaign, SandboxError> {
    Ok(SandboxCampaign {
        first: run_once(spec, workspace_a)?,
        second: run_once(spec, workspace_b)?,
    })
}

pub fn run_once(spec: &SandboxSpec, workspace: &Path) -> Result<RunRecord, SandboxError> {
    if spec.unmanaged_channels.is_empty() {
        return Err(SandboxError::InvalidSpec(
            "honesty ledger cannot be empty for the subprocess MVP unless every effect channel is controlled+replayed"
                .into(),
        ));
    }
    std::fs::create_dir_all(workspace).map_err(SandboxError::Io)?;
    let started = Instant::now();
    let mut cmd = Command::new(&spec.argv[0]);
    cmd.args(&spec.argv[1..])
        .current_dir(workspace)
        .env_clear()
        .envs(spec.env.iter())
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    let mut child = cmd.spawn().map_err(SandboxError::Io)?;
    if !spec.stdin.is_empty() {
        use std::io::Write;
        let stdin = child.stdin.as_mut().ok_or_else(|| {
            SandboxError::Execution("child stdin was not available despite being piped".into())
        })?;
        stdin.write_all(&spec.stdin).map_err(SandboxError::Io)?;
    }
    let out = child.wait_with_output().map_err(SandboxError::Io)?;
    let wall_time = started.elapsed();
    let mut artifacts = BTreeMap::new();
    for artifact in &spec.artifacts {
        let path = workspace.join(&artifact.path);
        let bytes = std::fs::read(&path).map_err(|source| SandboxError::ArtifactRead {
            path: artifact.path.clone(),
            source,
        })?;
        artifacts.insert(artifact.path.clone(), fnv_hex(&bytes));
    }
    Ok(RunRecord {
        spec_identity: spec.identity(),
        exit_code: out.status.code(),
        stdout_digest: fnv_hex(&out.stdout),
        stderr_digest: fnv_hex(&out.stderr),
        artifacts,
        unmanaged_channels: spec.unmanaged_channels.clone(),
        wall_time,
    })
}

#[derive(Debug)]
pub enum SandboxError {
    InvalidSpec(String),
    Execution(String),
    ArtifactRead {
        path: String,
        source: std::io::Error,
    },
    Io(std::io::Error),
}

impl std::fmt::Display for SandboxError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidSpec(s) => write!(f, "invalid sandbox spec: {s}"),
            Self::Execution(s) => write!(f, "sandbox execution failed: {s}"),
            Self::ArtifactRead { path, source } => {
                write!(f, "failed to read artifact {path}: {source}")
            }
            Self::Io(e) => write!(f, "sandbox io error: {e}"),
        }
    }
}

impl std::error::Error for SandboxError {}

impl From<std::io::Error> for SandboxError {
    fn from(value: std::io::Error) -> Self {
        Self::Io(value)
    }
}

fn validate_relative_path(path: &str) -> Result<(), SandboxError> {
    let p = Path::new(path);
    if p.is_absolute() || path.is_empty() || path.contains('\0') {
        return Err(SandboxError::InvalidSpec(format!(
            "artifact path must be non-empty, relative, and nul-free: {path:?}"
        )));
    }
    for c in p.components() {
        match c {
            Component::Normal(_) => {}
            _ => {
                return Err(SandboxError::InvalidSpec(format!(
                    "artifact path may not contain prefixes, roots, or parent traversal: {path:?}"
                )))
            }
        }
    }
    Ok(())
}

/// Local deterministic digest helper. Uses the same FNV-1a 128 core as the
/// v0 trace hash; deterministic, not cryptographic.
pub fn fnv_hex(bytes: &[u8]) -> String {
    const FNV128_OFFSET: u128 = 0x6c62_272e_07bb_0142_62b8_2175_6295_c58d;
    const FNV128_PRIME: u128 = 0x0000_0000_0100_0000_0000_0000_0000_013B;
    let mut state = FNV128_OFFSET;
    for &b in bytes {
        state ^= b as u128;
        state = state.wrapping_mul(FNV128_PRIME);
    }
    format!("{state:032x}")
}

#[cfg(test)]
mod tests;
