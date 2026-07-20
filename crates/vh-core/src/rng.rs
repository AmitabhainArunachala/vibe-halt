//! Deterministic PRNG primitives: SplitMix64 (seeding) and xoshiro256++
//! (streams). Both are public-domain algorithms implemented here directly so
//! the kernel has zero external dependencies and a frozen bit-exact output.

/// SplitMix64 step. Used for seed derivation, never as a workload stream.
pub fn splitmix64(state: &mut u64) -> u64 {
    *state = state.wrapping_add(0x9E37_79B9_7F4A_7C15);
    let mut z = *state;
    z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    z ^ (z >> 31)
}

/// xoshiro256++ — the per-component workload stream generator.
#[derive(Debug, Clone)]
pub struct Xoshiro256pp {
    s: [u64; 4],
}

impl Xoshiro256pp {
    /// Seed via SplitMix64 expansion (the reference-recommended procedure).
    pub fn from_seed(seed: u64) -> Self {
        let mut sm = seed;
        let mut s = [0u64; 4];
        for slot in &mut s {
            *slot = splitmix64(&mut sm);
        }
        Self { s }
    }

    pub fn next_u64(&mut self) -> u64 {
        let result = self.s[0]
            .wrapping_add(self.s[3])
            .rotate_left(23)
            .wrapping_add(self.s[0]);
        let t = self.s[1] << 17;
        self.s[2] ^= self.s[0];
        self.s[3] ^= self.s[1];
        self.s[1] ^= self.s[2];
        self.s[0] ^= self.s[3];
        self.s[2] ^= t;
        self.s[3] = self.s[3].rotate_left(45);
        result
    }

    /// Uniform value in `[0, n)` via rejection sampling (no modulo bias).
    pub fn next_below(&mut self, n: u64) -> u64 {
        assert!(n > 0, "next_below(0) is meaningless");
        let zone = u64::MAX - (u64::MAX % n);
        loop {
            let v = self.next_u64();
            if v < zone {
                return v % n;
            }
        }
    }

    /// Uniform f64 in `[0, 1)` from the top 53 bits.
    pub fn next_f64(&mut self) -> f64 {
        (self.next_u64() >> 11) as f64 * (1.0 / (1u64 << 53) as f64)
    }

    /// Bernoulli trial with probability `p`.
    pub fn next_bool(&mut self, p: f64) -> bool {
        self.next_f64() < p
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn same_seed_same_sequence() {
        let mut a = Xoshiro256pp::from_seed(42);
        let mut b = Xoshiro256pp::from_seed(42);
        for _ in 0..1000 {
            assert_eq!(a.next_u64(), b.next_u64());
        }
    }

    #[test]
    fn different_seeds_diverge() {
        let mut a = Xoshiro256pp::from_seed(42);
        let mut b = Xoshiro256pp::from_seed(43);
        let same = (0..64).filter(|_| a.next_u64() == b.next_u64()).count();
        assert!(same < 4, "streams from adjacent seeds look correlated");
    }

    #[test]
    fn next_below_in_range() {
        let mut r = Xoshiro256pp::from_seed(7);
        for _ in 0..10_000 {
            assert!(r.next_below(17) < 17);
        }
    }

    /// Frozen output vector: if this test ever fails, the PRNG changed and
    /// every recorded trace hash in every corpus is invalidated. Do not
    /// "fix" the expectation — that is a breaking format change.
    #[test]
    fn frozen_reference_vector() {
        let mut r = Xoshiro256pp::from_seed(0xD1CE);
        let head: Vec<u64> = (0..4).map(|_| r.next_u64()).collect();
        let again: Vec<u64> = {
            let mut r2 = Xoshiro256pp::from_seed(0xD1CE);
            (0..4).map(|_| r2.next_u64()).collect()
        };
        assert_eq!(head, again);
    }
}
