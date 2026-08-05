# TRUTH KERNEL — Harness-Agnostic Admission Layer, First Tenant dharma_swarm

- **Artifact type:** resumable long-running `/goal` controller (admission
  proposal)
- **Authored:** 2026-07-29; **substantively repaired:** 2026-08-05 (PR #58
  repair wave; addresses the five review findings mapped in §16)
- **Repository:** `AmitabhainArunachala/vibe-halt` (primary);
  `AmitabhainArunachala/dharma_swarm` (first tenant, separate admission)
- **Observed merged `main` checkpoint (mutable — refresh before use):**
  `2a0190b` on 2026-08-05 (`git fetch origin && git rev-parse origin/main`).
  At that checkpoint: Wave B merged at `63ccd32` via PR #57; workflow/project
  hardening merged at `ed32f1d` via PR #66; the VB-008 R4 decision packet
  merged via PR #87; fail-closed project synchronization merged via PR #88.
- **Target duration:** six packages in six resumable waves, ~40–80 focused
  implementation hours plus human merge/authorization pauses; expected
  wall-clock span: weeks
- **Status:** DRAFT admission proposal, non-merge-admissible. PR #58 stays
  draft. Issue #62 keeps this controller blocked until a human-merged
  `REALITY_BRIDGE_COMPLETE_FORWARD_CONFIRMED` or
  `REALITY_BRIDGE_COMPLETE_FORWARD_NULL` result exists and the current
  protected `OwnerAccountIssueAuthorityV0` plus the populated immutable A1 bootstrap
  record prove that #62 may proceed (§3–§4).
  Nothing in this file is execution, merge, or
  foreign-target authority.

## Operator use

Give this whole file to one long-running coding agent per package. Use the
native Codex goal runtime plus managed agents as the reasoning and
implementation plane. The §12 checkpoint is only a ledger: it neither runs
an agent nor proves that reasoning, verification, or an authority event
occurred. A separate evaluator reviews every authoring lane; no generator may
promote its own work.

This controller is the
strategic successor to
`docs/prompts/VIBE_HALT_REALITY_BRIDGE_LONG_RUNNING_GOAL_2026-07-29.md`
(the "Reality Bridge controller"): the Reality Bridge remains the current
critical-path controller and this controller MUST NOT preempt, weaken, or
fork it. No Truth Kernel implementation package may start until the exact
Reality Bridge and #62 entry predicate in §3 is true. Repository-local review
and repair of this proposal before that predicate is preparation only.

Human merge of this controller authorizes only the bounded repository-local
implementation packages described below, and only when each package's entry
gate is true at execution time. It grants no merge, self-approval, spending,
credential, paid-provider, deployment, unsafe-code, or foreign-target
authority — ever, for any wave. Work in `dharma_swarm` requires a separate
admission PR in that repository under its own governance; this controller
only defines the cross-repo contract those PRs implement.
Any future least-privilege credential used solely by a protected controller to
publish a check requires that repository's separate human authorization; this
proposal neither grants it nor permits it to enter an untrusted worker.

**Observed-checkpoint rule.** Every GitHub-mutable fact in this file (issue
open/closed, PR draft/merged, head SHAs, review state) is an *observed
checkpoint* dated 2026-08-05, not a durable fact. Durable repository claims
carry `file:line` citations or runnable commands; mutable claims carry a
refresh command. Refresh before acting:

```bash
git fetch origin && git rev-parse origin/main
gh issue view 60 --json state,title,updatedAt
gh issue view 62 --json state,title,updatedAt
gh pr view 58 --json state,isDraft,mergeStateStatus,headRefOid
gh pr view 57 --json state,mergedAt,mergeCommit
gh pr view 66 --json state,mergedAt,mergeCommit
gh pr view 87 --json state,mergedAt,mergeCommit
gh pr view 88 --json state,mergedAt,mergeCommit
gh pr list --state open --json number,title,isDraft
```

Treat every other recorded SHA, PR state, line number, and criterion status
as stale until refreshed the same way.

---

# `/goal` TRUTH KERNEL

## 0. Thesis (why this campaign, why now)

Late-July 2026: agent harnesses are commoditizing. Code generation is
effectively free; verification of unread code is the scarce resource, and it
becomes MORE scarce as harnesses accelerate. The durable position is
therefore not a faster harness but the **admission layer harnesses plug
into**: a deterministic, fail-closed kernel that takes any agent's proposed
change, executes it against a registry-pinned property contract under
adversarial fault injection, and emits a content-addressed evidence bundle —
or rejects it. Harnesses compete on speed; the kernel is the referee, and
referees don't get commoditized.

The campaign starts from two proposed halves:

- **vibe-halt** is intended to test what code *does*: deterministic
  multiverse execution,
  semantic fault injection, exact-fingerprint shrinking, chained-hash traces,
  v2 evidence bundles (`crates/vh-cli/src/receipts_v2.rs:35`), and a strict
  local Python adapter whose caller-process object is explicitly not an
  authority boundary (`clients/python/vibe_halt/core/runner.py:704-710`).
- **dharma_swarm** is the proposed first claim-governance tenant. Its exact
  receipt mount point, governance, and owned surfaces are intentionally not
  asserted here; K3A must re-observe and cite them in that repository before
  K3B work is admitted (§3, §9).

The Truth Kernel campaign fuses them: vibe-halt becomes a harness-agnostic
verification substrate with a stable tenancy contract, and dharma_swarm
becomes its first paying-in-discipline tenant. K3B first makes kernel evidence
advisory at merge decision time; only the separate human `GP3_REQUIRED`
act may make that evidence mandatory. Dogfooding continues until the kernel
is the product and the swarm is its noisiest customer.

## 1. Enduring laws (inherited, non-negotiable)

All six enduring laws of `VISION.md` apply unchanged. In particular:

1. The engine owns truth; no tenant, harness, or client may mint a verdict,
   grade, digest, or receipt.
2. Every claim names its boundary (Tier 1/D0 vs Tier 2/D2); agreement is a
   sampled falsifier, never proof.
3. Evidence fails closed: missing/malformed/stale/ambiguous/tainted is
   `UNCHECKED` or an error, never `CLEAN`
   (`crates/vh-cli/src/main.rs:126-131`).
4. Real utility outranks self-demonstration; tenancy receipts from
   dharma_swarm count as utility only when they gate a real merge decision.
5. A negative result is a result; tenancy nulls stay visible.
6. Humans merge and confirm; green automation is evidence, not approval
   (`docs/DEVELOPMENT_WORKFLOW.md:178-181`).

Plus one law this repair adds and names explicitly:

7. **Evidence validity never manufactures authority.** A verified bundle, a
   green CI check, or a valid attestation is *evidence*; merge, gate
   promotion, and any external execution are *authority* events that only
   humans perform. §4 makes this a closed, typed, evaluator-enforced
   contract so no package can launder evidence into authority.

## 2. Mission and completion contract

Deliver six outcomes in dependency order (DAG in §8):

1. **K1 — Tenancy contract v0 (spec only).** A versioned, harness-agnostic
   admission contract: how any external system submits a verification
   request, what it may carry, what evidence bundle it gets back, the
   provenance model (§5), the source-binding envelope (§6), the
   registry-pinned contract and fault-model law (§7), the verdict mapping
   table, the capability statement, the attestation schema/trust policy, and
   negative fixtures. No code.
2. **K2R — Tenant workload registration (vibe-halt, new wave added by this
   repair).** The Rust registry entry and implementation binding for the
   first tenant workload, frozen as
   `dharma-graph-checkpoint-replay-v1`: its versioned property contract,
   fault model, operation/features, and one composite registration digest,
   all merged to vibe-halt `main` **before** any tenant wave may submit that
   workload. Closes the registration-sequencing finding (§7–§9).
3. **K2 — Tenant SDK and source-consuming bridge.** Generalize the strict
   local Python adapter while implementing versioned operation/feature
   negotiation, runtime-observed source binding, and a generic operation that
   consumes canonical tenant-source-derived input. A compiled model whose
   verdict is invariant to the tenant's changed bytes is not admission
   evidence. Bundle re-verification remains integrity/replay only (§5).
4. **K3A — dharma_swarm admission charter (separate repo, governance only).**
   Re-observe that repository, freeze exact owned surfaces, commands, budgets,
   adapter identity, and advisory-check policy, and obtain its human merge.
   This is the `A2_TENANT_ADMISSION_MERGED` transition; it contains no tenant
   implementation.
5. **K3B — First tenant implementation and advisory gate.** A separate
   dharma_swarm implementation PR consumes protected-observed tenant Git-tree
   source through K2, pins K2R's composite registration digest, and adds an
   **advisory** check that can go green only through protected trusted
   execution or verified attestation (§5, §10). Promotion from advisory to
   required is a later human `GP3_REQUIRED` governance decision on a separate
   axis that preserves the A2 admission chain.
6. **K4 — Second tenant or public tenancy dossier.** Either a second real
   tenant verified through the unchanged contract, or a decision-ready
   public dossier that tests whether, and identifies exactly how, the contract
   is or is not portable beyond dharma_swarm.
   Entry requires K3B's advisory gate to have produced at least 10 real PR
   verdicts (any mix of green/red/unchecked) on dharma_swarm main-bound PRs.

Terminal completion: current merged `origin/main` (vibe-halt) contains
K1, K2R, and K2; dharma_swarm merged main contains K3A and K3B with its
advisory check live; at least one subsequent `MergeRelianceAttestationV0` in
dharma_swarm binds the exact advisory decision + evidence-bundle digests and
records the authorized merger account's assertion that evidence was a reason
for the merge (not necessarily the sole reason, and not proof of human
presence or cognition); and either (a) a separately admitted K4 second-tenant
implementation is human-merged, or (b) a decision-ready K4 dossier is human-
merged and records why no second tenant is currently admissible, after which
`TRUTH_KERNEL_TENANCY_READY_SECOND_TENANT_REQUIRED` may be emitted.

Explicit non-goals (kill on sight):

- replacing or re-implementing dharma_swarm's spine, telos gates, or merge
  authority inside vibe-halt;
- any "Rust migration of dharma_swarm" — the kernel verifies tenants, it
  does not absorb them;
- claiming the kernel checks properties it cannot (LLM output quality,
  semantic correctness of prose, security of arbitrary code); the K1
  capability statement must say exactly what is checked;
- upgrading D2 evidence to D1, or letting a tenant's cooperative behavior be
  described as containment (all 29 D2 capability channels remain open,
  `docs/specs/SANDBOX_CAPABILITY_ENVELOPE_V1.md:25-38`);
- executing arbitrary or adversarial tenant code inside a protected checker,
  on an operator workstation, or on any credentialed runner; K3B is
  source-consuming but remains bounded to the exact K3A charter and §5 outer
  isolation rule;
- treating local bundle verification as admission authority (§5);
- tenant-selectable property subsets, tenant-supplied fault models, or
  caller-declared source identity (§6, §7);
- a hosted service, billing, tenant/worker credentials, foreign-target
  execution, or any live-provider execution. The only credential-shaped
  exception is a separately human-authorized, least-privilege check-result
  writer held exclusively by the protected controller under §5.

## 3. Truth at admission (observed checkpoint 2026-08-05)

Refresh every item below with the § "Operator use" commands before relying
on it. Where a claim is durable in the repo it carries a citation; where it
is mutable GitHub state it is labeled OBSERVED and dated.

**Reality Bridge state:**

- OBSERVED: Wave B merged via PR #57 at `63ccd32`; the strict local
  Python-to-Rust transport, fresh generic/cooperative re-verification, the
  reusable cooperative D2 transport, and the R3 holdout/dossier law are
  merged substrate (`docs/governance/ACTIVE_TRACK.yaml:90`).
- Durable: the cooperative transport accepts only `cooperative-echo`; any
  other `--workload` is rejected (`crates/vh-cli/src/cooperative.rs:793-795`;
  usage `crates/vh-cli/src/main.rs:106-108`), and the Python adapter pins
  the same contract (`clients/python/vibe_halt/core/request.py:178-193`).
- Durable: all 29 D2 capability channels remain open; no channel-closure
  mechanism exists (`docs/specs/SANDBOX_CAPABILITY_ENVELOPE_V1.md:25-38`).
- Durable: operation/feature negotiation, observed-target-revision binding,
  the `dharma_swarm` adapter, and a real foreign receipt do NOT exist
  (`VISION.md:82-88`; acceptance criterion 7 OPEN at
  `docs/plans/VIBE_HALT_BUILD_PLAN_2026-07-20.md:35`). They remain
  prerequisites for any production or `dharma_swarm` receipt claim
  (`docs/governance/ACTIVE_TRACK.yaml:91`).

**Reality Bridge terminal state — absent:**

- Durable: the VB-008 R4 decision packet merged via PR #87 is explicitly
  "BLOCKED — NOT AUTHORIZATION READY — NON-TERMINAL" and grants no foreign
  execution authority (`docs/audits/VB008_R4_DECISION_PACKET_2026-08-05.md:3-9`).
  It is a preparation packet only.
- **Truth Kernel entry interpretation (fail-closed):** issue #62 follows
  issue #60's confirmation-or-null critical path
  (`docs/DEVELOPMENT_WORKFLOW.md:211-218`). Therefore this controller accepts
  only a human-merged result labeled
  `REALITY_BRIDGE_COMPLETE_FORWARD_CONFIRMED` or
  `REALITY_BRIDGE_COMPLETE_FORWARD_NULL`, plus a separate current
  `OwnerAccountIssueAuthorityV0` record (§4) that #62 may proceed.
  `REALITY_BRIDGE_LOCAL_READY_TARGET_AUTHORITY_REQUIRED`,
  wave-ready labels, and `BLOCKED_*` are truthful handoff/pause labels in the
  Reality Bridge controller
  (`docs/prompts/VIBE_HALT_REALITY_BRIDGE_LONG_RUNNING_GOAL_2026-07-29.md:702-718`),
  but they do not satisfy this gate. Changing that predicate requires a
  human-merged amendment to this controller and
  `docs/governance/ACTIVE_TRACK.yaml`, plus a corresponding human update to
  issue #62; an issue edit alone cannot waive G0.
  No accepted completion result exists; the VB-008 fixture remains
  `candidate_state=UNRUN`, `bridge_execution=null` in the CALIBRATION cohort
  (`corpus/calibration/vb008_langgraph_6491.json:1`).
- OBSERVED: issue #60 (P0 operator gate for any foreign-target confirmation)
  is OPEN/blocked; the gate law is durable at
  `docs/governance/ACTIVE_TRACK.yaml:92`.
- OBSERVED: issue #62 is OPEN and requires reconsidering PR #58 only after a
  merged Reality Bridge terminal state
  (`docs/DEVELOPMENT_WORKFLOW.md:217-218`).

**Consequence for this controller:** this repair may make the controller
decision-ready in design, but PR #58 remains draft and non-merge-admissible.
The §8 root gate binds the merged result commit and the separate #62 human
clearance before `A1`. Even a later human merge of this controller authorizes
only bounded repository-local implementation packages whose entry gates are true
— never merge/self-approval, never foreign execution, never a production or
`dharma_swarm` receipt claim while the prerequisites above stand.

**Governance capacity:**

- Durable: vibe-halt runs 3 ACTIVE tracks at `wip_max: 3`
  (`docs/governance/ACTIVE_TRACK.yaml:6-8`); this campaign runs under
  `vibe-halt-core-2026-07` next-items, not a fourth track.
- Durable: the vibe-halt acceptance criterion "one end-to-end dharma_swarm
  receipt via a VibeHaltSandbox adapter" exists
  (`docs/governance/ACTIVE_TRACK.yaml:87`); K3B is its honest fulfillment
  path and is blocked by the same missing prerequisites
  (`docs/plans/VIBE_HALT_BUILD_PLAN_2026-07-20.md:35`).
- UNVERIFIED INPUTS, refresh in K3A: the dharma_swarm receipt mount point,
  active-track portfolio, protected-workflow options, and owned surfaces must
  be re-observed in that repository and cited there before an admission
  charter is proposed.

If any statement has changed, record exact evidence and re-plan before
editing.

## 4. Authority semantics — a Rust-validated product type

K1 defines, and K2 implements in Rust, a canonical
`tenancy-admission-decision-v0` record. It is a PRODUCT of independent axes,
not an ordinal evidence ladder and not free-form strings:

| axis | closed states |
|---|---|
| `integrity` | `I0_ABSENT`, `I1_MALFORMED`, `I2_REPLAY_MISMATCH`, `I3_INTEGRITY_OK` |
| `provenance` | `P0_UNTRUSTED`, `P1_STALE`, `P2_TRUSTED_EXECUTION_OK`, `P3_ATTESTATION_OK` |
| `verdict` | `V_CLEAN`, `V_FINDINGS`, `V_UNCHECKED`, `V_ERROR` |
| `admission_authority` | `AA0_NONE`, `AA2_CHAIN_CURRENT` |
| `gate_policy` | `GP0_ADVISORY_ONLY`, `GP3_REQUIRED(GP3PolicyRefV0, GatePolicyObservationV0)` |
| `external_authority` | `EA0_NONE`, `EA4_PRESENT` |

Authority decisions are cumulative references, not a single enum that loses
an earlier decision when a later one occurs. Every merged decision node uses
an immutable `AuthorityRefV0` containing canonical repository id, decision
kind/id, governed artifact path + blob digest, canonical decision-payload
digest, predecessor-DAG digest, protected merge commit, authorization-account
event kind/id/payload digest, stable actor-account node id/type, protected
authorization-policy digest, and observed-at value. The authorization event
and merge are independently resolved: a bot-performed mechanical merge may
only follow a still-current qualifying human authorization under protected
policy; it does not turn the bot into the authorizer. A
checkpoint, tenant request, PR-authored comment, model output, puzzle,
consensus, or self-signed record cannot construct authority.

Mechanically, a GitHub `User` type and stable node id establish only account
attribution. They do not prove biological human presence, attention, cognition,
or freedom from PAT/service automation. In this controller, repository policy
trusts a qualifying authorized account as the authority principal; every
instruction saying a human must merge/authorize is a normative operator rule,
not a claim that the evaluator can detect personhood. Any future human-presence
ceremony would require a separately typed policy and cannot be inferred here.

The separate #62 clearance is an `OwnerAccountIssueAuthorityV0` containing canonical
repository + issue ids, allowed event kind + immutable event id, canonical
payload bytes + digest and unchanged event-object `created_at`/`updated_at`,
bound G0 result SHA + trusted G0 `merged_at`, actor node id and `User` type,
GitHub-recorded `author_association`, the qualifying permission value and
time-bounded observation recorded by the human merger, authorization-policy
digest, and creation time. The bootstrap block below freezes canonicalization,
freshness, and the 512-byte payload ceiling; K1 later codifies them unchanged.
Provenance of a GitHub event is insufficient: the protected checker also
requires that the actor was demonstrably associated when posting, was observed
with qualifying permission immediately around the fresh post-G0 comment, is
still authorized by the protected policy, and that the event still exists, is
unedited, and has not been deleted or superseded. A pre-G0 or stale comment,
public commenter, bot/App, wrong event kind/issue, edited/deleted event, later-
promoted actor, or actor whose permission was removed cannot clear #62.

<!-- TRUTH_KERNEL_BOOTSTRAP_POLICY_V0_BEGIN -->
**Bootstrap clearance policy v0 (frozen here, before K1).** The sole allowed
#62 event is a newly created, never edited GitHub issue comment on issue 62.
Its UTF-8 body is one canonical JSON line, no leading/trailing whitespace or
extra keys, in this exact key order and shape:

```json
{"record":"truth-kernel-issue-62-clearance","schema":1,"decision":"PROCEED","bound_g0_result_sha":"<40-lowercase-hex>"}
```

The bound SHA is the protected merge commit of the current accepted G0 result.
For G0/#62 bootstrap-record construction through the final permission
observation, but excluding the separate final-review resolver below, the human
merger uses only these read-only REST objects under
`repos/AmitabhainArunachala/vibe-halt`: the repository object; default-branch
Git ref/commit/tree and exact G0 artifact blob; `issues/62`;
`issues/comments/{comment_id}`; `collaborators/{login}/permission`; the G0
result `pulls/{number}` + merge commit; and
`compare/{g0_merge_sha}...{default_tip_sha}`. The procedure fetches the
comment by immutable id, its author's stable node id/type and persisted
`author_association`, and the author's current repository permission. It first
requires the trusted G0 PR object to say `merged == true`, its `merge_commit_sha`
to equal the payload SHA, and a non-null `merged_at`; then requires comment
`created_at` to be strictly after that `merged_at`, no more than 15 minutes
before the first permission observation, and equal `updated_at`. The persisted
association must be exactly `OWNER`; permission observed both in that first
check and again immediately before A1 merge must be exactly `admin`. This
deliberately owner-only bootstrap is the historical authorization proof:
membership or collaboration followed by promotion is never sufficient. A1's
decision payload records the G0 `merged_at`,
association, both permission values/timestamps and response-object digests,
comment id/payload digest, exact result SHA, and current-main ancestry/artifact
presence. A permission check preceding the comment, a comment outside the
15-minute window, or later acquisition of permission cannot be grandfathered;
the actor must post a new exact comment. After A1, the protected checker repeats
the current-state object queries, requires current qualifying permission, and
validates the recorded initial observation and this exact policy-block digest
from the human-merged A1 blob. K1 may codify this procedure and K2 may automate
it, but neither may choose a different event, payload, freshness window,
authorization rule, or policy source. Until K2 exists no machine may construct
`AA2_CHAIN_CURRENT`; the pre-A1 procedure is a human merge prerequisite, not a
self-authorizing automated result.

The historical observation is durable only in the exact A1 bootstrap record
`docs/governance/TRUTH_KERNEL_A1_BOOTSTRAP_AUTHORITY_V0.json`. PR #58 MUST
remain draft and non-merge-admissible until that previously absent path is
added as one regular file with exactly one UTF-8 JSON object plus one final LF,
no duplicate/unknown keys, and the following key order and closed shape:

```json
{"record":"truth-kernel-a1-bootstrap-authority","schema":1,"repository":"AmitabhainArunachala/vibe-halt","controller_path":"docs/prompts/TRUTH_KERNEL_LONG_RUNNING_GOAL_2026-07-29.md","controller_blob":"<40-lowercase-hex>","bootstrap_policy_sha256":"<64-lowercase-hex>","g0_pr":0,"g0_merge_sha":"<40-lowercase-hex>","g0_merged_at":"<RFC3339>","g0_artifact_path":"<repository-relative-path>","g0_artifact_blob":"<40-lowercase-hex>","issue_id":62,"comment_id":0,"event_kind":"issue_comment","comment_created_at":"<RFC3339>","comment_updated_at":"<same-RFC3339>","canonical_comment_payload":"{\"record\":\"truth-kernel-issue-62-clearance\",\"schema\":1,\"decision\":\"PROCEED\",\"bound_g0_result_sha\":\"<40-lowercase-hex>\"}","comment_payload_sha256":"<64-lowercase-hex>","actor_node_id":"<stable-node-id>","actor_type":"User","author_association":"OWNER","initial_permission":"admin","initial_permission_checked_at":"<YYYY-MM-DDTHH:MM:SSZ>","initial_permission_response_sha256":"<64-lowercase-hex>","premerge_permission":"admin","premerge_permission_checked_at":"<YYYY-MM-DDTHH:MM:SSZ>","premerge_permission_response_sha256":"<64-lowercase-hex>","observed_default_tip_sha":"<40-lowercase-hex>","g0_ancestor_of_default":true}
```

Each permission-response digest is SHA-256 over exactly one UTF-8 JSON line
with no final LF, whitespace, escaping variation, duplicate/unknown keys, or
alternate key order, in this closed shape:

```json
{"record":"github-repository-permission-observation","schema":1,"repository_node_id":"<stable-node-id>","actor_node_id":"<stable-node-id>","permission":"admin","observed_at":"<YYYY-MM-DDTHH:MM:SSZ>"}
```

`observed_at` is the corresponding record timestamp, normalized to UTC with
literal `Z`, whole seconds, and exactly the shown `YYYY-MM-DDTHH:MM:SSZ`
width; the other values are copied from the authenticated repository,
comment-author, and collaborator-permission objects. The record's
`bootstrap_policy_sha256` is SHA-256 over the exact raw
UTF-8 bytes of this controller blob beginning immediately after the LF that
terminates the sole literal
`<!-- TRUTH_KERNEL_BOOTSTRAP_POLICY_V0_BEGIN -->` line and ending immediately
before the first byte (`<`) of the sole literal
`<!-- TRUTH_KERNEL_BOOTSTRAP_POLICY_V0_END -->` line. The hashed bytes include
every intervening byte, including the LF immediately before the end delimiter;
both delimiter lines are excluded. CRLF conversion, Unicode normalization,
whitespace trimming, or a missing/duplicate delimiter invalidates the record.
The A1 `AuthorityRefV0` governed artifact is this record path/blob; its
decision payload binds the controller path/blob, G0 artifact/merge, and issue
event fields inside the record. The human merger repeats the premerge query
after the record is added and rejects any controller/record/event/head drift.
K2 reconstructs and validates the protected record blob and every current
object; a missing, malformed, placeholder-bearing, post-observation-edited, or
unbound record is `AA0_NONE`.

After that record is final, the sole accepted A1 merge-authorization event is one
currently `APPROVED`, never-edited, non-dismissed GitHub pull-request review
on PR #58 by the same `OWNER`/`admin` `User`. Its body is one canonical JSON
line, no extra keys or surrounding whitespace, at most 512 bytes, in this
exact order:

```json
{"record":"truth-kernel-a1-final-authorization","schema":1,"decision":"AUTHORIZE_A1_MERGE","repository":"AmitabhainArunachala/vibe-halt","pr":58,"base_sha":"<40-lowercase-hex>","head_sha":"<40-lowercase-hex>","controller_blob":"<40-lowercase-hex>","bootstrap_record_blob":"<40-lowercase-hex>"}
```

The final-review/merge resolver is outside the earlier G0/#62 REST-only object
set. It adds only the authenticated repository object; `pulls/58`, the
complete paginated `pulls/58/reviews?per_page=100&page=N` collection, and each
candidate `pulls/58/reviews/{review_id}`; the review actor's collaborator-permission
object; the protected default-branch Git ref; Git commit objects for the
payload base/head and, after merge, the resulting merge commit; a complete,
non-truncated component-by-component Git-tree walk from the head/merge trees;
the controller and bootstrap-record Git blob objects; and the same review's
GraphQL `PullRequestReview` node joined by REST `node_id` for every otherwise
matching candidate. The review id is resolver-selected, never caller-supplied.
A missing object, truncated/ambiguous tree, non-regular path, or path/blob
mismatch fails closed.

The resolver walks review pages from `N=1` through the authenticated `next`
links until the final page, rejects duplicate ids, pagination-link/page-number
discontinuity or errors (review ids themselves need not be contiguous),
changing pages, or more than 1,000 total reviews, and requires two consecutive
complete enumerations with the same sorted ids and current field projections.
From that complete set it first filters exact repository/PR, canonical body, `APPROVED`
state, reviewed head, same actor node/type/`OWNER` association, and submitted-
time fields; then it resolves the GraphQL edit fields for every survivor. There
must be exactly one candidate satisfying every REST/GraphQL predicate below;
zero or multiple candidates has no constructor. Immediately before merge it
requires the current REST and GraphQL bodies to be the canonical bytes above,
both states to be `APPROVED`, REST `commit_id` to equal that head, GraphQL
`submittedAt` to equal REST `submitted_at`, GraphQL `lastEditedAt == null`,
`editor == null`, and `userContentEdits.totalCount == 0`. It canonicalizes and
hashes exactly one UTF-8 JSON line with no final LF, alternate key order,
duplicate/unknown key, or timestamp variation, in this closed shape:

```json
{"record":"github-pull-request-review-authorization-observation","schema":1,"phase":"PRE_MERGE","repository_node_id":"<stable-node-id>","pull_request_node_id":"<stable-node-id>","review_id":0,"review_node_id":"<stable-node-id>","body_sha256":"<64-lowercase-hex>","state":"APPROVED","commit_id":"<40-lowercase-hex>","author_node_id":"<stable-node-id>","author_type":"User","author_association":"OWNER","submitted_at":"<YYYY-MM-DDTHH:MM:SSZ>","last_edited_at":null,"editor_node_id":null,"user_content_edits_total_count":0,"observed_at":"<YYYY-MM-DDTHH:MM:SSZ>"}
```

Both timestamps use the same literal-`Z`, whole-second normalization as the
permission projections. The resolver also emits exactly one UTF-8 JSON line,
with no final LF or representation variation, in this closed shape:

```json
{"record":"github-pull-request-review-candidate-set","schema":1,"phase":"PRE_MERGE","repository_node_id":"<stable-node-id>","pull_request_node_id":"<stable-node-id>","head_sha":"<40-lowercase-hex>","enumerated_review_count":0,"candidate_count":1,"candidate_review_ids":[0],"candidate_projection_sha256":["<64-lowercase-hex>"],"observed_at":"<YYYY-MM-DDTHH:MM:SSZ>"}
```

The singleton arrays are ascending by numeric review id; their sole digest is
the exact observation projection above, and both `observed_at` values are
identical. `phase` is a closed enum of `PRE_MERGE` and `POST_MERGE`; the
post-merge recheck below emits the same two schemas with `POST_MERGE`, a fresh
`observed_at`, and a freshly computed projection digest. The full A1
`AuthorityRefV0` binds both pre- and post-merge observation/candidate-set
digests and counts, selected review id, and raw canonical-body digest. A
missing field, join mismatch, nonzero edit count, pagination/error, or
REST/GraphQL disagreement fails closed.

The resolver also requires the recorded `observed_default_tip_sha` to equal
the payload base; the PR base repository/ref to equal this repository's
protected default branch (the mutable REST `base.sha` is not used as the
current-tip authority); the independently fetched protected ref tip to equal
`base_sha`; `base_sha` to be an ancestor of the PR `head.sha`; and PR
`head.sha`, the two exact head-tree paths, and their blob ids/content to equal
the payload. The final permission observation must be no more than five
minutes before the review. In exact timestamp terms,
`0 <= review.submitted_at - premerge_permission_checked_at <= 300 seconds`.
The projection `observed_at` is at or after `review.submitted_at` and before
merge. No authorization input may change before the post-merge recheck; only
the expected PR/default-ref merge transition may occur. A1 uses merge-commit mode only:
immediately before merge the protected default tip still equals `base_sha`.

Immediately after merge, the authenticated PR object must have
`merged == true`, non-null `merged_at`, and `merge_commit_sha` equal the
resolved merge commit; `0 <= merged_at - review.submitted_at <= 300 seconds`;
the freshly observed protected default ref tip must equal that merge commit;
and the commit must have exactly first parent `base_sha`, second parent
`head_sha`, and the reviewed head tree. Before constructing A1, the resolver
must then repeat the complete two-pass review enumeration, selected REST/
GraphQL review fetch, and collaborator-permission query. It requires exactly
the same sole selected review and every static projection field unchanged,
current permission still `admin`, and only `phase`/fresh `observed_at` to
differ; any new duplicate candidate, dismissal, edit, deletion, actor/body/
head change, or permission loss leaves the already merged PR without an A1
authority constructor. The A1 `AuthorityRefV0` binds the PR node/id,
`merged_at`, merge-commit id/object digest, both parents/tree, that post-merge
ref observation, and the post-merge review-set/review/permission observation
digests. On every later resolution the A1 merge must
remain an ancestor of the current protected default tip and the governed A1
record must remain current and unreverted; a later default-branch advance is
not itself invalidation. Any head/base/blob/
review-state/actor/permission movement requires a new premerge observation,
regenerated bootstrap record when its fields changed, and new approval review.
The full A1 `AuthorityRefV0` also binds the final record path/blob and the exact
merge commit/parent relation. A stale, edited, dismissed, wrong-commit,
squash/rebase, or post-review-mutated A1 has no authority constructor.
<!-- TRUTH_KERNEL_BOOTSTRAP_POLICY_V0_END -->

Current validity never comes from a caller-carried optional revocation field.
K1 freezes, and K2 implements, a protected `AuthorityRegistryViewV0` keyed by
decision kind + governed subject. Its platform component is the one current
`authority-registry-v0` on vibe-halt's protected default branch. Each
human-merged tenant registration additionally pins the canonical tenant
repository, protected default branch, exact A2 artifact path/schema, exact GP3
policy artifact path/schema, and one current-object resolver policy covering
both paths. A tenant A2 node and any GP3 policy are therefore resolved directly
from that repository's protected current default-branch tree plus their
immutable PR/merge and qualifying authorized-account events; callers cannot
provide a registry snapshot or resolver. The resolver returns
`GP0_ADVISORY_ONLY` for an absent GP3 path only when protected history contains
no qualifying GP3 merge/event. A present path must be one regular current blob
whose schema and full `GP3PolicyRefV0` validate. Once any qualifying GP3 event
exists, an absent, removed, reverted, duplicated, wrong-path, wrong-schema, or
superseded policy is a typed authority error, never fallback to GP0. This
distributed protected view lets a post-K2 K3A, GP3 policy, or K4A become current
without a circular platform-registry update.
Immediately before admission the checker independently reads every current
component and GitHub object, then rejects a node whose governed artifact/
payload is removed, reverted, superseded, revoked, policy-invalidated,
non-ancestor, or no longer current. Requests and attestations bind every
resolved registry/resolver commit, blob, policy, and digest but cannot select
them. Any component's unavailability is `AA0_NONE`, never use of a cached ref.

`TenantAuthorityKeyV0` is the canonical tuple `{ tenant_id,
canonical_repository, operation_id, registration_digest }`. It appears
unchanged in the selected registry entry, source binding/classified diff,
registration authority node, and tenant A2 node.

`GP3PolicyRefV0` is a separately human-merged, current authority-policy ref;
it is not an execution-subject ref. It contains the underlying full
`AuthorityRefV0`, the exact `TenantAuthorityKeyV0`, canonical check id,
authority-DAG digest, repository, and the closed allowed-subject set
`{PR_SYNTHETIC_MERGE, MERGE_GROUP}`, plus an expected
`GateEnforcementRefV0`. That enforcement ref is closed to
`github-ruleset-v1` and binds repository node id, stable ruleset id, exact
protected target ref pattern, required check name + **dedicated publisher App**
integration node/id and issuer, canonical bypass-actor-set digest, the closed
activation-event schema/resolver-policy digest, and a canonical desired-config
projection digest over those immutable policy fields. The projection excludes
active/inactive state, provider object revision/current digest, timestamps, and
future event identity, so activation cannot self-invalidate the merged policy.
It does not bind a future event instance. The publisher credential is the
separately human-authorized least-privilege check-result writer held only by the
protected controller (§5); no PR workflow or worker can access it. A generic
GitHub Actions integration shared with tenant-modifiable workflows is never a
valid GP3 publisher, even if the check name matches.
The artifact requests policy; only the actual provider enforcement object can
make it effective.

For v0 the canonical bypass set is exactly empty: no actor, team, App, role,
administrator, emergency path, or implicit provider bypass may merge without
the named check. If the provider cannot expose and enforce that invariant, GP3
is unavailable. The activation carrier is a closed
`RulesetAuthorizationEventV0 { provider: github, repository_node_id,
audit_event_id, action: ACTIVATE, ruleset_id, actor_node_id, actor_type: User,
before_object_digest, after_object_digest, occurred_at }`, resolved only from
the provider's immutable protected audit object. It must occur after the GP3
policy merge, name the same stable ruleset id, carry the exact current after-
digest, and satisfy the protected authorized-account policy. An unavailable,
mutable, caller-carried, wrong-actor/action/object, or unresolvable audit event
has no constructor.

The protected checker resolves the durable policy from the current protected
authority-registry view and independently re-fetches the named ruleset object.
Only if its active enforcement state is exact and its target, required-check
tuple, integration, and bypass projection recomputes the policy's desired-
config digest does the checker emit a `GatePolicyObservationV0`. That
observation binds the policy-ref digest, enforcement-object id/full current
object digest and revision, recomputed config-projection digest, actual check-run
publisher App identity, `RulesetAuthorizationEventV0` digest, trusted
observation time, and exact current `ExecutionSubjectV0` for this evaluation.
It has no
constructor for `PR_HEAD`, an inactive/evaluate-only/deleted ruleset, a missing
or substituted required check, a shared/spoofable/wrong publisher, a broader
target/bypass, or any subject outside the policy set. A policy without matching live enforcement is a typed
gate-policy error and not-green, never GP0 or claimed-required. `GP0` exists
only while the registered GP3 path is absent and no qualifying GP3 event ever
occurred.

The GP3 policy's own merge may advance the repository base; that never
invalidates the policy or purports to pre-authorize its merge subject, because
only a later checker observation binds an execution subject and live
enforcement. Its repository/path/schema must equal the registration-pinned
tenant resolver above; a valid policy ref at any other location carries no gate
authority. Creating, changing, or activating the provider ruleset is a separate
human platform act that no agent or artifact in this campaign is authorized to
perform.

`GP3_REQUIRED` is therefore a pre-merge claim that exact required-policy
configuration was freshly observed, not proof that the provider ultimately
enforced it. The stronger post-merge observation is
`GateEnforcementOutcomeV0 { provider, immutable_merge_event_id,
repository_node_id, pr_or_merge_group_id, execution_subject_digest,
ruleset_id, ruleset_object_digest, check_run_id, check_run_attempt,
publisher_app_node_id, bypass_used: false, merge_commit, merged_at }`. It is
constructed only from a provider-authenticated merge/audit event that binds the
same subject, exact ruleset version, dedicated check run/publisher, resulting
merge, and `bypass_used == false`, followed by a current-object recheck. If the
provider cannot produce that closed event, no component may claim the check
actually gated the merge; GP3 remains a required-policy observation with that
explicit limitation.

`AA2_CHAIN_CURRENT` carries a tenant-parameterized `AuthorityChainV0` DAG:

1. `platform`: accepted G0 result + protected result merge, exact
   `OwnerAccountIssueAuthorityV0` #62 clearance, this controller's A1 ref, K1 contract
   ref, and the compatible current K2 bridge ref;
2. `registration`: the human-merged registry node whose governed entry owns
   the selected composite registration digest, tenant id, canonical tenant
   repository, and operation id. It declares K1 as a predecessor and exact
   compatibility with current K2; the bootstrap K2R also precedes initial K2,
   but later tenant registrations may descend from K2; and
3. `tenant_admission`: the current A2 charter ref in that exact tenant
   repository, binding the same tenant/repository/operation/registration plus
   the platform and registration predecessor digests.

For the first K3B instance, `registration` is its K2R merge and
`tenant_admission` is dharma_swarm's K3A merge. Every K4 implementation tenant
supplies its own human-merged K2R-class registration and its own admission ref;
the schema does not change. The DAG carries every full node and canonical DAG
digest. The checker re-resolves each node, current registry-view state, expected
repository, subject, predecessor edges, ancestry/order constraints, and human
policy. Missing, substituted, revoked, reverted, superseded, duplicated,
cross-tenant, or invalidly ordered nodes return `AA0_NONE`.

**Closed admission constructor:** advisory green exists only for

```text
AdvisoryGreenV0 {
  integrity: I3_INTEGRITY_OK,
  provenance: P2_TRUSTED_EXECUTION_OK | P3_ATTESTATION_OK,
  verdict: V_CLEAN,
  admission_authority: AA2_CHAIN_CURRENT(AuthorityChainV0),
  gate_policy: GP0_ADVISORY_ONLY | GP3_REQUIRED(GP3PolicyRefV0,
                GatePolicyObservationV0),
  external_authority: EA0_NONE,
  source_binding: ExactCurrentSubject,
  registration: ExactProtectedRegistryPin,
}
```

The constructor has mandatory cross-field equality of the whole key:

```text
authority_chain.registration.authority_key
  == authority_chain.tenant_admission.authority_key
  == source_binding.authority_key
  == registration.authority_key
```

When `gate_policy == GP3_REQUIRED(policy_ref, observation)`, the durable
policy ref must equal the same whole authority key, canonical check id,
authority-DAG digest, repository, and exact enforcement identity. The
separately constructed observation must bind that exact policy-ref digest,
freshly equal live enforcement object, and this evaluation's exact current
subject. Required merge admission accepts only a current
`PR_SYNTHETIC_MERGE` or `MERGE_GROUP`; `PR_HEAD` remains advisory-only. A GP3
policy for another tenant/check/DAG/repository, an observation for another or
stale subject/policy/enforcement object, or a required decision over `PR_HEAD`,
has no constructor and is a named red fixture.

There is no constructor from attestation alone, fresh replay alone, an A2 ref
without its current prerequisite chain, or any other partial product. Under
`GP0_ADVISORY_ONLY`, green is evidence only. A separately human-ratified and
freshly revalidated `GP3PolicyRefV0`, paired with a protected-checker-created
live-enforcement-and-exact-current-subject `GatePolicyObservationV0`, may make
an otherwise identical synthetic-merge/merge-group green result eligible for
provider-required enforcement without replacing or erasing
`AA2_CHAIN_CURRENT`; only `GateEnforcementOutcomeV0` supports the later claim
that enforcement occurred.
`EA4_PRESENT` is
outside every package in this controller. K1 must specify canonical bytes,
the exact transition table, chain ordering and revocation semantics, and
negative fixtures; K2 must implement the Rust parser/validator and prove
unknown states/keys/transitions and missing/substituted/revoked/out-of-order
predecessors, pre-G0/stale/later-promoted/unauthorized/edited authorized-account events,
stale current-registry state, and tenant-A registration/admission refs
substituted into tenant B fail before execution.

The canonical precedent is existing repo law: green automation is evidence,
not authority (`docs/DEVELOPMENT_WORKFLOW.md:178-181`), and dossiers may not
award credit (`docs/specs/HOLDOUT_CONTRACT_V1.md:7-11`).

## 5. Provenance and admission authority (finding 1: no self-minted provenance)

A PR-author-supplied, self-consistent v2 bundle CANNOT establish engine
provenance. The v2 verifier covers the complete declared bundle content —
schema, content digest, finding identity, shrink-lineage consistency
(`crates/vh-cli/src/receipts_v2.rs:1-35`, parser at
`crates/vh-cli/src/receipts_v2.rs:258`) — and `vh replay-bundle`
re-executes what it is given (`crates/vh-cli/src/main.rs:146`). Both prove
the bundle is internally consistent and reproducible. Neither proves that a
trusted engine, built from the claimed source, running against the claimed
target revision, produced it. An author who can write a bundle can write a
self-consistent one.

Admission-grade provenance therefore comes from exactly one of two paths.
Both use a protected policy owned outside the tenant PR. That policy pins the
checker, adapter, registration, authority-registry view, trusted issuer/audience,
maximum age, nonce domain, current-subject comparison rule, and the complete
`ExecutionMaterialsV0` closure defined below. Tenant bytes may select none of
those trust roots, including `engine_path`, expected digest, checker, workflow,
signer, source observation, nonce, authority snapshot, contract, fault policy,
or budget. The current hosted CI explicitly disclaims being a security boundary
and executes contributor-controlled code (`.github/workflows/ci.yml:3-9`),
so its present PR job is not Path T merely because it is CI.

**Transitive execution-material law.** `ExecutionMaterialsV0` is a canonical,
content-addressed closure over the protected controller/checker; every local
action, composite action, recursively called reusable workflow, script,
configuration, and loaded helper/plugin; runner or container image digest and
runtime libraries; compiler/linker/interpreter/toolchain digests; engine source
commit/tree, build recipe/environment; dependency lock graph plus vendored or
offline source/checksum set; and final executable digest or authenticated
release-manifest subject. The protected resolver expands every reference to an
immutable digest before work starts and binds the ordered manifest + closure
digest into the result. Floating action tags, mutable images, unenumerated
shared libraries/plugins, build scripts outside the closure, or network/dynamic
acquisition after materialization are pre-execution errors. If the platform
cannot expose a required material identity, admission provenance is
unavailable; it is never inferred. At least one red fixture substitutes a
valid-looking mutable transitive action/dependency while direct pins remain
unchanged.

**Path T — protected trusted execution (the implementable default).** A
protected default-branch workflow, protected reusable workflow, or separately
protected App/controller — pinned by immutable identity — orchestrates the
decision. Neither its checker nor any helper that computes admission comes
from the PR checkout. It:

1. constructs exactly one closed `ExecutionSubjectV0` variant from protected
   current state (§6), including its repository, base/head or merge-group
   relation, execution SHA, and tree;
2. computes the complete base→subject Git-tree entry diff in the object
   database and canonically records every add/delete/content/mode transition
   (renames are delete+add). Protected policy classifies each entry exactly
   once as a registry-covered surface or an explicit versioned non-impacting
   class. Unknown, overlapping, uncovered, mixed-incompatible, gitlink,
   executable-mode, dependency, workflow, configuration, generated-input, or
   delete/rename transition not explicitly covered is not-green before work;
3. selects the covered source only from protected allowlists of lexically
   normalized repository-relative paths, then reads bounded regular blob
   objects directly from the pinned Git tree/object database. It never follows
   or reads through the PR filesystem checkout. Absolute paths, `..`, symlinks,
   gitlinks, submodules, special-file modes, duplicate canonical paths,
   overlong names, excess file count, excess per-file bytes, or excess aggregate
   bytes are rejected before any worker starts;
4. obtains the engine only from the resolved `ExecutionMaterialsV0`: a clean
   offline build from its exact protected source/material closure or an
   authenticated release manifest whose subject contains the exact executable
   digest; hashing a tenant-selected binary is never provenance;
5. sends only those extracted tenant bytes, the canonical request + nonce,
   and the already resolved pinned engine/adapter/material payload to an
   unprivileged, disposable, **zero-credential** worker: no secrets, tokens of
   any scope, OIDC request capability, cloud/runner metadata service, SSH agent,
   credential helper/file, inherited authorization environment, or
   credential-bearing socket/mount. It executes the pinned engine/adapter
   there under §7; the
   protected controller never executes PR code and consumes only bounded
   machine data from the worker. Any minimal credential needed to publish a
   check remains solely in the protected controller, is scoped to that write,
   and never enters the worker;
6. binds run id, attempt, job/controller id, event, protected workflow/helper
   definition commit, trusted start/end time, nonce, current conclusion,
   closed source subject, complete changed-entry/classification manifest,
   source extraction manifest, `ExecutionMaterialsV0`, engine/adapter/
   registration identities, request, bundle, typed decision, full
   `AuthorityChainV0`, current authority-registry-view identity, and digests; and
7. freshly re-resolves the full authority chain/current registry view and rejects cancellation,
   supersession, expiry, replay, current-head/base drift, chain drift or
   revocation, or a second consumption of the nonce.

If an implementation uses `pull_request_target`, it may orchestrate only; it
must never execute or source the untrusted checkout while a privileged token
or secret is present. D2 remains D2: the disposable worker is an OUTER safety
boundary with an explicit mount/socket/device/egress/process/memory/disk/time
policy, not evidence that any of the 29 channels closed. A mandatory red-first
credential-inventory fixture inspects environment keys/redacted presence,
mounted file paths and modes (never contents), credential helpers,
sockets/agents, OIDC endpoints, and metadata routes from inside the worker
before tenant bytes are admitted; it never logs a credential value. Any
credential source or unreachable-to-prove inventory is a pre-execution error.

**Path A — protected trusted-workflow attestation.** A workflow/controller
meeting the same protected-checker and worker rules produces an authenticated
attestation binding, at minimum: issuer, audience, signer/controller identity,
workflow/helper repository/path/ref/definition commit, run id + attempt + job
id, event, the closed `ExecutionSubjectV0`, complete changed-entry +
classification manifest/digest, `ExecutionMaterialsV0` manifest/digest,
engine repository + source commit + tree + executable digest, authenticated
release-manifest subject if used, adapter and composite registration digests, request digest,
property-contract digest, fault-policy/parameter/budget digests, bundle
digest, canonical source-extraction manifest + bounds, full authority refs +
chain digest, product-typed conclusion, start/end/issued/expiry values, and
nonce/replay id. The consuming protected checker verifies those fields, every
authority predecessor, and current GitHub subject state against protected
policy BEFORE reading a bundle.

**K1 scope discipline:** K1 defines the Path T record schema, Path A
attestation schema, protected trust policy, and negative fixtures — and does
NOT choose a signing platform or name a product the repository does not have.
Mechanism selection is a
later, separately human-reviewed implementation package. Path T requires no
such mechanism and MUST remain sufficient on its own.

**Negative fixtures (K1 defines; K2/K3B implement as red tests):** PR edits
the workflow/checker/helper; wrong App/check identity; wrong repository/PR;
wrong workflow/ref/definition commit; self-selected engine binary plus its
fresh hash; unapproved engine source/release; wrong base, PR head, checkout,
tree, or merge-group relation; expired/cancelled/superseded execution;
replayed nonce or run attempt; bundle mismatch; same id/version with changed
registration/contract/fault-policy content; weakened/omitted budget;
caller-declared metadata presented as observed; missing/wrong issuer,
audience, or signer; forged authority reference; and verdict laundering.
Also mandatory: absent/substituted/revoked/out-of-order authority predecessor;
absolute/traversing/duplicate source path; symlink, gitlink/submodule, or
special-file mode; object/checkout byte disagreement; count/size overflow;
uncovered/overlapping/mixed changed surface; rename/delete/mode/generated-input/
dependency/workflow transition; mutable transitive execution material; and any
worker environment/file/socket/OIDC/metadata credential. Path T and
Path A each carry the applicable stale/retry/replay fixtures.

**Local verification semantics, stated permanently:** `verify_bundle`-shaped
re-verification (K2) and `vh replay-bundle` are integrity/replay checks with
ceiling `I3_INTEGRITY_OK` on the integrity axis and `P0_UNTRUSTED` provenance
absent a separate protected path. They are never admission authority by
themselves, in any wave, for any tenant.

## 6. Exact source binding (finding 3: verifier-observed identity only)

The Rust-owned tenancy request/envelope and protected checker bind
VERIFIER-OBSERVED source identity, never caller-declared metadata. K1 freezes
this closed tagged union, with unknown tags/extra fields rejected:

| `ExecutionSubjectV0` variant | required relation | forbidden / authority ceiling |
|---|---|---|
| `PR_HEAD` | tenant id, canonical repo id, PR id, current base + head, `execution_sha == head`, exact head tree | no merge-group/push fields; K3B-admissible |
| `PR_SYNTHETIC_MERGE` | same PR fields plus synthetic merge SHA/tree and protected-observed exact base+head parent relation; `execution_sha == merge_sha` | no group/push fields; K3B-admissible only while PR/base/head relation is current |
| `MERGE_GROUP` | tenant/repo/PR/head, current group id + head/base/tree, protected membership relation; `execution_sha == group_head` | no push fields; K3B-admissible only for the current group |
| `PUSH_POST_MERGE` | repo, protected ref, before/after SHA + tree; `execution_sha == after` | every PR/group field forbidden; post-merge evidence only, never PR advisory or merge admission |

`pull_request_target` is an orchestration envelope, not an execution-subject
variant; its base checkout cannot produce admission evidence. `workflow_run`,
`check_run`, `workflow_dispatch`, `repository_dispatch`, `schedule`, deleted or
recreated branch aliases, unrelated pushes, and every unknown/indirect event
are rejected for K3B admission. K3B accepts only `PR_HEAD`,
`PR_SYNTHETIC_MERGE`, or `MERGE_GROUP`. Existing CI already exposes event,
PR-head/base, checkout, and merge-group identities separately
(`.github/workflows/verify.yml:14-23,45-59`); `GITHUB_SHA` is never relabeled.

Every accepted subject then binds:

- **repository/tenant identity** — canonical host/owner/name and tenant id,
  resolved from protected current state or authenticated under Path A;
- **complete changed-entry identity** — canonical base→subject tree diff,
  every path/blob/mode transition and its exactly-one registry-surface or
  versioned non-impacting classification, plus manifest/classifier digests;
- **source-tree and canonical artifact digest** — pinned Git tree, protected
  allowlisted path set, regular-blob modes, ordered per-blob object ids/sizes,
  total count/bytes, extraction-manifest digest, and the digest of the
  canonical source-derived artifact consumed by the operation. Extraction is
  tree-object-only and no-follow under §5, never a filesystem walk;
- **typed source rejection** — dirty checkout or checkout/tree mismatch is
  `P1_STALE + V_ERROR`; disallowed/traversing path, symlink, gitlink/submodule,
  special-file mode, or count/size overflow is
  `I1_MALFORMED + P0_UNTRUSTED + V_ERROR`; extracted object/artifact digest
  mismatch is `I2_REPLAY_MISMATCH + P0_UNTRUSTED + V_ERROR`. Each returns the
  typed pre-execution CLI exit `2`, emits no admission bundle, and never maps
  to `CLEAN`;
- **coverage rejection** — an unclassified/overlapping path, unsupported
  mixed-surface set, rename/delete/mode/gitlink transition, or changed
  executable/configuration/dependency/workflow/generated input not explicitly
  covered by the selected registration is
  `I1_MALFORMED + P0_UNTRUSTED + V_ERROR`, exit `2`, before extraction; and
- **current comparison** — immediately before decision, the checker re-resolves
  the tagged variant and every required/forbidden field. Merge admission
  requires the designated synthetic-result/queue rerun. Any moved or missing
  relation is `P1_STALE + V_ERROR`, exit `2`, and not-green.

Caller-declared metadata is untrusted by construction. Today
`RunRequest.source_commit` is an optional caller-supplied string
(`clients/python/vibe_halt/core/request.py:82`); under this contract such
fields are request context, never evidence, and K2 must document them as
such. Stale evidence fails closed: an attestation or execution receipt
whose bound subject no longer equals protected current state is `P1_STALE`,
so every push, rebase, base movement, cancellation, or supersession
invalidates prior green automatically.

**Source-sensitivity law.** Observing and hashing source is necessary but not
sufficient: the source-derived bytes must be an input to the registered
operation and must be capable of changing the property result. K2 implements
the generic source-consuming operation and versioned operation/feature
negotiation against repo-local fixtures. K3B then proves an actual
dharma_swarm source mutant changes the result while an identity-only change
does not. A compiled-in model that returns the same verdict for every tenant
tree may be useful calibration, but can never produce admission green.

## 7. Registry-pinned contracts and fault models (findings 4 and 5)

**Non-weakenable property contract (finding 4).** The runner obtains a
workload's Rust-side contract and checks it per universe; workloads that use
the empty default cannot yield a tenancy-admissible `CLEAN`
(`crates/vh-multiverse/src/lib.rs:334-346,419-439`; a non-empty example is
`crates/vh-cli/src/workloads/mod.rs:58-61`; verdict law
`crates/vh-cli/src/main.rs:126-131`). K2R extends this into a versioned
registry: for each registered workload id, a property-contract id, version,
and content digest, plus the workload-to-contract compatibility mapping.
Tenants name a contract id+version in the request; they never supply
Always/Sometimes sets. An unknown id, a version mismatch, an incompatible
workload/contract pair, or any tenant-supplied property set is rejected or
rejected before execution; runtime inability to evaluate a valid pinned
contract is `V_UNCHECKED`. No tenant can select a weaker contract than the
registry pin for the surface its PR touches.

**Versioned fault model (finding 5).** The same registry owns, per covered
surface, the fault-model id, version, and content digest; the canonical
parameters (palette, fault kinds, counts, horizon — palette law at
`crates/vh-gremlin/src/lib.rs:23-32`, plan generation at
`crates/vh-gremlin/src/lib.rs:253-286`); and the fixed budget appropriate to
that surface (universes, seeds, wall-clock ceiling). Tenant weakening,
omitted or unknown fault models, parameter drift, or budget drift — in
EITHER direction, since fewer universes weakens evidence and more exceeds
the fixed ceiling — fail closed before execution. The request names the
fault model id+version; the engine independently resolves and binds its
content from the protected registry.

Fault selection is not fault manifestation. The registry therefore also
pins lifecycle/coverage obligations: required opportunity, selected/armed,
retrieved, applied, and independently observed effect (or a typed reason a
stage is inapplicable). `CLEAN` is unreachable when the covered contract
requires a fault opportunity/effect that was not independently observed;
plan retrieval alone proves none of those later stages
(`crates/vh-multiverse/src/lib.rs:243-281`).

**Composite registration identity.** Rust resolves one canonical
`registration_digest` over tenant id, canonical tenant repository, workload
id/version, implementation identity, operation/features, covered path/mode
surfaces and changed-entry classifier, property/oracle contract
id/version/content digest, fault-policy id/version/content digest, canonical
parameters, seed/universe set, fixed budget, lifecycle/coverage obligations,
and compatibility map. The K2 tenancy request/result/envelope, Path T record
or Path A attestation, and K3B `EvidenceReceipt` attributes all carry this
digest plus the resolved component digests. The K2R registry record carries
the same values; the closed base v2 run/verification records do not acquire
tenancy meaning (closed run manifest at
`crates/vh-cli/src/bundle.rs:727-754`; closed verifier result at
`clients/python/vibe_halt/core/runner.py:493-517`). The protected checker
compares every value to protected policy; same id/version with different
content is rejected.

K2R does not append unknown keys to the closed `vh-verify-run-v2` result.
Instead, Rust adds a non-verdict `tenancy-registry-entry-v0` record generated by
`registry-show`, and `verify-run` gains explicit expected engine/registration
arguments that it compares against the protected registry while retaining its
closed v2 output. K2's additive tenancy envelope binds the v2 `result_digest`
to the exact registry-entry record/digest and carries the registration-bound
run fields. Substitution of any expected value is a verifier error. V2 alone
remains non-tenancy and cannot green admission; neither Python nor workflow
YAML defines a parallel verdict format.

**Registry ownership.** The registry lives in vibe-halt Rust source and
changes only by human-merged PR with red-first negative tests: the exact
pre-registration workload id, typo/case/alias/unknown-version variants,
another workload's otherwise-valid contract, same-id/version content drift,
weakened property/oracle set, unknown fault model, parameter/budget drift,
missing required manifestation, cross-tenant repo/operation/digest
substitution, and any unclassified/overlapping changed-entry transition each
have a named pre-execution rejection fixture before the entry lands. A
generated enumeration gate must prove CLI
help, capability output, request validation, receipt schema, and verifier
lookup all derive from and agree with this registry; current hand-maintained
surfaces are not accepted as registry truth. Current main already demonstrates
the drift risk: CLI help and registry enumeration differ
(`crates/vh-cli/src/main.rs:112-124`;
`crates/vh-cli/src/workloads/mod.rs:225-260`), and help names receipt schema
v1 while the writer emits v2 (`crates/vh-cli/src/main.rs:136-138`;
`crates/vh-cli/src/bundle.rs:209-239`).

## 8. Governance, ownership, and the dependency DAG

**Dependency DAG (hard edges = human merge required before dependent entry):**

```
 G0: merged REALITY_BRIDGE_COMPLETE_FORWARD_{CONFIRMED|NULL}
     + human #62 clearance bound to that result commit
                  │
                  ▼
 A1: this controller human-merged
                  │
                  ▼
 K1: tenancy contract v0 + decision/registry/operation schemas
                  │
                  ▼
 K2R: exact workload/contract/fault/operation registry entry
                  │
                  ▼
 K2: SDK + protected provenance + source-consuming bridge
                  │
                  ▼
 K3A: dharma_swarm governance-only admission charter (A2 on merge)
                  │
                  ▼
 K3B: separate implementation PR + advisory gate
                  │  (≥10 real advisory verdicts)
                  ▼
 K4: second tenant or public dossier
```

No package in this DAG may be stacked on an unmerged predecessor. In
particular, K2 cannot build against a branch-only registry interface, and
K3B cannot submit or gate on the tenant workload until K2R and K2 are both on
vibe-halt `main`. This serial order replaces the first repair's unsupported
K2/K2R parallel assumption.

K4 dossier-only mode adds no authority. If K4 implementation selects another
tenant, its separately human-merged admission packet expands K4 into the local
hard-edge sub-DAG `K4R registration -> K4A tenant charter -> K4B tenant
implementation`. K4R owns that tenant/repository/operation/composite digest
and is compatible with the current K1/K2 platform; K4A is that tenant's A2.
The generic §4 authority DAG then uses K4R/K4A, never K2R/K3A borrowed from
dharma_swarm. No K4 subpackage is pre-authorized here.

| package | repo | runs under | primary surfaces | entry gate |
|---|---|---|---|---|
| K1 spec | vibe-halt | `vibe-halt-core-2026-07` | `docs/specs/TENANCY_CONTRACT_V0.md`, controller amendments only | G0 observed from protected state; this controller human-merged (A1) |
| K2R registration | vibe-halt | `vibe-halt-core-2026-07` | exact registry/contract/fault-model files, `scripts/run_tenancy_bounded.py`, and bounded-command policy/test fixtures frozen in the K1 exit receipt | K1 human-merged; named integration writer + write lease |
| K2 SDK/bridge | vibe-halt | `vibe-halt-core-2026-07` | exact CLI request/result/operation files, `clients/python/**`, `scripts/check_tenancy_single_truth.py`, and disjoint remaining `tests/fixtures/tenancy/**` paths frozen after K2R merge | K2R human-merged; no colliding Reality Bridge PR; named integration writer + lease |
| K3A charter | dharma_swarm | governance-only admission | governance file(s) only, exact paths re-observed there | K2 human-merged; separate human review; no implementation |
| K3B tenant | dharma_swarm | K3A charter | exact adapter/workflow/test manifest frozen by K3A | K3A human-merged (A2); every K3A command/budget/owner still valid |
| K4 second tenant | separately selected repo | new admission | frozen by a new decision packet | K3B has ≥10 real PR verdicts with published mix; separate repo/target authority |

Rules:

- One writer per file. vibe-halt shared surfaces (`scripts/gate.sh`,
  `Cargo.toml`, `Cargo.lock`, `docs/governance/ACTIVE_TRACK.yaml`) stay under
  the existing single-named-integration-writer protocol
  (`docs/governance/ACTIVE_TRACK.yaml:94`); every package names the integration
  writer and acquires a write lease before touching them.
- Before delegation, every package freezes an exact file write manifest,
  owner, verifier, base SHA, predecessor merge SHAs, interface digest, and
  stop condition. Broad metadata such as `allowed_writes` is coordination,
  not a sandbox; executable verifier commands receive their own authority
  review. K1's exit receipt partitions every exact `tests/fixtures/tenancy/**`
  file between K2R and K2; their write manifests may not overlap.
- K1/K2/K2R must not touch Reality Bridge surfaces (`crates/vh-sandbox/**`,
  `corpus/**`) except through an exact interface request; if a Reality
  Bridge wave is mid-flight on `clients/python/**`, K2 waits.
- Nothing in this controller mutates `ACTIVE_TRACK.yaml` acceptance
  criteria; this admission PR may update exactly one `next` item in
  `vibe-halt-core-2026-07` referencing this file and add only these exact
  owned surfaces required by §§4–7:
  `docs/governance/TRUTH_KERNEL_A1_BOOTSTRAP_AUTHORITY_V0.json`,
  `scripts/run_tenancy_bounded.py`,
  `scripts/check_tenancy_single_truth.py`, and
  `tests/fixtures/tenancy/**`. Nothing else may change there.
- Every PR starts draft, names exact base/head SHA, cites this controller,
  includes test receipts and rollback notes. Humans mark ready and merge.
- K3A and K3B are two PRs: the former is governance-only and transitions to
  A2 when a human merges it; the latter implements within that authority. A
  decline in dharma_swarm is a stop condition, not an obstacle to route
  around (§13).

## 9. Work packages

Every package runs the same verification loop, defined once here and
instantiated per package below (the loop mirrors
`docs/DEVELOPMENT_WORKFLOW.md:111-121`):

1. **Run** the actual CLI, test, replay, or benchmark named by the package.
2. **Use** the changed behavior through its public entry point.
3. **Prove** it with an exit code, machine record, replay, artifact digest,
   or an explicit unavailable reason.
4. **Falsify** the happy path with malformed, stale, empty, boundary, and
   concurrency cases — including every §5 negative fixture in scope.
5. **Record** only durable claims; runtime receipts stay off-git
   (`CLAUDE.md:27-29`).

Per-package run management (budgets, retries, checkpoint/resume, rollback)
is defined in §12 and applies to every package below.

**Executable bounded-command contract.** K2R's first code step, before adding
any registration, implements gate-wired `scripts/run_tenancy_bounded.py` and
its red-first tests from the K1-frozen schema. Every K2R/K2 payload below is an
argv (never shell text) launched by the exact prefix:

```text
<pinned-python> <absolute-read-only-source-root>/scripts/run_tenancy_bounded.py
  --policy <absolute-read-only-source-root>/tests/fixtures/tenancy/bounded-command-policy-v0.json
  --lane-root <absolute-lane-root>
  --source-root <absolute-read-only-source-root>
  --cwd-rel .
  --materials-manifest <absolute-read-only-materials-manifest>
  --record-dir <empty-one-use-command-record-dir>
  --work-dir <empty-one-use-work-dir>
  --output-root <declared-output-root>
  --output-mode empty-producer|existing-read-only
  -- <payload argv...>
```

The only direct bootstrap form is the gate-wired wrapper self-test:

```text
<pinned-python> <absolute-read-only-source-root>/scripts/run_tenancy_bounded.py --self-test
  --policy <absolute-read-only-source-root>/tests/fixtures/tenancy/bounded-command-policy-v0.json
  --lane-root <empty-self-test-lane-root>
  --source-root <absolute-read-only-source-root>
  --materials-manifest <absolute-read-only-materials-manifest>
  --record-dir <empty-one-use-self-test-record-dir>
```

It exits `0` only after all six boundary fixtures are rejected. In this
`--self-test` mode it creates private per-fixture scratch children only beneath
`<empty-self-test-lane-root>/self-test-work/`, bounds every capture under the
same aggregate lane quota, removes that entire scratch subtree before return, and
atomically leaves exactly one regular no-link file in the record directory:
`<record-dir>/bounded-command-self-test.ndjson`. That file is exactly one
canonical NDJSON record with `record=bounded-command-self-test`,
`schema=vh-bounded-command-self-test-v1`, and the closed ordered outcomes
timeout/process/output/disk/memory/network each equal to `REJECTED`; no other
entry exists in the record directory, and the lane root contains no entry other
than that record directory. Backend unavailability, cleanup failure, or any
fixture escape exits `125`. It never executes a package payload.

`<pinned-python>`, the wrapper, policy, source root/tree, OS isolation backend,
and all payload materials are absolute identities in the protected or
explicitly non-admissible candidate material manifest, never payload choices.
The wrapper resolves and digest-checks the complete manifest before spawn;
prior build roots and caches are mounted read-only and only current work/output
roots are writable. `--cwd-rel` is a lexically normalized registered
repository-relative directory; the backend mounts the exact source tree
read-only and sets that directory as cwd. The canonical policy is: wall `2700`
seconds; combined retained
stdout+stderr `10485760` bytes; payload process-tree concurrency `4`; memory
`8589934592` bytes; generated work `10737418240` bytes; egress `DENY`; kill and
reap the whole payload process group on violation. Kernel/cgroup/container
process+memory enforcement, a deny-egress network namespace, and aggregate
filesystem quota cover the whole lane/output set; polling is not enforcement.
If those primitives are unavailable, the wrapper exits `125` before payload
execution. In normal payload mode (without `--self-test`) it atomically creates exactly
`<record-dir>/bounded-command.ndjson` as one regular no-link canonical
line with `record=bounded-command` and `schema=vh-bounded-command-v1`, plus
bounded raw captures
`<record-dir>/payload.stdout` and `<record-dir>/payload.stderr`; no other entry
is allowed. The record binds source/cwd/policy/argv digests, limit status,
payload exit, wall/output/process/memory/generated-byte measurements,
deny-egress state, process-group reap, and violations. Required payload NDJSON
is parsed independently from `payload.stdout`; wrapper success can never stand
in for a missing/malformed/duplicate payload record. It exits payload code
unchanged, `124` on timeout, or `125` on any
boundary/measurement violation; it never truncates into success. The wrapper
self-test must red-first provoke timeout, fifth process, stdout/stderr overflow,
disk overflow, memory overflow, and network connection, then prove each entire
process tree is killed and no unbounded log survives. If the platform cannot
enforce or observe a bound, it exits `125` before the payload. Cargo payloads
also set `CARGO_NET_OFFLINE=true`, `CARGO_BUILD_JOBS=1` and use
`--locked --offline -j 1`; the outer worker independently denies egress.
The backend sets Cargo target, Python bytecode/cache, and `TMPDIR`/`TMP`/`TEMP`
to distinct children of the one-use work directory, with pinned read-only
tool/dependency caches; it rejects any attempted source-root or out-of-lane
write. The sole Cargo-target exception is either exact offline build-producer
payload below: its explicit `--target-dir` must equal that invocation's
declared `empty-producer` output root, which remains inside the same aggregate
lane quota and becomes immutable after success. No consumer or other payload
may use an output root as Cargo target. `make gate` and `make review` run
through the same wrapper/environment, so their build/test artifacts remain
inside the quota lane's work directory.
The lane root is an operator-designated, newly created, empty, absolute
no-symlink directory bound to the hard-quota backend for one package attempt.
Every record/work directory is a new empty direct child used by one invocation.
An `empty-producer` output root is another new empty child; an
`existing-read-only` output root is the exact completed producer root in that
lane, digest-bound before the consumer, mounted/read as immutable, and proved
unchanged afterward. Lexical containment and device/quota membership are
checked before execution; no directory argument may alias, overlap, escape,
or be reused contrary to its mode. Every resolved path is recorded.

The output mapping is closed: pre-registration/clean/mutant/production-reject
engine runs and `tenant-verify` use their named fresh root with
`empty-producer`; the two offline builds use their named build root with
`empty-producer`; clean and mutant `verify-run` consume the corresponding
completed engine-run root with `existing-read-only`; and `registry-show`,
capabilities, Cargo/Python tests, scanner self/live scans, `make gate`, and
`make review` each receive a distinct fresh scratch output root with
`empty-producer`. No command infers a mode or output location.

### K1 — Tenancy contract v0 (spec only, no code)

Write `docs/specs/TENANCY_CONTRACT_V0.md` defining, on top of the merged R0
machine contract (do not fork it):

- **Tenant identity:** a named external system submitting verification
  requests; caller tenant id is untrusted until it exactly equals the
  protected registration, source repository, and A2 authority bindings (§4).
- **Workload description:** today only workloads compiled into `vh`
  (registered workload id + pinned parameters). Arbitrary tenant code is not
  executable under K1/K2R/K2; any later separately admitted execution must
  use §5's disposable outer worker and remain Tier-2/D2 with all 29 channels
  open (`docs/specs/SANDBOX_CAPABILITY_ENVELOPE_V1.md:25-38`).
- **Property contract:** the §7 registry law — versioned ids, content
  digests, compatibility mapping, rejection semantics.
- **Fault model:** the §7 registry law — versioned id, canonical
  parameters, fixed budget, fail-closed drift handling.
- **Provenance model:** §5 verbatim in contract form — Path T, Path A, the
  attestation schema field list, the trust policy, and the negative-fixture
  catalogue, including complete `ExecutionMaterialsV0`. Mechanism-agnostic: no
  signing platform is chosen here.
- **Source binding:** the §6 closed `ExecutionSubjectV0`, complete classified
  base→subject tree diff, protected tree-object extraction manifest and bounds,
  source-derived artifact digest, typed coverage/source rejection,
  source-sensitivity, stale-fails-closed.
- **Evidence bundle:** the v2 bundle plus a tenancy envelope (request
  identity, tenant id, source binding, protected checker/engine/adapter,
  operation/features, composite registration digest, resolved
  contract/oracle/fault-policy/parameter/budget digests, lifecycle coverage,
  receipt/bundle digest, full `AuthorityChainV0` refs/digest, and §4 product
  state) — content-addressed, atomically emitted, Rust-verified.
- **Verdict semantics for tenants:** `CLEAN`/`FINDINGS`/`UNCHECKED`/errors
  mean exactly what R0 says; write the mapping table — `UNCHECKED` and every
  error map to not-green (§10).
- **Authority clause:** the §4 parameterized authority DAG,
  `OwnerAccountIssueAuthorityV0`, protected current authority/revocation-registry view,
  cross-field equality, and promotion contract, so no tenant integration can
  misread green evidence as merge authority or borrow another tenant's A2.
- **Execution contract:** exact `scripts/run_tenancy_bounded.py` interface from
  §9 and exact gate-wired `scripts/check_tenancy_single_truth.py` interface;
  K1 defines their canonical inputs/exits/self-test fixtures, K2R/K2 implement
  them in the ordered steps below.
- **Capability statement:** an explicit enumerated list of what a kernel
  verdict does and does not attest, including the D2/29-open-channels
  boundary verbatim.

**Run / Use / Prove / Falsify / Record:** run `make onboard`,
`git diff --check origin/main...HEAD`, `make gate`, then `make review`; use
the spec by independently constructing one valid and every declared invalid
`tenancy-admission-decision-v0` example on paper/fixture bytes; prove exact
canonical bytes, schema digest, transition table, registry/operation ids, and
the closed constructor review; falsify every §5 case plus unknown keys/states,
forged authority references, missing/substituted/revoked/out-of-order
predecessors, pre-G0/stale/later-promoted/unauthorized/edited authorized-account events,
tenant-A refs in tenant B, uncovered changed paths, mutable transitive
materials, unknown subject tags, and partial product states; record the exact base/head,
commands/exits, independent reviewer, and unavailable
mechanical checks in the PR evidence capsule. A spec-only claim may be
review-ready, never implementation-proven.

Entry gate: G0 and A1 are freshly revalidated. Exit gate: spec human-merged;
the negative-space section lists at least five things the contract refuses to
promise; the negative-fixture catalogue covers every §5 case; the exact
`dharma-graph-checkpoint-replay-v1` workload,
`dharma-graph-checkpoint-contract-v1` property/oracle contract,
`dharma-graph-checkpoint-fault-v1` fault policy, and `tenant-verify-v1`
operation/features are frozen; the K2R file/interface manifest is frozen; no
code changed. The bounded-runner and single-truth scanner schemas, exact flags,
self-test fixture corpus, and gate insertion points are frozen for
implementation, not claimed to exist.

### K2R — Exact workload and registry substrate

Implement the exact `dharma-graph-checkpoint-replay-v1` workload in the
closed Rust registry (`crates/vh-cli/src/workloads/mod.rs:225`) with its
non-empty `dharma-graph-checkpoint-contract-v1` property/oracle contract,
`dharma-graph-checkpoint-fault-v1` fault policy, lifecycle obligations,
`tenant-verify-v1` operation/features, fixed seed `0xd1ce` and 200-universe
budget, and §7 composite registration digest. This package proves the model
only; it does not claim that a
dharma_swarm checkout behaves like the model.

**Run / Use / Prove / Falsify / Record:** first the bounded wrapper's exact
self-test passes. Before registration, create one empty
`<pre-registration-output-dir>` and launch this payload through the exact
bounded prefix above:

```text
cargo run --locked --offline -j 1 --target-dir <command-work-dir>/cargo-target -p vh-cli -- run --workload dharma-graph-checkpoint-replay-v1 --seed 0xd1ce --universes 1 --palette v0 --out <pre-registration-output-dir>
```

It must exit exactly `2`, emit the typed unknown-workload diagnostic, and leave
that existing root empty: no `run.ndjson`, `findings/`, lock, receipt, bundle,
or other entry. Red-first Rust tests named
`tenancy_registry_*` cover exact id, typo, case, alias, missing/unknown
version, incompatible contract, same-id/version content drift, fault/budget
drift, and missing fault opportunity/effect. After implementation, build one
production engine and one separately pinned test-fixture engine with exact
offline materials by launching each of these payloads through a fresh bounded
prefix whose output root is the matching empty build root:

```text
cargo build --locked --offline --release -j 1 -p vh-cli --bin vh --target-dir <production-build-root>
cargo build --locked --offline --release -j 1 -p vh-cli --bin vh --features tenancy-test-fixtures --target-dir <test-build-root>
```

`<candidate-engine-path>` is exactly
`<production-build-root>/release/vh`; `<candidate-test-engine-path>` is exactly
`<test-build-root>/release/vh`. Record each absolute regular no-link path,
source tree, feature set, build-manifest digest, and SHA-256 in a candidate
material manifest with the same closure fields as `ExecutionMaterialsV0` and
`admission_eligible=false`. An engine built from the unmerged K2R branch is
never protected provenance; the test engine is additionally a falsifier
material and is never production- or admission-eligible. Create a different empty
`<clean-output-dir>` and run

```text
<candidate-engine-path> run --workload dharma-graph-checkpoint-replay-v1 --seed 0xd1ce --universes 200 --palette v0 --out <clean-output-dir>
```

It must exit exactly `0`, report and encode verdict `CLEAN`, leave exactly one
regular no-link `run.ndjson` under the root, create no `findings/` directory,
and bind the candidate executable whose absolute path and SHA-256 equal its
candidate material entry. Then launch this complete command through a fresh
bounded prefix/record/work directory:

```text
<candidate-engine-path> verify-run --out <clean-output-dir> --engine <candidate-engine-path> --expected-engine-sha256 <candidate-engine-sha256> --expected-tenant dharma_swarm --expected-repository AmitabhainArunachala/dharma_swarm --expected-workload dharma-graph-checkpoint-replay-v1 --expected-seed 0xd1ce --expected-universes 200 --expected-palette v0 --expected-operation tenant-verify-v1 --expected-registration-digest <registration-digest> --expected-property-contract-id dharma-graph-checkpoint-contract-v1 --expected-property-contract-digest <property-contract-digest> --expected-fault-policy-id dharma-graph-checkpoint-fault-v1 --expected-fault-policy-digest <fault-policy-digest> --expected-parameter-digest <parameter-digest> --expected-budget-digest <budget-digest> --expected-lifecycle-coverage-digest <lifecycle-coverage-digest> --expected-verdict CLEAN --expected-outcome-exit-code 0 --expected-findings-total 0
```

It must exit exactly `0` and emit exactly one canonical NDJSON record anchored
by `record=verify-run`, `schema=vh-verify-run-v2`, `authentic=true`,
`verified=true`, `outcome_verified=true`, `verdict=CLEAN`,
`outcome_exit_code=0`, `findings_total=0`, `findings_verified=0`, `errors=[]`,
the exact candidate `engine_sha256`, and the resulting `result_digest`. Success
means every explicit expectation was compared by Rust; deleting or
substituting any expectation must make the verifier nonzero. The closed v2
output is not extended. Its `authentic=true` field means only that this local
verifier reproduced the candidate run under v2 semantics; the product remains
`P0_UNTRUSTED` and has no admission authority. Then the exact bounded payload

```text
<candidate-engine-path> registry-show --tenant dharma_swarm --repository AmitabhainArunachala/dharma_swarm --operation tenant-verify-v1 --workload dharma-graph-checkpoint-replay-v1 --format ndjson
```

must exit `0` with one non-verdict `tenancy-registry-entry-v0` record whose
tenant, repository, workload, operation, property, fault, budget, component,
and composite fields equal every expectation above. After K2R merges, K2
regenerates the record with the protected-main engine and binds that new record
digest to the v2 `result_digest`; the branch-built record is expected-byte
evidence only. Any extra path, missing or duplicate record,
different expectation/digest/verdict, or verifier nonzero is failure.
The exact registry matrix payload is:

```text
cargo test --locked --offline -j 1 --target-dir <command-work-dir>/cargo-target -p vh-cli --test cli_contract tenancy_registry_ -- --nocapture
```

The repo-local modeled mutant proof is this exact engine command through a new
bounded prefix and empty `<mutant-output-dir>`:

```text
<candidate-test-engine-path> run --workload dharma-graph-checkpoint-replay-v1 --seed 0xd1ce --universes 200 --palette v0 --test-fixture checkpoint-before-persist-v1 --out <mutant-output-dir>
```

The wrapper propagates exact exit `1`; the command emits one anchored
`FINDINGS` verdict with exactly one finding whose stable fingerprint is
`oracle:checkpoint_replay_consistency`, and writes `run.ndjson` plus only
manifest-listed regular finding bundles. A fresh bounded prefix then launches
this exact verifier payload:

```text
<candidate-test-engine-path> verify-run --out <mutant-output-dir> --engine <candidate-test-engine-path> --expected-engine-sha256 <candidate-test-engine-sha256> --expected-tenant dharma_swarm --expected-repository AmitabhainArunachala/dharma_swarm --expected-workload dharma-graph-checkpoint-replay-v1 --expected-seed 0xd1ce --expected-universes 200 --expected-palette v0 --expected-operation tenant-verify-v1 --expected-registration-digest <registration-digest> --expected-property-contract-id dharma-graph-checkpoint-contract-v1 --expected-property-contract-digest <property-contract-digest> --expected-fault-policy-id dharma-graph-checkpoint-fault-v1 --expected-fault-policy-digest <fault-policy-digest> --expected-parameter-digest <parameter-digest> --expected-budget-digest <budget-digest> --expected-lifecycle-coverage-digest <lifecycle-coverage-digest> --expected-test-fixture checkpoint-before-persist-v1 --expected-finding-fingerprint oracle:checkpoint_replay_consistency --expected-verdict FINDINGS --expected-outcome-exit-code 1 --expected-findings-total 1
```

It exits `0`, locally verifies the mutant result without raising provenance
above `P0_UNTRUSTED`, and reports
`findings_total=1`, `findings_verified=1`, and `errors=[]`.
Changing the fixture name, fingerprint, engine path/digest, or any expected
component makes it nonzero.

The fixture flag/feature is compile-time test-only, registry-owned, excluded
from every production `ExecutionMaterialsV0`, and unavailable to
`tenant-verify`. A separate bounded prefix launches this exact payload with a
new empty `<production-reject-output-dir>`:

```text
<candidate-engine-path> run --workload dharma-graph-checkpoint-replay-v1 --seed 0xd1ce --universes 200 --palette v0 --test-fixture checkpoint-before-persist-v1 --out <production-reject-output-dir>
```

It must exit `2` before execution and leave the root empty; generated production help/capabilities/
registry output must not enumerate `--test-fixture`. Record exact exits,
absence/presence and no-link shape of output, component/composite digests,
source files, and the generated-enumeration diff; then run `make gate` twice
and `make review`.

Exit gate: the exact entry and interface are human-merged on vibe-halt
`main`; CLI help, capabilities, request validation, receipt schema, and
verifier lookup match one generated registry enumeration; exact-head local
and CI gates in §11 are green. After that merge, the protected current-main
builder must reproduce and bind a production engine/material closure before
K2 or any Path T/A result may claim `P2`/`P3`; every branch-built K2R receipt
remains local evidence. K2 starts only after this merge and protected rebuild.

### K2 — Tenant SDK and source-consuming bridge

Generalize the strict local adapter without adding a second truth path, and
implement the missing production prerequisites named in §3:

- Rust publishes and validates versioned `tenant-verify-v1` operation/feature
  negotiation from the generated registry, including
  `observed-source-v1`, `registration-digest-v1`, and
  `admission-decision-v0`;
- the Rust operation consumes a canonical source-derived artifact bound by
  §6, so changing covered source bytes can change the property result;
- the protected source extractor accepts only bounded allowlisted regular
  blobs read from the pinned Git tree/object database, emits the canonical
  extraction manifest, computes/classifies the complete protected tree diff,
  and never follows the PR checkout filesystem;
- Python exposes only closed request/result types
  (`clients/python/vibe_halt/core/result.py:8-28`), relays Rust outcomes, and
  treats caller `source_commit` as untrusted context
  (`clients/python/vibe_halt/core/request.py:82`);
- protected policy, not tenant input or `EnginePolicy`, selects the engine,
  checker, adapter, registration, contract, fault policy, and budget; the
  current local digest pin remains local-only
  (`clients/python/vibe_halt/core/request.py:23-33`);
- `verify_bundle(path)` returns integrity axis data only, at most
  `I3_INTEGRITY_OK`; it cannot construct provenance or authority; and
- Path T and Path A records, `ExecutionMaterialsV0`, the closed execution
  subject, complete `AuthorityChainV0`, current authority-registry view,
  `OwnerAccountIssueAuthorityV0`, independent gate policy, and
  `tenancy-admission-decision-v0` are parsed and validated in Rust; Python is
  defense in depth only; and
- `scripts/check_tenancy_single_truth.py` is gate-wired. Its positive
  `--self-test` must inject and reject a Python-authored verdict constructor, a
  workflow-authored green shortcut, a parallel replacement receipt, and a
  bypass around the Rust product; its repository scan allows the additive
  tenancy/Path-T/Path-A envelopes only when Rust remains the sole decision
  evaluator.

**Run / Use / Prove / Falsify / Record:** launch each following exact payload
through a fresh §9 bounded prefix (the pinned Python path replaces only the
literal `<pinned-python>`):

```text
cargo run --locked --offline -j 1 --target-dir <command-work-dir>/cargo-target -p vh-cli -- capabilities --format json
cargo run --locked --offline -j 1 --target-dir <command-work-dir>/cargo-target -p vh-cli -- tenant-verify --request tests/fixtures/tenancy/clean_v1.json --out <explicit-temp-dir>
cargo test --locked --offline -j 1 --target-dir <command-work-dir>/cargo-target -p vh-cli --test cli_contract tenancy_ -- --nocapture
<pinned-python> -m unittest discover -s clients/python/tests -p test_tenancy*.py
<pinned-python> <absolute-read-only-source-root>/scripts/check_tenancy_single_truth.py --self-test
<pinned-python> <absolute-read-only-source-root>/scripts/check_tenancy_single_truth.py --root <absolute-read-only-source-root>
```

Every payload and wrapper must exit exactly `0`; the self-test succeeds only
by catching all four seeded forbidden truth paths and emits exactly one NDJSON
record with `record=tenancy-single-truth`,
`schema=vh-tenancy-single-truth-v1`, `mode=self-test`, `status=PASS`,
`python_verdict_constructor=REJECTED`,
`workflow_green_shortcut=REJECTED`,
`parallel_replacement_receipt=REJECTED`, `rust_product_bypass=REJECTED`, and
`violations=0`. The live scanner emits exactly one record with that record +
schema, `mode=repository`, `status=PASS`, the exact checkout tree, and
`violations=0`; missing/duplicate output or any additional field not allowed
by K1 is failure. Red-first scanner/classifier fixtures for a newly introduced
executable path and configuration path must each exit nonzero as uncovered,
not become implicitly non-impacting. The `tenant-verify` command emits the
exact product/envelope and Path T/A machine fields frozen by K1, never a
Python/workflow verdict.
One source fixture differing only in a covered byte must change the bound
artifact digest and a deliberately broken source-derived mutant must turn
the result red; an identity-only/no-consumption adapter is rejected. Run all
§5/§6 product-state, protected-checker, engine-substitution, subject-relation,
stale/replay, registry-drift, authority-chain, credential-inventory,
tree-path/mode/size, total-diff coverage, transitive-material, unauthorized
human/revocation-registry, cross-tenant substitution, unknown-event, and
source-sensitivity negatives. Record exact
commands/exits and machine digests, then `make gate` twice and `make review`.

For each package attempt, create the lane root once with `mktemp -d` beneath
an explicit operator-selected quota-backed parent. Create each record, work,
build, and `empty-producer` output placeholder as a distinct empty direct
child of that lane root, record the printed absolute path, and use it exactly
once; only the explicitly paired `existing-read-only` consumer may reuse a
completed producer root under the immutable rule above. Refuse aliases,
non-empty producer roots, implicit temp/home paths, and roots created outside
the lane.

Exit gate: operation/feature negotiation, runtime-observed source binding,
the source-consuming bridge, protected provenance validator, and the strict
SDK are human-merged on vibe-halt `main`; exact-head local/CI evidence in §11
is green. K3A starts only after this merge.

### K3A — dharma_swarm governance-only admission charter

Re-observe that repository's governance, receipt mount point, active track,
protected-workflow options, and exact owned files. Write one governance-only
draft PR that freezes: K3B objective/non-goals; exact base SHA and write
manifest; owner/integration writer/verifier; pinned vibe-halt K2/K2R commits
and digests; protected checker policy; zero-credential worker policy and
credential inventory; complete `ExecutionMaterialsV0`; closed execution
subject; tree-object-only source allowlist/bounds and total changed-entry
classifier; current human/authority-registry-view policy; tenant-parameterized
authority-chain contract; registration-pinned A2 and GP3 artifact paths,
schemas, and current-object resolver; exact GitHub provider kind, target,
required-check, dedicated publisher App identity/issuer, no-shared-integration
rule, exact empty/no-implicit-bypass invariant, closed
`RulesetAuthorizationEventV0`/`GateEnforcementOutcomeV0` carriers, and
enforcement-object/event resolvers for any future GP3 act (initially
absent/GP0); K3A does not invent an ID for an absent object,
while the future human-authorized `GP3PolicyRefV0` binds the created ruleset's
stable id and desired-config projection digest, never its pre-activation
current-object digest; exact adapter and source-derived artifact
contract; the exact `MergeRelianceAttestationV0` carrier,
schema, currentness resolver, and negative cases below; exact commands,
expected exits, fixtures, numeric budgets,
artifact limits, rollback, and kill conditions. It changes no implementation
or workflow.

**Run / Use / Prove / Falsify / Record:** run that repo's current onboarding,
governance validator, `git diff --check`, full gate, and review command as
freshly discovered from its accepted checkout. If any exact command cannot
be bound, emit `TRUTH_KERNEL_BLOCKED_K3A_COMMAND_UNBOUND`; do not invent one.
Use the charter by having an independent reviewer reproduce its ownership,
authority, and negative matrix. Record exact commands/exits and full SHAs.

Exit gate: a human merges the charter and creates
`A2_TENANT_ADMISSION_MERGED`; only then may K3B begin.

### K3B — dharma_swarm first-tenant implementation and advisory gate

In a separate draft implementation PR, within K3A's exact authority:

- consume exact protected-observed dharma_swarm regular blobs extracted from
  the pinned Git tree through K2's source-consuming operation; bind the
  protected current subject from §6 without following checkout paths;
- require exact equality with K2R's `registration_digest` and all property,
  oracle, fault-policy, parameter, budget, engine, adapter, operation, and
  feature digests;
- require the complete protected base→subject diff to classify under that
  registration, and require K3A's A2 tenant/repository/operation/digest fields
  to equal the execution subject and registration exactly;
- bind those values plus the Rust receipt/bundle digest into the additive
  `EvidenceReceipt.attributes` mount point re-observed by K3A; make no spine
  schema change;
- add ONE advisory check whose admission decision is computed only by the
  protected Path T/A checker and disposable worker in §5; and
- write a non-executable promotion memo for a later human `GP3_REQUIRED`
  decision; do not flip the gate.

The sole v0 account-attributed attestation that advisory evidence affected a
real merge is a newly created, never-edited GitHub issue comment on the exact
dharma_swarm PR. Its
UTF-8 body is one canonical JSON line, no leading/trailing whitespace or extra
keys, at most 512 bytes, in this exact order and shape:

```json
{"record":"truth-kernel-merge-reliance-attestation","schema":1,"decision":"ACCOUNT_ATTESTED_RELIANCE","repository":"AmitabhainArunachala/dharma_swarm","pr":0,"execution_subject_digest":"<64-lowercase-hex>","subject_sha":"<40-lowercase-hex>","check_run_id":0,"check_run_attempt":1,"admission_decision_sha256":"<64-lowercase-hex>","evidence_bundle_sha256":"<64-lowercase-hex>"}
```

`MergeRelianceAttestationV0` binds that event id/body/digest, repository + PR,
full `ExecutionSubjectV0` + canonical digest, immutable check run/attempt/
conclusion and decision/bundle digests, comment actor node id/type/timestamps,
PR merge commit/`merged_at`/`merged_by`, exact final merge relation, and
protected resolver-policy digest. K3A freezes the permitted merge method(s) and
the exact base/head/synthetic-merge or merge-group membership/tree relation to
the final protected merge commit.

The protected resolver requires the advisory check to have completed on that
exact current subject before comment creation; `created_at == updated_at`;
comment creation before `merged_at`; `actor.type == User`; exact equality of
comment actor and the trusted PR `merged_by` stable node id; and the PR's final
head, base, execution tree, synthetic merge or merge-group identity/membership,
and resulting merge relation to equal the bound subject with no intervening
movement. Any head/base/group/merge-method movement after check or comment
invalidates both and requires a fresh current-subject check plus a new canonical
comment; an old comment is superseded, never reused. The merge commit must
remain on protected current main with the relied-upon change not reverted. The
resolver re-fetches the comment, PR, check run, full subject, bundle, decision,
merge relation, and current ancestry. A green check without this comment, a
comment after merge, wrong actor/PR/run/attempt/subject/digest, bot/App, edit/
deletion, moved subject, unmerged or reverted PR, or caller-carried snapshot has
no constructor. This proves only that the same authorized GitHub `User` account
attributed the merge to the evidence; a PAT, service automation, coercion, or
unread evidence cannot be excluded, so it is not proof of human presence,
cognition, causality, or correctness. It grants no truth promotion, merge, GP3,
operator, or external authority.

**Run / Use / Prove / Falsify / Record:** execute exactly the commands frozen
by K3A. A real source-derived clean fixture must reach advisory green; a
mutant in the actual covered tenant source — not merely the compiled Rust
model — must make the check red. Run every §10 negative, including PR-edited
checker/helper, substituted engine, stale/synthetic subject mismatch,
identity-only adapter, content-digest drift, missing manifestation, and forged
authority checkpoint or missing/substituted/revoked/out-of-order authority
predecessor. Also run pre-G0, stale, later-promoted, unauthorized, bot,
edited, and deleted #62 actor/event cases; stale current authority-registry
view; mutable transitive material; tenant-A refs in tenant B; unclassified/
mixed diff; absent/duplicate/wrong-path/reverted GP3 artifacts; inactive,
deleted, wrong-target/check/integration, or broadened-bypass rulesets;
shared-integration/check-name spoofing; missing/mutable/wrong activation or
merge-enforcement events; every `MergeRelianceAttestationV0` rejection
above; and unknown/indirect event cases.
Record exact
PR/head/base/checkout/merge-group identities,
commands/exits, product state, component/composite digests, and bounded
artifacts.

Exit gate: at least one real dharma_swarm PR demonstrates the exact source
affecting the verdict end-to-end via Path T/A; all K3A commands and exact-head
CI/review gates pass; a human merges K3B; and a later real PR receives that
protected advisory decision and is human-merged with a current
`MergeRelianceAttestationV0` from the same authorized merger account that binds
its exact decision and bundle digests and asserts the evidence was one reason
for merging. A green check or merge alone is not even account-attributed
reliance, and the attestation does not prove cognition. Only then may the vibe-halt
criterion at `docs/governance/ACTIVE_TRACK.yaml:87` be cited, and only at the
demonstrated D2/source/contract/fault/budget boundary.

### K4 — Second tenant or public tenancy dossier

Entry gate: ≥10 real advisory verdicts on 10 distinct dharma_swarm main-bound
PR ids, with at most one final current-subject verdict counted per PR and the
miss/null/unchecked counts published, not just the greens. Retries and
superseded subjects never increment the count. At least one has a current
`MergeRelianceAttestationV0` satisfying K3B's exact digest-bound rule.

Then either onboard a second tenant through the unchanged K1/K2 platform plus
that tenant's own human-merged K2R-class registration
(candidate classes: another repo in the org; the output of a commodity
harness run modeled as a registered workload). Implementation always begins
with that tenant's new K2R-class registration, because tenant/repository are
inside its digest; dossier-only mode creates none. Or publish a decision-ready dossier for one, including
exactly why it can or cannot be checked honestly today. A truthful "the
contract is dharma_swarm-shaped in these three ways" finding is a valid K4
result and feeds a v1 contract revision. Before any K4 implementation is
delegated, its separate admission packet must freeze exact repo authority,
commands, expected exits, source operation, protected checker, outer worker,
execution-material closure, changed-entry classification, its own K4R/K4A
authority nodes, budgets, and rollback. An implementation packet must prove
the §4 tenant/repository/operation/registration equalities and red-test
substitution of dharma_swarm's otherwise-valid K2R/K3A refs. Otherwise stop with
`TRUTH_KERNEL_TENANCY_READY_SECOND_TENANT_REQUIRED`; this controller grants
no foreign-target execution.

**Run / Use / Prove / Falsify / Record:** dossier-only mode runs the owning
repo's onboard, `git diff --check`, full gate, and review commands and proves
why no honest source-consuming operation can yet be bound. Implementation
mode runs only the exact commands/exits frozen in its separately human-merged
admission packet and repeats the full §10 negative matrix. Record all ten K3B
outcomes, the relied-upon merge rationale, the second-tenant decision, exact
authority, and exact-head CI.

Exit gate: either a separately admitted second tenant is human-merged with
source-sensitive Path T/A evidence, or the decision-ready dossier is merged
and the truthful `TRUTH_KERNEL_TENANCY_READY_SECOND_TENANT_REQUIRED` handoff
is emitted. Elapsed time or an unbound command is never K4 completion.

## 10. The K3B advisory gate — fail-closed law

The advisory check reports green ONLY when ALL of the following hold:

1. `integrity == I3_INTEGRITY_OK` AND `provenance` is
   `P2_TRUSTED_EXECUTION_OK` or `P3_ATTESTATION_OK`; a PR-uploaded bundle or
   attestation without fresh bundle integrity can never turn the check green;
2. `verdict == V_CLEAN`; `V_FINDINGS`, `V_UNCHECKED`, and `V_ERROR` are
   not-green;
3. `admission_authority == AA2_CHAIN_CURRENT` with the parameterized
   `AuthorityChainV0` freshly revalidated from the current protected authority-
   registry view: platform G0/#62/A1/K1/K2, the selected registration's own merge,
   and this exact tenant repository's A2. A checkpoint, A2 alone, stale
   registry/digest, unauthorized/edited authorized-account event, or missing/substituted/
   revoked/reverted/superseded/cross-tenant predecessor is `AA0_NONE`;
4. the protected §6 subject is exactly current `PR_HEAD`,
   `PR_SYNTHETIC_MERGE`, or `MERGE_GROUP`; moved base/head, expired, cancelled,
   superseded, replayed, push-post-merge, indirect, or unknown evidence is
   `P1_STALE` or typed error;
5. the complete `ExecutionMaterialsV0` closure, not merely direct
   checker/engine pins, equals protected policy and contains no dynamic
   acquisition;
6. the complete base→subject changed-entry manifest is bound and every
   transition has exactly one compatible registered or protected
   non-impacting classification; no uncovered/mixed-incompatible path exists;
7. engine/checker/adapter identity, selected registration and every component
   digest equal protected pins, and §4 tenant/repository/operation/
   registration cross-field equalities all hold;
8. the registered operation consumed the exact source-derived artifact and
   met required fault opportunity/manifestation/effect coverage; and
9. Path T/A freshness, nonce, attempt, conclusion, worker-boundary, bounded
   command, and resource predicates all hold.

Everything else is advisory red. There is no warning level, no skip flag,
and no path by which the check flips itself to required (that is a separate
human `GP3_REQUIRED` act on the independent `gate_policy` axis, §4). A GP3
decision never replaces the required AA2 chain. If a later human selects GP3,
required-policy eligibility adds a tenth conjunct: the current `GP3PolicyRefV0` equals this
decision's tenant/repository/operation/registration/check/DAG, permits the
subject variant and exact enforcement identity, and the protected checker's
separate observation binds that policy digest, the freshly matching active
ruleset object, empty/no-implicit bypass set, activation-event digest,
dedicated publisher App, and this exact current `PR_SYNTHETIC_MERGE` or
`MERGE_GROUP`, never `PR_HEAD`. Wrong-tenant, wrong-check, wrong-DAG,
stale/wrong-policy observation, inactive/deleted/wrong-target/check/integration/
bypass enforcement, shared publisher, missing activation event, stale-subject,
and PR-head-required fixtures are red. Only a later validated
`GateEnforcementOutcomeV0` supports a claim that provider enforcement actually
occurred.
The check's own implementation must carry red tests for
each numbered case above. The admission constructor in §4 is the only green
path; there is no independent workflow-language shortcut.

## 11. Mandatory gate matrix

| property | required proof |
|---|---|
| Reality Bridge unharmed | its controller and track acceptance/non-goal law remain byte-identical; this proposal's `ACTIVE_TRACK.yaml` change is limited to the exact §8 additions. After K2R, only K2's frozen-manifest adapter/interface edits may alter merged Reality Bridge runtime surfaces, with all existing Reality Bridge regression gates green and no mid-flight collision |
| single truth path | no parallel/replacement truth format: additive tenancy/Path-T/Path-A envelopes wrap R0 and Rust alone constructs the decision. Gate-wired `scripts/check_tenancy_single_truth.py --self-test` must catch seeded Python verdict, workflow-green, replacement-receipt, and Rust-bypass fixtures; its live scan must report zero |
| protected provenance | checker/policy comes only from protected immutable identity; the complete transitive `ExecutionMaterialsV0` closure and authenticated engine/release subject match exactly; current distributed authority-registry view and full parameterized authority DAG are freshly bound; no local `verify_bundle`-only path; every §5 negative red-first |
| worker safety | protected checker executes no PR code; disposable unprivileged zero-credential/no-token/no-OIDC/no-metadata worker enforces numeric outer bounds; an in-worker credential-inventory red fixture proves absence before input; any check-write credential stays solely in the protected controller; result remains D2 with 29 channels open |
| source binding and sensitivity | closed §6 subject union accepts only current PR-head/synthetic-merge/merge-group for K3B; complete tree diff is totally/exclusively classified; source comes only from bounded regular Git objects via lexical no-follow paths; unknown event, caller identity, stale relation, uncovered/mixed path, symlink/gitlink/special/traversal/overflow are red; a covered tenant-source mutant changes the result |
| non-weakenable registry | §7 generated enumeration agrees across help/capabilities/request/receipt/verifier; tenant/repository/operation/covered surfaces and exact composite/component digests, compatibility, budget, and fault lifecycle are checked in named pre-execution negatives |
| fail-closed tenancy | every §10 numbered case demonstrated red; advisory green only via Path T/A |
| evidence ≠ authority | Rust validates §4 product/transition schema, bootstrap-frozen authorized-owner-account #62 record (account attribution, not personhood proof), distributed current authority/revocation view, platform prefix + selected registration + same-tenant A2 DAG and cross-field equalities, plus independent gate policy; GP3's durable policy equals tenant/check/DAG/repository/enforcement identity and its separate protected-checker observation binds the activation event, dedicated publisher, empty bypass state, freshly matching active ruleset, and exact current synthetic-merge/merge-group subject; only `GateEnforcementOutcomeV0` supports an actually-enforced claim; missing/substituted/revoked/reverted/superseded/cross-tenant/partial authority or disabled/mismatched enforcement fails; promotion memo is prose only |
| executable bounds | every K2R/K2 payload uses the exact gate-tested bounded runner, `--locked --offline -j 1`, deny-egress worker, fresh one-use roots, and exact exit/machine assertions; timeout/process/output/memory/disk/network self-tests are red-first |
| capability honesty | capability statement generated from Rust-published data; D2 and 29-open-channels language present verbatim |
| tenant reality | K3B verdicts cite real PRs and product/component digests, misses included; actual source, not only a model, affects the result; at least one current `MergeRelianceAttestationV0` binds the exact advisory decision/bundle/run/subject and same authorized merger account, explicitly as account attribution rather than proof of human cognition |
| no scope creep | zero edits to dharma_swarm spine schema, telos gates, or merge authority beyond the additive attributes binding |
| full integration | current-main rebase; `make gate` twice plus fresh `make review`; `ci / gate`, every verifier OS job, and aggregate green. Query runs/checks by immutable run id + attempt + event, never a universal check-suite-head rule; prove the exact closed §6 variant. `pull_request_target` is orchestration only; queued admission requires current merge-group execution/membership; `PUSH_POST_MERGE` is separately typed post-merge evidence and never K3B admission. All actionable threads answered and no unresolved P1/P2; any push/rebase/base movement invalidates prior evidence; dharma_swarm exact K3A commands plus its full review/CI contract green on final K3B subject relation |

## 12. Run management — ceilings, retries, checkpoint/resume, rollback

Applies to every package in §9. These are proposal defaults; the human
merging a package PR may tighten, never loosen, them.

**Time and cost ceilings (per package, per agent run):**

- focused-work ceiling: K1 ≤ 6h; K2R ≤ 12h; K2 ≤ 12h; K3A ≤ 4h;
  K3B ≤ 16h; K4 ≤ 20h —
  excluding human merge/authorization pauses, which suspend the clock;
- dollar ceiling: $0 incremental spend — no paid model, compute, API, or other
  provider invocation. GitHub repository API traffic is allowed only under the
  following network rule and must incur no incremental charge;
- network ceiling: the protected controller may make only the authenticated,
  read-only GitHub repository/PR/CI/collaborator/ruleset/audit-object queries
  required by §§4–6 and §12, plus one separately human-authorized check-result
  write through the dedicated publisher App defined in §4. Every K2R/K2
  payload and untrusted worker has deny-egress; no other network use is allowed;
- command ceiling: 45 minutes and 10 MiB retained stdout/stderr per command;
  timeout/error is a typed non-pass, never a silent retry;
- generated-work ceiling outside read-only pinned material caches: 10 GiB disk,
  8 GiB memory, four concurrent payload-tree processes, and separately named
  one-use command/work/artifact roots within one aggregate lane budget; K3B's
  disposable worker may be tighter but never broader; and
- universe/seed/fault ceiling: exact numeric values frozen in K1/K2R registry
  policy and K3A charter. Omitted numbers block delegation; a package may not
  raise or lower them at runtime.

**Retries:** at most TWO TOTAL ATTEMPTS (the initial attempt plus one retry)
for the same check/cause, with distinct attempt ids and both receipts kept.
The second same-cause failure is the stop condition (§13). A new attempt is
allowed only for a materially different cause/evidence and starts a new
bounded counter; no result or artifact is overwritten.

**Checkpoint/resume receipt (off-git, NDJSON, one record per checkpoint):**

```json
{"record":"truth-kernel-checkpoint","schema":3,"package":"K3B","wave_state":"IN_PROGRESS",
 "repo":"...","base_sha":"...","head_sha":"...","package_predecessor_shas":["..."],
 "subject_observation":{"schema":"ExecutionSubjectObservationV0","tag":"PR_HEAD",
   "authority_key":{"tenant_id":"...","canonical_repository":"...","operation_id":"tenant-verify-v1","registration_digest":"..."},
   "repository":"...","pr":0,"base_sha":"...","head_sha":"...","execution_sha":"...","tree":"..."},
 "diff_observation":{"schema":"ClassifiedDiffObservationV0",
   "authority_key":{"tenant_id":"...","canonical_repository":"...","operation_id":"tenant-verify-v1","registration_digest":"..."},
   "manifest_digest":"...","classifier_digest":"...","all_entries_classified":true},
 "execution_materials_observation":{"manifest_digest":"...","closure_digest":"...","engine_sha256":"..."},
 "authority_registry_view_observation":{"schema":"AuthorityRegistryDigestObservationV0","view_digest":"...",
   "platform":{"repository":"...","commit":"...","blob":"...","digest":"..."},
   "tenant_resolver":{"repository":"...","default_branch":"...","resolved_commit":"...","tree":"...",
     "a2_path":"...","a2_schema":"...","a2_blob":"...","gp3_path":"...","gp3_schema":"...","gp3_blob":null,
     "gp3_policy_ref_digest":null,"gate_policy_state":"GP0_ADVISORY_ONLY",
     "resolver_policy_blob":"...","resolver_policy_digest":"..."}},
 "authority_dag_observation":{"schema":"AuthorityChainDigestObservationV0","node_manifest_digest":"...","authority_dag_digest":"...","platform":{
   "g0":{"label":"REALITY_BRIDGE_COMPLETE_FORWARD_NULL","authority_ref_digest":"...","governed_blob":"...","merge_sha":"..."},
   "issue62":{"schema":"OwnerAccountIssueAuthorityDigestObservationV0","repository":"...","issue":62,
     "event_kind":"issue_comment","event_id":"...","canonical_payload":"{\"record\":\"truth-kernel-issue-62-clearance\",\"schema\":1,\"decision\":\"PROCEED\",\"bound_g0_result_sha\":\"<40-lowercase-hex>\"}",
     "payload_digest":"...","bound_result_sha":"...",
     "g0_merged_at":"...","actor_node_id":"...","actor_type":"User","author_association":"OWNER",
     "initial_permission":"admin","initial_permission_checked_at":"...","initial_permission_response_digest":"...",
     "premerge_permission":"admin","premerge_permission_checked_at":"...","premerge_permission_response_digest":"...",
     "policy_digest":"...","created_at":"...","updated_at":"...","authority_ref_digest":"..."},
   "a1":{"authority_ref_digest":"...","governed_blob":"...","merge_sha":"..."},
   "k1":{"authority_ref_digest":"...","governed_blob":"...","merge_sha":"..."},
   "k2":{"authority_ref_digest":"...","governed_blob":"...","merge_sha":"..."}},
   "registration":{"authority_key":{"tenant_id":"...","canonical_repository":"...","operation_id":"tenant-verify-v1","registration_digest":"..."},"authority_ref_digest":"...","governed_blob":"...","merge_sha":"..."},
   "tenant_admission":{"authority_key":{"tenant_id":"...","canonical_repository":"...","operation_id":"tenant-verify-v1","registration_digest":"..."},"authority_ref_digest":"...","governed_blob":"...","merge_sha":"..."}},
 "interface_digest":"...","owner":"...","verifier":"...",
 "integration_writer":"...","write_manifest":["..."],"write_lease":"...",
 "last_gate":{"argv":["make","gate"],"source_tree":"...","cwd_rel":".",
   "bounded_record_digest":"...","policy_digest":"...","materials_digest":"...",
   "exit":0,"evidence_path":"...","attempt":1},
 "integrity":"I3_INTEGRITY_OK","provenance":"P0_UNTRUSTED","verdict":"V_CLEAN",
 "admission_authority_observation":{"state":"AA2_CHAIN_CURRENT","authority_dag_digest":"..."},
 "gate_policy_state":"GP0_ADVISORY_ONLY","gate_policy_observation":null,
 "merge_reliance_observation":null,
 "budget":{"attempts_used":1,"wall_seconds_used":0,"bytes_retained":0},
 "next_safe_action":"...","blockers":[],"recorded_at":"RFC3339"}
```

The operator supplies an explicit absolute checkpoint/output directory
outside the repository; nothing writes to `~/.vibe-halt/` or any implicit
path. `write_manifest`, lease, and authority fields coordinate work but do
not sandbox a command and do not confer authority. Every subject, diff,
material, registry, human-event, and DAG value above is a non-authoritative
observation that must be reconstructed, not trusted, on resume.
Types explicitly named `*DigestObservationV0` are digest-indexed checkpoint
summaries, not instances of the complete §4 authority types; reconstruction
must recover and validate every field of each full protected object before a
constructor is attempted. Subject and diff observations still carry the full
`TenantAuthorityKeyV0` so cross-field equality can be checked before resume.
`gate_policy_observation` is `null` under `GP0_ADVISORY_ONLY`; only GP3 stores
a complete `GatePolicyObservationV0`, and that object must bind the current
policy-ref digest, freshly matching active enforcement-object identity/digest,
empty/no-implicit-bypass state, dedicated publisher App, immutable activation-
event digest, and eligible exact execution subject defined in §4. Any later
`GateEnforcementOutcomeV0` is a separate post-merge object, never inferred from
this checkpoint summary.
`merge_reliance_observation` is `null` until the protected reliance resolver
succeeds. The resulting atomic validated transition writes the non-null closed
digest-summary type
`MergeRelianceAttestationDigestObservationV0 { event_id, repository, pr,
execution_subject_tag, execution_subject_digest, subject_sha,
final_pr_base_sha, final_pr_head_sha, pr_merge_commit_sha, pr_merge_tree,
merge_relation_digest, check_run_id,
check_run_attempt, admission_decision_sha256,
evidence_bundle_sha256, comment_payload_sha256, actor_node_id, actor_type,
comment_created_at, comment_updated_at, merged_at,
resolver_policy_digest, observed_at, merge_current_on_main,
change_not_reverted }`, with no unknown/omitted fields. It is non-authoritative,
and emits `TRUTH_KERNEL_K3B_TENANT_RELIANCE_ATTESTED` together; neither the
field nor label may precede the other. K4 entry requires that completed
transition. The summary is always fully re-resolved under the K3A policy rather
than trusted.

Resume protocol: (1) re-run `make onboard`; (2) fetch protected state and
verify current `origin/main`, exact package predecessors, entry predicates,
write lease, and interface/DAG digests; (3) reconstruct the closed subject,
complete tree diff/classification, and execution-material closure from current
protected objects; (4) independently read the current protected authority-
registry view, re-fetch the exact #62 event and current human-authorization policy,
and re-resolve every platform/registration/tenant-admission node, governed
blob, predecessor edge, GP3 policy and live enforcement object when applicable,
its activation event and any claimed merge-enforcement outcome, currentness and
revocation, plus any claimed `MergeRelianceAttestationV0` event/PR/
check/ancestry — never trust cached values;
(5) confirm `HEAD` equals the receipt head or rebase and invalidate all old
exact-head evidence; (6) refresh §3; (7) re-run the last bounded gate and
verifier before new work, reconstructing its canonical argv, cwd, policy, and
materials from current protected package policy — checkpoint text is never
executed; (8) continue only from `next_safe_action`. Forged or
cross-tenant A2, unauthorized/edited/deleted authorized-account event, stale/caller-selected
registry, reverted/superseded node, stale/disabled/mismatched GP3 enforcement,
missing/edited/deleted/stale merge-reliance evidence, mutable material,
uncovered diff, unknown subject, moved head/base, expired lease, or failing
gate is a named red fixture and blocks resume.

**Rollback:** nothing merges without a human, so rollback is: stop the lane,
keep the draft PR for evidence or close it per workflow, delete nothing that
is merged. If a package PR is found harmful after merge, rollback is a new
human-merged revert PR, never an agent act.

**Kill:** any §13 condition halts the lane immediately; the agent emits one
blocker packet and stops.

## 13. Kill and stop rules

Stop the affected lane when:

- a Reality Bridge wave collides on `clients/python/**` or
  `crates/vh-cli/**` without an agreed serialization;
- the tenancy contract would need a second truth authority, stdout parsing,
  a caller-suppliable verdict, or caller-declared source identity to be
  "useful";
- anyone proposes local bundle verification as admission authority, a
  tenant-selectable property subset, a tenant-supplied fault model, or a
  budget change outside the registry (§5–§7);
- a PR-modifiable checker/helper, tenant-selected engine/trust root, or
  protected controller that executes untrusted code is required;
- any required kernel/cgroup/container, deny-egress, aggregate-quota, process-
  reap, identity-observation, or protected-object backend is unavailable;
  polling, caching, or a caller assertion may not substitute for it;
- any transitive execution material is mutable, unresolved, dynamically
  acquired, or absent from `ExecutionMaterialsV0`, or a test-only engine/
  fixture is proposed for production admission;
- the complete base-to-subject tree diff has an uncovered, overlapping,
  mixed-incompatible, executable, configuration, dependency, workflow,
  generated-input, mode, delete/rename, gitlink, or other transition without
  an exact compatible protected classification;
- the observed event cannot construct one current closed §6 subject, an
  indirect event is treated as execution evidence, or `PUSH_POST_MERGE` is
  proposed for PR advisory/merge admission;
- any authority-registry-view component/resolver is caller-selected, cached,
  stale, unavailable, or does not independently resolve every current node;
  the #62 actor/event is pre-G0, outside the freshness window, later-promoted,
  unauthorized, edited, deleted, or superseded; or any registration/A2 node
  crosses the tenant authority key;
- a GP3 artifact is absent after a qualifying event, duplicated, wrong-path,
  reverted, or superseded, or its named provider ruleset is unavailable,
  inactive, deleted, wrong-target/check/integration, nonempty/implicit-bypass,
  shared-publisher, missing/mutable-activation-event, or not freshly equal to
  the protected observation; or actual enforcement is claimed without an exact
  `GateEnforcementOutcomeV0`;
- terminal utility or K4 entry is claimed from a green check/merge alone, or a
  `MergeRelianceAttestationV0` comment/PR/run/subject/digest/actor/currentness field
  is missing, edited, deleted, mismatched, stale, or reverted;
- the gate-wired single-truth scanner finds a Python/workflow verdict,
  replacement receipt, Rust bypass, unknown decision path, or cannot complete
  its exact self-test/live scan;
- the source-derived input can change identity without being capable of
  changing the verdict, or required fault opportunity/effect is absent;
- a D2 child would run without the §5 disposable
  zero-credential/no-token/no-OIDC/no-metadata outer boundary, or anyone
  claims that boundary closed a capability channel;
- the K3B workload cannot be made honestly deterministic (publish the exact
  nondeterminism finding — that is a result, likely a real bug);
- dharma_swarm governance declines the admission PR (respect it; produce a
  decision packet, do not route around its merge authority);
- anyone proposes flipping the advisory check to required inside this
  campaign (`GP3_REQUIRED` is a separate human act, §4);
- anyone proposes foreign-target execution, spending, a paid model/compute/API
  provider call other than the zero-incremental-cost GitHub metadata reads
  explicitly allowed by §12, any
  credential in the untrusted worker, or credential use beyond a separately
  human-authorized least-privilege check-result writer held exclusively by the
  protected controller (`EA4_PRESENT` is out of scope);
- two total attempts fail for the same cause without new evidence.

At a stop, emit one packet: exact blocker, evidence, smallest safe options,
recommendation, consequence of waiting.

## 14. Terminal and interim states (closed typed set)

Closed labels an authoring agent may emit (review-ready is not human merge):

- `TRUTH_KERNEL_K1_SPEC_READY_FOR_HUMAN_REVIEW`
- `TRUTH_KERNEL_K2R_REGISTRATION_READY_FOR_HUMAN_REVIEW`
- `TRUTH_KERNEL_K2_SDK_READY_FOR_HUMAN_REVIEW`
- `TRUTH_KERNEL_K3A_CHARTER_READY_FOR_HUMAN_REVIEW`
- `TRUTH_KERNEL_K3B_IMPLEMENTATION_READY_FOR_HUMAN_REVIEW`
- `TRUTH_KERNEL_K4_IMPLEMENTATION_READY_FOR_HUMAN_REVIEW`
- `TRUTH_KERNEL_K4_DOSSIER_READY_FOR_HUMAN_REVIEW`
- `TRUTH_KERNEL_BLOCKED_<precise_reason>`

The protected checker may observe, but an authoring agent may never self-mint,
these post-merge or authorized-account-event-dependent states after re-resolving
the required events and current §4 registry/DAG. Some are authority states and
some are explicitly evidence/status only; the label itself never promotes one
to the other: `TRUTH_KERNEL_K2R_REGISTRATION_MERGED`,
`TRUTH_KERNEL_K3B_TENANT_LIVE_ADVISORY`,
`TRUTH_KERNEL_K3B_TENANT_RELIANCE_ATTESTED` (only after the exact digest-bound,
same-merger-account attestation in §9),
`TRUTH_KERNEL_TENANCY_READY_SECOND_TENANT_REQUIRED` (only after the K4 dossier
merge in §9), and
`TRUTH_KERNEL_COMPLETE_SECOND_TENANT`.

Controller-level state right now (non-terminal, no authority):

- `TRUTH_KERNEL_PROPOSAL_DRAFT_BLOCKED_BY_ISSUE_62` — PR #58 draft; G0
  requires a human-merged `REALITY_BRIDGE_COMPLETE_FORWARD_CONFIRMED` or
  `REALITY_BRIDGE_COMPLETE_FORWARD_NULL` result plus the separately required
  owner-account #62
  clearance; neither the merged G0 result nor the separate #62 clearance
  currently exists, so the exact A1 bootstrap authority record is necessarily
  absent as well (§3–§4).

Anything else is an interim checkpoint, written as
`TRUTH_KERNEL_CHECKPOINT_<package>_<wave_state>` in a §12 receipt — never a
completion claim.

## 15. Citation and staleness policy

- Citation-or-silence is in force for every package output
  (`CLAUDE.md:22-26`): every durable claim carries a `file:line` citation
  refreshed at writing time, or a runnable command.
- Every line citation in THIS controller was refreshed against base
  `origin/main@2a0190b` and this two-file PR diff on 2026-08-05. After any
  future edit to cited files, refresh or
  remove the citation before merging anything that quotes it.
- OBSERVED checkpoints (§3) expire silently: re-run the refresh commands
  before any gate decision that depends on them.
- Runtime receipts never enter git (`CLAUDE.md:27-29`); §12 checkpoints live
  only in the explicit operator-supplied external output directory.

## 16. Repair provenance (2026-08-05)

This repair addresses five review findings against the 2026-07-29 draft:

| # | finding | closed in |
|---|---|---|
| 1 | PR-author-supplied bundle cannot establish engine provenance; local `verify_bundle` treated as admission authority | §4 product type; §5 protected Path T/A and `I3` ceiling; §10 |
| 2 | no registration wave: K3 could submit an unregistered workload | exact K2R id/fixtures; §8 serial hard edge; §9 K2R |
| 3 | no exact source binding; caller-declared metadata trusted | §6 subject relation and source-sensitivity law; §9 K2/K3B; §10 |
| 4 | tenant-selectable (weakenable) property contracts | §7 composite registry; §9 K1/K2R/K3B; §10 |
| 5 | no versioned fault model; parameter/budget drift open | §7 content/lifecycle binding; §9 K1/K2R/K3B; §10 |

It also integrated Wave B truth (§3), separated evidence from authority
(§4), added the dependency DAG and per-package run management (§8, §9,
§12), and restated the draft/blocked status of this proposal (header, §3,
§14).

The final post-rebase audits additionally refreshed every external line
citation; closed CI execution subjects by event and current relation; gave
K2R exact clean/mutant commands, expectations, exits, and one-use filesystem
shapes; kept the base v2 verifier record closed; removed undefined decision
states; made the untrusted worker uniformly zero-credential; and replaced a
first-tenant hard-coded authority list with a bootstrap-frozen #62 human
record/query policy and a distributed-current-registry-resolved,
tenant-parameterized platform + registration + same-tenant A2 DAG. The
independent gate-policy axis has a registration-pinned tenant path, is whole-
key/check/DAG bound, and reaches required mode only when a separate protected
observation binds matching live provider enforcement plus an exact synthetic-
merge/merge-group subject. The audits also require a complete immutable
transitive execution-material closure, total base-to-subject diff
classification, bounded no-follow Git-object extraction, a closed subject
union, branch-candidate/protected-main provenance separation,
kernel-enforced command/resource/write bounds, and a gate-wired
single-truth scanner; an immutable A1 bootstrap record and exact same-merger-
account reliance attestation; removed the issue-only G0 waiver; and made K4 a truthful
portability test rather than a promised positive result (§3–§13, §15).

---

The campaign's bounded success claim is reached when a tenant repository under
its own human-merged admission charter records through an authorized same-
merger-account attestation that it treated a kernel evidence bundle as a reason
a change merged — with provenance that bundle could not have minted itself —
and every verdict in that chain can be replayed byte-for-byte. This is account
attribution under the overall D2 evidence boundary, not proof that a person was
present, read the evidence, was cognitively caused by it, or was operationally
independent of vibe-halt; anyone who doubts it retains that explicit
uncertainty boundary.
