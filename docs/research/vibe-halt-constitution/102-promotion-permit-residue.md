# The smallest useful proved `PromotionPermit` residue

Research resolution for [Choose the smallest proved PromotionPermit residue](https://github.com/AmitabhainArunachala/vibe-halt/issues/102).

**Repository basis:** accepted Vibe Halt `main` at
`d19ba9e1198c0ab7ef4bb1bdf69a299d056f9754`

**Research date:** 2026-08-22

**Scope:** architecture and proof obligations only; no proof or product
implementation was attempted

## Verdict

The first proved residue should be a **pure, total, fail-closed Rust decision
function** whose only successful output means:

> this exact candidate delta, under this exact frozen campaign and independently
> admitted evidence, may enter human review.

It must not mean “correct,” “safe,” “merge,” “deploy,” or even “the diff repairs
the intended bug class.” Those are not the theorem. The useful theorem is much
smaller: **a review-candidacy token cannot be minted for a different subject,
campaign, path set, action, or admission quorum, and cannot be minted from `HALT`, `UNKNOWN`, an
open mandatory obligation, or an unrecognized input state.**

The provisional court recommendation for the still-open human decision is
**Verus over an isolated safe-Rust subset**. It is the shortest path to proving
the executable predicate Vibe Halt would actually call, without inserting a
second-language compiler or a hand-maintained model-to-Rust equivalence gap.
The residue is intentionally small enough that a later, decorrelated Lean model
plus exhaustive differential testing can cross-check it in the Cedar pattern.
Isabelle/HOL remains the stronger later court for an isolation kernel; it is not
proportionate to this first finite decision algebra.

This is a recommendation to [the proof-court grilling decision](https://github.com/AmitabhainArunachala/vibe-halt/issues/109),
not a ratification of it.

## Why this is the right cut

Current Vibe Halt already contains three pieces of the shape:

- exact revision binding and authority/modality separation in
  [`modality.rs`](https://github.com/AmitabhainArunachala/vibe-halt/blob/d19ba9e1198c0ab7ef4bb1bdf69a299d056f9754/crates/vh-cli/src/modality.rs#L1-L9);
- a private, non-`Clone`, non-`Default` fresh-run witness whose fields are
  available only after canonical verification and fresh execution in
  [`bundle.rs`](https://github.com/AmitabhainArunachala/vibe-halt/blob/d19ba9e1198c0ab7ef4bb1bdf69a299d056f9754/crates/vh-cli/src/bundle.rs#L914-L934); and
- a closed admission matrix that rejects mismatched engine, target, condition,
  oracle, budget, and fault-plan facts in
  [`admission.rs`](https://github.com/AmitabhainArunachala/vibe-halt/blob/d19ba9e1198c0ab7ef4bb1bdf69a299d056f9754/crates/vh-cli/src/admission.rs#L307-L387).

But that admission controller is deliberately one fixed faulty/control demo. It
does not bind a real repair candidate to its base revision, diff, consequential
paths, campaign, action, and ratified admission quorum. The residue should generalize only that
binding and non-escalation law—not attempt to prove the Dharma repository.

This also preserves Product Lock v1: a future repair arm cannot certify its own
patch, and the builder—not Vibe Halt—uses a bounded receipt in a merge or
release decision ([lines 31–49](https://github.com/AmitabhainArunachala/vibe-halt/blob/d19ba9e1198c0ab7ef4bb1bdf69a299d056f9754/docs/specs/PRODUCT_LOCK_V1.md#L31-L49)).

## Exact residue

The eight named parameters are **indices**, not prose labels:

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

Each digest/id is a distinct fixed-size newtype. None is a free-form `String`,
and none can be implicitly converted to another.
`AdmissionQuorumWitness<Policy>` is not “judge id differs from repairer id”; it
proves that the exact attestation set satisfies the frozen authority policy and
that treatment has zero weight for its own candidate. The policy may require one
mechanically separate judge, 2-of-2, or another threshold. Which topology and
people satisfy it remains a human constitutional decision.

The smallest externally serializable record is:

```text
PromotionPermitV1 {
  schema_domain: "vh-review-candidacy-permit-v1",
  capability: HumanReviewCandidate,       // the only inhabitant
  base_revision: BaseRevision,
  candidate_revision: CandidateRevision,
  diff_digest: DiffDigest,
  path_set_digest: PathSetDigest,
  materialization_receipt_id: MaterializationReceiptId,
  campaign_id: CampaignId,
  action: HumanReview,
  governability_projection_payload_id: Digest,
  governability_gate_decision: GovernabilityGateDecision,
  governability_gate_witness_id: Digest,
  admission_quorum_policy_id: PolicyId,
  admission_payload_id: Digest,
  admission_record_id: Digest,
}
```

`CampaignSpecId` first commits to the frozen property/oracle contract,
execution-envelope and controller-set identities, required evidence contract and
schemas, seed-domain policy, budgets, policy version, and authority view—without
any holdout commitment. Pool and selector commitments bind that spec. Final
`CampaignId` is then derived from `CampaignSpecId` plus the ordered holdout
commitment statement identities. It must not commit to produced evidence or its
own future closure.

`admission_payload_id` binds the exact evidence closure, action, assessment,
pure governability projection, action-specific governability-gate decision,
policy, and quorum-policy id. Authorized judges sign identical payload bytes;
only afterward does `admission_record_id` bind that payload plus the ordered
attestation set. `AdmissionQuorumWitness<Policy>` is derived by verifying the
record and is never inside the bytes it verifies. These transitive commitments
must be checked at consumption time; their presence in a hash does not make
their contents true.

`GovernabilityGateDecision<Action, Scope, Policy>` is either
`RequiredAndGovernable(projection)` or
`RequiredButUngoverned(projection, unbounded_blind_spots)` or
`NotRequired(projection, ratified_policy_clause)`. It is pure input to the
signed payload, not an authority witness. Only verification of the completed
admission record derives `GovernabilityGateWitness`; this avoids placing the
same quorum's future signatures inside the bytes it signs. The residue does not
choose which decision is lawful for human review—that remains #112/#114—but it
cannot omit or silently default the decision.
`RequiredButUngoverned` remains attributable signed `HALT`/`UNKNOWN` evidence
and can never derive a gate witness or permit.

`MaterializationReceiptId` binds the canonical base, diff, candidate tree,
path set, clean-state checks, materializer role, and immutable snapshot digest
for the audited execution. It does not grant authority over a filesystem path.
A byte-identical independent rematerialization is a new signed observation; a
permit remains review eligibility for immutable candidate content, never a
workspace-mutation token.

The internal constructor consumes checked facts rather than caller-authored
phantom witnesses:

```text
issue_review_permit(
  frozen: FrozenCampaignFacts,
  admission: AuthenticatedAdmissionFacts,
  current_authority_view: AuthorityView,
) -> Result<PromotionPermit<...>, PermitReject>
```

All record fields are private. There is no raw constructor, `Default`, lossy
deserializer, permissive unknown-field path, or conversion from a receipt string.
The only capability value is `HumanReviewCandidate`. There is deliberately no
`Merge`, `Deploy`, `Settle`, `Waive`, or `OverrideUnknown` variant, and no
`From<PromotionPermit>` implementation for an external-action token.

The word “promotion” therefore means **promotion from untrusted repair output to
a candidate that a human is allowed to review**. If that name repeatedly causes
consumers to infer merge authority, rename the public object
`ReviewCandidacyPermit`; preserving semantics matters more than preserving the
working title.

## The theorem suite

Let `issue(F, A, V) = Ok(P)`. The first proof should establish all and only the
following.

### 1. Exact-subject preservation

```text
P.base      = F.base
P.candidate = F.candidate
P.diff      = F.diff
P.paths     = F.paths
P.materialization = F.materialization
P.campaign  = F.campaign
P.action    = HumanReview
P.governability_projection = A.governability_projection
P.governability_gate_decision = A.governability_gate_decision
P.governability_gate_witness_id = A.governability_gate_witness_id
P.admission_quorum_policy_id = A.quorum_policy
P.admission_payload_id = A.payload_id
P.admission_record_id = A.record_id
```

Every corresponding field in `A`, its signed statement, the evidence manifest,
and `F` must agree. The checker never selects “the latest” revision, infers a
missing path set, or accepts a same-repository substitute.

### 2. Admission preconditions

```text
A.assessment = Assessment<HumanReview>::PROCEED
and A.mandatory_open_obligations = empty
and A.campaign = F.campaign
and A.policy = F.policy
and A.authority_view = V.digest
and governability_gate_valid_for(
      HumanReview, F.scope, F.policy,
      A.governability_projection,
      A.governability_gate_decision,
      A.record_id)
and authorized_quorum(V, F.policy, A.attestation_set)
and independent_under(V, F.policy, A.attestation_set, F.treatment_identity)
and all_not_revoked(V, A.attestation_set)
and authenticated(A)
```

`authorized_quorum` and `independent_under` evaluate the frozen policy. The
proof does not choose that policy and cannot prove social or organizational
independence from names alone.

### 3. Fail-closed completeness

The function is total and panic-free. Missing, malformed, duplicate,
unrecognized, stale, revoked, mismatched, `HALT`, and `UNKNOWN` inputs return a
typed rejection. In particular:

```text
issue(... Admission<HumanReview, UNKNOWN> ...) != Ok(_)
issue(... Admission<HumanReview, HALT> ...)    != Ok(_)
```

No default enum arm maps future schema values to `PROCEED`; a new value makes
the old checker reject.

### 4. Non-escalation

```text
P.capability = HumanReviewCandidate
```

Issuance preserves the evidence modality and authority grade carried by the
admission. It cannot construct `Proven`, alter `UNKNOWN`, or create an external
disposition. This is the residue-level analogue of current
[`AUTHORITY_CANNOT_LIFT_MODALITY_V1`](https://github.com/AmitabhainArunachala/vibe-halt/blob/d19ba9e1198c0ab7ef4bb1bdf69a299d056f9754/crates/vh-cli/src/modality.rs#L92-L138).

### 5. Determinism and non-ambiguity

For equal structured inputs, issuance returns equal outputs. For any two
successful permits, equality implies equality of every explicit index and the
admission-attestation digest. Canonical serialization is domain-separated and
round-trips without omission or reordering.

This is an injectivity theorem about the structured encoding, not about SHA-256.
Collision resistance remains a cryptographic assumption at the evidence
boundary.

### Compact statement

The core result can be summarized as:

```text
IssueSound:
  issue(F, A, V) = Ok(P)
  -> ExactBinding(P, F, A)
  and AdmissionEligible(A, F, V)
  and Capability(P) = HumanReviewCandidate
  and PreservesModalityAndAuthority(P, A)

IssueComplete:
  WellFormed(F, A, V)
  and AdmissionEligible(A, F, V)
  -> exists P, issue(F, A, V) = Ok(P)

NoConsequenceConstructor:
  there is no total function in the residue API
  PromotionPermit<...> -> MergeOrDeployAuthority
```

`IssueComplete` is completeness relative to the frozen policy, not a claim that
the policy is wise or the campaign is sufficient.

`NoConsequenceConstructor` is an **API-confinement theorem**, not a whole-system
theorem that consumers cannot reinterpret bytes or ignore the library. The
production boundary must additionally seal raw constructors, carry compile-fail
capability tests, fuzz parser/authentication adapters, mutate every issuance
conjunct, and run an end-to-end consequence-router negative that rejects permit
bytes at merge and deploy endpoints.

## Proof, DST, and external trust are three different jobs

Trying to place every obligation in the proof would recreate the whole vibe
mess inside the court. Trying to place every obligation in DST would leave the
admission algebra convention-based. The cut is:

| Obligation | Formal residue | DST / independent campaign | External trust boundary |
|---|---:|---:|---:|
| Exact equality of base, candidate, diff, path-set, campaign, action, quorum-policy, attestation-set, policy and authority-view ids | **Prove** | Mutation/differential fixtures | Content-address construction |
| Only exact `PROCEED` with zero mandatory open obligations can issue | **Prove** | Adversarial receipt mutations | Meaning of the frozen policy |
| `HALT`, `UNKNOWN`, malformed and future enum states fail closed | **Prove** | Fuzz parser/serializer and version skew | Schema governance |
| Permit grants review candidacy only and cannot lift modality/authority | **Prove API algebra** | End-to-end consequence-router negative tests | External institution must not ignore the API |
| Candidate actually materializes from `base + diff` | Bind a materialization-witness digest; do not claim more | **Re-materialize in an independent disposable tree; compare candidate digest** | Correct Git/canonical-diff semantics |
| Consequential path set is complete | Bind the frozen set; do not claim completeness | **MAP, dependency analysis, hostile path discovery, retained misses** | Human/domain definition of “consequential” |
| Repair fixes the bug class rather than the replay | Require the campaign result; do not prove target behavior | **Minimal replay, fresh seeds, widened palettes, hidden holdouts** | Oracle/property-contract adequacy |
| Behavior-preserving declutter | Bind equivalence evidence | **Trace/differential campaigns; counterexample search** | Declared observational boundary |
| Runner controlled every claimed channel | Check controller-set identity and admitted grade | **Leak battery, replay, host/profile campaigns** | Correct controller/kernel model |
| Judge is organizationally independent | Check a policy witness | Test key/process/write-surface separation | **Ratified authority policy and real custody** |
| Signature is valid and signer not revoked | Consume authenticated facts | Corrupt/truncate/substitute/replay fixtures | **Cryptographic implementation, keys, current revocation view** |
| Digests do not collide | No finite program proof in this slice | Cross-check encodings; collision fixtures cannot prove security | **Cryptographic assumption / selected primitive** |
| Compiled binary implements proved source | Source theorem only | Differential/conformance tests on shipped artifact | **rustc/toolchain unless a later refinement proof closes it** |

The formal proof therefore prevents **evidence laundering through the permit
algebra**. DST supplies discriminating evidence about the messy target. The
authority and cryptographic planes say who made which claim. None substitutes
for the other two.

## Initial implementation subset and TCB

### Recommended slice

Use a sibling crate or module with:

- safe Rust only and `#![forbid(unsafe_code)]`;
- fixed-size digest/id newtypes and closed enums;
- no I/O, filesystem, network, clock, randomness, environment access, FFI,
  async, threads, dynamic dispatch, plugin loading, or target code;
- no general JSON parser in the proved core; canonical bytes are validated at a
  separate bounded boundary and converted to fixed structured inputs;
- one pure `issue_review_permit` function and one canonical encoder;
- no `assume`, `external_body`, `external_fn_specification`, or trusted
  dependency specification in the residue itself; and
- pinned Verus, Z3, Rust toolchain, flags, source digest, proof transcript and
  assumption audit in the evidence bundle.

Verus distinguishes executable Rust from ghost specification and proof code;
the executable code remains ordinary Rust after ghost material is erased
([Verus modes](https://verus-lang.github.io/verus/guide/modes.html)). Its systems
paper reports a production Rust persistent-log integration in which standard
`rustc` consumes the executable code, while explicitly calling dependency
specifications trusted ([SOSP 2024 paper, §4.2.5](https://verus-lang.github.io/paper-sosp24-artifact/assets/paper-20240921-162720-b7db935.pdf)).

### Published TCB and assumptions

The first proof would still trust:

1. the theorem statement and frozen policy semantics;
2. Verus's Rust-to-verification-condition translation, its trusted automation,
   Z3, and the pinned Verus toolchain;
3. Rust's type checker and the pinned `rustc` compiler for the executable;
4. the small unproved canonical parser/authentication adapter at the boundary;
5. signature, hash, key-custody, and revocation implementations;
6. the authority registry's factual representation of roles and separation;
7. the campaign's DST evidence, oracle contract, channel ledger, and holdout
   integrity; and
8. the external consequence router honoring “review only.”

Verus's own guide lists `assume`, `external_body`, external specifications, and
ignored items as assumption-introducing mechanisms and warns that external
specifications can subvert guarantees
([assumptions and trusted components](https://verus-lang.github.io/verus/guide/tcb.html),
[`assume_specification` warning](https://verus-lang.github.io/verus/guide/reference-assume-specification.html)).
The artifact should therefore fail its admission gate if any such mechanism
appears in the residue without an individually named, human-ratified assumption.

This TCB is materially larger than an LCF proof kernel. The countervailing gain
is that the first theorem applies directly to the executable Rust predicate. No
document may abbreviate that trade as “mathematically proved end to end.”

## Court comparison

| Court / route | What would actually be proved | Strength for this residue | Cost / implementation gap | Decision input |
|---|---|---|---|---|
| **Verus + restricted safe Rust** | Preconditions/postconditions and total fail-closed behavior of the executable Rust decision function | Direct fit; same-language integration; sufficient automation for finite algebra; current production-style Rust precedent | Larger trusted verifier/SMT/toolchain than an LCF kernel; active-development subset; `rustc` remains trusted | **Recommended first slice**, conditional on zero unreviewed assumptions and direct integration |
| **Lean + Aeneas over safe sequential Rust** | Kernel-checked theorem over Aeneas's functional translation of the Rust function | Strong proof-checking kernel and closer-to-code route than a hand model; good future independent court | Charon/Aeneas translation and hand-written external models become a seam; Aeneas targets a subset of safe sequential Rust and excludes unsafe/concurrent code | Best **revival/second-court** candidate if the tiny core translates with no handwritten semantic holes |
| **Hand-written Lean model + Rust DRT** | Theorems about a definitional policy model, with differential tests relating Rust behavior to it | Excellent decorrelation and a very small policy object; Cedar is a direct authorization precedent | The Rust correspondence is tested, not proved; parser/input domains can escape the generator | Strong second implementation after v1; not the sole source-level proof |
| **Dafny** | Functional correctness of a Dafny implementation, compiled to a target language | Fastest likely proof of this finite function; excellent contracts, audit command, and solver automation | Vibe Halt would either execute non-Rust generated code/FFI or trust a translation; official Rust backend is still described as partial and growing | Choose only if the organization accepts Dafny as the residue's source language and measures the integration seam |
| **Isabelle/HOL + refinement to implementation** | Highest-assurance abstract theorem and, with substantial work, refinement to restricted implementation | Mature LCF lineage, explicit assumptions, strongest path for future isolation/noninterference residue | A hand model or new μRust/refinement stack is disproportionate for a small review gate; code generation alone does not prove the Vibe Rust binary | Reserve for an isolation engine or when the permit grows into a real consequence kernel |

Primary-source constraints behind the comparison:

- Verus supports a subset of Rust and describes itself as under active
  development ([official README](https://github.com/verus-lang/verus/blob/11eda20f4eac528b292048122701eda0b96b9650/README.md)). That is a
  reason to pin and prototype, not to inflate its assurance.
- Aeneas functionalizes a subset of safe, sequential Rust into Lean/HOL4/other
  backends, and external library definitions may require hand-written models
  ([official Aeneas README](https://github.com/AeneasVerif/aeneas/blob/74a460a2f80ecea481bbdf1a08f881633c3bb097/README.md),
  [supported scope](https://aeneasverif.github.io/aeneas/)).
- Lean's kernel checks elaborated declarations and is intentionally small; bugs
  in elaboration do not bypass kernel checking
  ([Lean language reference](https://lean-lang.org/doc/reference/latest/Elaboration-and-Compilation/)).
- Dafny verifies programs against built-in specifications via Boogie/Z3 and
  compiles verified source, but its official installation documentation labels
  Rust support “partial and growing”
  ([Dafny reference](https://dafny.org/dafny/DafnyRef/DafnyRef),
  [backend status](https://dafny.org/latest/Installation)). Its `audit` command
  exists precisely to surface assumptions that weaken a verification claim.
- Isabelle's code-generation documentation distinguishes proof from trusted
  computation gaps; generated computation may be wrapped in an oracle
  ([Isabelle code generation manual, §6.6](https://isabelle.in.tum.de/website-Isabelle2021-1/dist/Isabelle2021-1/doc/codegen.pdf)).

## What the verified-system precedents actually license

### Nitro and seL4: prove the cut, publish the assumptions

AWS did not point Isabelle at the old Nitro Hypervisor. It split out a
separation kernel, restricted its Rust dialect, and proved that residue in
Isabelle/HOL. AWS describes the result as 330,000 lines of machine-checked math
over μRust contracts and named confidentiality, integrity, functional
correctness, runtime-error, and memory-safety properties
([Amazon Science](https://www.amazon.science/blog/ec2s-formally-verified-isolation-engine-provides-mathematical-assurance-of-virtual-machine-isolation)).
The lesson for Vibe Halt is the architectural cut, not “use Isabelle everywhere.”

seL4 likewise publishes the exact configurations and assumptions to which its
proofs apply; hardware, boot, assembly, and some interfaces remain explicit
assumptions, and not every platform has every property
([proof assumptions](https://sel4.systems/Verification/assumptions.html),
[verified configurations](https://docs.sel4.systems/projects/sel4/verified-configurations.html)).
The permit must carry the same discipline: its theorem applies to one versioned
predicate and named boundary, not to the campaign's world model.

### Cedar: proof and differential testing are complements

Cedar is the closest precedent because it is an authorization decision engine.
Its Lean formalization proves properties such as “allowed only if explicitly
permitted” and fail-closed authorization laws
([cedar-spec verified properties](https://github.com/cedar-policy/cedar-spec/blob/e6c3e1f1f5c997ba1d09a80902db314643a26f5f/cedar-lean/README.md)).
Its production engine is safe Rust, and the project uses property-based and
differential randomized testing to compare Rust against the Lean definition
([Cedar security architecture](https://github.com/cedar-policy/cedar-docs/blob/52c026dce798c4e9358a8c5fea1ddf2cfdfa20b8/docs/collections/_other/security.md),
[DRT repository](https://github.com/cedar-policy/cedar-spec/tree/e6c3e1f1f5c997ba1d09a80902db314643a26f5f)).

That is the correct long-term weave for the permit: proof guards the decision
law; DST attacks the implementation and its boundary. The first slice chooses
direct Verus proof to avoid leaving the production Rust predicate merely
differentially related. A later Lean model should be decorrelated, not generated
from the same Verus specification.

## Counterexample and mutation matrix

The proof and the executable campaign must retain at least these adversarial
fixtures:

| Mutation / counterexample | Required result | Why proof alone is insufficient |
|---|---|---|
| Change only `BaseRevision`, `CandidateRevision`, or `DiffDigest` after a valid campaign | `Reject::SubjectMismatch` | Must also test parser and signed-envelope field plumbing |
| Replace the consequential path set with a subset having the same visible finding replay | Reject | Completeness of the original map remains a DST/domain claim |
| Replay a valid admission under another `CampaignId`, envelope, controller set, budget, or policy | Reject | Cross-envelope evidence laundering happens outside the pure core too |
| Judge equals treatment signer, is unauthorized, or is revoked in current authority view | Reject | “Independent” depends on correct real-world custody and policy data |
| Valid signature from the wrong role/key | Reject | Cryptographic verification and trust routing are outside Verus core |
| `UNKNOWN` with an empty serialized open-obligation list | Reject because verdict is `UNKNOWN` | Protects against one inconsistent representation, not oracle omission |
| Unknown future verdict/schema tag | Reject | Version-skew parser must preserve the unknown value rather than default |
| Original replay passes but fresh hidden holdouts fail | No eligible admission reaches the core | Repair overfit is learned through DST, not theorem proving |
| `base + diff` materializes a tree other than `candidate` | No eligible admission reaches the core | Real Git/diff semantics are not in the first formal model |
| Same permit presented to merge/deploy endpoint | External router rejects: review capability only | A caller can ignore any library unless consequence enforcement is wired |
| Remove any one equality/check from the implementation | Proof fails and a retained negative fixture passes through the mutant | Guards vacuous or accidentally unused proof clauses |
| Introduce `assume`/trusted external spec that states eligibility | Proof artifact invalid | A green verifier run with an axiom is not the required theorem |

The reduced finite-domain model should also be exhaustively enumerated against
an independently written reference table. This does not replace the proof; it
detects a wrong but internally consistent specification.

## Admission gate for the proof experiment

Before this object can be described as “proved,” require all of the following:

1. the theorem, claim boundary, TCB, assumptions, and excluded properties are
   frozen before proof work;
2. the executable safe-Rust function verifies under an immutable Verus/toolchain
   pin with no `assume`, `external_body`, external function specification, or
   ignored core item;
3. all theorem dependencies and solver/verifier flags are recorded and the
   proof reruns from a clean environment;
4. canonical encoding round-trip, field-substitution, future-tag, parser fuzz,
   and every-binding-axis fixtures pass;
5. mutation testing shows every issuance conjunct is load-bearing;
6. an independent reduced-domain evaluator agrees exhaustively;
7. the unproved boundary adapter is separately size-bounded, fuzzed, and listed
   in the TCB;
8. the product API exposes only human-review candidacy, and a consequence-router
   test proves the token cannot authorize merge or deployment; and
9. the evidence bundle signs and content-addresses the exact source, theorem,
   Verus release, Rust toolchain, flags, TCB manifest, test corpus, and result.

Passing those gates establishes the bounded `IssueSound` claim. It does not
establish that any particular repair deserves `PROCEED`; that still requires the
independent campaign.

## Kill and revival criteria

### Kill or reorient the Verus first-court experiment

Timebox the first slice to ten working days after the policy schema is frozen.
Stop and preserve the specification if any of these holds:

- the pure core cannot verify without an unreviewed `assume`, trusted external
  specification, unsafe escape, or duplicate unproved production wrapper;
- integrating the verified function requires copying its logic into ordinary
  Rust instead of compiling/calling the same executable body;
- one subject/campaign/action/quorum binding can be removed without failing either the
  proof or a retained mutant fixture;
- the theorem proves only a hand-written model while the shipped decision takes
  a materially different parsing or control path;
- verification is non-reproducible under the pinned toolchain or silently
  changes with solver flags; or
- the permit is consumed as merge, deployment, override, or evidence-promotion
  authority.

The last condition kills the **constitutional object**, not just the tool
choice.

### Revival falsifiers

- **Lean/Aeneas revival:** the same restricted Rust core translates at pinned
  commits with no hand-written external models or admitted holes; Lean proves
  the same theorem; parser/consequence boundaries remain explicit.
- **Hand-Lean/Cedar-pattern revival:** a decorrelated Lean definition and Rust
  evaluator agree exhaustively on the reduced domain and under property-based
  differential testing, while the implementation gap is still printed in the
  receipt.
- **Dafny revival:** the residue is intentionally adopted as Dafny source, its
  compiled artifact is the executed subject, `dafny audit` is clean, and the
  Rust/FFI seam is removed or admitted explicitly.
- **Isabelle revival:** the residue grows into a genuine isolation or
  consequence-enforcement kernel whose confidentiality/integrity/noninterference
  theorems justify a restricted-language refinement stack; code-to-model
  correspondence is funded, not assumed.

A newer tool release, more signatures, or a model-generated proof is not by
itself a revival event. The previously failed obligation must pass unchanged.

## Uncertainty and non-claims

- No Verus, Lean/Aeneas, Dafny, or Isabelle prototype was run in this research
  ticket. Tool-fit assessments are proposed and must be falsified by the bounded
  experiment.
- The exact `AdmissionQuorum<Policy>`, key custody, threshold,
  revocation latency, and permitted appeal is intentionally unresolved here.
- The exact first Dharma campaign may require additional subject fields. Add
  them only if the campaign archaeology proves they are consequence-relevant;
  never hide them inside a narrative string.
- The proof will not establish SHA-256 collision resistance, signature security,
  path-map completeness, oracle validity, holdout secrecy, target determinism,
  repair correctness, organizational independence, compiler correctness, or
  institutional compliance.
- A successful theorem is evidence about one source artifact under one toolchain
  and specification. It is not evidence that Vibe Halt as a whole is proved.

## Decision carried to the map

Adopt the **review-only binding theorem** as the smallest provable residue. Ask
the human proof-court decision to prototype Verus first, with Lean/Aeneas or a
decorrelated Cedar-shaped Lean model as the explicit fallback/cross-check. Keep
Isabelle for a later isolation residue that earns its cost.

The constitutional sentence is:

> Vibe Halt may prove that a candidate is exactly the candidate an independent
> campaign admitted to human review. It may never prove, infer, or encode from
> that permit that the candidate may merge, deploy, or bind the world.
