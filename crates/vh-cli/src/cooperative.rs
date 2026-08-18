//! R2 — reusable cooperative D2 transport behind the generic R1
//! operation surface.
//!
//! This is a declared boundary file: it uses host tempdirs and the
//! `vh-sandbox` cassette broker to run a cooperative child twice and
//! report a typed, engine-only outcome. It introduces no Python truth
//! authority and keeps the existing `vh-cassette-transport-v1` wire
//! format as the single canonical protocol.

use std::io::Write;
use std::path::{Component, Path, PathBuf};

use vh_sandbox::{
    run_once_with_cassette, CassetteV2, LlmRequestV2, SandboxCampaign, SandboxError, SandboxSpec,
    TapeEntry,
};

use vh_cli::receipts::{render_line, Val};

pub(crate) const COOPERATIVE_OUTCOME_SCHEMA: &str = "vh-cooperative-outcome-v1";
pub(crate) const SCOPE: &str = "vibe-halt.run.v0";
type OutcomeFields = Vec<(&'static str, Val)>;

/// Tiny cooperative child: makes one child-visible cassette request and
/// writes the returned body to `out.txt`. The source is the only code
/// the child executes; the cassette supplies all "external" behavior.
pub(crate) const COOPERATIVE_ECHO_CHILD: &str = r#"
import os, sys, time

MAILBOX = os.path.join('.vh-sandbox-io', 'llm')
CALL_DEADLINE = 10.0

def field(tag, value):
    return tag.encode() + b' ' + str(len(value)).encode() + b':' + value + b'\n'

def make_request(provider, model, messages, params=()):
    out = b'vh-llm-request-v2\n'
    out += field('provider', provider.encode())
    out += field('model', model.encode())
    out += ('messages %d\n' % len(messages)).encode()
    for role, content in messages:
        out += field('role', role.encode())
        out += field('content', content.encode())
    out += b'tools 0\n'
    out += b'tool-choice absent\n'
    out += b'structured-output absent\n'
    items = sorted(dict(params).items())
    out += ('params %d\n' % len(items)).encode()
    for k, v in items:
        out += field('param-key', k.encode())
        out += field('param-value', v.encode())
    return out

def write_frame(path, data):
    tmp = path + '.tmp'
    with open(tmp, 'wb') as f:
        f.write(data)
    os.replace(tmp, path)

def read_frame(path):
    start = time.monotonic()
    while not os.path.exists(path):
        if time.monotonic() - start > CALL_DEADLINE:
            sys.exit(41)
        time.sleep(0.002)
    with open(path, 'rb') as f:
        return f.read()

def read_body(data):
    nl = data.index(b'\n')
    head = data[:nl].decode()
    pos = nl + 1
    tag = b'body '
    if not data[pos:pos + len(tag)] == tag:
        sys.exit(43)
    pos += len(tag)
    colon = data.index(b':', pos)
    ln = int(data[pos:colon])
    pos = colon + 1
    return data[pos:pos + ln]

req = make_request('fixture', 'cooperative-echo', [('user', 'hello')], [('temperature', '0')])
write_frame(os.path.join(MAILBOX, 'req-0'), req)
resp = read_frame(os.path.join(MAILBOX, 'resp-0'))
body = read_body(resp)
with open('out.txt', 'wb') as f:
    f.write(body)
"#;

fn fixture_request() -> LlmRequestV2 {
    LlmRequestV2 {
        provider: "fixture".into(),
        model: "cooperative-echo".into(),
        messages: vec![("user".into(), "hello".into())],
        tools: Vec::new(),
        tool_choice: None,
        structured_output: None,
        params: std::collections::BTreeMap::from([("temperature".into(), "0".into())]),
    }
}

pub(crate) fn fixture_cassette() -> CassetteV2 {
    let mut cassette = CassetteV2::default();
    cassette.push(
        fixture_request(),
        TapeEntry::Success {
            status: 200,
            body: b"cooperative-reply\n".to_vec(),
        },
    );
    cassette
}

fn cassette_root_prefix(cassette: &CassetteV2) -> String {
    cassette
        .identity()
        .rsplit(':')
        .next()
        .unwrap_or("builtin")
        .chars()
        .take(16)
        .collect()
}

/// Stable logical input name for the cooperative child source. The
/// absolute staging path is per-invocation and deliberately OUTSIDE
/// deterministic identity; identity binds this logical name plus the
/// source content digest instead (item 2).
pub(crate) const CHILD_LOGICAL_NAME: &str = "cooperative_echo.py";

pub(crate) fn write_new_file(path: &Path, bytes: &[u8]) -> Result<(), std::io::Error> {
    let mut options = std::fs::OpenOptions::new();
    options.write(true).create_new(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    let mut file = options.open(path)?;
    file.write_all(bytes)?;
    file.sync_all()
}

/// Write the trusted child source and deterministic initial artifact into a
/// fresh run workspace. Precreating `out.txt` keeps the global sandbox law
/// strict on nonzero exits; the timeout oracle observes an empty artifact
/// rather than making missing outputs globally acceptable.
fn write_child_source(workspace: &Path) -> Result<(), SandboxError> {
    std::fs::create_dir(workspace)?;
    write_new_file(
        &workspace.join(CHILD_LOGICAL_NAME),
        COOPERATIVE_ECHO_CHILD.as_bytes(),
    )?;
    write_new_file(&workspace.join("out.txt"), b"")?;
    Ok(())
}

fn child_spec(cassette: &CassetteV2) -> Result<SandboxSpec, SandboxError> {
    SandboxSpec::new(vec![
        "/usr/bin/python3".into(),
        "-S".into(),
        "-s".into(),
        CHILD_LOGICAL_NAME.into(),
    ])?
    // macOS' Python consults DARWIN_USER_TEMP_DIR at startup and emits a
    // warning when the scrubbed environment cannot resolve it. Pinning a
    // public, deterministic temp root keeps the exact-empty stderr oracle
    // portable without granting access to caller-controlled environment.
    .allow_env("TMPDIR", "/tmp")?
    .with_cassette_identity(cassette.identity())
    .declare_artifact("out.txt")?
    .declare_input_bytes(CHILD_LOGICAL_NAME, COOPERATIVE_ECHO_CHILD.as_bytes())
}

/// Cross the sandbox boundary while retaining a conservative admission count.
///
/// The counter is deliberately one-sided: zero proves this boundary was not
/// invoked. A positive value records an admitted attempt only; it does not
/// attest that process spawn, loading, or child execution occurred.
fn run_admitted_sandbox_attempt(
    spec: &SandboxSpec,
    workspace: &Path,
    cassette: &CassetteV2,
    attempts: &mut u64,
) -> Result<vh_sandbox::RunRecord, SandboxError> {
    *attempts += 1;
    run_once_with_cassette(spec, workspace, cassette)
}

/// Atomically reserve a fresh, invocation-exclusive workspace. With an
/// output root, the single `create_dir` of `workspace/` inside the
/// already-validated-empty root IS the reservation: `AlreadyExists`
/// means a competing invocation won, and the loser refuses without
/// touching any caller's data. Without an output root, reserve the
/// first free uniquely suffixed temp directory the same way — never
/// `remove_dir_all` on a shared path.
struct WorkspaceLease {
    path: PathBuf,
    cleanup: bool,
}

const MAX_WORKSPACE_RESERVATION_ATTEMPTS: u32 = 128;

#[cfg(unix)]
pub(crate) fn create_private_directory(path: &Path) -> std::io::Result<()> {
    use std::os::unix::fs::DirBuilderExt;

    let mut builder = std::fs::DirBuilder::new();
    builder.mode(0o700).create(path)
}

#[cfg(not(unix))]
pub(crate) fn create_private_directory(path: &Path) -> std::io::Result<()> {
    std::fs::create_dir(path)
}

/// Reserve an invocation-unique directory for cross-module boundary tests.
/// Environment and process identity stay in this already-declared boundary
/// file; deterministic evidence modules receive only the resulting path.
#[cfg(test)]
pub(crate) fn reserve_test_directory(label: &str) -> Result<PathBuf, String> {
    let root = std::env::temp_dir();
    for suffix in 0..MAX_WORKSPACE_RESERVATION_ATTEMPTS {
        let path = root.join(format!(
            "vh-cli-test-{}-{label}-{suffix}",
            std::process::id()
        ));
        match create_private_directory(&path) {
            Ok(()) => return Ok(path),
            Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => continue,
            Err(_) => return Err("test directory reservation failed".to_string()),
        }
    }
    Err("test directory reservation attempts exhausted".to_string())
}

impl WorkspaceLease {
    fn path(&self) -> &Path {
        &self.path
    }
}

impl Drop for WorkspaceLease {
    fn drop(&mut self) {
        if !self.cleanup {
            return;
        }
        // The directory is controller-created and invocation-unique. Refuse
        // to recurse if its final component was replaced by a symlink.
        if let Ok(metadata) = std::fs::symlink_metadata(&self.path) {
            if metadata.is_dir() && !metadata.file_type().is_symlink() {
                let _ = std::fs::remove_dir_all(&self.path);
            }
        }
    }
}

fn reserve_workspace(
    out_dir: Option<&Path>,
    _label: &str,
    cassette: &CassetteV2,
) -> Result<WorkspaceLease, String> {
    #[cfg(not(unix))]
    {
        let _ = (out_dir, cassette);
        Err("cooperative workspace ownership cannot be proven on this platform".into())
    }
    #[cfg(unix)]
    {
        match out_dir {
            Some(root) => {
                #[cfg(unix)]
                let invoking_uid = invoking_uid_from_anonymous_fd()?;
                #[cfg(unix)]
                validate_directory_owner_chain(root, invoking_uid, true)?;
                let workspace = root.join("workspace");
                create_private_directory(&workspace).map_err(|e| {
                    if e.kind() == std::io::ErrorKind::AlreadyExists {
                        format!(
                            "output root {root:?} is already reserved by a competing invocation; \
                         refusing without touching any data"
                        )
                    } else {
                        format!("cannot reserve workspace in {root:?}: {e}")
                    }
                })?;
                #[cfg(unix)]
                if let Err(error) = validate_directory_owner_chain(&workspace, invoking_uid, true) {
                    let _ = std::fs::remove_dir(&workspace);
                    return Err(error);
                }
                Ok(WorkspaceLease {
                    path: workspace,
                    cleanup: false,
                })
            }
            None => {
                let temp_root = std::env::temp_dir();
                #[cfg(unix)]
                let invoking_uid = invoking_uid_from_anonymous_fd()?;
                #[cfg(unix)]
                validate_directory_owner_chain(&temp_root, invoking_uid, false)
                    .map_err(|_| "ambient temp root owner chain is unsafe".to_string())?;
                let prefix = cassette_root_prefix(cassette);
                for n in 0..MAX_WORKSPACE_RESERVATION_ATTEMPTS {
                    let candidate = temp_root.join(format!(
                        "vh-cooperative-{}-{prefix}-{n}",
                        std::process::id()
                    ));
                    match create_private_directory(&candidate) {
                        Ok(()) => {
                            #[cfg(unix)]
                            if let Err(error) =
                                validate_directory_owner_chain(&candidate, invoking_uid, true)
                            {
                                let _ = std::fs::remove_dir(&candidate);
                                return Err(error);
                            }
                            return Ok(WorkspaceLease {
                                path: candidate,
                                cleanup: true,
                            });
                        }
                        Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => continue,
                        Err(e) => return Err(format!("cannot reserve workspace: {e}")),
                    }
                }
                Err(format!(
                    "cannot reserve a fresh workspace within the {}-attempt bound",
                    MAX_WORKSPACE_RESERVATION_ATTEMPTS
                ))
            }
        }
    }
}

fn run_cooperative_campaign(
    cassette_override: Option<&CassetteV2>,
    label: &str,
    out_dir: Option<&Path>,
    executions: &mut u64,
) -> Result<(SandboxCampaign, WorkspaceLease), String> {
    let owned_cassette;
    let cassette: &CassetteV2 = match cassette_override {
        Some(c) => c,
        None => {
            owned_cassette = fixture_cassette();
            &owned_cassette
        }
    };
    let root = reserve_workspace(out_dir, label, cassette)?;

    let spec = child_spec(cassette).map_err(|e| format!("invalid cooperative spec: {e}"))?;
    let a = root.path().join("a");
    let b = root.path().join("b");
    write_child_source(&a).map_err(|e| format!("cannot place child in workspace a: {e}"))?;
    write_child_source(&b).map_err(|e| format!("cannot place child in workspace b: {e}"))?;

    let first = run_admitted_sandbox_attempt(&spec, &a, cassette, executions)
        .map_err(|e| format!("cooperative run a failed: {e}"))?;
    let second = run_admitted_sandbox_attempt(&spec, &b, cassette, executions)
        .map_err(|e| format!("cooperative run b failed: {e}"))?;
    Ok((SandboxCampaign { first, second }, root))
}

enum CooperativeV2RunError {
    Refusal(crate::protocol::RefusalReason),
    Boundary(String),
}

#[cfg(test)]
type BeforeObservationHook<'a> = Option<&'a mut dyn FnMut(&Path, &Path) -> Result<(), String>>;

/// Stage the Rust-owned target, take a bounded owned observation of both
/// copies, enforce any exact caller constraint, and only then cross the child
/// execution boundary. The sandbox independently rechecks each logical input
/// immediately before spawn. Its final observation-to-loader race remains an
/// explicitly Open D2 channel.
fn run_cooperative_campaign_v2_inner(
    cassette: &CassetteV2,
    out_dir: Option<&Path>,
    requested_revision: &crate::protocol::RequestedTargetRevision,
    executions: &mut u64,
    #[cfg(test)] before_observation: BeforeObservationHook<'_>,
) -> Result<
    (
        SandboxCampaign,
        WorkspaceLease,
        crate::protocol::FreshObservedRevision,
    ),
    CooperativeV2RunError,
> {
    let root = reserve_workspace(out_dir, "cooperative-v2", cassette)
        .map_err(CooperativeV2RunError::Boundary)?;
    let spec = child_spec(cassette)
        .map_err(|error| CooperativeV2RunError::Boundary(format!("invalid spec: {error}")))?;
    let a = root.path().join("a");
    let b = root.path().join("b");
    write_child_source(&a).map_err(|error| {
        CooperativeV2RunError::Boundary(format!("cannot stage target a: {error}"))
    })?;
    write_child_source(&b).map_err(|error| {
        CooperativeV2RunError::Boundary(format!("cannot stage target b: {error}"))
    })?;
    #[cfg(test)]
    if let Some(hook) = before_observation {
        hook(&a, &b).map_err(CooperativeV2RunError::Boundary)?;
    }
    let observe = |path: &Path| {
        crate::protocol::resolve_fresh_target_path(
            &path.join(CHILD_LOGICAL_NAME),
            vh_sandbox::MAX_CASSETTE_BYTES,
        )
        .map_err(|_| {
            CooperativeV2RunError::Refusal(crate::protocol::RefusalReason::MissingObservation)
        })
    };
    let fresh_a = observe(&a)?;
    let fresh_b = observe(&b)?;
    if fresh_a.bytes() != COOPERATIVE_ECHO_CHILD.as_bytes()
        || fresh_b.bytes() != COOPERATIVE_ECHO_CHILD.as_bytes()
        || fresh_a.digest() != fresh_b.digest()
    {
        return Err(CooperativeV2RunError::Refusal(
            crate::protocol::RefusalReason::RequestedRevisionMismatch,
        ));
    }
    if requested_revision.exact_digest() != Some(fresh_a.digest()) {
        return Err(CooperativeV2RunError::Refusal(
            crate::protocol::RefusalReason::RequestedRevisionMismatch,
        ));
    }
    let first = run_admitted_sandbox_attempt(&spec, &a, cassette, executions)
        .map_err(|error| CooperativeV2RunError::Boundary(format!("run a failed: {error}")))?;
    let second = run_admitted_sandbox_attempt(&spec, &b, cassette, executions)
        .map_err(|error| CooperativeV2RunError::Boundary(format!("run b failed: {error}")))?;
    Ok((SandboxCampaign { first, second }, root, fresh_a))
}

/// The declared cooperative workload oracle. A cassette `Timeout` may be
/// reported as a target finding ONLY when this oracle is positively
/// evaluated and recorded (item 3): fully consumed, untainted,
/// identically reproduced evidence with supported terminations. Every
/// other failure shape — controller, spawn, I/O, staging, unsupported
/// termination, receipt, or verifier failure — stays typed
/// ERROR/UNCHECKED and is never FINDINGS.
pub(crate) const COOPERATIVE_ORACLE: &str = "cooperative-llm-call-completed";
/// Stable finding identity for the oracle-verified cassette Timeout.
pub(crate) const FINDING_TIMEOUT: &str = "cooperative-llm-call-completed:timeout";
fn exact_transport_shape(record: &vh_sandbox::RunRecord) -> bool {
    let expected_request = fixture_request().digest();
    matches!(
        record.transport.as_ref(),
        Some(transport)
            if transport.served == [expected_request]
                && transport.unconsumed == 0
                && transport.taint.is_none()
    )
}

fn exact_artifact_shape(record: &vh_sandbox::RunRecord, expected: &[u8]) -> bool {
    record.artifacts.len() == 1
        && record.artifacts.get("out.txt") == Some(&vh_sandbox::fnv_hex(expected))
}

fn exact_common_run_shape(record: &vh_sandbox::RunRecord, cassette: &CassetteV2) -> bool {
    let expected_spec = child_spec(cassette).map(|spec| spec.identity());
    let executable_is_bound = matches!(
        &record.executable,
        vh_sandbox::ExecutableIdentity::Resolved { path, digest }
            if path == "/usr/bin/python3"
                && digest.len() == 32
                && digest.bytes().all(|byte| byte.is_ascii_hexdigit())
                && !digest.bytes().any(|byte| byte.is_ascii_uppercase())
    );
    let empty_stream = |stream: &vh_sandbox::StreamObservation| {
        stream.byte_len == 0 && !stream.truncated && stream.digest == vh_sandbox::fnv_hex(b"")
    };
    let spec_ok = expected_spec.is_ok_and(|identity| record.spec_identity == identity);
    let process_ok = matches!(
        record.process_tree,
        vh_sandbox::ProcessTreeState::DirectChildReaped
    );
    let streams_ok = empty_stream(&record.stdout) && empty_stream(&record.stderr);
    let channels_ok = vh_sandbox::CapabilityChannel::ALL.iter().all(|channel| {
        matches!(
            record.capability.status(*channel),
            vh_sandbox::ChannelStatus::Open { .. }
        )
    });
    spec_ok && executable_is_bound && process_ok && streams_ok && channels_ok
}

fn outcome_fields(
    campaign: &SandboxCampaign,
    cassette: &CassetteV2,
) -> (i32, Vec<(&'static str, Val)>) {
    let first = &campaign.first;
    let second = &campaign.second;

    let mut errors: Vec<String> = Vec::new();

    if first.transport_tainted() {
        if let Some(taint) = first.transport.as_ref().and_then(|t| t.taint.as_ref()) {
            errors.push(format!("first run transport taint: {taint}"));
        } else {
            errors.push("first run transport taint: unconsumed or malformed history".into());
        }
    }
    if second.transport_tainted() {
        if let Some(taint) = second.transport.as_ref().and_then(|t| t.taint.as_ref()) {
            errors.push(format!("second run transport taint: {taint}"));
        } else {
            errors.push("second run transport taint: unconsumed or malformed history".into());
        }
    }

    let diverged = first.identity() != second.identity();

    // Pinned diagnostic bound (item 9): nothing longer ever crosses
    // into stdout, stderr, or a receipt, whatever the boundary carried.
    let errors: Vec<String> = errors.iter().map(|e| bounded_diagnostic(e)).collect();

    // Oracle evaluation (item 3). A generic nonzero exit is NEVER
    // equated with a target finding.
    let (verdict, exit_code, verified, findings_count, oracle_evaluation, finding_identity) =
        if !errors.is_empty() {
            // Transport evidence failure (taint or unconsumed tape): the
            // oracle cannot be evaluated.
            ("UNCHECKED", 3, false, 0, "indeterminate", "none")
        } else if diverged {
            // Reproduction failure is unsupported evidence, never a target
            // finding. In particular, no consensus/divergence signal can
            // acquire epistemic standing merely by being repeated.
            ("UNCHECKED", 3, false, 0, "indeterminate", "none")
        } else {
            let exact_fixture = cassette.len() == 1
                && cassette
                    .entry(0)
                    .is_some_and(|(request, _)| request == &fixture_request());
            let exact_transport = exact_transport_shape(first) && exact_transport_shape(second);
            let success_body = cassette.entry(0).and_then(|(_, entry)| match entry {
                TapeEntry::Success { status: 200, body } => Some(body.as_slice()),
                _ => None,
            });
            let clean = exact_fixture
                && exact_transport
                && exact_common_run_shape(first, cassette)
                && exact_common_run_shape(second, cassette)
                && matches!(first.termination, vh_sandbox::TerminationOutcome::Exited(0))
                && matches!(
                    second.termination,
                    vh_sandbox::TerminationOutcome::Exited(0)
                )
                && success_body.is_some_and(|body| {
                    exact_artifact_shape(first, body) && exact_artifact_shape(second, body)
                });
            let timeout = exact_fixture
                && exact_transport
                && exact_common_run_shape(first, cassette)
                && exact_common_run_shape(second, cassette)
                && matches!(cassette.entry(0), Some((_, TapeEntry::Timeout)))
                && matches!(
                    first.termination,
                    vh_sandbox::TerminationOutcome::Exited(43)
                )
                && matches!(
                    second.termination,
                    vh_sandbox::TerminationOutcome::Exited(43)
                )
                && exact_artifact_shape(first, b"")
                && exact_artifact_shape(second, b"");
            if clean {
                ("CLEAN", 0, true, 0, "completed", "none")
            } else if timeout {
                (
                    "FINDINGS",
                    1,
                    true,
                    1,
                    "not-completed:timeout",
                    FINDING_TIMEOUT,
                )
            } else {
                // Generic nonzero exits, wrong exact exit codes, alternate
                // cassette shapes, unexpected artifacts/transports, signals,
                // or deadline kills never satisfy the declared oracle.
                ("UNCHECKED", 3, false, 0, "indeterminate", "none")
            }
        };

    let evidence_digest = first.identity();
    let result_digest = second.identity();
    let transport = first
        .transport
        .as_ref()
        .map(|t| t.identity_str())
        .unwrap_or_else(|| "none".into());

    let fields = vec![
        ("record", Val::S("cooperative-outcome".into())),
        ("schema", Val::S(COOPERATIVE_OUTCOME_SCHEMA.into())),
        ("verdict", Val::S(verdict.into())),
        ("tier", Val::S("TIER2".into())),
        ("grade", Val::S("D2".into())),
        ("scope", Val::S(SCOPE.into())),
        ("evidence_digest", Val::S(evidence_digest)),
        ("result_digest", Val::S(result_digest)),
        ("transport", Val::S(transport)),
        ("oracle", Val::S(COOPERATIVE_ORACLE.into())),
        ("oracle_evaluation", Val::S(oracle_evaluation.into())),
        ("finding_identity", Val::S(finding_identity.into())),
        ("findings_count", Val::N(findings_count)),
        ("exit_code", Val::N(exit_code as u64)),
        ("verified", Val::B(verified)),
        ("errors", Val::S(errors_to_json_array(&errors))),
    ];
    (exit_code, fields)
}

/// Hard pin for diagnostics routed through cooperative and receipt-replay
/// boundaries. No such attacker-influenced diagnostic string may exceed this
/// bound, regardless of what the underlying boundary error carried.
pub(crate) const MAX_DIAGNOSTIC_BYTES: usize = 256;

/// Truncate a diagnostic to [`MAX_DIAGNOSTIC_BYTES`] on a char
/// boundary, with an explicit truncation marker. Defense in depth on
/// top of category redaction: even a future non-redacted error path
/// cannot emit an unbounded diagnostic.
pub(crate) fn bounded_diagnostic(s: &str) -> String {
    // Keep boundary diagnostics printable ASCII before applying the byte cap.
    // This keeps caller-controlled argv/path data from injecting lines,
    // terminal state, bidi overrides, or invisible Unicode formatting while
    // ensuring the escaped representation itself is bounded.
    let mut escaped = String::new();
    let mut exceeded = false;
    for character in s.chars() {
        if !character.is_ascii() || character.is_control() {
            escaped.extend(character.escape_default());
        } else {
            escaped.push(character);
        }
        if escaped.len() > MAX_DIAGNOSTIC_BYTES {
            exceeded = true;
            break;
        }
    }
    if !exceeded {
        return escaped;
    }
    const MARKER: &str = "...[truncated]";
    let mut end = MAX_DIAGNOSTIC_BYTES.saturating_sub(MARKER.len());
    while !escaped.is_char_boundary(end) {
        end -= 1;
    }
    format!("{}{}", &escaped[..end], MARKER)
}

fn errors_to_json_array(errors: &[String]) -> String {
    let parts: Vec<String> = errors.iter().map(|e| serde_json_escape(e)).collect();
    format!("[{}]", parts.join(","))
}

fn serde_json_escape(s: &str) -> String {
    let mut out = String::with_capacity(s.len() + 2);
    out.push('"');
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if (c as u32) < 0x20 => out.push_str(&format!("\\u{:04x}", c as u32)),
            c => out.push(c),
        }
    }
    out.push('"');
    out
}

pub(crate) fn cmd_cooperative(args: &[String], usage: &str) -> i32 {
    let mut executions = 0u64;
    cmd_cooperative_inner(args, usage, &mut executions)
}

#[cfg(unix)]
fn invoking_uid_from_anonymous_fd() -> Result<u32, String> {
    use std::os::fd::OwnedFd;
    use std::os::unix::fs::MetadataExt;
    use std::os::unix::net::UnixStream;

    // Safe Rust exposes descriptor ownership but not geteuid. An anonymous
    // socket pair creates no pathname and its fd metadata is kernel-authored.
    let (socket, _peer) = UnixStream::pair()
        .map_err(|_| "cannot create anonymous uid-observation descriptor".to_string())?;
    let owned: OwnedFd = socket.into();
    let file: std::fs::File = owned.into();
    file.metadata()
        .map(|metadata| metadata.uid())
        .map_err(|_| "cannot inspect anonymous uid-observation descriptor".to_string())
}

#[cfg(unix)]
fn validate_directory_owner_chain(
    path: &Path,
    effective_uid: u32,
    require_leaf_owner: bool,
) -> Result<(), String> {
    use std::os::unix::fs::MetadataExt;

    if !path.is_absolute()
        || path
            .components()
            .any(|part| matches!(part, Component::ParentDir | Component::CurDir))
    {
        return Err("trusted directory must be absolute and traversal-free".into());
    }
    let mut current = PathBuf::new();
    let component_count = path.components().count();
    for (index, part) in path.components().enumerate() {
        current.push(part.as_os_str());
        let metadata = std::fs::symlink_metadata(&current)
            .map_err(|_| "cannot reobserve --out owner chain".to_string())?;
        if metadata.file_type().is_symlink() {
            let trusted_macos_alias = cfg!(target_os = "macos")
                && (current == Path::new("/tmp") || current == Path::new("/var"));
            if trusted_macos_alias {
                continue;
            }
            return Err("--out owner chain contains a symlink".into());
        }
        if !metadata.is_dir() {
            return Err("--out owner chain contains a non-directory".into());
        }
        if metadata.uid() != 0 && metadata.uid() != effective_uid {
            return Err("--out owner chain has an untrusted owner".into());
        }
        if require_leaf_owner && index + 1 == component_count && metadata.uid() != effective_uid {
            return Err("--out directory is not owned by the invoking uid".into());
        }
        let mode = metadata.mode();
        if mode & 0o022 != 0 {
            if mode & 0o1000 == 0 {
                return Err("--out has a group/other-writable non-sticky parent".into());
            }
            if metadata.uid() != 0 {
                return Err("--out has a non-root-owned shared sticky parent".into());
            }
        }
    }
    Ok(())
}

/// Validate (and when absent, create) the cooperative output root
/// BEFORE any cassette bytes are loaded or any child is launched.
/// Refuses symlinks, non-directories, non-empty directories, and (on Unix)
/// an owner chain outside root/the invoking uid, a pre-existing
/// group/other-writable root, or any group/other-writable ancestor unless it
/// is both sticky and root-owned; never deletes or overwrites pre-existing
/// bytes. Same-user/ACL path races remain inside the stated D2 filesystem
/// boundary.
pub(crate) fn prepare_output_root(path: &Path) -> Result<PathBuf, String> {
    if !path.is_absolute()
        || path
            .components()
            .any(|part| matches!(part, Component::ParentDir | Component::CurDir))
    {
        return Err("--out must be an absolute traversal-free path".into());
    }
    #[cfg(unix)]
    let invoking_uid = invoking_uid_from_anonymous_fd()?;
    let mut current = PathBuf::new();
    let component_count = path.components().count();
    for (index, part) in path.components().enumerate() {
        current.push(part.as_os_str());
        match std::fs::symlink_metadata(&current) {
            Ok(metadata) if metadata.file_type().is_symlink() => {
                let trusted_macos_alias = cfg!(target_os = "macos")
                    && (current == Path::new("/tmp") || current == Path::new("/var"));
                if !trusted_macos_alias {
                    return Err("--out contains a symlink component".into());
                }
            }
            Ok(metadata) =>
            {
                #[cfg(unix)]
                if index + 1 < component_count && metadata.is_dir() {
                    use std::os::unix::fs::MetadataExt;
                    if metadata.uid() != 0 && metadata.uid() != invoking_uid {
                        return Err("--out owner chain has an untrusted owner".into());
                    }
                    let mode = metadata.mode();
                    if mode & 0o022 != 0 {
                        if mode & 0o1000 == 0 {
                            return Err("--out has a group/other-writable non-sticky parent".into());
                        }
                        if metadata.uid() != 0 {
                            return Err("--out has a non-root-owned shared sticky parent".into());
                        }
                    }
                }
            }
            Err(error)
                if error.kind() == std::io::ErrorKind::NotFound && index + 1 == component_count =>
            {
                break;
            }
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                return Err("--out has a missing parent component".into());
            }
            Err(_) => return Err("cannot inspect --out path".into()),
        }
    }
    let _created_by_this_call = match std::fs::symlink_metadata(path) {
        Ok(meta) => {
            if meta.file_type().is_symlink() {
                return Err("--out is a symlink".into());
            }
            if !meta.is_dir() {
                return Err("--out exists and is not a directory".into());
            }
            #[cfg(unix)]
            {
                use std::os::unix::fs::MetadataExt;
                if meta.mode() & 0o022 != 0 {
                    return Err("--out directory is group/other-writable".into());
                }
            }
            let mut entries =
                std::fs::read_dir(path).map_err(|_| "cannot inspect --out".to_string())?;
            if entries.next().is_some() {
                return Err("--out is not empty".into());
            }
            false
        }
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            create_private_directory(path).map_err(|_| "cannot create --out".to_string())?;
            true
        }
        Err(_) => return Err("cannot inspect --out".into()),
    };
    #[cfg(unix)]
    if let Err(error) = validate_directory_owner_chain(path, invoking_uid, true) {
        if _created_by_this_call {
            // Remove only the exact empty leaf we created; never recurse or
            // touch a pre-existing output root.
            let _ = std::fs::remove_dir(path);
        }
        return Err(error);
    }
    Ok(path.to_path_buf())
}

fn cmd_cooperative_inner(args: &[String], usage: &str, executions: &mut u64) -> i32 {
    let mut workload = "cooperative-echo".to_string();
    let mut cassette_path: Option<String> = None;
    let mut out_path: Option<String> = None;

    let mut it = args.iter();
    while let Some(arg) = it.next() {
        match arg.as_str() {
            "--workload" => {
                workload = match it.next() {
                    Some(v) => v.clone(),
                    None => {
                        eprintln!("error: --workload requires a value\n\n{usage}");
                        return 2;
                    }
                };
            }
            "--cassette" => {
                cassette_path = match it.next() {
                    Some(v) => Some(v.clone()),
                    None => {
                        eprintln!("error: --cassette requires a value\n\n{usage}");
                        return 2;
                    }
                };
            }
            "--out" => {
                out_path = match it.next() {
                    Some(v) => Some(v.clone()),
                    None => {
                        eprintln!("error: --out requires a value\n\n{usage}");
                        return 2;
                    }
                };
            }
            other => {
                eprintln!(
                    "error: unknown argument: {}\n\n{usage}",
                    bounded_diagnostic(other)
                );
                return 2;
            }
        }
    }

    if workload != "cooperative-echo" {
        eprintln!(
            "error: unknown cooperative workload '{}' (expected cooperative-echo)\n\n{usage}",
            bounded_diagnostic(&workload)
        );
        return 2;
    }

    // Output refusal comes FIRST: before cassette bytes are loaded and
    // before any child is launched (item 6). Pre-existing bytes are
    // never touched on refusal.
    let out_dir = match out_path {
        Some(path) => match prepare_output_root(Path::new(&path)) {
            Ok(dir) => Some(dir),
            Err(msg) => {
                eprintln!("error: {msg}");
                return 2;
            }
        },
        None => None,
    };

    let cassette = match cassette_path {
        Some(path) => {
            // Item 5: size enforced before parsing and before any
            // unbounded allocation.
            let bytes = match vh_sandbox::read_bounded_file(
                Path::new(&path),
                vh_sandbox::MAX_CASSETTE_BYTES,
            ) {
                Ok(b) => b,
                Err(SandboxError::Oversized { max, .. }) => {
                    eprintln!("error: cassette exceeds the {max}-byte bound");
                    return 2;
                }
                Err(e) => {
                    eprintln!("error: cannot read cassette: category={}", e.category());
                    return 2;
                }
            };
            // Item 9: attacker-controlled parse content is redacted to a
            // stable bounded category.
            match CassetteV2::parse_detailed(&bytes) {
                Ok(c) => {
                    if c.file_bytes() != bytes {
                        eprintln!("error: malformed cassette: category=noncanonical-encoding");
                        return 2;
                    }
                    Some(c)
                }
                Err(e) => {
                    eprintln!("error: malformed cassette: category={}", e.category());
                    return 2;
                }
            }
        }
        None => None,
    };

    let cassette_ref = cassette.as_ref();
    let (campaign, workspace) =
        match run_cooperative_campaign(cassette_ref, &workload, out_dir.as_deref(), executions) {
            Ok(c) => c,
            Err(e) => {
                eprintln!("error: cooperative run failed: {}", bounded_diagnostic(&e));
                return 2;
            }
        };

    let effective_cassette;
    let cassette_for_outcome: &CassetteV2 = match cassette_ref {
        Some(c) => c,
        None => {
            effective_cassette = fixture_cassette();
            &effective_cassette
        }
    };
    let (_provisional_exit, provisional_fields) = outcome_fields(&campaign, cassette_for_outcome);
    let transport = field_string(&provisional_fields, "transport");

    // A checked outcome may be emitted only after a persisted canonical
    // receipt has passed the strict verifier's fresh replay. Direct CLI runs
    // without --out use a private ephemeral evidence directory and obey the
    // same promotion path; the lease removes it after emission.
    let ephemeral_receipt_dir = if out_dir.is_none() {
        match reserve_workspace(None, "receipt", cassette_for_outcome) {
            Ok(lease) => Some(lease),
            Err(error) => {
                eprintln!(
                    "error: cannot reserve ephemeral receipt directory: {}",
                    bounded_diagnostic(&error)
                );
                return 2;
            }
        }
    } else {
        None
    };
    let receipt_dir = out_dir
        .as_deref()
        .or_else(|| ephemeral_receipt_dir.as_ref().map(WorkspaceLease::path))
        .expect("user or ephemeral cooperative receipt directory");
    let written_receipt = match write_cooperative_receipt(
        &workload,
        cassette_for_outcome,
        &campaign,
        workspace.path(),
        &provisional_fields,
        receipt_dir,
    ) {
        Ok(bytes) => bytes,
        Err(error) => {
            eprintln!(
                "error: cannot persist cooperative receipt: {}",
                bounded_diagnostic(&error)
            );
            return 2;
        }
    };
    let receipt_path = receipt_dir.join(COOPERATIVE_RECEIPT_NAME);
    let observed_receipt = match vh_sandbox::read_bounded_file(&receipt_path, MAX_RECEIPT_BYTES) {
        Ok(bytes) if bytes == written_receipt => bytes,
        Ok(_) => {
            eprintln!("error: persisted cooperative receipt changed before reverification");
            return 2;
        }
        Err(error) => {
            eprintln!(
                "error: cannot read persisted cooperative receipt: category={}",
                error.category()
            );
            return 2;
        }
    };
    let expected_request = ExpectedCooperativeRequest {
        workload: workload.clone(),
        cassette_bytes: cassette_for_outcome.file_bytes(),
    };
    let (verify_code, verify_fields) =
        verify_cooperative_receipt(&observed_receipt, Some(&expected_request), executions);
    if verify_code != 0 || verify_fields.is_empty() {
        eprintln!("error: strict cooperative reverification rejected the persisted receipt");
        return 2;
    }
    let (exit_code, fields) = match promoted_outcome_fields(&verify_fields, transport) {
        Ok(promoted) => promoted,
        Err(error) => {
            eprintln!("error: {}", bounded_diagnostic(&error));
            return 2;
        }
    };
    let line = render_line(&fields);

    if let Some(out_dir) = out_dir.as_deref() {
        if write_new_file(
            &out_dir.join("outcome.ndjson"),
            format!("{line}\n").as_bytes(),
        )
        .is_err()
        {
            eprintln!("error: cannot create outcome receipt without clobbering");
            return 2;
        }
    }

    println!("vibe-halt cooperative: workload={workload}");
    if let Some(c) = cassette_ref {
        println!("  cassette: {}", c.identity());
    } else {
        println!("  cassette: {}", fixture_cassette().identity());
    }
    println!(
        "  identities: first={} second={}",
        campaign.first.identity(),
        campaign.second.identity()
    );
    if let Some(t) = campaign.first.transport.as_ref() {
        println!(
            "  transport: served={} unconsumed={} taint={}",
            t.served.len(),
            t.unconsumed,
            t.taint.as_deref().unwrap_or("none")
        );
    }
    let verdict = match exit_code {
        0 => "CLEAN",
        1 => "FINDINGS",
        _ => "UNCHECKED",
    };
    println!("  verdict: {verdict} (Tier-2 D2)");
    println!("{line}");
    exit_code
}

// ---- item 4: persisted cooperative receipt + strict reverification ----

pub(crate) const COOPERATIVE_RECEIPT_SCHEMA: &str = "vh-cooperative-receipt-v1";
pub(crate) const COOPERATIVE_VERIFY_SCHEMA: &str = "vh-cooperative-verify-v1";
pub(crate) const COOPERATIVE_RECEIPT_NAME: &str = "cooperative.receipt";
/// Published maximum receipt size. Bounded by construction: cassette ≤
/// [`vh_sandbox::MAX_CASSETTE_BYTES`], two bounded artifacts, small
/// fixed fields. The verifier enforces it before allocation.
pub(crate) const MAX_RECEIPT_BYTES: u64 = 4 << 20; // 4 MiB
/// Domain separator for the canonical engine-request digest.
const ENGINE_REQUEST_DOMAIN: &str = "vh-cooperative-engine-request-v1";

fn frame_field(out: &mut Vec<u8>, tag: &str, bytes: &[u8]) {
    out.extend_from_slice(tag.as_bytes());
    out.push(b' ');
    out.extend_from_slice(bytes.len().to_string().as_bytes());
    out.push(b':');
    out.extend_from_slice(bytes);
    out.push(b'\n');
}

fn plain_line(out: &mut Vec<u8>, line: &str) {
    out.extend_from_slice(line.as_bytes());
    out.push(b'\n');
}

/// Domain-separated, length-framed digest of the canonical engine
/// request: workload, recomputed cassette identity, and child-source
/// digest. The output-root path is deliberately excluded — it is
/// invocation context, never engine input.
fn engine_request_digest(workload: &str, cassette_identity: &str, source_digest: &str) -> String {
    let mut out = Vec::new();
    plain_line(&mut out, ENGINE_REQUEST_DOMAIN);
    frame_field(&mut out, "workload", workload.as_bytes());
    frame_field(&mut out, "cassette-identity", cassette_identity.as_bytes());
    frame_field(&mut out, "child-source-digest", source_digest.as_bytes());
    vh_digest::sha256_hex(&out)
}

/// SHA-256 of the currently executing engine binary. Both the receipt
/// writer and the verifier compute this over their own executable; the
/// verifier requires equality, binding the receipt to the executing
/// engine.
pub(crate) fn current_engine_sha256() -> Result<String, String> {
    let exe = std::env::current_exe()
        .map_err(|_| "cannot resolve current executable: category=current-exe".to_string())?;
    current_engine_sha256_at(&exe)
}

fn current_engine_sha256_at(path: &Path) -> Result<String, String> {
    let bytes =
        vh_sandbox::read_bounded_file(path, vh_sandbox::MAX_EXECUTABLE_BYTES).map_err(|error| {
            format!(
                "cannot read current executable: category={}",
                error.category()
            )
        })?;
    Ok(vh_digest::sha256_hex(&bytes))
}

fn field_string(fields: &[(&str, Val)], name: &str) -> String {
    fields
        .iter()
        .find(|(k, _)| *k == name)
        .and_then(|(_, v)| v.as_str().map(str::to_string))
        .unwrap_or_default()
}

fn field_u64(fields: &[(&str, Val)], name: &str) -> u64 {
    fields
        .iter()
        .find(|(k, _)| *k == name)
        .and_then(|(_, v)| v.as_u64())
        .unwrap_or(0)
}

fn field_bool(fields: &[(&str, Val)], name: &str) -> bool {
    fields
        .iter()
        .find(|(k, _)| *k == name)
        .map(|(_, v)| matches!(v, Val::B(true)))
        .unwrap_or(false)
}

/// Construct the public cooperative outcome only after the strict persisted
/// receipt verifier has authenticated and freshly replayed it. The initial
/// campaign's outcome is provisional input to the receipt writer, never a
/// second promotion authority.
fn promoted_outcome_fields(
    verify_fields: &[(&str, Val)],
    transport: String,
) -> Result<(i32, OutcomeFields), String> {
    if field_string(verify_fields, "record") != "cooperative-verify"
        || field_string(verify_fields, "schema") != COOPERATIVE_VERIFY_SCHEMA
        || !field_bool(verify_fields, "authentic")
    {
        return Err("strict cooperative reverification did not authenticate the receipt".into());
    }
    let exit_code = field_u64(verify_fields, "outcome_exit_code");
    let exit_code = match exit_code {
        0 | 1 | 3 => exit_code as i32,
        _ => {
            return Err("strict cooperative reverification returned an invalid outcome code".into())
        }
    };
    Ok((
        exit_code,
        vec![
            ("record", Val::S("cooperative-outcome".into())),
            ("schema", Val::S(COOPERATIVE_OUTCOME_SCHEMA.into())),
            ("verdict", Val::S(field_string(verify_fields, "verdict"))),
            ("tier", Val::S("TIER2".into())),
            ("grade", Val::S("D2".into())),
            ("scope", Val::S(SCOPE.into())),
            ("workload", Val::S(field_string(verify_fields, "workload"))),
            (
                "cassette_identity",
                Val::S(field_string(verify_fields, "cassette_identity")),
            ),
            (
                "child_source_digest",
                Val::S(field_string(verify_fields, "child_source_digest")),
            ),
            (
                "engine_request_digest",
                Val::S(field_string(verify_fields, "engine_request_digest")),
            ),
            (
                "evidence_digest",
                Val::S(field_string(verify_fields, "evidence_digest")),
            ),
            (
                "result_digest",
                Val::S(field_string(verify_fields, "result_digest")),
            ),
            ("transport", Val::S(transport)),
            ("oracle", Val::S(COOPERATIVE_ORACLE.into())),
            (
                "oracle_evaluation",
                Val::S(field_string(verify_fields, "oracle_evaluation")),
            ),
            (
                "finding_identity",
                Val::S(field_string(verify_fields, "finding_identity")),
            ),
            (
                "findings_count",
                Val::N(field_u64(verify_fields, "findings_count")),
            ),
            ("exit_code", Val::N(exit_code as u64)),
            ("verified", Val::B(field_bool(verify_fields, "verified"))),
            ("authentic", Val::B(true)),
            (
                "engine_sha256",
                Val::S(field_string(verify_fields, "engine_sha256")),
            ),
            (
                "receipt_sha256",
                Val::S(field_string(verify_fields, "receipt_sha256")),
            ),
            ("errors", Val::S(field_string(verify_fields, "errors"))),
        ],
    ))
}

fn transport_summary(record: &vh_sandbox::RunRecord) -> String {
    record
        .transport
        .as_ref()
        .map(|t| t.identity_str())
        .unwrap_or_else(|| "none".into())
}

/// Read the original `out.txt` artifact bytes from a run workspace when
/// the run record carries the artifact; bounded like every other
/// trust-boundary read.
fn read_artifact_bytes(
    workspace: &Path,
    sub: &str,
    record: &vh_sandbox::RunRecord,
) -> Result<Option<Vec<u8>>, String> {
    if !record.artifacts.contains_key("out.txt") {
        return Ok(None);
    }
    let bytes = vh_sandbox::read_bounded_file(
        &workspace.join(sub).join("out.txt"),
        vh_sandbox::MAX_CASSETTE_BYTES,
    )
    .map_err(|e| format!("cannot read original artifact: {e}"))?;
    if record.artifacts.get("out.txt") != Some(&vh_sandbox::fnv_hex(&bytes)) {
        return Err("original artifact bytes do not match the run record".into());
    }
    Ok(Some(bytes))
}

/// Build and atomically persist the bounded cooperative receipt. The
/// canonical body has one fixed schema with exact field order and
/// types; a final digest record carries SHA-256 over the exact
/// preceding body bytes.
fn write_cooperative_receipt(
    workload: &str,
    cassette: &CassetteV2,
    campaign: &SandboxCampaign,
    workspace: &Path,
    fields: &[(&str, Val)],
    out_dir: &Path,
) -> Result<Vec<u8>, String> {
    let body = build_cooperative_receipt(workload, cassette, campaign, workspace, fields)?;
    publish_cooperative_receipt(out_dir, &body)?;
    Ok(body)
}

fn build_cooperative_receipt(
    workload: &str,
    cassette: &CassetteV2,
    campaign: &SandboxCampaign,
    workspace: &Path,
    fields: &[(&str, Val)],
) -> Result<Vec<u8>, String> {
    let cassette_bytes = cassette.file_bytes();
    let cassette_identity = cassette.identity();
    let source_digest = format!(
        "sha256:{}",
        vh_digest::sha256_hex(COOPERATIVE_ECHO_CHILD.as_bytes())
    );
    let engine_sha256 = current_engine_sha256()?;
    let first_artifact = read_artifact_bytes(workspace, "a", &campaign.first)?;
    let second_artifact = read_artifact_bytes(workspace, "b", &campaign.second)?;

    let mut body: Vec<u8> = Vec::new();
    plain_line(&mut body, COOPERATIVE_RECEIPT_SCHEMA);
    plain_line(&mut body, &format!("workload {workload}"));
    plain_line(&mut body, &format!("cassette-identity {cassette_identity}"));
    frame_field(&mut body, "cassette", &cassette_bytes);
    plain_line(
        &mut body,
        &format!("child-source-name {CHILD_LOGICAL_NAME}"),
    );
    plain_line(&mut body, &format!("child-source-digest {source_digest}"));
    frame_field(&mut body, "child-source", COOPERATIVE_ECHO_CHILD.as_bytes());
    plain_line(
        &mut body,
        &format!("first-identity {}", campaign.first.identity()),
    );
    plain_line(
        &mut body,
        &format!("second-identity {}", campaign.second.identity()),
    );
    for (tag, artifact) in [("first", &first_artifact), ("second", &second_artifact)] {
        match artifact {
            Some(bytes) => {
                frame_field(&mut body, &format!("{tag}-artifact"), bytes);
                plain_line(
                    &mut body,
                    &format!("{tag}-artifact-digest {}", vh_sandbox::fnv_hex(bytes)),
                );
            }
            None => {
                plain_line(&mut body, &format!("{tag}-artifact absent"));
                plain_line(&mut body, &format!("{tag}-artifact-digest none"));
            }
        }
    }
    frame_field(
        &mut body,
        "transport-first",
        transport_summary(&campaign.first).as_bytes(),
    );
    frame_field(
        &mut body,
        "transport-second",
        transport_summary(&campaign.second).as_bytes(),
    );
    plain_line(&mut body, &format!("oracle {COOPERATIVE_ORACLE}"));
    plain_line(
        &mut body,
        &format!(
            "oracle-evaluation {}",
            field_string(fields, "oracle_evaluation")
        ),
    );
    plain_line(
        &mut body,
        &format!(
            "finding-identity {}",
            field_string(fields, "finding_identity")
        ),
    );
    plain_line(
        &mut body,
        &format!("verdict {}", field_string(fields, "verdict")),
    );
    plain_line(
        &mut body,
        &format!("exit-code {}", field_u64(fields, "exit_code")),
    );
    plain_line(
        &mut body,
        &format!("verified {}", field_bool(fields, "verified")),
    );
    plain_line(
        &mut body,
        &format!("findings-count {}", field_u64(fields, "findings_count")),
    );
    frame_field(
        &mut body,
        "errors",
        field_string(fields, "errors").as_bytes(),
    );
    plain_line(
        &mut body,
        &format!(
            "engine-request-digest {}",
            engine_request_digest(workload, &cassette_identity, &source_digest)
        ),
    );
    plain_line(&mut body, &format!("engine-sha256 {engine_sha256}"));
    let body_digest = vh_digest::sha256_hex(&body);
    plain_line(&mut body, &format!("digest sha256:{body_digest}"));
    if body.len() as u64 > MAX_RECEIPT_BYTES {
        return Err("receipt exceeds the published maximum receipt size".into());
    }

    Ok(body)
}

fn publish_cooperative_receipt(out_dir: &Path, body: &[u8]) -> Result<(), String> {
    let tmp = out_dir.join("cooperative.receipt.tmp");
    let final_path = out_dir.join(COOPERATIVE_RECEIPT_NAME);
    write_new_file(&tmp, body).map_err(|_| "cannot create receipt temporary file".to_string())?;
    match std::fs::hard_link(&tmp, &final_path) {
        Ok(()) => {
            std::fs::remove_file(&tmp)
                .map_err(|_| "receipt published but temporary link remains".to_string())?;
            let published = vh_sandbox::read_bounded_file(&final_path, MAX_RECEIPT_BYTES)
                .map_err(|_| "published receipt cannot be re-opened safely".to_string())?;
            if published != body {
                return Err("published receipt bytes changed during publication".into());
            }
            Ok(())
        }
        Err(_) => {
            // Delete only the controller-created temporary inode. A
            // pre-existing final receipt is never removed or overwritten.
            let _ = std::fs::remove_file(&tmp);
            Err("cannot publish receipt without clobbering an existing path".into())
        }
    }
}

/// A parsed canonical receipt. Parsing is positional and total: exactly
/// one supported schema, exact field order, no duplicates, unknowns,
/// reordering, truncation, blank interior lines, or trailing data.
struct ParsedReceipt {
    workload: String,
    cassette_identity: String,
    cassette_bytes: Vec<u8>,
    child_source_name: String,
    child_source_digest: String,
    child_source: Vec<u8>,
    first_identity: String,
    second_identity: String,
    first_artifact: Option<Vec<u8>>,
    first_artifact_digest: String,
    second_artifact: Option<Vec<u8>>,
    second_artifact_digest: String,
    transport_first: String,
    transport_second: String,
    oracle: String,
    oracle_evaluation: String,
    finding_identity: String,
    verdict: String,
    exit_code: u64,
    verified: bool,
    findings_count: u64,
    errors_json: String,
    engine_request_digest: String,
    engine_sha256: String,
    body_digest: String,
    body_bytes: Vec<u8>,
}

struct ReceiptReader<'a> {
    bytes: &'a [u8],
    pos: usize,
}

impl<'a> ReceiptReader<'a> {
    fn take_line(&mut self) -> Result<&'a str, &'static str> {
        let rest = &self.bytes[self.pos..];
        let nl = rest.iter().position(|b| *b == b'\n').ok_or("truncated")?;
        let line = std::str::from_utf8(&rest[..nl]).map_err(|_| "invalid-utf8")?;
        if line.is_empty() {
            return Err("blank-interior-line");
        }
        self.pos += nl + 1;
        Ok(line)
    }

    fn expect_exact(&mut self, expected: &str) -> Result<(), &'static str> {
        match self.take_line()? {
            line if line == expected => Ok(()),
            _ => Err("field-order"),
        }
    }

    /// `<tag> <value>\n` where value is nonempty and whitespace-free.
    fn expect_value(&mut self, tag: &str) -> Result<String, &'static str> {
        let line = self.take_line()?;
        let value = line
            .strip_prefix(tag)
            .and_then(|rest| rest.strip_prefix(' '))
            .ok_or("field-order")?;
        if value.is_empty() || value.chars().any(char::is_whitespace) {
            return Err("field-type");
        }
        Ok(value.to_string())
    }

    fn expect_hex(&mut self, tag: &str, len: usize) -> Result<String, &'static str> {
        let value = self.expect_value(tag)?;
        if value.len() != len
            || !value
                .chars()
                .all(|c| c.is_ascii_digit() || ('a'..='f').contains(&c))
        {
            return Err("field-type");
        }
        Ok(value)
    }

    fn expect_u64(&mut self, tag: &str) -> Result<u64, &'static str> {
        let value = self.expect_value(tag)?;
        let parsed: u64 = value.parse().map_err(|_| "field-type")?;
        if parsed.to_string() != value {
            return Err("noncanonical-number");
        }
        Ok(parsed)
    }

    /// `<tag> <len>:<raw bytes>\n` with exact length accounting.
    fn expect_framed(&mut self, tag: &str) -> Result<Vec<u8>, &'static str> {
        let rest = &self.bytes[self.pos..];
        let prefix = format!("{tag} ");
        if !rest.starts_with(prefix.as_bytes()) {
            return Err("field-order");
        }
        let mut idx = prefix.len();
        let len_start = idx;
        while idx < rest.len() && rest[idx].is_ascii_digit() {
            idx += 1;
        }
        if idx == len_start || idx >= rest.len() || rest[idx] != b':' {
            return Err("field-type");
        }
        let len_text = std::str::from_utf8(&rest[len_start..idx]).map_err(|_| "field-type")?;
        let len: usize = len_text.parse().map_err(|_| "field-type")?;
        if len.to_string() != len_text {
            return Err("noncanonical-number");
        }
        let value_start = idx + 1;
        let value_end = value_start.checked_add(len).ok_or("field-type")?;
        let field_end = value_end.checked_add(1).ok_or("field-type")?;
        if rest.len() < field_end {
            return Err("truncated");
        }
        if rest[value_end] != b'\n' {
            return Err("field-type");
        }
        let value = rest[value_start..value_end].to_vec();
        self.pos += field_end;
        Ok(value)
    }
}

fn parse_receipt(bytes: &[u8]) -> Result<ParsedReceipt, &'static str> {
    let mut r = ReceiptReader { bytes, pos: 0 };
    r.expect_exact(COOPERATIVE_RECEIPT_SCHEMA)?;
    let workload = r.expect_value("workload")?;
    let cassette_identity = r.expect_value("cassette-identity")?;
    let cassette_bytes = r.expect_framed("cassette")?;
    let child_source_name = r.expect_value("child-source-name")?;
    let child_source_digest = r.expect_value("child-source-digest")?;
    let child_source = r.expect_framed("child-source")?;
    let first_identity = r.expect_hex("first-identity", 32)?;
    let second_identity = r.expect_hex("second-identity", 32)?;
    let mut artifacts = Vec::new();
    for tag in ["first", "second"] {
        let save = r.pos;
        match r.expect_exact(&format!("{tag}-artifact absent")) {
            Ok(()) => {
                r.expect_exact(&format!("{tag}-artifact-digest none"))?;
                artifacts.push((None, "none".to_string()));
            }
            Err(_) => {
                r.pos = save;
                let bytes = r.expect_framed(&format!("{tag}-artifact"))?;
                let digest = r.expect_hex(&format!("{tag}-artifact-digest"), 32)?;
                artifacts.push((Some(bytes), digest));
            }
        }
    }
    let (first_artifact, first_artifact_digest) = artifacts.remove(0);
    let (second_artifact, second_artifact_digest) = artifacts.remove(0);
    let transport_first = r.expect_framed("transport-first")?;
    let transport_second = r.expect_framed("transport-second")?;
    let transport_first = String::from_utf8(transport_first).map_err(|_| "invalid-utf8")?;
    let transport_second = String::from_utf8(transport_second).map_err(|_| "invalid-utf8")?;
    let oracle = r.expect_value("oracle")?;
    let oracle_evaluation = r.expect_value("oracle-evaluation")?;
    let finding_identity = r.expect_value("finding-identity")?;
    let verdict = r.expect_value("verdict")?;
    if !matches!(verdict.as_str(), "CLEAN" | "FINDINGS" | "UNCHECKED") {
        return Err("field-type");
    }
    let exit_code = r.expect_u64("exit-code")?;
    let verified = match r.expect_value("verified")?.as_str() {
        "true" => true,
        "false" => false,
        _ => return Err("field-type"),
    };
    let findings_count = r.expect_u64("findings-count")?;
    let errors_json = r.expect_framed("errors")?;
    let errors_json = String::from_utf8(errors_json).map_err(|_| "invalid-utf8")?;
    let engine_request_digest = r.expect_hex("engine-request-digest", 64)?;
    let engine_sha256 = r.expect_hex("engine-sha256", 64)?;
    let body_end = r.pos;
    let digest_line = r.take_line()?;
    let body_digest = digest_line
        .strip_prefix("digest sha256:")
        .ok_or("field-order")?
        .to_string();
    if body_digest.len() != 64
        || !body_digest
            .chars()
            .all(|c| c.is_ascii_digit() || ('a'..='f').contains(&c))
    {
        return Err("field-type");
    }
    if r.pos != bytes.len() {
        return Err("trailing-data");
    }
    Ok(ParsedReceipt {
        workload,
        cassette_identity,
        cassette_bytes,
        child_source_name,
        child_source_digest,
        child_source,
        first_identity,
        second_identity,
        first_artifact,
        first_artifact_digest,
        second_artifact,
        second_artifact_digest,
        transport_first,
        transport_second,
        oracle,
        oracle_evaluation,
        finding_identity,
        verdict,
        exit_code,
        verified,
        findings_count,
        errors_json,
        engine_request_digest,
        engine_sha256,
        body_digest,
        body_bytes: bytes[..body_end].to_vec(),
    })
}

/// Closed workload enum: the verifier NEVER executes receipt-provided
/// source. The workload name selects the source compiled into this
/// trusted executable, and the persisted source must byte-equal it.
enum CooperativeWorkload {
    CooperativeEcho,
}

struct ExpectedCooperativeRequest {
    workload: String,
    cassette_bytes: Vec<u8>,
}

impl CooperativeWorkload {
    fn from_name(name: &str) -> Option<Self> {
        match name {
            "cooperative-echo" => Some(Self::CooperativeEcho),
            _ => None,
        }
    }

    fn source(&self) -> &'static str {
        match self {
            Self::CooperativeEcho => COOPERATIVE_ECHO_CHILD,
        }
    }
}

/// Strict reverification (item 4): validate the canonical receipt,
/// recompute every embedded content/identity digest, bind the executing
/// engine, then reconstruct a fresh unique workspace from the validated
/// request, the persisted cassette, and the compiled-in trusted source,
/// run the campaign twice again, reevaluate the oracle, and compare
/// every derived field against the receipt. Returns the process exit
/// code and the typed verify record.
fn verify_cooperative_receipt(
    bytes: &[u8],
    expected_request: Option<&ExpectedCooperativeRequest>,
    executions: &mut u64,
) -> (i32, Vec<(&'static str, Val)>) {
    let receipt_sha256 = vh_digest::sha256_hex(bytes);
    // One path observation is used for both comparison and reporting. The
    // loaded-image/filesystem race remains an explicitly open D2 channel.
    let observed_engine = current_engine_sha256();
    let engine_sha256 = observed_engine
        .as_ref()
        .cloned()
        .unwrap_or_else(|_| "0".repeat(64));

    let finish = |authentic: bool,
                  outcome_verified: bool,
                  verdict: &str,
                  outcome_exit_code: u64,
                  workload: &str,
                  cassette_identity: &str,
                  child_source_digest: &str,
                  engine_request_digest: &str,
                  oracle_evaluation: &str,
                  finding_identity: &str,
                  findings_count: u64,
                  evidence_digest: &str,
                  result_digest: &str,
                  errors: Vec<String>,
                  receipt_sha256: &str,
                  engine_sha256: &str| {
        let verified = authentic && outcome_verified;
        let exit_code = if authentic { 0 } else { 1 };
        let fields = vec![
            ("record", Val::S("cooperative-verify".into())),
            ("schema", Val::S(COOPERATIVE_VERIFY_SCHEMA.into())),
            ("verdict", Val::S(verdict.into())),
            ("tier", Val::S("TIER2".into())),
            ("grade", Val::S("D2".into())),
            ("scope", Val::S(SCOPE.into())),
            ("workload", Val::S(workload.into())),
            ("cassette_identity", Val::S(cassette_identity.into())),
            ("child_source_digest", Val::S(child_source_digest.into())),
            (
                "engine_request_digest",
                Val::S(engine_request_digest.into()),
            ),
            ("oracle", Val::S(COOPERATIVE_ORACLE.into())),
            ("oracle_evaluation", Val::S(oracle_evaluation.into())),
            ("finding_identity", Val::S(finding_identity.into())),
            ("findings_count", Val::N(findings_count)),
            ("evidence_digest", Val::S(evidence_digest.into())),
            ("result_digest", Val::S(result_digest.into())),
            ("engine_sha256", Val::S(engine_sha256.into())),
            ("receipt_sha256", Val::S(receipt_sha256.into())),
            ("authentic", Val::B(authentic)),
            ("verified", Val::B(verified)),
            ("outcome_exit_code", Val::N(outcome_exit_code)),
            ("exit_code", Val::N(exit_code as u64)),
            ("errors", Val::S(errors_to_json_array(&errors))),
        ];
        (exit_code, fields)
    };

    let parsed = match parse_receipt(bytes) {
        Ok(p) => p,
        Err(category) => {
            // Structural failures are usage-class: reported on stderr by
            // the command wrapper, never as a machine record.
            eprintln!("error: malformed cooperative receipt: {category}");
            return (2, Vec::new());
        }
    };

    // Pre-execution checks. Every failure here is fail-closed BEFORE
    // any child launch: the execution counter stays at zero.
    let mut pre_errors: Vec<String> = Vec::new();
    if vh_digest::sha256_hex(&parsed.body_bytes) != parsed.body_digest {
        pre_errors.push("body-digest-mismatch".into());
    }
    if parsed.cassette_bytes.len() as u64 > vh_sandbox::MAX_CASSETTE_BYTES {
        pre_errors.push("cassette-oversized".into());
    }
    let cassette = if parsed.cassette_bytes.len() as u64 <= vh_sandbox::MAX_CASSETTE_BYTES {
        match CassetteV2::parse_detailed(&parsed.cassette_bytes) {
            Ok(cassette) if cassette.file_bytes() == parsed.cassette_bytes => Some(cassette),
            Ok(_) => {
                pre_errors.push("cassette-noncanonical".into());
                None
            }
            Err(_) => {
                pre_errors.push("cassette-unparseable".into());
                None
            }
        }
    } else {
        None
    };
    if let Some(cassette) = cassette.as_ref() {
        if cassette.identity() != parsed.cassette_identity {
            pre_errors.push("cassette-identity-mismatch".into());
        }
    }
    let recomputed_source_digest =
        format!("sha256:{}", vh_digest::sha256_hex(&parsed.child_source));
    if recomputed_source_digest != parsed.child_source_digest {
        pre_errors.push("source-digest-mismatch".into());
    }
    match CooperativeWorkload::from_name(&parsed.workload) {
        Some(workload) => {
            if parsed.child_source_name != CHILD_LOGICAL_NAME
                || parsed.child_source != workload.source().as_bytes()
            {
                pre_errors.push("source-mismatch".into());
            }
        }
        None => pre_errors.push("unknown-workload".into()),
    }
    for (tag, bytes, digest) in [
        (
            "first",
            &parsed.first_artifact,
            &parsed.first_artifact_digest,
        ),
        (
            "second",
            &parsed.second_artifact,
            &parsed.second_artifact_digest,
        ),
    ] {
        match bytes {
            Some(bytes) if bytes.len() as u64 > vh_sandbox::MAX_ARTIFACT_BYTES => {
                pre_errors.push(format!("{tag}-artifact-oversized"));
            }
            Some(bytes) if vh_sandbox::fnv_hex(bytes) == *digest => {}
            None if digest == "none" => {}
            _ => pre_errors.push(format!("{tag}-artifact-digest-mismatch")),
        }
    }
    let decoded_errors = decode_json_string_array(&parsed.errors_json);
    match &decoded_errors {
        Ok(errors)
            if errors_to_json_array(errors) == parsed.errors_json
                && errors
                    .iter()
                    .all(|error| error.len() <= MAX_DIAGNOSTIC_BYTES) => {}
        _ => pre_errors.push("errors-noncanonical".into()),
    }
    let claimed_shape = match parsed.verdict.as_str() {
        "CLEAN" => {
            parsed.exit_code == 0
                && parsed.verified
                && parsed.findings_count == 0
                && parsed.oracle_evaluation == "completed"
                && parsed.finding_identity == "none"
                && decoded_errors.as_ref().is_ok_and(Vec::is_empty)
        }
        "FINDINGS" => {
            parsed.exit_code == 1
                && parsed.verified
                && parsed.findings_count == 1
                && parsed.oracle_evaluation == "not-completed:timeout"
                && parsed.finding_identity == FINDING_TIMEOUT
                && decoded_errors.as_ref().is_ok_and(Vec::is_empty)
        }
        "UNCHECKED" => {
            parsed.exit_code == 3
                && !parsed.verified
                && parsed.findings_count == 0
                && parsed.oracle_evaluation == "indeterminate"
                && parsed.finding_identity == "none"
        }
        _ => false,
    };
    if parsed.oracle != COOPERATIVE_ORACLE || !claimed_shape {
        pre_errors.push("derived-shape-mismatch".into());
    }
    let recomputed_engine_request_digest = engine_request_digest(
        &parsed.workload,
        &parsed.cassette_identity,
        &parsed.child_source_digest,
    );
    if recomputed_engine_request_digest != parsed.engine_request_digest {
        pre_errors.push("engine-request-digest-mismatch".into());
    }
    match &observed_engine {
        Ok(current) if current == &parsed.engine_sha256 => {}
        Ok(_) => pre_errors.push("engine-mismatch".into()),
        Err(_) => pre_errors.push("engine-unresolved".into()),
    }
    if let Some(expected) = expected_request {
        if parsed.workload != expected.workload {
            pre_errors.push("expected-workload-mismatch".into());
        }
        if parsed.cassette_bytes != expected.cassette_bytes {
            pre_errors.push("expected-cassette-mismatch".into());
        }
    }
    if !pre_errors.is_empty() {
        // Never reflect a parsed or caller-supplied workload through the
        // machine-readable failure record. The only supported cooperative
        // workload has a fixed public identifier.
        let safe_workload = "cooperative-echo";
        let safe_cassette_identity = format!("vh-cassette-v2:sha256:{}", "0".repeat(64));
        let safe_source_digest = format!("sha256:{}", "0".repeat(64));
        let safe_sha256 = "0".repeat(64);
        let safe_identity = "0".repeat(32);
        return finish(
            false,
            false,
            "UNCHECKED",
            3,
            safe_workload,
            &safe_cassette_identity,
            &safe_source_digest,
            &safe_sha256,
            "indeterminate",
            "none",
            0,
            &safe_identity,
            &safe_identity,
            pre_errors,
            &receipt_sha256,
            &engine_sha256,
        );
    }

    // Fresh replay from the validated canonical request, the persisted
    // cassette, and the compiled-in trusted source.
    let cassette = cassette.expect("pre-execution cassette validation established Some");
    let (campaign, replay_workspace) =
        match run_cooperative_campaign(Some(&cassette), &parsed.workload, None, executions) {
            Ok(v) => v,
            Err(e) => {
                return finish(
                    false,
                    false,
                    "UNCHECKED",
                    3,
                    &parsed.workload,
                    &parsed.cassette_identity,
                    &parsed.child_source_digest,
                    &parsed.engine_request_digest,
                    "indeterminate",
                    "none",
                    0,
                    &parsed.first_identity,
                    &parsed.second_identity,
                    vec![bounded_diagnostic(&format!("replay-failed: {e}"))],
                    &receipt_sha256,
                    &engine_sha256,
                );
            }
        };
    let (_replay_code, replay_fields) = outcome_fields(&campaign, &cassette);

    let mut mismatches: Vec<String> = Vec::new();
    if campaign.first.identity() != parsed.first_identity {
        mismatches.push("replay-first-identity-mismatch".into());
    }
    if campaign.second.identity() != parsed.second_identity {
        mismatches.push("replay-second-identity-mismatch".into());
    }
    for (tag, sub, record, recorded_bytes, recorded_digest) in [
        (
            "first",
            "a",
            &campaign.first,
            &parsed.first_artifact,
            &parsed.first_artifact_digest,
        ),
        (
            "second",
            "b",
            &campaign.second,
            &parsed.second_artifact,
            &parsed.second_artifact_digest,
        ),
    ] {
        let replay_artifact = match read_artifact_bytes(replay_workspace.path(), sub, record) {
            Ok(v) => v,
            Err(e) => {
                mismatches.push(bounded_diagnostic(&e));
                continue;
            }
        };
        if &replay_artifact != recorded_bytes {
            mismatches.push(format!("replay-{tag}-artifact-mismatch"));
        }
        match recorded_bytes {
            Some(bytes) => {
                if vh_sandbox::fnv_hex(bytes) != *recorded_digest {
                    mismatches.push(format!("{tag}-artifact-digest-mismatch"));
                }
            }
            None => {
                if recorded_digest != "none" {
                    mismatches.push(format!("{tag}-artifact-digest-mismatch"));
                }
            }
        }
    }
    if transport_summary(&campaign.first) != parsed.transport_first {
        mismatches.push("replay-first-transport-mismatch".into());
    }
    if transport_summary(&campaign.second) != parsed.transport_second {
        mismatches.push("replay-second-transport-mismatch".into());
    }
    if parsed.oracle != COOPERATIVE_ORACLE {
        mismatches.push("oracle-mismatch".into());
    }
    if field_string(&replay_fields, "oracle_evaluation") != parsed.oracle_evaluation {
        mismatches.push("replay-oracle-evaluation-mismatch".into());
    }
    if field_string(&replay_fields, "finding_identity") != parsed.finding_identity {
        mismatches.push("replay-finding-identity-mismatch".into());
    }
    if field_string(&replay_fields, "verdict") != parsed.verdict {
        mismatches.push("replay-verdict-mismatch".into());
    }
    if field_u64(&replay_fields, "exit_code") != parsed.exit_code {
        mismatches.push("replay-exit-code-mismatch".into());
    }
    if field_bool(&replay_fields, "verified") != parsed.verified {
        mismatches.push("replay-verified-mismatch".into());
    }
    if field_u64(&replay_fields, "findings_count") != parsed.findings_count {
        mismatches.push("replay-findings-count-mismatch".into());
    }
    if field_string(&replay_fields, "errors") != parsed.errors_json {
        mismatches.push("replay-errors-mismatch".into());
    }

    let authentic = mismatches.is_empty();
    // The verifier's reported fields are the FRESH replay's, never the
    // receipt's claims; the receipt's own verdict is never trusted.
    let replay_errors_json = field_string(&replay_fields, "errors");
    let mut reported_errors = mismatches;
    if authentic {
        if let Ok(decoded) = decode_json_string_array(&replay_errors_json) {
            reported_errors = decoded;
        }
    }
    finish(
        authentic,
        field_bool(&replay_fields, "verified"),
        &field_string(&replay_fields, "verdict"),
        field_u64(&replay_fields, "exit_code"),
        &parsed.workload,
        &parsed.cassette_identity,
        &parsed.child_source_digest,
        &parsed.engine_request_digest,
        &field_string(&replay_fields, "oracle_evaluation"),
        &field_string(&replay_fields, "finding_identity"),
        field_u64(&replay_fields, "findings_count"),
        &campaign.first.identity(),
        &campaign.second.identity(),
        reported_errors,
        &receipt_sha256,
        &engine_sha256,
    )
}

/// Decode a `["a","b"]` JSON string array of the exact shape produced
/// by `errors_to_json_array`.
fn decode_json_string_array(json: &str) -> Result<Vec<String>, String> {
    parse_string_array_manual(json)
}

fn parse_string_array_manual(json: &str) -> Result<Vec<String>, String> {
    let s = json.trim();
    if !s.starts_with('[') || !s.ends_with(']') {
        return Err("not an array".into());
    }
    let inner = &s[1..s.len() - 1];
    if inner.is_empty() {
        return Ok(Vec::new());
    }
    let mut out = Vec::new();
    let mut chars = inner.chars().peekable();
    loop {
        if out.len() == 64 {
            return Err("too many diagnostics".into());
        }
        if chars.next() != Some('"') {
            return Err("expected string".into());
        }
        let mut value = String::new();
        loop {
            match chars.next() {
                Some('"') => break,
                Some('\\') => match chars.next() {
                    Some('"') => value.push('"'),
                    Some('\\') => value.push('\\'),
                    Some('n') => value.push('\n'),
                    Some('r') => value.push('\r'),
                    Some('t') => value.push('\t'),
                    Some('u') => {
                        let hex: String = chars.by_ref().take(4).collect();
                        let code = u32::from_str_radix(&hex, 16).map_err(|_| "bad escape")?;
                        value.push(char::from_u32(code).ok_or("bad escape")?);
                    }
                    _ => return Err("bad escape".into()),
                },
                Some(c) => value.push(c),
                None => return Err("unterminated".into()),
            }
            if value.len() > MAX_DIAGNOSTIC_BYTES {
                return Err("diagnostic too long".into());
            }
        }
        out.push(value);
        match chars.next() {
            Some(',') => continue,
            None => break,
            _ => return Err("expected comma".into()),
        }
    }
    if chars.next().is_some() {
        return Err("trailing".into());
    }
    Ok(out)
}

pub(crate) fn cmd_verify_cooperative(args: &[String], usage: &str) -> i32 {
    let mut receipt_path: Option<String> = None;
    let mut expected_workload: Option<String> = None;
    let mut expected_cassette_path: Option<String> = None;
    let mut expect_default_cassette = false;
    let mut it = args.iter();
    while let Some(arg) = it.next() {
        match arg.as_str() {
            "--receipt" => {
                if receipt_path.is_some() {
                    eprintln!("error: duplicate --receipt\n\n{usage}");
                    return 2;
                }
                receipt_path = match it.next() {
                    Some(v) => Some(v.clone()),
                    None => {
                        eprintln!("error: --receipt requires a value\n\n{usage}");
                        return 2;
                    }
                };
            }
            "--expected-workload" => {
                if expected_workload.is_some() {
                    eprintln!("error: duplicate --expected-workload\n\n{usage}");
                    return 2;
                }
                expected_workload = match it.next() {
                    Some(value) => Some(value.clone()),
                    None => {
                        eprintln!("error: --expected-workload requires a value\n\n{usage}");
                        return 2;
                    }
                };
            }
            "--expected-cassette" => {
                if expected_cassette_path.is_some() || expect_default_cassette {
                    eprintln!("error: duplicate expected cassette mode\n\n{usage}");
                    return 2;
                }
                expected_cassette_path = match it.next() {
                    Some(value) => Some(value.clone()),
                    None => {
                        eprintln!("error: --expected-cassette requires a value\n\n{usage}");
                        return 2;
                    }
                };
            }
            "--expect-default-cassette" => {
                if expected_cassette_path.is_some() || expect_default_cassette {
                    eprintln!("error: duplicate expected cassette mode\n\n{usage}");
                    return 2;
                }
                expect_default_cassette = true;
            }
            other => {
                eprintln!(
                    "error: unknown argument: {}\n\n{usage}",
                    bounded_diagnostic(other)
                );
                return 2;
            }
        }
    }
    let receipt_path = match receipt_path {
        Some(p) => p,
        None => {
            eprintln!("error: verify-cooperative requires --receipt PATH\n\n{usage}");
            return 2;
        }
    };
    let path = PathBuf::from(&receipt_path);
    if !path.is_absolute()
        || path.file_name().and_then(|name| name.to_str()) != Some(COOPERATIVE_RECEIPT_NAME)
        || path
            .components()
            .any(|part| matches!(part, Component::ParentDir | Component::CurDir))
    {
        eprintln!("error: receipt must be an absolute canonical cooperative.receipt path");
        return 2;
    }
    // Strictly bounded before allocation.
    let bytes = match vh_sandbox::read_bounded_file(&path, MAX_RECEIPT_BYTES) {
        Ok(b) => b,
        Err(e) => {
            eprintln!("error: cannot read receipt: category={}", e.category());
            return 2;
        }
    };
    let expected_mode = expected_cassette_path.is_some() || expect_default_cassette;
    if expected_workload.is_some() != expected_mode {
        eprintln!(
            "error: expected request binding requires --expected-workload and exactly one cassette mode"
        );
        return 2;
    }
    let expected_cassette_bytes = if expect_default_cassette {
        Some(fixture_cassette().file_bytes())
    } else if let Some(expected_path) = expected_cassette_path {
        let expected_path = PathBuf::from(expected_path);
        if !expected_path.is_absolute() {
            eprintln!("error: expected cassette path must be absolute");
            return 2;
        }
        let expected_bytes =
            match vh_sandbox::read_bounded_file(&expected_path, vh_sandbox::MAX_CASSETTE_BYTES) {
                Ok(value) => value,
                Err(SandboxError::Oversized { max, .. }) => {
                    eprintln!("error: expected cassette exceeds the {max}-byte bound");
                    return 2;
                }
                Err(error) => {
                    eprintln!(
                        "error: cannot read expected cassette: category={}",
                        error.category()
                    );
                    return 2;
                }
            };
        match CassetteV2::parse_detailed(&expected_bytes) {
            Ok(value) if value.file_bytes() == expected_bytes => Some(expected_bytes),
            _ => {
                eprintln!("error: expected cassette is malformed or noncanonical");
                return 2;
            }
        }
    } else {
        None
    };
    let expected_request = expected_workload.map(|workload| ExpectedCooperativeRequest {
        workload,
        cassette_bytes: expected_cassette_bytes.expect("expected mode validated"),
    });
    let mut executions = 0u64;
    let (code, fields) =
        verify_cooperative_receipt(&bytes, expected_request.as_ref(), &mut executions);
    if !fields.is_empty() {
        println!("{}", render_line(&fields));
    }
    code
}

// ---- issue #90: negotiated, revision-bound cooperative v2 ----

#[derive(Clone)]
struct CooperativeV2Request {
    protocol_schema: String,
    manifest_id: String,
    operation: String,
    features: Vec<String>,
    requested_revision: crate::protocol::RequestedTargetRevision,
    cassette_path: Option<String>,
    out_path: Option<String>,
}

fn parse_cooperative_v2_args(args: &[String]) -> Result<CooperativeV2Request, String> {
    let mut protocol_schema = None;
    let mut manifest_id = None;
    let mut operation = None;
    let mut features = Vec::new();
    let mut requested_revision = None;
    let mut cassette_path = None;
    let mut out_path = None;
    let mut it = args.iter();
    while let Some(argument) = it.next() {
        let mut value = |flag: &str| {
            it.next()
                .cloned()
                .ok_or_else(|| format!("{flag} requires a value"))
        };
        match argument.as_str() {
            "--protocol-schema" if protocol_schema.is_none() => {
                protocol_schema = Some(value(argument)?)
            }
            "--manifest-id" if manifest_id.is_none() => manifest_id = Some(value(argument)?),
            "--operation" if operation.is_none() => operation = Some(value(argument)?),
            "--require-feature" => features.push(value(argument)?),
            "--requested-target-revision" if requested_revision.is_none() => {
                requested_revision = Some(
                    crate::protocol::RequestedTargetRevision::parse(&value(argument)?)
                        .map_err(|_| "invalid requested target revision".to_string())?,
                )
            }
            "--cassette" if cassette_path.is_none() => cassette_path = Some(value(argument)?),
            "--out" if out_path.is_none() => out_path = Some(value(argument)?),
            other => return Err(format!("duplicate or unknown argument: {other}")),
        }
    }
    Ok(CooperativeV2Request {
        protocol_schema: protocol_schema.ok_or("missing --protocol-schema")?,
        manifest_id: manifest_id.ok_or("missing --manifest-id")?,
        operation: operation.ok_or("missing --operation")?,
        features,
        requested_revision: requested_revision.ok_or("missing --requested-target-revision")?,
        cassette_path,
        out_path,
    })
}

fn load_cooperative_cassette(path: Option<&str>) -> Result<CassetteV2, String> {
    match path {
        None => Ok(fixture_cassette()),
        Some(path) => {
            let bytes =
                vh_sandbox::read_bounded_file(Path::new(path), vh_sandbox::MAX_CASSETTE_BYTES)
                    .map_err(|error| {
                        format!("cannot read cassette: category={}", error.category())
                    })?;
            let cassette = CassetteV2::parse_detailed(&bytes)
                .map_err(|error| format!("malformed cassette: category={}", error.category()))?;
            if cassette.file_bytes() != bytes {
                return Err("malformed cassette: category=noncanonical-encoding".into());
            }
            Ok(cassette)
        }
    }
}

fn emit_v2_refusal(
    reason: crate::protocol::RefusalReason,
    manifest: &crate::protocol::ProtocolManifest,
) -> i32 {
    let stdout = std::io::stdout();
    let mut output = stdout.lock();
    emit_v2_refusal_to(reason, manifest, &mut output)
}

fn emit_v2_refusal_to<W: Write>(
    reason: crate::protocol::RefusalReason,
    manifest: &crate::protocol::ProtocolManifest,
    output: &mut W,
) -> i32 {
    let bytes =
        crate::protocol::encode_refusal(reason, &manifest.engine_sha256, &manifest.manifest_id);
    match output.write_all(&bytes) {
        Ok(()) => 4,
        Err(_) => 2,
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum V2VerifyFailureReason {
    MalformedReceipt,
    ExpectedRequestMismatch,
    RevisionMismatch,
    IdentityMismatch,
    FreshReplayFailed,
}

impl V2VerifyFailureReason {
    fn as_str(self) -> &'static str {
        match self {
            Self::MalformedReceipt => "malformed-receipt",
            Self::ExpectedRequestMismatch => "expected-request-mismatch",
            Self::RevisionMismatch => "revision-mismatch",
            Self::IdentityMismatch => "identity-mismatch",
            Self::FreshReplayFailed => "fresh-replay-failed",
        }
    }
}

fn classify_v2_verify_failure(error: &str, executions: u64) -> V2VerifyFailureReason {
    if executions > 0 {
        return V2VerifyFailureReason::FreshReplayFailed;
    }
    if error.contains("malformed") || error.contains("unparseable") {
        V2VerifyFailureReason::MalformedReceipt
    } else if error.contains("expected-request") || error.contains("expected-cassette") {
        V2VerifyFailureReason::ExpectedRequestMismatch
    } else if error.contains("revision") || error.contains("observation") {
        V2VerifyFailureReason::RevisionMismatch
    } else {
        V2VerifyFailureReason::IdentityMismatch
    }
}

fn encode_v2_verify_failure(
    reason: V2VerifyFailureReason,
    manifest: &crate::protocol::ProtocolManifest,
    receipt_sha256: &str,
    executions: u64,
) -> Vec<u8> {
    let mut record = Vec::new();
    plain_line(&mut record, crate::protocol::VERIFY_FAILURE_SCHEMA);
    frame_field(&mut record, "reason", reason.as_str().as_bytes());
    frame_field(
        &mut record,
        "engine-sha256",
        manifest.engine_sha256.as_bytes(),
    );
    frame_field(&mut record, "manifest-id", manifest.manifest_id.as_bytes());
    frame_field(&mut record, "receipt-sha256", receipt_sha256.as_bytes());
    plain_line(&mut record, &format!("executions {executions}"));
    plain_line(&mut record, "authentic false");
    plain_line(&mut record, "verified false");
    plain_line(&mut record, "exit-code 1");
    record
}

#[derive(Clone)]
struct ParsedV2Receipt {
    protocol_schema: String,
    manifest_id: String,
    engine_sha256: String,
    operation: String,
    request_schema: String,
    outcome_schema: String,
    receipt_schema: String,
    verifier_schema: String,
    features: Vec<String>,
    observation_subject: String,
    revision_algorithm: String,
    revision_policy: String,
    execution_binding: String,
    observation_to_exec_channel: String,
    requested_revision: String,
    claimed_revision: crate::protocol::ClaimedObservedRevision,
    cassette_identity: String,
    engine_request_id: String,
    first_identity: String,
    second_identity: String,
    evidence_id: String,
    legacy_receipt: Vec<u8>,
    body_digest: String,
    body_bytes: Vec<u8>,
}

fn build_v2_receipt(
    manifest: &crate::protocol::ProtocolManifest,
    matched: &crate::protocol::RevisionMatched<'_>,
    cassette: &CassetteV2,
    campaign: &SandboxCampaign,
    legacy_receipt: &[u8],
) -> Result<Vec<u8>, String> {
    let request = matched.request();
    let fresh = matched.fresh();
    let claimed = fresh.digest();
    let engine_request_id = crate::protocol::engine_request_id(request, &cassette.identity());
    let evidence_id = crate::protocol::evidence_id(
        &engine_request_id,
        claimed,
        &campaign.first.identity(),
        &campaign.second.identity(),
    );
    let mut body = Vec::new();
    plain_line(&mut body, crate::protocol::RECEIPT_SCHEMA);
    frame_field(
        &mut body,
        "protocol-schema",
        crate::protocol::MANIFEST_SCHEMA.as_bytes(),
    );
    frame_field(&mut body, "manifest-id", manifest.manifest_id.as_bytes());
    frame_field(
        &mut body,
        "engine-sha256",
        manifest.engine_sha256.as_bytes(),
    );
    frame_field(
        &mut body,
        "operation",
        crate::protocol::OPERATION.as_bytes(),
    );
    frame_field(
        &mut body,
        "request-schema",
        crate::protocol::REQUEST_SCHEMA.as_bytes(),
    );
    frame_field(
        &mut body,
        "outcome-schema",
        crate::protocol::OUTCOME_SCHEMA.as_bytes(),
    );
    frame_field(
        &mut body,
        "receipt-schema",
        crate::protocol::RECEIPT_SCHEMA.as_bytes(),
    );
    frame_field(
        &mut body,
        "verifier-schema",
        crate::protocol::VERIFY_SCHEMA.as_bytes(),
    );
    plain_line(&mut body, &format!("features {}", request.features().len()));
    for feature in request.features() {
        frame_field(&mut body, "feature", feature.as_bytes());
    }
    frame_field(
        &mut body,
        "observation-subject",
        crate::protocol::OBSERVATION_SUBJECT.as_bytes(),
    );
    frame_field(
        &mut body,
        "revision-algorithm",
        crate::protocol::REVISION_ALGORITHM.as_bytes(),
    );
    frame_field(
        &mut body,
        "revision-policy",
        crate::protocol::REVISION_POLICY.as_bytes(),
    );
    frame_field(
        &mut body,
        "execution-binding",
        crate::protocol::EXECUTION_BINDING.as_bytes(),
    );
    frame_field(
        &mut body,
        "observation-to-exec-channel",
        crate::protocol::OBSERVATION_TO_EXEC_CHANNEL.as_bytes(),
    );
    frame_field(
        &mut body,
        "requested-target-revision",
        request.requested_revision().wire_value().as_bytes(),
    );
    frame_field(&mut body, "claimed-observed-revision", claimed.as_bytes());
    frame_field(
        &mut body,
        "cassette-identity",
        cassette.identity().as_bytes(),
    );
    frame_field(&mut body, "engine-request-id", engine_request_id.as_bytes());
    frame_field(
        &mut body,
        "first-identity",
        campaign.first.identity().as_bytes(),
    );
    frame_field(
        &mut body,
        "second-identity",
        campaign.second.identity().as_bytes(),
    );
    frame_field(&mut body, "evidence-id", evidence_id.as_bytes());
    frame_field(&mut body, "legacy-receipt", legacy_receipt);
    let digest = vh_digest::sha256_hex(&body);
    plain_line(&mut body, &format!("digest sha256:{digest}"));
    if body.len() as u64 > MAX_RECEIPT_BYTES {
        return Err("v2 receipt exceeds the published maximum receipt size".into());
    }
    Ok(body)
}

fn parse_v2_receipt(bytes: &[u8]) -> Result<ParsedV2Receipt, &'static str> {
    if bytes.len() as u64 > MAX_RECEIPT_BYTES {
        return Err("oversized");
    }
    let mut reader = ReceiptReader { bytes, pos: 0 };
    reader.expect_exact(crate::protocol::RECEIPT_SCHEMA)?;
    let framed_string = |reader: &mut ReceiptReader<'_>, tag: &str| {
        String::from_utf8(reader.expect_framed(tag)?).map_err(|_| "invalid-utf8")
    };
    let protocol_schema = framed_string(&mut reader, "protocol-schema")?;
    let manifest_id = framed_string(&mut reader, "manifest-id")?;
    let engine_sha256 = framed_string(&mut reader, "engine-sha256")?;
    let operation = framed_string(&mut reader, "operation")?;
    let request_schema = framed_string(&mut reader, "request-schema")?;
    let outcome_schema = framed_string(&mut reader, "outcome-schema")?;
    let receipt_schema = framed_string(&mut reader, "receipt-schema")?;
    let verifier_schema = framed_string(&mut reader, "verifier-schema")?;
    let count = reader.expect_u64("features")?;
    if count > crate::protocol::MAX_FEATURES as u64 {
        return Err("field-bound");
    }
    let count = usize::try_from(count).map_err(|_| "field-bound")?;
    let mut features = Vec::with_capacity(count);
    for _ in 0..count {
        features.push(framed_string(&mut reader, "feature")?);
    }
    let observation_subject = framed_string(&mut reader, "observation-subject")?;
    let revision_algorithm = framed_string(&mut reader, "revision-algorithm")?;
    let revision_policy = framed_string(&mut reader, "revision-policy")?;
    let execution_binding = framed_string(&mut reader, "execution-binding")?;
    let observation_to_exec_channel = framed_string(&mut reader, "observation-to-exec-channel")?;
    let requested_revision = framed_string(&mut reader, "requested-target-revision")?;
    let claimed_revision_text = framed_string(&mut reader, "claimed-observed-revision")?;
    let claimed_revision = crate::protocol::ClaimedObservedRevision::parse(&claimed_revision_text)
        .map_err(|_| "field-type")?;
    let cassette_identity = framed_string(&mut reader, "cassette-identity")?;
    let engine_request_id = framed_string(&mut reader, "engine-request-id")?;
    let first_identity = framed_string(&mut reader, "first-identity")?;
    let second_identity = framed_string(&mut reader, "second-identity")?;
    let evidence_id = framed_string(&mut reader, "evidence-id")?;
    let legacy_receipt = reader.expect_framed("legacy-receipt")?;
    let body_end = reader.pos;
    let digest_line = reader.take_line()?;
    let body_digest = digest_line
        .strip_prefix("digest sha256:")
        .ok_or("field-order")?
        .to_string();
    if reader.pos != bytes.len() || body_digest.len() != 64 {
        return Err("trailing-data");
    }
    Ok(ParsedV2Receipt {
        protocol_schema,
        manifest_id,
        engine_sha256,
        operation,
        request_schema,
        outcome_schema,
        receipt_schema,
        verifier_schema,
        features,
        observation_subject,
        revision_algorithm,
        revision_policy,
        execution_binding,
        observation_to_exec_channel,
        requested_revision,
        claimed_revision,
        cassette_identity,
        engine_request_id,
        first_identity,
        second_identity,
        evidence_id,
        legacy_receipt,
        body_digest,
        body_bytes: bytes[..body_end].to_vec(),
    })
}

#[derive(Clone)]
struct ExpectedV2Request {
    negotiated: crate::protocol::NegotiatedRequest,
    cassette_bytes: Vec<u8>,
}

fn verify_v2_receipt(
    bytes: &[u8],
    expected: &ExpectedV2Request,
    executions: &mut u64,
    output_schema: &str,
) -> Result<(i32, Vec<u8>), String> {
    let parsed =
        parse_v2_receipt(bytes).map_err(|category| format!("malformed v2 receipt: {category}"))?;
    let manifest = crate::protocol::ProtocolManifest::current()?;
    let mut errors = Vec::new();
    if vh_digest::sha256_hex(&parsed.body_bytes) != parsed.body_digest {
        errors.push("body-digest-mismatch");
    }
    if parsed.protocol_schema != crate::protocol::MANIFEST_SCHEMA
        || parsed.manifest_id != manifest.manifest_id
        || parsed.engine_sha256 != manifest.engine_sha256
        || parsed.operation != crate::protocol::OPERATION
        || parsed.request_schema != crate::protocol::REQUEST_SCHEMA
        || parsed.outcome_schema != crate::protocol::OUTCOME_SCHEMA
        || parsed.receipt_schema != crate::protocol::RECEIPT_SCHEMA
        || parsed.verifier_schema != crate::protocol::VERIFY_SCHEMA
        || parsed.observation_subject != crate::protocol::OBSERVATION_SUBJECT
        || parsed.revision_algorithm != crate::protocol::REVISION_ALGORITHM
        || parsed.revision_policy != crate::protocol::REVISION_POLICY
        || parsed.execution_binding != crate::protocol::EXECUTION_BINDING
        || parsed.observation_to_exec_channel != crate::protocol::OBSERVATION_TO_EXEC_CHANNEL
    {
        errors.push("descriptor-mismatch");
    }
    if parsed.manifest_id != expected.negotiated.manifest_id()
        || parsed.features != expected.negotiated.features()
        || parsed.requested_revision != expected.negotiated.requested_revision().wire_value()
    {
        errors.push("expected-request-mismatch");
    }
    let cassette = CassetteV2::parse_detailed(&expected.cassette_bytes)
        .map_err(|_| "expected cassette is malformed".to_string())?;
    if cassette.file_bytes() != expected.cassette_bytes
        || parsed.cassette_identity != cassette.identity()
    {
        errors.push("expected-cassette-mismatch");
    }
    let legacy = parse_receipt(&parsed.legacy_receipt)
        .map_err(|category| format!("embedded legacy receipt malformed: {category}"))?;
    if legacy.cassette_bytes != expected.cassette_bytes
        || legacy.cassette_identity != parsed.cassette_identity
        || legacy.first_identity != parsed.first_identity
        || legacy.second_identity != parsed.second_identity
        || legacy.engine_sha256 != parsed.engine_sha256
    {
        errors.push("embedded-receipt-mismatch");
    }
    // Do not construct a fresh or verified authority type for evidence whose
    // framing, descriptor, expected-request binding, cassette, or embedded
    // legacy receipt has already failed. Authority promotion is downstream of
    // all non-executing integrity and policy checks.
    if !errors.is_empty() {
        return Err(errors.join(","));
    }
    let receipt_requested =
        crate::protocol::RequestedTargetRevision::parse(&parsed.requested_revision)
            .map_err(|_| "receipt requested revision malformed".to_string())?;
    let negotiated = crate::protocol::negotiate(
        &manifest,
        &parsed.protocol_schema,
        &parsed.manifest_id,
        &parsed.operation,
        &parsed.features,
        receipt_requested.clone(),
    )
    .map_err(|reason| format!("receipt negotiation failed: {}", reason.as_str()))?;
    let engine_request_id = crate::protocol::engine_request_id(&negotiated, &cassette.identity());
    if engine_request_id != parsed.engine_request_id
        || engine_request_id
            != crate::protocol::engine_request_id(&expected.negotiated, &cassette.identity())
    {
        errors.push("engine-request-id-mismatch");
    }
    if !errors.is_empty() {
        return Err(errors.join(","));
    }

    let fresh = crate::protocol::resolve_fresh_compiled_target();
    if fresh.bytes() != COOPERATIVE_ECHO_CHILD.as_bytes() {
        errors.push("fresh-observation-bytes-mismatch");
    }
    if expected.negotiated.requested_revision().exact_digest() != Some(fresh.digest()) {
        errors.push("requested-revision-mismatch");
    }
    if !errors.is_empty() {
        return Err(errors.join(","));
    }
    let matched = crate::protocol::match_requested_revision(&negotiated, &fresh)
        .map_err(|_| "requested-revision-mismatch".to_string())?;
    let claimed = &parsed.claimed_revision;
    let verified = crate::protocol::VerifiedObservedRevision::promote(claimed, &matched)
        .map_err(|_| "claimed/fresh observation mismatch".to_string())?;
    let evidence_id = crate::protocol::evidence_id(
        &engine_request_id,
        claimed.digest(),
        &parsed.first_identity,
        &parsed.second_identity,
    );
    if evidence_id != parsed.evidence_id {
        errors.push("evidence-id-mismatch");
    }
    if !errors.is_empty() {
        return Err(errors.join(","));
    }

    let legacy_expected = ExpectedCooperativeRequest {
        workload: "cooperative-echo".into(),
        cassette_bytes: expected.cassette_bytes.clone(),
    };
    let (legacy_code, legacy_fields) =
        verify_cooperative_receipt(&parsed.legacy_receipt, Some(&legacy_expected), executions);
    if legacy_code != 0 || legacy_fields.is_empty() || !field_bool(&legacy_fields, "authentic") {
        return Err("embedded legacy receipt failed fresh replay".into());
    }
    let receipt_sha256 = vh_digest::sha256_hex(bytes);
    let outcome_verified = field_bool(&legacy_fields, "verified");
    let verification_result_id = crate::protocol::verification_result_id(
        &receipt_sha256,
        fresh.digest(),
        verified.digest(),
        true,
        outcome_verified,
    );
    let outcome_exit_code = field_u64(&legacy_fields, "outcome_exit_code") as i32;
    let mut record = Vec::new();
    plain_line(&mut record, output_schema);
    frame_field(
        &mut record,
        "verdict",
        field_string(&legacy_fields, "verdict").as_bytes(),
    );
    frame_field(&mut record, "tier", b"TIER2");
    frame_field(&mut record, "grade", b"D2");
    frame_field(&mut record, "scope", SCOPE.as_bytes());
    frame_field(
        &mut record,
        "protocol-schema",
        crate::protocol::MANIFEST_SCHEMA.as_bytes(),
    );
    frame_field(&mut record, "manifest-id", manifest.manifest_id.as_bytes());
    frame_field(
        &mut record,
        "engine-sha256",
        manifest.engine_sha256.as_bytes(),
    );
    frame_field(
        &mut record,
        "operation",
        crate::protocol::OPERATION.as_bytes(),
    );
    plain_line(&mut record, &format!("features {}", parsed.features.len()));
    for feature in &parsed.features {
        frame_field(&mut record, "feature", feature.as_bytes());
    }
    frame_field(
        &mut record,
        "request-schema",
        crate::protocol::REQUEST_SCHEMA.as_bytes(),
    );
    frame_field(
        &mut record,
        "outcome-schema",
        crate::protocol::OUTCOME_SCHEMA.as_bytes(),
    );
    frame_field(
        &mut record,
        "receipt-schema",
        crate::protocol::RECEIPT_SCHEMA.as_bytes(),
    );
    frame_field(
        &mut record,
        "verifier-schema",
        crate::protocol::VERIFY_SCHEMA.as_bytes(),
    );
    frame_field(
        &mut record,
        "observation-subject",
        crate::protocol::OBSERVATION_SUBJECT.as_bytes(),
    );
    frame_field(
        &mut record,
        "revision-algorithm",
        crate::protocol::REVISION_ALGORITHM.as_bytes(),
    );
    frame_field(
        &mut record,
        "revision-policy",
        crate::protocol::REVISION_POLICY.as_bytes(),
    );
    frame_field(
        &mut record,
        "requested-target-revision",
        parsed.requested_revision.as_bytes(),
    );
    frame_field(
        &mut record,
        "claimed-observed-revision",
        claimed.digest().as_bytes(),
    );
    frame_field(
        &mut record,
        "fresh-observed-revision",
        fresh.digest().as_bytes(),
    );
    frame_field(
        &mut record,
        "verified-observed-revision",
        verified.digest().as_bytes(),
    );
    frame_field(&mut record, "revision-binding", b"bound");
    frame_field(
        &mut record,
        "execution-binding",
        crate::protocol::EXECUTION_BINDING.as_bytes(),
    );
    frame_field(
        &mut record,
        "observation-to-exec-channel",
        crate::protocol::OBSERVATION_TO_EXEC_CHANNEL.as_bytes(),
    );
    frame_field(
        &mut record,
        "cassette-identity",
        parsed.cassette_identity.as_bytes(),
    );
    frame_field(
        &mut record,
        "engine-request-id",
        engine_request_id.as_bytes(),
    );
    frame_field(&mut record, "evidence-id", evidence_id.as_bytes());
    frame_field(
        &mut record,
        "result-digest",
        field_string(&legacy_fields, "result_digest").as_bytes(),
    );
    frame_field(&mut record, "receipt-sha256", receipt_sha256.as_bytes());
    frame_field(
        &mut record,
        "verification-result-id",
        verification_result_id.as_bytes(),
    );
    frame_field(&mut record, "oracle", COOPERATIVE_ORACLE.as_bytes());
    frame_field(
        &mut record,
        "oracle-evaluation",
        field_string(&legacy_fields, "oracle_evaluation").as_bytes(),
    );
    frame_field(
        &mut record,
        "finding-identity",
        field_string(&legacy_fields, "finding_identity").as_bytes(),
    );
    plain_line(
        &mut record,
        &format!(
            "findings-count {}",
            field_u64(&legacy_fields, "findings_count")
        ),
    );
    plain_line(&mut record, "authentic true");
    plain_line(&mut record, &format!("verified {outcome_verified}"));
    plain_line(
        &mut record,
        &format!("outcome-exit-code {outcome_exit_code}"),
    );
    plain_line(&mut record, "exit-code 0");
    plain_line(&mut record, &format!("executions {}", *executions));
    frame_field(
        &mut record,
        "errors",
        field_string(&legacy_fields, "errors").as_bytes(),
    );
    Ok((outcome_exit_code, record))
}

pub(crate) fn cmd_cooperative_v2(args: &[String], usage: &str) -> i32 {
    let stdout = std::io::stdout();
    let mut output = stdout.lock();
    cmd_cooperative_v2_inner(
        args,
        usage,
        &mut output,
        #[cfg(test)]
        None,
    )
}

fn cmd_cooperative_v2_inner<W: Write>(
    args: &[String],
    usage: &str,
    output: &mut W,
    #[cfg(test)] before_observation: BeforeObservationHook<'_>,
) -> i32 {
    let request = match parse_cooperative_v2_args(args) {
        Ok(request) => request,
        Err(error) => {
            eprintln!("error: {}\n\n{usage}", bounded_diagnostic(&error));
            return 2;
        }
    };
    let manifest = match crate::protocol::ProtocolManifest::current() {
        Ok(manifest) => manifest,
        Err(error) => {
            eprintln!("error: {}", bounded_diagnostic(&error));
            return 2;
        }
    };
    let negotiated = match crate::protocol::negotiate(
        &manifest,
        &request.protocol_schema,
        &request.manifest_id,
        &request.operation,
        &request.features,
        request.requested_revision.clone(),
    ) {
        Ok(negotiated) => negotiated,
        Err(reason) => return emit_v2_refusal_to(reason, &manifest, output),
    };
    let out_dir = match request.out_path.as_deref() {
        Some(path) => match prepare_output_root(Path::new(path)) {
            Ok(path) => Some(path),
            Err(error) => {
                eprintln!("error: {error}");
                return 2;
            }
        },
        None => None,
    };
    let cassette = match load_cooperative_cassette(request.cassette_path.as_deref()) {
        Ok(cassette) => cassette,
        Err(error) => {
            eprintln!("error: {}", bounded_diagnostic(&error));
            return 2;
        }
    };
    let mut executions = 0;
    let (campaign, workspace, fresh) = match run_cooperative_campaign_v2_inner(
        &cassette,
        out_dir.as_deref(),
        negotiated.requested_revision(),
        &mut executions,
        #[cfg(test)]
        before_observation,
    ) {
        Ok(value) => value,
        Err(CooperativeV2RunError::Refusal(reason)) => {
            return emit_v2_refusal_to(reason, &manifest, output)
        }
        Err(CooperativeV2RunError::Boundary(error)) => {
            eprintln!(
                "error: cooperative v2 run failed: {}",
                bounded_diagnostic(&error)
            );
            return 2;
        }
    };
    let (_, provisional_fields) = outcome_fields(&campaign, &cassette);
    let matched = match crate::protocol::match_requested_revision(&negotiated, &fresh) {
        Ok(matched) => matched,
        Err(reason) => return emit_v2_refusal_to(reason, &manifest, output),
    };
    let legacy_receipt = match build_cooperative_receipt(
        "cooperative-echo",
        &cassette,
        &campaign,
        workspace.path(),
        &provisional_fields,
    ) {
        Ok(bytes) => bytes,
        Err(error) => {
            eprintln!("error: {}", bounded_diagnostic(&error));
            return 2;
        }
    };
    let v2_receipt =
        match build_v2_receipt(&manifest, &matched, &cassette, &campaign, &legacy_receipt) {
            Ok(bytes) => bytes,
            Err(error) => {
                eprintln!("error: {}", bounded_diagnostic(&error));
                return 2;
            }
        };
    let ephemeral = if out_dir.is_none() {
        match reserve_workspace(None, "v2-receipt", &cassette) {
            Ok(lease) => Some(lease),
            Err(error) => {
                eprintln!("error: {}", bounded_diagnostic(&error));
                return 2;
            }
        }
    } else {
        None
    };
    let receipt_dir = out_dir
        .as_deref()
        .or_else(|| ephemeral.as_ref().map(WorkspaceLease::path))
        .expect("receipt directory");
    if let Err(error) = publish_cooperative_receipt(receipt_dir, &v2_receipt) {
        eprintln!("error: {}", bounded_diagnostic(&error));
        return 2;
    }
    let receipt_path = receipt_dir.join(COOPERATIVE_RECEIPT_NAME);
    let observed_receipt = match vh_sandbox::read_bounded_file(&receipt_path, MAX_RECEIPT_BYTES) {
        Ok(bytes) if bytes == v2_receipt => bytes,
        Ok(_) => {
            eprintln!("error: persisted cooperative v2 receipt changed before reverification");
            return 2;
        }
        Err(error) => {
            eprintln!(
                "error: cannot read persisted cooperative v2 receipt: category={}",
                error.category()
            );
            return 2;
        }
    };
    let expected = ExpectedV2Request {
        negotiated,
        cassette_bytes: cassette.file_bytes(),
    };
    match verify_v2_receipt(
        &observed_receipt,
        &expected,
        &mut executions,
        crate::protocol::OUTCOME_SCHEMA,
    ) {
        Ok((outcome_code, record)) => match output.write_all(&record) {
            Ok(()) => outcome_code,
            Err(_) => 2,
        },
        Err(error) => {
            eprintln!(
                "error: v2 reverification failed: {}",
                bounded_diagnostic(&error)
            );
            2
        }
    }
}

pub(crate) fn cmd_verify_cooperative_v2(args: &[String], usage: &str) -> i32 {
    let mut receipt_path = None;
    let mut operation = None;
    let mut features = Vec::new();
    let mut requested_revision = None;
    let mut protocol_schema = None;
    let mut manifest_id = None;
    let mut request_schema = None;
    let mut outcome_schema = None;
    let mut receipt_schema = None;
    let mut verifier_schema = None;
    let mut observation_subject = None;
    let mut revision_algorithm = None;
    let mut revision_policy = None;
    let mut execution_binding = None;
    let mut observation_to_exec_channel = None;
    let mut expected_cassette_path = None;
    let mut expect_default_cassette = false;
    let mut it = args.iter();
    while let Some(argument) = it.next() {
        let mut value = |flag: &str| {
            it.next()
                .cloned()
                .ok_or_else(|| format!("{flag} requires a value"))
        };
        let parsed: Result<(), String> = match argument.as_str() {
            "--receipt" if receipt_path.is_none() => {
                value(argument).map(|v| receipt_path = Some(v))
            }
            "--expected-operation" if operation.is_none() => {
                value(argument).map(|v| operation = Some(v))
            }
            "--expected-feature" => value(argument).map(|v| features.push(v)),
            "--expected-requested-target-revision" if requested_revision.is_none() => {
                value(argument).map(|v| requested_revision = Some(v))
            }
            "--expected-protocol-schema" if protocol_schema.is_none() => {
                value(argument).map(|v| protocol_schema = Some(v))
            }
            "--expected-manifest-id" if manifest_id.is_none() => {
                value(argument).map(|v| manifest_id = Some(v))
            }
            "--expected-cassette"
                if expected_cassette_path.is_none() && !expect_default_cassette =>
            {
                value(argument).map(|v| expected_cassette_path = Some(v))
            }
            "--expect-default-cassette"
                if expected_cassette_path.is_none() && !expect_default_cassette =>
            {
                expect_default_cassette = true;
                Ok(())
            }
            "--expected-request-schema" if request_schema.is_none() => {
                value(argument).map(|v| request_schema = Some(v))
            }
            "--expected-outcome-schema" if outcome_schema.is_none() => {
                value(argument).map(|v| outcome_schema = Some(v))
            }
            "--expected-receipt-schema" if receipt_schema.is_none() => {
                value(argument).map(|v| receipt_schema = Some(v))
            }
            "--expected-verifier-schema" if verifier_schema.is_none() => {
                value(argument).map(|v| verifier_schema = Some(v))
            }
            "--expected-observation-subject" if observation_subject.is_none() => {
                value(argument).map(|v| observation_subject = Some(v))
            }
            "--expected-revision-algorithm" if revision_algorithm.is_none() => {
                value(argument).map(|v| revision_algorithm = Some(v))
            }
            "--expected-revision-policy" if revision_policy.is_none() => {
                value(argument).map(|v| revision_policy = Some(v))
            }
            "--expected-execution-binding" if execution_binding.is_none() => {
                value(argument).map(|v| execution_binding = Some(v))
            }
            "--expected-observation-to-exec-channel" if observation_to_exec_channel.is_none() => {
                value(argument).map(|v| observation_to_exec_channel = Some(v))
            }
            other => Err(format!("duplicate or unknown argument: {other}")),
        };
        if let Err(error) = parsed {
            eprintln!("error: {}\n\n{usage}", bounded_diagnostic(&error));
            return 2;
        }
    }
    let receipt_path = match receipt_path {
        Some(value) => PathBuf::from(value),
        None => {
            eprintln!("error: verify-cooperative-v2 requires --receipt PATH\n\n{usage}");
            return 2;
        }
    };
    if !receipt_path.is_absolute()
        || receipt_path.file_name().and_then(|name| name.to_str()) != Some(COOPERATIVE_RECEIPT_NAME)
        || receipt_path
            .components()
            .any(|part| matches!(part, Component::ParentDir | Component::CurDir))
    {
        eprintln!("error: receipt must be an absolute canonical cooperative.receipt path");
        return 2;
    }
    let manifest = match crate::protocol::ProtocolManifest::current() {
        Ok(manifest) => manifest,
        Err(error) => {
            eprintln!("error: {}", bounded_diagnostic(&error));
            return 2;
        }
    };
    let bytes = match vh_sandbox::read_bounded_file(&receipt_path, MAX_RECEIPT_BYTES) {
        Ok(bytes) => bytes,
        Err(SandboxError::Oversized { .. }) => {
            let failure = encode_v2_verify_failure(
                V2VerifyFailureReason::MalformedReceipt,
                &manifest,
                "unavailable",
                0,
            );
            print!("{}", String::from_utf8_lossy(&failure));
            return 1;
        }
        Err(error) => {
            eprintln!("error: cannot read receipt: category={}", error.category());
            return 2;
        }
    };
    if !bytes.starts_with(format!("{}\n", crate::protocol::RECEIPT_SCHEMA).as_bytes()) {
        return emit_v2_refusal(
            crate::protocol::RefusalReason::UnsupportedReceiptSchema,
            &manifest,
        );
    }
    for (flag, actual, required) in [
        (
            "--expected-request-schema",
            request_schema.as_deref(),
            crate::protocol::REQUEST_SCHEMA,
        ),
        (
            "--expected-outcome-schema",
            outcome_schema.as_deref(),
            crate::protocol::OUTCOME_SCHEMA,
        ),
        (
            "--expected-receipt-schema",
            receipt_schema.as_deref(),
            crate::protocol::RECEIPT_SCHEMA,
        ),
        (
            "--expected-verifier-schema",
            verifier_schema.as_deref(),
            crate::protocol::VERIFY_SCHEMA,
        ),
        (
            "--expected-observation-subject",
            observation_subject.as_deref(),
            crate::protocol::OBSERVATION_SUBJECT,
        ),
        (
            "--expected-revision-algorithm",
            revision_algorithm.as_deref(),
            crate::protocol::REVISION_ALGORITHM,
        ),
        (
            "--expected-revision-policy",
            revision_policy.as_deref(),
            crate::protocol::REVISION_POLICY,
        ),
        (
            "--expected-execution-binding",
            execution_binding.as_deref(),
            crate::protocol::EXECUTION_BINDING,
        ),
        (
            "--expected-observation-to-exec-channel",
            observation_to_exec_channel.as_deref(),
            crate::protocol::OBSERVATION_TO_EXEC_CHANNEL,
        ),
    ] {
        match actual {
            Some(value) if value == required => {}
            Some(_) => {
                eprintln!("error: {flag} does not match the Rust-owned descriptor");
                return 2;
            }
            None => {
                eprintln!("error: v2 expected request is missing {flag}");
                return 2;
            }
        }
    }
    let requested_revision = match requested_revision
        .as_deref()
        .and_then(|value| crate::protocol::RequestedTargetRevision::parse(value).ok())
    {
        Some(value) => value,
        None => {
            eprintln!("error: v2 expected request is incomplete");
            return 2;
        }
    };
    let negotiated = match crate::protocol::negotiate(
        &manifest,
        protocol_schema.as_deref().unwrap_or(""),
        manifest_id.as_deref().unwrap_or(""),
        operation.as_deref().unwrap_or(""),
        &features,
        requested_revision,
    ) {
        Ok(value) => value,
        Err(reason) => return emit_v2_refusal(reason, &manifest),
    };
    let cassette_bytes = if expect_default_cassette {
        fixture_cassette().file_bytes()
    } else if let Some(path) = expected_cassette_path.as_deref() {
        match load_cooperative_cassette(Some(path)) {
            Ok(cassette) => cassette.file_bytes(),
            Err(error) => {
                eprintln!("error: {}", bounded_diagnostic(&error));
                return 2;
            }
        }
    } else {
        eprintln!("error: v2 expected cassette mode is required");
        return 2;
    };
    let expected = ExpectedV2Request {
        negotiated,
        cassette_bytes,
    };
    let mut executions = 0;
    match verify_v2_receipt(
        &bytes,
        &expected,
        &mut executions,
        crate::protocol::VERIFY_SCHEMA,
    ) {
        Ok((_outcome_code, record)) => {
            print!("{}", String::from_utf8_lossy(&record));
            0
        }
        Err(error) => {
            let failure = encode_v2_verify_failure(
                classify_v2_verify_failure(&error, executions),
                &manifest,
                &vh_digest::sha256_hex(&bytes),
                executions,
            );
            print!("{}", String::from_utf8_lossy(&failure));
            1
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    /// Test-only staging base: each test removes and recreates its own
    /// labelled directory. The production campaign path never uses this
    /// — it reserves a fresh per-invocation workspace instead.
    fn cooperative_root(label: &str, cassette: &CassetteV2) -> PathBuf {
        let prefix = cassette_root_prefix(cassette);
        std::env::temp_dir().join(format!(
            "vh-cooperative-test-{}-{label}-{prefix}",
            std::process::id()
        ))
    }

    fn make_cassette(extra_entry: bool) -> CassetteV2 {
        let mut cassette = CassetteV2::default();
        cassette.push(
            fixture_request(),
            TapeEntry::Success {
                status: 200,
                body: b"first-of-one\n".to_vec(),
            },
        );
        if extra_entry {
            cassette.push(
                fixture_request(),
                TapeEntry::Success {
                    status: 200,
                    body: b"extra-unconsumed\n".to_vec(),
                },
            );
        }
        cassette
    }

    fn run_one(label: &str, cassette: &CassetteV2) -> vh_sandbox::RunRecord {
        let root = cooperative_root(label, cassette);
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&root).unwrap();
        let spec = child_spec(cassette).unwrap();
        let workspace = root.join("u0");
        write_child_source(&workspace).unwrap();
        run_once_with_cassette(&spec, &workspace, cassette).unwrap()
    }

    #[test]
    fn cooperative_echo_clean_run_twice() {
        let _cassette = fixture_cassette();
        let mut executions = 0u64;
        let (campaign, _workspace) =
            run_cooperative_campaign(None, "echo-clean", None, &mut executions).unwrap();
        assert_eq!(executions, 2);
        assert!(!campaign.first.transport_tainted());
        assert!(!campaign.second.transport_tainted());
        assert_eq!(campaign.first.identity(), campaign.second.identity());
        assert!(matches!(
            campaign.first.termination,
            vh_sandbox::TerminationOutcome::Exited(0)
        ));
    }

    #[test]
    fn cooperative_cassette_miss_taints_unchecked() {
        let empty = CassetteV2::default();
        let record = run_one("miss", &empty);
        assert!(record.transport_tainted());
        assert!(record
            .transport
            .as_ref()
            .unwrap()
            .taint
            .as_ref()
            .unwrap()
            .contains("beyond the recorded tape"));
    }

    #[test]
    fn cooperative_unconsumed_history_taints_unchecked() {
        let cassette = make_cassette(true);
        let record = run_one("unconsumed", &cassette);
        assert!(record.transport_tainted());
        assert_eq!(record.transport.as_ref().unwrap().unconsumed, 1);
    }

    #[test]
    fn admitted_attempt_cannot_become_false_zero_after_child_start() {
        let cassette = CassetteV2::default();
        let root = cooperative_root("attempt-post-spawn-artifact", &cassette);
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&root).unwrap();
        let source = root.join("attempt.py");
        std::fs::write(
            &source,
            "from pathlib import Path\nPath('started.txt').write_text('started')\nout = Path('out.txt')\nif out.exists(): out.unlink()\nout.mkdir()\n",
        )
        .unwrap();
        let spec = SandboxSpec::new(vec![
            "/usr/bin/python3".into(),
            "-S".into(),
            "-s".into(),
            "attempt.py".into(),
        ])
        .unwrap()
        .with_cassette_identity(cassette.identity())
        .declare_artifact("out.txt")
        .unwrap()
        .declare_input_file(&source)
        .unwrap();
        let workspace = root.join("u0");
        std::fs::create_dir_all(&workspace).unwrap();
        std::fs::copy(&source, workspace.join("attempt.py")).unwrap();
        let mut attempts = 0;

        let error = run_admitted_sandbox_attempt(&spec, &workspace, &cassette, &mut attempts)
            .expect_err("a directory cannot satisfy the declared file artifact");

        assert_eq!(
            std::fs::read(workspace.join("started.txt")).unwrap(),
            b"started"
        );
        assert!(matches!(error, SandboxError::ArtifactBoundary { .. }));
        assert_eq!(attempts, 1, "post-spawn failure must not report zero");
        assert_eq!(
            classify_v2_verify_failure("post-spawn artifact boundary", attempts),
            V2VerifyFailureReason::FreshReplayFailed
        );
        let manifest = crate::protocol::ProtocolManifest::current().unwrap();
        let failure = String::from_utf8(encode_v2_verify_failure(
            V2VerifyFailureReason::FreshReplayFailed,
            &manifest,
            &"0".repeat(64),
            attempts,
        ))
        .unwrap();
        assert!(failure.contains("reason 19:fresh-replay-failed\n"));
        assert!(failure.contains("executions 1\n"));
        assert!(failure.contains("authentic false\n"));
        assert!(failure.contains("verified false\n"));
        assert!(!failure.contains("CLEAN"));
    }

    #[test]
    fn cooperative_duplicate_requests_are_distinct_ordered_entries() {
        let mut cassette = CassetteV2::default();
        let req = fixture_request();
        cassette.push(
            req.clone(),
            TapeEntry::Success {
                status: 200,
                body: b"reply-alpha\n".to_vec(),
            },
        );
        cassette.push(
            req,
            TapeEntry::Success {
                status: 200,
                body: b"reply-beta\n".to_vec(),
            },
        );

        let child = "import os, sys, time\nMAILBOX = os.path.join('.vh-sandbox-io', 'llm')\nCALL = 10.0\ndef field(tag, value):\n    return tag.encode() + b' ' + str(len(value)).encode() + b':' + value + b'\\n'\ndef make_request():\n    out = b'vh-llm-request-v2\\n'\n    out += field('provider', b'fixture')\n    out += field('model', b'cooperative-echo')\n    out += b'messages 1\\n'\n    out += field('role', b'user')\n    out += field('content', b'hello')\n    out += b'tools 0\\n'\n    out += b'tool-choice absent\\n'\n    out += b'structured-output absent\\n'\n    out += b'params 1\\n'\n    out += field('param-key', b'temperature')\n    out += field('param-value', b'0')\n    return out\ndef write_frame(path, data):\n    tmp = path + '.tmp'\n    with open(tmp, 'wb') as f: f.write(data)\n    os.replace(tmp, path)\ndef read_body(path):\n    start = time.monotonic()\n    while not os.path.exists(path):\n        if time.monotonic() - start > CALL: sys.exit(41)\n        time.sleep(0.002)\n    with open(path, 'rb') as f: data = f.read()\n    nl = data.index(b'\\n')\n    head = data[:nl].decode()\n    pos = nl + 1\n    tag = b'body '\n    if not data[pos:pos + len(tag)] == tag: sys.exit(43)\n    pos += len(tag)\n    colon = data.index(b':', pos)\n    ln = int(data[pos:colon])\n    pos = colon + 1\n    return data[pos:pos + ln]\nreq = make_request()\nfor i in range(2):\n    write_frame(os.path.join(MAILBOX, 'req-%d' % i), req)\n    body = read_body(os.path.join(MAILBOX, 'resp-%d' % i))\n    with open('out.txt', 'ab' if i else 'wb') as f: f.write(body)\n";
        let root = cooperative_root("dup", &cassette);
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&root).unwrap();
        let source = root.join("dup.py");
        std::fs::write(&source, child).unwrap();
        let spec = SandboxSpec::new(vec!["/usr/bin/python3".into(), "dup.py".into()])
            .unwrap()
            .with_cassette_identity(cassette.identity())
            .declare_artifact("out.txt")
            .unwrap()
            .declare_input_file(&source)
            .unwrap();
        let a = root.join("a");
        let b = root.join("b");
        for ws in [&a, &b] {
            std::fs::create_dir_all(ws).unwrap();
            std::fs::copy(&source, ws.join("dup.py")).unwrap();
        }
        let campaign = SandboxCampaign {
            first: run_once_with_cassette(&spec, &a, &cassette).unwrap(),
            second: run_once_with_cassette(&spec, &b, &cassette).unwrap(),
        };
        assert!(!campaign.first.transport_tainted());
        assert_eq!(campaign.first.identity(), campaign.second.identity());
        let out_a = std::fs::read(a.join("out.txt")).unwrap();
        let out_b = std::fs::read(b.join("out.txt")).unwrap();
        assert_eq!(out_a, out_b);
        assert_eq!(out_a, b"reply-alpha\nreply-beta\n");
    }

    #[test]
    fn cooperative_extra_request_taints_unchecked() {
        let cassette = make_cassette(false);
        let child = r#"
import os, sys, time
MAILBOX = os.path.join('.vh-sandbox-io', 'llm')
CALL_DEADLINE = 10.0

def field(tag, value):
    return tag.encode() + b' ' + str(len(value)).encode() + b':' + value + b'\n'

def make_request(provider, model, messages, params=()):
    out = b'vh-llm-request-v2\n'
    out += field('provider', provider.encode())
    out += field('model', model.encode())
    out += ('messages %d\n' % len(messages)).encode()
    for role, content in messages:
        out += field('role', role.encode())
        out += field('content', content.encode())
    out += b'tools 0\n'
    out += b'tool-choice absent\n'
    out += b'structured-output absent\n'
    items = sorted(dict(params).items())
    out += ('params %d\n' % len(items)).encode()
    for k, v in items:
        out += field('param-key', k.encode())
        out += field('param-value', v.encode())
    return out

def write_frame(path, data):
    tmp = path + '.tmp'
    with open(tmp, 'wb') as f:
        f.write(data)
    os.replace(tmp, path)

def read_frame(path):
    start = time.monotonic()
    while not os.path.exists(path):
        if time.monotonic() - start > CALL_DEADLINE:
            sys.exit(41)
        time.sleep(0.002)
    with open(path, 'rb') as f:
        return f.read()

def read_body(data):
    nl = data.index(b'\n')
    pos = data.index(b'body ', nl + 1)
    pos += len(b'body ')
    colon = data.index(b':', pos)
    ln = int(data[pos:colon])
    pos = colon + 1
    return data[pos:pos + ln]

req = make_request('fixture', 'cooperative-echo', [('user', 'hello')], [('temperature', '0')])
write_frame(os.path.join(MAILBOX, 'req-0'), req)
read_frame(os.path.join(MAILBOX, 'resp-0'))
write_frame(os.path.join(MAILBOX, 'req-1'), req)
read_frame(os.path.join(MAILBOX, 'resp-1'))
"#;
        let root = cooperative_root("extra", &cassette);
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&root).unwrap();
        let source = root.join("extra.py");
        std::fs::write(&source, child).unwrap();
        let spec = SandboxSpec::new(vec!["/usr/bin/python3".into(), "extra.py".into()])
            .unwrap()
            .with_cassette_identity(cassette.identity())
            .declare_artifact("out.txt")
            .unwrap()
            .declare_input_file(&source)
            .unwrap();
        let workspace = root.join("u0");
        std::fs::create_dir_all(&workspace).unwrap();
        std::fs::copy(&source, workspace.join("extra.py")).unwrap();
        std::fs::write(workspace.join("out.txt"), b"").unwrap();
        let record = run_once_with_cassette(&spec, &workspace, &cassette).unwrap();
        assert!(record.transport_tainted());
        assert!(record
            .transport
            .as_ref()
            .unwrap()
            .taint
            .as_ref()
            .unwrap()
            .contains("beyond the recorded tape"));
    }

    #[test]
    fn cooperative_malformed_frame_taints_unchecked() {
        let cassette = make_cassette(false);
        let child = "import os\nMAILBOX = os.path.join('.vh-sandbox-io', 'llm')\nos.makedirs(MAILBOX, exist_ok=True)\nwith open(os.path.join(MAILBOX, 'req-0'), 'wb') as f:\n    f.write(b'bad frame')\nimport time\ntime.sleep(0.5)\n";
        let root = cooperative_root("malformed", &cassette);
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&root).unwrap();
        let source = root.join("malformed.py");
        std::fs::write(&source, child).unwrap();
        let spec = SandboxSpec::new(vec!["/usr/bin/python3".into(), "malformed.py".into()])
            .unwrap()
            .with_cassette_identity(cassette.identity())
            .declare_artifact("out.txt")
            .unwrap()
            .declare_input_file(&source)
            .unwrap();
        let workspace = root.join("u0");
        std::fs::create_dir_all(&workspace).unwrap();
        std::fs::copy(&source, workspace.join("malformed.py")).unwrap();
        std::fs::write(workspace.join("out.txt"), b"").unwrap();
        let record = run_once_with_cassette(&spec, &workspace, &cassette).unwrap();
        assert!(record.transport_tainted());
        assert!(record
            .transport
            .as_ref()
            .unwrap()
            .taint
            .as_ref()
            .unwrap()
            .contains("malformed"));
    }

    #[test]
    fn cooperative_timeout_with_unconsumed_tape_taints_unchecked() {
        let cassette = make_cassette(false);
        let child = "import time\ntime.sleep(2.0)\n";
        let root = cooperative_root("timeout", &cassette);
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&root).unwrap();
        let source = root.join("timeout.py");
        std::fs::write(&source, child).unwrap();
        let spec = SandboxSpec::new(vec!["/usr/bin/python3".into(), "timeout.py".into()])
            .unwrap()
            .with_cassette_identity(cassette.identity())
            .with_budget(vh_sandbox::SandboxBudget::new(Duration::from_secs(1), 1 << 20).unwrap())
            .declare_artifact("out.txt")
            .unwrap()
            .declare_input_file(&source)
            .unwrap();
        let workspace = root.join("u0");
        std::fs::create_dir_all(&workspace).unwrap();
        std::fs::copy(&source, workspace.join("timeout.py")).unwrap();
        let record = run_once_with_cassette(&spec, &workspace, &cassette).unwrap();
        assert!(record.transport_tainted());
        assert_eq!(record.transport.as_ref().unwrap().unconsumed, 1);
        assert!(matches!(
            record.termination,
            vh_sandbox::TerminationOutcome::TimedOut
        ));
    }

    /// Item 9: a malformed child request frame carrying attacker content
    /// must produce a stable bounded taint category — the sentinel must
    /// not appear in the taint, nor in the rendered outcome receipt line.
    #[test]
    fn broker_taint_and_receipt_redact_attacker_content() {
        let cassette = make_cassette(false);
        let sentinel = "S3CR3T-SENTINEL-PR57";
        let child = format!(
            "import os\nMAILBOX = os.path.join('.vh-sandbox-io', 'llm')\n\
             with open(os.path.join(MAILBOX, 'req-0'), 'wb') as f:\n    f.write(b'bad {sentinel} frame')\n\
             import time\ntime.sleep(0.5)\n"
        );
        let root = cooperative_root("redact", &cassette);
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&root).unwrap();
        let source = root.join("redact.py");
        std::fs::write(&source, child).unwrap();
        let spec = SandboxSpec::new(vec!["/usr/bin/python3".into(), "redact.py".into()])
            .unwrap()
            .with_cassette_identity(cassette.identity())
            .declare_artifact("out.txt")
            .unwrap()
            .declare_input_file(&source)
            .unwrap();
        let workspace = root.join("u0");
        std::fs::create_dir_all(&workspace).unwrap();
        std::fs::copy(&source, workspace.join("redact.py")).unwrap();
        std::fs::write(workspace.join("out.txt"), b"").unwrap();
        let record = run_once_with_cassette(&spec, &workspace, &cassette).unwrap();
        assert!(record.transport_tainted());
        let taint = record
            .transport
            .as_ref()
            .unwrap()
            .taint
            .as_ref()
            .unwrap()
            .clone();
        assert!(
            !taint.contains(sentinel),
            "attacker content leaked into broker taint: {taint}"
        );
        assert!(taint.len() <= 256, "taint must stay bounded: {taint}");

        let campaign = SandboxCampaign {
            first: record.clone(),
            second: record,
        };
        let (_, fields) = outcome_fields(&campaign, &cassette);
        let line = render_line(&fields);
        assert!(
            !line.contains(sentinel),
            "attacker content leaked into the outcome receipt line: {line}"
        );
    }

    /// Item 6: an injected execution counter proves no child launches
    /// when the output root is refused.
    #[test]
    fn output_refusal_leaves_execution_counter_at_zero() {
        let root = cooperative_root("refusal", &fixture_cassette());
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&root).unwrap();
        let out = root.join("occupied");
        std::fs::create_dir_all(&out).unwrap();
        std::fs::write(out.join("keep.txt"), b"keep").unwrap();
        let args = vec!["--out".to_string(), out.display().to_string()];
        let mut executions = 0u64;
        let code = cmd_cooperative_inner(&args, "usage", &mut executions);
        assert_eq!(code, 2);
        assert_eq!(executions, 0, "no child may launch after refusal");
        assert_eq!(std::fs::read(out.join("keep.txt")).unwrap(), b"keep");
    }

    // ---- item 3: deterministic child-failure semantics ----

    fn timeout_cassette() -> CassetteV2 {
        let mut cassette = CassetteV2::default();
        cassette.push(fixture_request(), TapeEntry::Timeout);
        cassette
    }

    fn named_field<'a>(fields: &'a [(&'a str, Val)], name: &str) -> &'a Val {
        &fields
            .iter()
            .find(|(k, _)| *k == name)
            .unwrap_or_else(|| panic!("missing field {name}"))
            .1
    }

    /// A fully consumed, untainted, identically reproduced cassette
    /// `Timeout` is a finding ONLY through the declared
    /// `cooperative-llm-call-completed` oracle: FINDINGS, exit 1,
    /// verified=true, findings_count=1, and the exact stable identity.
    #[test]
    fn cassette_timeout_is_an_oracle_verified_finding() {
        let cassette = timeout_cassette();
        let campaign = SandboxCampaign {
            first: run_one("timeout-oracle", &cassette),
            second: run_one("timeout-oracle", &cassette),
        };
        assert!(!campaign.first.transport_tainted());
        assert_eq!(campaign.first.identity(), campaign.second.identity());
        let (code, fields) = outcome_fields(&campaign, &cassette);
        assert_eq!(code, 1);
        assert!(matches!(named_field(&fields, "verdict"), Val::S(v) if v == "FINDINGS"));
        assert!(matches!(named_field(&fields, "verified"), Val::B(true)));
        assert!(matches!(named_field(&fields, "findings_count"), Val::N(1)));
        assert!(
            matches!(named_field(&fields, "oracle"), Val::S(v) if v == "cooperative-llm-call-completed")
        );
        assert!(
            matches!(named_field(&fields, "finding_identity"), Val::S(v) if v == "cooperative-llm-call-completed:timeout")
        );
    }

    /// A generic nonzero child exit after a fully consumed, untainted,
    /// reproduced SUCCESS exchange is unsupported evidence: UNCHECKED,
    /// never FINDINGS.
    #[test]
    fn generic_nonzero_exit_is_never_a_finding() {
        let cassette = make_cassette(false);
        let child = format!("{COOPERATIVE_ECHO_CHILD}\nimport sys\nsys.exit(7)\n");
        let root = cooperative_root("nonzero", &cassette);
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&root).unwrap();
        let source = root.join("nonzero.py");
        std::fs::write(&source, &child).unwrap();
        let spec = SandboxSpec::new(vec!["/usr/bin/python3".into(), "nonzero.py".into()])
            .unwrap()
            .with_cassette_identity(cassette.identity())
            .declare_artifact("out.txt")
            .unwrap()
            .declare_input_file(&source)
            .unwrap();
        let run = |ws: &str| {
            let workspace = root.join(ws);
            std::fs::create_dir_all(&workspace).unwrap();
            std::fs::copy(&source, workspace.join("nonzero.py")).unwrap();
            run_once_with_cassette(&spec, &workspace, &cassette).unwrap()
        };
        let campaign = SandboxCampaign {
            first: run("a"),
            second: run("b"),
        };
        assert!(!campaign.first.transport_tainted());
        assert_eq!(campaign.first.identity(), campaign.second.identity());
        assert!(matches!(
            campaign.first.termination,
            vh_sandbox::TerminationOutcome::Exited(7)
        ));
        let (code, fields) = outcome_fields(&campaign, &cassette);
        assert_eq!(code, 3, "generic nonzero exit must stay UNCHECKED");
        assert!(matches!(named_field(&fields, "verdict"), Val::S(v) if v == "UNCHECKED"));
        assert!(matches!(named_field(&fields, "findings_count"), Val::N(0)));
        assert!(matches!(named_field(&fields, "verified"), Val::B(false)));
    }

    /// A cassette `Timeout` with unconsumed recorded tape is transport
    /// evidence failure — UNCHECKED, never the oracle finding.
    #[test]
    fn timeout_with_unconsumed_tape_is_unchecked_not_a_finding() {
        let mut cassette = timeout_cassette();
        cassette.push(fixture_request(), TapeEntry::Timeout);
        let campaign = SandboxCampaign {
            first: run_one("timeout-unconsumed", &cassette),
            second: run_one("timeout-unconsumed", &cassette),
        };
        let (code, fields) = outcome_fields(&campaign, &cassette);
        assert_eq!(code, 3);
        assert!(matches!(named_field(&fields, "verdict"), Val::S(v) if v == "UNCHECKED"));
        assert!(matches!(named_field(&fields, "findings_count"), Val::N(0)));
    }

    #[test]
    fn divergence_is_unchecked_and_never_a_target_finding() {
        let cassette = fixture_cassette();
        let first = run_one("divergence-unchecked", &cassette);
        let mut second = first.clone();
        second.termination = vh_sandbox::TerminationOutcome::Exited(7);
        let campaign = SandboxCampaign { first, second };
        assert_ne!(campaign.first.identity(), campaign.second.identity());
        let (code, fields) = outcome_fields(&campaign, &cassette);
        assert_eq!(code, 3);
        assert!(matches!(named_field(&fields, "verdict"), Val::S(v) if v == "UNCHECKED"));
        assert!(matches!(named_field(&fields, "verified"), Val::B(false)));
        assert!(matches!(named_field(&fields, "findings_count"), Val::N(0)));
    }

    #[test]
    fn bounded_diagnostic_includes_marker_inside_the_byte_cap() {
        let diagnostic = bounded_diagnostic(&"é".repeat(MAX_DIAGNOSTIC_BYTES));
        assert!(diagnostic.len() <= MAX_DIAGNOSTIC_BYTES);
        assert!(diagnostic.ends_with("...[truncated]"));
        assert!(diagnostic.is_char_boundary(diagnostic.len()));
    }

    #[test]
    fn ephemeral_replay_workspace_is_removed_when_lease_drops() {
        let mut executions = 0;
        let workspace;
        {
            let (_campaign, lease) =
                run_cooperative_campaign(None, "cleanup", None, &mut executions).unwrap();
            workspace = lease.path().to_path_buf();
            assert!(workspace.is_dir());
        }
        assert_eq!(executions, 2);
        assert!(!workspace.exists(), "ephemeral workspace must be cleaned");
    }

    fn receipt_fixture(
        label: &str,
    ) -> (
        PathBuf,
        CassetteV2,
        SandboxCampaign,
        WorkspaceLease,
        Vec<(&'static str, Val)>,
    ) {
        let cassette = fixture_cassette();
        let root = cooperative_root(label, &cassette);
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir(&root).unwrap();
        let mut executions = 0;
        let (campaign, workspace) = run_cooperative_campaign(
            Some(&cassette),
            "cooperative-echo",
            Some(&root),
            &mut executions,
        )
        .unwrap();
        assert_eq!(executions, 2);
        let (_, fields) = outcome_fields(&campaign, &cassette);
        (root, cassette, campaign, workspace, fields)
    }

    #[test]
    fn receipt_publication_never_clobbers_existing_temp_or_final_bytes() {
        for occupied_name in ["cooperative.receipt.tmp", COOPERATIVE_RECEIPT_NAME] {
            let label = if occupied_name.ends_with(".tmp") {
                "no-clobber-tmp"
            } else {
                "no-clobber-final"
            };
            let (root, cassette, campaign, workspace, fields) = receipt_fixture(label);
            let occupied = root.join(occupied_name);
            std::fs::write(&occupied, b"precious-existing-bytes").unwrap();
            let result = write_cooperative_receipt(
                "cooperative-echo",
                &cassette,
                &campaign,
                workspace.path(),
                &fields,
                &root,
            );
            assert!(result.is_err());
            assert_eq!(
                std::fs::read(&occupied).unwrap(),
                b"precious-existing-bytes"
            );
        }
    }

    #[test]
    fn expected_request_mismatch_fails_before_fresh_replay() {
        let (root, cassette, campaign, workspace, fields) = receipt_fixture("expected-mismatch");
        write_cooperative_receipt(
            "cooperative-echo",
            &cassette,
            &campaign,
            workspace.path(),
            &fields,
            &root,
        )
        .unwrap();
        let bytes = std::fs::read(root.join(COOPERATIVE_RECEIPT_NAME)).unwrap();
        let expected = ExpectedCooperativeRequest {
            workload: "cooperative-echo".into(),
            cassette_bytes: timeout_cassette().file_bytes(),
        };
        let mut executions = 0;
        let (code, verify_fields) =
            verify_cooperative_receipt(&bytes, Some(&expected), &mut executions);
        assert_eq!(code, 1);
        assert_eq!(executions, 0);
        assert!(matches!(
            named_field(&verify_fields, "authentic"),
            Val::B(false)
        ));
        assert!(field_string(&verify_fields, "errors").contains("expected-cassette-mismatch"));
    }

    fn negotiated_v2_fixture(
        label: &str,
    ) -> (
        Vec<u8>,
        ExpectedV2Request,
        crate::protocol::ProtocolManifest,
    ) {
        let (_root, cassette, campaign, workspace, fields) = receipt_fixture(label);
        let manifest = crate::protocol::ProtocolManifest::current().unwrap();
        let features = crate::protocol::MANDATORY_FEATURES
            .iter()
            .map(|feature| feature.to_string())
            .collect::<Vec<_>>();
        let negotiated = crate::protocol::negotiate(
            &manifest,
            crate::protocol::MANIFEST_SCHEMA,
            &manifest.manifest_id,
            crate::protocol::OPERATION,
            &features,
            crate::protocol::RequestedTargetRevision::parse(&format!(
                "sha256:{}",
                vh_digest::sha256_hex(COOPERATIVE_ECHO_CHILD.as_bytes())
            ))
            .unwrap(),
        )
        .unwrap();
        let legacy_receipt = build_cooperative_receipt(
            "cooperative-echo",
            &cassette,
            &campaign,
            workspace.path(),
            &fields,
        )
        .unwrap();
        let fresh = crate::protocol::resolve_fresh_compiled_target();
        let matched = crate::protocol::match_requested_revision(&negotiated, &fresh).unwrap();
        let receipt =
            build_v2_receipt(&manifest, &matched, &cassette, &campaign, &legacy_receipt).unwrap();
        let expected = ExpectedV2Request {
            negotiated,
            cassette_bytes: cassette.file_bytes(),
        };
        (receipt, expected, manifest)
    }

    fn find_bytes(haystack: &[u8], needle: &[u8]) -> usize {
        haystack
            .windows(needle.len())
            .position(|window| window == needle)
            .unwrap_or_else(|| panic!("missing byte fixture {:?}", String::from_utf8_lossy(needle)))
    }

    fn refresh_outer_receipt_digest(receipt: &mut [u8]) {
        let marker = b"digest sha256:";
        let marker_start = receipt
            .windows(marker.len())
            .rposition(|window| window == marker)
            .expect("outer digest line");
        let digest_start = marker_start + marker.len();
        let digest_end = digest_start + 64;
        assert_eq!(receipt.get(digest_end), Some(&b'\n'));
        let digest = vh_digest::sha256_hex(&receipt[..marker_start]);
        receipt[digest_start..digest_end].copy_from_slice(digest.as_bytes());
    }

    #[test]
    fn negotiated_v2_rejects_missing_claim_before_replay() {
        let (mut receipt, expected, _) = negotiated_v2_fixture("v2-missing-claim");
        let prefix = b"claimed-observed-revision ";
        let start = find_bytes(&receipt, prefix);
        let end = start
            + receipt[start..]
                .iter()
                .position(|byte| *byte == b'\n')
                .expect("claim line")
            + 1;
        receipt.drain(start..end);
        refresh_outer_receipt_digest(&mut receipt);
        let mut executions = 0;
        let error = verify_v2_receipt(
            &receipt,
            &expected,
            &mut executions,
            crate::protocol::VERIFY_SCHEMA,
        )
        .unwrap_err();
        assert!(error.contains("malformed v2 receipt"), "{error}");
        assert_eq!(executions, 0, "missing claim must refuse before replay");
    }

    #[test]
    fn negotiated_v2_rejects_mutated_claim_before_replay() {
        let (mut receipt, expected, _) = negotiated_v2_fixture("v2-mutated-claim");
        let prefix = b"claimed-observed-revision 64:";
        let value_start = find_bytes(&receipt, prefix) + prefix.len();
        receipt[value_start..value_start + 64].fill(b'0');
        refresh_outer_receipt_digest(&mut receipt);
        let mut executions = 0;
        let error = verify_v2_receipt(
            &receipt,
            &expected,
            &mut executions,
            crate::protocol::VERIFY_SCHEMA,
        )
        .unwrap_err();
        assert!(
            error.contains("claimed/fresh observation mismatch"),
            "{error}"
        );
        assert_eq!(executions, 0, "mutated claim must refuse before replay");
    }

    #[test]
    fn negotiated_v2_rejects_alternate_expected_request_before_replay() {
        let (receipt, expected, manifest) = negotiated_v2_fixture("v2-request-substitution");
        let alternate = crate::protocol::negotiate(
            &manifest,
            crate::protocol::MANIFEST_SCHEMA,
            &manifest.manifest_id,
            crate::protocol::OPERATION,
            expected.negotiated.features(),
            crate::protocol::RequestedTargetRevision::parse(&format!("sha256:{}", "0".repeat(64)))
                .unwrap(),
        )
        .unwrap();
        let substituted = ExpectedV2Request {
            negotiated: alternate,
            cassette_bytes: expected.cassette_bytes,
        };
        let mut executions = 0;
        let error = verify_v2_receipt(
            &receipt,
            &substituted,
            &mut executions,
            crate::protocol::VERIFY_SCHEMA,
        )
        .unwrap_err();
        assert!(error.contains("expected-request-mismatch"), "{error}");
        assert_eq!(executions, 0, "request substitution must precede replay");
    }

    #[test]
    fn negotiated_v2_rejects_alternate_expected_cassette_before_replay() {
        let (receipt, mut expected, _) = negotiated_v2_fixture("v2-cassette-substitution");
        expected.cassette_bytes = timeout_cassette().file_bytes();
        let mut executions = 0;
        let error = verify_v2_receipt(
            &receipt,
            &expected,
            &mut executions,
            crate::protocol::VERIFY_SCHEMA,
        )
        .unwrap_err();
        assert!(error.contains("expected-cassette-mismatch"), "{error}");
        assert_eq!(executions, 0, "cassette substitution must precede replay");
    }

    #[test]
    fn negotiated_v2_rejects_operation_and_feature_mutations_before_replay() {
        let (receipt, expected, _) = negotiated_v2_fixture("v2-bound-field-substitution");
        let mutations: [(&[u8], &[u8]); 2] = [
            (b"cooperative-target-v1", b"cooperative-target-v2"),
            (b"cooperative-cassette-v2", b"cooperative-cassette-v3"),
        ];
        for (original, replacement) in mutations {
            let mut changed = receipt.clone();
            let start = find_bytes(&changed, original);
            changed[start..start + original.len()].copy_from_slice(replacement);
            refresh_outer_receipt_digest(&mut changed);
            let mut executions = 0;
            let error = verify_v2_receipt(
                &changed,
                &expected,
                &mut executions,
                crate::protocol::VERIFY_SCHEMA,
            )
            .unwrap_err();
            assert!(
                error.contains("descriptor-mismatch")
                    || error.contains("expected-request-mismatch"),
                "{error}"
            );
            assert_eq!(
                executions, 0,
                "bound operation/feature mutation must precede replay"
            );
        }
    }

    #[test]
    fn negotiated_identity_dag_propagates_revision_without_minting_invalid_receipts() {
        let (_root, cassette, campaign, workspace, fields) = receipt_fixture("v2-identity-dag");
        let manifest = crate::protocol::ProtocolManifest::current().unwrap();
        let features = crate::protocol::MANDATORY_FEATURES
            .iter()
            .map(|feature| feature.to_string())
            .collect::<Vec<_>>();
        let fresh = crate::protocol::resolve_fresh_compiled_target();
        let exact =
            crate::protocol::RequestedTargetRevision::parse(&format!("sha256:{}", fresh.digest()))
                .unwrap();
        let different =
            crate::protocol::RequestedTargetRevision::parse(&format!("sha256:{}", "0".repeat(64)))
                .unwrap();
        let exact_request = crate::protocol::negotiate(
            &manifest,
            crate::protocol::MANIFEST_SCHEMA,
            &manifest.manifest_id,
            crate::protocol::OPERATION,
            &features,
            exact,
        )
        .unwrap();
        let different_request = crate::protocol::negotiate(
            &manifest,
            crate::protocol::MANIFEST_SCHEMA,
            &manifest.manifest_id,
            crate::protocol::OPERATION,
            &features,
            different,
        )
        .unwrap();
        let legacy = build_cooperative_receipt(
            "cooperative-echo",
            &cassette,
            &campaign,
            workspace.path(),
            &fields,
        )
        .unwrap();
        let exact_matched =
            crate::protocol::match_requested_revision(&exact_request, &fresh).unwrap();
        let exact_receipt =
            build_v2_receipt(&manifest, &exact_matched, &cassette, &campaign, &legacy).unwrap();
        let exact_parsed = parse_v2_receipt(&exact_receipt).unwrap();
        let different_request_id =
            crate::protocol::engine_request_id(&different_request, &cassette.identity());
        let different_evidence_id = crate::protocol::evidence_id(
            &different_request_id,
            fresh.digest(),
            &campaign.first.identity(),
            &campaign.second.identity(),
        );
        assert_ne!(exact_parsed.engine_request_id, different_request_id);
        assert_ne!(exact_parsed.evidence_id, different_evidence_id);
        assert!(crate::protocol::match_requested_revision(&different_request, &fresh).is_err());
    }

    #[test]
    fn negotiated_v2_positive_standalone_reverification_is_fresh() {
        let (receipt, expected, _) = negotiated_v2_fixture("v2-positive-reverify");
        let mut executions = 0;
        let (outcome_code, record) = verify_v2_receipt(
            &receipt,
            &expected,
            &mut executions,
            crate::protocol::VERIFY_SCHEMA,
        )
        .unwrap();
        assert_eq!(outcome_code, 0);
        assert_eq!(
            executions, 2,
            "standalone verifier must freshly replay twice"
        );
        assert!(record.starts_with(format!("{}\n", crate::protocol::VERIFY_SCHEMA).as_bytes()));
        let first_newline = record.iter().position(|byte| *byte == b'\n').unwrap();
        assert!(
            record[first_newline + 1..].starts_with(b"verdict "),
            "verification schema must be followed directly by its first field"
        );
    }

    #[test]
    fn legacy_v1_receipt_and_verifier_remain_explicitly_compatible() {
        let (_root, cassette, campaign, workspace, fields) = receipt_fixture("v1-compatibility");
        let bytes = build_cooperative_receipt(
            "cooperative-echo",
            &cassette,
            &campaign,
            workspace.path(),
            &fields,
        )
        .unwrap();
        assert!(bytes.starts_with(format!("{COOPERATIVE_RECEIPT_SCHEMA}\n").as_bytes()));
        let expected = ExpectedCooperativeRequest {
            workload: "cooperative-echo".into(),
            cassette_bytes: cassette.file_bytes(),
        };
        let mut executions = 0;
        let (code, verify_fields) =
            verify_cooperative_receipt(&bytes, Some(&expected), &mut executions);
        assert_eq!(code, 0);
        assert_eq!(executions, 2);
        assert_eq!(
            field_string(&verify_fields, "schema"),
            COOPERATIVE_VERIFY_SCHEMA
        );
    }

    #[test]
    fn staged_target_substitution_refuses_before_any_child_execution() {
        let cassette = fixture_cassette();
        let root = cooperative_root("v2-staged-substitution", &cassette);
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir(&root).unwrap();
        let manifest = crate::protocol::ProtocolManifest::current().unwrap();
        let exact_original = format!(
            "sha256:{}",
            vh_digest::sha256_hex(COOPERATIVE_ECHO_CHILD.as_bytes())
        );
        let mut args = vec![
            "--protocol-schema".to_string(),
            crate::protocol::MANIFEST_SCHEMA.to_string(),
            "--manifest-id".to_string(),
            manifest.manifest_id.clone(),
            "--operation".to_string(),
            crate::protocol::OPERATION.to_string(),
        ];
        for feature in crate::protocol::MANDATORY_FEATURES {
            args.push("--require-feature".to_string());
            args.push((*feature).to_string());
        }
        args.extend([
            "--requested-target-revision".to_string(),
            exact_original,
            "--out".to_string(),
            root.to_string_lossy().into_owned(),
        ]);
        let mut mutate = |a: &Path, _b: &Path| {
            std::fs::write(a.join(CHILD_LOGICAL_NAME), b"substituted target bytes\n")
                .map_err(|error| error.to_string())
        };
        let mut output = Vec::new();
        let code = cmd_cooperative_v2_inner(&args, "test usage", &mut output, Some(&mut mutate));
        let refusal = String::from_utf8(output).unwrap();
        assert_eq!(code, 4);
        assert!(refusal.starts_with("vh-engine-negotiation-refusal-v1\n"));
        assert!(refusal.contains("reason 27:requested-revision-mismatch\n"));
        assert!(refusal.contains("executions 0\n"));
        assert!(!refusal.contains("CLEAN"));
        assert!(!root.join(COOPERATIVE_RECEIPT_NAME).exists());
    }

    #[test]
    fn negotiated_registry_refuses_unsupported_invalid_and_stale_requests() {
        let manifest = crate::protocol::ProtocolManifest::current().unwrap();
        let good = crate::protocol::MANDATORY_FEATURES
            .iter()
            .map(|feature| feature.to_string())
            .collect::<Vec<_>>();
        let negotiate = |features: &[String], manifest_id: &str| {
            crate::protocol::negotiate(
                &manifest,
                crate::protocol::MANIFEST_SCHEMA,
                manifest_id,
                crate::protocol::OPERATION,
                features,
                crate::protocol::RequestedTargetRevision::parse(&format!(
                    "sha256:{}",
                    vh_digest::sha256_hex(COOPERATIVE_ECHO_CHILD.as_bytes())
                ))
                .unwrap(),
            )
        };
        let mut unsupported = good.clone();
        unsupported.push("unsupported-v1".into());
        unsupported.sort();
        assert_eq!(
            negotiate(&unsupported, &manifest.manifest_id),
            Err(crate::protocol::RefusalReason::UnsupportedFeature)
        );
        let mut duplicate = good.clone();
        duplicate.insert(1, duplicate[0].clone());
        assert_eq!(
            negotiate(&duplicate, &manifest.manifest_id),
            Err(crate::protocol::RefusalReason::InvalidFeatureSet)
        );
        let mut unsorted = good.clone();
        unsorted.reverse();
        assert_eq!(
            negotiate(&unsorted, &manifest.manifest_id),
            Err(crate::protocol::RefusalReason::InvalidFeatureSet)
        );
        let oversized = vec!["fresh-replay-v1".to_string(); crate::protocol::MAX_FEATURES + 1];
        assert_eq!(
            negotiate(&oversized, &manifest.manifest_id),
            Err(crate::protocol::RefusalReason::InvalidFeatureSet)
        );
        assert_eq!(
            negotiate(&good, &"0".repeat(64)),
            Err(crate::protocol::RefusalReason::ProtocolManifestMismatch)
        );
        assert_eq!(
            crate::protocol::negotiate(
                &manifest,
                crate::protocol::MANIFEST_SCHEMA,
                &manifest.manifest_id,
                crate::protocol::OPERATION,
                &good,
                crate::protocol::RequestedTargetRevision::parse("unknown").unwrap(),
            ),
            Err(crate::protocol::RefusalReason::RequestedRevisionMismatch)
        );
    }

    #[test]
    fn negotiated_verifier_cli_rejects_duplicate_singletons_and_relative_receipt() {
        let duplicate = vec![
            "--receipt".to_string(),
            "/tmp/cooperative.receipt".to_string(),
            "--expected-request-schema".to_string(),
            crate::protocol::REQUEST_SCHEMA.to_string(),
            "--expected-request-schema".to_string(),
            crate::protocol::REQUEST_SCHEMA.to_string(),
        ];
        assert_eq!(cmd_verify_cooperative_v2(&duplicate, "usage"), 2);

        let relative = vec![
            "--receipt".to_string(),
            COOPERATIVE_RECEIPT_NAME.to_string(),
        ];
        assert_eq!(cmd_verify_cooperative_v2(&relative, "usage"), 2);
    }
}
