#![forbid(unsafe_code)]

//! Tier-1 deterministic delta debugging for plain-data fault plans.
//!
//! The checked default is deliberately finite: at most 256 initial
//! injections, 131,072 oracle calls, and 64 MiB of deterministic exact-cache
//! weight. Crossing a bound returns [`ShrinkFailure`] with original lineage
//! and any best reproducer accepted under the configured verification mode.
//! Raise limits explicitly through [`ShrinkConfig`] when a larger evidence
//! budget is intentional.
//!
//! # Boolean oracle contract
//!
//! An oracle result of `true` MUST mean that the candidate reproduces the
//! same stable failure fingerprint captured from the original plan, not merely
//! that it produces any failure. The fingerprint should bind every distinction
//! relevant to causal identity (for example, typed termination plus property
//! name and detail). This crate treats the Boolean as opaque: paired evaluation
//! can detect adjacent verdict instability, but cannot detect a broad predicate
//! that silently switches from one failure to another while shrinking.

use std::collections::BTreeMap;
use std::error::Error;
use std::fmt;
use std::num::NonZeroUsize;

use vh_gremlin::{FaultInjection, FaultKind, FaultPlan};

const CACHE_ENTRY_OVERHEAD_WEIGHT_BYTES: usize = 128;
pub const DEFAULT_MAX_CACHE_WEIGHT_BYTES: NonZeroUsize = match NonZeroUsize::new(64 * 1_024 * 1_024)
{
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

#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OracleVerification {
    /// The caller asserts that identical plans always produce identical
    /// same-fingerprint verdicts. This is the zero-overhead verification
    /// compatibility mode.
    CallerAssertedDeterministic,
    /// Every distinct candidate is evaluated twice and contradictory adjacent
    /// Boolean verdicts fail closed. This checks only verdict stability, not
    /// full observable replay identity or adversarial stateful oracles.
    PairedVerdictChecked,
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
    /// The 1-minimal deletion subsequence accepted by the caller's exact
    /// same-fingerprint oracle.
    plan: FaultPlan,
    /// Exact input lineage for reproducing or independently auditing the
    /// minimization receipt.
    original_plan: FaultPlan,
    original_injections: usize,
    minimized_injections: usize,
    /// Actual caller invocations. Paired verification makes this twice the
    /// distinct candidate count.
    oracle_calls: usize,
    /// Structurally distinct candidates whose oracle evaluation was started.
    /// Resource-preflight rejections are excluded; divergence and unwind
    /// panics are included once in failure receipts.
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
}

/// Evidence-preserving failure from a checked shrink operation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ShrinkFailure {
    original_plan: FaultPlan,
    /// `None` means no candidate received a failing verdict accepted under the
    /// configured verification mode before the operation failed.
    best_plan: Option<FaultPlan>,
    cause: Box<ShrinkError>,
    config: ShrinkConfig,
    oracle_calls: usize,
    /// Distinct candidates whose oracle evaluation started. Diverged or
    /// panicked candidates count once even though they are not cached.
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
    /// The caller supplied more injections than the configured safe bound.
    InitialPlanTooLarge {
        injections: usize,
        limit: NonZeroUsize,
    },
    /// Delta debugging is undefined when the supplied input does not reproduce
    /// the failure. [`ShrinkFailure`] retains the rejected original plan.
    InitialPlanDidNotFail { oracle_calls: usize },
    /// Adjacent evaluations of one structurally identical candidate disagreed.
    OracleDiverged {
        candidate: FaultPlan,
        first: bool,
        second: bool,
        oracle_calls: usize,
    },
    /// The oracle unwound while evaluating this exact candidate. The call that
    /// panicked is included in `oracle_calls`. Process aborts remain outside
    /// Rust's unwind boundary. Rust's process-global panic hook can still emit
    /// diagnostics to stderr; evidence runners should sanitize boundary stderr.
    OraclePanicked {
        candidate: FaultPlan,
        oracle_calls: usize,
    },
    /// The next distinct candidate could not be fully evaluated without
    /// exceeding the caller's explicit budget.
    OracleBudgetExceeded {
        candidate: FaultPlan,
        limit: NonZeroUsize,
        oracle_calls: usize,
    },
    /// Retaining another exact candidate key would cross the deterministic
    /// cache-weight budget. Weight is a portable accounting unit, not an RSS
    /// measurement: every key pays its bytes plus a fixed entry charge.
    CacheBudgetExceeded {
        candidate: FaultPlan,
        limit: NonZeroUsize,
        used: usize,
        required: usize,
        oracle_calls: usize,
    },
    /// An internal ddmin candidate was not a deletion subsequence of the
    /// original input. Failing closed protects cache identity if the
    /// algorithm is extended incorrectly.
    InternalCandidateNotSubsequence { candidate: FaultPlan },
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

fn plan_injections(plan: &FaultPlan) -> &[FaultInjection] {
    // FaultPlan owns canonical ordering and exposes no mutable construction
    // path. Candidate identity still ratchets every injection field/variant
    // explicitly below, while plan access goes through its public invariant-
    // preserving API.
    plan.injections()
}

fn injection_schema_ratcheted_eq(left: &FaultInjection, right: &FaultInjection) -> bool {
    // No `..` in any pattern: new fields or variants fail compilation here
    // instead of silently weakening exact candidate identity.
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

/// Canonical exact identity for deletion subsequences. Duplicate equal
/// injections map greedily to the earliest remaining source position, so two
/// structurally equal candidates share a key while unequal candidates cannot.
fn candidate_key(original: &FaultPlan, candidate: &FaultPlan) -> Result<Box<[u64]>, ShrinkError> {
    let original = plan_injections(original);
    let candidate_injections = plan_injections(candidate);
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

/// Produce a 1-minimal same-fingerprint subsequence by deleting contiguous
/// chunks in Tier-1 deterministic order. The oracle MUST return true only when
/// the candidate reproduces the original plan's stable failure fingerprint;
/// "any failure" is too broad and can silently switch causes. This
/// compatibility entrypoint fails closed
/// by panicking on non-reproduction, invalid plans, internal errors, or a
/// resource limit (including the default 256-injection ceiling). It uses
/// caller-asserted one-evaluation semantics and therefore cannot detect a
/// flaky oracle. Callers preserving those semantics should use
/// [`try_shrink_caller_asserted`] for typed failures; new production callers
/// should prefer the paired-verdict [`try_shrink`] default. A successful
/// 1-minimal result cannot lose any one remaining injection and still fail;
/// it is not promised to have globally minimum cardinality.
///
/// Identical plans and Tier-1 deterministic oracle behavior produce identical
/// results and oracle-call order.
pub fn shrink<F>(plan: FaultPlan, mut oracle: F) -> FaultPlan
where
    F: FnMut(&FaultPlan) -> bool,
{
    match try_shrink_caller_asserted(plan, &mut oracle) {
        Ok(report) => report.into_plan(),
        Err(error) => panic!("vh-shrink: {error}"),
    }
}

/// Checked Tier-1 deterministic delta debugging with an auditable result and
/// the safe default: adjacent Boolean verdict verification plus finite oracle
/// and cache-weight budgets.
///
/// Each distinct candidate is evaluated twice and then cached exactly. A
/// successful report records a `plan` that still has the caller-selected stable
/// failure fingerprint and is 1-minimal with respect to the accepted, exactly
/// cached Boolean verdicts. The caller MUST compare that exact fingerprint,
/// not merely ask whether any failure occurred. That conclusion is conditional
/// on deterministic oracle behavior: paired adjacent agreement is evidence
/// against accidental flakiness, not a proof of full replay identity,
/// fingerprint correctness, or protection from adversarial stateful oracles.
pub fn try_shrink<F>(plan: FaultPlan, oracle: F) -> Result<ShrinkReport, ShrinkFailure>
where
    F: FnMut(&FaultPlan) -> bool,
{
    try_shrink_with_config(plan, oracle, ShrinkConfig::default())
}

/// Explicit zero-overhead verification mode for callers that independently
/// establish deterministic oracle behavior and exact stable-fingerprint
/// matching. Exact caching and its finite cache-weight budget remain enabled.
pub fn try_shrink_caller_asserted<F>(
    plan: FaultPlan,
    oracle: F,
) -> Result<ShrinkReport, ShrinkFailure>
where
    F: FnMut(&FaultPlan) -> bool,
{
    try_shrink_with_config(plan, oracle, ShrinkConfig::caller_asserted())
}

/// Configurable checked shrink. Oracle-call, exact-cache-weight, and initial
/// plan-size bounds fail closed with original lineage and the best known
/// reproducer. The call budget is enforced before starting a candidate,
/// including both calls in paired-verdict mode. As with every entrypoint, an
/// oracle `true` MUST mean the original stable failure fingerprint reproduced.
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
        return Err(oracle.into_failure(None, ShrinkError::InitialPlanDidNotFail { oracle_calls }));
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

#[cfg(test)]
mod tests {
    use super::*;

    fn injection(at_nanos: u64, fault: FaultKind) -> FaultInjection {
        FaultInjection { at_nanos, fault }
    }

    #[test]
    fn duplicate_injections_have_one_canonical_structural_key() {
        let duplicate = injection(10, FaultKind::CrashRestart);
        let tail = injection(20, FaultKind::DiskWriteFail);
        let original = FaultPlan::new(vec![duplicate.clone(), duplicate.clone(), tail.clone()]);
        let candidate = FaultPlan::new(vec![duplicate, tail]);

        assert_eq!(
            candidate_key(&original, &candidate)
                .expect("deletion subsequence")
                .as_ref(),
            &[0b101]
        );
        assert_eq!(
            candidate_key(&original, &candidate),
            candidate_key(&original, &candidate)
        );
    }

    #[test]
    fn duplicate_rich_subsequence_keys_are_equal_iff_plans_are_equal() {
        let a = injection(10, FaultKind::CrashRestart);
        let b = injection(20, FaultKind::NetworkDelay { delay_nanos: 7 });
        let c = injection(30, FaultKind::DiskWriteFail);
        let original = FaultPlan::new(vec![a.clone(), a.clone(), a, b.clone(), b, c]);
        let candidates: Vec<(FaultPlan, Box<[u64]>)> = (0u64..(1 << 6))
            .map(|mask| {
                let candidate = FaultPlan::new(
                    original
                        .injections()
                        .iter()
                        .enumerate()
                        .filter(|(index, _)| mask & (1 << index) != 0)
                        .map(|(_, injection)| injection.clone())
                        .collect(),
                );
                let key = candidate_key(&original, &candidate).expect("deletion subsequence");
                (candidate, key)
            })
            .collect();

        for (left_plan, left_key) in &candidates {
            for (right_plan, right_key) in &candidates {
                assert_eq!(left_key == right_key, left_plan == right_plan);
            }
        }
    }

    #[test]
    fn every_fault_variant_and_payload_has_a_distinct_exact_key() {
        let original = FaultPlan::new(vec![
            injection(10, FaultKind::CrashRestart),
            injection(10, FaultKind::NetworkDelay { delay_nanos: 7 }),
            injection(10, FaultKind::NetworkDelay { delay_nanos: 8 }),
            injection(10, FaultKind::NetworkPartition { duration_nanos: 7 }),
            injection(10, FaultKind::NetworkPartition { duration_nanos: 8 }),
            injection(10, FaultKind::DiskWriteFail),
            injection(10, FaultKind::ClockSkew { skew_nanos: 7 }),
            injection(10, FaultKind::ClockSkew { skew_nanos: 8 }),
        ]);

        let keys: Vec<Box<[u64]>> = original
            .injections()
            .iter()
            .enumerate()
            .map(|(index, injection)| {
                let key = candidate_key(&original, &FaultPlan::new(vec![injection.clone()]))
                    .expect("singleton deletion subsequence");
                assert_eq!(key.as_ref(), &[1u64 << index]);
                key
            })
            .collect();
        for (index, key) in keys.iter().enumerate() {
            assert!(!keys[..index].contains(key));
        }
    }

    #[test]
    fn large_plan_keys_are_compact_fixed_width_bitsets() {
        let original = FaultPlan::new(
            (0..2_000)
                .map(|at_nanos| injection(at_nanos, FaultKind::CrashRestart))
                .collect(),
        );
        let key = candidate_key(&original, &original).expect("identity subsequence");
        assert_eq!(key.len(), 32);
        assert_eq!(key.iter().map(|word| word.count_ones()).sum::<u32>(), 2_000);
    }

    #[test]
    fn a_non_subsequence_fails_closed_instead_of_aliasing() {
        let original = FaultPlan::new(vec![injection(10, FaultKind::CrashRestart)]);
        let candidate = FaultPlan::new(vec![injection(11, FaultKind::CrashRestart)]);

        assert_eq!(
            candidate_key(&original, &candidate),
            Err(ShrinkError::InternalCandidateNotSubsequence { candidate })
        );
    }
}
