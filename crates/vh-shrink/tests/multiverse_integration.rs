#![forbid(unsafe_code)]

use vh_gremlin::{FaultInjection, FaultKind, FaultPlan};
use vh_multiverse::{
    run_universe_with_fault_plan, FaultPlanDiscipline, RunOutcome, UniverseCtx, UniverseResult,
    Workload,
};
use vh_shrink::{try_shrink, OracleVerification};

const SEED: u64 = 0xD1CE;
const UNIVERSE: u64 = 7;
const PROPERTY: &str = "crash_and_disk_failure_must_not_interact";
const EXPECTED_DETAIL: &str = "crash and disk failure were both present";

struct InteractingFaultWorkload;

impl Workload for InteractingFaultWorkload {
    fn name(&self) -> &str {
        "vh-shrink-interacting-faults"
    }

    fn run(&self, ctx: &mut UniverseCtx) -> RunOutcome {
        let plan = ctx.fault_plan_or(FaultPlan::default);
        let has_crash = plan
            .injections()
            .iter()
            .any(|injection| injection.fault == FaultKind::CrashRestart);
        let has_disk_failure = plan
            .injections()
            .iter()
            .any(|injection| injection.fault == FaultKind::DiskWriteFail);

        ctx.record(
            "verify.fault-plan",
            &format!("injections={}", plan.injections().len()),
        );
        ctx.always(PROPERTY, !(has_crash && has_disk_failure), || {
            EXPECTED_DETAIL.to_string()
        });
        RunOutcome::Completed
    }
}

fn injection(at_nanos: u64, fault: FaultKind) -> FaultInjection {
    FaultInjection { at_nanos, fault }
}

fn fails(plan: &FaultPlan) -> bool {
    let first =
        run_universe_with_fault_plan(SEED, UNIVERSE, &InteractingFaultWorkload, plan.clone());
    let replay =
        run_universe_with_fault_plan(SEED, UNIVERSE, &InteractingFaultWorkload, plan.clone());
    replay_pair_reproduces_expected_failure(&first, &replay)
}

fn replay_pair_reproduces_expected_failure(
    first: &UniverseResult,
    replay: &UniverseResult,
) -> bool {
    // The Boolean oracle maps non-replayability to false because its public
    // compatibility surface has only two states. A future typed oracle should
    // distinguish NonReplayable from a genuine different-failure result.
    first.observably_equal(replay) && reproduces_expected_failure(first)
}

fn reproduces_expected_failure(result: &UniverseResult) -> bool {
    let failures = result.always_failures();
    result.lifecycle().outcome() == &RunOutcome::Completed
        && result.lifecycle().fault_plan() == &FaultPlanDiscipline::OverrideRetrieved
        && failures.len() == 1
        && failures[0].name == PROPERTY
        && failures[0].detail == EXPECTED_DETAIL
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

#[test]
fn shrinker_minimizes_through_the_public_fault_plan_replay_hook() {
    let input = FaultPlan::new(vec![
        injection(1, FaultKind::NetworkDelay { delay_nanos: 3 }),
        injection(10, FaultKind::CrashRestart),
        injection(10, FaultKind::ClockSkew { skew_nanos: 2 }),
        injection(10, FaultKind::DiskWriteFail),
        injection(20, FaultKind::NetworkPartition { duration_nanos: 5 }),
        injection(30, FaultKind::NetworkDelay { delay_nanos: 1 }),
    ]);
    let expected = FaultPlan::new(vec![
        injection(10, FaultKind::CrashRestart),
        injection(10, FaultKind::DiskWriteFail),
    ]);

    let report = try_shrink(input, fails).expect("input plan must reproduce the named failure");

    assert_eq!(report.original_injections(), 6);
    assert_eq!(report.minimized_injections(), 2);
    assert_eq!(report.plan(), &expected);
    assert_eq!(
        report.oracle_verification(),
        OracleVerification::PairedVerdictChecked
    );
    assert_eq!(report.oracle_calls(), report.distinct_candidates() * 2);
    assert!(fails(report.plan()));

    let first = run_universe_with_fault_plan(
        SEED,
        UNIVERSE,
        &InteractingFaultWorkload,
        report.plan().clone(),
    );
    let replay = run_universe_with_fault_plan(
        SEED,
        UNIVERSE,
        &InteractingFaultWorkload,
        report.plan().clone(),
    );
    assert!(first.observably_equal(&replay));

    for removed in 0..report.plan().injections().len() {
        assert!(!fails(&without(report.plan(), removed)));
    }
}

struct ReportsFailureWithoutConsumingOverride;

impl Workload for ReportsFailureWithoutConsumingOverride {
    fn name(&self) -> &str {
        "vh-shrink-invalid-lifecycle"
    }

    fn run(&self, ctx: &mut UniverseCtx) -> RunOutcome {
        ctx.always(PROPERTY, false, || EXPECTED_DETAIL.to_string());
        RunOutcome::Completed
    }
}

#[test]
fn expected_failure_with_invalid_replay_lifecycle_is_not_a_reproducer() {
    let result = run_universe_with_fault_plan(
        SEED,
        UNIVERSE,
        &ReportsFailureWithoutConsumingOverride,
        FaultPlan::new(vec![injection(0, FaultKind::CrashRestart)]),
    );

    assert!(result
        .always_failures()
        .iter()
        .any(|failure| failure.name == PROPERTY && failure.detail == EXPECTED_DETAIL));
    assert_eq!(result.lifecycle().outcome(), &RunOutcome::Completed);
    assert_eq!(
        result.lifecycle().fault_plan(),
        &FaultPlanDiscipline::OverrideNeverRetrieved
    );
    assert!(!reproduces_expected_failure(&result));
}

struct ReportsSameNameWithDifferentDetail;

impl Workload for ReportsSameNameWithDifferentDetail {
    fn name(&self) -> &str {
        "vh-shrink-different-fingerprint"
    }

    fn run(&self, ctx: &mut UniverseCtx) -> RunOutcome {
        let _ = ctx.fault_plan_or(FaultPlan::default);
        ctx.always(PROPERTY, false, || "different causal detail".to_string());
        RunOutcome::Completed
    }
}

#[test]
fn same_property_name_with_different_detail_is_not_the_expected_fingerprint() {
    let result = run_universe_with_fault_plan(
        SEED,
        UNIVERSE,
        &ReportsSameNameWithDifferentDetail,
        FaultPlan::new(vec![injection(0, FaultKind::CrashRestart)]),
    );

    assert_eq!(result.lifecycle().outcome(), &RunOutcome::Completed);
    assert_eq!(
        result.lifecycle().fault_plan(),
        &FaultPlanDiscipline::OverrideRetrieved
    );
    assert!(result
        .always_failures()
        .iter()
        .any(|failure| failure.name == PROPERTY && failure.detail == "different causal detail"));
    assert!(!reproduces_expected_failure(&result));
}

#[test]
fn full_observable_divergence_is_not_a_reproducer_even_when_fingerprint_matches() {
    let minimal = FaultPlan::new(vec![
        injection(10, FaultKind::CrashRestart),
        injection(10, FaultKind::DiskWriteFail),
    ]);
    let extra_fault = FaultPlan::new(vec![
        injection(1, FaultKind::NetworkDelay { delay_nanos: 3 }),
        injection(10, FaultKind::CrashRestart),
        injection(10, FaultKind::DiskWriteFail),
    ]);
    let first = run_universe_with_fault_plan(SEED, UNIVERSE, &InteractingFaultWorkload, minimal);
    let replay =
        run_universe_with_fault_plan(SEED, UNIVERSE, &InteractingFaultWorkload, extra_fault);

    assert!(reproduces_expected_failure(&first));
    assert!(reproduces_expected_failure(&replay));
    assert!(!first.observably_equal(&replay));
    assert!(!replay_pair_reproduces_expected_failure(&first, &replay));
}
