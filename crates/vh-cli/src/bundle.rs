//! Evidence-store boundary I/O (convergence C4, audit R4): write NDJSON
//! run receipts + self-contained finding bundles, and re-execute a
//! bundle standalone. This file is a declared deny-list exemption for
//! `std::fs` ONLY — the receipt CONTENT is built and parsed by the pure
//! `vh_cli::receipts` module; nothing here touches clocks, environment,
//! or randomness, so identical runs write identical bytes.

use std::fs;
use std::path::{Path, PathBuf};

use vh_cli::receipts::{
    palette_by_name, render_line, FindingBundle, Val, FINDING_BUNDLE_SCHEMA, RUN_RECEIPTS_SCHEMA,
};
use vh_cli::workloads;
use vh_multiverse::{run_universe_with_palette, MultiverseReport, UniverseResult};

/// Everything `write_run_receipts` needs beyond the report itself —
/// the CLI invocation identity that belongs in the manifest.
pub struct RunIdentity<'a> {
    pub palette_name: &'a str,
    pub universes_requested: u64,
    pub check_divergence: bool,
    pub verdict_label: &'a str,
}

fn finding_id(universe: u64, trace_hash: &str) -> String {
    let prefix: String = trace_hash.chars().take(12).collect();
    format!("u{universe}-{prefix}")
}

fn bundle_for(
    report: &MultiverseReport,
    universe: u64,
    id: &str,
    palette_name: &str,
) -> FindingBundle {
    let r = &report.results()[universe as usize];
    let contract_violations: Vec<String> = report
        .contract_violations()
        .iter()
        .filter(|(u, _)| *u == universe)
        .map(|(_, v)| v.clone())
        .collect();
    FindingBundle {
        finding_id: id.to_string(),
        workload: report.workload().to_string(),
        seed: report.root_seed(),
        palette: palette_name.to_string(),
        universe,
        trace_hash: r.trace_hash().to_string(),
        trace_events: r.trace_events() as u64,
        fault_plan_digest: r.fault_plan_digest().map(str::to_string),
        failures: r
            .always_failures()
            .iter()
            .map(|f| (f.name.clone(), f.detail.clone()))
            .collect(),
        contract_violations,
        invalid_completion: (!r.lifecycle().is_valid_completion())
            .then(|| format!("{:?}", r.lifecycle())),
    }
}

/// Universes that get a replay bundle: always-failing ∪ invalid ∪
/// contract-violating, deduplicated, ascending. Divergent universes are
/// recorded in `run.ndjson` but never bundled — a bundle is a replay
/// PROMISE, and divergence is the absence of that promise.
fn finding_universes(report: &MultiverseReport) -> Vec<u64> {
    let mut out: Vec<u64> = report
        .failing_universes()
        .into_iter()
        .chain(report.invalid_universes())
        .chain(report.contract_violations().iter().map(|(u, _)| *u))
        .collect();
    out.sort_unstable();
    out.dedup();
    let divergent = report.divergent_universes();
    out.retain(|u| !divergent.contains(u));
    out
}

/// Write `run.ndjson` + `findings/<id>/finding.ndjson` under `dir`.
/// Returns a one-line summary for stdout. Fails closed: any I/O error is
/// an error, never a silent partial receipt. A non-empty `dir` is
/// refused BEFORE any write (C3-honesty; PR #19 thread
/// PRRT_kwDOTdlCIM6S0Hr9): overwriting `run.ndjson` in place would leave
/// a prior run's `findings/<id>/` bundles behind as orphans the fresh
/// manifest no longer lists. Refusal only — existing contents are never
/// deleted, cleared, renamed, or replaced.
pub fn write_run_receipts(
    dir: &str,
    report: &MultiverseReport,
    id: &RunIdentity<'_>,
) -> Result<String, String> {
    let base = Path::new(dir);
    if base.exists() {
        let mut entries =
            fs::read_dir(base).map_err(|e| format!("cannot inspect --out {dir}: {e}"))?;
        if entries.next().is_some() {
            return Err(format!(
                "--out {dir} is not empty; refusing to write receipts into a non-empty \
                 directory (a prior run's findings/ bundles would survive as orphans \
                 the fresh manifest no longer lists) — point --out at a new or empty \
                 directory; existing contents were not touched"
            ));
        }
    }
    fs::create_dir_all(base).map_err(|e| format!("cannot create {dir}: {e}"))?;

    let findings = finding_universes(report);
    let mut lines: Vec<String> = Vec::with_capacity(report.results().len() + findings.len() + 1);
    lines.push(render_line(&[
        ("record", Val::S("manifest".into())),
        ("schema", Val::S(RUN_RECEIPTS_SCHEMA.into())),
        ("workload", Val::S(report.workload().to_string())),
        ("seed", Val::S(format!("0x{:x}", report.root_seed()))),
        ("universes", Val::N(id.universes_requested)),
        ("palette", Val::S(id.palette_name.to_string())),
        ("divergence_check", Val::B(id.check_divergence)),
        ("verdict", Val::S(id.verdict_label.to_string())),
        ("findings", Val::N(findings.len() as u64)),
        (
            "divergent",
            Val::N(report.divergent_universes().len() as u64),
        ),
        (
            "sometimes_unreached",
            Val::N(report.merged().unreached_sometimes().len() as u64),
        ),
    ]));
    let divergent = report.divergent_universes();
    for r in report.results() {
        let u = r.universe_id();
        lines.push(universe_line(r, divergent.contains(&u), &findings));
    }
    for &u in &findings {
        let fid = finding_id(u, report.results()[u as usize].trace_hash());
        let bundle = bundle_for(report, u, &fid, id.palette_name);
        let fdir = base.join("findings").join(&fid);
        fs::create_dir_all(&fdir).map_err(|e| format!("cannot create {}: {e}", fdir.display()))?;
        let fpath = fdir.join("finding.ndjson");
        fs::write(&fpath, bundle.to_ndjson())
            .map_err(|e| format!("cannot write {}: {e}", fpath.display()))?;
        lines.push(render_line(&[
            ("record", Val::S("finding".into())),
            ("finding_id", Val::S(fid.clone())),
            ("universe", Val::N(u)),
            ("path", Val::S(format!("findings/{fid}/finding.ndjson"))),
        ]));
    }
    let run_path = base.join("run.ndjson");
    fs::write(&run_path, lines.join("\n") + "\n")
        .map_err(|e| format!("cannot write {}: {e}", run_path.display()))?;
    Ok(format!(
        "receipts: {dir} ({} universes, {} finding bundle(s), {RUN_RECEIPTS_SCHEMA})",
        report.results().len(),
        findings.len()
    ))
}

fn universe_line(r: &UniverseResult, divergent: bool, findings: &[u64]) -> String {
    let u = r.universe_id();
    let mut fields = vec![
        ("record", Val::S("universe".into())),
        ("universe", Val::N(u)),
        ("trace_hash", Val::S(r.trace_hash().to_string())),
        ("trace_events", Val::N(r.trace_events() as u64)),
        (
            "fault_plan_digest",
            match r.fault_plan_digest() {
                Some(d) => Val::S(d.to_string()),
                None => Val::Null,
            },
        ),
        ("valid", Val::B(r.lifecycle().is_valid_completion())),
        ("divergent", Val::B(divergent)),
        ("always_failures", Val::N(r.always_failures().len() as u64)),
    ];
    if findings.contains(&u) {
        fields.push(("finding_id", Val::S(finding_id(u, r.trace_hash()))));
    }
    render_line(&fields)
}

/// `vh replay-bundle PATH`: re-execute a finding bundle with no other
/// repo state and verify the EXACT recorded identity. Exit contract:
/// 0 = reproduced byte-exactly (trace hash, event count, every failure
/// name+detail, contract violations, lifecycle validity, plan digest);
/// 1 = executed but did NOT reproduce the recorded finding (divergence
/// from the bundle — each differing observable is printed);
/// 2 = usage / unreadable / malformed bundle / unknown workload.
pub fn cmd_replay_bundle(args: &[String], usage: &str) -> i32 {
    let path = match args {
        [p] => PathBuf::from(p),
        _ => {
            eprintln!("error: replay-bundle takes exactly one PATH\n\n{usage}");
            return 2;
        }
    };
    let file = if path.is_dir() {
        path.join("finding.ndjson")
    } else {
        path
    };
    let text = match fs::read_to_string(&file) {
        Ok(t) => t,
        Err(e) => {
            eprintln!("error: cannot read {}: {e}", file.display());
            return 2;
        }
    };
    let bundle = match FindingBundle::parse(&text) {
        Ok(b) => b,
        Err(e) => {
            eprintln!("error: malformed bundle {}: {e}", file.display());
            return 2;
        }
    };
    let workload = match workloads::by_name(&bundle.workload) {
        Some(w) => w,
        None => {
            eprintln!(
                "error: bundle names unknown workload {:?} (this build cannot replay it)",
                bundle.workload
            );
            return 2;
        }
    };
    let palette = match palette_by_name(&bundle.palette) {
        Ok(p) => p,
        Err(e) => {
            eprintln!("error: {e}");
            return 2;
        }
    };

    // Run-twice honesty before comparing to the recorded identity: one
    // execution that disagrees with ITSELF can neither confirm nor deny
    // the bundle.
    let a = run_universe_with_palette(bundle.seed, bundle.universe, workload.as_ref(), palette);
    let b = run_universe_with_palette(bundle.seed, bundle.universe, workload.as_ref(), palette);
    if !a.observably_equal(&b) {
        println!(
            "replay-bundle: DIVERGENT — two replays of (seed 0x{:x}, universe {}) disagree; nothing can be verified",
            bundle.seed, bundle.universe
        );
        return 1;
    }

    let mut mismatches: Vec<String> = Vec::new();
    if a.trace_hash() != bundle.trace_hash {
        mismatches.push(format!(
            "trace_hash: got {}, bundle {}",
            a.trace_hash(),
            bundle.trace_hash
        ));
    }
    if a.trace_events() as u64 != bundle.trace_events {
        mismatches.push(format!(
            "trace_events: got {}, bundle {}",
            a.trace_events(),
            bundle.trace_events
        ));
    }
    if a.fault_plan_digest().map(str::to_string) != bundle.fault_plan_digest {
        mismatches.push(format!(
            "fault_plan_digest: got {:?}, bundle {:?}",
            a.fault_plan_digest(),
            bundle.fault_plan_digest
        ));
    }
    let got_failures: Vec<(String, String)> = a
        .always_failures()
        .iter()
        .map(|f| (f.name.clone(), f.detail.clone()))
        .collect();
    if got_failures != bundle.failures {
        mismatches.push(format!(
            "failures: got {got_failures:?}, bundle {:?}",
            bundle.failures
        ));
    }
    let got_contract = workload.property_contract().violations(&a);
    if got_contract != bundle.contract_violations {
        mismatches.push(format!(
            "contract_violations: got {got_contract:?}, bundle {:?}",
            bundle.contract_violations
        ));
    }
    let got_invalid =
        (!a.lifecycle().is_valid_completion()).then(|| format!("{:?}", a.lifecycle()));
    if got_invalid != bundle.invalid_completion {
        mismatches.push(format!(
            "invalid_completion: got {got_invalid:?}, bundle {:?}",
            bundle.invalid_completion
        ));
    }
    // A bundle records a FINDING; replaying to a finding-free universe is
    // a mismatch even if the bundle was (malformed-ly) finding-free too.
    if bundle.failures.is_empty()
        && bundle.contract_violations.is_empty()
        && bundle.invalid_completion.is_none()
    {
        mismatches.push("bundle records no finding — nothing to reproduce".into());
    }

    if mismatches.is_empty() {
        println!(
            "replay-bundle: REPRODUCED {} (workload {} seed 0x{:x} universe {} hash {} events {} {FINDING_BUNDLE_SCHEMA})",
            bundle.finding_id,
            bundle.workload,
            bundle.seed,
            bundle.universe,
            bundle.trace_hash,
            bundle.trace_events
        );
        0
    } else {
        println!(
            "replay-bundle: MISMATCH {} — the recorded finding did not reproduce:",
            bundle.finding_id
        );
        for m in &mismatches {
            println!("  {m}");
        }
        1
    }
}
