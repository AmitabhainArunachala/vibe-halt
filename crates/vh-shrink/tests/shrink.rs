#![forbid(unsafe_code)]

use std::num::NonZeroUsize;

use vh_gremlin::{FaultInjection, FaultKind, FaultPlan};
use vh_shrink::{
    shrink, try_shrink, try_shrink_caller_asserted, try_shrink_with_config, OracleVerification,
    ShrinkConfig, ShrinkError, DEFAULT_MAX_CACHE_WEIGHT_BYTES, DEFAULT_MAX_INITIAL_INJECTIONS,
    DEFAULT_MAX_ORACLE_CALLS,
};

fn injection(at_nanos: u64) -> FaultInjection {
    FaultInjection {
        at_nanos,
        fault: FaultKind::CrashRestart,
    }
}

fn plan(times: &[u64]) -> FaultPlan {
    FaultPlan::new(times.iter().copied().map(injection).collect())
}

fn contains(candidate: &FaultPlan, at_nanos: u64) -> bool {
    candidate
        .injections()
        .iter()
        .any(|injection| injection.at_nanos == at_nanos)
}

fn without(candidate: &FaultPlan, removed: usize) -> FaultPlan {
    FaultPlan::new(
        candidate
            .injections()
            .iter()
            .enumerate()
            .filter(|(index, _)| *index != removed)
            .map(|(_, injection)| injection.clone())
            .collect(),
    )
}

#[test]
fn removes_every_fault_except_the_single_cause() {
    let minimized = shrink(plan(&[10, 20, 30, 40, 50]), |candidate| {
        contains(candidate, 30)
    });
    assert_eq!(minimized, plan(&[30]));
}

#[test]
fn preserves_an_interacting_pair_in_original_order() {
    let minimized = shrink(plan(&[10, 20, 30, 40, 50, 60]), |candidate| {
        contains(candidate, 20) && contains(candidate, 50)
    });
    assert_eq!(minimized, plan(&[20, 50]));
    for index in 0..minimized.injections().len() {
        let candidate = without(&minimized, index);
        assert!(!(contains(&candidate, 20) && contains(&candidate, 50)));
    }
}

#[test]
fn broad_any_failure_oracle_can_switch_causes_but_exact_fingerprint_preserves_original() {
    fn fingerprint(candidate: &FaultPlan) -> Option<&'static str> {
        if contains(candidate, 10) {
            Some("original-cause")
        } else if contains(candidate, 20) {
            Some("different-cause")
        } else {
            None
        }
    }

    let input = plan(&[10, 20]);
    let original_fingerprint = fingerprint(&input).expect("original plan fails");

    // A broad "any failure" predicate accepts the first failing singleton
    // ddmin encounters, even though its causal fingerprint changed.
    let broad =
        try_shrink_caller_asserted(input.clone(), |candidate| fingerprint(candidate).is_some())
            .expect("the broad predicate accepts either failure");
    assert_eq!(broad.plan(), &plan(&[20]));
    assert_eq!(fingerprint(broad.plan()), Some("different-cause"));

    // The required contract compares the exact fingerprint captured from the
    // original input, so the different failure cannot replace it.
    let exact = try_shrink_caller_asserted(input, |candidate| {
        fingerprint(candidate) == Some(original_fingerprint)
    })
    .expect("the exact original fingerprint remains reproducible");
    assert_eq!(exact.plan(), &plan(&[10]));
    assert_eq!(fingerprint(exact.plan()), Some(original_fingerprint));
}

#[test]
fn returns_empty_when_the_empty_plan_still_fails() {
    let minimized = shrink(plan(&[10, 20, 30]), |_| true);
    assert_eq!(minimized, FaultPlan::default());
}

#[test]
fn empty_input_is_queried_once() {
    let mut calls = 0;
    let minimized = shrink(FaultPlan::default(), |_| {
        calls += 1;
        true
    });
    assert_eq!(minimized, FaultPlan::default());
    assert_eq!(calls, 1);
}

#[test]
fn preserves_a_failing_singleton() {
    let minimized = shrink(plan(&[10]), |candidate| contains(candidate, 10));
    assert_eq!(minimized, plan(&[10]));
}

#[test]
fn preserves_required_duplicate_injections() {
    let minimized = shrink(plan(&[10, 20, 20, 30]), |candidate| {
        candidate
            .injections()
            .iter()
            .filter(|injection| injection.at_nanos == 20)
            .count()
            == 2
    });
    assert_eq!(minimized, plan(&[20, 20]));
}

#[test]
fn checked_api_rejects_a_nonfailing_input() {
    let input = plan(&[10, 20, 30]);
    let mut calls = 0;
    let result = try_shrink(input.clone(), |_| {
        calls += 1;
        false
    });
    let failure = result.expect_err("input must be rejected");
    assert_eq!(failure.original_plan(), &input);
    assert_eq!(failure.best_plan(), None);
    assert_eq!(
        failure.cause(),
        &ShrinkError::InitialPlanDidNotFail { oracle_calls: 2 }
    );
    assert_eq!(calls, 2);
}

#[test]
fn oracle_panic_preserves_typed_candidate_and_best_reproducer_lineage() {
    let input = plan(&[10]);
    let original = input.clone();
    let mut calls = 0usize;
    let failure = try_shrink(input, |candidate| {
        calls += 1;
        if candidate.injections().is_empty() {
            panic!("oracle panic fixture");
        }
        true
    })
    .expect_err("the empty candidate must panic on its first paired call");

    assert_eq!(calls, 3);
    assert_eq!(failure.original_plan(), &original);
    assert_eq!(failure.best_plan(), Some(&original));
    assert_eq!(failure.oracle_calls(), 3);
    assert_eq!(failure.distinct_candidates(), 2);
    assert_eq!(
        failure.cache_weight_bytes(),
        128 + std::mem::size_of::<u64>()
    );
    assert_eq!(
        failure.cause(),
        &ShrinkError::OraclePanicked {
            candidate: FaultPlan::default(),
            oracle_calls: 3,
        }
    );
}

#[test]
fn initial_oracle_panic_accounting_is_frozen_for_either_paired_call() {
    let input = plan(&[10]);

    for panic_at in [1usize, 2] {
        let mut calls = 0usize;
        let failure = try_shrink(input.clone(), |_| {
            calls += 1;
            if calls == panic_at {
                panic!("initial paired-call panic fixture");
            }
            true
        })
        .expect_err("the initial candidate must fail closed on either panic");

        assert_eq!(calls, panic_at);
        assert_eq!(failure.original_plan(), &input);
        assert_eq!(failure.best_plan(), None);
        assert_eq!(failure.oracle_calls(), panic_at);
        assert_eq!(failure.distinct_candidates(), 1);
        assert_eq!(failure.cache_weight_bytes(), 0);
        assert_eq!(
            failure.cause(),
            &ShrinkError::OraclePanicked {
                candidate: input.clone(),
                oracle_calls: panic_at,
            }
        );
    }
}

#[test]
#[should_panic(expected = "initial fault plan did not reproduce")]
fn compatibility_api_fails_closed_for_a_nonfailing_input() {
    let _ = shrink(plan(&[10, 20, 30]), |_| false);
}

#[test]
fn oracle_call_order_and_result_are_repeatable() {
    fn run() -> (FaultPlan, Vec<Vec<u64>>) {
        let mut calls = Vec::new();
        let minimized = shrink(plan(&[10, 20, 30, 40, 50]), |candidate| {
            calls.push(
                candidate
                    .injections()
                    .iter()
                    .map(|injection| injection.at_nanos)
                    .collect(),
            );
            contains(candidate, 20) && contains(candidate, 40)
        });
        (minimized, calls)
    }

    assert_eq!(run(), run());
}

fn subset_mask(candidate: &FaultPlan) -> usize {
    candidate
        .injections()
        .iter()
        .fold(0usize, |mask, fault| mask | (1usize << fault.at_nanos))
}

fn table_verdict(truth_table: u32, candidate: &FaultPlan) -> bool {
    truth_table & (1u32 << subset_mask(candidate)) != 0
}

#[test]
fn exhaustive_four_fault_oracles_produce_one_minimal_ordered_subsequences() {
    let input = plan(&[0, 1, 2, 3]);
    let full_plan_bit = 1u32 << 0b1111;
    let mut checked = 0u32;

    // A four-element input has 16 subsets and therefore 2^16 Boolean
    // oracles. Check every oracle for which the full input reproduces.
    for truth_table in 0u32..=u16::MAX as u32 {
        if truth_table & full_plan_bit == 0 {
            continue;
        }

        let report = try_shrink_caller_asserted(input.clone(), |candidate| {
            table_verdict(truth_table, candidate)
        })
        .expect("the full plan is failing by construction");

        assert!(table_verdict(truth_table, report.plan()));
        assert!(report
            .plan()
            .injections()
            .windows(2)
            .all(|pair| pair[0].at_nanos < pair[1].at_nanos));
        for removed in 0..report.plan().injections().len() {
            assert!(
                !table_verdict(truth_table, &without(report.plan(), removed)),
                "truth table {truth_table:#06x} was not 1-minimal"
            );
        }
        assert_eq!(report.original_injections(), 4);
        assert_eq!(
            report.minimized_injections(),
            report.plan().injections().len()
        );
        checked += 1;
    }

    assert_eq!(checked, 32_768);
}

#[test]
fn identical_candidates_are_never_replayed_within_one_shrink() {
    let input = plan(&[0, 1, 2]);
    let interacting = plan(&[0, 2]);
    let original = input.clone();
    let mut observed = Vec::new();

    let minimized = shrink(input, |candidate| {
        assert!(
            !observed.contains(candidate),
            "duplicate candidate oracle call: {candidate:?}"
        );
        observed.push(candidate.clone());
        candidate == &original || candidate == &interacting
    });

    assert_eq!(minimized, interacting);
}

#[test]
fn all_faults_required_has_a_linear_distinct_call_bound() {
    let times: Vec<u64> = (0..256).collect();
    let input = plan(&times);
    let original = input.clone();
    let mut calls = 0usize;

    let report = try_shrink_caller_asserted(input, |candidate| {
        calls += 1;
        candidate == &original
    })
    .expect("the original plan fails");

    assert_eq!(report.plan(), &original);
    assert_eq!(report.oracle_calls(), calls);
    assert_eq!(report.distinct_candidates(), calls);
    assert!(calls <= 512, "unexpected oracle-call growth: {calls}");
}

#[test]
fn dense_required_subset_stays_within_an_honest_quadratic_call_budget() {
    const SIZE: usize = 256;
    let times: Vec<u64> = (0..SIZE as u64).collect();
    let input = plan(&times);
    let required: Vec<u64> = times.iter().copied().filter(|time| time % 2 == 0).collect();
    let mut calls = 0usize;

    let report = try_shrink_caller_asserted(input, |candidate| {
        calls += 1;
        required.iter().all(|time| contains(candidate, *time))
    })
    .expect("the full plan contains the dense required subset");

    assert_eq!(report.plan().injections().len(), required.len());
    assert!(required.iter().all(|time| contains(report.plan(), *time)));
    assert_eq!(report.oracle_calls(), calls);
    assert_eq!(report.distinct_candidates(), calls);
    assert!(
        calls <= SIZE * SIZE,
        "unexpected ddmin call growth: {calls}"
    );
}

#[test]
fn paired_verdict_check_fails_closed_on_a_flaky_oracle() {
    let input = plan(&[10]);
    let mut calls = 0usize;
    let error = try_shrink(input.clone(), |_| {
        calls += 1;
        calls.is_multiple_of(2)
    })
    .expect_err("adjacent verdicts disagree");

    assert_eq!(error.original_plan(), &input);
    assert_eq!(error.best_plan(), None);
    assert_eq!(
        error.cause(),
        &ShrinkError::OracleDiverged {
            candidate: input,
            first: false,
            second: true,
            oracle_calls: 2,
        }
    );
    assert_eq!(calls, 2);
    assert_eq!(error.oracle_calls(), 2);
    assert_eq!(error.distinct_candidates(), 1);
    assert_eq!(error.cache_weight_bytes(), 0);
}

#[test]
fn paired_verdict_receipt_distinguishes_calls_from_candidates() {
    let report = try_shrink(plan(&[10, 20, 30]), |candidate| contains(candidate, 20))
        .expect("stable oracle");

    assert_eq!(report.plan(), &plan(&[20]));
    assert_eq!(
        report.oracle_verification(),
        OracleVerification::PairedVerdictChecked
    );
    assert_eq!(report.oracle_calls(), report.distinct_candidates() * 2);
}

#[test]
fn oracle_budget_is_enforced_before_starting_the_next_candidate() {
    let input = plan(&[10]);
    let original = input.clone();
    let config = ShrinkConfig::caller_asserted()
        .with_max_oracle_calls(NonZeroUsize::new(1).expect("one oracle call is a nonzero budget"));
    let mut calls = 0usize;
    let error = try_shrink_with_config(
        input,
        |candidate| {
            calls += 1;
            contains(candidate, 10)
        },
        config,
    )
    .expect_err("empty-plan evaluation would exceed the budget");

    assert_eq!(calls, 1);
    assert_eq!(error.original_plan(), &original);
    assert_eq!(error.best_plan(), Some(&original));
    assert!(matches!(
        error.cause(),
        ShrinkError::OracleBudgetExceeded {
            candidate,
            limit,
            oracle_calls: 1,
        } if candidate.injections().is_empty() && limit.get() == 1
    ));
}

#[test]
fn mid_run_failure_preserves_the_best_smaller_reproducer() {
    let input = plan(&[10, 20, 30, 40]);
    let config = ShrinkConfig::caller_asserted()
        .with_max_oracle_calls(NonZeroUsize::new(4).expect("four is nonzero"));
    let failure =
        try_shrink_with_config(input.clone(), |candidate| contains(candidate, 20), config)
            .expect_err("the fifth distinct candidate exceeds the budget");

    assert_eq!(failure.original_plan(), &input);
    assert_eq!(failure.best_plan(), Some(&plan(&[10, 20])));
    assert_eq!(failure.config(), config);
    assert_eq!(failure.oracle_calls(), 4);
    assert_eq!(failure.distinct_candidates(), 4);
    assert_eq!(
        failure.cache_weight_bytes(),
        4 * (128 + std::mem::size_of::<u64>())
    );
    assert!(matches!(
        failure.cause(),
        ShrinkError::OracleBudgetExceeded {
            candidate,
            limit,
            oracle_calls: 4,
        } if candidate == &plan(&[20]) && limit.get() == 4
    ));
}

#[test]
fn checked_default_is_paired_and_resource_bounded() {
    let config = ShrinkConfig::default();
    assert_eq!(
        config.verification(),
        OracleVerification::PairedVerdictChecked
    );
    assert_eq!(config.max_oracle_calls(), Some(DEFAULT_MAX_ORACLE_CALLS));
    assert_eq!(
        config.max_cache_weight_bytes(),
        DEFAULT_MAX_CACHE_WEIGHT_BYTES
    );
    assert_eq!(
        config.max_initial_injections(),
        DEFAULT_MAX_INITIAL_INJECTIONS
    );
}

#[test]
fn paired_oracle_budget_one_refuses_before_any_call() {
    let config = ShrinkConfig::paired_verdict()
        .with_max_oracle_calls(NonZeroUsize::new(1).expect("one is nonzero"));
    let input = plan(&[10]);
    let mut calls = 0usize;
    let error = try_shrink_with_config(
        input.clone(),
        |_| {
            calls += 1;
            true
        },
        config,
    )
    .expect_err("a paired candidate needs two calls");

    assert_eq!(calls, 0);
    assert_eq!(
        error.cause(),
        &ShrinkError::OracleBudgetExceeded {
            candidate: input,
            limit: NonZeroUsize::new(1).expect("one is nonzero"),
            oracle_calls: 0,
        }
    );
}

#[test]
fn paired_oracle_budget_three_never_starts_an_incomplete_pair() {
    let config = ShrinkConfig::paired_verdict()
        .with_max_oracle_calls(NonZeroUsize::new(3).expect("three is nonzero"));
    let input = plan(&[10]);
    let mut calls = 0usize;
    let error = try_shrink_with_config(
        input,
        |candidate| {
            calls += 1;
            contains(candidate, 10)
        },
        config,
    )
    .expect_err("the empty candidate cannot receive a complete pair");

    assert_eq!(calls, 2);
    assert!(matches!(
        error.cause(),
        ShrinkError::OracleBudgetExceeded {
            candidate,
            limit,
            oracle_calls: 2,
        } if candidate.injections().is_empty() && limit.get() == 3
    ));
}

#[test]
fn cache_weight_budget_fails_before_an_unrecordable_oracle_call() {
    let input = plan(&[10]);
    let original = input.clone();
    let one_entry_weight = 128 + std::mem::size_of::<u64>();
    let config = ShrinkConfig::caller_asserted().with_max_cache_weight_bytes(
        NonZeroUsize::new(one_entry_weight).expect("entry weight is nonzero"),
    );
    let mut calls = 0usize;
    let error = try_shrink_with_config(
        input,
        |candidate| {
            calls += 1;
            contains(candidate, 10)
        },
        config,
    )
    .expect_err("only the initial candidate fits the cache budget");

    assert_eq!(calls, 1);
    assert_eq!(error.original_plan(), &original);
    assert_eq!(error.best_plan(), Some(&original));
    assert!(matches!(
        error.cause(),
        ShrinkError::CacheBudgetExceeded {
            candidate,
            limit,
            used,
            required,
            oracle_calls: 1,
        } if candidate.injections().is_empty()
            && limit.get() == one_entry_weight
            && *used == one_entry_weight
            && *required == one_entry_weight
    ));
}

#[test]
fn cache_weight_budget_below_one_key_refuses_before_any_oracle_call() {
    let input = plan(&[10]);
    let limit = NonZeroUsize::new(127).expect("127 is nonzero");
    let config = ShrinkConfig::caller_asserted().with_max_cache_weight_bytes(limit);
    let mut calls = 0usize;
    let error = try_shrink_with_config(
        input.clone(),
        |_| {
            calls += 1;
            true
        },
        config,
    )
    .expect_err("the initial exact key cannot fit");

    assert_eq!(calls, 0);
    assert_eq!(
        error.cause(),
        &ShrinkError::CacheBudgetExceeded {
            candidate: input,
            limit,
            used: 0,
            required: 128 + std::mem::size_of::<u64>(),
            oracle_calls: 0,
        }
    );
}

#[test]
fn report_binds_the_exact_original_plan_lineage() {
    let input = plan(&[10, 20, 30]);
    let report = try_shrink(input.clone(), |candidate| contains(candidate, 20))
        .expect("stable paired oracle");
    assert_eq!(report.original_plan(), &input);
    assert_eq!(report.original_injections(), input.injections().len());
    assert_eq!(report.config(), ShrinkConfig::default());
    assert_eq!(
        report.cache_weight_bytes(),
        report.distinct_candidates() * (128 + std::mem::size_of::<u64>())
    );
}

#[test]
fn oversized_input_fails_before_the_oracle_and_preserves_retry_lineage() {
    let times: Vec<u64> = (0..=DEFAULT_MAX_INITIAL_INJECTIONS.get() as u64).collect();
    let input = plan(&times);
    let mut calls = 0usize;
    let failure = try_shrink(input.clone(), |_| {
        calls += 1;
        true
    })
    .expect_err("default input bound must fail closed");

    assert_eq!(calls, 0);
    assert_eq!(failure.original_plan(), &input);
    assert_eq!(failure.best_plan(), None);
    assert_eq!(failure.config(), ShrinkConfig::default());
    assert_eq!(failure.oracle_calls(), 0);
    assert_eq!(failure.distinct_candidates(), 0);
    assert_eq!(failure.cache_weight_bytes(), 0);
    assert_eq!(
        failure.cause(),
        &ShrinkError::InitialPlanTooLarge {
            injections: input.injections().len(),
            limit: DEFAULT_MAX_INITIAL_INJECTIONS,
        }
    );
}

#[test]
fn upstream_plan_canonicalization_is_preserved_in_shrink_lineage() {
    let input = FaultPlan::new(vec![injection(10), injection(30), injection(20)]);
    let canonical_times: Vec<u64> = input
        .injections()
        .iter()
        .map(|injection| injection.at_nanos)
        .collect();
    assert_eq!(canonical_times, vec![10, 20, 30]);

    let report = try_shrink(input.clone(), |candidate| contains(candidate, 20))
        .expect("the canonicalized input reproduces");
    assert_eq!(report.original_plan(), &input);
    assert_eq!(report.plan(), &plan(&[20]));
}

#[test]
fn public_errors_have_stable_human_readable_categories() {
    let error = try_shrink(plan(&[10]), |_| false).expect_err("input does not reproduce");
    assert_eq!(
        error.to_string(),
        "initial fault plan did not reproduce the failure"
    );
    let as_error: &dyn std::error::Error = &error;
    assert!(as_error.source().is_some());
    assert!(as_error.source().expect("typed cause").source().is_none());
}
