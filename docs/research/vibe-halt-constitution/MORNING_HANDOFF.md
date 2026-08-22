# Morning handoff — the machine, the refinery, and the court

**Status:** planning deliverable; not ratified law and not capability evidence

**Wayfinder map:**
[#100 — Ratify the Vibe Halt evolution constitution](https://github.com/AmitabhainArunachala/vibe-halt/issues/100)

**Accepted evidence base:**
[`d19ba9e1198c0ab7ef4bb1bdf69a299d056f9754`](https://github.com/AmitabhainArunachala/vibe-halt/tree/d19ba9e1198c0ab7ef4bb1bdf69a299d056f9754)

## What was settled at the planning layer

Vibe Halt's highest coherent form is one organism with separated powers:

- the **shaker** reaches real software, injects hostile worlds, finds failures,
  and makes them replayable;
- the **refinery** maps a mess, proposes repairs and decluttering, and may
  isolate a small consequential residue; and
- the **court** consumes independently produced evidence and decides only
  whether one typed action has met its frozen admission policy.

This weaves Antithesis-like testing and “clear up the vibe-coded mess” without
letting the repairer grade its own work. Every treatment output—including a
new guard or distilled kernel—becomes a new untrusted `SubjectId`, returns to
`FREEZE → MAP`, and survives an independent fresh/hidden re-shake. A proof may
describe only a residue already present in that frozen subject; it cannot
bypass correspondence and bypass testing.

The promise is deliberately finite: a supported target leaves as a repaired
candidate with bounded evidence, a replayable `HALT`, or an `UNKNOWN` that names
the open obligations and discovery blind spots. Vibe Halt does not promise to
repair arbitrary software or prove a whole application.

## The load-bearing type boundary

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
    RequiredAndGovernable(Projection)
  | RequiredButUngoverned(Projection, BlindSpots)
  | NotRequired(Projection, RatifiedPolicyClause)
```

An integrity failure is not laundered into a verdict. A valid `HALT` or
`UNKNOWN` can retain `RequiredButUngoverned` as signed evidence, but that branch
cannot produce a gate witness or review permit. `NotRequired` cannot be a silent
default: it names the action-specific clause that made governability optional.
This is the small AI-native-language contribution of the run—modality and
authority constrain construction, rather than living only in explanatory
receipt text.

## The first reality-bearing engineering question

The first campaign is one pinned, real Dharma CPython mutation-to-review seam:
`DarwinEngine.apply_sealed_packet`. It must prove two identity-coupled facts:

1. Vibe Halt discovers, independently replays, and shrinks one real behavioral
   mutant through the real target path; and
2. a human-supplied repair for that same finding re-enters at MAP, survives
   fresh and hidden validation, while a same-path/different-bytes candidate is
   rejected as an integrity mismatch before mutation.

The initial `CooperativeD2` run is an honest mechanism/reach phase. With the
accepted Vibe Halt base's subprocess channels still open, it can emit a signed
`CampaignAssessment<HumanReview>` but cannot mint a review-candidacy permit. A
later campaign under a separately eligible execution envelope may construct
that permit if every action-specific gate is satisfied
([accepted capability boundary](101-dharma-promotion-campaign.md#L217-L220)).

## Evidence cannot sign itself

The proposed graph removes two subtle self-reference failures:

```text
CampaignSpecId
  → ordered holdout commitments
  → CampaignId
  → candidate freeze / SubjectId / role observations
  → EvidenceClosureId
  → AdmissionPayloadId
  → ordered judge attestations
  → AdmissionRecordId
  → quorum and governability-gate witnesses
```

Judges sign identical payload bytes; their signatures are assembled only
afterward. The pure governability projection contains no future quorum witness.
Signatures establish attribution under a policy epoch, never the truth of the
claim they carry.

## Human decisions that remain

Do not ratify the integrated draft by implication. Grill these tickets live,
one question at a time, in dependency order:

1. [#114 — Decide the atomic admission object](https://github.com/AmitabhainArunachala/vibe-halt/issues/114)
2. [#116 — Ratify institutional authority, appeal, and amendment](https://github.com/AmitabhainArunachala/vibe-halt/issues/116)
3. [#110 — Ratify evidence trust and key custody](https://github.com/AmitabhainArunachala/vibe-halt/issues/110)
4. [#112 — Ratify what GOVERNABLE requires](https://github.com/AmitabhainArunachala/vibe-halt/issues/112)
5. [#113 — Ratify reach investment and revival thresholds](https://github.com/AmitabhainArunachala/vibe-halt/issues/113)
6. [#117 — Ratify success metrics and kill conditions](https://github.com/AmitabhainArunachala/vibe-halt/issues/117)
7. [#115 — Ratify the first campaign contract](https://github.com/AmitabhainArunachala/vibe-halt/issues/115)
8. [#109 — Choose the first proof court](https://github.com/AmitabhainArunachala/vibe-halt/issues/109)
9. [#111 — Ratify the final Vibe Halt constitution](https://github.com/AmitabhainArunachala/vibe-halt/issues/111)

The first morning question is therefore not “ptrace or WASI?” It is: **what is
the one atomic public object whose construction Vibe Halt may authorize?** Until
that is answered, mechanism and proof-court choices optimize an unsettled
authority boundary.

## What did not happen overnight

- No product code was implemented.
- No foreign target was executed.
- No production or ambient developer tree was modified.
- No merge, deployment, release, spend, credential use, or external contact was
  performed.
- No human ratification gate was answered, assigned away, or silently closed.
- No research conclusion was promoted into capability or constitutional law.

The integrated proposal is in
[the evolution-constitution draft](000-evolution-constitution-draft.md); the
individual research boundaries and their nonclaims are indexed in
[the directory README](README.md).
