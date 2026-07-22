//! Seeded vibe-bug corpus workloads (track vibe-bug-corpus-2026-07).
//!
//! Each workload embodies ONE real-world bug class in its own
//! application logic — the runtime is never weakened. The law each
//! violates is a named end-state oracle whose failure detail names the
//! violated items; fault-free universes PASS (vacuous-failure
//! doctrine). Entries + pinned recall claims: `corpus/entries/`.
//!
//! Classes here: lost update (VB-001), retry double-apply (VB-002),
//! dirty read (VB-003), crash-window TOCTOU (VB-004), stale-sweep
//! re-dispatch (VB-007, HARVESTED from langchain-ai/langgraph#7417),
//! unvalidated checkpoint (VB-008, HARVESTED from langgraph#6491),
//! transient-fatal abort (VB-009, HARVESTED from OpenHands#12064),
//! resume-becomes-replay (VB-010, HARVESTED from langgraph#7361),
//! blind stream append (VB-011, HARVESTED from langchain#22227),
//! same-timestamp race (VB-006, SEEDED for the C2 PCT bet).
//! VB-005 (fsync-lie durability hole) lives in `disk.rs` as the
//! paranoid WAL under a lying-hardware palette. VB-006 is reserved for
//! the C2 same-timestamp race (convergence charter §4).

use std::collections::BTreeMap;

use vh_gremlin::{FaultInjection, FaultKind, FaultPalette, FaultPlan, PaletteChooser};
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
        let fault_palette = ctx.fault_palette();
        let universe_seed = ctx.universe_seed();
        let mut rt = ctx.runtime(|| {
            // Delays only: every message eventually delivers, so a
            // serialized history always counts to LU_REQUESTED — any
            // shortfall is the overlap bug, never message loss.
            let count = 2 + gremlin.next_below(3);
            let horizon = LU_ROUNDS * LU_ROUND_SPACING;
            let chooser = PaletteChooser::new(fault_palette, universe_seed, 1);
            FaultPlan::new(
                (0..count)
                    .map(|_| FaultInjection {
                        at_nanos: gremlin.next_below(horizon),
                        fault: {
                            if fault_palette == FaultPalette::Swarm {
                                let _ = chooser.choose(&mut gremlin);
                            }
                            FaultKind::NetworkDelay {
                                delay_nanos: 5_000 + gremlin.next_below(45_000),
                            }
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
        let fault_palette = ctx.fault_palette();
        let universe_seed = ctx.universe_seed();
        let mut rt = ctx.runtime(|| {
            // Blackout budget: <=4 partitions x <=50k = 200k, under the
            // 6x40k=240k retry budget — every item is eventually acked,
            // so the only exactly-once violations are OVER-application.
            let count = 2 + gremlin.next_below(3);
            let horizon = DA_ITEMS * DA_ITEM_SPACING;
            let chooser = PaletteChooser::new(fault_palette, universe_seed, 3);
            FaultPlan::new(
                (0..count)
                    .map(|_| {
                        let at_nanos = gremlin.next_below(horizon);
                        let fault = match chooser.choose(&mut gremlin) {
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
        let fault_palette = ctx.fault_palette();
        let universe_seed = ctx.universe_seed();
        let mut rt = ctx.runtime(|| {
            let count = 1 + gremlin.next_below(3);
            let horizon = DR_OPS * DR_OP_SPACING;
            let chooser = PaletteChooser::new(fault_palette, universe_seed, 1);
            FaultPlan::new(
                (0..count)
                    .map(|_| FaultInjection {
                        at_nanos: gremlin.next_below(horizon),
                        fault: {
                            if fault_palette == FaultPalette::Swarm {
                                let _ = chooser.choose(&mut gremlin);
                            }
                            FaultKind::CrashRestart
                        },
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
        let fault_palette = ctx.fault_palette();
        let universe_seed = ctx.universe_seed();
        let horizon = TT_CHECK_BASE + TT_PAIRS * TT_PAIR_SPACING;
        let mut rt = ctx.runtime(|| {
            let count = 1 + gremlin.next_below(3);
            let chooser = PaletteChooser::new(fault_palette, universe_seed, 1);
            FaultPlan::new(
                (0..count)
                    .map(|_| FaultInjection {
                        at_nanos: gremlin.next_below(horizon),
                        fault: {
                            if fault_palette == FaultPalette::Swarm {
                                let _ = chooser.choose(&mut gremlin);
                            }
                            FaultKind::CrashRestart
                        },
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

// ---------------------------------------------------------------- VB-007

const SR_DISPATCHER: u32 = 0;
const SR_WORKER: u32 = 1;
const SR_TASKS: u64 = 4;
const SR_TASK_SPACING: u64 = 100_000;
/// The stale-run sweep window: a task whose completion has not come back
/// by this long after dispatch is presumed dead and re-dispatched.
/// Normal round trip is 2 x BASE_LATENCY (2_000); armed delays reach
/// 50_000 — the bug window is a merely-slow call outliving the sweep.
const SR_SWEEP_NANOS: u64 = 20_000;

/// VB-007 stale-sweep re-dispatch (HARVESTED — langchain-ai/langgraph
/// issue #7417, 2026-04-05): LangGraph Cloud's stale-run sweep marks a
/// long tool call (~180s+, heartbeat 120s hardcoded) as dead and
/// re-dispatches it from the last checkpoint WHILE the original is
/// still running; both complete and the side effect lands twice.
/// Reduced mechanism: a dispatcher re-sends any task whose completion
/// missed the sweep deadline; the worker applies every receipt with no
/// idempotency key. The palette is DELAY-ONLY — no message is ever
/// lost, so every duplicate application is purely the sweep presuming a
/// slow call dead (VB-002's cousin: there the retry answers real loss;
/// here nothing was lost at all).
pub struct StaleRedispatch;

fn exactly_once_dispatch_oracle(end: &EndState) -> Result<(), String> {
    let mut violations = Vec::new();
    for task in 0..SR_TASKS {
        let applied = end
            .get(&format!("applied:{task}"))
            .cloned()
            .unwrap_or_else(|| "0".to_string());
        if applied != "1" {
            violations.push(format!("task {task} applied {applied} times"));
        }
    }
    if violations.is_empty() {
        Ok(())
    } else {
        Err(format!(
            "exactly-once violated: {} (stale sweep re-dispatched an in-flight call; worker has no idempotency key)",
            violations.join(", ")
        ))
    }
}

impl Workload for StaleRedispatch {
    fn name(&self) -> &str {
        "corpus-stale-redispatch"
    }

    fn property_contract(&self) -> PropertyContract {
        PropertyContract::new(&[], &["redispatch_fired"]).with_oracles(&["exactly_once_dispatch"])
    }

    fn end_state_oracles(&self) -> Vec<EndStateOracle> {
        vec![EndStateOracle {
            name: "exactly_once_dispatch",
            check: exactly_once_dispatch_oracle,
        }]
    }

    fn run(&self, ctx: &mut UniverseCtx) -> RunOutcome {
        ctx.declare_sometimes("redispatch_fired");
        let mut gremlin = ctx.stream("gremlin");
        let fault_palette = ctx.fault_palette();
        let universe_seed = ctx.universe_seed();
        let mut rt = ctx.runtime(|| {
            // Delays only: every dispatch and every completion is
            // eventually delivered, so an at-least-once sweep is never
            // NEEDED — any duplicate application is the sweep firing on
            // a merely-slow call (the harvested mechanism).
            let count = 2 + gremlin.next_below(3);
            let horizon = SR_TASKS * SR_TASK_SPACING;
            let chooser = PaletteChooser::new(fault_palette, universe_seed, 1);
            FaultPlan::new(
                (0..count)
                    .map(|_| FaultInjection {
                        at_nanos: gremlin.next_below(horizon),
                        fault: {
                            if fault_palette == FaultPalette::Swarm {
                                let _ = chooser.choose(&mut gremlin);
                            }
                            FaultKind::NetworkDelay {
                                delay_nanos: 5_000 + gremlin.next_below(45_000),
                            }
                        },
                    })
                    .collect(),
            )
        });

        // Even timer tokens dispatch task token/2; odd tokens are that
        // task's sweep deadline.
        let mut done = [false; SR_TASKS as usize];
        let mut applied = [0u64; SR_TASKS as usize];
        for task in 0..SR_TASKS {
            rt.set_timer(task * SR_TASK_SPACING, task * 2);
        }
        while let Some(ev) = rt.step() {
            match ev {
                StepEvent::Timer { token } => {
                    let task = token / 2;
                    if token % 2 == 0 {
                        rt.send(SR_DISPATCHER, SR_WORKER, &format!("task:{task}"));
                        let now = rt.now_nanos();
                        rt.set_timer(now + SR_SWEEP_NANOS, task * 2 + 1);
                    } else if !done[task as usize] {
                        // The sweep: completion missed the deadline, so
                        // the run is presumed dead and re-enqueued — the
                        // original is still in flight (delay-only).
                        rt.sometimes("redispatch_fired");
                        rt.send(SR_DISPATCHER, SR_WORKER, &format!("task:{task}"));
                    }
                }
                StepEvent::Delivered { to, payload, .. } => {
                    if to == SR_WORKER {
                        if let Some(task) = payload.strip_prefix("task:") {
                            let task: u64 = task.parse().expect("deterministic payload");
                            // The bug: apply on every receipt — no
                            // idempotency key, no in-flight check.
                            applied[task as usize] += 1;
                            let reply = format!("done:{task}");
                            rt.send(SR_WORKER, SR_DISPATCHER, &reply);
                        }
                    } else if let Some(task) = payload.strip_prefix("done:") {
                        let task: u64 = task.parse().expect("deterministic payload");
                        done[task as usize] = true;
                    }
                }
                StepEvent::Crashed => unreachable!("delay-only palette"),
            }
        }
        for task in 0..SR_TASKS {
            rt.declare_end(
                &format!("applied:{task}"),
                &applied[task as usize].to_string(),
            );
        }
        rt.finish();
        RunOutcome::Completed
    }
}

// ---------------------------------------------------------------- VB-008

const UC_NODE: u32 = 0;
const UC_CKPTS: u64 = 6;
const UC_CKPT_SPACING: u64 = 30_000;

/// VB-008 unvalidated checkpoint (HARVESTED — langchain-ai/langgraph
/// issue #6491, 2025-11-24): LangGraph validates node INPUT but not
/// node OUTPUT, so invalid state is checkpointed successfully and only
/// explodes later, when `get_state_history()` re-validates on
/// retrieval — the checkpoint is permanently unrecoverable. Reduced
/// mechanism: a checkpointer persists records and acknowledges after
/// fsync WITHOUT ever validating or reading back what it wrote
/// (validation lives on the read path only). A torn write persists
/// half a record while the writer sees Ok; retrieval rejects the
/// malformed record and the acknowledged checkpoint is gone for good.
/// Contrast demo-disk's paranoid WAL: its verify-after-fsync closes
/// exactly this window.
pub struct UnvalidatedCheckpoint;

fn checkpoint_recoverable_oracle(end: &EndState) -> Result<(), String> {
    let mut violations = Vec::new();
    for ckpt in 0..UC_CKPTS {
        if end.get(&format!("acked:{ckpt}")).is_none() {
            continue;
        }
        if end.get(&format!("recovered:{ckpt}")).map(String::as_str) != Some("true") {
            violations.push(format!("checkpoint {ckpt}"));
        }
    }
    if violations.is_empty() {
        Ok(())
    } else {
        Err(format!(
            "acknowledged but unrecoverable at retrieval: {} (persisted unvalidated; validation lives on the read path only)",
            violations.join(", ")
        ))
    }
}

impl Workload for UnvalidatedCheckpoint {
    fn name(&self) -> &str {
        "corpus-unvalidated-checkpoint"
    }

    fn property_contract(&self) -> PropertyContract {
        PropertyContract::new(&[], &[]).with_oracles(&["checkpoint_recoverable"])
    }

    fn end_state_oracles(&self) -> Vec<EndStateOracle> {
        vec![EndStateOracle {
            name: "checkpoint_recoverable",
            check: checkpoint_recoverable_oracle,
        }]
    }

    fn run(&self, ctx: &mut UniverseCtx) -> RunOutcome {
        let mut ops = ctx.stream("ops");
        let mut gremlin = ctx.stream("gremlin");
        let fault_palette = ctx.fault_palette();
        let universe_seed = ctx.universe_seed();
        let mut rt = ctx.runtime(|| {
            // Torn writes only: every write is acknowledged Ok and every
            // record is durably fsynced, so the ONLY way an acknowledged
            // checkpoint can be unrecoverable is the missing write-side
            // validation (the harvested asymmetry) meeting a tear.
            let count = 1 + gremlin.next_below(3);
            let horizon = UC_CKPTS * UC_CKPT_SPACING;
            let chooser = PaletteChooser::new(fault_palette, universe_seed, 1);
            FaultPlan::new(
                (0..count)
                    .map(|_| FaultInjection {
                        at_nanos: gremlin.next_below(horizon),
                        fault: {
                            if fault_palette == FaultPalette::Swarm {
                                let _ = chooser.choose(&mut gremlin);
                            }
                            FaultKind::TornWrite
                        },
                    })
                    .collect(),
            )
        });

        // Expected full record per checkpoint, deterministic: the
        // retrieval validator demands the exact framed record
        // ("ckpt:<id>:<payload>#end"); a torn half-record loses the
        // terminator and fails validation.
        let mut expected: Vec<String> = Vec::new();
        for ckpt in 0..UC_CKPTS {
            expected.push(format!("ckpt:{ckpt}:d{:04}#end", ops.next_below(10_000)));
            rt.set_timer(ckpt * UC_CKPT_SPACING, ckpt);
        }
        let mut acked = [false; UC_CKPTS as usize];
        while let Some(ev) = rt.step() {
            match ev {
                StepEvent::Timer { token } => {
                    let ckpt = token as usize;
                    // The bug: write -> flush -> fsync -> ACK. No
                    // validation, no read-back verify — the writer
                    // trusts its own Ok (langgraph#6491's output-side
                    // gap; demo-disk shows the paranoid fix).
                    let ok = rt.disk_write(UC_NODE, &expected[ckpt]).is_ok()
                        && rt.disk_flush(UC_NODE).is_ok()
                        && rt.disk_fsync(UC_NODE).is_ok();
                    if ok {
                        acked[ckpt] = true;
                    }
                }
                StepEvent::Delivered { .. } => unreachable!("no messages"),
                StepEvent::Crashed => unreachable!("torn-only palette"),
            }
        }
        // Retrieval (the get_state_history moment): read durable state
        // and validate each record on the way out.
        let durable = rt.disk_read_durable(UC_NODE);
        for (ckpt, want) in expected.iter().enumerate() {
            if acked[ckpt] {
                rt.declare_end(&format!("acked:{ckpt}"), "true");
                let recovered = durable.iter().any(|r| r == want);
                rt.declare_end(&format!("recovered:{ckpt}"), &recovered.to_string());
            }
        }
        rt.finish();
        RunOutcome::Completed
    }
}

// ---------------------------------------------------------------- VB-009

const SA_CLIENT: u32 = 0;
const SA_BACKEND: u32 = 1;
const SA_TASKS: u64 = 5;
const SA_TASK_SPACING: u64 = 80_000;
/// Reply deadline: normal round trip is 2 x BASE_LATENCY (2_000); armed
/// delays reach 50_000 and partitions eat messages outright — the bug
/// window is any transient failure outliving this deadline.
const SA_TIMEOUT: u64 = 30_000;

/// VB-009 transient-fatal abort (HARVESTED — OpenHands/OpenHands issue
/// #12064, 2025-12-16, fixed by PR #12117): a LiteLLM-proxy 502 Bad
/// Gateway surfaces as `litellm.APIError`, which is MISSING from
/// `LLM_RETRY_EXCEPTIONS` in openhands/llm/llm.py — the retry logic
/// does not recognize the transient error, the agent controller
/// catches the unhandled exception, and the whole agent crashes,
/// abandoning the session and every remaining accepted task. Reduced
/// mechanism: a client awaits each backend reply under a deadline; on
/// deadline it classifies the failure as FATAL (the missing retriable
/// entry) and aborts the entire session — though the fault was a
/// transient partition or a merely-slow reply and the network heals.
/// Distinct from demo-net-buggy (fire-and-forget never LEARNS of
/// failure): this client learns, misclassifies, and takes the whole
/// session down with it — blast radius, not blindness.
pub struct TransientFatalAbort;

fn session_complete_oracle(end: &EndState) -> Result<(), String> {
    let mut violations = Vec::new();
    for task in 0..SA_TASKS {
        if end.get(&format!("completed:{task}")).map(String::as_str) != Some("true") {
            violations.push(format!("task {task}"));
        }
    }
    if violations.is_empty() {
        Ok(())
    } else {
        Err(format!(
            "accepted work abandoned: {} never completed (transient failure classified as fatal; session aborted instead of retried)",
            violations.join(", ")
        ))
    }
}

impl Workload for TransientFatalAbort {
    fn name(&self) -> &str {
        "corpus-transient-fatal-abort"
    }

    fn property_contract(&self) -> PropertyContract {
        PropertyContract::new(&[], &["session_aborted"]).with_oracles(&["session_complete"])
    }

    fn end_state_oracles(&self) -> Vec<EndStateOracle> {
        vec![EndStateOracle {
            name: "session_complete",
            check: session_complete_oracle,
        }]
    }

    fn run(&self, ctx: &mut UniverseCtx) -> RunOutcome {
        ctx.declare_sometimes("session_aborted");
        let mut gremlin = ctx.stream("gremlin");
        let fault_palette = ctx.fault_palette();
        let universe_seed = ctx.universe_seed();
        let mut rt = ctx.runtime(|| {
            // Transient faults only: partitions heal and delays deliver,
            // so a retrying client would always finish the session —
            // every abandonment is the fatal misclassification, never a
            // permanently dead backend.
            let count = 2 + gremlin.next_below(3);
            let horizon = SA_TASKS * SA_TASK_SPACING;
            let chooser = PaletteChooser::new(fault_palette, universe_seed, 2);
            FaultPlan::new(
                (0..count)
                    .map(|_| {
                        let at_nanos = gremlin.next_below(horizon);
                        let fault = match chooser.choose(&mut gremlin) {
                            0 => FaultKind::NetworkPartition {
                                duration_nanos: 20_000 + gremlin.next_below(30_000),
                            },
                            _ => FaultKind::NetworkDelay {
                                delay_nanos: 5_000 + gremlin.next_below(45_000),
                            },
                        };
                        FaultInjection { at_nanos, fault }
                    })
                    .collect(),
            )
        });

        // Even tokens dispatch task token/2; odd tokens are that task's
        // reply deadline. All SA_TASKS are accepted up front — the
        // session's promise is to complete them all.
        let mut completed = [false; SA_TASKS as usize];
        let mut aborted = false;
        for task in 0..SA_TASKS {
            rt.set_timer(task * SA_TASK_SPACING, task * 2);
        }
        while let Some(ev) = rt.step() {
            match ev {
                StepEvent::Timer { token } => {
                    let task = token / 2;
                    if aborted {
                        // The crashed controller steps nothing further.
                        continue;
                    }
                    if token % 2 == 0 {
                        rt.send(SA_CLIENT, SA_BACKEND, &format!("req:{task}"));
                        let now = rt.now_nanos();
                        rt.set_timer(now + SA_TIMEOUT, task * 2 + 1);
                    } else if !completed[task as usize] {
                        // The bug: the transient failure is not in the
                        // retriable set — the unhandled classification
                        // kills the whole session, not the one call.
                        rt.sometimes("session_aborted");
                        aborted = true;
                    }
                }
                StepEvent::Delivered { to, payload, .. } => {
                    if to == SA_BACKEND {
                        if let Some(task) = payload.strip_prefix("req:") {
                            let task: u64 = task.parse().expect("deterministic payload");
                            let reply = format!("ok:{task}");
                            rt.send(SA_BACKEND, SA_CLIENT, &reply);
                        }
                    } else if let Some(task) = payload.strip_prefix("ok:") {
                        if !aborted {
                            let task: u64 = task.parse().expect("deterministic payload");
                            completed[task as usize] = true;
                        }
                        // After the abort the controller is gone; late
                        // replies fall on the floor.
                    }
                }
                StepEvent::Crashed => unreachable!("no CrashRestart in palette"),
            }
        }
        for task in 0..SA_TASKS {
            rt.declare_end(
                &format!("completed:{task}"),
                &completed[task as usize].to_string(),
            );
        }
        rt.finish();
        RunOutcome::Completed
    }
}

// ---------------------------------------------------------------- VB-010

const RR_NODE: u32 = 0;
const RR_STEPS: u64 = 5;
const RR_STEP_SPACING: u64 = 30_000;

/// VB-010 resume-becomes-replay (HARVESTED — langchain-ai/langgraph
/// issue #7361, 2026-03-31, regression in 1.1.x): resuming a graph
/// from a specific `checkpoint_id` re-executes from the BEGINNING
/// instead of continuing at the interrupt point — "the second run for
/// resume still run from the beginning of the graph, not interrupt
/// trigger point" — though the checkpoint with the progress exists
/// (removing checkpoint_id from config is a workaround: the data was
/// fine, the resume path misuses it). Reduced mechanism: a pipeline
/// applies side-effecting steps and durably fsyncs a progress cursor
/// after each; on crash-recovery it READS the cursor back — and
/// restarts from step 0 anyway. Every pre-crash side effect lands a
/// second time.
pub struct ResumeReplay;

fn resume_at_most_once_oracle(end: &EndState) -> Result<(), String> {
    let mut violations = Vec::new();
    for step in 0..RR_STEPS {
        let applied = end
            .get(&format!("applied:{step}"))
            .cloned()
            .unwrap_or_else(|| "0".to_string());
        let n: u64 = applied.parse().unwrap_or(0);
        if n > 1 {
            violations.push(format!("step {step} applied {n} times"));
        }
    }
    if violations.is_empty() {
        Ok(())
    } else {
        Err(format!(
            "resume replayed completed work: {} (durable cursor read then ignored; resume restarted from step 0)",
            violations.join(", ")
        ))
    }
}

impl Workload for ResumeReplay {
    fn name(&self) -> &str {
        "corpus-resume-replay"
    }

    fn property_contract(&self) -> PropertyContract {
        PropertyContract::new(&[], &["crash_resume"]).with_oracles(&["resume_at_most_once"])
    }

    fn end_state_oracles(&self) -> Vec<EndStateOracle> {
        vec![EndStateOracle {
            name: "resume_at_most_once",
            check: resume_at_most_once_oracle,
        }]
    }

    fn run(&self, ctx: &mut UniverseCtx) -> RunOutcome {
        ctx.declare_sometimes("crash_resume");
        let mut gremlin = ctx.stream("gremlin");
        let fault_palette = ctx.fault_palette();
        let universe_seed = ctx.universe_seed();
        let mut rt = ctx.runtime(|| {
            // Crashes only (0..=2 — crash-free universes exist and must
            // PASS, vacuous-failure doctrine): the cursor is fsynced
            // before each crash can matter, so recovery ALWAYS has the
            // truth on disk — every duplicate application is the resume
            // path ignoring it.
            let count = gremlin.next_below(3);
            let horizon = RR_STEPS * RR_STEP_SPACING;
            let chooser = PaletteChooser::new(fault_palette, universe_seed, 1);
            FaultPlan::new(
                (0..count)
                    .map(|_| FaultInjection {
                        at_nanos: gremlin.next_below(horizon),
                        fault: {
                            if fault_palette == FaultPalette::Swarm {
                                let _ = chooser.choose(&mut gremlin);
                            }
                            FaultKind::CrashRestart
                        },
                    })
                    .collect(),
            )
        });

        // applied[] is the OUTSIDE WORLD (emails sent, tools invoked):
        // it survives crashes. The progress cursor is durably fsynced
        // after every step — recovery genuinely has it.
        let mut applied = [0u64; RR_STEPS as usize];
        for step in 0..RR_STEPS {
            rt.set_timer(step * RR_STEP_SPACING, step);
        }
        while let Some(ev) = rt.step() {
            match ev {
                StepEvent::Timer { token } => {
                    let step = token as usize;
                    applied[step] += 1;
                    let _ = rt.disk_write(RR_NODE, &format!("cursor={}", step + 1));
                    let _ = rt.disk_flush(RR_NODE);
                    let _ = rt.disk_fsync(RR_NODE);
                }
                StepEvent::Crashed => {
                    // Recovery: the durable cursor is read back — and
                    // then ignored (the bug): the resume schedules the
                    // pipeline from step 0, replaying completed work.
                    rt.sometimes("crash_resume");
                    let durable = rt.disk_read_durable(RR_NODE);
                    let _resume_from = durable
                        .iter()
                        .filter_map(|r| r.strip_prefix("cursor=")?.parse::<u64>().ok())
                        .max()
                        .unwrap_or(0);
                    let now = rt.now_nanos();
                    for step in 0..RR_STEPS {
                        rt.set_timer(now + (step + 1) * RR_STEP_SPACING, step);
                    }
                }
                StepEvent::Delivered { .. } => unreachable!("no messages"),
            }
        }
        for step in 0..RR_STEPS {
            rt.declare_end(
                &format!("applied:{step}"),
                &applied[step as usize].to_string(),
            );
        }
        rt.finish();
        RunOutcome::Completed
    }
}

// ---------------------------------------------------------------- VB-011

const BS_PRODUCER: u32 = 0;
const BS_CONSUMER: u32 = 1;
const BS_CHUNKS: u64 = 8;
const BS_CHUNK_SPACING: u64 = 20_000;

/// VB-011 blind stream append (HARVESTED — langchain-ai/langchain
/// issue #22227, 2024-05-28, closed): `astream_events` (V1 and V2)
/// delivers duplicate content in `on_chat_model_stream` — nested
/// callback/streaming layers re-emit the same chunk, and consumers see
/// every token twice ("Books| Books|", "1|1|.|.|"). The consumer-side
/// defect this harvests: assembling a stream by BLIND APPEND, trusting
/// the event stream to be exactly-once-in-order — no sequence numbers,
/// no deduplication, no reorder handling. Reduced mechanism: a
/// producer streams uniquely-numbered chunks; the consumer appends
/// every delivery in arrival order, ignoring the sequence number it
/// was handed; a duplicated or reordered delivery corrupts the
/// assembled document.
pub struct BlindStreamAppend;

fn stream_integrity_oracle(end: &EndState) -> Result<(), String> {
    let assembled = end.get("assembled").cloned().unwrap_or_default();
    let expected = end.get("expected").cloned().unwrap_or_default();
    if assembled == expected {
        Ok(())
    } else {
        Err(format!(
            "assembled stream {assembled:?} != sent stream {expected:?} (duplicated or reordered delivery appended blindly; no sequence discipline at the consumer)"
        ))
    }
}

impl Workload for BlindStreamAppend {
    fn name(&self) -> &str {
        "corpus-blind-stream-append"
    }

    fn property_contract(&self) -> PropertyContract {
        PropertyContract::new(&[], &[]).with_oracles(&["stream_integrity"])
    }

    fn end_state_oracles(&self) -> Vec<EndStateOracle> {
        vec![EndStateOracle {
            name: "stream_integrity",
            check: stream_integrity_oracle,
        }]
    }

    fn run(&self, ctx: &mut UniverseCtx) -> RunOutcome {
        let mut ops = ctx.stream("ops");
        let mut gremlin = ctx.stream("gremlin");
        let fault_palette = ctx.fault_palette();
        let universe_seed = ctx.universe_seed();
        let mut rt = ctx.runtime(|| {
            // Duplicates and pairwise reorders only (0..=2 — fault-free
            // universes exist and must PASS): with the eos trailer below
            // no content chunk is ever lost, so the assembled stream can
            // only differ from the sent stream through the consumer's
            // missing sequence discipline meeting a shaped delivery.
            let count = gremlin.next_below(3);
            let horizon = BS_CHUNKS * BS_CHUNK_SPACING;
            let chooser = PaletteChooser::new(fault_palette, universe_seed, 2);
            FaultPlan::new(
                (0..count)
                    .map(|_| {
                        let at_nanos = gremlin.next_below(horizon);
                        let fault = match chooser.choose(&mut gremlin) {
                            0 => FaultKind::NetworkDuplicate,
                            _ => FaultKind::NetworkReorder,
                        };
                        FaultInjection { at_nanos, fault }
                    })
                    .collect(),
            )
        });

        // Unique token per chunk: any duplication or swap is visible in
        // the assembled document.
        let mut tokens: Vec<String> = Vec::new();
        for chunk in 0..BS_CHUNKS {
            tokens.push(format!("t{chunk}x{}", ops.next_below(100)));
            rt.set_timer(chunk * BS_CHUNK_SPACING, chunk);
        }
        // End-of-stream trailer: a held pairwise reorder releases its
        // captive only when a FOLLOWING send occurs; without a trailer a
        // reorder arming near the last chunk silently EXPIRES it —
        // content loss, which would blur the oracle's attribution. The
        // trailer guarantees every held content chunk is released
        // (reordered, not lost); a held trailer expiring is harmless
        // because the consumer ignores it.
        rt.set_timer(BS_CHUNKS * BS_CHUNK_SPACING, BS_CHUNKS);
        let mut assembled: Vec<String> = Vec::new();
        while let Some(ev) = rt.step() {
            match ev {
                StepEvent::Timer { token } => {
                    if token == BS_CHUNKS {
                        rt.send(BS_PRODUCER, BS_CONSUMER, "eos");
                        continue;
                    }
                    let chunk = token as usize;
                    rt.send(
                        BS_PRODUCER,
                        BS_CONSUMER,
                        &format!("chunk:{chunk}:{}", tokens[chunk]),
                    );
                }
                StepEvent::Delivered { payload, .. } => {
                    if let Some(rest) = payload.strip_prefix("chunk:") {
                        // The bug: the sequence number is RIGHT THERE in
                        // the payload and the consumer appends anyway —
                        // no dedupe, no ordering, arrival order is
                        // trusted as stream order.
                        if let Some((_seq, tok)) = rest.split_once(':') {
                            assembled.push(tok.to_string());
                        }
                    }
                }
                StepEvent::Crashed => unreachable!("no CrashRestart in palette"),
            }
        }
        rt.declare_end("assembled", &assembled.join("|"));
        rt.declare_end("expected", &tokens.join("|"));
        rt.finish();
        RunOutcome::Completed
    }
}

// ---------------------------------------------------------------- VB-006

const ST_WRITER: u32 = 0;
const ST_STORE: u32 = 1;
const ST_ROUNDS: u64 = 6;
const ST_ROUND_SPACING: u64 = 40_000;

/// VB-006 same-timestamp race (SEEDED for convergence C2 / Track-2 W3;
/// reserved since the campaign charter): each round the writer sends
/// `init` then `commit` back-to-back, so both arrive at the SAME
/// virtual time — a same-timestamp scheduler frontier of exactly two.
/// The store applies `commit` without checking that `init` arrived (the
/// bug: an ordering assumption with no guard). Under FIFO v0 the
/// insertion-order tiebreak ALWAYS delivers init first — the bug is
/// invisible by construction, in any universe, at any seed. Only a
/// same-timestamp schedule strategy (PCT / uniform tiebreak) can flip
/// the pair and expose it. No faults are injected at all: the race is
/// pure scheduling.
pub struct SameTimestampRace;

fn init_before_commit_oracle(end: &EndState) -> Result<(), String> {
    let mut violations = Vec::new();
    for round in 0..ST_ROUNDS {
        if end.get(&format!("commit_base:{round}")).map(String::as_str) == Some("missing-init") {
            violations.push(format!("round {round}"));
        }
    }
    if violations.is_empty() {
        Ok(())
    } else {
        Err(format!(
            "commit applied before its init: {} (same-timestamp delivery order assumed, never guarded)",
            violations.join(", ")
        ))
    }
}

impl Workload for SameTimestampRace {
    fn name(&self) -> &str {
        "corpus-same-timestamp-race"
    }

    fn property_contract(&self) -> PropertyContract {
        PropertyContract::new(&[], &[]).with_oracles(&["init_before_commit"])
    }

    fn end_state_oracles(&self) -> Vec<EndStateOracle> {
        vec![EndStateOracle {
            name: "init_before_commit",
            check: init_before_commit_oracle,
        }]
    }

    fn run(&self, ctx: &mut UniverseCtx) -> RunOutcome {
        let mut ops = ctx.stream("ops");
        let mut rt = ctx.runtime(|| FaultPlan::new(Vec::new()));

        // Per-universe payload texture only — behavior is identical in
        // every universe under FIFO; the SCHEDULE is the only variable.
        let mut vals: Vec<u64> = Vec::new();
        for round in 0..ST_ROUNDS {
            vals.push(ops.next_below(1_000));
            rt.set_timer(round * ST_ROUND_SPACING, round);
        }
        let mut inited = [false; ST_ROUNDS as usize];
        let mut outcome: Vec<Option<&'static str>> = vec![None; ST_ROUNDS as usize];
        while let Some(ev) = rt.step() {
            match ev {
                StepEvent::Timer { token } => {
                    let round = token as usize;
                    // Back-to-back sends: identical delivery time, one
                    // same-timestamp frontier per round.
                    rt.send(
                        ST_WRITER,
                        ST_STORE,
                        &format!("init:{round}:{}", vals[round]),
                    );
                    rt.send(ST_WRITER, ST_STORE, &format!("commit:{round}"));
                }
                StepEvent::Delivered { payload, .. } => {
                    if let Some(rest) = payload.strip_prefix("init:") {
                        let round: usize = rest
                            .split(':')
                            .next()
                            .and_then(|r| r.parse().ok())
                            .expect("deterministic payload");
                        inited[round] = true;
                    } else if let Some(round) = payload.strip_prefix("commit:") {
                        let round: usize = round.parse().expect("deterministic payload");
                        // The bug: apply the commit against whatever
                        // base is present — no init-arrived guard.
                        outcome[round] = Some(if inited[round] { "ok" } else { "missing-init" });
                    }
                }
                StepEvent::Crashed => unreachable!("empty fault plan"),
            }
        }
        for (round, oc) in outcome.iter().enumerate() {
            rt.declare_end(
                &format!("commit_base:{round}"),
                oc.unwrap_or("never-committed"),
            );
        }
        rt.finish();
        RunOutcome::Completed
    }
}
