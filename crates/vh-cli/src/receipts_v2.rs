//! Evidence bundle v2 (post-audit C3): a strict, content-addressed
//! finding bundle whose verification covers the COMPLETE declared
//! observation, plus the shrink lineage debt from PR #20 / audit D.3.
//!
//! v1 (`receipts.rs`) remains readable exactly as it is: a FIFO-only
//! SELF-CONSISTENT replay format — never authenticated provenance. v2 is
//! what `vh run --out` writes.
//!
//! Structure (NDJSON, flat records, fixed record order):
//!   1. one `finding` head record — full execution + provenance identity;
//!   2. `check` records — the FULL ordered assertion transcript
//!      (passing checks included);
//!   3. `failure` records — ordered always-failures;
//!   4. `sometimes` records — declared reachability map (sorted);
//!   5. `contract` records, then at most one `invalid_completion`;
//!   6. optional shrink lineage: one `shrink` record, the minimized
//!      plan's `injection` records, its `minimized_failure` records;
//!   7. one final `digest` record: SHA-256 over the exact UTF-8 bytes of
//!      every preceding line (each including its trailing newline).
//!
//! The parser is fail-closed: duplicate keys, unknown head keys, missing
//! fields, unknown record kinds, out-of-order records, a finding id that
//! does not recompute, mismatched schema/algorithm tags, or a content
//! digest that does not recompute are all hard errors.
//!
//! Claim boundary: the digest makes a bundle cryptographically
//! content-addressed WITH AN UNREVIEWED LOCAL SHA-256 (crates/vh-digest)
//! until the verifier track independently checks the known-answer
//! vectors (V1). It is not tamper-proof, signed, or authenticated
//! provenance. The trace hash stays tagged as the legacy internal FNV
//! format; it is not a cryptographic identity.

use crate::receipts::{parse_line, render_line, Val};

pub const FINDING_BUNDLE_SCHEMA_V2: &str = "vh-finding-bundle-v2";
pub const RUN_RECEIPTS_SCHEMA_V2: &str = "vh-run-receipts-v2";
/// The trace chain hash is the frozen v0 FNV-1a-128 format — legacy,
/// internal, never a cryptographic content identity (audit A.3).
pub const TRACE_HASH_ALGORITHM: &str = "fnv1a-128-legacy";
/// Minimizer identity for shrink lineage records.
pub const MINIMIZER: &str = "vh-shrink-ddmin-v1";

// Local copies of the identity schema/algorithm tags, cross-checked
// against the owning crates by `schema_tags_match_owning_crates` below —
// a drifted tag is a test failure, not a silent rebind.
pub const END_STATE_SCHEMA: &str = "vh-end-state-observation-v1";
pub const OBSERVATION_SCHEMA: &str = "vh-complete-observation-v1";
pub const CANONICAL_ALGORITHM: &str = "vh-canonical-length-framing-v1";

/// Build/host identity of the producing CLI. Mechanical fields only;
/// `declared_source_commit` is caller-declared (`--source-commit`) and
/// `None` means UNKNOWN — provenance is recorded, never verified here,
/// and is expected to differ across hosts (V1 compares observations,
/// not provenance).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Provenance {
    pub cli_version: String,
    pub build_profile: String,
    pub target_os: String,
    pub target_arch: String,
    pub declared_source_commit: Option<String>,
}

/// Shrink lineage (PR #20 debt, audit D.3): the minimized plan is
/// persisted IN the bundle, bound to the exact baseline it minimizes,
/// with its own expected replay identity — so a repro consumes the
/// minimized plan rather than regenerating the original.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ShrinkLineage {
    /// Must equal the head's `fault_plan_digest` — a lineage spliced
    /// from a different baseline fails at parse time.
    pub original_digest: String,
    pub original_injections: u64,
    /// Ledger digest the minimized-plan replay must reproduce.
    pub minimized_digest: String,
    /// SHA-256 of the minimized replay's complete-observation canonical
    /// bytes — the minimized run's own full identity.
    pub minimized_observation_sha256: String,
    pub oracle_calls: u64,
    pub distinct_candidates: u64,
    /// The minimized plan itself: (at_nanos, canonical fault rendering),
    /// in plan order. Rebuilt via `FaultKind::parse_canonical`.
    pub minimized_plan: Vec<(u64, String)>,
    /// The minimized replay's always-failures — the failure fingerprint
    /// the minimized plan must still produce.
    pub minimized_failures: Vec<(String, String)>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FindingBundleV2 {
    pub finding_id: String,
    pub workload: String,
    pub seed: u64,
    pub universe: u64,
    pub palette: String,
    pub schedule_policy: String,
    pub tape_digest: Option<String>,
    pub trace_hash: String,
    pub trace_events: u64,
    pub fault_plan_digest: Option<String>,
    pub end_state_sha256: String,
    pub observation_sha256: String,
    pub provenance: Provenance,
    /// Full ordered assertion transcript — passing checks included.
    pub checks: Vec<(String, bool)>,
    pub failures: Vec<(String, String)>,
    /// Sorted (BTreeMap order) sometimes-reachability map.
    pub sometimes: Vec<(String, bool)>,
    pub contract_violations: Vec<String>,
    pub invalid_completion: Option<String>,
    pub lineage: Option<ShrinkLineage>,
}

/// The v2 finding id recomputes from public fields: universe number plus
/// the first 12 hex chars of the complete-observation SHA-256.
pub fn finding_id_v2(universe: u64, observation_sha256: &str) -> String {
    let prefix: String = observation_sha256.chars().take(12).collect();
    format!("u{universe}-{prefix}")
}

fn opt_val(v: &Option<String>) -> Val {
    match v {
        Some(s) => Val::S(s.clone()),
        None => Val::Null,
    }
}

impl FindingBundleV2 {
    /// Render the bundle, computing and appending the content digest.
    pub fn to_ndjson(&self) -> String {
        let mut body = String::new();
        for line in self.body_lines() {
            body.push_str(&line);
            body.push('\n');
        }
        let digest = vh_digest::sha256_hex(body.as_bytes());
        body.push_str(&render_line(&[
            ("record", Val::S("digest".into())),
            ("alg", Val::S(vh_digest::ALGORITHM.into())),
            ("value", Val::S(digest)),
        ]));
        body.push('\n');
        body
    }

    fn body_lines(&self) -> Vec<String> {
        let p = &self.provenance;
        let mut lines = vec![render_line(&[
            ("record", Val::S("finding".into())),
            ("schema", Val::S(FINDING_BUNDLE_SCHEMA_V2.into())),
            ("finding_id", Val::S(self.finding_id.clone())),
            ("workload", Val::S(self.workload.clone())),
            ("seed", Val::S(format!("0x{:x}", self.seed))),
            ("universe", Val::N(self.universe)),
            ("palette", Val::S(self.palette.clone())),
            ("schedule_policy", Val::S(self.schedule_policy.clone())),
            ("tape_digest", opt_val(&self.tape_digest)),
            ("trace_hash", Val::S(self.trace_hash.clone())),
            ("trace_events", Val::N(self.trace_events)),
            ("trace_hash_algorithm", Val::S(TRACE_HASH_ALGORITHM.into())),
            ("fault_plan_digest", opt_val(&self.fault_plan_digest)),
            ("end_state_sha256", Val::S(self.end_state_sha256.clone())),
            ("end_state_schema", Val::S(END_STATE_SCHEMA.into())),
            (
                "observation_sha256",
                Val::S(self.observation_sha256.clone()),
            ),
            ("observation_schema", Val::S(OBSERVATION_SCHEMA.into())),
            ("canonical_algorithm", Val::S(CANONICAL_ALGORITHM.into())),
            ("digest_schema", Val::S(vh_digest::DIGEST_SCHEMA.into())),
            ("cli_version", Val::S(p.cli_version.clone())),
            ("build_profile", Val::S(p.build_profile.clone())),
            ("target_os", Val::S(p.target_os.clone())),
            ("target_arch", Val::S(p.target_arch.clone())),
            ("declared_source_commit", opt_val(&p.declared_source_commit)),
        ])];
        for (name, passed) in &self.checks {
            lines.push(render_line(&[
                ("record", Val::S("check".into())),
                ("name", Val::S(name.clone())),
                ("passed", Val::B(*passed)),
            ]));
        }
        for (name, detail) in &self.failures {
            lines.push(render_line(&[
                ("record", Val::S("failure".into())),
                ("name", Val::S(name.clone())),
                ("detail", Val::S(detail.clone())),
            ]));
        }
        for (name, hit) in &self.sometimes {
            lines.push(render_line(&[
                ("record", Val::S("sometimes".into())),
                ("name", Val::S(name.clone())),
                ("hit", Val::B(*hit)),
            ]));
        }
        for detail in &self.contract_violations {
            lines.push(render_line(&[
                ("record", Val::S("contract".into())),
                ("detail", Val::S(detail.clone())),
            ]));
        }
        if let Some(detail) = &self.invalid_completion {
            lines.push(render_line(&[
                ("record", Val::S("invalid_completion".into())),
                ("detail", Val::S(detail.clone())),
            ]));
        }
        if let Some(l) = &self.lineage {
            lines.push(render_line(&[
                ("record", Val::S("shrink".into())),
                ("original_digest", Val::S(l.original_digest.clone())),
                ("original_injections", Val::N(l.original_injections)),
                ("minimized_digest", Val::S(l.minimized_digest.clone())),
                (
                    "minimized_injections",
                    Val::N(l.minimized_plan.len() as u64),
                ),
                (
                    "minimized_observation_sha256",
                    Val::S(l.minimized_observation_sha256.clone()),
                ),
                ("oracle_calls", Val::N(l.oracle_calls)),
                ("distinct_candidates", Val::N(l.distinct_candidates)),
                ("minimizer", Val::S(MINIMIZER.into())),
            ]));
            for (at_nanos, fault) in &l.minimized_plan {
                lines.push(render_line(&[
                    ("record", Val::S("injection".into())),
                    ("at_nanos", Val::N(*at_nanos)),
                    ("fault", Val::S(fault.clone())),
                ]));
            }
            for (name, detail) in &l.minimized_failures {
                lines.push(render_line(&[
                    ("record", Val::S("minimized_failure".into())),
                    ("name", Val::S(name.clone())),
                    ("detail", Val::S(detail.clone())),
                ]));
            }
        }
        lines
    }

    /// Strict fail-closed parse. See the module doc for everything that
    /// is rejected.
    pub fn parse(text: &str) -> Result<FindingBundleV2, String> {
        // The digest is over exact bytes, so the line structure is
        // strict: newline-terminated records, no blank padding anywhere.
        let raw: Vec<&str> = text.split('\n').collect();
        let (tail, body_and_digest) = raw.split_last().ok_or("empty bundle")?;
        if !tail.is_empty() {
            return Err("bundle must end with a newline".into());
        }
        if body_and_digest.iter().any(|l| l.is_empty()) {
            return Err("blank line inside bundle — digest bytes are exact".into());
        }
        let lines = body_and_digest.to_vec();
        let digest_line = *lines.last().ok_or("empty bundle")?;
        let body_lines = &lines[..lines.len() - 1];

        // 1. Content digest recomputes over the exact body bytes.
        let digest_rec = parse_line(digest_line)?;
        let (kind, fields) = split_record(digest_rec)?;
        if kind != "digest" {
            return Err(format!("last record must be \"digest\", got {kind:?}"));
        }
        let alg = take_str(&fields, "alg")?;
        if alg != vh_digest::ALGORITHM {
            return Err(format!("unsupported digest alg {alg:?}"));
        }
        let recorded = take_str(&fields, "value")?;
        let mut body = String::new();
        for l in body_lines {
            body.push_str(l);
            body.push('\n');
        }
        let recomputed = vh_digest::sha256_hex(body.as_bytes());
        if recorded != recomputed {
            return Err(format!(
                "content digest mismatch: recorded {recorded}, recomputed {recomputed} — \
                 the bundle bytes are not the bytes that were written"
            ));
        }

        // 2. Head record: exact key set, no duplicates, no unknowns.
        let head = parse_line(body_lines.first().ok_or("bundle has no head record")?)?;
        let bundle = parse_head(head)?;

        // 3. Remaining records in enforced stage order.
        let mut bundle = bundle;
        let mut declared_injections: Option<u64> = None;
        let mut stage = 0u8; // check=1 failure=2 sometimes=3 contract=4 invalid=5 shrink=6 injection=7 minimized_failure=8
        for line in &body_lines[1..] {
            let rec = parse_line(line)?;
            let (kind, fields) = split_record(rec)?;
            let this_stage = match kind.as_str() {
                "check" => 1,
                "failure" => 2,
                "sometimes" => 3,
                "contract" => 4,
                "invalid_completion" => 5,
                "shrink" => 6,
                "injection" => 7,
                "minimized_failure" => 8,
                other => return Err(format!("unknown record kind {other:?}")),
            };
            if this_stage < stage {
                return Err(format!(
                    "record kind {kind:?} out of order (records follow the fixed v2 stage order)"
                ));
            }
            if this_stage == 5 && stage == 5 {
                return Err("duplicate invalid_completion record".into());
            }
            if this_stage == 6 && stage == 6 {
                return Err("duplicate shrink record".into());
            }
            stage = this_stage;
            match kind.as_str() {
                "check" => bundle
                    .checks
                    .push((take_str(&fields, "name")?, take_bool(&fields, "passed")?)),
                "failure" => bundle
                    .failures
                    .push((take_str(&fields, "name")?, take_str(&fields, "detail")?)),
                "sometimes" => bundle
                    .sometimes
                    .push((take_str(&fields, "name")?, take_bool(&fields, "hit")?)),
                "contract" => bundle
                    .contract_violations
                    .push(take_str(&fields, "detail")?),
                "invalid_completion" => {
                    bundle.invalid_completion = Some(take_str(&fields, "detail")?)
                }
                "shrink" => {
                    let minimizer = take_str(&fields, "minimizer")?;
                    if minimizer != MINIMIZER {
                        return Err(format!("unknown minimizer {minimizer:?}"));
                    }
                    bundle.lineage = Some(ShrinkLineage {
                        original_digest: take_str(&fields, "original_digest")?,
                        original_injections: take_u64(&fields, "original_injections")?,
                        minimized_digest: take_str(&fields, "minimized_digest")?,
                        minimized_observation_sha256: take_str(
                            &fields,
                            "minimized_observation_sha256",
                        )?,
                        oracle_calls: take_u64(&fields, "oracle_calls")?,
                        distinct_candidates: take_u64(&fields, "distinct_candidates")?,
                        minimized_plan: Vec::new(),
                        minimized_failures: Vec::new(),
                    });
                    // Validated after the loop against the actual
                    // injection record count.
                    declared_injections = Some(take_u64(&fields, "minimized_injections")?);
                }
                "injection" => match bundle.lineage.as_mut() {
                    Some(l) => l
                        .minimized_plan
                        .push((take_u64(&fields, "at_nanos")?, take_str(&fields, "fault")?)),
                    None => return Err("injection record without a shrink record".into()),
                },
                "minimized_failure" => match bundle.lineage.as_mut() {
                    Some(l) => l
                        .minimized_failures
                        .push((take_str(&fields, "name")?, take_str(&fields, "detail")?)),
                    None => return Err("minimized_failure record without a shrink record".into()),
                },
                _ => unreachable!("kind matched above"),
            }
        }

        // 4. Cross-record consistency.
        let expect_id = finding_id_v2(bundle.universe, &bundle.observation_sha256);
        if bundle.finding_id != expect_id {
            return Err(format!(
                "finding_id {:?} does not recompute (expected {expect_id:?})",
                bundle.finding_id
            ));
        }
        if let Some(l) = &bundle.lineage {
            let declared = declared_injections.ok_or("internal: shrink declared-count missing")?;
            if declared != l.minimized_plan.len() as u64 {
                return Err(format!(
                    "shrink record declares {declared} minimized injections, bundle carries {}",
                    l.minimized_plan.len()
                ));
            }
            match &bundle.fault_plan_digest {
                Some(d) if *d == l.original_digest => {}
                other => {
                    return Err(format!(
                        "shrink lineage original_digest {:?} does not match the bundle's \
                         fault_plan_digest {:?} — lineage spliced from a different baseline",
                        l.original_digest, other
                    ))
                }
            }
        }
        if bundle.failures.is_empty()
            && bundle.contract_violations.is_empty()
            && bundle.invalid_completion.is_none()
        {
            return Err("bundle records no finding — nothing to reproduce".into());
        }
        Ok(bundle)
    }
}

fn split_record(rec: Vec<(String, Val)>) -> Result<(String, Vec<(String, Val)>), String> {
    // Duplicate keys are rejected in EVERY record.
    for (i, (k, _)) in rec.iter().enumerate() {
        if rec.iter().skip(i + 1).any(|(k2, _)| k2 == k) {
            return Err(format!("duplicate key {k:?} in record"));
        }
    }
    let kind = rec
        .iter()
        .find(|(k, _)| k == "record")
        .and_then(|(_, v)| v.as_str())
        .ok_or("record kind missing")?
        .to_string();
    Ok((kind, rec))
}

fn take_str(fields: &[(String, Val)], key: &str) -> Result<String, String> {
    match fields.iter().find(|(k, _)| k == key) {
        Some((_, Val::S(s))) => Ok(s.clone()),
        Some((_, other)) => Err(format!("field {key:?} must be a string, got {other:?}")),
        None => Err(format!("missing field {key:?}")),
    }
}

fn take_opt_str(fields: &[(String, Val)], key: &str) -> Result<Option<String>, String> {
    match fields.iter().find(|(k, _)| k == key) {
        Some((_, Val::S(s))) => Ok(Some(s.clone())),
        Some((_, Val::Null)) => Ok(None),
        Some((_, other)) => Err(format!("field {key:?} must be string|null, got {other:?}")),
        None => Err(format!("missing field {key:?}")),
    }
}

fn take_u64(fields: &[(String, Val)], key: &str) -> Result<u64, String> {
    match fields.iter().find(|(k, _)| k == key) {
        Some((_, Val::N(n))) => Ok(*n),
        Some((_, other)) => Err(format!("field {key:?} must be a number, got {other:?}")),
        None => Err(format!("missing field {key:?}")),
    }
}

fn take_bool(fields: &[(String, Val)], key: &str) -> Result<bool, String> {
    match fields.iter().find(|(k, _)| k == key) {
        Some((_, Val::B(b))) => Ok(*b),
        Some((_, other)) => Err(format!("field {key:?} must be a bool, got {other:?}")),
        None => Err(format!("missing field {key:?}")),
    }
}

const HEAD_KEYS: &[&str] = &[
    "record",
    "schema",
    "finding_id",
    "workload",
    "seed",
    "universe",
    "palette",
    "schedule_policy",
    "tape_digest",
    "trace_hash",
    "trace_events",
    "trace_hash_algorithm",
    "fault_plan_digest",
    "end_state_sha256",
    "end_state_schema",
    "observation_sha256",
    "observation_schema",
    "canonical_algorithm",
    "digest_schema",
    "cli_version",
    "build_profile",
    "target_os",
    "target_arch",
    "declared_source_commit",
];

fn parse_head(head: Vec<(String, Val)>) -> Result<FindingBundleV2, String> {
    let (kind, fields) = split_record(head)?;
    if kind != "finding" {
        return Err(format!("first record must be \"finding\", got {kind:?}"));
    }
    // Exact key set: no unknown critical fields, nothing missing.
    for (k, _) in &fields {
        if !HEAD_KEYS.contains(&k.as_str()) {
            return Err(format!("unknown head field {k:?} — v2 heads are closed"));
        }
    }
    for required in HEAD_KEYS {
        if !fields.iter().any(|(k, _)| k == required) {
            return Err(format!("missing head field {required:?}"));
        }
    }
    let schema = take_str(&fields, "schema")?;
    if schema != FINDING_BUNDLE_SCHEMA_V2 {
        return Err(format!(
            "unsupported bundle schema {schema:?} (this parser reads {FINDING_BUNDLE_SCHEMA_V2:?}; \
             v1 bundles are read by the v1 parser only)"
        ));
    }
    for (key, expect) in [
        ("trace_hash_algorithm", TRACE_HASH_ALGORITHM),
        ("end_state_schema", END_STATE_SCHEMA),
        ("observation_schema", OBSERVATION_SCHEMA),
        ("canonical_algorithm", CANONICAL_ALGORITHM),
        ("digest_schema", vh_digest::DIGEST_SCHEMA),
    ] {
        let got = take_str(&fields, key)?;
        if got != expect {
            return Err(format!(
                "{key} {got:?} does not match this build's {expect:?}"
            ));
        }
    }
    Ok(FindingBundleV2 {
        finding_id: take_str(&fields, "finding_id")?,
        workload: take_str(&fields, "workload")?,
        seed: crate::receipts::parse_seed(&take_str(&fields, "seed")?)?,
        universe: take_u64(&fields, "universe")?,
        palette: take_str(&fields, "palette")?,
        schedule_policy: take_str(&fields, "schedule_policy")?,
        tape_digest: take_opt_str(&fields, "tape_digest")?,
        trace_hash: take_str(&fields, "trace_hash")?,
        trace_events: take_u64(&fields, "trace_events")?,
        fault_plan_digest: take_opt_str(&fields, "fault_plan_digest")?,
        end_state_sha256: take_str(&fields, "end_state_sha256")?,
        observation_sha256: take_str(&fields, "observation_sha256")?,
        provenance: Provenance {
            cli_version: take_str(&fields, "cli_version")?,
            build_profile: take_str(&fields, "build_profile")?,
            target_os: take_str(&fields, "target_os")?,
            target_arch: take_str(&fields, "target_arch")?,
            declared_source_commit: take_opt_str(&fields, "declared_source_commit")?,
        },
        checks: Vec::new(),
        failures: Vec::new(),
        sometimes: Vec::new(),
        contract_violations: Vec::new(),
        invalid_completion: None,
        lineage: None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample(lineage: bool) -> FindingBundleV2 {
        let observation_sha256 =
            "aaaabbbbccccddddeeeeffff00001111222233334444555566667777888899ab".to_string();
        FindingBundleV2 {
            finding_id: finding_id_v2(7, &observation_sha256),
            workload: "demo-buggy".into(),
            seed: 0xD1CE,
            universe: 7,
            palette: "v0".into(),
            schedule_policy: "fifo".into(),
            tape_digest: None,
            trace_hash: "9ce6199f133f4d3c9dd0da0075e352d2".into(),
            trace_events: 45,
            fault_plan_digest: Some("or1g1nal".into()),
            end_state_sha256: "e5e5".repeat(16),
            observation_sha256,
            provenance: Provenance {
                cli_version: "0.1.0".into(),
                build_profile: "debug".into(),
                target_os: "linux".into(),
                target_arch: "x86_64".into(),
                declared_source_commit: None,
            },
            checks: vec![
                ("oracle:wal_durability".into(), false),
                ("inv:frame".into(), true),
            ],
            failures: vec![(
                "oracle:wal_durability".into(),
                "acked k1 missing\n\"quoted\"".into(),
            )],
            sometimes: vec![("crash_resume".into(), true)],
            contract_violations: vec![],
            invalid_completion: None,
            lineage: lineage.then(|| ShrinkLineage {
                original_digest: "or1g1nal".into(),
                original_injections: 9,
                minimized_digest: "m1n1".into(),
                minimized_observation_sha256: "beef".repeat(16),
                oracle_calls: 23,
                distinct_candidates: 17,
                minimized_plan: vec![
                    (500, "crash_restart".into()),
                    (900, "network_delay:12".into()),
                ],
                minimized_failures: vec![(
                    "oracle:wal_durability".into(),
                    "acked k1 missing".into(),
                )],
            }),
        }
    }

    #[test]
    fn roundtrip_is_byte_identical_with_and_without_lineage() {
        for lineage in [false, true] {
            let b = sample(lineage);
            let text = b.to_ndjson();
            let parsed = FindingBundleV2::parse(&text).unwrap();
            assert_eq!(parsed, b);
            assert_eq!(parsed.to_ndjson(), text);
        }
    }

    #[test]
    fn any_flipped_body_byte_fails_the_content_digest() {
        let text = sample(true).to_ndjson();
        let digest_start = text.rfind("{\"record\":\"digest\"").unwrap();
        // Flip one character in several body positions (avoid producing
        // a byte-identical char, avoid the digest line itself).
        for pos in [10, 60, 120, digest_start - 5] {
            let mut bytes = text.clone().into_bytes();
            bytes[pos] = if bytes[pos] == b'x' { b'y' } else { b'x' };
            if let Ok(t) = String::from_utf8(bytes) {
                let err = FindingBundleV2::parse(&t).unwrap_err();
                assert!(
                    err.contains("digest mismatch")
                        || err.contains("duplicate")
                        || err.contains("unknown")
                        || err.contains("missing")
                        || err.contains("expected"),
                    "tamper at {pos} slipped through: {err}"
                );
            }
        }
    }

    #[test]
    fn head_rejects_duplicates_unknowns_and_missing() {
        let good = sample(false).to_ndjson();
        // Digest is rebuilt after each head edit so these cases probe the
        // SEMANTIC head validation, not (only) the content digest.
        let dup = rebuild_digest(&good.replacen(
            "\"workload\":\"demo-buggy\"",
            "\"workload\":\"demo-buggy\",\"workload\":\"demo-buggy\"",
            1,
        ));
        assert!(FindingBundleV2::parse(&dup)
            .unwrap_err()
            .contains("duplicate"));
        let unknown = rebuild_digest(&good.replacen(
            "\"workload\":\"demo-buggy\"",
            "\"workload\":\"demo-buggy\",\"confidence\":\"high\"",
            1,
        ));
        assert!(FindingBundleV2::parse(&unknown)
            .unwrap_err()
            .contains("unknown head field"));
        let missing = rebuild_digest(&good.replacen(",\"trace_events\":45", "", 1));
        assert!(FindingBundleV2::parse(&missing)
            .unwrap_err()
            .contains("missing"));
    }

    #[test]
    fn schema_and_algorithm_tags_are_pinned() {
        let good = sample(false).to_ndjson();
        for (from, to) in [
            ("vh-finding-bundle-v2", "vh-finding-bundle-v1"),
            ("fnv1a-128-legacy", "sha256"),
            ("vh-complete-observation-v1", "vh-complete-observation-v2"),
            ("vh-canonical-length-framing-v1", "debug-format"),
        ] {
            let bad = good.replacen(from, to, 1);
            assert!(
                FindingBundleV2::parse(&bad).is_err(),
                "tag swap {from} -> {to} accepted"
            );
        }
    }

    #[test]
    fn finding_id_must_recompute() {
        let mut b = sample(false);
        b.finding_id = "u7-000000000000".into();
        let err = FindingBundleV2::parse(&b.to_ndjson()).unwrap_err();
        assert!(err.contains("does not recompute"), "{err}");
    }

    #[test]
    fn record_order_is_enforced() {
        let b = sample(false);
        let text = b.to_ndjson();
        // Move the failure record before the check records.
        let lines: Vec<&str> = text.trim_end().split('\n').collect();
        let mut reordered: Vec<&str> = vec![lines[0]];
        reordered.push(lines[3]); // failure
        reordered.push(lines[1]); // check
        reordered.push(lines[2]); // check
        reordered.extend(&lines[4..lines.len() - 1]);
        let mut body = reordered.join("\n");
        body.push('\n');
        let digest = vh_digest::sha256_hex(body.as_bytes());
        body.push_str(&render_line(&[
            ("record", Val::S("digest".into())),
            ("alg", Val::S("sha256".into())),
            ("value", Val::S(digest)),
        ]));
        body.push('\n');
        let err = FindingBundleV2::parse(&body).unwrap_err();
        assert!(err.contains("out of order"), "{err}");
    }

    #[test]
    fn lineage_consistency_is_enforced() {
        // Declared injection count must match records.
        let good = sample(true).to_ndjson();
        let bad = good.replacen(
            "\"minimized_injections\":2",
            "\"minimized_injections\":3",
            1,
        );
        // Digest now stale -> first error is the digest; rebuild honestly.
        let rebuilt = rebuild_digest(&bad);
        assert!(FindingBundleV2::parse(&rebuilt)
            .unwrap_err()
            .contains("declares 3 minimized injections"));

        // Spliced baseline: lineage from a different original plan.
        let spliced = rebuild_digest(&good.replacen(
            "\"original_digest\":\"or1g1nal\"",
            "\"original_digest\":\"someoneelses\"",
            1,
        ));
        assert!(FindingBundleV2::parse(&spliced)
            .unwrap_err()
            .contains("spliced"));

        // Injection record without a shrink record.
        let mut b = sample(false);
        b.failures.push(("x".into(), "y".into()));
        let text = b.to_ndjson();
        let with_orphan = rebuild_digest(&text.replacen(
            "{\"record\":\"digest\"",
            "{\"record\":\"injection\",\"at_nanos\":1,\"fault\":\"crash_restart\"}\n{\"record\":\"digest\"",
            1,
        ));
        assert!(FindingBundleV2::parse(&with_orphan).is_err());
    }

    #[test]
    fn finding_free_bundles_are_rejected() {
        let mut b = sample(false);
        b.failures.clear();
        b.contract_violations.clear();
        b.invalid_completion = None;
        let err = FindingBundleV2::parse(&b.to_ndjson()).unwrap_err();
        assert!(err.contains("records no finding"), "{err}");
    }

    #[test]
    fn v1_parser_rejects_v2_and_vice_versa() {
        let v2 = sample(false).to_ndjson();
        assert!(crate::receipts::FindingBundle::parse(&v2).is_err());
        // A v1 bundle is rejected by the v2 parser with a typed message.
        let v1 = crate::receipts::FindingBundle {
            finding_id: "u7-abc".into(),
            workload: "demo-buggy".into(),
            seed: 1,
            palette: "v0".into(),
            universe: 7,
            trace_hash: "aa".into(),
            trace_events: 1,
            fault_plan_digest: None,
            failures: vec![("f".into(), "d".into())],
            contract_violations: vec![],
            invalid_completion: None,
        }
        .to_ndjson();
        assert!(FindingBundleV2::parse(&v1).is_err());
    }

    #[test]
    fn trailing_garbage_and_blank_lines_are_rejected() {
        let good = sample(false).to_ndjson();
        assert!(FindingBundleV2::parse(&(good.clone() + "extra\n")).is_err());
        assert!(FindingBundleV2::parse(&good.replacen(
            "\n{\"record\":\"check",
            "\n\n{\"record\":\"check",
            1
        ))
        .is_err());
        assert!(FindingBundleV2::parse(good.trim_end()).is_err());
    }

    /// The local schema-tag copies must match the owning crates — a tag
    /// drift is a build failure here, never a silent rebind.
    #[test]
    fn schema_tags_match_owning_crates() {
        use vh_multiverse::observation as obs;
        let end_state = sample(false); // silence unused warnings path
        drop(end_state);
        assert_eq!(END_STATE_SCHEMA, obs::END_STATE_IDENTITY_SCHEMA);
        assert_eq!(
            OBSERVATION_SCHEMA,
            obs::COMPLETE_OBSERVATION_IDENTITY_SCHEMA
        );
        assert_eq!(CANONICAL_ALGORITHM, obs::CANONICAL_IDENTITY_ALGORITHM);
        assert_eq!(vh_digest::DIGEST_SCHEMA, "vh-sha256-v1");
    }

    /// Rebuild the trailing digest record after an intentional body edit,
    /// so tests probe SEMANTIC validation rather than the digest check.
    fn rebuild_digest(text: &str) -> String {
        let body_end = text.rfind("{\"record\":\"digest\"").unwrap();
        let body = &text[..body_end];
        let digest = vh_digest::sha256_hex(body.as_bytes());
        format!(
            "{body}{}\n",
            render_line(&[
                ("record", Val::S("digest".into())),
                ("alg", Val::S("sha256".into())),
                ("value", Val::S(digest)),
            ])
        )
    }
}
