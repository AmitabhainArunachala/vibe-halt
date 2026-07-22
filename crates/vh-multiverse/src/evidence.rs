//! Runner-owned semantic fault-lifecycle evidence (Phase-1 sim runtime).
//!
//! This is the truthful ladder the retrieval-only ledger could not claim
//! (hardening-loop-4 GAP 5, DEFERRED item closed 2026-07-21, ahead of its
//! 2026-08-15 due date): with the RUNTIME owning fault scheduling, each
//! injection's lifecycle is measured by the runner itself —
//!
//! `Offered → Armed → Injected → Manifested → Recovered`
//!
//! Stage semantics, exactly what is measured and nothing more:
//!
//! * **Offered** — the runtime's event loop reached the injection's
//!   scheduled virtual time. An injection beyond the workload's last
//!   `step()` is honestly never offered.
//! * **Armed** — the runtime installed the fault (partition window set,
//!   one-shot fault queued, crash initiated, clock-skew offset added to
//!   the workload-visible local clock).
//! * **Injected** — the armed fault intercepted a concrete operation
//!   (a send dropped or delayed, a write failed or torn, an fsync lied,
//!   volatile state wiped by a crash, a clock read returning skewed
//!   time). A skew whose clock is never read honestly stays Armed.
//! * **Manifested** — the effect crossed the workload-visible API
//!   surface (an `Err` returned, a shaped/late delivery handed over, a
//!   torn record read back, lied-about data actually lost at a crash,
//!   the `Crashed` event delivered). For faults whose effect is
//!   unconditional at the point of injection (a partition drop, a
//!   skewed clock read), the two timestamps coincide by design —
//!   non-delivery / the skewed reading IS the effect.
//! * **Recovered** — the faulted channel demonstrably operated normally
//!   again (post-heal delivery, post-failure successful write, honest
//!   fsync persisting lied-about data, first workload-initiated
//!   operation after a crash was observed).
//!
//! Every stage transition is recorded by the runtime as a trace event at
//! the moment it is measured; a stage that never happened stays `None`.
//! Workloads cannot construct or edit any of this: fields are private,
//! stage advancement is `pub(crate)`, and the ladder is monotone —
//! attempting an out-of-order transition is a kernel bug and panics.

use std::fmt::Write as _;

/// The measured lifecycle of ONE planned injection. Construction and
/// mutation are crate-internal (runner-owned); downstream code reads.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InjectionOutcome {
    at_nanos: u64,
    fault: String,
    offered_at: Option<u64>,
    armed_at: Option<u64>,
    injected_at: Option<u64>,
    manifested_at: Option<u64>,
    recovered_at: Option<u64>,
}

impl InjectionOutcome {
    pub(crate) fn new(at_nanos: u64, fault: String) -> Self {
        Self {
            at_nanos,
            fault,
            offered_at: None,
            armed_at: None,
            injected_at: None,
            manifested_at: None,
            recovered_at: None,
        }
    }

    /// The plan's scheduled injection time.
    pub fn at_nanos(&self) -> u64 {
        self.at_nanos
    }

    /// Canonical fault rendering ([`vh_gremlin::FaultKind::canonical`]).
    pub fn fault(&self) -> &str {
        &self.fault
    }

    pub fn offered_at(&self) -> Option<u64> {
        self.offered_at
    }

    pub fn armed_at(&self) -> Option<u64> {
        self.armed_at
    }

    pub fn injected_at(&self) -> Option<u64> {
        self.injected_at
    }

    pub fn manifested_at(&self) -> Option<u64> {
        self.manifested_at
    }

    pub fn recovered_at(&self) -> Option<u64> {
        self.recovered_at
    }

    /// One deterministic line for versioned observable renderings
    /// (doctor `vh-doctor-observable-v3`). Absent stages render as `-`.
    pub fn canonical(&self) -> String {
        fn stage(v: Option<u64>) -> String {
            v.map_or_else(|| "-".to_string(), |t| t.to_string())
        }
        let mut s = String::new();
        let _ = write!(
            s,
            "at={} fault={} offered={} armed={} injected={} manifested={} recovered={}",
            self.at_nanos,
            self.fault,
            stage(self.offered_at),
            stage(self.armed_at),
            stage(self.injected_at),
            stage(self.manifested_at),
            stage(self.recovered_at),
        );
        s
    }

    pub(crate) fn offer(&mut self, now: u64) {
        assert!(self.offered_at.is_none(), "injection offered twice");
        self.offered_at = Some(now);
    }

    pub(crate) fn arm(&mut self, now: u64) {
        assert!(self.offered_at.is_some(), "armed before offered");
        assert!(self.armed_at.is_none(), "injection armed twice");
        self.armed_at = Some(now);
    }

    /// First interception only: later interceptions by the same armed
    /// window (a partition dropping several messages) keep the first
    /// injection time; each interception is separately trace-recorded.
    pub(crate) fn inject(&mut self, now: u64) {
        assert!(self.armed_at.is_some(), "injected before armed");
        if self.injected_at.is_none() {
            self.injected_at = Some(now);
        }
    }

    pub(crate) fn manifest(&mut self, now: u64) {
        assert!(self.injected_at.is_some(), "manifested before injected");
        if self.manifested_at.is_none() {
            self.manifested_at = Some(now);
        }
    }

    /// Recovery requires the fault to have been armed, not necessarily
    /// injected: a partition that intercepted nothing still heals.
    pub(crate) fn recover(&mut self, now: u64) {
        assert!(self.armed_at.is_some(), "recovered before armed");
        if self.recovered_at.is_none() {
            self.recovered_at = Some(now);
        }
    }

    pub(crate) fn is_injected(&self) -> bool {
        self.injected_at.is_some()
    }

    pub(crate) fn is_recovered(&self) -> bool {
        self.recovered_at.is_some()
    }
}

/// The complete runner-owned fault-lifecycle ledger of one universe that
/// constructed the sim runtime: one [`InjectionOutcome`] per planned
/// injection, in plan (time-canonical) order. Part of the observable
/// [`crate::UniverseResult`] and its equality.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimeEvidence {
    injections: Vec<InjectionOutcome>,
}

impl RuntimeEvidence {
    pub(crate) fn new(injections: Vec<InjectionOutcome>) -> Self {
        Self { injections }
    }

    /// Per-injection measured lifecycles, in plan order.
    pub fn injections(&self) -> &[InjectionOutcome] {
        &self.injections
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ladder_advances_monotonically_and_renders_absent_stages() {
        let mut o = InjectionOutcome::new(50, "disk_write_fail".to_string());
        assert_eq!(
            o.canonical(),
            "at=50 fault=disk_write_fail offered=- armed=- injected=- manifested=- recovered=-"
        );
        o.offer(50);
        o.arm(50);
        o.inject(60);
        o.inject(70); // later interception keeps the first time
        o.manifest(60);
        o.recover(80);
        assert_eq!(o.injected_at(), Some(60));
        assert_eq!(
            o.canonical(),
            "at=50 fault=disk_write_fail offered=50 armed=50 injected=60 manifested=60 recovered=80"
        );
    }

    #[test]
    #[should_panic(expected = "injected before armed")]
    fn out_of_order_injection_is_a_kernel_bug() {
        let mut o = InjectionOutcome::new(0, "disk_write_fail".to_string());
        o.offer(0);
        o.inject(1);
    }

    #[test]
    #[should_panic(expected = "manifested before injected")]
    fn manifestation_requires_injection() {
        let mut o = InjectionOutcome::new(0, "fsync_lie".to_string());
        o.offer(0);
        o.arm(0);
        o.manifest(1);
    }

    #[test]
    fn recovery_without_injection_is_legal_for_healed_idle_faults() {
        // A partition that intercepted no traffic still heals.
        let mut o = InjectionOutcome::new(10, "network_partition:100".to_string());
        o.offer(10);
        o.arm(10);
        o.recover(200);
        assert!(o.recovered_at().is_some());
        assert!(o.injected_at().is_none());
    }
}
