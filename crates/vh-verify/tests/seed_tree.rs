#![forbid(unsafe_code)]

use vh_core::SeedTree;

fn stream_head(tree: &SeedTree, universe: u64, name: &str, len: usize) -> Vec<u64> {
    let mut stream = tree.stream(universe, name);
    (0..len).map(|_| stream.next_u64()).collect()
}

#[test]
fn seed_tree_composition_has_literal_independent_vectors() {
    // Independently derived by composing the specified wrapping universe
    // derivation, FNV-1a name domain, SplitMix64 expansion, and xoshiro256++.
    // These literals freeze composition errors that component-only vectors
    // cannot detect.
    let cases = [
        (
            0,
            0,
            "",
            0x6e78_9e6a_a1b9_65f4,
            [
                0x004e_c78c_d94b_c4f1,
                0x80f2_eb7e_8f55_3538,
                0x714d_6b20_771d_4523,
                0x829c_3d39_2f3c_7e67,
            ],
        ),
        (
            0xD1CE,
            0,
            "ops",
            0xc042_d56e_fd8a_d139,
            [
                0x28b6_7a90_d558_8abf,
                0x8f11_1f11_a2f4_a1b1,
                0x5f2a_ffb3_468f_04e1,
                0x6d13_3a42_fd0f_bcf8,
            ],
        ),
        (
            0xD1CE,
            7,
            "target",
            0xbee6_ef31_1fe8_3f51,
            [
                0x07de_a945_2720_5b83,
                0x2ea9_4639_9380_0bff,
                0x724d_96fa_b2d4_b44b,
                0x3f48_4879_b14c_1962,
            ],
        ),
        (
            u64::MAX,
            u64::MAX,
            "α/β\0",
            0xe4d9_7177_1b65_2c20,
            [
                0xfb35_e572_21c7_b800,
                0x94a9_f9e5_59cb_7321,
                0xc748_b6f5_beff_8ad6,
                0xf2d2_3e16_f99a_31f3,
            ],
        ),
    ];

    for (root, universe, name, expected_seed, expected_head) in cases {
        let tree = SeedTree::new(root);
        assert_eq!(tree.universe_seed(universe), expected_seed);
        assert_eq!(
            stream_head(&tree, universe, name, expected_head.len()),
            expected_head
        );
    }
}

#[test]
fn target_stream_is_independent_of_stream_set_and_draw_order() {
    let tree = SeedTree::new(0xD1CE);
    let baseline = stream_head(&tree, 7, "target", 128);
    let sibling_a_head = stream_head(&tree, 7, "sibling.a", 128);
    let sibling_b_head = stream_head(&tree, 7, "sibling.b", 128);
    assert_ne!(baseline, sibling_a_head);
    assert_ne!(baseline, sibling_b_head);
    assert_ne!(sibling_a_head, sibling_b_head);

    let mut sibling_a = tree.stream(7, "sibling.a");
    let mut sibling_b = tree.stream(7, "sibling.b");
    for _ in 0..17 {
        let _ = sibling_b.next_u64();
    }
    for _ in 0..31 {
        let _ = sibling_a.next_u64();
    }
    let with_existing_siblings = stream_head(&tree, 7, "target", 128);

    let mut target = tree.stream(7, "target");
    let mut interleaved = Vec::new();
    for index in 0..128 {
        if index % 3 == 0 {
            let _ = sibling_a.next_u64();
        }
        if index % 5 == 0 {
            let _ = sibling_b.next_u64();
        }
        interleaved.push(target.next_u64());
    }

    assert_eq!(with_existing_siblings, baseline);
    assert_eq!(interleaved, baseline);
}

#[test]
fn adjacent_universes_pass_fixed_integer_correlation_smoke() {
    const PAIRS: u64 = 4_096;
    const BITS_PER_PAIR: u64 = 64;

    let tree = SeedTree::new(42);
    let mut equal_bits = 0u64;
    let mut duplicate_words = 0u64;

    for universe in 0..PAIRS {
        let left = tree.stream(universe, "ops").next_u64();
        let right = tree.stream(universe + 1, "ops").next_u64();
        if left == right {
            duplicate_words += 1;
        }
        equal_bits += BITS_PER_PAIR - (left ^ right).count_ones() as u64;
    }

    let total_bits = PAIRS * BITS_PER_PAIR;
    let midpoint = total_bits / 2;
    let deviation = equal_bits.abs_diff(midpoint);

    assert_eq!(
        duplicate_words, 0,
        "Tier 1 adjacent-universe smoke found duplicate first words"
    );
    assert!(
        deviation <= total_bits / 100,
        "Tier 1 adjacent-universe equal-bit ratio left the fixed 49%-51% band: {equal_bits}/{total_bits}"
    );
}
