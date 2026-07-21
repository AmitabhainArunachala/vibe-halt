#![forbid(unsafe_code)]

//! Tier-1 deterministic delta debugging for plain-data fault plans.
//!
//! The checked default is finite: at most 256 initial injections, 131,072
//! oracle calls, and 64 MiB of deterministic exact-cache weight. Crossing a
//! bound returns [`ShrinkFailure`] with original lineage and any best accepted
//! reproducer. Every determinism statement in this crate is scoped to Tier 1:
//! identical in-process inputs under the deterministic substrate.
//!
//! # Boolean oracle contract
//!
//! An oracle result of `true` MUST mean that the candidate reproduces the same
//! stable failure fingerprint captured from the original plan, not merely that
//! it produces any failure. The Boolean is intentionally opaque here; callers
//! bind the captured identity into an [`EvidenceManifest`] after success.

use std::collections::BTreeMap;
use std::error::Error;
use std::fmt;
use std::num::NonZeroUsize;

use vh_gremlin::{FaultInjection, FaultKind, FaultPlan};

const CACHE_ENTRY_OVERHEAD_WEIGHT_BYTES: usize = 128;
const FNV128_OFFSET: u128 = 0x6c62_272e_07bb_0142_62b8_2175_6295_c58d;
const FNV128_PRIME: u128 = 0x0000_0000_0100_0000_0000_0000_0000_013B;

pub const DEFAULT_MAX_CACHE_WEIGHT_BYTES: NonZeroUsize =
    match NonZeroUsize::new(64 * 1_024 * 1_024) {
        Some(value) => value,
        None => unreachable!(),
    };
pub const DEFAULT_MAX_ORACLE_CALLS: NonZeroUsize = match NonZeroUsize::new(131_072) {
    Some(value) => value,
    None => unreachable!(),
};
pub const DEFAULT_MAX_INITIAL_INJECTIONS: NonZeroUsize = match NonZeroUsize::new(256) {
    Some(value) => value,
    None => unreachable!(),
};

pub const EVIDENCE_MANIFEST_SCHEMA: &str = "vh-shrink-evidence-v1";
pub const PLAN_FINGERPRINT_SCHEMA: &str = "vh-shrink-plan-v1";

#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OracleVerification {
    /// The caller independently established Tier-1 deterministic oracle
    /// behavior. This compatibility mode evaluates each distinct candidate
    /// once.
    CallerAssertedDeterministic,
    /// Each distinct candidate is evaluated twice. Agreement is sampled
    /// falsifier evidence only; it does not prove oracle purity.
    PairedVerdictChecked,
}

impl OracleVerification {
    pub const fn token(self) -> &'static str {
        match self {
            Self::CallerAssertedDeterministic => "caller-asserted",
            Self::PairedVerdictChecked => "paired-verdict",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ShrinkConfig {
    verification: OracleVerification,
    max_oracle_calls: Option<NonZeroUsize>,
    max_cache_weight_bytes: NonZeroUsize,
    max_initial_injections: NonZeroUsize,
}

impl ShrinkConfig {
    pub const fn caller_asserted() -> Self {
        Self {
            verification: OracleVerification::CallerAssertedDeterministic,
            max_oracle_calls: None,
            max_cache_weight_bytes: DEFAULT_MAX_CACHE_WEIGHT_BYTES,
            max_initial_injections: DEFAULT_MAX_INITIAL_INJECTIONS,
        }
    }

    pub const fn paired_verdict() -> Self {
        Self {
            verification: OracleVerification::PairedVerdictChecked,
            max_oracle_calls: Some(DEFAULT_MAX_ORACLE_CALLS),
            max_cache_weight_bytes: DEFAULT_MAX_CACHE_WEIGHT_BYTES,
            max_initial_injections: DEFAULT_MAX_INITIAL_INJECTIONS,
        }
    }

    #[must_use]
    pub const fn with_max_oracle_calls(mut self, limit: NonZeroUsize) -> Self {
        self.max_oracle_calls = Some(limit);
        self
    }

    #[must_use]
    pub const fn with_max_cache_weight_bytes(mut self, limit: NonZeroUsize) -> Self {
        self.max_cache_weight_bytes = limit;
        self
    }

    #[must_use]
    pub const fn with_max_initial_injections(mut self, limit: NonZeroUsize) -> Self {
        self.max_initial_injections = limit;
        self
    }

    pub const fn verification(self) -> OracleVerification {
        self.verification
    }

    pub const fn max_oracle_calls(self) -> Option<NonZeroUsize> {
        self.max_oracle_calls
    }

    pub const fn max_cache_weight_bytes(self) -> NonZeroUsize {
        self.max_cache_weight_bytes
    }

    pub const fn max_initial_injections(self) -> NonZeroUsize {
        self.max_initial_injections
    }
}

impl Default for ShrinkConfig {
    fn default() -> Self {
        Self::paired_verdict()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ShrinkReport {
    plan: FaultPlan,
    original_plan: FaultPlan,
    original_injections: usize,
    minimized_injections: usize,
    oracle_calls: usize,
    distinct_candidates: usize,
    oracle_verification: OracleVerification,
    config: ShrinkConfig,
    cache_weight_bytes: usize,
}

impl ShrinkReport {
    pub fn plan(&self) -> &FaultPlan {
        &self.plan
    }

    pub fn into_plan(self) -> FaultPlan {
        self.plan
    }

    pub fn original_plan(&self) -> &FaultPlan {
        &self.original_plan
    }

    pub fn original_injections(&self) -> usize {
        self.original_injections
    }

    pub fn minimized_injections(&self) -> usize {
        self.minimized_injections
    }

    pub fn oracle_calls(&self) -> usize {
        self.oracle_calls
    }

    pub fn distinct_candidates(&self) -> usize {
        self.distinct_candidates
    }

    pub fn oracle_verification(&self) -> OracleVerification {
        self.oracle_verification
    }

    pub fn config(&self) -> ShrinkConfig {
        self.config
    }

    pub fn cache_weight_bytes(&self) -> usize {
        self.cache_weight_bytes
    }

    /// Bind this Tier-1 shrink result to the source/build and replay identity
    /// supplied by the caller. The manifest fingerprints both the original and
    /// minimized plans with verifier-owned framing; it does not reuse a
    /// Track-1 fingerprint.
    pub fn bind_evidence(&self, identity: EvidenceIdentity) -> EvidenceManifest {
        EvidenceManifest {
            identity,
            original_plan_fingerprint: plan_fingerprint(&self.original_plan),
            minimized_plan_fingerprint: plan_fingerprint(&self.plan),
            original_injections: self.original_injections,
            minimized_injections: self.minimized_injections,
            oracle_calls: self.oracle_calls,
            distinct_candidates: self.distinct_candidates,
            oracle_verification: self.oracle_verification,
            cache_weight_bytes: self.cache_weight_bytes,
        }
    }
}

/// Source, build, workload, universe, oracle, and failure identity captured by
/// the caller at the shrink boundary. Construction rejects empty textual
/// fields so a publication receipt cannot silently omit its lineage.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EvidenceIdentity {
    source_commit: String,
    source_tree: String,
    build_identity: String,
    workload: String,
    root_seed: u64,
    universe_id: u64,
    oracle_identity: String,
    failure_fingerprint: String,
}

impl EvidenceIdentity {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        source_commit: impl Into<String>,
        source_tree: impl Into<String>,
        build_identity: impl Into<String>,
        workload: impl Into<String>,
        root_seed: u64,
        universe_id: u64,
        oracle_identity: impl Into<String>,
        failure_fingerprint: impl Into<String>,
    ) -> Result<Self, EvidenceIdentityError> {
        let identity = Self {
            source_commit: source_commit.into(),
            source_tree: source_tree.into(),
            build_identity: build_identity.into(),
            workload: workload.into(),
            root_seed,
            universe_id,
            oracle_identity: oracle_identity.into(),
            failure_fingerprint: failure_fingerprint.into(),
        };
        for (field, value) in [
            ("source_commit", identity.source_commit.as_str()),
            ("source_tree", identity.source_tree.as_str()),
            ("build_identity", identity.build_identity.as_str()),
            ("workload", identity.workload.as_str()),
            ("oracle_identity", identity.oracle_identity.as_str()),
            ("failure_fingerprint", identity.failure_fingerprint.as_str()),
        ] {
            if value.is_empty() {
                return Err(EvidenceIdentityError::EmptyField { field });
            }
        }
        Ok(identity)
    }

    pub fn source_commit(&self) -> &str {
        &self.source_commit
    }

    pub fn source_tree(&self) -> &str {
        &self.source_tree
    }

    pub fn build_identity(&self) -> &str {
        &self.build_identity
    }

    pub fn workload(&self) -> &str {
        &self.workload
    }

    pub fn root_seed(&self) -> u64 {
        self.root_seed
    }

    pub fn universe_id(&self) -> u64 {
        self.universe_id
    }

    pub fn oracle_identity(&self) -> &str {
        &self.oracle_identity
    }

    pub fn failure_fingerprint(&self) -> &str {
        &self.failure_fingerprint
    }
}

#[non_exhaustive]
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EvidenceIdentityError {
    EmptyField { field: &'static str },
}

impl fmt::Display for EvidenceIdentityError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptyField { field } => write!(formatter, "evidence identity field '{field}' is empty"),
        }
    }
}

impl Error for EvidenceIdentityError {}

/// Publication-grade binding of a [`ShrinkReport`] to the exact failure and
/// build that produced it. `canonical()` is a single injection-safe receipt;
/// `fingerprint()` is verifier-owned FNV-1a-128 framing over that receipt.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EvidenceManifest {
    identity: EvidenceIdentity,
    original_plan_fingerprint: String,
    minimized_plan_fingerprint: String,
    original_injections: usize,
    minimized_injections: usize,
    oracle_calls: usize,
    distinct_candidates: usize,
    oracle_verification: OracleVerification,
    cache_weight_bytes: usize,
}

impl EvidenceManifest {
    pub fn schema(&self) -> &'static str {
        EVIDENCE_MANIFEST_SCHEMA
    }

    pub fn identity(&self) -> &EvidenceIdentity {
        &self.identity
    }

    pub fn original_plan_fingerprint(&self) -> &str {
        &self.original_plan_fingerprint
    }

    pub fn minimized_plan_fingerprint(&self) -> &str {
        &self.minimized_plan_fingerprint
    }

    pub fn original_injections(&self) -> usize {
        self.original_injections
    }

    pub fn minimized_injections(&self) -> usize {
        self.minimized_injections
    }

    pub fn oracle_calls(&self) -> usize {
        self.oracle_calls
    }

    pub fn distinct_candidates(&self) -> usize {
        self.distinct_candidates
    }

    pub fn oracle_verification(&self) -> OracleVerification {
        self.oracle_verification
    }

    pub fn cache_weight_bytes(&self) -> usize {
        self.cache_weight_bytes
    }

    pub fn canonical(&self) -> String {
        format!(
            "manifest-schema={} determinism-tier=Tier-1 evidence-grade=D0 source-commit={} source-tree={} build={} workload={} root-seed=0x{:016x} universe={} oracle={} failure-fingerprint={} original-plan-schema={} original-plan-fingerprint={} minimized-plan-fingerprint={} original-injections={} minimized-injections={} oracle-verification={} oracle-calls={} distinct-candidates={} cache-weight-bytes={}",
            EVIDENCE_MANIFEST_SCHEMA,
            receipt_token(self.identity.source_commit()),
            receipt_token(self.identity.source_tree()),
            receipt_token(self.identity.build_identity()),
            receipt_token(self.identity.workload()),
            self.identity.root_seed(),
            self.identity.universe_id(),
            receipt_token(self.identity.oracle_identity()),
            receipt_token(self.identity.failure_fingerprint()),
            PLAN_FINGERPRINT_SCHEMA,
            self.original_plan_fingerprint,
            self.minimized_plan_fingerprint,
            self.original_injections,
            self.minimized_injections,
            self.oracle_verification.token(),
            self.oracle_calls,
            self.distinct_candidates,
            self.cache_weight_bytes,
        )
    }

    pub fn fingerprint(&self) -> String {
        let state = fingerprint_absorb_bytes(FNV128_OFFSET, EVIDENCE_MANIFEST_SCHEMA.as_bytes());
        let state = fingerprint_absorb_bytes(state, self.canonical().as_bytes());
        format!("{state:032x}")
    }
}

/// Verifier-owned, schema-ratcheted fingerprint of a fault plan. Every current
/// `FaultKind` variant and payload is matched explicitly; additions fail this
/// crate's compilation until the identity doctrine is updated.
pub fn plan_fingerprint(plan: &FaultPlan) -> String {
    let mut state = fingerprint_absorb_bytes(FNV128_OFFSET, PLAN_FINGERPRINT_SCHEMA.as_bytes());
    state = fingerprint_absorb(
        state,
        &u64::try_from(plan.injections().len())
            .expect("fault-plan length must fit u64")
            .to_le_bytes(),
    );
    for injection in plan.injections() {
        let FaultInjection { at_nanos, fault } = injection;
        state = fingerprint_absorb(state, &at_nanos.to_le_bytes());
        match fault {
            FaultKind::CrashRestart => {
                state = fingerprint_absorb_bytes(state, b"crash-restart");
            }
            FaultKind::NetworkDelay { delay_nanos } => {
                state = fingerprint_absorb_bytes(state, b"network-delay");
                state = fingerprint_absorb(state, &delay_nanos.to_le_bytes());
            }
            FaultKind::NetworkPartition { duration_nanos } => {
                state = fingerprint_absorb_bytes(state, b"network-partition");
                state = fingerprint_absorb(state, &duration_nanos.to_le_bytes());
            }
            FaultKind::DiskWriteFail => {
                state = fingerprint_absorb_bytes(state, b"disk-write-fail");
            }
            FaultKind::ClockSkew { skew_nanos } => {
                state = fingerprint_absorb_bytes(state, b"clock-skew");
                state = fingerprint_absorb(state, &skew_nanos.to_le_bytes());
            }
            FaultKind::TornWrite => {
                state = fingerprint_absorb_bytes(state, b"torn-write");
            }
            FaultKind::FsyncLie => {
                state = fingerprint_absorb_bytes(state, b"fsync-lie");
            }
            FaultKind::NetworkDuplicate => {
                state = fingerprint_absorb_bytes(state, b"network-duplicate");
            }
            FaultKind::NetworkReorder => {
                state = fingerprint_absorb_bytes(state, b"network-reorder");
            }
        }
    }
    format!("{state:032x}")
}

/// Evidence-preserving failure from a checked shrink operation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ShrinkFailure {
    original_plan: FaultPlan,
    best_plan: Option<FaultPlan>,
    cause: Box<ShrinkError>,
    config: ShrinkConfig,
    oracle_calls: usize,
    distinct_candidates: usize,
    cache_weight_bytes: usize,
}

impl ShrinkFailure {
    pub fn original_plan(&self) -> &FaultPlan {
        &self.original_plan
    }

    pub fn best_plan(&self) -> Option<&FaultPlan> {
        self.best_plan.as_ref()
    }

    pub fn cause(&self) -> &ShrinkError {
        &self.cause
    }

    pub fn config(&self) -> ShrinkConfig {
        self.config
    }

    pub fn oracle_calls(&self) -> usize {
        self.oracle_calls
    }

    pub fn distinct_candidates(&self) -> usize {
        self.distinct_candidates
    }

    pub fn cache_weight_bytes(&self) -> usize {
        self.cache_weight_bytes
    }

    pub fn into_best_plan(self) -> Option<FaultPlan> {
        self.best_plan
    }

    fn initial(original_plan: FaultPlan, cause: ShrinkError, config: ShrinkConfig) -> Self {
        Self {
            original_plan,
            best_plan: None,
            cause: Box::new(cause),
            config,
            oracle_calls: 0,
            distinct_candidates: 0,
            cache_weight_bytes: 0,
        }
    }
}

impl fmt::Display for ShrinkFailure {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.cause.fmt(formatter)
    }
}

impl Error for ShrinkFailure {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        Some(self.cause.as_ref())
    }
}

#[non_exhaustive]
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ShrinkError {
    InitialPlanTooLarge {
        injections: usize,
        limit: NonZeroUsize,
    },
    InitialPlanDidNotFail {
        oracle_calls: usize,
    },
    OracleDiverged {
        candidate: FaultPlan,
        first: bool,
        second: bool,
        oracle_calls: usize,
    },
    OraclePanicked {
        candidate: FaultPlan,
        oracle_calls: usize,
    },
    OracleBudgetExceeded {
        candidate: FaultPlan,
        limit: NonZeroUsize,
        oracle_calls: usize,
    },
    CacheBudgetExceeded {
        candidate: FaultPlan,
        limit: NonZeroUsize,
        used: usize,
        required: usize,
        oracle_calls: usize,
    },
    InternalCandidateNotSubsequence {
        candidate: FaultPlan,
    },
}

impl fmt::Display for ShrinkError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InitialPlanTooLarge { injections, limit } => write!(
                formatter,
                "initial fault plan has {injections} injections, exceeding limit {limit}"
            ),
            Self::InitialPlanDidNotFail { .. } => {
                formatter.write_str("initial fault plan did not reproduce the failure")
            }
            Self::OracleDiverged { candidate, .. } => write!(
                formatter,
                "oracle diverged while replaying a {}-injection candidate",
                candidate.injections().len()
            ),
            Self::OraclePanicked { candidate, .. } => write!(
                formatter,
                "oracle panicked while replaying a {}-injection candidate",
                candidate.injections().len()
            ),
            Self::OracleBudgetExceeded {
                candidate, limit, ..
            } => write!(
                formatter,
                "oracle-call budget {limit} cannot evaluate a {}-injection candidate",
                candidate.injections().len()
            ),
            Self::CacheBudgetExceeded {
                candidate, limit, ..
            } => write!(
                formatter,
                "cache-weight budget {limit} cannot retain a {}-injection candidate",
                candidate.injections().len()
            ),
            Self::InternalCandidateNotSubsequence { .. } => {
                formatter.write_str("internal shrink candidate was not a deletion subsequence")
            }
        }
    }
}

impl Error for ShrinkError {}

fn injection_schema_ratcheted_eq(left: &FaultInjection, right: &FaultInjection) -> bool {
    let FaultInjection {
        at_nanos: left_at,
        fault: left_fault,
    } = left;
    let FaultInjection {
        at_nanos: right_at,
        fault: right_fault,
    } = right;
    let faults_equal = match left_fault {
        FaultKind::CrashRestart => matches!(right_fault, FaultKind::CrashRestart),
        FaultKind::NetworkDelay {
            delay_nanos: left_delay,
        } => match right_fault {
            FaultKind::NetworkDelay {
                delay_nanos: right_delay,
            } => left_delay == right_delay,
            _ => false,
        },
        FaultKind::NetworkPartition {
            duration_nanos: left_duration,
        } => match right_fault {
            FaultKind::NetworkPartition {
                duration_nanos: right_duration,
            } => left_duration == right_duration,
            _ => false,
        },
        FaultKind::DiskWriteFail => matches!(right_fault, FaultKind::DiskWriteFail),
        FaultKind::ClockSkew {
            skew_nanos: left_skew,
        } => match right_fault {
            FaultKind::ClockSkew {
                skew_nanos: right_skew,
            } => left_skew == right_skew,
            _ => false,
        },
        FaultKind::TornWrite => matches!(right_fault, FaultKind::TornWrite),
        FaultKind::FsyncLie => matches!(right_fault, FaultKind::FsyncLie),
        FaultKind::NetworkDuplicate => matches!(right_fault, FaultKind::NetworkDuplicate),
        FaultKind::NetworkReorder => matches!(right_fault, FaultKind::NetworkReorder),
    };
    left_at == right_at && faults_equal
}

fn candidate_key(original: &FaultPlan, candidate: &FaultPlan) -> Result<Box<[u64]>, ShrinkError> {
    let original = original.injections();
    let candidate_injections = candidate.injections();
    let mut key = vec![0u64; original.len().div_ceil(u64::BITS as usize)];
    let mut cursor = 0usize;

    for injection in candidate_injections {
        let Some(offset) = original[cursor..]
            .iter()
            .position(|source| injection_schema_ratcheted_eq(source, injection))
        else {
            return Err(ShrinkError::InternalCandidateNotSubsequence {
                candidate: candidate.clone(),
            });
        };
        let source_index = cursor + offset;
        key[source_index / u64::BITS as usize] |= 1u64 << (source_index % u64::BITS as usize);
        cursor = source_index + 1;
    }

    Ok(key.into_boxed_slice())
}

fn cache_entry_weight(original_len: usize) -> Option<usize> {
    original_len
        .div_ceil(u64::BITS as usize)
        .checked_mul(std::mem::size_of::<u64>())?
        .checked_add(CACHE_ENTRY_OVERHEAD_WEIGHT_BYTES)
}

struct CachedOracle<F> {
    oracle: F,
    original_plan: FaultPlan,
    observations: BTreeMap<Box<[u64]>, bool>,
    cache_weight: usize,
    entry_weight: usize,
    calls: usize,
    distinct_candidates: usize,
    config: ShrinkConfig,
}

impl<F> CachedOracle<F>
where
    F: FnMut(&FaultPlan) -> bool,
{
    fn new(oracle: F, config: ShrinkConfig, original_plan: FaultPlan, entry_weight: usize) -> Self {
        Self {
            oracle,
            original_plan,
            observations: BTreeMap::new(),
            cache_weight: 0,
            entry_weight,
            calls: 0,
            distinct_candidates: 0,
            config,
        }
    }

    fn check(&mut self, candidate: &FaultPlan) -> Result<bool, ShrinkError> {
        let key = candidate_key(&self.original_plan, candidate)?;
        if let Some(verdict) = self.observations.get(key.as_ref()) {
            return Ok(*verdict);
        }

        let Some(next_cache_weight) = self.cache_weight.checked_add(self.entry_weight) else {
            return Err(ShrinkError::CacheBudgetExceeded {
                candidate: candidate.clone(),
                limit: self.config.max_cache_weight_bytes,
                used: self.cache_weight,
                required: self.entry_weight,
                oracle_calls: self.calls,
            });
        };
        if next_cache_weight > self.config.max_cache_weight_bytes.get() {
            return Err(ShrinkError::CacheBudgetExceeded {
                candidate: candidate.clone(),
                limit: self.config.max_cache_weight_bytes,
                used: self.cache_weight,
                required: self.entry_weight,
                oracle_calls: self.calls,
            });
        }

        let required_calls = match self.config.verification {
            OracleVerification::CallerAssertedDeterministic => 1,
            OracleVerification::PairedVerdictChecked => 2,
        };
        if let Some(limit) = self.config.max_oracle_calls {
            if self.calls.saturating_add(required_calls) > limit.get() {
                return Err(ShrinkError::OracleBudgetExceeded {
                    candidate: candidate.clone(),
                    limit,
                    oracle_calls: self.calls,
                });
            }
        }

        self.distinct_candidates += 1;
        let first = self.invoke(candidate)?;
        if self.config.verification == OracleVerification::PairedVerdictChecked {
            let second = self.invoke(candidate)?;
            if first != second {
                return Err(ShrinkError::OracleDiverged {
                    candidate: candidate.clone(),
                    first,
                    second,
                    oracle_calls: self.calls,
                });
            }
        }

        self.cache_weight = next_cache_weight;
        self.observations.insert(key, first);
        Ok(first)
    }

    fn invoke(&mut self, candidate: &FaultPlan) -> Result<bool, ShrinkError> {
        self.calls += 1;
        match std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| (self.oracle)(candidate))) {
            Ok(verdict) => Ok(verdict),
            Err(_) => Err(ShrinkError::OraclePanicked {
                candidate: candidate.clone(),
                oracle_calls: self.calls,
            }),
        }
    }

    fn into_report(self, plan: FaultPlan) -> ShrinkReport {
        ShrinkReport {
            original_injections: self.original_plan.injections().len(),
            minimized_injections: plan.injections().len(),
            plan,
            original_plan: self.original_plan,
            oracle_calls: self.calls,
            distinct_candidates: self.distinct_candidates,
            oracle_verification: self.config.verification,
            config: self.config,
            cache_weight_bytes: self.cache_weight,
        }
    }

    fn into_failure(self, best_plan: Option<FaultPlan>, cause: ShrinkError) -> ShrinkFailure {
        ShrinkFailure {
            original_plan: self.original_plan,
            best_plan,
            cause: Box::new(cause),
            config: self.config,
            oracle_calls: self.calls,
            distinct_candidates: self.distinct_candidates,
            cache_weight_bytes: self.cache_weight,
        }
    }
}

/// Compatibility entrypoint using caller-asserted Tier-1 deterministic oracle
/// behavior. Panics on any checked failure; new callers should use
/// [`try_shrink`].
pub fn shrink<F>(plan: FaultPlan, mut oracle: F) -> FaultPlan
where
    F: FnMut(&FaultPlan) -> bool,
{
    match try_shrink_caller_asserted(plan, &mut oracle) {
        Ok(report) => report.into_plan(),
        Err(error) => panic!("vh-shrink: {error}"),
    }
}

/// Checked Tier-1 deterministic delta debugging with paired Boolean verdict
/// sampling and finite resource budgets.
pub fn try_shrink<F>(plan: FaultPlan, oracle: F) -> Result<ShrinkReport, ShrinkFailure>
where
    F: FnMut(&FaultPlan) -> bool,
{
    try_shrink_with_config(plan, oracle, ShrinkConfig::default())
}

/// Checked compatibility mode for callers that independently establish
/// deterministic exact-fingerprint oracle behavior.
pub fn try_shrink_caller_asserted<F>(
    plan: FaultPlan,
    oracle: F,
) -> Result<ShrinkReport, ShrinkFailure>
where
    F: FnMut(&FaultPlan) -> bool,
{
    try_shrink_with_config(plan, oracle, ShrinkConfig::caller_asserted())
}

/// Configurable checked shrink. A successful result is 1-minimal with respect
/// to deletion and the exactly cached Boolean oracle; global minimum
/// cardinality is not promised.
pub fn try_shrink_with_config<F>(
    plan: FaultPlan,
    oracle: F,
    config: ShrinkConfig,
) -> Result<ShrinkReport, ShrinkFailure>
where
    F: FnMut(&FaultPlan) -> bool,
{
    let original_injections = plan.injections().len();
    if original_injections > config.max_initial_injections.get() {
        return Err(ShrinkFailure::initial(
            plan,
            ShrinkError::InitialPlanTooLarge {
                injections: original_injections,
                limit: config.max_initial_injections,
            },
            config,
        ));
    }
    let Some(entry_weight) = cache_entry_weight(original_injections) else {
        let candidate = plan.clone();
        return Err(ShrinkFailure::initial(
            plan,
            ShrinkError::CacheBudgetExceeded {
                candidate,
                limit: config.max_cache_weight_bytes,
                used: 0,
                required: usize::MAX,
                oracle_calls: 0,
            },
            config,
        ));
    };
    if entry_weight > config.max_cache_weight_bytes.get() {
        let candidate = plan.clone();
        return Err(ShrinkFailure::initial(
            plan,
            ShrinkError::CacheBudgetExceeded {
                candidate,
                limit: config.max_cache_weight_bytes,
                used: 0,
                required: entry_weight,
                oracle_calls: 0,
            },
            config,
        ));
    }

    let mut oracle = CachedOracle::new(oracle, config, plan.clone(), entry_weight);
    let initial_fails = match oracle.check(&plan) {
        Ok(verdict) => verdict,
        Err(cause) => return Err(oracle.into_failure(None, cause)),
    };
    if !initial_fails {
        let oracle_calls = oracle.calls;
        return Err(oracle.into_failure(
            None,
            ShrinkError::InitialPlanDidNotFail { oracle_calls },
        ));
    }
    if plan.injections().is_empty() {
        return Ok(oracle.into_report(plan));
    }

    let empty = FaultPlan::default();
    let empty_fails = match oracle.check(&empty) {
        Ok(verdict) => verdict,
        Err(cause) => return Err(oracle.into_failure(Some(plan), cause)),
    };
    if empty_fails {
        return Ok(oracle.into_report(empty));
    }

    let mut current = plan;
    let mut granularity = 2usize;
    while current.injections().len() >= 2 {
        let len = current.injections().len();
        let chunk_len = len.div_ceil(granularity);
        let mut reduced = false;
        let mut start = 0usize;

        while start < len {
            let end = (start + chunk_len).min(len);
            let mut injections = Vec::with_capacity(len - (end - start));
            injections.extend_from_slice(&current.injections()[..start]);
            injections.extend_from_slice(&current.injections()[end..]);
            let candidate = FaultPlan::new(injections);
            let candidate_fails = match oracle.check(&candidate) {
                Ok(verdict) => verdict,
                Err(cause) => return Err(oracle.into_failure(Some(current), cause)),
            };
            if candidate_fails {
                current = candidate;
                granularity = granularity.saturating_sub(1).max(2);
                reduced = true;
                break;
            }
            start = end;
        }

        if !reduced {
            if granularity >= len {
                break;
            }
            granularity = granularity.saturating_mul(2).min(len);
        }
    }

    Ok(oracle.into_report(current))
}

fn fingerprint_absorb(mut state: u128, bytes: &[u8]) -> u128 {
    for byte in bytes {
        state ^= u128::from(*byte);
        state = state.wrapping_mul(FNV128_PRIME);
    }
    state
}

fn fingerprint_absorb_bytes(state: u128, bytes: &[u8]) -> u128 {
    let length = u64::try_from(bytes.len()).expect("fingerprint field length must fit u64");
    let state = fingerprint_absorb(state, &length.to_le_bytes());
    fingerprint_absorb(state, bytes)
}

fn receipt_token(value: &str) -> String {
    const HEX: &[u8; 16] = b"0123456789ABCDEF";
    let mut encoded = String::with_capacity(value.len().saturating_mul(3));
    for byte in value.bytes() {
        if byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.' | b':') {
            encoded.push(char::from(byte));
        } else {
            encoded.push('%');
            encoded.push(char::from(HEX[usize::from(byte >> 4)]));
            encoded.push(char::from(HEX[usize::from(byte & 0x0f)]));
        }
    }
    encoded
}

#[cfg(test)]
mod tests {
    use super::*;

    fn injection(at_nanos: u64, fault: FaultKind) -> FaultInjection {
        FaultInjection { at_nanos, fault }
    }

    #[test]
    fn every_fault_variant_has_distinct_plan_identity() {
        let variants = [
            FaultKind::CrashRestart,
            FaultKind::NetworkDelay { delay_nanos: 7 },
            FaultKind::NetworkPartition { duration_nanos: 7 },
            FaultKind::DiskWriteFail,
            FaultKind::ClockSkew { skew_nanos: 7 },
            FaultKind::TornWrite,
            FaultKind::FsyncLie,
            FaultKind::NetworkDuplicate,
            FaultKind::NetworkReorder,
        ];
        let fingerprints: Vec<String> = variants
            .into_iter()
            .map(|fault| plan_fingerprint(&FaultPlan::new(vec![injection(5, fault)])))
            .collect();
        for (index, fingerprint) in fingerprints.iter().enumerate() {
            assert!(!fingerprints[..index].contains(fingerprint));
        }
    }

    #[test]
    fn evidence_identity_rejects_silent_omissions() {
        let error = EvidenceIdentity::new("", "tree", "build", "work", 1, 2, "oracle", "fp")
            .expect_err("empty source identity must fail closed");
        assert_eq!(
            error,
            EvidenceIdentityError::EmptyField {
                field: "source_commit"
            }
        );
    }
}
