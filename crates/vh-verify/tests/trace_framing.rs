#![forbid(unsafe_code)]

use vh_trace::Trace;

const FNV128_OFFSET: u128 = 0x6c62_272e_07bb_0142_62b8_2175_6295_c58d;
const FNV128_PRIME: u128 = 0x0000_0000_0100_0000_0000_0000_0000_013B;

fn reference_absorb(mut state: u128, bytes: &[u8]) -> u128 {
    for byte in bytes {
        state ^= u128::from(*byte);
        state = state.wrapping_mul(FNV128_PRIME);
    }
    state
}

fn reference_hash(events: &[(u64, String, String)]) -> String {
    let mut state = FNV128_OFFSET;
    for (at_nanos, kind, data) in events {
        state = reference_absorb(state, &at_nanos.to_le_bytes());
        state = reference_absorb(state, &(kind.len() as u64).to_le_bytes());
        state = reference_absorb(state, kind.as_bytes());
        state = reference_absorb(state, &(data.len() as u64).to_le_bytes());
        state = reference_absorb(state, data.as_bytes());
    }
    format!("{state:032x}")
}

#[test]
fn empty_fields_have_a_literal_stable_baseline() {
    assert_eq!(Trace::new().hash_hex(), "6c62272e07bb014262b821756295c58d");

    let mut first = Trace::new();
    let mut second = Trace::new();
    first.record(0, "", "");
    second.record(0, "", "");
    assert_eq!(first.hash_hex(), second.hash_hex());
    assert_eq!(first.hash_hex(), "c5975d646bc2ca7ec51f289603d0f8ad");
    assert_ne!(first.hash_hex(), Trace::new().hash_hex());
}

#[test]
fn separator_and_nul_bytes_are_retained_deterministically() {
    let kind = "kind\u{1f}with\u{1e}separators";
    let data = "data\0\u{1e}record\u{1f}field";
    let mut first = Trace::new();
    let mut second = Trace::new();
    let mut changed_kind_separator = Trace::new();
    let mut changed_nul = Trace::new();

    first.record(9, kind, data);
    second.record(9, kind, data);
    changed_kind_separator.record(9, "kind\u{1e}with\u{1e}separators", data);
    changed_nul.record(9, kind, "data\u{1}\u{1e}record\u{1f}field");

    assert_eq!(first.hash_hex(), second.hash_hex());
    assert_eq!(first.hash_hex(), "84877f9b79d7d36ac535a44fd87d2989");
    assert_eq!(
        changed_kind_separator.hash_hex(),
        "b25b51232555c777db18c9f56b3242c8"
    );
    assert_eq!(changed_nul.hash_hex(), "48f0a9122b2b42f9f10987a9e6d18b08");
    assert_ne!(first.hash_hex(), changed_kind_separator.hash_hex());
    assert_ne!(first.hash_hex(), changed_nul.hash_hex());
    assert_eq!(first.events(), second.events());
    assert_eq!(first.events()[0].kind, kind);
    assert_eq!(first.events()[0].data, data);
}

#[test]
fn long_payload_is_deterministic_and_content_sensitive() {
    let payload = "v".repeat(64 * 1_024);
    let mut changed = payload.clone();
    changed.push('!');

    let mut first = Trace::new();
    let mut replay = Trace::new();
    let mut different = Trace::new();
    first.record(u64::MAX, "long", &payload);
    replay.record(u64::MAX, "long", &payload);
    different.record(u64::MAX, "long", &changed);

    assert_eq!(first.hash_hex(), replay.hash_hex());
    assert_eq!(first.hash_hex(), "49fc2c15455819ca5d479390cdf05d96");
    assert_eq!(different.hash_hex(), "f6a4ff8c81c9507f4f8bb435d2b7d302");
    assert_ne!(first.hash_hex(), different.hash_hex());
}

#[test]
fn distinct_event_sequences_must_not_share_framing() {
    let mut one_event = Trace::new();
    one_event.record(7, "a", "x\u{1e}AAAAAAAA\u{1f}b\u{1f}y");

    let mut two_events = Trace::new();
    two_events.record(7, "a", "x");
    two_events.record(0x4141_4141_4141_4141, "b", "y");

    assert_ne!(one_event.events(), two_events.events());
    assert_ne!(
        one_event.hash_hex(),
        two_events.hash_hex(),
        "Tier 1 trace identity must distinguish different event sequences"
    );
}

#[test]
fn production_hash_matches_an_independent_prefix_model() {
    let events = [
        (0, String::new(), String::new()),
        (1, "κind\0".to_string(), "雪/🧪".to_string()),
        (255, "k".repeat(255), "d".repeat(256)),
        (256, "k".repeat(256), "d".repeat(255)),
        (
            u64::MAX,
            "long".to_string(),
            format!("{}!", "v".repeat(64 * 1_024)),
        ),
    ];
    let mut production = Trace::new();

    assert_eq!(production.hash_hex(), reference_hash(&[]));
    for prefix_len in 1..=events.len() {
        let (at_nanos, kind, data) = &events[prefix_len - 1];
        production.record(*at_nanos, kind, data);
        assert_eq!(
            production.hash_hex(),
            reference_hash(&events[..prefix_len]),
            "reference-model mismatch at prefix {prefix_len}"
        );
    }
}
