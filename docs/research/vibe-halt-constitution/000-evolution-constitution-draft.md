# Vibe Halt Evolution Constitution

> **STATUS: UNRATIFIED DRAFT — NO AUTHORITY, NO CAPABILITY CLAIM**
>
> This document proposes a constitutional evolution of Vibe Halt. It does not
> amend Product Lock v1, authorize implementation, certify current capability,
> permit foreign-target execution, or authorize any merge, deployment, spend,
> transfer, mint, tally, or other consequence. It becomes effective only after
> the open human ratification gates are resolved, the final text is explicitly
> ratified, and that ratification is merged.

**Drafting base:** accepted Vibe Halt `main` at
[`d19ba9e1198c0ab7ef4bb1bdf69a299d056f9754`](https://github.com/AmitabhainArunachala/vibe-halt/tree/d19ba9e1198c0ab7ef4bb1bdf69a299d056f9754)

**Wayfinder map:**
[#100 — Vibe Halt: From Vibe Mess to Governable Consequence](https://github.com/AmitabhainArunachala/vibe-halt/issues/100)

**Source status:** synthesis of existing law, operator-locked map premises, and
unratified research recommendations from tickets #101–#108. Incorporation is
not ratification; all authority-bearing choices remain in the human gates.

## Status language and source hierarchy

Every clause in this draft has one of four statuses:

| Mark | Meaning |
|---|---|
| **`[LAW]`** | Already ratified and merged in [Product Lock v1, status and authority lines 1–9](https://github.com/AmitabhainArunachala/vibe-halt/blob/d19ba9e1198c0ab7ef4bb1bdf69a299d056f9754/docs/specs/PRODUCT_LOCK_V1.md#L1-L9) or directly enforced by accepted code at the drafting base. This draft may restate it but cannot silently change it. |
| **`[LOCKED MAP PREMISE]`** | An operator-approved premise recorded in [Wayfinder map #100](https://github.com/AmitabhainArunachala/vibe-halt/issues/100). It directs this draft, but is not itself a merged constitutional artifact. |
| **`[RESEARCH RECOMMENDATION]`** | An evidence-backed proposal from a child research ticket. It remains challengeable and unratified. |
| **`[UNRESOLVED CHOICE]`** | A decision reserved to a live human grilling ticket. No implementation or agent may answer it by construction, sequencing, or default. |

If sources conflict, accepted code and merged law describe current capability;
the later human-ratified constitution governs future direction. A research
recommendation never overrides law merely because it is newer or more detailed.

## Preamble — consequence must not inherit fluency

Vibe coding made authorship abundant before it made evidence trustworthy. A
system can now produce convincing code, tests, fixes, reviews, receipts, and
explanations from one generative source. Fluency can therefore counterfeit an
entire chain of assurance while leaving the consequence untouched.

Vibe Halt exists to break that inheritance.

The evolved product would accept software as it is: dynamic, dependent,
agent-authored, partly unknown, and often too large to prove whole. It would map
the consequence-bearing paths, shake a supported real target under hostile
worlds, preserve the smallest replay of what breaks, propose treatment without
trusting the treating hand, cut out the residue small enough to reason about,
and grade the exact claim that survived. What it could not establish would
remain `UNKNOWN` in public. This paragraph is destination, not current
capability.

The destination is not a machine that declares reality true. It is a machine
that makes it harder for an unearned claim to bind reality.

Vibe Halt is therefore one organism with three constitutional functions:

1. an Antithesis-adjacent **shaker** for supported real targets;
2. an untrusted **refinery** that can repair and declutter vibe-coded mess; and
3. an independent **admission court** that types when a bounded machine claim
   may be presented to an external institution.

The machine is nested inside the institution. It grades settlement; it is not
the sovereign that settles.

---

## Article I — Purpose, jurisdiction, and the honest promise

### 1.1 Purpose

**`[LAW]`** Vibe Halt is an AI-native adversarial verification environment for
builders who cannot safely trust AI-generated or AI-modified software.
([Product Lock v1, lines 11–24](https://github.com/AmitabhainArunachala/vibe-halt/blob/d19ba9e1198c0ab7ef4bb1bdf69a299d056f9754/docs/specs/PRODUCT_LOCK_V1.md#L11-L24))

**`[LOCKED MAP PREMISE]`** Its evolved form is the **assurance refinery and
settlement typechecker for vibe-coded systems**: it maps, shakes, treats,
declutters, distills, and grades when a machine-generated claim may bind under a
separately constituted external policy.
([Wayfinder map #100](https://github.com/AmitabhainArunachala/vibe-halt/issues/100))

### 1.2 Broad intake, bounded standing

**`[LAW]`** A whole repository, application, workflow, agentic system, or other
mess may enter the front door. That breadth creates no corresponding proof
claim. Every externally meaningful statement remains bound to the exact
subject, revision, scope, properties, execution envelope, controller set,
fault model, campaign, budget, and evidence grade actually observed.

Unsupported, incomplete, stale, untrusted, divergent, non-replayable, or
otherwise unresolved mandatory coverage is `UNKNOWN`, never evidence of
safety.
([Product Lock v1, lines 17–24](https://github.com/AmitabhainArunachala/vibe-halt/blob/d19ba9e1198c0ab7ef4bb1bdf69a299d056f9754/docs/specs/PRODUCT_LOCK_V1.md#L17-L24),
[decision boundary, lines 68–85](https://github.com/AmitabhainArunachala/vibe-halt/blob/d19ba9e1198c0ab7ef4bb1bdf69a299d056f9754/docs/specs/PRODUCT_LOCK_V1.md#L68-L85))

### 1.3 The honest product promise

**`[LOCKED MAP PREMISE + RESEARCH RECOMMENDATION]`** No supported target should
leave as an undifferentiated mess. The output is not one overloaded status. It
is a product of orthogonal types:

```text
AttemptValidity = Valid | Invalid(IntegrityViolation)

Assessment<Action> =
    HALT(BlockingFinding)
  | PROCEED(BoundedObligationsSatisfied)
  | UNKNOWN(OpenObligations)

Governability =
    GOVERNABLE(BoundaryWitnesses)
  | UNGOVERNED(UnboundedBlindSpots)

GovernabilityGateDecision<Action, Scope, Policy> =
    RequiredAndGovernable(GovernabilityProjectionPayloadId)
  | RequiredButUngoverned(GovernabilityProjectionPayloadId, UnboundedBlindSpots)
  | NotRequired(GovernabilityProjectionPayloadId, RatifiedPolicyClauseId)

VerifyGovernabilityGate(AdmissionRecordId, Policy) ->
  Result<GovernabilityGateWitness<Action, Scope, Policy>, GateUnsatisfied>

ReviewCandidacyPermit
  = constructible only from Valid
    + Assessment<HumanReview>::PROCEED
    + GovernabilityGateWitness<HumanReview, Scope, Policy>
```

A valid campaign therefore returns an action-typed assessment and a separate
governability projection. `HALT` carries a blocking finding and the smallest
independently usable replay the evidence supports. `UNKNOWN` carries the exact
open surfaces, missing obligations, and evidence ceiling. `GOVERNABLE` says the
consequence accounting is bounded; it is not a verdict and is never an
alternative spelling of `PROCEED`.

The pure gate decision inside the signed admission payload forces the policy
choice to remain visible without settling it here. `RequiredAndGovernable`
cannot be built from `UNGOVERNED`;
`RequiredButUngoverned` preserves the fail-closed state in signed `HALT` or
`UNKNOWN` admissions but cannot derive a gate witness or permit;
`NotRequired` must cite a ratified action-specific policy clause and still bind
the disclosed governability projection. Only after the outer admission quorum
signs that payload can verification derive `GovernabilityGateWitness`; the
projection itself contains no admission-quorum witness and creates no signature
cycle. Issues #112 and #114 decide which decision constructor, if either, is
lawful for human review.

This is a terminal classification contract, not a promise that arbitrary
software can be fixed, proved, or made safe. A wrong digest, signature,
subject, schema, candidate, or other integrity violation makes the **attempt**
`Invalid`; it does not become an ordinary `HALT` or a successful `UNKNOWN` and
must not enter a positive denominator. The exact public atomic object and the
compatibility of bounded unknowns with `GOVERNABLE` remain unresolved in
[#114 — Decide the atomic admission object](https://github.com/AmitabhainArunachala/vibe-halt/issues/114)
and [#112 — Ratify what GOVERNABLE requires](https://github.com/AmitabhainArunachala/vibe-halt/issues/112).
([Wayfinder map #100](https://github.com/AmitabhainArunachala/vibe-halt/issues/100),
[campaign terminal algebra, lines 437–486](101-dharma-promotion-campaign.md#L437-L486),
[governability research boundary, lines 28–46](108-governable-refinery.md#L28-L46))

### 1.4 Jurisdiction

**`[LAW]`** Vibe Halt may issue a bounded receipt, but the builder—not Vibe
Halt—uses it in a merge or release decision. Vibe Halt does not author or modify
production code under Product Lock v1.
([Product Lock v1, lines 31–49](https://github.com/AmitabhainArunachala/vibe-halt/blob/d19ba9e1198c0ab7ef4bb1bdf69a299d056f9754/docs/specs/PRODUCT_LOCK_V1.md#L31-L49))

**`[LOCKED MAP PREMISE]`** The evolved court may grade evidence and issue only
the bounded admission objects ratified for it. By virtue of that role it may
not merge, deploy, transfer, mint, tally, spend, waive, or execute any
consequence; those acts remain outside its authority root.
([Product Lock v1 non-goals, lines 137–145](https://github.com/AmitabhainArunachala/vibe-halt/blob/d19ba9e1198c0ab7ef4bb1bdf69a299d056f9754/docs/specs/PRODUCT_LOCK_V1.md#L137-L145),
[Wayfinder map #100](https://github.com/AmitabhainArunachala/vibe-halt/issues/100))

---

## Article II — One organism, separated powers

### 2.1 Canonical loop

**`[LOCKED MAP PREMISE]`** The constitutional product loop is:

```text
MAP
  → SHAKE
  → SHRINK / EXPLAIN
  → TREAT / DECLUTTER / DISTILL-CODE
  → re-enter as untrusted input at MAP
  → INDEPENDENT RE-SHAKE / HIDDEN HOLDOUT
  → optional PROVE / MODEL ALREADY-FROZEN RESIDUE
  → ADMIT
  → external human or institutional disposition
```

No arrow is an evidentiary shortcut. Later stages consume typed artifacts from
earlier stages and retain their scope. Repetition creates a new attempt, not a
retroactive improvement of an old result.
Any guard, kernel, model implementation, or consequence-routing change created
during distillation is treatment output with a new `SubjectId` and returns to
`FREEZE/MAP`; only analysis of a residue already present in the re-shaken
subject may proceed toward admission, and only with a separately admitted
correspondence/bypass record.
([Wayfinder map #100](https://github.com/AmitabhainArunachala/vibe-halt/issues/100),
[refinery loop, lines 13–43](108-governable-refinery.md#L13-L43))

### 2.2 The three functions

| Function | Constitutional job | May produce | Must never infer |
|---|---|---|---|
| **Shaker** | Govern a declared real-target execution, inject hostile choices, observe consequential paths, replay and minimize failures | observations, findings, channel ledger, coverage ledger, replay evidence | that finite testing is proof; that an open channel is controlled |
| **Refinery** | Diagnose, patch, remove accidental complexity, extract contracts, narrow trusted residue, and resubmit candidates | untrusted candidate revisions, repair claims, declutter equivalence claims | that authorship creates admission; that a green known replay closes a bug class |
| **Admission court** | Verify exact evidence, apply a frozen policy, and emit one bounded `Assessment<Action> = HALT | PROCEED | UNKNOWN` | signed admission and, where allowed, a review-candidacy permit | merge/deploy authority; truth from signature; permission from `UNKNOWN` |

The functions share an evidence graph but not authority. The refinery is a
constitutive function of the product and an epistemically untrusted guest of
the court.
([Wayfinder map #100](https://github.com/AmitabhainArunachala/vibe-halt/issues/100),
[hostile repair re-entry, lines 317–362](108-governable-refinery.md#L317-L362))

### 2.3 No self-granting cycle

**`[LAW]`** Product Lock requires any future patch-generation capability to
remain separate from verification authority and forbids it from certifying its
own patch.

**`[LOCKED MAP PREMISE]`** This constitution strengthens that boundary: the
actor that authors or selects a candidate has **zero admission, holdout,
scope-ratification, trust-root, or appeal-quorum weight for that candidate**. It
is not enough to make treatment one vote in a larger quorum. A treatment
narrative, local test pass, model consensus, token vote, signature, operator
assertion, or the fact that “Vibe Halt wrote it” creates no evidentiary
preference.

Every repair output is fresh vibe-coded input. It begins again at `MAP` with a
new subject identity, new claims, new consequential paths, and new capability
obligations.
([Product Lock v1, lines 46–49](https://github.com/AmitabhainArunachala/vibe-halt/blob/d19ba9e1198c0ab7ef4bb1bdf69a299d056f9754/docs/specs/PRODUCT_LOCK_V1.md#L46-L49),
[Wayfinder map #100](https://github.com/AmitabhainArunachala/vibe-halt/issues/100))

---

## Article III — Epistemic and authority semantics

### 3.1 Orthogonal axes

**`[LAW]`** Accepted code already treats epistemic modality and authority as
orthogonal:

```text
Modality = Proposed | Documented | Implemented | Observed | Replayed | Proven
Authority = Unratified | HumanMerged | OperatorAuthorized | ExternalConfirmed
```

Authority may change who ratifies or may act on a claim. Authority cannot lift
what is known. Modality may advance only by an exact adjacent witness bound to
the same revision. The accepted promotion evaluator exposes no transition to
`Proven`; because `Claim` is still publicly constructible runtime data, this is
an evaluator boundary, not yet a sealed proof-carrying type.
([accepted modality types, lines 1–42](https://github.com/AmitabhainArunachala/vibe-halt/blob/d19ba9e1198c0ab7ef4bb1bdf69a299d056f9754/crates/vh-cli/src/modality.rs#L1-L42),
[public runtime `Claim`, lines 50–56](https://github.com/AmitabhainArunachala/vibe-halt/blob/d19ba9e1198c0ab7ef4bb1bdf69a299d056f9754/crates/vh-cli/src/modality.rs#L50-L56),
[promotion evaluator, lines 92–138](https://github.com/AmitabhainArunachala/vibe-halt/blob/d19ba9e1198c0ab7ef4bb1bdf69a299d056f9754/crates/vh-cli/src/modality.rs#L92-L138))

### 3.2 Claim identity

**`[RESEARCH RECOMMENDATION]`** Every consequential claim should be represented
as a typed object whose identity includes at least:

```text
Claim<
  Subject,
  Scope,
  ExecutionEnvelope,
  ControllerSet,
  Campaign,
  Modality,
  Authority
>
```

Omitted identity is not a wildcard. It is `Unresolved(reason)` and constrains
the strongest lawful projection.
([typed evidence identity, lines 95–190](104-execution-envelopes.md#L95-L190))

### 3.3 Evidence state and target verdict are distinct

**`[LAW]`** Engine states such as `CLEAN`, `FINDINGS`, `UNCHECKED`, and `ERROR`
are evidence-layer states. Product Lock defines `HALT | PROCEED | UNKNOWN` as a
separate, versioned, fail-closed policy projection; `UNKNOWN` outranks
`PROCEED`, and findings remain independently publishable.
([Product Lock v1 decision contract, lines 68–85](https://github.com/AmitabhainArunachala/vibe-halt/blob/d19ba9e1198c0ab7ef4bb1bdf69a299d056f9754/docs/specs/PRODUCT_LOCK_V1.md#L68-L85))

**`[RESEARCH RECOMMENDATION]`** This draft adds an orthogonal integrity gate so
malformed evidence cannot masquerade as a target assessment:

```text
AttemptValidity = Valid | Invalid(IntegrityViolation)

Assessment<Action> for a Valid attempt:
HALT
  = an admitted blocking finding reproduced
  | a mandatory precondition failed on the exact valid subject

PROCEED
  = every mandatory declared obligation completed within the exact boundary,
    evidence verified, and no blocking finding remains

UNKNOWN
  = any mandatory obligation is unsupported, incomplete, stale, untrusted,
    divergent, non-replayable, errored, open, or otherwise unresolved
```

([campaign terminal algebra, lines 437–486](101-dharma-promotion-campaign.md#L437-L486))

### 3.4 No coercion from authority to modality

**`[LAW]`** The accepted authority evaluator preserves modality when authority
changes.

**`[LOCKED MAP PREMISE]`** The evolved constitution generalizes that invariant:
no type, API, policy, emergency path, vote, signature threshold, or human
override may implement:

```text
Authority<X> -> Modality<HigherThanX>
ExternalDisposition<Admission<Action, UNKNOWN>, ActAnyway>
  -> Admission<Action, PROCEED>
```

An external actor may make a separately typed and attributable act despite an
unchanged `UNKNOWN` only if the external institution's own constitution permits
that class of exception. Whether any such class is permitted is unresolved in
[#116 — Ratify institutional authority, appeal, and amendment](https://github.com/AmitabhainArunachala/vibe-halt/issues/116).
([accepted authority-preservation evaluator, lines 132–138](https://github.com/AmitabhainArunachala/vibe-halt/blob/d19ba9e1198c0ab7ef4bb1bdf69a299d056f9754/crates/vh-cli/src/modality.rs#L132-L138),
[Wayfinder map #100](https://github.com/AmitabhainArunachala/vibe-halt/issues/100))

---

## Article IV — Evidence before verdict

### 4.1 Irreducible evidence boundary

**`[LOCKED MAP PREMISE]`** Every emitted verdict must be content-addressed,
attributable, and independently replayable to the limit claimed. “Signed” means
the verifier can attribute exact statement bytes to an authorized role under a
versioned policy. It never means the statement is true.

**Current capability notice:** accepted main content-addresses strict bundles
with SHA-256 but explicitly does not yet provide signed or authenticated
provenance. Trace-v0's FNV identity remains a legacy replay identity, not a
cross-party security primitive. This article specifies direction; it does not
upgrade current standing.
([accepted receipt boundary, lines 18–31](https://github.com/AmitabhainArunachala/vibe-halt/blob/d19ba9e1198c0ab7ef4bb1bdf69a299d056f9754/crates/vh-cli/src/receipts_v2.rs#L18-L31),
[Trace-v0 limits, lines 309–315](https://github.com/AmitabhainArunachala/vibe-halt/blob/d19ba9e1198c0ab7ef4bb1bdf69a299d056f9754/docs/specs/TRACE_FORMAT_V0.md#L309-L315))

### 4.2 Exact evidence identity

**`[RESEARCH RECOMMENDATION]`** Observation and admission form an acyclic
evidence graph. They must not share one self-referential identity:

```text
CampaignSpecId = H(FrozenManifestWithoutHoldoutCommitments)
HoldoutCommitment<Curator> = Commit(CampaignSpecId, CuratorMaterial)
CampaignId = H(CampaignSpecId, OrderedHoldoutCommitmentStatementIds)

ObservationClosureId<Role> = (
  SubjectId,
  EnvelopeId,
  ControllerSetId,
  CampaignId,
  RoleObservationSetId,
)

ObservationAttestation<Role> = Sign<Role>(ObservationClosureId<Role>)

EvidenceClosureId = (
  OrderedObservationClosureIds,
  OrderedObservationAttestations,
  RequiredStatementAndBlobManifest,
)

AdmissionPayloadId = (
  EvidenceClosureId,
  PolicyId,
  Action,
  Assessment<Action>,
  GovernabilityProjectionPayloadId,
  GovernabilityGateDecision<Action, Scope, Policy>,
  AdmissionQuorumPolicyId,
)

JudgeAttestation<Judge> = Sign<Judge>(AdmissionPayloadId)
AdmissionRecordId = (AdmissionPayloadId, OrderedJudgeAttestations)
AdmissionQuorumWitness<Policy> = VerifyQuorum(AdmissionRecordId, Policy)
```

`SubjectId` binds source, produced artifact, and the signed immutable
materialization receipt, including the interpreter, loader, dependency, and
build closure where applicable. A workspace pathname is not authority.
`EnvelopeId` binds the
exact execution mechanism and host profile. `ControllerSetId` carries every
capability disposition. `CampaignSpecId` commits to the property/oracle
contract, fault palette, seed-domain policy, input cassette,
effect-tape schema/controller policy, budgets, thresholds,
and required evidence schemas—never a future commitment or produced evidence.
Holdout commitments bind that spec; only their ordered statement identities then
construct final `CampaignId`. Each runner or replayer signs only its own
observation closure and never signs the verdict. `EvidenceClosureId` later binds
those role-scoped closures, attestations, and required blobs without asking an
earlier signer to attest to future evidence. Each authorized judge signs the
same action-typed `AdmissionPayloadId`; only afterward does
`AdmissionRecordId` bind the payload and attestations, from which the ratified
quorum witness is verified. No identifier contains a signature over itself.

Any missing fact is explicitly `NotApplicable` or `Unresolved(reason)`, never
silently absent.

### 4.3 Closure, not a bag of receipts

**`[RESEARCH RECOMMENDATION]`** Admission must consume a content-addressed
closure manifest that enumerates every required statement, evidence blob,
capability ledger, replay, policy, and verification artifact by content identity
and byte length; each role-owned constituent carries its own attestation.
Deleting an inconvenient receipt cannot make a claim more admissible. A
signature authenticates a statement; closure establishes which statements were
mandatory; replay tests whether an observation can be reproduced. None
establishes oracle adequacy by itself.

[#103 — Evidence sovereignty without a root oracle](103-evidence-sovereignty.md#L13-L45)
recommends DSSE-authenticated in-toto Statements over exact
SHA-256 subjects, a threshold TUF role policy whose root cannot sign verdicts,
independent judges, witnessed transparency, redundant content-addressed
storage, and a two-curator commit/reveal holdout. It explicitly makes missing,
stale, contradictory, unavailable, or non-quorate input project to `UNKNOWN`.
The detailed signer thresholds, key custody, independence grade, availability
cost, rotation, revocation, and hidden-selector topology remain unratified and
reserved to human decision #110.

### 4.4 Hidden validation

**`[LOCKED MAP PREMISE]`** A repairer optimizes against what it can see.
Admission therefore requires both:

1. the original minimized replay, which must become non-falsifying for the
   intended reason; and
2. fresh seeds, widened fault palettes, and a held-out mechanism class that the
   treatment process could not read before candidate freeze.

The candidate, evaluation contract, cohort eligibility law, budgets, and
commitment must freeze before reveal. Leaked or prematurely revealed holdouts
become calibration-only. Misses, unsupported cases, invalid attempts, and
aborts remain in the denominator.
([accepted holdout law, lines 63–93](https://github.com/AmitabhainArunachala/vibe-halt/blob/d19ba9e1198c0ab7ef4bb1bdf69a299d056f9754/docs/plans/VIBE_HALT_BUILD_PLAN_2026-07-20.md#L63-L93),
[Wayfinder map #100](https://github.com/AmitabhainArunachala/vibe-halt/issues/100))

Cryptography can make a commitment challengeable. It cannot prove that a
curator never leaked a secret or that an oracle represents human intent. Those
remain named institutional and capability channels.

### 4.5 Transparency is challengeability

**`[RESEARCH RECOMMENDATION]`** Evidence, admissions, external dispositions,
appeals, supersessions, revocations, censures, and policy transitions should be
append-only and consistency-checkable. Inclusion proves publication; witnessed
consistency helps expose equivocation. Neither proves truth, correctness,
availability, or universal observation.

An artifact advertised as publicly replayable must make the exact claimed
evidence retrievable to the claimed public. A public digest over private bytes
is `PUBLIC_COMMITMENT / PRIVATE_AUDIT`, not public replay.

---

## Article V — Execution envelopes and non-transferable evidence

### 5.1 Three envelopes, three trust shapes

**`[LOCKED MAP PREMISE + RESEARCH RECOMMENDATION]`** Vibe Halt retains three
execution envelopes. They are not interchangeable maturity levels:

| Envelope | Constitutional role | Honest ceiling |
|---|---|---|
| `CooperativeD2` | Bootstrap, map, diagnose, and obtain first contact with the real native workflow | All relevant uncontrolled channels remain visible; no admission-grade `PROCEED` when a mandatory claim depends on them |
| `NativeInterposed` | Primary reach lane for the existing unmodified native artifact, especially messy CPython/Linux software | Only the exact artifact, host/kernel/CPU profile, controller set, topology, and campaign; determinism and containment are separately graded |
| `CapabilityClosed` | Primary closure lane for deliberately recompiled, capability-shaped, or distilled residues such as WASI components | A new artifact identity; capability closure does not imply determinism or native behavioral equivalence |

The intended sequence is `CooperativeD2` to learn the real Dharma path,
`NativeInterposed` to govern claims about that same native path, and
`CapabilityClosed` where a deliberately extracted residue benefits from
absence-by-construction rather than watched ambient authority.
([execution-envelope comparison, lines 11–35](104-execution-envelopes.md#L11-L35))

### 5.2 No evidence laundering

**`[RESEARCH RECOMMENDATION]`** Evidence is indexed forever by:

```text
(artifact, execution envelope, controller set, campaign, claim)
```

No lawful cast exists from cooperative evidence to interposed evidence, native
evidence to WASM evidence, capability closure to determinism, or determinism to
a security boundary.

The only lawful cross-envelope object is relational:

```text
Corroboration<
  Evidence<Artifact1, Envelope1, Controllers1>,
  Evidence<Artifact2, Envelope2, Controllers2>,
  DifferentialWitness
>
```

It retains both identities and can support only the named correspondence
claim. It cannot transfer either side's grade.

### 5.3 Mechanism choice by falsifier

**`[LOCKED MAP PREMISE]`** Reach is proven by one real target governed end to
end, not by allegiance to `ptrace`, seccomp, WASI, or any other mechanism. Each
reach investment must freeze its compatibility, channel-closure, replay,
containment, performance, and timebox criteria before implementation, and must
record the rejected path's specific revival falsifier.

The numerical investment and revival thresholds remain unresolved in
[#113](https://github.com/AmitabhainArunachala/vibe-halt/issues/113).

---

## Article VI — The untrusted refinery

### 6.1 Treatment is in scope and outside admission authority

**`[LOCKED MAP PREMISE]`** Autonomous diagnosis, repair, and decluttering are
constitutional product scope, but only inside disposable worktrees, branches,
or equivalent isolated subjects. Treatment may inspect findings, propose a
patch, reduce accidental complexity, extract a contract, and request another
campaign. It may not:

- write the accepted production tree;
- commit to an accepted or protected branch, merge, deploy, spend, or perform
  the consequence;
- emit `PROCEED` or create a review permit;
- waive, hide, delete, or reclassify an open channel;
- choose or alter the judge, property oracle, evidence schema/store, hidden
  cohort, campaign policy, Vibe kernel, fault palette, trust root, or
  transparency history; or
- make its own re-shake the only evidence used for admission.

These are mechanical write and capability boundaries, not prompt
instructions. If treatment can cross them, the attempt is invalid.
([Product Lock v1 separation law, lines 46–49](https://github.com/AmitabhainArunachala/vibe-halt/blob/d19ba9e1198c0ab7ef4bb1bdf69a299d056f9754/docs/specs/PRODUCT_LOCK_V1.md#L46-L49),
[Wayfinder map #100](https://github.com/AmitabhainArunachala/vibe-halt/issues/100))

### 6.2 Repair re-enters as hostile input

Every treatment output receives a new candidate identity and re-enters at
`MAP`. The full consequential-path inventory, property obligations, dependency
closure, channel ledger, original replay, new claims, and hidden validation
apply again. “Generated by Vibe Halt” is provenance only.

A repaired known replay is necessary and insufficient. If a fresh held-out case
in the same mechanism class still fails, the treatment overfit the example and
must not receive positive admission.

### 6.3 Declutter is a behavior claim

**`[LOCKED MAP PREMISE]`** Decluttering is removal of accidental complexity
under an explicit observational boundary. If any governed-channel behavior
changes, the change is repair and inherits every repair obligation.

Behavior preservation may be supported by trace equivalence, differential
campaigns, or a formal residue proof, but only within the named observation
boundary. Fewer lines, files, or dependencies are not evidence of equivalence,
and a smaller source tree that expands privileged effects is not a successful
declutter.

### 6.4 `GOVERNABLE`

**`[LOCKED MAP PREMISE; COMPATIBILITY RULE UNRESOLVED]`** `GOVERNABLE` is not a
synonym for clean, correct, safe, proved, or finished. The locked premise is
that it requires explicit consequential paths, claims, properties,
dependencies, channels, replays, revalidation, bounded uncertainty, and no
silent ungraded consequence.
[#108 — Governable-repository refinery](108-governable-refinery.md#L13-L46)
recommends construing this as a bounded repository or consequence state in
which:

1. every declared consequential path is identified;
2. claims, executable properties, dependencies, external effects, and
   capability channels are explicit for those paths;
3. blocking failures carry minimal independently usable replays where the
   evidence supports them;
4. admitted repairs survive independent fresh and hidden revalidation;
5. residual uncertainty is bounded and attached to its exact path and claim;
6. every behavior-changing “declutter” is treated as repair; and
7. no ungraded path can silently produce the declared consequence.

A bounded path-level `UNKNOWN` **might** coexist with `GOVERNABLE` if it carries
a structural `ConsequenceBoundWitness` and the external consequence policy
prevents it from being silently crossed. That is a research recommendation,
not a locked answer. Every blind spot defaults to `UnboundedOpen`; scope
authority cannot mint the bound merely by narrowing prose. Each obligation
retains its own modality rather than inheriting a global `Replayed` label from
the accounting graph.

Whether any bounded `UNKNOWN` is compatible with the public label, and whether
`GOVERNABLE` is atomic per repository, per consequence, per path set, or
another object, are reserved to
[#114](https://github.com/AmitabhainArunachala/vibe-halt/issues/114); its final
requirements are reserved to [#112](https://github.com/AmitabhainArunachala/vibe-halt/issues/112).

### 6.5 Five maps and an unbounded-open stop

**`[RESEARCH RECOMMENDATION]`**
[#108 — Governable-repository refinery](108-governable-refinery.md#L84-L145)
rejects one omniscient repository graph. The refinery should preserve five
linked, content-addressed maps:

1. artifact and provenance;
2. executable structure;
3. capability and consequence;
4. claim, contract, and oracle; and
5. evidence and decision.

A consequential path class relates an entry point, authority source,
state/data transformation, consequence sink, and execution envelope. Every
declared, discovered, and platform-mandatory class must be graded as governed,
halted, bounded-open-with-witness, or externally excluded under a named scope
authority.
Mandatory sink classes cannot disappear by tenant exclusion. A blind spot that
may reach an unbounded consequence prevents construction of `GOVERNABLE`; it is
not merely another `UNKNOWN` detail.

The precise platform consequence taxonomy, bounded-open rule, adapter support
profiles, mapping sufficiency, and public atomic object remain for
[#112 — Ratify what GOVERNABLE requires](https://github.com/AmitabhainArunachala/vibe-halt/issues/112)
and [#114 — Decide the atomic admission object](https://github.com/AmitabhainArunachala/vibe-halt/issues/114).
The research architecture informs those decisions but does not close them.

---

## Article VII — Antithesis-grade is a measured vector

### 7.1 The name is earned on a declared target profile

**`[LAW]`** Product Lock makes an Antithesis-adjacent environment a conditional
north star and defines independently confirmed severity-weighted yield under
equal budgets as the product benchmark
([Product Lock v1 north star, lines 26–29](https://github.com/AmitabhainArunachala/vibe-halt/blob/d19ba9e1198c0ab7ef4bb1bdf69a299d056f9754/docs/specs/PRODUCT_LOCK_V1.md#L26-L29),
[product benchmark, lines 87–108](https://github.com/AmitabhainArunachala/vibe-halt/blob/d19ba9e1198c0ab7ef4bb1bdf69a299d056f9754/docs/specs/PRODUCT_LOCK_V1.md#L87-L108)).

**`[RESEARCH RECOMMENDATION]`** “Antithesis-grade” should therefore be an
external benchmark vector, never a badge, analogy, or claim imported from
another system's architecture. For a named target profile, Vibe Halt must
demonstrate:

1. the real declared target and dependencies execute;
2. every claim-relevant nondeterministic source is controlled or typed `Open`;
3. workloads, schedules, and environmental faults reach meaningful choice
   points;
4. measured feedback explores more discriminating states than frozen null
   strategies;
5. failures replay and minimize independently from an exact world identity;
6. failure, recovery, safety, and liveness are exercised across the declared
   topology;
7. throughput and evidence cost are operationally usable; and
8. independently confirmed severity-weighted yield beats preregistered,
   equal-budget baselines.
([research capability vector, lines 61–117](107-antithesis-grade-path.md#L61-L117))

Finite testing remains testing. Search coverage is a navigation signal, not
proof, property adequacy, or permission.

### 7.2 Capability ladder

**`[RESEARCH RECOMMENDATION]`** Progress should be reported as an explicit
capability rung, each with a fresh evidence boundary:

| Rung | Capability | Ceiling |
|---|---|---|
| `A0 TruthKernel` | seeded in-process universes, typed evidence, replay, registered properties, bounded shrink | no real-target claim |
| `A1 RealTargetObservatory` | one exact real Dharma path maps, runs, records open channels, and emits signed replay evidence | `PROCEED` forbidden where mandatory claims depend on open channels |
| `A2 NativeDeterministicProfile` | exact native profile denies, virtualizes, or replays every reachable declared effect and fails closed on unsupported effects | exact artifact/host/controller profile only; security boundary separately graded |
| `A3 WholeTopologyDST` | declared processes, dependencies, drivers, faults, recovery, safety, and liveness share one controlled world | undeclared services and performance remain `UNKNOWN` |
| `A4 FindingDepth` | each admitted finding has independent replay, reduction, and causal limits | minimality is not proof of root cause |
| `A5 MeasuredSearch` | guidance beats frozen uniform/null strategies on a discriminating holdout | reward signals cannot lift a verdict |
| `A6 RealFaultAdvantage` | supported real targets beat decorrelated AI review on confirmed severity-weighted yield | no generalization outside tested classes |
| `A7 AntithesisAdjacentOperation` | A2–A6 repeat across multiple real distributed targets with usable economics | measured adjacency, never equivalence to proprietary internals or formal proof |

The order is **reach → exact control → replay/minimization → discriminating
search → economic yield**. Search before reach optimizes fixtures. A hypervisor
before a real target earns only a hypervisor, not a product proof.

### 7.3 Measurement without a single green score

Campaigns publish the raw vector: exact target fidelity; controlled and open
channels; topology and property-opportunity coverage; consequential states per
cost; search advantage; replay and shrink success; p50/p95 time to first
confirmed fault; slowdown; operator time; evidence volume; confirmed yield;
invalid-claim rate; baseline-unique misses; and real decisions changed.

No composite score may conceal a zero-tolerance evidence violation or trade
away an unknown channel. Definitions, budgets, baselines, denominators,
severity weights, stopping rules, and holdouts freeze before the credit run.
([research measurement and anti-Goodhart rules, lines 76–130](107-antithesis-grade-path.md#L76-L130))

---

## Article VIII — First constitutional campaign: Dharma promotion

### 8.1 Subject and claim

**`[LOCKED MAP PREMISE + RESEARCH RECOMMENDATION]`** The first campaign should
bind one human-frozen canonical revision of Dharma Swarm to the smallest
production-shaped CPython mutation-to-review seam:

```text
entrypoint: DarwinEngine.apply_sealed_packet(...)
operation: CandidateForHumanReview
initial envelope: CooperativeD2
real checkout writes: forbidden
commit / PR / merge / deploy: forbidden
initial artifact: signed CampaignAssessment<HumanReview>
later positive artifact: exact PromotionPermit<CandidateForHumanReview>
```
([first-campaign research resolution, lines 21–56](101-dharma-promotion-campaign.md#L21-L56))

The target snapshot used during research is archaeological evidence only. The
campaign must pin a fresh canonical Dharma SHA, clean materialization, exact
CPython interpreter and dependency closure, exact nonempty patch, proof
command, policy, oracle, palettes, budgets, and channel ledger before execution.
It must also freeze a distinct containment profile: zero target credentials,
default-deny network and data policy, host/VM isolation, write boundaries, and
the human authorization for running the exact target. Determinism and
containment receive separate grades; a disposable worktree is not a security
boundary.

### 8.2 Why this seam

Current Dharma has meaningful fail-closed gates and signed path-scoped
promotion packets, but the observed packet does not bind the full base
revision, canonical diff, candidate tree, workspace, campaign, and independent
admission quorum into one consequence object. The first campaign asks the smallest
load-bearing question:
([Dharma promotion gate, lines 1–122](https://github.com/AIKAGRYA/dharma_swarm/blob/abc8bd35c34c729ed421f9615df504f5615868bd/dharma_swarm/promotion_gate.py#L1-L122),
[`apply_sealed_packet`, lines 28–179](https://github.com/AIKAGRYA/dharma_swarm/blob/abc8bd35c34c729ed421f9615df504f5615868bd/dharma_swarm/sealed_packet_apply.py#L28-L179))

> Can Vibe Halt touch a real AI-authored promotion path, distinguish the exact
> authorized candidate from a same-path substitute, and stop before it becomes
> its own judge?

### 8.3 Required attack pair and boundaries

The campaign has two independent proof obligations. Neither can substitute for
the other:

1. **Shaker obligation:** a source-derived Dharma behavioral mutant on the
   mutation-to-review path, with a frozen property and fault palette, must be
   found through the real CPython target, independently replayed, and shrunk
   without changing finding identity. The exact mutant/property/palette is a
   required choice in
   [#115 — Ratify the first Dharma campaign contract](https://github.com/AmitabhainArunachala/vibe-halt/issues/115),
   not supplied by this draft.
2. **Admission-integrity obligation:** the same-path/different-bytes attack and
   judge-separation negatives must fail before consequence.
([campaign dual obligations, lines 352–382](101-dharma-promotion-campaign.md#L352-L382))

For the admission-integrity obligation, the preregistered paired arms are:

- **Control A:** a human-supplied nonempty repair candidate bound by shared
  `FaultClassId`, `FindingId`, and `RepairClaimId` to the exact baseline mutant,
  minimal replay, hidden mechanism class, frozen base, diff, path set, candidate
  tree, campaign, and review operation. The original replay and hidden class
  must be revalidated against A; autonomous repair is not required;
- **Treatment B:** different bytes on the same authorized paths presented with
  A's otherwise valid legacy path-scoped packet; required boundary outcome:
  `Rejected(IntegrityViolation::BindingMismatch)` before consequence and no
  target assessment.

Mandatory negative cases include wrong or dirty base, stale campaign,
workspace swap, trust-key swap, oracle/judge edit, empty diff, holdout leak,
post-freeze race, digest/signature corruption, and independent replay
divergence. One wrong-subject permit, treatment-selected trust root, or real
checkout write falsifies the admission design.

### 8.4 Terminal meaning

A positive permit, if a later envelope earns one, authorizes **human review
candidacy only**. It does not state that the candidate is correct, safe,
merged, deployable, or proved. `HALT` and `UNKNOWN` are valid successful
classifications when supported by honest evidence.

The initial `CooperativeD2` phase earns A1 real-target evidence and must not
issue a review permit at accepted main: all 29 channels remain `Open`, and the
campaign has not mechanically established their non-relevance to review
eligibility. It may end honestly in `HALT` or `UNKNOWN`. A later review permit
requires fresh evidence under an envelope that closes every channel relevant
to the action—or an explicitly narrower action whose independence from each
open channel is mechanically demonstrated and human-ratified. No D1 standing
may be inferred from D2.
([accepted capability envelope, lines 14–40](https://github.com/AmitabhainArunachala/vibe-halt/blob/d19ba9e1198c0ab7ef4bb1bdf69a299d056f9754/docs/specs/SANDBOX_CAPABILITY_ENVELOPE_V1.md#L14-L40),
[campaign terminal boundary, lines 437–486](101-dharma-promotion-campaign.md#L437-L486))

The final campaign contract remains reserved to
[#115](https://github.com/AmitabhainArunachala/vibe-halt/issues/115).

### 8.5 Second campaign

**`[LOCKED MAP PREMISE]`** The second named proving ground is Sarathi cycle
admission with the narrow theorem:

```text
ObservedHalt -> NoNewWorkLease
```

It must not be advertised as a global halt guarantee. No design work for that
campaign is authorized by this draft.
([Wayfinder map #100](https://github.com/AmitabhainArunachala/vibe-halt/issues/100))

---

## Article IX — The first provable residue

### 9.1 The cut

**`[LOCKED MAP PREMISE + RESEARCH RECOMMENDATION]`** The first residue is not
Dharma, CPython, the repairer, or the whole Vibe Halt engine. It is one pure,
total, fail-closed decision function whose only successful output means that
one exact candidate may enter human review:

```text
PromotionPermit<
  BaseRevision,
  CandidateRevision,
  DiffDigest,
  PathSetDigest,
  MaterializationReceiptId,
  CampaignId,
  GovernabilityGateWitness<HumanReview, Scope, Policy>,
  AdmissionQuorumWitness<Policy>
>
```

The indices are typed fixed identities, not narrative strings.
`CampaignSpecId` commits to the pre-candidate campaign policy, execution
envelope, controller-set identities, oracle, budget, and required evidence
contract/schema. `CampaignId` then adds the ordered holdout-commitment
statements—still before candidate freeze and never including produced evidence.
The later `SubjectId`, `EvidenceClosureId`, and `AdmissionRecordId` bind the
candidate artifact, actual observations, and ratified quorum attestation set.
The public capability has one inhabitant: `HumanReviewCandidate`. There is no
`Merge`, `Deploy`, `Settle`, `Waive`, or `OverrideUnknown` constructor.
([residue shape, lines 61–127](102-promotion-permit-residue.md#L61-L127))

### 9.2 The first theorem boundary

**`[RESEARCH RECOMMENDATION]`** The proof should establish only:

1. exact base/candidate/diff/path/materialization/campaign/action,
   governability-gate/quorum, and admission binding;
2. issuance only from authenticated `Assessment<HumanReview>::PROCEED` with no
   mandatory open obligations under the frozen policy and authority view;
3. typed rejection of `HALT`, `UNKNOWN`, malformed, stale, revoked, duplicate,
   mismatched, and future-unrecognized inputs;
4. total, panic-free, deterministic, canonical behavior;
5. preservation of modality and authority; and
6. no constructor in the residue API from the permit to merge, deploy, or
   other consequence.

It does not prove that the path map is complete, the repair fixes the bug class,
the oracle expresses intent, an organization is independent, SHA-256 cannot
collide, the compiler is correct, or the external actor honors the API. Those
obligations remain respectively in DST, evidence, institutional trust, and the
published trusted computing base.

“No consequence constructor” is an **API-confinement property**, not a theorem
that every consumer in the system behaves. The admitted implementation must
also use sealed production constructors, compile-fail capability tests,
parser/authentication fuzzing, mutation tests for every issuance conjunct, and
an end-to-end consequence-router negative that rejects permit bytes at merge
and deploy boundaries.
([theorem suite and API-confinement boundary, lines 167–283](102-promotion-permit-residue.md#L167-L283))

### 9.3 Court remains a human choice

[#102 — Compare the smallest proved PromotionPermit residue](102-promotion-permit-residue.md#L13-L37)
provisionally recommends a restricted safe-Rust predicate proved
with Verus, later decorrelated by a Lean model and differential testing. It
reserves Isabelle/HOL for a later isolation/noninterference kernel. This is not
a decision. The first proof court, proof TCB, timebox, and revival path remain
open in [#109](https://github.com/AmitabhainArunachala/vibe-halt/issues/109).
([proof-court comparison, lines 364–394](102-promotion-permit-residue.md#L364-L394))

---

## Article X — A court that cannot become the state

### 10.1 Minimum separated seats

**`[LOCKED MAP PREMISE + RESEARCH RECOMMENDATION]`** The smallest
non-sovereign topology distinguishes five authority-bearing seats plus one
untrusted treatment guest:

1. **scope authority**, which proposes and freezes subject, claims, policy,
   paths, envelope, campaign, and budget but cannot certify completeness;
2. **treatment**, which authors an untrusted candidate in a disposable surface;
3. **runner**, which observes the frozen campaign and signs only observation;
4. **admission judge or ratified judge quorum**, which verifies evidence and
   emits only `Assessment<Action> = HALT | PROCEED | UNKNOWN`;
5. **transparency service plus independent monitor**, which makes chronology
   and equivocation challengeable; and
6. **external decision authority**, outside Vibe Halt's trust root, which alone
   chooses and records the real-world act.

The platform-mandatory consequence taxonomy is non-excludable. Exclusions are
typed, attributable, expiring claims. An independent coverage assessment may
force `UNKNOWN` or `UNGOVERNED` even when scope authority accepted the declared
map; authority over scope is not a root oracle for completeness.

Whether admission uses one mechanically independent judge, a 2-of-2 pair, a
threshold, or organizationally diverse courts is unresolved. Separate keys
held by one operator are a weaker authority grade and must never be marketed as
institutional independence.
([non-sovereign authority topology, lines 10–41](105-nonsovereign-authority.md#L10-L41),
[authority-grading research, lines 160–170](105-nonsovereign-authority.md#L160-L170))

### 10.2 Distinct constitutional objects

**`[RESEARCH RECOMMENDATION]`** Scope, observation, admission, and consequence
must not be optional fields on one mutable receipt:

```text
FrozenScope<Subject, Revisions, Paths, Properties, Envelope,
            Controllers, Campaign, Budget, Policy>

Observation<Scope, EvidenceSet, RunnerIdentity, RunnerAuthorityGrade>

Admission<Scope, EvidenceClosure, Policy,
          Action, Assessment<Action>,
          GovernabilityGateWitness<Action, Scope, Policy>,
          AdmissionQuorumWitness<Policy>>

ExternalDisposition<AdmissionDigest<Action>, Action, ExternalAuthority,
                    Reason, Jurisdiction, Expiry>

Appeal<OldAdmissionDigest, NewScopeOrEvidence>
Supersession<OldAdmissionDigest, NewAdmissionDigest>
Revocation<CredentialOrPolicyDigest, Reason, EffectiveTime>
```

Normal action requires both the external institution's authorization and the
admission object its policy demands. The admission court can withhold a normal
precondition; it cannot perform the action.
([typed constitutional objects, lines 63–127](105-nonsovereign-authority.md#L63-L127))

### 10.3 `UNKNOWN` cannot be overwritten

**`[LOCKED MAP PREMISE]`** No actor inside Vibe Halt may override `UNKNOWN`.
No external actor may silently rewrite it. If an external institution elects
to act despite `UNKNOWN`, it creates a distinct, scoped, attributable,
expiring, appealable
`ExternalDisposition<Admission<Action, UNKNOWN>, ActAnyway>` while the
admission remains `Assessment<Action>::UNKNOWN` in the append-only history. A later
`Assessment<Action>::PROCEED` requires a new scope or evidence and a new
admission for that same action type.

The constitution of an external institution may forbid exceptional action for
some consequence classes. This draft does not decide which exceptions exist.
([external-disposition boundary, lines 120–127](105-nonsovereign-authority.md#L120-L127),
[exception-policy boundary, lines 207–221](105-nonsovereign-authority.md#L207-L221),
[#116 — Ratify institutional authority, appeal, and amendment](https://github.com/AmitabhainArunachala/vibe-halt/issues/116))

### 10.4 Appeal, revocation, and amendment

**`[RESEARCH RECOMMENDATION]`** An appeal never edits the challenged admission;
it creates a linked new campaign or evidence set and a new admission.
Credential revocation is append-only and forces future trust-path evaluation;
it does not erase historical bytes or decide their semantics. Semantic censure
points to the exact prior claim and changes future reliance without pretending
its signature was invalid.

Constitutional policy is a versioned content-addressed artifact. Any amendment
must name old and new policy identities, meet the old authorization and new
acceptance thresholds, state its effective boundary, enter transparency before
use, and never reinterpret old evidence without a new admission.

The institutional topology, appeal standing, exception classes, and amendment
thresholds remain reserved to
[#116 — Ratify institutional authority, appeal, and amendment](https://github.com/AmitabhainArunachala/vibe-halt/issues/116).
Its cryptographic key custody and quorum mechanics remain reserved to
[#110 — Ratify evidence trust and key custody](https://github.com/AmitabhainArunachala/vibe-halt/issues/110).
([appeal, revocation, and amendment research, lines 172–192](105-nonsovereign-authority.md#L172-L192))

---

## Article XI — Falsification, investment, and graceful death

### 11.1 Zero-tolerance invariants

**`[LOCKED MAP PREMISE + RESEARCH RECOMMENDATION]`** The following are not
metrics to average. One demonstrated violation invalidates the attempt and
kills the affected claim or grade until the unchanged falsifier passes in a
fresh campaign:

- wrong-subject, stale, wrong-envelope, or non-canonical evidence is accepted;
- a mandatory open, unsupported, or divergent channel yields `PROCEED`;
- treatment selects or edits its judge, oracle, holdout, policy, evidence, or
  trust root;
- evidence for candidate A authorizes candidate B;
- a leaked or altered holdout retains credit;
- a miss, error, unknown, invalid attempt, or abort is removed from a frozen
  denominator;
- fixture, model, port, or weaker-envelope evidence is reported as native
  real-target evidence; or
- authority, signature, consensus, or operator override lifts modality.
([research zero-tolerance invariants, lines 63–79](106-falsification-economic-kill-matrix.md#L63-L79))

### 11.2 Typed stage decisions

```text
StageDecision<Stage, FrozenContract, EvidenceSet> =
    Advance
  | Hold(UnknownObligations)
  | Reorient(FalsifiedHypothesis, PreservedKernel)
  | Kill(ViolatedInvariant)
  | Invalid(CompromisedAttempt)
```

There is no constructor from a product verdict, signature, model consensus, or
operator assertion directly to `Advance`. “Kill” applies to a claim,
hypothesis, capability grade, investment path, or compromised attempt—not to
the retained evidence or the people who proposed it.

### 11.3 Investment gates

**`[RESEARCH RECOMMENDATION]`** The evolved program proceeds through typed
gates:

| Gate | Advance evidence | Kill or reorient when |
|---|---|---|
| `0 Truthful kernel` | tamper, parser, replay, divergence, and open-channel negatives all fail closed | any silent acceptance occurs |
| `1 Whole target` | one exact Dharma target completes map → coverage → attack → evidence → independent replay → verdict | Product Lock's six-week path cannot traverse one target; stop breadth and repair intake-to-receipt |
| `2 Treatment/admission` | known bad and fixed control discriminate; exact identity and hidden revalidation survive; treatment cannot touch judge surfaces | replay overfit, permit rebind, or repairer influence occurs |
| `3 Governable refinery` | every declared, discovered, and platform-mandatory path class in the frozen accounting set has a property/channel grade; blind spots remain ledgered; no `UnboundedOpen` path reaches consequence; admitted repair survives hidden validation; bounded `UNKNOWN` advances only if #112 ratifies it | scope deletion, tautological properties, oracle edits, unknown suppression, or trusted-effect expansion creates apparent cleanup |
| `4 Real-fault advantage` | at least one important confirmed reproducible Dharma fault missed by frozen AI baselines, then the accepted hidden benchmark succeeds | three eligible equal-budget tournaments show no severity-weighted yield advantage; pause expansion for human reorientation |
| `5 Antithesis-grade` | exact real profile meets frozen control, replay, topology, search-advantage, throughput, and cost criteria | mandatory channels cannot close, replay fails, or search cannot beat its null on a discriminating target |
| `6 Non-sovereign settlement` | one external consequence gate consumes a bounded admission while an independent party can verify, challenge, and replay it | grader can silently permit, actor and judge collapse, or outage becomes an undeclared sovereign veto |
([research stage gates, lines 80–210](106-falsification-economic-kill-matrix.md#L80-L210))

Product Lock's existing six-week and three-tournament rules remain law unless a
human-ratified amendment changes them. Whether a new constitutional campaign
restarts the clock, and the remaining numerical refinery, reach, market, and
settlement thresholds, are unresolved in
[#117 — Ratify success metrics and kill conditions](https://github.com/AmitabhainArunachala/vibe-halt/issues/117).
([Product Lock v1 six-week proof, lines 123–135](https://github.com/AmitabhainArunachala/vibe-halt/blob/d19ba9e1198c0ab7ef4bb1bdf69a299d056f9754/docs/specs/PRODUCT_LOCK_V1.md#L123-L135),
[kill and reorientation law, lines 147–156](https://github.com/AmitabhainArunachala/vibe-halt/blob/d19ba9e1198c0ab7ef4bb1bdf69a299d056f9754/docs/specs/PRODUCT_LOCK_V1.md#L147-L156))

### 11.4 Anti-Goodhart protocol

**`[RESEARCH RECOMMENDATION]`**
Before a credit-bearing campaign reveals any result, freeze target and artifact
identity, eligible population, properties, fault model, budgets, baselines,
severity weights, confirmation owner, denominators, miss-retention rule,
deduplication, holdout commitment, exact tool/policy/environment identities,
stopping rule, and the threshold for every stage decision. Publish raw measures
and retained nulls. A later adaptation creates a new campaign.
([research anti-Goodhart protocol, lines 222–238](106-falsification-economic-kill-matrix.md#L222-L238))

---

## Article XII — Settlement without sovereignty

### 12.1 The future act

**`[LOCKED MAP PREMISE]`** The long-horizon object is not a universal truth
token. It is a typed relationship between one bounded machine claim and one
externally governed consequence:

```text
Act<
  Kind,
  Subject,
  Claim,
  Admission,
  ExternalDisposition
>
```

A software release, agent action, transfer, mint, tonne, or tally may one day
require a particular admission grade under its own institution's policy. Vibe
Halt supplies the challengeable grade and open-channel ledger. It does not
become the wallet, registry, election authority, legislature, or state.

### 12.2 Future theaters

Crypto, carbon, voting, medical, public authority, and other high-consequence
domains are future theaters, not evidence for the first campaign and not
implementation scope of this map. In each theater the same constitutional
boundary applies:

- proving accounting code does not prove the asset or world sensor;
- verifying a contract does not close its oracle, relayer, keeper, governance,
  or human-key channels;
- verifying tally logic does not close phone malware, eligibility, or coercion;
- a signed procedure does not make its input true; and
- `UNKNOWN` must remain visible wherever reality is not controlled.

### 12.3 The destination sentence

Vibe Halt's highest honest form is Antithesis for the supported real target, a
refinery for the vibe-coded mass, a proof court for the distilled residue, and
a type system for when a bounded claim may be offered to settlement.

It does not abolish uncertainty. It gives uncertainty a type, a location, an
owner, a replay where possible, and a constitutional right not to be silently
promoted.

---

## Article XIII — Explicit non-claims

These non-claims preserve the accepted implementation boundary and the
planning run's no-authority boundary; they must be rechecked against the final
ratification revision rather than inherited by prose.
([accepted README boundary, lines 24–74](https://github.com/AmitabhainArunachala/vibe-halt/blob/d19ba9e1198c0ab7ef4bb1bdf69a299d056f9754/README.md#L24-L74),
[accepted receipt boundary, lines 26–31](https://github.com/AmitabhainArunachala/vibe-halt/blob/d19ba9e1198c0ab7ef4bb1bdf69a299d056f9754/crates/vh-cli/src/receipts_v2.rs#L26-L31),
[Wayfinder map #100](https://github.com/AmitabhainArunachala/vibe-halt/issues/100))

This draft does not claim that:

- Vibe Halt currently governs a real foreign target end to end;
- current evidence is signed or settlement-grade;
- `CooperativeD2` is deterministic, contained, or D1;
- native interposition or a capability-closed backend exists in accepted main;
- a WASI artifact is the native CPython subject;
- finite simulation proves a whole program or world;
- coverage, consensus, signatures, transparency, or formal syntax establishes
  truth;
- the refinery can autonomously repair every repository;
- `GOVERNABLE` means safe, correct, proved, or free of unknowns;
- the first residue proves Dharma or the adequacy of its property contract;
- any proof assistant, verifier, compiler, kernel, key topology, or holdout
  protocol has been selected or implemented;
- Vibe Halt may certify its own repair, override `UNKNOWN`, or execute the
  consequence it grades; or
- this planning run authorizes product code, foreign execution, merge,
  deployment, spending, or institutional action.

---

## Article XIV — Research reconciliation

### 14.1 Evidence sovereignty — research #103

[#103 — Evidence sovereignty without a root oracle](103-evidence-sovereignty.md#L13-L80)
is incorporated as a **recommendation**, not adopted law. Its load-bearing
contribution is the separation:

```text
VerifySignature<Statement<T>, Authorized<Role>, PolicyEpoch>
    -> Attributed<T, Role>

Project<ClosedEvidenceSet, AdmissionQuorum<Policy>, FreshPolicy>
    -> Assessment<Action> = HALT | PROCEED | UNKNOWN
```

No constructor exists from a signature, root approval, transparency receipt,
or external override directly to `PROCEED`. A TUF-shaped root distributes role
and schema authority; it is not an oracle. A transparency service makes
publication and equivocation challengeable; it is not a judge. A holdout
commitment makes later substitution detectable; it cannot prove non-leakage.

The proposed 2-of-3 policy root, 2-of-2 judge agreement, two witnesses, two
content-addressed replicas, and two-curator selector are deliberately strong
v1 research defaults. A 2-of-2 judge rule is fail-safe against one judge
manufacturing permission, but either judge can withhold and force `UNKNOWN`;
where normal action requires `PROCEED`, that is an effective veto and a real
liveness/sovereignty cost. Thresholds must therefore be parameterized by a
ratified `AdmissionQuorum<Policy>` and published with their independence and
common-mode vector.

The liveness, cost, custody, and actual independence choices remain unresolved
in [#110 — Ratify evidence trust and key custody](https://github.com/AmitabhainArunachala/vibe-halt/issues/110).
Ratification must publish the chosen topology's trusted computing base rather
than silently adopting the most elaborate diagram.

### 14.2 Governable-repository refinery — research #108

[#108 — Governable-repository refinery](108-governable-refinery.md#L13-L46)
is incorporated as a **recommendation**, not a capability claim. It defines
`GOVERNABLE` as closed accounting of consequential uncertainty over the five
linked maps, orthogonal to `HALT | PROCEED | UNKNOWN`. It recommends:

```text
FREEZE → MAP → CONTRACT → SHAKE → SHRINK/EXPLAIN
       → untrusted TREAT/DECLUTTER/DISTILL-CODE → hostile re-entry
       → independent fresh/hidden RE-SHAKE
       → optional PROVE ALREADY-FROZEN RESIDUE → ADMIT
```

It also draws the decisive limit: accounting can close over declared,
discovered, and platform-mandatory path classes, but finite discovery cannot
prove that an arbitrary program has no other behavior. Bounded blind spots may
remain explicit and force `UNKNOWN`; an unbounded blind spot to consequence
blocks `GOVERNABLE` itself. The final bounded-open rule, atomic object, support
profiles, and numerical gates remain for
[#112 — Ratify what GOVERNABLE requires](https://github.com/AmitabhainArunachala/vibe-halt/issues/112),
[#114 — Decide the atomic admission object](https://github.com/AmitabhainArunachala/vibe-halt/issues/114),
and [#117 — Ratify success metrics and kill conditions](https://github.com/AmitabhainArunachala/vibe-halt/issues/117).

---

## Article XV — Human ratification gates

This draft deliberately does not answer the following tickets. It reserves them
to live human decision and requires the tracker to keep final ratification
blocked until every prerequisite decision is resolved. Tracker state is mutable;
the linked [Wayfinder map #100](https://github.com/AmitabhainArunachala/vibe-halt/issues/100)
is the delivery record, not epistemic evidence by itself.

| Gate | Decision reserved to the operator | This draft's non-decision |
|---|---|---|
| [#109 — Choose the first proof court](https://github.com/AmitabhainArunachala/vibe-halt/issues/109) | Court, source language, trusted computing base, timebox, and revival condition | Records Verus as a research recommendation only |
| [#110 — Ratify evidence trust and key custody](https://github.com/AmitabhainArunachala/vibe-halt/issues/110) | Root custody, role keys, judge/curator thresholds, transparency, revocation, liveness trade | Requires attribution and separation but chooses no custodians or threshold |
| [#112 — Ratify what `GOVERNABLE` requires](https://github.com/AmitabhainArunachala/vibe-halt/issues/112) | Exact mandatory criteria and whether any bounded unknown is compatible | Preserves the locked definition but sets no final threshold |
| [#113 — Ratify reach investment and revival thresholds](https://github.com/AmitabhainArunachala/vibe-halt/issues/113) | Native-interposition and capability-closure time, compatibility, replay, containment, and cost gates | Fixes envelope identity and no-laundering only |
| [#114 — Decide the atomic admission object](https://github.com/AmitabhainArunachala/vibe-halt/issues/114) | Repository, consequence, operation, path set, claim, or another atomic unit | Uses typed scopes without choosing the public atom |
| [#115 — Ratify the first Dharma campaign contract](https://github.com/AmitabhainArunachala/vibe-halt/issues/115) | Exact target SHA, path, claims, negative arms, envelopes, budgets, roles, and terminal mapping | Recommends `apply_sealed_packet` and review candidacy only |
| [#116 — Ratify institutional authority, appeal, and amendment](https://github.com/AmitabhainArunachala/vibe-halt/issues/116) | External exception classes, institutional seats, appeal standing, amendment authority, independence grade | Forbids silent modality lifting but grants no exception or key topology |
| [#117 — Ratify success metrics and kill conditions](https://github.com/AmitabhainArunachala/vibe-halt/issues/117) | Numerical thresholds, inherited clocks, economic acceptance, pause/revival procedure | Preserves existing law and proposes a raw stage vector |
| [#111 — Ratify the final Vibe Halt constitution](https://github.com/AmitabhainArunachala/vibe-halt/issues/111) | Accept, amend, or reject the integrated constitution as a whole | This file remains `UNRATIFIED` until that explicit act and merge |

No tool availability, implementation sequence, agent vote, model consensus,
signature count, or passage of time may implicitly answer these questions.

---

## Article XVI — Research provenance

The following artifacts informed this draft but do not themselves carry
ratification authority:

- [#101 — First Dharma promotion campaign](101-dharma-promotion-campaign.md)
- [#102 — Compare the smallest proved PromotionPermit residue](102-promotion-permit-residue.md)
- [#103 — Evidence sovereignty without a root oracle](103-evidence-sovereignty.md)
- [#104 — Execution envelopes without evidence laundering](104-execution-envelopes.md)
- [#105 — Keep the settlement typechecker non-sovereign](105-nonsovereign-authority.md)
- [#106 — Falsification and economic kill matrix](106-falsification-economic-kill-matrix.md)
- [#107 — Path to Antithesis-grade real-target testing](107-antithesis-grade-path.md)
- [#108 — Governable-repository refinery](108-governable-refinery.md)

The constitution must cite the exact accepted revision when it is finally
ratified. If implementation or law changes before ratification, affected
research claims must be rechecked rather than carried forward by narrative.

---

## Ratification block

```text
Document status: UNRATIFIED
Constitution digest: NOT YET FROZEN
Accepted code revision: d19ba9e1198c0ab7ef4bb1bdf69a299d056f9754
Research incorporated as recommendations: #101–#108
Required human gates: #109, #110, #112, #113, #114, #115, #116, #117
Final human gate: #111
Effective authority: NONE
```

Until that block is replaced by an explicit, content-addressed human
ratification and merged artifact, the governing product document remains
[Product Lock v1](https://github.com/AmitabhainArunachala/vibe-halt/blob/d19ba9e1198c0ab7ef4bb1bdf69a299d056f9754/docs/specs/PRODUCT_LOCK_V1.md#L1-L9).
