//! Demo workloads: a toy write-ahead KV service in two variants.
//!
//! `demo` acknowledges writes only after flush (correct durability).
//! `demo-buggy` acknowledges at put time, before flush — the classic
//! vibe-coded durability bug. Under crash gremlins, some universes lose
//! acknowledged writes and the `durability` always-property fires with a
//! one-command repro.

mod corpus;
mod disk;
mod net;

use std::collections::BTreeMap;

pub use corpus::{CrashToctou, DirtyRead, LostUpdate, RetryDoubleApply};
pub use disk::WalDemo;
pub use net::EchoDemo;
use vh_gremlin::{FaultKind, FaultPlan};
use vh_multiverse::{
    EndState, EndStateOracle, PropertyContract, RunOutcome, UniverseCtx, Workload,
};

const OPS: u64 = 40;
const OP_SPACING_NANOS: u64 = 25_000;
const HORIZON_NANOS: u64 = OPS * OP_SPACING_NANOS;
const FAULT_COUNT: usize = 3;

pub struct KvDemo {
    /// true = acknowledge before flush (the seeded bug).
    pub ack_before_flush: bool,
}

impl Workload for KvDemo {
    fn name(&self) -> &str {
        if self.ack_before_flush {
            "demo-buggy"
        } else {
            "demo"
        }
    }

    /// The runner verifies this contract per universe (hardening-loop-4
    /// GAP 5): every universe must be judged by the `durability`
    /// end-state oracle and declare both crash sometimes-properties.
    fn property_contract(&self) -> PropertyContract {
        PropertyContract::new(&[], &["crash_injected", "crash_with_dirty_wal"])
            .with_oracles(&["durability"])
    }

    /// Durability re-expressed as an end-state oracle (Phase-2 pulled
    /// early, 2026-07-21): the run declares `acked:*` / `committed:*`
    /// facts; the runner judges them post-run. Oracles read state and
    /// record no trace events, so the frozen demo TRACE identity
    /// (9ce6199f133f4d3c9dd0da0075e352d2, 45 events) is untouched by
    /// this re-expression — pinned by doctor and by
    /// `demo_trace_identity_survives_the_oracle_reexpression` in
    /// tests/demo.rs.
    fn end_state_oracles(&self) -> Vec<EndStateOracle> {
        vec![EndStateOracle {
            name: "durability",
            check: durability_oracle,
        }]
    }

    fn run(&self, ctx: &mut UniverseCtx) -> RunOutcome {
        ctx.declare_sometimes("crash_injected");
        ctx.declare_sometimes("crash_with_dirty_wal");

        let mut ops = ctx.stream("ops");
        let mut gremlin = ctx.stream("gremlin");
        let plan =
            ctx.fault_plan_or(|| FaultPlan::generate(&mut gremlin, HORIZON_NANOS, FAULT_COUNT));
        let mut cursor = 0usize;

        // committed survives crashes; wal is volatile; acked is the client's
        // view of "the system told me this write is safe".
        let mut committed: BTreeMap<String, String> = BTreeMap::new();
        let mut wal: Vec<(String, String)> = Vec::new();
        let mut acked: BTreeMap<String, String> = BTreeMap::new();

        let mut lost_acked_to_crash = false;
        for i in 0..OPS {
            let now = i * OP_SPACING_NANOS;
            ctx.advance_to(now);

            let (next, due) = plan.due(cursor, now);
            cursor = next;
            let faults: Vec<FaultKind> = due.iter().map(|inj| inj.fault.clone()).collect();
            for fault in faults {
                ctx.record("fault", fault.label());
                if fault == FaultKind::CrashRestart {
                    ctx.sometimes("crash_injected");
                    if !wal.is_empty() {
                        ctx.sometimes("crash_with_dirty_wal");
                        // In the buggy variant every dirty-wal entry was
                        // already acknowledged, so this crash loses acked
                        // writes.
                        if self.ack_before_flush {
                            lost_acked_to_crash = true;
                        }
                    }
                    wal.clear();
                    ctx.record("crash", "wal lost, committed state restored");
                }
            }

            if ops.next_bool(0.7) {
                // put: unique key per op, so durability checking is exact.
                let key = format!("k{i}");
                let value = format!("v{}", ops.next_below(1000));
                wal.push((key.clone(), value.clone()));
                ctx.record("put", &format!("{key}={value}"));
                if self.ack_before_flush {
                    acked.insert(key, value);
                }
            } else {
                for (k, v) in wal.drain(..) {
                    committed.insert(k.clone(), v.clone());
                    if !self.ack_before_flush {
                        acked.insert(k, v);
                    }
                }
                ctx.record("flush", "");
            }
        }

        // Clean shutdown: drain the final WAL before the oracle so that
        // end-of-run unflushed writes cannot fail durability on their own —
        // a failure now requires a crash to have dropped acknowledged
        // writes (PR #1 review GAP: previously the buggy variant failed
        // even in crash-free universes).
        for (k, v) in wal.drain(..) {
            committed.insert(k.clone(), v.clone());
            if !self.ack_before_flush {
                acked.insert(k, v);
            }
        }
        ctx.record("flush", "final");

        for (key, value) in &acked {
            ctx.declare_end(&format!("acked:{key}"), value);
        }
        for (key, value) in &committed {
            ctx.declare_end(&format!("committed:{key}"), value);
        }
        debug_assert!(
            lost_acked_to_crash || acked.iter().all(|(k, v)| committed.get(k) == Some(v)),
            "durability can only fail via a crash after the final flush"
        );
        ctx.record(
            "final",
            &format!("committed={} acked={}", committed.len(), acked.len()),
        );
        RunOutcome::Completed
    }
}

/// The demo's durability law over declared end state: every acknowledged
/// write must be committed with the acknowledged value. The detail names
/// every violated key (BTreeMap order — deterministic), preserving the
/// per-key evidence granularity of the inline checks it replaces.
fn durability_oracle(end: &EndState) -> Result<(), String> {
    let mut violations = Vec::new();
    for (key, value) in end {
        if let Some(k) = key.strip_prefix("acked:") {
            let stored = end.get(&format!("committed:{k}"));
            if stored != Some(value) {
                violations.push(format!(
                    "acknowledged write {k}={value} missing after crash (committed={stored:?})"
                ));
            }
        }
    }
    if violations.is_empty() {
        Ok(())
    } else {
        Err(violations.join("; "))
    }
}

/// Leaks process-global state into the trace so consecutive runs differ.
/// Exists so operators can watch the divergence detector catch a leak:
/// `vh run --workload demo-nondet` must exit nonzero.
pub struct NondetDemo;

impl Workload for NondetDemo {
    fn name(&self) -> &str {
        "demo-nondet"
    }

    fn run(&self, ctx: &mut UniverseCtx) -> RunOutcome {
        use std::sync::atomic::{AtomicU64, Ordering};
        static LEAK: AtomicU64 = AtomicU64::new(0);
        let leaked = LEAK.fetch_add(1, Ordering::SeqCst);
        ctx.record("leak", &format!("counter={leaked}"));
        RunOutcome::Completed
    }
}

pub fn by_name(name: &str) -> Option<Box<dyn Workload>> {
    match name {
        "demo" => Some(Box::new(KvDemo {
            ack_before_flush: false,
        })),
        "demo-buggy" => Some(Box::new(KvDemo {
            ack_before_flush: true,
        })),
        "demo-nondet" => Some(Box::new(NondetDemo)),
        "demo-net" => Some(Box::new(EchoDemo { no_retry: false })),
        "demo-net-buggy" => Some(Box::new(EchoDemo { no_retry: true })),
        "demo-disk" => Some(Box::new(WalDemo {
            ack_at_flush: false,
            lie_palette: false,
        })),
        "demo-disk-buggy" => Some(Box::new(WalDemo {
            ack_at_flush: true,
            lie_palette: false,
        })),
        "corpus-lost-update" => Some(Box::new(LostUpdate)),
        "corpus-retry-double-apply" => Some(Box::new(RetryDoubleApply)),
        "corpus-dirty-read" => Some(Box::new(DirtyRead)),
        "corpus-crash-toctou" => Some(Box::new(CrashToctou)),
        "corpus-fsync-lie" => Some(Box::new(WalDemo {
            ack_at_flush: false,
            lie_palette: true,
        })),
        _ => None,
    }
}
