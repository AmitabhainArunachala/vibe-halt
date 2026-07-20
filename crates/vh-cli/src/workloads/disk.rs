//! demo-disk: a write-ahead log on the Phase-1 SimDisk. The RUNTIME owns
//! fault injection (write failures, torn writes, crash/restart); the
//! workload only writes, flushes, fsyncs, verifies, and recovers.
//!
//! `demo-disk` (correct, paranoid WAL): a record is acknowledged only
//! after write → flush → fsync → READ-BACK VERIFY succeeds — the
//! verify-after-fsync closes the silent-torn-write window, and crash
//! recovery rebuilds from the durable layer, rewriting unacknowledged
//! records. Acked ⇒ durable-and-intact, so the campaign is CLEAN.
//!
//! `demo-disk-buggy` (seeded bug): acknowledges at FLUSH time — before
//! fsync, without verify. A crash between flush-ack and the next fsync
//! erases the OS cache and the acknowledged records with it; a torn
//! write gets acknowledged unverified. Both violate the
//! `wal_durability` oracle: the classic flushed-is-not-fsynced fallacy.

use std::collections::BTreeMap;

use vh_gremlin::{FaultInjection, FaultKind, FaultPlan};
use vh_multiverse::{
    EndState, EndStateOracle, PropertyContract, RunOutcome, SimRuntime, StepEvent, UniverseCtx,
    Workload,
};

const OPS: u64 = 24;
const OP_SPACING_NANOS: u64 = 20_000;
const HORIZON_NANOS: u64 = OPS * OP_SPACING_NANOS;
const NODE: u32 = 0;
const WRITE_RETRIES: u64 = 3;
const VERIFY_ROUNDS: u64 = 2;

pub struct WalDemo {
    /// true = acknowledge at flush, before fsync/verify (the seeded bug).
    pub ack_at_flush: bool,
}

impl WalDemo {
    fn plan(rng: &mut vh_core::Xoshiro256pp) -> FaultPlan {
        let count = 2 + rng.next_below(3); // 2..=4
        let injections = (0..count)
            .map(|_| {
                let at_nanos = rng.next_below(HORIZON_NANOS);
                let fault = match rng.next_below(3) {
                    0 => FaultKind::DiskWriteFail,
                    1 => FaultKind::TornWrite,
                    _ => FaultKind::CrashRestart,
                };
                FaultInjection { at_nanos, fault }
            })
            .collect();
        FaultPlan::new(injections)
    }
}

fn wal_durability_oracle(end: &EndState) -> Result<(), String> {
    let mut violations = Vec::new();
    for (key, value) in end {
        if let Some(record) = key.strip_prefix("acked:") {
            let stored = end.get(&format!("durable:{record}"));
            if stored != Some(value) {
                violations.push(format!(
                    "acknowledged record {record}={value} not intact in final durable state (found {stored:?})"
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

/// Parse a WAL entry "r<i>=<v>" into (record id, value). Torn prefixes
/// either fail to parse or parse to a truncated value — recovery keeps
/// whatever parses; the ORACLE is what catches a truncated value that
/// was acknowledged at full length.
fn parse_entry(entry: &str) -> Option<(String, String)> {
    let (k, v) = entry.split_once('=')?;
    if k.starts_with('r') && !v.is_empty() {
        Some((k.to_string(), v.to_string()))
    } else {
        None
    }
}

struct WalClient {
    ack_at_flush: bool,
    /// Coverage sometimes-marks are declared only by the correct
    /// variant; the buggy variant must not hit undeclared names
    /// (fail-closed declaration discipline).
    mark_coverage: bool,
    /// Written this cycle, not yet acknowledged: record -> value.
    pending: BTreeMap<String, String>,
    /// Acknowledged to the client: record -> value.
    acked: BTreeMap<String, String>,
}

impl WalClient {
    fn write_with_retry(&mut self, rt: &mut SimRuntime<'_>, record: &str, value: &str) {
        let entry = format!("{record}={value}");
        for attempt in 0..WRITE_RETRIES {
            match rt.disk_write(NODE, &entry) {
                Ok(()) => {
                    self.pending.insert(record.to_string(), value.to_string());
                    return;
                }
                Err(_) => {
                    if self.mark_coverage && attempt + 1 < WRITE_RETRIES {
                        rt.sometimes("write_failed_and_retried");
                    }
                }
            }
        }
        rt.record("app.give_up", &format!("write {entry}"));
    }

    /// Commit cycle: flush (+ack there if buggy), fsync, then read-back
    /// verify and ack (correct variant), rewriting torn/lost records.
    ///
    /// The buggy variant fsyncs ONLY at final shutdown (`final_commit`):
    /// mid-run it trusts the flushed OS cache, so every stretch between
    /// a flush-ack and the end of the run is a crash window in which
    /// acknowledged records are volatile — losses therefore require a
    /// crash, exactly like the classic bug (a crash-free buggy run
    /// persists everything at shutdown and passes; the PR #1 review GAP
    /// doctrine: the oracle must not fire on crash-free runs).
    fn commit(&mut self, rt: &mut SimRuntime<'_>, final_commit: bool) {
        let _ = rt.disk_flush(NODE);
        if self.ack_at_flush {
            // The bug: flushed == safe, in the client's imagination.
            let pending = std::mem::take(&mut self.pending);
            self.acked.extend(pending);
            if final_commit {
                let _ = rt.disk_fsync(NODE);
            }
            return;
        }
        let _ = rt.disk_fsync(NODE);
        for _round in 0..VERIFY_ROUNDS {
            let visible = rt.disk_read_all(NODE);
            let missing: BTreeMap<String, String> = self
                .pending
                .iter()
                .filter(|(k, v)| !visible.contains(&format!("{k}={v}")))
                .map(|(k, v)| (k.clone(), v.clone()))
                .collect();
            if missing.is_empty() {
                break;
            }
            if self.mark_coverage {
                rt.sometimes("torn_write_detected");
            }
            for (record, value) in &missing {
                let entry = format!("{record}={value}");
                if rt.disk_write(NODE, &entry).is_err() && self.mark_coverage {
                    rt.sometimes("write_failed_and_retried");
                }
            }
            let _ = rt.disk_flush(NODE);
            let _ = rt.disk_fsync(NODE);
        }
        let visible = rt.disk_read_all(NODE);
        let pending = std::mem::take(&mut self.pending);
        for (record, value) in pending {
            if visible.contains(&format!("{record}={value}")) {
                self.acked.insert(record, value);
            } else {
                rt.record("app.give_up", &format!("verify {record}={value}"));
            }
        }
    }

    /// Crash recovery: rebuild from the durable layer; unacknowledged
    /// pending records were volatile and are rewritten this cycle.
    fn recover(&mut self, rt: &mut SimRuntime<'_>) {
        let durable = rt.disk_read_durable(NODE);
        let pending = std::mem::take(&mut self.pending);
        for (record, value) in pending {
            let entry = format!("{record}={value}");
            if !durable.contains(&entry) {
                self.write_with_retry(rt, &record, &value);
            } else {
                self.pending.insert(record, value);
            }
        }
    }
}

impl Workload for WalDemo {
    fn name(&self) -> &str {
        if self.ack_at_flush {
            "demo-disk-buggy"
        } else {
            "demo-disk"
        }
    }

    fn property_contract(&self) -> PropertyContract {
        if self.ack_at_flush {
            PropertyContract::new(&[], &[]).with_oracles(&["wal_durability"])
        } else {
            PropertyContract::new(
                &[],
                &[
                    "write_failed_and_retried",
                    "torn_write_detected",
                    "crash_recovered",
                ],
            )
            .with_oracles(&["wal_durability"])
        }
    }

    fn end_state_oracles(&self) -> Vec<EndStateOracle> {
        vec![EndStateOracle {
            name: "wal_durability",
            check: wal_durability_oracle,
        }]
    }

    fn run(&self, ctx: &mut UniverseCtx) -> RunOutcome {
        if !self.ack_at_flush {
            ctx.declare_sometimes("write_failed_and_retried");
            ctx.declare_sometimes("torn_write_detected");
            ctx.declare_sometimes("crash_recovered");
        }
        let mut values = ctx.stream("values");
        let mut gremlin = ctx.stream("gremlin");
        let mut rt = ctx.runtime(|| Self::plan(&mut gremlin));

        let mut client = WalClient {
            ack_at_flush: self.ack_at_flush,
            mark_coverage: !self.ack_at_flush,
            pending: BTreeMap::new(),
            acked: BTreeMap::new(),
        };

        let mut next_op: u64 = 0;
        for op in 0..OPS {
            rt.set_timer((op + 1) * OP_SPACING_NANOS, op);
        }

        while let Some(ev) = rt.step() {
            match ev {
                StepEvent::Timer { token: op } => {
                    next_op = op + 1;
                    let value = values.next_below(1_000).to_string();
                    client.write_with_retry(&mut rt, &format!("r{op}"), &value);
                    if op % 4 == 3 {
                        client.commit(&mut rt, false);
                    }
                }
                StepEvent::Crashed => {
                    if !self.ack_at_flush {
                        rt.sometimes("crash_recovered");
                    }
                    client.recover(&mut rt);
                    // Timers died with the crash epoch: re-arm the
                    // remaining operations after the current instant.
                    let now = rt.now_nanos();
                    for (k, op) in (next_op..OPS).enumerate() {
                        rt.set_timer(now + (k as u64 + 1) * OP_SPACING_NANOS, op);
                    }
                }
                StepEvent::Delivered { .. } => {
                    unreachable!("demo-disk sends no messages")
                }
            }
        }

        client.commit(&mut rt, true);

        for (record, value) in &client.acked {
            rt.declare_end(&format!("acked:{record}"), value);
        }
        for entry in rt.disk_read_durable(NODE) {
            if let Some((record, value)) = parse_entry(&entry) {
                rt.declare_end(&format!("durable:{record}"), &value);
            }
        }
        rt.finish();
        RunOutcome::Completed
    }
}
