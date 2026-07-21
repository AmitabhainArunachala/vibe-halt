#![forbid(unsafe_code)]

use std::panic::{catch_unwind, AssertUnwindSafe};

use vh_gremlin::{FaultInjection, FaultKind, FaultPlan};
use vh_multiverse::{
    run_universe_with_fault_plan, PropertyContract, RunOutcome, StepEvent, UniverseCtx, Workload,
};

const SEED: u64 = 0xD1CE;
const UNIVERSE: u64 = 5;

fn injection(at_nanos: u64, fault: FaultKind) -> FaultInjection {
    FaultInjection { at_nanos, fault }
}

struct StackedReorders;

impl Workload for StackedReorders {
    fn name(&self) -> &str {
        "vh-verify-loop5-stacked-reorders"
    }

    fn property_contract(&self) -> PropertyContract {
        PropertyContract::new(&["every_send_is_delivered_or_attested_dropped"], &[])
    }

    fn run(&self, ctx: &mut UniverseCtx) -> RunOutcome {
        let mut rt = ctx.runtime(FaultPlan::default);
        let armed = rt.step().is_none();
        rt.send(1, 2, "A");
        rt.send(1, 2, "B");
        rt.send(1, 2, "C");

        let mut delivered = Vec::new();
        while let Some(event) = rt.step() {
            if let StepEvent::Delivered { payload, .. } = event {
                delivered.push(payload);
            }
        }
        let drops = rt.drops();
        rt.always(
            "every_send_is_delivered_or_attested_dropped",
            armed && delivered.len() as u64 + drops == 3,
            || format!("delivered={delivered:?} drops={drops}"),
        );
        rt.finish();
        RunOutcome::Completed
    }
}

#[test]
#[ignore = "Track-1 hardening-loop-5 expected-failure repro"]
fn stacked_reorders_do_not_silently_lose_messages() {
    let result = run_universe_with_fault_plan(
        SEED,
        UNIVERSE,
        &StackedReorders,
        FaultPlan::new(vec![
            injection(0, FaultKind::NetworkReorder),
            injection(0, FaultKind::NetworkReorder),
        ]),
    );
    assert!(
        result.always_failures().is_empty(),
        "LOOP5_REPRO_STACKED_REORDER: stacked one-shot reorders accepted three sends but neither delivered nor attested a drop for all three; failures={:?}",
        result.always_failures()
    );
}

struct TornRecoveryBeforeRead;

impl Workload for TornRecoveryBeforeRead {
    fn name(&self) -> &str {
        "vh-verify-loop5-torn-recovery-before-read"
    }

    fn property_contract(&self) -> PropertyContract {
        PropertyContract::new(&["torn_path_completed"], &[])
    }

    fn run(&self, ctx: &mut UniverseCtx) -> RunOutcome {
        let mut rt = ctx.runtime(FaultPlan::default);
        let armed = rt.step().is_none();
        let torn_ok = rt.disk_write(0, "abcdef").is_ok();
        rt.set_timer(10, 1);
        let timer_10 = matches!(rt.step(), Some(StepEvent::Timer { token: 1 }));
        let intact_ok = rt.disk_write(0, "abcdef").is_ok();
        rt.set_timer(20, 2);
        let timer_20 = matches!(rt.step(), Some(StepEvent::Timer { token: 2 }));
        let visible = rt.disk_read_all(0);
        rt.always(
            "torn_path_completed",
            armed
                && torn_ok
                && timer_10
                && intact_ok
                && timer_20
                && visible == vec!["abc".to_string(), "abcdef".to_string()],
            || {
                format!(
                    "armed={armed} torn={torn_ok} timer10={timer_10} intact={intact_ok} timer20={timer_20} visible={visible:?}"
                )
            },
        );
        rt.finish();
        RunOutcome::Completed
    }
}

#[test]
#[ignore = "Track-1 hardening-loop-5 expected-failure repro"]
fn torn_write_lifecycle_never_recovers_before_manifesting() {
    let result = run_universe_with_fault_plan(
        SEED,
        UNIVERSE,
        &TornRecoveryBeforeRead,
        FaultPlan::new(vec![injection(0, FaultKind::TornWrite)]),
    );
    assert!(result.always_failures().is_empty());
    let evidence = result.runtime_evidence().expect("runtime evidence");
    let torn = evidence.injections().first().expect("torn injection");
    let manifested = torn.manifested_at().expect("torn prefix was read");
    let recovered = torn.recovered_at().expect("intact write marked recovery");
    assert!(
        manifested <= recovered,
        "LOOP5_REPRO_TORN_LADDER: observed Manifested at {manifested} after Recovered at {recovered}; the published Tier-1 ladder is not timestamp-monotone"
    );
}

fn gate_block<'a>(gate: &'a str, workload: &str) -> &'a str {
    let heading = format!("== corpus recall gate: {workload}");
    let start = gate.find(&heading).expect("corpus gate heading");
    let remainder = &gate[start + heading.len()..];
    let end = remainder.find("\necho \"== ").unwrap_or(remainder.len());
    &remainder[..end]
}

#[test]
#[ignore = "Track-1 hardening-loop-5 expected-failure repro"]
fn corpus_recall_gates_hold_the_exact_pinned_counts() {
    const SCHEMA: &str = include_str!("../../../corpus/SCHEMA.md");
    const GATE: &str = include_str!("../../../scripts/gate.sh");
    assert!(SCHEMA.contains("The gate then holds exactly that claim."));

    for (workload, pinned) in [
        ("corpus-lost-update", 29),
        ("corpus-retry-double-apply", 76),
        ("corpus-dirty-read", 83),
        ("corpus-crash-toctou", 21),
        ("corpus-fsync-lie", 21),
    ] {
        let block = gate_block(GATE, workload);
        let exact = format!("\"$fails\" -ne {pinned}");
        assert!(
            block.contains(&exact),
            "LOOP5_REPRO_CORPUS_RECALL: {workload} publishes found {pinned}/100, but its gate does not enforce that exact count; block={block:?}"
        );
    }
}

struct FakeLifecycleTraceEvent;

impl Workload for FakeLifecycleTraceEvent {
    fn name(&self) -> &str {
        "vh-verify-loop5-fake-lifecycle-event"
    }

    fn property_contract(&self) -> PropertyContract {
        PropertyContract::new(&["workload_completed"], &[])
    }

    fn run(&self, ctx: &mut UniverseCtx) -> RunOutcome {
        let mut rt = ctx.runtime(FaultPlan::default);
        rt.record("fault.manifested", "i=0 network_delay:0");
        rt.always("workload_completed", true, String::new);
        rt.finish();
        RunOutcome::Completed
    }
}

#[test]
#[ignore = "Track-1 hardening-loop-5 expected-failure repro"]
fn workload_cannot_emit_runner_reserved_lifecycle_trace_kinds() {
    let attempted = catch_unwind(AssertUnwindSafe(|| {
        run_universe_with_fault_plan(
            SEED,
            UNIVERSE,
            &FakeLifecycleTraceEvent,
            FaultPlan::default(),
        )
    }));
    assert!(
        attempted.is_err(),
        "LOOP5_REPRO_LIFECYCLE_SPOOF: SimRuntime::record accepted the runner-reserved fault.manifested kind; the RuntimeEvidence ledger remains unforgeable, but the raw trace namespace is spoofable"
    );
}
