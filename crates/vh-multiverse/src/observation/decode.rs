//! Strict decoders for canonical observation bytes.

use std::collections::BTreeMap;
use std::fmt;

use vh_props::EndState;

use super::{
    CANONICAL_IDENTITY_ALGORITHM, COMPLETE_FIELDS, COMPLETE_OBSERVATION_IDENTITY_SCHEMA,
    END_STATE_IDENTITY_SCHEMA, KIND_BYTES, KIND_ENUM, KIND_MAP, KIND_OPTION, KIND_SEQUENCE,
    KIND_STRING, KIND_STRUCT, KIND_U64, MAGIC,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DecodeError {
    Truncated,
    InvalidMagic,
    InvalidUtf8,
    WrongAlgorithm,
    WrongSchema,
    WrongFieldCount,
    UnexpectedField,
    UnexpectedKind,
    InvalidValue,
    NonCanonicalMap,
    TrailingBytes,
}

impl fmt::Display for DecodeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let message = match self {
            Self::Truncated => "truncated canonical observation",
            Self::InvalidMagic => "invalid canonical observation magic",
            Self::InvalidUtf8 => "canonical string is not UTF-8",
            Self::WrongAlgorithm => "canonical identity algorithm mismatch",
            Self::WrongSchema => "canonical observation schema mismatch",
            Self::WrongFieldCount => "canonical observation field count mismatch",
            Self::UnexpectedField => "duplicate, unknown, or reordered canonical field",
            Self::UnexpectedKind => "canonical field kind mismatch",
            Self::InvalidValue => "invalid or noncanonical field value",
            Self::NonCanonicalMap => "map keys are duplicated or not strictly ordered",
            Self::TrailingBytes => "trailing bytes after canonical observation",
        };
        f.write_str(message)
    }
}

impl std::error::Error for DecodeError {}

/// Decode and validate the raw end-state identity. The returned map is the
/// exact oracle input represented by the bytes.
pub fn decode_end_state(bytes: &[u8]) -> Result<EndState, DecodeError> {
    let mut envelope = Envelope::new(bytes, END_STATE_IDENTITY_SCHEMA, 1)?;
    let mut field = envelope.field("state", KIND_MAP)?;
    let count = field.u64()?;
    let mut result = BTreeMap::new();
    let mut previous: Option<String> = None;
    for _ in 0..count {
        let key = field.string()?;
        if previous.as_ref().is_some_and(|p| p >= &key) {
            return Err(DecodeError::NonCanonicalMap);
        }
        let value = field.string()?;
        previous = Some(key.clone());
        result.insert(key, value);
    }
    field.end()?;
    envelope.end()?;
    Ok(result)
}

/// Strictly validate the complete observation envelope and every nested
/// value. Unknown, duplicate, reordered, truncated, trailing, and
/// noncanonical-map encodings are rejected.
pub fn validate_complete_observation(bytes: &[u8]) -> Result<(), DecodeError> {
    let mut envelope = Envelope::new(
        bytes,
        COMPLETE_OBSERVATION_IDENTITY_SCHEMA,
        COMPLETE_FIELDS.len() as u64,
    )?;

    envelope.field("universe-id", KIND_U64)?.exact_u64()?;
    envelope.field("trace-hash", KIND_STRING)?.exact_string()?;
    envelope.field("trace-events", KIND_U64)?.exact_u64()?;
    validate_checks(envelope.field("always-checks", KIND_SEQUENCE)?)?;
    validate_failures(envelope.field("always-failures", KIND_SEQUENCE)?)?;
    validate_sometimes(envelope.field("sometimes", KIND_MAP)?)?;
    validate_lifecycle(envelope.field("lifecycle", KIND_STRUCT)?)?;
    validate_option_string(envelope.field("fault-plan-digest", KIND_OPTION)?)?;
    validate_runtime(envelope.field("runtime-evidence", KIND_OPTION)?)?;
    validate_policy(envelope.field("schedule-policy", KIND_ENUM)?)?;
    validate_option_string(envelope.field("decision-tape-digest", KIND_OPTION)?)?;

    let mut state = envelope.field("end-state-identity", KIND_BYTES)?;
    let state_bytes = state.bytes()?;
    state.end()?;
    decode_end_state(state_bytes)?;
    envelope.end()
}

fn validate_checks(mut value: Reader<'_>) -> Result<(), DecodeError> {
    let count = value.u64()?;
    for _ in 0..count {
        let mut item = value.framed()?;
        item.string()?;
        item.boolean()?;
        item.end()?;
    }
    value.end()
}

fn validate_failures(mut value: Reader<'_>) -> Result<(), DecodeError> {
    let count = value.u64()?;
    for _ in 0..count {
        let mut item = value.framed()?;
        item.string()?;
        item.string()?;
        item.end()?;
    }
    value.end()
}

fn validate_sometimes(mut value: Reader<'_>) -> Result<(), DecodeError> {
    let count = value.u64()?;
    let mut previous: Option<String> = None;
    for _ in 0..count {
        let key = value.string()?;
        if previous.as_ref().is_some_and(|p| p >= &key) {
            return Err(DecodeError::NonCanonicalMap);
        }
        value.boolean()?;
        previous = Some(key);
    }
    value.end()
}

fn validate_lifecycle(mut value: Reader<'_>) -> Result<(), DecodeError> {
    match value.byte()? {
        0 => {}
        1 | 2 => {
            value.string()?;
        }
        _ => return Err(DecodeError::InvalidValue),
    }
    match value.byte()? {
        0 | 3 => {
            value.u64()?;
        }
        1 | 2 => {}
        _ => return Err(DecodeError::InvalidValue),
    }
    value.end()
}

fn validate_option_string(mut value: Reader<'_>) -> Result<(), DecodeError> {
    match value.byte()? {
        0 => {}
        1 => {
            value.string()?;
        }
        _ => return Err(DecodeError::InvalidValue),
    }
    value.end()
}

fn validate_runtime(mut value: Reader<'_>) -> Result<(), DecodeError> {
    match value.byte()? {
        0 => return value.end(),
        1 => {}
        _ => return Err(DecodeError::InvalidValue),
    }
    let count = value.u64()?;
    for _ in 0..count {
        let mut item = value.framed()?;
        item.u64()?;
        item.string()?;
        for _ in 0..5 {
            item.option_u64()?;
        }
        item.end()?;
    }
    value.end()
}

fn validate_policy(mut value: Reader<'_>) -> Result<(), DecodeError> {
    match value.byte()? {
        0 | 2 => {}
        1 => {
            value.u64()?;
        }
        _ => return Err(DecodeError::InvalidValue),
    }
    value.end()
}

struct Envelope<'a> {
    reader: Reader<'a>,
    remaining_fields: u64,
}

impl<'a> Envelope<'a> {
    fn new(bytes: &'a [u8], schema: &str, field_count: u64) -> Result<Self, DecodeError> {
        let mut reader = Reader::new(bytes);
        if reader.take(MAGIC.len())? != MAGIC {
            return Err(DecodeError::InvalidMagic);
        }
        if reader.string()? != CANONICAL_IDENTITY_ALGORITHM {
            return Err(DecodeError::WrongAlgorithm);
        }
        if reader.string()? != schema {
            return Err(DecodeError::WrongSchema);
        }
        let actual = reader.u64()?;
        if actual != field_count {
            return Err(DecodeError::WrongFieldCount);
        }
        Ok(Self {
            reader,
            remaining_fields: actual,
        })
    }

    fn field(&mut self, name: &str, kind: u8) -> Result<Reader<'a>, DecodeError> {
        if self.remaining_fields == 0 {
            return Err(DecodeError::WrongFieldCount);
        }
        if self.reader.string()? != name {
            return Err(DecodeError::UnexpectedField);
        }
        if self.reader.byte()? != kind {
            return Err(DecodeError::UnexpectedKind);
        }
        self.remaining_fields -= 1;
        self.reader.framed()
    }

    fn end(self) -> Result<(), DecodeError> {
        if self.remaining_fields != 0 {
            return Err(DecodeError::WrongFieldCount);
        }
        self.reader.end()
    }
}

struct Reader<'a> {
    bytes: &'a [u8],
    cursor: usize,
}

impl<'a> Reader<'a> {
    fn new(bytes: &'a [u8]) -> Self {
        Self { bytes, cursor: 0 }
    }

    fn remaining(&self) -> usize {
        self.bytes.len().saturating_sub(self.cursor)
    }

    fn take(&mut self, len: usize) -> Result<&'a [u8], DecodeError> {
        if len > self.remaining() {
            return Err(DecodeError::Truncated);
        }
        let end = self.cursor.checked_add(len).ok_or(DecodeError::Truncated)?;
        let value = &self.bytes[self.cursor..end];
        self.cursor = end;
        Ok(value)
    }

    fn byte(&mut self) -> Result<u8, DecodeError> {
        Ok(self.take(1)?[0])
    }

    fn u64(&mut self) -> Result<u64, DecodeError> {
        let bytes: [u8; 8] = self
            .take(8)?
            .try_into()
            .map_err(|_| DecodeError::Truncated)?;
        Ok(u64::from_le_bytes(bytes))
    }

    fn bytes(&mut self) -> Result<&'a [u8], DecodeError> {
        let len = usize::try_from(self.u64()?).map_err(|_| DecodeError::Truncated)?;
        self.take(len)
    }

    fn string(&mut self) -> Result<String, DecodeError> {
        let value = std::str::from_utf8(self.bytes()?).map_err(|_| DecodeError::InvalidUtf8)?;
        Ok(value.to_string())
    }

    fn framed(&mut self) -> Result<Reader<'a>, DecodeError> {
        Ok(Reader::new(self.bytes()?))
    }

    fn boolean(&mut self) -> Result<bool, DecodeError> {
        match self.byte()? {
            0 => Ok(false),
            1 => Ok(true),
            _ => Err(DecodeError::InvalidValue),
        }
    }

    fn option_u64(&mut self) -> Result<Option<u64>, DecodeError> {
        match self.byte()? {
            0 => Ok(None),
            1 => Ok(Some(self.u64()?)),
            _ => Err(DecodeError::InvalidValue),
        }
    }

    fn exact_u64(mut self) -> Result<u64, DecodeError> {
        let value = self.u64()?;
        self.end()?;
        Ok(value)
    }

    fn exact_string(mut self) -> Result<String, DecodeError> {
        let value = self.string()?;
        self.end()?;
        Ok(value)
    }

    fn end(self) -> Result<(), DecodeError> {
        if self.remaining() == 0 {
            Ok(())
        } else {
            Err(DecodeError::TrailingBytes)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::observation::{envelope, Field, Writer};

    fn empty_end_state() -> Vec<u8> {
        let mut map = Writer::new();
        map.u64(0);
        envelope(
            END_STATE_IDENTITY_SCHEMA,
            vec![Field::new("state", KIND_MAP, map.finish())],
        )
    }

    fn minimal_complete_fields() -> Vec<Field> {
        let mut empty_sequence = Writer::new();
        empty_sequence.u64(0);
        let empty_sequence = empty_sequence.finish();
        let mut empty_map = Writer::new();
        empty_map.u64(0);
        let mut lifecycle = Writer::new();
        lifecycle.byte(0);
        lifecycle.byte(0);
        lifecycle.u64(0);
        let mut none = Writer::new();
        none.byte(0);
        let none = none.finish();
        let mut fifo = Writer::new();
        fifo.byte(0);
        vec![
            Field::u64("universe-id", 0),
            Field::string("trace-hash", "x"),
            Field::u64("trace-events", 0),
            Field::new("always-checks", KIND_SEQUENCE, empty_sequence.clone()),
            Field::new("always-failures", KIND_SEQUENCE, empty_sequence),
            Field::new("sometimes", KIND_MAP, empty_map.finish()),
            Field::new("lifecycle", KIND_STRUCT, lifecycle.finish()),
            Field::new("fault-plan-digest", KIND_OPTION, none.clone()),
            Field::new("runtime-evidence", KIND_OPTION, none.clone()),
            Field::new("schedule-policy", KIND_ENUM, fifo.finish()),
            Field::new("decision-tape-digest", KIND_OPTION, none),
            Field::bytes("end-state-identity", &empty_end_state()),
        ]
    }

    #[test]
    fn strict_parser_rejects_duplicate_unknown_reordered_and_trailing_fields() {
        let valid = minimal_complete_fields();
        let bytes = envelope(COMPLETE_OBSERVATION_IDENTITY_SCHEMA, valid.clone());
        assert_eq!(validate_complete_observation(&bytes), Ok(()));

        let mut duplicate = valid.clone();
        duplicate[1] = duplicate[0].clone();
        assert_eq!(
            validate_complete_observation(&envelope(
                COMPLETE_OBSERVATION_IDENTITY_SCHEMA,
                duplicate
            )),
            Err(DecodeError::UnexpectedField)
        );

        let mut unknown = valid.clone();
        unknown[0].name = "not-a-field".to_string();
        assert_eq!(
            validate_complete_observation(&envelope(COMPLETE_OBSERVATION_IDENTITY_SCHEMA, unknown)),
            Err(DecodeError::UnexpectedField)
        );

        let mut reordered = valid;
        reordered.swap(0, 1);
        assert_eq!(
            validate_complete_observation(&envelope(
                COMPLETE_OBSERVATION_IDENTITY_SCHEMA,
                reordered
            )),
            Err(DecodeError::UnexpectedField)
        );

        let mut trailing = bytes;
        trailing.push(0);
        assert_eq!(
            validate_complete_observation(&trailing),
            Err(DecodeError::TrailingBytes)
        );
    }

    #[test]
    fn strict_parser_rejects_duplicate_reordered_truncated_and_malformed_state() {
        let mut duplicate = Writer::new();
        duplicate.u64(2);
        duplicate.string("a");
        duplicate.string("1");
        duplicate.string("a");
        duplicate.string("2");
        let duplicate = envelope(
            END_STATE_IDENTITY_SCHEMA,
            vec![Field::new("state", KIND_MAP, duplicate.finish())],
        );
        assert_eq!(
            decode_end_state(&duplicate),
            Err(DecodeError::NonCanonicalMap)
        );

        let mut reordered = Writer::new();
        reordered.u64(2);
        reordered.string("b");
        reordered.string("1");
        reordered.string("a");
        reordered.string("2");
        let reordered = envelope(
            END_STATE_IDENTITY_SCHEMA,
            vec![Field::new("state", KIND_MAP, reordered.finish())],
        );
        assert_eq!(
            decode_end_state(&reordered),
            Err(DecodeError::NonCanonicalMap)
        );

        let valid = empty_end_state();
        for end in 0..valid.len() {
            assert!(decode_end_state(&valid[..end]).is_err());
        }
        let mut malformed = valid;
        malformed[0] ^= 0xff;
        assert_eq!(decode_end_state(&malformed), Err(DecodeError::InvalidMagic));
    }

    #[test]
    fn malformed_probe_never_panics() {
        let mut state = 0xD1CE_5EED_u64;
        for case in 0..2056_usize {
            state = state
                .wrapping_mul(6_364_136_223_846_793_005)
                .wrapping_add(1);
            let len = (state as usize ^ case) % 128;
            let mut bytes = Vec::with_capacity(len);
            for _ in 0..len {
                state = state.rotate_left(9).wrapping_add(0x9e37_79b9);
                bytes.push(state as u8);
            }
            let _ = decode_end_state(&bytes);
            let _ = validate_complete_observation(&bytes);
        }
    }
}
