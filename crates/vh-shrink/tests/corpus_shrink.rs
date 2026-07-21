#![forbid(unsafe_code)]

use vh_gremlin::{FaultInjection, FaultKind, FaultPlan};
use vh_multiverse::{
    run_universe_with_fault_plan, FaultPlanDiscipline, RunOutcome, UniverseResult, Workload,
};
use vh_shrink::{try_shrink, EvidenceIdentity, OracleVerification};
use vh_verify::observably_equal_independent;

const ROOT_SEED: u64 = 0xD1CE;
const FAILURE_FINGERPRINT_SCHEMA: &str = "vh-shrink-failure-v1";
const FNV128_OFFSET: u128 = 0x6c62_272e_07bb_0142_62b8_2175_6295_c58d;
const FNV128_PRIME: u128 = 0x0000_0000_0100_0000_0000_0000_0000_013B;

fn injection(at_nanos: u64, fault: FaultKind) -> FaultInjection {
    FaultInjection { at_nanos, fault }
}

fn fingerprint_absorb(mut state: u128, bytes: &[u8]) -> u128 {
    for byte in bytes {
        state ^= u128::from(*byte);
        state = state.wrapping_mul(FNV128_PRIME);
    }
    state
}

fn fingerprint_absorb_bytes(state: u128, bytes: &[u8]) -> u128 {
    let length = u64::try_from(bytes.len()).expect("failure field length must fit u64");
    let state = fingerprint_absorb(state, &length.to_le_bytes());
    fingerprint_absorb(state, bytes)
}

/// Verifier-owned failure projection: valid replay lifecycle plus the complete
/// ordered failure transcript. It intentionally excludes trace/runtime fields
/// that change when irrelevant injections are deleted.
fn failure_fingerprint(result: &UniverseResult, expected_oracle: &str) -> Option<String> {
    if result.lifecycle().outcome() != &RunOutcome::Completed
        || result.lifecycle().fault_plan() != &FaultPlanDiscipline::OverrideRetrieved
        || !result
            .always_failures()
            .iter()
            .any(|failure| failure.name == expected_oracle)
    {
        return None;
    }

    let mut state = fingerprint_absorb_bytes(FNV128_OFFSET, FAILURE_FINGERPRINT_SCHEMA.as_bytes());
    state = fingerprint_absorb(
        state,
        &u64::try_from(result.always_failures().len())
            .expect("failure count must fit u64")
            .to_le_bytes(),
    );
    for failure in result.always_failures() {
        state = fingerprint_absorb_bytes(state, failure.name.as_bytes());
        state = fingerprint_absorb_bytes(state, failure.detail.as_bytes());
    }
    Some(format!("{state:032x}"))
}

fn candidate_reproduces(
    workload: &dyn Workload,
    universe: u64,
    plan: &FaultPlan,
    expected_oracle: &str,
    expected_failure_fingerprint: &str,
) -> bool {
    let first = run_universe_with_fault_plan(ROOT_SEED, universe, workload, plan.clone());
    let replay = run_universe_with_fault_plan(ROOT_SEED, universe, workload, plan.clone());
    first.observably_equal(&replay)
        && observably_equal_independent(&first, &replay)
        && failure_fingerprint(&first, expected_oracle).as_deref()
            == Some(expected_failure_fingerprint)
}

fn capture_failure(
    workload: &dyn Workload,
    universe: u64,
    plan: &FaultPlan,
    expected_oracle: &str,
) -> String {
    let first = run_universe_with_fault_plan(ROOT_SEED, universe, workload, plan.clone());
    let replay = run_universe_with_fault_plan(ROOT_SEED, universe, workload, plan.clone());
    assert!(first.observably_equal(&replay));
    assert!(observably_equal_independent(&first, &replay));
    failure_fingerprint(&first, expected_oracle).expect("initial corpus plan must reproduce failure")
}

fn without(plan: &FaultPlan, removed: usize) -> FaultPlan {
    FaultPlan::new(
        plan.injections()
            .iter()
            .enumerate()
            .filter(|(index, _)| *index != removed)
            .map(|(_, injection)| injection.clone())
            .collect(),
    )
}

// The source under shrink is the Track-1 tip this verifier is stacked on.
// These are audited-source identities, not a self-referential hash of this
// verifier commit; the Actions receipt separately records the exact harness
// head/tree that executed the manifest.
const AUDITED_SOURCE_COMMIT: &str = "6760e9902fc7cab35de4728e82102a3a7c594612";
const AUDITED_SOURCE_TREE: &str = "7ee68b1301f61999070e91099995d0769f06e333";
const BUILD_IDENTITY: &str =
    "rustc-1.94.1-e408947bf-x86_64-unknown-linux-gnu;profile=test;locked-offline";

fn shrink_corpus_case(
    workload_name: &str,
    universe: u64,
    oracle_name: &str,
    input: FaultPlan,
    expected_minimized: FaultPlan,
) {
    let workload = vh_cli::workloads::by_name(workload_name).expect("public corpus workload");
    let failure = capture_failure(&*workload, universe, &input, oracle_name);
    let oracle_identity = format!("{oracle_name}@{FAILURE_FINGERPRINT_SCHEMA}");

    let report = try_shrink(input.clone(), |candidate| {
        candidate_reproduces(&*workload, universe, candidate, oracle_name, &failure)
    })
    .expect("corpus plan must shrink under the exact failure oracle");

    assert_eq!(report.original_plan(), &input);
    assert_eq!(report.plan(), &expected_minimized);
    assert_eq!(
        report.oracle_verification(),
        OracleVerification::PairedVerdictChecked
    );
    assert_eq!(report.oracle_calls(), report.distinct_candidates() * 2);
    assert!(candidate_reproduces(
        &*workload,
        universe,
        report.plan(),
        oracle_name,
        &failure
    ));
    for removed in 0..report.plan().injections().len() {
        assert!(!candidate_reproduces(
            &*workload,
            universe,
            &without(report.plan(), removed),
            oracle_name,
            &failure
        ));
    }

    let identity = EvidenceIdentity::new(
        AUDITED_SOURCE_COMMIT,
        AUDITED_SOURCE_TREE,
        BUILD_IDENTITY,
        workload.name(),
        ROOT_SEED,
        universe,
        &oracle_identity,
        &failure,
    )
    .expect("complete evidence identity");
    let manifest = report.bind_evidence(identity.clone());
    let replay_manifest = report.bind_evidence(identity);
    assert_eq!(manifest, replay_manifest);
    assert_eq!(manifest.identity().failure_fingerprint(), failure);
    assert_eq!(manifest.identity().oracle_identity(), oracle_identity);
    assert_ne!(
        manifest.original_plan_fingerprint(),
        manifest.minimized_plan_fingerprint()
    );
    assert_eq!(manifest.minimized_injections(), expected_minimized.injections().len());

    println!(
        "shrink-evidence: manifest-fingerprint={} {}",
        manifest.fingerprint(),
        manifest.canonical()
    );
}

#[test]
fn duplicate_corpus_failure_shrinks_and_binds_publication_evidence() {
    let input = FaultPlan::new(vec![
        injection(0, FaultKind::ClockSkew { skew_nanos: 1 }),
        injection(0, FaultKind::DiskWriteFail),
        injection(0, FaultKind::TornWrite),
        injection(0, FaultKind::FsyncLie),
        injection(0, FaultKind::NetworkDuplicate),
    ]);
    let expected = FaultPlan::new(vec![injection(0, FaultKind::NetworkDuplicate)]);
    shrink_corpus_case(
        "corpus-retry-double-apply",
        1,
        "oracle:exactly_once",
        input,
        expected,
    );
}

#[test]
fn fsync_lie_corpus_failure_shrinks_to_lie_plus_crash_and_binds_evidence() {
    let input = FaultPlan::new(vec![
        injection(0, FaultKind::ClockSkew { skew_nanos: 1 }),
        injection(0, FaultKind::NetworkDelay { delay_nanos: 0 }),
        injection(0, FaultKind::NetworkDuplicate),
        injection(0, FaultKind::NetworkReorder),
        injection(70_000, FaultKind::FsyncLie),
        injection(100_000, FaultKind::CrashRestart),
    ]);
    let expected = FaultPlan::new(vec![
        injection(70_000, FaultKind::FsyncLie),
        injection(100_000, FaultKind::CrashRestart),
    ]);
    shrink_corpus_case(
        "corpus-fsync-lie",
        5,
        "oracle:wal_durability",
        input,
        expected,
    );
}
