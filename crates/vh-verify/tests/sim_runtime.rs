#![forbid(unsafe_code)]

use vh_gremlin::{FaultInjection, FaultKind, FaultPlan};
use vh_multiverse::{
    run_universe_with_fault_plan, DeliveryNote, FaultPlanDiscipline, PropertyContract, RunOutcome,
    StepEvent, UniverseCtx, UniverseResult, Workload,
};
use vh_verify::{observable_fingerprint, observably_equal_independent, OBSERVABLE_FINGERPRINT_SCHEMA};

const SEED: u64 = 0xD1CE;
const UNIVERSE: u64 = 17;

fn injection(at_nanos: u64, fault: FaultKind) -> FaultInjection {
    FaultInjection { at_nanos, fault }
}

fn run_pair(label: &str, workload: &dyn Workload, plan: FaultPlan) -> UniverseResult {
    let first = run_universe_with_fault_plan(SEED, UNIVERSE, workload, plan.clone());
    let replay = run_universe_with_fault_plan(SEED, UNIVERSE, workload, plan);

    assert!(
        first.observably_equal(&replay),
        "Tier-1 runner observation diverged for {label}"
    );
    assert!(
        observably_equal_independent(&first, &replay),
        "Tier-1 verifier projection diverged for {label}"
    );
    assert_eq!(
        first.lifecycle().fault_plan(),
        &FaultPlanDiscipline::OverrideRetrieved
    );
    assert!(first.lifecycle().is_valid_completion());
    assert!(first.fault_plan_digest().is_some());
    assert!(first.runtime_evidence().is_some());
    assert!(first.always_failures().is_empty());
    assert!(first.always_checks().iter().all(|check| check.passed));

    println!(
        "verify-vector: vector={label} determinism-tier=Tier-1 observable-schema={} observable-fingerprint={} trace-hash={} events={}",
        OBSERVABLE_FINGERPRINT_SCHEMA,
        observable_fingerprint(&first),
        first.trace_hash(),
        first.trace_events()
    );
    first
}

fn only_injection(result: &UniverseResult) -> &vh_multiverse::InjectionOutcome {
    let evidence = result.runtime_evidence().expect("sim runtime evidence");
    assert_eq!(evidence.injections().len(), 1);
    &evidence.injections()[0]
}

struct PartitionInFlightSelfSend;

impl Workload for PartitionInFlightSelfSend {
    fn name(&self) -> &str {
        "vh-verify-partition-in-flight-self-send"
    }

    fn property_contract(&self) -> PropertyContract {
        PropertyContract::new(&["partition_drops_in_flight_then_self_send_recovers"], &[])
    }

    fn run(&self, ctx: &mut UniverseCtx) -> RunOutcome {
        let mut rt = ctx.runtime(FaultPlan::default);
        rt.send(9, 9, "in-flight");
        rt.set_timer(1_500, 1);

        let timer = rt.step();
        let timer_ok = matches!(&timer, Some(StepEvent::Timer { token: 1 }));
        let drops_after_partition = rt.drops();

        rt.send(9, 9, "after-heal");
        let delivered = rt.step();
        let delivery_ok = matches!(
            &delivered,
            Some(StepEvent::Delivered {
                from: 9,
                to: 9,
                payload,
                note: DeliveryNote {
                    delayed: false,
                    duplicate: false,
                    reordered: false,
                },
            }) if payload == "after-heal"
        );
        let drained = rt.step().is_none();
        let final_drops = rt.drops();

        rt.always(
            "partition_drops_in_flight_then_self_send_recovers",
            timer_ok
                && drops_after_partition == 1
                && delivery_ok
                && drained
                && final_drops == 1,
            || {
                format!(
                    "timer={timer:?} delivered={delivered:?} drops={drops_after_partition}->{final_drops} drained={drained}"
                )
            },
        );
        rt.finish();
        RunOutcome::Completed
    }
}

#[test]
fn partition_drops_an_in_flight_message_and_recovers_on_self_send() {
    let result = run_pair(
        "partition-in-flight-self-send",
        &PartitionInFlightSelfSend,
        FaultPlan::new(vec![injection(
            500,
            FaultKind::NetworkPartition {
                duration_nanos: 1_000,
            },
        )]),
    );
    let outcome = only_injection(&result);
    assert_eq!(outcome.fault(), "network_partition:1000");
    assert_eq!(outcome.offered_at(), Some(500));
    assert_eq!(outcome.armed_at(), Some(500));
    assert_eq!(outcome.injected_at(), Some(1_000));
    assert_eq!(outcome.manifested_at(), Some(1_000));
    assert_eq!(outcome.recovered_at(), Some(2_500));
}

struct ZeroDelaySelfSend;

impl Workload for ZeroDelaySelfSend {
    fn name(&self) -> &str {
        "vh-verify-zero-delay-self-send"
    }

    fn property_contract(&self) -> PropertyContract {
        PropertyContract::new(&["zero_delay_keeps_base_latency_and_note"], &[])
    }

    fn run(&self, ctx: &mut UniverseCtx) -> RunOutcome {
        let mut rt = ctx.runtime(FaultPlan::default);
        let armed = rt.step().is_none();
        rt.send(4, 4, "zero-delay");
        let delivered = rt.step();
        let delivery_ok = matches!(
            &delivered,
            Some(StepEvent::Delivered {
                from: 4,
                to: 4,
                payload,
                note: DeliveryNote {
                    delayed: true,
                    duplicate: false,
                    reordered: false,
                },
            }) if payload == "zero-delay"
        );
        let now = rt.now_nanos();
        rt.always(
            "zero_delay_keeps_base_latency_and_note",
            armed && delivery_ok && now == 1_000,
            || format!("armed={armed} delivered={delivered:?} now={now}"),
        );
        rt.finish();
        RunOutcome::Completed
    }
}

#[test]
fn zero_delay_is_still_a_shaped_delivery_at_base_latency() {
    let result = run_pair(
        "zero-delay-self-send",
        &ZeroDelaySelfSend,
        FaultPlan::new(vec![injection(
            0,
            FaultKind::NetworkDelay { delay_nanos: 0 },
        )]),
    );
    let outcome = only_injection(&result);
    assert_eq!(outcome.fault(), "network_delay:0");
    assert_eq!(outcome.offered_at(), Some(0));
    assert_eq!(outcome.armed_at(), Some(0));
    assert_eq!(outcome.injected_at(), Some(0));
    assert_eq!(outcome.manifested_at(), Some(1_000));
    assert_eq!(outcome.recovered_at(), Some(1_000));
}

struct DuplicateEdge;

impl Workload for DuplicateEdge {
    fn name(&self) -> &str {
        "vh-verify-duplicate-edge"
    }

    fn property_contract(&self) -> PropertyContract {
        PropertyContract::new(&["duplicate_is_original_then_tagged_copy"], &[])
    }

    fn run(&self, ctx: &mut UniverseCtx) -> RunOutcome {
        let mut rt = ctx.runtime(FaultPlan::default);
        let armed = rt.step().is_none();
        rt.send(1, 2, "dup");
        let mut deliveries = Vec::new();
        while let Some(event) = rt.step() {
            if let StepEvent::Delivered { payload, note, .. } = event {
                let DeliveryNote {
                    delayed,
                    duplicate,
                    reordered,
                } = note;
                deliveries.push((rt.now_nanos(), payload, delayed, duplicate, reordered));
            }
        }
        let expected = vec![
            (1_000, "dup".to_string(), false, false, false),
            (1_001, "dup".to_string(), false, true, false),
        ];
        rt.always(
            "duplicate_is_original_then_tagged_copy",
            armed && deliveries == expected,
            || format!("armed={armed} deliveries={deliveries:?}"),
        );
        rt.finish();
        RunOutcome::Completed
    }
}

#[test]
fn duplicate_delivers_one_original_and_one_tagged_copy() {
    let result = run_pair(
        "duplicate-edge",
        &DuplicateEdge,
        FaultPlan::new(vec![injection(0, FaultKind::NetworkDuplicate)]),
    );
    let outcome = only_injection(&result);
    assert_eq!(outcome.fault(), "network_duplicate");
    assert_eq!(outcome.injected_at(), Some(0));
    assert_eq!(outcome.manifested_at(), Some(1_001));
    assert_eq!(outcome.recovered_at(), Some(1_001));
}

struct ReorderEdge;

impl Workload for ReorderEdge {
    fn name(&self) -> &str {
        "vh-verify-reorder-edge"
    }

    fn property_contract(&self) -> PropertyContract {
        PropertyContract::new(&["reorder_swaps_exactly_one_pair"], &[])
    }

    fn run(&self, ctx: &mut UniverseCtx) -> RunOutcome {
        let mut rt = ctx.runtime(FaultPlan::default);
        let armed = rt.step().is_none();
        rt.send(1, 2, "first");
        rt.send(3, 4, "second");
        let mut deliveries = Vec::new();
        while let Some(event) = rt.step() {
            if let StepEvent::Delivered {
                from,
                to,
                payload,
                note,
            } = event
            {
                let DeliveryNote {
                    delayed,
                    duplicate,
                    reordered,
                } = note;
                deliveries.push((
                    rt.now_nanos(),
                    from,
                    to,
                    payload,
                    delayed,
                    duplicate,
                    reordered,
                ));
            }
        }
        let expected = vec![
            (1_000, 3, 4, "second".to_string(), false, false, false),
            (1_002, 1, 2, "first".to_string(), false, false, true),
        ];
        let drops = rt.drops();
        rt.always(
            "reorder_swaps_exactly_one_pair",
            armed && deliveries == expected && drops == 0,
            || format!("armed={armed} deliveries={deliveries:?} drops={drops}"),
        );
        rt.finish();
        RunOutcome::Completed
    }
}

#[test]
fn reorder_holds_one_message_and_releases_it_after_the_next() {
    let result = run_pair(
        "reorder-edge",
        &ReorderEdge,
        FaultPlan::new(vec![injection(0, FaultKind::NetworkReorder)]),
    );
    let outcome = only_injection(&result);
    assert_eq!(outcome.fault(), "network_reorder");
    assert_eq!(outcome.injected_at(), Some(0));
    assert_eq!(outcome.manifested_at(), Some(1_002));
    assert_eq!(outcome.recovered_at(), Some(1_002));
}

struct TornWriteEdge;

impl Workload for TornWriteEdge {
    fn name(&self) -> &str {
        "vh-verify-torn-write-edge"
    }

    fn property_contract(&self) -> PropertyContract {
        PropertyContract::new(&["torn_write_prefix_is_visible_and_recoverable"], &[])
    }

    fn run(&self, ctx: &mut UniverseCtx) -> RunOutcome {
        let mut rt = ctx.runtime(FaultPlan::default);
        let armed = rt.step().is_none();
        let torn_ok = rt.disk_write(0, "abcdef").is_ok();
        rt.set_timer(10, 1);
        let timer_10 = matches!(rt.step(), Some(StepEvent::Timer { token: 1 }));
        let visible = rt.disk_read_all(0);
        let flushed = rt.disk_flush(0).is_ok();
        let synced = rt.disk_fsync(0).is_ok();
        let durable_prefix = rt.disk_read_durable(0);

        rt.set_timer(20, 2);
        let timer_20 = matches!(rt.step(), Some(StepEvent::Timer { token: 2 }));
        let intact_ok = rt.disk_write(0, "abcdef").is_ok();
        let visible_after_recovery = rt.disk_read_all(0);

        rt.always(
            "torn_write_prefix_is_visible_and_recoverable",
            armed
                && torn_ok
                && timer_10
                && visible == vec!["abc".to_string()]
                && flushed
                && synced
                && durable_prefix == vec!["abc".to_string()]
                && timer_20
                && intact_ok
                && visible_after_recovery == vec!["abc".to_string(), "abcdef".to_string()],
            || {
                format!(
                    "armed={armed} torn_ok={torn_ok} timer10={timer_10} visible={visible:?} durable={durable_prefix:?} timer20={timer_20} intact_ok={intact_ok} after={visible_after_recovery:?}"
                )
            },
        );
        rt.finish();
        RunOutcome::Completed
    }
}

#[test]
fn torn_write_returns_success_but_exposes_only_a_prefix_until_rewritten() {
    let result = run_pair(
        "torn-write-edge",
        &TornWriteEdge,
        FaultPlan::new(vec![injection(0, FaultKind::TornWrite)]),
    );
    let outcome = only_injection(&result);
    assert_eq!(outcome.fault(), "torn_write");
    assert_eq!(outcome.injected_at(), Some(0));
    assert_eq!(outcome.manifested_at(), Some(10));
    assert_eq!(outcome.recovered_at(), Some(20));
}

struct FsyncLieLostAtCrash;

impl Workload for FsyncLieLostAtCrash {
    fn name(&self) -> &str {
        "vh-verify-fsync-lie-crash"
    }

    fn property_contract(&self) -> PropertyContract {
        PropertyContract::new(&["fsync_lie_manifests_only_when_crash_loses_cache"], &[])
    }

    fn run(&self, ctx: &mut UniverseCtx) -> RunOutcome {
        let mut rt = ctx.runtime(FaultPlan::default);
        rt.set_timer(1, 1);
        let armed = matches!(rt.step(), Some(StepEvent::Timer { token: 1 }));
        let write_ok = rt.disk_write(0, "claimed-durable").is_ok();
        let flush_ok = rt.disk_flush(0).is_ok();
        let lie_returned_ok = rt.disk_fsync(0).is_ok();
        let pre_crash = rt.disk_read_all(0);
        let crashed = matches!(rt.step(), Some(StepEvent::Crashed));
        let durable_after_crash = rt.disk_read_durable(0);

        rt.always(
            "fsync_lie_manifests_only_when_crash_loses_cache",
            armed
                && write_ok
                && flush_ok
                && lie_returned_ok
                && pre_crash == vec!["claimed-durable".to_string()]
                && crashed
                && durable_after_crash.is_empty(),
            || {
                format!(
                    "armed={armed} write={write_ok} flush={flush_ok} fsync={lie_returned_ok} pre={pre_crash:?} crashed={crashed} durable={durable_after_crash:?}"
                )
            },
        );
        rt.finish();
        RunOutcome::Completed
    }
}

#[test]
fn fsync_lie_manifests_at_the_crash_that_loses_claimed_data() {
    let result = run_pair(
        "fsync-lie-crash",
        &FsyncLieLostAtCrash,
        FaultPlan::new(vec![
            injection(0, FaultKind::FsyncLie),
            injection(100, FaultKind::CrashRestart),
        ]),
    );
    let evidence = result.runtime_evidence().expect("runtime evidence");
    assert_eq!(evidence.injections().len(), 2);
    let lie = &evidence.injections()[0];
    assert_eq!(lie.fault(), "fsync_lie");
    assert_eq!(lie.injected_at(), Some(1));
    assert_eq!(lie.manifested_at(), Some(100));
    assert_eq!(lie.recovered_at(), None);
    let crash = &evidence.injections()[1];
    assert_eq!(crash.fault(), "crash_restart");
    assert_eq!(crash.injected_at(), Some(100));
    assert_eq!(crash.manifested_at(), Some(100));
    assert_eq!(crash.recovered_at(), Some(100));
}

struct FsyncLieHealedByHonestFsync;

impl Workload for FsyncLieHealedByHonestFsync {
    fn name(&self) -> &str {
        "vh-verify-fsync-lie-healed"
    }

    fn property_contract(&self) -> PropertyContract {
        PropertyContract::new(&["honest_fsync_recovers_an_unmanifested_lie"], &[])
    }

    fn run(&self, ctx: &mut UniverseCtx) -> RunOutcome {
        let mut rt = ctx.runtime(FaultPlan::default);
        rt.set_timer(1, 1);
        let armed = matches!(rt.step(), Some(StepEvent::Timer { token: 1 }));
        let write_ok = rt.disk_write(0, "eventually-durable").is_ok();
        let flush_ok = rt.disk_flush(0).is_ok();
        let lie_returned_ok = rt.disk_fsync(0).is_ok();
        rt.set_timer(2, 2);
        let timer = matches!(rt.step(), Some(StepEvent::Timer { token: 2 }));
        let honest_ok = rt.disk_fsync(0).is_ok();
        let durable = rt.disk_read_durable(0);

        rt.always(
            "honest_fsync_recovers_an_unmanifested_lie",
            armed
                && write_ok
                && flush_ok
                && lie_returned_ok
                && timer
                && honest_ok
                && durable == vec!["eventually-durable".to_string()],
            || {
                format!(
                    "armed={armed} write={write_ok} flush={flush_ok} lie={lie_returned_ok} timer={timer} honest={honest_ok} durable={durable:?}"
                )
            },
        );
        rt.finish();
        RunOutcome::Completed
    }
}

#[test]
fn honest_fsync_recovers_a_lie_before_it_manifests() {
    let result = run_pair(
        "fsync-lie-healed",
        &FsyncLieHealedByHonestFsync,
        FaultPlan::new(vec![injection(0, FaultKind::FsyncLie)]),
    );
    let lie = only_injection(&result);
    assert_eq!(lie.injected_at(), Some(1));
    assert_eq!(lie.manifested_at(), None);
    assert_eq!(lie.recovered_at(), Some(2));
}

struct CrashEpochSemantics;

impl Workload for CrashEpochSemantics {
    fn name(&self) -> &str {
        "vh-verify-crash-epoch-semantics"
    }

    fn property_contract(&self) -> PropertyContract {
        PropertyContract::new(&["crash_preserves_durable_and_drops_cache_timers_and_delivery"], &[])
    }

    fn run(&self, ctx: &mut UniverseCtx) -> RunOutcome {
        let mut rt = ctx.runtime(FaultPlan::default);
        let durable_write = rt.disk_write(0, "durable").is_ok();
        let durable_flush = rt.disk_flush(0).is_ok();
        let durable_fsync = rt.disk_fsync(0).is_ok();
        let cached_write = rt.disk_write(0, "cache-only").is_ok();
        let cached_flush = rt.disk_flush(0).is_ok();
        rt.send(5, 5, "old-epoch");
        rt.set_timer(50, 1);
        rt.set_timer(200, 2);

        let first_timer = matches!(rt.step(), Some(StepEvent::Timer { token: 1 }));
        let crashed = matches!(rt.step(), Some(StepEvent::Crashed));
        let post_crash_view = rt.disk_read_all(0);
        let drained = rt.step().is_none();
        let drops = rt.drops();

        rt.always(
            "crash_preserves_durable_and_drops_cache_timers_and_delivery",
            durable_write
                && durable_flush
                && durable_fsync
                && cached_write
                && cached_flush
                && first_timer
                && crashed
                && post_crash_view == vec!["durable".to_string()]
                && drained
                && drops == 1,
            || {
                format!(
                    "durable={durable_write}/{durable_flush}/{durable_fsync} cache={cached_write}/{cached_flush} timer={first_timer} crashed={crashed} view={post_crash_view:?} drained={drained} drops={drops}"
                )
            },
        );
        rt.finish();
        RunOutcome::Completed
    }
}

#[test]
fn crash_epoch_clears_cache_and_cancels_old_timers_and_deliveries() {
    let result = run_pair(
        "crash-epoch",
        &CrashEpochSemantics,
        FaultPlan::new(vec![injection(100, FaultKind::CrashRestart)]),
    );
    let crash = only_injection(&result);
    assert_eq!(crash.offered_at(), Some(100));
    assert_eq!(crash.armed_at(), Some(100));
    assert_eq!(crash.injected_at(), Some(100));
    assert_eq!(crash.manifested_at(), Some(100));
    assert_eq!(crash.recovered_at(), Some(100));
}

struct RuntimeRecordsWithoutApplicationEvents {
    send: bool,
}

impl Workload for RuntimeRecordsWithoutApplicationEvents {
    fn name(&self) -> &str {
        if self.send {
            "vh-verify-runtime-records-send"
        } else {
            "vh-verify-runtime-records-idle"
        }
    }

    fn property_contract(&self) -> PropertyContract {
        PropertyContract::new(&["runtime_path_completed"], &[])
    }

    fn run(&self, ctx: &mut UniverseCtx) -> RunOutcome {
        let mut rt = ctx.runtime(FaultPlan::default);
        let completed = if self.send {
            rt.send(8, 8, "runtime-owned");
            matches!(rt.step(), Some(StepEvent::Delivered { .. }))
        } else {
            true
        };
        rt.always("runtime_path_completed", completed, || {
            "runtime path did not complete".to_string()
        });
        rt.finish();
        RunOutcome::Completed
    }
}

#[test]
fn runtime_effects_are_observable_without_workload_record_calls() {
    let idle = run_pair(
        "runtime-recording-idle",
        &RuntimeRecordsWithoutApplicationEvents { send: false },
        FaultPlan::default(),
    );
    let send = run_pair(
        "runtime-recording-send",
        &RuntimeRecordsWithoutApplicationEvents { send: true },
        FaultPlan::default(),
    );
    assert_ne!(idle.trace_hash(), send.trace_hash());
    assert!(send.trace_events() > idle.trace_events());
    assert_ne!(observable_fingerprint(&idle), observable_fingerprint(&send));
}
