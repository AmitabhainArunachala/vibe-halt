# Falsification and economic kill matrix

Research ticket: [Build the falsification and economic kill matrix](https://github.com/AmitabhainArunachala/vibe-halt/issues/106)

Accepted Vibe Halt base: `d19ba9e1198c0ab7ef4bb1bdf69a299d056f9754`.

## Verdict

Vibe Halt should not have one success score. It needs a sequence of typed
investment gates whose raw measurements remain visible. A stage advances only
when its own predeclared falsifiers survive; success at a later-looking metric
cannot forgive a broken evidence invariant at an earlier stage.

The current product law already contains most of the hard skeleton: exact
target identity, fail-closed `UNKNOWN`, equal-budget comparison, independent
confirmation, retained misses, an unseen holdout, real-target proof, replay,
shrink, and explicit kill conditions. The constitution should preserve these,
add repair/refinery measurements, and replace any temptation to optimize a
single composite number with a Pareto report over severity, money, wall time,
operator time, invalid claims, and coverage boundaries.

The gates operate over distinct types; they must not overload one green/red
field:

```text
AttemptValidity = Valid | Invalid(IntegrityViolation)
Assessment<Action> = HALT | PROCEED | UNKNOWN
Governability = GOVERNABLE(BoundaryWitnesses) | UNGOVERNED
StageDecision = Advance | Hold | Reorient | Kill | Invalid
```

An integrity-compromised attempt is `Invalid`, not a product `HALT` or a
successful `UNKNOWN`. A valid `HALT | PROCEED | UNKNOWN` assessment is typed by
the action it grades. Governability is orthogonal to that assessment. A stage
investment decision consumes those objects but is not any one of them.

## Current primary-source law

- `PROCEED` is bounded to the recorded revision, checks, properties, fault
  model, budget, and evidence grade; unresolved mandatory areas are `UNKNOWN`,
  which outranks `PROCEED` ([Product Lock v1, lines 68–85](https://github.com/AmitabhainArunachala/vibe-halt/blob/d19ba9e1198c0ab7ef4bb1bdf69a299d056f9754/docs/specs/PRODUCT_LOCK_V1.md#L68-L85)).
- Product Lock defines `confirmed_yield` from preregistered severity-weighted,
  independently confirmed, root-cause-deduplicated faults under equal budget;
  unconfirmed findings earn zero credit ([lines 87–108](https://github.com/AmitabhainArunachala/vibe-halt/blob/d19ba9e1198c0ab7ef4bb1bdf69a299d056f9754/docs/specs/PRODUCT_LOCK_V1.md#L87-L108)).
- The first product proof is one exact Dharma target through map, coverage,
  attack, evidence, replay, and a target decision, plus one important confirmed
  fault missed by all equal-budget AI baselines ([lines 123–135](https://github.com/AmitabhainArunachala/vibe-halt/blob/d19ba9e1198c0ab7ef4bb1bdf69a299d056f9754/docs/specs/PRODUCT_LOCK_V1.md#L123-L135)).
- Failure to traverse the first target in six weeks stops breadth; no yield
  advantage across three preregistered eligible tournaments pauses expansion
  and forces a human reorientation decision ([lines 147–156](https://github.com/AmitabhainArunachala/vibe-halt/blob/d19ba9e1198c0ab7ef4bb1bdf69a299d056f9754/docs/specs/PRODUCT_LOCK_V1.md#L147-L156)).
- The acceptance denominator is an independently curated, preregistered,
  candidate-secret holdout of at least 25 real AI-authored defects, retaining
  misses and spanning at least five repositories and five mechanism clusters;
  acceptance is at least 80% recall on the first frozen execution
  ([Build Plan, lines 63–89](https://github.com/AmitabhainArunachala/vibe-halt/blob/d19ba9e1198c0ab7ef4bb1bdf69a299d056f9754/docs/plans/VIBE_HALT_BUILD_PLAN_2026-07-20.md#L63-L89)).
- Existing technical criteria also require three previously unknown
  human-confirmed defects, representative median shrink of at least 90%, at
  least 1,000 Tier-1 universes/hour on a named box, and one Dharma adapter
  receipt ([Build Plan, lines 27–35](https://github.com/AmitabhainArunachala/vibe-halt/blob/d19ba9e1198c0ab7ef4bb1bdf69a299d056f9754/docs/plans/VIBE_HALT_BUILD_PLAN_2026-07-20.md#L27-L35)).
- Known published defects can validate mechanism transfer but cannot count as
  unknown discoveries ([VISION, lines 121–140](https://github.com/AmitabhainArunachala/vibe-halt/blob/d19ba9e1198c0ab7ef4bb1bdf69a299d056f9754/VISION.md#L121-L140)).

## Absolute constitutional invariants

These are not metrics to average. One observed violation invalidates the run
and caps the corresponding capability until repaired and independently
replayed.

| Invariant | Zero-tolerance falsifier | Terminal consequence |
|---|---|---|
| Evidence identity | Tampered, stale, wrong-revision, wrong-envelope, or non-canonical evidence is accepted | `INVALID`; revoke the affected verifier/admission grade |
| Mandatory-channel honesty | A mandatory open, divergent, unsupported, or unobserved channel yields `PROCEED` | `KILL_GRADE`; no result under that policy may bind |
| Writer/judge separation | Treatment selects its judge/trust root, edits judge-owned surfaces, sees hidden holdouts, or certifies its own output | `INVALID`; discard candidate and rotate compromised holdout/key material |
| Exact mutation binding | Evidence for base/candidate/diff A authorizes B | `KILL_ADMISSION`; first-campaign constitution falsified |
| Holdout integrity | Cohort, seeds, oracle, or eligibility leaks to a system allowed to adapt before the credit run | `INVALID_ATTEMPT`; revealed cohort becomes calibration-only |
| Miss retention | A missed or unsupported eligible case disappears from the denominator | `INVALID_BENCHMARK` |
| Claim boundary | A local fixture, model, port, or cooperative envelope is reported as evidence for an unmodified foreign/native target | `KILL_CLAIM`; correction must be append-only |
| Authority/modality orthogonality | Authority, operator override, or signature alone promotes `UNKNOWN` or constructs proof | `KILL_CONSTITUTION`; no exception path |

## Stage gates

### Gate 0 — Evidence kernel remains truthful

**Run/use/prove:** full gate at the accepted SHA; frozen reference identities;
tamper, parser, replay, divergence, and open-channel negatives.

**Advance when:** all named mechanical gates pass and every unsupported case
fails closed with a typed reason.

**Kill/reorient when:** any silent acceptance occurs. Stop reach, repair, and
court expansion until the truth substrate is restored. A green higher layer
cannot compensate.

### Gate 1 — Whole-target path exists

**Measurements:** exact revision bound; manifest parsed; target map and coverage
plan emitted before findings; attack actually attempted; coverage ledger
retains every mandatory unknown; independent replay attempted; one target-level
decision emitted.

**Advance when:** one cleanly materialized Dharma revision completes the entire
path, even if the honest terminal decision is `HALT` or `UNKNOWN`.

**Kill/reorient when:** the six-week Product Lock timer expires without one
end-to-end target. Stop adding engines, languages, proof courts, or theaters and
repair only intake-to-receipt.

### Gate 2 — First treatment/admission campaign discriminates

**Measurements:** one frozen `FaultClassId` and `FindingId` is found, replayed,
and shrunk on the known-bad baseline; the human-supplied fixed control binds a
`RepairClaimId` to that same finding, clears the original replay and hidden
mechanism class without false blocking; exact mutation identity survives replay;
treatment cannot alter judge-owned surfaces; fresh hidden seeds and widened
palettes are used; candidate re-enters at `MAP`; all terminal receipts verify
independently.

**Advance when:** the campaign distinguishes the identity-coupled
baseline/fixed/rebind arms and returns a truthful action-typed assessment for the
real Dharma path. Initial D2 advancement does not require or permit a review
token.

**Kill/reorient when:** a replay-specific patch passes while the held-out fault
class still fails; an unrelated candidate consumes the permit; or the repair
agent can influence its judge. This kills autonomous-treatment admission, not
the evidence kernel.

### Gate 3 — The refinery makes a target `GOVERNABLE`

Report raw before/after values, never one “cleanup score”:

- consequential paths declared / mapped / mandatory-unknown;
- controlled, open, and absent capability channels by class;
- executable properties and independent oracles per consequential path;
- unique minimized counterexamples and replay success;
- repair candidates accepted/rejected/unknown on hidden revalidation;
- trusted-residue surface: files, dependencies, interfaces, privileged effects;
- behavioral equivalence observations for declared declutters;
- human review minutes and unexplained decision count.

**Advance when:** every declared, discovered, and platform-mandatory
consequential path class in the frozen accounting set is mapped to a property
and channel grade; discovery blind spots remain in a bounded/unbounded ledger;
no `UnboundedOpen` blind spot can reach the declared consequence; and every
accepted repair passed independent hidden revalidation. Human gate
[#112](https://github.com/AmitabhainArunachala/vibe-halt/issues/112) decides
whether a structurally bounded `UNKNOWN` may advance this gate; until ratified,
it produces `Hold(UnknownObligations)`, not `Advance`.

**Kill/reorient when:** “governable” improves by deleting scope after reveal,
adding tautological properties, editing oracles, suppressing unknowns, or
reducing lines while expanding the trusted effect surface. Preserve the target
map and pivot to the specific opacity source; do not claim universal repair.

### Gate 4 — Real-fault advantage exists

**Primary raw measures:**

1. independently confirmed severity-weighted unique faults;
2. USD spend;
3. wall-clock minutes;
4. human/operator minutes;
5. time to first confirmed fault;
6. invalid claims / all externally reported claims;
7. independent replay successes / replay attempts;
8. faults unique to Vibe Halt and faults unique to each baseline;
9. whether each finding changed a real merge/release decision.

Keep Product Lock's `confirmed_yield` for continuity, but never publish it
alone. Multiplying dollars by wall time can reward odd operational choices and
hide whether a result is cheap-but-slow or fast-but-expensive. Publish the raw
vector and Pareto comparisons under identical ceilings.

**Advance when:** the first Dharma tournament yields at least one important
confirmed reproducible fault missed by all preregistered decorrelated AI
reviewers, then the frozen `N >= 25` holdout reaches at least 80% recall on its
first eligible run.

**Kill/reorient when:** three eligible equal-budget tournaments show no
severity-weighted yield advantage. Preserve the evidence kernel, publish which
fault classes baselines won, and require human ratification before broader
reach or category investment.

### Gate 5 — Antithesis-grade investment earns its name

**Measurements:** exact real artifacts supported; controlled nondeterminism
classes; deterministic replay rate by environment; divergence numerator and
denominator; search advantage over frozen uniform/random baselines; named-box
throughput; shrink ratio distribution; host compatibility; overhead; unknown
syscalls/effects; previously unknown confirmed faults.

**Advance when:** the named target profile meets its predeclared control and
replay contract on two evidence hosts, reaches at least 1,000 Tier-1
universes/hour where that criterion applies, and guided search beats its frozen
baseline on a discriminating holdout rather than a saturated demo.

**Kill/reorient when:** the reach spike cannot close its mandatory channels,
cannot replay the exact workload across the two hosts, or search fails its
predeclared advantage threshold. Retain cooperative D2 and recorded nulls;
revive only when the failed environmental or instrumentation condition changes.

### Gate 6 — Settlement use remains non-sovereign

**Measurements:** independent organizations/judges represented; signing and
revocation latency; public-log inclusion and consistency; appeal outcomes;
override claims by reason and later disposition; fraction of acts blocked by
mandatory unknowns; false admission incidents; availability under judge loss;
concentration of key and policy authority.

**Advance when:** one real external consequence gate consumes a bounded Vibe
Halt decision without granting Vibe Halt merge/deployment authority, and an
independent party can verify, replay, and challenge the supporting evidence.
Only an authorized policy-bound role may append a revocation or supersession,
and it must preserve the public history; challenge alone has no direct modality
or credential effect.

**Kill/reorient when:** the operator, signer, token vote, or product owner can
silently turn `UNKNOWN` into permission; the same authority controls treatment,
holdout, judge, and amendment; or a verifier outage becomes de facto sovereign
veto without an explicit external governance process.

## Anti-Goodhart protocol

Before every credit-bearing campaign freeze:

1. target identity, eligible population, properties, fault model, budgets,
   severity weights, confirmation owner, baselines, and terminal mapping;
2. every denominator and the rule retaining misses/errors/unknowns;
3. independent curator, treatment, judge, and confirmation roles;
4. opaque commitment to hidden cases/seeds plus post-run authenticated reveal;
5. exact code, executable, protocol, policy, and environment identities;
6. root-cause deduplication and diversity caps;
7. prohibited post-reveal adaptations and calibration-only rerun rules;
8. raw result publication, including nulls and invalid attempts;
9. one explicit adversarial falsification of any green result; and
10. the threshold that yields `ADVANCE`, `HOLD`, `REORIENT`, `KILL`, or
    `INVALID`.

## Decisions still requiring the operator

- Whether the first campaign's six-week clock restarts for the constitution or
  remains inherited from Product Lock v1.
- Numerical repair holdout, regression, operator-time, and trusted-residue
  thresholds; current law does not supply honest values.
- The number and independence grade of external settlement judges.
- Product willingness-to-pay and retention thresholds. Repository evidence can
  measure engineering value, not decide a market without real users.

## Small typed contribution

Do not encode investment status as prose attached to a green receipt:

```text
StageDecision<Stage, FrozenContract, EvidenceSet> =
    Advance
  | Hold(UnknownObligations)
  | Reorient(FalsifiedHypothesis, PreservedKernel)
  | Kill(ViolatedInvariant)
  | Invalid(CompromisedAttempt)
```

No constructor from a product verdict, signature, model consensus, or operator
assertion directly to `Advance` should exist. Promotion consumes the frozen
contract and recomputed evidence set; authority may ratify scope but cannot
repair missing modality.
