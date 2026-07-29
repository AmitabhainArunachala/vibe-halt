//! Evidence-store boundary I/O (convergence C4, audit R4; v2: post-audit
//! C3): write NDJSON run receipts + self-contained finding bundles, and
//! re-execute a bundle standalone. This file is a declared deny-list
//! exemption for `std::fs` ONLY — receipt CONTENT is built and parsed by
//! the pure `vh_cli::receipts` / `vh_cli::receipts_v2` modules; nothing
//! here touches clocks, environment, or randomness, so identical runs
//! write identical bytes.
//!
//! `--out` writes the v2 schema. v1 bundles remain replayable exactly as
//! before — labeled FIFO-only SELF-CONSISTENT replay, never
//! authenticated provenance.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use vh_cli::receipts::{json_escape, palette_by_name, parse_line, render_line, FindingBundle, Val};
use vh_cli::receipts_v2::{
    finding_id_v2, FindingBundleV2, Provenance, ShrinkLineage, FINDING_BUNDLE_SCHEMA_V2,
    RUN_RECEIPTS_SCHEMA_V2,
};
use vh_cli::shrink_cli::ShrinkOutcome;
use vh_cli::workloads;
use vh_gremlin::{FaultInjection, FaultKind, FaultPlan};
use vh_multiverse::{
    run_universe_with_fault_plan, run_universe_with_palette, MultiverseReport, SchedulePolicy,
    UniverseResult,
};

/// Everything `write_run_receipts` needs beyond the report itself —
/// the CLI invocation identity that belongs in the manifest.
pub struct RunIdentity<'a> {
    pub palette_name: &'a str,
    pub universes_requested: u64,
    pub check_divergence: bool,
    pub verdict_label: &'a str,
    pub provenance: &'a Provenance,
    /// Shrink lineage for its universe's bundle, when `--shrink` ran
    /// with `--out` (the PR #20 / audit D.3 debt: the minimized plan is
    /// persisted, bound, and replayable — not just printed).
    pub lineage: Option<&'a ShrinkOutcome>,
}

fn schedule_label(policy: SchedulePolicy) -> String {
    match policy {
        SchedulePolicy::Fifo => "fifo".to_string(),
        SchedulePolicy::Pct { depth } => format!("pct:{depth}"),
        SchedulePolicy::UniformTiebreak => "uniform".to_string(),
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

/// Build the shrink-lineage record block for a universe, replaying the
/// minimized plan (twice — single-replay law) to record its OWN identity
/// so `replay-bundle` later consumes the minimized plan rather than
/// regenerating the original.
fn lineage_for(
    outcome: &ShrinkOutcome,
    workload: &dyn vh_multiverse::Workload,
) -> Result<ShrinkLineage, String> {
    let plan = outcome.minimized_plan.clone();
    let a = run_universe_with_fault_plan(outcome.seed, outcome.universe, workload, plan.clone());
    let b = run_universe_with_fault_plan(outcome.seed, outcome.universe, workload, plan);
    if !a.observably_equal(&b) {
        return Err(format!(
            "minimized-plan replay of universe {} is not self-consistent; refusing to record lineage",
            outcome.universe
        ));
    }
    let minimized_digest = a
        .fault_plan_digest()
        .ok_or("minimized-plan replay retrieved no plan — lineage cannot bind")?
        .to_string();
    let original_digest = outcome
        .baseline_plan_digest
        .clone()
        .ok_or("shrink baseline recorded no plan digest — lineage cannot bind")?;
    Ok(ShrinkLineage {
        original_digest,
        original_injections: outcome.original_injections as u64,
        minimized_digest,
        minimized_observation_sha256: vh_digest::sha256_hex(
            a.complete_observation_identity().canonical_bytes(),
        ),
        oracle_calls: outcome.oracle_calls as u64,
        distinct_candidates: outcome.distinct_candidates as u64,
        minimized_plan: outcome
            .minimized_plan
            .injections()
            .iter()
            .map(|inj| (inj.at_nanos, inj.fault.canonical()))
            .collect(),
        minimized_failures: a
            .always_failures()
            .iter()
            .map(|f| (f.name.clone(), f.detail.clone()))
            .collect(),
    })
}

fn bundle_v2_for(
    report: &MultiverseReport,
    universe: u64,
    palette_name: &str,
    provenance: &Provenance,
    lineage: Option<ShrinkLineage>,
) -> FindingBundleV2 {
    let r = &report.results()[universe as usize];
    // Destructured WITHOUT `..` on purpose: when the observation grows a
    // field, this stops compiling until the v2 schema decides how the
    // new observable is persisted (the C1 ratchet, carried into
    // evidence).
    let vh_multiverse::UniverseObservation {
        universe_id: _,
        trace_hash,
        trace_events,
        always_checks,
        always_failures,
        sometimes,
        lifecycle,
        fault_plan_digest,
        runtime_evidence: _, // covered injectively by the complete-observation identity
        schedule_policy,
        decision_tape_digest,
        end_state_identity,
        complete_observation_identity,
    } = r.observation();
    let observation_sha256 = vh_digest::sha256_hex(complete_observation_identity.canonical_bytes());
    let contract_violations: Vec<String> = report
        .contract_violations()
        .iter()
        .filter(|(u, _)| *u == universe)
        .map(|(_, v)| v.clone())
        .collect();
    FindingBundleV2 {
        finding_id: finding_id_v2(universe, &observation_sha256),
        workload: report.workload().to_string(),
        seed: report.root_seed(),
        universe,
        palette: palette_name.to_string(),
        schedule_policy: schedule_label(schedule_policy),
        tape_digest: decision_tape_digest.map(str::to_string),
        trace_hash: trace_hash.to_string(),
        trace_events: trace_events as u64,
        fault_plan_digest: fault_plan_digest.map(str::to_string),
        end_state_sha256: vh_digest::sha256_hex(end_state_identity.canonical_bytes()),
        observation_sha256,
        provenance: provenance.clone(),
        checks: always_checks
            .iter()
            .map(|c| (c.name.clone(), c.passed))
            .collect(),
        failures: always_failures
            .iter()
            .map(|f| (f.name.clone(), f.detail.clone()))
            .collect(),
        sometimes: sometimes.iter().map(|(k, v)| (k.clone(), *v)).collect(),
        contract_violations,
        invalid_completion: (!lifecycle.is_valid_completion()).then(|| format!("{lifecycle:?}")),
        lineage,
    }
}

/// Write `run.ndjson` + `findings/<id>/finding.ndjson` under `dir` (v2).
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

    // Shrink lineage (if any) binds to its exact universe's bundle.
    let lineage_universe = id.lineage.map(|o| o.universe);
    let lineage = match id.lineage {
        None => None,
        Some(outcome) => {
            let workload = workloads::by_name(&outcome.workload)
                .ok_or_else(|| format!("unknown workload {:?} for lineage", outcome.workload))?;
            Some(lineage_for(outcome, workload.as_ref())?)
        }
    };

    let findings = finding_universes(report);
    let schedule = report
        .results()
        .first()
        .map(|r| schedule_label(r.schedule_policy()))
        .unwrap_or_else(|| "fifo".to_string());
    let p = id.provenance;
    let mut lines: Vec<String> = Vec::with_capacity(report.results().len() + findings.len() + 2);
    lines.push(render_line(&[
        ("record", Val::S("manifest".into())),
        ("schema", Val::S(RUN_RECEIPTS_SCHEMA_V2.into())),
        ("workload", Val::S(report.workload().to_string())),
        ("seed", Val::S(format!("0x{:x}", report.root_seed()))),
        ("universes", Val::N(id.universes_requested)),
        ("palette", Val::S(id.palette_name.to_string())),
        ("schedule_policy", Val::S(schedule)),
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
        ("cli_version", Val::S(p.cli_version.clone())),
        ("build_profile", Val::S(p.build_profile.clone())),
        ("target_os", Val::S(p.target_os.clone())),
        ("target_arch", Val::S(p.target_arch.clone())),
        (
            "declared_source_commit",
            match &p.declared_source_commit {
                Some(s) => Val::S(s.clone()),
                None => Val::Null,
            },
        ),
    ]));
    let divergent = report.divergent_universes();
    for r in report.results() {
        let u = r.universe_id();
        lines.push(universe_line(r, divergent.contains(&u), &findings));
    }
    for &u in &findings {
        let this_lineage = (lineage_universe == Some(u))
            .then(|| lineage.clone())
            .flatten();
        let bundle = bundle_v2_for(report, u, id.palette_name, id.provenance, this_lineage);
        let fid = bundle.finding_id.clone();
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
    // run.ndjson carries its own trailing content digest, same law as
    // bundles: the manifest is content-addressed too.
    let mut body = lines.join("\n") + "\n";
    let digest = vh_digest::sha256_hex(body.as_bytes());
    body.push_str(&render_line(&[
        ("record", Val::S("digest".into())),
        ("alg", Val::S(vh_digest::ALGORITHM.into())),
        ("value", Val::S(digest)),
    ]));
    body.push('\n');
    let run_path = base.join("run.ndjson");
    fs::write(&run_path, body).map_err(|e| format!("cannot write {}: {e}", run_path.display()))?;
    Ok(format!(
        "receipts: {dir} ({} universes, {} finding bundle(s), {RUN_RECEIPTS_SCHEMA_V2})",
        report.results().len(),
        findings.len()
    ))
}

fn universe_line(r: &UniverseResult, divergent: bool, findings: &[u64]) -> String {
    let u = r.universe_id();
    let observation_sha256 =
        vh_digest::sha256_hex(r.complete_observation_identity().canonical_bytes());
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
        (
            "end_state_sha256",
            Val::S(vh_digest::sha256_hex(
                r.end_state_identity().canonical_bytes(),
            )),
        ),
        ("observation_sha256", Val::S(observation_sha256.clone())),
        ("valid", Val::B(r.lifecycle().is_valid_completion())),
        ("divergent", Val::B(divergent)),
        ("always_failures", Val::N(r.always_failures().len() as u64)),
    ];
    if findings.contains(&u) {
        fields.push(("finding_id", Val::S(finding_id_v2(u, &observation_sha256))));
    }
    render_line(&fields)
}

/// `vh replay-bundle PATH`: re-execute a finding bundle with no other
/// repo state and verify the recorded identity. Dispatches on the
/// bundle's schema line: v2 verifies the COMPLETE observation (identity
/// digests included) plus any shrink lineage by consuming the minimized
/// plan; v1 remains supported within its explicit limitation (FIFO-only
/// self-consistent replay). Exit contract: 0 = reproduced; 1 = executed
/// but did not reproduce (or the bundle's own content digest fails);
/// 2 = usage / unreadable / malformed / unknown workload / unsupported
/// replay profile.
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
    match bundle_schema(&text) {
        Some(s) if s == FINDING_BUNDLE_SCHEMA_V2 => replay_v2(&text, &file),
        _ => replay_v1(&text, &file),
    }
}

/// Peek the schema of the first record without committing to a parser.
fn bundle_schema(text: &str) -> Option<String> {
    let first = text.lines().find(|l| !l.trim().is_empty())?;
    let fields = parse_line(first).ok()?;
    fields
        .iter()
        .find(|(k, _)| k == "schema")
        .and_then(|(_, v)| v.as_str())
        .map(str::to_string)
}

fn replay_v2(text: &str, file: &Path) -> i32 {
    let bundle = match FindingBundleV2::parse(text) {
        Ok(b) => b,
        Err(e) => {
            // A failed content digest is a failed reproduction claim (the
            // bytes are not the bytes that were recorded), not a usage
            // error — anchored MISMATCH, exit 1.
            if e.contains("content digest mismatch") {
                println!("replay-bundle: MISMATCH (content digest) — {e}");
                return 1;
            }
            eprintln!("error: malformed v2 bundle {}: {e}", file.display());
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
    // Today's writer emits bundles only for FIFO, untaped runs (the CLI
    // rejects --out with exploratory schedules). Fail closed on anything
    // else rather than replaying under the wrong profile.
    if bundle.schedule_policy != "fifo" || bundle.tape_digest.is_some() {
        eprintln!(
            "error: bundle records schedule_policy={:?} tape={:?} — this build replays \
             fifo/untaped v2 bundles only (UNCHECKED, not attempted)",
            bundle.schedule_policy,
            bundle.tape_digest.is_some()
        );
        return 2;
    }

    // Run-twice honesty before comparing to the recorded identity.
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
    compare_result_to_v2(&a, &bundle, &mut mismatches);

    // Shrink lineage: consume the MINIMIZED plan (never regenerate the
    // original) and hold it to its recorded identity.
    if let Some(lineage) = &bundle.lineage {
        match rebuild_plan(&lineage.minimized_plan) {
            Err(e) => mismatches.push(format!("lineage plan does not rebuild: {e}")),
            Ok(plan) => {
                let ma = run_universe_with_fault_plan(
                    bundle.seed,
                    bundle.universe,
                    workload.as_ref(),
                    plan.clone(),
                );
                let mb = run_universe_with_fault_plan(
                    bundle.seed,
                    bundle.universe,
                    workload.as_ref(),
                    plan,
                );
                if !ma.observably_equal(&mb) {
                    println!(
                        "replay-bundle: DIVERGENT — two minimized-plan replays disagree; nothing can be verified"
                    );
                    return 1;
                }
                match ma.fault_plan_digest() {
                    Some(d) if d == lineage.minimized_digest => {}
                    other => mismatches.push(format!(
                        "minimized plan digest: got {other:?}, lineage {:?} — the consumed \
                         plan is not the recorded minimized plan",
                        lineage.minimized_digest
                    )),
                }
                let got_obs =
                    vh_digest::sha256_hex(ma.complete_observation_identity().canonical_bytes());
                if got_obs != lineage.minimized_observation_sha256 {
                    mismatches.push(format!(
                        "minimized observation sha256: got {got_obs}, lineage {}",
                        lineage.minimized_observation_sha256
                    ));
                }
                let got_failures: Vec<(String, String)> = ma
                    .always_failures()
                    .iter()
                    .map(|f| (f.name.clone(), f.detail.clone()))
                    .collect();
                if got_failures != lineage.minimized_failures {
                    mismatches.push(format!(
                        "minimized failures: got {got_failures:?}, lineage {:?}",
                        lineage.minimized_failures
                    ));
                }
            }
        }
    }

    if mismatches.is_empty() {
        let lineage_note = match &bundle.lineage {
            Some(l) => format!(
                " lineage-minimized {} -> {} injection(s) consumed+verified;",
                l.original_injections,
                l.minimized_plan.len()
            ),
            None => String::new(),
        };
        println!(
            "replay-bundle: REPRODUCED {} (workload {} seed 0x{:x} universe {} observation {}…;{lineage_note} {FINDING_BUNDLE_SCHEMA_V2})",
            bundle.finding_id,
            bundle.workload,
            bundle.seed,
            bundle.universe,
            &bundle.observation_sha256[..12.min(bundle.observation_sha256.len())],
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

/// `vh verify-run --out DIR`: verify a `vh run --out` receipt directory.
/// Reads `DIR/run.ndjson`, recomputes its content digest, checks the
/// manifest schema/verdict, and re-executes every finding bundle via
/// `vh replay-bundle`. Emits exactly one NDJSON machine record
/// (`vh-verify-run-v1`) to stdout: exit 0 if every bundle reproduces and
/// the manifest is intact, exit 1 if any verification fails, exit 2 on
/// usage or unreadable input.
pub fn cmd_verify_run(args: &[String], usage: &str) -> i32 {
    let mut out_dir: Option<String> = None;
    let mut engine: Option<String> = None;
    let mut it = args.iter();
    while let Some(arg) = it.next() {
        match arg.as_str() {
            "--out" => match it.next() {
                Some(v) => out_dir = Some(v.clone()),
                None => {
                    eprintln!("error: --out requires a value\n\n{usage}");
                    return 2;
                }
            },
            "--engine" => match it.next() {
                Some(v) => engine = Some(v.clone()),
                None => {
                    eprintln!("error: --engine requires a value\n\n{usage}");
                    return 2;
                }
            },
            other => {
                eprintln!("error: unknown argument {other:?}\n\n{usage}");
                return 2;
            }
        }
    }
    let Some(dir) = out_dir else {
        eprintln!("error: verify-run requires --out DIR\n\n{usage}");
        return 2;
    };
    let Some(engine) = engine else {
        eprintln!("error: verify-run requires --engine PATH\n\n{usage}");
        return 2;
    };
    let dir_path = Path::new(&dir);
    let engine_path = Path::new(&engine);
    let run_path = dir_path.join("run.ndjson");
    let text = match fs::read_to_string(&run_path) {
        Ok(t) => t,
        Err(e) => {
            eprintln!("error: cannot read {}: {e}", run_path.display());
            return 2;
        }
    };

    let raw: Vec<&str> = text.split('\n').collect();
    let (tail, body_and_digest) = match raw.split_last() {
        Some(split) => split,
        None => {
            eprintln!("error: empty run receipt");
            return 2;
        }
    };
    if !tail.is_empty() {
        eprintln!("error: run receipt must end with a newline");
        return 2;
    }
    if body_and_digest.iter().any(|l| l.is_empty()) {
        eprintln!("error: blank line inside run receipt");
        return 2;
    }

    let digest_line = match body_and_digest.last() {
        Some(l) => *l,
        None => {
            eprintln!("error: empty run receipt");
            return 2;
        }
    };
    let body_lines = &body_and_digest[..body_and_digest.len() - 1];
    let mut body = String::new();
    for l in body_lines {
        body.push_str(l);
        body.push('\n');
    }

    let result_digest = vh_digest::sha256_hex(text.as_bytes());
    let mut errors: Vec<String> = Vec::new();

    // Content-digest check.
    let evidence_digest = match parse_line(digest_line) {
        Ok(rec) => {
            let kind = rec
                .iter()
                .find(|(k, _)| k == "record")
                .and_then(|(_, v)| v.as_str());
            if kind != Some("digest") {
                render_verify_run_record(
                    false,
                    "",
                    &result_digest,
                    "",
                    0,
                    0,
                    &["last record is not a digest record".into()],
                );
                return 1;
            }
            let recorded = match rec
                .iter()
                .find(|(k, _)| k == "value")
                .and_then(|(_, v)| v.as_str())
            {
                Some(v) => v,
                None => {
                    render_verify_run_record(
                        false,
                        "",
                        &result_digest,
                        "",
                        0,
                        0,
                        &["digest record missing value".into()],
                    );
                    return 1;
                }
            };
            let recomputed = vh_digest::sha256_hex(body.as_bytes());
            if recorded != recomputed {
                render_verify_run_record(
                    false,
                    "",
                    &result_digest,
                    "",
                    0,
                    0,
                    &[format!(
                        "content digest mismatch: recorded {recorded}, recomputed {recomputed}"
                    )],
                );
                return 1;
            }
            recomputed
        }
        Err(e) => {
            render_verify_run_record(
                false,
                "",
                &result_digest,
                "",
                0,
                0,
                &[format!("malformed digest record: {e}")],
            );
            return 1;
        }
    };

    // Manifest check.
    let manifest = match body_lines.first() {
        Some(line) => match parse_line(line) {
            Ok(rec) => rec,
            Err(e) => {
                render_verify_run_record(
                    false,
                    "",
                    &result_digest,
                    &evidence_digest,
                    0,
                    0,
                    &[format!("malformed manifest: {e}")],
                );
                return 1;
            }
        },
        None => {
            render_verify_run_record(
                false,
                "",
                &result_digest,
                &evidence_digest,
                0,
                0,
                &["no manifest record".into()],
            );
            return 1;
        }
    };
    let schema = manifest
        .iter()
        .find(|(k, _)| k == "schema")
        .and_then(|(_, v)| v.as_str());
    if schema != Some(RUN_RECEIPTS_SCHEMA_V2) {
        render_verify_run_record(
            false,
            "",
            &result_digest,
            &evidence_digest,
            0,
            0,
            &[format!("unsupported run receipt schema {schema:?}")],
        );
        return 1;
    }
    let verdict = manifest
        .iter()
        .find(|(k, _)| k == "verdict")
        .and_then(|(_, v)| v.as_str())
        .unwrap_or("")
        .to_string();
    let findings_total = manifest
        .iter()
        .find(|(k, _)| k == "findings")
        .and_then(|(_, v)| v.as_u64())
        .unwrap_or(0) as usize;

    // Collect finding records.
    let mut finding_paths: Vec<(String, String)> = Vec::new();
    for line in body_lines.iter().skip(1) {
        let rec = match parse_line(line) {
            Ok(r) => r,
            Err(e) => {
                errors.push(format!("malformed record: {e}"));
                continue;
            }
        };
        let record_type = rec
            .iter()
            .find(|(k, _)| k == "record")
            .and_then(|(_, v)| v.as_str());
        if record_type == Some("finding") {
            let id = rec
                .iter()
                .find(|(k, _)| k == "finding_id")
                .and_then(|(_, v)| v.as_str())
                .unwrap_or("unknown")
                .to_string();
            let path = rec
                .iter()
                .find(|(k, _)| k == "path")
                .and_then(|(_, v)| v.as_str())
                .unwrap_or("")
                .to_string();
            finding_paths.push((id, path));
        }
    }
    if finding_paths.len() != findings_total {
        errors.push(format!(
            "manifest claims {} finding(s) but receipt lists {}",
            findings_total,
            finding_paths.len()
        ));
    }

    // Re-execute each finding bundle by invoking the pinned vh binary.
    let mut verified_count = 0usize;
    for (id, path) in &finding_paths {
        let abs = dir_path.join(path);
        let output = match Command::new(engine_path)
            .arg("replay-bundle")
            .arg(&abs)
            .output()
        {
            Ok(o) => o,
            Err(e) => {
                errors.push(format!("finding {id}: cannot spawn replay-bundle: {e}"));
                continue;
            }
        };
        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            let stdout = String::from_utf8_lossy(&output.stdout);
            errors.push(format!(
                "finding {id}: replay-bundle failed (exit {}): stderr={stderr} stdout={stdout}",
                output.status.code().unwrap_or(-1)
            ));
            continue;
        }
        verified_count += 1;
    }

    let verified = errors.is_empty();
    let final_verdict = if verified {
        verdict.clone()
    } else {
        "ERROR".to_string()
    };
    render_verify_run_record(
        verified,
        &final_verdict,
        &result_digest,
        &evidence_digest,
        finding_paths.len(),
        verified_count,
        &errors,
    );
    if verified {
        0
    } else {
        1
    }
}

fn render_verify_run_record(
    verified: bool,
    verdict: &str,
    result_digest: &str,
    evidence_digest: &str,
    findings_total: usize,
    findings_verified: usize,
    errors: &[String],
) {
    let errors_json = if errors.is_empty() {
        "[]".to_string()
    } else {
        format!(
            "[{}]",
            errors
                .iter()
                .map(|e| format!("\"{}\"", json_escape(e)))
                .collect::<Vec<_>>()
                .join(",")
        )
    };
    println!(
        "{}",
        render_line(&[
            ("record", Val::S("verify-run".into())),
            ("schema", Val::S("vh-verify-run-v1".into())),
            ("verified", Val::B(verified)),
            ("verdict", Val::S(verdict.to_string())),
            ("evidence_digest", Val::S(evidence_digest.to_string())),
            ("result_digest", Val::S(result_digest.to_string())),
            ("findings_total", Val::N(findings_total as u64)),
            ("findings_verified", Val::N(findings_verified as u64)),
            ("errors", Val::S(errors_json)),
        ])
    );
}

fn compare_result_to_v2(a: &UniverseResult, bundle: &FindingBundleV2, out: &mut Vec<String>) {
    if a.trace_hash() != bundle.trace_hash {
        out.push(format!(
            "trace_hash: got {}, bundle {}",
            a.trace_hash(),
            bundle.trace_hash
        ));
    }
    if a.trace_events() as u64 != bundle.trace_events {
        out.push(format!(
            "trace_events: got {}, bundle {}",
            a.trace_events(),
            bundle.trace_events
        ));
    }
    if a.fault_plan_digest().map(str::to_string) != bundle.fault_plan_digest {
        out.push(format!(
            "fault_plan_digest: got {:?}, bundle {:?}",
            a.fault_plan_digest(),
            bundle.fault_plan_digest
        ));
    }
    let end_state = vh_digest::sha256_hex(a.end_state_identity().canonical_bytes());
    if end_state != bundle.end_state_sha256 {
        out.push(format!(
            "end_state_sha256: got {end_state}, bundle {}",
            bundle.end_state_sha256
        ));
    }
    let obs = vh_digest::sha256_hex(a.complete_observation_identity().canonical_bytes());
    if obs != bundle.observation_sha256 {
        out.push(format!(
            "observation_sha256: got {obs}, bundle {} — the COMPLETE observation differs",
            bundle.observation_sha256
        ));
    }
    let got_checks: Vec<(String, bool)> = a
        .always_checks()
        .iter()
        .map(|c| (c.name.clone(), c.passed))
        .collect();
    if got_checks != bundle.checks {
        out.push(format!(
            "assertion transcript: got {got_checks:?}, bundle {:?}",
            bundle.checks
        ));
    }
    let got_failures: Vec<(String, String)> = a
        .always_failures()
        .iter()
        .map(|f| (f.name.clone(), f.detail.clone()))
        .collect();
    if got_failures != bundle.failures {
        out.push(format!(
            "failures: got {got_failures:?}, bundle {:?}",
            bundle.failures
        ));
    }
    let got_sometimes: Vec<(String, bool)> =
        a.sometimes().iter().map(|(k, v)| (k.clone(), *v)).collect();
    if got_sometimes != bundle.sometimes {
        out.push(format!(
            "sometimes: got {got_sometimes:?}, bundle {:?}",
            bundle.sometimes
        ));
    }
    let got_invalid =
        (!a.lifecycle().is_valid_completion()).then(|| format!("{:?}", a.lifecycle()));
    if got_invalid != bundle.invalid_completion {
        out.push(format!(
            "invalid_completion: got {got_invalid:?}, bundle {:?}",
            bundle.invalid_completion
        ));
    }
}

fn rebuild_plan(records: &[(u64, String)]) -> Result<FaultPlan, String> {
    let mut injections = Vec::with_capacity(records.len());
    for (at_nanos, canonical) in records {
        injections.push(FaultInjection {
            at_nanos: *at_nanos,
            fault: FaultKind::parse_canonical(canonical)?,
        });
    }
    Ok(FaultPlan::new(injections))
}

/// The v1 replay path, unchanged in substance: self-consistent replay
/// within v1's explicit limitation (FIFO-only; no observation identity,
/// no lineage, no content digest).
fn replay_v1(text: &str, file: &Path) -> i32 {
    let bundle = match FindingBundle::parse(text) {
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
            "replay-bundle: REPRODUCED {} (workload {} seed 0x{:x} universe {} hash {} events {} vh-finding-bundle-v1; v1 = FIFO-only self-consistent replay, not authenticated provenance)",
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
