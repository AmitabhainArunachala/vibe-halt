//! The seed tree: `root_seed → universe_id → named component streams`.
//!
//! The load-bearing property is stream independence by NAME: adding a new
//! named stream to a workload must not perturb the values drawn by any
//! existing stream. This is what keeps old failing seeds replayable across
//! versions of a workload.
//!
//! Qualification (PR #1 review): independence holds modulo derived-key
//! collisions — stream keys come from FNV-1a 64 over the name, so two
//! distinct names that collide under FNV-1a 64 receive the same stream.
//! Under a uniform birthday approximation that probability is ~2.7e-8 at
//! one million names; realistic workloads use tens of names. FNV is
//! non-cryptographic: adversarially *chosen* colliding names are feasible,
//! so stream names must come from the workload author, never from
//! untrusted input.

use crate::rng::{splitmix64, Xoshiro256pp};

const FNV64_OFFSET: u64 = 0xcbf2_9ce4_8422_2325;
const FNV64_PRIME: u64 = 0x0000_0100_0000_01b3;

/// FNV-1a 64-bit hash (names → stream keys).
pub fn fnv1a64(bytes: &[u8]) -> u64 {
    let mut h = FNV64_OFFSET;
    for &b in bytes {
        h ^= b as u64;
        h = h.wrapping_mul(FNV64_PRIME);
    }
    h
}

#[derive(Debug, Clone, Copy)]
pub struct SeedTree {
    root: u64,
}

impl SeedTree {
    pub fn new(root: u64) -> Self {
        Self { root }
    }

    pub fn root(&self) -> u64 {
        self.root
    }

    /// Deterministic per-universe seed derived from the root.
    pub fn universe_seed(&self, universe_id: u64) -> u64 {
        let mut s = self
            .root
            .wrapping_add(0x9E37_79B9_7F4A_7C15u64.wrapping_mul(universe_id.wrapping_add(1)));
        splitmix64(&mut s)
    }

    /// A named, independent PRNG stream inside one universe.
    pub fn stream(&self, universe_id: u64, name: &str) -> Xoshiro256pp {
        let mut s = self.universe_seed(universe_id) ^ fnv1a64(name.as_bytes());
        Xoshiro256pp::from_seed(splitmix64(&mut s))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn universes_get_distinct_seeds() {
        let t = SeedTree::new(42);
        assert_ne!(t.universe_seed(0), t.universe_seed(1));
        assert_ne!(t.universe_seed(1), t.universe_seed(2));
    }

    #[test]
    fn streams_are_reproducible() {
        let t = SeedTree::new(42);
        let mut a = t.stream(7, "ops");
        let mut b = t.stream(7, "ops");
        for _ in 0..100 {
            assert_eq!(a.next_u64(), b.next_u64());
        }
    }

    #[test]
    fn streams_are_independent_by_name() {
        let t = SeedTree::new(42);
        // Drawing from "gremlin" must not change what "ops" yields.
        let ops_alone: Vec<u64> = {
            let mut ops = t.stream(7, "ops");
            (0..50).map(|_| ops.next_u64()).collect()
        };
        let ops_with_sibling: Vec<u64> = {
            let mut gremlin = t.stream(7, "gremlin");
            let _ = gremlin.next_u64();
            let mut ops = t.stream(7, "ops");
            (0..50).map(|_| ops.next_u64()).collect()
        };
        assert_eq!(ops_alone, ops_with_sibling);
    }

    #[test]
    fn same_name_different_universe_differs() {
        let t = SeedTree::new(42);
        let mut a = t.stream(1, "ops");
        let mut b = t.stream(2, "ops");
        assert_ne!(a.next_u64(), b.next_u64());
    }

    /// Correlation smoke (PR #1 review GAP): plain inequality would pass
    /// even for streams related by `x ^ 1`. The xor-Hamming weight between
    /// adjacent-universe streams must sit near the 50% expected for
    /// independent uniform bits. Deterministic: fixed seed, fixed bounds
    /// (mean 2048 over 64 words; bounds are ±6σ wide, σ = 32).
    #[test]
    fn adjacent_universe_streams_are_uncorrelated_smoke() {
        let t = SeedTree::new(42);
        for (ua, ub) in [(0u64, 1u64), (1, 2), (7, 8)] {
            let mut a = t.stream(ua, "ops");
            let mut b = t.stream(ub, "ops");
            let hamming: u32 = (0..64)
                .map(|_| (a.next_u64() ^ b.next_u64()).count_ones())
                .sum();
            assert!(
                (1848..=2248).contains(&hamming),
                "universes {ua}/{ub}: xor-Hamming {hamming} outside [1848, 2248]"
            );
        }
    }
}
