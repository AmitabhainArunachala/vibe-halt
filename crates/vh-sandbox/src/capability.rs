//! Sealed capability receipt, exhaustive channel inventory, exact
//! termination taxonomy, and raw-count divergence evidence for the
//! Tier-2/D2 subprocess sandbox (C4: truthful subprocess observation and
//! capability envelope).
//!
//! Nothing in this module performs subprocess execution or host I/O —
//! `crate::run_once`/`crate::run_twice` in `lib.rs` own that boundary
//! logic and produce these types from what they actually observed. Being
//! I/O-free means this file needs no determinism-denylist exemption at
//! all: `scripts/check_determinism_denylist.py` scans it with the full
//! pattern set and it is expected to pass every clock/env/net/thread/fs
//! rule the same as a kernel crate would.
//!
//! Doctrine this module enforces structurally, not just by convention:
//! - a [`CapabilityChannel`] is closed only by controller evidence, never
//!   by omission — `CapabilityReceipt::all_open` is the only constructor
//!   and it always seeds the complete, versioned inventory;
//! - two different signals, or an exact signal versus unknown, never
//!   collapse to the same [`TerminationOutcome::as_identity_str`] value.

use std::collections::BTreeMap;

use vh_trace::Trace;

/// Schema for the versioned channel inventory. Growing the inventory
/// (`CapabilityChannel::ALL`) is a schema bump.
pub const CAPABILITY_SCHEMA: &str = "vh-sandbox-capability-v1";
/// Schema for the raw-count divergence evidence aggregate.
pub const DIVERGENCE_REPORT_SCHEMA: &str = "vh-sandbox-divergence-v1";

/// The exhaustive, versioned inventory of effect channels a subprocess
/// sandbox run may or may not actually control. Every channel is always
/// present in a [`CapabilityReceipt`]; there is no "channel omitted means
/// closed" reading anywhere in this type.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum CapabilityChannel {
    NetworkDns,
    WallClock,
    MonotonicClock,
    CpuClock,
    VdsoTime,
    HardwareTime,
    EntropyDevices,
    Getrandom,
    HardwareRng,
    FilesystemContent,
    FilesystemMetadata,
    FilesystemOrder,
    FilesystemSpace,
    FilesystemLocks,
    FilesystemEscape,
    LoaderDependencies,
    AslrAddressOutput,
    ProcessThreadIdentity,
    HostIdentity,
    ProcSysDevAccess,
    Signals,
    Timers,
    ThreadsForksExecDescendants,
    InheritedFileDescriptors,
    IpcSharedMemory,
    AsyncIoUring,
    CpuFpFeatures,
    JitGcFinalizers,
    UnsupportedSyscallsEffects,
}

impl CapabilityChannel {
    /// The complete inventory, in declaration order. Pinning the exact
    /// length in a test (`CapabilityChannel::ALL.len()`) turns an
    /// accidental channel removal into a gate failure.
    pub const ALL: [CapabilityChannel; 29] = [
        Self::NetworkDns,
        Self::WallClock,
        Self::MonotonicClock,
        Self::CpuClock,
        Self::VdsoTime,
        Self::HardwareTime,
        Self::EntropyDevices,
        Self::Getrandom,
        Self::HardwareRng,
        Self::FilesystemContent,
        Self::FilesystemMetadata,
        Self::FilesystemOrder,
        Self::FilesystemSpace,
        Self::FilesystemLocks,
        Self::FilesystemEscape,
        Self::LoaderDependencies,
        Self::AslrAddressOutput,
        Self::ProcessThreadIdentity,
        Self::HostIdentity,
        Self::ProcSysDevAccess,
        Self::Signals,
        Self::Timers,
        Self::ThreadsForksExecDescendants,
        Self::InheritedFileDescriptors,
        Self::IpcSharedMemory,
        Self::AsyncIoUring,
        Self::CpuFpFeatures,
        Self::JitGcFinalizers,
        Self::UnsupportedSyscallsEffects,
    ];

    pub fn as_str(self) -> &'static str {
        match self {
            Self::NetworkDns => "network_dns",
            Self::WallClock => "wall_clock",
            Self::MonotonicClock => "monotonic_clock",
            Self::CpuClock => "cpu_clock",
            Self::VdsoTime => "vdso_time",
            Self::HardwareTime => "hardware_time",
            Self::EntropyDevices => "entropy_devices",
            Self::Getrandom => "os_random_syscall",
            Self::HardwareRng => "hardware_rng",
            Self::FilesystemContent => "filesystem_content",
            Self::FilesystemMetadata => "filesystem_metadata",
            Self::FilesystemOrder => "filesystem_order",
            Self::FilesystemSpace => "filesystem_space",
            Self::FilesystemLocks => "filesystem_locks",
            Self::FilesystemEscape => "filesystem_escape",
            Self::LoaderDependencies => "loader_dependencies",
            Self::AslrAddressOutput => "aslr_address_output",
            Self::ProcessThreadIdentity => "process_thread_identity",
            Self::HostIdentity => "host_identity",
            Self::ProcSysDevAccess => "proc_sys_dev_access",
            Self::Signals => "signals",
            Self::Timers => "timers",
            Self::ThreadsForksExecDescendants => "threads_forks_exec_descendants",
            Self::InheritedFileDescriptors => "inherited_file_descriptors",
            Self::IpcSharedMemory => "ipc_shared_memory",
            Self::AsyncIoUring => "async_io_uring",
            Self::CpuFpFeatures => "cpu_fp_features",
            Self::JitGcFinalizers => "jit_gc_finalizers",
            Self::UnsupportedSyscallsEffects => "unsupported_syscalls_effects",
        }
    }
}

/// Whether a single [`CapabilityChannel`] is controller-controlled or
/// left open. `Closed` requires a stated evidence string; there is no
/// bare boolean that could be flipped without saying why.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ChannelStatus {
    /// Not controlled or replayed by this runner. `reason` explains why
    /// (usually: this MVP implements no interposition for the channel).
    Open { reason: String },
    /// The controller has synchronous evidence the channel is denied,
    /// virtualized, or replayed exactly. `evidence` cites the mechanism.
    Closed { evidence: String },
}

/// Tier-2 evidence grade. D1 requires every channel in the receipt to be
/// `Closed`; this MVP implements no channel closure, so D1 is currently
/// unreachable through the public API — not asserted false, just never
/// constructible.
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

/// Controller-produced, sealed capability receipt: what a subprocess run
/// actually controlled, observed, rejected, or left open. Caller input
/// (a [`crate::SandboxSpec`]) may request a profile; it has no field or
/// method that can assert a channel closed. The only way to read this
/// type's contents is via `status`/`open_channels`/`is_d1`; the only way
/// to construct one is the crate-internal `CapabilityReceipt::all_open`,
/// which seeds the complete inventory as `Open`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CapabilityReceipt {
    channels: BTreeMap<CapabilityChannel, ChannelStatus>,
}

impl CapabilityReceipt {
    /// The only public-surface constructor. Every channel in
    /// [`CapabilityChannel::ALL`] is present and `Open` with the given
    /// reason; there is no way to construct a receipt with a channel
    /// silently absent (which would be indistinguishable from closed).
    pub(crate) fn all_open(reason: &str) -> Self {
        let channels = CapabilityChannel::ALL
            .iter()
            .map(|&channel| {
                (
                    channel,
                    ChannelStatus::Open {
                        reason: reason.to_string(),
                    },
                )
            })
            .collect();
        Self { channels }
    }

    /// Status of one channel. Panics only if the inventory itself is
    /// incomplete, which `all_open` never allows.
    pub fn status(&self, channel: CapabilityChannel) -> &ChannelStatus {
        self.channels
            .get(&channel)
            .expect("CapabilityReceipt always carries the complete channel inventory")
    }

    /// D1 only when every channel is `Closed`. Never true for a receipt
    /// built by this package's `run_once`.
    pub fn is_d1(&self) -> bool {
        self.channels
            .values()
            .all(|status| matches!(status, ChannelStatus::Closed { .. }))
    }

    pub fn evidence_grade(&self) -> EvidenceGrade {
        if self.is_d1() {
            EvidenceGrade::D1
        } else {
            EvidenceGrade::D2
        }
    }

    pub fn open_channels(&self) -> Vec<CapabilityChannel> {
        self.channels
            .iter()
            .filter(|(_, status)| matches!(status, ChannelStatus::Open { .. }))
            .map(|(&channel, _)| channel)
            .collect()
    }

    pub fn identity(&self) -> String {
        let mut t = Trace::new();
        t.record(0, "schema", CAPABILITY_SCHEMA);
        for (channel, status) in &self.channels {
            let (tag, detail) = match status {
                ChannelStatus::Open { reason } => ("open", reason.as_str()),
                ChannelStatus::Closed { evidence } => ("closed", evidence.as_str()),
            };
            t.record(
                0,
                "channel",
                &format!("{}:{}:{}", channel.as_str(), tag, detail),
            );
        }
        t.hash_hex()
    }
}

// Test-only mutator: proves the `is_d1`/`evidence_grade` boolean logic
// against a fully-closed receipt without adding any production-reachable
// way to mint one. Gated behind `cfg(test)` so it does not exist in the
// plain lib build (a `pub(crate)` helper called only from `tests.rs`
// would otherwise be flagged dead code there).
#[cfg(test)]
impl CapabilityReceipt {
    pub(crate) fn set_status_for_test(
        &mut self,
        channel: CapabilityChannel,
        status: ChannelStatus,
    ) {
        self.channels.insert(channel, status);
    }
}

/// Exact, non-collapsing termination classification. Two different
/// signals, or an exact signal versus unknown, can never share an
/// [`TerminationOutcome::as_identity_str`] value.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TerminationOutcome {
    /// The process ran to completion and exited normally with this code.
    Exited(i32),
    /// The process was terminated by this exact Unix signal.
    /// `core_dumped` is `Some(true)` only when the controller positively
    /// observed a core-dump indication; `None` means not observably
    /// confirmed either way on this platform (never guessed `false`).
    Signaled {
        signal: i32,
        core_dumped: Option<bool>,
    },
    /// The controller's configured deadline elapsed before the child
    /// exited; the direct child was killed. See `process_tree` on the
    /// owning `RunRecord` for whether the kill was confirmed reaped.
    TimedOut,
    /// `Command::spawn` (fork/exec) itself failed; no process ever ran.
    SpawnFailed { message: String },
    /// A resource-limit outcome the controller actually observed. Not
    /// reachable in this MVP (no rlimit enforcement is implemented); the
    /// variant exists so a future observation never has to collapse into
    /// `Unknown`.
    ResourceLimited { message: String },
    /// A termination cause the safe runner cannot classify. Never
    /// silently merged with any variant above.
    Unknown { reason: String },
}

impl TerminationOutcome {
    pub fn as_identity_str(&self) -> String {
        match self {
            Self::Exited(code) => format!("exited:{code}"),
            Self::Signaled {
                signal,
                core_dumped,
            } => {
                let core = match core_dumped {
                    Some(true) => "true",
                    Some(false) => "false",
                    None => "unknown",
                };
                format!("signaled:{signal}:core_dumped={core}")
            }
            Self::TimedOut => "timed_out".to_string(),
            Self::SpawnFailed { message } => format!("spawn_failed:{message}"),
            Self::ResourceLimited { message } => format!("resource_limited:{message}"),
            Self::Unknown { reason } => format!("unknown:{reason}"),
        }
    }
}

/// Process-tree cleanup state the controller can actually prove in safe
/// Rust. Descendant/process-group cleanup cannot be proven this way — it
/// stays represented by `CapabilityChannel::ThreadsForksExecDescendants`
/// remaining `Open`, not by a field here.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProcessTreeState {
    /// `Command::spawn` failed; there is no child to account for.
    NoChildProcess,
    /// The direct child was waited on and its exit status collected.
    DirectChildReaped,
    /// The direct child could not be confirmed reaped (e.g. `wait()`
    /// itself failed after a kill). Never silently treated as reaped.
    DirectChildReapFailed { message: String },
}

impl ProcessTreeState {
    pub fn as_identity_str(&self) -> String {
        match self {
            Self::NoChildProcess => "no_child_process".to_string(),
            Self::DirectChildReaped => "direct_child_reaped".to_string(),
            Self::DirectChildReapFailed { message } => {
                format!("direct_child_reap_failed:{message}")
            }
        }
    }
}

/// What the controller could bind the executed program's identity to.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ExecutableIdentity {
    /// `argv[0]` resolved to a concrete file whose bytes were hashed.
    Resolved { path: String, digest: String },
    /// `argv[0]` relied on `PATH` search (or otherwise could not be
    /// resolved to a concrete file by the controller without
    /// reimplementing platform `PATH` resolution). The exact executed
    /// bytes are an open channel here, never guessed from the name alone.
    Unresolved { argv0: String },
}

impl ExecutableIdentity {
    pub fn as_identity_str(&self) -> String {
        match self {
            Self::Resolved { path, digest } => format!("resolved:{path}:{digest}"),
            Self::Unresolved { argv0 } => format!("unresolved:{argv0}"),
        }
    }
}

/// A subprocess output stream as actually retained by the bounded
/// reader. `byte_len` is the true on-disk size even when `truncated` is
/// true and `digest` only covers the retained prefix — the identity
/// never silently pretends to cover bytes it discarded.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StreamObservation {
    pub digest: String,
    pub byte_len: u64,
    pub truncated: bool,
}

impl StreamObservation {
    pub fn as_identity_str(&self) -> String {
        format!("{}:{}:{}", self.digest, self.byte_len, self.truncated)
    }
}

/// Raw, confidence-free divergence evidence over a declared sample of
/// run-twice pairs. Replaces a single-pair 0.0/1.0 rate: the
/// numerator/denominator and sample identity are load-bearing, the
/// rendered decimal rate is a derived convenience only. Pairwise
/// agreement remains a sampled falsifier, never proof.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DivergenceReport {
    pub diverged: usize,
    pub sample: usize,
    pub sample_identity: String,
}

impl DivergenceReport {
    /// Aggregate raw run-twice evidence from an ordered sequence of
    /// `(first_identity, second_identity)` pairs. Binds pair order and
    /// each pair's own identity so two different suites can never share
    /// a sample identity; a suite of size zero has `sample: 0` (never a
    /// silently manufactured `0.000`).
    pub fn from_identity_pairs<'a, I>(pairs: I) -> Self
    where
        I: IntoIterator<Item = (&'a str, &'a str)>,
    {
        let mut t = Trace::new();
        t.record(0, "schema", DIVERGENCE_REPORT_SCHEMA);
        let mut diverged = 0usize;
        let mut sample = 0usize;
        for (i, (first, second)) in pairs.into_iter().enumerate() {
            let d = first != second;
            if d {
                diverged += 1;
            }
            sample += 1;
            t.record(i as u64, "pair", &format!("{first}:{second}:{d}"));
        }
        Self {
            diverged,
            sample,
            sample_identity: t.hash_hex(),
        }
    }

    /// Confidence-free rendering only; `diverged`/`sample` are the
    /// load-bearing fields.
    pub fn rate(&self) -> f64 {
        if self.sample == 0 {
            0.0
        } else {
            self.diverged as f64 / self.sample as f64
        }
    }

    pub fn verdict_line(&self, grade: EvidenceGrade) -> String {
        format!(
            "tier=Tier-2 d-grade={} divergence-rate={:.3} evidence=run-twice agreement (sampled falsifier — not proof) diverged={}/{} sample-identity={}",
            grade.as_str(),
            self.rate(),
            self.diverged,
            self.sample,
            self.sample_identity,
        )
    }
}
