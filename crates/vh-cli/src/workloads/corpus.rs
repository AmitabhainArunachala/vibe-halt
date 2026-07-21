//! Seeded vibe-bug corpus workloads (track vibe-bug-corpus-2026-07).
//!
//! Each workload embodies ONE real-world bug class in its own
//! application logic — the runtime is never weakened. The law each
//! violates is a named end-state oracle whose failure detail names the
//! violated items; fault-free universes PASS (vacuous-failure
//! doctrine). Entries + pinned recall claims: `corpus/entries/`.
//!
//! Classes here: lost update (VB-001), retry double-apply (VB-002),
//! dirty read (VB-003), crash-window TOCTOU (VB-004). VB-005
//! (fsync-lie durability hole) lives in `disk.rs` as the paranoid WAL
//! under a lying-hardware palette.

use std::collections::BTreeMap;

use vh_gremlin::{FaultInjection, FaultKind, FaultPlan};
use vh_multiverse::{
    EndState, EndStateOracle, PropertyContract, RunOutcome, StepEvent, UniverseCtx, Workload,
};

// ---------------------------------------------------------------- VB-001

const LU_STORE: u32 = 0;
const LU_WRITERS: [u32; 2] = [1, 2];
const LU_ROUNDS: u64 = 3;
const LU_ROUND_SPACING: u64 = 100_000;
const LU_REQUESTED: u64 = LU_ROUNDS * 2;

/// VB-001 lost-update: two writers increment a shared counter through
/// read-modify-write messages against a store that applies blind
/// last-write-wins sets (no compare-and-swap — the bug). A delayed read
/// reply overlaps the writers' cycles: both read the same value, both
/// write value+1, one increment vanishes.
pub struct LostUpdate;

fn lost_update_oracle(end: &EndState) -> Result<(), String> {
    let counter = end.get("counter").cloned().unwrap_or_default();
    let requested = end.get("requested").cloned().unwrap_or_default();
    if counter == requested {
        Ok(())
    } else {
        Err(format!(
            "final counter {counter} != {requested} requested increments — update(s) lost to read-modify-write overlap"
        ))
    }
}

impl Workload for LostUpdate {
    fn name(&self) -> &str {
        "corpus-lost-update"
    }

    fn property_contract(&self) -> PropertyContract {
        PropertyContract::new(&[], &[]).with_oracles(&["no_lost_updates"])
    }

    fn end_state_oracles(&self) -> Vec<EndStateOracle> {
        vec![EndStateOracle {
            name: "no_lost_updates",
            check: lost_update_oracle,
        }]
    }

    fn run(&self, ctx: &mut UniverseCtx) -> RunOutcome {
        let mut gremlin = ctx.stream("gremlin");
        let mut rt = ctx.runtime(|| {
            // Delays only: every message eventually delivers, so a
            // serialized history always counts to LU_REQUESTED — any
            // shortfall is the overlap bug, never message loss.
            let count = 2 + gremlin.next_below(3);
            let horizon = LU_ROUNDS * LU_ROUND_SPACING;
            FaultPlan::new(
                (0..count)
                    .map(|_| FaultInjection {
                        at_nanos: gremlin.next_below(horizon),
                        fault: FaultKind::NetworkDelay {
                            delay_nanos: 5_000 + gremlin.next_below(45_000),
                        },
                    })
                    .collect(),
            )
        });

        let mut counter: u64 = 0;
        for round in 0..LU_ROUNDS {
            // Writer cycles staggered inside each round.
            rt.set_timer(round * LU_ROUND_SPACING, LU_WRITERS[0] as u64);
            rt.set_timer(round * LU_ROUND_SPACING + 10_000, LU_WRITERS[1] as u64);
        }
        while let Some(ev) = rt.step() {
            match ev {
                StepEvent::Timer { token } => {
                    rt.send(token as u32, LU_STORE, &format!("read:{token}"));
                }
                StepEvent::Delivered { to, payload, .. } => {
                    if to == LU_STORE {
                        if let Some(w) = payload.strip_prefix("read:") {
                            let w: u32 = w.parse().expect("deterministic payload");
                            let reply = format!("val:{counter}");
                            rt.send(LU_STORE, w, &reply);
                        } else if let Some(x) = payload.strip_prefix("write:") {
                            // The bug: blind set, no compare-and-swap.
                            counter = x.parse().expect("deterministic payload");
                        }
                    } else if let Some(v) = payload.strip_prefix("val:") {
                        let v: u64 = v.parse().expect("deterministic payload");
                        let write = format!("write:{}", v + 1);
                        rt.send(to, LU_STORE, &write);
                    }
                }
                StepEvent::Crashed => unreachable!("delay-only palette"),
            }
        }
        rt.declare_end("counter", &counter.to_string());
        rt.declare_end("requested", &LU_REQUESTED.to_string());
        rt.finish();
        RunOutcome::Completed
    }
}

// ---------------------------------------------------------------- VB-002

const DA_CLIENT: u32 = 0;
const DA_SERVER: u32 = 1;
const DA_ITEMS: u64 = 4;
const DA_ITEM_SPACING: u64 = 80_000;
const DA_TIMEOUT: u64 = 40_000;
const DA_MAX_ATTEMPTS: u64 = 6;

/// VB-002 retry double-apply: the client retries un-acked appends (as
/// it must, the network is lossy) but the server applies every receipt
/// with no idempotency key (the bug). A duplicated delivery, a delayed
/// append racing its own retry, or a partition-eaten ack all turn one
/// logical append into two applications.
pub struct RetryDoubleApply;

fn exactly_once_oracle(end: &EndState) -> Result<(), String> {
    let mut violations = Vec::new();
    for item in 0..DA_ITEMS {
        let applied = end
            .get(&format!("applied:{item}"))
            .cloned()
            .unwrap_or_else(|| "0".to_string());
        if applied != "1" {
            violations.push(format!("item {item} applied {applied} times"));
        }
    }
    if violations.is_empty() {
        Ok(())
    } else {
        Err(format!(
            "exactly-once violated: {} (no idempotency key on the retry path)",
            violations.join(", ")
        ))
    }
}

impl Workload for RetryDoubleApply {
    fn name(&self) -> &str {
        "corpus-retry-double-apply"
    }

    fn property_contract(&self) -> PropertyContract {
        PropertyContract::new(&[], &[]).with_oracles(&["exactly_once"])
    }

    fn end_state_oracles(&self) -> Vec<EndStateOracle> {
        vec![EndStateOracle {
            name: "exactly_once",
            check: exactly_once_oracle,
        }]
    }

    fn run(&self, ctx: &mut UniverseCtx) -> RunOutcome {
        let mut gremlin = ctx.stream("gremlin");
        let mut rt = ctx.runtime(|| {
            // Blackout budget: <=4 partitions x <=50k = 200k, under the
            // 6x40k=240k retry budget — every item is eventually acked,
            // so the only exactly-once violations are OVER-application.
            let count = 2 + gremlin.next_below(3);
            let horizon = DA_ITEMS * DA_ITEM_SPACING;
            FaultPlan::new(
                (0..count)
                    .map(|_| {
                        let at_nanos = gremlin.next_below(horizon);
                        let fault = match gremlin.next_below(3) {
                            0 => FaultKind::NetworkPartition {
                                duration_nanos: 20_000 + gremlin.next_below(30_000),
                            },
                            1 => FaultKind::NetworkDelay {
                                delay_nanos: 5_000 + gremlin.next_below(45_000),
                            },
                            _ => FaultKind::NetworkDuplicate,
                        };
                        FaultInjection { at_nanos, fault }
                    })
                    .collect(),
            )
        });

        let mut acked = [false; DA_ITEMS as usize];
        let mut applied = [0u64; DA_ITEMS as usize];
        for item in 0..DA_ITEMS {
            rt.set_timer(item * DA_ITEM_SPACING, item * 16);
        }
        while let Some(ev) = rt.step() {
            match ev {
                StepEvent::Timer { token } => {
                    let (item, attempt) = (token / 16, token % 16);
                    if acked[item as usize] || attempt >= DA_MAX_ATTEMPTS {
                        continue;
                    }
                    rt.send(DA_CLIENT, DA_SERVER, &format!("append:{item}"));
                    let now = rt.now_nanos();
                    rt.set_timer(now + DA_TIMEOUT, item * 16 + attempt + 1);
                }
                StepEvent::Delivered { to, payload, .. } => {
                    if to == DA_SERVER {
                        if let Some(item) = payload.strip_prefix("append:") {
                            let item: u64 = item.parse().expect("deterministic payload");
                            // The bug: apply on every receipt, no dedupe.
                            applied[item as usize] += 1;
                            let ack = format!("ack:{item}");
                            rt.send(DA_SERVER, DA_CLIENT, &ack);
                        }
                    } else if let Some(item) = payload.strip_prefix("ack:") {
                        let item: u64 = item.parse().expect("deterministic payload");
                        acked[item as usize] = true;
                    }
                }
                StepEvent::Crashed => unreachable!("no CrashRestart in palette"),
            }
        }
        for item in 0..DA_ITEMS {
            rt.declare_end(
                &format!("applied:{item}"),
                &applied[item as usize].to_string(),
            );
        }
        rt.finish();
        RunOutcome::Completed
    }
}

// ---------------------------------------------------------------- VB-003

const DR_NODE: u32 = 0;
const DR_OPS: u64 = 18;
const DR_OP_SPACING: u64 = 20_000;

/// VB-003 dirty read: a reporter publishes values it read from the FULL
/// disk view — buffer and OS cache included — as if they were settled
/// facts (the bug: read-your-unflushed-writes handed downstream). A
/// crash erases the volatile layers and the published values never
/// existed durably.
pub struct DirtyRead;

fn published_durable_oracle(end: &EndState) -> Result<(), String> {
    let mut violations = Vec::new();
    for (key, value) in end {
        if let Some(record) = key.strip_prefix("published:") {
            let stored = end.get(&format!("durable:{record}"));
            if stored != Some(value) {
                violations.push(format!(
                    "published {record}={value} never became durable (found {stored:?})"
                ));
            }
        }
    }
    if violations.is_empty() {
        Ok(())
    } else {
        Err(format!(
            "dirty read published volatile data: {}",
            violations.join("; ")
        ))
    }
}

impl Workload for DirtyRead {
    fn name(&self) -> &str {
        "corpus-dirty-read"
    }

    fn property_contract(&self) -> PropertyContract {
        PropertyContract::new(&[], &[]).with_oracles(&["published_implies_durable"])
    }

    fn end_state_oracles(&self) -> Vec<EndStateOracle> {
        vec![EndStateOracle {
            name: "published_implies_durable",
            check: published_durable_oracle,
        }]
    }

    fn run(&self, ctx: &mut UniverseCtx) -> RunOutcome {
        let mut values = ctx.stream("values");
        let mut gremlin = ctx.stream("gremlin");
        let mut rt = ctx.runtime(|| {
            let count = 1 + gremlin.next_below(3);
            let horizon = DR_OPS * DR_OP_SPACING;
            FaultPlan::new(
                (0..count)
                    .map(|_| FaultInjection {
                        at_nanos: gremlin.next_below(horizon),
                        fault: FaultKind::CrashRestart,
                    })
                    .collect(),
            )
        });

        let mut published: BTreeMap<String, String> = BTreeMap::new();
        for op in 0..DR_OPS {
            rt.set_timer((op + 1) * DR_OP_SPACING, op);
        }
        let mut next_op = 0u64;
        while let Some(ev) = rt.step() {
            match ev {
                StepEvent::Timer { token: op } => {
                    next_op = op + 1;
                    let value = values.next_below(1_000).to_string();
                    let _ = rt.disk_write(DR_NODE, &format!("r{op}={value}"));
                    if op % 3 == 2 {
                        let _ = rt.disk_flush(DR_NODE);
                    }
                    if op % 9 == 8 {
                        let _ = rt.disk_fsync(DR_NODE);
                    }
                    if op % 2 == 1 {
                        // The bug: publish from the full volatile view.
                        for entry in rt.disk_read_all(DR_NODE) {
                            if let Some((k, v)) = entry.split_once('=') {
                                published.entry(k.to_string()).or_insert(v.to_string());
                            }
                        }
                    }
                }
                StepEvent::Crashed => {
                    // Lost volatile records are simply gone; what was
                    // already published stays published — that is the bug.
                }
                StepEvent::Delivered { .. } => unreachable!("no messages"),
            }
        }
        let _ = next_op;
        let _ = rt.disk_flush(DR_NODE);
        let _ = rt.disk_fsync(DR_NODE);
        for (record, value) in &published {
            rt.declare_end(&format!("published:{record}"), value);
        }
        for entry in rt.disk_read_durable(DR_NODE) {
            if let Some((k, v)) = entry.split_once('=') {
                rt.declare_end(&format!("durable:{k}"), v);
            }
        }
        rt.finish();
        RunOutcome::Completed
    }
}

// ---------------------------------------------------------------- VB-004

const TT_NODE: u32 = 0;
const TT_PAIRS: u64 = 5;
const TT_PAIR_SPACING: u64 = 60_000;
const TT_CHECK_BASE: u64 = 40_000;
const TT_ACT_DELTA: u64 = 10_000;
const TT_SETUP_TOKEN: u64 = 1_000;
const TT_CHECK_BASE_TOKEN: u64 = 2_000;
const TT_ACT_BASE_TOKEN: u64 = 3_000;

/// VB-004 crash-window TOCTOU: a session token lives in the VOLATILE
/// disk layers by design. Each privileged action is guarded by a
/// check-then-act pair with a window between them; a crash inside the
/// window kills the session, but the act handler trusts the remembered
/// check and never re-validates after restart (the bug). The workload
/// truthfully records the process epoch at check and at act — the
/// oracle demands they match per action.
pub struct CrashToctou;

fn toctou_oracle(end: &EndState) -> Result<(), String> {
    let mut violations = Vec::new();
    for (key, _) in end.iter() {
        if let Some(k) = key.strip_prefix("acted:") {
            let check = end.get(&format!("check_epoch:{k}"));
            let act = end.get(&format!("act_epoch:{k}"));
            if check != act {
                violations.push(format!(
                    "action {k} acted in epoch {act:?} on a check from epoch {check:?}"
                ));
            }
        }
    }
    if violations.is_empty() {
        Ok(())
    } else {
        Err(format!(
            "check-then-act crossed a crash window: {}",
            violations.join("; ")
        ))
    }
}

impl Workload for CrashToctou {
    fn name(&self) -> &str {
        "corpus-crash-toctou"
    }

    fn property_contract(&self) -> PropertyContract {
        PropertyContract::new(&[], &[]).with_oracles(&["act_epoch_matches_check"])
    }

    fn end_state_oracles(&self) -> Vec<EndStateOracle> {
        vec![EndStateOracle {
            name: "act_epoch_matches_check",
            check: toctou_oracle,
        }]
    }

    fn run(&self, ctx: &mut UniverseCtx) -> RunOutcome {
        let mut gremlin = ctx.stream("gremlin");
        let horizon = TT_CHECK_BASE + TT_PAIRS * TT_PAIR_SPACING;
        let mut rt = ctx.runtime(|| {
            let count = 1 + gremlin.next_below(3);
            FaultPlan::new(
                (0..count)
                    .map(|_| FaultInjection {
                        at_nanos: gremlin.next_below(horizon),
                        fault: FaultKind::CrashRestart,
                    })
                    .collect(),
            )
        });

        let mut crash_count: u64 = 0;
        let mut check_epoch: BTreeMap<u64, u64> = BTreeMap::new();
        let mut acted: Vec<(u64, u64, u64)> = Vec::new();
        rt.set_timer(1_000, TT_SETUP_TOKEN);
        for k in 0..TT_PAIRS {
            rt.set_timer(TT_CHECK_BASE + k * TT_PAIR_SPACING, TT_CHECK_BASE_TOKEN + k);
            rt.set_timer(
                TT_CHECK_BASE + k * TT_PAIR_SPACING + TT_ACT_DELTA,
                TT_ACT_BASE_TOKEN + k,
            );
        }
        while let Some(ev) = rt.step() {
            match ev {
                StepEvent::Timer { token } if token == TT_SETUP_TOKEN => {
                    // The session token is volatile by scenario design.
                    let _ = rt.disk_write(TT_NODE, "token=1");
                    let _ = rt.disk_flush(TT_NODE);
                }
                StepEvent::Timer { token } if token >= TT_ACT_BASE_TOKEN => {
                    let k = token - TT_ACT_BASE_TOKEN;
                    // The bug: trust the remembered check unconditionally.
                    if let Some(&epoch) = check_epoch.get(&k) {
                        let _ = rt.disk_write(TT_NODE, &format!("action{k}=done"));
                        let _ = rt.disk_flush(TT_NODE);
                        let _ = rt.disk_fsync(TT_NODE);
                        acted.push((k, epoch, crash_count));
                    }
                }
                StepEvent::Timer { token } if token >= TT_CHECK_BASE_TOKEN => {
                    let k = token - TT_CHECK_BASE_TOKEN;
                    if rt.disk_read_all(TT_NODE).contains(&"token=1".to_string()) {
                        check_epoch.insert(k, crash_count);
                    }
                }
                StepEvent::Timer { .. } => {}
                StepEvent::Crashed => {
                    crash_count += 1;
                    // Restart re-arms NOTHING and re-checks NOTHING: the
                    // remembered check_epoch map survives in app memory —
                    // that persistence-of-belief is the seeded bug. Note
                    // pending check/act timers died with the epoch; the
                    // driver re-arms only the remaining schedule.
                    let now = rt.now_nanos();
                    for k in 0..TT_PAIRS {
                        let check_at = TT_CHECK_BASE + k * TT_PAIR_SPACING;
                        if check_at > now {
                            rt.set_timer(check_at, TT_CHECK_BASE_TOKEN + k);
                        }
                        let act_at = check_at + TT_ACT_DELTA;
                        if act_at > now {
                            rt.set_timer(act_at, TT_ACT_BASE_TOKEN + k);
                        }
                    }
                }
                StepEvent::Delivered { .. } => unreachable!("no messages"),
            }
        }
        for (k, check, act) in &acted {
            rt.declare_end(&format!("acted:{k}"), "true");
            rt.declare_end(&format!("check_epoch:{k}"), &check.to_string());
            rt.declare_end(&format!("act_epoch:{k}"), &act.to_string());
        }
        rt.finish();
        RunOutcome::Completed
    }
}
