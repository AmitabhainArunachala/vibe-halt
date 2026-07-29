//! Evaluation-contract boundary for holdout/calibration dossiers (Wave B/C R3).
//!
//! This module validates the public, synthetic dossier schema without
//! executing any target code. It is the only `vh-cli` surface that reads
//! arbitrary dossier files, so it lives next to `bundle.rs` as a declared
//! I/O boundary and carries a per-file `std::fs` exemption in the
//! determinism deny-list.
//!
//! Authority limits: this validator checks shape, state transitions, and
//! commitment/reveal consistency. It does NOT select hidden cohorts,
//! generate real secrets, award criterion-3/4 credit, or execute target
//! code.

use std::fs;
use std::path::PathBuf;

use vh_cli::receipts::{parse_line, Val};

pub const DOSSIER_SCHEMA: &str = "vibe-halt.eval-dossier.v1";
pub const MANIFEST_SCHEMA: &str = "vibe-halt.holdout-manifest.v1";
pub const COMMITMENT_DOMAIN: &str = "vh-eval-dossier-commitment-v1";
pub const MIN_SALT_LEN: usize = 32;

const REQUIRED_STRING_FIELDS: &[&str] = &[
    "dossier_id",
    "vb_id",
    "title",
    "class",
    "source_repo",
    "source_issue",
    "source_url",
    "workload",
    "oracle",
    "mechanism",
    "pre_fix_revision",
    "post_fix_revision",
    "injection_seam",
];

/// Fields bound into the reveal/canonical commitment. The commitment
/// fields themselves (`commitment_domain`, `commitment_salt`,
/// `commitment_digest`, `reveal`) and the `record`/`schema` envelope are
/// intentionally excluded so the digest is not self-referential.
const CANONICAL_FIELDS: &[&str] = &[
    "dossier_id",
    "vb_id",
    "title",
    "class",
    "source_repo",
    "source_issue",
    "source_url",
    "workload",
    "oracle",
    "mechanism",
    "pre_fix_revision",
    "post_fix_revision",
    "injection_seam",
    "evaluator_image",
    "toolchain",
    "treatment_command",
    "control_command",
    "required_facts",
    "status",
    "cohort",
    "candidate_state",
    "candidate_state_log",
    "bridge_execution",
    "fixed_control_miss",
    "acceptance_credit",
];

const VALID_PRE_COHORT: &[&str] = &["DRAFT", "NOT_ADMISSIBLE", "ADMISSIBLE"];
const VALID_COHORT: &[&str] = &["HOLDOUT", "CALIBRATION"];
const VALID_CANDIDATE: &[&str] = &["UNRUN", "AUTHORITY_BLOCKED", "DETECTED", "MISS", "INVALID"];
const VALID_BRIDGE: &[&str] = &["FORWARD_CONFIRMED", "FORWARD_NULL", "FORWARD_INVALID"];

fn get_str<'a>(fields: &'a [(String, Val)], key: &str) -> Option<&'a str> {
    fields
        .iter()
        .find(|(k, _)| k == key)
        .and_then(|(_, v)| v.as_str())
}

fn get_bool(fields: &[(String, Val)], key: &str) -> Option<bool> {
    fields
        .iter()
        .find(|(k, _)| k == key)
        .and_then(|(_, v)| match v {
            Val::B(b) => Some(*b),
            _ => None,
        })
}

/// Render the canonical dossier bytes that the reveal must expose and the
/// commitment must bind. Values are unquoted so that the canonical form is
/// not itself JSON and cannot be confused with the surrounding record.
fn canonical(fields: &[(String, Val)]) -> String {
    let mut lines = Vec::with_capacity(CANONICAL_FIELDS.len());
    for key in CANONICAL_FIELDS {
        let val = fields.iter().find(|(k, _)| k == *key).map(|(_, v)| v);
        let rendered = match val {
            Some(Val::S(s)) => s.clone(),
            Some(Val::B(true)) => "true".to_string(),
            Some(Val::B(false)) => "false".to_string(),
            Some(Val::N(n)) => n.to_string(),
            Some(Val::Null) | None => "null".to_string(),
        };
        lines.push(format!("{key}={rendered}"));
    }
    lines.join("\n")
}

fn commitment_digest(domain: &str, salt: &str, reveal: &str) -> String {
    let mut input = String::with_capacity(domain.len() + 1 + salt.len() + 1 + reveal.len());
    input.push_str(domain);
    input.push('\0');
    input.push_str(salt);
    input.push('\0');
    input.push_str(reveal);
    vh_digest::sha256_hex(input.as_bytes())
}

fn allowed_candidate_transition(from: &str, to: &str) -> bool {
    if from == to {
        return true;
    }
    matches!(
        (from, to),
        ("UNRUN", "AUTHORITY_BLOCKED")
            | ("UNRUN", "DETECTED")
            | ("UNRUN", "MISS")
            | ("UNRUN", "INVALID")
            | ("AUTHORITY_BLOCKED", "DETECTED")
            | ("AUTHORITY_BLOCKED", "MISS")
            | ("AUTHORITY_BLOCKED", "INVALID")
            | ("DETECTED", "INVALID")
            | ("MISS", "INVALID")
    )
}

/// Validate a single flat `dossier` record. Returns `Ok(())` or a list of
/// human-readable violations. This function does not read files or execute
/// targets.
fn validate_dossier(fields: &[(String, Val)]) -> Result<(), Vec<String>> {
    let mut errors: Vec<String> = Vec::new();

    if get_str(fields, "record") != Some("dossier") {
        errors.push("record must be \"dossier\"".to_string());
    }
    if get_str(fields, "schema") != Some(DOSSIER_SCHEMA) {
        errors.push(format!("schema must be \"{DOSSIER_SCHEMA}\""));
    }

    for key in REQUIRED_STRING_FIELDS {
        match get_str(fields, key) {
            Some(s) if !s.is_empty() => {}
            _ => errors.push(format!("missing or empty required string field {key:?}")),
        }
    }

    let status = get_str(fields, "status").unwrap_or("");
    if !status.is_empty() && !VALID_PRE_COHORT.contains(&status) {
        errors.push(format!("invalid status {status:?}"));
    }

    let cohort = get_str(fields, "cohort").unwrap_or("");
    if !cohort.is_empty() && !VALID_COHORT.contains(&cohort) {
        errors.push(format!("invalid cohort {cohort:?}"));
    }

    let candidate = get_str(fields, "candidate_state").unwrap_or("");
    if candidate.is_empty() {
        errors.push("missing candidate_state".to_string());
    } else if !VALID_CANDIDATE.contains(&candidate) {
        errors.push(format!("invalid candidate_state {candidate:?}"));
    }

    if status == "ADMISSIBLE" && !VALID_COHORT.contains(&cohort) {
        errors.push("ADMISSIBLE dossier must have a valid cohort".to_string());
    }

    if let Some(log) = get_str(fields, "candidate_state_log") {
        let states: Vec<&str> = log
            .split(';')
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .collect();
        if states.is_empty() {
            errors.push("candidate_state_log is empty".to_string());
        } else {
            if states.last().copied().unwrap_or("") != candidate {
                errors.push(format!(
                    "candidate_state_log ends with {:?}, but candidate_state is {candidate:?}",
                    states.last()
                ));
            }
            for s in &states {
                if !VALID_CANDIDATE.contains(s) {
                    errors.push(format!("candidate_state_log contains invalid state {s:?}"));
                }
            }
            for pair in states.windows(2) {
                if !allowed_candidate_transition(pair[0], pair[1]) {
                    errors.push(format!(
                        "append-only violation: candidate_state transition {} -> {} is not allowed",
                        pair[0], pair[1]
                    ));
                }
            }
        }
    } else {
        errors.push("missing candidate_state_log".to_string());
    }

    let bridge = get_str(fields, "bridge_execution").unwrap_or("");
    if !bridge.is_empty() && !VALID_BRIDGE.contains(&bridge) {
        errors.push(format!("invalid bridge_execution {bridge:?}"));
    }

    if cohort == "CALIBRATION" && get_bool(fields, "acceptance_credit") == Some(true) {
        errors.push("HOLDOUT credit claimed for a CALIBRATION dossier".to_string());
    }

    if !bridge.is_empty() {
        match (bridge, candidate) {
            ("FORWARD_CONFIRMED", "DETECTED") => {}
            ("FORWARD_NULL", "MISS") => {}
            ("FORWARD_INVALID", "INVALID") => {}
            _ => errors.push(format!(
                "bridge_execution {bridge:?} inconsistent with candidate_state {candidate:?}"
            )),
        }
        if bridge == "FORWARD_CONFIRMED" && get_bool(fields, "fixed_control_miss") != Some(true) {
            errors.push("FORWARD_CONFIRMED requires fixed_control_miss=true".to_string());
        }
        for key in [
            "evaluator_image",
            "toolchain",
            "treatment_command",
            "control_command",
        ] {
            if matches!(get_str(fields, key), None | Some("")) {
                errors.push(format!("bridge_execution set but {key} is missing/empty"));
            }
        }
    } else if get_bool(fields, "fixed_control_miss") == Some(true) {
        errors.push("fixed_control_miss=true without a bridge_execution".to_string());
    }

    let domain = get_str(fields, "commitment_domain").unwrap_or("");
    let salt = get_str(fields, "commitment_salt").unwrap_or("");
    let digest = get_str(fields, "commitment_digest").unwrap_or("");
    let reveal = get_str(fields, "reveal").unwrap_or("");

    if domain != COMMITMENT_DOMAIN {
        errors.push(format!("commitment_domain must be {COMMITMENT_DOMAIN:?}"));
    }
    if !salt.is_empty() && salt.len() < MIN_SALT_LEN {
        errors.push(format!(
            "commitment_salt too short ({} < {MIN_SALT_LEN})",
            salt.len()
        ));
    }

    let expected_reveal = canonical(fields);
    if !reveal.is_empty() && reveal != expected_reveal {
        errors.push(
            "reveal does not match canonical dossier fields (fields changed after commitment)"
                .to_string(),
        );
    }
    if !digest.is_empty() && !reveal.is_empty() {
        let expected_digest = commitment_digest(domain, salt, reveal);
        if digest != expected_digest {
            errors.push(
                "commitment_digest does not recompute from domain, salt, and reveal".to_string(),
            );
        }
    }

    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors)
    }
}

/// `vh eval-validate --dossier PATH`
///
/// Exit codes: 0 = all dossier records VALID, 1 = at least one dossier
/// INVALID, 2 = usage or unreadable file.
pub fn cmd_eval_validate(args: &[String], usage: &str) -> i32 {
    let mut path: Option<String> = None;
    let mut it = args.iter();
    while let Some(arg) = it.next() {
        match arg.as_str() {
            "--dossier" => match it.next() {
                Some(v) => path = Some(v.clone()),
                None => {
                    eprintln!("error: --dossier requires a value\n\n{usage}");
                    return 2;
                }
            },
            other => {
                eprintln!("error: unknown argument {other:?}\n\n{usage}");
                return 2;
            }
        }
    }

    let Some(path) = path else {
        eprintln!("error: --dossier PATH required\n\n{usage}");
        return 2;
    };

    let text = match fs::read_to_string(PathBuf::from(&path)) {
        Ok(t) => t,
        Err(e) => {
            eprintln!("error: cannot read dossier {path}: {e}\n\n{usage}");
            return 2;
        }
    };

    let mut dossier_count: u64 = 0;
    let mut invalid_count: u64 = 0;
    let mut manifest_seen = false;

    for (line_no, line) in text.lines().enumerate() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        let fields = match parse_line(line) {
            Ok(f) => f,
            Err(e) => {
                eprintln!("error: line {}: {e}", line_no + 1);
                return 2;
            }
        };
        let kind = fields
            .iter()
            .find(|(k, _)| k == "record")
            .and_then(|(_, v)| v.as_str())
            .unwrap_or("");
        match kind {
            "manifest" => {
                let schema = fields
                    .iter()
                    .find(|(k, _)| k == "schema")
                    .and_then(|(_, v)| v.as_str())
                    .unwrap_or("");
                if schema != MANIFEST_SCHEMA {
                    eprintln!("error: manifest schema must be {MANIFEST_SCHEMA:?}");
                    return 2;
                }
                manifest_seen = true;
            }
            "dossier" => {
                dossier_count += 1;
                match validate_dossier(&fields) {
                    Ok(()) => {}
                    Err(errors) => {
                        invalid_count += 1;
                        eprintln!("dossier on line {}: INVALID", line_no + 1);
                        for msg in errors {
                            eprintln!("  - {msg}");
                        }
                    }
                }
            }
            other => {
                eprintln!("error: line {}: unknown record kind {other:?}", line_no + 1);
                return 2;
            }
        }
    }

    if dossier_count == 0 {
        eprintln!("error: no dossier records found in {path}");
        return 2;
    }

    println!("eval-validate: {dossier_count} dossier(s) checked");
    if invalid_count == 0 {
        println!("verdict: VALID");
        if manifest_seen {
            println!("manifest: {MANIFEST_SCHEMA}");
        }
        0
    } else {
        println!("verdict: INVALID ({invalid_count} dossier(s))");
        1
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use super::*;
    use vh_cli::receipts::Val;

    fn base_dossier() -> Vec<(String, Val)> {
        let mut fields: BTreeMap<String, Val> = BTreeMap::new();
        fields.insert("record".into(), Val::S("dossier".into()));
        fields.insert("schema".into(), Val::S(DOSSIER_SCHEMA.into()));
        fields.insert("dossier_id".into(), Val::S("VB-008-6491".into()));
        fields.insert("vb_id".into(), Val::S("VB-008".into()));
        fields.insert(
            "title".into(),
            Val::S("unvalidated checkpoint (langgraph#6491)".into()),
        );
        fields.insert("class".into(), Val::S("dirty-read".into()));
        fields.insert(
            "source_repo".into(),
            Val::S("langchain-ai/langgraph".into()),
        );
        fields.insert("source_issue".into(), Val::S("6491".into()));
        fields.insert(
            "source_url".into(),
            Val::S("https://github.com/langchain-ai/langgraph/issues/6491".into()),
        );
        fields.insert(
            "workload".into(),
            Val::S("corpus-unvalidated-checkpoint".into()),
        );
        fields.insert("oracle".into(), Val::S("checkpoint_recoverable".into()));
        fields.insert(
            "mechanism".into(),
            Val::S("write-side accepts invalid state; read-side rejects it".into()),
        );
        fields.insert(
            "pre_fix_revision".into(),
            Val::S("SYNTHETIC-PRE-FIX-PLACEHOLDER".into()),
        );
        fields.insert(
            "post_fix_revision".into(),
            Val::S("SYNTHETIC-FIXED-CONTROL-PLACEHOLDER".into()),
        );
        fields.insert(
            "injection_seam".into(),
            Val::S("SYNTHETIC-INJECTION-SEAM-PLACEHOLDER".into()),
        );
        fields.insert(
            "evaluator_image".into(),
            Val::S("SYNTHETIC-EVALUATOR-IMAGE-NOT-EXECUTED".into()),
        );
        fields.insert(
            "toolchain".into(),
            Val::S("SYNTHETIC-TOOLCHAIN-NOT-EXECUTED".into()),
        );
        fields.insert(
            "treatment_command".into(),
            Val::S("SYNTHETIC-TREATMENT-COMMAND-NOT-EXECUTED".into()),
        );
        fields.insert(
            "control_command".into(),
            Val::S("SYNTHETIC-CONTROL-COMMAND-NOT-EXECUTED".into()),
        );
        fields.insert(
            "required_facts".into(),
            Val::S("SYNTHETIC-REQUIRED-FACTS-PLACEHOLDER".into()),
        );
        fields.insert("status".into(), Val::S("ADMISSIBLE".into()));
        fields.insert("cohort".into(), Val::S("CALIBRATION".into()));
        fields.insert("candidate_state".into(), Val::S("UNRUN".into()));
        fields.insert("candidate_state_log".into(), Val::S("UNRUN".into()));
        fields.insert("bridge_execution".into(), Val::Null);
        fields.insert("fixed_control_miss".into(), Val::B(false));
        fields.insert("acceptance_credit".into(), Val::B(false));
        fields.insert("commitment_domain".into(), Val::S(COMMITMENT_DOMAIN.into()));
        fields.insert(
            "commitment_salt".into(),
            Val::S("synthetic-public-salt-vb008-00000000000000000000000000000000".into()),
        );

        let mut as_vec: Vec<(String, Val)> = fields.into_iter().collect();
        // Sort so canonical is deterministic and the reveal can be produced.
        as_vec.sort_by(|a, b| a.0.cmp(&b.0));

        let reveal = canonical(&as_vec);
        let digest = commitment_digest(
            COMMITMENT_DOMAIN,
            "synthetic-public-salt-vb008-00000000000000000000000000000000",
            &reveal,
        );
        as_vec.push(("reveal".into(), Val::S(reveal)));
        as_vec.push(("commitment_digest".into(), Val::S(digest)));
        as_vec
    }

    fn set_field(fields: &mut [(String, Val)], key: &str, value: Val) {
        for (k, v) in fields.iter_mut() {
            if k == key {
                *v = value;
                return;
            }
        }
        panic!("field {key} not found");
    }

    fn recompute_commitment(fields: &mut [(String, Val)]) {
        let domain = get_str(fields, "commitment_domain").unwrap_or(COMMITMENT_DOMAIN);
        let salt = get_str(fields, "commitment_salt").unwrap_or("");
        let reveal = canonical(fields);
        let digest = commitment_digest(domain, salt, &reveal);
        set_field(fields, "reveal", Val::S(reveal));
        set_field(fields, "commitment_digest", Val::S(digest));
    }

    #[test]
    fn valid_calibration_dossier_passes() {
        let fields = base_dossier();
        assert!(validate_dossier(&fields).is_ok());
    }

    #[test]
    fn missing_required_field_fails() {
        let mut fields = base_dossier();
        set_field(&mut fields, "oracle", Val::S("".into()));
        assert!(validate_dossier(&fields).is_err());
    }

    #[test]
    fn cohort_changed_after_commitment_fails() {
        // Changing cohort after the reveal was computed invalidates the reveal/canonical match.
        let mut fields = base_dossier();
        set_field(&mut fields, "cohort", Val::S("HOLDOUT".into()));
        let err = validate_dossier(&fields).unwrap_err();
        assert!(err.iter().any(|m| m.contains("reveal does not match")));
    }

    #[test]
    fn append_only_transition_detected_to_miss_fails() {
        let mut fields = base_dossier();
        set_field(&mut fields, "candidate_state", Val::S("MISS".into()));
        set_field(
            &mut fields,
            "candidate_state_log",
            Val::S("UNRUN;DETECTED;MISS".into()),
        );
        recompute_commitment(&mut fields);
        let err = validate_dossier(&fields).unwrap_err();
        assert!(err
            .iter()
            .any(|m| m.contains("DETECTED -> MISS") || m.contains("append-only violation")));
    }

    #[test]
    fn holdout_credit_on_calibration_fails() {
        let mut fields = base_dossier();
        set_field(&mut fields, "acceptance_credit", Val::B(true));
        let err = validate_dossier(&fields).unwrap_err();
        assert!(err.iter().any(|m| m.contains("HOLDOUT credit")));
    }

    #[test]
    fn insufficient_salt_entropy_fails() {
        let mut fields = base_dossier();
        set_field(&mut fields, "commitment_salt", Val::S("short".into()));
        recompute_commitment(&mut fields);
        let err = validate_dossier(&fields).unwrap_err();
        assert!(err.iter().any(|m| m.contains("too short")));
    }

    #[test]
    fn domain_separation_required() {
        let mut fields = base_dossier();
        set_field(
            &mut fields,
            "commitment_domain",
            Val::S("wrong-domain".into()),
        );
        recompute_commitment(&mut fields);
        let err = validate_dossier(&fields).unwrap_err();
        assert!(err.iter().any(|m| m.contains("commitment_domain")));
    }

    #[test]
    fn bad_digest_fails() {
        let mut fields = base_dossier();
        set_field(
            &mut fields,
            "commitment_digest",
            Val::S("0000000000000000000000000000000000000000000000000000000000000000".into()),
        );
        let err = validate_dossier(&fields).unwrap_err();
        assert!(err.iter().any(|m| m.contains("does not recompute")));
    }

    #[test]
    fn reveal_mismatch_fails() {
        let mut fields = base_dossier();
        set_field(&mut fields, "reveal", Val::S("tampered".into()));
        let err = validate_dossier(&fields).unwrap_err();
        assert!(err.iter().any(|m| m.contains("reveal does not match")));
    }

    #[test]
    fn forward_confirmed_requires_detection_and_control_miss() {
        let mut fields = base_dossier();
        set_field(&mut fields, "candidate_state", Val::S("DETECTED".into()));
        set_field(
            &mut fields,
            "candidate_state_log",
            Val::S("UNRUN;DETECTED".into()),
        );
        set_field(
            &mut fields,
            "bridge_execution",
            Val::S("FORWARD_CONFIRMED".into()),
        );
        set_field(&mut fields, "fixed_control_miss", Val::B(false));
        recompute_commitment(&mut fields);
        let err = validate_dossier(&fields).unwrap_err();
        assert!(err.iter().any(|m| m.contains("fixed_control_miss=true")));
    }

    #[test]
    fn frozen_evaluator_and_commands_required_when_executed() {
        let mut fields = base_dossier();
        set_field(&mut fields, "candidate_state", Val::S("DETECTED".into()));
        set_field(
            &mut fields,
            "candidate_state_log",
            Val::S("UNRUN;DETECTED".into()),
        );
        set_field(
            &mut fields,
            "bridge_execution",
            Val::S("FORWARD_CONFIRMED".into()),
        );
        set_field(&mut fields, "fixed_control_miss", Val::B(true));
        set_field(&mut fields, "evaluator_image", Val::S("".into()));
        recompute_commitment(&mut fields);
        let err = validate_dossier(&fields).unwrap_err();
        assert!(err.iter().any(|m| m.contains("evaluator_image")));
    }

    #[test]
    fn forward_confirmed_with_detection_and_control_miss_passes() {
        let mut fields = base_dossier();
        set_field(&mut fields, "candidate_state", Val::S("DETECTED".into()));
        set_field(
            &mut fields,
            "candidate_state_log",
            Val::S("UNRUN;DETECTED".into()),
        );
        set_field(
            &mut fields,
            "bridge_execution",
            Val::S("FORWARD_CONFIRMED".into()),
        );
        set_field(&mut fields, "fixed_control_miss", Val::B(true));
        recompute_commitment(&mut fields);
        assert!(validate_dossier(&fields).is_ok());
    }
}
