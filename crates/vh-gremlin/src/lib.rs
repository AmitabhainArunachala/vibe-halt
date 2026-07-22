//! vh-gremlin — the fault model.
//!
//! A `FaultPlan` is generated deterministically from a dedicated PRNG stream
//! and is part of the universe's identity: same seed → same gremlins. Plans
//! are plain data so the shrinker (Phase 2) can delete injections and replay.

#![forbid(unsafe_code)]

use vh_core::Xoshiro256pp;

const FNV64_OFFSET: u64 = 0xcbf2_9ce4_8422_2325;
const FNV64_PRIME: u64 = 0x0000_0100_0000_01b3;

fn fnv1a64(bytes: &[u8]) -> u64 {
    let mut h = FNV64_OFFSET;
    for &b in bytes {
        h ^= b as u64;
        h = h.wrapping_mul(FNV64_PRIME);
    }
    h
}

/// Fault-plan palette selection. v0 is the frozen/default generator;
/// swarm is opt-in until the corpus bakeoff proves it beats v0.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FaultPalette {
    V0,
    Swarm,
}

impl FaultPalette {
    pub fn name(self) -> &'static str {
        match self {
            FaultPalette::V0 => "v0",
            FaultPalette::Swarm => "swarm",
        }
    }
}

#[derive(Debug, Clone)]
struct PaletteMask {
    len: usize,
    enabled: [bool; MAX_PALETTE_KIND_COUNT],
    weights: [u64; MAX_PALETTE_KIND_COUNT],
    total: u64,
}

const SWARM_KIND_COUNT: usize = 5;
const MAX_PALETTE_KIND_COUNT: usize = 16;
const PALETTE_STREAM_TAG: &[u8] = b"palette";

impl PaletteMask {
    fn from_universe_seed(universe_seed: u64, kind_count: usize) -> Self {
        assert!(
            kind_count > 0 && kind_count <= MAX_PALETTE_KIND_COUNT,
            "palette kind count must be in 1..=16"
        );
        let mut rng = Xoshiro256pp::from_seed(universe_seed ^ fnv1a64(PALETTE_STREAM_TAG));
        let mut enabled = [false; MAX_PALETTE_KIND_COUNT];
        let mut weights = [0u64; MAX_PALETTE_KIND_COUNT];
        let mut total = 0u64;

        // TigerBeetle random_enum_weights idiom (tigerbeetle@97c7a8ef38
        // src/testing/fuzz.zig): per-run randomized enum mask plus wild
        // weights, so a campaign explores different fault-family emphasis
        // without importing a dependency or mutating the frozen v0 stream.
        for i in 0..kind_count {
            let include = rng.next_bool(0.65);
            enabled[i] = include;
            if include {
                // Exponential-ish integer buckets: mostly modest weights,
                // occasional huge emphasis, deterministic per universe.
                let shift = rng.next_below(8) as u32;
                let jitter = 1 + rng.next_below(1u64 << shift);
                weights[i] = (1u64 << shift) + jitter;
                total += weights[i];
            }
        }
        if total == 0 {
            let idx = rng.next_below(kind_count as u64) as usize;
            enabled[idx] = true;
            weights[idx] = 1;
            total = 1;
        }
        Self {
            len: kind_count,
            enabled,
            weights,
            total,
        }
    }

    fn choose(&self, rng: &mut Xoshiro256pp) -> usize {
        let mut pick = rng.next_below(self.total);
        for i in 0..self.len {
            let weight = self.weights[i];
            if !self.enabled[i] {
                continue;
            }
            if pick < weight {
                return i;
            }
            pick -= weight;
        }
        unreachable!("palette mask has positive total weight")
    }
}

#[derive(Debug, Clone)]
pub struct PaletteChooser {
    palette: FaultPalette,
    swarm_mask: Option<PaletteMask>,
    kind_count: usize,
}

impl PaletteChooser {
    pub fn new(palette: FaultPalette, universe_seed: u64, kind_count: usize) -> Self {
        assert!(
            kind_count > 0 && kind_count <= MAX_PALETTE_KIND_COUNT,
            "palette kind count must be in 1..=16"
        );
        let swarm_mask = match palette {
            FaultPalette::V0 => None,
            FaultPalette::Swarm => Some(PaletteMask::from_universe_seed(universe_seed, kind_count)),
        };
        Self {
            palette,
            swarm_mask,
            kind_count,
        }
    }

    pub fn choose(&self, rng: &mut Xoshiro256pp) -> u64 {
        match (self.palette, &self.swarm_mask) {
            (FaultPalette::V0, None) => rng.next_below(self.kind_count as u64),
            (FaultPalette::Swarm, Some(mask)) => mask.choose(rng) as u64,
            _ => unreachable!("palette chooser internal state is inconsistent"),
        }
    }
}

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
    /// A single write persists only a prefix of its record while the
    /// writer still sees success — the tear surfaces through later reads
    /// or recovery, never through the write's return value.
    TornWrite,
    /// A single fsync reports success without making anything durable.
    FsyncLie,
    /// The next message is delivered twice.
    NetworkDuplicate,
    /// The next message is held and delivered after the message that
    /// follows it (a pairwise reorder; if no message follows, no swap
    /// occurs and the fault honestly never advances past Armed).
    NetworkReorder,
}

impl FaultKind {
    pub fn label(&self) -> &'static str {
        match self {
            FaultKind::CrashRestart => "crash_restart",
            FaultKind::NetworkDelay { .. } => "network_delay",
            FaultKind::NetworkPartition { .. } => "network_partition",
            FaultKind::DiskWriteFail => "disk_write_fail",
            FaultKind::ClockSkew { .. } => "clock_skew",
            FaultKind::TornWrite => "torn_write",
            FaultKind::FsyncLie => "fsync_lie",
            FaultKind::NetworkDuplicate => "network_duplicate",
            FaultKind::NetworkReorder => "network_reorder",
        }
    }

    /// Canonical rendering — label plus every parameter — for versioned
    /// evidence digests (`vh-fault-plan-v1` in vh-multiverse). Changing
    /// an EXISTING rendering is a digest schema bump, never a refactor.
    /// ADDING a variant is additive: new canonical strings appear, every
    /// previously recorded digest is computed over unchanged renderings
    /// and remains valid (2026-07-21, Phase-1 runtime faults).
    pub fn canonical(&self) -> String {
        match self {
            FaultKind::CrashRestart => "crash_restart".to_string(),
            FaultKind::NetworkDelay { delay_nanos } => format!("network_delay:{delay_nanos}"),
            FaultKind::NetworkPartition { duration_nanos } => {
                format!("network_partition:{duration_nanos}")
            }
            FaultKind::DiskWriteFail => "disk_write_fail".to_string(),
            FaultKind::ClockSkew { skew_nanos } => format!("clock_skew:{skew_nanos}"),
            FaultKind::TornWrite => "torn_write".to_string(),
            FaultKind::FsyncLie => "fsync_lie".to_string(),
            FaultKind::NetworkDuplicate => "network_duplicate".to_string(),
            FaultKind::NetworkReorder => "network_reorder".to_string(),
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
    /// This is the frozen v0 generator and remains the default.
    pub fn generate(rng: &mut Xoshiro256pp, horizon_nanos: u64, count: usize) -> Self {
        Self::generate_with_palette(rng, horizon_nanos, count, FaultPalette::V0, 0)
    }

    /// Generate `count` injections with an explicit palette. `universe_seed`
    /// is ignored by v0; swarm derives its mask from
    /// `universe_seed ^ fnv1a64("palette")` without perturbing the caller's
    /// existing gremlin stream. The old [`FaultPlan::generate`] stays
    /// bit-identical.
    pub fn generate_with_palette(
        rng: &mut Xoshiro256pp,
        horizon_nanos: u64,
        count: usize,
        palette: FaultPalette,
        universe_seed: u64,
    ) -> Self {
        match palette {
            FaultPalette::V0 => Self::generate_v0(rng, horizon_nanos, count),
            FaultPalette::Swarm => Self::generate_swarm(rng, horizon_nanos, count, universe_seed),
        }
    }

    fn generate_v0(rng: &mut Xoshiro256pp, horizon_nanos: u64, count: usize) -> Self {
        let horizon = horizon_nanos.max(1);
        let injections: Vec<FaultInjection> = (0..count)
            .map(|_| {
                let at_nanos = rng.next_below(horizon);
                let kind = rng.next_below(5);
                let fault = draw_v0_fault(rng, horizon, kind);
                FaultInjection { at_nanos, fault }
            })
            .collect();
        Self::new(injections)
    }

    fn generate_swarm(
        rng: &mut Xoshiro256pp,
        horizon_nanos: u64,
        count: usize,
        universe_seed: u64,
    ) -> Self {
        let horizon = horizon_nanos.max(1);
        let chooser = PaletteChooser::new(FaultPalette::Swarm, universe_seed, SWARM_KIND_COUNT);
        let injections: Vec<FaultInjection> = (0..count)
            .map(|_| {
                let at_nanos = rng.next_below(horizon);
                let kind = chooser.choose(rng);
                let fault = draw_v0_fault(rng, horizon, kind);
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

fn draw_v0_fault(rng: &mut Xoshiro256pp, horizon: u64, kind: u64) -> FaultKind {
    match kind {
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
    fn explicit_v0_palette_is_bit_identical_to_legacy_generate() {
        let mut a = Xoshiro256pp::from_seed(0xD1CE);
        let mut b = Xoshiro256pp::from_seed(0xD1CE);
        assert_eq!(
            FaultPlan::generate(&mut a, 1_000_000, 64),
            FaultPlan::generate_with_palette(&mut b, 1_000_000, 64, FaultPalette::V0, 0xfeed_beef,)
        );
    }

    #[test]
    fn swarm_palette_is_deterministic_and_universe_specific() {
        let mut a = Xoshiro256pp::from_seed(99);
        let mut b = Xoshiro256pp::from_seed(99);
        let mut c = Xoshiro256pp::from_seed(99);
        let left =
            FaultPlan::generate_with_palette(&mut a, 1_000_000, 32, FaultPalette::Swarm, 123);
        let same =
            FaultPlan::generate_with_palette(&mut b, 1_000_000, 32, FaultPalette::Swarm, 123);
        let other_universe =
            FaultPlan::generate_with_palette(&mut c, 1_000_000, 32, FaultPalette::Swarm, 124);
        assert_eq!(left, same);
        assert_ne!(left, other_universe);
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

    /// Frozen-surface regression (2026-07-21): `generate` is part of the
    /// frozen demo path — the demo workload's plans, and therefore the
    /// frozen doctor trace identity, depend on this exact palette and
    /// draw sequence. The Phase-1 fault kinds (torn write, fsync lie,
    /// duplicate, reorder) must NEVER appear from `generate`; workloads
    /// that want them construct plans explicitly via `FaultPlan::new`.
    #[test]
    fn frozen_generate_palette_excludes_phase1_kinds() {
        let mut r = Xoshiro256pp::from_seed(0xD1CE);
        let plan = FaultPlan::generate(&mut r, 1_000_000, 512);
        for inj in plan.injections() {
            assert!(
                matches!(
                    inj.fault,
                    FaultKind::CrashRestart
                        | FaultKind::NetworkDelay { .. }
                        | FaultKind::NetworkPartition { .. }
                        | FaultKind::DiskWriteFail
                        | FaultKind::ClockSkew { .. }
                ),
                "generate emitted a non-v0 fault kind: {:?}",
                inj.fault
            );
        }
    }

    /// The canonical renderings of the Phase-1 additions are frozen from
    /// birth: these strings enter `vh-fault-plan-v1` digests.
    #[test]
    fn phase1_canonical_renderings_are_stable() {
        assert_eq!(FaultKind::TornWrite.canonical(), "torn_write");
        assert_eq!(FaultKind::FsyncLie.canonical(), "fsync_lie");
        assert_eq!(FaultKind::NetworkDuplicate.canonical(), "network_duplicate");
        assert_eq!(FaultKind::NetworkReorder.canonical(), "network_reorder");
        assert_eq!(FaultKind::TornWrite.label(), "torn_write");
        assert_eq!(FaultKind::FsyncLie.label(), "fsync_lie");
        assert_eq!(FaultKind::NetworkDuplicate.label(), "network_duplicate");
        assert_eq!(FaultKind::NetworkReorder.label(), "network_reorder");
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
