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

use vh_cli::receipts::{json_escape, palette_by_name, parse_line, render_line, FindingBundle, Val};
use vh_cli::receipts_v2::{
    exceeds_bundle_record_bound, finding_id_v2, FindingBundleV2, Provenance, ShrinkLineage,
    FINDING_BUNDLE_SCHEMA_V2, RUN_RECEIPTS_SCHEMA_V2,
};
use vh_cli::shrink_cli::ShrinkOutcome;
use vh_cli::workloads;
use vh_gremlin::{FaultInjection, FaultKind, FaultPlan};
use vh_multiverse::{
    run_multiverse_with_palette, run_universe_with_fault_plan, run_universe_with_palette,
    MultiverseConfig, MultiverseReport, SchedulePolicy, UniverseCount, UniverseResult, Verdict,
};

const MAX_RUN_RECEIPT_BYTES: u64 = 64 << 20;
const MAX_FINDING_BUNDLE_BYTES: u64 = 16 << 20;
const MAX_VERIFY_ENGINE_BYTES: u64 = 128 << 20;
/// Fresh semantic verification is intentionally narrower than the raw runner's
/// million-universe construction ceiling. This direct-CLI work bound prevents
/// a tiny self-digested manifest from buying unbounded replay CPU/RAM.
pub(crate) const MAX_VERIFY_UNIVERSES: u64 = 10_000;

fn checked_aggregate_bundle_bytes(current: u64, additional: u64) -> Result<u64, String> {
    let total = current
        .checked_add(additional)
        .ok_or_else(|| "aggregate finding bundle size overflow".to_string())?;
    if total > MAX_RUN_RECEIPT_BYTES {
        return Err("aggregate finding bundles exceed the verifier byte budget".into());
    }
    Ok(total)
}

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

/// Canonical, digest-free `run.ndjson` body. The writer and verifier share
/// this exact renderer so a verifier never blesses a looser dialect than the
/// producer emits.
fn run_receipt_body(report: &MultiverseReport, id: &RunIdentity<'_>, findings: &[u64]) -> String {
    let schedule = report
        .results()
        .first()
        .map(|r| schedule_label(r.schedule_policy()))
        .unwrap_or_else(|| "fifo".to_string());
    let p = id.provenance;
    let mut lines: Vec<String> = Vec::with_capacity(report.results().len() + findings.len() + 1);
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
    for result in report.results() {
        let universe = result.universe_id();
        lines.push(universe_line(
            result,
            divergent.contains(&universe),
            findings,
        ));
    }
    for &universe in findings {
        let result = &report.results()[universe as usize];
        let observation_sha256 =
            vh_digest::sha256_hex(result.complete_observation_identity().canonical_bytes());
        let finding_id = finding_id_v2(universe, &observation_sha256);
        lines.push(render_line(&[
            ("record", Val::S("finding".into())),
            ("finding_id", Val::S(finding_id.clone())),
            ("universe", Val::N(universe)),
            (
                "path",
                Val::S(format!("findings/{finding_id}/finding.ndjson")),
            ),
        ]));
    }
    lines.join("\n") + "\n"
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
    if id.universes_requested > MAX_VERIFY_UNIVERSES {
        return Err(format!(
            "--out universe count exceeds the receipt verification work bound ({MAX_VERIFY_UNIVERSES})"
        ));
    }
    let base = crate::cooperative::prepare_output_root(Path::new(dir)).map_err(|error| {
        if error.contains("not empty") {
            format!(
                "--out {dir} is not empty; refusing to write receipts into a non-empty \
                 directory (a prior run's findings/ bundles would survive as orphans \
                 the fresh manifest no longer lists) — point --out at a new or empty \
                 directory; existing contents were not touched"
            )
        } else {
            format!("--out {dir} refused: {error}")
        }
    })?;

    struct ReceiptLock(Option<PathBuf>);
    impl ReceiptLock {
        fn release(&mut self) -> Result<(), String> {
            let path = self
                .0
                .as_ref()
                .ok_or_else(|| "receipt writer lock was already released".to_string())?;
            // Publication includes removing the controller-owned marker. If
            // that cannot happen, strict verification is guaranteed to fail,
            // so the writer must not report success.
            fs::remove_dir(path)
                .map_err(|_| "cannot release exclusive receipt writer lock".to_string())?;
            self.0 = None;
            Ok(())
        }
    }
    impl Drop for ReceiptLock {
        fn drop(&mut self) {
            // Non-recursive removal cannot delete attacker-planted contents.
            // If the path was replaced, leave it behind so verification fails.
            if let Some(path) = &self.0 {
                let _ = fs::remove_dir(path);
            }
        }
    }
    let lock_path = base.join(".vh-receipt-lock");
    crate::cooperative::create_private_directory(&lock_path)
        .map_err(|_| "cannot reserve exclusive receipt writer".to_string())?;
    let mut receipt_lock = ReceiptLock(Some(lock_path));

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
    // Bound the COMPLETE run receipt before publishing any bundle bytes so a
    // writer never creates evidence its verifier is specified to reject.
    let mut body = run_receipt_body(report, id, &findings);
    let digest = vh_digest::sha256_hex(body.as_bytes());
    body.push_str(&render_line(&[
        ("record", Val::S("digest".into())),
        ("alg", Val::S(vh_digest::ALGORITHM.into())),
        ("value", Val::S(digest)),
    ]));
    body.push('\n');
    if body.len() as u64 > MAX_RUN_RECEIPT_BYTES {
        return Err(format!(
            "run receipt exceeds the {MAX_RUN_RECEIPT_BYTES}-byte verification bound"
        ));
    }
    if body
        .lines()
        .any(|line| line.len() > vh_cli::receipts::MAX_FLAT_RECORD_BYTES)
    {
        return Err("run receipt contains a record beyond the parser's flat-record bound".into());
    }
    // Render and validate the complete bundle set before publishing its first
    // byte. Writer and verifier therefore share the same aggregate byte cap.
    let mut rendered_bundles = Vec::with_capacity(findings.len());
    let mut aggregate_bundle_bytes = 0u64;
    for &u in &findings {
        let this_lineage = (lineage_universe == Some(u))
            .then(|| lineage.clone())
            .flatten();
        let bundle = bundle_v2_for(report, u, id.palette_name, id.provenance, this_lineage);
        let fid = bundle.finding_id.clone();
        let bundle_bytes = bundle.to_ndjson();
        if bundle_bytes.len() as u64 > MAX_FINDING_BUNDLE_BYTES {
            return Err(format!(
                "finding bundle exceeds the {MAX_FINDING_BUNDLE_BYTES}-byte verification bound"
            ));
        }
        if bundle_bytes
            .lines()
            .any(|line| line.len() > vh_cli::receipts::MAX_FLAT_RECORD_BYTES)
        {
            return Err(
                "finding bundle contains a record beyond the parser's flat-record bound".into(),
            );
        }
        if exceeds_bundle_record_bound(&bundle_bytes) {
            return Err("finding bundle exceeds the canonical record-count bound".into());
        }
        aggregate_bundle_bytes =
            checked_aggregate_bundle_bytes(aggregate_bundle_bytes, bundle_bytes.len() as u64)?;
        rendered_bundles.push((fid, bundle_bytes));
    }
    let findings_root = base.join("findings");
    if !rendered_bundles.is_empty() {
        crate::cooperative::create_private_directory(&findings_root)
            .map_err(|_| "cannot create exclusive findings directory".to_string())?;
    }
    for (fid, bundle_bytes) in rendered_bundles {
        let fdir = findings_root.join(&fid);
        crate::cooperative::create_private_directory(&fdir)
            .map_err(|_| format!("cannot create exclusive finding directory {fid}"))?;
        let fpath = fdir.join("finding.ndjson");
        crate::cooperative::write_new_file(&fpath, bundle_bytes.as_bytes())
            .map_err(|e| format!("cannot write {}: {e}", fpath.display()))?;
    }
    // run.ndjson carries its own trailing content digest, same law as
    // bundles: the manifest is content-addressed too.
    let run_path = base.join("run.ndjson");
    crate::cooperative::write_new_file(&run_path, body.as_bytes())
        .map_err(|e| format!("cannot write {}: {e}", run_path.display()))?;
    receipt_lock.release()?;
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
    let metadata = match fs::symlink_metadata(&path) {
        Ok(metadata) if !metadata.file_type().is_symlink() => metadata,
        _ => {
            eprintln!("error: replay path must be an existing no-link file or directory");
            return 2;
        }
    };
    let file = if metadata.is_dir() {
        path.join("finding.ndjson")
    } else if metadata.is_file() {
        path
    } else {
        eprintln!("error: replay path is not a regular file or directory");
        return 2;
    };
    let bytes = match vh_sandbox::read_bounded_file(&file, MAX_FINDING_BUNDLE_BYTES) {
        Ok(bytes) => bytes,
        Err(_) => {
            eprintln!("error: finding bundle boundary read refused");
            return 2;
        }
    };
    let text = match std::str::from_utf8(&bytes) {
        Ok(text) => text,
        Err(_) => {
            eprintln!("error: finding bundle is not UTF-8");
            return 2;
        }
    };
    match bundle_schema(text) {
        Some(s) if s == FINDING_BUNDLE_SCHEMA_V2 => replay_v2(text, &file),
        _ => replay_v1(text, &file),
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
                println!(
                    "replay-bundle: MISMATCH (content digest) — {}",
                    crate::cooperative::bounded_diagnostic(&e)
                );
                return 1;
            }
            let safe_file = crate::cooperative::bounded_diagnostic(&file.to_string_lossy());
            eprintln!(
                "error: malformed v2 bundle {}: {}",
                safe_file,
                crate::cooperative::bounded_diagnostic(&e)
            );
            return 2;
        }
    };
    let workload = match workloads::by_name(&bundle.workload) {
        Some(w) => w,
        None => {
            let safe_workload = crate::cooperative::bounded_diagnostic(&bundle.workload);
            eprintln!(
                "error: bundle names unknown workload {safe_workload:?} (this build cannot replay it)"
            );
            return 2;
        }
    };
    let palette = match palette_by_name(&bundle.palette) {
        Ok(p) => p,
        Err(e) => {
            eprintln!("error: {}", crate::cooperative::bounded_diagnostic(&e));
            return 2;
        }
    };
    // Today's writer emits bundles only for FIFO, untaped runs (the CLI
    // rejects --out with exploratory schedules). Fail closed on anything
    // else rather than replaying under the wrong profile.
    if bundle.schedule_policy != "fifo" || bundle.tape_digest.is_some() {
        let safe_schedule = crate::cooperative::bounded_diagnostic(&bundle.schedule_policy);
        eprintln!(
            "error: bundle records schedule_policy={:?} tape={:?} — this build replays \
             fifo/untaped v2 bundles only (UNCHECKED, not attempted)",
            safe_schedule,
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
        let safe_finding_id = crate::cooperative::bounded_diagnostic(&bundle.finding_id);
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
            safe_finding_id,
            bundle.workload,
            bundle.seed,
            bundle.universe,
            &bundle.observation_sha256[..12.min(bundle.observation_sha256.len())],
        );
        0
    } else {
        let safe_finding_id = crate::cooperative::bounded_diagnostic(&bundle.finding_id);
        println!(
            "replay-bundle: MISMATCH {} — the recorded finding did not reproduce:",
            safe_finding_id
        );
        for m in &mismatches {
            println!("  {}", crate::cooperative::bounded_diagnostic(m));
        }
        1
    }
}

#[derive(Debug)]
struct VerifyManifest {
    workload: String,
    seed: u64,
    universes: UniverseCount,
    palette: String,
    divergence_check: bool,
    provenance: Provenance,
}

const GENERIC_ENGINE_REQUEST_DOMAIN: &str = "vh-generic-engine-request-v1";
const TIER1_RUN_COMMAND_DOMAIN: &str = "vh-tier1-run-command-v1";
const TIER1_RUN_CONDITION_DOMAIN: &str = "vh-tier1-run-condition-v1";
const TIER1_ORACLE_CONTRACT_DOMAIN: &str = "vh-tier1-oracle-contract-v1";
const TIER1_WORKLOAD_TARGET_REVISION_DOMAIN: &str = "vh-tier1-workload-target-revision-v1";
const TIER1_VERIFICATION_RESULT_DOMAIN: &str = "vh-tier1-verification-result-v1";

fn digest_frame(bytes: &mut Vec<u8>, tag: &str, value: &[u8]) {
    bytes.extend_from_slice(tag.as_bytes());
    bytes.push(b' ');
    bytes.extend_from_slice(value.len().to_string().as_bytes());
    bytes.push(b':');
    bytes.extend_from_slice(value);
    bytes.push(b'\n');
}

fn framed_digest<'a>(
    domain: &str,
    fields: impl IntoIterator<Item = (&'a str, &'a [u8])>,
) -> String {
    let mut bytes = Vec::new();
    bytes.extend_from_slice(domain.as_bytes());
    bytes.push(b'\n');
    for (tag, value) in fields {
        digest_frame(&mut bytes, tag, value);
    }
    vh_digest::sha256_hex(&bytes)
}

fn is_canonical_sha256(value: &str) -> bool {
    value.len() == 64
        && value.bytes().all(|byte| byte.is_ascii_hexdigit())
        && !value.bytes().any(|byte| byte.is_ascii_uppercase())
}

fn condition_id(seed: u64, universes: u64, palette: &str, check_divergence: bool) -> String {
    let seed = format!("0x{seed:x}");
    let universes = universes.to_string();
    framed_digest(
        TIER1_RUN_CONDITION_DOMAIN,
        [
            ("seed", seed.as_bytes()),
            ("universes", universes.as_bytes()),
            ("palette", palette.as_bytes()),
            ("schedule", b"fifo" as &[u8]),
            (
                "divergence-check",
                if check_divergence { b"true" } else { b"false" },
            ),
        ],
    )
}

fn command_id(workload: &str, condition_id: &str) -> String {
    framed_digest(
        TIER1_RUN_COMMAND_DOMAIN,
        [
            ("workload", workload.as_bytes()),
            ("condition-id", condition_id.as_bytes()),
            ("record-tape", b"false" as &[u8]),
        ],
    )
}

fn oracle_contract_id(workload: &dyn vh_multiverse::Workload) -> String {
    let contract = workload.property_contract();
    let mut bytes = Vec::new();
    bytes.extend_from_slice(TIER1_ORACLE_CONTRACT_DOMAIN.as_bytes());
    bytes.push(b'\n');
    for (index, name) in contract.required_always().iter().enumerate() {
        digest_frame(&mut bytes, &format!("always-{index}"), name.as_bytes());
    }
    for (index, name) in contract.required_sometimes().iter().enumerate() {
        digest_frame(&mut bytes, &format!("sometimes-{index}"), name.as_bytes());
    }
    for (index, name) in contract.required_oracles().iter().enumerate() {
        digest_frame(&mut bytes, &format!("oracle-{index}"), name.as_bytes());
    }
    vh_digest::sha256_hex(&bytes)
}

/// An independently supplied, exact Tier-1 campaign request. The schedule is
/// deliberately constructed as FIFO-only: v2 receipts do not authenticate
/// exploratory scheduling or decision-tape profiles.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct RunExpectation {
    workload: String,
    seed: u64,
    universes: UniverseCount,
    palette: String,
    schedule_policy: SchedulePolicy,
    check_divergence: bool,
    command_id: String,
    condition_id: String,
    oracle_contract_id: String,
}

impl RunExpectation {
    pub(crate) fn fifo(
        workload: &str,
        seed: u64,
        universes: u64,
        palette: &str,
        check_divergence: bool,
    ) -> Result<Self, String> {
        if universes > MAX_VERIFY_UNIVERSES {
            return Err(format!(
                "expected universe count exceeds the verifier work bound ({MAX_VERIFY_UNIVERSES})"
            ));
        }
        let universes = UniverseCount::try_from(universes).map_err(|error| error.to_string())?;
        let closed_workload = workloads::by_name(workload).ok_or_else(|| {
            format!("expected workload {workload:?} is outside the closed registry")
        })?;
        if closed_workload.name() != workload {
            return Err("closed workload registry returned a non-canonical identity".into());
        }
        palette_by_name(palette)?;
        let condition_id = condition_id(seed, universes.get(), palette, check_divergence);
        let command_id = command_id(workload, &condition_id);
        let oracle_contract_id = oracle_contract_id(closed_workload.as_ref());
        Ok(Self {
            workload: workload.to_string(),
            seed,
            universes,
            palette: palette.to_string(),
            schedule_policy: SchedulePolicy::Fifo,
            check_divergence,
            command_id,
            condition_id,
            oracle_contract_id,
        })
    }

    pub(crate) fn workload(&self) -> &str {
        &self.workload
    }

    pub(crate) fn seed(&self) -> u64 {
        self.seed
    }

    pub(crate) fn universes(&self) -> u64 {
        self.universes.get()
    }

    pub(crate) fn palette(&self) -> &str {
        &self.palette
    }

    pub(crate) fn schedule_policy(&self) -> SchedulePolicy {
        self.schedule_policy
    }

    pub(crate) fn check_divergence(&self) -> bool {
        self.check_divergence
    }

    pub(crate) fn target_revision_for_engine(&self, engine_sha256: &str) -> Result<String, String> {
        if !is_canonical_sha256(engine_sha256) {
            return Err("engine SHA-256 must be 64 lowercase hexadecimal characters".into());
        }
        Ok(framed_digest(
            TIER1_WORKLOAD_TARGET_REVISION_DOMAIN,
            [
                ("engine-sha256", engine_sha256.as_bytes()),
                ("closed-workload-id", self.workload.as_bytes()),
            ],
        ))
    }

    pub(crate) fn command_id(&self) -> &str {
        &self.command_id
    }

    pub(crate) fn condition_id(&self) -> &str {
        &self.condition_id
    }

    pub(crate) fn oracle_contract_id(&self) -> &str {
        &self.oracle_contract_id
    }

    fn matches_manifest(&self, manifest: &VerifyManifest) -> Result<(), String> {
        if self.workload() != manifest.workload {
            return Err("run workload does not match the exact expectation".into());
        }
        if self.seed() != manifest.seed {
            return Err("run seed does not match the exact expectation".into());
        }
        if self.universes != manifest.universes {
            return Err("run universe count does not match the exact expectation".into());
        }
        if self.palette() != manifest.palette {
            return Err("run palette does not match the exact expectation".into());
        }
        if self.schedule_policy() != SchedulePolicy::Fifo {
            return Err("run schedule does not match the exact FIFO expectation".into());
        }
        if self.check_divergence() != manifest.divergence_check {
            return Err("run divergence setting does not match the exact expectation".into());
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum FreshRunOutcome {
    Clean,
    Findings,
    Unchecked,
}

impl FreshRunOutcome {
    fn label(self) -> &'static str {
        match self {
            Self::Clean => "CLEAN",
            Self::Findings => "FINDINGS",
            Self::Unchecked => "UNCHECKED",
        }
    }

    fn exit_code(self) -> i32 {
        match self {
            Self::Clean => 0,
            Self::Findings => 1,
            Self::Unchecked => 3,
        }
    }

    fn semantically_verified(self) -> bool {
        !matches!(self, Self::Unchecked)
    }
}

/// Unforgeable-by-safe-callers evidence that this exact verifier image freshly
/// reproduced and authenticated a complete v2 run tree. Construction is kept
/// private to the verifier core; notably this type has no `Clone`, `Default`,
/// `From`, or raw constructor.
#[derive(Debug)]
pub(crate) struct FreshRunProof {
    engine_sha256: String,
    workload_target_revision: String,
    command_id: String,
    condition_id: String,
    oracle_contract_id: String,
    outcome: FreshRunOutcome,
    evidence_digest: String,
    result_digest: String,
    finding_count: usize,
    budget_universes: u64,
    results_len: usize,
    budget_exhausted: bool,
    fault_plan_digests: Vec<String>,
    verification_result_id: String,
}

impl FreshRunProof {
    pub(crate) fn engine_sha256(&self) -> &str {
        &self.engine_sha256
    }

    pub(crate) fn workload_target_revision(&self) -> &str {
        &self.workload_target_revision
    }

    pub(crate) fn command_id(&self) -> &str {
        &self.command_id
    }

    pub(crate) fn condition_id(&self) -> &str {
        &self.condition_id
    }

    pub(crate) fn oracle_contract_id(&self) -> &str {
        &self.oracle_contract_id
    }

    pub(crate) fn outcome(&self) -> FreshRunOutcome {
        self.outcome
    }

    pub(crate) fn evidence_digest(&self) -> &str {
        &self.evidence_digest
    }

    pub(crate) fn result_digest(&self) -> &str {
        &self.result_digest
    }

    pub(crate) fn finding_count(&self) -> usize {
        self.finding_count
    }

    pub(crate) fn budget_universes(&self) -> u64 {
        self.budget_universes
    }

    pub(crate) fn results_len(&self) -> usize {
        self.results_len
    }

    pub(crate) fn budget_exhausted(&self) -> bool {
        self.budget_exhausted
    }

    /// Ordered by universe id. `"null"` is the canonical value for a
    /// workload that retrieved no plan; no vector element is omitted.
    pub(crate) fn fault_plan_digests(&self) -> &[String] {
        &self.fault_plan_digests
    }

    pub(crate) fn verification_result_id(&self) -> &str {
        &self.verification_result_id
    }
}

fn generic_engine_request_digest(manifest: &VerifyManifest) -> String {
    let mut bytes = Vec::new();
    bytes.extend_from_slice(GENERIC_ENGINE_REQUEST_DOMAIN.as_bytes());
    bytes.push(b'\n');
    digest_frame(&mut bytes, "workload", manifest.workload.as_bytes());
    digest_frame(
        &mut bytes,
        "seed",
        format!("0x{:x}", manifest.seed).as_bytes(),
    );
    digest_frame(
        &mut bytes,
        "universes",
        manifest.universes.get().to_string().as_bytes(),
    );
    digest_frame(&mut bytes, "palette", manifest.palette.as_bytes());
    digest_frame(
        &mut bytes,
        "divergence-check",
        if manifest.divergence_check {
            b"true"
        } else {
            b"false"
        },
    );
    digest_frame(&mut bytes, "schedule", b"fifo");
    digest_frame(&mut bytes, "record-tape", b"false");
    match &manifest.provenance.declared_source_commit {
        Some(commit) => {
            digest_frame(&mut bytes, "source-commit-present", b"true");
            digest_frame(&mut bytes, "source-commit", commit.as_bytes());
        }
        None => digest_frame(&mut bytes, "source-commit-present", b"false"),
    }
    vh_digest::sha256_hex(&bytes)
}

fn exact_manifest(line: &str) -> Result<VerifyManifest, String> {
    const KEYS: [&str; 17] = [
        "record",
        "schema",
        "workload",
        "seed",
        "universes",
        "palette",
        "schedule_policy",
        "divergence_check",
        "verdict",
        "findings",
        "divergent",
        "sometimes_unreached",
        "cli_version",
        "build_profile",
        "target_os",
        "target_arch",
        "declared_source_commit",
    ];
    let fields = parse_line(line).map_err(|error| format!("malformed manifest: {error}"))?;
    if fields.len() != KEYS.len()
        || fields
            .iter()
            .zip(KEYS)
            .any(|((actual, _), expected)| actual != expected)
    {
        return Err("manifest keys/order do not match the closed v2 schema".into());
    }
    let string_at = |index: usize| match &fields[index].1 {
        Val::S(value) => Ok(value.clone()),
        _ => Err(format!("manifest field {:?} must be a string", KEYS[index])),
    };
    let number_at = |index: usize| match &fields[index].1 {
        Val::N(value) => Ok(*value),
        _ => Err(format!("manifest field {:?} must be a u64", KEYS[index])),
    };
    let bool_at = |index: usize| match &fields[index].1 {
        Val::B(value) => Ok(*value),
        _ => Err(format!("manifest field {:?} must be a bool", KEYS[index])),
    };

    if string_at(0)? != "manifest" || string_at(1)? != RUN_RECEIPTS_SCHEMA_V2 {
        return Err("unsupported manifest record/schema".into());
    }
    let workload = string_at(2)?;
    let seed_text = string_at(3)?;
    let seed_digits = seed_text
        .strip_prefix("0x")
        .filter(|digits| !digits.is_empty())
        .ok_or("manifest seed must be canonical lowercase hexadecimal")?;
    let seed = u64::from_str_radix(seed_digits, 16)
        .map_err(|_| "manifest seed is outside the u64 domain")?;
    if seed_text != format!("0x{seed:x}") {
        return Err("manifest seed is not canonical lowercase hexadecimal".into());
    }
    let requested_universes = number_at(4)?;
    if requested_universes > MAX_VERIFY_UNIVERSES {
        return Err(format!(
            "manifest universe count exceeds the verifier work bound ({MAX_VERIFY_UNIVERSES})"
        ));
    }
    let universes =
        UniverseCount::try_from(requested_universes).map_err(|error| error.to_string())?;
    let palette = string_at(5)?;
    palette_by_name(&palette)?;
    if string_at(6)? != "fifo" {
        return Err("v2 run receipts can only verify the fifo/untaped profile".into());
    }
    let divergence_check = bool_at(7)?;
    if !matches!(string_at(8)?.as_str(), "CLEAN" | "FINDINGS" | "UNCHECKED") {
        return Err("manifest verdict is outside the closed tri-state".into());
    }
    let _ = (number_at(9)?, number_at(10)?, number_at(11)?);
    let declared_source_commit = match &fields[16].1 {
        Val::S(value) => Some(value.clone()),
        Val::Null => None,
        _ => return Err("manifest declared_source_commit must be string|null".into()),
    };
    let provenance = Provenance {
        cli_version: string_at(12)?,
        build_profile: string_at(13)?,
        target_os: string_at(14)?,
        target_arch: string_at(15)?,
        declared_source_commit,
    };
    let current = crate::build_provenance(provenance.declared_source_commit.clone());
    if provenance != current {
        return Err("manifest build provenance does not match this verifier engine".into());
    }
    Ok(VerifyManifest {
        workload,
        seed,
        universes,
        palette,
        divergence_check,
        provenance,
    })
}

fn exact_digest(line: &str, body: &[u8]) -> Result<String, String> {
    let fields = parse_line(line).map_err(|error| format!("malformed digest record: {error}"))?;
    if fields.len() != 3
        || fields[0].0 != "record"
        || fields[1].0 != "alg"
        || fields[2].0 != "value"
    {
        return Err("digest keys/order do not match the closed schema".into());
    }
    let recorded = match (&fields[0].1, &fields[1].1, &fields[2].1) {
        (Val::S(record), Val::S(alg), Val::S(value))
            if record == "digest" && alg == vh_digest::ALGORITHM =>
        {
            value
        }
        _ => return Err("digest record/type/algorithm mismatch".into()),
    };
    if recorded.len() != 64
        || !recorded.bytes().all(|byte| byte.is_ascii_hexdigit())
        || recorded.bytes().any(|byte| byte.is_ascii_uppercase())
    {
        return Err("digest value must be 64 lowercase hexadecimal characters".into());
    }
    if render_line(&[
        ("record", Val::S("digest".into())),
        ("alg", Val::S(vh_digest::ALGORITHM.into())),
        ("value", Val::S(recorded.clone())),
    ]) != line
    {
        return Err("digest record is not canonical".into());
    }
    let recomputed = vh_digest::sha256_hex(body);
    if *recorded != recomputed {
        return Err("run receipt content digest mismatch".into());
    }
    Ok(recomputed)
}

fn verify_receipt_tree(dir: &Path, finding_ids: &[String]) -> Result<(), String> {
    let mut root_names = Vec::new();
    for entry in fs::read_dir(dir).map_err(|_| "cannot enumerate receipt root")? {
        let entry = entry.map_err(|_| "cannot enumerate receipt root")?;
        root_names.push(entry.file_name());
        if root_names.len() > 2 {
            return Err("receipt root contains unexpected entries".into());
        }
    }
    let run_name = std::ffi::OsStr::new("run.ndjson");
    let findings_name = std::ffi::OsStr::new("findings");
    if !root_names.iter().any(|name| name == run_name)
        || root_names
            .iter()
            .any(|name| name != run_name && name != findings_name)
        || (finding_ids.is_empty() && root_names.iter().any(|name| name == findings_name))
        || (!finding_ids.is_empty() && !root_names.iter().any(|name| name == findings_name))
    {
        return Err("receipt root does not match the canonical run tree".into());
    }
    if finding_ids.is_empty() {
        return Ok(());
    }
    let findings_root = dir.join("findings");
    let metadata = fs::symlink_metadata(&findings_root)
        .map_err(|_| "cannot inspect canonical findings directory")?;
    if metadata.file_type().is_symlink() || !metadata.is_dir() {
        return Err("canonical findings path is not a no-link directory".into());
    }
    let expected: std::collections::BTreeSet<&str> =
        finding_ids.iter().map(String::as_str).collect();
    let mut observed = std::collections::BTreeSet::new();
    for entry in fs::read_dir(&findings_root).map_err(|_| "cannot enumerate findings")? {
        let entry = entry.map_err(|_| "cannot enumerate findings")?;
        let name = entry
            .file_name()
            .into_string()
            .map_err(|_| "finding directory name is not UTF-8")?;
        if !expected.contains(name.as_str()) || !observed.insert(name.clone()) {
            return Err("findings directory contains an unexpected/orphan entry".into());
        }
        let metadata = entry
            .file_type()
            .map_err(|_| "cannot inspect finding directory")?;
        if !metadata.is_dir() || metadata.is_symlink() {
            return Err("finding entry is not a no-link directory".into());
        }
        let mut children =
            fs::read_dir(entry.path()).map_err(|_| "cannot enumerate finding directory")?;
        let child = children
            .next()
            .ok_or("finding directory is empty")?
            .map_err(|_| "cannot enumerate finding directory")?;
        if child.file_name() != std::ffi::OsStr::new("finding.ndjson")
            || children.next().is_some()
            || !child
                .file_type()
                .map_err(|_| "cannot inspect finding bundle")?
                .is_file()
        {
            return Err("finding directory does not contain exactly finding.ndjson".into());
        }
    }
    if observed.len() != expected.len() {
        return Err("one or more canonical finding directories are missing".into());
    }
    Ok(())
}

fn verify_finding_bundles(
    dir: &Path,
    report: &MultiverseReport,
    finding_universes: &[u64],
    palette_name: &str,
    provenance: &Provenance,
) -> Result<usize, String> {
    let workload = workloads::by_name(report.workload())
        .ok_or("recomputed report names an unavailable workload")?;
    let palette = palette_by_name(palette_name)?;
    let first_shrink_universe = report.failing_universes().first().copied();
    let mut lineage_seen = false;
    let mut aggregate_bundle_bytes = 0u64;
    for &universe in finding_universes {
        let expected = bundle_v2_for(report, universe, palette_name, provenance, None);
        let path = dir
            .join("findings")
            .join(&expected.finding_id)
            .join("finding.ndjson");
        let bytes = vh_sandbox::read_bounded_file(&path, MAX_FINDING_BUNDLE_BYTES)
            .map_err(|_| "finding bundle boundary read refused")?;
        aggregate_bundle_bytes =
            checked_aggregate_bundle_bytes(aggregate_bundle_bytes, bytes.len() as u64)?;
        let text = std::str::from_utf8(&bytes).map_err(|_| "finding bundle is not UTF-8")?;
        let mut actual = FindingBundleV2::parse(text)
            .map_err(|error| format!("finding bundle is invalid: {error}"))?;
        if actual.to_ndjson() != text {
            return Err("finding bundle bytes are not the canonical v2 rendering".into());
        }
        let lineage = actual.lineage.take();
        if actual != expected {
            return Err("finding bundle does not bind to the fresh report observation".into());
        }
        // A finding is a replay promise even when the containing campaign was
        // explicitly UNCHECKED. Re-run it twice here and bind both executions
        // to the freshly regenerated expected bundle.
        let replay_a =
            run_universe_with_palette(report.root_seed(), universe, workload.as_ref(), palette);
        let replay_b =
            run_universe_with_palette(report.root_seed(), universe, workload.as_ref(), palette);
        if !replay_a.observably_equal(&replay_b) {
            return Err("fresh finding replays diverged".into());
        }
        let mut replay_mismatches = Vec::new();
        compare_result_to_v2(&replay_a, &expected, &mut replay_mismatches);
        if !replay_mismatches.is_empty() {
            return Err("fresh finding replay does not equal the regenerated observation".into());
        }
        if let Some(actual_lineage) = lineage {
            if lineage_seen || Some(universe) != first_shrink_universe || palette_name != "v0" {
                return Err(
                    "shrink lineage is attached outside the single eligible finding".into(),
                );
            }
            let outcome = vh_cli::shrink_cli::shrink_universe(
                report.workload(),
                report.root_seed(),
                universe,
            )
            .map_err(|_| "recorded shrink lineage cannot be recomputed")?;
            let expected_lineage = lineage_for(&outcome, workload.as_ref())?;
            if actual_lineage != expected_lineage {
                return Err("shrink lineage does not equal the fresh minimizer result".into());
            }
            lineage_seen = true;
        }
    }
    Ok(finding_universes.len())
}

fn bounded_verify_error(error: &str) -> String {
    crate::cooperative::bounded_diagnostic(error)
}

struct VerifyRunAttempt {
    proof: Result<FreshRunProof, String>,
    result_digest: String,
    evidence_digest: String,
    engine_request_digest: String,
    findings_total: usize,
    findings_verified: usize,
}

pub(crate) struct VerificationResultFacts<'a> {
    pub(crate) engine_sha256: &'a str,
    pub(crate) workload_target_revision: &'a str,
    pub(crate) expected: &'a RunExpectation,
    pub(crate) outcome: FreshRunOutcome,
    pub(crate) evidence_digest: &'a str,
    pub(crate) result_digest: &'a str,
    pub(crate) finding_count: usize,
    pub(crate) results_len: usize,
    pub(crate) budget_exhausted: bool,
    pub(crate) fault_plan_digests: &'a [String],
}

pub(crate) fn verification_result_id(facts: &VerificationResultFacts<'_>) -> String {
    let finding_count = facts.finding_count.to_string();
    let budget_universes = facts.expected.universes().to_string();
    let results_len = facts.results_len.to_string();
    let mut bytes = Vec::new();
    bytes.extend_from_slice(TIER1_VERIFICATION_RESULT_DOMAIN.as_bytes());
    bytes.push(b'\n');
    digest_frame(&mut bytes, "engine-sha256", facts.engine_sha256.as_bytes());
    digest_frame(
        &mut bytes,
        "workload-target-revision",
        facts.workload_target_revision.as_bytes(),
    );
    digest_frame(
        &mut bytes,
        "command-id",
        facts.expected.command_id().as_bytes(),
    );
    digest_frame(
        &mut bytes,
        "condition-id",
        facts.expected.condition_id().as_bytes(),
    );
    digest_frame(
        &mut bytes,
        "oracle-contract-id",
        facts.expected.oracle_contract_id().as_bytes(),
    );
    digest_frame(&mut bytes, "outcome", facts.outcome.label().as_bytes());
    digest_frame(
        &mut bytes,
        "evidence-digest",
        facts.evidence_digest.as_bytes(),
    );
    digest_frame(&mut bytes, "result-digest", facts.result_digest.as_bytes());
    digest_frame(&mut bytes, "finding-count", finding_count.as_bytes());
    digest_frame(&mut bytes, "budget-universes", budget_universes.as_bytes());
    digest_frame(&mut bytes, "results-len", results_len.as_bytes());
    digest_frame(
        &mut bytes,
        "budget-exhausted",
        if facts.budget_exhausted {
            b"true"
        } else {
            b"false"
        },
    );
    for (universe, digest) in facts.fault_plan_digests.iter().enumerate() {
        digest_frame(
            &mut bytes,
            &format!("fault-plan-digest-{universe}"),
            digest.as_bytes(),
        );
    }
    vh_digest::sha256_hex(&bytes)
}

/// Shared semantic core for both the legacy record-rendering command and the
/// typed Tier-1 API. `expected == None` exists solely for the byte-compatible
/// CLI wrapper, whose historical request surface did not carry an independent
/// expectation; typed consumers must call [`verify_run_fresh`].
fn verify_run_inner(
    dir_path: &Path,
    engine_sha256: &str,
    bytes: &[u8],
    text: &str,
    expected: Option<&RunExpectation>,
    fresh_replay_attempts: &mut u64,
) -> VerifyRunAttempt {
    let result_digest = vh_digest::sha256_hex(bytes);
    let mut evidence_digest = String::new();
    let mut findings_total = 0usize;
    let mut findings_verified = 0usize;
    let mut engine_request_digest = String::new();

    let proof: Result<FreshRunProof, String> = (|| {
        let max_lines = (MAX_VERIFY_UNIVERSES as usize)
            .saturating_mul(2)
            .saturating_add(2);
        if text
            .bytes()
            .filter(|byte| *byte == b'\n')
            .take(max_lines + 1)
            .count()
            > max_lines
        {
            return Err("run receipt exceeds the canonical line-count bound".into());
        }
        let raw: Vec<&str> = text.split('\n').collect();
        let (tail, body_and_digest) = raw.split_last().ok_or("empty run receipt")?;
        if !tail.is_empty() || body_and_digest.iter().any(|line| line.is_empty()) {
            return Err("run receipt line framing is not canonical".into());
        }
        let (digest_line, body_lines) = body_and_digest
            .split_last()
            .ok_or("run receipt has no digest record")?;
        if body_lines.is_empty() {
            return Err("run receipt has no manifest record".into());
        }
        let mut body = String::new();
        for line in body_lines {
            body.push_str(line);
            body.push('\n');
        }
        evidence_digest = exact_digest(digest_line, body.as_bytes())?;
        let manifest = exact_manifest(body_lines[0])?;
        engine_request_digest = generic_engine_request_digest(&manifest);

        // An independently supplied mismatch is rejected before the closed
        // workload is selected or any universe executes.
        if let Some(expected) = expected {
            expected.matches_manifest(&manifest)?;
        }
        let inferred_expectation = if expected.is_none() {
            Some(RunExpectation::fifo(
                &manifest.workload,
                manifest.seed,
                manifest.universes.get(),
                &manifest.palette,
                manifest.divergence_check,
            )?)
        } else {
            None
        };
        let expected = expected
            .or(inferred_expectation.as_ref())
            .ok_or("verifier expectation was not established")?;

        let workload = workloads::by_name(&manifest.workload)
            .ok_or("manifest workload is outside the closed registry")?;
        if oracle_contract_id(workload.as_ref()) != expected.oracle_contract_id() {
            return Err("fresh workload oracle contract does not match the expectation".into());
        }
        let palette = palette_by_name(&manifest.palette)?;
        let config = MultiverseConfig {
            root_seed: manifest.seed,
            universes: manifest.universes,
            check_divergence: manifest.divergence_check,
        };
        *fresh_replay_attempts += 1;
        let report = run_multiverse_with_palette(&config, workload.as_ref(), palette);
        let outcome = match report.verdict() {
            Verdict::Clean => FreshRunOutcome::Clean,
            Verdict::Findings => FreshRunOutcome::Findings,
            Verdict::Unchecked => FreshRunOutcome::Unchecked,
        };
        let findings = finding_universes(&report);
        findings_total = findings.len();
        let identity = RunIdentity {
            palette_name: &manifest.palette,
            universes_requested: manifest.universes.get(),
            check_divergence: manifest.divergence_check,
            verdict_label: outcome.label(),
            provenance: &manifest.provenance,
            lineage: None,
        };
        if body != run_receipt_body(&report, &identity, &findings) {
            return Err("run receipt body does not equal the fresh semantic reproduction".into());
        }
        let finding_ids: Vec<String> = findings
            .iter()
            .map(|&universe| {
                let result = &report.results()[universe as usize];
                let digest =
                    vh_digest::sha256_hex(result.complete_observation_identity().canonical_bytes());
                finding_id_v2(universe, &digest)
            })
            .collect();
        verify_receipt_tree(dir_path, &finding_ids)?;
        findings_verified = verify_finding_bundles(
            dir_path,
            &report,
            &findings,
            &manifest.palette,
            &manifest.provenance,
        )?;
        if findings_verified != findings_total {
            return Err("fresh verification did not exhaust the complete finding set".into());
        }

        let results_len = report.results().len();
        let budget_exhausted = results_len as u64 == expected.universes();
        if !budget_exhausted || report.universes_requested() != expected.universes() {
            return Err("fresh verification did not exhaust the exact universe budget".into());
        }
        let mut fault_plan_digests = Vec::with_capacity(results_len);
        for (universe, result) in report.results().iter().enumerate() {
            if result.universe_id() != universe as u64 {
                return Err("fresh report universe order is not canonical".into());
            }
            let digest = match result.fault_plan_digest() {
                Some(digest) => digest.to_string(),
                None => "null".to_string(),
            };
            fault_plan_digests.push(digest);
        }

        let workload_target_revision = expected.target_revision_for_engine(engine_sha256)?;
        let verification_result_id = verification_result_id(&VerificationResultFacts {
            engine_sha256,
            workload_target_revision: &workload_target_revision,
            expected,
            outcome,
            evidence_digest: &evidence_digest,
            result_digest: &result_digest,
            finding_count: findings_total,
            results_len,
            budget_exhausted,
            fault_plan_digests: &fault_plan_digests,
        });
        Ok(FreshRunProof {
            engine_sha256: engine_sha256.to_string(),
            workload_target_revision,
            command_id: expected.command_id().to_string(),
            condition_id: expected.condition_id().to_string(),
            oracle_contract_id: expected.oracle_contract_id().to_string(),
            outcome,
            evidence_digest: evidence_digest.clone(),
            result_digest: result_digest.clone(),
            finding_count: findings_total,
            budget_universes: expected.universes(),
            results_len,
            budget_exhausted,
            fault_plan_digests,
            verification_result_id,
        })
    })();

    VerifyRunAttempt {
        proof,
        result_digest,
        evidence_digest,
        engine_request_digest,
        findings_total,
        findings_verified,
    }
}

/// Verify an exact, independently declared Tier-1 campaign using the supplied
/// executing engine image. A caller receives no partially populated proof:
/// every boundary, canonical receipt/tree check, and fresh reproduction must
/// complete first.
pub(crate) fn verify_run_fresh(
    out_dir: &Path,
    engine_path: &Path,
    expected: &RunExpectation,
) -> Result<FreshRunProof, String> {
    let engine_bytes = vh_sandbox::read_bounded_file(engine_path, MAX_VERIFY_ENGINE_BYTES)
        .map_err(|_| "verifier engine boundary read refused".to_string())?;
    let supplied_engine_sha256 = vh_digest::sha256_hex(&engine_bytes);
    let engine_sha256 = crate::cooperative::current_engine_sha256()
        .map_err(|_| "executing verifier image boundary read refused".to_string())?;
    if engine_sha256 != supplied_engine_sha256 {
        return Err("--engine does not identify the executing verifier image".into());
    }
    let bytes = vh_sandbox::read_bounded_file(&out_dir.join("run.ndjson"), MAX_RUN_RECEIPT_BYTES)
        .map_err(|_| "run receipt boundary read refused".to_string())?;
    let text = std::str::from_utf8(&bytes).map_err(|_| "run receipt is not UTF-8".to_string())?;
    let mut fresh_replay_attempts = 0;
    verify_run_inner(
        out_dir,
        &engine_sha256,
        &bytes,
        text,
        Some(expected),
        &mut fresh_replay_attempts,
    )
    .proof
}

const DEMO_ADMISSION_SEED: u64 = 0xd1ce;
const DEMO_ADMISSION_UNIVERSES: u64 = 4;
const MAX_ADMISSION_RECEIPT_BYTES: u64 = 1 << 20;

fn run_expected_receipt(out_dir: &Path, expected: &RunExpectation) -> Result<(), String> {
    if expected.schedule_policy() != SchedulePolicy::Fifo {
        return Err("demo admission accepts only the exact FIFO plan".into());
    }
    let workload = workloads::by_name(expected.workload())
        .ok_or_else(|| "demo admission workload left the closed registry".to_string())?;
    let palette = palette_by_name(expected.palette())?;
    let config = MultiverseConfig {
        root_seed: expected.seed(),
        universes: UniverseCount::try_from(expected.universes())
            .map_err(|error| error.to_string())?,
        check_divergence: expected.check_divergence(),
    };
    let report = run_multiverse_with_palette(&config, workload.as_ref(), palette);
    let verdict = match report.verdict() {
        Verdict::Clean => "CLEAN",
        Verdict::Findings => "FINDINGS",
        Verdict::Unchecked => "UNCHECKED",
    };
    let provenance = crate::build_provenance(None);
    let out = out_dir
        .to_str()
        .ok_or_else(|| "demo admission output path is not UTF-8".to_string())?;
    write_run_receipts(
        out,
        &report,
        &RunIdentity {
            palette_name: expected.palette(),
            universes_requested: expected.universes(),
            check_divergence: expected.check_divergence(),
            verdict_label: verdict,
            provenance: &provenance,
            lineage: None,
        },
    )?;
    Ok(())
}

/// Execute and freshly verify the one closed faulty/fixed Tier-1 calibration
/// pair. The canonical receipt is useful for dogfood and parser integration;
/// it is not a foreign-target or arbitrary-repository result.
pub(crate) fn cmd_demo_admission(args: &[String], usage: &str) -> i32 {
    let mut out: Option<String> = None;
    let mut iterator = args.iter();
    while let Some(argument) = iterator.next() {
        match argument.as_str() {
            "--out" if out.is_none() => match iterator.next() {
                Some(value) => out = Some(value.clone()),
                None => {
                    eprintln!("error: --out requires a value\n\n{usage}");
                    return 2;
                }
            },
            "--out" => {
                eprintln!("error: duplicate --out\n\n{usage}");
                return 2;
            }
            other => {
                eprintln!(
                    "error: unknown argument: {}\n\n{usage}",
                    crate::cooperative::bounded_diagnostic(other)
                );
                return 2;
            }
        }
    }
    let Some(out) = out else {
        eprintln!("error: demo-admission requires --out\n\n{usage}");
        return 2;
    };

    let result: Result<crate::admission::Admission, String> = (|| {
        let engine_path = crate::current_engine_path()?;
        let engine_sha256 = crate::cooperative::current_engine_sha256()?;
        // This is the authority-critical ordering: both exact role requests
        // and their target identities are frozen before either arm executes.
        let plan = crate::admission::PairedExecutionPlan::tier1_kv_demo(
            &engine_sha256,
            DEMO_ADMISSION_SEED,
            DEMO_ADMISSION_UNIVERSES,
        )?;
        let output_root = crate::cooperative::prepare_output_root(Path::new(&out))?;
        let treatment_root = output_root.join("treatment");
        let fixed_control_root = output_root.join("fixed-control");

        run_expected_receipt(&treatment_root, plan.treatment_expectation())?;
        run_expected_receipt(&fixed_control_root, plan.fixed_control_expectation())?;
        let treatment =
            verify_run_fresh(&treatment_root, &engine_path, plan.treatment_expectation())?;
        let fixed_control = verify_run_fresh(
            &fixed_control_root,
            &engine_path,
            plan.fixed_control_expectation(),
        )?;
        Ok(crate::admission::Admission::classify(
            plan,
            treatment,
            fixed_control,
        ))
    })();

    let admission = match result {
        Ok(admission) => admission,
        Err(error) => {
            eprintln!(
                "error: demo admission failed closed: {}",
                crate::cooperative::bounded_diagnostic(&error)
            );
            return 2;
        }
    };
    let receipt = admission.receipt();
    let receipt_path = Path::new(&out).join("admission.receipt");
    if crate::cooperative::write_new_file(&receipt_path, receipt.canonical_bytes()).is_err() {
        eprintln!("error: demo admission receipt publication failed closed");
        return 2;
    }
    let published = match vh_sandbox::read_bounded_file(&receipt_path, MAX_ADMISSION_RECEIPT_BYTES)
    {
        Ok(bytes) => bytes,
        Err(_) => {
            eprintln!("error: demo admission receipt read-back failed closed");
            return 2;
        }
    };
    if published != receipt.canonical_bytes()
        || crate::admission::RealExecutionReceipt::verify_canonical(&published, receipt.sha256())
            .is_err()
    {
        eprintln!("error: demo admission receipt verification failed closed");
        return 2;
    }
    match std::str::from_utf8(&published) {
        Ok(text) => print!("{text}"),
        Err(_) => {
            eprintln!("error: demo admission receipt is not UTF-8");
            return 2;
        }
    }
    debug_assert_eq!(
        admission.fixed_control_miss(),
        admission.kind_label() == "CONFIRMED"
    );
    admission.exit_code()
}

/// `vh verify-run --out DIR --engine PATH` performs a fresh semantic
/// reproduction with this engine. Caller-authored verdict/count/path fields
/// are never trusted: the full receipt body and every bundle's base observation
/// are regenerated from the closed workload registry and compared
/// byte-for-byte/injectively. If a bundle carries shrink lineage, the lineage
/// is independently recomputed; absence of lineage is not proof that shrinking
/// was requested. No replay subprocess is spawned from receipt bytes.
pub fn cmd_verify_run(args: &[String], usage: &str) -> i32 {
    let mut out_dir: Option<String> = None;
    let mut engine: Option<String> = None;
    let mut it = args.iter();
    while let Some(arg) = it.next() {
        match arg.as_str() {
            "--out" => match it.next() {
                Some(value) => out_dir = Some(value.clone()),
                None => {
                    eprintln!("error: --out requires a value\n\n{usage}");
                    return 2;
                }
            },
            "--engine" => match it.next() {
                Some(value) => engine = Some(value.clone()),
                None => {
                    eprintln!("error: --engine requires a value\n\n{usage}");
                    return 2;
                }
            },
            other => {
                let safe_other = crate::cooperative::bounded_diagnostic(other);
                eprintln!("error: unknown argument {safe_other:?}\n\n{usage}");
                return 2;
            }
        }
    }
    let (Some(dir), Some(engine)) = (out_dir, engine) else {
        eprintln!("error: verify-run requires --out DIR --engine PATH\n\n{usage}");
        return 2;
    };
    let engine_bytes =
        match vh_sandbox::read_bounded_file(Path::new(&engine), MAX_VERIFY_ENGINE_BYTES) {
            Ok(bytes) => bytes,
            Err(_) => {
                eprintln!("error: verifier engine boundary read refused");
                return 2;
            }
        };
    let supplied_engine_sha256 = vh_digest::sha256_hex(&engine_bytes);
    let engine_sha256 = match crate::cooperative::current_engine_sha256() {
        Ok(digest) if digest == supplied_engine_sha256 => digest,
        Ok(_) => {
            eprintln!("error: --engine does not identify the executing verifier image");
            return 2;
        }
        Err(_) => {
            eprintln!("error: executing verifier image boundary read refused");
            return 2;
        }
    };
    let dir_path = Path::new(&dir);
    let run_path = dir_path.join("run.ndjson");
    let bytes = match vh_sandbox::read_bounded_file(&run_path, MAX_RUN_RECEIPT_BYTES) {
        Ok(bytes) => bytes,
        Err(_) => {
            eprintln!("error: run receipt boundary read refused");
            return 2;
        }
    };
    let text = match std::str::from_utf8(&bytes) {
        Ok(text) => text,
        Err(_) => {
            eprintln!("error: run receipt is not UTF-8");
            return 2;
        }
    };
    let mut fresh_replay_attempts = 0;
    let attempt = verify_run_inner(
        dir_path,
        &engine_sha256,
        &bytes,
        text,
        None,
        &mut fresh_replay_attempts,
    );
    let errors = attempt
        .proof
        .as_ref()
        .err()
        .map(|error| vec![bounded_verify_error(error)])
        .unwrap_or_default();
    let authentic = errors.is_empty();
    let outcome = attempt.proof.as_ref().ok().map(FreshRunProof::outcome);
    let outcome_verified = outcome.is_some_and(FreshRunOutcome::semantically_verified);
    render_verify_run_record(
        authentic,
        outcome_verified,
        outcome.map(FreshRunOutcome::label).unwrap_or("ERROR"),
        outcome.map(FreshRunOutcome::exit_code).unwrap_or(2),
        &attempt.result_digest,
        &attempt.evidence_digest,
        &engine_sha256,
        &attempt.engine_request_digest,
        attempt.findings_total,
        attempt.findings_verified,
        &errors,
    );
    if authentic {
        0
    } else {
        1
    }
}

#[allow(clippy::too_many_arguments)]
fn render_verify_run_record(
    authentic: bool,
    outcome_verified: bool,
    verdict: &str,
    outcome_exit_code: i32,
    result_digest: &str,
    evidence_digest: &str,
    engine_sha256: &str,
    engine_request_digest: &str,
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
            ("schema", Val::S("vh-verify-run-v2".into())),
            ("authentic", Val::B(authentic)),
            ("verified", Val::B(authentic && outcome_verified),),
            ("outcome_verified", Val::B(outcome_verified)),
            ("verdict", Val::S(verdict.to_string())),
            ("outcome_exit_code", Val::N(outcome_exit_code as u64)),
            ("evidence_digest", Val::S(evidence_digest.to_string())),
            ("result_digest", Val::S(result_digest.to_string())),
            ("engine_sha256", Val::S(engine_sha256.to_string())),
            (
                "engine_request_digest",
                Val::S(engine_request_digest.to_string()),
            ),
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
    let got_contract = workloads::by_name(&bundle.workload)
        .map(|workload| workload.property_contract().violations(a))
        .unwrap_or_else(|| vec!["bundle workload unavailable".into()]);
    if got_contract != bundle.contract_violations {
        out.push(format!(
            "contract_violations: got {got_contract:?}, bundle {:?}",
            bundle.contract_violations
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
            let safe_file = crate::cooperative::bounded_diagnostic(&file.to_string_lossy());
            eprintln!(
                "error: malformed bundle {}: {}",
                safe_file,
                crate::cooperative::bounded_diagnostic(&e)
            );
            return 2;
        }
    };
    let workload = match workloads::by_name(&bundle.workload) {
        Some(w) => w,
        None => {
            let safe_workload = crate::cooperative::bounded_diagnostic(&bundle.workload);
            eprintln!(
                "error: bundle names unknown workload {safe_workload:?} (this build cannot replay it)"
            );
            return 2;
        }
    };
    let palette = match palette_by_name(&bundle.palette) {
        Ok(p) => p,
        Err(e) => {
            eprintln!("error: {}", crate::cooperative::bounded_diagnostic(&e));
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
        let safe_finding_id = crate::cooperative::bounded_diagnostic(&bundle.finding_id);
        let safe_trace_hash = crate::cooperative::bounded_diagnostic(&bundle.trace_hash);
        println!(
            "replay-bundle: REPRODUCED {} (workload {} seed 0x{:x} universe {} hash {} events {} vh-finding-bundle-v1; v1 = FIFO-only self-consistent replay, not authenticated provenance)",
            safe_finding_id,
            bundle.workload,
            bundle.seed,
            bundle.universe,
            safe_trace_hash,
            bundle.trace_events
        );
        0
    } else {
        let safe_finding_id = crate::cooperative::bounded_diagnostic(&bundle.finding_id);
        println!(
            "replay-bundle: MISMATCH {} — the recorded finding did not reproduce:",
            safe_finding_id
        );
        for m in &mismatches {
            println!("  {}", crate::cooperative::bounded_diagnostic(m));
        }
        1
    }
}

#[cfg(test)]
mod writer_boundary_tests {
    use super::*;

    const TIER1_TEST_SEED: u64 = 0xd1ce;
    const TIER1_TEST_UNIVERSES: u64 = 4;

    struct TestReceiptDir(PathBuf);

    impl TestReceiptDir {
        fn new(label: &str) -> Self {
            let path = crate::cooperative::reserve_test_directory(label)
                .expect("reserve isolated receipt directory");
            Self(path)
        }

        fn path(&self) -> &Path {
            &self.0
        }
    }

    impl Drop for TestReceiptDir {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    fn write_tier1_test_receipt(workload_name: &str) -> TestReceiptDir {
        let dir = TestReceiptDir::new(workload_name);
        let workload = workloads::by_name(workload_name).expect("closed test workload");
        let config = MultiverseConfig {
            root_seed: TIER1_TEST_SEED,
            universes: UniverseCount::try_from(TIER1_TEST_UNIVERSES)
                .expect("nonzero bounded test budget"),
            check_divergence: true,
        };
        let report = run_multiverse_with_palette(
            &config,
            workload.as_ref(),
            palette_by_name("v0").expect("closed v0 palette"),
        );
        let verdict = match report.verdict() {
            Verdict::Clean => "CLEAN",
            Verdict::Findings => "FINDINGS",
            Verdict::Unchecked => "UNCHECKED",
        };
        let provenance = crate::build_provenance(None);
        write_run_receipts(
            dir.path().to_str().expect("UTF-8 test path"),
            &report,
            &RunIdentity {
                palette_name: "v0",
                universes_requested: TIER1_TEST_UNIVERSES,
                check_divergence: true,
                verdict_label: verdict,
                provenance: &provenance,
                lineage: None,
            },
        )
        .expect("write canonical test receipt");
        dir
    }

    fn tier1_expectation(workload: &str) -> RunExpectation {
        RunExpectation::fifo(workload, TIER1_TEST_SEED, TIER1_TEST_UNIVERSES, "v0", true)
            .expect("valid closed test expectation")
    }

    #[test]
    fn aggregate_bundle_byte_bound_is_exact_and_overflow_safe() {
        assert_eq!(
            checked_aggregate_bundle_bytes(MAX_RUN_RECEIPT_BYTES - 1, 1).unwrap(),
            MAX_RUN_RECEIPT_BYTES
        );
        assert!(checked_aggregate_bundle_bytes(MAX_RUN_RECEIPT_BYTES, 1).is_err());
        assert!(checked_aggregate_bundle_bytes(u64::MAX, 1).is_err());
    }

    #[test]
    fn verify_error_escapes_controls_inside_the_byte_cap() {
        let diagnostic = bounded_verify_error(&format!("bad\n\x1b[31m\u{202e}{}", "é".repeat(256)));
        assert!(!diagnostic.chars().any(char::is_control));
        assert!(diagnostic.contains("\\n\\u{1b}[31m\\u{202e}"));
        assert!(diagnostic.is_ascii());
        assert!(diagnostic.len() <= crate::cooperative::MAX_DIAGNOSTIC_BYTES);
        assert!(diagnostic.ends_with("...[truncated]"));
    }

    #[test]
    fn fresh_proofs_bind_distinct_targets_under_one_shared_condition() {
        let buggy_dir = write_tier1_test_receipt("demo-buggy");
        let control_dir = write_tier1_test_receipt("demo");
        let buggy_expected = tier1_expectation("demo-buggy");
        let control_expected = tier1_expectation("demo");
        let engine = crate::current_engine_path().expect("test engine path");

        let buggy = verify_run_fresh(buggy_dir.path(), &engine, &buggy_expected)
            .expect("fresh buggy proof");
        let control = verify_run_fresh(control_dir.path(), &engine, &control_expected)
            .expect("fresh control proof");

        assert_eq!(buggy.outcome(), FreshRunOutcome::Findings);
        assert_eq!(control.outcome(), FreshRunOutcome::Clean);
        assert_eq!(buggy.finding_count(), 2);
        assert_eq!(control.finding_count(), 0);
        assert_ne!(
            buggy.workload_target_revision(),
            control.workload_target_revision()
        );
        assert_ne!(buggy.command_id(), control.command_id());
        assert_eq!(buggy.condition_id(), control.condition_id());
        assert_eq!(buggy.oracle_contract_id(), control.oracle_contract_id());
        assert_eq!(buggy.engine_sha256(), control.engine_sha256());
        assert_eq!(buggy.budget_universes(), TIER1_TEST_UNIVERSES);
        assert_eq!(buggy.budget_universes(), control.budget_universes());
        assert_eq!(buggy.results_len(), TIER1_TEST_UNIVERSES as usize);
        assert_eq!(buggy.results_len(), control.results_len());
        assert!(buggy.budget_exhausted());
        assert!(control.budget_exhausted());
        assert_eq!(buggy.fault_plan_digests(), control.fault_plan_digests());
        assert_ne!(buggy.evidence_digest(), control.evidence_digest());
        assert_ne!(buggy.result_digest(), control.result_digest());
        assert_ne!(
            buggy.verification_result_id(),
            control.verification_result_id()
        );
        assert_eq!(
            buggy.workload_target_revision(),
            buggy_expected
                .target_revision_for_engine(buggy.engine_sha256())
                .expect("valid observed engine SHA")
        );
        assert_eq!(buggy.command_id(), buggy_expected.command_id());
        assert_eq!(buggy.condition_id(), buggy_expected.condition_id());
        assert_eq!(
            buggy.oracle_contract_id(),
            buggy_expected.oracle_contract_id()
        );
    }

    fn assert_expectation_refused_before_replay(dir: &TestReceiptDir, expected: &RunExpectation) {
        let bytes = fs::read(dir.path().join("run.ndjson")).expect("read canonical receipt");
        let text = std::str::from_utf8(&bytes).expect("canonical receipt UTF-8");
        let engine_sha256 = crate::cooperative::current_engine_sha256().expect("engine digest");
        let mut fresh_replay_attempts = 0;

        let attempt = verify_run_inner(
            dir.path(),
            &engine_sha256,
            &bytes,
            text,
            Some(expected),
            &mut fresh_replay_attempts,
        );

        assert!(attempt.proof.is_err());
        assert_eq!(
            fresh_replay_attempts, 0,
            "an expectation mismatch must precede fresh execution"
        );
    }

    #[test]
    fn every_expectation_mismatch_returns_no_proof_before_replay() {
        let buggy_dir = write_tier1_test_receipt("demo-buggy");
        let mismatches = [
            RunExpectation::fifo("demo", TIER1_TEST_SEED, TIER1_TEST_UNIVERSES, "v0", true)
                .unwrap(),
            RunExpectation::fifo(
                "demo-buggy",
                TIER1_TEST_SEED + 1,
                TIER1_TEST_UNIVERSES,
                "v0",
                true,
            )
            .unwrap(),
            RunExpectation::fifo(
                "demo-buggy",
                TIER1_TEST_SEED,
                TIER1_TEST_UNIVERSES + 1,
                "v0",
                true,
            )
            .unwrap(),
            RunExpectation::fifo(
                "demo-buggy",
                TIER1_TEST_SEED,
                TIER1_TEST_UNIVERSES,
                "swarm",
                true,
            )
            .unwrap(),
            RunExpectation::fifo(
                "demo-buggy",
                TIER1_TEST_SEED,
                TIER1_TEST_UNIVERSES,
                "v0",
                false,
            )
            .unwrap(),
        ];
        for expected in &mismatches {
            assert_expectation_refused_before_replay(&buggy_dir, expected);
        }
    }

    #[test]
    fn fresh_verification_returns_no_proof_for_tampered_receipt() {
        let control_dir = write_tier1_test_receipt("demo");
        let expected = tier1_expectation("demo");
        let engine = crate::current_engine_path().expect("test engine path");
        let run_path = control_dir.path().join("run.ndjson");
        let mut bytes = fs::read(&run_path).expect("read canonical receipt");
        bytes.push(b'\n');
        fs::write(&run_path, bytes).expect("tamper isolated receipt");

        let result = verify_run_fresh(control_dir.path(), &engine, &expected);

        assert!(result.is_err());
    }
}
