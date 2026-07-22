//! The Phase-1 Tier-1 simulated runtime: SimNet + SimDisk on the
//! deterministic scheduler, with RUNNER-OWNED fault injection.
//!
//! FoundationDB/Antithesis lineage: the runtime — not the workload — owns
//! fault scheduling. A workload declares interaction points (sends,
//! writes, timers, steps); the runtime drains the fault plan through a
//! [`vh_core::Scheduler`] of typed events, applies faults to concrete
//! operations, and measures each injection's semantic lifecycle into the
//! [`RuntimeEvidence`] ledger (see `evidence.rs` for the stage doctrine).
//!
//! Epistemics closed mechanically (loop-4 thread): every delivery, drop,
//! disk operation, crash, and lifecycle transition is recorded into the
//! trace BY THE RUNTIME — a workload cannot under-record runtime effects.
//! Workloads still record their own application events through the same
//! capability surface ([`SimRuntime::record`]).
//!
//! Determinism: all ordering flows through the scheduler's total
//! `(VirtualTime, seq)` order; fault events are scheduled at construction
//! (in canonical plan order) so a fault at time T is applied before any
//! delivery scheduled later for time T. State lives in `BTreeMap`s and
//! `VecDeque`s; there is no wall clock, OS randomness, or hash-order
//! iteration anywhere in this module.
//!
//! Fault semantics (v1, documented limits):
//! * `NetworkPartition` is network-wide for its duration; sends AND
//!   in-flight deliveries inside the window are dropped.
//! * `NetworkDelay`/`NetworkDuplicate`/`NetworkReorder`/`DiskWriteFail`/
//!   `TornWrite`/`FsyncLie` are one-shot: armed FIFO per kind, consumed
//!   by the next matching operation. One-shot arms SURVIVE a crash (they
//!   model environment faults, not process state).
//! * A held reorder that never sees a following send expires undelivered
//!   at [`SimRuntime::finish`] (recorded; the injection honestly stays
//!   Armed).
//! * `CrashRestart` wipes every node's volatile disk layers (buf+cache),
//!   cancels in-flight deliveries and timers (epoch bump), and surfaces
//!   as [`StepEvent::Crashed`]; durable disk state survives.
//! * `ClockSkew` advances the workload-visible LOCAL clock: after it
//!   arms, [`SimRuntime::now_nanos`] reads (and the timestamps of
//!   application [`SimRuntime::record`] events) diverge from the global
//!   scheduler frame by the accumulated skew. Runtime effects, timers,
//!   and lifecycle marks stay in the global frame — the divergence is
//!   measurable in-trace by comparing the two (convergence C3, audit D6;
//!   replaces the v1 offered-and-skipped no-op).

use std::collections::{BTreeMap, VecDeque};

use vh_core::{Scheduler, VirtualTime};
use vh_gremlin::{FaultKind, FaultPlan};
use vh_trace::DecisionTape;

use crate::evidence::{InjectionOutcome, RuntimeEvidence};
use crate::UniverseCtx;

pub type NodeId = u32;

/// Fixed base delivery latency for every message.
pub const BASE_LATENCY_NANOS: u64 = 1_000;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DiskError {
    /// The write was reported failed; nothing was appended.
    WriteFailed,
}

/// How a delivery was shaped by armed faults — attested by the runtime,
/// so workloads can mark coverage without guessing at causes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct DeliveryNote {
    pub delayed: bool,
    pub duplicate: bool,
    pub reordered: bool,
}

/// Workload-visible events surfaced by [`SimRuntime::step`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StepEvent {
    Delivered {
        from: NodeId,
        to: NodeId,
        payload: String,
        note: DeliveryNote,
    },
    Timer {
        token: u64,
    },
    /// The process crashed and restarted: volatile disk layers are gone,
    /// in-flight deliveries and timers are cancelled, durable state
    /// survives. The workload must rebuild from durable state.
    Crashed,
}

enum RtEvent {
    Fault {
        index: usize,
    },
    Deliver {
        epoch: u64,
        from: NodeId,
        to: NodeId,
        payload: String,
        note: DeliveryNote,
        delay_idx: Option<usize>,
        dup_idx: Option<usize>,
        reorder_idx: Option<usize>,
    },
    Timer {
        epoch: u64,
        token: u64,
    },
}

#[derive(Default)]
struct NodeDisk {
    buf: Vec<String>,
    cache: Vec<String>,
    durable: Vec<String>,
}

struct HeldMsg {
    from: NodeId,
    to: NodeId,
    payload: String,
    reorder_idx: usize,
}

enum LifecycleStage {
    Offered,
    Armed,
    Injected,
    Manifested,
    Recovered,
}

/// The simulated runtime for one universe. Constructed through
/// [`UniverseCtx::runtime`] (one per universe, fail closed); while it
/// lives, every interaction — network, disk, timers, time, evidence —
/// flows through it. Dropping it (or calling [`SimRuntime::finish`])
/// finalizes the fault-lifecycle ledger into the universe's observable
/// result.
pub struct SimRuntime<'a> {
    ctx: &'a mut UniverseCtx,
    sched: Scheduler<RtEvent>,
    /// Scheduler choice-point tape (`vh-decision-tape-v1`): a SEPARATE
    /// additive stream recording (site, candidate-set digest, chosen
    /// index, policy) per pop. Its digest is finalized into the
    /// universe result; the frozen execution trace never sees it
    /// (convergence C1, the standing W2 interface request).
    tape: DecisionTape,
    faults: Vec<FaultKind>,
    outcomes: Vec<InjectionOutcome>,
    epoch: u64,
    // network
    partition_until: u64,
    active_partitions: Vec<usize>,
    pending_delays: VecDeque<(usize, u64)>,
    pending_dups: VecDeque<usize>,
    pending_reorders: VecDeque<usize>,
    held: Option<HeldMsg>,
    drops: u64,
    // clock
    clock_skew_nanos: u64,
    pending_skew_reads: Vec<usize>,
    // disk
    disks: BTreeMap<NodeId, NodeDisk>,
    pending_write_fails: VecDeque<usize>,
    pending_torn: VecDeque<usize>,
    pending_fsync_lies: VecDeque<usize>,
    write_fail_recovery: Vec<(usize, NodeId)>,
    torn_recovery: Vec<(usize, NodeId)>,
    torn_records: Vec<(usize, NodeId, String)>,
    fsync_lies: Vec<(usize, NodeId, Vec<String>)>,
    // crash
    crash_awaiting_recovery: Option<usize>,
    finished: bool,
}

impl UniverseCtx {
    /// Construct the Phase-1 simulated runtime for this universe.
    ///
    /// The fault plan is retrieved through the SAME override-or-generate
    /// path as [`UniverseCtx::fault_plan_or`] — retrieval discipline and
    /// the `vh-fault-plan-v1` digest are identical, so shrinker/replay
    /// overrides flow through the runtime unchanged. From that point the
    /// RUNTIME owns injection: every planned injection is scheduled as a
    /// runner event and measured into [`RuntimeEvidence`].
    ///
    /// Fail closed: one runtime per universe in v1 (crash/restart is
    /// modeled INSIDE the runtime via epochs, not by re-construction).
    pub fn runtime(&mut self, generate: impl FnOnce() -> FaultPlan) -> SimRuntime<'_> {
        assert!(
            self.runtime_evidence.is_none(),
            "one sim runtime per universe (v1): a second construction would fragment the fault-lifecycle ledger"
        );
        let plan = self.fault_plan_or(generate);
        SimRuntime::new(self, plan)
    }
}

impl<'a> SimRuntime<'a> {
    fn new(ctx: &'a mut UniverseCtx, plan: FaultPlan) -> Self {
        let mut sched = Scheduler::new();
        let mut faults = Vec::new();
        let mut outcomes = Vec::new();
        for (index, inj) in plan.injections().iter().enumerate() {
            faults.push(inj.fault.clone());
            outcomes.push(InjectionOutcome::new(inj.at_nanos, inj.fault.canonical()));
            sched.schedule(VirtualTime(inj.at_nanos), RtEvent::Fault { index });
        }
        Self {
            ctx,
            sched,
            tape: DecisionTape::new(),
            faults,
            outcomes,
            epoch: 0,
            partition_until: 0,
            active_partitions: Vec::new(),
            pending_delays: VecDeque::new(),
            pending_dups: VecDeque::new(),
            pending_reorders: VecDeque::new(),
            held: None,
            drops: 0,
            clock_skew_nanos: 0,
            pending_skew_reads: Vec::new(),
            disks: BTreeMap::new(),
            pending_write_fails: VecDeque::new(),
            pending_torn: VecDeque::new(),
            pending_fsync_lies: VecDeque::new(),
            write_fail_recovery: Vec::new(),
            torn_recovery: Vec::new(),
            torn_records: Vec::new(),
            fsync_lies: Vec::new(),
            crash_awaiting_recovery: None,
            finished: false,
        }
    }

    // ---- capability delegation (ctx is exclusively borrowed) ----

    /// The GLOBAL scheduler clock. Runtime effects, timers, and
    /// lifecycle marks are stamped in this frame; `ClockSkew` never
    /// touches it.
    fn global_now(&self) -> u64 {
        self.ctx.clock.now().nanos()
    }

    /// The workload-visible LOCAL clock: the global frame plus any
    /// accumulated `ClockSkew`. The first read observing a newly armed
    /// skew is that fault's interception AND its workload-visible effect
    /// at once (Injected and Manifested coincide, like a partition
    /// drop); the divergence is measured into the trace as a
    /// `clock.read` event. A workload that never reads its clock
    /// honestly leaves the skew at Armed.
    pub fn now_nanos(&mut self) -> u64 {
        let global = self.global_now();
        let local = global.saturating_add(self.clock_skew_nanos);
        if !self.pending_skew_reads.is_empty() {
            for index in std::mem::take(&mut self.pending_skew_reads) {
                self.mark(index, LifecycleStage::Injected, global);
                self.mark(index, LifecycleStage::Manifested, global);
            }
            self.ctx.trace.record(
                global,
                "clock.read",
                &format!(
                    "local={local} global={global} skew={}",
                    self.clock_skew_nanos
                ),
            );
        }
        local
    }

    pub fn universe_id(&self) -> u64 {
        self.ctx.universe_id()
    }

    /// Record an APPLICATION trace event (runtime effects are recorded by
    /// the runtime itself). Stamped with the workload's LOCAL clock — a
    /// skewed component honestly records skewed timestamps.
    pub fn record(&mut self, kind: &str, data: &str) {
        let now = self.now_nanos();
        self.ctx.trace.record(now, kind, data);
    }

    pub fn always<F: FnOnce() -> String>(&mut self, name: &str, condition: bool, detail: F) {
        self.ctx.props.always(name, condition, detail);
    }

    pub fn declare_sometimes(&mut self, name: &str) {
        self.ctx.props.declare_sometimes(name);
    }

    pub fn sometimes(&mut self, name: &str) {
        self.ctx.props.sometimes(name);
    }

    /// Declare one end-state fact for post-run oracle judgment (see
    /// [`UniverseCtx::declare_end`] — records no trace event).
    pub fn declare_end(&mut self, key: &str, value: &str) {
        self.ctx.declare_end(key, value);
    }

    /// Messages dropped so far (partition, crash-epoch, reorder expiry) —
    /// runtime-attested, for coverage assertions.
    pub fn drops(&self) -> u64 {
        self.drops
    }

    // ---- network ----

    /// Send a message. Fire-and-forget: the sender learns nothing — a
    /// partition drop is visible only as non-delivery. Armed faults are
    /// consumed in fixed order: partition (window), reorder (hold),
    /// delay, duplicate.
    pub fn send(&mut self, from: NodeId, to: NodeId, payload: &str) {
        self.note_workload_op();
        let now = self.global_now();
        self.ctx
            .trace
            .record(now, "net.send", &format!("{from}->{to} {payload}"));

        if now < self.partition_until {
            let idx = self.covering_partition(now);
            self.mark(idx, LifecycleStage::Injected, now);
            self.mark(idx, LifecycleStage::Manifested, now);
            self.drops += 1;
            self.ctx.trace.record(
                now,
                "net.drop",
                &format!("partition i={idx} {from}->{to} {payload}"),
            );
            return;
        }

        if let Some(reorder_idx) = self.pending_reorders.pop_front() {
            self.ctx.trace.record(
                now,
                "net.hold",
                &format!("reorder i={reorder_idx} {from}->{to} {payload}"),
            );
            self.held = Some(HeldMsg {
                from,
                to,
                payload: payload.to_string(),
                reorder_idx,
            });
            return;
        }

        let mut latency = BASE_LATENCY_NANOS;
        let mut note = DeliveryNote::default();
        let mut delay_idx = None;
        if let Some((idx, delay)) = self.pending_delays.pop_front() {
            latency += delay;
            note.delayed = true;
            delay_idx = Some(idx);
            self.mark(idx, LifecycleStage::Injected, now);
            self.ctx.trace.record(
                now,
                "net.delay",
                &format!("i={idx} +{delay} {from}->{to} {payload}"),
            );
        }
        let deliver_at = now + latency;
        let dup = self.pending_dups.pop_front();
        if let Some(idx) = dup {
            self.mark(idx, LifecycleStage::Injected, now);
            self.ctx.trace.record(
                now,
                "net.duplicate",
                &format!("i={idx} {from}->{to} {payload}"),
            );
        }
        self.sched.schedule(
            VirtualTime(deliver_at),
            RtEvent::Deliver {
                epoch: self.epoch,
                from,
                to,
                payload: payload.to_string(),
                note,
                delay_idx,
                dup_idx: None,
                reorder_idx: None,
            },
        );
        if let Some(idx) = dup {
            self.sched.schedule(
                VirtualTime(deliver_at + 1),
                RtEvent::Deliver {
                    epoch: self.epoch,
                    from,
                    to,
                    payload: payload.to_string(),
                    note: DeliveryNote {
                        duplicate: true,
                        ..note
                    },
                    delay_idx: None,
                    dup_idx: Some(idx),
                    reorder_idx: None,
                },
            );
        }
        // A held reorder releases behind the NEXT message on the network:
        // its delivery is scheduled one nano after this send's, making the
        // swap concrete (and only then is the reorder Injected).
        if let Some(held) = self.held.take() {
            let idx = held.reorder_idx;
            self.mark(idx, LifecycleStage::Injected, now);
            self.ctx.trace.record(
                now,
                "net.release",
                &format!(
                    "reorder i={idx} {}->{} {}",
                    held.from, held.to, held.payload
                ),
            );
            self.sched.schedule(
                VirtualTime(deliver_at + 2),
                RtEvent::Deliver {
                    epoch: self.epoch,
                    from: held.from,
                    to: held.to,
                    payload: held.payload,
                    note: DeliveryNote {
                        reordered: true,
                        ..DeliveryNote::default()
                    },
                    delay_idx: None,
                    dup_idx: None,
                    reorder_idx: Some(idx),
                },
            );
        }
    }

    /// Arm a workload timer at an absolute virtual time (`at >= now`).
    /// Timers are process state: a crash cancels pending timers. Timers
    /// live in the GLOBAL frame: a skewed component computing `at` from
    /// its local clock schedules further into the global future than it
    /// believes — that drift IS the fault manifesting.
    pub fn set_timer(&mut self, at_nanos: u64, token: u64) {
        self.note_workload_op();
        assert!(
            at_nanos >= self.global_now(),
            "timer scheduled into the past: {at_nanos} < now {}",
            self.global_now()
        );
        self.sched.schedule(
            VirtualTime(at_nanos),
            RtEvent::Timer {
                epoch: self.epoch,
                token,
            },
        );
    }

    // ---- disk ----

    /// Append a record to the node's application buffer (volatile).
    pub fn disk_write(&mut self, node: NodeId, record: &str) -> Result<(), DiskError> {
        self.note_workload_op();
        let now = self.global_now();
        if let Some(idx) = self.pending_write_fails.pop_front() {
            self.mark(idx, LifecycleStage::Injected, now);
            self.mark(idx, LifecycleStage::Manifested, now);
            self.write_fail_recovery.push((idx, node));
            self.ctx
                .trace
                .record(now, "disk.write_fail", &format!("i={idx} n{node} {record}"));
            return Err(DiskError::WriteFailed);
        }
        if let Some(idx) = self.pending_torn.pop_front() {
            let keep = record.chars().count() / 2;
            let prefix: String = record.chars().take(keep).collect();
            self.mark(idx, LifecycleStage::Injected, now);
            self.torn_records.push((idx, node, prefix.clone()));
            self.torn_recovery.push((idx, node));
            self.disks.entry(node).or_default().buf.push(prefix.clone());
            self.ctx
                .trace
                .record(now, "disk.torn", &format!("i={idx} n{node} kept={prefix}"));
            return Ok(());
        }
        self.disks
            .entry(node)
            .or_default()
            .buf
            .push(record.to_string());
        self.ctx
            .trace
            .record(now, "disk.write", &format!("n{node} {record}"));
        // An intact write on this node closes earlier write-fail and torn
        // windows: the channel demonstrably works again.
        for (idx, n) in std::mem::take(&mut self.write_fail_recovery) {
            if n == node {
                self.mark(idx, LifecycleStage::Recovered, now);
            } else {
                self.write_fail_recovery.push((idx, n));
            }
        }
        for (idx, n) in std::mem::take(&mut self.torn_recovery) {
            if n == node {
                self.mark(idx, LifecycleStage::Recovered, now);
            } else {
                self.torn_recovery.push((idx, n));
            }
        }
        Ok(())
    }

    /// Move the node's application buffer into the OS cache (volatile).
    pub fn disk_flush(&mut self, node: NodeId) -> Result<(), DiskError> {
        self.note_workload_op();
        let now = self.global_now();
        let d = self.disks.entry(node).or_default();
        let moved = d.buf.len();
        let entries: Vec<String> = d.buf.drain(..).collect();
        d.cache.extend(entries);
        self.ctx
            .trace
            .record(now, "disk.flush", &format!("n{node} moved={moved}"));
        Ok(())
    }

    /// Persist the node's OS cache. An armed fsync-lie returns Ok while
    /// persisting NOTHING; the lie manifests only if a later crash
    /// actually loses the claimed-durable data, and recovers if a later
    /// honest fsync persists it first.
    pub fn disk_fsync(&mut self, node: NodeId) -> Result<(), DiskError> {
        self.note_workload_op();
        let now = self.global_now();
        if let Some(idx) = self.pending_fsync_lies.pop_front() {
            let claimed = self.disks.entry(node).or_default().cache.clone();
            self.mark(idx, LifecycleStage::Injected, now);
            self.fsync_lies.push((idx, node, claimed));
            self.ctx.trace.record(
                now,
                "disk.fsync_lie",
                &format!("i={idx} n{node} claimed={}", self.disks[&node].cache.len()),
            );
            return Ok(());
        }
        let d = self.disks.entry(node).or_default();
        let persisted = d.cache.len();
        let entries: Vec<String> = d.cache.drain(..).collect();
        d.durable.extend(entries);
        self.ctx
            .trace
            .record(now, "disk.fsync", &format!("n{node} persisted={persisted}"));
        let durable = self.disks[&node].durable.clone();
        for (idx, n, claimed) in std::mem::take(&mut self.fsync_lies) {
            if n == node && claimed.iter().all(|r| durable.contains(r)) {
                self.mark(idx, LifecycleStage::Recovered, now);
            } else {
                self.fsync_lies.push((idx, n, claimed));
            }
        }
        Ok(())
    }

    /// Read the node's full view: durable, then cache, then buffer.
    pub fn disk_read_all(&mut self, node: NodeId) -> Vec<String> {
        self.note_workload_op();
        let now = self.global_now();
        let d = self.disks.entry(node).or_default();
        let mut out = d.durable.clone();
        out.extend(d.cache.iter().cloned());
        out.extend(d.buf.iter().cloned());
        self.ctx
            .trace
            .record(now, "disk.read", &format!("n{node} entries={}", out.len()));
        self.manifest_torn_in(node, &out, now);
        out
    }

    /// Read only the node's durable layer (the post-crash recovery view).
    pub fn disk_read_durable(&mut self, node: NodeId) -> Vec<String> {
        self.note_workload_op();
        let now = self.global_now();
        let out = self.disks.entry(node).or_default().durable.clone();
        self.ctx.trace.record(
            now,
            "disk.read_durable",
            &format!("n{node} entries={}", out.len()),
        );
        self.manifest_torn_in(node, &out, now);
        out
    }

    // ---- event loop ----

    /// Surface the next workload-visible event, applying and measuring
    /// faults along the way. `None` when the scheduler is drained.
    pub fn step(&mut self) -> Option<StepEvent> {
        loop {
            // The sole runtime pop site (C0-granted wiring). Recording
            // is OPT-IN: the per-pop candidate digest costs ~50% wall
            // at the 200-universe runtime demo, so the C1 kill
            // criterion put the tape behind the flag; the un-recorded
            // arm is the original pop, bit-for-bit.
            let Self {
                sched, tape, ctx, ..
            } = self;
            let (at, ev) = if ctx.record_tape {
                sched.pop_recorded("runtime.step", "fifo-v0", |d| {
                    tape.record_decision(
                        &d.site_id,
                        &d.candidate_set_digest,
                        d.chosen_index,
                        &d.policy_id,
                    );
                })?
            } else {
                sched.pop()?
            };
            let now = at.nanos();
            self.ctx.clock.advance_to(at);
            match ev {
                RtEvent::Fault { index } => {
                    if let Some(step_ev) = self.apply_fault(index, now) {
                        return Some(step_ev);
                    }
                }
                RtEvent::Deliver {
                    epoch,
                    from,
                    to,
                    payload,
                    note,
                    delay_idx,
                    dup_idx,
                    reorder_idx,
                } => {
                    if epoch != self.epoch {
                        self.drops += 1;
                        self.ctx.trace.record(
                            now,
                            "net.drop",
                            &format!("crash-epoch {from}->{to} {payload}"),
                        );
                        continue;
                    }
                    if now < self.partition_until {
                        let idx = self.covering_partition(now);
                        self.mark(idx, LifecycleStage::Injected, now);
                        self.mark(idx, LifecycleStage::Manifested, now);
                        self.drops += 1;
                        self.ctx.trace.record(
                            now,
                            "net.drop",
                            &format!("partition i={idx} in-flight {from}->{to} {payload}"),
                        );
                        continue;
                    }
                    self.ctx
                        .trace
                        .record(now, "net.deliver", &format!("{from}->{to} {payload}"));
                    for idx in [delay_idx, dup_idx, reorder_idx].into_iter().flatten() {
                        self.mark(idx, LifecycleStage::Manifested, now);
                        self.mark(idx, LifecycleStage::Recovered, now);
                    }
                    // A completed delivery after a partition's window is
                    // the network demonstrably working again.
                    for i in 0..self.active_partitions.len() {
                        let idx = self.active_partitions[i];
                        let healed = self.outcomes[idx]
                            .armed_at()
                            .map(|_| self.partition_window_end(idx) <= now)
                            .unwrap_or(false);
                        if healed && !self.outcomes[idx].is_recovered() {
                            self.mark(idx, LifecycleStage::Recovered, now);
                        }
                    }
                    return Some(StepEvent::Delivered {
                        from,
                        to,
                        payload,
                        note,
                    });
                }
                RtEvent::Timer { epoch, token } => {
                    if epoch != self.epoch {
                        self.ctx.trace.record(
                            now,
                            "timer.drop",
                            &format!("crash-epoch token={token}"),
                        );
                        continue;
                    }
                    self.ctx
                        .trace
                        .record(now, "timer.fire", &format!("token={token}"));
                    return Some(StepEvent::Timer { token });
                }
            }
        }
    }

    /// Finalize: expire any held reorder, record the runtime summary, and
    /// bind the fault-lifecycle ledger into the universe's observable
    /// result. Dropping the runtime does the same.
    pub fn finish(self) {}

    // ---- internals ----

    fn apply_fault(&mut self, index: usize, now: u64) -> Option<StepEvent> {
        self.mark(index, LifecycleStage::Offered, now);
        let fault = self.faults[index].clone();
        match fault {
            FaultKind::CrashRestart => {
                self.mark(index, LifecycleStage::Armed, now);
                self.mark(index, LifecycleStage::Injected, now);
                // Lied-about durability manifests exactly when the crash
                // erases cache data an Ok fsync claimed was persisted.
                let disks = &self.disks;
                let lies = std::mem::take(&mut self.fsync_lies);
                let mut manifested_lies = Vec::new();
                for (idx, n, claimed) in lies {
                    let durable = disks.get(&n).map(|d| d.durable.clone()).unwrap_or_default();
                    if claimed.iter().any(|r| !durable.contains(r)) {
                        manifested_lies.push((idx, n, claimed));
                    } else {
                        self.fsync_lies.push((idx, n, claimed));
                    }
                }
                for (idx, _, _) in &manifested_lies {
                    self.mark(*idx, LifecycleStage::Manifested, now);
                }
                for d in self.disks.values_mut() {
                    d.buf.clear();
                    d.cache.clear();
                }
                if let Some(held) = self.held.take() {
                    self.drops += 1;
                    self.ctx.trace.record(
                        now,
                        "net.drop",
                        &format!(
                            "crash-held-reorder {}->{} {}",
                            held.from, held.to, held.payload
                        ),
                    );
                }
                self.epoch += 1;
                self.ctx
                    .trace
                    .record(now, "crash", &format!("epoch={}", self.epoch));
                self.mark(index, LifecycleStage::Manifested, now);
                self.crash_awaiting_recovery = Some(index);
                return Some(StepEvent::Crashed);
            }
            FaultKind::NetworkDelay { delay_nanos } => {
                self.mark(index, LifecycleStage::Armed, now);
                self.pending_delays.push_back((index, delay_nanos));
            }
            FaultKind::NetworkPartition { duration_nanos } => {
                self.mark(index, LifecycleStage::Armed, now);
                self.partition_until = self.partition_until.max(now + duration_nanos);
                self.active_partitions.push(index);
            }
            FaultKind::DiskWriteFail => {
                self.mark(index, LifecycleStage::Armed, now);
                self.pending_write_fails.push_back(index);
            }
            FaultKind::TornWrite => {
                self.mark(index, LifecycleStage::Armed, now);
                self.pending_torn.push_back(index);
            }
            FaultKind::FsyncLie => {
                self.mark(index, LifecycleStage::Armed, now);
                self.pending_fsync_lies.push_back(index);
            }
            FaultKind::NetworkDuplicate => {
                self.mark(index, LifecycleStage::Armed, now);
                self.pending_dups.push_back(index);
            }
            FaultKind::NetworkReorder => {
                self.mark(index, LifecycleStage::Armed, now);
                self.pending_reorders.push_back(index);
            }
            FaultKind::ClockSkew { skew_nanos } => {
                self.mark(index, LifecycleStage::Armed, now);
                self.clock_skew_nanos = self.clock_skew_nanos.saturating_add(skew_nanos);
                // A zero-magnitude skew can be generated
                // (next_below(horizon/20 + 1) includes 0) and cannot
                // diverge any reading: it honestly stays Armed forever
                // instead of claiming a manifestation that never
                // happened (review finding on this PR).
                if skew_nanos > 0 {
                    self.pending_skew_reads.push(index);
                }
                self.ctx.trace.record(
                    now,
                    "clock.skew",
                    &format!(
                        "i={index} +{skew_nanos} accumulated={}",
                        self.clock_skew_nanos
                    ),
                );
            }
        }
        None
    }

    /// Advance one injection's ladder AND record the transition as a
    /// trace event — every ledger advancement has a matching trace event
    /// at the moment it was measured.
    fn mark(&mut self, index: usize, stage: LifecycleStage, now: u64) {
        let (kind, already) = match stage {
            LifecycleStage::Offered => ("fault.offered", false),
            LifecycleStage::Armed => ("fault.armed", false),
            LifecycleStage::Injected => ("fault.injected", self.outcomes[index].is_injected()),
            LifecycleStage::Manifested => (
                "fault.manifested",
                self.outcomes[index].manifested_at().is_some(),
            ),
            LifecycleStage::Recovered => ("fault.recovered", self.outcomes[index].is_recovered()),
        };
        match stage {
            LifecycleStage::Offered => self.outcomes[index].offer(now),
            LifecycleStage::Armed => self.outcomes[index].arm(now),
            LifecycleStage::Injected => self.outcomes[index].inject(now),
            LifecycleStage::Manifested => self.outcomes[index].manifest(now),
            LifecycleStage::Recovered => self.outcomes[index].recover(now),
        }
        if !already {
            let data = format!("i={index} {}", self.outcomes[index].fault());
            self.ctx.trace.record(now, kind, &data);
        }
    }

    /// The first active partition whose window covers `now` (plan order —
    /// deterministic attribution for drops under overlapping windows).
    fn covering_partition(&self, now: u64) -> usize {
        *self
            .active_partitions
            .iter()
            .find(|&&idx| {
                self.outcomes[idx]
                    .armed_at()
                    .is_some_and(|armed| armed <= now && now < self.partition_window_end(idx))
            })
            .or_else(|| self.active_partitions.first())
            .expect("partition drop with no active partition is a kernel bug")
    }

    fn partition_window_end(&self, idx: usize) -> u64 {
        let armed = self.outcomes[idx].armed_at().unwrap_or(0);
        match self.faults[idx] {
            FaultKind::NetworkPartition { duration_nanos } => armed + duration_nanos,
            _ => 0,
        }
    }

    fn manifest_torn_in(&mut self, node: NodeId, contents: &[String], now: u64) {
        let hits: Vec<usize> = self
            .torn_records
            .iter()
            .filter(|(_, n, prefix)| *n == node && contents.contains(prefix))
            .map(|(idx, _, _)| *idx)
            .collect();
        for idx in hits {
            self.mark(idx, LifecycleStage::Manifested, now);
        }
    }

    /// First workload-initiated operation after a crash was surfaced:
    /// the process demonstrably resumed — the crash is Recovered.
    fn note_workload_op(&mut self) {
        if let Some(idx) = self.crash_awaiting_recovery.take() {
            let now = self.global_now();
            self.mark(idx, LifecycleStage::Recovered, now);
        }
    }

    fn finalize(&mut self) {
        if self.finished {
            return;
        }
        self.finished = true;
        let now = self.global_now();
        if let Some(held) = self.held.take() {
            self.drops += 1;
            self.ctx.trace.record(
                now,
                "net.drop",
                &format!(
                    "reorder-expired {}->{} {}",
                    held.from, held.to, held.payload
                ),
            );
        }
        self.ctx.trace.record(
            now,
            "runtime.end",
            &format!(
                "pending={} drops={} epoch={}",
                self.sched.len(),
                self.drops,
                self.epoch
            ),
        );
        self.ctx.runtime_evidence = Some(RuntimeEvidence::new(std::mem::take(&mut self.outcomes)));
        if self.ctx.record_tape {
            self.ctx.decision_tape_digest = Some(self.tape.digest_hex());
        }
    }
}

impl Drop for SimRuntime<'_> {
    fn drop(&mut self) {
        self.finalize();
    }
}
