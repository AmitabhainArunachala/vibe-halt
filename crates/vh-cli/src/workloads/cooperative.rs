//! Cooperative D2 transport workload (Wave B / R2).
//!
//! Exposes the existing child-visible cassette protocol (`vh-cassette-v2` /
//! `vh-cassette-transport-v1`) as a reusable workload behind the R1
//! `vh run --workload <NAME>` and Python `RunRequest` surface. The fixture
//! is a minimal echo child that makes one LLM request through the file
//! mailbox and writes the response bytes to `out.txt`.
//!
//! This is a boundary-crate workload: it performs host filesystem I/O and
//! spawns a subprocess through `vh_sandbox`, so it is not subject to the
//! kernel-grade deny-list. The relevant per-file exemptions are registered in
//! `scripts/check_determinism_denylist.py`.

use std::collections::BTreeMap;

use vh_multiverse::{EndStateOracle, PropertyContract, RunOutcome, UniverseCtx, Workload};
use vh_sandbox::{fnv_hex, CassetteV2, LlmRequestV2, SandboxSpec, TapeEntry, TerminationOutcome};

const EXPECTED_BODY: &[u8] = b"echo-reply";

// Python child SDK for one request/response through the vh-cassette-transport-v1
// file mailbox. It matches the canonical request framing produced by
// `LlmRequestV2::canonical_bytes` and the response framing produced by
// `TapeEntry::response_frame`. No second protocol is introduced.
const COOP_CHILD: &str = r###"import os, sys, time
MAILBOX = os.path.join('.vh-sandbox-io', 'llm')
DEADLINE_SECONDS = 20.0

def _field(tag, value):
    return tag.encode() + b' ' + str(len(value)).encode() + b':' + value + b'\n'

def _opt(tag, value):
    if value is None:
        return tag.encode() + b' absent\n'
    return _field(tag, value.encode())

def canonical_request(provider, model, messages, tools=(), tool_choice=None,
                      structured_output=None, params=()):
    out = b'vh-llm-request-v2\n'
    out += _field('provider', provider.encode())
    out += _field('model', model.encode())
    out += ('messages %d\n' % len(messages)).encode()
    for role, content in messages:
        out += _field('role', role.encode())
        out += _field('content', content.encode())
    out += ('tools %d\n' % len(tools)).encode()
    for name, schema in tools:
        out += _field('tool-name', name.encode())
        out += _field('tool-schema', schema.encode())
    out += _opt('tool-choice', tool_choice)
    out += _opt('structured-output', structured_output)
    items = sorted(dict(params).items())
    out += ('params %d\n' % len(items)).encode()
    for k, v in items:
        out += _field('param-key', k.encode())
        out += _field('param-value', v.encode())
    return out

def _read_field(data, pos, tag):
    tagb = tag.encode() + b' '
    if data[pos:pos + len(tagb)] != tagb:
        sys.exit(43)
    pos += len(tagb)
    colon = data.index(b':', pos)
    ln = int(data[pos:colon])
    pos = colon + 1
    val = data[pos:pos + ln]
    if data[pos + ln:pos + ln + 1] != b'\n':
        sys.exit(43)
    return val, pos + ln + 1

def llm_call(**kw):
    frame = canonical_request(**kw)
    tmp = os.path.join(MAILBOX, 'req-0.tmp')
    final = os.path.join(MAILBOX, 'req-0')
    with open(tmp, 'wb') as f:
        f.write(frame)
    os.replace(tmp, final)
    resp_path = os.path.join(MAILBOX, 'resp-0')
    start = time.monotonic()
    while not os.path.exists(resp_path):
        if time.monotonic() - start > DEADLINE_SECONDS:
            sys.exit(41)
        time.sleep(0.002)
    with open(resp_path, 'rb') as f:
        data = f.read()
    nl = data.index(b'\n')
    head = data[:nl].decode()
    pos = nl + 1
    if head.startswith('transport-error'):
        sys.stderr.write('transport error: %s\n' % head)
        sys.exit(42)
    if head.startswith('success '):
        body, _ = _read_field(data, pos, 'body')
        return {'kind': 'success', 'status': int(head.split(' ')[1]), 'body': body}
    if head.startswith('provider-error '):
        body, _ = _read_field(data, pos, 'body')
        return {'kind': 'provider-error', 'status': int(head.split(' ')[1]), 'body': body}
    if head == 'timeout':
        return {'kind': 'timeout'}
    sys.exit(43)

result = llm_call(provider='fixture', model='echo', messages=[('user', 'hello')], params=[('temperature', '0')])
with open('out.txt', 'wb') as f:
    f.write(result['body'])
print(result['body'].decode(), end='')
"###;

pub struct CooperativeEcho;

impl Workload for CooperativeEcho {
    fn name(&self) -> &str {
        "cooperative-echo"
    }

    fn property_contract(&self) -> PropertyContract {
        PropertyContract::new(&[], &[]).with_oracles(&["cooperative_echo_ok"])
    }

    fn end_state_oracles(&self) -> Vec<EndStateOracle> {
        vec![EndStateOracle {
            name: "cooperative_echo_ok",
            check: |state| match state.get("cooperative-echo") {
                Some(v) if v == "ok" => Ok(()),
                other => Err(format!("expected cooperative-echo=ok, got {other:?}")),
            },
        }]
    }

    fn run(&self, ctx: &mut UniverseCtx) -> RunOutcome {
        let expected_digest = fnv_hex(EXPECTED_BODY);

        // Reproducible per-universe workspace: same (root seed, universe) =>
        // same path, so the two divergence-check passes clean up and recreate
        // identically. The path itself is not part of the sandbox identity.
        let seed = ctx.universe_seed();
        let workspace = std::env::temp_dir().join(format!("vh-cooperative-echo-{seed:016x}"));
        let _ = std::fs::remove_dir_all(&workspace);

        let request = LlmRequestV2 {
            provider: "fixture".into(),
            model: "echo".into(),
            messages: vec![("user".into(), "hello".into())],
            tools: Vec::new(),
            tool_choice: None,
            structured_output: None,
            params: BTreeMap::from([("temperature".into(), "0".into())]),
        };
        let mut cassette = CassetteV2::default();
        cassette.push(
            request,
            TapeEntry::Success {
                status: 200,
                body: EXPECTED_BODY.to_vec(),
            },
        );

        let spec = match SandboxSpec::new(vec![
            "/usr/bin/python3".into(),
            "-c".into(),
            COOP_CHILD.into(),
        ]) {
            Ok(s) => s,
            Err(e) => return RunOutcome::ExecutionError(format!("invalid sandbox spec: {e}")),
        };
        let spec = match spec
            .with_cassette_identity(cassette.identity())
            .declare_artifact("out.txt")
        {
            Ok(s) => s,
            Err(e) => return RunOutcome::ExecutionError(format!("invalid sandbox spec: {e}")),
        };

        let record = match vh_sandbox::run_once_with_cassette(&spec, &workspace, &cassette) {
            Ok(r) => r,
            Err(e) => return RunOutcome::ExecutionError(format!("sandbox run failed: {e}")),
        };

        if record.transport_tainted() {
            let taint = record
                .transport
                .as_ref()
                .and_then(|t| t.taint.as_deref())
                .unwrap_or("unconsumed");
            return RunOutcome::ExecutionError(format!("transport tainted: {taint}"));
        }

        if record.termination != TerminationOutcome::Exited(0) {
            return RunOutcome::ExecutionError(format!(
                "child did not exit cleanly: {:?}",
                record.termination
            ));
        }

        let artifact_digest = record.artifacts.get("out.txt");
        if artifact_digest != Some(&expected_digest) {
            return RunOutcome::ExecutionError(format!(
                "artifact digest mismatch: expected {expected_digest}, got {artifact_digest:?}"
            ));
        }

        // Bind the cassette transport receipt into the universe trace so the
        // complete observation carries the served/unconsumed/taint evidence.
        let transport = record
            .transport
            .as_ref()
            .expect("cassette run carries a transport receipt");
        ctx.record("cooperative_echo", "completed");
        ctx.record("transport", &transport.identity_str());
        ctx.declare_end("cooperative-echo", "ok");

        RunOutcome::Completed
    }
}
