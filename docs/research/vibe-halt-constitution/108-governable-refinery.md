# Research 108 — Architecture of a governable-repository refinery

**Status:** research resolution; constitution input, not implementation or
capability evidence

**Accepted Vibe Halt base:**
[`d19ba9e1198c0ab7ef4bb1bdf69a299d056f9754`](https://github.com/AmitabhainArunachala/vibe-halt/tree/d19ba9e1198c0ab7ef4bb1bdf69a299d056f9754)

**Question:** What architecture can turn an arbitrary *supported* messy
repository into `GOVERNABLE` without pretending to comprehend, repair, or prove
arbitrary software?

## Resolution

Build the refinery as an evidence-spined loop whose output is a **closed
accounting of consequential uncertainty**, not a cleanliness score:

```text
FREEZE → MAP → CONTRACT → SHAKE → SHRINK/EXPLAIN
   ↑                                   |
   +-- TREAT/DECLUTTER/DISTILL-CODE ----+  (new untrusted subject)
                         |
                  INDEPENDENT RE-SHAKE
                         |
          optional PROVE FROZEN RESIDUE → ADMIT
```

This research recommends that `GOVERNABLE` be a state of the target map,
orthogonal to the target verdict:

- `GOVERNABLE + HALT`: a consequential defect is reproducible and blocking.
- `GOVERNABLE + UNKNOWN`: **proposed, not ratified**—the unresolved or
  uncontrolled surfaces carry structural consequence-bound witnesses and
  prevent `PROCEED`.
- `GOVERNABLE + PROCEED`: every mandatory path class completed the frozen,
  bounded admission policy; this is still not a general safety proof.

The architecture therefore promises that no **declared, discovered, or
platform-mandatory consequential path class** disappears between intake and
admission. It cannot promise that finite analyses discovered every behavior of
an arbitrary program. A discovery blind spot is itself an `Open` obligation; a
blind spot whose possible consequences cannot be bounded prevents
`GOVERNABLE`, not merely `PROCEED`.

Whether the public label may coexist with any bounded `UNKNOWN` remains the
human decision in [Ratify what GOVERNABLE requires](https://github.com/AmitabhainArunachala/vibe-halt/issues/112).

This recommendation extends, rather than weakens, Product Lock v1. The current law already says
that a broad whole-repository front door creates no broad proof claim, requires
an exact observed revision and coverage ledger, and maps unsupported or
unverifiable coverage to `UNKNOWN`
([Product Lock v1, lines 17–24](https://github.com/AmitabhainArunachala/vibe-halt/blob/d19ba9e1198c0ab7ef4bb1bdf69a299d056f9754/docs/specs/PRODUCT_LOCK_V1.md#L17-L24)).
It also requires any future patch generator to remain separate from
verification authority
([lines 31–49](https://github.com/AmitabhainArunachala/vibe-halt/blob/d19ba9e1198c0ab7ef4bb1bdf69a299d056f9754/docs/specs/PRODUCT_LOCK_V1.md#L31-L49)).

## What `supported` means

“Arbitrary supported repository” is deliberately narrower than “any
repository.” A target is supported for a campaign only when a versioned adapter
profile can:

1. bind the submitted source tree, submodules, lockfiles, build definitions,
   generated inputs, container/runtime coordinates, permitted commands, and
   data boundary to immutable digests;
2. account for every observed artifact as included, externally scoped out, or
   `Open`—never silently omitted;
3. parse or dynamically observe the target's relevant entry points,
   dependencies, and consequence mechanisms with named methods and known blind
   spots;
4. run the authorized workflows in a declared execution envelope and publish
   its capability-channel ledger; and
5. attach an executable oracle to every mandatory property, or return
   `UNKNOWN` before treatment.

Support is negotiated per exact adapter version, language/runtime profile, and
target revision. “Python supported” cannot silently imply that native
extensions, runtime code generation, arbitrary plugins, browser code, shell
children, or external services were analyzed. The current main is honest about
this distinction: it has not demonstrated a real Dharma adapter, arbitrary
foreign deterministic targets, D1 containment, or a real acceptance holdout
([README, lines 24–67](https://github.com/AmitabhainArunachala/vibe-halt/blob/d19ba9e1198c0ab7ef4bb1bdf69a299d056f9754/README.md#L24-L67)).

## The five maps, not one omniscient graph

A single “repository graph” would blur evidence kinds. Preserve five linked,
content-addressed maps:

| Map | Nodes and edges | Evidence and honest boundary |
|---|---|---|
| Artifact/provenance | files, packages, lock entries, generated assets, images, models, datasets, build inputs | filesystem/build observations plus declared provenance; unresolved or runtime-fetched material stays `Open` |
| Executable structure | entry points, modules, calls, jobs, handlers, workflows, data/control-flow edges | language-adapter static analysis plus runtime observations; reflection and dynamic dispatch retain adapter-specific uncertainty |
| Capability/consequence | authority sources, secrets, state stores, external services, and effect sinks such as writes, sends, spends, merges, deploys, or model/tool actions | target declarations **unioned with** a platform-mandatory sink taxonomy; target omission cannot narrow the denominator |
| Claim/contract/oracle | invariants, pre/postconditions, always/sometimes/reachability properties, oracle implementations, authority and modality | source and witness recorded separately; inferred properties begin as proposals, not truth |
| Evidence/decision | campaigns, universes, findings, replays, reductions, repairs, holdouts, judges, verdict projections | exact subject/envelope/policy identities and signed attestations; signatures attribute statements but do not make them true |

SPDX 3.0.1 is useful prior art for the artifact layer because its model can
represent packages, files, snippets, builds, AI models, datasets, provenance,
integrity, vulnerabilities, and relationships; it is an interchange substrate,
not proof that the inventory is complete
([SPDX 3.0.1 scope](https://spdx.github.io/spdx-spec/v3.0.1/scope/)).
CodeQL path queries demonstrate the useful source→flow→sink abstraction for the
executable layer, while its own documentation notes that global data flow trades
precision and cost; this is an analysis result, not a total semantics
([CodeQL path queries](https://codeql.github.com/docs/writing-codeql-queries/creating-path-queries/),
[global data-flow limits](https://codeql.github.com/docs/codeql-language-guides/analyzing-data-flow-in-csharp/#global-data-flow)).

### Consequential-path classes

A consequential path is a finite abstraction, not one concrete execution:

```text
PathClass = EntryPoint
          × AuthoritySource
          × State/Data Transformation Class
          × ConsequenceSink
          × ExecutionEnvelope
```

The platform supplies non-optional sink classes for at least filesystem/state
mutation, outbound communication, process/tool invocation, secret access,
identity/authorization decisions, financial or asset movement, governance
actions, and merge/deploy/release. A tenant may add classes but cannot remove a
platform-mandatory class. Every class has exactly one grade:

```text
PathGrade = Governed(ContractRef, OracleRef, EvidenceRef)
          | Halted(FindingRef)
          | BoundedOpen(Obligation, ConsequenceBoundWitness, ControllerStatus)
          | UnboundedOpen(Obligation, PossibleConsequence)
          | Excluded(ScopeAuthority, Reason, Expiry)
```

Mandatory sink classes cannot be `Excluded`. An `Open` path may coexist with
`GOVERNABLE` only if the human ratifies that rule and a structural controller
or non-reachability witness—not scope authority alone—bounds its possible
consequence and control status; it forces the target verdict to `UNKNOWN`. An unresolved
loader, reflection, plugin, FFI, or escape channel that can reach an unknown
consequence is `UnboundedOpen` and blocks construction of `GOVERNABLE`.

This distinguishes **accounting closure** from impossible behavioral
exhaustiveness. The existing sandbox ledger is the right precedent: omission
never means closed, and D1 requires all 29 named channels to be controller-
closed
([Sandbox Capability Envelope v1, lines 14–40](https://github.com/AmitabhainArunachala/vibe-halt/blob/d19ba9e1198c0ab7ef4bb1bdf69a299d056f9754/docs/specs/SANDBOX_CAPABILITY_ENVELOPE_V1.md#L14-L40)).

## Staged refinery architecture

### 0. FREEZE — establish the subject and denominator

- Materialize the target only in an authorized disposable workspace.
- Bind base revision, tree digest, dependency/lock digests, build/runtime
  profile, adapter set, execution envelope, commands, data policy, budget, and
  platform consequence taxonomy.
- Freeze exclusions, severity policy, mandatory workflows, judge identity, and
  which fields treatment may change.
- Reject an unfrozen or internally inconsistent intake; do not let treatment
  “repair” the campaign definition.

The freeze is an attestation about a digest-bound subject, not proof of source
truth. The in-toto Statement specification is useful shape prior art: subjects
are associated by digest with a typed predicate
([in-toto Statement v1](https://github.com/in-toto/attestation/blob/051624ce466deaed4c5a66e66877f69b471fccbe/spec/v1/statement.md)).

### 1. MAP — create an accounting ledger before interpreting tests

Run adapter-specific discovery and reconcile its results with the frozen
manifest:

- enumerate source, configuration, build, workflow, migration, policy, model,
  and data-schema artifacts;
- resolve dependency and build graphs as far as the environment permits;
- identify static and runtime entry points;
- map data/control flows into the platform and tenant consequence sinks;
- map capability acquisition and bypass routes;
- record parser failures, unresolved imports, dynamic loading, generated code,
  external service behavior, and unobserved paths as typed open obligations;
  and
- have a decorrelated mapper inspect the target without seeing the first map,
  then preserve disagreements rather than averaging them away.

The map is versioned. Any target change invalidates affected edges. Changes to
entry points, dependency/build configuration, capability policy, or discovery
inputs trigger full remapping; a repairer's claim that its diff is “local” is
not authority to narrow impact.

### 2. CONTRACT — extract candidates, then qualify oracles

Collect property candidates from, in descending evidentiary order:

1. externally authorized domain rules and protocol specifications;
2. executable tests, types, assertions, schemas, policy, and public API
   contracts;
3. incident reports and exact historical replays;
4. differential or reference implementations;
5. dynamic invariant mining, static inference, and model-generated proposals.

The last category is valuable reconnaissance only. Daikon's foundational work
explicitly describes its results as **likely** invariants inferred from observed
traces and analyzes their dependence on test suites and instrumentation
([Ernst et al., 2001](https://plse.cs.washington.edu/daikon/pubs/invariants-tse2001-abstract.html)).
Therefore an inferred invariant enters as `Proposed<Property>`. Ratification
may change its authority but cannot lift its epistemic modality; Vibe Halt's
current promotion evaluator already requires an adjacent, revision-matched
witness and exposes no transition to `Proven`; the public runtime `Claim` record
is not yet a sealed proof type
([`modality.rs`, law](https://github.com/AmitabhainArunachala/vibe-halt/blob/d19ba9e1198c0ab7ef4bb1bdf69a299d056f9754/crates/vh-cli/src/modality.rs#L1-L9),
[`modality.rs`, public runtime `Claim`](https://github.com/AmitabhainArunachala/vibe-halt/blob/d19ba9e1198c0ab7ef4bb1bdf69a299d056f9754/crates/vh-cli/src/modality.rs#L50-L56),
[`modality.rs`, evaluator](https://github.com/AmitabhainArunachala/vibe-halt/blob/d19ba9e1198c0ab7ef4bb1bdf69a299d056f9754/crates/vh-cli/src/modality.rs#L92-L138)).

Qualify each oracle with at least one independent pressure test:

- kill predeclared mechanism-relevant mutants;
- distinguish known faulty/fixed controls;
- agree with an independent reference where one exists;
- satisfy metamorphic relations that do not depend on exact expected output;
  and
- include reachability/sometimes obligations so a never-exercised assertion
  cannot look healthy.

Antithesis's public property model makes the same asymmetry explicit: one
counterexample can disprove an always-property, examples cannot prove it, and
sometimes/reachability properties prevent vacuous “passes”
([Antithesis properties](https://antithesis.com/docs/properties_assertions/properties/),
[assertion cataloging](https://antithesis.com/docs/properties_assertions/assertions/)).
An oracle that fails qualification remains `Open`; treatment may not edit or
replace it.

### 3. SHAKE — explore the bounded state space and preserve misses

Compile a campaign from the exact target map and property graph. Each run binds
the target, adapter, envelope/controller set, property and oracle digests,
fault/input/schedule palette, seed domain, budget, and completeness obligations.
Feed back multiple coverage signals—structural, path-class, property
reachability, state novelty, and fault lifecycle—without collapsing them into
one score.

Antithesis is the operational prior art for combining a controlled environment,
faults, reproducible timelines, state-space guidance, and executable properties
([how Antithesis works](https://antithesis.com/docs/introduction/how_antithesis_works/)).
Its documentation also says that deterministic simulation requires explicit
state-space exploration and invariants, and that retrofitting pluggable
nondeterminism is generally impractical for existing production systems
([DST overview](https://antithesis.com/docs/resources/deterministic_simulation_testing/)).
That is evidence for preserving multiple execution envelopes, not for calling
finite exploration exhaustive.

All unattempted, invalid, timed-out, diverged, or unsupported mandatory cells
remain in the coverage denominator and force `UNKNOWN`. A campaign that found
no counterexample reports only the explored boundary.

### 4. SHRINK / EXPLAIN — minimize the witness, not the claim boundary

Reduce the failing input, fault plan, schedule decisions, dependency slice, and
when appropriate the inducing diff while preserving:

- the exact finding/property fingerprint;
- target validity under the same adapter;
- the evidence grade and execution envelope; and
- replay on a fresh verifier.

Delta debugging provides the core precedent: it reduces a failure-inducing
input or change to a 1-minimal reproducer by repeated tests
([Zeller and Hildebrandt, 2002](https://www.st.cs.uni-saarland.de/papers/tse2002/)).
Grammar-aware reduction such as Perses avoids wasting most attempts on
syntactically invalid programs
([Sun et al., 2018](https://github.com/uw-pluverse/perses/tree/58e3912e198850302fd77b7ad06696bd9bf621cb)).

A minimal reproducer is not automatically a unique root cause. Causal language
requires an intervention that changes the outcome while holding the frozen
conditions fixed; otherwise the output is a ranked explanation hypothesis.

### 5. TREAT / DECLUTTER — let an untrusted worker propose a new target

Treatment receives the public map, findings, and replays. It may:

- diagnose and propose the smallest target-code patch;
- extract or strengthen target-owned contracts;
- remove dead or duplicated structure;
- isolate a consequential guard or other candidate residue; and
- request additional visible counterexamples.

It may not write the verifier, oracle, property authority, fault palette,
coverage denominator, evidence format, trust root, holdout, or campaign freeze.
A requested scope or property change is a separate external-authority proposal,
not part of the patch.

The visible repair loop may use a counterexample-guided form:

```text
candidate := synthesize(base, visible_counterexamples, frozen_contracts)
result    := visible_verifier(candidate)
if counterexample(result): add it to the visible set and iterate
else: emit TreatmentCandidate, never PROCEED
```

CEGIS alternates candidate synthesis with a verifier that returns a new
counterexample; program sketching is the foundational implementation pattern
([Solar-Lezama, 2013](https://link.springer.com/article/10.1007/s10009-012-0249-7)).
SemFix shows a repair-specific instance using tests, symbolic execution,
constraint solving, and synthesis over a complexity-layered search space
([Nguyen et al., 2013](https://research.ibm.com/publications/semfix-program-repair-via-semantic-analysis)).

But visible verification is optimization feedback, not admission. Empirical
repair research found that automatically generated patches often overfit the
training suite and break undertested behavior; patch minimization alone did not
remove that failure mode
([Smith et al., 2015](https://www.cs.cmu.edu/~clegoues/docs/smith15fse.pdf)).

“Declutter” is a claim, not a bypass. If observable behavior on any governed
channel changes, the candidate is a repair and inherits every repair
obligation. When both versions compile to a supported formal representation,
translation validation can strengthen the bounded equivalence claim; Alive2 is
useful precedent precisely because it proves refinement for a bounded LLVM IR
transformation scope and states its interprocedural and bounded limitations
([Lopes et al., 2021](https://web.ist.utl.pt/nuno.lopes/pubs.php?id=alive2-pldi21),
[Alive2 limits](https://github.com/AliveToolkit/alive2/tree/01a5ec45c8152995755f7331827407a9de19f262)).

### 6. HOSTILE RE-ENTRY — erase the refinery's authorship privilege

Every candidate restarts at `FREEZE/MAP` as a new untrusted target:

```text
TreatmentCandidate<Base, Candidate, DiffDigest>
    ──re-enter──>
Target<Candidate, NewTreeDigest, NewMapDigest>
```

Re-entry must:

- recompute source, dependency, entry-point, consequence, capability, contract,
  and evidence graphs;
- retain all prior findings, misses, exclusions, and open obligations;
- mark claimed removals as superseded rather than deleting history;
- classify any changed property, oracle, support profile, or sink taxonomy as a
  scope-change request; and
- give the candidate no “generated by Vibe Halt” trust bit.

This is the constitutional center of the refinery: its own hand produces more
vibe-coded input until a different authority validates it.

### 7. INDEPENDENT RE-SHAKE — test generalization, not replay memorization

An independent admission seat, with a separately frozen runner and oracle set,
checks:

1. the original minimal replay no longer falsifies the property;
2. all prior regressions and fixed controls retain their expected outcomes;
3. fresh seed domains and widened, mechanism-relevant fault palettes;
4. a cryptographically committed holdout unavailable to treatment;
5. property reachability and mutation sensitivity;
6. differential/metamorphic observations on governed channels;
7. no new consequential path, capability, dependency, or open-channel class;
   and
8. the candidate's exact base, revision, diff, path set, campaign, and judge
   identities.

When a holdout result is exposed to treatment, that cohort is burned for
acceptance: it may become regression/calibration evidence, but a later candidate
needs a newly frozen unseen cohort. Vibe Halt's existing holdout law already
requires independent curation, preserves every miss, freezes the candidate and
evaluation policy before reveal, and awards acceptance credit only to the first
complete pre-frozen attempt
([Build Plan, lines 63–93](https://github.com/AmitabhainArunachala/vibe-halt/blob/d19ba9e1198c0ab7ef4bb1bdf69a299d056f9754/docs/plans/VIBE_HALT_BUILD_PLAN_2026-07-20.md#L63-L93)).

### 8. DISTILL / PROVE — a code branch and an analysis branch

For high-consequence paths, treatment may propose a small guard, reference
model, state machine, or capability kernel through which the messy mass must
request the effect. That proposal is target or consequence-routing code: it is
a **new untrusted `SubjectId`** and must return to `FREEZE → MAP → CONTRACT`,
then undergo the full independent fresh/hidden re-shake. It cannot be inserted
after the re-shake that evaluated its predecessor. The new map must then show
both:

- what the residue enforces and, if available, the proof/refinement witness;
  and
- every bypass channel by which the mass could reach the effect without the
  residue.

A post-re-shake distillation step is analysis-only: it may prove or model a
residue already present in the frozen, mapped, independently re-shaken subject.
It may not add code, reroute an effect, or change a bypass surface. If it
proposes such a change, the output returns to `FREEZE` as treatment. Its proof
cannot bind admission until a separately admitted correspondence and bypass
record connects the residue to the exact surrounding subject.

A proof of the residue never proves the surrounding repository. An open bypass
forces `UNKNOWN`; a proved predicate with an ungoverned loader or deployment
path is not a settlement guarantee.

### 9. ADMIT — project evidence into a bounded decision

The admission seat consumes only signed, content-addressed records and the
frozen policy. It derives `HALT | PROCEED | UNKNOWN`; it does not consume the
repairer's narrative as proof. Product Lock v1 already defines the fail-closed
projection and says `UNKNOWN` outranks `PROCEED`
([lines 68–85](https://github.com/AmitabhainArunachala/vibe-halt/blob/d19ba9e1198c0ab7ef4bb1bdf69a299d056f9754/docs/specs/PRODUCT_LOCK_V1.md#L68-L85)).

`GOVERNABLE` and the target verdict are emitted separately so neither can be
marketed as the other.

## Typed claim shape

The smallest useful type contribution is to make accounting closure,
epistemic standing, and admission authority separate:

```text
GovernabilityProjectionPayload<
    TargetRevision,
    TreeDigest,
    SupportProfile,
    ExecutionEnvelope,
    MapDigest,
> {
    artifact_ledger: TotalAccounting<ObservedArtifact>,
    consequence_ledger: TotalAccounting<PlatformAndTenantSinkClass>,
    path_ledger: TotalAccounting<DiscoveredPathClass, PathGrade>,
    discovery_blind_spots:
      BoundedOpen<Obligation, ConsequenceBoundWitness>
      | UnboundedOpen<Obligation>,
    contract_ledger: TotalAccounting<MandatoryWorkflow, QualifiedOracle>,
    evidence_graph: ContentAddressedEvidenceGraph,
    obligation_modalities: Map<Obligation, Modality>,
}

GovernabilityGateDecision<Action, Scope, Policy> =
    RequiredAndGovernable(GovernabilityProjectionPayloadId)
  | RequiredButUngoverned(GovernabilityProjectionPayloadId, UnboundedBlindSpots)
  | NotRequired(GovernabilityProjectionPayloadId, RatifiedPolicyClauseId)

TreatmentCandidate<BaseRevision, CandidateRevision, DiffDigest> {
    affected_path_set: PathSetDigest,
    visible_counterexamples: EvidenceSetDigest,
    writer: TreatmentIdentity,
    authority_to_admit: Never,
}

IndependentRevalidation<CandidateRevision, CampaignId, ValidatorIdentity> {
    public_regression: Outcome,
    fresh_universes: Outcome,
    hidden_holdout: Outcome,
    behavior_delta: AccountedDelta,
    new_open_channels: LedgerDelta,
}

Admission<Action, TargetRevision, MapDigest, EvidenceClosureId,
          GovernabilityProjectionPayloadId,
          GovernabilityGateDecision<Action, Scope, Policy>,
          PolicyDigest, AdmissionRecordId,
          AdmissionQuorumWitness<Policy>> =
    Assessment<Action>::HALT(FindingSet)
  | Assessment<Action>::PROCEED(BoundedObligationsSatisfied)
  | Assessment<Action>::UNKNOWN(OpenObligationSet)
```

There is no constructor from `TreatmentCandidate`, a signature, a scalar score,
model consensus, or human authority directly to a positive governability
projection or
`Assessment<Action>::PROCEED`. If bounded unknowns are ratified as compatible,
the pure projection evaluator may construct `GOVERNABLE` only after verifying
total accounting over the frozen
observed universe and a controller/non-reachability witness for every remaining
blind spot. Scope authority cannot mint that witness. The projection contains
no admission signature; the outer admission quorum signs the assessment,
projection, and gate decision together, avoiding a self-referential quorum.
Accounting closure does
not globally promote heterogeneous proposed, observed, and replayed obligations
to `Replayed`; authority changes never improve their modalities.

## Measurable admission criteria

The measurements are a vector; no weighted sum may offset an identity,
coverage, independence, or open-channel failure.

| Dimension | Required evidence | Fail-closed interpretation |
|---|---|---|
| Subject identity | exact base/candidate tree, dependency, build, adapter, envelope and policy digests | any mismatch invalidates the attempt |
| Artifact accounting | `accounted observed artifacts / observed artifacts = 100%`; each item included, scoped-out, or open | this is closure over the observed set, not proof that discovery found every artifact |
| Consequence accounting | every platform-mandatory and tenant-declared sink class present; every discovered path class graded | a missing mandatory class or unbounded blind spot blocks `GOVERNABLE` |
| Contract accounting | every mandatory workflow/path has a revision-bound property, authority, modality, oracle, and oracle-qualification record | conflict, absent oracle, or unqualified inferred invariant forces `UNKNOWN` |
| Exploration | structural/path/property/fault-lifecycle coverage, reached sometimes-properties, exhausted frozen budget, misses retained | raw line coverage or universe count cannot substitute for semantic reach |
| Finding evidence | exact one-command replay and fingerprint-preserving reduction; representative median shrink tracked | a prose diagnosis is not a finding witness |
| Repair generalization | original replay repaired; public regressions, fresh universes, and unseen holdout pass; no unexplained governed behavior delta | visible-suite success alone is non-admissible |
| Independence | treatment cannot read/write judge, oracle, freeze, trust root, or holdout; distinct attributable identities | overlap or leakage invalidates the campaign, not merely its score |
| Declutter | complexity delta reported beside observed trace/refinement equivalence and map delta | fewer files/LOC never compensates for changed behavior or lost coverage |
| Utility | independently confirmed severity-weighted yield, wall time, spend, operator time, and changed real decisions | prose volume, candidate count, and self-reported fixes receive no credit |

`PROCEED` has the stricter predicate: every mandatory check must be `Governed`,
complete, replayable, and finding-free under the frozen policy. If human gate
[#112](https://github.com/AmitabhainArunachala/vibe-halt/issues/112) ratifies
bounded-unknown compatibility, `GOVERNABLE` may carry only
`BoundedOpen<Obligation, ConsequenceBoundWitness>` obligations and therefore an
action-typed `UNKNOWN`; under every option it may not carry an `UnboundedOpen`
consequence. This research does not select that compatibility rule.

The existing external-evaluation bar—at least 80% recall over an independently
curated, preregistered `N >= 25` real-defect holdout spanning at least five
repositories and five mechanism clusters—remains an engine/product acceptance
criterion, not proof that one repaired repository is correct
([Build Plan, criterion](https://github.com/AmitabhainArunachala/vibe-halt/blob/d19ba9e1198c0ab7ef4bb1bdf69a299d056f9754/docs/plans/VIBE_HALT_BUILD_PLAN_2026-07-20.md#L20-L35),
[measurement contract](https://github.com/AmitabhainArunachala/vibe-halt/blob/d19ba9e1198c0ab7ef4bb1bdf69a299d056f9754/docs/plans/VIBE_HALT_BUILD_PLAN_2026-07-20.md#L69-L84)).

## Anti-Goodhart controls

Freeze these before treatment sees findings:

- target/support/envelope identities and execution budgets;
- mandatory consequence taxonomy, workflows, properties, severity weights,
  oracle qualification, and blocking policy;
- map and coverage denominators, including all exclusions and misses;
- seed-domain partitioning and fault-palette families;
- holdout commitment, curator, judge, and one-shot acceptance rule;
- equal-budget baselines and root-cause deduplication; and
- complexity and behavior-preservation metrics used for declutter claims.

Then enforce the following invariants:

1. **Denominators only grow append-only.** Deleting a test, sink, entry point,
   artifact, finding, miss, or unsupported surface creates a visible removal
   record and requires external scope authority.
2. **Metric improvement may be suspicious.** A sudden coverage or pass-rate
   jump accompanied by fewer properties, narrower palettes, disabled
   instrumentation, unreachable assertions, or new exclusions invalidates the
   comparison.
3. **No repairer-chosen judge.** Treatment cannot select trust roots, holdout
   cohorts, seeds, or the only revalidation engine.
4. **No training on acceptance.** A revealed holdout becomes non-credit
   calibration; the next acceptance candidate gets a new unseen cohort.
5. **No complexity barter.** Smaller code cannot offset a new capability,
   behavior delta, weaker oracle, or lost path coverage.
6. **No scalar cleanliness score.** Publish the full vector, nulls, invalids,
   misses, open obligations, and confidence/grade boundaries.
7. **No contract laundering.** Model-generated or trace-inferred properties
   retain `Proposed` modality until an appropriate witness promotes them.
8. **Adversarial map audit.** A decorrelated mapper and post-treatment map-diff
   search specifically for hidden entry points, sinks, and bypasses.

QuickCheck's original property-testing model—generate many inputs from stated
properties and report counterexamples—is useful, but it does not turn the
tested property or sample into a proof
([Claessen and Hughes, 2000](https://doi.org/10.1145/351240.351266)).
The anti-Goodhart design therefore measures both the tests and whether the
properties were meaningfully reached.

## Limits that remain `UNKNOWN`

- No finite static/dynamic analysis establishes complete behavior for arbitrary
  software. Dynamic imports, reflection, `eval`, JITs, native extensions, FFI,
  generated code, plugins, and self-modification widen the blind spot.
- A dependency/SBOM graph does not prove which bytes were loaded or what an
  external service did.
- Static source→sink analysis can be imprecise or unsupported; runtime coverage
  can only describe executions that occurred.
- Tests, inferred invariants, differential references, metamorphic relations,
  and mutation scores can all share the same mistaken specification.
- Hidden holdouts estimate generalization for a frozen population; they do not
  prove correctness and become training data after disclosure.
- Behavior preservation is only over named governed observations unless a
  stronger formal refinement witness exists.
- A proved residue says nothing about bypass paths, compiler/runtime/hardware
  assumptions, operators, or physical-world oracles outside its theorem.
- An `Open` channel described in a receipt is honesty, not containment. Current
  Vibe Halt records all 29 subprocess capability channels as open and therefore
  cannot turn that run into D1 evidence
  ([README, lines 65–74](https://github.com/AmitabhainArunachala/vibe-halt/blob/d19ba9e1198c0ab7ef4bb1bdf69a299d056f9754/README.md#L65-L74)).
- Signatures and content addresses attribute and bind evidence; they do not
  validate the oracle, close execution channels, or confer authority.

The honest end state for a supported target is therefore one of:

1. a revision-bound candidate that independently survived the frozen campaign;
2. `HALT` with a minimal replay and retained map;
3. `UNKNOWN` with exact open obligations and missing witnesses; or
4. `UNSUPPORTED`/`UNGOVERNED` when the refinery cannot bound its own blind spot.

## Staged delivery wedges

These are dependency-ordered proofs, not a promise to ship every adapter:

| Wedge | Smallest honest proof | What it still cannot claim |
|---|---|---|
| R0 — Whole-target ledger | one exact Dharma CPython path produces the five maps, coverage ledger, and typed target verdict | no autonomous repair, no repository-wide determinism |
| R1 — Closed treatment loop | one predeclared Dharma `FaultClassId`/`FindingId` is found and shrunk, then an exact human-supplied `RepairClaimId` candidate in a disposable tree is remapped and independently re-shaken against the original replay and hidden class | no autonomous repair claim, no previously unknown fault, no general repair efficacy |
| R2 — Generalization | prospective candidates face fresh universes and a one-shot hidden holdout with treatment/judge isolation | no correctness proof; population remains frozen and bounded |
| R3 — Declutter | one behavior-preserving simplification or consequential guard extraction survives map-diff and independent behavior/refinement checks | no whole-repository equivalence |
| R4 — Supported-repo refinery | multiple repositories in one explicit runtime profile meet preregistered time, coverage, overfit, and yield criteria | no “any language/any repository” claim |
| R5 — Proved residue | one small consequential residue has a machine-checked theorem and closed bypass ledger at its claimed envelope | no proof of the vibe mass or external world |

R0/R1 are the first campaign's constitutional proof. Scaling adapter breadth
before those close would optimize intake while the evidence spine is still
unproven.

## Kill and revival falsifiers

The exact economic budgets and thresholds remain operator-ratified campaign
inputs. The architecture can still name categorical and prospective falsifiers:

| Hypothesis | Kill / reorient when | Preserved asset | Revival falsifier |
|---|---|---|---|
| Whole-target maps bound consequential uncertainty | any admitted `GOVERNABLE` target later yields one confirmed platform-mandatory consequence path that was neither graded nor represented by an open blind spot; repeated misses across three preregistered supported targets kill broad-map claims | evidence kernel, findings, adapter observations | a default-deny consequence interlock or repaired adapter taxonomy catches the withheld bypass prospectively on a new target set |
| The refinery adds value beyond Antithesis-style finding | across three preregistered equal-budget campaigns, independent repair success/severity-weighted yield does not beat the frozen baseline or treatment cost exceeds the ratified ceiling | MAP/SHAKE/SHRINK product | a redesigned treatment loop wins a new unseen campaign without weaker oracles or scope |
| Repair generalizes | holdout overfit, new behavior regressions, or new open-channel rate exceeds the frozen bound; any hidden-cohort leak invalidates the attempt immediately | public counterexamples become regression assets | fresh architecture and a new unseen cohort pass prospectively; revealed cohorts remain calibration-only |
| Declutter preserves behavior | any unfiled governed-channel behavior change occurs | candidate is relabeled as repair and evidence retained | new candidate passes trace/refinement comparison and the independent holdout |
| The graph scales economically | one supported real target cannot complete FREEZE→MAP→receipt inside its preregistered budget, or three targets require open-ended manual ontology repair | narrow language adapter and source facts | narrower support profile completes prospectively under a new frozen budget before breadth resumes |
| Writer/judge separation is mechanically real | treatment can modify or select a judge/oracle/freeze/trust root, read the holdout, or cause its output to skip hostile re-entry | invalidate the whole campaign, rotate compromised material, preserve public evidence | isolation test demonstrates denied access and a fresh end-to-end campaign succeeds |
| Residue proof governs consequences | any target path bypasses the proved guard at the claimed envelope | theorem remains valid only for its exact residue | controller evidence closes every bypass class, or scope is honestly narrowed and re-ratified |

The existing Product Lock already supplies the broader economic falsifier: if
three preregistered equal-budget target tournaments show no advantage in
confirmed severity-weighted yield, pause expansion and pivot the engine
portfolio while preserving the evidence kernel
([lines 147–156](https://github.com/AmitabhainArunachala/vibe-halt/blob/d19ba9e1198c0ab7ef4bb1bdf69a299d056f9754/docs/specs/PRODUCT_LOCK_V1.md#L147-L156)).

## Decision frontier left for humans

This research supports but does not answer the open constitutional decisions:

- which exact consequence classes and bounded-open rule construct
  `GOVERNABLE`;
- whether a repository can be `GOVERNABLE + UNKNOWN`, or whether the public
  label should be reserved for fully mediated consequence paths;
- the first proof court and residue language;
- the first campaign's budgets, holdout size, independence topology, and
  economic thresholds;
- which scope authority may exclude a non-mandatory path, for how long, and how
  appeal works; and
- whether `GOVERNABLE` attaches to a whole tree, one deployment artifact, one
  workflow, or only the atomic admission object chosen elsewhere in the map.

## Bottom line

The refinery is possible if it stops promising to understand the forest and
instead makes every known road to consequence appear on a signed, revision-
bound map—together with every place the mapmaker could not see. It earns its
difference from Antithesis by proposing treatment and declutter; it preserves
Antithesis's epistemic value by treating every proposal as hostile input and
making an independent shaker rediscover the right to proceed.

`GOVERNABLE` is not “the mess is clean.” It is “the mess can no longer bind
without leaving an evidence-graded path through the ledger.”
