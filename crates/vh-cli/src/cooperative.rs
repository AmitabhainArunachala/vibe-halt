//! R2 — reusable cooperative D2 transport behind the generic R1
//! operation surface.
//!
//! This is a declared boundary file: it uses host tempdirs and the
//! `vh-sandbox` cassette broker to run a cooperative child twice and
//! report a typed, engine-only outcome. It introduces no Python truth
//! authority and keeps the existing `vh-cassette-transport-v1` wire
//! format as the single canonical protocol.

use std::path::{Path, PathBuf};

use vh_sandbox::{
    run_once_with_cassette, CassetteV2, LlmRequestV2, SandboxCampaign, SandboxError, SandboxSpec,
    TapeEntry,
};

use vh_cli::receipts::{render_line, Val};

pub(crate) const COOPERATIVE_OUTCOME_SCHEMA: &str = "vh-cooperative-outcome-v1";
pub(crate) const SCOPE: &str = "vibe-halt.run.v0";

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

fn cooperative_root(label: &str, cassette: &CassetteV2) -> PathBuf {
    let prefix = cassette_root_prefix(cassette);
    std::env::temp_dir().join(format!("vh-cooperative-{label}-{prefix}"))
}

fn stage_child_source(root: &Path) -> Result<PathBuf, SandboxError> {
    let source = root.join("cooperative_echo.py");
    std::fs::create_dir_all(root)?;
    std::fs::write(&source, COOPERATIVE_ECHO_CHILD)?;
    Ok(source)
}

fn place_child_source(source: &Path, workspace: &Path) -> Result<(), SandboxError> {
    std::fs::create_dir_all(workspace)?;
    std::fs::copy(source, workspace.join("cooperative_echo.py"))?;
    Ok(())
}

fn child_spec(cassette: &CassetteV2, source: &Path) -> Result<SandboxSpec, SandboxError> {
    SandboxSpec::new(vec![
        "/usr/bin/python3".into(),
        "cooperative_echo.py".into(),
    ])?
    .with_cassette_identity(cassette.identity())
    .declare_artifact("out.txt")?
    .declare_input_file(source)
}

fn run_cooperative_campaign(
    cassette_override: Option<&CassetteV2>,
    label: &str,
) -> Result<SandboxCampaign, String> {
    let owned_cassette;
    let cassette: &CassetteV2 = match cassette_override {
        Some(c) => c,
        None => {
            owned_cassette = fixture_cassette();
            &owned_cassette
        }
    };
    let root = cooperative_root(label, cassette);
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(&root).map_err(|e| format!("cannot create cooperative root: {e}"))?;

    let source =
        stage_child_source(&root).map_err(|e| format!("cannot stage child source: {e}"))?;
    let spec =
        child_spec(cassette, &source).map_err(|e| format!("invalid cooperative spec: {e}"))?;
    let a = root.join("a");
    let b = root.join("b");
    place_child_source(&source, &a)
        .map_err(|e| format!("cannot place child in workspace a: {e}"))?;
    place_child_source(&source, &b)
        .map_err(|e| format!("cannot place child in workspace b: {e}"))?;

    Ok(SandboxCampaign {
        first: run_once_with_cassette(&spec, &a, cassette)
            .map_err(|e| format!("cooperative run a failed: {e}"))?,
        second: run_once_with_cassette(&spec, &b, cassette)
            .map_err(|e| format!("cooperative run b failed: {e}"))?,
    })
}

fn outcome_fields(campaign: &SandboxCampaign) -> (i32, Vec<(&'static str, Val)>) {
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
    if diverged && errors.is_empty() {
        errors.push("cooperative run-twice identities diverged".into());
    }

    let child_failed = |rec: &vh_sandbox::RunRecord| {
        !matches!(rec.termination, vh_sandbox::TerminationOutcome::Exited(0))
    };

    if child_failed(first) && errors.is_empty() {
        errors.push(format!(
            "first run child did not exit cleanly: {:?}",
            first.termination
        ));
    }
    if child_failed(second) && errors.is_empty() {
        errors.push(format!(
            "second run child did not exit cleanly: {:?}",
            second.termination
        ));
    }

    let (verdict, exit_code, verified, findings_count) = if !errors.is_empty() {
        if first.transport_tainted()
            || second.transport_tainted()
            || unconsumed(first)
            || unconsumed(second)
        {
            ("UNCHECKED", 3, false, 0)
        } else {
            ("FINDINGS", 1, false, if diverged { 1 } else { 0 })
        }
    } else {
        ("CLEAN", 0, true, 0)
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
        ("findings_count", Val::N(findings_count)),
        ("exit_code", Val::N(exit_code as u64)),
        ("verified", Val::B(verified)),
        ("errors", Val::S(errors_to_json_array(&errors))),
    ];
    (exit_code, fields)
}

fn unconsumed(record: &vh_sandbox::RunRecord) -> bool {
    record.transport.as_ref().is_some_and(|t| t.unconsumed > 0)
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
                eprintln!("error: unknown argument: {other}\n\n{usage}");
                return 2;
            }
        }
    }

    if workload != "cooperative-echo" {
        eprintln!(
            "error: unknown cooperative workload '{workload}' (expected cooperative-echo)\n\n{usage}"
        );
        return 2;
    }

    let cassette = match cassette_path {
        Some(path) => {
            let bytes = match std::fs::read(&path) {
                Ok(b) => b,
                Err(e) => {
                    eprintln!("error: cannot read cassette {path}: {e}\n\n{usage}");
                    return 2;
                }
            };
            match CassetteV2::parse(&bytes) {
                Ok(c) => Some(c),
                Err(e) => {
                    eprintln!("error: malformed cassette {path}: {e}\n\n{usage}");
                    return 2;
                }
            }
        }
        None => None,
    };

    let cassette_ref = cassette.as_ref();
    let campaign = match run_cooperative_campaign(cassette_ref, &workload) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("error: cooperative run failed: {e}");
            return 2;
        }
    };

    let (exit_code, fields) = outcome_fields(&campaign);
    let line = render_line(&fields);

    if let Some(out) = out_path {
        let out_dir = PathBuf::from(out);
        if out_dir.exists() {
            let mut entries = match std::fs::read_dir(&out_dir) {
                Ok(e) => e,
                Err(e) => {
                    eprintln!("error: cannot inspect --out {out_dir:?}: {e}");
                    return 2;
                }
            };
            if entries.next().is_some() {
                eprintln!("error: --out {out_dir:?} is not empty; refusing to write receipts into a non-empty directory");
                return 2;
            }
        } else {
            if let Err(e) = std::fs::create_dir_all(&out_dir) {
                eprintln!("error: cannot create --out {out_dir:?}: {e}");
                return 2;
            }
        }
        if let Err(e) = std::fs::write(out_dir.join("outcome.ndjson"), line.clone() + "\n") {
            eprintln!("error: cannot write outcome receipt: {e}");
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

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
        let source = stage_child_source(&root).unwrap();
        let spec = child_spec(cassette, &source).unwrap();
        let workspace = root.join("u0");
        place_child_source(&source, &workspace).unwrap();
        run_once_with_cassette(&spec, &workspace, cassette).unwrap()
    }

    #[test]
    fn cooperative_echo_clean_run_twice() {
        let _cassette = fixture_cassette();
        let campaign = run_cooperative_campaign(None, "echo-clean").unwrap();
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
}
