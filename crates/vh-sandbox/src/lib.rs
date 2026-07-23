//! vh-sandbox — Tier-2 D2 subprocess sandbox MVP; D1 is a future backend.
//!
//! This crate is deliberately a **boundary crate**: it owns subprocess
//! execution, environment scrubbing, artifact reads, and LLM cassette replay.
//! It is not part of the Tier-1 deterministic kernel. Its identities are
//! deterministic renderings of specs and observations; wall time and host I/O
//! are boundary telemetry only and never enter identity digests.
//!
//! `capability` (see [`capability`]) owns the sealed capability receipt, the
//! exhaustive channel inventory, the exact termination taxonomy, and the
//! raw-count divergence report. This file owns the actual subprocess
//! boundary logic that produces those types: spawning, a bounded
//! deadline with no unbounded wait, bounded output capture, and world
//! binding (executable bytes when resolvable, target OS/arch, declared
//! artifacts and input files).
//!
//! Known, cited scope limits (not silently closed):
//! - stdin writes are not themselves deadline-bounded — a child that
//!   never reads stdin and never exits can still block the write inside
//!   `run_once` past the configured deadline. Tracked here, not hidden.
//! - "initial filesystem/fixtures" binding is the freshly created empty
//!   workspace case only; fixture-seeded workspaces are a later package's
//!   concern (C6 reference profile).
//! - `CapabilityChannel::WallClock`/`MonotonicClock`/etc. staying `Open`
//!   for every run is not a bug: this package implements no channel
//!   interposition. That is C7's (separately authorized, unsafe-helper)
//!   job; see `docs/prompts/VIBE_HALT_POST_AUDIT_TIER2_REACH_LONG_RUNNING_GOAL_2026-07-22.md`.

#![forbid(unsafe_code)]

pub mod capability;

pub use capability::{
    CapabilityChannel, CapabilityReceipt, ChannelStatus, DivergenceReport, EvidenceGrade,
    ExecutableIdentity, ProcessTreeState, StreamObservation, TerminationOutcome, CAPABILITY_SCHEMA,
    DIVERGENCE_REPORT_SCHEMA,
};

use std::collections::BTreeMap;
use std::path::{Component, Path};
use std::process::{Child, Command, Stdio};
use std::time::{Duration, Instant};

use vh_trace::Trace;

pub const SANDBOX_SPEC_SCHEMA: &str = "vh-sandbox-spec-v2";
pub const CASSETTE_SCHEMA: &str = "vh-cassette-v1";
pub const RUN_RECORD_SCHEMA: &str = "vh-sandbox-run-v2";

/// Compile-time target triple pieces used as boundary-wide world
/// identity. This is the *build* target, not a live `uname` probe: this
/// crate's determinism-denylist exemption
/// (`scripts/check_determinism_denylist.py`) does not cover live
/// environment-variable reads or platform-specific extension traits, and
/// `cfg!` is a language-level compile-time construct rather than either
/// of those, so it stays inside the allowed surface.
const TARGET_OS: &str = if cfg!(target_os = "linux") {
    "linux"
} else if cfg!(target_os = "macos") {
    "macos"
} else if cfg!(target_os = "windows") {
    "windows"
} else {
    "unknown-os"
};

const TARGET_ARCH: &str = if cfg!(target_arch = "x86_64") {
    "x86_64"
} else if cfg!(target_arch = "aarch64") {
    "aarch64"
} else {
    "unknown-arch"
};

/// Reason recorded against every channel in a freshly produced capability
/// receipt. Uniform on purpose: nothing in this package closes any
/// channel, so there is exactly one honest reason to give.
const OPEN_CHANNEL_REASON: &str =
    "not controlled or replayed by the Tier-2/D2 safe-Rust subprocess runner in this package; \
     channel closure requires a separately authorized unsafe-helper package (C7)";

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

/// Explicit controller-configured execution budget. Every safe-runner
/// execution has one; there is no unbounded wait. `max_output_bytes`
/// bounds each of stdout/stderr independently.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SandboxBudget {
    pub deadline: Duration,
    pub max_output_bytes: usize,
}

impl SandboxBudget {
    pub const DEFAULT_DEADLINE: Duration = Duration::from_secs(30);
    pub const DEFAULT_MAX_OUTPUT_BYTES: usize = 1 << 20;

    pub fn new(deadline: Duration, max_output_bytes: usize) -> Result<Self, SandboxError> {
        if deadline.is_zero() {
            return Err(SandboxError::InvalidSpec(
                "sandbox budget deadline must be nonzero — an unbounded wait is never permitted"
                    .into(),
            ));
        }
        if max_output_bytes == 0 {
            return Err(SandboxError::InvalidSpec(
                "sandbox budget max_output_bytes must be nonzero".into(),
            ));
        }
        Ok(Self {
            deadline,
            max_output_bytes,
        })
    }
}

impl Default for SandboxBudget {
    fn default() -> Self {
        Self {
            deadline: Self::DEFAULT_DEADLINE,
            max_output_bytes: Self::DEFAULT_MAX_OUTPUT_BYTES,
        }
    }
}

/// Explicit subprocess universe spec. Environment is allowlist-only; pinned
/// defaults are applied by [`SandboxSpec::new`]. This is a *request*
/// descriptor only: it has no field or method that can assert any
/// [`CapabilityChannel`] closed. The sealed receipt is produced solely by
/// the runner (see [`run_once`]) and lives on [`RunRecord`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SandboxSpec {
    pub argv: Vec<String>,
    pub stdin: Vec<u8>,
    pub env: BTreeMap<String, String>,
    pub artifacts: Vec<ArtifactSpec>,
    /// Source/script and lockfile/dependency inputs bound into identity at
    /// declare time: path -> content digest.
    pub input_files: BTreeMap<String, String>,
    pub budget: SandboxBudget,
    /// Bound when a real child-visible cassette transport (C5) supplies
    /// one; `None` here means "no cassette used", which is itself part of
    /// identity, not an absence of a field.
    pub cassette_identity: Option<String>,
    /// Bound when a separately admitted unsafe-helper supervisor (C7) is
    /// in the loop; `None` in this package always.
    pub supervisor_identity: Option<String>,
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
            input_files: BTreeMap::new(),
            budget: SandboxBudget::default(),
            cassette_identity: None,
            supervisor_identity: None,
        })
    }

    pub fn with_stdin(mut self, stdin: impl Into<Vec<u8>>) -> Self {
        self.stdin = stdin.into();
        self
    }

    pub fn with_budget(mut self, budget: SandboxBudget) -> Self {
        self.budget = budget;
        self
    }

    pub fn with_cassette_identity(mut self, identity: impl Into<String>) -> Self {
        self.cassette_identity = Some(identity.into());
        self
    }

    pub fn with_supervisor_identity(mut self, identity: impl Into<String>) -> Self {
        self.supervisor_identity = Some(identity.into());
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

    /// Bind a source/script or lockfile/dependency input into identity by
    /// reading and hashing it now, at declare time (it is a precondition
    /// the controller has available, not an output).
    pub fn declare_input_file(mut self, path: impl AsRef<Path>) -> Result<Self, SandboxError> {
        let path_ref = path.as_ref();
        let bytes = std::fs::read(path_ref).map_err(|source| SandboxError::ArtifactRead {
            path: path_ref.display().to_string(),
            source,
        })?;
        self.input_files
            .insert(path_ref.display().to_string(), fnv_hex(&bytes));
        Ok(self)
    }

    pub fn identity(&self) -> String {
        let mut t = Trace::new();
        t.record(0, "schema", SANDBOX_SPEC_SCHEMA);
        t.record(0, "target-os", TARGET_OS);
        t.record(0, "target-arch", TARGET_ARCH);
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
        for (path, digest) in &self.input_files {
            t.record(0, "input-file", &format!("{path}={digest}"));
        }
        t.record(
            0,
            "budget",
            &format!(
                "deadline_ms={} max_output_bytes={}",
                self.budget.deadline.as_millis(),
                self.budget.max_output_bytes
            ),
        );
        t.record(
            0,
            "cassette",
            self.cassette_identity.as_deref().unwrap_or("none"),
        );
        t.record(
            0,
            "supervisor",
            self.supervisor_identity.as_deref().unwrap_or("none"),
        );
        t.hash_hex()
    }
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

/// Complete public observation of one subprocess run: the controller's
/// sealed capability receipt plus exact termination, process-tree,
/// stream, artifact, and world identity. `wall_time` is boundary
/// telemetry and is intentionally excluded from [`RunRecord::identity`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RunRecord {
    pub spec_identity: String,
    pub target_os: &'static str,
    pub target_arch: &'static str,
    pub executable: ExecutableIdentity,
    pub termination: TerminationOutcome,
    pub process_tree: ProcessTreeState,
    pub stdout: StreamObservation,
    pub stderr: StreamObservation,
    pub artifacts: BTreeMap<String, String>,
    pub capability: CapabilityReceipt,
    pub wall_time: Duration,
}

impl RunRecord {
    pub fn evidence_grade(&self) -> EvidenceGrade {
        self.capability.evidence_grade()
    }

    pub fn identity(&self) -> String {
        let mut t = Trace::new();
        t.record(0, "schema", RUN_RECORD_SCHEMA);
        t.record(0, "spec", &self.spec_identity);
        t.record(0, "target-os", self.target_os);
        t.record(0, "target-arch", self.target_arch);
        t.record(0, "executable", &self.executable.as_identity_str());
        t.record(0, "termination", &self.termination.as_identity_str());
        t.record(0, "process-tree", &self.process_tree.as_identity_str());
        t.record(0, "stdout", &self.stdout.as_identity_str());
        t.record(0, "stderr", &self.stderr.as_identity_str());
        for (path, digest) in &self.artifacts {
            t.record(0, "artifact", &format!("{path}={digest}"));
        }
        t.record(0, "capability", &self.capability.identity());
        t.hash_hex()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SandboxCampaign {
    pub first: RunRecord,
    pub second: RunRecord,
}

impl SandboxCampaign {
    /// Raw-count divergence evidence for this one run-twice pair. See
    /// [`DivergenceReport::from_identity_pairs`] to aggregate many pairs
    /// into one declared-suite report.
    pub fn divergence_report(&self) -> DivergenceReport {
        DivergenceReport::from_identity_pairs([(
            self.first.identity().as_str(),
            self.second.identity().as_str(),
        )])
    }

    pub fn verdict_line(&self) -> String {
        let grade = if self.first.evidence_grade() == EvidenceGrade::D1
            && self.second.evidence_grade() == EvidenceGrade::D1
        {
            EvidenceGrade::D1
        } else {
            EvidenceGrade::D2
        };
        self.divergence_report().verdict_line(grade)
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
    std::fs::create_dir_all(workspace).map_err(SandboxError::Io)?;
    let io_dir = workspace.join(".vh-sandbox-io");
    std::fs::create_dir_all(&io_dir).map_err(SandboxError::Io)?;
    let stdout_path = io_dir.join("stdout.raw");
    let stderr_path = io_dir.join("stderr.raw");
    // Redirect to files rather than piping stdout/stderr: reading two
    // live pipes concurrently without deadlocking needs either OS-level
    // threads (denied even on this boundary crate — parallelism stays at
    // the multiverse boundary) or non-blocking file descriptors (the
    // platform-specific extension module for that is not part of this
    // crate's exemption). Files sidestep the deadlock entirely and let
    // the bounded wait loop below own the deadline.
    let stdout_file = std::fs::File::create(&stdout_path).map_err(SandboxError::Io)?;
    let stderr_file = std::fs::File::create(&stderr_path).map_err(SandboxError::Io)?;

    let started = Instant::now();
    let mut cmd = Command::new(&spec.argv[0]);
    cmd.args(&spec.argv[1..])
        .current_dir(workspace)
        .env_clear()
        .envs(spec.env.iter())
        .stdin(Stdio::piped())
        .stdout(Stdio::from(stdout_file))
        .stderr(Stdio::from(stderr_file));

    let (termination, process_tree) =
        execute_bounded(&mut cmd, &spec.stdin, &spec.budget, started)?;
    let wall_time = started.elapsed();

    let no_process_ran = matches!(termination, TerminationOutcome::SpawnFailed { .. });
    let ran_to_completion_or_signal =
        !no_process_ran && !matches!(termination, TerminationOutcome::TimedOut);

    let (stdout, stderr) = if no_process_ran {
        (empty_stream(), empty_stream())
    } else {
        (
            read_bounded_stream(&stdout_path, spec.budget.max_output_bytes)?,
            read_bounded_stream(&stderr_path, spec.budget.max_output_bytes)?,
        )
    };

    // Declared artifacts are a postcondition of the target actually
    // running to completion or a signal; a killed-by-deadline or
    // never-spawned run cannot be expected to have produced them, so we
    // do not manufacture an artifact-read error that would mask the real
    // termination finding.
    let artifacts = if ran_to_completion_or_signal {
        let mut artifacts = BTreeMap::new();
        for artifact in &spec.artifacts {
            let path = workspace.join(&artifact.path);
            let bytes = std::fs::read(&path).map_err(|source| SandboxError::ArtifactRead {
                path: artifact.path.clone(),
                source,
            })?;
            artifacts.insert(artifact.path.clone(), fnv_hex(&bytes));
        }
        artifacts
    } else {
        BTreeMap::new()
    };

    let executable = resolve_executable_identity(&spec.argv[0]);

    Ok(RunRecord {
        spec_identity: spec.identity(),
        target_os: TARGET_OS,
        target_arch: TARGET_ARCH,
        executable,
        termination,
        process_tree,
        stdout,
        stderr,
        artifacts,
        capability: CapabilityReceipt::all_open(OPEN_CHANNEL_REASON),
        wall_time,
    })
}

fn empty_stream() -> StreamObservation {
    StreamObservation {
        digest: fnv_hex(&[]),
        byte_len: 0,
        truncated: false,
    }
}

/// Spawn, write stdin, and wait for the direct child with an explicit
/// deadline. No unbounded wait remains: on expiry, stdin is closed, the
/// child is killed, and the direct child is waited on (reaped) before
/// returning. Descendant/process-group cleanup cannot be proven this way
/// in safe Rust; that stays represented by
/// `CapabilityChannel::ThreadsForksExecDescendants` remaining `Open` on
/// the receipt, not by anything returned here.
fn execute_bounded(
    cmd: &mut Command,
    stdin_bytes: &[u8],
    budget: &SandboxBudget,
    started: Instant,
) -> Result<(TerminationOutcome, ProcessTreeState), SandboxError> {
    let mut child = match cmd.spawn() {
        Ok(child) => child,
        Err(e) => {
            return Ok((
                TerminationOutcome::SpawnFailed {
                    message: e.to_string(),
                },
                ProcessTreeState::NoChildProcess,
            ))
        }
    };

    write_stdin_best_effort(&mut child, stdin_bytes)?;
    // Close stdin so a child waiting on EOF is never blocked by the
    // controller. This does not by itself prove the pipe channel is
    // fully closed end-to-end (inherited descriptors stay a separate,
    // always-open channel).
    let _ = child.stdin.take();

    loop {
        match child.try_wait().map_err(SandboxError::Io)? {
            Some(status) => {
                return Ok((
                    classify_exit_status(&status),
                    ProcessTreeState::DirectChildReaped,
                ));
            }
            None => {
                if started.elapsed() >= budget.deadline {
                    child.kill().map_err(SandboxError::Io)?;
                    let process_tree = match child.wait() {
                        Ok(_) => ProcessTreeState::DirectChildReaped,
                        Err(e) => ProcessTreeState::DirectChildReapFailed {
                            message: e.to_string(),
                        },
                    };
                    return Ok((TerminationOutcome::TimedOut, process_tree));
                }
                // This crate's determinism-denylist exemption
                // (scripts/check_determinism_denylist.py) does not cover
                // OS-level threads, which are denied even on this
                // boundary crate, so there is no courteous sleep-based
                // poll interval available here. `spin_loop` only hints
                // the CPU to reduce busy-poll power; it does not sleep.
                // Deadlines in this package should stay small in tests
                // for exactly this reason — this is a documented MVP
                // cost, not a hidden default.
                std::hint::spin_loop();
            }
        }
    }
}

fn write_stdin_best_effort(child: &mut Child, stdin_bytes: &[u8]) -> Result<(), SandboxError> {
    if stdin_bytes.is_empty() {
        return Ok(());
    }
    use std::io::Write;
    match child.stdin.as_mut() {
        Some(stdin) => {
            // Best-effort: a child that exits early after only partially
            // (or never) reading stdin causes a write error here (broken
            // pipe). That is not itself a termination classification —
            // the wait loop observes the real outcome, not this write.
            let _ = stdin.write_all(stdin_bytes);
            Ok(())
        }
        None => Err(SandboxError::Execution(
            "child stdin was not available despite being piped".into(),
        )),
    }
}

/// Classify a completed `std::process::ExitStatus`. The platform-specific
/// extension trait needed for `ExitStatusExt::signal()`/`core_dumped()`
/// is not part of this crate's determinism-denylist exemption, so the
/// exact signal is instead recovered from `ExitStatus`'s own
/// already-permitted `std::process` `Display` rendering (verified
/// against this repo's pinned toolchain:
/// `"signal: {N} (SIGNAME)"`, optionally with a `"(core dumped)"` suffix
/// on platforms that report it). An unparseable rendering stays typed
/// `Unknown` — never guessed.
fn classify_exit_status(status: &std::process::ExitStatus) -> TerminationOutcome {
    if let Some(code) = status.code() {
        return TerminationOutcome::Exited(code);
    }
    let rendered = status.to_string();
    match rendered.strip_prefix("signal: ") {
        Some(rest) => {
            let digits: String = rest.chars().take_while(char::is_ascii_digit).collect();
            match digits.parse::<i32>() {
                Ok(signal) => {
                    // A positive "(core dumped)" observation is trustworthy
                    // on any platform that emits it; its *absence* is not
                    // treated as a confirmed non-dump, since not every
                    // platform's Display rendering reports this bit at
                    // all — staying None there is the honest choice.
                    let core_dumped = if rendered.contains("(core dumped)") {
                        Some(true)
                    } else {
                        None
                    };
                    TerminationOutcome::Signaled {
                        signal,
                        core_dumped,
                    }
                }
                Err(_) => TerminationOutcome::Unknown {
                    reason: format!("unparsed signal rendering: {rendered:?}"),
                },
            }
        }
        None => TerminationOutcome::Unknown {
            reason: format!("unclassified exit status rendering: {rendered:?}"),
        },
    }
}

/// Read a subprocess output stream bounded to `cap` bytes. `byte_len` in
/// the result is the true on-disk length even when more than `cap` bytes
/// were written; only the retained prefix is ever read into memory.
fn read_bounded_stream(path: &Path, cap: usize) -> Result<StreamObservation, SandboxError> {
    use std::io::Read;
    let byte_len = std::fs::metadata(path).map_err(SandboxError::Io)?.len();
    let file = std::fs::File::open(path).map_err(SandboxError::Io)?;
    let mut buf = Vec::new();
    file.take(cap as u64)
        .read_to_end(&mut buf)
        .map_err(SandboxError::Io)?;
    Ok(StreamObservation {
        digest: fnv_hex(&buf),
        byte_len,
        truncated: byte_len > cap as u64,
    })
}

/// Resolve `argv[0]` to a concrete file and hash its bytes when the
/// controller can do so without reimplementing platform `PATH` search
/// (i.e. the caller already gave a path containing a separator). A bare
/// command name relying on `PATH` search stays honestly `Unresolved`
/// rather than guessing which file the OS actually executed.
fn resolve_executable_identity(argv0: &str) -> ExecutableIdentity {
    if argv0.contains('/') {
        match std::fs::read(argv0) {
            Ok(bytes) => ExecutableIdentity::Resolved {
                path: argv0.to_string(),
                digest: fnv_hex(&bytes),
            },
            Err(_) => ExecutableIdentity::Unresolved {
                argv0: argv0.to_string(),
            },
        }
    } else {
        ExecutableIdentity::Unresolved {
            argv0: argv0.to_string(),
        }
    }
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
