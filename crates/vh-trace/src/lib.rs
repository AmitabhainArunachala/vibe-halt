//! vh-trace — the append-only event trace and its chained hash.
//!
//! The trace is the spine of vibe-halt: replay, shrinking, divergence
//! detection, and evidence all hang off it. Two runs of the same universe
//! are "identical" if and only if their trace hashes match.
//!
//! Format spec: `docs/specs/TRACE_FORMAT_V0.md`. Hash is chained FNV-1a 128
//! in v0 (fast, deterministic; NOT cryptographic — v1 upgrades to SHA-256
//! when traces become cross-party evidence).

const FNV128_OFFSET: u128 = 0x6c62_272e_07bb_0142_62b8_2175_6295_c58d;
const FNV128_PRIME: u128 = 0x0000_0000_0100_0000_0000_0000_0000_013B;

/// Field separator (US) and record separator (RS) absorbed between hash
/// inputs so that ("ab","c") never collides with ("a","bc").
const FIELD_SEP: u8 = 0x1f;
const RECORD_SEP: u8 = 0x1e;

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
        self.absorb(&at_nanos.to_le_bytes());
        self.absorb(&[FIELD_SEP]);
        self.absorb(kind.as_bytes());
        self.absorb(&[FIELD_SEP]);
        self.absorb(data.as_bytes());
        self.absorb(&[RECORD_SEP]);
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
}
