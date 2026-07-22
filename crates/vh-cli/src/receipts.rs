//! Pure NDJSON receipt encoding/decoding for the evidence store
//! (convergence C4, audit R4). No filesystem, no clock, no environment —
//! this module builds and parses receipt STRINGS only; all I/O lives in
//! the declared CLI boundary (`bundle.rs`).
//!
//! Two versioned schemas:
//! * `vh-run-receipts-v1` — `run.ndjson`: one manifest record, one record
//!   per universe, one record per bundled finding.
//! * `vh-finding-bundle-v1` — `finding.ndjson`: a self-contained replay
//!   bundle (finding identity + one record per failure/violation). A
//!   bundle plus the `vh` binary is sufficient to re-execute and verify —
//!   no other repo state.
//!
//! The parser is deliberately NOT a general JSON parser: it accepts the
//! flat one-object-per-line records these schemas emit (string / u64 /
//! bool / null values, no nesting, no arrays) and rejects everything
//! else. Determinism: identical inputs render identical bytes; there is
//! no wall clock, hostname, or environment in any record.

use vh_gremlin::FaultPalette;

pub const RUN_RECEIPTS_SCHEMA: &str = "vh-run-receipts-v1";
pub const FINDING_BUNDLE_SCHEMA: &str = "vh-finding-bundle-v1";

/// JSON string escaping for exactly what we emit.
pub fn json_escape(s: &str) -> String {
    let mut out = String::with_capacity(s.len() + 2);
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if (c as u32) < 0x20 => out.push_str(&format!("\\u{:04x}", c as u32)),
            c => out.push(c),
        }
    }
    out
}

/// A flat record value: the only shapes the receipt schemas emit.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Val {
    S(String),
    N(u64),
    B(bool),
    Null,
}

impl Val {
    fn render(&self) -> String {
        match self {
            Val::S(s) => format!("\"{}\"", json_escape(s)),
            Val::N(n) => n.to_string(),
            Val::B(b) => b.to_string(),
            Val::Null => "null".to_string(),
        }
    }

    pub fn as_str(&self) -> Option<&str> {
        match self {
            Val::S(s) => Some(s),
            _ => None,
        }
    }

    pub fn as_u64(&self) -> Option<u64> {
        match self {
            Val::N(n) => Some(*n),
            _ => None,
        }
    }
}

/// Render one flat record line (insertion order preserved — deterministic).
pub fn render_line(fields: &[(&str, Val)]) -> String {
    let body: Vec<String> = fields
        .iter()
        .map(|(k, v)| format!("\"{}\":{}", json_escape(k), v.render()))
        .collect();
    format!("{{{}}}", body.join(","))
}

/// Parse one flat record line back into ordered fields. Rejects nesting,
/// arrays, floats, and trailing garbage — receipts are the only dialect.
pub fn parse_line(line: &str) -> Result<Vec<(String, Val)>, String> {
    let mut chars = line.trim().chars().peekable();
    if chars.next() != Some('{') {
        return Err("record must start with '{'".into());
    }
    let mut fields = Vec::new();
    loop {
        match chars.peek() {
            Some('}') => {
                chars.next();
                break;
            }
            Some('"') => {}
            other => return Err(format!("expected key or '}}', got {other:?}")),
        }
        let key = parse_string(&mut chars)?;
        if chars.next() != Some(':') {
            return Err(format!("expected ':' after key {key:?}"));
        }
        let val = match chars.peek() {
            Some('"') => Val::S(parse_string(&mut chars)?),
            Some('t') | Some('f') | Some('n') => {
                let word: String = std::iter::from_fn(|| {
                    chars
                        .peek()
                        .copied()
                        .filter(char::is_ascii_alphabetic)
                        .inspect(|_| {
                            chars.next();
                        })
                })
                .collect();
                match word.as_str() {
                    "true" => Val::B(true),
                    "false" => Val::B(false),
                    "null" => Val::Null,
                    other => return Err(format!("unknown literal {other:?}")),
                }
            }
            Some(c) if c.is_ascii_digit() => {
                let digits: String = std::iter::from_fn(|| {
                    chars
                        .peek()
                        .copied()
                        .filter(char::is_ascii_digit)
                        .inspect(|_| {
                            chars.next();
                        })
                })
                .collect();
                Val::N(digits.parse().map_err(|e| format!("bad number: {e}"))?)
            }
            other => return Err(format!("unsupported value start {other:?} for key {key:?}")),
        };
        fields.push((key, val));
        match chars.next() {
            Some(',') => {}
            Some('}') => break,
            other => return Err(format!("expected ',' or '}}', got {other:?}")),
        }
    }
    if chars.next().is_some() {
        return Err("trailing garbage after record".into());
    }
    Ok(fields)
}

fn parse_string(chars: &mut std::iter::Peekable<std::str::Chars<'_>>) -> Result<String, String> {
    if chars.next() != Some('"') {
        return Err("expected '\"'".into());
    }
    let mut out = String::new();
    loop {
        match chars.next() {
            None => return Err("unterminated string".into()),
            Some('"') => return Ok(out),
            Some('\\') => match chars.next() {
                Some('"') => out.push('"'),
                Some('\\') => out.push('\\'),
                Some('n') => out.push('\n'),
                Some('r') => out.push('\r'),
                Some('t') => out.push('\t'),
                Some('u') => {
                    let hex: String = (0..4).filter_map(|_| chars.next()).collect();
                    let code =
                        u32::from_str_radix(&hex, 16).map_err(|e| format!("bad \\u{hex}: {e}"))?;
                    out.push(char::from_u32(code).ok_or_else(|| format!("bad codepoint {code}"))?);
                }
                other => return Err(format!("unknown escape {other:?}")),
            },
            Some(c) => out.push(c),
        }
    }
}

fn field<'a>(fields: &'a [(String, Val)], key: &str) -> Option<&'a Val> {
    fields.iter().find(|(k, _)| k == key).map(|(_, v)| v)
}

fn require_str(fields: &[(String, Val)], key: &str) -> Result<String, String> {
    field(fields, key)
        .and_then(Val::as_str)
        .map(str::to_string)
        .ok_or_else(|| format!("missing string field {key:?}"))
}

fn require_u64(fields: &[(String, Val)], key: &str) -> Result<u64, String> {
    field(fields, key)
        .and_then(Val::as_u64)
        .ok_or_else(|| format!("missing numeric field {key:?}"))
}

pub fn palette_by_name(name: &str) -> Result<FaultPalette, String> {
    match name {
        "v0" => Ok(FaultPalette::V0),
        "swarm" => Ok(FaultPalette::Swarm),
        other => Err(format!("unknown palette {other:?}; expected v0 or swarm")),
    }
}

pub fn parse_seed(s: &str) -> Result<u64, String> {
    let (digits, radix) = match s.strip_prefix("0x") {
        Some(hex) => (hex, 16),
        None => (s, 10),
    };
    u64::from_str_radix(digits, radix).map_err(|e| format!("bad seed {s}: {e}"))
}

/// A self-contained finding replay bundle: everything `vh replay-bundle`
/// needs to re-execute the universe and verify the exact finding.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FindingBundle {
    pub finding_id: String,
    pub workload: String,
    pub seed: u64,
    pub palette: String,
    pub universe: u64,
    pub trace_hash: String,
    pub trace_events: u64,
    pub fault_plan_digest: Option<String>,
    /// `(name, detail)` per always-failure, in recorded (deterministic) order.
    pub failures: Vec<(String, String)>,
    /// Property-contract violation details, in recorded order.
    pub contract_violations: Vec<String>,
    /// Debug rendering of an invalid lifecycle completion, if any.
    pub invalid_completion: Option<String>,
}

impl FindingBundle {
    pub fn to_ndjson(&self) -> String {
        let mut lines = vec![render_line(&[
            ("record", Val::S("finding".into())),
            ("schema", Val::S(FINDING_BUNDLE_SCHEMA.into())),
            ("finding_id", Val::S(self.finding_id.clone())),
            ("workload", Val::S(self.workload.clone())),
            ("seed", Val::S(format!("0x{:x}", self.seed))),
            ("palette", Val::S(self.palette.clone())),
            ("universe", Val::N(self.universe)),
            ("trace_hash", Val::S(self.trace_hash.clone())),
            ("trace_events", Val::N(self.trace_events)),
            (
                "fault_plan_digest",
                match &self.fault_plan_digest {
                    Some(d) => Val::S(d.clone()),
                    None => Val::Null,
                },
            ),
        ])];
        for (name, detail) in &self.failures {
            lines.push(render_line(&[
                ("record", Val::S("failure".into())),
                ("name", Val::S(name.clone())),
                ("detail", Val::S(detail.clone())),
            ]));
        }
        for detail in &self.contract_violations {
            lines.push(render_line(&[
                ("record", Val::S("contract".into())),
                ("detail", Val::S(detail.clone())),
            ]));
        }
        if let Some(detail) = &self.invalid_completion {
            lines.push(render_line(&[
                ("record", Val::S("invalid_completion".into())),
                ("detail", Val::S(detail.clone())),
            ]));
        }
        lines.join("\n") + "\n"
    }

    pub fn parse(text: &str) -> Result<FindingBundle, String> {
        let mut lines = text.lines().filter(|l| !l.trim().is_empty());
        let head = parse_line(lines.next().ok_or("empty bundle")?)?;
        let record = require_str(&head, "record")?;
        if record != "finding" {
            return Err(format!("first record must be \"finding\", got {record:?}"));
        }
        let schema = require_str(&head, "schema")?;
        if schema != FINDING_BUNDLE_SCHEMA {
            return Err(format!(
                "unsupported bundle schema {schema:?} (this build reads {FINDING_BUNDLE_SCHEMA:?})"
            ));
        }
        let mut bundle = FindingBundle {
            finding_id: require_str(&head, "finding_id")?,
            workload: require_str(&head, "workload")?,
            seed: parse_seed(&require_str(&head, "seed")?)?,
            palette: require_str(&head, "palette")?,
            universe: require_u64(&head, "universe")?,
            trace_hash: require_str(&head, "trace_hash")?,
            trace_events: require_u64(&head, "trace_events")?,
            fault_plan_digest: match field(&head, "fault_plan_digest") {
                Some(Val::S(s)) => Some(s.clone()),
                _ => None,
            },
            failures: Vec::new(),
            contract_violations: Vec::new(),
            invalid_completion: None,
        };
        for line in lines {
            let rec = parse_line(line)?;
            match require_str(&rec, "record")?.as_str() {
                "failure" => bundle
                    .failures
                    .push((require_str(&rec, "name")?, require_str(&rec, "detail")?)),
                "contract" => bundle
                    .contract_violations
                    .push(require_str(&rec, "detail")?),
                "invalid_completion" => {
                    bundle.invalid_completion = Some(require_str(&rec, "detail")?)
                }
                other => return Err(format!("unknown record kind {other:?}")),
            }
        }
        Ok(bundle)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample() -> FindingBundle {
        FindingBundle {
            finding_id: "u7-9ce6199f1334".into(),
            workload: "demo-buggy".into(),
            seed: 0xD1CE,
            palette: "v0".into(),
            universe: 7,
            trace_hash: "9ce6199f133f4d3c9dd0da0075e352d2".into(),
            trace_events: 45,
            fault_plan_digest: Some("abcd".into()),
            failures: vec![(
                "oracle:durability".into(),
                "acked write k1=\"v\\1\"\nmissing".into(),
            )],
            contract_violations: vec!["missing oracle".into()],
            invalid_completion: None,
        }
    }

    #[test]
    fn bundle_roundtrips_through_ndjson_bit_identically() {
        let b = sample();
        let text = b.to_ndjson();
        let parsed = FindingBundle::parse(&text).unwrap();
        assert_eq!(parsed, b);
        // Determinism: render → parse → render is byte-identical.
        assert_eq!(parsed.to_ndjson(), text);
    }

    #[test]
    fn escaping_survives_hostile_details() {
        let mut b = sample();
        b.failures = vec![(
            "n\"a\\me".into(),
            "line1\nline2\ttab \"quoted\" \\slash\u{1}".into(),
        )];
        let parsed = FindingBundle::parse(&b.to_ndjson()).unwrap();
        assert_eq!(parsed.failures, b.failures);
    }

    #[test]
    fn parser_rejects_wrong_schema_and_garbage() {
        assert!(FindingBundle::parse("").is_err());
        assert!(FindingBundle::parse("{\"record\":\"finding\",\"schema\":\"v999\"}").is_err());
        assert!(FindingBundle::parse("not json").is_err());
        let mut text = sample().to_ndjson();
        text.push_str("{\"record\":\"mystery\"}\n");
        assert!(FindingBundle::parse(&text).is_err());
        // Trailing garbage on a record line is rejected, not ignored.
        assert!(parse_line("{\"a\":1} extra").is_err());
        // Nesting is not part of the dialect.
        assert!(parse_line("{\"a\":{\"b\":1}}").is_err());
    }

    #[test]
    fn seed_renders_hex_and_reparses() {
        assert_eq!(parse_seed("0xd1ce").unwrap(), 0xD1CE);
        assert_eq!(parse_seed("42").unwrap(), 42);
        assert!(parse_seed("0xzz").is_err());
    }
}
