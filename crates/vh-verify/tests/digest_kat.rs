//! V1 — independent known-answer verification of the C3 digest surface
//! (post-audit controller §5A; interface request on PR #41).
//!
//! The verifier asserts the standard FIPS 180-4 / NIST CAVP vectors
//! against `vh-digest` AS A BLACK BOX — the implementation is never
//! copied here, and these expected literals are transcribed from the
//! public standard, not from the core crate's own tests. Until this
//! suite exists on merged main, core's claim language stays
//! "content-addressed with an unreviewed local implementation".

/// The four standard vectors, plus tag stability.
#[test]
fn sha256_known_answer_vectors_hold_black_box() {
    assert_eq!(
        vh_digest::sha256_hex(b""),
        "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
    );
    assert_eq!(
        vh_digest::sha256_hex(b"abc"),
        "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
    );
    assert_eq!(
        vh_digest::sha256_hex(b"abcdbcdecdefdefgefghfghighijhijkijkljklmklmnlmnomnopnopq"),
        "248d6a61d20638b8e5c026930c3e6039a33ce45964ff2167f6ecedd419db06c1"
    );
    let mut hasher = vh_digest::Sha256::new();
    for _ in 0..1000 {
        hasher.update(&[b'a'; 1000]);
    }
    assert_eq!(
        vh_digest::hex(&hasher.finalize()),
        "cdc76e5c9914fb9281a1c7e284d73e67f1809a48a497200e046d39ccc7112cd0"
    );
    assert_eq!(vh_digest::ALGORITHM, "sha256");
    assert_eq!(vh_digest::DIGEST_SCHEMA, "vh-sha256-v1");
}

/// Incremental hashing must be split-invariant from the OUTSIDE too —
/// an independent probe over a message none of the core tests use.
#[test]
fn incremental_split_invariance_black_box() {
    let msg: Vec<u8> = (0..257u32).map(|i| (i * 31 % 251) as u8).collect();
    let one_shot = vh_digest::sha256_hex(&msg);
    for split in [0, 1, 55, 56, 63, 64, 65, 128, 256, 257] {
        let mut h = vh_digest::Sha256::new();
        h.update(&msg[..split]);
        h.update(&msg[split..]);
        assert_eq!(vh_digest::hex(&h.finalize()), one_shot, "split {split}");
    }
}

/// Canonical-byte behavior: the C1 identity byte streams the digests are
/// computed over must themselves be replay-stable, and digesting them
/// must be deterministic — checked through the verifier's own SimRuntime
/// reference, independent of any core evidence code path.
#[test]
fn canonical_identity_bytes_are_stable_and_digest_deterministically() {
    use vh_verify::full_observation::{sim_reference_workload, SIM_REFERENCE_ROOT_SEED};
    let workload = sim_reference_workload();
    let a = vh_multiverse::run_universe(SIM_REFERENCE_ROOT_SEED, 3, &workload);
    let b = vh_multiverse::run_universe(SIM_REFERENCE_ROOT_SEED, 3, &workload);
    assert_eq!(
        a.end_state_identity().canonical_bytes(),
        b.end_state_identity().canonical_bytes()
    );
    assert_eq!(
        a.complete_observation_identity().canonical_bytes(),
        b.complete_observation_identity().canonical_bytes()
    );
    assert_eq!(
        vh_digest::sha256_hex(a.complete_observation_identity().canonical_bytes()),
        vh_digest::sha256_hex(b.complete_observation_identity().canonical_bytes())
    );
    assert_eq!(a.observation().universe_id, 3);
    assert_eq!(a.trace_hash().len(), 32, "frozen trace hash width");
}
