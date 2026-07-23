//! CLI smoke for the Tier-2 D2 subprocess sandbox MVP (D1 is a future
//! backend).
//!
//! This file is a boundary file: it uses host tempdirs and subprocess sandbox
//! fixtures to exercise D2-honest run-twice evidence, not Tier-1 identity.

use vh_sandbox::{
    run_once as sandbox_run_once, Cassette, CassetteEntry, LlmRequest, SandboxCampaign,
    SandboxError, SandboxSpec,
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
        _ => {
            eprintln!("error: unknown sandbox-demo mode '{mode}' (expected clean, cassette-miss, or nondet)");
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
