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

    /// Canonical rendering — label plus every parameter — for versioned
    /// evidence digests (`vh-fault-plan-v1` in vh-multiverse). Changing
    /// this output is a digest schema bump, never a refactor.
    pub fn canonical(&self) -> String {
        match self {
            FaultKind::CrashRestart => "crash_restart".to_string(),
            FaultKind::NetworkDelay { delay_nanos } => format!("network_delay:{delay_nanos}"),
            FaultKind::NetworkPartition { duration_nanos } => {
                format!("network_partition:{duration_nanos}")
            }
            FaultKind::DiskWriteFail => "disk_write_fail".to_string(),
            FaultKind::ClockSkew { skew_nanos } => format!("clock_skew:{skew_nanos}"),
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
    /// Sorted by `at_nanos`; ties keep construction order (stable sort).
    /// Private: `due()` assumes time order, so a publicly writable vector
    /// let safe code construct plans that silently skipped injections
    /// (PR #1 hardening-loop-2 GAP). Construction canonicalizes instead.
    injections: Vec<FaultInjection>,
}

impl FaultPlan {
    /// Build a plan from arbitrary injections. Construction is the
    /// canonicalization boundary: injections are stable-sorted by
    /// `at_nanos` (ties keep the caller's order), so every plan `due()`
    /// ever sees is time-ordered by construction — an unsorted input can
    /// no longer smuggle injections past the cursor.
    pub fn new(mut injections: Vec<FaultInjection>) -> Self {
        injections.sort_by_key(|i| i.at_nanos);
        Self { injections }
    }

    /// The canonical (time-ordered) injections, read-only.
    pub fn injections(&self) -> &[FaultInjection] {
        &self.injections
    }

    /// Generate `count` injections uniformly over `[0, horizon_nanos)`.
    ///
    /// v0 draws fault kinds uniformly; Phase 1 replaces this with targeted
    /// scheduling biased toward state-transition edges and novel coverage.
    pub fn generate(rng: &mut Xoshiro256pp, horizon_nanos: u64, count: usize) -> Self {
        let horizon = horizon_nanos.max(1);
        let injections: Vec<FaultInjection> = (0..count)
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
        Self::new(injections)
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
        for w in plan.injections().windows(2) {
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
        assert_eq!(end, plan.injections().len());
        for inj in rest {
            assert!(inj.at_nanos > 500 || first.is_empty());
        }
    }

    /// Negative regression (hardening-loop-2 GAP): before canonical
    /// construction, a publicly built unsorted plan made `due()` skip the
    /// out-of-order injection entirely — the fault never fired and the run
    /// was blessed with a weaker plan than reported.
    #[test]
    fn unsorted_construction_is_canonicalized_so_due_misses_nothing() {
        let early = FaultInjection {
            at_nanos: 10,
            fault: FaultKind::DiskWriteFail,
        };
        let late = FaultInjection {
            at_nanos: 900,
            fault: FaultKind::CrashRestart,
        };
        // Caller supplies out-of-order injections.
        let plan = FaultPlan::new(vec![late.clone(), early.clone()]);
        assert_eq!(plan.injections(), &[early.clone(), late.clone()]);

        // Drain in two steps: the early injection MUST surface in the
        // first window (the pre-repair plan skipped it forever).
        let (cursor, first) = plan.due(0, 500);
        assert_eq!(first, &[early]);
        let (end, rest) = plan.due(cursor, 1_000);
        assert_eq!(rest, &[late]);
        assert_eq!(end, plan.injections().len());
    }

    /// Ties keep caller order (stable sort), deterministically.
    #[test]
    fn tied_injections_keep_caller_order() {
        let a = FaultInjection {
            at_nanos: 5,
            fault: FaultKind::DiskWriteFail,
        };
        let b = FaultInjection {
            at_nanos: 5,
            fault: FaultKind::CrashRestart,
        };
        let plan = FaultPlan::new(vec![a.clone(), b.clone()]);
        assert_eq!(plan.injections(), &[a, b]);
    }
}
