# First Dharma promotion campaign

**Issue:** [#101 — Map the first Dharma promotion campaign](https://github.com/AmitabhainArunachala/vibe-halt/issues/101)

**Accepted Vibe Halt base:** `d19ba9e1198c0ab7ef4bb1bdf69a299d056f9754`

**Dharma research snapshot:** canonical local `origin/main` at `abc8bd35c34c729ed421f9615df504f5615868bd`

**Research date:** 2026-08-22

**Scope:** source archaeology and proposed campaign contract only; the foreign target was not executed or changed

The Dharma remote was inspected before any repository operation and is
`https://github.com/AIKAGRYA/dharma_swarm.git`. No fetch was performed, so the
Dharma SHA above is the canonical **locally observed** remote-tracking snapshot,
not a claim that GitHub had no later commit. The working checkout was also dirty
and behind that ref; no conclusion below treats its ambient files as an accepted
target. The first real campaign must replace the snapshot with one human-frozen
canonical SHA and materialize it cleanly.

## Verdict

Bind the first campaign to
`DarwinEngine.apply_sealed_packet(...)`, not to free-running
`auto_evolve(...)` and not to the current Vibe observer. It is the smallest
production-shaped CPython seam that already traverses a supplied proposal,
Dharma gates, a fresh proof command, disposable-workspace mutation, testing,
evaluation, and archival. The campaign must stop one authority step earlier
than Dharma does today: a valid run emits an action-typed
`CampaignAssessment<HumanReview>`. A later positive output may be a signed
`PromotionPermit<CandidateForHumanReview>` for one exact base, patch, candidate
tree, immutable materialization receipt, policy, and campaign, but only after an admissible
`PROCEED` under a sufficiently controlled envelope. It must never apply to the
real checkout, commit, open a PR, merge, or deploy.

The decisive current defect is narrower than “Dharma has no gate.” It has a
real fail-closed, signed, path-scoped promotion door. Its own module says the
packet is scoped to a “specific mutation,” but the checked payload contains an
authorized **path list**, not base revision, diff digest, candidate tree, or
workspace identity ([promotion gate, lines 1–11 and 47–122](https://github.com/AIKAGRYA/dharma_swarm/blob/abc8bd35c34c729ed421f9615df504f5615868bd/dharma_swarm/promotion_gate.py#L1-L122)). A valid packet for bytes A can therefore
be presented with different bytes B at the same path unless another boundary
rejects the rebind. The first campaign should make that substitution its named
negative treatment and make the extracted `PromotionPermit` the first residue
sent toward a proof court.

The first campaign must close two independent obligations: the real shaker must
find, replay, and shrink one named behavioral Dharma mutant; and the admission
boundary must distinguish the exact authorized candidate from every
predeclared rebind. The exact behavioral mutant, property, and fault palette
remain for the human campaign contract. Wrapper-level diff checking alone is
not proof that Vibe Halt functioned as an Antithesis-adjacent shaker.

An honest successful D2 campaign may end `HALT` or `UNKNOWN`. Success means both
obligations were attempted, identities stayed exact, and evidence is signed,
content-addressed, and independently replayable to its claimed ceiling. It does
not mean Dharma's candidate is good, and it does not grant review or merge.

## Status legend

- **Current code** means directly observed in the two pinned source snapshots.
- **Historical evidence** means a committed prior incident or quarantine, not a
  claim about current behavior.
- **Proposal** means a required campaign rule or adversarial probe. No proposed
  rule was implemented or tested during this research.

## Current code — the exact mutation path

There are two live-shaped mutation entrances in accepted Dharma:

1. `auto_evolve(..., shadow=False)` verifies a caller-supplied promotion packet
   and caller-supplied trusted judge keys **before** dispatching LLM calls to
   generate proposals, then sends generated diffs through the sandbox cycle
   ([`evolution.py`, lines 3300–3453](https://github.com/AIKAGRYA/dharma_swarm/blob/abc8bd35c34c729ed421f9615df504f5615868bd/dharma_swarm/evolution.py#L3300-L3453)). This is too large and too raceable for the first admission proof: model generation happens after authorization, so the authorized bytes do not yet exist.
2. `apply_sealed_packet(..., shadow=False)` consumes a pre-existing build,
   review, proof, and diff packet. It still has binding gaps, but its candidate
   can be frozen before admission and no provider call is required
   ([`sealed_packet_apply.py`, lines 28–179](https://github.com/AIKAGRYA/dharma_swarm/blob/abc8bd35c34c729ed421f9615df504f5615868bd/dharma_swarm/sealed_packet_apply.py#L28-L179)). This is the first-campaign seam.

The current sealed path is:

```text
caller
  │
  ├─ promotion_verification + trusted_judge_public_keys
  ├─ packet root: build_packet / review_packet / proof_packet / diff
  ├─ workspace (separate caller argument)
  └─ proof command (indirectly supplied by packet metadata)
        │
        ▼
apply_sealed_packet
  verify promotion signature + path authority
  check kill switch and packet shape
  read and guard diff; compare changed paths to authorized_source_files
  Darwin gate_check(proposal)
  run proof command once against the unpatched caller workspace
  apply_diff_and_test(proposal, same proof command, caller workspace)
        │
        ├─ DiffApplier backs up and writes target paths
        ├─ shell test command runs in the mutated workspace
        └─ rollback on failure; retain bytes on pass
  evaluate → archive_result
        │
        └─ current terminal: applied/archive record

separate optional path
  commit_if_worthy → git add declared paths → git commit
```

The call/effect map is exact at the pinned snapshot:

| Stage | Current authority/data input | Current external effect | Current boundary |
|---|---|---|---|
| Promotion door | Packet decision, live flag, operator-lease-present bit, no blockers, Forge statistical packet, governed admission, telos decision, seven required receipt statuses, source-path list, final judge signature, caller-provided trusted judge keys | None by itself | Fails closed on missing fields/signature, but checks path scope rather than exact candidate identity ([`promotion_gate.py`, lines 47–122](https://github.com/AIKAGRYA/dharma_swarm/blob/abc8bd35c34c729ed421f9615df504f5615868bd/dharma_swarm/promotion_gate.py#L47-L122)) |
| Packet intake | Caller packet root, caller workspace, packet JSON, proof seal, gate results, allowed paths, proof command, diff reference | Reads packet files and candidate diff | Promotion check runs before diff load; changed paths are compared to authorized paths only after parsing ([`sealed_packet_apply.py`, lines 43–150](https://github.com/AIKAGRYA/dharma_swarm/blob/abc8bd35c34c729ed421f9615df504f5615868bd/dharma_swarm/sealed_packet_apply.py#L43-L150)) |
| Dharma proposal gate | Description plus diff and self-modification category | Telos/gate evaluation and trace/archive state | A self-modification must receive `ALLOW`; `BLOCK` or `REVIEW` rejects it ([`evolution.py`, lines 1479–1634](https://github.com/AIKAGRYA/dharma_swarm/blob/abc8bd35c34c729ed421f9615df504f5615868bd/dharma_swarm/evolution.py#L1479-L1634)) |
| Fresh proof | Packet-derived shell command and caller workspace | Spawns a shell subprocess before patch application | Nonzero exit refuses, but the command and its target-side oracle live in the candidate's ambient workspace ([`sealed_packet_apply.py`, lines 192–226](https://github.com/AIKAGRYA/dharma_swarm/blob/abc8bd35c34c729ed421f9615df504f5615868bd/dharma_swarm/sealed_packet_apply.py#L192-L226)) |
| Apply and test | In-memory diff, proof command, caller workspace | Writes files, creates backups, starts a shell test subprocess; keeps patch on exit 0 and rolls back on failure | Empty/no parsed patch currently returns a passing apply result; nonempty patches use only the low-level protected-live-root guard by default ([`diff_applier.py`, lines 221–347 and 366–458](https://github.com/AIKAGRYA/dharma_swarm/blob/abc8bd35c34c729ed421f9615df504f5615868bd/dharma_swarm/diff_applier.py#L221-L458)) |
| Evaluate/archive | Test result, gate result, evidence tier, observer block | Writes experiment/archive records; a passing candidate can remain in the disposable workspace | Current derived states distinguish local/component/system pass but do not construct a review permit ([`execution_profile.py`, lines 13–60](https://github.com/AIKAGRYA/dharma_swarm/blob/abc8bd35c34c729ed421f9615df504f5615868bd/dharma_swarm/execution_profile.py#L13-L60), [`evolution.py`, lines 1850–2057](https://github.com/AIKAGRYA/dharma_swarm/blob/abc8bd35c34c729ed421f9615df504f5615868bd/dharma_swarm/evolution.py#L1850-L2057)) |
| Optional commit | A second promotion packet and second caller-provided trust-key set, fitness threshold, proposal component, caller workspace | `git add -- <diff paths>`, then `git commit` | It stages the current bytes at each path and has no explicit clean-base/candidate-tree equality check ([`evolution.py`, lines 3475–3568](https://github.com/AIKAGRYA/dharma_swarm/blob/abc8bd35c34c729ed421f9615df504f5615868bd/dharma_swarm/evolution.py#L3475-L3568)) |

The current daemon can reconstruct archived proposals and call the commit path;
there is no typed “eligible for independent human review” object and no PR or
human-merge boundary in this mutation chain
([`evolution.py`, lines 3572–3705](https://github.com/AIKAGRYA/dharma_swarm/blob/abc8bd35c34c729ed421f9615df504f5615868bd/dharma_swarm/evolution.py#L3572-L3705)). The first campaign must stop before that optional commit path.

## Current code — authority is plural

A signature is only one authority input. The current Forge verifier composes:

1. a statistical `promotable_candidate` predicate;
2. exactly named signed receipts for preregistration, sequential alpha,
   scaffold parity, independent judge, run-level critic, mutation sandbox, and
   budget matching;
3. trusted receipt public keys;
4. a governed-work admission decision;
5. a formal telos decision;
6. an operator lease plus an injected lease verifier;
7. mutation budget and mission-contract presence;
8. `signal.source_files`, which becomes the authorization path list;
9. a final promotion-verification signing key; and
10. the final judge public keys accepted by the mutating caller.

The seven receipt names and statistical conjunction are defined in
[`promote.py`, lines 19–29 and 132–237](https://github.com/AIKAGRYA/dharma_swarm/blob/abc8bd35c34c729ed421f9615df504f5615868bd/dharma_swarm/forge_v1/forge_v2/promote.py#L19-L237). `verify_promotion` verifies receipt authorship and explicit pass payloads, invokes governed admission/telos/lease checking, and copies source paths into `authorized_source_files`
([`verify_promotion.py`, lines 78–142 and 288–421](https://github.com/AIKAGRYA/dharma_swarm/blob/abc8bd35c34c729ed421f9615df504f5615868bd/dharma_swarm/forge_v1/forge_v2/verify_promotion.py#L78-L421)). The final packet can be Ed25519-signed and checked against a supplied trust set
([lines 200–262](https://github.com/AIKAGRYA/dharma_swarm/blob/abc8bd35c34c729ed421f9615df504f5615868bd/dharma_swarm/forge_v1/forge_v2/verify_promotion.py#L200-L262)).

The constitutional concern is not that these inputs exist. It is that the
mutation API receives the packet and its accepted trust roots in the same call.
If treatment controls that caller, a mathematically valid self-selected key is
still self-granting authority. In the proposed campaign, trust roots, policy,
judge executable, holdout commitment, and oracle digests are preconfigured in a
judge-owned read-only plane; treatment may supply none of them.

## Current code — what is already fail-closed

The source contains useful negative controls. They were inspected, not run in
this research:

- Empty signals, missing receipts, untrusted self-signed receipts, a default
  governed-admission `REVIEW`, a red holdout despite green local pytest, a
  signed payload without explicit pass, and an unverified bare lease all refuse
  ([promotion-door tests, lines 129–324](https://github.com/AIKAGRYA/dharma_swarm/blob/abc8bd35c34c729ed421f9615df504f5615868bd/tests/test_forge_v2_promotion_door.py#L129-L324)).
- The packet must name an authorization scope; a different source path refuses,
  exact receipt names are required, and an advisory review cannot substitute
  for the deterministic packet guard
  ([promotion-door tests, lines 327–400](https://github.com/AIKAGRYA/dharma_swarm/blob/abc8bd35c34c729ed421f9615df504f5615868bd/tests/test_forge_v2_promotion_door.py#L327-L400)).
- The sealed path refuses a kill switch, missing packet files, an unsealed or
  failing proof, absent live-promotion verification, and a named blocked judge
  path; its positive test demonstrates that a packet scoped to
  `dharma_swarm/safe_leaf.py` reaches the apply call in a supplied workspace
  ([sealed-packet tests, lines 2097–2296](https://github.com/AIKAGRYA/dharma_swarm/blob/abc8bd35c34c729ed421f9615df504f5615868bd/tests/test_evolution.py#L2097-L2296)).
- The blocked-path set excludes the mutation engine, promotion gate, Forge
  verifier, governed admission, telos, governance scripts, and other central
  judge surfaces from **sealed-packet** live writes
  ([`promotion_gate.py`, lines 18–44](https://github.com/AIKAGRYA/dharma_swarm/blob/abc8bd35c34c729ed421f9615df504f5615868bd/dharma_swarm/promotion_gate.py#L18-L44)).
- Scratch classification requires a marker under an approved worktree root and
  protected live roots are denied, but the marker's `git_base_sha` is parsed,
  not compared there with the actual current `HEAD` or tree
  ([`evolution_safety.py`, lines 133–202](https://github.com/AIKAGRYA/dharma_swarm/blob/abc8bd35c34c729ed421f9615df504f5615868bd/dharma_swarm/evolution_safety.py#L133-L202)).

These controls should be retained as calibration. They do not cover same-path
byte substitution, clean-base binding, workspace identity, immutable oracle
custody, or a candidate-for-review permit.

## Historical evidence — why the writer cannot become the judge

Vibe Halt already has a local historical counterexample. Commit
[`cba8020`](https://github.com/AmitabhainArunachala/vibe-halt/commit/cba802014be19d55bd1691f9382040b44476e5dd)
quarantined a Python client whose runner ignored the supplied target and
fabricated clean-looking universe results, default properties, and a perfect
reproducibility score. Current Python result objects now prevent public callers
from directly minting checked outcomes, while explicitly stating that this is
not an in-process security boundary and that Rust remains the authority
([`result.py`, lines 84–132 and 184–190](https://github.com/AmitabhainArunachala/vibe-halt/blob/d19ba9e1198c0ab7ef4bb1bdf69a299d056f9754/clients/python/vibe_halt/core/result.py#L84-L190)).

This incident is historical evidence for the campaign rule: repair output is a
new hostile candidate at `MAP`; its provenance earns no evidentiary shortcut.
The repairer may not choose, edit, or impersonate its judge.

## Current Vibe boundary — useful, but not yet admission evidence

At accepted Vibe Halt:

- Product Lock requires an exact target revision, a whole-target map, a
  disposable harness, replayable evidence, and a bounded
  `HALT | PROCEED | UNKNOWN`; unsupported, incomplete, stale, untrusted,
  divergent, or nonreplayable evidence is `UNKNOWN`, which outranks `PROCEED`
  ([Product Lock v1, lines 13–49 and 68–85](https://github.com/AmitabhainArunachala/vibe-halt/blob/d19ba9e1198c0ab7ef4bb1bdf69a299d056f9754/docs/specs/PRODUCT_LOCK_V1.md#L13-L85)).
- Product Lock names Dharma as the first proving ground, forbids auto-merge,
  auto-deploy, and self-approval, and requires any future patch generator to be
  separate from verification and unable to certify its own patch
  ([lines 31–49, 110–145](https://github.com/AmitabhainArunachala/vibe-halt/blob/d19ba9e1198c0ab7ef4bb1bdf69a299d056f9754/docs/specs/PRODUCT_LOCK_V1.md#L31-L145)).
- Authority and epistemic modality are orthogonal; changing authority preserves
  modality, and an adjacent promotion requires the exact witness and revision
  ([`modality.rs`, lines 1–56 and 92–138](https://github.com/AmitabhainArunachala/vibe-halt/blob/d19ba9e1198c0ab7ef4bb1bdf69a299d056f9754/crates/vh-cli/src/modality.rs#L1-L138)).
- Current subprocess capability evidence is D2: all 29 channels can be reported
  open, while D1 is constructible only when every channel has controller
  evidence; the public constructor currently produces only `all_open`
  ([`capability.rs`, lines 139–250](https://github.com/AmitabhainArunachala/vibe-halt/blob/d19ba9e1198c0ab7ef4bb1bdf69a299d056f9754/crates/vh-sandbox/src/capability.rs#L139-L250)).
- Vibe's current strict bundles are content-addressed with SHA-256 but explicitly
  **unsigned** and not authenticated provenance; the legacy trace chain still
  uses non-cryptographic FNV
  ([`receipts_v2.rs`, lines 1–31](https://github.com/AmitabhainArunachala/vibe-halt/blob/d19ba9e1198c0ab7ef4bb1bdf69a299d056f9754/crates/vh-cli/src/receipts_v2.rs#L1-L31), [README, lines 98–113](https://github.com/AmitabhainArunachala/vibe-halt/blob/d19ba9e1198c0ab7ef4bb1bdf69a299d056f9754/README.md#L98-L113)).
- The current Dharma-side Vibe observer is explicitly calibration-only and not
  diff-bound; `OBSERVE` mode blocks findings but lets error, unchecked, and
  missing-observer states continue, while `REQUIRE` blocks every non-clean state
  ([`vibe_halt_observer.py`, lines 1–5, 88–127, and 329–481](https://github.com/AIKAGRYA/dharma_swarm/blob/abc8bd35c34c729ed421f9615df504f5615868bd/dharma_swarm/vibe_halt_observer.py#L1-L481)). Its tests confirm that proposal context is not delivered to the child and that a stale receipt does not govern the current proposal
  ([observer tests, lines 767–1038](https://github.com/AIKAGRYA/dharma_swarm/blob/abc8bd35c34c729ed421f9615df504f5615868bd/tests/test_vibe_halt_observer.py#L767-L1038)).

Therefore the first campaign begins in the honest `CooperativeD2` envelope and
may emit a replayed concrete `HALT` or a bounded `UNKNOWN`. It must not call the
current observer's `CLEAN` calibration receipt a diff-level `PROCEED`. Signed
admission evidence is a proposed prerequisite, not a capability present in
accepted Vibe Halt.

## Proposed red-team model

The following are source-derived attack hypotheses, not executed exploits. Each
is a mandatory negative arm because it changes a consequence while preserving
something the current gate accepts.

| Attack | Source-derived seam | Exact campaign mutation | Required result |
|---|---|---|---|
| Same-path replay/rebind | Current packet integrity protects its own fields and authorized paths, but contains no base SHA, canonical diff digest, or candidate tree ([verification fields, lines 27–46](https://github.com/AIKAGRYA/dharma_swarm/blob/abc8bd35c34c729ed421f9615df504f5615868bd/dharma_swarm/forge_v1/forge_v2/verify_promotion.py#L27-L46)) | Obtain a permit for patch A; preserve packet and path, substitute patch B | `Rejected(IntegrityViolation::BindingMismatch)`; if B reaches mutation, kill admission |
| Stale replay | Packet has no campaign nonce/epoch/expiry or consumed-for-consequence marker; receipt epoch binding does not bind candidate bytes ([receipt signature body, lines 149–197](https://github.com/AIKAGRYA/dharma_swarm/blob/abc8bd35c34c729ed421f9615df504f5615868bd/dharma_swarm/forge_v1/forge_v2/verify_promotion.py#L149-L197)) | Reuse an old otherwise valid permit against a later base or new campaign | `Rejected(IntegrityViolation::StaleSubject)`; identical evidence replay for verification remains lawful, consequence rebind does not |
| Workspace swap | `workspace` is a separate caller argument and is not in the promotion payload ([sealed call signature, lines 28–39](https://github.com/AIKAGRYA/dharma_swarm/blob/abc8bd35c34c729ed421f9615df504f5615868bd/dharma_swarm/sealed_packet_apply.py#L28-L39)) | Verify materialization W1, then invoke the same attempt against unverified W2 with the same relative paths | `Rejected(IntegrityViolation::WorkspaceMismatch)` before write; an independently rematerialized byte-identical W2 is a new signed observation, not a workspace rebind |
| Dirty-base/ambient-byte smuggling | Scratch marker parsing does not verify actual tree equality, and optional commit stages current path bytes rather than a candidate-tree digest ([`evolution_safety.py`, lines 133–202](https://github.com/AIKAGRYA/dharma_swarm/blob/abc8bd35c34c729ed421f9615df504f5615868bd/dharma_swarm/evolution_safety.py#L133-L202), [`evolution.py`, lines 3475–3568](https://github.com/AIKAGRYA/dharma_swarm/blob/abc8bd35c34c729ed421f9615df504f5615868bd/dharma_swarm/evolution.py#L3475-L3568)) | Add an unrelated tracked or untracked change before materialization or between test and review | observed mismatch is `Rejected(IntegrityViolation::SubjectChanged)`; an unobservable mandatory tree identity yields `UNKNOWN`, never `PROCEED` |
| Authorization-to-use race | `auto_evolve` verifies before generating bytes; the sealed path verifies before reading the diff and runs a pre-patch proof before later mutation ([`auto_evolve`, lines 3348–3453](https://github.com/AIKAGRYA/dharma_swarm/blob/abc8bd35c34c729ed421f9615df504f5615868bd/dharma_swarm/evolution.py#L3348-L3453), [`sealed_packet_apply.py`, lines 43–50 and 192–242](https://github.com/AIKAGRYA/dharma_swarm/blob/abc8bd35c34c729ed421f9615df504f5615868bd/dharma_swarm/sealed_packet_apply.py#L43-L242)) | Change workspace/base/oracle after authorization or after pre-proof but before candidate freeze | observed change is `Rejected(IntegrityViolation::SnapshotChanged)`; an unobservable race surface yields `UNKNOWN` |
| Oracle edit | The sealed route blocks central judge paths, but proof/test commands execute inside the mutable target workspace; generic auto-evolve does not apply the sealed-path blocklist ([`promotion_gate.py`, lines 18–44](https://github.com/AIKAGRYA/dharma_swarm/blob/abc8bd35c34c729ed421f9615df504f5615868bd/dharma_swarm/promotion_gate.py#L18-L44), [`sealed_packet_apply.py`, lines 192–226](https://github.com/AIKAGRYA/dharma_swarm/blob/abc8bd35c34c729ed421f9615df504f5615868bd/dharma_swarm/sealed_packet_apply.py#L192-L226), generic [`auto_evolve`, lines 3348–3453](https://github.com/AIKAGRYA/dharma_swarm/blob/abc8bd35c34c729ed421f9615df504f5615868bd/dharma_swarm/evolution.py#L3348-L3453)) | Candidate alters a target-side test/helper/config that makes its own test pass, or attempts a blocked judge path | judge-owned boundary compromise is `Rejected(IntegrityViolation::OracleBoundary)`; a valid exact candidate changing a governed target-side oracle is `HALT(oracle_surface_changed)` |
| Trust-root injection | Mutating entry points accept `trusted_judge_public_keys` from their caller ([`evolution.py`, lines 3300–3313 and 3475–3499](https://github.com/AIKAGRYA/dharma_swarm/blob/abc8bd35c34c729ed421f9615df504f5615868bd/dharma_swarm/evolution.py#L3300-L3499)) | Treatment signs with a new key and supplies that key as trusted | `Rejected(IntegrityViolation::UntrustedAuthority)`; trust-set changes are external, versioned claims |
| Holdout overfit/leak | Current Vibe design requires an independently curated candidate-secret holdout; current observer runs a visible fixed workload ([VISION, lines 169–188](https://github.com/AmitabhainArunachala/vibe-halt/blob/d19ba9e1198c0ab7ef4bb1bdf69a299d056f9754/VISION.md#L169-L188)) | Reveal a holdout seed/oracle/fault class to treatment before candidate freeze, or patch only the minimal replay while the hidden class still fails | leaked cohort invalidates acceptance and becomes calibration-only; valid hidden failure is `HALT`; unavailable uncompromised holdout evidence is `UNKNOWN` |

An expected integrity rejection is evidence about the campaign boundary, not a
positive target verdict:

```text
NegativeArmOutcome =
    Rejected(IntegrityViolation)
  | UnexpectedlyAccepted(FalsifiesAdmission)
```

The rejected subattempt remains in the denominator and evidence graph but never
inhabits `Assessment<Action>` or `PromotionPermit`.

## Proposed clean-revision materialization

The campaign may mutate only a disposable candidate worktree. A marker file is
not identity. Before any candidate is admitted, the materializer must:

1. freeze the canonical repository identity and one full 40-character base
   commit; record the remote URL and fetch provenance separately;
2. create a fresh detached worktree or immutable copy from that commit, never
   reuse the current dirty checkout;
3. require `HEAD == base_commit`, `tree == base_tree`, zero tracked diff, zero
   staged diff, and zero untracked files before patching;
4. canonicalize the nonempty unified diff, hash it with SHA-256, parse and sort
   its exact path set, and reject symlink/path escape before any test;
5. apply exactly once in the treatment-owned worktree and derive
   `candidate_tree` from the resulting bytes, not from the repair narrative;
6. recheck that only the authorized paths differ, then freeze the candidate
   snapshot read-only for admission;
7. bind interpreter, dependency lock, test command, environment allowlist,
   Vibe executable/source, policy, oracle, fault palette, budget, and channel
   ledger digests;
8. have the judge consume the frozen snapshot without write authority; and
9. independently rematerialize base plus patch in a third clean workspace and
   require the same candidate tree and replay result.

Any observed identity mismatch before candidate freeze is
`Rejected(IntegrityViolation)` and receives no target assessment; any mandatory
identity that cannot be observed is `UNKNOWN` on a valid exact subject. No
clean-materialized result may be generalized to the ambient developer checkout.

The signed `MaterializationReceiptId` binds the canonical base, canonical diff,
candidate tree, path set, clean-state checks, materializer role, and immutable
workspace-snapshot digest used by that execution attempt. A filesystem pathname
is never authority. A byte-identical independent rematerialization carries its
own receipt and observation closure; it corroborates the subject but does not
retroactively authorize swapping W1 for W2 inside an already frozen attempt.

## Proposed smallest executable campaign contract

“Executable” here means every field, role, arm, transition, and terminal mapping
is fixed sufficiently for a later runner to implement without deciding policy
after seeing results. It does not claim the runner exists now.

### 1. Subject and operation

```text
campaign_schema       = vibe-halt.dharma-promotion.v1
repository            = https://github.com/AIKAGRYA/dharma_swarm.git
base_commit            = <human-frozen full canonical SHA>
entrypoint              = DarwinEngine.apply_sealed_packet
language/runtime        = CPython, exact interpreter + dependency closure bound
execution_envelope      = CooperativeD2
operation               = CandidateForHumanReview
real_checkout_writes    = forbidden
commit/pr/merge/deploy   = forbidden
initial_artifact         = CampaignAssessment<HumanReview>
later_positive_artifact = PromotionPermit<CandidateForHumanReview>
```

The exact candidate patch and exact proof command must be supplied and hashed in
the preregistration manifest. The campaign must use a **nonempty** patch; the
current empty-diff success behavior is itself a negative control, never a green
candidate.

The manifest must separately bind a containment profile: zero target
credentials, default-deny network/data access, host or VM isolation, resource
limits, permitted write paths, cleanup, and the human authorization for this
exact foreign-target execution. `CooperativeD2` is an evidence envelope, not a
security boundary; a disposable worktree provides recoverability, not
containment.

### 2. Roles and write domains

| Role | May read | May write | May never supply or change |
|---|---|---|---|
| Treatment | Base snapshot, public contract, original minimized replay, public calibration seeds | Its one disposable candidate worktree and candidate proposal | Judge code/keys, policy, property oracle, evidence format/store, Vibe kernel, hidden seeds/fault classes, real repo |
| Materializer | Frozen repository object and candidate diff | Fresh base/candidate snapshots and identity receipt | Verdict, trust roots, oracle semantics |
| Holdout curator | Preregistered fault-class contract and candidate identity after freeze | Opaque pre-freeze commitment; authenticated post-freeze reveal to the authorized execution/admission boundary | Candidate patch or treatment workspace |
| Runner | Frozen campaign, baseline mutant, candidate snapshots, public and revealed run inputs | Raw observations plus `ObservationAttestation<Runner>` over `ObservationClosureId<Runner>`; never a verdict | Candidate code, policy, trust roots, assessment |
| Independent replayer | Runner closure plus independently fetched frozen inputs | `ObservationAttestation<Replayer>` over its own replay/rematerialization closure | Candidate, assessment, trust policy |
| Admission judge/quorum | Read-only frozen scope, completed `EvidenceClosureId`, channel ledger, governability projection/gate decision, and holdout record | Each judge signs identical `AdmissionPayloadId<HumanReview>` bytes containing `Assessment<HumanReview>` plus the pure governability payload/decision; the ordered signatures are assembled afterward into `AdmissionRecordId` | Candidate code, production tree, observation bytes, merge/deploy state |
| Human reviewer | Candidate permit and replayable evidence | External review/merge decision record | Epistemic modality; a human decision cannot rewrite `UNKNOWN` as `PROVEN` |

The judge process, trust roots, policy, oracle, evidence code, and holdout store
must be mounted read-only or otherwise mechanically outside treatment's
capabilities. Logical instructions to “not edit tests” are insufficient.

### 3. Preregistered paired arms

Use two independently required, identity-coupled calibration families.

**Behavioral shaker family:** freeze one source-derived behavioral mutant on
the real CPython mutation-to-review path, a property/oracle that distinguishes
it, a mechanism-relevant fault palette, and exact `FaultClassId` and
`FindingId`. The mutant must be found through the real target, replayed by an
independent verifier, and shrunk without changing that identity. Its exact code
and threshold remain a human-ratified campaign choice.

**Admission-integrity family:** use one candidate pair that differs only in
exact mutation identity:

- **Control A:** exact human-supplied nonempty repair patch A for that same
  `FaultClassId` and `FindingId`, bound through a frozen `RepairClaimId` to the
  base, canonical path set, expected candidate tree, original replay, and hidden
  mechanism class. It is eligible for a bounded positive assessment only if the
  original replay and hidden class no longer falsify and every mandatory
  obligation completes. Autonomous repair is not required.
- **Treatment B:** patch B touches the same authorized path set and is submitted
  with A's otherwise valid legacy path-scoped promotion packet. B must differ in
  canonical diff digest and candidate tree. Expected boundary outcome:
  `Rejected(IntegrityViolation::BindingMismatch)` before mutation, with no
  target assessment.

Then run the remaining attack rows as fail-closed subcases: wrong base, dirty
base, workspace W2, stale campaign, trust-key swap, oracle edit, empty diff,
holdout leak, post-freeze race, signature/digest corruption, and replay
divergence. A single campaign need not demonstrate autonomous repair; it must
demonstrate that repair output re-enters as an untrusted exact candidate.

### 4. Procedure

1. Publish the typed manifest, public property contract, mandatory channel
   ledger, budgets, verdict mapping, role keys, and holdout commitment.
2. Freeze base and both patch digests before any judge result.
3. Materialize clean base independently for A and every negative subcase.
4. Run a **baseline MAP** over the exact base revision. Freeze the
   tenant-declared and platform-mandatory artifact, dependency, entrypoint,
   consequence, capability, and contract classes; record newly discovered
   classes; and ledger every discovery blind spot as bounded or unbounded. No
   actor certifies that it found “every consequential path.”
5. Exercise the existing sealed-packet call and the frozen behavioral mutant in
   a disposable worktree; record every authority input and external effect. The
   real shaker must discover the named behavioral violation under the frozen
   property/fault plan; a wrapper-generated binding error does not satisfy this
   obligation. The campaign wrapper, not Dharma's current packet, enforces exact
   candidate identity.
6. Independently replay and shrink the behavioral finding. Require A's shared
   `RepairClaimId` to bind that exact `FaultClassId`, `FindingId`, minimal
   replay, canonical diff, and expected candidate tree.
7. Materialize `base + A`, verify clean base/path/tree equality, emit the signed
   `MaterializationReceiptId` and `SubjectId`, and freeze the candidate snapshot
   read-only **before** using it as evidence. Any later byte change is a new
   untrusted candidate and a new attempt.
8. While holdout material remains hidden, run a **candidate MAP** over clones of
   that exact immutable subject. Recompute the artifact, dependency, entrypoint,
   consequence, capability, and contract maps; record deltas and every bounded
   or unbounded discovery blind spot. This is the refinery's mandatory re-entry
   at MAP, not a narrative update by treatment.
9. Replay the original minimized finding against a clone of that exact subject
   and verify post-run subject equality. This public replay is mandatory but
   cannot substitute for hidden-class validation.
10. Only now reveal holdout material to the authorized execution/admission
    boundary, never to treatment, and re-shake with fresh seeds, a widened fault
    palette, and at least one hidden
   fault class that treatment never observed.
11. The runner emits raw observations for each arm, includes every open channel
   and invalid/unknown subattempt, and signs only
   `ObservationClosureId<Runner>`.
12. The independent replayer rematerializes and replays from that closure and
    signs its own `ObservationClosureId<Replayer>`.
13. A content-addressed `EvidenceClosureId` binds both role-scoped closures,
    their attestations, the paired matrix, and attack subcases. Each member of
    the ratified judge quorum verifies that closure and signs identical,
    action-typed `AdmissionPayloadId<HumanReview>` bytes, including the pure
    governability projection and action-specific gate decision. Only afterward is an
    `AdmissionRecordId` assembled from that payload and the ordered judge
    attestations, then verified against the frozen quorum policy.
14. Destroy or retain the disposable worktree per policy; do not write the real
    tree. Deliver a candidate plus permit to a human review queue only if a later
    `Assessment<HumanReview>::PROCEED` under an eligible envelope constructs
    that separate permit.

### 5. Terminal mapping

```text
AttemptValidity =
    Invalid(
      exact identity/signature/authority mismatch
      | malformed or wrong-schema evidence
      | dirty/changed subject after freeze
      | judge/holdout/oracle write-boundary compromise
      | unauthorized effect escaped containment
    )
  | Valid

Assessment<HumanReview> for a Valid attempt =
    HALT(
      reproduced blocking behavioral finding
      | mandatory precondition failed on the exact valid subject
      | original replay or hidden fault-class failure
      | contained attempt to violate a prohibited target effect
    )
  | UNKNOWN(
      mandatory surface unsupported or unobserved
      | mandatory capability channel remains open
      | evidence unavailable despite valid identity
      | replay or rematerialization divergence
      | holdout unavailable without evidence of compromise
      | treatment exclusion cannot be established
    )
  | PROCEED(
      all mandatory checks completed,
      every evidence artifact verified,
      two clean materializations matched,
      behavioral and binding obligations passed,
      all paired negatives were rejected,
      hidden validation passed,
      every channel relevant to HumanReview was closed or mechanically
      demonstrated irrelevant under the ratified policy
    )
```

Only `Valid + Assessment<HumanReview>::PROCEED` plus a verified
`GovernabilityGateWitness<HumanReview, Scope, Policy>` may be consumed by the
separate permit constructor. `RequiredButUngoverned`, `HALT`, `UNKNOWN`, and
`Invalid` remain campaign records and never inhabit `PromotionPermit`.

Because accepted Vibe leaves all 29 subprocess channels open, the initial D2
campaign does not construct a review permit. It may issue a concrete replayed
`HALT` or bounded `UNKNOWN`. A later human-ratified narrower action could alter
that ceiling only by mechanically demonstrating independence from every open
channel; it may not relabel D2 as native interposition or D1.

### 6. Proposed `PromotionPermit` residue

The permit is a signed attestation about one candidate's review eligibility,
not a bearer capability and not a merge token:

```text
PromotionPermitV1 {
  schema,
  campaign_id,
  campaign_manifest_sha256,
  canonical_repository,
  base_commit,
  base_tree_sha256,
  patch_sha256,
  ordered_path_set_sha256,
  candidate_tree_sha256,
  materialization_receipt_sha256,
  capability: HumanReviewCandidate,
  execution_envelope,
  interpreter_and_dependency_closure_sha256,
  vibe_engine_and_toolchain_sha256,
  target_map_sha256,
  property_oracle_contract_sha256,
  fault_palette_and_budget_sha256,
  holdout_commitment_and_reveal_sha256,
  capability_channel_ledger_sha256,
  evidence_root_sha256,
  replay_recipe_sha256,
  valid_attempt_receipt_sha256,
  admission_payload_id,
  admission_record_id,
  governability_projection_payload_sha256,
  governability_gate_decision,
  governability_gate_witness_sha256,
  preserved_evidence_modality,
  preserved_authority_grade,
  issued_at,
  expires_at,
  admission_quorum_policy_id,
}
```

`CampaignAssessmentV1<Action>` is the separate object that carries
`Assessment<Action> = HALT | PROCEED | UNKNOWN`, reasons, and open obligations.
The permit contains the identities of the exact
`Assessment<HumanReview>::PROCEED` payload and its assembled admission record,
plus derived `GovernabilityGateWitness<HumanReview, Scope, Policy>` and
`AdmissionQuorumWitness<Policy>` values satisfying the ratified policy. The
gate witness must be derived from the completed record; the pure projection and
gate decision inside that record contain no future quorum signature.
An `UNKNOWN` or `HALT` assessment remains signed evidence but cannot be
serialized inside a permit-shaped object.

The only lawful positive rule is:

```text
FreshIndependentCampaignWitness<
  exact base,
  exact patch,
  exact candidate tree,
  exact materialization receipt,
  exact envelope,
  exact policy,
  exact campaign,
  GovernabilityGateWitness<HumanReview, Scope, Policy>,
  AdmissionQuorumWitness<Policy>
>
  -> PromotionPermit<CandidateForHumanReview>
```

No constructor exists from a repair narrative, local pytest pass, path-scoped
packet, operator override, model consensus, or signature alone. Replaying the
same permit to verify the same statement is allowed. Rebinding it to different
bytes, base, materialization receipt, policy, campaign, or action is not. The
permit is review eligibility for immutable candidate content; it is never
authorization to mutate the audited worktree or any other workspace.

## Judge and holdout boundary

The minimum independent boundary is structural:

```text
treatment namespace                 admission namespace
-------------------                 -------------------
candidate worktree  -- read-only -> frozen candidate snapshot
public calibration                   judge executable(s) + policy
original replay                      oracle/property contract
                                      trust roots + role signing keys
                                      hidden seed/fault store
                                      append-only evidence store
```

The holdout curator commits to the hidden seed domain, fault classes, oracle
cases, eligibility rules, and budgets before candidate freeze. Treatment sees
only the commitment. Reveal occurs to the authorized runner/judge boundary
after `candidate_tree` is fixed. Any leaked or prematurely revealed cohort
becomes calibration-only and the attempt is invalid; a new hidden cohort and
commitment are required.

Validation must include both the original minimized replay and fresh/widened
holdout universes. Passing the replay but failing its hidden mechanism class is
an overfit `HALT`, not partial proof. The independent replayer must reconstruct
the same candidate tree and verify the evidence closure and role attestations
without access to treatment's process or narrative.

## Campaign falsifiers

Any one of these invalidates admission, not merely the affected candidate:

1. A permit for A authorizes different bytes, base, path set, materialization
   receipt, campaign, policy, or action B, or is treated as permission to mutate
   any workspace.
2. A dirty, unknown, or changed base/candidate tree receives `PROCEED`.
3. Treatment can edit or select the judge, oracle, property contract, evidence
   format/store, Vibe kernel, holdout, or trusted key set and still receive
   `PROCEED`.
4. A treatment-supplied public key becomes trusted within the campaign call.
5. A missing observer, exception, unsupported mandatory surface, open mandatory
   channel, or replay divergence becomes `PROCEED`.
6. The original replay is made green while a hidden case in the same fault
   class fails, yet the candidate remains positive.
7. A leaked holdout retains credit rather than becoming calibration-only.
8. A malformed, unsigned, wrong-type, wrong-revision, stale, or digest-divergent
   evidence bundle is accepted.
9. A current calibration-only Vibe observer receipt is treated as diff-bound
   admission evidence.
10. An empty/no-op diff, or a diff whose declared and materialized path sets
    differ, obtains a permit.
11. Two independent clean materializations do not produce the same candidate
    tree and replay result, but an observed identity mismatch is not `Invalid`
    or an unavailable comparison is not `UNKNOWN`.
12. The campaign itself applies to the real checkout, commits, opens a PR,
    merges, deploys, or silently turns `UNKNOWN` into permission.

The **initial A1/D2 mechanism criterion** is the inverse: the baseline mutant is
found, replayed, and shrunk; A is bound to and revalidates that same finding; B
and every mandatory attack arm are rejected before consequence; the evidence
graph verifies and replays independently; and all open channels remain visible.
Its terminal artifact is a signed `CampaignAssessment<HumanReview>` that may be
`HALT` or `UNKNOWN`; it issues no permit.

Only a fresh later campaign under a human-ratified eligible envelope may add the
**permit criterion**: `Valid + Assessment<HumanReview>::PROCEED` plus the
ratified `AdmissionQuorumWitness<Policy>` constructs one exact
`CandidateForHumanReview` permit. Keeping those criteria separate is the first
proof that the refinery can touch a real CPython promotion path without
becoming its own judge or laundering D2 into permission.
