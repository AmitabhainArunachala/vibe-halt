//! Cassette v2 (post-audit C5, audit E.2): ordered request/effect
//! history with a canonical child-visible framing — the PURE codec.
//! All transport I/O lives in `lib.rs` (this crate's declared boundary
//! exemption); this module is deterministic byte manipulation only.
//!
//! History law: entries are an ORDERED list, consumed strictly in
//! sequence. Request N must exactly match entry N's recorded request —
//! repeated identical requests therefore consume distinct ordered
//! entries, and a one-key map overwrite cannot exist. Exact-match-or-
//! miss: there is no fuzzy matching, no live fallback, no capture in
//! evidence mode. A miss taints the run UNCHECKED — missing evidence is
//! never a target defect (FINDINGS) and never silent success.
//!
//! Framing: every variable-length value is length-prefixed
//! (`<decimal-len>:<raw bytes>`), so message content may contain any
//! bytes including newlines; the parser reads exact counts and rejects
//! everything malformed, truncated, or trailing. One frame format is
//! shared by the cassette file, the child's request frames, and the
//! broker's response frames — small, auditable, deterministic.

use std::collections::BTreeMap;
use std::fmt;

/// Cassette schema (ordered history). v1 (`vh-cassette-v1`) remains the
/// legacy parent-side demo format in `lib.rs`.
pub const CASSETTE_SCHEMA_V2: &str = "vh-cassette-v2";
/// Child-visible framing contract version, bound into run records.
pub const TRANSPORT_SCHEMA: &str = "vh-cassette-transport-v1";

/// Typed, fail-closed rejection from the cassette/request framing parser.
///
/// Frames can cross a child-process trust boundary. Keeping parse failures
/// structured lets callers distinguish a malformed shape, impossible
/// cardinality, arithmetic overflow, and ordinary truncation without ever
/// allocating from an untrusted count first.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CassetteParseError {
    UnterminatedLine,
    InvalidUtf8 {
        context: String,
    },
    UnexpectedLine {
        expected: String,
        actual: String,
    },
    UnexpectedField {
        field: String,
    },
    MissingFieldSpace {
        field: String,
    },
    MalformedLength {
        field: String,
    },
    LengthOverflow {
        field: String,
        value: String,
    },
    LengthArithmeticOverflow {
        field: String,
        length: usize,
    },
    TruncatedField {
        field: String,
        declared: usize,
        available: usize,
    },
    FieldNotNewlineTerminated {
        field: String,
    },
    ExpectedCount {
        field: String,
        actual: String,
    },
    CountOverflow {
        field: String,
        value: String,
    },
    CountExceedsFrame {
        field: String,
        count: usize,
        remaining: usize,
        minimum_per_item: usize,
    },
    InvalidNumber {
        field: String,
        value: String,
    },
    DuplicateParam {
        key: String,
    },
    UnknownTapeEntry {
        head: String,
    },
    UnsupportedCassette {
        head: String,
    },
    TrailingBytes {
        consumed: usize,
        total: usize,
    },
}

impl fmt::Display for CassetteParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnterminatedLine => write!(f, "unterminated line"),
            Self::InvalidUtf8 { context } => write!(f, "{context} not utf-8"),
            Self::UnexpectedLine { expected, actual } => {
                write!(f, "expected line {expected:?}, got {actual:?}")
            }
            Self::UnexpectedField { field } => write!(f, "expected field {field:?}"),
            Self::MissingFieldSpace { field } => {
                write!(f, "expected space after field tag {field:?}")
            }
            Self::MalformedLength { field } => {
                write!(f, "malformed length for field {field:?}")
            }
            Self::LengthOverflow { field, value } => {
                write!(f, "length overflow for field {field:?}: {value:?}")
            }
            Self::LengthArithmeticOverflow { field, length } => write!(
                f,
                "length arithmetic overflow for field {field:?}: declared {length}"
            ),
            Self::TruncatedField {
                field,
                declared,
                available,
            } => write!(
                f,
                "truncated field {field:?}: declared {declared} byte(s), only {available} available"
            ),
            Self::FieldNotNewlineTerminated { field } => {
                write!(f, "field {field:?} not newline-terminated")
            }
            Self::ExpectedCount { field, actual } => {
                write!(f, "expected count line for {field:?}, got {actual:?}")
            }
            Self::CountOverflow { field, value } => {
                write!(f, "bad count for {field:?}: {value:?}")
            }
            Self::CountExceedsFrame {
                field,
                count,
                remaining,
                minimum_per_item,
            } => write!(
                f,
                "count for {field:?} ({count}) cannot fit in remaining frame \
                 ({remaining} byte(s), minimum {minimum_per_item} per item)"
            ),
            Self::InvalidNumber { field, value } => {
                write!(f, "bad number for {field:?}: {value:?}")
            }
            Self::DuplicateParam { key } => write!(f, "duplicate param {key:?}"),
            Self::UnknownTapeEntry { head } => {
                write!(f, "unknown tape entry head {head:?}")
            }
            Self::UnsupportedCassette { head } => {
                write!(f, "unsupported cassette head {head:?}")
            }
            Self::TrailingBytes { consumed, total } => write!(
                f,
                "trailing bytes after frame ({consumed} of {total} consumed)"
            ),
        }
    }
}

impl std::error::Error for CassetteParseError {}

const fn empty_field_bytes(tag: &str) -> usize {
    // `<tag> 0:\n`
    tag.len() + 4
}

const MIN_MESSAGE_BYTES: usize = empty_field_bytes("role") + empty_field_bytes("content");
const MIN_TOOL_BYTES: usize = empty_field_bytes("tool-name") + empty_field_bytes("tool-schema");
const MIN_PARAM_BYTES: usize = empty_field_bytes("param-key") + empty_field_bytes("param-value");
const MIN_STREAM_CHUNK_BYTES: usize = empty_field_bytes("chunk");
const MIN_CASSETTE_ENTRY_BYTES: usize =
    empty_field_bytes("request") + empty_field_bytes("response");

/// Canonical child-visible LLM request. Field order is fixed; every
/// behaviorally relevant field participates in the canonical bytes.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct LlmRequestV2 {
    pub provider: String,
    pub model: String,
    /// Ordered `(role, content)` messages.
    pub messages: Vec<(String, String)>,
    /// Ordered `(name, json-schema)` tool declarations.
    pub tools: Vec<(String, String)>,
    pub tool_choice: Option<String>,
    /// Structured-output / response-format parameter, verbatim.
    pub structured_output: Option<String>,
    /// Sampling and remaining behaviorally relevant parameters, sorted.
    pub params: BTreeMap<String, String>,
}

impl LlmRequestV2 {
    /// Canonical bytes — the exact frame a child transmits and the exact
    /// bytes the request digest covers.
    pub fn canonical_bytes(&self) -> Vec<u8> {
        let mut w = FrameWriter::new();
        w.line("vh-llm-request-v2");
        w.field("provider", self.provider.as_bytes());
        w.field("model", self.model.as_bytes());
        w.count("messages", self.messages.len());
        for (role, content) in &self.messages {
            w.field("role", role.as_bytes());
            w.field("content", content.as_bytes());
        }
        w.count("tools", self.tools.len());
        for (name, schema) in &self.tools {
            w.field("tool-name", name.as_bytes());
            w.field("tool-schema", schema.as_bytes());
        }
        w.opt_field("tool-choice", self.tool_choice.as_deref());
        w.opt_field("structured-output", self.structured_output.as_deref());
        w.count("params", self.params.len());
        for (k, v) in &self.params {
            w.field("param-key", k.as_bytes());
            w.field("param-value", v.as_bytes());
        }
        w.finish()
    }

    /// SHA-256 over the canonical bytes — the exact-match key.
    pub fn digest(&self) -> String {
        vh_digest::sha256_hex(&self.canonical_bytes())
    }

    pub fn parse(bytes: &[u8]) -> Result<LlmRequestV2, CassetteParseError> {
        let mut r = FrameReader::new(bytes);
        r.expect_line("vh-llm-request-v2")?;
        let provider = r.field_string("provider")?;
        let model = r.field_string("model")?;
        let n_messages = r.bounded_count("messages", MIN_MESSAGE_BYTES)?;
        let mut messages = Vec::with_capacity(n_messages);
        for _ in 0..n_messages {
            messages.push((r.field_string("role")?, r.field_string("content")?));
        }
        let n_tools = r.bounded_count("tools", MIN_TOOL_BYTES)?;
        let mut tools = Vec::with_capacity(n_tools);
        for _ in 0..n_tools {
            tools.push((r.field_string("tool-name")?, r.field_string("tool-schema")?));
        }
        let tool_choice = r.opt_field_string("tool-choice")?;
        let structured_output = r.opt_field_string("structured-output")?;
        let n_params = r.bounded_count("params", MIN_PARAM_BYTES)?;
        let mut params = BTreeMap::new();
        for _ in 0..n_params {
            let k = r.field_string("param-key")?;
            let v = r.field_string("param-value")?;
            if params.insert(k.clone(), v).is_some() {
                return Err(CassetteParseError::DuplicateParam { key: k });
            }
        }
        r.expect_end()?;
        Ok(LlmRequestV2 {
            provider,
            model,
            messages,
            tools,
            tool_choice,
            structured_output,
            params,
        })
    }
}

/// The recorded effect a request replays to. Streams carry exact chunks
/// in exact order plus the terminal frame; errors and timeouts are
/// first-class recorded outcomes, never synthesized.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TapeEntry {
    Success {
        status: u16,
        body: Vec<u8>,
    },
    ProviderError {
        status: u16,
        body: Vec<u8>,
    },
    Timeout,
    Stream {
        chunks: Vec<Vec<u8>>,
        terminal: String,
    },
}

impl TapeEntry {
    /// The exact response-frame bytes the child receives.
    pub fn response_frame(&self) -> Vec<u8> {
        let mut w = FrameWriter::new();
        match self {
            TapeEntry::Success { status, body } => {
                w.line(&format!("success {status}"));
                w.field("body", body);
            }
            TapeEntry::ProviderError { status, body } => {
                w.line(&format!("provider-error {status}"));
                w.field("body", body);
            }
            TapeEntry::Timeout => w.line("timeout"),
            TapeEntry::Stream { chunks, terminal } => {
                w.line(&format!("stream {}", chunks.len()));
                for chunk in chunks {
                    w.field("chunk", chunk);
                }
                w.field("terminal", terminal.as_bytes());
            }
        }
        w.finish()
    }

    pub fn parse(bytes: &[u8]) -> Result<TapeEntry, CassetteParseError> {
        let mut r = FrameReader::new(bytes);
        let head = r.take_line()?;
        let entry = if let Some(status) = head.strip_prefix("success ") {
            TapeEntry::Success {
                status: status
                    .parse()
                    .map_err(|_| CassetteParseError::InvalidNumber {
                        field: "status".to_string(),
                        value: status.to_string(),
                    })?,
                body: r.field_bytes("body")?,
            }
        } else if let Some(status) = head.strip_prefix("provider-error ") {
            TapeEntry::ProviderError {
                status: status
                    .parse()
                    .map_err(|_| CassetteParseError::InvalidNumber {
                        field: "status".to_string(),
                        value: status.to_string(),
                    })?,
                body: r.field_bytes("body")?,
            }
        } else if head == "timeout" {
            TapeEntry::Timeout
        } else if let Some(n) = head.strip_prefix("stream ") {
            let n: usize = n.parse().map_err(|_| CassetteParseError::CountOverflow {
                field: "stream chunks".to_string(),
                value: n.to_string(),
            })?;
            r.ensure_count_fits("stream chunks", n, MIN_STREAM_CHUNK_BYTES)?;
            let mut chunks = Vec::with_capacity(n);
            for _ in 0..n {
                chunks.push(r.field_bytes("chunk")?);
            }
            TapeEntry::Stream {
                chunks,
                terminal: String::from_utf8(r.field_bytes("terminal")?).map_err(|_| {
                    CassetteParseError::InvalidUtf8 {
                        context: "terminal".to_string(),
                    }
                })?,
            }
        } else {
            return Err(CassetteParseError::UnknownTapeEntry { head });
        };
        r.expect_end()?;
        Ok(entry)
    }
}

/// Ordered cassette: the persistent, versioned request/effect history.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct CassetteV2 {
    entries: Vec<(LlmRequestV2, TapeEntry)>,
}

impl CassetteV2 {
    pub fn push(&mut self, request: LlmRequestV2, entry: TapeEntry) {
        self.entries.push((request, entry));
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    pub fn entry(&self, index: usize) -> Option<&(LlmRequestV2, TapeEntry)> {
        self.entries.get(index)
    }

    /// Deterministic persistent file bytes.
    pub fn file_bytes(&self) -> Vec<u8> {
        let mut w = FrameWriter::new();
        w.line(&format!("{CASSETTE_SCHEMA_V2} {}", self.entries.len()));
        for (request, entry) in &self.entries {
            w.field("request", &request.canonical_bytes());
            w.field("response", &entry.response_frame());
        }
        w.finish()
    }

    pub fn parse(bytes: &[u8]) -> Result<CassetteV2, CassetteParseError> {
        let mut r = FrameReader::new(bytes);
        let head = r.take_line()?;
        let count = head
            .strip_prefix(&format!("{CASSETTE_SCHEMA_V2} "))
            .ok_or_else(|| CassetteParseError::UnsupportedCassette { head: head.clone() })?;
        let n: usize = count
            .parse()
            .map_err(|_| CassetteParseError::CountOverflow {
                field: "cassette entries".to_string(),
                value: count.to_string(),
            })?;
        r.ensure_count_fits("cassette entries", n, MIN_CASSETTE_ENTRY_BYTES)?;
        let mut cassette = CassetteV2::default();
        for _ in 0..n {
            let request = LlmRequestV2::parse(&r.field_bytes("request")?)?;
            let entry = TapeEntry::parse(&r.field_bytes("response")?)?;
            cassette.push(request, entry);
        }
        r.expect_end()?;
        Ok(cassette)
    }

    /// Content identity: schema tag + SHA-256 of the exact file bytes.
    pub fn identity(&self) -> String {
        format!(
            "{CASSETTE_SCHEMA_V2}:sha256:{}",
            vh_digest::sha256_hex(&self.file_bytes())
        )
    }
}

/// Broker-side transport receipt, bound into the run record: everything
/// that happened on the tape, in order, plus the taint that keeps a
/// broken transcript from ever reading as success.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct TransportReceipt {
    /// Digests of requests served, in service order.
    pub served: Vec<String>,
    /// Recorded entries never consumed by the child (complete
    /// consumption is required: leftovers taint).
    pub unconsumed: u64,
    /// First transport violation, if any: miss, out-of-order, malformed
    /// frame, or a request after the tape was exhausted.
    pub taint: Option<String>,
}

impl TransportReceipt {
    pub fn tainted(&self) -> bool {
        self.taint.is_some() || self.unconsumed > 0
    }

    pub fn identity_str(&self) -> String {
        format!(
            "transport={TRANSPORT_SCHEMA} served={} unconsumed={} taint={}",
            self.served.join(","),
            self.unconsumed,
            self.taint.as_deref().unwrap_or("none")
        )
    }
}

// ---- length-prefixed framing primitives ----

struct FrameWriter {
    out: Vec<u8>,
}

impl FrameWriter {
    fn new() -> Self {
        FrameWriter { out: Vec::new() }
    }
    fn line(&mut self, s: &str) {
        self.out.extend_from_slice(s.as_bytes());
        self.out.push(b'\n');
    }
    fn field(&mut self, tag: &str, bytes: &[u8]) {
        self.out.extend_from_slice(tag.as_bytes());
        self.out.push(b' ');
        self.out
            .extend_from_slice(bytes.len().to_string().as_bytes());
        self.out.push(b':');
        self.out.extend_from_slice(bytes);
        self.out.push(b'\n');
    }
    fn opt_field(&mut self, tag: &str, value: Option<&str>) {
        match value {
            Some(v) => self.field(tag, v.as_bytes()),
            None => self.line(&format!("{tag} absent")),
        }
    }
    fn count(&mut self, tag: &str, n: usize) {
        self.line(&format!("{tag} {n}"));
    }
    fn finish(self) -> Vec<u8> {
        self.out
    }
}

struct FrameReader<'a> {
    bytes: &'a [u8],
    pos: usize,
}

impl<'a> FrameReader<'a> {
    fn new(bytes: &'a [u8]) -> Self {
        FrameReader { bytes, pos: 0 }
    }

    fn take_line(&mut self) -> Result<String, CassetteParseError> {
        let rest = &self.bytes[self.pos..];
        let nl = rest
            .iter()
            .position(|&b| b == b'\n')
            .ok_or(CassetteParseError::UnterminatedLine)?;
        let line = std::str::from_utf8(&rest[..nl])
            .map_err(|_| CassetteParseError::InvalidUtf8 {
                context: "line".to_string(),
            })?
            .to_string();
        self.pos += nl + 1;
        Ok(line)
    }

    fn expect_line(&mut self, expected: &str) -> Result<(), CassetteParseError> {
        let got = self.take_line()?;
        if got != expected {
            return Err(CassetteParseError::UnexpectedLine {
                expected: expected.to_string(),
                actual: got,
            });
        }
        Ok(())
    }

    fn field_bytes(&mut self, tag: &str) -> Result<Vec<u8>, CassetteParseError> {
        // `<tag> <len>:` prefix, then exactly len raw bytes, then '\n'.
        let rest = &self.bytes[self.pos..];
        let tag_bytes = tag.as_bytes();
        if rest.len() < tag_bytes.len() + 1 || &rest[..tag_bytes.len()] != tag_bytes {
            return Err(CassetteParseError::UnexpectedField {
                field: tag.to_string(),
            });
        }
        if rest[tag_bytes.len()] != b' ' {
            return Err(CassetteParseError::MissingFieldSpace {
                field: tag.to_string(),
            });
        }
        let mut idx = tag_bytes.len() + 1;
        let len_start = idx;
        while idx < rest.len() && rest[idx].is_ascii_digit() {
            idx += 1;
        }
        if idx == len_start || idx >= rest.len() || rest[idx] != b':' {
            return Err(CassetteParseError::MalformedLength {
                field: tag.to_string(),
            });
        }
        let len_text =
            std::str::from_utf8(&rest[len_start..idx]).expect("ASCII digits are valid utf-8");
        let len: usize = len_text
            .parse()
            .map_err(|_| CassetteParseError::LengthOverflow {
                field: tag.to_string(),
                value: len_text.to_string(),
            })?;
        let value_start =
            idx.checked_add(1)
                .ok_or_else(|| CassetteParseError::LengthArithmeticOverflow {
                    field: tag.to_string(),
                    length: len,
                })?;
        let value_end = value_start.checked_add(len).ok_or_else(|| {
            CassetteParseError::LengthArithmeticOverflow {
                field: tag.to_string(),
                length: len,
            }
        })?;
        let field_end = value_end.checked_add(1).ok_or_else(|| {
            CassetteParseError::LengthArithmeticOverflow {
                field: tag.to_string(),
                length: len,
            }
        })?;
        if rest.len() < field_end {
            return Err(CassetteParseError::TruncatedField {
                field: tag.to_string(),
                declared: len,
                available: rest.len().saturating_sub(value_start),
            });
        }
        let value = rest[value_start..value_end].to_vec();
        if rest[value_end] != b'\n' {
            return Err(CassetteParseError::FieldNotNewlineTerminated {
                field: tag.to_string(),
            });
        }
        self.pos += field_end;
        Ok(value)
    }

    fn field_string(&mut self, tag: &str) -> Result<String, CassetteParseError> {
        String::from_utf8(self.field_bytes(tag)?).map_err(|_| CassetteParseError::InvalidUtf8 {
            context: format!("field {tag:?}"),
        })
    }

    fn opt_field_string(&mut self, tag: &str) -> Result<Option<String>, CassetteParseError> {
        // Peek: `<tag> absent\n` or a length-prefixed field.
        let save = self.pos;
        let line = self.take_line()?;
        if line == format!("{tag} absent") {
            return Ok(None);
        }
        self.pos = save;
        Ok(Some(self.field_string(tag)?))
    }

    fn count(&mut self, tag: &str) -> Result<usize, CassetteParseError> {
        let line = self.take_line()?;
        let value = line.strip_prefix(&format!("{tag} ")).ok_or_else(|| {
            CassetteParseError::ExpectedCount {
                field: tag.to_string(),
                actual: line.clone(),
            }
        })?;
        value
            .parse()
            .map_err(|_| CassetteParseError::CountOverflow {
                field: tag.to_string(),
                value: value.to_string(),
            })
    }

    fn bounded_count(
        &mut self,
        tag: &str,
        minimum_per_item: usize,
    ) -> Result<usize, CassetteParseError> {
        let count = self.count(tag)?;
        self.ensure_count_fits(tag, count, minimum_per_item)?;
        Ok(count)
    }

    fn ensure_count_fits(
        &self,
        tag: &str,
        count: usize,
        minimum_per_item: usize,
    ) -> Result<(), CassetteParseError> {
        let remaining = self.bytes.len() - self.pos;
        if count > remaining / minimum_per_item {
            return Err(CassetteParseError::CountExceedsFrame {
                field: tag.to_string(),
                count,
                remaining,
                minimum_per_item,
            });
        }
        Ok(())
    }

    fn expect_end(&self) -> Result<(), CassetteParseError> {
        if self.pos != self.bytes.len() {
            return Err(CassetteParseError::TrailingBytes {
                consumed: self.pos,
                total: self.bytes.len(),
            });
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn request(content: &str) -> LlmRequestV2 {
        LlmRequestV2 {
            provider: "fixture".into(),
            model: "echo-2".into(),
            messages: vec![
                ("system".into(), "be deterministic".into()),
                ("user".into(), content.into()),
            ],
            tools: vec![("lookup".into(), r#"{"type":"object"}"#.into())],
            tool_choice: Some("auto".into()),
            structured_output: None,
            params: BTreeMap::from([("temperature".into(), "0".into())]),
        }
    }

    #[test]
    fn request_roundtrips_and_digest_is_stable() {
        let r = request("hello\nwith newline and 7:colons");
        let bytes = r.canonical_bytes();
        let parsed = LlmRequestV2::parse(&bytes).unwrap();
        assert_eq!(parsed, r);
        assert_eq!(parsed.canonical_bytes(), bytes);
        assert_eq!(parsed.digest(), r.digest());
        // Any content difference changes the digest.
        assert_ne!(request("hello").digest(), request("hello ").digest());
    }

    #[test]
    fn tape_entries_roundtrip_including_streams() {
        let entries = [
            TapeEntry::Success {
                status: 200,
                body: b"alpha\nbeta".to_vec(),
            },
            TapeEntry::ProviderError {
                status: 529,
                body: b"overloaded".to_vec(),
            },
            TapeEntry::Timeout,
            TapeEntry::Stream {
                chunks: vec![b"s1".to_vec(), b"".to_vec(), b"s3:with colon".to_vec()],
                terminal: "done".into(),
            },
        ];
        for e in entries {
            let frame = e.response_frame();
            assert_eq!(TapeEntry::parse(&frame).unwrap(), e, "{frame:?}");
        }
    }

    #[test]
    fn cassette_roundtrips_and_orders_identical_requests_distinctly() {
        let mut c = CassetteV2::default();
        // Two IDENTICAL requests with DIFFERENT responses: ordered
        // history, not a map overwrite.
        c.push(
            request("same"),
            TapeEntry::Success {
                status: 200,
                body: b"first".to_vec(),
            },
        );
        c.push(
            request("same"),
            TapeEntry::Success {
                status: 200,
                body: b"second".to_vec(),
            },
        );
        let parsed = CassetteV2::parse(&c.file_bytes()).unwrap();
        assert_eq!(parsed, c);
        assert_eq!(parsed.len(), 2);
        let (r0, e0) = parsed.entry(0).unwrap();
        let (r1, e1) = parsed.entry(1).unwrap();
        assert_eq!(r0.digest(), r1.digest(), "identical requests");
        assert_ne!(e0, e1, "distinct ordered responses survived");
        assert_eq!(parsed.identity(), c.identity());
        assert!(c.identity().starts_with("vh-cassette-v2:sha256:"));
    }

    #[test]
    fn parsers_fail_closed_on_malformed_truncated_and_trailing() {
        let r = request("x");
        let mut bytes = r.canonical_bytes();
        // Trailing garbage.
        let mut trailing = bytes.clone();
        trailing.extend_from_slice(b"extra\n");
        assert!(LlmRequestV2::parse(&trailing).is_err());
        // Truncation.
        bytes.truncate(bytes.len() - 3);
        assert!(LlmRequestV2::parse(&bytes).is_err());
        // Length lies.
        let lied = String::from_utf8(r.canonical_bytes()).unwrap().replacen(
            "provider 7:fixture",
            "provider 6:fixture",
            1,
        );
        assert!(LlmRequestV2::parse(lied.as_bytes()).is_err());
        // Unknown tape head.
        assert!(TapeEntry::parse(b"fuzzy-match\n").is_err());
        // Cassette with wrong schema.
        assert!(CassetteV2::parse(b"vh-cassette-v1 0\n").is_err());
    }

    #[test]
    fn hostile_counts_are_typed_rejections_before_allocation() {
        fn assert_impossible_count(error: CassetteParseError, field: &str) {
            assert!(
                matches!(
                    error,
                    CassetteParseError::CountExceedsFrame {
                        field: ref got,
                        count,
                        ..
                    } if got == field && count == usize::MAX
                ),
                "expected typed impossible-count rejection for {field:?}, got {error:?}"
            );
        }

        let max = usize::MAX;
        let messages = format!("vh-llm-request-v2\nprovider 0:\nmodel 0:\nmessages {max}\n");
        assert_impossible_count(
            LlmRequestV2::parse(messages.as_bytes()).unwrap_err(),
            "messages",
        );

        let tools = format!("vh-llm-request-v2\nprovider 0:\nmodel 0:\nmessages 0\ntools {max}\n");
        assert_impossible_count(LlmRequestV2::parse(tools.as_bytes()).unwrap_err(), "tools");

        let params = format!(
            "vh-llm-request-v2\nprovider 0:\nmodel 0:\nmessages 0\ntools 0\n\
             tool-choice absent\nstructured-output absent\nparams {max}\n"
        );
        assert_impossible_count(
            LlmRequestV2::parse(params.as_bytes()).unwrap_err(),
            "params",
        );

        let stream = format!("stream {max}\n");
        assert_impossible_count(
            TapeEntry::parse(stream.as_bytes()).unwrap_err(),
            "stream chunks",
        );

        let cassette = format!("{CASSETTE_SCHEMA_V2} {max}\n");
        assert_impossible_count(
            CassetteV2::parse(cassette.as_bytes()).unwrap_err(),
            "cassette entries",
        );
    }

    #[test]
    fn usize_max_field_length_is_a_typed_arithmetic_rejection() {
        let frame = format!("vh-llm-request-v2\nprovider {}:\n", usize::MAX);
        let error = LlmRequestV2::parse(frame.as_bytes()).unwrap_err();
        assert!(
            matches!(
                error,
                CassetteParseError::LengthArithmeticOverflow {
                    ref field,
                    length
                } if field == "provider" && length == usize::MAX
            ),
            "expected typed length-arithmetic rejection, got {error:?}"
        );
    }

    #[test]
    fn count_bounds_do_not_reject_large_well_formed_frames() {
        let mut dense = request("dense");
        dense.messages = vec![(String::new(), String::new()); 1_024];
        assert_eq!(
            LlmRequestV2::parse(&dense.canonical_bytes()).unwrap(),
            dense
        );

        let stream = TapeEntry::Stream {
            chunks: vec![Vec::new(); 1_024],
            terminal: String::new(),
        };
        assert_eq!(TapeEntry::parse(&stream.response_frame()).unwrap(), stream);
    }

    #[test]
    fn transport_receipt_taints_on_miss_or_leftovers() {
        let clean = TransportReceipt {
            served: vec!["d1".into()],
            unconsumed: 0,
            taint: None,
        };
        assert!(!clean.tainted());
        let miss = TransportReceipt {
            taint: Some("miss at 1".into()),
            ..clean.clone()
        };
        assert!(miss.tainted());
        let leftover = TransportReceipt {
            unconsumed: 2,
            ..clean.clone()
        };
        assert!(leftover.tainted());
        assert!(leftover.identity_str().contains("unconsumed=2"));
    }
}
