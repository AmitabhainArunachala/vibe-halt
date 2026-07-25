//! C6 — measured cassette-backed D2 reach campaign (post-audit
//! controller §11).
//!
//! Boundary file (host tempdirs + subprocess fixtures), same as
//! `sandbox_demo.rs`. Three commands behind `vh sandbox-campaign`:
//!
//! * the REFERENCE CAMPAIGN: N run-twice pairs of the C5 child-visible
//!   cooperative CPython fixture, each pair in fresh workspaces,
//!   publishing RAW divergence counts (numerator/denominator over the
//!   declared suite — never a rate without its counts), the full open-
//!   channel inventory, and aggregate transport receipts;
//! * the LEAK BATTERY: direct probes against the channels the safe
//!   runner cannot close. A probed channel reports its measured result;
//!   an unprobed channel reports UNPROBED/UNCHECKED. No path exists to
//!   a false CLEAN: every channel stays `Open` on the sealed receipt and
//!   the battery never claims closure.
//! * RECEIPT REPLAY: re-run one pair of the reference harness FROM a
//!   previously written campaign receipt (the harness travels inside
//!   the content-addressed receipt: fixture source + cassette bytes).
//!
//! Claim boundary: everything here is Tier-2 / D2. A clean campaign
//! means the cooperative fixture reproduced under run-twice sampling
//! with every channel still open — evidence, not proof, and never D1.
//! Criterion-4 live fire (public checkouts) requires separate operator
//! target authority and is NOT exercised here.

use vh_sandbox::{
    run_once_with_cassette, run_twice, CassetteV2, DivergenceReport, RunRecord, SandboxCampaign,
    SandboxError, SandboxSpec,
};

use crate::sandbox_demo::{
    child_spec, fixture_cassette_v2, place_child_source, stage_child_source, PY_CHILD,
};

pub const CAMPAIGN_RECEIPT_SCHEMA: &str = "vh-sandbox-campaign-v1";
const DEFAULT_PAIRS: usize = 100;

pub(crate) fn cmd_sandbox_campaign(args: &[String], usage: &str) -> i32 {
    let mut mode = "reference".to_string();
    let mut pairs = DEFAULT_PAIRS;
    let mut out: Option<String> = None;
    let mut receipt: Option<String> = None;
    let mut it = args.iter();
    while let Some(arg) = it.next() {
        let mut value_for = |flag: &str| {
            it.next()
                .cloned()
                .ok_or_else(|| format!("{flag} requires a value"))
        };
        let parsed = match arg.as_str() {
            "--mode" => value_for("--mode").map(|v| mode = v),
            "--pairs" => value_for("--pairs").and_then(|v| {
                v.parse::<usize>()
                    .map(|n| pairs = n)
                    .map_err(|e| format!("bad --pairs: {e}"))
            }),
            "--out" => value_for("--out").map(|v| out = Some(v)),
            "--receipt" => value_for("--receipt").map(|v| receipt = Some(v)),
            other => Err(format!("unknown argument: {other}")),
        };
        if let Err(e) = parsed {
            eprintln!("error: {e}\n\n{usage}");
            return 2;
        }
    }
    match mode.as_str() {
        "reference" => run_reference_campaign(pairs, out.as_deref()),
        "leak-battery" => run_leak_battery(),
        "replay" => match receipt {
            Some(path) => replay_from_receipt(&path),
            None => {
                eprintln!("error: --mode replay requires --receipt FILE\n\n{usage}");
                2
            }
        },
        other => {
            eprintln!(
                "error: unknown sandbox-campaign mode {other:?} (expected reference, \
                 leak-battery, or replay)"
            );
            2
        }
    }
}

fn campaign_root(label: &str) -> Result<std::path::PathBuf, SandboxError> {
    let p = std::env::temp_dir().join(format!("vh-sandbox-campaign-{label}"));
    let _ = std::fs::remove_dir_all(&p);
    std::fs::create_dir_all(&p)?;
    Ok(p)
}

fn hex_encode(bytes: &[u8]) -> String {
    vh_digest::hex(bytes)
}

fn hex_decode(s: &str) -> Result<Vec<u8>, String> {
    if !s.len().is_multiple_of(2) {
        return Err("odd hex length".into());
    }
    (0..s.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&s[i..i + 2], 16).map_err(|e| format!("bad hex: {e}")))
        .collect()
}

/// One pair of the reference harness: fresh workspaces, child source
/// placed, both runs through the child-visible cassette transport.
fn run_reference_pair(
    root: &std::path::Path,
    source: &std::path::Path,
    spec: &SandboxSpec,
    cassette: &CassetteV2,
    pair: usize,
) -> Result<SandboxCampaign, SandboxError> {
    let a = root.join(format!("p{pair}-a"));
    let b = root.join(format!("p{pair}-b"));
    place_child_source(source, &a)?;
    place_child_source(source, &b)?;
    Ok(SandboxCampaign {
        first: run_once_with_cassette(spec, &a, cassette)?,
        second: run_once_with_cassette(spec, &b, cassette)?,
    })
}

fn transport_summary(record: &RunRecord) -> (usize, u64, bool) {
    match &record.transport {
        Some(t) => (t.served.len(), t.unconsumed, t.tainted()),
        None => (0, 0, false),
    }
}

fn run_reference_campaign(pairs: usize, out: Option<&str>) -> i32 {
    if pairs == 0 {
        eprintln!("error: --pairs must be nonzero — zero work is never certified");
        return 2;
    }
    println!("vibe-halt sandbox-campaign: mode=reference pairs={pairs}");
    let cassette = fixture_cassette_v2(4, false);
    let (root, source, spec) = match (|| {
        let root = campaign_root("reference")?;
        // The fixture is staged at ONE fixed path shared with replay
        // mode: the spec identity binds (path, content digest), so the
        // receipt-replayed harness must stage at the identical path to
        // reproduce the identical identity.
        let fixture_dir = campaign_root("fixture")?;
        let source = stage_child_source(&fixture_dir, "campaign")?;
        let spec = child_spec(&cassette, &source)?;
        Ok::<_, SandboxError>((root, source, spec))
    })() {
        Ok(v) => v,
        Err(e) => {
            println!("  FAIL campaign setup: {e}");
            println!("  verdict: FINDINGS");
            return 1;
        }
    };

    let mut identity_pairs: Vec<(String, String)> = Vec::new();
    let mut tainted_runs = 0usize;
    let mut first_record: Option<RunRecord> = None;
    for pair in 0..pairs {
        match run_reference_pair(&root, &source, &spec, &cassette, pair) {
            Err(e) => {
                println!("  FAIL pair {pair}: {e}");
                println!("  verdict: FINDINGS");
                return 1;
            }
            Ok(campaign) => {
                for record in [&campaign.first, &campaign.second] {
                    if record.transport_tainted() {
                        tainted_runs += 1;
                    }
                }
                identity_pairs.push((campaign.first.identity(), campaign.second.identity()));
                if first_record.is_none() {
                    first_record = Some(campaign.first.clone());
                }
            }
        }
    }
    let report = DivergenceReport::from_identity_pairs(
        identity_pairs.iter().map(|(a, b)| (a.as_str(), b.as_str())),
    );
    let reference = first_record.expect("pairs >= 1");
    let open_channels = reference.capability.open_channels().len();
    println!(
        "  suite: fixture={} cassette={}",
        spec.identity(),
        cassette.identity()
    );
    println!(
        "  pairs: diverged={}/{} (raw counts over the declared suite; run-twice sampling, not proof)",
        report.diverged, report.sample
    );
    println!(
        "  channels: open={open_channels} closed=0 (sealed receipt; nothing in this package closes a channel)"
    );
    println!(
        "  transport: tainted-runs={tainted_runs} (per-run served/unconsumed bound into each identity)"
    );
    if let Some(path) = out {
        match write_campaign_receipt(path, &spec, &cassette, &identity_pairs, &report) {
            Ok(summary) => println!("  {summary}"),
            Err(e) => {
                println!("  FAIL receipt: {e}");
                println!("  verdict: FINDINGS");
                return 1;
            }
        }
    }
    if tainted_runs > 0 {
        println!("  verdict: UNCHECKED (transport taint in {tainted_runs} run(s))");
        3
    } else if report.diverged == 0 {
        println!("  verdict: CLEAN (Tier-2 D2 — every channel remains open; cassette-backed cooperative profile)");
        0
    } else {
        println!(
            "  DIVERGENT sandbox reference campaign: {}/{} pairs differ",
            report.diverged, report.sample
        );
        println!("  verdict: FINDINGS");
        1
    }
}

/// Content-addressed campaign receipt: the declared suite (fixture
/// source + cassette bytes embedded), per-pair identities, raw counts,
/// and a trailing SHA-256 over the exact preceding bytes — the same
/// digest law as evidence v2. The harness is RUNNABLE from this file
/// (`--mode replay --receipt FILE`).
fn write_campaign_receipt(
    path: &str,
    spec: &SandboxSpec,
    cassette: &CassetteV2,
    identity_pairs: &[(String, String)],
    report: &DivergenceReport,
) -> Result<String, String> {
    let mut body = String::new();
    body.push_str(&format!(
        "{CAMPAIGN_RECEIPT_SCHEMA} pairs={} diverged={}\n",
        report.sample, report.diverged
    ));
    body.push_str(&format!("spec {}\n", spec.identity()));
    body.push_str(&format!("cassette-identity {}\n", cassette.identity()));
    body.push_str(&format!(
        "cassette-bytes {}\n",
        hex_encode(&cassette.file_bytes())
    ));
    body.push_str(&format!(
        "fixture-source {}\n",
        hex_encode(PY_CHILD.replace("%MODE%", "campaign").as_bytes())
    ));
    for (a, b) in identity_pairs {
        body.push_str(&format!("pair {a} {b}\n"));
    }
    let digest = vh_digest::sha256_hex(body.as_bytes());
    body.push_str(&format!("digest sha256 {digest}\n"));
    std::fs::write(path, &body).map_err(|e| format!("cannot write {path}: {e}"))?;
    Ok(format!(
        "receipt: {path} ({CAMPAIGN_RECEIPT_SCHEMA}, content sha256 {digest})"
    ))
}

/// Re-run ONE pair of the harness from a campaign receipt. Strict: the
/// receipt's own content digest must recompute; the embedded cassette
/// must match its recorded identity; the fresh pair must reproduce the
/// receipt's first recorded pair identities exactly.
fn replay_from_receipt(path: &str) -> i32 {
    println!("vibe-halt sandbox-campaign: mode=replay receipt={path}");
    let text = match std::fs::read_to_string(path) {
        Ok(t) => t,
        Err(e) => {
            eprintln!("error: cannot read {path}: {e}");
            return 2;
        }
    };
    let parsed = match parse_receipt(&text) {
        Ok(p) => p,
        Err(e) => {
            eprintln!("error: malformed campaign receipt: {e}");
            return 2;
        }
    };
    let (root, source_path) = match (|| {
        let root = campaign_root("replay")?;
        // Same fixed staging path as the reference campaign — the spec
        // identity binds (path, content digest), and the content now
        // comes from the receipt.
        let fixture_dir = campaign_root("fixture")?;
        let source_path = fixture_dir.join("agent_fixture.py");
        std::fs::write(&source_path, &parsed.fixture_source)?;
        Ok::<_, SandboxError>((root, source_path))
    })() {
        Ok(v) => v,
        Err(e) => {
            println!("  FAIL replay setup: {e}");
            return 1;
        }
    };
    let spec = match child_spec(&parsed.cassette, &source_path) {
        Ok(s) => s,
        Err(e) => {
            println!("  FAIL replay spec: {e}");
            return 1;
        }
    };
    match run_reference_pair(&root, &source_path, &spec, &parsed.cassette, 0) {
        Err(e) => {
            println!("  FAIL replay pair: {e}");
            1
        }
        Ok(campaign) => {
            let got = (campaign.first.identity(), campaign.second.identity());
            let (served, unconsumed, tainted) = transport_summary(&campaign.first);
            println!("  transport: served={served} unconsumed={unconsumed} tainted={tainted}");
            if tainted || campaign.second.transport_tainted() {
                println!("  verdict: UNCHECKED (transport taint on replay)");
                return 3;
            }
            if got == parsed.first_pair {
                println!("  replay: REPRODUCED pair identities {} / {}", got.0, got.1);
                println!("  verdict: CLEAN (Tier-2 D2; harness ran from the receipt alone)");
                0
            } else {
                println!(
                    "  replay: MISMATCH got ({}, {}), receipt ({}, {})",
                    got.0, got.1, parsed.first_pair.0, parsed.first_pair.1
                );
                println!("  verdict: FINDINGS");
                1
            }
        }
    }
}

#[derive(Debug)]
struct ParsedReceipt {
    cassette: CassetteV2,
    fixture_source: Vec<u8>,
    first_pair: (String, String),
}

fn parse_receipt(text: &str) -> Result<ParsedReceipt, String> {
    let lines: Vec<&str> = text.lines().collect();
    let digest_line = lines.last().ok_or("empty receipt")?;
    let recorded = digest_line
        .strip_prefix("digest sha256 ")
        .ok_or("missing digest trailer")?;
    let body_end = text.rfind("digest sha256 ").expect("found above");
    let recomputed = vh_digest::sha256_hex(&text.as_bytes()[..body_end]);
    if recorded != recomputed {
        return Err(format!(
            "content digest mismatch: recorded {recorded}, recomputed {recomputed}"
        ));
    }
    let head = lines.first().ok_or("empty receipt")?;
    if !head.starts_with(&format!("{CAMPAIGN_RECEIPT_SCHEMA} ")) {
        return Err(format!("unsupported receipt head {head:?}"));
    }
    let mut cassette_identity = None;
    let mut cassette_bytes = None;
    let mut fixture_source = None;
    let mut first_pair = None;
    for line in &lines[1..lines.len() - 1] {
        if let Some(v) = line.strip_prefix("cassette-identity ") {
            cassette_identity = Some(v.to_string());
        } else if let Some(v) = line.strip_prefix("cassette-bytes ") {
            cassette_bytes = Some(hex_decode(v)?);
        } else if let Some(v) = line.strip_prefix("fixture-source ") {
            fixture_source = Some(hex_decode(v)?);
        } else if let Some(v) = line.strip_prefix("pair ") {
            if first_pair.is_none() {
                let mut parts = v.split(' ');
                let a = parts.next().ok_or("pair line missing first identity")?;
                let b = parts.next().ok_or("pair line missing second identity")?;
                first_pair = Some((a.to_string(), b.to_string()));
            }
        }
    }
    let cassette = CassetteV2::parse(&cassette_bytes.ok_or("receipt missing cassette-bytes")?)
        .map_err(|e| format!("embedded cassette: {e}"))?;
    let recorded_identity = cassette_identity.ok_or("receipt missing cassette-identity")?;
    if cassette.identity() != recorded_identity {
        return Err(format!(
            "embedded cassette identity {} does not match recorded {recorded_identity}",
            cassette.identity()
        ));
    }
    Ok(ParsedReceipt {
        cassette,
        fixture_source: fixture_source.ok_or("receipt missing fixture-source")?,
        first_pair: first_pair.ok_or("receipt records no pair")?,
    })
}

// ---- leak battery ----

struct Probe {
    name: &'static str,
    channel: &'static str,
    /// Python program writing the probed observable into out.txt.
    source: &'static str,
    /// True when two runs MUST differ (the leak is per-process).
    must_diverge: bool,
}

const PROBES: &[Probe] = &[
    Probe {
        name: "wall-clock",
        channel: "wall/monotonic/cpu time",
        source: "import time\nopen('out.txt','w').write(str(time.time_ns()))\n",
        must_diverge: true,
    },
    Probe {
        name: "entropy",
        channel: "entropy devices / OS RNG",
        source: "import os\nopen('out.txt','w').write(os.urandom(16).hex())\n",
        must_diverge: true,
    },
    Probe {
        name: "pid-proc",
        channel: "PID/TID/proc identity",
        source: "import os\nopen('out.txt','w').write(str(os.getpid()))\n",
        must_diverge: true,
    },
    Probe {
        name: "aslr-address",
        channel: "ASLR/address output",
        source: "open('out.txt','w').write(hex(id(object())))\n",
        must_diverge: true,
    },
    Probe {
        name: "socket-attempt",
        channel: "real network/DNS",
        source: "import socket\ns=socket.socket()\ntry:\n    s.connect(('127.0.0.1', 9))\n    r='connected'\nexcept OSError as e:\n    r='refused:%s' % type(e).__name__\nopen('out.txt','w').write(r)\n",
        must_diverge: false,
    },
    Probe {
        name: "fs-escape",
        channel: "filesystem escape",
        source: "import os\nr=str(sorted(os.listdir(os.path.sep))[:3])\nopen('out.txt','w').write('escaped:'+r)\n",
        must_diverge: false,
    },
    Probe {
        name: "thread-spawn",
        channel: "threads/forks/descendants",
        source: "import threading\nout=[]\nt=threading.Thread(target=lambda: out.append('ran'))\nt.start()\nt.join()\nopen('out.txt','w').write(','.join(out))\n",
        must_diverge: false,
    },
];

/// Channels the battery names but does not probe in this package. They
/// remain `Open` on every sealed receipt; UNPROBED here means exactly
/// "no measurement exists", never closure.
const UNPROBED_CHANNELS: &[&str] = &[
    "inherited file descriptors",
    "signals/timers",
    "io_uring/IPC/shared memory",
    "JIT/GC/finalizer behavior",
];

fn run_leak_battery() -> i32 {
    println!("vibe-halt sandbox-campaign: mode=leak-battery");
    let root = match campaign_root("leak-battery") {
        Ok(r) => r,
        Err(e) => {
            println!("  FAIL battery setup: {e}");
            return 1;
        }
    };
    let mut failures = 0usize;
    for probe in PROBES {
        let outcome = (|| {
            let probe_root = root.join(probe.name);
            std::fs::create_dir_all(&probe_root)?;
            let source = probe_root.join("probe.py");
            std::fs::write(&source, probe.source)?;
            let spec = SandboxSpec::new(vec!["/usr/bin/python3".into(), "probe.py".into()])?
                .declare_artifact("out.txt")?
                .declare_input_file(&source)?;
            let a = probe_root.join("a");
            let b = probe_root.join("b");
            for ws in [&a, &b] {
                std::fs::create_dir_all(ws)?;
                std::fs::copy(&source, ws.join("probe.py"))?;
            }
            run_twice(&spec, &a, &b)
        })();
        match outcome {
            Err(e) => {
                println!("  probe {}: ERROR {e}", probe.name);
                failures += 1;
            }
            Ok(campaign) => {
                let diverged = campaign.divergence_report().diverged > 0;
                let result = if diverged { "DIVERGENT" } else { "AGREED" };
                println!(
                    "  probe {}: {result} channel=[{}] grade=D2-open (channel remains Open on the sealed receipt; agreement is never closure)",
                    probe.name, probe.channel
                );
                if probe.must_diverge && !diverged {
                    println!(
                        "  BATTERY FAIL: probe {} was expected to diverge across processes but agreed — investigate before trusting any campaign",
                        probe.name
                    );
                    failures += 1;
                }
            }
        }
    }
    for channel in UNPROBED_CHANNELS {
        println!("  unprobed [{channel}]: UNCHECKED (no measurement in this package; channel remains Open)");
    }
    if failures == 0 {
        println!(
            "  battery: PASS — no probe can produce false CLEAN; every channel stays Open and every unprobed channel stays UNCHECKED"
        );
        0
    } else {
        println!("  battery: {failures} failure(s)");
        1
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_receipt() -> String {
        let cassette = fixture_cassette_v2(4, false);
        let mut body = format!("{CAMPAIGN_RECEIPT_SCHEMA} pairs=1 diverged=0\n");
        body.push_str("spec dummy-spec-identity\n");
        body.push_str(&format!("cassette-identity {}\n", cassette.identity()));
        body.push_str(&format!(
            "cassette-bytes {}\n",
            hex_encode(&cassette.file_bytes())
        ));
        body.push_str(&format!(
            "fixture-source {}\n",
            hex_encode(PY_CHILD.replace("%MODE%", "campaign").as_bytes())
        ));
        body.push_str("pair aaaa bbbb\n");
        let digest = vh_digest::sha256_hex(body.as_bytes());
        body.push_str(&format!("digest sha256 {digest}\n"));
        body
    }

    #[test]
    fn receipt_roundtrips_and_binds_its_embedded_cassette() {
        let text = sample_receipt();
        let parsed = parse_receipt(&text).expect("valid receipt parses");
        assert_eq!(parsed.first_pair, ("aaaa".into(), "bbbb".into()));
        assert_eq!(parsed.cassette.len(), 4);
        assert!(!parsed.fixture_source.is_empty());
    }

    #[test]
    fn tampered_receipt_fails_the_content_digest() {
        let text = sample_receipt().replace("pair aaaa bbbb", "pair cccc dddd");
        let err = parse_receipt(&text).unwrap_err();
        assert!(err.contains("content digest mismatch"), "{err}");
    }

    #[test]
    fn cassette_identity_mismatch_is_rejected() {
        // Rebuild the digest around a corrupted cassette-identity line so
        // the failure is the identity binding, not the content digest.
        let mut body = sample_receipt();
        let body_end = body.rfind("digest sha256 ").unwrap();
        let mut head = body[..body_end].to_string();
        head = head.replacen(
            &format!(
                "cassette-identity {}",
                fixture_cassette_v2(4, false).identity()
            ),
            "cassette-identity vh-cassette-v2:sha256:deadbeef",
            1,
        );
        let digest = vh_digest::sha256_hex(head.as_bytes());
        body = format!("{head}digest sha256 {digest}\n");
        let err = parse_receipt(&body).unwrap_err();
        assert!(err.contains("does not match recorded"), "{err}");
    }

    #[test]
    fn hex_roundtrips() {
        let bytes = vec![0u8, 1, 255, 16, 128];
        assert_eq!(hex_decode(&hex_encode(&bytes)).unwrap(), bytes);
        assert!(hex_decode("abc").is_err());
        assert!(hex_decode("zz").is_err());
    }
}
