# Keep the settlement typechecker non-sovereign

Research resolution for [Keep the settlement typechecker non-sovereign](https://github.com/AmitabhainArunachala/vibe-halt/issues/105).

Ratification status: **research recommendation only**. Human gates
[#116](https://github.com/AmitabhainArunachala/vibe-halt/issues/116) and
[#110](https://github.com/AmitabhainArunachala/vibe-halt/issues/110) decide the
institutional seats and cryptographic quorum/custody respectively.

## Verdict

Vibe Halt must be a **constitutional witness and admission court, never the actor whose consequences it grades**. It may freeze scope, produce attributable observations, evaluate a versioned policy, and retain an append-only history. It must not merge, deploy, transfer, mint, tally, waive, or silently convert an `UNKNOWN` into a `PROCEED`.

The smallest honest v1 topology has five mechanically distinct
authority-bearing seats plus one untrusted treatment guest. Treatment is a
product function but carries no authority over its own candidate:

1. **Scope authority** proposes and freezes the declared subject, revisions,
   consequential paths, property contract, execution envelope, campaign budget,
   and policy version. It cannot run treatment, judge the result, or certify
   that its own scope is complete. Platform-mandatory consequence classes are
   non-excludable; exclusions are typed and expiring; an independent coverage
   assessment may force `UNKNOWN` or `UNGOVERNED`.
2. **Treatment guest** authors an untrusted candidate in a disposable write
   surface and has zero admission, scope, trust-root, holdout, or appeal weight
   for that candidate.
3. **Runner** executes the frozen campaign and signs an observation bound to the exact subject, revision, envelope, controller set, and campaign. A runner signature proves attribution, not truth.
4. **Independent admission judge or ratified quorum** verifies evidence and evaluates the frozen policy. It emits `Assessment<Action> = HALT | PROCEED | UNKNOWN`; it cannot edit the target, scope, evidence, or policy.
5. **Transparency service plus independent monitor** retains statements, receipts, supersessions, revocations, and external dispositions in a consistency-checkable history. Logging proves inclusion and makes equivocation challengeable; it does not prove the statement correct.
6. **External decision authority** alone chooses whether to merge, deploy, transfer, mint, or otherwise act. Its decision is a separately signed disposition. If it acts despite `UNKNOWN`, the underlying admission remains `UNKNOWN` forever.

This research distinguishes a mechanism grade, in which one organization may
operate several mechanically separated seats and publishes that weaker
authority grade, from a settlement grade with stronger organizational
independence. It does not choose which grade belongs in v1 or whether admission
uses one judge, 2-of-2, or another threshold; #110 does. Treatment and admission
must remain mechanically separate under every option.

For its own candidate, treatment has zero admission, holdout, scope-ratification,
trust-root, or appeal-quorum weight. “Not the sole judge” is too weak: placing
treatment inside a threshold still lets the writer influence its own admission.

## The invariant already present in Vibe Halt

Current `main` already encodes the seed of this constitution:

- authority and epistemic modality are orthogonal; changing authority preserves modality;
- promotion requires an adjacent, revision-matched witness;
- the accepted promotion evaluator exposes no transition to `Proven`, although
  the current public runtime `Claim` record is not yet a sealed proof type;
- current evidence content addresses are explicitly not signed or authenticated provenance.

See [`modality.rs` lines 1–9](https://github.com/AmitabhainArunachala/vibe-halt/blob/d19ba9e1198c0ab7ef4bb1bdf69a299d056f9754/crates/vh-cli/src/modality.rs#L1-L9),
[the public runtime `Claim` at lines 50–56](https://github.com/AmitabhainArunachala/vibe-halt/blob/d19ba9e1198c0ab7ef4bb1bdf69a299d056f9754/crates/vh-cli/src/modality.rs#L50-L56),
[promotion and authority change at lines 92–138](https://github.com/AmitabhainArunachala/vibe-halt/blob/d19ba9e1198c0ab7ef4bb1bdf69a299d056f9754/crates/vh-cli/src/modality.rs#L92-L138),
the [non-promotable `Proven` controller](https://github.com/AmitabhainArunachala/vibe-halt/blob/d19ba9e1198c0ab7ef4bb1bdf69a299d056f9754/crates/vh-cli/src/modality.rs#L261-L274),
and the [current receipts-v2 claim boundary](https://github.com/AmitabhainArunachala/vibe-halt/blob/d19ba9e1198c0ab7ef4bb1bdf69a299d056f9754/crates/vh-cli/src/receipts_v2.rs#L20-L31).

The institutional rule is the same rule lifted one level:

> Authority may authorize an act within its jurisdiction. It cannot improve the epistemic grade of the evidence on which it acts.

## Typed constitutional objects

The minimum objects should be distinct types, not optional fields on one mutable verdict:

```text
FrozenScope<
  SubjectId<SubjectDigest, MaterializationReceiptId>,
  BaseRevision,
  CandidateRevision,
  DiffDigest,
  ConsequentialPathSet,
  PropertyContract,
  Envelope,
  ControllerSet,
  CampaignId,
  Budget,
  PolicyVersion
>

Observation<Scope, EvidenceSet, RunnerIdentity, RunnerAuthorityGrade>

Admission<Scope, EvidenceClosure, PolicyVersion,
          Action, Assessment<Action>,
          GovernabilityProjectionPayloadId,
          GovernabilityGateDecision<Action, Scope, PolicyVersion>,
          AdmissionRecordId,
          AdmissionQuorumWitness<PolicyVersion>>
  where Assessment<Action> = HALT | PROCEED | UNKNOWN(OpenObligations)

ExternalDisposition<
  AdmissionDigest<Action>,
  Action,
  ExternalAuthority,
  Reason,
  Jurisdiction,
  Expiry
>

Appeal<OldAdmissionDigest, NewScopeOrEvidence>
Supersession<OldAdmissionDigest, NewAdmissionDigest>
Revocation<CredentialOrPolicyDigest, Reason, EffectiveTime>
```

These are semantic aliases of the acyclic identity graph in the execution
envelope research, not a parallel wire format. In particular, every admission
judge signs the same `AdmissionPayloadId`; the ordered signatures form
`AdmissionRecordId` only afterward. `SubjectId` carries the immutable
materialization receipt, and the pure governability projection contains no
admission-quorum witness.

There is deliberately no function:

```text
ExternalDisposition<Admission<Action, UNKNOWN>, ActAnyway>
  -> Admission<Action, PROCEED>
```

An external authority can create
`ExternalDisposition<Admission<Action, UNKNOWN>, ActAnyway>`. That object is
attributable, scoped, expiring, appealable, and visible. It does not mutate or
supersede the epistemic assessment. A later `Assessment<Action>::PROCEED`
requires a new admission with new evidence or a newly frozen policy; it is
linked to, rather than overwriting, the earlier record.

This is the small language-design contribution: **make the absence of an authority-to-modality coercion a type-system property**. An operator override can inhabit an action type, never an evidence-promotion type.

## What signatures and transparency do—and do not—mean

The [in-toto Attestation Framework](https://github.com/in-toto/attestation/blob/v1.2.0/spec/README.md) separates predicate, subject-binding statement, authentication envelope, and bundle. That is the right evidence shape: authenticate who made a claim and bind it to exact artifacts without pretending the predicate became true by being signed.

[SCITT RFC 9943](https://www.rfc-editor.org/rfc/rfc9943.html) standardizes signed statements, transparency-service receipts, and auditable histories. It is unusually explicit that transparency does not prevent dishonest or compromised issuers; it creates accountability and leaves trust decisions to relying parties. This matches Vibe Halt's boundary exactly.

[Certificate Transparency RFC 9162](https://www.rfc-editor.org/rfc/rfc9162.html) supplies the append-only Merkle-log precedent, along with the warning: a log can show split views unless monitors compare signed tree heads and check consistency. A receipt without independent monitoring is therefore not a constitutional guarantee.

[The Update Framework specification](https://theupdateframework.github.io/specification/v1.0.28/) supplies useful key-governance precedents: role separation, signature thresholds, versioned metadata, rollback and freeze defenses, offline root keys, and explicit key rotation. These mechanisms limit compromise. They still do not decide whether a campaign's oracle or property contract was valid.

The invariant for every signed object is:

```text
valid_signature(statement, key)
  => attributable(statement, key)
  != valid_epistemic_claim(statement)
```

## Candidate key and policy topology

Use role keys and a versioned trust policy, borrowing the shape—not the semantic claims—of TUF:

| Role | Key posture | May sign | Must not sign |
|---|---|---|---|
| Constitutional root | Offline, threshold where operationally possible | role membership, policy-version transitions, emergency credential revocation | campaign observations or verdicts |
| Scope authority | Online, short-lived | frozen scope and campaign request | observation, admission, disposition |
| Runner | Ephemeral or workload-bound | observation/evidence envelope | scope, admission, external action |
| Admission judge/quorum member | Separate online key and execution boundary | `Assessment<Action> = HALT | PROCEED | UNKNOWN` over verified inputs | target patch, scope mutation, external action |
| Transparency service | Separate service key | inclusion/consistency receipt | truth or admission claim |
| External authority | Outside Vibe Halt's trust root | merge/deploy/settle disposition | modality promotion |

V1 should require at least one independent monitor and publish the exact authority grade of each campaign:

```text
AuthorityGrade =
  SingleOperatorSeparatedKeys
  | OrganizationallySeparated
  | ThresholdDiverse
  | ExternallyWitnessed
```

`SingleOperatorSeparatedKeys` is useful for mechanism validation but cannot support a claim of capture resistance. A product should not rename separated keys held by one person as independent governance.

## Appeal, revocation, and constitutional amendment

### Appeal

An appeal never edits an existing admission. It creates a new frozen scope or supplies new evidence, triggers a new independent run, and yields a new admission. The transparency history links both. A successful appeal means the new admission governs future reliance under the applicable policy; it does not make the earlier observation retroactively false.

### Revocation

Credential or policy revocation is append-only and prospective. It prevents future reliance after an effective time and forces re-evaluation of admissions whose trust path intersects the revoked material. It does not erase historical evidence. Emergency revocation can be fast; declaring a new epistemic verdict still requires a new admission.

### Constitutional amendment

Policy is an artifact with a content digest and semantic version. An amendment must:

1. identify old and new policy digests;
2. carry the old policy's required authorization and the new policy's acceptance threshold, analogous to TUF root continuity;
3. declare its effective time and whether prior admissions require re-evaluation;
4. enter the transparency history before use;
5. never reinterpret an old admission under new semantics without producing a new admission.

There is no silent policy drift and no emergency path that edits the verdict algebra.

## Capture-resistance ladder

The organism can strengthen without lying about its current grade:

1. **Mechanical separation:** treatment cannot write target-independent judge, oracle, evidence, policy, holdout, or trust-root surfaces.
2. **Key separation:** each seat signs only its own object type.
3. **Process separation:** the judge receives content-addressed evidence, not the repairer's narrative.
4. **Transparency plus monitoring:** all statements, dispositions, supersessions, and revocations are auditable; at least one monitor is outside the transparency service.
5. **Threshold diversity:** admission or constitutional transitions require independent organizations or implementations.
6. **Federated witnessing:** more than one transparency service or observer can register the same signed statement; disagreement remains visible rather than collapsed into consensus truth.

Model consensus, token voting, identity attestations, proof-of-work, or signature count cannot lift modality. Diversity is a capture-control and fault-detection mechanism, not a truth constructor.

## What `UNKNOWN` can consequentially do

`UNKNOWN` is not sovereign because Vibe Halt does not execute the consequence. It is consequential because the external institution's policy can require a `PROCEED` admission before normal execution:

```text
NormalAuthorization<Action>
  requires Admission<Action, PROCEED>
  and ExternalDisposition<Authorize(Action)>

ExceptionalAuthorization<Action>
  requires Admission<Action, UNKNOWN>
  and ExternalDisposition<ActAnyway, NamedAuthority, Reason, Expiry>
```

The exceptional path must be visibly different, bounded, and unavailable for actions whose constitution forbids exceptions. Vibe Halt grades; the institution decides which grades are admissible for which actions. That is how `UNKNOWN` can stop routine settlement without making the grader the state.

## Falsifiers

The non-sovereign design is falsified if any of these is possible:

- treatment selects, edits, or directly writes the judge, oracle, evidence schema, holdout, trust root, or admission policy;
- one mutable record permits an external override to replace `UNKNOWN` with `PROCEED`;
- admission and external action share one signing role or one API authority;
- a signature, log receipt, model consensus, vote, or issuer reputation is treated as epistemic validation;
- the transparency service can equivocate without an independent monitor being able to produce evidence of inconsistency;
- an appeal overwrites the challenged admission instead of creating a linked new one;
- revocation or constitutional amendment occurs without a signed, versioned, append-only transition;
- a campaign operated entirely by one controller is marketed as institutionally independent;
- the system can mint, merge, deploy, transfer, or tally by itself;
- an exceptional external action silently becomes precedent for later normal admission.

## Decision for the map

Adopt the five-seat topology and the typed separation between `Admission` and `ExternalDisposition`. Treat signatures as attribution, transparency as challengeability, diversity as capture resistance, and none of them as truth. Keep human or institutional action outside Vibe Halt's authority root. `UNKNOWN` may be acted against only through a separately attributable external disposition; it can never be rewritten or promoted by authority.

The remaining human decision is key custody and threshold policy: who holds the constitutional root, which seats must be organizationally independent for the first admitted campaign, and which actions—if any—may use the exceptional path.
