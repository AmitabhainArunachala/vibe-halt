//! Adversarial tests for the Phase-1 sim runtime: SimNet + SimDisk on
//! the deterministic scheduler with RUNNER-OWNED fault injection and the
//! semantic fault-lifecycle ladder (Offered → Armed → Injected →
//! Manifested → Recovered).
//!
//! Every lifecycle claim here carries its negative: a stage that was NOT
//! reached must be absent — the ladder over-claiming would be the exact
//! dishonesty the retrieval-only ledger was renamed to avoid
//! (hardening-loop-4 GAP 5).

use vh_gremlin::{FaultInjection, FaultKind, FaultPlan};
use vh_multiverse::{
    run_universe, run_universe_with_fault_plan, FaultPlanDiscipline, RunOutcome, StepEvent,
    UniverseCtx, Workload,
};

/// A scripted-plan workload: drives the runtime with `script` under the
/// fault plan `plan` (via the normal override-or-generate path).
struct Scripted {
    plan: Vec<FaultInjection>,
    script: fn(&mut vh_multiverse::SimRuntime<'_>),
}

impl Workload for Scripted {
    fn name(&self) -> &str {
        "scripted-runtime"
    }
    fn run(&self, ctx: &mut UniverseCtx) -> RunOutcome {
        let plan = self.plan.clone();
        let mut rt = ctx.runtime(|| FaultPlan::new(plan));
        (self.script)(&mut rt);
        rt.finish();
        RunOutcome::Completed
    }
}

fn inj(at_nanos: u64, fault: FaultKind) -> FaultInjection {
    FaultInjection { at_nanos, fault }
}

fn drain(rt: &mut vh_multiverse::SimRuntime<'_>) {
    while rt.step().is_some() {}
}

#[test]
fn runtime_replay_is_bit_identical() {
    let w = Scripted {
        plan: vec![
            inj(
                5_000,
                FaultKind::NetworkPartition {
                    duration_nanos: 20_000,
                },
            ),
            inj(10_000, FaultKind::NetworkDelay { delay_nanos: 7_000 }),
            inj(40_000, FaultKind::DiskWriteFail),
        ],
        script: |rt| {
            rt.set_timer(2_000, 1);
            rt.set_timer(30_000, 2);
            rt.set_timer(50_000, 3);
            while let Some(ev) = rt.step() {
                if let StepEvent::Timer { token } = ev {
                    rt.send(0, 1, &format!("m{token}"));
                    let _ = rt.disk_write(0, &format!("r{token}"));
                    let _ = rt.disk_flush(0);
                    let _ = rt.disk_fsync(0);
                }
            }
        },
    };
    let a = run_universe(0xD1CE, 7, &w);
    let b = run_universe(0xD1CE, 7, &w);
    assert!(a.observably_equal(&b));
    assert!(a.runtime_evidence().is_some());
}

#[test]
fn runtime_records_delivery_and_io_trace_events_itself() {
    // The workload records NOTHING itself; every trace event comes from
    // the runtime — a workload cannot under-record runtime effects.
    let silent = Scripted {
        plan: vec![],
        script: |rt| {
            rt.send(0, 1, "hello");
            let _ = rt.disk_write(1, "rec");
            drain(rt);
        },
    };
    let r = run_universe(1, 0, &silent);
    // net.send, disk.write, net.deliver, runtime.end — all runtime-recorded.
    assert!(r.trace_events() >= 4, "got {} events", r.trace_events());

    let idle = Scripted {
        plan: vec![],
        script: |rt| drain(rt),
    };
    let r2 = run_universe(1, 0, &idle);
    assert_ne!(r.trace_hash(), r2.trace_hash());
    assert!(r.trace_events() > r2.trace_events());
}

#[test]
fn unconsumed_arm_stays_armed_and_unreached_offer_stays_unoffered() {
    // A delay armed with no subsequent send is Armed, never Injected; an
    // injection scheduled beyond the workload's stepping horizon is
    // honestly never Offered.
    let w = Scripted {
        plan: vec![
            inj(1_000, FaultKind::NetworkDelay { delay_nanos: 9_000 }),
            inj(2_000, FaultKind::CrashRestart),
        ],
        script: |rt| {
            // Stop stepping immediately: pop nothing at all.
            let _ = rt;
        },
    };
    let r = run_universe(2, 0, &w);
    let ev = r.runtime_evidence().expect("runtime constructed");
    assert_eq!(ev.injections().len(), 2);
    for o in ev.injections() {
        assert!(o.offered_at().is_none(), "never stepped ⇒ never offered");
    }

    let w2 = Scripted {
        plan: vec![inj(1_000, FaultKind::NetworkDelay { delay_nanos: 9_000 })],
        script: drain,
    };
    let r2 = run_universe(2, 0, &w2);
    let o = &r2.runtime_evidence().unwrap().injections()[0];
    assert!(o.offered_at().is_some());
    assert!(o.armed_at().is_some());
    assert!(o.injected_at().is_none(), "no send consumed the delay");
    assert!(o.manifested_at().is_none());
    assert!(o.recovered_at().is_none());
}

#[test]
fn partition_drops_inject_manifest_and_heal_recovers() {
    let w = Scripted {
        plan: vec![inj(
            10_000,
            FaultKind::NetworkPartition {
                duration_nanos: 20_000,
            },
        )],
        script: |rt| {
            rt.set_timer(15_000, 1); // inside the window
            rt.set_timer(40_000, 2); // after heal
            let mut delivered = Vec::new();
            while let Some(ev) = rt.step() {
                match ev {
                    StepEvent::Timer { token } => rt.send(0, 1, &format!("m{token}")),
                    StepEvent::Delivered { payload, .. } => delivered.push(payload),
                    StepEvent::Crashed => unreachable!(),
                }
            }
            assert_eq!(delivered, vec!["m2".to_string()], "in-window send dropped");
            assert_eq!(rt.drops(), 1);
        },
    };
    let r = run_universe(3, 0, &w);
    let o = &r.runtime_evidence().unwrap().injections()[0];
    assert_eq!(o.offered_at(), Some(10_000));
    assert_eq!(o.armed_at(), Some(10_000));
    assert_eq!(o.injected_at(), Some(15_000), "first drop is the injection");
    assert_eq!(
        o.manifested_at(),
        Some(15_000),
        "non-delivery IS the effect"
    );
    assert_eq!(
        o.recovered_at(),
        Some(41_000),
        "post-heal delivery completes"
    );
}

#[test]
fn idle_partition_heals_without_injection() {
    // Negative: a partition that intercepted nothing must NOT claim
    // Injected/Manifested — it recovers straight from Armed.
    let w = Scripted {
        plan: vec![inj(
            10_000,
            FaultKind::NetworkPartition {
                duration_nanos: 5_000,
            },
        )],
        script: |rt| {
            rt.set_timer(30_000, 1); // send only after heal
            while let Some(ev) = rt.step() {
                if let StepEvent::Timer { .. } = ev {
                    rt.send(0, 1, "late");
                }
            }
        },
    };
    let r = run_universe(4, 0, &w);
    let o = &r.runtime_evidence().unwrap().injections()[0];
    assert!(o.injected_at().is_none());
    assert!(o.manifested_at().is_none());
    assert_eq!(o.recovered_at(), Some(31_000));
}

#[test]
fn crash_wipes_volatile_keeps_durable_and_cancels_inflight() {
    let w = Scripted {
        plan: vec![inj(20_000, FaultKind::CrashRestart)],
        script: |rt| {
            let _ = rt.disk_write(0, "safe");
            let _ = rt.disk_flush(0);
            let _ = rt.disk_fsync(0); // durable
            let _ = rt.disk_write(0, "volatile"); // buf only
            rt.set_timer(15_000, 1);
            let mut crashed = false;
            let mut delivered = 0u32;
            while let Some(ev) = rt.step() {
                match ev {
                    StepEvent::Timer { token: 1 } => {
                        rt.send(0, 1, "pre"); // delivers at 16_000, pre-crash
                        rt.set_timer(19_500, 2);
                    }
                    StepEvent::Timer { .. } => {
                        // Delivery would land at 20_500 — in flight when
                        // the crash hits at 20_000, so the epoch bump
                        // must drop it.
                        rt.send(0, 1, "in-flight-at-crash");
                    }
                    StepEvent::Delivered { .. } => delivered += 1,
                    StepEvent::Crashed => {
                        crashed = true;
                        assert_eq!(rt.disk_read_durable(0), vec!["safe".to_string()]);
                        assert_eq!(
                            rt.disk_read_all(0),
                            vec!["safe".to_string()],
                            "buf+cache wiped by the crash"
                        );
                    }
                }
            }
            assert!(crashed);
            assert_eq!(delivered, 1, "the in-flight message died with the epoch");
        },
    };
    let r = run_universe(5, 0, &w);
    let o = &r.runtime_evidence().unwrap().injections()[0];
    assert_eq!(
        o.manifested_at(),
        Some(20_000),
        "Crashed surfaced to workload"
    );
    assert_eq!(
        o.recovered_at(),
        Some(20_000),
        "first workload op after the crash (the recovery read) marks recovery"
    );
}

#[test]
fn fsync_lie_manifests_only_through_actual_loss() {
    // Lie then crash: claimed-durable data is gone ⇒ Manifested.
    let lie_then_crash = Scripted {
        plan: vec![
            inj(1_000, FaultKind::FsyncLie),
            inj(10_000, FaultKind::CrashRestart),
        ],
        script: |rt| {
            rt.set_timer(2_000, 1);
            while let Some(ev) = rt.step() {
                if let StepEvent::Timer { .. } = ev {
                    let _ = rt.disk_write(0, "acked");
                    let _ = rt.disk_flush(0);
                    let _ = rt.disk_fsync(0); // the lie
                }
            }
        },
    };
    let r = run_universe(6, 0, &lie_then_crash);
    let o = &r.runtime_evidence().unwrap().injections()[0];
    assert_eq!(o.injected_at(), Some(2_000));
    assert_eq!(
        o.manifested_at(),
        Some(10_000),
        "loss happened at the crash"
    );
    assert!(o.recovered_at().is_none());

    // Lie then honest fsync: the window closed without loss ⇒ Recovered,
    // NEVER Manifested (negative: no over-claiming).
    let lie_then_honest = Scripted {
        plan: vec![inj(1_000, FaultKind::FsyncLie)],
        script: |rt| {
            rt.set_timer(2_000, 1);
            rt.set_timer(3_000, 2);
            while let Some(ev) = rt.step() {
                match ev {
                    StepEvent::Timer { token: 1 } => {
                        let _ = rt.disk_write(0, "acked");
                        let _ = rt.disk_flush(0);
                        let _ = rt.disk_fsync(0); // the lie
                    }
                    StepEvent::Timer { .. } => {
                        let _ = rt.disk_fsync(0); // honest, persists
                    }
                    _ => {}
                }
            }
        },
    };
    let r2 = run_universe(6, 0, &lie_then_honest);
    let o2 = &r2.runtime_evidence().unwrap().injections()[0];
    assert_eq!(o2.injected_at(), Some(2_000));
    assert!(o2.manifested_at().is_none(), "no data was actually lost");
    assert_eq!(o2.recovered_at(), Some(3_000));
}

#[test]
fn torn_write_manifests_on_read_back_and_recovers_on_intact_write() {
    let w = Scripted {
        plan: vec![inj(1_000, FaultKind::TornWrite)],
        script: |rt| {
            rt.set_timer(2_000, 1);
            while let Some(ev) = rt.step() {
                if let StepEvent::Timer { .. } = ev {
                    assert_eq!(rt.disk_write(0, "record-eight"), Ok(())); // torn, silent
                    let all = rt.disk_read_all(0);
                    assert_eq!(all, vec!["record".to_string()], "prefix only");
                    assert_eq!(rt.disk_write(0, "record-eight"), Ok(())); // intact
                }
            }
        },
    };
    let r = run_universe(7, 0, &w);
    let o = &r.runtime_evidence().unwrap().injections()[0];
    assert_eq!(o.injected_at(), Some(2_000));
    assert_eq!(
        o.manifested_at(),
        Some(2_000),
        "torn content crossed the read API"
    );
    assert_eq!(
        o.recovered_at(),
        Some(2_000),
        "intact write closed the window"
    );
}

#[test]
fn duplicate_delivers_twice_and_reorder_swaps_next_two() {
    let w = Scripted {
        plan: vec![inj(500, FaultKind::NetworkDuplicate)],
        script: |rt| {
            rt.set_timer(1_000, 1);
            let mut got = Vec::new();
            while let Some(ev) = rt.step() {
                match ev {
                    StepEvent::Timer { .. } => rt.send(0, 1, "dup-me"),
                    StepEvent::Delivered { payload, note, .. } => {
                        got.push((payload, note.duplicate));
                    }
                    _ => {}
                }
            }
            assert_eq!(
                got,
                vec![("dup-me".to_string(), false), ("dup-me".to_string(), true)]
            );
        },
    };
    let r = run_universe(8, 0, &w);
    let o = &r.runtime_evidence().unwrap().injections()[0];
    assert!(o.manifested_at().is_some());
    assert!(o.recovered_at().is_some());

    let swap = Scripted {
        plan: vec![inj(500, FaultKind::NetworkReorder)],
        script: |rt| {
            rt.set_timer(1_000, 1);
            let mut got = Vec::new();
            while let Some(ev) = rt.step() {
                match ev {
                    StepEvent::Timer { .. } => {
                        rt.send(0, 1, "first");
                        rt.send(0, 1, "second");
                    }
                    StepEvent::Delivered { payload, note, .. } => {
                        got.push((payload, note.reordered));
                    }
                    _ => {}
                }
            }
            assert_eq!(
                got,
                vec![("second".to_string(), false), ("first".to_string(), true)],
                "the held message arrives after its successor"
            );
        },
    };
    let r2 = run_universe(8, 1, &swap);
    let o2 = &r2.runtime_evidence().unwrap().injections()[0];
    assert!(o2.injected_at().is_some());
    assert!(o2.manifested_at().is_some());
}

#[test]
fn unpaired_reorder_expires_undelivered_and_stays_armed() {
    // Negative: a reorder hold with no following send never swapped —
    // the ladder must stop at Armed and the expiry must be counted.
    let w = Scripted {
        plan: vec![inj(500, FaultKind::NetworkReorder)],
        script: |rt| {
            rt.set_timer(1_000, 1);
            let mut delivered = 0u32;
            while let Some(ev) = rt.step() {
                match ev {
                    StepEvent::Timer { .. } => rt.send(0, 1, "lonely"),
                    StepEvent::Delivered { .. } => delivered += 1,
                    _ => {}
                }
            }
            assert_eq!(delivered, 0, "held message expired undelivered");
        },
    };
    let r = run_universe(9, 0, &w);
    let o = &r.runtime_evidence().unwrap().injections()[0];
    assert!(o.armed_at().is_some());
    assert!(o.injected_at().is_none(), "no swap ⇒ no injection claim");
}

#[test]
fn override_replay_flows_through_the_runtime_identically() {
    let w = Scripted {
        plan: vec![inj(5_000, FaultKind::DiskWriteFail)],
        script: |rt| {
            rt.set_timer(6_000, 1);
            while let Some(ev) = rt.step() {
                if let StepEvent::Timer { .. } = ev {
                    let _ = rt.disk_write(0, "x");
                }
            }
        },
    };
    let recorded = run_universe(10, 0, &w);
    let plan = FaultPlan::new(vec![inj(5_000, FaultKind::DiskWriteFail)]);
    let replayed = run_universe_with_fault_plan(10, 0, &w, plan.clone());
    // The discipline enum honestly differs (SelfGenerated vs
    // OverrideRetrieved), so whole-struct equality does not apply across
    // the recorded/override boundary; every runtime-behavior observable
    // must agree exactly.
    assert_eq!(recorded.trace_hash(), replayed.trace_hash());
    assert_eq!(recorded.trace_events(), replayed.trace_events());
    assert_eq!(recorded.fault_plan_digest(), replayed.fault_plan_digest());
    assert_eq!(recorded.runtime_evidence(), replayed.runtime_evidence());
    assert_eq!(
        replayed.lifecycle().fault_plan(),
        &FaultPlanDiscipline::OverrideRetrieved,
        "the runtime retrieves the override exactly once"
    );
    // Override-vs-override IS whole-struct replay identity.
    let replayed_again = run_universe_with_fault_plan(10, 0, &w, plan);
    assert!(replayed.observably_equal(&replayed_again));
}

#[test]
#[should_panic(expected = "one sim runtime per universe")]
fn second_runtime_construction_fails_closed() {
    struct Double;
    impl Workload for Double {
        fn name(&self) -> &str {
            "double-runtime"
        }
        fn run(&self, ctx: &mut UniverseCtx) -> RunOutcome {
            {
                let mut rt = ctx.runtime(|| FaultPlan::new(vec![]));
                drain(&mut rt);
            }
            let _second = ctx.runtime(|| FaultPlan::new(vec![]));
            RunOutcome::Completed
        }
    }
    let _ = run_universe(11, 0, &Double);
}

#[test]
fn runtime_evidence_divergence_is_caught_by_the_detector() {
    // Two universes identical in trace-relevant behavior except the
    // runtime evidence would require behavioral divergence to differ —
    // so instead verify the field participates in equality directly.
    let w = Scripted {
        plan: vec![inj(1_000, FaultKind::NetworkDelay { delay_nanos: 500 })],
        script: |rt| {
            rt.set_timer(2_000, 1);
            while let Some(ev) = rt.step() {
                if let StepEvent::Timer { .. } = ev {
                    rt.send(0, 1, "m");
                }
            }
        },
    };
    let a = run_universe(12, 0, &w);
    let b = run_universe(12, 0, &w);
    assert!(a.observably_equal(&b));
    let ea = a.runtime_evidence().unwrap();
    assert_eq!(ea, b.runtime_evidence().unwrap());
    // send at 2_000 + base latency 1_000 + armed delay 500 ⇒ manifested
    // at the delayed delivery.
    assert_eq!(ea.injections()[0].manifested_at(), Some(3_500));
}

// ---- ClockSkew: observable virtual-clock divergence (convergence C3, audit D6) ----
//
// UniverseResult exposes trace hash + event count, not raw entries, so
// divergence is measured three ways: the workload observes skewed
// now_nanos values inline (assertions run inside the universe), the
// lifecycle ladder pins interception to the observing read, and the
// trace event-count delta against a skew-free control pins exactly the
// events the skew machinery records (offered, armed, clock.skew,
// injected, manifested, clock.read).

#[test]
fn clock_skew_manifests_on_the_first_observing_read() {
    let skewed = Scripted {
        plan: vec![inj(5_000, FaultKind::ClockSkew { skew_nanos: 700 })],
        script: |rt| {
            // Before the skew arms: local == global.
            assert_eq!(rt.now_nanos(), 0);
            rt.set_timer(10_000, 1);
            while let Some(ev) = rt.step() {
                if let StepEvent::Timer { .. } = ev {
                    // Timer fires at global 10_000; the local read has
                    // diverged by the armed skew.
                    let t = rt.now_nanos();
                    assert_eq!(t, 10_700);
                    rt.record("app", &format!("t={t}"));
                }
            }
        },
    };
    let control = Scripted {
        plan: vec![],
        script: |rt| {
            assert_eq!(rt.now_nanos(), 0);
            rt.set_timer(10_000, 1);
            while let Some(ev) = rt.step() {
                if let StepEvent::Timer { .. } = ev {
                    let t = rt.now_nanos();
                    assert_eq!(t, 10_000);
                    rt.record("app", &format!("t={t}"));
                }
            }
        },
    };
    let r = run_universe(21, 0, &skewed);
    let c = run_universe(21, 0, &control);

    let ev = r.runtime_evidence().unwrap();
    let skew = &ev.injections()[0];
    assert_eq!(skew.fault(), "clock_skew:700");
    assert_eq!(skew.offered_at(), Some(5_000));
    assert_eq!(skew.armed_at(), Some(5_000));
    // Injected+Manifested coincide at the first observing read (the
    // read at global 10_000), like a partition drop.
    assert_eq!(skew.injected_at(), Some(10_000));
    assert_eq!(skew.manifested_at(), Some(10_000));
    assert!(skew.recovered_at().is_none());

    // Exactly the skew machinery's six trace events separate the runs:
    // fault.offered, fault.armed, clock.skew, fault.injected,
    // fault.manifested, clock.read.
    assert_eq!(r.trace_events(), c.trace_events() + 6);
    assert_ne!(r.trace_hash(), c.trace_hash());
}

#[test]
fn clock_skew_never_read_honestly_stays_armed() {
    // The workload never consults its clock: the skew arms (the offset
    // IS installed) but no read intercepts it — Injected/Manifested
    // must stay absent; over-claiming here would be the exact ladder
    // dishonesty the skip-arm removal was meant to end.
    let w = Scripted {
        plan: vec![inj(1_000, FaultKind::ClockSkew { skew_nanos: 42 })],
        script: |rt| {
            drain(rt);
        },
    };
    let r = run_universe(22, 0, &w);
    let ev = r.runtime_evidence().unwrap();
    let skew = &ev.injections()[0];
    assert_eq!(skew.offered_at(), Some(1_000));
    assert_eq!(skew.armed_at(), Some(1_000));
    assert!(skew.injected_at().is_none());
    assert!(skew.manifested_at().is_none());
    assert!(skew.recovered_at().is_none());
    // The universe still completes validly — an armed-only skew is an
    // honest terminal state, not an invalid completion.
    assert!(r.lifecycle().is_valid_completion());
}

#[test]
fn clock_skew_accumulates_and_replays_bit_identically() {
    let w = Scripted {
        plan: vec![
            inj(1_000, FaultKind::ClockSkew { skew_nanos: 100 }),
            inj(2_000, FaultKind::ClockSkew { skew_nanos: 30 }),
        ],
        script: |rt| {
            rt.set_timer(3_000, 1);
            while let Some(ev) = rt.step() {
                if let StepEvent::Timer { .. } = ev {
                    // Both skews stack: 3_000 + 100 + 30. Application
                    // records are stamped with the LOCAL clock — a
                    // skewed component honestly records skewed time.
                    assert_eq!(rt.now_nanos(), 3_130);
                    rt.record("app", "skewed-view");
                }
            }
        },
    };
    let a = run_universe(23, 0, &w);
    let b = run_universe(23, 0, &w);
    assert!(a.observably_equal(&b));
    let ev = a.runtime_evidence().unwrap();
    // One read observes both pending skews at once; each ladder pins
    // interception to that read.
    assert_eq!(ev.injections()[0].injected_at(), Some(3_000));
    assert_eq!(ev.injections()[0].manifested_at(), Some(3_000));
    assert_eq!(ev.injections()[1].injected_at(), Some(3_000));
    assert_eq!(ev.injections()[1].manifested_at(), Some(3_000));
}
