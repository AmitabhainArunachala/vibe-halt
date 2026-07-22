//! vh-trace — the append-only event trace and its chained hash.
//!
//! The trace is the spine of vibe-halt: replay, shrinking, divergence
//! detection, and evidence all hang off it. Two runs of the same universe
//! are "identical" if and only if their trace hashes match.
//!
//! Format spec: `docs/specs/TRACE_FORMAT_V0.md`. Hash is chained FNV-1a 128
//! in v0 (fast, deterministic; NOT cryptographic — v1 upgrades to SHA-256
//! when traces become cross-party evidence).
//!
//! Framing is length-prefixed, not separator-framed: every field is either
//! fixed-width or preceded by its little-endian length, so the absorbed byte
//! stream decodes to exactly one event sequence regardless of payload
//! content. (The original separator framing was non-injective — payloads
//! containing the separator bytes could forge event boundaries; found in
//! PR #1 review and repaired pre-release.)

#![forbid(unsafe_code)]

const FNV128_OFFSET: u128 = 0x6c62_272e_07bb_0142_62b8_2175_6295_c58d;
const FNV128_PRIME: u128 = 0x0000_0000_0100_0000_0000_0000_0000_013B;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TraceEvent {
    /// Virtual-time nanos at which the event was recorded.
    pub at_nanos: u64,
    /// Short machine-readable kind, e.g. "put", "crash", "fault.network".
    pub kind: String,
    /// Free-form payload. Must itself be deterministic content.
    pub data: String,
}

#[derive(Debug, Clone)]
pub struct Trace {
    events: Vec<TraceEvent>,
    state: u128,
}

/// Additive Track-2 decision-tape stream. This is deliberately NOT the v0
/// execution trace: it has its own schema record and digest so scheduler
/// choice recording can land without mutating `TRACE_FORMAT_V0.md` or any
/// frozen trace hash.
#[derive(Debug, Clone)]
pub struct DecisionTape {
    trace: Trace,
    decisions: u64,
}

impl Default for DecisionTape {
    fn default() -> Self {
        Self::new()
    }
}

impl Default for Trace {
    fn default() -> Self {
        Self::new()
    }
}

impl Trace {
    pub fn new() -> Self {
        Self {
            events: Vec::new(),
            state: FNV128_OFFSET,
        }
    }

    fn absorb(&mut self, bytes: &[u8]) {
        for &b in bytes {
            self.state ^= b as u128;
            self.state = self.state.wrapping_mul(FNV128_PRIME);
        }
    }

    pub fn record(&mut self, at_nanos: u64, kind: &str, data: &str) {
        // Injective framing: fixed-width at, then length-prefixed fields.
        self.absorb(&at_nanos.to_le_bytes());
        self.absorb(&(kind.len() as u64).to_le_bytes());
        self.absorb(kind.as_bytes());
        self.absorb(&(data.len() as u64).to_le_bytes());
        self.absorb(data.as_bytes());
        self.events.push(TraceEvent {
            at_nanos,
            kind: kind.to_string(),
            data: data.to_string(),
        });
    }

    /// The chained hash over every event recorded so far, as 32 hex chars.
    pub fn hash_hex(&self) -> String {
        format!("{:032x}", self.state)
    }

    pub fn events(&self) -> &[TraceEvent] {
        &self.events
    }

    pub fn len(&self) -> usize {
        self.events.len()
    }

    pub fn is_empty(&self) -> bool {
        self.events.is_empty()
    }
}

impl DecisionTape {
    pub const SCHEMA: &'static str = "vh-decision-tape-v1";

    pub fn new() -> Self {
        let mut trace = Trace::new();
        trace.record(0, "schema", Self::SCHEMA);
        Self {
            trace,
            decisions: 0,
        }
    }

    pub fn record_decision(
        &mut self,
        site_id: &str,
        candidate_set_digest: &str,
        chosen_index: u64,
        policy_id: &str,
    ) {
        self.decisions += 1;
        self.trace.record(
            self.decisions,
            "decision",
            &format!(
                "site={site_id} candidates={candidate_set_digest} chosen={chosen_index} policy={policy_id}"
            ),
        );
    }

    pub fn digest_hex(&self) -> String {
        self.trace.hash_hex()
    }

    pub fn len(&self) -> u64 {
        self.decisions
    }

    pub fn is_empty(&self) -> bool {
        self.decisions == 0
    }

    pub fn events(&self) -> &[TraceEvent] {
        self.trace.events()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn same_events_same_hash() {
        let mut a = Trace::new();
        let mut b = Trace::new();
        for t in [&mut a, &mut b] {
            t.record(1, "put", "k1=v1");
            t.record(2, "flush", "");
            t.record(3, "crash", "");
        }
        assert_eq!(a.hash_hex(), b.hash_hex());
    }

    #[test]
    fn different_events_different_hash() {
        let mut a = Trace::new();
        let mut b = Trace::new();
        a.record(1, "put", "k1=v1");
        b.record(1, "put", "k1=v2");
        assert_ne!(a.hash_hex(), b.hash_hex());
    }

    #[test]
    fn field_boundaries_matter() {
        // ("ab","c") must not collide with ("a","bc").
        let mut a = Trace::new();
        let mut b = Trace::new();
        a.record(1, "ab", "c");
        b.record(1, "a", "bc");
        assert_ne!(a.hash_hex(), b.hash_hex());
    }

    #[test]
    fn empty_trace_has_stable_hash() {
        assert_eq!(Trace::new().hash_hex(), Trace::new().hash_hex());
    }

    /// Regression for the PR #1 review BLOCKER: under separator framing,
    /// a payload containing the separator bytes could make one event absorb
    /// byte-identically to two events. Length-prefixed framing must keep
    /// these distinct.
    #[test]
    fn separator_bytes_in_payload_cannot_forge_event_boundaries() {
        let mut two_events = Trace::new();
        two_events.record(7, "a", "x");
        two_events.record(0x4141_4141_4141_4141, "b", "y");

        let mut one_event = Trace::new();
        one_event.record(7, "a", "x\u{1e}AAAAAAAA\u{1f}b\u{1f}y");

        assert_ne!(two_events.hash_hex(), one_event.hash_hex());
    }

    #[test]
    fn event_count_is_part_of_framing() {
        // ("k","") then ("","d") must differ from ("k","d") alone even
        // though the concatenated payload bytes agree.
        let mut a = Trace::new();
        a.record(1, "k", "");
        a.record(1, "", "d");
        let mut b = Trace::new();
        b.record(1, "k", "d");
        assert_ne!(a.hash_hex(), b.hash_hex());
    }

    #[test]
    fn decision_tape_has_stable_separate_digest() {
        fn tape() -> DecisionTape {
            let mut t = DecisionTape::new();
            t.record_decision("runtime.step", "abc", 0, "fifo-v0");
            t.record_decision("runtime.step", "def", 0, "fifo-v0");
            t
        }

        let a = tape();
        let b = tape();
        assert_eq!(a.digest_hex(), b.digest_hex());
        assert_eq!(a.len(), 2);
        assert_eq!(a.events()[0].kind, "schema");
        assert_eq!(a.events()[0].data, DecisionTape::SCHEMA);
    }

    #[test]
    fn decision_tape_digest_is_not_the_empty_trace_hash() {
        assert_ne!(DecisionTape::new().digest_hex(), Trace::new().hash_hex());
    }
}
