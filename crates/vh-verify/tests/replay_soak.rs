#![forbid(unsafe_code)]

use vh_verify::{
    format_error_receipt, format_receipt, replay_soak, ReplaySoakError,
    OBSERVABLE_FINGERPRINT_SCHEMA, REFERENCE_OBSERVABLE_FINGERPRINT, REFERENCE_TRACE_EVENTS,
    REFERENCE_TRACE_HASH,
};

#[test]
fn fixed_reference_replays_to_one_tier_1_identity() {
    let first = replay_soak(64).expect("Tier-1 replay soak");
    let second = replay_soak(64).expect("Tier-1 replay soak");

    assert_eq!(first.runs(), 64);
    assert_eq!(first, second);
    assert_eq!(first.trace_events(), REFERENCE_TRACE_EVENTS);
    assert_eq!(first.trace_hash(), REFERENCE_TRACE_HASH);
    assert_eq!(first.observable_schema(), OBSERVABLE_FINGERPRINT_SCHEMA);
    assert_eq!(
        first.observable_fingerprint(),
        REFERENCE_OBSERVABLE_FINGERPRINT
    );
    assert_eq!(
        format_receipt(&first, 123),
        "soak: receipt-schema=vh-verify-soak-v1 verdict=PASS determinism-tier=Tier-1 evidence-grade=D0 root-seed=0x000000000000d1ce universe=0 workload=vh-verify-reference trace-format=v0 runs=64 hash=eafa30e8a7a6c82939ea3f755bc866ab events=33 observable-schema=vh-verify-observable-v3 observable-fingerprint=bf78c94b6f72ae77ad0a00a86e36c2e9 upH=123 upH-scope=boundary-telemetry"
    );
}

#[test]
fn zero_run_soak_is_rejected() {
    assert_eq!(replay_soak(0), Err(ReplaySoakError::ZeroRuns));
    assert_eq!(ReplaySoakError::ZeroRuns.code(), "zero-runs");
    assert_eq!(
        format_error_receipt(&ReplaySoakError::ZeroRuns),
        "soak: receipt-schema=vh-verify-soak-v1 verdict=ERROR determinism-tier=Tier-1 evidence-grade=D0 root-seed=0x000000000000d1ce universe=0 workload=vh-verify-reference trace-format=v0 observable-schema=vh-verify-observable-v3 requested-runs=0 error-code=zero-runs"
    );
}
