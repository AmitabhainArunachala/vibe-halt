//! Canonical, versioned bytes for replay identity.
//!
//! These bytes are the identity. They are deliberately not a new digest:
//! persisted evidence will hash this exact representation under a separately
//! reviewed algorithm. Every value is explicitly framed; no host formatting,
//! allocator identity, locale, or collection iteration order enters it.

mod decode;

use std::collections::BTreeMap;

use vh_props::{AlwaysCheck, AlwaysFailure, EndState};

use crate::{FaultPlanDiscipline, RunOutcome, RuntimeEvidence, SchedulePolicy, UniverseLifecycle};

pub use decode::{decode_end_state, validate_complete_observation, DecodeError};

/// Algorithm tag for the injective byte framing used by both identities.
pub const CANONICAL_IDENTITY_ALGORITHM: &str = "vh-canonical-length-framing-v1";
/// Schema for the raw state consumed by end-state oracles.
pub const END_STATE_IDENTITY_SCHEMA: &str = "vh-end-state-observation-v1";
/// Schema for every public observable of one universe execution.
pub const COMPLETE_OBSERVATION_IDENTITY_SCHEMA: &str = "vh-complete-observation-v1";

pub(super) const MAGIC: &[u8; 8] = b"VHOBS\0\x01\0";
pub(super) const KIND_U64: u8 = 1;
pub(super) const KIND_STRING: u8 = 2;
pub(super) const KIND_SEQUENCE: u8 = 3;
pub(super) const KIND_MAP: u8 = 4;
pub(super) const KIND_STRUCT: u8 = 5;
pub(super) const KIND_OPTION: u8 = 6;
pub(super) const KIND_BYTES: u8 = 7;
pub(super) const KIND_ENUM: u8 = 8;

pub(super) const COMPLETE_FIELDS: &[(&str, u8)] = &[
    ("universe-id", KIND_U64),
    ("trace-hash", KIND_STRING),
    ("trace-events", KIND_U64),
    ("always-checks", KIND_SEQUENCE),
    ("always-failures", KIND_SEQUENCE),
    ("sometimes", KIND_MAP),
    ("lifecycle", KIND_STRUCT),
    ("fault-plan-digest", KIND_OPTION),
    ("runtime-evidence", KIND_OPTION),
    ("schedule-policy", KIND_ENUM),
    ("decision-tape-digest", KIND_OPTION),
    ("end-state-identity", KIND_BYTES),
];

#[derive(Debug, Clone, PartialEq, Eq)]
struct CanonicalIdentity {
    bytes: Vec<u8>,
}

/// Injective canonical representation of the exact map read by end-state
/// oracles. Map entries are ordered by key and duplicate keys are rejected by
/// the decoder.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EndStateIdentity(CanonicalIdentity);

impl EndStateIdentity {
    pub fn algorithm(&self) -> &'static str {
        CANONICAL_IDENTITY_ALGORITHM
    }

    pub fn schema(&self) -> &'static str {
        END_STATE_IDENTITY_SCHEMA
    }

    pub fn canonical_bytes(&self) -> &[u8] {
        &self.0.bytes
    }

    pub(crate) fn from_end_state(state: &EndState) -> Self {
        let mut map = Writer::new();
        map.u64(state.len() as u64);
        for (key, value) in state {
            map.string(key);
            map.string(value);
        }
        Self(CanonicalIdentity {
            bytes: envelope(
                END_STATE_IDENTITY_SCHEMA,
                vec![Field::new("state", KIND_MAP, map.finish())],
            ),
        })
    }
}

/// Injective canonical representation of every public universe observable.
/// Struct equality remains the divergence definition; these bytes provide the
/// versioned transport identity consumed by later evidence schemas.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CompleteObservationIdentity(CanonicalIdentity);

impl CompleteObservationIdentity {
    pub fn algorithm(&self) -> &'static str {
        CANONICAL_IDENTITY_ALGORITHM
    }

    pub fn schema(&self) -> &'static str {
        COMPLETE_OBSERVATION_IDENTITY_SCHEMA
    }

    pub fn canonical_bytes(&self) -> &[u8] {
        &self.0.bytes
    }
}

pub(crate) struct ObservationSource<'a> {
    pub universe_id: u64,
    pub trace_hash: &'a str,
    pub trace_events: usize,
    pub always_checks: &'a [AlwaysCheck],
    pub always_failures: &'a [AlwaysFailure],
    pub sometimes: &'a BTreeMap<String, bool>,
    pub lifecycle: &'a UniverseLifecycle,
    pub fault_plan_digest: Option<&'a str>,
    pub runtime_evidence: Option<&'a RuntimeEvidence>,
    pub schedule_policy: SchedulePolicy,
    pub decision_tape_digest: Option<&'a str>,
    pub end_state_identity: &'a EndStateIdentity,
}

pub(crate) fn complete_identity(source: ObservationSource<'_>) -> CompleteObservationIdentity {
    let fields = vec![
        Field::u64("universe-id", source.universe_id),
        Field::string("trace-hash", source.trace_hash),
        Field::u64("trace-events", source.trace_events as u64),
        Field::new(
            "always-checks",
            KIND_SEQUENCE,
            encode_always_checks(source.always_checks),
        ),
        Field::new(
            "always-failures",
            KIND_SEQUENCE,
            encode_always_failures(source.always_failures),
        ),
        Field::new("sometimes", KIND_MAP, encode_sometimes(source.sometimes)),
        Field::new("lifecycle", KIND_STRUCT, encode_lifecycle(source.lifecycle)),
        Field::new(
            "fault-plan-digest",
            KIND_OPTION,
            encode_option_string(source.fault_plan_digest),
        ),
        Field::new(
            "runtime-evidence",
            KIND_OPTION,
            encode_runtime_evidence(source.runtime_evidence),
        ),
        Field::new(
            "schedule-policy",
            KIND_ENUM,
            encode_schedule_policy(source.schedule_policy),
        ),
        Field::new(
            "decision-tape-digest",
            KIND_OPTION,
            encode_option_string(source.decision_tape_digest),
        ),
        Field::bytes(
            "end-state-identity",
            source.end_state_identity.canonical_bytes(),
        ),
    ];
    CompleteObservationIdentity(CanonicalIdentity {
        bytes: envelope(COMPLETE_OBSERVATION_IDENTITY_SCHEMA, fields),
    })
}

#[derive(Debug, Clone)]
pub(super) struct Field {
    pub name: String,
    pub kind: u8,
    pub payload: Vec<u8>,
}

impl Field {
    fn new(name: &str, kind: u8, payload: Vec<u8>) -> Self {
        Self {
            name: name.to_string(),
            kind,
            payload,
        }
    }

    fn u64(name: &str, value: u64) -> Self {
        let mut w = Writer::new();
        w.u64(value);
        Self::new(name, KIND_U64, w.finish())
    }

    fn string(name: &str, value: &str) -> Self {
        let mut w = Writer::new();
        w.string(value);
        Self::new(name, KIND_STRING, w.finish())
    }

    fn bytes(name: &str, value: &[u8]) -> Self {
        let mut w = Writer::new();
        w.bytes(value);
        Self::new(name, KIND_BYTES, w.finish())
    }
}

pub(super) fn envelope(schema: &str, fields: Vec<Field>) -> Vec<u8> {
    let mut w = Writer::new();
    w.raw(MAGIC);
    w.string(CANONICAL_IDENTITY_ALGORITHM);
    w.string(schema);
    w.u64(fields.len() as u64);
    for field in fields {
        w.string(&field.name);
        w.byte(field.kind);
        w.bytes(&field.payload);
    }
    w.finish()
}

#[derive(Default)]
pub(super) struct Writer {
    bytes: Vec<u8>,
}

impl Writer {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn byte(&mut self, value: u8) {
        self.bytes.push(value);
    }

    pub fn raw(&mut self, value: &[u8]) {
        self.bytes.extend_from_slice(value);
    }

    pub fn u64(&mut self, value: u64) {
        self.raw(&value.to_le_bytes());
    }

    pub fn bytes(&mut self, value: &[u8]) {
        self.u64(value.len() as u64);
        self.raw(value);
    }

    pub fn string(&mut self, value: &str) {
        self.bytes(value.as_bytes());
    }

    pub fn item(&mut self, item: Vec<u8>) {
        self.bytes(&item);
    }

    pub fn finish(self) -> Vec<u8> {
        self.bytes
    }
}

fn encode_always_checks(checks: &[AlwaysCheck]) -> Vec<u8> {
    let mut out = Writer::new();
    out.u64(checks.len() as u64);
    for check in checks {
        let mut item = Writer::new();
        item.string(&check.name);
        item.byte(u8::from(check.passed));
        out.item(item.finish());
    }
    out.finish()
}

fn encode_always_failures(failures: &[AlwaysFailure]) -> Vec<u8> {
    let mut out = Writer::new();
    out.u64(failures.len() as u64);
    for failure in failures {
        let mut item = Writer::new();
        item.string(&failure.name);
        item.string(&failure.detail);
        out.item(item.finish());
    }
    out.finish()
}

fn encode_sometimes(values: &BTreeMap<String, bool>) -> Vec<u8> {
    let mut out = Writer::new();
    out.u64(values.len() as u64);
    for (name, reached) in values {
        out.string(name);
        out.byte(u8::from(*reached));
    }
    out.finish()
}

fn encode_option_string(value: Option<&str>) -> Vec<u8> {
    let mut out = Writer::new();
    match value {
        None => out.byte(0),
        Some(value) => {
            out.byte(1);
            out.string(value);
        }
    }
    out.finish()
}

fn encode_lifecycle(value: &UniverseLifecycle) -> Vec<u8> {
    let mut out = Writer::new();
    match value.outcome() {
        RunOutcome::Completed => out.byte(0),
        RunOutcome::InvalidAssumption(detail) => {
            out.byte(1);
            out.string(detail);
        }
        RunOutcome::ExecutionError(detail) => {
            out.byte(2);
            out.string(detail);
        }
    }
    match value.fault_plan() {
        FaultPlanDiscipline::SelfGenerated { retrievals } => {
            out.byte(0);
            out.u64(*retrievals);
        }
        FaultPlanDiscipline::OverrideRetrieved => out.byte(1),
        FaultPlanDiscipline::OverrideNeverRetrieved => out.byte(2),
        FaultPlanDiscipline::OverrideRetrievedMultiply { retrievals } => {
            out.byte(3);
            out.u64(*retrievals);
        }
    }
    out.finish()
}

fn encode_runtime_evidence(value: Option<&RuntimeEvidence>) -> Vec<u8> {
    let mut out = Writer::new();
    let Some(value) = value else {
        out.byte(0);
        return out.finish();
    };
    out.byte(1);
    out.u64(value.injections().len() as u64);
    for injection in value.injections() {
        let mut item = Writer::new();
        item.u64(injection.at_nanos());
        item.string(injection.fault());
        for stage in [
            injection.offered_at(),
            injection.armed_at(),
            injection.injected_at(),
            injection.manifested_at(),
            injection.recovered_at(),
        ] {
            match stage {
                None => item.byte(0),
                Some(at) => {
                    item.byte(1);
                    item.u64(at);
                }
            }
        }
        out.item(item.finish());
    }
    out.finish()
}

fn encode_schedule_policy(value: SchedulePolicy) -> Vec<u8> {
    let mut out = Writer::new();
    match value {
        SchedulePolicy::Fifo => out.byte(0),
        SchedulePolicy::Pct { depth } => {
            out.byte(1);
            out.u64(depth);
        }
        SchedulePolicy::UniformTiebreak => out.byte(2),
    }
    out.finish()
}
