//! Demo workloads: a toy write-ahead KV service in two variants.
//!
//! `demo` acknowledges writes only after flush (correct durability).
//! `demo-buggy` acknowledges at put time, before flush — the classic
//! vibe-coded durability bug. Under crash gremlins, some universes lose
//! acknowledged writes and the `durability` always-property fires with a
//! one-command repro.

use std::collections::BTreeMap;

use vh_gremlin::{FaultKind, FaultPlan};
use vh_multiverse::{UniverseCtx, Workload};

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

    fn run(&self, ctx: &mut UniverseCtx) {
        ctx.props.declare_sometimes("crash_injected");
        ctx.props.declare_sometimes("crash_with_dirty_wal");

        let mut ops = ctx.stream("ops");
        let mut gremlin = ctx.stream("gremlin");
        let plan = FaultPlan::generate(&mut gremlin, HORIZON_NANOS, FAULT_COUNT);
        let mut cursor = 0usize;

        // committed survives crashes; wal is volatile; acked is the client's
        // view of "the system told me this write is safe".
        let mut committed: BTreeMap<String, String> = BTreeMap::new();
        let mut wal: Vec<(String, String)> = Vec::new();
        let mut acked: BTreeMap<String, String> = BTreeMap::new();

        for i in 0..OPS {
            let now = i * OP_SPACING_NANOS;
            ctx.advance_to(now);

            let (next, due) = plan.due(cursor, now);
            cursor = next;
            let faults: Vec<FaultKind> = due.iter().map(|inj| inj.fault.clone()).collect();
            for fault in faults {
                ctx.record("fault", fault.label());
                if fault == FaultKind::CrashRestart {
                    ctx.props.sometimes("crash_injected");
                    if !wal.is_empty() {
                        ctx.props.sometimes("crash_with_dirty_wal");
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
                if self.ack_before_flush {
                    // buggy variant already acked at put time
                }
            }
        }

        for (key, value) in &acked {
            let stored = committed.get(key);
            ctx.props.always("durability", stored == Some(value), || {
                format!(
                    "acknowledged write {key}={value} missing after crash (committed={:?})",
                    stored
                )
            });
        }
        ctx.record(
            "final",
            &format!("committed={} acked={}", committed.len(), acked.len()),
        );
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

    fn run(&self, ctx: &mut UniverseCtx) {
        use std::sync::atomic::{AtomicU64, Ordering};
        static LEAK: AtomicU64 = AtomicU64::new(0);
        let leaked = LEAK.fetch_add(1, Ordering::SeqCst);
        ctx.record("leak", &format!("counter={leaked}"));
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
        _ => None,
    }
}
