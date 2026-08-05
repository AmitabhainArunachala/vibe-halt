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
//! boundary logic that produces those types: spawning, deadline-polled
//! execution, bounded retained-output readback, and world binding (executable bytes
//! when resolvable, target OS/arch, declared artifacts and input files).
//! Direct-child reap after kill is best effort and can remain inside an OS
//! wait for an uninterruptible process; D2 makes no hard end-to-end latency
//! or process-tree containment claim.
//!
//! Known, cited scope limits (not silently closed):
//! - "initial filesystem/fixtures" binding is the freshly created empty
//!   workspace case only; fixture-seeded workspaces are a later package's
//!   concern (C6 reference profile).
//! - executable bytes are observed immediately before spawn. Because this
//!   safe runner does not own a hostile-code filesystem/loader boundary,
//!   replacement in the remaining observation-to-exec race stays covered
//!   by the open filesystem/loader capability channels rather than being
//!   misrepresented as D1 closure.
//! - `CapabilityChannel::WallClock`/`MonotonicClock`/etc. staying `Open`
//!   for every run is not a bug: this package implements no channel
//!   interposition. That is C7's (separately authorized, unsafe-helper)
//!   job; see `docs/prompts/VIBE_HALT_POST_AUDIT_TIER2_REACH_LONG_RUNNING_GOAL_2026-07-22.md`.

#![forbid(unsafe_code)]

pub mod capability;
pub mod cassette_v2;

pub use capability::{
    CapabilityChannel, CapabilityReceipt, ChannelStatus, DivergenceReport, EvidenceGrade,
    ExecutableIdentity, ProcessTreeState, StreamObservation, TerminationOutcome, CAPABILITY_SCHEMA,
    DIVERGENCE_REPORT_SCHEMA,
};
pub use cassette_v2::{
    CassetteV2, LlmRequestV2, TapeEntry, TransportReceipt, CASSETTE_SCHEMA_V2, TRANSPORT_SCHEMA,
};

use std::collections::BTreeMap;
use std::path::{Component, Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

use vh_trace::Trace;

pub const SANDBOX_SPEC_SCHEMA: &str = "vh-sandbox-spec-v2";
pub const CASSETTE_SCHEMA: &str = "vh-cassette-v1";
pub const RUN_RECORD_SCHEMA: &str = "vh-sandbox-run-v2";

/// The one published maximum byte size for a cassette file or a
/// child-visible request frame. Anything larger is rejected from its
/// size — before parsing and before an attacker-sized allocation.
pub const MAX_CASSETTE_BYTES: u64 = 1 << 20; // 1 MiB
/// Maximum host file admitted by `declare_input_file`. Declared inputs are
/// hashed before the execution deadline, so they must have their own hard
/// boundary instead of relying on available memory or a blocking pathname.
pub const MAX_INPUT_FILE_BYTES: u64 = 16 << 20; // 16 MiB
/// Maximum executable image observed for launch identity. The OS loader
/// remains an open D2 capability, but controller observation itself must not
/// block on a FIFO or allocate an attacker-sized file.
pub const MAX_EXECUTABLE_BYTES: u64 = 128 << 20; // 128 MiB
/// Cooperative artifacts are derived from a bounded cassette response and
/// carry the same one-mebibyte ceiling. Applying the limit at sandbox
/// collection time prevents a child-controlled FIFO/device/symlink or huge
/// file from reaching a later receipt layer first.
pub const MAX_ARTIFACT_BYTES: u64 = MAX_CASSETTE_BYTES;
/// Hard public bounds revalidated at the execution boundary even when callers
/// mutate the public request structs instead of using their constructors.
pub const MAX_SANDBOX_DEADLINE: Duration = Duration::from_secs(300);
pub const MAX_CAPTURE_BYTES: usize = 16 << 20;
pub const MAX_STDIN_BYTES: usize = 16 << 20;
const MAX_SPEC_COLLECTION_ITEMS: usize = 1024;
const MAX_SPEC_TEXT_BYTES: usize = 1 << 20;

fn reject_link_or_traversal_components(path: &Path) -> Result<(), SandboxError> {
    if path.as_os_str().is_empty()
        || path
            .components()
            .any(|part| matches!(part, Component::ParentDir | Component::CurDir))
    {
        return Err(SandboxError::BoundaryFile("path-traversal"));
    }
    let mut current = PathBuf::new();
    for part in path.components() {
        current.push(part.as_os_str());
        match std::fs::symlink_metadata(&current) {
            Ok(metadata) if metadata.file_type().is_symlink() => {
                // macOS publishes `/tmp` and `/var` as system-owned aliases
                // into `/private`. They are unavoidable roots for the host
                // temp directory, not caller-planted receipt components.
                let trusted_macos_alias = cfg!(target_os = "macos")
                    && (current == Path::new("/tmp") || current == Path::new("/var"));
                if !trusted_macos_alias {
                    return Err(SandboxError::BoundaryFile("symlink"));
                }
            }
            Ok(_) => {}
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                return Err(SandboxError::BoundaryFile("missing-component"));
            }
            Err(_) => return Err(SandboxError::BoundaryFile("path-inspection")),
        }
    }
    Ok(())
}

/// Create a workspace one component at a time, refusing every pre-existing
/// symlink before the first write through it. This avoids `create_dir_all`'s
/// symlink-following behavior. A hostile same-user replacement between a
/// check and the next syscall remains represented by the open D2 filesystem
/// channel; safe Rust has no portable `openat` directory-handle API.
fn prepare_workspace_directory(path: &Path) -> Result<(), SandboxError> {
    if path.as_os_str().is_empty()
        || path
            .components()
            .any(|part| matches!(part, Component::ParentDir | Component::CurDir))
    {
        return Err(SandboxError::BoundaryFile("path-traversal"));
    }
    let mut current = PathBuf::new();
    for part in path.components() {
        current.push(part.as_os_str());
        match std::fs::symlink_metadata(&current) {
            Ok(metadata) if metadata.file_type().is_symlink() => {
                let trusted_macos_alias = cfg!(target_os = "macos")
                    && (current == Path::new("/tmp") || current == Path::new("/var"));
                if !trusted_macos_alias {
                    return Err(SandboxError::BoundaryFile("symlink"));
                }
            }
            Ok(metadata) if metadata.is_dir() => {}
            Ok(_) => return Err(SandboxError::BoundaryFile("non-directory-component")),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                create_private_directory(&current)
                    .map_err(|_| SandboxError::BoundaryFile("workspace-create-refused"))?;
            }
            Err(_) => return Err(SandboxError::BoundaryFile("path-inspection")),
        }
    }
    reject_link_or_traversal_components(path)
}

#[cfg(unix)]
fn create_private_directory(path: &Path) -> std::io::Result<()> {
    use std::os::unix::fs::DirBuilderExt;

    let mut builder = std::fs::DirBuilder::new();
    builder.mode(0o700).create(path)
}

#[cfg(not(unix))]
fn create_private_directory(path: &Path) -> std::io::Result<()> {
    std::fs::create_dir(path)
}

#[cfg(any(target_os = "linux", target_os = "android"))]
const BOUNDARY_OPEN_FLAGS: i32 = 0x20000 | 0x800; // O_NOFOLLOW | O_NONBLOCK
#[cfg(any(
    target_os = "macos",
    target_os = "ios",
    target_os = "freebsd",
    target_os = "openbsd",
    target_os = "netbsd",
    target_os = "dragonfly"
))]
const BOUNDARY_OPEN_FLAGS: i32 = 0x100 | 0x4; // O_NOFOLLOW | O_NONBLOCK

#[cfg(unix)]
fn open_boundary_file(path: &Path) -> Result<std::fs::File, SandboxError> {
    use std::os::unix::fs::OpenOptionsExt;

    #[cfg(not(any(
        target_os = "linux",
        target_os = "android",
        target_os = "macos",
        target_os = "ios",
        target_os = "freebsd",
        target_os = "openbsd",
        target_os = "netbsd",
        target_os = "dragonfly"
    )))]
    {
        let _ = path;
        return Err(SandboxError::BoundaryFile("unsupported-platform"));
    }

    #[cfg(any(
        target_os = "linux",
        target_os = "android",
        target_os = "macos",
        target_os = "ios",
        target_os = "freebsd",
        target_os = "openbsd",
        target_os = "netbsd",
        target_os = "dragonfly"
    ))]
    std::fs::OpenOptions::new()
        .read(true)
        .custom_flags(BOUNDARY_OPEN_FLAGS)
        .open(path)
        .map_err(|_| SandboxError::BoundaryFile("open-refused"))
}

/// Open a controller-owned subprocess capture file and retain that exact
/// regular-file handle through collection. The child can unlink or replace
/// the pathname in its writable workspace, but it cannot redirect the
/// controller's already-open handle to a FIFO, device, or symlink target.
#[cfg(unix)]
fn create_private_io_file(path: &Path) -> Result<std::fs::File, SandboxError> {
    use std::os::unix::fs::OpenOptionsExt;

    if let Some(parent) = path.parent() {
        reject_link_or_traversal_components(parent)?;
    }
    let file = std::fs::OpenOptions::new()
        .read(true)
        .write(true)
        .create_new(true)
        .mode(0o600)
        .custom_flags(BOUNDARY_OPEN_FLAGS)
        .open(path)
        .map_err(|_| SandboxError::BoundaryFile("capture-open-refused"))?;
    if !file
        .metadata()
        .map_err(|_| SandboxError::BoundaryFile("capture-metadata"))?
        .is_file()
    {
        return Err(SandboxError::BoundaryFile("capture-non-regular-file"));
    }
    Ok(file)
}

#[cfg(not(unix))]
fn create_private_io_file(path: &Path) -> Result<std::fs::File, SandboxError> {
    if let Some(parent) = path.parent() {
        reject_link_or_traversal_components(parent)?;
    }
    let file = std::fs::OpenOptions::new()
        .read(true)
        .write(true)
        .create_new(true)
        .open(path)
        .map_err(|_| SandboxError::BoundaryFile("capture-open-refused"))?;
    if !file
        .metadata()
        .map_err(|_| SandboxError::BoundaryFile("capture-metadata"))?
        .is_file()
    {
        return Err(SandboxError::BoundaryFile("capture-non-regular-file"));
    }
    Ok(file)
}

#[cfg(not(unix))]
fn open_boundary_file(path: &Path) -> Result<std::fs::File, SandboxError> {
    std::fs::OpenOptions::new()
        .read(true)
        .open(path)
        .map_err(|_| SandboxError::BoundaryFile("open-refused"))
}

/// Read a file that crosses a trust boundary (cassette bytes, child
/// request frames) with the size enforced BEFORE parsing and before any
/// unbounded allocation: the opened file's metadata is checked first
/// (a sparse oversize file is rejected without reading), then the read
/// goes through a bounded reader of at most `max + 1` bytes so a file
/// that grows between the metadata check and the read still overflows
/// into a typed rejection rather than memory.
pub fn read_bounded_file(path: &Path, max: u64) -> Result<Vec<u8>, SandboxError> {
    use std::io::Read;
    reject_link_or_traversal_components(path)?;
    let file = open_boundary_file(path)?;
    // Metadata is taken from the opened handle, never from a separately
    // resolved pathname. O_NONBLOCK makes opening a FIFO non-blocking; the
    // regular-file check then rejects it before any read.
    let metadata = file
        .metadata()
        .map_err(|_| SandboxError::BoundaryFile("metadata"))?;
    if !metadata.is_file() {
        return Err(SandboxError::BoundaryFile("non-regular-file"));
    }
    if metadata.len() > max {
        return Err(SandboxError::Oversized {
            max,
            actual: metadata.len(),
        });
    }
    let mut buf = Vec::new();
    file.take(max.saturating_add(1))
        .read_to_end(&mut buf)
        .map_err(SandboxError::Io)?;
    if buf.len() as u64 > max {
        return Err(SandboxError::Oversized {
            max,
            actual: buf.len() as u64,
        });
    }
    Ok(buf)
}

/// Observe an executable through the same symlink-following semantics the OS
/// loader uses, while retaining the opened-handle regular-file and size
/// bounds. This intentionally differs from receipt/input reads, which reject
/// links: `/bin/sh` and versioned interpreter links are valid launch paths.
#[cfg(unix)]
fn read_bounded_executable(path: &Path, max: u64) -> Result<Vec<u8>, SandboxError> {
    use std::io::Read;
    use std::os::unix::fs::OpenOptionsExt;

    let file = std::fs::OpenOptions::new()
        .read(true)
        .custom_flags(BOUNDARY_OPEN_FLAGS & !0x20000 & !0x100)
        .open(path)
        .map_err(|_| SandboxError::BoundaryFile("executable-open-refused"))?;
    let metadata = file
        .metadata()
        .map_err(|_| SandboxError::BoundaryFile("executable-metadata"))?;
    if !metadata.is_file() {
        return Err(SandboxError::BoundaryFile("executable-non-regular-file"));
    }
    if metadata.len() > max {
        return Err(SandboxError::Oversized {
            max,
            actual: metadata.len(),
        });
    }
    let mut bytes = Vec::new();
    file.take(max.saturating_add(1))
        .read_to_end(&mut bytes)
        .map_err(SandboxError::Io)?;
    if bytes.len() as u64 > max {
        return Err(SandboxError::Oversized {
            max,
            actual: bytes.len() as u64,
        });
    }
    Ok(bytes)
}

#[cfg(not(unix))]
fn read_bounded_executable(path: &Path, max: u64) -> Result<Vec<u8>, SandboxError> {
    read_bounded_file(path, max)
}

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
/// execution has one. It bounds live execution polling before kill, not the
/// OS's subsequent direct-child reap latency. `max_output_bytes` bounds the
/// bytes retained from each of stdout/stderr independently, not temporary
/// capture-file growth while the child is live.
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
                "sandbox live-execution deadline must be nonzero".into(),
            ));
        }
        if max_output_bytes == 0 {
            return Err(SandboxError::InvalidSpec(
                "sandbox budget max_output_bytes must be nonzero".into(),
            ));
        }
        if deadline > MAX_SANDBOX_DEADLINE {
            return Err(SandboxError::InvalidSpec(format!(
                "sandbox budget deadline exceeds the {}s hard bound",
                MAX_SANDBOX_DEADLINE.as_secs()
            )));
        }
        if max_output_bytes > MAX_CAPTURE_BYTES {
            return Err(SandboxError::InvalidSpec(format!(
                "sandbox budget max_output_bytes exceeds the {MAX_CAPTURE_BYTES}-byte retained-output hard bound"
            )));
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
    /// Inputs bound under a STABLE LOGICAL NAME with content binding, for
    /// material whose absolute staging path must never enter deterministic
    /// identity (e.g. a per-invocation unique workspace): logical name ->
    /// content digest.
    pub input_logical_files: BTreeMap<String, String>,
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
            input_logical_files: BTreeMap::new(),
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
        let artifact = ArtifactSpec::new(path)?;
        if self
            .artifacts
            .iter()
            .any(|existing| existing.path == artifact.path)
            || self.input_logical_files.contains_key(&artifact.path)
        {
            return Err(SandboxError::InvalidSpec(format!(
                "duplicate or colliding declared path {:?}",
                artifact.path
            )));
        }
        self.artifacts.push(artifact);
        self.artifacts.sort_by(|a, b| a.path.cmp(&b.path));
        Ok(self)
    }

    /// Bind a source/script or lockfile/dependency input into identity by
    /// reading and hashing it now, at declare time (it is a precondition
    /// the controller has available, not an output).
    pub fn declare_input_file(mut self, path: impl AsRef<Path>) -> Result<Self, SandboxError> {
        let path_ref = path.as_ref();
        let bytes = read_bounded_file(path_ref, MAX_INPUT_FILE_BYTES)?;
        let key = path_ref.display().to_string();
        if self.input_files.contains_key(&key) {
            return Err(SandboxError::InvalidSpec(format!(
                "duplicate declared input path {key:?}"
            )));
        }
        // Physical input identity is legacy FNV under the frozen v2 schema;
        // harden how bytes are acquired without silently changing existing
        // receipt identities.
        self.input_files.insert(key, fnv_hex(&bytes));
        Ok(self)
    }

    /// Bind input CONTENT under a stable logical name. Unlike
    /// [`SandboxSpec::declare_input_file`], no filesystem path enters
    /// identity, so an invocation-isolated workspace with a unique
    /// absolute staging path still yields the same deterministic
    /// identity for the same logical input bytes.
    pub fn declare_input_bytes(
        mut self,
        logical_name: impl Into<String>,
        bytes: &[u8],
    ) -> Result<Self, SandboxError> {
        let name = logical_name.into();
        validate_relative_path(&name)?;
        if self.input_logical_files.contains_key(&name)
            || self.artifacts.iter().any(|artifact| artifact.path == name)
        {
            return Err(SandboxError::InvalidSpec(format!(
                "duplicate or colliding declared path {name:?}"
            )));
        }
        self.input_logical_files
            .insert(name, vh_digest::sha256_hex(bytes));
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
        for (name, digest) in &self.input_logical_files {
            t.record(0, "input-logical", &format!("{name}={digest}"));
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
    /// Child-visible cassette transport receipt (C5). `None` = no
    /// cassette was attached — itself part of the identity via the
    /// spec's `cassette` field; legacy identities are unchanged.
    pub transport: Option<cassette_v2::TransportReceipt>,
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
        if let Some(transport) = &self.transport {
            t.record(0, "transport", &transport.identity_str());
        }
        t.hash_hex()
    }

    /// A tainted transport (miss, malformed frame, out-of-tape request,
    /// or unconsumed recorded entries) can never read as success: the
    /// caller must report UNCHECKED, not CLEAN/FINDINGS.
    pub fn transport_tainted(&self) -> bool {
        self.transport.as_ref().is_some_and(|t| t.tainted())
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
    run_once_inner(spec, workspace, None)
}

/// Run with a child-visible cassette transport (C5): the CHILD makes
/// each request through the file-mailbox protocol under
/// `.vh-sandbox-io/llm/` in its working directory, and the broker —
/// serviced inside the same single-threaded bounded wait loop that owns
/// the deadline — replays the ordered tape exact-match-or-miss. The
/// spec must already bind the cassette's identity
/// ([`SandboxSpec::with_cassette_identity`]); a mismatch is an error,
/// never a silent rebind.
pub fn run_once_with_cassette(
    spec: &SandboxSpec,
    workspace: &Path,
    cassette: &cassette_v2::CassetteV2,
) -> Result<RunRecord, SandboxError> {
    let cassette_bytes = cassette.file_bytes();
    if cassette_bytes.len() as u64 > MAX_CASSETTE_BYTES {
        return Err(SandboxError::Oversized {
            max: MAX_CASSETTE_BYTES,
            actual: cassette_bytes.len() as u64,
        });
    }
    match spec.cassette_identity.as_deref() {
        Some(bound) if bound == cassette.identity() => {}
        other => {
            return Err(SandboxError::Execution(format!(
                "spec cassette identity {:?} does not bind the supplied cassette {:?} — \
                 refusing to run with an unbound tape",
                other,
                cassette.identity()
            )))
        }
    }
    run_once_inner(spec, workspace, Some(cassette))
}

/// Own the sandbox's reserved transport directory for exactly one run. A
/// fixed child-visible name is part of the cassette protocol, so exclusivity
/// plus scoped cleanup prevents stale files from a previous run from becoming
/// inputs to the next one.
struct IoDirectoryLease {
    path: PathBuf,
}

impl Drop for IoDirectoryLease {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.path);
    }
}

/// Relative mailbox directory (under the child's cwd) for the
/// `vh-cassette-transport-v1` frame files. A protocol constant, not
/// configuration: the child SDK and broker agree on it by contract.
pub const LLM_MAILBOX_DIR: &str = ".vh-sandbox-io/llm";

fn run_once_inner(
    spec: &SandboxSpec,
    workspace: &Path,
    cassette: Option<&cassette_v2::CassetteV2>,
) -> Result<RunRecord, SandboxError> {
    validate_spec_at_run_boundary(spec)?;
    if cassette.is_none() && spec.cassette_identity.is_some() {
        return Err(SandboxError::InvalidSpec(
            "spec binds a cassette identity but no cassette transport was supplied".into(),
        ));
    }

    // Physical inputs are a live precondition, not a declare-time claim. A
    // caller can mutate or replace them after `declare_input_file`; observe the
    // same no-link regular bytes again immediately before any workspace write
    // or child spawn and require the frozen legacy digest.
    for (path, expected_digest) in &spec.input_files {
        let bytes = read_bounded_file(Path::new(path), MAX_INPUT_FILE_BYTES).map_err(|error| {
            SandboxError::InputMismatch {
                path: path.clone(),
                category: error.category(),
            }
        })?;
        if fnv_hex(&bytes) != *expected_digest {
            return Err(SandboxError::InputMismatch {
                path: path.clone(),
                category: "digest-mismatch",
            });
        }
    }

    // Logical inputs are not merely declarative identity fields: observe the
    // regular, no-link staged file immediately before preparing/spawning the
    // child and require its bytes to match the bound digest.
    for (name, expected_digest) in &spec.input_logical_files {
        let bytes =
            read_bounded_file(&workspace.join(name), MAX_CASSETTE_BYTES).map_err(|error| {
                SandboxError::InputMismatch {
                    path: name.clone(),
                    category: error.category(),
                }
            })?;
        if vh_digest::sha256_hex(&bytes) != *expected_digest {
            return Err(SandboxError::InputMismatch {
                path: name.clone(),
                category: "digest-mismatch",
            });
        }
    }

    prepare_workspace_directory(workspace)?;
    let io_dir = workspace.join(".vh-sandbox-io");
    create_private_directory(&io_dir)
        .map_err(|_| SandboxError::BoundaryFile("io-directory-not-exclusive"))?;
    let _io_lease = IoDirectoryLease {
        path: io_dir.clone(),
    };
    let stdin_path = io_dir.join("stdin.raw");
    let stdout_path = io_dir.join("stdout.raw");
    let stderr_path = io_dir.join("stderr.raw");
    // Materialize stdin before the execution deadline starts, then hand the
    // child a read-only regular-file descriptor. A controller-side
    // `ChildStdin::write_all` can block forever when a child never drains a
    // full pipe, preventing the deadline loop from ever running. A prepared
    // regular file preserves exact input bytes and EOF without any live
    // controller write for the child to backpressure.
    use std::io::Write;
    let mut stdin_writer = create_private_io_file(&stdin_path)?;
    stdin_writer
        .write_all(&spec.stdin)
        .map_err(SandboxError::Io)?;
    stdin_writer.flush().map_err(SandboxError::Io)?;
    stdin_writer.sync_all().map_err(SandboxError::Io)?;
    let stdin_file = open_boundary_file(&stdin_path)?;
    drop(stdin_writer);
    // Redirect to files rather than piping stdout/stderr: reading two
    // live pipes concurrently without deadlocking needs either OS-level
    // threads (denied even on this boundary crate — parallelism stays at
    // the multiverse boundary) or non-blocking file descriptors (the
    // platform-specific extension module for that is not part of this
    // crate's exemption). Files sidestep the deadlock entirely and let
    // the bounded wait loop below own the deadline.
    let stdout_file = create_private_io_file(&stdout_path)?;
    let stderr_file = create_private_io_file(&stderr_path)?;
    let stdout_child = stdout_file.try_clone().map_err(SandboxError::Io)?;
    let stderr_child = stderr_file.try_clone().map_err(SandboxError::Io)?;

    // Observe executable bytes before spawn, not after the child has had an
    // opportunity to replace its own path. This binds the run to the
    // controller-observed launch input. Filesystem and loader channels
    // remain Open because safe Rust cannot eliminate the final
    // observation-to-exec race against a hostile same-user writer.
    let executable = resolve_executable_identity(&spec.argv[0]);
    let started = Instant::now();
    let mut cmd = Command::new(&spec.argv[0]);
    cmd.args(&spec.argv[1..])
        .current_dir(workspace)
        .env_clear()
        .envs(spec.env.iter())
        .stdin(Stdio::from(stdin_file))
        .stdout(Stdio::from(stdout_child))
        .stderr(Stdio::from(stderr_child));

    let mut broker = match cassette {
        None => None,
        Some(cassette) => {
            let llm_dir = workspace.join(LLM_MAILBOX_DIR);
            create_private_directory(&llm_dir)
                .map_err(|_| SandboxError::BoundaryFile("mailbox-directory-not-exclusive"))?;
            Some(BrokerState::new(llm_dir, cassette))
        }
    };
    let (termination, process_tree) =
        execute_bounded(&mut cmd, &spec.budget, started, broker.as_mut())?;
    // Final drain: a request frame written just before child exit still
    // gets classified (served or tainted), never silently dropped.
    if let Some(broker) = broker.as_mut() {
        // The child is reaped, so drain only the finite declared tape plus
        // one extra slot that proves an out-of-tape request is tainted.
        for _ in 0..=broker.cassette.len() {
            broker.service();
            if broker.taint.is_some() {
                break;
            }
        }
    }
    let transport = broker.map(BrokerState::into_receipt);
    let wall_time = started.elapsed();

    let no_process_ran = matches!(termination, TerminationOutcome::SpawnFailed { .. });
    let ran_to_completion_or_signal =
        !no_process_ran && !matches!(termination, TerminationOutcome::TimedOut);

    let (stdout, stderr) = if no_process_ran {
        (empty_stream(), empty_stream())
    } else {
        (
            read_bounded_stream(stdout_file, spec.budget.max_output_bytes)?,
            read_bounded_stream(stderr_file, spec.budget.max_output_bytes)?,
        )
    };

    // Declared artifacts are a postcondition of the target actually
    // running to completion or a signal; a killed-by-deadline or
    // never-spawned run cannot be expected to have produced them. A
    // completed process, however, must satisfy every declaration even on
    // nonzero exit. Cooperative workloads that intentionally inspect a
    // failure therefore precreate their deterministic artifact rather than
    // weakening this global sandbox law.
    let artifacts = if ran_to_completion_or_signal {
        let mut artifacts = BTreeMap::new();
        for artifact in &spec.artifacts {
            let path = workspace.join(&artifact.path);
            let bytes = read_bounded_file(&path, MAX_ARTIFACT_BYTES).map_err(|error| {
                SandboxError::ArtifactBoundary {
                    path: artifact.path.clone(),
                    category: error.category(),
                }
            })?;
            artifacts.insert(artifact.path.clone(), fnv_hex(&bytes));
        }
        artifacts
    } else {
        BTreeMap::new()
    };

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
        transport,
        wall_time,
    })
}

fn validate_spec_at_run_boundary(spec: &SandboxSpec) -> Result<(), SandboxError> {
    if spec.argv.is_empty() || spec.argv.iter().any(|value| value.is_empty()) {
        return Err(SandboxError::InvalidSpec(
            "argv must contain at least one non-empty element".into(),
        ));
    }
    SandboxBudget::new(spec.budget.deadline, spec.budget.max_output_bytes)?;
    if spec.stdin.len() > MAX_STDIN_BYTES {
        return Err(SandboxError::InvalidSpec(format!(
            "stdin exceeds the {MAX_STDIN_BYTES}-byte hard bound"
        )));
    }
    if spec.argv.len() > MAX_SPEC_COLLECTION_ITEMS
        || spec.env.len() > MAX_SPEC_COLLECTION_ITEMS
        || spec.artifacts.len() > MAX_SPEC_COLLECTION_ITEMS
        || spec.input_files.len() > MAX_SPEC_COLLECTION_ITEMS
        || spec.input_logical_files.len() > MAX_SPEC_COLLECTION_ITEMS
    {
        return Err(SandboxError::InvalidSpec(
            "sandbox spec exceeds the collection-item hard bound".into(),
        ));
    }
    let mut text_bytes = 0usize;
    for argument in &spec.argv {
        if argument.contains('\0') {
            return Err(SandboxError::InvalidSpec("argv contains a nul byte".into()));
        }
        text_bytes = text_bytes.saturating_add(argument.len());
    }
    for (key, value) in &spec.env {
        if key.is_empty() || key.contains('=') || key.contains('\0') || value.contains('\0') {
            return Err(SandboxError::InvalidSpec(
                "environment contains an invalid key or nul byte".into(),
            ));
        }
        text_bytes = text_bytes
            .saturating_add(key.len())
            .saturating_add(value.len());
    }
    let mut declared_paths = std::collections::BTreeSet::new();
    for artifact in &spec.artifacts {
        validate_relative_path(&artifact.path)?;
        if !declared_paths.insert(artifact.path.as_str()) {
            return Err(SandboxError::InvalidSpec(
                "duplicate declared artifact path".into(),
            ));
        }
        text_bytes = text_bytes.saturating_add(artifact.path.len());
    }
    for (name, digest) in &spec.input_logical_files {
        validate_relative_path(name)?;
        if !declared_paths.insert(name.as_str()) {
            return Err(SandboxError::InvalidSpec(
                "colliding logical input/artifact path".into(),
            ));
        }
        if digest.len() != 64
            || !digest.bytes().all(|byte| byte.is_ascii_hexdigit())
            || digest.bytes().any(|byte| byte.is_ascii_uppercase())
        {
            return Err(SandboxError::InvalidSpec(
                "logical input digest must be lowercase SHA-256".into(),
            ));
        }
        text_bytes = text_bytes
            .saturating_add(name.len())
            .saturating_add(digest.len());
    }
    for (path, digest) in &spec.input_files {
        if path.is_empty()
            || path.contains('\0')
            || digest.len() != 32
            || !digest.bytes().all(|byte| byte.is_ascii_hexdigit())
            || digest.bytes().any(|byte| byte.is_ascii_uppercase())
        {
            return Err(SandboxError::InvalidSpec(
                "physical input path/digest is malformed".into(),
            ));
        }
        text_bytes = text_bytes
            .saturating_add(path.len())
            .saturating_add(digest.len());
    }
    for identity in [&spec.cassette_identity, &spec.supervisor_identity]
        .into_iter()
        .flatten()
    {
        if identity.is_empty() || identity.contains('\0') {
            return Err(SandboxError::InvalidSpec(
                "bound identity must be non-empty and nul-free".into(),
            ));
        }
        text_bytes = text_bytes.saturating_add(identity.len());
    }
    if text_bytes > MAX_SPEC_TEXT_BYTES {
        return Err(SandboxError::InvalidSpec(format!(
            "sandbox spec text exceeds the {MAX_SPEC_TEXT_BYTES}-byte hard bound"
        )));
    }
    Ok(())
}

/// Single-threaded cassette broker: serviced from inside the bounded
/// wait loop, so the deadline still owns every wait. The child writes
/// `req-<N>` atomically (temp + rename); the broker answers with
/// `resp-<N>` the same way. Sequence is strict from 0; the recorded
/// entry at position N must digest-match request N (exact-match-or-miss
/// over ordered history — repeated identical requests consume distinct
/// entries). Every violation taints; a taint response frame
/// (`transport-error …`) tells the child to fail fast instead of
/// hanging.
struct BrokerState<'a> {
    dir: std::path::PathBuf,
    cassette: &'a cassette_v2::CassetteV2,
    next_seq: usize,
    served: Vec<String>,
    taint: Option<String>,
}

impl<'a> BrokerState<'a> {
    fn new(dir: std::path::PathBuf, cassette: &'a cassette_v2::CassetteV2) -> Self {
        BrokerState {
            dir,
            cassette,
            next_seq: 0,
            served: Vec::new(),
            taint: None,
        }
    }

    fn set_taint(&mut self, reason: String) {
        if self.taint.is_none() {
            self.taint = Some(reason);
        }
    }

    /// Service at most one currently visible request frame. Returning after
    /// one frame guarantees the owning wait loop re-checks its execution
    /// deadline between requests; after any taint, no further attacker-made
    /// path is inspected. A broker-side I/O failure is itself a taint.
    fn service(&mut self) {
        if self.taint.is_some() {
            return;
        }
        let req_path = self.dir.join(format!("req-{}", self.next_seq));
        match std::fs::symlink_metadata(&req_path) {
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                self.taint_future_or_malformed_request();
                return;
            }
            Err(_) => {
                self.set_taint(format!("unreadable request {}: metadata", self.next_seq));
                return;
            }
            Ok(_) => {}
        }
        let bytes = match read_bounded_file(&req_path, MAX_CASSETTE_BYTES) {
            Ok(bytes) => bytes,
            Err(SandboxError::Oversized { max, .. }) => {
                self.set_taint(format!(
                    "request {} exceeds the {}-byte bound",
                    self.next_seq, max
                ));
                return;
            }
            Err(e) => {
                self.set_taint(format!(
                    "unreadable request {}: {}",
                    self.next_seq,
                    e.category()
                ));
                return;
            }
        };
        let mut served_digest: Option<String> = None;
        let reply: Vec<u8> = match cassette_v2::LlmRequestV2::parse_detailed(&bytes) {
            Err(e) => {
                // Redacted category only: the frame crosses a trust
                // boundary and may carry attacker-controlled bytes.
                self.set_taint(format!(
                    "malformed request {}: {}",
                    self.next_seq,
                    e.category()
                ));
                b"transport-error malformed\n".to_vec()
            }
            Ok(request) if request.canonical_bytes() != bytes => {
                self.set_taint(format!("noncanonical request {}", self.next_seq));
                b"transport-error noncanonical\n".to_vec()
            }
            Ok(request) => {
                let digest = request.digest();
                match self.cassette.entry(self.next_seq) {
                    None => {
                        self.set_taint(format!(
                            "request {} beyond the recorded tape (digest {digest})",
                            self.next_seq
                        ));
                        format!("transport-error miss {digest}\n").into_bytes()
                    }
                    Some((recorded, entry)) => {
                        if recorded.digest() == digest {
                            served_digest = Some(digest);
                            entry.response_frame()
                        } else {
                            self.set_taint(format!(
                                "request {} digest {digest} does not match recorded {}",
                                self.next_seq,
                                recorded.digest()
                            ));
                            format!("transport-error miss {digest}\n").into_bytes()
                        }
                    }
                }
            }
        };
        if let Err(category) = self.publish_response(&reply) {
            self.set_taint(format!(
                "cannot answer request {}: {category}",
                self.next_seq
            ));
            return;
        }
        if let Some(digest) = served_digest {
            self.served.push(digest);
        }
        self.next_seq += 1;
    }

    fn taint_future_or_malformed_request(&mut self) {
        let entries = match std::fs::read_dir(&self.dir) {
            Ok(entries) => entries,
            Err(_) => {
                self.set_taint("cannot enumerate request mailbox".into());
                return;
            }
        };
        let entry_bound = self.cassette.len().saturating_mul(2).saturating_add(4);
        for (index, entry) in entries.enumerate() {
            if index >= entry_bound {
                self.set_taint("request mailbox exceeds the entry-count bound".into());
                return;
            }
            let Ok(entry) = entry else {
                self.set_taint("cannot enumerate request mailbox entry".into());
                return;
            };
            let Some(name) = entry.file_name().to_str().map(str::to_string) else {
                self.set_taint("request mailbox contains a non-UTF-8 entry".into());
                return;
            };
            let Some(suffix) = name.strip_prefix("req-") else {
                continue;
            };
            if suffix.ends_with(".tmp") {
                continue;
            }
            match suffix.parse::<usize>() {
                Ok(sequence) if sequence <= self.next_seq => {}
                Ok(_) => {
                    self.set_taint(format!(
                        "out-of-sequence request visible before {}",
                        self.next_seq
                    ));
                    return;
                }
                Err(_) => {
                    self.set_taint("malformed request filename".into());
                    return;
                }
            }
        }
    }

    fn publish_response(&self, reply: &[u8]) -> Result<(), &'static str> {
        use std::io::Write;

        let tmp = self.dir.join(format!("resp-{}.tmp", self.next_seq));
        let final_path = self.dir.join(format!("resp-{}", self.next_seq));
        let mut file = create_private_io_file(&tmp).map_err(|_| "temp-create-refused")?;
        file.write_all(reply).map_err(|_| "temp-write")?;
        file.flush().map_err(|_| "temp-flush")?;
        std::fs::hard_link(&tmp, &final_path).map_err(|_| "final-exists-or-link-failed")?;
        let observed = read_bounded_file(&final_path, MAX_CASSETTE_BYTES)
            .map_err(|_| "published-read-refused")?;
        if observed != reply {
            return Err("published-byte-mismatch");
        }
        std::fs::remove_file(&tmp).map_err(|_| "temp-cleanup")?;
        Ok(())
    }

    fn into_receipt(self) -> cassette_v2::TransportReceipt {
        let unconsumed = self.cassette.len().saturating_sub(self.next_seq) as u64;
        cassette_v2::TransportReceipt {
            served: self.served,
            unconsumed,
            taint: self.taint,
        }
    }
}

fn empty_stream() -> StreamObservation {
    StreamObservation {
        digest: fnv_hex(&[]),
        byte_len: 0,
        truncated: false,
    }
}

/// Spawn and wait for the direct child with an explicit deadline. Stdin is
/// already a prepared regular-file descriptor, so no live input write can
/// block entry into this loop. On expiry, the child is killed and the direct
/// child is waited on (reaped) before returning. Descendant/process-group
/// cleanup cannot be proven this way in safe Rust; that stays represented by
/// `CapabilityChannel::ThreadsForksExecDescendants` remaining `Open` on
/// the receipt, not by anything returned here.
fn execute_bounded(
    cmd: &mut Command,
    budget: &SandboxBudget,
    started: Instant,
    mut broker: Option<&mut BrokerState<'_>>,
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

    loop {
        let status = match child.try_wait() {
            Ok(status) => status,
            Err(error) => {
                // Once spawn succeeds every return path makes a best-effort
                // kill and reap. Do not let a transient observation error drop
                // a live Child handle without cleanup.
                let _ = child.kill();
                let _ = child.wait();
                return Err(SandboxError::Io(error));
            }
        };
        match status {
            Some(status) => {
                return Ok((
                    classify_exit_status(&status),
                    ProcessTreeState::DirectChildReaped,
                ));
            }
            None => {
                if started.elapsed() >= budget.deadline {
                    let kill_error = child.kill().err();
                    let process_tree = match child.wait() {
                        Ok(_) => ProcessTreeState::DirectChildReaped,
                        Err(e) => ProcessTreeState::DirectChildReapFailed {
                            message: e.to_string(),
                        },
                    };
                    if let Some(error) = kill_error {
                        // `wait` above still reaps an already-exited child. If
                        // kill genuinely failed while it remained live, the
                        // wait owns that direct child rather than abandoning it.
                        if !matches!(process_tree, ProcessTreeState::DirectChildReaped) {
                            return Err(SandboxError::Io(error));
                        }
                    }
                    return Ok((TerminationOutcome::TimedOut, process_tree));
                }
                // The cassette broker (C5) is serviced from THIS loop:
                // one thread owns the deadline, the child wait, and the
                // mailbox — no pipes, no threads, no platform extension.
                if let Some(broker) = broker.as_deref_mut() {
                    broker.service();
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
fn read_bounded_stream(
    mut file: std::fs::File,
    cap: usize,
) -> Result<StreamObservation, SandboxError> {
    use std::io::{Read, Seek, SeekFrom};
    let metadata = file.metadata().map_err(SandboxError::Io)?;
    if !metadata.is_file() {
        return Err(SandboxError::BoundaryFile("capture-non-regular-file"));
    }
    let initial_len = metadata.len();
    file.seek(SeekFrom::Start(0)).map_err(SandboxError::Io)?;
    let mut buf = Vec::new();
    (&mut file)
        .take((cap as u64).saturating_add(1))
        .read_to_end(&mut buf)
        .map_err(SandboxError::Io)?;
    let observed_len = buf.len() as u64;
    let final_len = file.metadata().map_err(SandboxError::Io)?.len();
    let byte_len = initial_len.max(final_len).max(observed_len);
    if buf.len() > cap {
        buf.truncate(cap);
    }
    Ok(StreamObservation {
        digest: fnv_hex(&buf),
        byte_len,
        truncated: byte_len > cap as u64,
    })
}

/// Resolve an absolute `argv[0]` to a concrete file and hash its bytes
/// immediately before spawn. Every non-absolute spelling stays honestly
/// `Unresolved`: `Command::current_dir(workspace)` can change how a relative
/// program path is resolved, and bare names use platform `PATH` search.
/// Filesystem and loader capability channels remain Open: this observation is
/// not a hostile-race-proof `fexecve` equivalent.
fn resolve_executable_identity(argv0: &str) -> ExecutableIdentity {
    let path = Path::new(argv0);
    if path.is_absolute() {
        match read_bounded_executable(path, MAX_EXECUTABLE_BYTES) {
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
    ArtifactBoundary {
        path: String,
        category: &'static str,
    },
    InputMismatch {
        path: String,
        category: &'static str,
    },
    /// Stable category for a refused trust-boundary path. It deliberately
    /// carries no caller-selected pathname or OS diagnostic text.
    BoundaryFile(&'static str),
    /// A trust-boundary input exceeded the published byte bound. The
    /// message carries sizes and the controller-supplied path only —
    /// never attacker-controlled content.
    Oversized {
        max: u64,
        actual: u64,
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
            Self::ArtifactBoundary { path, category } => {
                write!(f, "artifact {path} refused: {category}")
            }
            Self::InputMismatch { path, category } => {
                write!(f, "logical input {path} refused: {category}")
            }
            Self::BoundaryFile(category) => write!(f, "boundary file refused: {category}"),
            Self::Oversized { max, actual } => {
                write!(
                    f,
                    "input exceeds the {max}-byte bound (actual size {actual} bytes)"
                )
            }
            Self::Io(e) => write!(f, "sandbox io error: {e}"),
        }
    }
}

impl std::error::Error for SandboxError {}

impl SandboxError {
    /// Stable attacker-content-free category suitable for a bounded boundary
    /// diagnostic or taint record.
    pub fn category(&self) -> &'static str {
        match self {
            Self::InvalidSpec(_) => "invalid-spec",
            Self::Execution(_) => "execution",
            Self::ArtifactRead { .. } => "artifact-read",
            Self::ArtifactBoundary { category, .. }
            | Self::InputMismatch { category, .. }
            | Self::BoundaryFile(category) => category,
            Self::Oversized { .. } => "oversized",
            Self::Io(_) => "io",
        }
    }
}

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
    if p.components().next().is_some_and(
        |component| matches!(component, Component::Normal(name) if name == ".vh-sandbox-io"),
    ) {
        return Err(SandboxError::InvalidSpec(
            "declared paths may not overlap the reserved .vh-sandbox-io namespace".into(),
        ));
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
