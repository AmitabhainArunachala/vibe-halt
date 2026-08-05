# VB-008 R4 Decision Packet — 2026-08-05

**STATUS: BLOCKED — NOT AUTHORIZATION READY — NON-TERMINAL.** This packet grants **no
foreign execution authority** of any kind. It is a preparation-only decision
artifact for vibe-halt issue #60: it records a verified public-metadata snapshot,
records current capability truth, and lists the exact blockers that stand
between this repository and any R4 foreign-target confirmation attempt.
Nothing in this file authorizes cloning, fetching, installing, instrumenting,
or executing `langchain-ai/langgraph` or any other foreign code.

**Truthful typed checkpoint (now):**

- packet state: `R4_NOT_AUTHORIZATION_READY_TARGET_OPERATION_AND_DOSSIER_INCOMPLETE`
- terminal: `false`; this packet does not close issue #60 and does not satisfy
  any downstream requirement for a terminal Reality Bridge result
- existing synthetic fixture: `status=ADMISSIBLE`, `cohort=CALIBRATION` only;
  that admission validates schema shape and grants zero R4 execution authority
  (`corpus/calibration/vb008_langgraph_6491.json:1`)
- real-target dossier: `ABSENT`; any proposed record must remain
  `status=NOT_ADMISSIBLE` until exact permissible revisions, mechanism,
  independent oracle, commands, environment, and budget are bound
  (`docs/prompts/VIBE_HALT_REALITY_BRIDGE_LONG_RUNNING_GOAL_2026-07-29.md:549-553`)
- `candidate_state`: `UNRUN` (as checked in, `corpus/calibration/vb008_langgraph_6491.json:1` field `candidate_state`)
- `bridge_execution`: `null` (same file, field `bridge_execution`)
- `acceptance_credit`: `false` (same file, field `acceptance_credit`)
- cohort: `CALIBRATION` only (same file, field `cohort`); no unknown-bug or
  holdout credit is claimed or claimable from this packet.

**Absence of recorded authority or an executed attempt is not `FORWARD_NULL`.** A
`FORWARD_NULL` requires a valid bridge-coupled approved attempt, verified
receipts, fixed-budget exhaustion, an identical fixed-control protocol, and
absence of the frozen mechanism; it maps from `MISS`
(`docs/specs/HOLDOUT_CONTRACT_V1.md:91`;
`docs/prompts/VIBE_HALT_REALITY_BRIDGE_LONG_RUNNING_GOAL_2026-07-29.md:95-107`;
`docs/prompts/VIBE_HALT_REALITY_BRIDGE_LONG_RUNNING_GOAL_2026-07-29.md:560-592`).
No authorization or attempt is recorded in issue #60, its comments, or the
repository evidence, so `bridge_execution` remains `null` and
`candidate_state` remains `UNRUN`. The schema permits a later transition to
`AUTHORITY_BLOCKED` but does not define its trigger
(`docs/specs/HOLDOUT_CONTRACT_V1.md:51-69`); this packet conservatively keeps
`UNRUN` while technical blockers remain. Absence of execution is silence, not
a null result.

**Recommended decision: DEFER** (see Decision matrix). First resolve mechanism
eligibility and the fail-closed null-validator gap. Candidate-neutral
revision-bound target-operation support may be developed against repo-local
synthetic fixtures in parallel, but it cannot admit VB-008 by itself. This
packet neither authorizes nor implements any of those packages.

## Scope and explicit forbidden actions

This packet is produced under preparation-only authority: reading this
repository, read-only `gh` issue/PR/API calls against public GitHub metadata,
and repo-local non-foreign-target checks (`make onboard`, `vh eval-validate`,
`git diff --check`, `make gate`).

The following remain **forbidden** and were not performed in the
foreign-target lane:

- cloning or fetching a foreign repository; downloading archives; installing
  dependencies; running, importing, or testing LangGraph or any foreign code;
- instrumenting or writing to any foreign target;
- contacting live services other than read-only public GitHub metadata;
- using the existing GitHub credential to mutate the foreign repository or
  record target authorization; inspecting or exposing secret material;
  spending money;
- commenting on or writing to the foreign target; approving, merging, or
  claiming target authority.

Repository-local publication of this one packet on a draft PR is allowed by
the development workflow, but publication, CI, review, or merge of this packet
does not authorize R4 execution (`docs/DEVELOPMENT_WORKFLOW.md:85-116`).

## Provenance snapshot

- vibe-halt preparation base: `ed32f1ddf7ee1c9cc676c40b5901b220b02fe25a`,
  verified locally via `git rev-parse HEAD`; the publication PR must report
  its own exact head and fresh checks.
- This packet is the only repository path changed by the preparation branch.
- All upstream GitHub facts below were re-fetched read-only via `gh` on
  2026-08-05 (~13:14 UTC) and are stated as of that timestamp; issue/PR
  liveness facts are mutable and must be refreshed and rebound at any future
  authorization review.

## Candidate identity table (OBSERVED symptom/fix context, not R4 admission)

The checked-in synthetic record is admitted only to the non-credit
`CALIBRATION` schema fixture. The public revisions below are **provisional
upstream symptom/fix candidates**, not an admitted R4 treatment/control pair.
No qualifying independent mechanism, oracle, treatment/control command, or
execution receipt exists.

| item | identity | evidence |
|---|---|---|
| target repo | `langchain-ai/langgraph`, public, not archived, default branch `main` | `gh api repos/langchain-ai/langgraph` 2026-08-05 |
| source issue | #6491 OPEN, "Invalid state saved to checkpoint without validation, causing permanent corruption.", author `goma-25`, created 2025-11-24T07:10:14Z, labels `bug`,`pending`,`external`, 4 comments | https://github.com/langchain-ai/langgraph/issues/6491 |
| proposed fix PR | #6512 OPEN, "fix(langgraph): validate node outputs against State schema before checkpointing", author `dumko2001`, created 2025-11-27T14:14:54Z, base `main`, head branch `fix/invalid-state-checkpoint`, `reviewDecision=REVIEW_REQUIRED`, `reviews=0`; REST reports `mergeable=true`, `mergeable_state=blocked`, `rebaseable=false` | https://github.com/langchain-ai/langgraph/pull/6512 |
| provisional upstream symptom revision | commit `630bd9ab953da68e9a00c46e8245176a76c697f7`, tree `49ce2c73d203f7625d1aa3292edda00be4b20ab4`, parent `b945b1f21e0eaf4b06235e7c72afd2e3c0ced7f6`, committer date 2025-11-26T00:02:44Z, message "fix: release name should be same as package name (#6503)" | https://github.com/langchain-ai/langgraph/commit/630bd9ab953da68e9a00c46e8245176a76c697f7 |
| provisional upstream proposed-fix revision | commit `34783594f8611c2a8174a6929f3e8834456043db`, tree `a2e9559390a757bedc4f6a40a27400449b1343a7`, **sole parent** `630bd9ab953da68e9a00c46e8245176a76c697f7`, committer date 2025-11-27T14:13:13Z, message "fix: validate state updates before checkpointing …" | https://github.com/langchain-ai/langgraph/commit/34783594f8611c2a8174a6929f3e8834456043db |
| observed upstream `main` | commit `fb3d5f0399222504e015fe959e0e79fdc6e00a65`, tree `397ac863feda9d56cc447abcded5957fbd28786d`; GitHub compare reports 630 commits after `630bd9ab…` | https://github.com/langchain-ai/langgraph/commit/fb3d5f0399222504e015fe959e0e79fdc6e00a65 |

Notes, stated plainly:

- The two candidate revisions form a one-commit parent/child diff, but they
  are 630 commits behind observed upstream `main`. Neither has been executed,
  built, or checksummed locally. They are identities, not admissions.
- OPEN/review/check state is mutable risk context, not an R4 admission
  predicate and not proof for or against correctness. Exact revisions remain
  unproven because no qualifying independent treatment/control execution or
  mechanism-matched receipt exists.
- The checked-in local dossier does NOT reference these revisions: its
  `pre_fix_revision` / `post_fix_revision` are the literal placeholders
  `SYNTHETIC-PRE-FIX-PLACEHOLDER-VB008` /
  `SYNTHETIC-FIXED-CONTROL-PLACEHOLDER-VB008`
  (`corpus/calibration/vb008_langgraph_6491.json:1`).

## License evidence

- Repository license: SPDX `MIT` (key `mit`), per
  `gh api repos/langchain-ai/langgraph` and `gh api repos/langchain-ai/langgraph/license`, 2026-08-05.
- `LICENSE` blob `fc0602feecdd6748623c852ab534e1ca612673c7` (1072 bytes) is
  byte-identical at BOTH provisional revisions (contents API at
  `ref=630bd9ab…` and `ref=34783594…`, 2026-08-05). Permalink at treatment:
  https://github.com/langchain-ai/langgraph/blob/630bd9ab953da68e9a00c46e8245176a76c697f7/LICENSE
- The observed MIT text is evidence for a future permitted-use decision, not
  legal or execution authority. A human must still explicitly approve the
  contemplated local test use. Per the R4 law, no foreign repository,
  dependency tree, generated
  environment, or copyrighted source snapshot may be committed into
  `vibe-halt` — only minimal provenance, protocol, and bounded evidence
  (`docs/prompts/VIBE_HALT_REALITY_BRIDGE_LONG_RUNNING_GOAL_2026-07-29.md:594-596`).

## Semantic mismatch — upstream symptom vs local reduced mechanism

These are related but **not equivalent**, and this packet does not claim
equivalence:

- The upstream issue demonstrates a **validation-asymmetry** bug: a node
  returns invalid Pydantic state (`None` inside a `List[str]` field), the
  write side checkpoints it without validation, and the read side
  (`get_state_history`) re-validates on retrieval, raises `ValidationError`,
  and the checkpoint history is permanently unrecoverable (issue #6491 body,
  fetched 2026-08-05; local provenance summary at
  `corpus/entries/VB-008-unvalidated-checkpoint.md:25-32`). The published
  reproduction names no torn-write injection and manifests the validation
  asymmetry deterministically.
- Local VB-008 requires the **reduced write-side-validation-gap plus
  torn-write mechanism**: the simulated checkpointer acknowledges after
  write → flush → fsync without validating or reading back, on a
  torn-writes-only palette, so an acknowledged checkpoint becomes
  unrecoverable only when the validation gap meets a tear — half a record
  persists, the terminator is gone, retrieval rejects it
  (`corpus/entries/VB-008-unvalidated-checkpoint.md:34-44`).

The local oracle is `checkpoint_recoverable` — every acknowledged checkpoint
must be recoverable at retrieval, `acked:<id>` ⇒ `recovered:<id>`
(`corpus/entries/VB-008-unvalidated-checkpoint.md:47-51`) — and its
`required_facts` demand that the oracle independently re-derive checkpoint
membership from the raw durable dump plus each checkpoint's expected framed
record, never trusting a workload-precomputed `recovered:<ckpt>` Boolean
(`corpus/entries/VB-008-unvalidated-checkpoint.md:23`).

The proposed-fix commit adds Pydantic output validation. Inspection of its
immutable one-commit diff supports no claim that it closes the local
torn-write/read-back gap. It is therefore not currently a valid fixed control
for the reduced VB-008 mechanism
(https://github.com/langchain-ai/langgraph/commit/34783594f8611c2a8174a6929f3e8834456043db).

**Admission blocker:** for the current VB-008 contract, exact
treatment/control revisions must be identified where the existing
torn-write/read-back mechanism manifests and is fixed. Redefining the target
as validation asymmetry would require a separately versioned, human-merged
mechanism, oracle, dossier, and commitment; it would forfeit comparability and
confirmation credit for the existing VB-008 reduction.

Until the mechanism-matched path is complete, the exact oracle, required
facts, and candidate pair remain **TO BE BOUND**; no real-target dossier exists
and any proposal must remain `NOT_ADMISSIBLE`. A mere reproduction of the
published symptom does not count if the frozen reduced mechanism was not
forced
(`docs/prompts/VIBE_HALT_REALITY_BRIDGE_LONG_RUNNING_GOAL_2026-07-29.md:542-553`;
`docs/prompts/VIBE_HALT_REALITY_BRIDGE_LONG_RUNNING_GOAL_2026-07-29.md:588-592`).

## Current VIBE HALT truth (capability and schema gaps)

- The checked-in VB-008 dossier is `status=ADMISSIBLE` only as a
  **deliberately synthetic CALIBRATION fixture**: revisions, injection seam,
  evaluator image, toolchain,
  treatment/control commands, and required facts are all `SYNTHETIC-*`
  placeholders; the salt is a synthetic public salt
  (`corpus/calibration/vb008_langgraph_6491.json:1`, exact fields and
  `commitment_salt`). The schema defines the revision and command fields as
  synthetic placeholders for this calibration shape
  (`docs/specs/HOLDOUT_CONTRACT_V1.md:31-38`). Preserve that original public
  commitment unchanged; a real-target dossier must be a new versioned record.
- `vh eval-validate` performs **shape/state/commitment validation only** and
  never executes target code: "this validator checks shape, state
  transitions, and commitment/reveal consistency. It does NOT select hidden
  cohorts, generate real secrets, award criterion-3/4 credit, or execute
  target code" (`crates/vh-cli/src/eval.rs:9-12`; rule 7 "Never executes any
  target code", `docs/specs/HOLDOUT_CONTRACT_V1.md:96-113`).
- The current cooperative transport **accepts only `cooperative-echo`**:
  any other `--workload` is rejected with exit 2
  (`crates/vh-cli/src/cooperative.rs:793-799`; usage line
  `crates/vh-cli/src/main.rs:106`). The executed child is a hardcoded
  fixture script (`crates/vh-cli/src/cooperative.rs:27-88`) and the Python
  adapter pins the same contract (`clients/python/vibe_halt/core/request.py:178-188`).
  It cannot bind, stage, or run a LangGraph checkout.
- **Observed-target-revision binding and a real foreign receipt remain
  absent**: the strict local client "does not yet implement the
  `dharma_swarm` sandbox ABC, operation/feature negotiation,
  observed-target-revision binding, or a real foreign-target receipt"
  (`VISION.md:84-88`); criterion 7 is OPEN with "operation/feature
  negotiation, observed-target-revision binding, the `dharma_swarm` adapter,
  and a real receipt do not [exist]"
  (`docs/plans/VIBE_HALT_BUILD_PLAN_2026-07-20.md:35`, reaffirmed
  `docs/plans/VIBE_HALT_BUILD_PLAN_2026-07-20.md:150-156`).
- Governance gate: "P0 OPERATOR GATE: issue #60 may authorize one exact
  VB-008 or VB-010 foreign-target confirmation or its predeclared null;
  execute neither without separate human approval of repository, revision,
  license, data boundary, oracle, fixed budget, and disposable environment"
  (`docs/governance/ACTIVE_TRACK.yaml:88`).
- Credit law: dossiers may not award criterion-3/4 credit
  (`docs/specs/HOLDOUT_CONTRACT_V1.md:7-11`); CALIBRATION dossiers claiming
  acceptance credit are rejected by the validator
  (`crates/vh-cli/src/eval.rs:220-222`); existing corpus entries receive no
  retrospective holdout credit (`corpus/SCHEMA.md:100-101`).
- **Validator gap:** the contract says `fixed_control_miss` is true only for
  `FORWARD_CONFIRMED` (`docs/specs/HOLDOUT_CONTRACT_V1.md:44,87-95`), but the
  current validator requires true for a confirmation without rejecting true
  for `FORWARD_NULL` or `FORWARD_INVALID`
  (`crates/vh-cli/src/eval.rs:224-247`). A red-first negative regression and
  fail-closed fix are required before R4 can rely on this field.

## Mechanism and oracle obligations for any future attempt

1. The human-ratified frozen mechanism must be forced, not merely a similar
   upstream symptom.
2. The oracle must produce a Rust-owned machine result and verified receipt
   through R1/R2 (`docs/prompts/VIBE_HALT_REALITY_BRIDGE_LONG_RUNNING_GOAL_2026-07-29.md:571-586`).
   Its required facts must be independently derived from durable target facts,
   never a workload-precomputed Boolean
   (`corpus/entries/VB-008-unvalidated-checkpoint.md:23`).
3. Treatment must manifest and the genuine mechanism-matched fixed control
   must pass under the identical protocol. The observed `630bd9ab…` /
   `34783594…` pair does not yet meet that obligation. A
   `FORWARD_CONFIRMED` claim additionally requires `fixed_control_miss=true`
   (`crates/vh-cli/src/eval.rs:233-235`;
   `docs/specs/HOLDOUT_CONTRACT_V1.md:89-90`).
4. Any result appearing only after changing the oracle or budget is
   exploratory and earns no confirmation credit
   (`docs/prompts/VIBE_HALT_REALITY_BRIDGE_LONG_RUNNING_GOAL_2026-07-29.md:588-592`).

## Field-by-field preauthorization checklist

`OBSERVED` means public evidence was read, not authorized. `PROPOSED` means a
future operator must approve the literal value. `TO BE BOUND` and `BLOCKER`
make authorization fail closed. No executable command is invented here.

| field | value |
|---|---|
| repo identity | OBSERVED: `langchain-ai/langgraph`; operator approval required |
| symptom/fix candidates | OBSERVED: `630bd9ab…` / `49ce2c73…` and `34783594…` / `a2e95593…`; BLOCKER as treatment/control until the mechanism contract is resolved |
| treatment commit / tree | TO BE BOUND: exact 40-hex commit and tree for the frozen mechanism |
| fixed-control commit / tree | TO BE BOUND: exact 40-hex commit and tree for the same mechanism and protocol |
| dirty/submodule policy | BLOCKER: require a byte-clean checkout, an explicit submodule policy, and recorded tree verification before setup |
| license / permitted use | OBSERVED: SPDX MIT and blob `fc0602feecdd6748623c852ab534e1ca612673c7`; explicit operator approval of the contemplated use still required |
| engine commit + executable digest | TO BE BOUND: merged-main vibe-halt commit and independently computed executable digest |
| adapter commits + executable/schema digests | BLOCKER: no revision-bound target adapter exists (`VISION.md:84-88`) |
| target source vs data | PROPOSED: an authorized source checkout may exist only inside the disposable environment; evaluator inputs are synthetic only; no user, production, or private data |
| secrets | PROPOSED: no tokens, credentials, signing agents, cloud metadata, or host environment secrets exposed |
| network and acquisition | BLOCKER: exact endpoints and artifact hashes required; proposed flow is fetch-only with no build/install hooks, verify hashes, disable egress, then unpack/install/build/test offline (`docs/prompts/VIBE_HALT_REALITY_BRIDGE_LONG_RUNNING_GOAL_2026-07-29.md:565-569`) |
| dependency allowlist | BLOCKER: exact toolchain, lockfiles, packages, hashes, and provenance not established |
| local injection seam | BLOCKER — dossier seam is `SYNTHETIC-INJECTION-SEAM-VB008`; a real seam (fixture/monkeypatch/mock/signal at an explicit hook) is undeclared |
| Rust-owned oracle + exact required facts | BLOCKER: must be bound before any attempt (see Mechanism and oracle obligations) |
| treatment command + expected exit | BLOCKER — never invented; dossier holds `SYNTHETIC-TREATMENT-COMMAND-NOT-EXECUTED` |
| control command + expected exit | BLOCKER — never invented; dossier holds `SYNTHETIC-CONTROL-COMMAND-NOT-EXECUTED` |
| attempts / time / CPU / RAM / disk / process / output bounds | BLOCKER: exact numeric one-attempt budget required (`docs/governance/ACTIVE_TRACK.yaml:88`) |
| dollar budget | PROPOSED: zero dollars; no paid services or paid compute |
| ephemeral environment | BLOCKER: name and attest a disposable VM image/runtime, unprivileged identity, mount/socket/device/capability policy, resource limits, and egress transition. A host worktree alone is insufficient because all 29 D2 channels remain open (`docs/specs/SANDBOX_CAPABILITY_ENVELOPE_V1.md:16-33`) |
| retained bounded evidence | BLOCKER: exact total byte/count/retention caps required for commands, exits, diagnostics, receipts, and digests. The existing 256-byte per-diagnostic cap is precedent, not a total R4 evidence bound (`crates/vh-cli/src/cooperative.rs:514-523`) |
| cleanup | BLOCKER: exact destruction, cache deletion, evidence export, and post-destroy verification required |
| rollback / kill | PROPOSED: halt and record `INVALID` on identity drift, boundary escape, unexpected writes, egress, or unverifiable evidence; destroy the disposable environment and inspect declared host/output surfaces rather than assuming they were untouched |

## Predeclared result mapping

Required by the evaluation law before any attempt:

- `DETECTED` → `FORWARD_CONFIRMED` (requires `fixed_control_miss=true`)
- `MISS` → `FORWARD_NULL` only after exactly one operator-approved, valid,
  bridge-coupled attempt exhausts its fixed budget; treatment and the genuine
  fixed control run under the identical protocol; every required receipt
  verifies; the frozen mechanism is absent; `fixed_control_miss=false`;
  `acceptance_credit=false`; and the append-only state log ends in `MISS`
- `INVALID` → `FORWARD_INVALID` (non-completing)

(`docs/specs/HOLDOUT_CONTRACT_V1.md:87-95`; transition law at
`docs/specs/HOLDOUT_CONTRACT_V1.md:60-69`.) Candidate/bridge pairing is
mechanically enforced at `crates/vh-cli/src/eval.rs:224-248`; verified
receipts, budget exhaustion, mechanism absence, identical protocol, and
correct null control semantics remain external proof obligations. The broader
authorization and budget law
is at
`docs/prompts/VIBE_HALT_REALITY_BRIDGE_LONG_RUNNING_GOAL_2026-07-29.md:95-107`
and
`docs/prompts/VIBE_HALT_REALITY_BRIDGE_LONG_RUNNING_GOAL_2026-07-29.md:560-592`.
A `DETECTED → MISS` transition is a red-matrix violation
(`docs/specs/HOLDOUT_CONTRACT_V1.md:69`). The validator gap recorded above must
be fixed before it can mechanically enforce the full null obligation.

## Decision matrix

| option | meaning | consequence |
|---|---|---|
| AUTHORIZE now | permit an R4 attempt immediately | **Invalid**: no admitted mechanism-matched dossier, revision-bound operation, oracle, seam, commands, environment, or numeric budget |
| **DEFER (recommended)** | publish this non-terminal packet; resolve mechanism eligibility and the validator gap; develop only candidate-neutral capability in parallel | Preserves `UNRUN`/`null` truth and prevents transport work from laundering admission |
| DECLINE | explicitly end or pause the VB-008 execution lane | Retains this packet as historical evidence; records operator-declined/authority pause; never becomes `FORWARD_NULL` |

## Exact blockers

1. No revision-bound target-operation support in the bridge
   (`VISION.md:84-88`; `docs/plans/VIBE_HALT_BUILD_PLAN_2026-07-20.md:35`).
2. Cooperative transport accepts only `cooperative-echo`
   (`crates/vh-cli/src/cooperative.rs:793-799`).
3. The observed upstream fix does not control the reduced torn-write
   mechanism; the contract path and genuine treatment/control pair are unresolved.
4. Real-target oracle and exact required facts are not bound; no independently
   reviewable harness can yet force the reduced mechanism on the real
   revisions (semantic mismatch section).
5. Treatment/control commands, injection seam, dependencies, VM boundary,
   evidence caps, and numeric resource budget are BLOCKERs.
6. The null-state validator does not fail closed on
   `fixed_control_miss=true`; a negative regression and fix are missing.
7. No separate human authorization under the P0 operator gate
   (`docs/governance/ACTIVE_TRACK.yaml:88`).
8. R1/R2/R3 are merged and the preparation base passes its local gate, but an
   executable named dossier and its operator authorization are absent. Every
   R4 predicate must be rechecked on then-current merged main
   (`docs/prompts/VIBE_HALT_REALITY_BRIDGE_LONG_RUNNING_GOAL_2026-07-29.md:560-563`).

The upstream issue/PR remaining open and unreviewed is mutable risk context,
not a legal admission blocker and not correctness evidence.

## Repo-local non-foreign-target validation commands

The preparation passes reported these repository-local checks at base
`ed32f1ddf7ee1c9cc676c40b5901b220b02fe25a` on 2026-08-05:

- `make onboard` → verdict READY (deny-list scan PASS, governance PASS).
- `cargo run -q --locked --offline -p vh-cli -- eval-validate --dossier corpus/calibration/vb008_langgraph_6491.json` → `verdict: VALID`, exit 0.
- `cargo run -q --locked --offline -p vh-cli -- eval-validate --dossier corpus/calibration/manifest.ndjson` → `verdict: VALID`, 2 dossiers, exit 0.
- `make gate` → `ALL PASS`, including the pinned VB-008 corpus recall gate
  (`scripts/gate.sh:331`).

None of these commands executes, binds, fetches, or validates any foreign
code. Because authoring output is not a committed runtime receipt, the exact
publication head must independently rerun `make onboard`, both validators,
`make gate`, `make review`, and `git diff --check origin/main...HEAD`; the draft
PR and exact-head CI own those results. Publication also records
`git status --short` and `git diff --stat origin/main...HEAD`
(`docs/DEVELOPMENT_WORKFLOW.md:67-107`).

## Human DEFER / AUTHORIZE_ONCE / DECLINE template (future)

To be completed only by a human after the blockers above are cleared and the
future implementation package is human-merged. `AUTHORIZE_ONCE` is invalid if
any field is missing, symbolic, expired, or fails exact identity verification.
Decision authority is non-delegable and covers only the named one-shot
attempt. An editable comment URL alone is not authority: the canonical decision
bytes and actor must be bound by a verified signature or a committed comment
snapshot and revalidated immediately before consumption.

```
Decision: DEFER | AUTHORIZE_ONCE | DECLINE
Reason: <required literal reason>
Authority ID / nonce: <unique non-reusable values>
Operator identity / role: <authenticated human handle and authority basis>
Authority attestation: <verified signature + key id OR committed comment snapshot>
If comment-backed: <issue URL / comment node id / author node id / createdAt / updatedAt / canonical body SHA-256 / snapshot commit+blob+file SHA-256>
Decided at / expires at: <RFC3339> / <RFC3339; required for AUTHORIZE_ONCE>
Attempt ID: <unique one-shot id>
Consumption rule: <first foreign checkout, dependency acquisition, or approved command consumes authority; partial execution consumes; retry requires new authority>
Consumption receipt: <off-git destination / digest / timestamp; runtime receipt never enters git>
Revocation recheck: <authenticated route and exact immediately-before-start command/check>
Authorization-base main: <merged-main 40-hex>
Ancestry proof: <packet/dossier/mechanism/implementation commits are ancestors of authorization-base main>
Fresh main proof: <make gate receipt + GitHub CI and Verify run IDs at authorization-base main>
Packet path / commit / blob / file SHA-256: <literal identities>
Implementation commit: <ancestor of authorization-base main>
Named real-target dossier path / commit / blob / file SHA-256: <literal identities>
Dossier validation proof: <validator executable digest / exact argv / exit 0 / canonical output SHA-256>
Repository and source issue: <literal URL> / <literal issue URL>
Frozen mechanism contract path / commit / blob: <literal values>
Treatment commit / tree: <40-hex> / <40-hex>
Fixed-control commit / tree: <40-hex> / <40-hex>
Checkout policy: <clean-tree verification / submodule policy / LFS policy>
License blob and permitted use: <blob> / APPROVED | DECLINED
Target source and data boundary: <literal allowed and forbidden bytes>
Secret and credential policy: <literal exclusions and environment scrub proof>
Dependency/toolchain lock and hashes: <literal artifact identities>
Network acquisition / test policy: <literal endpoints; fetch-only; egress-off point>
Disposable VM identity and controls: <image/runtime/uid/mount/socket/device/capability limits>
Allowed write roots and external side effects: <literal paths and explicit NONE list>
Runner identity: <named human/agent/process identity permitted to execute literal commands>
Injection seam and digest: <literal path/hook/digest>
Rust-owned oracle and required facts: <version/path/digest/facts>
Treatment / injection / control commands and expected exits: <literal argv records>
One-attempt numeric budget: <attempts/time/CPU/RAM/disk/process/output/$>
Engine / adapter / executable / protocol-schema digests: <literal identities>
Evidence policy: <allowed artifact classes / total caps / off-git export destination / access / redaction+secret-scan proof / foreign-source exclusion / retention / deletion proof>
Cleanup / rollback / kill rules: <literal commands and stop conditions>
Result law: DETECTED->FORWARD_CONFIRMED; MISS->FORWARD_NULL; INVALID->FORWARD_INVALID
Non-credit: acceptance_credit=false; no holdout or unknown-bug credit
Decision delegation: forbidden; the named runner has no scope discretion
Expansion or retry: requires a new Authority ID, nonce, attestation, and operator decision
```

The named runner may perform only the literal approved commands; it cannot
delegate decision authority, alter scope, choose new revisions, or retry under
the consumed authorization.

For `DEFER` or `DECLINE`, the operator identity, attestation, timestamp,
packet identity, and reason are sufficient; all execution fields remain
unfilled and grant no authority. Until an explicit valid decision exists, this
non-terminal checkpoint stands: `R4_NOT_AUTHORIZATION_READY_TARGET_OPERATION_AND_DOSSIER_INCOMPLETE`,
`candidate_state=UNRUN`, `bridge_execution=null`, `acceptance_credit=false`.
