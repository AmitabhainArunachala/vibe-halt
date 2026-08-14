//! Issue #90 negotiated cooperative protocol kernel.
//!
//! Rust owns the closed operation registry, engine manifest, request
//! admission, and revision-authority transitions.  The wire format is
//! positional and length-framed; Python is a strict consumer, never a second
//! registry or revision authority.

use std::path::Path;

pub(crate) const MANIFEST_SCHEMA: &str = "vh-protocol-manifest-v1";
pub(crate) const REFUSAL_SCHEMA: &str = "vh-engine-negotiation-refusal-v1";
pub(crate) const OPERATION: &str = "cooperative-target-v1";
pub(crate) const REQUEST_SCHEMA: &str = "vh-cooperative-request-v2";
pub(crate) const OUTCOME_SCHEMA: &str = "vh-cooperative-outcome-v2";
pub(crate) const RECEIPT_SCHEMA: &str = "vh-cooperative-receipt-v2";
pub(crate) const VERIFY_SCHEMA: &str = "vh-cooperative-verify-v2";
pub(crate) const VERIFY_FAILURE_SCHEMA: &str = "vh-cooperative-verify-failure-v1";
pub(crate) const OBSERVATION_SUBJECT: &str = "cooperative-child-source-v1";
pub(crate) const REVISION_ALGORITHM: &str = "sha256";
pub(crate) const REVISION_POLICY: &str = "bound-required";
pub(crate) const EXECUTION_BINDING: &str = "staged-d2";
pub(crate) const OBSERVATION_TO_EXEC_CHANNEL: &str = "open";

pub(crate) const MANIFEST_ID_DOMAIN: &str = "vh-protocol-manifest-id-v1";
pub(crate) const ENGINE_REQUEST_ID_DOMAIN: &str = "vh-cooperative-engine-request-v2";
pub(crate) const EVIDENCE_ID_DOMAIN: &str = "vh-cooperative-evidence-v2";
pub(crate) const VERIFICATION_RESULT_ID_DOMAIN: &str = "vh-cooperative-verification-result-v1";

pub(crate) const MAX_PROTOCOL_RECORD_BYTES: usize = 64 << 10;
pub(crate) const MAX_FEATURES: usize = 16;
pub(crate) const MAX_IDENTIFIER_BYTES: usize = 64;

pub(crate) const MANDATORY_FEATURES: [&str; 3] = [
    "cooperative-cassette-v2",
    "fresh-replay-v1",
    "observed-child-source-sha256-v1",
];
pub(crate) const OPTIONAL_FEATURES: [&str; 0] = [];

fn plain_line(out: &mut Vec<u8>, line: &str) {
    out.extend_from_slice(line.as_bytes());
    out.push(b'\n');
}

fn frame(out: &mut Vec<u8>, tag: &str, value: &[u8]) {
    out.extend_from_slice(tag.as_bytes());
    out.push(b' ');
    out.extend_from_slice(value.len().to_string().as_bytes());
    out.push(b':');
    out.extend_from_slice(value);
    out.push(b'\n');
}

fn lowercase_hex(value: &str, len: usize) -> bool {
    value.len() == len
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

fn canonical_identifier(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= MAX_IDENTIFIER_BYTES
        && value
            .bytes()
            .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'-')
        && !value.starts_with('-')
        && !value.ends_with('-')
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Descriptor {
    pub(crate) operation: &'static str,
    pub(crate) request_schema: &'static str,
    pub(crate) outcome_schema: &'static str,
    pub(crate) receipt_schema: &'static str,
    pub(crate) verifier_schema: &'static str,
    pub(crate) observation_subject: &'static str,
    pub(crate) revision_algorithm: &'static str,
    pub(crate) revision_policy: &'static str,
    pub(crate) execution_binding: &'static str,
    pub(crate) observation_to_exec_channel: &'static str,
    pub(crate) mandatory_features: &'static [&'static str],
    pub(crate) optional_features: &'static [&'static str],
}

pub(crate) const DESCRIPTOR: Descriptor = Descriptor {
    operation: OPERATION,
    request_schema: REQUEST_SCHEMA,
    outcome_schema: OUTCOME_SCHEMA,
    receipt_schema: RECEIPT_SCHEMA,
    verifier_schema: VERIFY_SCHEMA,
    observation_subject: OBSERVATION_SUBJECT,
    revision_algorithm: REVISION_ALGORITHM,
    revision_policy: REVISION_POLICY,
    execution_binding: EXECUTION_BINDING,
    observation_to_exec_channel: OBSERVATION_TO_EXEC_CHANNEL,
    mandatory_features: &MANDATORY_FEATURES,
    optional_features: &OPTIONAL_FEATURES,
};

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct ProtocolManifest {
    pub(crate) engine_sha256: String,
    pub(crate) manifest_id: String,
}

fn encode_descriptor(out: &mut Vec<u8>, descriptor: &Descriptor) {
    frame(out, "operation", descriptor.operation.as_bytes());
    frame(out, "request-schema", descriptor.request_schema.as_bytes());
    frame(out, "outcome-schema", descriptor.outcome_schema.as_bytes());
    frame(out, "receipt-schema", descriptor.receipt_schema.as_bytes());
    frame(
        out,
        "verifier-schema",
        descriptor.verifier_schema.as_bytes(),
    );
    frame(
        out,
        "observation-subject",
        descriptor.observation_subject.as_bytes(),
    );
    frame(
        out,
        "revision-algorithm",
        descriptor.revision_algorithm.as_bytes(),
    );
    frame(
        out,
        "revision-policy",
        descriptor.revision_policy.as_bytes(),
    );
    frame(
        out,
        "execution-binding",
        descriptor.execution_binding.as_bytes(),
    );
    frame(
        out,
        "observation-to-exec-channel",
        descriptor.observation_to_exec_channel.as_bytes(),
    );
    plain_line(
        out,
        &format!("mandatory-features {}", descriptor.mandatory_features.len()),
    );
    for feature in descriptor.mandatory_features {
        frame(out, "feature", feature.as_bytes());
    }
    plain_line(
        out,
        &format!("optional-features {}", descriptor.optional_features.len()),
    );
    for feature in descriptor.optional_features {
        frame(out, "feature", feature.as_bytes());
    }
}

fn descriptor_preimage(engine_sha256: &str) -> Vec<u8> {
    let mut out = Vec::new();
    plain_line(&mut out, MANIFEST_ID_DOMAIN);
    frame(&mut out, "schema", MANIFEST_SCHEMA.as_bytes());
    frame(&mut out, "engine-sha256", engine_sha256.as_bytes());
    encode_descriptor(&mut out, &DESCRIPTOR);
    out
}

impl ProtocolManifest {
    pub(crate) fn current() -> Result<Self, String> {
        let engine_sha256 = super::cooperative::current_engine_sha256()?;
        Ok(Self::from_engine_sha256(engine_sha256))
    }

    pub(crate) fn from_engine_sha256(engine_sha256: String) -> Self {
        let manifest_id = vh_digest::sha256_hex(&descriptor_preimage(&engine_sha256));
        Self {
            engine_sha256,
            manifest_id,
        }
    }

    pub(crate) fn encode(&self) -> Vec<u8> {
        let mut out = Vec::new();
        plain_line(&mut out, MANIFEST_SCHEMA);
        frame(&mut out, "engine-sha256", self.engine_sha256.as_bytes());
        frame(&mut out, "manifest-id", self.manifest_id.as_bytes());
        plain_line(&mut out, "descriptors 1");
        encode_descriptor(&mut out, &DESCRIPTOR);
        out
    }
}

pub(crate) fn cmd_protocol_manifest(args: &[String], usage: &str) -> i32 {
    if !args.is_empty() {
        eprintln!("error: protocol-manifest takes no arguments\n\n{usage}");
        return 2;
    }
    match ProtocolManifest::current() {
        Ok(manifest) => {
            let bytes = manifest.encode();
            if bytes.len() > MAX_PROTOCOL_RECORD_BYTES {
                eprintln!("error: protocol manifest exceeds its bounded wire profile");
                return 2;
            }
            print!("{}", String::from_utf8_lossy(&bytes));
            0
        }
        Err(error) => {
            eprintln!("error: {}", super::cooperative::bounded_diagnostic(&error));
            2
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum RefusalReason {
    UnsupportedOperation,
    UnsupportedFeature,
    InvalidFeatureSet,
    ProtocolManifestMismatch,
    RequestedRevisionMismatch,
    MissingObservation,
    UnsupportedReceiptSchema,
}

impl RefusalReason {
    pub(crate) fn as_str(self) -> &'static str {
        match self {
            Self::UnsupportedOperation => "unsupported-operation",
            Self::UnsupportedFeature => "unsupported-feature",
            Self::InvalidFeatureSet => "invalid-feature-set",
            Self::ProtocolManifestMismatch => "protocol-manifest-mismatch",
            Self::RequestedRevisionMismatch => "requested-revision-mismatch",
            Self::MissingObservation => "missing-observation",
            Self::UnsupportedReceiptSchema => "unsupported-receipt-schema",
        }
    }
}

pub(crate) fn encode_refusal(
    reason: RefusalReason,
    engine_sha256: &str,
    manifest_id: &str,
) -> Vec<u8> {
    let mut out = Vec::new();
    plain_line(&mut out, REFUSAL_SCHEMA);
    frame(&mut out, "reason", reason.as_str().as_bytes());
    frame(&mut out, "engine-sha256", engine_sha256.as_bytes());
    frame(&mut out, "manifest-id", manifest_id.as_bytes());
    plain_line(&mut out, "executions 0");
    out
}

pub(crate) fn evidence_id(
    engine_request_id: &str,
    claimed_observed_revision: &str,
    first_identity: &str,
    second_identity: &str,
) -> String {
    let mut out = Vec::new();
    plain_line(&mut out, EVIDENCE_ID_DOMAIN);
    frame(&mut out, "engine-request-id", engine_request_id.as_bytes());
    frame(
        &mut out,
        "claimed-observed-revision",
        claimed_observed_revision.as_bytes(),
    );
    frame(&mut out, "first-identity", first_identity.as_bytes());
    frame(&mut out, "second-identity", second_identity.as_bytes());
    vh_digest::sha256_hex(&out)
}

pub(crate) fn verification_result_id(
    receipt_sha256: &str,
    fresh_observed_revision: &str,
    verified_observed_revision: &str,
    authentic: bool,
    outcome_verified: bool,
) -> String {
    let mut out = Vec::new();
    plain_line(&mut out, VERIFICATION_RESULT_ID_DOMAIN);
    frame(&mut out, "receipt-sha256", receipt_sha256.as_bytes());
    frame(
        &mut out,
        "fresh-observed-revision",
        fresh_observed_revision.as_bytes(),
    );
    frame(
        &mut out,
        "verified-observed-revision",
        verified_observed_revision.as_bytes(),
    );
    frame(
        &mut out,
        "authentic",
        if authentic { b"true" } else { b"false" },
    );
    frame(
        &mut out,
        "outcome-verified",
        if outcome_verified { b"true" } else { b"false" },
    );
    vh_digest::sha256_hex(&out)
}

#[derive(Clone, Debug, PartialEq, Eq)]
enum RequestedRevisionValue {
    Unknown,
    Exact(String),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct RequestedTargetRevision(RequestedRevisionValue);

impl RequestedTargetRevision {
    pub(crate) fn parse(value: &str) -> Result<Self, ()> {
        if value == "unknown" {
            return Ok(Self(RequestedRevisionValue::Unknown));
        }
        let digest = value.strip_prefix("sha256:").ok_or(())?;
        if !lowercase_hex(digest, 64) {
            return Err(());
        }
        Ok(Self(RequestedRevisionValue::Exact(digest.to_string())))
    }

    pub(crate) fn wire_value(&self) -> String {
        match &self.0 {
            RequestedRevisionValue::Unknown => "unknown".into(),
            RequestedRevisionValue::Exact(digest) => format!("sha256:{digest}"),
        }
    }

    pub(crate) fn is_unknown(&self) -> bool {
        matches!(self.0, RequestedRevisionValue::Unknown)
    }

    pub(crate) fn exact_digest(&self) -> Option<&str> {
        match &self.0 {
            RequestedRevisionValue::Unknown => None,
            RequestedRevisionValue::Exact(digest) => Some(digest),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct ClaimedObservedRevision(String);

impl ClaimedObservedRevision {
    pub(crate) fn parse(digest: &str) -> Result<Self, ()> {
        lowercase_hex(digest, 64)
            .then(|| Self(digest.to_string()))
            .ok_or(())
    }

    pub(crate) fn digest(&self) -> &str {
        &self.0
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct FreshObservedRevision {
    digest: String,
    bytes: Vec<u8>,
}

impl FreshObservedRevision {
    fn from_resolved_bytes(bytes: Vec<u8>) -> Self {
        let digest = vh_digest::sha256_hex(&bytes);
        Self { digest, bytes }
    }

    pub(crate) fn digest(&self) -> &str {
        &self.digest
    }

    pub(crate) fn bytes(&self) -> &[u8] {
        &self.bytes
    }
}

/// Resolve a fresh revision from a bounded Rust-owned path snapshot. Callers
/// cannot construct the authority type from a digest or receipt field.
pub(crate) fn resolve_fresh_target_path(
    path: &Path,
    max: u64,
) -> Result<FreshObservedRevision, String> {
    let bytes = vh_sandbox::read_bounded_file(path, max)
        .map_err(|error| format!("target observation failed: category={}", error.category()))?;
    Ok(FreshObservedRevision::from_resolved_bytes(bytes))
}

/// Resolve the one closed, compiled-in cooperative target into a fresh
/// observation for standalone verification. No caller or sibling module can
/// supply arbitrary bytes to this constructor.
pub(crate) fn resolve_fresh_compiled_target() -> FreshObservedRevision {
    FreshObservedRevision::from_resolved_bytes(
        super::cooperative::COOPERATIVE_ECHO_CHILD
            .as_bytes()
            .to_vec(),
    )
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct VerifiedObservedRevision(String);

impl VerifiedObservedRevision {
    pub(crate) fn promote(
        claimed: &ClaimedObservedRevision,
        matched: &RevisionMatched<'_>,
    ) -> Result<Self, ()> {
        (claimed.digest() == matched.fresh.digest())
            .then(|| Self(matched.fresh.digest().to_string()))
            .ok_or(())
    }

    pub(crate) fn digest(&self) -> &str {
        &self.0
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct NegotiatedRequest {
    manifest_id: String,
    features: Vec<String>,
    requested_revision: RequestedTargetRevision,
}

impl NegotiatedRequest {
    pub(crate) fn manifest_id(&self) -> &str {
        &self.manifest_id
    }

    pub(crate) fn features(&self) -> &[String] {
        &self.features
    }

    pub(crate) fn requested_revision(&self) -> &RequestedTargetRevision {
        &self.requested_revision
    }
}

/// Proof that a negotiated `BoundRequired` request names the exact bytes in a
/// fresh Rust-owned observation. Its fields are private so sibling modules can
/// consume the proof but cannot manufacture it from strings or receipt data.
pub(crate) struct RevisionMatched<'a> {
    request: &'a NegotiatedRequest,
    fresh: &'a FreshObservedRevision,
}

impl<'a> RevisionMatched<'a> {
    pub(crate) fn request(&self) -> &'a NegotiatedRequest {
        self.request
    }

    pub(crate) fn fresh(&self) -> &'a FreshObservedRevision {
        self.fresh
    }
}

pub(crate) fn match_requested_revision<'a>(
    request: &'a NegotiatedRequest,
    fresh: &'a FreshObservedRevision,
) -> Result<RevisionMatched<'a>, RefusalReason> {
    match request.requested_revision.exact_digest() {
        Some(requested) if requested == fresh.digest() => Ok(RevisionMatched { request, fresh }),
        _ => Err(RefusalReason::RequestedRevisionMismatch),
    }
}

pub(crate) fn negotiate(
    manifest: &ProtocolManifest,
    protocol_schema: &str,
    manifest_id: &str,
    operation: &str,
    features: &[String],
    requested_revision: RequestedTargetRevision,
) -> Result<NegotiatedRequest, RefusalReason> {
    if protocol_schema != MANIFEST_SCHEMA || manifest_id != manifest.manifest_id {
        return Err(RefusalReason::ProtocolManifestMismatch);
    }
    if operation != OPERATION {
        return Err(RefusalReason::UnsupportedOperation);
    }
    if features.len() > MAX_FEATURES
        || features.iter().any(|value| !canonical_identifier(value))
        || features.windows(2).any(|pair| pair[0] >= pair[1])
    {
        return Err(RefusalReason::InvalidFeatureSet);
    }
    if features
        .iter()
        .any(|feature| !MANDATORY_FEATURES.contains(&feature.as_str()))
    {
        return Err(RefusalReason::UnsupportedFeature);
    }
    if MANDATORY_FEATURES
        .iter()
        .any(|mandatory| !features.iter().any(|feature| feature == mandatory))
    {
        return Err(RefusalReason::InvalidFeatureSet);
    }
    if requested_revision.is_unknown() {
        return Err(RefusalReason::RequestedRevisionMismatch);
    }
    Ok(NegotiatedRequest {
        manifest_id: manifest_id.to_string(),
        features: features.to_vec(),
        requested_revision,
    })
}

fn engine_request_id_for_descriptor(
    request: &NegotiatedRequest,
    cassette_identity: &str,
    descriptor: &Descriptor,
) -> String {
    let mut out = Vec::new();
    plain_line(&mut out, ENGINE_REQUEST_ID_DOMAIN);
    frame(&mut out, "manifest-id", request.manifest_id.as_bytes());
    frame(&mut out, "operation", descriptor.operation.as_bytes());
    frame(
        &mut out,
        "request-schema",
        descriptor.request_schema.as_bytes(),
    );
    frame(
        &mut out,
        "outcome-schema",
        descriptor.outcome_schema.as_bytes(),
    );
    frame(
        &mut out,
        "receipt-schema",
        descriptor.receipt_schema.as_bytes(),
    );
    frame(
        &mut out,
        "verifier-schema",
        descriptor.verifier_schema.as_bytes(),
    );
    frame(
        &mut out,
        "observation-subject",
        descriptor.observation_subject.as_bytes(),
    );
    frame(
        &mut out,
        "revision-algorithm",
        descriptor.revision_algorithm.as_bytes(),
    );
    frame(
        &mut out,
        "revision-policy",
        descriptor.revision_policy.as_bytes(),
    );
    frame(
        &mut out,
        "execution-binding",
        descriptor.execution_binding.as_bytes(),
    );
    frame(
        &mut out,
        "observation-to-exec-channel",
        descriptor.observation_to_exec_channel.as_bytes(),
    );
    frame(&mut out, "cassette-identity", cassette_identity.as_bytes());
    plain_line(&mut out, &format!("features {}", request.features.len()));
    for feature in &request.features {
        frame(&mut out, "feature", feature.as_bytes());
    }
    frame(
        &mut out,
        "requested-target-revision",
        request.requested_revision.wire_value().as_bytes(),
    );
    vh_digest::sha256_hex(&out)
}

pub(crate) fn engine_request_id(request: &NegotiatedRequest, cassette_identity: &str) -> String {
    engine_request_id_for_descriptor(request, cassette_identity, &DESCRIPTOR)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn manifest() -> ProtocolManifest {
        ProtocolManifest::from_engine_sha256("a".repeat(64))
    }

    fn features() -> Vec<String> {
        MANDATORY_FEATURES
            .iter()
            .map(|value| value.to_string())
            .collect()
    }

    #[test]
    fn manifest_is_canonical_and_digest_bound() {
        let manifest = manifest();
        assert_eq!(manifest.manifest_id.len(), 64);
        let encoded = manifest.encode();
        assert!(encoded.starts_with(format!("{MANIFEST_SCHEMA}\n").as_bytes()));
        assert!(encoded.len() <= MAX_PROTOCOL_RECORD_BYTES);
        assert_eq!(
            manifest,
            ProtocolManifest::from_engine_sha256("a".repeat(64))
        );
        assert_ne!(
            manifest,
            ProtocolManifest::from_engine_sha256("b".repeat(64))
        );
    }

    #[test]
    fn feature_closure_cannot_be_weakened_or_extended() {
        let manifest = manifest();
        let requested =
            RequestedTargetRevision::parse(&format!("sha256:{}", "0".repeat(64))).unwrap();
        assert!(negotiate(
            &manifest,
            MANIFEST_SCHEMA,
            &manifest.manifest_id,
            OPERATION,
            &features(),
            requested.clone(),
        )
        .is_ok());
        assert_eq!(
            negotiate(
                &manifest,
                MANIFEST_SCHEMA,
                &manifest.manifest_id,
                OPERATION,
                &features()[1..],
                requested.clone(),
            ),
            Err(RefusalReason::InvalidFeatureSet)
        );
        let mut unsupported = features();
        unsupported.push("unknown-v1".into());
        unsupported.sort();
        assert_eq!(
            negotiate(
                &manifest,
                MANIFEST_SCHEMA,
                &manifest.manifest_id,
                OPERATION,
                &unsupported,
                requested,
            ),
            Err(RefusalReason::UnsupportedFeature)
        );
    }

    #[test]
    fn authority_types_promote_only_claimed_fresh_equality() {
        let fresh = resolve_fresh_compiled_target();
        let manifest = manifest();
        let request = negotiate(
            &manifest,
            MANIFEST_SCHEMA,
            &manifest.manifest_id,
            OPERATION,
            &features(),
            RequestedTargetRevision::parse(&format!("sha256:{}", fresh.digest())).unwrap(),
        )
        .unwrap();
        let matched = match_requested_revision(&request, &fresh).unwrap();
        let claimed = ClaimedObservedRevision::parse(fresh.digest()).unwrap();
        let verified = VerifiedObservedRevision::promote(&claimed, &matched).unwrap();
        assert_eq!(verified.digest(), fresh.digest());
        let other = ClaimedObservedRevision::parse(&"0".repeat(64)).unwrap();
        assert!(VerifiedObservedRevision::promote(&other, &matched).is_err());
        let mismatched_request = negotiate(
            &manifest,
            MANIFEST_SCHEMA,
            &manifest.manifest_id,
            OPERATION,
            &features(),
            RequestedTargetRevision::parse(&format!("sha256:{}", "0".repeat(64))).unwrap(),
        )
        .unwrap();
        assert!(match_requested_revision(&mismatched_request, &fresh).is_err());
        assert_eq!(
            fresh.bytes(),
            super::super::cooperative::COOPERATIVE_ECHO_CHILD.as_bytes()
        );
    }

    #[test]
    fn request_identity_changes_with_revision_and_manifest() {
        let manifest = manifest();
        let base = negotiate(
            &manifest,
            MANIFEST_SCHEMA,
            &manifest.manifest_id,
            OPERATION,
            &features(),
            RequestedTargetRevision::parse(&format!("sha256:{}", "0".repeat(64))).unwrap(),
        )
        .unwrap();
        let exact = NegotiatedRequest {
            requested_revision: RequestedTargetRevision::parse(&format!(
                "sha256:{}",
                "1".repeat(64)
            ))
            .unwrap(),
            ..base.clone()
        };
        let base_id = engine_request_id(&base, "cassette");
        let exact_id = engine_request_id(&exact, "cassette");
        assert_ne!(base_id, exact_id);
        let changed_manifest = NegotiatedRequest {
            manifest_id: "b".repeat(64),
            ..base.clone()
        };
        let changed_manifest_id = engine_request_id(&changed_manifest, "cassette");
        assert_ne!(base_id, changed_manifest_id);

        let mut changed_request_ids = vec![exact_id, changed_manifest_id];
        let changed_descriptors = [
            Descriptor {
                operation: "cooperative-target-v2",
                ..DESCRIPTOR.clone()
            },
            Descriptor {
                request_schema: "vh-cooperative-request-v3",
                ..DESCRIPTOR.clone()
            },
            Descriptor {
                outcome_schema: "vh-cooperative-outcome-v3",
                ..DESCRIPTOR.clone()
            },
            Descriptor {
                receipt_schema: "vh-cooperative-receipt-v3",
                ..DESCRIPTOR.clone()
            },
            Descriptor {
                verifier_schema: "vh-cooperative-verify-v3",
                ..DESCRIPTOR.clone()
            },
            Descriptor {
                observation_subject: "cooperative-child-source-v2",
                ..DESCRIPTOR.clone()
            },
            Descriptor {
                revision_algorithm: "sha512",
                ..DESCRIPTOR.clone()
            },
            Descriptor {
                revision_policy: "unbound-allowed",
                ..DESCRIPTOR.clone()
            },
            Descriptor {
                execution_binding: "different-binding",
                ..DESCRIPTOR.clone()
            },
            Descriptor {
                observation_to_exec_channel: "closed",
                ..DESCRIPTOR.clone()
            },
        ];
        for descriptor in changed_descriptors {
            let changed_id = engine_request_id_for_descriptor(&base, "cassette", &descriptor);
            assert_ne!(base_id, changed_id);
            changed_request_ids.push(changed_id);
        }

        let changed_features = NegotiatedRequest {
            features: vec!["different-feature-v1".into()],
            ..base.clone()
        };
        let changed_feature_id = engine_request_id(&changed_features, "cassette");
        assert_ne!(base_id, changed_feature_id);
        changed_request_ids.push(changed_feature_id);
        let changed_cassette_id = engine_request_id(&base, "different-cassette");
        assert_ne!(base_id, changed_cassette_id);
        changed_request_ids.push(changed_cassette_id);

        let base_evidence = evidence_id(&base_id, &"1".repeat(64), "first", "second");
        for changed_request_id in changed_request_ids {
            let changed_evidence =
                evidence_id(&changed_request_id, &"1".repeat(64), "first", "second");
            assert_ne!(base_evidence, changed_evidence);
        }
        assert_ne!(
            base_evidence,
            evidence_id(&base_id, &"2".repeat(64), "first", "second")
        );
    }
}
