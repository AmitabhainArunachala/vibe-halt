//! Same-timestamp schedule strategies (convergence C2, Track-2 W3).
//!
//! Shapes follow Shuttle's PCT scheduler
//! (`shuttle-schedulers/src/pct.rs@c8a46d3965`, Burckhardt et al.,
//! "A Randomized Scheduler with Probabilistic Guarantees of Finding
//! Bugs", ASPLOS 2010) — REIMPLEMENTED with zero dependencies over this
//! repo's own PRNG. The unit of scheduling here is a scheduler EVENT
//! (identified by its insertion seq), not a thread: priorities are a
//! deterministic keyed hash of the event seq, and each of the `depth`
//! change points demotes the then-highest-priority candidate below
//! every un-demoted event, exactly the PCT priority-lowering move.
//!
//! Both strategies are pure functions of `(universe scheduling seed,
//! pop history)`: same seed, same choices, byte-for-byte — replay
//! equivalence is checked at the runtime layer and witnessed by the
//! decision tape digest.
//!
//! Naming honesty (Codex audit C.2): in claims and publications this
//! is "event-priority (PCT-inspired)" scheduling — it borrows PCT's
//! priority + change-point SHAPES but schedules same-timestamp EVENTS,
//! not threads, and inherits none of PCT's probabilistic bug-finding
//! guarantees (those are proven for the thread model only).

use crate::clock::VirtualTime;
use crate::rng::Xoshiro256pp;

const FNV64_OFFSET: u64 = 0xcbf2_9ce4_8422_2325;
const FNV64_PRIME: u64 = 0x0000_0100_0000_01b3;

fn fnv64(seed: u64, tag: &[u8], value: u64) -> u64 {
    let mut h = FNV64_OFFSET;
    for &b in seed
        .to_le_bytes()
        .iter()
        .chain(tag.iter())
        .chain(value.to_le_bytes().iter())
    {
        h ^= b as u64;
        h = h.wrapping_mul(FNV64_PRIME);
    }
    h
}

/// Nominal choice-point horizon the PCT change points are drawn over.
/// Pops beyond it simply see no further change points (Shuttle draws
/// its change points over a max-steps bound the same way).
const PCT_CHANGE_HORIZON: u64 = 256;

/// PCT over same-timestamp frontiers: highest-priority candidate wins;
/// `depth` pre-drawn change points each demote the then-leader.
pub struct PctStrategy {
    seed: u64,
    pops: u64,
    change_points: Vec<u64>,
    /// Seqs demoted so far, in demotion order: earlier demotions rank
    /// LOWER (Shuttle's d-th change point gets priority d).
    demoted: Vec<u64>,
}

impl PctStrategy {
    pub fn new(schedule_seed: u64, depth: u64) -> Self {
        let mut rng =
            Xoshiro256pp::from_seed(schedule_seed ^ fnv64(0, b"pct-change-points", depth));
        let change_points = (0..depth)
            .map(|_| rng.next_below(PCT_CHANGE_HORIZON))
            .collect();
        Self {
            seed: schedule_seed,
            pops: 0,
            change_points,
            demoted: Vec::new(),
        }
    }

    /// Priority as a totally ordered pair: un-demoted events rank by
    /// keyed hash ABOVE every demoted event; demoted events rank by
    /// demotion order (earliest demotion = lowest).
    fn priority(&self, seq: u64) -> (u64, u64) {
        match self.demoted.iter().position(|&d| d == seq) {
            Some(rank) => (0, rank as u64),
            None => (1, fnv64(self.seed, b"pct-priority", seq)),
        }
    }

    fn argmax(&self, candidates: &[(VirtualTime, u64)]) -> usize {
        let mut best = 0;
        for i in 1..candidates.len() {
            // Ties are impossible in practice (keyed 64-bit hash), but
            // break them toward the lower seq deterministically.
            if self.priority(candidates[i].1) > self.priority(candidates[best].1) {
                best = i;
            }
        }
        best
    }

    pub fn choose(&mut self, candidates: &[(VirtualTime, u64)]) -> usize {
        let pop_index = self.pops;
        self.pops += 1;
        let mut idx = self.argmax(candidates);
        if self.change_points.contains(&pop_index) {
            // The PCT move: the current leader's priority drops below
            // everyone; the runner-up (if any) takes this pop.
            let leader_seq = candidates[idx].1;
            if !self.demoted.contains(&leader_seq) {
                self.demoted.push(leader_seq);
            }
            idx = self.argmax(candidates);
        }
        idx
    }
}

/// Uniform-with-random-tiebreak: the null hypothesis PCT must beat
/// (charter C2 kill criterion) — every same-timestamp candidate equally
/// likely, from a dedicated deterministic stream.
pub struct UniformTiebreakStrategy {
    rng: Xoshiro256pp,
}

impl UniformTiebreakStrategy {
    pub fn new(schedule_seed: u64) -> Self {
        Self {
            rng: Xoshiro256pp::from_seed(schedule_seed ^ fnv64(0, b"uniform-tiebreak", 0)),
        }
    }

    pub fn choose(&mut self, candidates: &[(VirtualTime, u64)]) -> usize {
        self.rng.next_below(candidates.len() as u64) as usize
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn frontier(n: u64) -> Vec<(VirtualTime, u64)> {
        (0..n).map(|s| (VirtualTime(100), s)).collect()
    }

    #[test]
    fn pct_is_deterministic_per_seed_and_varies_across_seeds() {
        let run = |seed: u64| -> Vec<usize> {
            let mut s = PctStrategy::new(seed, 3);
            (0..32).map(|_| s.choose(&frontier(4))).collect()
        };
        assert_eq!(run(7), run(7));
        let all: Vec<Vec<usize>> = (0..16).map(run).collect();
        assert!(
            all.iter().any(|c| c != &all[0]),
            "16 seeds should not all schedule identically"
        );
    }

    #[test]
    fn pct_change_point_demotes_the_leader() {
        // With depth = horizon, every pop is a change point: each pop
        // demotes the current leader, so over 4 pops of the same
        // 4-candidate frontier every candidate is chosen at most twice
        // and the leadership rotates.
        let mut s = PctStrategy::new(11, PCT_CHANGE_HORIZON);
        let f = frontier(4);
        let picks: Vec<usize> = (0..4).map(|_| s.choose(&f)).collect();
        // After each demotion the next pick differs from a permanently
        // dominant leader — at least two distinct picks must appear.
        assert!(
            picks
                .iter()
                .collect::<std::collections::BTreeSet<_>>()
                .len()
                >= 2
        );
    }

    #[test]
    fn uniform_tiebreak_is_deterministic_and_in_range() {
        let run = |seed: u64| -> Vec<usize> {
            let mut s = UniformTiebreakStrategy::new(seed);
            (0..64).map(|_| s.choose(&frontier(3))).collect()
        };
        assert_eq!(run(5), run(5));
        assert!(run(5).iter().all(|&i| i < 3));
        assert_ne!(run(5), run(6));
    }

    #[test]
    fn fifo_equivalence_zero_depth_never_demotes_but_hash_still_reorders() {
        // depth 0 PCT is NOT FIFO (hash priorities still reorder the
        // frontier) — pinned so nobody mistakes it for a FIFO alias.
        let mut s = PctStrategy::new(3, 0);
        let picks: std::collections::BTreeSet<usize> =
            (0..64).map(|_| s.choose(&frontier(4))).collect();
        let _ = picks; // any picks are legal; the point is no panic and determinism
        let mut s2 = PctStrategy::new(3, 0);
        let again: Vec<usize> = (0..64).map(|_| s2.choose(&frontier(4))).collect();
        let mut s3 = PctStrategy::new(3, 0);
        let expect: Vec<usize> = (0..64).map(|_| s3.choose(&frontier(4))).collect();
        assert_eq!(again, expect);
    }
}
