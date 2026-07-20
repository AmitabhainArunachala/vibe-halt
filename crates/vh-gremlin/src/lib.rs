//! vh-gremlin — the fault model.
//!
//! A `FaultPlan` is generated deterministically from a dedicated PRNG stream
//! and is part of the universe's identity: same seed → same gremlins. Plans
//! are plain data so the shrinker (Phase 2) can delete injections and replay.

#![forbid(unsafe_code)]

use vh_core::Xoshiro256pp;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FaultKind {
    /// Process crash + restart; volatile state is lost.
    CrashRestart,
    /// A message/IO completion is delayed by this much virtual time.
    NetworkDelay { delay_nanos: u64 },
    /// Link down for a duration of virtual time.
    NetworkPartition { duration_nanos: u64 },
    /// A single write is reported failed (caller sees an error).
    DiskWriteFail,
    /// The component's local clock reads skewed by this much.
    ClockSkew { skew_nanos: u64 },
}

impl FaultKind {
    pub fn label(&self) -> &'static str {
        match self {
            FaultKind::CrashRestart => "crash_restart",
            FaultKind::NetworkDelay { .. } => "network_delay",
            FaultKind::NetworkPartition { .. } => "network_partition",
            FaultKind::DiskWriteFail => "disk_write_fail",
            FaultKind::ClockSkew { .. } => "clock_skew",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FaultInjection {
    pub at_nanos: u64,
    pub fault: FaultKind,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct FaultPlan {
    /// Sorted by `at_nanos`; ties keep generation order (stable sort).
    pub injections: Vec<FaultInjection>,
}

impl FaultPlan {
    /// Generate `count` injections uniformly over `[0, horizon_nanos)`.
    ///
    /// v0 draws fault kinds uniformly; Phase 1 replaces this with targeted
    /// scheduling biased toward state-transition edges and novel coverage.
    pub fn generate(rng: &mut Xoshiro256pp, horizon_nanos: u64, count: usize) -> Self {
        let horizon = horizon_nanos.max(1);
        let mut injections: Vec<FaultInjection> = (0..count)
            .map(|_| {
                let at_nanos = rng.next_below(horizon);
                let fault = match rng.next_below(5) {
                    0 => FaultKind::CrashRestart,
                    1 => FaultKind::NetworkDelay {
                        delay_nanos: rng.next_below(horizon / 10 + 1),
                    },
                    2 => FaultKind::NetworkPartition {
                        duration_nanos: rng.next_below(horizon / 4 + 1),
                    },
                    3 => FaultKind::DiskWriteFail,
                    _ => FaultKind::ClockSkew {
                        skew_nanos: rng.next_below(horizon / 20 + 1),
                    },
                };
                FaultInjection { at_nanos, fault }
            })
            .collect();
        injections.sort_by_key(|i| i.at_nanos);
        Self { injections }
    }

    /// Injections due at or before `now`, starting from index `cursor`.
    /// Returns the new cursor. Callers drain in virtual-time order.
    pub fn due(&self, cursor: usize, now_nanos: u64) -> (usize, &[FaultInjection]) {
        let start = cursor;
        let mut end = cursor;
        while end < self.injections.len() && self.injections[end].at_nanos <= now_nanos {
            end += 1;
        }
        (end, &self.injections[start..end])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generation_is_deterministic() {
        let mut a = Xoshiro256pp::from_seed(99);
        let mut b = Xoshiro256pp::from_seed(99);
        assert_eq!(
            FaultPlan::generate(&mut a, 1_000_000, 8),
            FaultPlan::generate(&mut b, 1_000_000, 8)
        );
    }

    #[test]
    fn injections_are_time_sorted() {
        let mut r = Xoshiro256pp::from_seed(5);
        let plan = FaultPlan::generate(&mut r, 1_000_000, 32);
        for w in plan.injections.windows(2) {
            assert!(w[0].at_nanos <= w[1].at_nanos);
        }
    }

    #[test]
    fn due_drains_in_order() {
        let mut r = Xoshiro256pp::from_seed(5);
        let plan = FaultPlan::generate(&mut r, 1_000, 10);
        let (cursor, first) = plan.due(0, 500);
        for inj in first {
            assert!(inj.at_nanos <= 500);
        }
        let (end, rest) = plan.due(cursor, 1_000);
        assert_eq!(end, plan.injections.len());
        for inj in rest {
            assert!(inj.at_nanos > 500 || first.is_empty());
        }
    }
}
