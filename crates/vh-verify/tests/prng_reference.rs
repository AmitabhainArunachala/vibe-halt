#![forbid(unsafe_code)]

//! Literal vectors independently derived from the public-domain references:
//! <https://prng.di.unimi.it/splitmix64.c>
//! <https://prng.di.unimi.it/xoshiro256plusplus.c>

use std::panic::{catch_unwind, AssertUnwindSafe};

use vh_core::rng::splitmix64;
use vh_core::Xoshiro256pp;

const SPLITMIX64_D1CE: [u64; 4] = [
    0x29c2_d060_2618_91fb,
    0xc042_d56e_fd8a_d139,
    0x140c_b338_ef93_3c26,
    0xd159_57dc_1dad_3f38,
];

const XOSHIRO256PP_D1CE: [u64; 8] = [
    0x47e4_b348_c016_200f,
    0xb3f4_9dc0_c55a_ccb4,
    0xa120_3c4b_5476_b7fd,
    0x283c_1b14_e6c5_25cb,
    0x52fb_041d_6eae_5eef,
    0x341f_c15b_f5f6_838b,
    0x7478_ddf6_01e4_1515,
    0xa98e_97e4_59b4_71a2,
];

const SPLITMIX64_ZERO: [u64; 4] = [
    0xe220_a839_7b1d_cdaf,
    0x6e78_9e6a_a1b9_65f4,
    0x06c4_5d18_8009_454f,
    0xf88b_b8a8_724c_81ec,
];

const XOSHIRO256PP_ZERO: [u64; 8] = [
    0x5317_5d61_490b_23df,
    0x61da_6f3d_c380_d507,
    0x5c0f_df91_ec9a_7bfc,
    0x02ee_bf8c_3bbe_5e1a,
    0x7eca_04eb_af4a_5eea,
    0x0543_c377_57f0_8d9a,
    0xdb74_90c7_5ab5_026e,
    0xd873_43e6_464b_c959,
];

const SPLITMIX64_MAX: [u64; 4] = [
    0xe4d9_7177_1b65_2c20,
    0xe99f_f867_dbf6_82c9,
    0x382f_f84c_b272_81e9,
    0x6d1d_b36c_cba9_82d2,
];

const XOSHIRO256PP_MAX: [u64; 8] = [
    0x56cc_f8ce_948e_27b2,
    0xe685_8843_2e5a_5b90,
    0xe3e9_b5a4_8119_ca8b,
    0x460f_1949_5532_ae73,
    0xa7d6_2040_ea92_63e1,
    0x66f1_fb2a_c940_2c14,
    0xe243_b47d_e8a7_3f68,
    0x7c93_fdab_4c7b_3dff,
];

#[test]
fn splitmix64_matches_official_literal_vector() {
    let mut state = 0xD1CE;
    let actual = SPLITMIX64_D1CE.map(|_| splitmix64(&mut state));
    assert_eq!(actual, SPLITMIX64_D1CE);
}

#[test]
fn xoshiro256pp_matches_official_literal_vector() {
    let mut rng = Xoshiro256pp::from_seed(0xD1CE);
    let actual = XOSHIRO256PP_D1CE.map(|_| rng.next_u64());
    assert_eq!(actual, XOSHIRO256PP_D1CE);
}

#[test]
fn zero_and_max_seed_boundaries_match_independent_vectors() {
    for (seed, expected_splitmix, expected_xoshiro) in [
        (0, SPLITMIX64_ZERO, XOSHIRO256PP_ZERO),
        (u64::MAX, SPLITMIX64_MAX, XOSHIRO256PP_MAX),
    ] {
        let mut state = seed;
        let actual_splitmix = expected_splitmix.map(|_| splitmix64(&mut state));
        assert_eq!(actual_splitmix, expected_splitmix);

        let mut rng = Xoshiro256pp::from_seed(seed);
        let actual_xoshiro = expected_xoshiro.map(|_| rng.next_u64());
        assert_eq!(actual_xoshiro, expected_xoshiro);
    }
}

#[test]
fn rejection_sampling_consumption_matches_literal_reference_words() {
    let mut half = Xoshiro256pp::from_seed(0xD1CE);
    assert_eq!(half.next_u64(), XOSHIRO256PP_D1CE[0]);
    assert_eq!(half.next_below(1u64 << 63), XOSHIRO256PP_D1CE[3]);
    assert_eq!(half.next_u64(), XOSHIRO256PP_D1CE[4]);

    let mut one = Xoshiro256pp::from_seed(0xD1CE);
    assert_eq!(one.next_below(1), 0);
    assert_eq!(one.next_u64(), XOSHIRO256PP_D1CE[1]);

    let mut max = Xoshiro256pp::from_seed(0xD1CE);
    assert_eq!(max.next_below(u64::MAX), XOSHIRO256PP_D1CE[0]);
    assert_eq!(max.next_u64(), XOSHIRO256PP_D1CE[1]);
}

#[test]
fn floating_draw_bits_and_consumption_are_frozen() {
    let mut rng = Xoshiro256pp::from_seed(0xD1CE);
    assert_eq!(rng.next_f64().to_bits(), 0x3fd1_f92c_d230_0588);
    assert_eq!(rng.next_u64(), XOSHIRO256PP_D1CE[1]);
}

#[test]
fn boolean_boundary_draws_consume_exactly_one_word() {
    let mut never = Xoshiro256pp::from_seed(0xD1CE);
    assert!(!never.next_bool(0.0));
    assert_eq!(never.next_u64(), XOSHIRO256PP_D1CE[1]);

    let mut always = Xoshiro256pp::from_seed(0xD1CE);
    assert!(always.next_bool(1.0));
    assert_eq!(always.next_u64(), XOSHIRO256PP_D1CE[1]);

    let mut exact_boundary = Xoshiro256pp::from_seed(0xD1CE);
    assert!(!exact_boundary.next_bool(f64::from_bits(0x3fd1_f92c_d230_0588)));
    assert_eq!(exact_boundary.next_u64(), XOSHIRO256PP_D1CE[1]);
}

#[test]
fn invalid_boolean_probabilities_panic_before_consuming_state() {
    for invalid in [
        f64::NAN,
        f64::INFINITY,
        f64::NEG_INFINITY,
        -f64::EPSILON,
        1.0 + f64::EPSILON,
    ] {
        let mut rejected = Xoshiro256pp::from_seed(0xD1CE);
        let mut untouched = rejected.clone();

        let panic = catch_unwind(AssertUnwindSafe(|| rejected.next_bool(invalid)));
        assert!(panic.is_err(), "invalid probability {invalid:?} must panic");

        let after_rejection = rejected.next_u64();
        let untouched_next = untouched.next_u64();
        assert_eq!(
            after_rejection, untouched_next,
            "rejecting {invalid:?} consumed PRNG state"
        );
        assert_eq!(after_rejection, XOSHIRO256PP_D1CE[0]);
    }
}

#[test]
#[should_panic(expected = "next_below(0) is meaningless")]
fn rejection_sampling_rejects_an_empty_domain() {
    let mut rng = Xoshiro256pp::from_seed(0xD1CE);
    let _ = rng.next_below(0);
}
