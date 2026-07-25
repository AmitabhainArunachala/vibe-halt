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

/// Cassette schema (ordered history). v1 (`vh-cassette-v1`) remains the
/// legacy parent-side demo format in `lib.rs`.
pub const CASSETTE_SCHEMA_V2: &str = "vh-cassette-v2";
/// Child-visible framing contract version, bound into run records.
pub const TRANSPORT_SCHEMA: &str = "vh-cassette-transport-v1";

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

    pub fn parse(bytes: &[u8]) -> Result<LlmRequestV2, String> {
        let mut r = FrameReader::new(bytes);
        r.expect_line("vh-llm-request-v2")?;
        let provider = r.field_string("provider")?;
        let model = r.field_string("model")?;
        let n_messages = r.count("messages")?;
        let mut messages = Vec::with_capacity(n_messages);
        for _ in 0..n_messages {
            messages.push((r.field_string("role")?, r.field_string("content")?));
        }
        let n_tools = r.count("tools")?;
        let mut tools = Vec::with_capacity(n_tools);
        for _ in 0..n_tools {
            tools.push((r.field_string("tool-name")?, r.field_string("tool-schema")?));
        }
        let tool_choice = r.opt_field_string("tool-choice")?;
        let structured_output = r.opt_field_string("structured-output")?;
        let n_params = r.count("params")?;
        let mut params = BTreeMap::new();
        for _ in 0..n_params {
            let k = r.field_string("param-key")?;
            let v = r.field_string("param-value")?;
            if params.insert(k.clone(), v).is_some() {
                return Err(format!("duplicate param {k:?}"));
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

    pub fn parse(bytes: &[u8]) -> Result<TapeEntry, String> {
        let mut r = FrameReader::new(bytes);
        let head = r.take_line()?;
        let entry = if let Some(status) = head.strip_prefix("success ") {
            TapeEntry::Success {
                status: status.parse().map_err(|e| format!("bad status: {e}"))?,
                body: r.field_bytes("body")?,
            }
        } else if let Some(status) = head.strip_prefix("provider-error ") {
            TapeEntry::ProviderError {
                status: status.parse().map_err(|e| format!("bad status: {e}"))?,
                body: r.field_bytes("body")?,
            }
        } else if head == "timeout" {
            TapeEntry::Timeout
        } else if let Some(n) = head.strip_prefix("stream ") {
            let n: usize = n.parse().map_err(|e| format!("bad chunk count: {e}"))?;
            let mut chunks = Vec::with_capacity(n);
            for _ in 0..n {
                chunks.push(r.field_bytes("chunk")?);
            }
            TapeEntry::Stream {
                chunks,
                terminal: String::from_utf8(r.field_bytes("terminal")?)
                    .map_err(|e| format!("terminal not utf-8: {e}"))?,
            }
        } else {
            return Err(format!("unknown tape entry head {head:?}"));
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

    pub fn parse(bytes: &[u8]) -> Result<CassetteV2, String> {
        let mut r = FrameReader::new(bytes);
        let head = r.take_line()?;
        let n: usize = head
            .strip_prefix(&format!("{CASSETTE_SCHEMA_V2} "))
            .ok_or_else(|| format!("unsupported cassette head {head:?}"))?
            .parse()
            .map_err(|e| format!("bad entry count: {e}"))?;
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

    fn take_line(&mut self) -> Result<String, String> {
        let rest = &self.bytes[self.pos..];
        let nl = rest
            .iter()
            .position(|&b| b == b'\n')
            .ok_or("unterminated line")?;
        let line =
            String::from_utf8(rest[..nl].to_vec()).map_err(|e| format!("line not utf-8: {e}"))?;
        self.pos += nl + 1;
        Ok(line)
    }

    fn expect_line(&mut self, expected: &str) -> Result<(), String> {
        let got = self.take_line()?;
        if got != expected {
            return Err(format!("expected line {expected:?}, got {got:?}"));
        }
        Ok(())
    }

    fn field_bytes(&mut self, tag: &str) -> Result<Vec<u8>, String> {
        // `<tag> <len>:` prefix, then exactly len raw bytes, then '\n'.
        let rest = &self.bytes[self.pos..];
        let tag_bytes = tag.as_bytes();
        if rest.len() < tag_bytes.len() + 1 || &rest[..tag_bytes.len()] != tag_bytes {
            return Err(format!("expected field {tag:?}"));
        }
        if rest[tag_bytes.len()] != b' ' {
            return Err(format!("expected space after field tag {tag:?}"));
        }
        let mut idx = tag_bytes.len() + 1;
        let len_start = idx;
        while idx < rest.len() && rest[idx].is_ascii_digit() {
            idx += 1;
        }
        if idx == len_start || idx >= rest.len() || rest[idx] != b':' {
            return Err(format!("malformed length for field {tag:?}"));
        }
        let len: usize = std::str::from_utf8(&rest[len_start..idx])
            .expect("digits are utf-8")
            .parse()
            .map_err(|e| format!("length overflow for field {tag:?}: {e}"))?;
        idx += 1;
        if rest.len() < idx + len + 1 {
            return Err(format!("truncated field {tag:?}"));
        }
        let value = rest[idx..idx + len].to_vec();
        if rest[idx + len] != b'\n' {
            return Err(format!("field {tag:?} not newline-terminated"));
        }
        self.pos += idx + len + 1;
        Ok(value)
    }

    fn field_string(&mut self, tag: &str) -> Result<String, String> {
        String::from_utf8(self.field_bytes(tag)?)
            .map_err(|e| format!("field {tag:?} not utf-8: {e}"))
    }

    fn opt_field_string(&mut self, tag: &str) -> Result<Option<String>, String> {
        // Peek: `<tag> absent\n` or a length-prefixed field.
        let save = self.pos;
        let line = self.take_line()?;
        if line == format!("{tag} absent") {
            return Ok(None);
        }
        self.pos = save;
        Ok(Some(self.field_string(tag)?))
    }

    fn count(&mut self, tag: &str) -> Result<usize, String> {
        let line = self.take_line()?;
        line.strip_prefix(&format!("{tag} "))
            .ok_or_else(|| format!("expected count line for {tag:?}, got {line:?}"))?
            .parse()
            .map_err(|e| format!("bad count for {tag:?}: {e}"))
    }

    fn expect_end(&self) -> Result<(), String> {
        if self.pos != self.bytes.len() {
            return Err(format!(
                "trailing bytes after frame ({} of {} consumed)",
                self.pos,
                self.bytes.len()
            ));
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
