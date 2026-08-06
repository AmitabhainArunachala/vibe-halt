# REALITY BRIDGE CONTRACT CLOSURE — Issue #90 Overnight Goal

- **Artifact:** resumable native Codex `/goal` controller
- **Delivery SSOT:** [issue #90](https://github.com/AmitabhainArunachala/vibe-halt/issues/90)
- **Observed base:** merged `main` at
  `2a0190b2388c2768bf73783f692fe8b7e6d30224` on 2026-08-06 JST
- **Execution ceiling:** repository-local implementation, tests, review, push,
  and a draft PR for human review
- **Claim boundary:** local Tier-2/D2 bridge-contract closure only
- **Terminal success:** `ISSUE90_DRAFT_GREEN_FOR_HUMAN_REVIEW`

This controller does not replace issue #90 or create another task ledger. It
turns that accepted work packet into a resumable execution contract. Give the
whole file to the native Codex goal runtime. Managed subagents may implement
bounded packages; receipt writers, shell loops, tmux sessions, and model
councils are not reasoning or implementation planes.

## 0. Outcome

Implement toward human-reviewed closure of the two missing bridge-contract
slices identified by the vision:

1. versioned operation/feature negotiation between the strict Python client
   and the exact copied Rust engine; and
2. a mechanically observed target revision bound into new cooperative receipt
   and verifier schemas, with the remaining D2 execution-binding channel named
   rather than silently promoted to causal proof.

Finish at one pushed **draft** PR linked to #90, with red-first negative
controls, targeted tests, two clean full gates, fresh self-review, exact-head
hosted CI, and fresh-context/decorrelated challenge of trust boundaries. Do not
merge, mark ready, execute a foreign target, or claim that #60, #67, criterion
7, or any
external-reality gate is complete.

The work is load-bearing because the current public client has no negotiated
operation/feature contract and no observed-target-revision binding
(`VISION.md:71-88`, `VISION.md:119-124`). The current generic receipt records a
caller-declared commit and explicitly says it is never verified
(`crates/vh-cli/src/receipts_v2.rs:60-72`). The cooperative receipt already
embeds and hashes the exact compiled child source
(`crates/vh-cli/src/cooperative.rs:1182-1214`), which is sufficient for a
candidate-neutral local positive control without touching foreign code.

## 1. Reconcile live truth before acting

Run, record, and interpret rather than assuming:

```bash
git fetch origin
git rev-parse origin/main
git merge-base --is-ancestor 2a0190b2388c2768bf73783f692fe8b7e6d30224 origin/main
gh issue view 90 --repo AmitabhainArunachala/vibe-halt --json state,title,labels,projectItems,updatedAt
gh issue view 60 --repo AmitabhainArunachala/vibe-halt --json state,title,updatedAt
gh issue view 67 --repo AmitabhainArunachala/vibe-halt --json state,title,updatedAt
gh pr list --repo AmitabhainArunachala/vibe-halt --state open --json number,title,isDraft,headRefName,headRefOid
git worktree list --porcelain
make onboard
make project-plan
make project-accept
make gate
```

At authorship, the historical observed start receipt (not admission authority)
is:

- merged-main CI run `31020511170`: success;
- merged-main Verify run `31020510919`: success;
- `make project-plan`: no additive changes;
- `make project-accept`: 37 items, 6 Accepted, 3 ClosedUnaccepted,
  28 Open, 0 violations, 0 warnings;
- baseline `make gate`: `ALL PASS` on exact base `2a0190b`.

At authorship, #90 is `OPEN`, has the `agent-ready` label, and its Project
Status is `In Progress` because this controller slice has begun. Those values
are untrusted coordination metadata, not authority predicates. Execution scope
comes from the operator's current instruction constrained by repository law and
this #90 controller. Live issue/label/project/PR state is used only to detect a
possible revocation, collision, or stale plan that demands conservative stop
and reconciliation; it can neither mint nor prove authority.

These are observed checkpoints, not durable facts. If `origin/main` moves,
rebase before the first edit or before publication, rerun the last green gate,
and record the new base. If #90 is closed, superseded, loses `agent-ready`, or
has a project Status outside `{Ready, In Progress}`, treat that as an unresolved
coordination/revocation signal and stop with
`ISSUE90_BLOCKED_COORDINATION_COLLISION`; do not infer who authored the change.

The open-PR scan is also a collision guard. If the sole #90 draft is on the
current branch, reuse and update it. If a different active implementation
branch or a second #90 PR exists, do not open a duplicate; reconcile ownership
or stop with `ISSUE90_BLOCKED_COORDINATION_COLLISION`.

Read the current versions of `AGENTS.md`, `CLAUDE.md`, `VISION.md`,
`docs/governance/ACTIVE_TRACK.yaml`,
`docs/plans/VIBE_HALT_BUILD_PLAN_2026-07-20.md`,
`docs/DEVELOPMENT_WORKFLOW.md`, the original Reality Bridge controller, every
file under `clients/python/vibe_halt/`, all Python tests, and the affected Rust
writer/verifier modules before editing. The repository workflow requires one
accepted issue, one bounded branch, a draft PR, exact-head evidence, and human
merge (`docs/DEVELOPMENT_WORKFLOW.md:8-25`,
`docs/DEVELOPMENT_WORKFLOW.md:95-105`,
`docs/DEVELOPMENT_WORKFLOW.md:178-192`).

## 2. Authority and effects

### Allowed

- edit repository-owned surfaces admitted below;
- build and test locally;
- create off-repo receipts under
  `/Users/dhyana/.vibe-halt/goals/issue-90-contract-closure-20260806/`;
- use managed subagents with disjoint write manifests;
- inspect public GitHub state;
- push the issue branch, open/update a **draft** PR, reply to review findings,
  and rerun CI;
- update honest issue/project coordination metadata for #90.

### Forbidden

- merge, approve, or mark any PR ready;
- execute, fetch, clone, install, instrument, or adapt a foreign target;
- enter or simulate the operator gate in #60;
- credentials, production data, external writes, outreach, provider spending,
  paid services, deployment, or target-specific integration;
- work on PR #58 or issues #62/#70, or broaden this into #82's language design;
- change branch protection, rulesets, repository permissions, or workflows;
- claim D1, close any of the 29 open capability channels, or hide a channel;
- add dependencies, introduce unsafe code, or change the frozen PRNG or trace
  formats;
- change `crates/vh-sandbox/**`, `ACTIVE_TRACK.yaml`, or corpus/holdout law
  without a newly demonstrated requirement and a separate human-ratified
  scope decision.

Green automation, model agreement, and project fields are evidence or
coordination metadata, never authority (`VISION.md:21-36`,
`docs/DEVELOPMENT_WORKFLOW.md:48-82`).

## 3. Non-convertible revision and execution-binding types

The smallest load-bearing type law is:

```text
RequestedTargetRevision<Caller>
    !=
ClaimedObservedRevision<Receipt>
    !=
FreshObservedRevision<EngineResolver>
    !=
VerifiedObservedRevision<Verifier>
```

`RequestedTargetRevision` is untrusted request metadata. It may express what
the caller intended to run and may be bound into request identity, but it can
never construct, default, coerce, copy, deserialize, or promote a fresh or
verified observation.

Requested and observed values use the same canonical coordinate shape while
remaining different authority types:

```text
TargetRevisionCoordinate = {
  subject: "cooperative-child-source-v1",
  algorithm: "sha256",
  digest: LowerHex<64>
}
```

The operation descriptor fixes `subject` and `algorithm`. Unknown tags,
cross-subject comparison, and algorithm mismatch refuse before execution; a
Git commit, source-file digest, and source-bundle digest are never implicitly
interchangeable.

`ClaimedObservedRevision` is an untrusted value deserialized from a receipt.
Parsing may construct this claim but cannot validate or promote it.

`FreshObservedRevision` is a closed Rust-owned sum type:

```text
FreshObservedRevision =
    Unknown
  | Sha256<RustResolvedTargetBytes>
```

Only the Rust resolution boundary may construct the `Sha256` case, from its
owned byte snapshot. A verifier may construct `VerifiedObservedRevision` only
after a fresh Rust resolution equals the receipt's claimed value and the
operation descriptor's binding policy is satisfied. Receipt parsing, Python,
CLI input, generic `source_commit`, and checkpoint restore can never construct
either trusted type.

Keep target observation and execution binding orthogonal:

```text
RevisionBinding =
    Unbound { reason: Unknown }
  | Bound { revision: VerifiedObservedRevision }

ExecutionBinding =
    StagedD2 { observation_to_exec_channel: Open }
  | CausallyBound { handoff: ClosedOwnedBytes }
```

The current cooperative fixture is the only required positive path. Rust owns
its static source, writes it once into a private fresh workspace, declares the
expected input bytes, and verifies/replays against the compiled-in source. That
supports a mechanically observed revision and a `StagedD2` execution binding;
it does **not** prove that the interpreter/loader executed the observed bytes.
`vh-sandbox` explicitly leaves the observation-to-exec filesystem race open.
Do not rename `StagedD2` to `CausallyBound` or claim behavior binding. Closing
the loader/observation-to-exec channel, including an owned-byte execution
handoff, is **not in #90 scope**. If #90 cannot be implemented without it, stop
with `ISSUE90_BLOCKED_SCHEMA_OR_TRUST_BOUNDARY` rather than attempting it.

Each operation descriptor declares `revision_policy` as `UnboundAllowed` or
`BoundRequired`. The cooperative target operation is `BoundRequired`; existing
generic v2 workloads remain legacy/unbound. `Unknown` is therefore honest for
an unbound operation but refuses a revision-bound operation.

Absence is malformed. `Unknown` is an explicit fact. A requested/observed
mismatch is a typed pre-execution refusal whenever the caller required an
exact match. Source observation without a closed execution handoff is not
behavior binding.

## 4. Negotiation contract

Rust owns one closed registry containing versioned operation identifiers and
their mandatory feature closure. Python may request an operation and require
additional supported features; neither Python nor a caller may weaken the
mandatory set.

Conceptual types:

```text
OperationId(name, version)
FeatureId(name, version)
OperationDescriptor(
  operation,
  request_schema,
  outcome_schema,
  receipt_schema,
  verifier_schema,
  observation_subject,
  revision_algorithm,
  revision_policy,
  execution_binding_ceiling,
  mandatory_features,
  optional_features
)
ProtocolManifest(schema, engine_sha256, descriptors, manifest_digest)

EngineNegotiationRefusal =
    UnsupportedOperation
  | UnsupportedFeature
  | InvalidFeatureSet
  | RequestedRevisionMismatch
  | MissingObservation
  | UnsupportedReceiptSchema

ClientProtocolFailure =
    ProtocolRecordMalformed
  | ProtocolEngineMismatch
  | ProtocolInvocationFailure
```

An engine refusal is strictly parsed, engine-issued, and consistent with the
same privately copied executable. It is authenticated/trusted only when that
copy also matches an external `EnginePolicy.expected_digest`. A malformed,
mismatched, or missing engine record cannot attest its own refusal: Python
reports a local `ERROR`, `verified=false`, and never recasts it as a Rust
verdict.

Use the repository's existing bounded canonical line/framing primitives and
strict positional parsers for the wire encoding; do not introduce serde,
bincode, JSON tolerance, or a new dependency. Every record starts with an
explicit schema tag, uses a fixed field order and closed enum spellings, frames
variable bytes with explicit lengths, and rejects duplicate, unknown, missing,
reordered, malformed, truncated, oversized, noncanonical, or trailing data.
This law applies to every new manifest, refusal, receipt, and verification
record consumed by Python: do not add a JSON protocol whose object-key order is
lost during parsing. Python validates the same positional framed bytes and
their canonical order before mapping fields.

The runner must query the **same privately copied executable** it later runs.
A successful preflight is not sufficient: the execution command must carry or
revalidate the selected operation, canonical feature closure, protocol schema,
and revision requirement immediately before any child/workload execution.
Unsupported or changed requirements return a typed refusal, increment no
execution counter, publish no checked receipt, and never return `CLEAN`.

Feature sets are bounded, canonical, sorted, unique, and identity-bearing. The
canonical manifest digest binds every complete operation descriptor, not only
a schema tag, and the execution request binds that digest. The request identity
includes:

```text
request schema
operation id/version
canonical requested feature set
Rust-owned mandatory feature closure
requested target revision, if any
```

Use an acyclic identity graph with frozen canonical preimages. Logical engine
identity and the client-side invocation envelope are deliberately distinct:

```text
manifest_id -> engine_request_id -> evidence_id -> receipt_sha256
            -> verification_result_id

invocation_id + fresh_output_root + engine_request_id + receipt_sha256
            -> invocation_envelope_id
```

`manifest_id` hashes the canonical manifest and descriptors.
`engine_request_id` hashes the manifest ID, selected operation, complete
feature closure, schemas, observation subject, revision algorithm, revision
policy, execution-binding ceiling, and requested revision.
`evidence_id` hashes only request-bound run/replay evidence plus the claimed
observation written by the Rust engine. `receipt_sha256` hashes the canonical
receipt bytes according to its schema. `verification_result_id` may bind the
receipt SHA, fresh/verified observation, and verifier result. No identity may
include itself, and no backward edge is permitted. The Python client-envelope
`request_digest` is a separate client correlation identity; it is not equal to
Rust's `engine_request_id` and cannot replace it. Likewise, invocation context
must not contaminate deterministic engine request/receipt identity. The Python
boundary generates or validates a bounded `invocation_id`, exclusively reserves
an empty private output root, runs the current copied engine, and binds the
invocation ID, root, engine request, and resulting receipt into its envelope.
The Rust verifier then performs a fresh replay. This proves process-local
admission and fresh semantic reproduction; it does **not** prove the temporal
origin of byte-identical receipt bytes. Same-user substitution after the engine
write remains an explicitly open D2 channel.

No caller-controlled empty feature list may downgrade a checked operation.

## 5. Schema law and compatibility

The existing receipt and verifier formats are closed. Scope the new negotiated,
revision-bound schemas to the cooperative target operation required by #90.
Do not migrate the generic Tier-1 v2 bundle/receipt system during this slice.
Adding negotiation or target-binding fields requires new cooperative schema
versions; never mutate existing bytes in place. Current strict parsers
intentionally reject schema drift and field
shape changes (`crates/vh-cli/src/receipts_v2.rs:21-31`,
`crates/vh-cli/src/cooperative.rs:1321-1350`,
`clients/python/vibe_halt/core/runner.py:493-578`,
`clients/python/vibe_halt/core/runner.py:581-690`).

Leave the generic finding-bundle v1/v2 families and their replay promises
unchanged (`crates/vh-cli/src/receipts_v2.rs:1-6`). For the affected cooperative
family, preserve `vh-cooperative-receipt-v1` / `vh-cooperative-verify-v1` only
through the existing explicit legacy verifier path. That path continues to
accept cooperative v1 and emit byte-identical v1 output with its current
self-consistent D2 replay meaning. The new dispatcher/client classifies that
schema as legacy/unbound without adding a v1 field; it never promotes the v1
record to a revision-bound outcome.
The new negotiated verifier path rejects cooperative v1 with typed
`UnsupportedReceiptSchema` before replay. The negotiated `BoundRequired`
operation accepts only the new cooperative schema, and every successful new
receipt carries a canonical SHA-256 claimed observation. `Unknown` belongs only
to a legacy/unbound operation or typed refusal record, never a successful
`BoundRequired` receipt. The new verifier constructs a fresh observation and
compares it before replay.

Existing generic `source_commit` remains unchanged declared provenance and is
already forbidden on cooperative requests. Add a distinct cooperative
`requested_target_revision` coordinate under the new negotiated request schema.
Under no path may either caller field populate a fresh or verified observation.

## 6. Definition of the green draft terminal

This section governs only `ISSUE90_DRAFT_GREEN_FOR_HUMAN_REVIEW`. It is an
agent-terminal predicate, not merge authority, project acceptance, or proof of
external reality. `ISSUE90_DRAFT_WITH_OPEN_REVIEW_DEBT` is a non-green handoff:
it may disclose unresolved P0-P2 findings after draft publication and must not
satisfy or impersonate this section.

All of the following are conjunctive:

1. Rust publishes a strict protocol manifest from a closed operation/feature
   registry tied to the executing engine digest.
2. The exact execution command revalidates the operation, mandatory and
   requested features, protocol schema, and requested-revision constraint
   before execution.
3. Rust constructs a fresh observation from Rust-resolved source bytes; a
   receipt parser constructs only an untrusted claim; and only fresh equality
   constructs a verified observation. The receipt and final claim name the
   `StagedD2` execution-binding ceiling unless a separately proven closed
   handoff exists.
4. New receipt and verifier schemas bind all identities in sections 3-5 and
   reject absence, mismatch, substitution, and unsupported schemas before
   replay. The standalone verifier receives the expected canonical request
   components, recomputes `engine_request_id` in Rust, and compares it before
   replay; Python never supplies a pre-minted Rust identity.
5. Python uses immutable typed request data, strictly parses only the Rust
   protocol/verification records, invokes without a shell, and maps refusals
   without creating a second verdict or revision authority. The existing
   client boundary remains client-only (`clients/python/vibe_halt/core/runner.py:704-773`).
6. Direct CLI and Python paths agree on Rust-owned operation, canonical
   features, claimed/fresh/verified revision, manifest/engine-request/evidence
   identities, receipt SHA, and verdict for the same local cooperative request.
   The Python client-envelope digest is checked for its own scope, not compared
   for equality with the Rust engine-request identity.
7. Absence of `EnginePolicy.expected_digest` remains explicitly
   `UNTRUSTED`/`UNCHECKED`; capability self-description cannot create a trust
   root (`clients/python/vibe_halt/core/request.py:23-63`).
   Python hashes the privately copied executable bytes and compares that digest
   with the manifest's engine digest for same-engine consistency; it separately
   compares against `EnginePolicy.expected_digest` when the caller supplies a
   trust root. The manifest must never authenticate its own engine digest.
8. Every mandatory negative control passes, existing gates remain unchanged in
   meaning, both full gates pass on the exact proposed head, and
   fresh-context/decorrelated review leaves no unresolved actionable P0-P2
   defect. Such review is advisory evidence, never authenticated human
   independence or merge authority.
9. A draft PR links #90, states the Tier-2/D2 local-only boundary, names every
   unproven external claim, and awaits human review.

## 7. Dependency DAG and packages

```text
G0 exact-base reconciliation and baseline proof
  -> G1 contract freeze plus red tests
      -> G2 Rust registry, negotiation, observation, and schemas
          -> G3 Python parity
              -> G4 integration and decorrelated falsification
                  -> G5 exact-head verification
                      -> G6 draft PR, CI, review, and handoff
```

### G0 — Reconcile and freeze

- complete section 1;
- persist base SHA, issue state, open-PR collision scan, `make onboard`,
  project conformance, and baseline gate exits;
- inventory every schema and public API that will change;
- estimate the G1-G6 critical path against the remaining wall-clock. Do not
  silently drop Python parity, negatives, gates, or hosted evidence to fit; if
  the conjunctive scope cannot fit, stop checkpointed at the timebox;
- freeze operation IDs, feature IDs, bounds, compatibility behavior, and the
  exact requested/observed revision semantics in the checkpoint before code.

Exit: `G0_BASELINE_GREEN` or a typed blocked terminal.

### G1 — Red-first contract

Add failing tests for newly absent behavior in negative cases 1-17 of section 9
before production changes. Record the regression guardrails and any other
already-enforced invariant as baseline-green rather than manufacturing red
tests. A red receipt records the exact test, intended failure, and why current
behavior is
insufficient. Do not use a handwritten successful receipt as a positive
oracle; the actual Rust engine produces every positive record.

Exit: `G1_RED_CONTRACT_FROZEN`.

### G2 — Rust truth path

- implement the Rust-owned closed registry and protocol manifest;
- add execution-side negotiation with a zero-execution refusal path;
- construct fresh observed revision only at the Rust resolution boundary;
- deserialize only claimed observation in the parser and promote only after
  fresh equality;
- introduce version-bumped cooperative writer/parser/verifier schemas;
- bind protocol, operation, full feature closure, requested revision,
  observed revision, engine, request, and evidence identities;
- extend the standalone expected-request boundary with operation, schemas,
  full feature closure, observation coordinate/policy, and requested revision;
  Rust recomputes and compares the expected `engine_request_id` before replay;
- keep the existing cooperative-v1 verifier path explicitly unbound, and make
  the new negotiated verifier reject v1 with `UnsupportedReceiptSchema` before
  replay;
- keep the cooperative workload closed: receipt-provided source is never
  executed (`crates/vh-cli/src/cooperative.rs:1543-1576`).

Exit: targeted Rust tests green with all G1 Rust negatives.

### G3 — Python parity

- expose immutable operation/feature/revision requirements without expanding
  Python truth authority;
- strictly parse the Rust protocol record and bind it to the copied engine;
- route typed negotiation refusals before execution;
- pass explicit expected request components to standalone reverification while
  leaving canonical Rust identity construction in Rust;
- validate the new verifier records and expose requested and observed values as
  read-only data;
- preserve no-shell invocation, bounds, output-root isolation, trust-root
  downgrade, and hostile-object revalidation.

Exit: targeted Python tests green with actual-engine positive controls and all
G1 Python negatives.

### G4 — Integrate and falsify

- prove CLI/Python equivalence for the local cooperative fixture;
- run mutation, substitution, downgrade, legacy, concurrency, and
  output-isolation tests;
- give at least one fresh-context reviewer the diff and red matrix read-only;
- fix or disposition each concrete P0-P2 finding with code/command evidence;
- update only current docs whose claims materially changed. Do not claim a
  foreign target or criterion 7.

Exit: `G4_FALSIFICATION_CLEAN` or `G4_REVIEW_DEBT_RECORDED`. Review debt does
not terminate before publication: carry it through the local checks that can
still honestly run, publish/update the draft with the debt disclosed, and only
then terminate as `ISSUE90_DRAFT_WITH_OPEN_REVIEW_DEBT`. It can never use the
green terminal while an actionable P0-P2 remains.

### G5 — Verify exact head

Run targeted Rust and Python tests and a preliminary full gate. Then create the
candidate commit. Its message names only checks actually run before the commit;
the exact-head commands remain pending evidence. On that exact commit run:

```bash
make gate
make gate
make review
git diff --check
make project-plan
make project-accept
git status --short
```

The two full gates must be consecutive on the same clean candidate commit.
`make review` must inspect that candidate rather than the merge base.
Generated/runtime receipts remain off-repo. Any formatter/test change or repair
commit resets the exact-head sequence.
Afterward, record commands and exits in the off-repo receipt and draft PR
without amending the candidate commit. Any amendment creates a new head and
requires the sequence again.

Exit: `G5_LOCAL_GREEN`.

### G6 — Draft PR and hosted evidence

- confirm the exact candidate commit from G5 contains only #90 scope;
- push the bounded branch;
- open a draft PR linked to #90 with exact base/head, changed surfaces,
  observable contract, tests, rollback, claim boundary, unproven claims, and
  human-only merge statement;
- wait for exact-head CI and Verify; inspect failures rather than weakening a
  gate;
- query unresolved threads through GraphQL, not flat comments alone;
- answer and resolve only after the remote diff contains the fix;
- update #90 board Evidence to the strongest demonstrated grade, leaving Gate
  state `Human pending` and Status `In Progress` until a human merges.

Exit only as one terminal in section 13.

## 8. Ownership and parallelism

Use the existing `vibe-halt-core-2026-07` track. Prefer the smallest number of
writers that preserves decorrelated falsification.

- **Rust integration writer:** `crates/vh-cli/src/main.rs`,
  `crates/vh-cli/src/cooperative.rs`, cooperative receipt/verifier surfaces,
  Rust integration tests, and at most one new narrow protocol module. Generic
  Tier-1 v2 receipt migration is out of scope.
- **Python writer after G1 interface freeze:**
  `clients/python/vibe_halt/core/{request,result,runner}.py`, public exports,
  and Python tests.
- **Falsifiers/reviewers:** read-only unless the integrator assigns one exact,
  disjoint test file.
- **Root integrator, the single shared-file writer:** `scripts/gate.sh` and any
  current documentation changed by a material claim transition.

Every agent receives: owner role, exact file manifest, verifier command, stop
condition, and the warning that other writers are active and their changes must
not be reverted. Exchange exact commits and frozen interfaces, not prose
assurances. If safe disjoint manifests cannot be maintained, serialize.

## 9. Mandatory negative matrix

1. unsupported operation -> typed refusal, zero execution, no receipt/CLEAN;
2. unsupported required feature -> same;
3. duplicate, malformed, oversized, or noncanonical feature set -> refusal;
4. malformed, duplicate, missing, unknown, reordered where canonical,
   truncated, oversized, or trailing protocol record -> refusal;
5. a stale manifest digest from a prior preflight, or an operation/feature set
   mutated between preflight and the execution request, is revalidated against
   the current same-binary registry and refused before child execution;
6. protocol record engine digest differs from the copied engine -> local
   client protocol `ERROR`, `verified=false`, never an engine refusal;
7. caller-requested revision cannot appear as fresh/verified observation by
   copy/default/coercion; parsing creates only a claimed observation;
8. exact-match requested revision differs from fresh observation ->
   zero-execution refusal;
9. missing claimed observation in a new receipt -> reject before replay;
10. mutated claimed observation -> fresh equality fails before replay;
11. substituted target bytes before Rust's final owned snapshot/recompute ->
    revision mismatch; substitution after that boundary is not claimed
    detectable and remains an explicitly open D2 loader/exec channel;
12. changed operation, features, schema, or revision changes bound request and
    evidence identity;
13. legacy receipt cannot promote to revision-bound checked evidence;
14. valid receipt from a different request/operation/feature set cannot be
    adopted;
15. direct CLI and Python disagreeing on any Rust-owned negotiated/bound value
    or receipt SHA -> error; client-envelope and Rust request digests retain
    their distinct typed scopes;
16. absent engine trust root -> explicit `UNTRUSTED`/`UNCHECKED`, never checked;
17. a pre-existing/non-empty output root or receipt is refused before the
    current engine runs; the successful client path must reserve an empty root,
    invoke the copied engine in the current process flow, and perform fresh
    Rust replay. Do not infer temporal byte provenance: post-write same-user
    substitution remains an open D2 channel.

Every negative fixture records whether execution occurred. A refusal that ran
the child is a failure even if its final verdict is not clean.

Regression guardrails are baseline-green and final-green, not manufactured
red tests:

- concurrency and competing output roots remain isolated; and
- doctor fingerprint, frozen PRNG/trace identities, corpus pins, D2 labels,
  and all 29 open channels remain unchanged.

## 10. Run/use/prove/record/falsify loop

For every package:

- **Run:** invoke the narrow test or real CLI/client path.
- **Use:** exercise it through the public command or Python API, not a private
  helper alone.
- **Prove:** retain exit, typed machine record, execution counter, identity
  comparison, or explicit unavailable reason.
- **Record:** append an off-repo checkpoint with exact base/head, command,
  exit, evidence path, blockers, and next action.
- **Falsify:** run at least one adversarial input owned by a different agent or
  fresh-context pass.

A receipt proves only the event it records. A model review is not a test. A
fresh-context pass by the author is a cross-check, not independent authority.

## 11. Checkpoint and resume

Write checkpoints outside git under:

```text
/Users/dhyana/.vibe-halt/goals/issue-90-contract-closure-20260806/
```

Minimum NDJSON shape:

```json
{"schema":"vh-issue90-checkpoint-v1","package":"G2","state":"IN_PROGRESS","base_sha":"...","head_sha":"...","last_command":"...","last_exit":0,"evidence":["..."],"blockers":[],"next_action":"...","recorded_at":"RFC3339"}
```

Never store an `authority_state` copied from an agent checkpoint. On resume,
re-resolve issue/PR state from GitHub and repository law. Resume by running
`make onboard`, reconciling `origin/main`, checking `git status`, rerunning the
last green narrow verifier, and continuing from `next_action`. A moved base or
failing last verifier invalidates the checkpoint until reconciled.

The absolute directory above is a per-host convenience, not a trust root or
portable authority artifact. Create it with private permissions on this host.
On another machine, or when it is absent, start from G0 and reconstruct state
from Git/GitHub plus executable checks; never copy a checkpoint's status into a
promotion decision.

## 12. Bounds and retry law

- wall-clock target: 8 hours; at 12 hours checkpoint and emit the typed
  timebox terminal rather than calling incomplete work complete;
- spend: $0;
- external effects: only the allowed GitHub draft-PR workflow;
- no more than three active implementation writers including the integrator;
- retry an identified infrastructure flake once on the same SHA, recording both
  runs; a repeated or different failure requires investigation;
- the known #85-class cross-platform TOCTOU signature permits one same-SHA
  rerun only when logs match that documented signature; never weaken a gate;
- two repair loops failing for the same root cause without new evidence trigger
  a blocked terminal.

## 13. Kill rules and typed terminals

Stop immediately when:

- baseline or merged main is red for an unexplained reason;
- scope requires foreign code, target-specific adaptation, a new dependency,
  unsafe code, D1, or a frozen-format change;
- Python, a caller, a receipt parser, or a checkpoint can construct a fresh or
  verified observation (a parser may construct only a claimed observation);
- negotiation is preflight-only and can be bypassed at execution;
- a revision-bound checked result can exist without a recomputable verified
  observation;
- legacy evidence silently receives the new revision-bound meaning;
- shared-file ownership collides without an explicit integration handoff;
- authority would need to expand beyond a draft PR;
- the 12-hour hard stop arrives without exact-head green evidence.

Allowed terminal states are the closed set:

- `ISSUE90_DRAFT_GREEN_FOR_HUMAN_REVIEW`
- `ISSUE90_DRAFT_WITH_OPEN_REVIEW_DEBT`
- `ISSUE90_BLOCKED_BASELINE`
- `ISSUE90_BLOCKED_SCHEMA_OR_TRUST_BOUNDARY`
- `ISSUE90_BLOCKED_AUTHORITY`
- `ISSUE90_BLOCKED_COORDINATION_COLLISION`
- `ISSUE90_BLOCKED_INFRASTRUCTURE`
- `ISSUE90_STOPPED_TIMEBOX_CHECKPOINTED`

Do not call a timeboxed stop completion. `ISSUE90_DRAFT_GREEN_FOR_HUMAN_REVIEW`
requires a pushed draft PR, exact-head hosted CI/Verify success, no unresolved
actionable P0-P2 review finding, clean local gates, and honest board state. It
does not authorize merge or #60.

## 14. Review council

After local G4 evidence exists, use the operator environment's decorrelated
review council, when available, against the exact diff, controller, and red
matrix. This is an advisory reviewer, not a repository runtime dependency or a
required authority lane. Preserve requested and reached provider/model
identities. An unavailable or substituted required lane blocks only a
`full-council` claim; it does not by itself block the draft-green terminal.
Any concrete unresolved actionable P0-P2 defect does block that terminal,
regardless of which lane found it. Council output cannot satisfy tests, human
review/merge, operator approval, or external confirmation. Rerun local
executable evidence after every council-driven change.

## 15. Final receipt

The final report and off-repo receipt must state:

- exact base/head and PR URL;
- files changed and ownership handoffs;
- protocol, operation, feature, request, receipt, and verifier schema IDs;
- how claimed, fresh, and verified revisions were constructed; which source
  bytes Rust observed; the execution-binding grade; and every open loader or
  observation-to-exec channel, including both the cooperative target-byte race
  and the copied-engine observation-to-exec race;
- all mandatory negative controls with commands/exits;
- targeted tests, both full gates, review, diff check, hosted CI/Verify URLs;
- unresolved threads and council lane/provider dispositions;
- evidence grade and the absence/presence of a trust root;
- every claim that remains unproven;
- the one next safe human action.

Before any later #60 execution, separately ratchet #60 and #67 so their entry
gates explicitly require #90 human-merged and green on merged main. Do not fold
any unrelated holdout-validator repair into #90.

The successful end state is narrow: the local bridge can state which operation
and features it executed, which target bytes Rust observed, and whether that
observation had only `StagedD2` or a separately proven closed execution
handoff. It is not yet proof that the interpreter executed those exact bytes,
or that a foreign target is admissible, useful, or authorized.
