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
    ///
    /// Fail-closed input contract: `p` must be finite and in `[0, 1]`.
    /// NaN, infinities, and out-of-range values previously collapsed into
    /// deterministic-but-invalid booleans while consuming a word (PR #1
    /// hardening-loop-3 GAP); they now panic BEFORE any PRNG state is
    /// consumed, so a rejected call cannot shift subsequent draws.
    pub fn next_bool(&mut self, p: f64) -> bool {
        assert!(
            p.is_finite() && (0.0..=1.0).contains(&p),
            "next_bool probability must be finite and in [0, 1]"
        );
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
    /// "fix" the expectations — that is a breaking format change.
    ///
    /// The literals below were derived independently of this implementation,
    /// from the official reference algorithms (Vigna's splitmix64.c and
    /// Blackman/Vigna's xoshiro256plusplus.c), and cross-checked against a
    /// second independent transcription in the PR #1 review. The original
    /// version of this test compared the implementation to itself, which
    /// froze nothing (PR #1 review BLOCKER).
    #[test]
    fn frozen_reference_vector() {
        // SplitMix64 stream from state 0xD1CE (also the xoshiro seed
        // expansion, by construction of from_seed).
        let mut sm = 0xD1CE_u64;
        let expansion: Vec<u64> = (0..4).map(|_| splitmix64(&mut sm)).collect();
        assert_eq!(
            expansion,
            vec![
                0x29c2_d060_2618_91fb,
                0xc042_d56e_fd8a_d139,
                0x140c_b338_ef93_3c26,
                0xd159_57dc_1dad_3f38,
            ]
        );

        // xoshiro256++ head for seed 0xD1CE.
        let mut r = Xoshiro256pp::from_seed(0xD1CE);
        let head: Vec<u64> = (0..8).map(|_| r.next_u64()).collect();
        assert_eq!(
            head,
            vec![
                0x47e4_b348_c016_200f,
                0xb3f4_9dc0_c55a_ccb4,
                0xa120_3c4b_5476_b7fd,
                0x283c_1b14_e6c5_25cb,
                0x52fb_041d_6eae_5eef,
                0x341f_c15b_f5f6_838b,
                0x7478_ddf6_01e4_1515,
                0xa98e_97e4_59b4_71a2,
            ]
        );
    }

    /// Rejection consumption is part of the frozen deterministic surface:
    /// a rejected draw advances the stream, so how many raw values a
    /// `next_below` call consumes changes every subsequent draw (PR #1
    /// review NIT). Literals derive from the frozen head above.
    #[test]
    fn next_below_consumption_is_frozen() {
        // Head words for seed 0xD1CE (see frozen_reference_vector):
        // [0] 47e4... (< 2^63)  [1] b3f4... (>= 2^63)  [2] a120... (>= 2^63)
        // [3] 283c... (< 2^63)  [4] 52fb...
        let mut r = Xoshiro256pp::from_seed(0xD1CE);
        let _ = r.next_u64(); // consume word 0

        // next_below(2^63): zone = 2^63; words 1 and 2 are rejected,
        // word 3 accepted. Three raw draws consumed.
        let v = r.next_below(1u64 << 63);
        assert_eq!(v, 0x283c_1b14_e6c5_25cb);
        // The next raw draw must be word 4 — proving exactly 3 draws
        // were consumed by the call above.
        assert_eq!(r.next_u64(), 0x52fb_041d_6eae_5eef);

        // n = 1: zone = u64::MAX, word 0 accepted, result always 0.
        let mut r1 = Xoshiro256pp::from_seed(0xD1CE);
        assert_eq!(r1.next_below(1), 0);
        assert_eq!(r1.next_u64(), 0xb3f4_9dc0_c55a_ccb4); // 1 draw consumed

        // n = u64::MAX: only the value u64::MAX is rejected; word 0 accepted.
        let mut r2 = Xoshiro256pp::from_seed(0xD1CE);
        assert_eq!(r2.next_below(u64::MAX), 0x47e4_b348_c016_200f);
        assert_eq!(r2.next_u64(), 0xb3f4_9dc0_c55a_ccb4); // 1 draw consumed
    }

    /// Invalid Bernoulli inputs are rejected BEFORE any PRNG state is
    /// consumed (PR #1 hardening-loop-3 GAP). Negative regression: on the
    /// pre-repair kernel, each of these calls consumed one word and
    /// returned a silent bool, so the final draw below landed on word 5
    /// instead of frozen head word 0.
    #[test]
    fn next_bool_rejects_invalid_inputs_without_consuming_state() {
        use std::panic::{catch_unwind, AssertUnwindSafe};
        let mut r = Xoshiro256pp::from_seed(0xD1CE);
        for bad in [
            f64::NAN,
            f64::INFINITY,
            f64::NEG_INFINITY,
            -f64::EPSILON,
            1.0 + f64::EPSILON,
        ] {
            let outcome = catch_unwind(AssertUnwindSafe(|| r.next_bool(bad)));
            assert!(outcome.is_err(), "next_bool({bad}) must panic, not answer");
        }
        // Zero state consumed by the five rejected calls: the next raw draw
        // is still frozen head word 0 (see frozen_reference_vector).
        assert_eq!(r.next_u64(), 0x47e4_b348_c016_200f);
    }

    /// Valid boundary inputs keep their frozen consumption semantics:
    /// exactly one word per call; p=0.0 is always-false and p=1.0 is
    /// always-true because next_f64 lies in [0, 1).
    #[test]
    fn next_bool_boundary_consumption_is_frozen() {
        let mut r = Xoshiro256pp::from_seed(0xD1CE);
        assert!(!r.next_bool(0.0)); // consumes frozen head word 0
        assert!(r.next_bool(1.0)); // consumes frozen head word 1

        // The next raw draw must be frozen head word 2 — proving exactly
        // one word per boundary call, unchanged by input validation.
        assert_eq!(r.next_u64(), 0xa120_3c4b_5476_b7fd);
    }
}
