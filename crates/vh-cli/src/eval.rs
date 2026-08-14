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

use std::collections::BTreeSet;
use std::fs;
use std::path::PathBuf;

use vh_cli::receipts::{parse_line, Val};

pub const DOSSIER_SCHEMA: &str = "vibe-halt.eval-dossier.v1";
pub const MANIFEST_SCHEMA: &str = "vibe-halt.holdout-manifest.v1";
pub const COMMITMENT_DOMAIN: &str = "vh-eval-dossier-commitment-v1";
pub const MIN_SALT_LEN: usize = 32;

const DOSSIER_STRING_FIELDS: &[&str] = &[
    "record",
    "schema",
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
    "commitment_domain",
    "commitment_salt",
    "reveal",
    "commitment_digest",
];

const DOSSIER_BOOL_FIELDS: &[&str] = &["fixed_control_miss", "acceptance_credit"];

const DOSSIER_FIELDS: &[&str] = &[
    "record",
    "schema",
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
    "commitment_domain",
    "commitment_salt",
    "reveal",
    "commitment_digest",
];

const MANIFEST_FIELDS: &[&str] = &["record", "schema", "name"];

/// A bridge result names an execution. None of these identity-bearing
/// fields may retain the public calibration sentinels when a bridge is set.
const EXECUTION_IDENTITY_FIELDS: &[&str] = &[
    "source_repo",
    "source_issue",
    "source_url",
    "workload",
    "oracle",
    "pre_fix_revision",
    "post_fix_revision",
    "injection_seam",
    "evaluator_image",
    "toolchain",
    "treatment_command",
    "control_command",
    "required_facts",
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

fn get_val<'a>(fields: &'a [(String, Val)], key: &str) -> Option<&'a Val> {
    fields
        .iter()
        .find(|(k, _)| k == key)
        .map(|(_, value)| value)
}

fn get_str<'a>(fields: &'a [(String, Val)], key: &str) -> Option<&'a str> {
    get_val(fields, key).and_then(Val::as_str)
}

fn get_bool(fields: &[(String, Val)], key: &str) -> Option<bool> {
    get_val(fields, key).and_then(|v| match v {
        Val::B(b) => Some(*b),
        _ => None,
    })
}

fn validate_key_set(fields: &[(String, Val)], allowed: &[&str], errors: &mut Vec<String>) {
    for key in allowed {
        match fields
            .iter()
            .filter(|(candidate, _)| candidate == key)
            .count()
        {
            0 => errors.push(format!("missing required field {key:?}")),
            1 => {}
            count => errors.push(format!("duplicate field {key:?} ({count} occurrences)")),
        }
    }
    for (key, _) in fields {
        if !allowed.contains(&key.as_str()) {
            errors.push(format!("unknown field {key:?}"));
        }
    }
}

fn validate_dossier_types(fields: &[(String, Val)], errors: &mut Vec<String>) {
    for key in DOSSIER_STRING_FIELDS {
        match get_val(fields, key) {
            Some(Val::S(value)) if !value.is_empty() => {
                // `reveal` intentionally contains the canonical newlines.
                // Every source field feeding `key=value\n` must remain a
                // single control-free line or two distinct dossiers can
                // collapse to identical commitment bytes.
                if *key != "reveal" && value.chars().any(char::is_control) {
                    errors.push(format!(
                        "field {key:?} contains a control character forbidden by canonical framing"
                    ));
                }
            }
            Some(Val::S(_)) => errors.push(format!("field {key:?} must not be empty")),
            Some(_) => errors.push(format!("field {key:?} must be a string")),
            None => {}
        }
    }
    for key in DOSSIER_BOOL_FIELDS {
        match get_val(fields, key) {
            Some(Val::B(_)) | None => {}
            Some(_) => errors.push(format!("field {key:?} must be a boolean")),
        }
    }
    match get_val(fields, "bridge_execution") {
        Some(Val::S(_)) | Some(Val::Null) | None => {}
        Some(_) => {
            errors.push("field \"bridge_execution\" must be a string enum or null".to_string())
        }
    }
}

fn validate_manifest(fields: &[(String, Val)]) -> Result<(), Vec<String>> {
    let mut errors = Vec::new();
    validate_key_set(fields, MANIFEST_FIELDS, &mut errors);
    for key in MANIFEST_FIELDS {
        match get_val(fields, key) {
            Some(Val::S(value)) if !value.is_empty() => {}
            Some(Val::S(_)) => errors.push(format!("manifest field {key:?} must not be empty")),
            Some(_) => errors.push(format!("manifest field {key:?} must be a string")),
            None => {}
        }
    }
    if get_str(fields, "record") != Some("manifest") {
        errors.push("manifest record must be \"manifest\"".to_string());
    }
    if get_str(fields, "schema") != Some(MANIFEST_SCHEMA) {
        errors.push(format!("manifest schema must be {MANIFEST_SCHEMA:?}"));
    }
    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors)
    }
}

fn has_calibration_marker(value: &str) -> bool {
    let upper = value.to_ascii_uppercase();
    upper.contains("SYNTHETIC-") || upper.contains("NOT-EXECUTED")
}

fn is_lower_sha256(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
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
    validate_key_set(fields, DOSSIER_FIELDS, &mut errors);
    validate_dossier_types(fields, &mut errors);

    if get_str(fields, "record") != Some("dossier") {
        errors.push("record must be \"dossier\"".to_string());
    }
    if get_str(fields, "schema") != Some(DOSSIER_SCHEMA) {
        errors.push(format!("schema must be \"{DOSSIER_SCHEMA}\""));
    }

    let status = get_str(fields, "status").unwrap_or("");
    if !VALID_PRE_COHORT.contains(&status) {
        errors.push(format!("invalid status {status:?}"));
    }

    let cohort = get_str(fields, "cohort").unwrap_or("");
    if !VALID_COHORT.contains(&cohort) {
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

    let bridge = match get_val(fields, "bridge_execution") {
        Some(Val::S(value)) => Some(value.as_str()),
        Some(Val::Null) | Some(_) | None => None,
    };
    if let Some(value) = bridge {
        if !VALID_BRIDGE.contains(&value) {
            errors.push(format!("invalid bridge_execution {value:?}"));
        }
    }

    if cohort == "CALIBRATION" && get_bool(fields, "acceptance_credit") == Some(true) {
        errors.push("HOLDOUT credit claimed for a CALIBRATION dossier".to_string());
    }

    if let Some(bridge) = bridge {
        match (bridge, candidate) {
            ("FORWARD_CONFIRMED", "DETECTED") => {}
            ("FORWARD_NULL", "MISS") => {}
            ("FORWARD_INVALID", "INVALID") => {}
            _ => errors.push(format!(
                "bridge_execution {bridge:?} inconsistent with candidate_state {candidate:?}"
            )),
        }
        match (bridge, get_bool(fields, "fixed_control_miss")) {
            ("FORWARD_CONFIRMED", Some(true)) => {}
            ("FORWARD_CONFIRMED", _) => {
                errors.push("FORWARD_CONFIRMED requires fixed_control_miss=true".to_string());
            }
            (_, Some(true)) => errors
                .push("fixed_control_miss=true is valid only for FORWARD_CONFIRMED".to_string()),
            _ => {}
        }
        if cohort == "CALIBRATION" {
            errors.push("CALIBRATION dossier cannot set bridge_execution".to_string());
        }
        for key in EXECUTION_IDENTITY_FIELDS {
            if let Some(value) = get_str(fields, key) {
                if has_calibration_marker(value) {
                    errors.push(format!(
                        "bridge_execution cannot promote synthetic/not-executed identity field {key:?}"
                    ));
                }
            }
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
        errors.push("fixed_control_miss=true is valid only for FORWARD_CONFIRMED".to_string());
    }

    let domain = get_str(fields, "commitment_domain").unwrap_or("");
    let salt = get_str(fields, "commitment_salt").unwrap_or("");
    let digest = get_str(fields, "commitment_digest").unwrap_or("");
    let reveal = get_str(fields, "reveal").unwrap_or("");

    if domain != COMMITMENT_DOMAIN {
        errors.push(format!("commitment_domain must be {COMMITMENT_DOMAIN:?}"));
    }
    if salt.len() < MIN_SALT_LEN {
        errors.push(format!(
            "commitment_salt too short ({} < {MIN_SALT_LEN})",
            salt.len()
        ));
    }
    if !is_lower_sha256(digest) {
        errors.push("commitment_digest must be exactly 64 lowercase hexadecimal bytes".to_string());
    }

    let expected_reveal = canonical(fields);
    if reveal != expected_reveal {
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

struct DocumentValidation {
    dossier_count: u64,
    manifest_count: u64,
    dossier_errors: Vec<(usize, Vec<String>)>,
    document_errors: Vec<String>,
}

impl DocumentValidation {
    fn is_valid(&self) -> bool {
        self.dossier_errors.is_empty() && self.document_errors.is_empty()
    }
}

fn validate_document(text: &str) -> DocumentValidation {
    let mut dossier_count: u64 = 0;
    let mut record_count: u64 = 0;
    let mut manifest_count: u64 = 0;
    let mut dossier_ids = BTreeSet::new();
    let mut dossier_errors = Vec::new();
    let mut document_errors = Vec::new();

    for (line_no, line) in text.lines().enumerate() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        let record_index = record_count;
        record_count += 1;
        let fields = match parse_line(line) {
            Ok(fields) => fields,
            Err(error) => {
                document_errors.push(format!("line {}: {error}", line_no + 1));
                continue;
            }
        };
        let kind = get_str(&fields, "record").unwrap_or("");
        match kind {
            "manifest" => {
                manifest_count += 1;
                if record_index != 0 {
                    document_errors.push(format!(
                        "manifest on line {} must be the first non-empty record",
                        line_no + 1
                    ));
                }
                if manifest_count > 1 {
                    document_errors.push(format!(
                        "manifest on line {} violates exact-one manifest multiplicity",
                        line_no + 1
                    ));
                }
                if let Err(errors) = validate_manifest(&fields) {
                    for error in errors {
                        document_errors.push(format!("manifest on line {}: {error}", line_no + 1));
                    }
                }
            }
            "dossier" => {
                dossier_count += 1;
                if let Some(dossier_id) = get_str(&fields, "dossier_id") {
                    if !dossier_ids.insert(dossier_id.to_string()) {
                        document_errors.push(format!(
                            "duplicate dossier_id {dossier_id:?} on line {}",
                            line_no + 1
                        ));
                    }
                }
                if let Err(errors) = validate_dossier(&fields) {
                    dossier_errors.push((line_no + 1, errors));
                }
            }
            other => {
                document_errors.push(format!(
                    "line {}: unknown record kind {other:?}",
                    line_no + 1
                ));
            }
        }
    }

    if dossier_count == 0 {
        document_errors.push("no dossier records found".to_string());
    }
    if manifest_count == 0 && dossier_count != 1 {
        document_errors.push(format!(
            "a manifest-less file must contain exactly one dossier, found {dossier_count}"
        ));
    }

    DocumentValidation {
        dossier_count,
        manifest_count,
        dossier_errors,
        document_errors,
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

    let validation = validate_document(&text);

    for (line_no, errors) in &validation.dossier_errors {
        eprintln!("dossier on line {line_no}: INVALID");
        for error in errors {
            eprintln!("  - {error}");
        }
    }
    for error in &validation.document_errors {
        eprintln!("document: INVALID: {error}");
    }

    println!(
        "eval-validate: {} dossier(s) checked",
        validation.dossier_count
    );
    if validation.is_valid() {
        println!("verdict: VALID");
        if validation.manifest_count == 1 {
            println!("manifest: {MANIFEST_SCHEMA}");
        }
        0
    } else {
        println!(
            "verdict: INVALID ({invalid_count} dossier(s), {} document violation(s))",
            validation.document_errors.len(),
            invalid_count = validation.dossier_errors.len()
        );
        1
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use super::*;
    use vh_cli::receipts::{json_escape, Val};

    const DOSSIER_KEYS: &[&str] = &[
        "record",
        "schema",
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
        "commitment_domain",
        "commitment_salt",
        "reveal",
        "commitment_digest",
    ];

    const DOSSIER_STRING_KEYS: &[&str] = &[
        "record",
        "schema",
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
        "commitment_domain",
        "commitment_salt",
        "reveal",
        "commitment_digest",
    ];

    const SYNTHETIC_IDENTITY_KEYS: &[&str] = &[
        "source_repo",
        "source_issue",
        "source_url",
        "workload",
        "oracle",
        "pre_fix_revision",
        "post_fix_revision",
        "injection_seam",
        "evaluator_image",
        "toolchain",
        "treatment_command",
        "control_command",
        "required_facts",
    ];

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

    fn remove_field(fields: &mut Vec<(String, Val)>, key: &str) {
        fields.retain(|(candidate, _)| candidate != key);
    }

    fn real_dossier() -> Vec<(String, Val)> {
        let mut fields = base_dossier();
        for (key, value) in [
            (
                "pre_fix_revision",
                "1111111111111111111111111111111111111111",
            ),
            (
                "post_fix_revision",
                "2222222222222222222222222222222222222222",
            ),
            ("injection_seam", "fixture::checkpoint::after_write"),
            (
                "evaluator_image",
                "sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
            ),
            ("toolchain", "rustc-1.89.0-x86_64-unknown-linux-gnu"),
            ("treatment_command", "vh-local-fixture treatment"),
            ("control_command", "vh-local-fixture fixed-control"),
            ("required_facts", "treatment_outcome;fixed_control_outcome"),
            ("cohort", "HOLDOUT"),
            (
                "commitment_salt",
                "public-shape-salt-vb008-00000000000000000000000000000000",
            ),
        ] {
            set_field(&mut fields, key, Val::S(value.into()));
        }
        recompute_commitment(&mut fields);
        fields
    }

    fn set_candidate(fields: &mut [(String, Val)], candidate: &str) {
        let log = match candidate {
            "UNRUN" => "UNRUN",
            "AUTHORITY_BLOCKED" => "UNRUN;AUTHORITY_BLOCKED",
            "DETECTED" => "UNRUN;DETECTED",
            "MISS" => "UNRUN;MISS",
            "INVALID" => "UNRUN;INVALID",
            other => panic!("unexpected candidate state {other}"),
        };
        set_field(fields, "candidate_state", Val::S(candidate.into()));
        set_field(fields, "candidate_state_log", Val::S(log.into()));
    }

    fn render_record(fields: &[(String, Val)]) -> String {
        let rendered: Vec<String> = fields
            .iter()
            .map(|(key, value)| {
                let value = match value {
                    Val::S(s) => format!("\"{}\"", json_escape(s)),
                    Val::N(n) => n.to_string(),
                    Val::B(b) => b.to_string(),
                    Val::Null => "null".to_string(),
                };
                format!("\"{}\":{value}", json_escape(key))
            })
            .collect();
        format!("{{{}}}", rendered.join(","))
    }

    fn validation_exit(text: &str) -> i32 {
        if validate_document(text).is_valid() {
            0
        } else {
            1
        }
    }

    fn manifest() -> Vec<(String, Val)> {
        vec![
            ("record".into(), Val::S("manifest".into())),
            ("schema".into(), Val::S(MANIFEST_SCHEMA.into())),
            ("name".into(), Val::S("wave-b-calibration-manifest".into())),
        ]
    }

    #[test]
    fn valid_calibration_dossier_passes() {
        let fields = base_dossier();
        assert!(validate_dossier(&fields).is_ok());
    }

    #[test]
    fn every_dossier_schema_field_is_required() {
        let base = base_dossier();
        let mut accepted_missing = Vec::new();
        for key in DOSSIER_KEYS {
            let mut fields = base.clone();
            remove_field(&mut fields, key);
            if CANONICAL_FIELDS.contains(key) {
                recompute_commitment(&mut fields);
            }
            if validate_dossier(&fields).is_ok() {
                accepted_missing.push(*key);
            }
        }
        assert!(
            accepted_missing.is_empty(),
            "missing fields accepted: {accepted_missing:?}"
        );
    }

    #[test]
    fn duplicate_and_unknown_dossier_fields_fail() {
        let base = base_dossier();
        let mut accepted_duplicates = Vec::new();
        for key in DOSSIER_KEYS {
            let mut fields = base.clone();
            let value = fields
                .iter()
                .find(|(candidate, _)| candidate == key)
                .map(|(_, value)| value.clone())
                .unwrap();
            fields.push(((*key).into(), value));
            if validate_dossier(&fields).is_ok() {
                accepted_duplicates.push(*key);
            }
        }
        assert!(
            accepted_duplicates.is_empty(),
            "duplicate fields accepted: {accepted_duplicates:?}"
        );

        let mut fields = base;
        fields.push(("future_extension".into(), Val::B(true)));
        assert!(validate_dossier(&fields).is_err(), "unknown field accepted");
    }

    #[test]
    fn dossier_fields_have_exact_types() {
        let base = base_dossier();
        let mut accepted_wrong_types = Vec::new();
        for key in DOSSIER_STRING_KEYS {
            let mut fields = base.clone();
            set_field(&mut fields, key, Val::N(7));
            if CANONICAL_FIELDS.contains(key) {
                recompute_commitment(&mut fields);
            }
            if validate_dossier(&fields).is_ok() {
                accepted_wrong_types.push(*key);
            }
        }
        for key in ["fixed_control_miss", "acceptance_credit"] {
            for value in [Val::Null, Val::N(0), Val::S("false".into())] {
                let mut fields = base.clone();
                set_field(&mut fields, key, value);
                recompute_commitment(&mut fields);
                if validate_dossier(&fields).is_ok() {
                    accepted_wrong_types.push(key);
                }
            }
        }
        for value in [Val::B(false), Val::N(0)] {
            let mut fields = base.clone();
            set_field(&mut fields, "bridge_execution", value);
            recompute_commitment(&mut fields);
            if validate_dossier(&fields).is_ok() {
                accepted_wrong_types.push("bridge_execution");
            }
        }
        assert!(
            accepted_wrong_types.is_empty(),
            "wrong field types accepted: {accepted_wrong_types:?}"
        );
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
    fn canonical_source_fields_reject_newline_boundary_collisions() {
        let mut left = base_dossier();
        set_field(&mut left, "title", Val::S("alpha\nclass=beta".into()));
        set_field(&mut left, "class", Val::S("gamma".into()));
        recompute_commitment(&mut left);

        let mut right = base_dossier();
        set_field(&mut right, "title", Val::S("alpha".into()));
        set_field(&mut right, "class", Val::S("beta\nclass=gamma".into()));
        recompute_commitment(&mut right);

        assert_eq!(get_str(&left, "reveal"), get_str(&right, "reveal"));
        assert_eq!(
            get_str(&left, "commitment_digest"),
            get_str(&right, "commitment_digest")
        );
        for fields in [&left, &right] {
            let errors = validate_dossier(fields).unwrap_err();
            assert!(
                errors
                    .iter()
                    .any(|error| error.contains("forbidden by canonical framing")),
                "{errors:?}"
            );
        }
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
        let mut fields = real_dossier();
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

    #[test]
    fn bridge_candidate_control_matrix_is_exact() {
        let candidates = ["UNRUN", "AUTHORITY_BLOCKED", "DETECTED", "MISS", "INVALID"];
        let bridges = [
            None,
            Some("FORWARD_CONFIRMED"),
            Some("FORWARD_NULL"),
            Some("FORWARD_INVALID"),
        ];
        let mut mismatches = Vec::new();
        for candidate in candidates {
            for bridge in bridges {
                for fixed_control_miss in [false, true] {
                    let mut fields = real_dossier();
                    set_candidate(&mut fields, candidate);
                    set_field(
                        &mut fields,
                        "bridge_execution",
                        bridge.map_or(Val::Null, |value| Val::S(value.into())),
                    );
                    set_field(
                        &mut fields,
                        "fixed_control_miss",
                        Val::B(fixed_control_miss),
                    );
                    recompute_commitment(&mut fields);

                    let expected = matches!(
                        (candidate, bridge, fixed_control_miss),
                        (_, None, false)
                            | ("DETECTED", Some("FORWARD_CONFIRMED"), true)
                            | ("MISS", Some("FORWARD_NULL"), false)
                            | ("INVALID", Some("FORWARD_INVALID"), false)
                    );
                    let actual = validate_dossier(&fields).is_ok();
                    if actual != expected {
                        mismatches.push((candidate, bridge, fixed_control_miss, actual));
                    }
                }
            }
        }
        assert!(mismatches.is_empty(), "matrix mismatches: {mismatches:?}");
    }

    #[test]
    fn synthetic_calibration_positive_cannot_promote() {
        let mut fields = base_dossier();
        set_candidate(&mut fields, "DETECTED");
        set_field(
            &mut fields,
            "bridge_execution",
            Val::S("FORWARD_CONFIRMED".into()),
        );
        set_field(&mut fields, "fixed_control_miss", Val::B(true));
        recompute_commitment(&mut fields);
        assert!(validate_dossier(&fields).is_err());
    }

    #[test]
    fn calibration_cohort_cannot_promote_real_shaped_identity() {
        let mut fields = real_dossier();
        set_field(&mut fields, "cohort", Val::S("CALIBRATION".into()));
        set_candidate(&mut fields, "MISS");
        set_field(
            &mut fields,
            "bridge_execution",
            Val::S("FORWARD_NULL".into()),
        );
        recompute_commitment(&mut fields);
        assert!(validate_dossier(&fields).is_err());
    }

    #[test]
    fn synthetic_or_not_executed_identity_cannot_promote() {
        let mut accepted = Vec::new();
        for key in SYNTHETIC_IDENTITY_KEYS {
            for marker in ["SYNTHETIC-IDENTITY", "identity-NOT-EXECUTED"] {
                let mut fields = real_dossier();
                set_field(&mut fields, key, Val::S(marker.into()));
                set_candidate(&mut fields, "DETECTED");
                set_field(
                    &mut fields,
                    "bridge_execution",
                    Val::S("FORWARD_CONFIRMED".into()),
                );
                set_field(&mut fields, "fixed_control_miss", Val::B(true));
                recompute_commitment(&mut fields);
                if validate_dossier(&fields).is_ok() {
                    accepted.push((*key, marker));
                }
            }
        }
        assert!(
            accepted.is_empty(),
            "synthetic identities accepted: {accepted:?}"
        );
    }

    #[test]
    fn manifest_shape_is_exact() {
        let dossier = render_record(&base_dossier());
        let mut accepted = Vec::new();

        let mut missing = manifest();
        remove_field(&mut missing, "name");
        if validation_exit(&format!("{}\n{dossier}\n", render_record(&missing))) == 0 {
            accepted.push("missing name");
        }

        let mut duplicate = manifest();
        duplicate.push(("name".into(), Val::S("duplicate".into())));
        if validation_exit(&format!("{}\n{dossier}\n", render_record(&duplicate))) == 0 {
            accepted.push("duplicate name");
        }

        let mut unknown = manifest();
        unknown.push(("extra".into(), Val::B(true)));
        if validation_exit(&format!("{}\n{dossier}\n", render_record(&unknown))) == 0 {
            accepted.push("unknown field");
        }

        let mut wrong_type = manifest();
        set_field(&mut wrong_type, "name", Val::N(1));
        if validation_exit(&format!("{}\n{dossier}\n", render_record(&wrong_type))) == 0 {
            accepted.push("wrong name type");
        }

        assert!(
            accepted.is_empty(),
            "invalid manifests accepted: {accepted:?}"
        );
    }

    #[test]
    fn readable_structural_failures_are_invalid_not_usage_errors() {
        let manifest_only = format!("{}\n", render_record(&manifest()));
        assert_eq!(validation_exit(&manifest_only), 1);
        assert_eq!(validation_exit("{not-json}\n"), 1);
        assert_eq!(validation_exit("{\"record\":1,\"schema\":\"x\"}\n"), 1);
    }

    #[test]
    fn manifest_multiplicity_order_and_dossier_ids_are_exact() {
        let manifest = render_record(&manifest());
        let first = base_dossier();
        let first_line = render_record(&first);
        let mut second = real_dossier();
        set_field(&mut second, "dossier_id", Val::S("VB-010-7361".into()));
        recompute_commitment(&mut second);
        let second_line = render_record(&second);

        let invalid_documents = [
            format!("{first_line}\n{manifest}\n"),
            format!("{manifest}\n{manifest}\n{first_line}\n"),
            format!("{first_line}\n{second_line}\n"),
            format!("{manifest}\n{first_line}\n{first_line}\n"),
        ];
        let accepted: Vec<usize> = invalid_documents
            .iter()
            .enumerate()
            .filter_map(|(index, text)| (validation_exit(text) == 0).then_some(index))
            .collect();
        assert!(
            accepted.is_empty(),
            "invalid document shapes accepted: {accepted:?}"
        );
    }

    #[test]
    fn checked_in_calibration_files_remain_valid() {
        let manifest = include_str!("../../../corpus/calibration/manifest.ndjson");
        assert_eq!(validation_exit(manifest), 0);

        let single = include_str!("../../../corpus/calibration/vb008_langgraph_6491.json");
        assert_eq!(validation_exit(single), 0);
    }
}
