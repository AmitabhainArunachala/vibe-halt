//! vh-core — the vibe-halt determinism kernel.
//!
//! Everything in this crate MUST be deterministic: no wall-clock time, no OS
//! randomness, no hash-order iteration, no threads, no I/O. The deny-list is
//! enforced mechanically by `scripts/check_determinism_denylist.py` (CI gate).

#![forbid(unsafe_code)]

pub mod clock;
pub mod rng;
pub mod sched;
pub mod seed;

pub use clock::{VirtualClock, VirtualTime};
pub use rng::Xoshiro256pp;
pub use sched::{Scheduler, SchedulerDecision};
pub use seed::SeedTree;
