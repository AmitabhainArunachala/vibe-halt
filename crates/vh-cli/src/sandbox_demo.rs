//! CLI smoke for the Tier-2 D2 subprocess sandbox MVP (D1 is a future
//! backend).
//!
//! This file is a boundary file: it uses host tempdirs and subprocess sandbox
//! fixtures to exercise D2-honest run-twice evidence, not Tier-1 identity.

use std::collections::BTreeMap;

use vh_sandbox::{
    run_once as sandbox_run_once, run_once_with_cassette, Cassette, CassetteEntry, CassetteV2,
    LlmRequest, LlmRequestV2, SandboxCampaign, SandboxError, SandboxSpec, TapeEntry,
};

pub(crate) fn cmd_sandbox_demo(args: &[String], usage: &str) -> i32 {
    let mut mode = "clean".to_string();
    let mut it = args.iter();
    while let Some(arg) = it.next() {
        match arg.as_str() {
            "--mode" => {
                mode = match it.next() {
                    Some(v) => v.clone(),
                    None => {
                        eprintln!("error: --mode requires a value\n\n{usage}");
                        return 2;
                    }
                };
            }
            other => {
                eprintln!("error: unknown argument: {other}\n\n{usage}");
                return 2;
            }
        }
    }
    match mode.as_str() {
        "clean" => render_clean(),
        "cassette-miss" => render_cassette_miss(),
        "nondet" => render_nondet(),
        "cassette-child" => render_cassette_child(),
        "cassette-child-miss" => render_cassette_child_taint("cassette-child-miss"),
        "cassette-child-extra" => render_cassette_child_taint("cassette-child-extra"),
        _ => {
            eprintln!(
                "error: unknown sandbox-demo mode '{mode}' (expected clean, cassette-miss, \
                 nondet, cassette-child, cassette-child-miss, or cassette-child-extra)"
            );
            2
        }
    }
}

fn render_clean() -> i32 {
    match sandbox_clean_campaign() {
        Ok(campaign) => {
            let report = campaign.divergence_report();
            println!("vibe-halt sandbox-demo: mode=clean");
            println!("  {}", campaign.verdict_line());
            println!(
                "  identities: first={} second={}",
                campaign.first.identity(),
                campaign.second.identity()
            );
            if report.diverged == 0 {
                println!("  verdict: CLEAN");
                0
            } else {
                println!("  verdict: FINDINGS (sandbox divergence)");
                1
            }
        }
        Err(e) => render_sandbox_error("clean", e),
    }
}

fn render_cassette_miss() -> i32 {
    let request = fixture_request("hello");
    let cassette = Cassette::default();
    match cassette.replay(&request) {
        Ok(_) => {
            println!("vibe-halt sandbox-demo: mode=cassette-miss");
            println!("  FAIL cassette: unexpected fuzzy/live response");
            println!("  verdict: FINDINGS");
            1
        }
        Err(miss) => {
            println!("vibe-halt sandbox-demo: mode=cassette-miss");
            println!("  FAIL cassette: miss digest={}", miss.digest);
            println!("  verdict: FINDINGS (fail-closed cassette miss)");
            1
        }
    }
}

fn render_nondet() -> i32 {
    match sandbox_nondet_campaign() {
        Ok(campaign) => {
            let report = campaign.divergence_report();
            println!("vibe-halt sandbox-demo: mode=nondet");
            println!("  {}", campaign.verdict_line());
            println!(
                "  identities: first={} second={}",
                campaign.first.identity(),
                campaign.second.identity()
            );
            if report.diverged > 0 {
                println!("  DIVERGENT sandbox subprocess observable records differ");
                println!("  verdict: FINDINGS");
                1
            } else {
                println!("  verdict: CLEAN");
                0
            }
        }
        Err(e) => render_sandbox_error("nondet", e),
    }
}

fn render_sandbox_error(mode: &str, e: SandboxError) -> i32 {
    println!("vibe-halt sandbox-demo: mode={mode}");
    println!("  FAIL sandbox: {e}");
    println!("  verdict: FINDINGS");
    1
}

fn sandbox_clean_campaign() -> Result<SandboxCampaign, SandboxError> {
    let root = sandbox_demo_root("clean")?;
    let spec = sandbox_demo_spec("stable")?;
    vh_sandbox::run_twice(&spec, &root.join("u0-a"), &root.join("u0-b"))
}

fn sandbox_nondet_campaign() -> Result<SandboxCampaign, SandboxError> {
    let root = sandbox_demo_root("nondet")?;
    let spec = sandbox_demo_spec("nondet")?;
    Ok(SandboxCampaign {
        first: sandbox_run_once(&spec, &root.join("u0-a"))?,
        second: sandbox_run_once(&spec, &root.join("u0-b"))?,
    })
}

fn sandbox_demo_root(label: &str) -> Result<std::path::PathBuf, SandboxError> {
    let p = std::env::temp_dir().join(format!("vh-sandbox-demo-{label}"));
    let _ = std::fs::remove_dir_all(&p);
    std::fs::create_dir_all(&p)?;
    Ok(p)
}

fn sandbox_demo_spec(mode: &str) -> Result<SandboxSpec, SandboxError> {
    let request = fixture_request("hello");
    let mut cassette = Cassette::default();
    cassette.insert(
        &request,
        CassetteEntry {
            response: b"fixture-response".to_vec(),
            boundary_telemetry: std::collections::BTreeMap::from([(
                "captured_by".to_string(),
                "offline-fixture".to_string(),
            )]),
        },
    );
    let response = String::from_utf8_lossy(&cassette.replay(&request).map_err(|m| {
        SandboxError::Execution(format!(
            "fixture cassette unexpectedly missed: {}",
            m.digest
        ))
    })?)
    .into_owned();
    let code = if mode == "nondet" {
        format!(
            "import os\nopen('out.txt','w').write('llm={response}\\n' + os.getcwd())\nprint(open('out.txt').read(), end='')"
        )
    } else {
        format!(
            "open('out.txt','w').write('llm={response}')\nprint(open('out.txt').read(), end='')"
        )
    };
    SandboxSpec::new(vec!["/usr/bin/python3".into(), "-c".into(), code])?
        .declare_artifact("out.txt")
}

fn fixture_request(content: &str) -> LlmRequest {
    LlmRequest {
        provider: "fixture".into(),
        model: "echo".into(),
        messages: vec![content.into()],
        params: std::collections::BTreeMap::from([("temperature".into(), "0".into())]),
    }
}

// ---- C5: child-visible cassette transport fixtures (audit E.2) ----
//
// The CHILD makes every request through the vh-cassette-transport-v1
// file mailbox. Nothing from the cassette is ever interpolated into the
// child's source: the Python below embeds only REQUEST parameters; the
// responses exist solely on the recorded tape the broker replays.

/// Python SDK + agent, written into the workspace and executed as the
/// cooperative single-thread CPython child. `%MODE%` selects the agent's
/// request sequence.
const PY_CHILD: &str = r#"
import os, sys, time

MAILBOX = os.path.join('.vh-sandbox-io', 'llm')
DEADLINE_SECONDS = 20.0
_seq = 0

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
    pos += ln
    if data[pos:pos + 1] != b'\n':
        sys.exit(43)
    return val, pos + 1

def llm_call(**kw):
    global _seq
    n = _seq
    _seq += 1
    frame = canonical_request(**kw)
    tmp = os.path.join(MAILBOX, 'req-%d.tmp' % n)
    final = os.path.join(MAILBOX, 'req-%d' % n)
    with open(tmp, 'wb') as f:
        f.write(frame)
    os.replace(tmp, final)
    resp_path = os.path.join(MAILBOX, 'resp-%d' % n)
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
    if head.startswith('success ') or head.startswith('provider-error '):
        body, _ = _read_field(data, pos, 'body')
        kind, status = head.split(' ')
        return {'kind': kind, 'status': int(status), 'body': body}
    if head == 'timeout':
        return {'kind': 'timeout'}
    if head.startswith('stream '):
        count = int(head.split(' ')[1])
        chunks = []
        for _ in range(count):
            chunk, pos = _read_field(data, pos, 'chunk')
            chunks.append(chunk)
        terminal, _ = _read_field(data, pos, 'terminal')
        return {'kind': 'stream', 'chunks': chunks, 'terminal': terminal.decode()}
    sys.exit(43)

def base(content):
    return dict(provider='fixture', model='echo-2',
                messages=[('system', 'be deterministic'), ('user', content)],
                params=[('temperature', '0')])

collected = []
first = llm_call(**base('same prompt'))
collected.append(first['body'])
second = llm_call(**base('same prompt'))
collected.append(second['body'])
stream = llm_call(tools=[('lookup', '{"type":"object"}')], tool_choice='auto',
                  **base('stream please'))
if stream['kind'] != 'stream' or stream['terminal'] != 'done':
    sys.exit(44)
collected.extend(stream['chunks'])
if '%MODE%' != 'cassette-child-short':
    err = llm_call(**base('and now fail'))
    if err['kind'] != 'provider-error' or err['status'] != 529:
        sys.exit(45)
    collected.append(err['body'])
with open('out.txt', 'wb') as f:
    f.write(b'|'.join(collected))
"#;

fn fixture_request_v2(content: &str, with_tools: bool) -> LlmRequestV2 {
    LlmRequestV2 {
        provider: "fixture".into(),
        model: "echo-2".into(),
        messages: vec![
            ("system".into(), "be deterministic".into()),
            ("user".into(), content.into()),
        ],
        tools: if with_tools {
            vec![("lookup".into(), r#"{"type":"object"}"#.into())]
        } else {
            Vec::new()
        },
        tool_choice: with_tools.then(|| "auto".into()),
        structured_output: None,
        params: BTreeMap::from([("temperature".into(), "0".into())]),
    }
}

/// The recorded tape the agent above consumes: two IDENTICAL requests
/// with distinct ordered responses, one streaming request, one provider
/// error. `entries` bounds how much of the tape exists (miss fixture);
/// `extra` appends a never-requested entry (leftover fixture).
fn fixture_cassette_v2(entries: usize, extra: bool) -> CassetteV2 {
    let mut cassette = CassetteV2::default();
    let tape: Vec<(LlmRequestV2, TapeEntry)> = vec![
        (
            fixture_request_v2("same prompt", false),
            TapeEntry::Success {
                status: 200,
                body: b"first-of-two".to_vec(),
            },
        ),
        (
            fixture_request_v2("same prompt", false),
            TapeEntry::Success {
                status: 200,
                body: b"second-of-two".to_vec(),
            },
        ),
        (
            fixture_request_v2("stream please", true),
            TapeEntry::Stream {
                chunks: vec![b"chunk-a".to_vec(), b"chunk-b".to_vec()],
                terminal: "done".into(),
            },
        ),
        (
            fixture_request_v2("and now fail", false),
            TapeEntry::ProviderError {
                status: 529,
                body: b"overloaded".to_vec(),
            },
        ),
    ];
    for (request, entry) in tape.into_iter().take(entries) {
        cassette.push(request, entry);
    }
    if extra {
        cassette.push(
            fixture_request_v2("never requested", false),
            TapeEntry::Success {
                status: 200,
                body: b"unreachable".to_vec(),
            },
        );
    }
    cassette
}

/// Write the child source into the demo root (the controller-held
/// precondition `declare_input_file` binds), returning its path.
fn stage_child_source(
    root: &std::path::Path,
    mode: &str,
) -> Result<std::path::PathBuf, SandboxError> {
    let source = root.join("agent_fixture.py");
    std::fs::write(&source, PY_CHILD.replace("%MODE%", mode))?;
    Ok(source)
}

fn child_spec(
    cassette: &CassetteV2,
    source: &std::path::Path,
) -> Result<SandboxSpec, SandboxError> {
    SandboxSpec::new(vec!["/usr/bin/python3".into(), "agent_fixture.py".into()])?
        .with_cassette_identity(cassette.identity())
        .declare_artifact("out.txt")?
        .declare_input_file(source)
}

fn place_child_source(
    source: &std::path::Path,
    workspace: &std::path::Path,
) -> Result<(), SandboxError> {
    std::fs::create_dir_all(workspace)?;
    std::fs::copy(source, workspace.join("agent_fixture.py"))?;
    Ok(())
}

fn run_child_pair(
    cassette: &CassetteV2,
    label: &str,
    mode: &str,
) -> Result<SandboxCampaign, SandboxError> {
    let root = sandbox_demo_root(label)?;
    let source = stage_child_source(&root, mode)?;
    let spec = child_spec(cassette, &source)?;
    let (a, b) = (root.join("u0-a"), root.join("u0-b"));
    place_child_source(&source, &a)?;
    place_child_source(&source, &b)?;
    Ok(SandboxCampaign {
        first: run_once_with_cassette(&spec, &a, cassette)?,
        second: run_once_with_cassette(&spec, &b, cassette)?,
    })
}

fn render_cassette_child() -> i32 {
    let cassette = fixture_cassette_v2(4, false);
    match run_child_pair(&cassette, "cassette-child", "cassette-child") {
        Err(e) => render_sandbox_error("cassette-child", e),
        Ok(campaign) => {
            println!("vibe-halt sandbox-demo: mode=cassette-child");
            println!("  cassette: {}", cassette.identity());
            println!("  {}", campaign.verdict_line());
            println!(
                "  identities: first={} second={}",
                campaign.first.identity(),
                campaign.second.identity()
            );
            let transport = campaign
                .first
                .transport
                .as_ref()
                .expect("cassette run carries a transport receipt");
            println!(
                "  transport: served={} unconsumed={} taint={}",
                transport.served.len(),
                transport.unconsumed,
                transport.taint.as_deref().unwrap_or("none")
            );
            let report = campaign.divergence_report();
            if campaign.first.transport_tainted() || campaign.second.transport_tainted() {
                println!("  verdict: UNCHECKED (transport taint — evidence incomplete)");
                3
            } else if report.diverged == 0 {
                println!("  verdict: CLEAN (child-visible cassette; Tier-2 D2 — network channel remains open until the supervisor)");
                0
            } else {
                println!("  verdict: FINDINGS (sandbox divergence)");
                1
            }
        }
    }
}

/// Both taint fixtures end UNCHECKED (exit 3): missing evidence is never
/// a target defect, and leftovers mean the recorded history was not the
/// history that ran.
fn render_cassette_child_taint(mode: &str) -> i32 {
    let cassette = match mode {
        "cassette-child-miss" => fixture_cassette_v2(3, false),
        _ => fixture_cassette_v2(4, true),
    };
    let root = match sandbox_demo_root(mode) {
        Ok(root) => root,
        Err(e) => return render_sandbox_error(mode, e),
    };
    let source = match stage_child_source(&root, mode) {
        Ok(source) => source,
        Err(e) => return render_sandbox_error(mode, e),
    };
    let spec = match child_spec(&cassette, &source) {
        Ok(spec) => spec,
        Err(e) => return render_sandbox_error(mode, e),
    };
    let workspace = root.join("u0-a");
    if let Err(e) = place_child_source(&source, &workspace) {
        return render_sandbox_error(mode, e);
    }
    match run_once_with_cassette(&spec, &workspace, &cassette) {
        Err(e) => render_sandbox_error(mode, e),
        Ok(record) => {
            println!("vibe-halt sandbox-demo: mode={mode}");
            println!("  cassette: {}", cassette.identity());
            let transport = record
                .transport
                .as_ref()
                .expect("cassette run carries a transport receipt");
            println!(
                "  transport: served={} unconsumed={} taint={}",
                transport.served.len(),
                transport.unconsumed,
                transport.taint.as_deref().unwrap_or("none")
            );
            if record.transport_tainted() {
                println!("  TAINT transport: recorded history did not match execution");
                println!("  verdict: UNCHECKED (transport taint — evidence incomplete, never CLEAN or FINDINGS)");
                3
            } else {
                println!("  FAIL taint fixture unexpectedly ran clean");
                println!("  verdict: FINDINGS");
                1
            }
        }
    }
}
