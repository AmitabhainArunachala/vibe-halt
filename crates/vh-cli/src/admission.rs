//! Typed admission for one predeclared Tier-1 faulty/fixed campaign.
//!
//! Dossier strings and parsed receipts never construct execution authority.
//! The only promoting entry point consumes two private `FreshRunProof` values
//! produced by `bundle::verify_run_fresh` after canonical tree verification
//! and fresh closed-registry execution.

use crate::bundle::{FreshRunOutcome, FreshRunProof, RunExpectation};

pub(crate) const REAL_EXECUTION_RECEIPT_SCHEMA: &str = "vh-real-execution-receipt-v1";

const PLAN_DOMAIN: &str = "vh-tier1-paired-execution-plan-v1";
const RECEIPT_DOMAIN: &str = "vh-real-execution-receipt-digest-v1";
const CONFIRMATION_AUTHORITY: &str = "RUST_FRESH_REPLAY";
const ADAPTER: &str = "vh-tier1-closed-registry-v1";
const OPERATION: &str = "tier1-paired-admission-v1";
const ORACLE: &str = "durability";
const TREATMENT_WORKLOAD: &str = "demo-buggy";
const FIXED_CONTROL_WORKLOAD: &str = "demo";
const PALETTE: &str = "v0";
const SCHEDULE: &str = "fifo";
const MAX_RECEIPT_BYTES: usize = 1 << 20;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum AdmissionKind {
    Confirmed,
    Null,
    Invalid,
}

impl AdmissionKind {
    const fn as_str(self) -> &'static str {
        match self {
            Self::Confirmed => "CONFIRMED",
            Self::Null => "NULL",
            Self::Invalid => "INVALID",
        }
    }

    fn parse(value: &str) -> Result<Self, String> {
        match value {
            "CONFIRMED" => Ok(Self::Confirmed),
            "NULL" => Ok(Self::Null),
            "INVALID" => Ok(Self::Invalid),
            _ => Err("unknown admission kind".into()),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ArmRole {
    Treatment,
    FixedControl,
}

impl ArmRole {
    const fn tag(self) -> &'static str {
        match self {
            Self::Treatment => "treatment",
            Self::FixedControl => "fixed-control",
        }
    }
}

/// A closed faulty/fixed plan. Construction freezes both roles before either
/// run and derives every identity from the observed engine plus typed request.
#[derive(Debug)]
pub(crate) struct PairedExecutionPlan {
    plan_id: String,
    engine_sha256: String,
    seed: u64,
    universes: u64,
    adapter: String,
    operation: String,
    oracle: String,
    treatment: RunExpectation,
    fixed_control: RunExpectation,
    treatment_revision: String,
    fixed_control_revision: String,
}

impl PairedExecutionPlan {
    pub(crate) fn tier1_kv_demo(
        engine_sha256: &str,
        seed: u64,
        universes: u64,
    ) -> Result<Self, String> {
        if !lowercase_hex(engine_sha256, 64) {
            return Err("engine SHA-256 must be 64 lowercase hexadecimal characters".into());
        }
        let treatment = RunExpectation::fifo(TREATMENT_WORKLOAD, seed, universes, PALETTE, true)?;
        let fixed_control =
            RunExpectation::fifo(FIXED_CONTROL_WORKLOAD, seed, universes, PALETTE, true)?;
        if treatment.condition_id() != fixed_control.condition_id() {
            return Err("faulty and fixed roles do not share one exact condition".into());
        }
        if treatment.oracle_contract_id() != fixed_control.oracle_contract_id() {
            return Err("faulty and fixed roles do not share one oracle contract".into());
        }
        let treatment_revision = treatment.target_revision_for_engine(engine_sha256)?;
        let fixed_control_revision = fixed_control.target_revision_for_engine(engine_sha256)?;
        if treatment_revision == fixed_control_revision {
            return Err("faulty and fixed target revisions must be distinct".into());
        }
        let mut plan = Self {
            plan_id: String::new(),
            engine_sha256: engine_sha256.to_string(),
            seed,
            universes,
            adapter: ADAPTER.to_string(),
            operation: OPERATION.to_string(),
            oracle: ORACLE.to_string(),
            treatment,
            fixed_control,
            treatment_revision,
            fixed_control_revision,
        };
        plan.plan_id = vh_digest::sha256_hex(&canonical_plan_bytes(&plan));
        Ok(plan)
    }

    pub(crate) fn treatment_expectation(&self) -> &RunExpectation {
        &self.treatment
    }

    pub(crate) fn fixed_control_expectation(&self) -> &RunExpectation {
        &self.fixed_control
    }

    pub(crate) fn plan_id(&self) -> &str {
        &self.plan_id
    }

    fn expectation(&self, role: ArmRole) -> &RunExpectation {
        match role {
            ArmRole::Treatment => &self.treatment,
            ArmRole::FixedControl => &self.fixed_control,
        }
    }

    fn target_revision(&self, role: ArmRole) -> &str {
        match role {
            ArmRole::Treatment => &self.treatment_revision,
            ArmRole::FixedControl => &self.fixed_control_revision,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct ProofFacts {
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

impl ProofFacts {
    fn consume(proof: FreshRunProof) -> Self {
        Self {
            engine_sha256: proof.engine_sha256().to_string(),
            workload_target_revision: proof.workload_target_revision().to_string(),
            command_id: proof.command_id().to_string(),
            condition_id: proof.condition_id().to_string(),
            oracle_contract_id: proof.oracle_contract_id().to_string(),
            outcome: proof.outcome(),
            evidence_digest: proof.evidence_digest().to_string(),
            result_digest: proof.result_digest().to_string(),
            finding_count: proof.finding_count(),
            budget_universes: proof.budget_universes(),
            results_len: proof.results_len(),
            budget_exhausted: proof.budget_exhausted(),
            fault_plan_digests: proof.fault_plan_digests().to_vec(),
            verification_result_id: proof.verification_result_id().to_string(),
        }
    }
}

#[derive(Debug)]
pub(crate) struct Admission {
    kind: AdmissionKind,
    fixed_control_miss: bool,
    receipt: RealExecutionReceipt,
}

impl Admission {
    pub(crate) fn classify(
        plan: PairedExecutionPlan,
        treatment: FreshRunProof,
        fixed_control: FreshRunProof,
    ) -> Self {
        let treatment = ProofFacts::consume(treatment);
        let fixed_control = ProofFacts::consume(fixed_control);
        let kind = classify_facts(&plan, &treatment, &fixed_control);
        let fixed_control_miss = kind == AdmissionKind::Confirmed;
        let receipt =
            RealExecutionReceipt::new(&plan, kind, fixed_control_miss, &treatment, &fixed_control);
        Self {
            kind,
            fixed_control_miss,
            receipt,
        }
    }

    pub(crate) const fn kind_label(&self) -> &'static str {
        self.kind.as_str()
    }

    pub(crate) const fn fixed_control_miss(&self) -> bool {
        self.fixed_control_miss
    }

    pub(crate) const fn receipt(&self) -> &RealExecutionReceipt {
        &self.receipt
    }

    pub(crate) const fn exit_code(&self) -> i32 {
        match self.kind {
            AdmissionKind::Confirmed => 1,
            AdmissionKind::Null => 0,
            AdmissionKind::Invalid => 3,
        }
    }
}

#[derive(Debug)]
pub(crate) struct RealExecutionReceipt {
    canonical_bytes: Vec<u8>,
    sha256: String,
}

impl RealExecutionReceipt {
    fn new(
        plan: &PairedExecutionPlan,
        kind: AdmissionKind,
        fixed_control_miss: bool,
        treatment: &ProofFacts,
        fixed_control: &ProofFacts,
    ) -> Self {
        let canonical_bytes =
            canonical_receipt_bytes(plan, kind, fixed_control_miss, treatment, fixed_control);
        let sha256 = vh_digest::sha256_hex(&canonical_bytes);
        Self {
            canonical_bytes,
            sha256,
        }
    }

    pub(crate) fn canonical_bytes(&self) -> &[u8] {
        &self.canonical_bytes
    }

    pub(crate) fn sha256(&self) -> &str {
        &self.sha256
    }

    /// Strict structure/integrity verification only. This never reconstructs
    /// a `FreshRunProof` and therefore cannot promote imported bytes.
    pub(crate) fn verify_canonical(bytes: &[u8], expected_sha256: &str) -> Result<(), String> {
        if bytes.len() > MAX_RECEIPT_BYTES {
            return Err("admission receipt exceeds the byte bound".into());
        }
        if !lowercase_hex(expected_sha256, 64) || vh_digest::sha256_hex(bytes) != expected_sha256 {
            return Err("admission receipt digest mismatch".into());
        }
        let mut cursor = Cursor::new(bytes);
        cursor.exact_line(RECEIPT_DOMAIN)?;
        require_equal(
            cursor.framed_string("schema")?,
            REAL_EXECUTION_RECEIPT_SCHEMA,
            "receipt schema",
        )?;
        let plan_bytes = cursor.framed("plan")?;
        let plan = parse_plan(plan_bytes)?;
        require_equal(cursor.framed_string("plan-id")?, plan.plan_id(), "plan id")?;
        let kind = AdmissionKind::parse(cursor.framed_string("admission-kind")?)?;
        let fixed_control_miss = cursor.boolean("fixed-control-miss")?;
        let authority = cursor.framed_string("confirmation-authority")?;
        let treatment = parse_facts(&mut cursor, ArmRole::Treatment)?;
        let fixed_control = parse_facts(&mut cursor, ArmRole::FixedControl)?;
        cursor.finish()?;

        let expected_kind = classify_facts(&plan, &treatment, &fixed_control);
        if kind != expected_kind || fixed_control_miss != (kind == AdmissionKind::Confirmed) {
            return Err("receipt admission state does not follow the closed matrix".into());
        }
        let expected_authority = authority_for(kind);
        require_equal(authority, expected_authority, "confirmation authority")?;
        let canonical =
            canonical_receipt_bytes(&plan, kind, fixed_control_miss, &treatment, &fixed_control);
        if canonical != bytes {
            return Err("admission receipt is not canonically encoded".into());
        }
        Ok(())
    }
}

fn classify_facts(
    plan: &PairedExecutionPlan,
    treatment: &ProofFacts,
    fixed_control: &ProofFacts,
) -> AdmissionKind {
    let bindings_valid = plan_is_valid(plan)
        && facts_match(plan, ArmRole::Treatment, treatment)
        && facts_match(plan, ArmRole::FixedControl, fixed_control)
        && treatment.engine_sha256 == fixed_control.engine_sha256
        && treatment.condition_id == fixed_control.condition_id
        && treatment.oracle_contract_id == fixed_control.oracle_contract_id
        && treatment.budget_universes == fixed_control.budget_universes
        && treatment.results_len == fixed_control.results_len
        && treatment.fault_plan_digests == fixed_control.fault_plan_digests
        && treatment.workload_target_revision != fixed_control.workload_target_revision;
    if !bindings_valid {
        return AdmissionKind::Invalid;
    }
    classify_outcomes(treatment.outcome, fixed_control.outcome)
}

fn classify_outcomes(treatment: FreshRunOutcome, fixed_control: FreshRunOutcome) -> AdmissionKind {
    match (treatment, fixed_control) {
        (FreshRunOutcome::Findings, FreshRunOutcome::Clean) => AdmissionKind::Confirmed,
        (FreshRunOutcome::Clean, FreshRunOutcome::Clean) => AdmissionKind::Null,
        _ => AdmissionKind::Invalid,
    }
}

fn facts_match(plan: &PairedExecutionPlan, role: ArmRole, facts: &ProofFacts) -> bool {
    let expected = plan.expectation(role);
    if !facts_shape_is_valid(facts) {
        return false;
    }
    let recomputed_verification_result_id =
        crate::bundle::verification_result_id(&crate::bundle::VerificationResultFacts {
            engine_sha256: &facts.engine_sha256,
            workload_target_revision: &facts.workload_target_revision,
            expected,
            outcome: facts.outcome,
            evidence_digest: &facts.evidence_digest,
            result_digest: &facts.result_digest,
            finding_count: facts.finding_count,
            results_len: facts.results_len,
            budget_exhausted: facts.budget_exhausted,
            fault_plan_digests: &facts.fault_plan_digests,
        });
    facts.engine_sha256 == plan.engine_sha256
        && facts.workload_target_revision == plan.target_revision(role)
        && facts.command_id == expected.command_id()
        && facts.condition_id == expected.condition_id()
        && facts.oracle_contract_id == expected.oracle_contract_id()
        && facts.budget_universes == expected.universes()
        && facts.results_len == expected.universes() as usize
        && facts.budget_exhausted
        && facts.verification_result_id == recomputed_verification_result_id
}

fn facts_shape_is_valid(facts: &ProofFacts) -> bool {
    let outcome_shape = match facts.outcome {
        FreshRunOutcome::Clean | FreshRunOutcome::Unchecked => facts.finding_count == 0,
        FreshRunOutcome::Findings => facts.finding_count > 0,
    };
    lowercase_hex(&facts.engine_sha256, 64)
        && lowercase_hex(&facts.workload_target_revision, 64)
        && lowercase_hex(&facts.command_id, 64)
        && lowercase_hex(&facts.condition_id, 64)
        && lowercase_hex(&facts.oracle_contract_id, 64)
        && lowercase_hex(&facts.evidence_digest, 64)
        && lowercase_hex(&facts.result_digest, 64)
        && lowercase_hex(&facts.verification_result_id, 64)
        && facts.budget_universes > 0
        && facts.results_len > 0
        && facts.finding_count <= facts.results_len
        && facts.fault_plan_digests.len() == facts.results_len
        && facts
            .fault_plan_digests
            .iter()
            .all(|digest| lowercase_hex(digest, 32))
        && outcome_shape
}

fn plan_is_valid(plan: &PairedExecutionPlan) -> bool {
    let Ok(expected) =
        PairedExecutionPlan::tier1_kv_demo(&plan.engine_sha256, plan.seed, plan.universes)
    else {
        return false;
    };
    canonical_plan_bytes(plan) == canonical_plan_bytes(&expected)
        && plan.plan_id == expected.plan_id
        && plan.plan_id == vh_digest::sha256_hex(&canonical_plan_bytes(plan))
}

fn outcome_label(outcome: FreshRunOutcome) -> &'static str {
    match outcome {
        FreshRunOutcome::Clean => "CLEAN",
        FreshRunOutcome::Findings => "FINDINGS",
        FreshRunOutcome::Unchecked => "UNCHECKED",
    }
}

fn parse_outcome(value: &str) -> Result<FreshRunOutcome, String> {
    match value {
        "CLEAN" => Ok(FreshRunOutcome::Clean),
        "FINDINGS" => Ok(FreshRunOutcome::Findings),
        "UNCHECKED" => Ok(FreshRunOutcome::Unchecked),
        _ => Err("unknown execution outcome".into()),
    }
}

fn authority_for(kind: AdmissionKind) -> &'static str {
    match kind {
        AdmissionKind::Confirmed | AdmissionKind::Null => CONFIRMATION_AUTHORITY,
        AdmissionKind::Invalid => "none",
    }
}

fn canonical_plan_bytes(plan: &PairedExecutionPlan) -> Vec<u8> {
    let mut out = Vec::new();
    plain_line(&mut out, PLAN_DOMAIN);
    frame(&mut out, "engine-sha256", plan.engine_sha256.as_bytes());
    frame(&mut out, "adapter", plan.adapter.as_bytes());
    frame(&mut out, "operation", plan.operation.as_bytes());
    frame(&mut out, "oracle", plan.oracle.as_bytes());
    numeric(&mut out, "seed", plan.seed);
    numeric(&mut out, "universes", plan.universes);
    frame(&mut out, "palette", PALETTE.as_bytes());
    frame(&mut out, "schedule", SCHEDULE.as_bytes());
    boolean(&mut out, "divergence-check", true);
    frame(
        &mut out,
        "condition-id",
        plan.treatment.condition_id().as_bytes(),
    );
    frame(
        &mut out,
        "oracle-contract-id",
        plan.treatment.oracle_contract_id().as_bytes(),
    );
    encode_plan_arm(&mut out, ArmRole::Treatment, plan);
    encode_plan_arm(&mut out, ArmRole::FixedControl, plan);
    out
}

fn encode_plan_arm(out: &mut Vec<u8>, role: ArmRole, plan: &PairedExecutionPlan) {
    let prefix = role.tag();
    let expected = plan.expectation(role);
    frame(out, &format!("{prefix}-role"), prefix.as_bytes());
    frame(
        out,
        &format!("{prefix}-workload"),
        expected.workload().as_bytes(),
    );
    frame(
        out,
        &format!("{prefix}-command-id"),
        expected.command_id().as_bytes(),
    );
    frame(
        out,
        &format!("{prefix}-target-revision"),
        plan.target_revision(role).as_bytes(),
    );
}

fn canonical_receipt_bytes(
    plan: &PairedExecutionPlan,
    kind: AdmissionKind,
    fixed_control_miss: bool,
    treatment: &ProofFacts,
    fixed_control: &ProofFacts,
) -> Vec<u8> {
    let mut out = Vec::new();
    plain_line(&mut out, RECEIPT_DOMAIN);
    frame(&mut out, "schema", REAL_EXECUTION_RECEIPT_SCHEMA.as_bytes());
    let plan_bytes = canonical_plan_bytes(plan);
    frame(&mut out, "plan", &plan_bytes);
    frame(&mut out, "plan-id", plan.plan_id.as_bytes());
    frame(&mut out, "admission-kind", kind.as_str().as_bytes());
    boolean(&mut out, "fixed-control-miss", fixed_control_miss);
    frame(
        &mut out,
        "confirmation-authority",
        authority_for(kind).as_bytes(),
    );
    encode_facts(&mut out, ArmRole::Treatment, treatment);
    encode_facts(&mut out, ArmRole::FixedControl, fixed_control);
    out
}

fn encode_facts(out: &mut Vec<u8>, role: ArmRole, facts: &ProofFacts) {
    let prefix = role.tag();
    for (name, value) in [
        ("engine-sha256", facts.engine_sha256.as_str()),
        (
            "workload-target-revision",
            facts.workload_target_revision.as_str(),
        ),
        ("command-id", facts.command_id.as_str()),
        ("condition-id", facts.condition_id.as_str()),
        ("oracle-contract-id", facts.oracle_contract_id.as_str()),
        ("outcome", outcome_label(facts.outcome)),
        ("evidence-digest", facts.evidence_digest.as_str()),
        ("result-digest", facts.result_digest.as_str()),
    ] {
        frame(out, &format!("{prefix}-{name}"), value.as_bytes());
    }
    numeric(
        out,
        &format!("{prefix}-finding-count"),
        facts.finding_count as u64,
    );
    numeric(
        out,
        &format!("{prefix}-budget-universes"),
        facts.budget_universes,
    );
    numeric(
        out,
        &format!("{prefix}-results-len"),
        facts.results_len as u64,
    );
    boolean(
        out,
        &format!("{prefix}-budget-exhausted"),
        facts.budget_exhausted,
    );
    numeric(
        out,
        &format!("{prefix}-fault-plan-count"),
        facts.fault_plan_digests.len() as u64,
    );
    for digest in &facts.fault_plan_digests {
        frame(
            out,
            &format!("{prefix}-fault-plan-digest"),
            digest.as_bytes(),
        );
    }
    frame(
        out,
        &format!("{prefix}-verification-result-id"),
        facts.verification_result_id.as_bytes(),
    );
}

fn parse_plan(bytes: &[u8]) -> Result<PairedExecutionPlan, String> {
    let mut cursor = Cursor::new(bytes);
    cursor.exact_line(PLAN_DOMAIN)?;
    let engine_sha256 = cursor.framed_string("engine-sha256")?.to_string();
    require_equal(cursor.framed_string("adapter")?, ADAPTER, "plan adapter")?;
    require_equal(
        cursor.framed_string("operation")?,
        OPERATION,
        "plan operation",
    )?;
    require_equal(cursor.framed_string("oracle")?, ORACLE, "plan oracle")?;
    let seed = cursor.numeric("seed")?;
    let universes = cursor.numeric("universes")?;
    require_equal(cursor.framed_string("palette")?, PALETTE, "plan palette")?;
    require_equal(cursor.framed_string("schedule")?, SCHEDULE, "plan schedule")?;
    if !cursor.boolean("divergence-check")? {
        return Err("plan divergence check must be enabled".into());
    }
    let condition_id = cursor.framed_string("condition-id")?.to_string();
    let oracle_contract_id = cursor.framed_string("oracle-contract-id")?.to_string();
    parse_plan_arm(&mut cursor, ArmRole::Treatment)?;
    parse_plan_arm(&mut cursor, ArmRole::FixedControl)?;
    cursor.finish()?;
    let plan = PairedExecutionPlan::tier1_kv_demo(&engine_sha256, seed, universes)?;
    if plan.treatment.condition_id() != condition_id
        || plan.treatment.oracle_contract_id() != oracle_contract_id
        || canonical_plan_bytes(&plan) != bytes
    {
        return Err("plan bytes do not equal the closed reconstructed plan".into());
    }
    Ok(plan)
}

fn parse_plan_arm(cursor: &mut Cursor<'_>, role: ArmRole) -> Result<(), String> {
    let prefix = role.tag();
    require_equal(
        cursor.framed_string(&format!("{prefix}-role"))?,
        prefix,
        "plan arm role",
    )?;
    let expected_workload = match role {
        ArmRole::Treatment => TREATMENT_WORKLOAD,
        ArmRole::FixedControl => FIXED_CONTROL_WORKLOAD,
    };
    require_equal(
        cursor.framed_string(&format!("{prefix}-workload"))?,
        expected_workload,
        "plan arm workload",
    )?;
    let command_id = cursor.framed_string(&format!("{prefix}-command-id"))?;
    if !lowercase_hex(command_id, 64) {
        return Err("plan command id is not canonical SHA-256".into());
    }
    let revision = cursor.framed_string(&format!("{prefix}-target-revision"))?;
    if !lowercase_hex(revision, 64) {
        return Err("plan target revision is not canonical SHA-256".into());
    }
    Ok(())
}

fn parse_facts(cursor: &mut Cursor<'_>, role: ArmRole) -> Result<ProofFacts, String> {
    let prefix = role.tag();
    let (
        engine_sha256,
        workload_target_revision,
        command_id,
        condition_id,
        oracle_contract_id,
        outcome,
        evidence_digest,
        result_digest,
    ) = {
        let mut string = |name: &str| {
            cursor
                .framed_string(&format!("{prefix}-{name}"))
                .map(str::to_string)
        };
        (
            string("engine-sha256")?,
            string("workload-target-revision")?,
            string("command-id")?,
            string("condition-id")?,
            string("oracle-contract-id")?,
            parse_outcome(&string("outcome")?)?,
            string("evidence-digest")?,
            string("result-digest")?,
        )
    };
    let finding_count = usize::try_from(cursor.numeric(&format!("{prefix}-finding-count"))?)
        .map_err(|_| "finding count exceeds platform bounds".to_string())?;
    let budget_universes = cursor.numeric(&format!("{prefix}-budget-universes"))?;
    let results_len = usize::try_from(cursor.numeric(&format!("{prefix}-results-len"))?)
        .map_err(|_| "result count exceeds platform bounds".to_string())?;
    let budget_exhausted = cursor.boolean(&format!("{prefix}-budget-exhausted"))?;
    let plan_count = usize::try_from(cursor.numeric(&format!("{prefix}-fault-plan-count"))?)
        .map_err(|_| "fault-plan count exceeds platform bounds".to_string())?;
    if plan_count > crate::bundle::MAX_VERIFY_UNIVERSES as usize {
        return Err("fault-plan count exceeds verifier bounds".into());
    }
    let mut fault_plan_digests = Vec::with_capacity(plan_count);
    for _ in 0..plan_count {
        fault_plan_digests.push(
            cursor
                .framed_string(&format!("{prefix}-fault-plan-digest"))?
                .to_string(),
        );
    }
    let verification_result_id = cursor
        .framed_string(&format!("{prefix}-verification-result-id"))?
        .to_string();
    Ok(ProofFacts {
        engine_sha256,
        workload_target_revision,
        command_id,
        condition_id,
        oracle_contract_id,
        outcome,
        evidence_digest,
        result_digest,
        finding_count,
        budget_universes,
        results_len,
        budget_exhausted,
        fault_plan_digests,
        verification_result_id,
    })
}

fn plain_line(out: &mut Vec<u8>, value: &str) {
    out.extend_from_slice(value.as_bytes());
    out.push(b'\n');
}

fn frame(out: &mut Vec<u8>, tag: &str, value: &[u8]) {
    out.extend_from_slice(tag.as_bytes());
    out.push(b' ');
    out.extend_from_slice(value.len().to_string().as_bytes());
    out.push(b':');
    out.extend_from_slice(value);
    out.push(b'\n');
}

fn numeric(out: &mut Vec<u8>, tag: &str, value: u64) {
    plain_line(out, &format!("{tag} {value}"));
}

fn boolean(out: &mut Vec<u8>, tag: &str, value: bool) {
    plain_line(out, &format!("{tag} {value}"));
}

fn lowercase_hex(value: &str, len: usize) -> bool {
    value.len() == len
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

fn require_equal(actual: &str, expected: &str, field: &str) -> Result<(), String> {
    if actual == expected {
        Ok(())
    } else {
        Err(format!("{field} mismatch"))
    }
}

struct Cursor<'a> {
    bytes: &'a [u8],
    position: usize,
}

impl<'a> Cursor<'a> {
    const fn new(bytes: &'a [u8]) -> Self {
        Self { bytes, position: 0 }
    }

    fn exact_line(&mut self, expected: &str) -> Result<(), String> {
        let line = self.line()?;
        if line == expected.as_bytes() {
            Ok(())
        } else {
            Err("unexpected positional record line".into())
        }
    }

    fn line(&mut self) -> Result<&'a [u8], String> {
        let rest = self
            .bytes
            .get(self.position..)
            .ok_or_else(|| "record cursor exceeded input".to_string())?;
        let offset = rest
            .iter()
            .position(|byte| *byte == b'\n')
            .ok_or_else(|| "record line is truncated".to_string())?;
        let line = &rest[..offset];
        self.position += offset + 1;
        Ok(line)
    }

    fn framed(&mut self, tag: &str) -> Result<&'a [u8], String> {
        let tag_bytes = tag.as_bytes();
        if self
            .bytes
            .get(self.position..self.position + tag_bytes.len())
            != Some(tag_bytes)
        {
            return Err(format!("expected framed field {tag}"));
        }
        self.position += tag_bytes.len();
        if self.bytes.get(self.position) != Some(&b' ') {
            return Err("framed field separator is not canonical".into());
        }
        self.position += 1;
        let digits_start = self.position;
        while self
            .bytes
            .get(self.position)
            .is_some_and(u8::is_ascii_digit)
        {
            self.position += 1;
        }
        let digits = self
            .bytes
            .get(digits_start..self.position)
            .ok_or_else(|| "framed length is missing".to_string())?;
        if digits.is_empty() || (digits.len() > 1 && digits[0] == b'0') {
            return Err("framed length is not canonical decimal".into());
        }
        if self.bytes.get(self.position) != Some(&b':') {
            return Err("framed length delimiter is missing".into());
        }
        self.position += 1;
        let digits =
            std::str::from_utf8(digits).map_err(|_| "framed length is not ASCII".to_string())?;
        let length = digits
            .parse::<usize>()
            .map_err(|_| "framed length is out of bounds".to_string())?;
        let end = self
            .position
            .checked_add(length)
            .ok_or_else(|| "framed length overflow".to_string())?;
        let value = self
            .bytes
            .get(self.position..end)
            .ok_or_else(|| "framed value is truncated".to_string())?;
        self.position = end;
        if self.bytes.get(self.position) != Some(&b'\n') {
            return Err("framed value has no canonical terminator".into());
        }
        self.position += 1;
        Ok(value)
    }

    fn framed_string(&mut self, tag: &str) -> Result<&'a str, String> {
        std::str::from_utf8(self.framed(tag)?)
            .map_err(|_| format!("framed field {tag} is not UTF-8"))
    }

    fn numeric(&mut self, tag: &str) -> Result<u64, String> {
        let line = self.line()?;
        let prefix = format!("{tag} ");
        let digits = line
            .strip_prefix(prefix.as_bytes())
            .ok_or_else(|| format!("expected numeric field {tag}"))?;
        if digits.is_empty()
            || (digits.len() > 1 && digits[0] == b'0')
            || !digits.iter().all(u8::is_ascii_digit)
        {
            return Err("numeric field is not canonical decimal".into());
        }
        std::str::from_utf8(digits)
            .map_err(|_| "numeric field is not ASCII".to_string())?
            .parse::<u64>()
            .map_err(|_| "numeric field is out of bounds".to_string())
    }

    fn boolean(&mut self, tag: &str) -> Result<bool, String> {
        let line = self.line()?;
        let true_line = format!("{tag} true");
        let false_line = format!("{tag} false");
        if line == true_line.as_bytes() {
            Ok(true)
        } else if line == false_line.as_bytes() {
            Ok(false)
        } else {
            Err(format!("expected canonical boolean field {tag}"))
        }
    }

    fn finish(&self) -> Result<(), String> {
        if self.position == self.bytes.len() {
            Ok(())
        } else {
            Err("record has trailing bytes".into())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const ENGINE: &str = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
    const SEED: u64 = 0xd1ce;
    const UNIVERSES: u64 = 4;

    fn fixture_plan() -> PairedExecutionPlan {
        PairedExecutionPlan::tier1_kv_demo(ENGINE, SEED, UNIVERSES).unwrap()
    }

    fn facts(plan: &PairedExecutionPlan, role: ArmRole, outcome: FreshRunOutcome) -> ProofFacts {
        let expected = plan.expectation(role);
        let marker = match role {
            ArmRole::Treatment => 'b',
            ArmRole::FixedControl => 'c',
        };
        let mut facts = ProofFacts {
            engine_sha256: plan.engine_sha256.clone(),
            workload_target_revision: plan.target_revision(role).to_string(),
            command_id: expected.command_id().to_string(),
            condition_id: expected.condition_id().to_string(),
            oracle_contract_id: expected.oracle_contract_id().to_string(),
            outcome,
            evidence_digest: std::iter::repeat_n(marker, 64).collect(),
            result_digest: std::iter::repeat_n(if marker == 'b' { 'd' } else { 'e' }, 64).collect(),
            finding_count: usize::from(outcome == FreshRunOutcome::Findings) * 2,
            budget_universes: UNIVERSES,
            results_len: UNIVERSES as usize,
            budget_exhausted: true,
            fault_plan_digests: ["1", "2", "3", "4"]
                .into_iter()
                .map(|digit| digit.repeat(32))
                .collect(),
            verification_result_id: String::new(),
        };
        facts.verification_result_id =
            crate::bundle::verification_result_id(&crate::bundle::VerificationResultFacts {
                engine_sha256: &facts.engine_sha256,
                workload_target_revision: &facts.workload_target_revision,
                expected,
                outcome: facts.outcome,
                evidence_digest: &facts.evidence_digest,
                result_digest: &facts.result_digest,
                finding_count: facts.finding_count,
                results_len: facts.results_len,
                budget_exhausted: facts.budget_exhausted,
                fault_plan_digests: &facts.fault_plan_digests,
            });
        facts
    }

    fn classified_fixture(
        treatment_outcome: FreshRunOutcome,
        control_outcome: FreshRunOutcome,
    ) -> (PairedExecutionPlan, ProofFacts, ProofFacts, AdmissionKind) {
        let plan = fixture_plan();
        let treatment = facts(&plan, ArmRole::Treatment, treatment_outcome);
        let control = facts(&plan, ArmRole::FixedControl, control_outcome);
        let kind = classify_facts(&plan, &treatment, &control);
        (plan, treatment, control, kind)
    }

    #[test]
    fn outcome_matrix_is_closed() {
        let outcomes = [
            FreshRunOutcome::Clean,
            FreshRunOutcome::Findings,
            FreshRunOutcome::Unchecked,
        ];
        for treatment in outcomes {
            for control in outcomes {
                let (_, _, _, got) = classified_fixture(treatment, control);
                let expected = match (treatment, control) {
                    (FreshRunOutcome::Findings, FreshRunOutcome::Clean) => AdmissionKind::Confirmed,
                    (FreshRunOutcome::Clean, FreshRunOutcome::Clean) => AdmissionKind::Null,
                    _ => AdmissionKind::Invalid,
                };
                assert_eq!(got, expected, "edge {treatment:?}/{control:?}");
            }
        }
    }

    #[test]
    fn every_binding_axis_fails_closed() {
        let plan = fixture_plan();
        let treatment = facts(&plan, ArmRole::Treatment, FreshRunOutcome::Findings);
        let control = facts(&plan, ArmRole::FixedControl, FreshRunOutcome::Clean);
        let mut mutations: Vec<(&str, ProofFacts)> = Vec::new();
        let mut changed = treatment.clone();
        changed.engine_sha256 = "0".repeat(64);
        mutations.push(("engine", changed));
        let mut changed = treatment.clone();
        changed.workload_target_revision = "0".repeat(64);
        mutations.push(("revision", changed));
        let mut changed = treatment.clone();
        changed.command_id = "0".repeat(64);
        mutations.push(("command", changed));
        let mut changed = treatment.clone();
        changed.condition_id = "0".repeat(64);
        mutations.push(("condition", changed));
        let mut changed = treatment.clone();
        changed.oracle_contract_id = "0".repeat(64);
        mutations.push(("oracle", changed));
        let mut changed = treatment.clone();
        changed.budget_universes += 1;
        mutations.push(("budget", changed));
        let mut changed = treatment.clone();
        changed.results_len -= 1;
        mutations.push(("results", changed));
        let mut changed = treatment.clone();
        changed.budget_exhausted = false;
        mutations.push(("exhaustion", changed));
        let mut changed = treatment.clone();
        changed.fault_plan_digests[0] = "0".repeat(32);
        mutations.push(("fault-vector", changed));
        let mut changed = treatment.clone();
        changed.verification_result_id = "x".repeat(64);
        mutations.push(("verification-shape", changed));

        for (axis, changed) in mutations {
            assert_eq!(
                classify_facts(&plan, &changed, &control),
                AdmissionKind::Invalid,
                "axis={axis}"
            );
        }
        assert_eq!(
            classify_facts(&plan, &control, &treatment),
            AdmissionKind::Invalid,
            "role swap"
        );
    }

    #[test]
    fn plan_digest_binds_changeable_and_fixed_axes() {
        let base = fixture_plan();
        let base_id = base.plan_id.clone();
        assert_ne!(
            base_id,
            PairedExecutionPlan::tier1_kv_demo(&"b".repeat(64), SEED, UNIVERSES)
                .unwrap()
                .plan_id
        );
        assert_ne!(
            base_id,
            PairedExecutionPlan::tier1_kv_demo(ENGINE, SEED + 1, UNIVERSES)
                .unwrap()
                .plan_id
        );
        assert_ne!(
            base_id,
            PairedExecutionPlan::tier1_kv_demo(ENGINE, SEED, UNIVERSES + 1)
                .unwrap()
                .plan_id
        );
        for mutate in ["adapter", "operation", "oracle", "treatment", "control"] {
            let mut changed = fixture_plan();
            match mutate {
                "adapter" => changed.adapter.push('x'),
                "operation" => changed.operation.push('x'),
                "oracle" => changed.oracle.push('x'),
                "treatment" => changed.treatment_revision = "0".repeat(64),
                "control" => changed.fixed_control_revision = "0".repeat(64),
                _ => unreachable!(),
            }
            assert_ne!(
                base_id,
                vh_digest::sha256_hex(&canonical_plan_bytes(&changed)),
                "axis={mutate}"
            );
            assert!(!plan_is_valid(&changed), "axis={mutate}");
        }
    }

    #[test]
    fn receipt_parser_is_exact_and_does_not_promote_bytes() {
        let (plan, treatment, control, kind) =
            classified_fixture(FreshRunOutcome::Findings, FreshRunOutcome::Clean);
        let receipt = RealExecutionReceipt::new(&plan, kind, true, &treatment, &control);
        RealExecutionReceipt::verify_canonical(receipt.canonical_bytes(), receipt.sha256())
            .unwrap();

        let mut tampered = receipt.canonical_bytes().to_vec();
        let index = tampered.iter().position(|byte| *byte == b'a').unwrap();
        tampered[index] = b'b';
        assert!(RealExecutionReceipt::verify_canonical(&tampered, receipt.sha256()).is_err());
        let mut trailing = receipt.canonical_bytes().to_vec();
        trailing.push(b'\n');
        let trailing_sha = vh_digest::sha256_hex(&trailing);
        assert!(RealExecutionReceipt::verify_canonical(&trailing, &trailing_sha).is_err());
        let truncated = &receipt.canonical_bytes()[..receipt.canonical_bytes().len() - 1];
        let truncated_sha = vh_digest::sha256_hex(truncated);
        assert!(RealExecutionReceipt::verify_canonical(truncated, &truncated_sha).is_err());

        let text = std::str::from_utf8(receipt.canonical_bytes()).unwrap();
        let id_tag = "treatment-verification-result-id 64:";
        let id_start = text.find(id_tag).unwrap() + id_tag.len();
        let mut forged_id = text.as_bytes().to_vec();
        forged_id[id_start..id_start + 64].fill(b'0');
        let forged_id_sha = vh_digest::sha256_hex(&forged_id);
        assert!(
            RealExecutionReceipt::verify_canonical(&forged_id, &forged_id_sha).is_err(),
            "a re-digested but semantically stale verification id must fail"
        );

        let impossible_count = text.replacen(
            "treatment-finding-count 2\n",
            "treatment-finding-count 5\n",
            1,
        );
        let impossible_count_sha = vh_digest::sha256_hex(impossible_count.as_bytes());
        assert!(
            RealExecutionReceipt::verify_canonical(
                impossible_count.as_bytes(),
                &impossible_count_sha,
            )
            .is_err(),
            "finding count cannot exceed the exact result budget"
        );
    }

    #[test]
    fn receipt_has_a_frozen_canonical_vector() {
        let (plan, treatment, control, kind) =
            classified_fixture(FreshRunOutcome::Findings, FreshRunOutcome::Clean);
        let receipt = RealExecutionReceipt::new(&plan, kind, true, &treatment, &control);
        assert_eq!(receipt.canonical_bytes().len(), 3650, "freeze byte length");
        assert_eq!(
            receipt.sha256(),
            "93dc372d84fdbc272ab9efa17b54af5fd071bb72192b9712d3d0f3f312d2c545",
            "freeze receipt SHA-256"
        );
    }
}
