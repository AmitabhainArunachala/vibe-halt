# Evidence sovereignty without a root oracle

Research ticket: [Design signed evidence and hidden holdouts without a root
oracle](https://github.com/AmitabhainArunachala/vibe-halt/issues/103)

Accepted Vibe Halt base: `d19ba9e1198c0ab7ef4bb1bdf69a299d056f9754`.

Ratification status: **research recommendation only**. Human-in-the-loop ticket
[#110](https://github.com/AmitabhainArunachala/vibe-halt/issues/110) remains the
authority to accept, amend, or reject this topology; this artifact does not
ratify it.

## Verdict

For a **settlement-grade** v1, the research recommendation is not one signing
key and not a blockchain. It is a typed, thresholded evidence graph:

1. exact evidence blobs remain SHA-256 content addressed;
2. each claim is an in-toto Statement in its own DSSE envelope;
3. a 2-of-3 offline TUF policy root authorizes narrowly typed roles, schemas,
   key epochs, log keys, and revocations, but has no verdict-signing role;
4. treatment, holdout curation, execution, and admission use distinct
   credentials and write surfaces;
5. two independent judges sign the **same** admission assessment, and the pure
   projection is `PROCEED` or `HALT` only on 2-of-2 agreement; valid-but-missing
   or unavailable evidence projects to `UNKNOWN`, while a wrong subject,
   digest, signature, schema, or contradictory closure makes the attempt
   `Invalid` rather than manufacturing a verdict;
6. every credit-bearing object is included in one public transparency log and
   a checkpoint countersigned by two independent witnesses before it may bind;
7. two independent holdout curators/custodians jointly commit a secret pool
   before candidate freeze, commit separate random selector shares, and reveal
   them only after the exact candidate is frozen and its first execution is
   checkpointed; a missing signature or reveal burns the attempt as `UNKNOWN`,
   never permits reselection; and
8. the final reveal publishes the full denominator, including misses and
   aborted attempts, so anyone can recompute the commitment, selection, and
   verdict.

A first-campaign mechanism grade may operate with fewer organizations or
replicas, but it must publish that weaker independence/liveness grade and must
not be marketed or consumed as settlement-grade evidence. Human ticket #110
decides whether this full topology is the smallest acceptable product v1, a
later grade, or should be replaced; research does not smuggle that choice into
the word “credible.”

This topology has a distributed **trust root**: a TUF client starts with trusted
root metadata and follows threshold-authorized updates from there. It has no
**root oracle**: root signatures can authorize which identity may utter which
statement type, but cannot turn that
statement into observation, replay, proof, or permission. TUF explicitly
supports threshold roles and out-of-band root bootstrap, while accepted Vibe
Halt already makes authority and epistemic modality orthogonal
([TUF specification §§2.1.1, 5.2](https://theupdateframework.github.io/specification/v1.0.26/#root-role),
[Vibe Halt modality boundary, lines 1–9](https://github.com/AmitabhainArunachala/vibe-halt/blob/d19ba9e1198c0ab7ef4bb1bdf69a299d056f9754/crates/vh-cli/src/modality.rs#L1-L9),
[promotion evaluator, lines 92–138](https://github.com/AmitabhainArunachala/vibe-halt/blob/d19ba9e1198c0ab7ef4bb1bdf69a299d056f9754/crates/vh-cli/src/modality.rs#L92-L138)).

The constitutional type rule is:

```text
VerifySignature<Statement<T>, Authorized<Role>, PolicyEpoch>
    -> Attributed<T, Role>                 // never Observed, Replayed, or Proven

AttemptValidity = Valid | Invalid(IntegrityViolation)

Project<ClosedEvidenceSet, AdmissionQuorum<PolicyVersion>, FreshPolicy>
    -> Assessment<Action> = HALT | PROCEED | UNKNOWN
```

There must be no constructor from `Signature`, `RootApproval`, `LogInclusion`,
or `OperatorOverride` directly to `PROCEED`.

This research instantiates `AdmissionQuorum` as 2-of-2. The root may rotate or
narrow authorities inside that protocol, but it may not rewrite the frozen
quorum rule, distinct-seat constraint, monotonic treatment of missing evidence,
or `UNKNOWN` dominance. Those are verifier semantics. A change requires a new
protocol/client version and human ratification; it cannot arrive as ordinary
TUF metadata, or the policy root would become the forbidden root oracle by
indirection.

## What accepted main proves—and does not

- Product Lock already requires exact source revision, an honest coverage
  ledger, independently replayable evidence where supported, and a bounded
  `HALT | PROCEED | UNKNOWN` target decision; unresolved mandatory coverage is
  `UNKNOWN`, and `UNKNOWN` outranks `PROCEED`
  ([Product Lock, lines 17–24](https://github.com/AmitabhainArunachala/vibe-halt/blob/d19ba9e1198c0ab7ef4bb1bdf69a299d056f9754/docs/specs/PRODUCT_LOCK_V1.md#L17-L24),
  [lines 31–49](https://github.com/AmitabhainArunachala/vibe-halt/blob/d19ba9e1198c0ab7ef4bb1bdf69a299d056f9754/docs/specs/PRODUCT_LOCK_V1.md#L31-L49),
  [lines 68–83](https://github.com/AmitabhainArunachala/vibe-halt/blob/d19ba9e1198c0ab7ef4bb1bdf69a299d056f9754/docs/specs/PRODUCT_LOCK_V1.md#L68-L83)).
- The accepted finding bundle hashes the exact preceding NDJSON bytes with
  SHA-256 and rejects structural or digest mismatch, but it explicitly does
  **not** provide a signature or authenticated provenance
  ([receipts v2, lines 1–31](https://github.com/AmitabhainArunachala/vibe-halt/blob/d19ba9e1198c0ab7ef4bb1bdf69a299d056f9754/crates/vh-cli/src/receipts_v2.rs#L1-L31), [lines 138–153](https://github.com/AmitabhainArunachala/vibe-halt/blob/d19ba9e1198c0ab7ef4bb1bdf69a299d056f9754/crates/vh-cli/src/receipts_v2.rs#L138-L153)).
- The SHA-256 implementation is pure safe Rust and accepted main has an
  independent black-box known-answer suite; neither fact identifies a signer
  or trust anchor, which the digest crate explicitly leaves out of scope
  ([digest implementation, lines 1–20](https://github.com/AmitabhainArunachala/vibe-halt/blob/d19ba9e1198c0ab7ef4bb1bdf69a299d056f9754/crates/vh-digest/src/lib.rs#L1-L20),
  [independent vectors, lines 1–35](https://github.com/AmitabhainArunachala/vibe-halt/blob/d19ba9e1198c0ab7ef4bb1bdf69a299d056f9754/crates/vh-verify/tests/digest_kat.rs#L1-L35)).
- Trace-v0's FNV-1a-128 is deliberately frozen, legacy, and internal. It must
  remain a replay-compatibility identity rather than being promoted into a
  cross-party security primitive
  ([trace source, lines 1–21](https://github.com/AmitabhainArunachala/vibe-halt/blob/d19ba9e1198c0ab7ef4bb1bdf69a299d056f9754/crates/vh-trace/src/lib.rs#L1-L21),
  [trace specification, lines 301–315](https://github.com/AmitabhainArunachala/vibe-halt/blob/d19ba9e1198c0ab7ef4bb1bdf69a299d056f9754/docs/specs/TRACE_FORMAT_V0.md#L301-L315)).
- The present holdout commitment has a **public synthetic salt** and says it
  carries no real selection secret; it is a shape check, not a hidden holdout
  ([Holdout Contract, lines 72–90](https://github.com/AmitabhainArunachala/vibe-halt/blob/d19ba9e1198c0ab7ef4bb1bdf69a299d056f9754/docs/specs/HOLDOUT_CONTRACT_V1.md#L72-L90)).
- Current admission correctly prevents parsed receipt bytes from reconstructing
  the private fresh-run proof, but its authority is local `RUST_FRESH_REPLAY`,
  not cross-party identity
  ([admission, lines 1–20](https://github.com/AmitabhainArunachala/vibe-halt/blob/d19ba9e1198c0ab7ef4bb1bdf69a299d056f9754/crates/vh-cli/src/admission.rs#L1-L20),
  [lines 266–303](https://github.com/AmitabhainArunachala/vibe-halt/blob/d19ba9e1198c0ab7ef4bb1bdf69a299d056f9754/crates/vh-cli/src/admission.rs#L266-L303)).
- The ratified acceptance denominator already requires an independently
  curated, preregistered, candidate-secret holdout, retained misses, frozen
  engine and policy identities, an opaque authenticated commitment, a later
  reveal, and first-run-only credit. The missing part is a concrete trust and
  secrecy protocol
  ([Build Plan, lines 63–93](https://github.com/AmitabhainArunachala/vibe-halt/blob/d19ba9e1198c0ab7ef4bb1bdf69a299d056f9754/docs/plans/VIBE_HALT_BUILD_PLAN_2026-07-20.md#L63-L93)).

The v1 design should therefore wrap the accepted content identities; it should
not rewrite the trace format or pretend a signature repairs an incomplete
observation.

## What the primary systems contribute

| System | Adopt | Refuse to infer |
|---|---|---|
| DSSE | Sign exact payload bytes plus an application-specific payload type using PAE; verify before parsing; keep one role statement per envelope. DSSE also specifies application-chosen `(t,n)` verification. ([DSSE protocol §§Signature Definition, Protocol, Multi-signature Verification](https://github.com/secure-systems-lab/dsse/blob/1d3370f62565bca041e97c8310b873ac340edc2e/protocol.md#signature-definition)) | DSSE deliberately leaves algorithms, key management, trust establishment, canonicalization, semantics, and policy to the application; `keyid` is only an unauthenticated lookup hint. ([DSSE scope](https://github.com/secure-systems-lab/dsse/blob/1d3370f62565bca041e97c8310b873ac340edc2e/governance/02-scope.md)) |
| in-toto Attestation Framework | Bind each claim to immutable subject digests and an explicit predicate type. Design the consumer monotonically: omitting an attestation can never turn deny into allow. ([Statement v1](https://github.com/in-toto/attestation/blob/051624ce466deaed4c5a66e66877f69b471fccbe/spec/v1/statement.md), [v1 parsing rules](https://github.com/in-toto/attestation/blob/051624ce466deaed4c5a66e66877f69b471fccbe/spec/v1/README.md#parsing-rules)) | Do not use an in-toto JSONL Bundle as the evidence closure. The specification says the bundle is not authenticated as a whole, so valid attestations can be deleted or replayed. Vibe Halt's content-addressed closure manifest must enumerate every required role-signed envelope digest. ([Bundle v1](https://github.com/in-toto/attestation/blob/051624ce466deaed4c5a66e66877f69b471fccbe/spec/v1/bundle.md#bundle-layer-specification)) |
| Sigstore | Optionally use short-lived identity certificates, transparency inclusion material, and offline-verifiable bundles for online workflow identities. Require certificate identity **and** issuer policy, not a bare key hint. ([keyless signing flow](https://docs.sigstore.dev/cosign/signing/overview/#the-signing-witnessing-and-verifying-process), [verification](https://docs.sigstore.dev/cosign/verifying/verify/#keyless-verification-using-openid-connect)) | Sigstore says it can attribute a signature to control of a digital identity, not establish that the artifact is good or that the identity should be trusted. Fulcio, OIDC, Rekor, monitors, and the TUF root are explicit compromise surfaces. ([Sigstore threat model, lines 216–240 and 295–307](https://github.com/sigstore/docs/blob/35180becb3f9c68ef39ccab9b4b4616170b3d237/content/en/about/threat-model.md#policy-considerations)) |
| Sigstore Bundle | Carry the exact certificate/public-key verification material, inclusion proof, and optional signed time needed for offline checking. Use one bundle per signer: the current bundle schema requires exactly one signature in a DSSE envelope. ([bundle proto, `VerificationMaterial` and `Bundle`](https://github.com/sigstore/protobuf-specs/blob/0342fe5797edd558c58098033220fb27a2542a28/protos/sigstore_bundle.proto#L79-L149), [Rekor entry proto](https://github.com/sigstore/protobuf-specs/blob/0342fe5797edd558c58098033220fb27a2542a28/protos/sigstore_rekor.proto#L100-L149)) | A bundle-carried root certificate is not a root of trust: the schema requires verifiers to chain to a CA they independently trust. Inclusion proves log membership under the trusted log key, not statement truth. ([bundle proto verification-material rules](https://github.com/sigstore/protobuf-specs/blob/0342fe5797edd558c58098033220fb27a2542a28/protos/sigstore_bundle.proto#L56-L80)) |
| TUF | Distribute a versioned threshold policy root, delegated role keys, schema/algorithm allowlists, expiry, and revocation. Root rotation must satisfy both old and new thresholds; expired or rolled-back metadata fails closed. ([TUF §§4.3, 5.3, 6.1](https://theupdateframework.github.io/specification/v1.0.26/#file-formats-rootjson)) | TUF does not remove bootstrap trust. Clients ship an initial root, and compromise of a threshold of root keys requires out-of-band recovery. The root is therefore a distributed authority registry, not evidence of a run. ([TUF root role](https://theupdateframework.github.io/specification/v1.0.26/#root-role)) |
| Transparency log + witnesses | Require an inclusion proof to a signed checkpoint, then witness cosignatures over append-only consistency. This makes publication and equivocation challengeable. ([RFC 9162 §§1, 2.1.3–2.1.4](https://www.rfc-editor.org/rfc/rfc9162.html), [transparency-dev witness](https://github.com/transparency-dev/witness/blob/6247fc953c2a4606763ff41b0a92d02076ebca0e/README.md#importance-of-witnesses)) | A log does not prevent a false claim, preserve an unavailable evidence blob, or by itself prevent a split view. CT explicitly says logs make misissuance detectable rather than impossible and that cross-client consistency needs shared observations. ([RFC 9162 §§1, 11.3](https://www.rfc-editor.org/rfc/rfc9162.html#section-1)) |
| VRF / public beacon | A VRF can prove a unique pseudorandom output for a key/input, and drand implements publicly verifiable threshold randomness without one generator. These are useful later for selector agility. ([RFC 9381 §§3.3–3.4](https://www.rfc-editor.org/rfc/rfc9381.html#section-3.3), [drand design](https://github.com/drand/drand/blob/2363f3b9ba5fd6f14e0b84a096b248479790d75d/README.md#public-randomness)) | A single VRF key holder can compute its output early, so it is the wrong sole holdout selector. A public beacon adds an external key, availability, and timing dependency. v1 should revive a beacon only if two-custodian reveal liveness is empirically unacceptable. |

Sigstore's own root-signing implementation is useful evidence that TUF can
deliver a publicly auditable trust root through geographically and
organizationally distributed keyholders; it is an implementation pattern, not
a reason to inherit Sigstore's public-good instance as Vibe Halt's sovereign
root ([Sigstore root-signing](https://github.com/sigstore/root-signing/tree/60cf2ce8b2e45d600dc17c4421e8c751e75772be)).

## Smallest credible v1 topology

| Seat | Minimum v1 identity and threshold | May sign | Cannot establish |
|---|---|---|---|
| Policy root | 3 offline, separately held root keys; 2 signatures required | Role delegations, accepted schemas/algorithms, policy epoch, log/witness/beacon keys, expiry, revocation | A run occurred; a property is adequate; `HALT` or `PROCEED` |
| Treatment | 1 online workflow identity, untrusted by admission | Candidate/base/diff/path-set freeze and treatment provenance | Any evidence grade, channel closure, or verdict |
| Holdout curators/custodians A/B | 2 identities and credential stores independent of treatment; 2-of-2 on the same pool commitment/reveal, plus one selector share each | Eligibility, reference labels and supporting source evidence, pool/oracle commitment and reveal, retained denominator, one 256-bit selector-share commitment/reveal each | Label truth, detection, replay, admission, or proof that secrecy was never breached |
| Campaign runner | 1 isolated workflow identity with no treatment write access | Exact run statement, execution envelope, capability ledger, result and evidence-closure inputs | Independent replay or admission |
| Judges A/B | 2 principals on separately administered hosts; 2-of-2 identical assessments | Replay statements and the same final assessment bytes | Merge/deploy, global safety, or a modality above the evidence consumed |
| Transparency | 1 public append-only log plus 2 independent checkpoint witnesses | Inclusion and consistency commitments | Claim truth, evidence availability, or policy adequacy |
| Evidence storage | 2 independently administered content-addressed replicas | Exact blobs named by SHA-256 and byte length | Authenticity or truth |

One judge may use a Sigstore keyless workflow identity; the other should use a
different identity issuer or a hardware-backed self-managed key, so compromise
of one OIDC issuer or Fulcio path does not forge both seats. Sigstore's threat
model explicitly treats OIDC account, OIDC issuer, Fulcio, Rekor, monitors, and
the TUF root as distinct compromise cases
([Sigstore threat model](https://github.com/sigstore/docs/blob/35180becb3f9c68ef39ccab9b4b4616170b3d237/content/en/about/threat-model.md#sigstore-threat-model)).

“Independent” must be a receipt, not an adjective. Each judge statement must
name its principal, issuer, credential class, organization, host/controller
identity, verifier artifact digest, and network/storage dependencies. v1 earns
only an `I1` independence grade when credentials and hosts differ; sharing an
organization, cloud, IdP, source tree, or verifier implementation remains an
explicit common-mode channel. A later `I2` grade may require distinct
organizations, providers, and verifier implementations.

The 2-of-2 rule intentionally lets one judge deny liveness but not manufacture
permission. Disagreement, loss, or compromise suspicion becomes `UNKNOWN`.
Individual findings remain publishable immediately; a binding `HALT` requires
the same independent replay quorum as a binding `PROCEED`. This is fail-safe,
not censorship-free: either judge can withhold and force `UNKNOWN`, which is an
effective veto wherever normal action requires `PROCEED`. That liveness and
sovereignty cost is part of human ratification, not something cryptography
erases.

## Evidence graph and closure

Every predicate is versioned and carried as an in-toto Statement whose
`subject` contains the immutable artifacts to which the claim applies. DSSE
authenticates the exact statement bytes and predicate type; the verifier maps
the verified key/certificate to a role under a fresh policy epoch
([in-toto Statement v1](https://github.com/in-toto/attestation/blob/051624ce466deaed4c5a66e66877f69b471fccbe/spec/v1/statement.md),
[DSSE protocol](https://github.com/secure-systems-lab/dsse/blob/1d3370f62565bca041e97c8310b873ac340edc2e/protocol.md)).

The minimum predicate set is:

| Predicate | Required bindings |
|---|---|
| `vh.candidate-freeze/v1` | base revision, candidate revision/tree, diff digest, path set, signed materialization-receipt id, target manifest, final campaign id, treatment identity, attempt id |
| `vh.holdout-commit/v1` | hidden-pool commitment, pool count, cutoff/eligibility/diversity policy, oracle digest, curator-custodian identities and selector-share commitments, pre-commit `CampaignSpecId` |
| `vh.run/v1` | candidate, engine/runner/controller artifacts, execution envelope, all 29 capability statuses and evidence, final campaign/property/palette/budget identities, holdout commitment, raw results and evidence-object digests |
| `vh.replay/v1` | exact run statement, independently fetched artifact digests, verifier/host identity, reproduced or divergent observations, every unresolved channel |
| `vh.assessment/v1` | post-reveal evidence-closure digest, policy epoch, action type, exact bounded `Assessment<Action> = HALT | PROCEED | UNKNOWN`, reasons and missing obligations, pure `GovernabilityProjectionPayloadId`, action-specific `GovernabilityGateDecision`, and admission-quorum policy id |
| `vh.holdout-reveal/v1` | canonical pool bytes, commitment nonce, both selector shares, deterministic selection transcript, every selected result including misses/invalids/aborts |
| `vh.censure/v1` | exact prior statement/verdict digest, challenger identity, reason and new evidence; it is an attributed challenge and has no direct modality effect |

After the first hidden holdout execution is durably checkpointed, the final
`EvidenceClosure` is a strictly parsed, length-framed, sorted manifest of every
payload, envelope, verification-material, log-proof, holdout commitment and
reveal, run, channel ledger, abort, retained outcome, and artifact blob by
digest and byte length. Judges A and B independently fetch those exact blobs,
replay the frozen campaign, and sign **identical assessment payload bytes**
naming that closure. The governability projection and decision are pure fields
in those bytes; they contain no judge signature or quorum witness. The ordered
judge attestations are assembled afterward into `AdmissionRecordId`, whose
verification derives both the admission-quorum witness and, when valid, the
governability-gate witness. Replay after reveal cannot earn holdout detection credit;
it tests reproducibility of the already-recorded first execution.

The pure projection first requires `AttemptValidity::Valid`, then consumes the
final closure and required assessment envelopes. An unavailable but otherwise
uncompromised required reveal, closure blob, assessment, or signature is
`UNKNOWN`. A malformed, wrong-subject, wrong-digest, invalid-signature,
wrong-schema, or contradictory object makes the attempt `Invalid` and produces
no target assessment. Only a valid complete projection can emit a binding
`HALT` or `PROCEED`. This avoids relying on an unauthenticated aggregate bundle,
whose deletion and replay limitations are explicit in the in-toto Bundle specification
([in-toto Bundle v1](https://github.com/in-toto/attestation/blob/051624ce466deaed4c5a66e66877f69b471fccbe/spec/v1/bundle.md#bundle-layer-specification)).

The trace-v0 FNV identity may appear as typed legacy data inside a run
statement; cross-party subjects and closure entries use the accepted SHA-256
content identity. A signature covers the SHA-bound statement but does not
retroactively convert the legacy trace hash into a cryptographic primitive
([Trace Format v0, lines 309–315](https://github.com/AmitabhainArunachala/vibe-halt/blob/d19ba9e1198c0ab7ef4bb1bdf69a299d056f9754/docs/specs/TRACE_FORMAT_V0.md#L309-L315)).

### Attribution is not truth

| Mechanism verifies | It does not verify |
|---|---|
| SHA-256 digest recomputes | who created the bytes; whether they are true |
| DSSE signature verifies under an authorized role key | the signer was uncompromised; the predicate is true |
| in-toto subject digest matches the target | the predicate covers every consequential path |
| transparency inclusion and witnessed consistency verify | the logged statement is correct; the blob remains available |
| holdout commitment opens | nobody leaked the preimage; the curator chose a fair eligible pool |
| independent replay matches | the oracle expresses human intent; untested channels are safe |
| two judges sign the same assessment | consensus is proof; authority may lift modality |

Sigstore states this boundary directly: its guarantees can identify control of
a digital identity but do not determine whether that identity should be
trusted or an artifact is good
([Sigstore threat model, policy considerations](https://github.com/sigstore/docs/blob/35180becb3f9c68ef39ccab9b4b4616170b3d237/content/en/about/threat-model.md#policy-considerations)).

## Hidden-holdout protocol v1

The protocol extends rather than reinterprets `vibe-halt.eval-dossier.v1`; the
existing public-salt commitment remains synthetic calibration data and earns no
secrecy claim
([Holdout Contract, lines 72–108](https://github.com/AmitabhainArunachala/vibe-halt/blob/d19ba9e1198c0ab7ef4bb1bdf69a299d056f9754/docs/specs/HOLDOUT_CONTRACT_V1.md#L72-L108)).

### Freeze and commit

Freeze `CampaignSpecId` first from the public manifest, property/oracle
contract, envelope/controller identities, seed-domain policy, budgets,
thresholds, authority view, and required evidence schemas. It contains no
holdout commitment. The commitment statements below bind that spec; only after
they exist is final `CampaignId` derived.

1. Curators A and B independently review and sign the same canonical eligible
   pool `M`, including entry identity, reference label and supporting evidence,
   oracle digest, repository/cluster labels, cutoff, dedupe outcome, and
   immutable ordering.
   The commitment is inadmissible with only one curator signature. They draw a
   fresh 256-bit nonce `n` from an approved random-bit generator and publish:

   ```text
   pool_commit = SHA256(
       frame("vh-hidden-pool-v1") ||
       frame(campaign_spec_id) || frame(M) || frame(n)
   )
   ```

   SHA-256 is standardized by FIPS 180-4; a cryptographic random-bit generator
   must use a specified DRBG/entropy construction rather than Vibe Halt's
   deterministic universe PRNG
   ([FIPS 180-4](https://csrc.nist.gov/pubs/fips/180-4/upd1/final),
   [NIST SP 800-90A Rev. 1](https://csrc.nist.gov/pubs/sp/800/90/a/r1/final)).
   The secret high-entropy nonce prevents cheap enumeration of a small pool;
   concealment is computational and assumption-bound, not perfect secrecy.

   The curators store the plaintext only in separated vaults and produce an
   HPKE-sealed ciphertext for the isolated runner. Before candidate freeze, they
   publish only its digest and recipient-key identity in the commitment
   statement; the ciphertext bytes remain embargoed in the curator vaults.
   HPKE is a specified hybrid public-key encryption construction for arbitrary
   plaintexts and recipient public keys, but its use here proves neither
   correct access control nor non-leakage
   ([RFC 9180 §§1, 5](https://www.rfc-editor.org/rfc/rfc9180.html#section-1)).

2. Curator-custodians A and B independently draw fresh 256-bit shares `rA` and
   `rB`, then publish DSSE-signed commitments before either share is revealed:

   ```text
   cA = SHA256(frame("vh-selector-share-v1") || frame(campaign_spec_id)
               || frame("A") || frame(rA))
   cB = SHA256(frame("vh-selector-share-v1") || frame(campaign_spec_id)
               || frame("B") || frame(rB))
   ```

   Commit-before-reveal is the classic remote coin-flipping shape: a party
   commits to its choice before learning the other choice, then opens it for
   verification. The protocol remains abortable, which v1 handles by failing
   closed rather than claiming fair termination
   ([Blum, *Coin Flipping by Telephone*](https://www.cs.cmu.edu/~mblum/research/pdf/coin/)).

3. The signed pool commitment, share commitments, selection algorithm, `N`,
   diversity caps, policy epoch, and accepted engine/adapter/oracle identities
   are included in a witnessed transparency checkpoint before treatment may
   freeze a candidate. Construct final
   `CampaignId = H(CampaignSpecId, ordered holdout-commitment statement ids)`;
   the candidate freeze and every run bind that final id. This preserves the
   existing law that the holdout and
   evaluation policy precede the credit run
   ([Build Plan, lines 69–84](https://github.com/AmitabhainArunachala/vibe-halt/blob/d19ba9e1198c0ab7ef4bb1bdf69a299d056f9754/docs/plans/VIBE_HALT_BUILD_PLAN_2026-07-20.md#L69-L84)).

4. Treatment signs and logs one exact `candidate-freeze` statement. The
   checkpoint containing it must reach the required witness quorum before
   either selector share is released. A new revision is a new attempt and
   cannot inherit the old candidate's freeze.

### Select, execute, and reveal

5. After witnessed freeze, the curators release the runner's committed
   ciphertext, and each custodian releases its signed share to the isolated
   holdout runner. The runner checks both commitments and derives one selector:

   ```text
   selector_seed = HKDF-SHA256(
       IKM  = frame(rA) || frame(rB),
       salt = SHA256(frame(pool_commit) || frame(candidate_digest)),
       info = frame("vh-holdout-selector-v1") ||
              frame(campaign_spec_id) || frame(policy_epoch)
   )
   ```

   HKDF supplies an explicit extract/expand construction and domain-specific
   `info`; Vibe Halt must freeze exact vectors and the deterministic
   without-replacement selector before any credit run
   ([RFC 5869](https://www.rfc-editor.org/rfc/rfc5869.html)).

6. Selection is computed over the committed canonical ordering. Every selected
   entry remains in the denominator whether it is detected, missed, unsupported,
   invalid, or crashes the evaluator. No retry can replace it. A custodian who
   never reveals leaves a structurally valid but incomplete attempt at
   `UNKNOWN`. A mismatched share, early reveal, commitment mismatch, altered
   ordering, candidate change, or treatment access is an integrity violation
   and makes the attempt `Invalid`; it does not become a product verdict.

7. The campaign runner performs the sole credit-bearing execution on the exact
   frozen candidate while the holdout remains hidden. Holdout materials live
   outside the target/treatment write surface. If the capability ledger cannot
   establish that the treatment and target lacked a pre-freeze or pre-execution
   read path to the pool, nonce, shares, or selected cases, the secrecy claim
   remains `UNKNOWN`; the current sandbox cannot close any of its 29 channels
   and therefore cannot issue this secrecy grade today
   ([Sandbox Capability Envelope, lines 14–33](https://github.com/AmitabhainArunachala/vibe-halt/blob/d19ba9e1198c0ab7ef4bb1bdf69a299d056f9754/docs/specs/SANDBOX_CAPABILITY_ENVELOPE_V1.md#L14-L33)).

8. After the first execution or abort is durably checkpointed, both curators
   sign the reveal of `M` and `n`, both publish their shares, and the system
   publishes every selected outcome and aborted attempt. The final
   `EvidenceClosure` is then logged. Both judges independently replay and sign
   identical assessments over that closure; anyone can recompute the pool
   commitment, selector, sample, denominator, replay result, and decision.
   Until the reveal, closure, and judge quorum verify, the public projection is
   `UNKNOWN`. Revealed material becomes calibration-only; every new adaptive
   candidate requires a new pool epoch, commitments, and first-run campaign,
   matching accepted holdout law
   ([Build Plan, lines 86–93](https://github.com/AmitabhainArunachala/vibe-halt/blob/d19ba9e1198c0ab7ef4bb1bdf69a299d056f9754/docs/plans/VIBE_HALT_BUILD_PLAN_2026-07-20.md#L86-L93)).

The security assumptions split by property. Before freeze, **both**
curator-custodians must keep `M` and the recipient ciphertext bytes confidential
because each has reviewed the full pool; either can destroy pool secrecy by
leaking it. At least one must honestly reject an ineligible pool, and at least
one must keep a full-entropy selector share secret to prevent pre-freeze
selection prediction. A last revealer can deny liveness but cannot choose a
replacement share; refusal burns the attempt as `UNKNOWN`. Cryptography cannot
prove that either curator never leaked `M`, so organizational separation and
capability evidence remain mandatory.

### Rejected path and revival falsifier

A single VRF selector is rejected for v1 because RFC 9381 pseudorandomness is
defined against parties that do not know the secret key; the key holder can
evaluate early and becomes a selector oracle
([RFC 9381 §3.4](https://www.rfc-editor.org/rfc/rfc9381.html#section-3.4)).

A threshold public beacon such as drand is the recorded revival path. Revive it
only if preregistered operations show that custodian non-reveal invalidates more
than the allowed campaign budget or makes independent administration
impractical. A revived design must pin the beacon group key, prove that the
chosen round was unknowable before candidate freeze, retain aborts, and map
beacon unavailability to `UNKNOWN` but an invalid proof to `Invalid`; drand describes its output
as threshold-generated, publicly verifiable randomness and requires clients to
verify against the collective public key
([drand implementation and protocol overview](https://github.com/drand/drand/blob/2363f3b9ba5fd6f14e0b84a096b248479790d75d/README.md#public-randomness)).

## Transparency and public audit

Every commitment, freeze, run, replay, assessment, reveal, policy update,
revocation, and censure is logged by envelope digest. A settlement-grade
consumer requires:

1. exact envelope bytes and verification material;
2. an inclusion proof to a signed checkpoint;
3. two authorized witness cosignatures on that checkpoint;
4. a fresh, non-rolled-back TUF policy root that authorizes the signer, log,
   and witnesses for the statement's policy epoch; and
5. all blobs in the signed `EvidenceClosure` available from two replicas.

Merkle inclusion establishes membership and consistency proofs establish
append-only evolution; independent witnesses retain a prior checkpoint, verify
consistency, and countersign the new one to resist a log's split view
([RFC 9162 §§2.1.3–2.1.4](https://www.rfc-editor.org/rfc/rfc9162.html#section-2.1.3),
[transparency-dev witness workflow](https://github.com/transparency-dev/witness/blob/6247fc953c2a4606763ff41b0a92d02076ebca0e/README.md#importance-of-witnesses)).

Public logging makes a claim discoverable, not true. Logging only a digest of
confidential evidence permits public chronology and equivocation audit but not
public semantic replay. Such a campaign must label itself `PUBLIC_COMMITMENT /
PRIVATE_AUDIT`, never `PUBLICLY_REPLAYABLE`, until the exact artifacts are
available to every claimed auditor. RFC 9162 likewise describes transparency
as making misissuance detectable, not preventing it
([RFC 9162 §1](https://www.rfc-editor.org/rfc/rfc9162.html#section-1)).

## Rotation, revocation, and censure

The TUF root publishes the accepted role keys or identity constraints,
thresholds, algorithm suites, expiry, policy epoch, log/witness keys, and
revocation records. Root private keys remain offline. A root update is accepted
only when signed by the old threshold and the new threshold in monotonically
versioned metadata; stale or expired metadata maps the entire admission check
to `UNKNOWN`
([TUF §§5.3 and 6.1](https://theupdateframework.github.io/specification/v1.0.26/#update-the-root-role)).

Every evidence statement binds `policy_epoch`, signer identity, statement
scope, target/campaign digest, and a transparency proof or signed timestamp.
Revocation is append-only and has two distinct types:

- **Credential compromise:** publish the affected identity/key and an effective
  checkpoint/time bound. Evidence demonstrably included before the bound may
  remain attributable under the frozen historical policy; evidence at or after
  the bound is invalid. If the compromise time is unknown, the affected epoch
  becomes `UNKNOWN`. Sigstore's own TUF guidance distinguishes compromise time
  so legitimate signatures before it can continue to verify
  ([Sigstore threat model, lines 284–294](https://github.com/sigstore/docs/blob/35180becb3f9c68ef39ccab9b4b4616170b3d237/content/en/about/threat-model.md#secure-distribution-and-revocation-of-sigstore-key-material)).
- **Semantic censure:** a sound signature may still carry a bad oracle,
  incomplete channel ledger, or false assertion. Anyone may publish an
  attributable `vh.censure/v1` challenge pointing to the exact evidence/verdict
  digest, but neither the challenger nor the policy root may change modality.
  Only a fresh 2-of-2 judge assessment over a new evidence closure can
  supersede the prior assessment as `UNKNOWN` or `HALT`. Key rotation alone
  must never imply that the old statement was true or false.

Revocation and censure never delete history and never promote modality. A root
threshold compromise requires an out-of-band rebootstrap and suspends all
affected settlement decisions as `UNKNOWN`; TUF warns that recovery after
threshold root compromise is extremely difficult
([TUF root role](https://theupdateframework.github.io/specification/v1.0.26/#root-role)).

## Threat model

| Threat | v1 control | Residual / fail-closed result |
|---|---|---|
| Treatment edits its result or substitutes a candidate | SHA-bound subjects, signed freeze, closure manifest, no treatment key in admission roles | Unobserved loader/filesystem substitution remains a channel; `UNKNOWN` |
| Treatment edits the judge, oracle, holdout, or trust config | Separate repositories/write surfaces, credentials, hosts, and TUF role paths; statements bind artifact digests | Host/admin compromise or collusion is not cryptographically disproved; disclose common mode or `UNKNOWN` |
| One online signer or judge key is compromised | Role-scoped keys, 2-of-2 assessment, short epochs, transparency, rotation/revocation | One judge can deny liveness, not create binding `HALT`/`PROCEED`; result `UNKNOWN` |
| One policy-root key is compromised | 2-of-3 offline threshold | Two compromised root keys are catastrophic; out-of-band recovery and epoch suspension |
| Log omits, rewrites, or forks | Inclusion/consistency proofs plus two witness cosignatures and independent monitors | Collusion of log and required witnesses, or clients accepting unwitnessed checkpoints, defeats this layer |
| Evidence blobs disappear | Signed closure detects omission; two content-addressed replicas | Both replicas can fail; detectability is not availability, so `UNKNOWN` |
| Holdout is guessed, leaked, or selected to flatter the candidate | 2-of-2 curator approval, secret pool nonce, two committed selector shares, pre-freeze commitments, frozen eligibility/diversity law, post-run full reveal | Either curator can leak the shared pool; curator collusion and biased ground truth remain institutional risks; no secrecy claim without access evidence |
| A custodian aborts after seeing the other reveal | Both commitments precede reveal; all-of-two reveal; attempt id is burned and logged | Availability loss is accepted; never choose a replacement share or silently retry |
| Treatment grinds many candidates/attempts | One public attempt id per base/scope, every freeze/abort retained, revealed holdout becomes calibration-only | Private unlogged experiments cannot be detected cryptographically; admission recognizes only registered attempts |
| Old valid evidence is replayed under new scope or policy | Exact subject/campaign/policy epoch, TUF versions/expiry, closure graph, signed censure | Historical signatures remain historical facts; current projection can be `UNKNOWN` |
| Two judges share one common failure | Independence receipt enumerates issuer, org, host, implementation, and dependencies | v1 `I1` is not proof of organizational or implementation diversity |
| Candidate detects the evaluator and behaves selectively | Exact execution envelope, controller set, capability ledger, fresh holdouts | A remaining observation/escape channel is `UNKNOWN`, not “passed” |

The design assumes correct standard cryptographic implementations, fewer than
two of three root authorities malicious or their keys compromised, both
curator-custodians preserving pool confidentiality, at least one curator
enforcing eligibility and preserving an unpredictable selector share, at least
one honest judge (with 2-of-2 permission), at least one honest checkpoint
witness, and enforcement of the declared write/access boundaries. Each
assumption is a named channel, not an invisible theorem.

## Proof obligations for a v1 prototype

1. A one-byte change to any payload, envelope, verification material, log
   proof, or artifact invalidates the closure or signature.
2. Changing `payloadType`, predicate version, subject digest, campaign,
   candidate, path set, policy epoch, execution envelope, property, palette,
   budget, or controller set fails closed.
3. Unknown or missing predicate types are monotonic: omission can only preserve
   or lower standing, never produce `PROCEED`, matching in-toto's monotonic
   policy guidance
   ([in-toto v1 parsing rules](https://github.com/in-toto/attestation/blob/051624ce466deaed4c5a66e66877f69b471fccbe/spec/v1/README.md#parsing-rules)).
4. A treatment, runner, curator, root, log, or single judge signature cannot
   satisfy either admission quorum seat.
5. The same principal, credential, host identity, or caller-supplied key cannot
   satisfy both judge seats.
6. Judges independently fetch all post-reveal evidence-closure blobs, verify
   exact bytes, replay the admitted campaign, and sign identical assessment
   payloads. Divergence or assessment-byte mismatch yields `UNKNOWN`.
7. Expired, rolled-back, missing, or threshold-insufficient TUF metadata yields
   `UNKNOWN`; old-to-new root rotation passes only the dual-threshold chain
   required by TUF
   ([TUF §5.3](https://theupdateframework.github.io/specification/v1.0.26/#update-the-root-role)).
8. Evidence after a revocation bound is rejected; ambiguous compromise time
   demotes the whole affected epoch. Semantic censure works even when the
   original signature remains valid.
9. Missing inclusion proof, invalid checkpoint, absent witness quorum, or
   unavailable closure blob yields `UNKNOWN`.
10. The hidden-pool commitment and both selector commitments reproduce exactly;
    early/missing/mismatched/reused reveal, changed pool order, deleted miss, or
    second credit attempt is invalid.
11. A public post-run verifier can recompute the selected cohort, denominator,
    every retained outcome, both judge assessments, and final deterministic
    projection without trusting a Vibe Halt server.
12. The legacy FNV trace hash is never accepted where a SHA-256 subject,
    envelope, or closure digest is required.
13. An external authority act may ratify scope or record an override claim, but
    it cannot alter the evidence modality or make `UNKNOWN` project to
    `PROCEED`, matching accepted `AUTHORITY_CANNOT_LIFT_MODALITY_V1`
    ([modality evaluator, lines 92–138](https://github.com/AmitabhainArunachala/vibe-halt/blob/d19ba9e1198c0ab7ef4bb1bdf69a299d056f9754/crates/vh-cli/src/modality.rs#L92-L138)).

## Explicit non-claims

This topology does **not** claim:

- a signature, signer quorum, transparency entry, token vote, or policy-root
  action is true;
- key possession proves personhood, organizational independence, lack of
  compromise, or lack of collusion;
- a hash commitment proves its preimage was never leaked;
- holdout performance proves exhaustive coverage, general safety, property
  adequacy, or behavior outside the recorded target/envelope/budget;
- replay proves the real world, a native loader, hardware, or any Open channel
  matched the recorded model;
- log inclusion guarantees availability, confidentiality, correctness, or
  universal observation;
- revocation erases history or repairs a false historical claim;
- two judges agreeing constitutes formal proof;
- `PROCEED` authorizes merge, deployment, spending, minting, voting, or any
  other consequence outside the explicitly named external authority act; or
- this research artifact implements, certifies, or operationally deploys any
  cryptographic system.

## Falsifiers and revival rules

The evidence-sovereignty design is falsified for settlement use if any of these
is demonstrated:

- one credential, one administrator, or one service can manufacture a binding
  `PROCEED`;
- the policy root can sign a verdict or silently raise modality;
- treatment can supply a judge key, alter judge/oracle/holdout/evidence code,
  read the hidden pool before freeze, or omit a failed attempt;
- deleting one required attestation from the aggregate still yields
  `PROCEED`;
- either judge can validate a different closure or assessment payload;
- a stale/revoked role or unwitnessed/forked log view remains admissible;
- holdout reveal cannot reproduce the exact committed pool, selected cohort,
  and denominator including misses; or
- public marketing says “publicly replayable” when auditors can see only a
  digest and cannot retrieve the exact evidence.

Operational falsification does not justify weakening the invariant. If 2-of-2
judging or all-of-two selector reveal causes unacceptable liveness/cost, retain
the resulting `UNKNOWN` receipts and revisit topology. The recorded selector
revival is a threshold public beacon; the recorded transparency revival is two
independent logs if witness tooling cannot meet the checkpoint contract; the
recorded identity revival is hardware-backed role keys if a shared
OIDC/Fulcio path prevents meaningful independence. Each path revives only when
the measured failure condition changes.

## Recommendation for human ratification

Ticket #110 should accept, amend, or reject **DSSE/in-toto claims + TUF
threshold role policy + 2-of-2 independent admission + witnessed transparency
and a two-party hidden selector commit/reveal** as the v1 evidence-sovereignty
shape. Sigstore is permitted as
verification material and identity plumbing, never as a truth source. A
signature constructs only attributable speech; replay and closed-channel
evidence may lift modality; the deterministic policy projection alone emits a
bounded verdict; no internal or external authority can lift `UNKNOWN` into
permission.
