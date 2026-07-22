# VIBE-HALT — Post-Audit Truth Bridge, Tier-2 Reach, and Supervisor Spike

**Artifact type:** long-running autonomous `/goal` controller
**Authored:** 2026-07-22
**Repository:** `AmitabhainArunachala/vibe-halt`
**Live evidence anchor when authored:** `main` at `fb0ca58942d23569cef95ac233fc5b4971d2b24b`
**Required convergence baseline:** merged `main` containing PR #23 head `25d4c8d930c4db6a1b22852f66afca8666248c39`. Any later PR head is acceptable only after a full diff against `25d4c8d`, fresh exact-head CI, and independent human review confirming the same audit corrections with no unrelated change.
**Status:** admission-ready after PR #23 is human-merged; implementation-ready only after C0 is human-merged

## Operator use

For a true overnight implementation run, use two human keys before dispatch: (1) merge PR #23 at the reviewed audited head, then (2) merge the C0 docs/governance PR containing this controller, lifecycle transitions, exact ownership, and supersession banners. Only then dispatch the `/goal` below.

If this controller is dispatched after PR #23 but before C0, the only authorized write is the C0 admission PR. The executor must not begin implementation overnight; it may perform read-only reconciliation, threat modeling, and test design while awaiting the C0 merge.

This controller grants no merge or self-approval authority, no sibling-repository creation authority, no unsafe authority, and no spending authority. Human merge of C0 activates only the safe-phase ownership stated there. C7 and a separate explicit location/repository decision are required for helper work. Every SHA, PR state, line number, CI result, track status, and tool capability recorded here remains stale until independently refreshed.

---

# `/goal` VIBE-HALT — POST-AUDIT TRUTH BRIDGE AND TIER-2 REACH CAMPAIGN

You are the long-running critical-path controller for `AmitabhainArunachala/vibe-halt`. Continue the build from the independently audited post-convergence baseline. This is a multi-worktree, multi-PR implementation and verification campaign, not another architecture essay, confidence score, dashboard, or broad rewrite.

Your mission is to do three things in strict order:

1. close the load-bearing false-confidence paths found by the independent audit;
2. turn the Tier-2 cassette from a parent-side demo into a real child-visible, persistent, receipt-bound transport while honestly remaining D2 wherever channels are still open;
3. only after those foundations are merged, prepare and—only with separately human-ratified unsafe-boundary authority—run the capped Linux deterministic-process-supervisor spike recommended in audit F.14.

Keep working across bounded packages, CI/review cycles, human merge pauses, rebases, and resumptions until the campaign completion contract is true on current merged `origin/main`, the supervisor spike fires its stop rule, or exactly one irreducible decision-ready operator gate remains. A report is not an iteration. Every control-loop pass must end in one of:

- a gate-green bounded commit or draft PR;
- a repaired exact-head PR materially closer to merge;
- a falsification or proof artifact bound to an exact SHA and environment;
- safe work on an independent package while another package awaits a human;
- or one precise operator packet after every safe in-scope alternative is exhausted.

## 0. Truth sources and preflight

### 0.1 Refresh live state before trusting this controller

Resolve the default branch and fetch it. Record:

- exact `origin/main` SHA and tree;
- all open PRs, their base/head SHAs, draft state, mergeability, changed surfaces, CI jobs, reviews, and unresolved threads;
- all active branches/worktrees that can collide;
- current `docs/governance/ACTIVE_TRACK.yaml` status and ownership;
- available Rust/Python/Linux tooling and host capabilities;
- remaining budget if the operator has recorded one.

At authorship, main was `fb0ca589...`; PR #23 was an open draft at `25d4c8d...`, cleanly mergeable, with all six Actions jobs green but zero submitted reviews or review threads; PR #25 was an open, cleanly mergeable, docs-only charter mirror. Green CI and clean mergeability are evidence, not approval or review completion. Every value must be refreshed.

Run `make onboard` before any non-trivial read or edit, then `make gate`. Also establish the actual toolchain with `rustc --version`, `cargo --version`, `python3 --version`, `uname -a`, and `git status --short`. `make onboard READY` proves only this checkout's admission checks. It is not merge approval, CI proof, or acceptance evidence.

If Cargo or rustc is unavailable, do not repeat the audit runner's limitation and present checked-in receipts as fresh execution. Continue only read-only reconciliation and docs-safe admission work, then emit a host-capability gate with the exact missing tools.

### 0.2 Required baseline gate

Before any implementation package, prove that merged main contains the substance of PR #23 commit `25d4c8d`:

- the PCT result is narrowed to the measured event-priority/workload/metric null;
- the strategy is named event-priority (PCT-inspired) and disclaims thread-PCT guarantees;
- the bakeoff fails closed on child execution/verdict failures;
- VB-006 requires every expected base fact and `ok` value;
- the event-priority recall gate pins the exact count and first-failure index;
- the convergence ledger publishes guided exploration as unproven on the schedule axis.

If PR #23 is still open, do not recreate it, cherry-pick around it, or stack the new campaign on it. Report: `OPERATOR DECISION — review the current PR #23 head against 25d4c8d and issue #24's accepted corrections; if accepted, mark it ready and merge it. Do not merge on CI or mergeability alone.` Do not ask the operator to merge PR #25: leave it draft or close it unmerged. If the operator deliberately wants the charter archived in-repo, it may be rebased onto post-#23 main and rechecked later, but it is never a functional dependency.

The residual threads on already-merged PRs #17, #19, and #20 require new corrective PRs; they neither require nor permit re-merging those historical PRs and are not a prerequisite to the operator's PR #23 decision.

### 0.3 Read before writing

Read in full, from current main:

- `AGENTS.md` and `CLAUDE.md`;
- `docs/governance/ACTIVE_TRACK.yaml`;
- `docs/plans/VIBE_HALT_BUILD_PLAN_2026-07-20.md`;
- `docs/specs/DETERMINISM_TIERS.md` and `docs/specs/TRACE_FORMAT_V0.md`;
- `docs/prompts/TRACK1_NIGHT_GOAL_2026-07-20.md` as a historical one-night digest, not active authority;
- `docs/prompts/TRACK1_PHASE1_SANDBOX_GOAL_2026-07-21.md`;
- `docs/prompts/CONVERGENCE_CAMPAIGN_EXECUTOR_2026-07-22.md` as historical campaign shape, not active authority;
- `docs/audits/antithesis-dst-2026-07-21/{EXECUTIVE_VERDICT.md,FIRST_PRINCIPLES_AND_TARGET_ARCHITECTURE.md,INTEGRATION_ROADMAP.md,CONVERGENCE_LEDGER_2026-07-22.md,EVIDENCE_LEDGER.jsonl}`;
- issue #24 in full, especially audit report comment `5043766546` and builder disposition `5044459137`;
- current code and tests for every surface before editing it;
- the merged-PR review debt, refreshed rather than inferred:
  - PR #17 thread `PRRT_kwDOTdlCIM6Szodl` is current and unresolved; the checked-in 12-insertion receipt is mechanically false and routes to CD/G.6;
  - PR #19 thread `PRRT_kwDOTdlCIM6S0Hr9` is current and unresolved; dirty `--out` reuse can leave orphaned findings and routes to C3;
  - PR #20 threads `PRRT_kwDOTdlCIM6S0K19` and `PRRT_kwDOTdlCIM6S0K2B` remain unresolved markers. Final head `b4e0b9e...` added the exact-fingerprint digest and registered `scripts/gate.sh`, so verify those original comments before classifying them as administrative. The substantive remaining work is audit D.3: minimized-plan identity, persisted lineage, and a repro that actually consumes the minimized plan.

The issue #24 audit is the controlling finding set. Never reduce a finding to a paraphrased title when its cited mechanism is available.

### 0.4 Active authority and supersession

Until C0 is human-merged, this file is a proposal only. After C0 merges, this file is the sole active execution controller for the core post-audit campaign. The following remain historical evidence and reusable mechanics, but no longer grant core execution authority, ownership, branch rules, scope exclusions, or stale baselines: `docs/prompts/CONVERGENCE_CAMPAIGN_EXECUTOR_2026-07-22.md`, `docs/prompts/TRACK1_NIGHT_GOAL_2026-07-20.md`, `docs/prompts/TRACK1_PHASE1_SANDBOX_GOAL_2026-07-21.md`, `docs/plans/VIBE_HALT_PHASE1_NIGHT_PLAN_2026-07-20.md`, `docs/prompts/CODEX_NIGHT_ADDENDUM_2026-07-20.md`, and `docs/prompts/TRACK2_1000X_INSTANTIATION_2026-07-21.md`.

C0 adds a one-line `Superseded for core execution by docs/prompts/VIBE_HALT_POST_AUDIT_TIER2_REACH_LONG_RUNNING_GOAL_2026-07-22.md` banner to each of those files without rewriting its recorded history. `docs/prompts/CODEX_PR2_MASTER_SPEC_2026-07-20.md` remains the verifier track's standing contract and is **not** superseded; V1 coordinates through its interface-request law. `ACTIVE_TRACK.yaml`, the ratified build plan, determinism tiers, and trace-format law remain controlling. This controller overrides none of them except through an explicit human-merged C0/CD/C7 diff.

## 1. Claim boundary and strategic decision

The current project has strong Tier-1 kernel work and an honestly labeled Tier-2 D2 subprocess MVP. It does **not** yet have complete replay identity, mechanically pinned corpus recall, complete evidence bundles, a child-visible provider transport, an authenticated evidence provenance story, a D1 process boundary, or progress toward three previously unknown human-confirmed bugs.

The strategic sequence is fixed:

```text
audit blockers -> child-visible cassette-backed D2 -> measured target reach
               -> separately admitted Linux supervisor spike -> D1 or killed spike
```

Cassette fidelity and supervision are complements. A cassette closes provider/tool nondeterminism for a cooperative target. It does not close time, entropy, process identity, filesystem, network escape, scheduling, signals, descendants, vDSO, ASLR, JIT/GC, or unsupported syscalls. Env scrubbing plus a cassette therefore has a D2 ceiling.

The spike is **not** a VM, hypervisor, arbitrary-binary record/replay system, or multithreaded determinism promise. Its only initial target profile is Linux x86-64, one process, one thread, one single-threaded CPython/CLI fixture, with unsupported effects rejected. The preferred audited-unsafe boundary is a sibling repository/workspace. The only acceptable in-repository fallback is a separately built, explicitly excluded `tools/vh-supervisor-linux` workspace with its own ratified unsafe charter. It must never become a root Cargo workspace member, a hidden deny-list exemption, or `unsafe` inside `crates/**`.

Maintain a live F.13 success-criterion risk ledger. Seed it from the audit's evidence-based order and require new evidence for any reorder: criterion 4 unknown human-confirmed bugs = Extreme/BLOCKER; criterion 2 measured Tier-2 divergence = Extreme/BLOCKER; criterion 7 dharma adapter = High; criterion 5 replay/shrink = High; criterion 3 corpus recall = High-medium; criterion 1 1,000-run/two-machine identity = Medium; criterion 6 throughput = Low. This ordering controls effort; it is not a confidence score. A generic cassette fixture does not close criterion 7.

## 2. Standing law

1. **Human-only authority.** Never merge, approve, self-approve, dismiss a required review, push to `main`, or enable auto-merge. Every implementation PR begins draft. The human operator alone marks ready and merges.
2. **Citation-or-silence.** Every factual claim in specs, PR bodies, review replies, receipts, and conclusions has a current `file:line`, GitHub item, or runnable command plus observed output. Checked-in historical output is labeled historical.
3. **Tier and D-grade every cross-boundary claim.** `deterministic` without Tier and D-grade is invalid. Missing evidence, a cassette miss, an open channel, an unknown effect, incomplete tape consumption, or unsupported capability yields `UNCHECKED`/`DIVERGENT`, never `CLEAN`/`FINDINGS` by invention.
4. **Gates are law.** Run `make gate` before every commit. A red gate is a finding. Reproduce, repair, or stop; never weaken, skip, rename, widen a tolerance, or route around it to obtain green.
5. **Frozen identities move only by migration.** PRNG vectors and trace v0 do not silently change. Any public observation-schema change gets a versioned migration, changelog, negative regression, exact semantic cause, and verifier-track interface coordination. Re-derive every fingerprint; never copy another track's value.
6. **One owner per surface.** Reconcile `ACTIVE_TRACK.yaml` before editing. Shared surfaces are append-only. Use one clean worktree and one writer per package. Never edit verifier-owned `crates/vh-verify/**`, `crates/vh-shrink/**`, `.github/workflows/verify.yml`, or `AGENTS.md` from the core lane; send a precise `INTERFACE REQUEST` and require an independent verifier response where needed.
7. **No hidden unsafe or dependency escape.** Root workspace keeps `unsafe_code = forbid`. Kernel crates remain deny-list pure. No external dependency or scanner relaxation without an explicit same-PR determinism review and operator-ratified exception; the supervisor helper has its own boundary instead.
8. **Runtime receipts stay out of git.** Use explicit temporary/output directories, CI artifacts, and PR comments. Do not write implicitly to `~/.vibe-halt/`, commit raw runtime receipts, secrets, `.env`, credentials, or live provider data.
9. **Preserve history; correct false claims.** Do not rewrite inconvenient nulls or failures. Append dispositions, algorithm tags, and supersession notes. Fix mechanically false receipts rather than rationalizing them.
10. **No scope drift into guided exploration.** Palette guidance lost; schedule guidance remains unmeasured on a discriminating depth>=2 corpus. Do not revive PCT, RL, novelty dashboards, causality UI, or a new evaluator framework during this campaign.

## 3. C0 — admission, lifecycle, and ownership (first PR; human merge required)

Current governance may still show three ACTIVE tracks at `wip_max: 3`, Track 2 ACTIVE, and the broad core track PAUSED. Opening a fourth track is forbidden. The controller must reuse the existing portfolio.

Create one minimal docs/governance-only admission PR, branch `claude/post-audit-c0-admission`, containing:

- add this controller at its final `docs/prompts/` path and add the section 0.4 supersession banners;
- transition `vibe-halt-1000x-exploration` from `ACTIVE` to the validator-supported `SHIPPED` status, preserving its palette null, narrow schedule null, and depth>=2 revival falsifier in an append-only disposition. Here `SHIPPED` means the experiment package closed with a negative/unmeasurable result; it does not mean guided exploration was validated or a capability shipped;
- transition `vibe-halt-core-2026-07` from `PAUSED` to `ACTIVE`; retain its existing surfaces and explicitly add `DESIGN.md`, `docs/audits/**`, and `crates/vh-digest/**` for CD, campaign dispositions, and the C3 safe digest surface;
- narrow core `next:` to C1-C6/CD and state in `non_goals` that Tier-3/hypervisor work remains excluded; only the separately human-admitted C7 single-process falsifier may cross that boundary;
- leave verifier and corpus tracks `ACTIVE` and retain their exclusive ownership. Total ACTIVE tracks after the diff must be exactly three;
- register the package-to-track split: core owns C1/C2-core/C3/C4/C5/C6/CD; verifier owns V1; corpus owns K1 and any later corpus admissions;
- keep `scripts/gate.sh`, `Cargo.toml`, `Cargo.lock`, and `ACTIVE_TRACK.yaml` shared under their existing protocol; assign one integration writer before any package touches them;
- grant no sibling-repository, excluded-workspace, unsafe-helper, public-harness execution, credential, or spending authority.

Changing an existing track's `status`, `next`, `non_goals`, or owned-surface list is not an append-only shared-surface edit. Human merge of C0 explicitly ratifies only the lifecycle and ownership mutations enumerated above; no executor may infer a broader exception.

This docs/governance proposal is the sole pre-admission exception. It grants no code ownership until human merge. Run the governance checker/self-tests, `make onboard`, and `make gate`; publish the draft C0 PR and stop all code work until it is human-merged. After merge, fetch main and prove exactly three ACTIVE tracks with no ownership overlap before C1/CD/K1/V1 work.

If equivalent lifecycle/controller changes are already merged, cite them and skip C0. Never create a duplicate track or controller.

## 4. Package graph

After C0 merges, use the following dependency graph. Parallel work is allowed only in clean worktrees with disjoint writers and explicit PR bases.

```text
C0 admission
  +-- C1 observation identity --+
  +-- C4 sandbox truth envelope +--> C3 evidence v2 --> C5 cassette --+--> C6 D2 reach
  +-- C2-core oracle/gate <-----> K1 corpus contracts ----------------+
  +-- CDa static doctrine ----------> CDb observation docs            |
                                                                       +--> V1 verifier ratchet

C1 + C2-core + K1 + C3 + C4 + C5 + V1 + CDa/CDb --> C7 unsafe admission
C5 -----------------------------------------------> C6 and C7 may proceed in parallel
C7 human authority -------------------------------> S1-S7 conditional spike
```

C1, C2-core, C4, CDa, and K1 may proceed after C0 only when their exact file manifests are disjoint. C3 waits for the final C1 and C4 contracts. C5 waits for C3's v2 contract and C4's run-record/capability contract. CDb waits for C1. V1 waits for C1 and C3. C6 waits for C2-core, K1, C3, C5, and V1. C7 waits for the merged A.1/B.1/D.1-D.2/E.1-E.2 foundations, V1, C5's minimal CPython fixture, and CDa/CDb; it does **not** wait for a four-week C6 harvest. C6 and C7 may proceed in parallel after their own gates. A dependency PR may not masquerade as main-based. Any explicit stack names and pins its parent, retargets after merge, and reruns every gate.

### 4.1 Coordinator, workers, and surface locks

One coordinator owns live-state refresh, the package DAG, merge order, and a single top-level `POST-AUDIT CAMPAIGN STATE` comment on issue #24. Only the coordinator updates that state record. Each worker receives exactly one package, one clean worktree, one base SHA, one track, and an exact writable-file manifest; it returns commits and evidence to the coordinator and never expands its manifest.

Before parallel work, publish a file-lock table. `ACTIVE_TRACK.yaml`, `Cargo.toml`, `Cargo.lock`, `scripts/gate.sh`, and `crates/vh-cli/src/main.rs` have one named integration writer and are serialized. Apparent package independence does not permit two writers under the broad `crates/vh-cli/**` glob. If manifests overlap, serialize the packages.

## 5. C1 — complete observation and end-state identity

**Findings routed:** A.1 BLOCKER; A.2/A.5 GAP; prerequisites for D.1/D.2. A.4 is closed by V1, not by a core-lane edit to verifier-owned files.

Make the raw state consumed by end-state oracles part of replay identity. Re-read the actual types before selecting an encoding. C1 closes A.1 with versioned canonical bytes and full field equality, not a second digest implementation. Ordered maps, explicit length framing, and schema/domain tags are required; allocator addresses, locale, float display, panic text, or unordered iteration may never feed it. Do not use trace-v0 FNV as an adversarial content identity. C3 later derives the reviewed SHA-256 identity over these same canonical bytes for persisted evidence.

Required behavior:

- two executions with different oracle-consumed end state can never compare equal merely because both pass the same oracle;
- `UniverseResult` and its exhaustive observation view include an end-state identity and a versioned complete-observation identity;
- the ordered pass/fail transcript, sometimes map, lifecycle, plan, runtime evidence, schedule policy, tape digest, and future public observables are covered by the complete identity or compared field-for-field;
- algorithm and schema identifiers are explicit; trace FNV-1a-128 remains tagged legacy/internal and is not silently rebranded cryptographic;
- a compile-time/exhaustive ratchet fails when a public observable grows without acknowledgment;
- Tier-1 frozen trace values do not change unless a separately documented migration is truly required.

Adversarial tests must include: same trace + same passing oracle + different unused/raw end-state value => divergent observation; different transcript containing only a new passing oracle => divergent observation; reordered map construction => identical canonical state; malformed/duplicate canonical fields rejected.

Close the demonstrated deny-list class as well. Add adversarial scanner/self-test fixtures for safe-reference-to-raw-pointer-to-integer flows, raw-pointer Debug/`{:p}` formatting, and any equivalent address value reaching an observable. Prefer a simple fail-closed prohibition on raw-pointer/address operations in kernel crates over a fragile sink-specific regex. If the current scanner cannot express the semantic rule without broad false positives, document the remaining limitation and require a separately reviewed type-aware lint plan; do not restore the D0 `by construction` claim prematurely.

Correct the trace crate's hash-only identity prose to the complete-observation doctrine without changing trace-v0 bytes.

If verifier-owned projections break, do not edit them. Post an interface request with the exact new fields/schema and a minimal migration contract. The verifier must re-derive its own projection/fingerprint.

**Acceptance:** the constructive A.1 false negative is impossible and gate-protected; the address-derived observable class is rejected or the D0 claim is mechanically narrowed; hash-only identity prose is corrected; complete identity documentation matches code; full suite and exact-head CI green.
**Kill/stop:** unexplained frozen trace drift, an encoding dependent on Debug/host formatting, or an unverifiable migration stops publication.

## 5A. V1 — verifier-owned full-observation and cross-OS ratchet

**Finding routed:** A.4 GAP and the independent-verifier half of A.1/A.2.

The core lane does not edit verifier-owned surfaces. After C1 and C3's public contracts are stable, post an `INTERFACE REQUEST` that requires the verifier track to re-derive—not copy—the expanded observation and add a separate exact-head PR.

The independent response must:

- preserve the audit's accurate finding that the existing shared literal already proves equality among successful OS jobs; the strengthening target is 1,000-run full-schema SimRuntime scope and explicit artifact aggregation, not a fictitious absence of comparison;
- exercise a representative SimRuntime reference rather than only the shallow legacy verifier workload;
- run the ratified 1,000 Tier-1 replays where the success criterion requires 1,000, while keeping any smaller fast gate explicitly non-promotional;
- emit one normalized comparable full-observation artifact plus a separate provenance artifact per Ubuntu, macOS, and Windows job;
- compare the normalized observation artifacts in an aggregate job rather than relying only on three jobs matching a manually copied literal; OS/toolchain/world provenance is retained and expected to differ;
- cover end state, complete property transcript, lifecycle/runtime evidence, fault plan, schedule policy, and decision tape when present;
- independently check C3 SHA-256 known-answer vectors and canonical-byte behavior without copying the core implementation;
- independently test the address-leak negative class and every C1 schema migration;
- fail closed on missing artifacts, schema mismatch, or partial job success.

Same authorship is not independent verification. Require a separately identified verifier owner/reviewer or explicitly downgrade the authority claim.

**Acceptance:** the verifier projection breaks on unacknowledged observable growth; 1,000-run full-SimRuntime observation equality is mechanically byte-compared across the CI OS matrix; provenance stays explicit; schema/fingerprint/digest evidence is independently derived at the exact merged head.
**Kill/stop:** any cross-OS difference in the declared normalized semantic observation is a determinism finding; provenance differences are expected and retained. Do not normalize an observation difference away until its semantic cause is proven.

## 6. C2 — oracle fail-closed behavior and mechanically exact recall

**Findings routed:** B.1 BLOCKER; B.2/B.3 GAP; criterion-3 evidence integrity.

Reproduce every audit-listed missing-summary path before changing it. Audit all 11 entries at current main; do not assume the historical list is exhaustive. An oracle must distinguish malformed/missing state from a valid empty state and must check independent facts rather than a Boolean precomputed by the workload.

For every corpus entry, mechanically bind:

- workload and oracle schema/version;
- root seed or committed dispersed seed manifest;
- universe budget;
- palette/fault-plan generator version and digest;
- schedule policy and tape requirement;
- exact failing count, exact clean count, divergence count, and expected exit/verdict;
- at least one corrected/fault-free control that passes where the model permits;
- required-key/required-progress facts so silence cannot become success.

The gate must reject recall drift upward as well as downward. A move from 70/100 to 100/100 may mean a guaranteed-failure palette and is not automatically an improvement. If a correctness repair legitimately changes a count, measure before/after at the same exact head, explain the semantic cause, and re-pin in the same PR. Never tune a palette, seed, or budget merely to reach the target.

Preserve PR #23's exact VB-006 gate and narrow null. Do not build the depth>=2 revival instrument in this package; record it as the only schedule-guidance falsifier that could justify renewed investment.

`corpus/**` remains corpus-track owned. Split this work into C2-core (workloads, oracles, and the serialized shared-gate integration) and K1 (corpus schema, entries, and prose) whenever both surfaces change. Use separate writers and PRs. C2 is complete only when both exact-head PRs are human-merged and their combined current-main gate is green.

**Acceptance:** all published recall claims are exact executable assertions; missing facts fail closed; controls prevent guaranteed-failure gaming; current criterion-3 status is reported honestly even if below target.
**Kill/stop:** if an exact count is nondeterministic, the affected claim becomes UNCHECKED and the underlying identity defect is fixed before any tolerance is considered.

## 7. C3 — evidence bundle v2, strict verification, and shrink lineage

**Findings routed:** D.1 BLOCKER; D.2-D.6; A.3 trust boundary; PR #19 thread `PRRT_kwDOTdlCIM6S0Hr9`; and audit D.3's persisted-lineage debt remaining after PR #20. Do not reimplement PR #20's already-landed fingerprint digest or gate registration merely because historical thread markers remain.

Do not retrofit v1 additively. Keep v1 explicitly FIFO-only and label it self-consistent replay, not authenticated provenance. Introduce a strict v2 schema whose verification covers the complete declared observation.

Minimum v2 manifest/bundle fields:

- schema and canonical-encoding versions;
- workload name plus source/content digest;
- source commit/tree, build profile, toolchain, target triple, and relevant feature/config identity;
- root seed, universe, palette/generator, full fault plan, schedule policy, decision-tape schema/digest, and exact tape consumption;
- end-state and complete-observation identities;
- full ordered property transcript, sometimes map, runtime/lifecycle evidence, process outcome where applicable, and divergence evidence;
- original and minimized plan identities, exact failure fingerprint including details, minimizer version/budget, and replay lineage;
- cassette/helper/world/capability identities when present;
- algorithm-tagged cryptographic content digest over the canonical v2 bundle.

Use no ambiguous parser behavior: reject duplicate required fields, unknown critical fields, missing fields, trailing data, mismatched algorithm/schema tags, finding IDs that do not recompute, extra/missing tape effects, and unsupported v1/v2 combinations. `REPRODUCED` means the full v2 identity matched. Otherwise emit a typed `DIVERGED`, `UNCHECKED`, or usage/corruption outcome.

For PR #20's remaining debt, the existing `fingerprint-digest` is necessary but insufficient. Persist and verify the minimized-plan digest and canonical injections, bind them to the exact baseline and source identity, write or update the receipt after minimization, and emit a repro that consumes the minimized plan or its bundle rather than regenerating the original plan. Negative tests must cover a spliced baseline, changed failure detail, changed minimized plan, and an original-plan repro falsely presented as minimized evidence.

Do not change trace-v0's FNV format in this package. Add a small zero-dependency safe-Rust SHA-256 implementation only through a ratified, deny-list-pure digest surface, with standard known-answer vectors, boundary tests, and independent verifier comparison. Until that path is independently reviewed, say `cryptographically content-addressed with an unreviewed local implementation`, not `tamper-proof` or `authenticated provenance`. Signatures and external trust anchors are out of scope.

Fix dirty output fail-closed. For v0.1, `--out DIR` must refuse a non-empty caller-supplied directory before writing, without deleting, clearing, renaming, or replacing existing contents. A generated fresh child run directory is also acceptable. Add a negative test reproducing PR #19's exact stale-finding mechanism and proving existing user files remain untouched.

Fix Python evidence defaults: no publicly constructible evidence object may manufacture `1.0` reproducibility or `All properties held` without runner-owned evidence. Keep the execution quarantine closed.

**Acceptance:** changing any declared observation, schedule/tape, minimized plan, finding details/ID, world identity, or critical header makes replay fail closed; dirty-dir behavior is deterministic and has no orphans; v1 remains readable only within its explicit limitations; v2 bundles reproduce from copied artifacts under their stated support envelope.
**Kill/stop:** canonical encoding/digest must be stable for the same semantic bundle under the same declared provenance. Whole v2 bundle digests are expected to differ when OS/toolchain/world provenance differs. V1 compares the normalized complete-observation identity and declared comparable fields while retaining non-equal provenance separately. Never normalize away a real observation difference. If the same declared bundle cannot produce stable canonical bytes, v2 remains experimental and no cross-party/tamper claim ships.

## 8. C4 — truthful subprocess observation and capability envelope

**Findings routed:** E.1 BLOCKER; E.3-E.5; prerequisites for E.2 and F.14.

Refactor the sandbox result around a controller-produced, sealed capability receipt. User/caller input may request a profile; it may not assert that a channel is closed. The runner reports what it actually controlled, observed, rejected, or left open.

At minimum distinguish:

- normal exit code;
- exact Unix signal where available;
- core-dump status where available;
- controller timeout;
- spawn/exec failure;
- resource-limit outcome when actually observable;
- process-tree completion/cleanup state;
- unsupported/unknown termination cause.

Two different signals or an exact signal versus unknown may never collapse to the same identity. An unknown remains typed unknown and prevents D1.

Bind the world: executable/interpreter bytes, argv/stdin, environment, source/scripts, lockfiles/dependencies available to the controller, initial filesystem/fixtures, declared artifacts, OS/kernel/arch, sandbox version, capability/policy schema, cassette, and supervisor/helper identity when present. Boundary wall time may be telemetry but never replay identity.

Replace the short caller-controlled honesty vector with an exhaustive versioned channel inventory covering at least: real network/DNS; wall, monotonic, CPU, vDSO, and hardware time; entropy devices/getrandom/hardware RNG; filesystem content/metadata/order/space/locks/escape; loader/dependencies; ASLR/address output; PID/TID/hostname/uname/proc/sys/dev; signals/timers; threads/forks/exec/descendants; inherited file descriptors; IPC/shared memory; async/io_uring; CPU/FP features; JIT/GC/finalizers; and unsupported syscalls/effects. A channel is closed only by controller evidence, never an empty list.

Replace the one-pair 0.0/1.0 `divergence rate` with numerator, denominator, campaign/sample identity, and confidence-free raw counts over a declared suite. Pairwise agreement remains a sampled falsifier, not proof.

Every safe-runner execution has an explicit configured deadline and bounded output. On expiry, close inputs, kill and wait for the direct child, and report exact cleanup state. If descendants or process-group cleanup cannot be proven in safe Rust, record that channel open. No unbounded wait remains. The safe runner is not a hostile-code security boundary.

Keep every unclosed channel D2. Safe Rust limitations are reported, not bypassed. Process-group/resource enforcement that needs raw ABI work belongs to the later helper.

**Acceptance:** exact termination distinctions and world/capability identity are gate-protected; no caller can mint D1; unsupported channels mechanically yield D2/UNCHECKED; Tier-1 identities remain untouched.
**Kill/stop:** if the safe runner cannot observe a distinction, encode `Unknown(open_channel)` and defer closure rather than guessing.

## 9. C5 — real child-visible cassette transport

**Finding routed:** E.2 BLOCKER; agent-system reach prerequisite.

The child must make the request. Parent-side lookup followed by interpolating returned bytes into generated Python is forbidden as acceptance evidence.

Build a persistent, versioned cassette plus one minimal child-visible endpoint/SDK/stdio transport. A Unix-domain-socket protocol is acceptable for the Linux-first cooperative profile; another transport is acceptable only if it has an equally small, auditable, deterministic framing contract. Do not build transparent multi-provider TLS interception in this phase.

The canonical request covers provider, model, ordered roles/messages/content, tools and schemas, tool choice, structured-output parameters, sampling parameters, and all behaviorally relevant fields. History semantics cover ordered repeated identical requests rather than a one-key map overwrite.

The response/effect tape represents:

- success status and bytes;
- provider/tool errors and status codes;
- timeout/cancellation;
- tool calls and structured payloads;
- exact streaming chunks, order, and terminal frame;
- request sequence, response sequence, and complete consumption;
- cassette miss/mismatch and transcript taint.

Bind cassette file digest, schema, broker/SDK version, request/response history, and miss/extra state into `RunRecord` and evidence v2. Exact-match-or-miss is law. There is no fuzzy matching, silent live fallback, or live capture in evidence mode. A miss is `UNCHECKED`, not `FINDINGS`; missing evidence is not a target defect.

Before the supervisor, real network remains an open channel unless a separately proven mechanism denies it. Therefore a successful cooperative cassette run is Tier 2 / D2, not D1. The receipt must say so.

Required fixtures:

- child makes one real protocol request and consumes cassette response;
- repeated identical requests consume distinct ordered entries;
- error, timeout, tool-call, and stream cases replay exactly;
- request mismatch, missing entry, extra unconsumed entry, reordered stream, and direct-network-capable profile all fail closed or remain UNCHECKED;
- no parent source interpolation can satisfy the positive test.

**Acceptance:** a single-thread CPython child uses the transport, two runs reproduce the complete cassette history and observation, every miss/extra effect taints, and the full identity enters v2 evidence.
**Scope checkpoint:** two focused working days is not an acceptance escape hatch. If the minimal child fixture is still blocked, stop scope expansion, preserve the failing evidence, and raise one precise operator gate. E.2 remains open; C6/C7 do not treat a parent-side or black-box proposal as satisfying C5, safe-phase completion, or supervisor readiness. Do not grow an eval platform.

## 10. CD — documentation and doctrine integrity (parallel docs-only package)

Route audit G.1-G.6 without rewriting history. CDa may handle static G.1/G.2/G.4/G.5/G.6 errata after C0. CDb handles G.3/A.5 only after C1's observation contract is final:

- canonicalize `REJ-R1..R3` for first-principles rejections and `REC-R0..R8` for roadmap recommendations; mark legacy bare `R#` non-canonical and correct the executor's RL reference;
- state that the ratified seven build-plan/governance criteria are canonical; mark conflicting DESIGN metrics historical/aspirational unless separately ratified;
- grow trace/observation documentation to include decision tape and the C1 identities;
- reconcile Tier-2 scope and the $10k allocation before C7; recorded budget unknown means no spending authority;
- correct the boundary-crate SSOT and call the current implementation `Tier-2 D2 subprocess MVP; D1 future backend`;
- preserve PR #17's captured historical text, then append a conspicuous erratum stating that its recorded `12 insertions` diffstat is false and that `git diff --numstat 947dba9^ 947dba9 -- DESIGN.md` proves `11  0`; do not rewrite the receipt as though it originally emitted 11;
- preserve PR #23's narrow null and all fired kill criteria.

CDa may merge after C0; CDb waits for C1. Both must be on main before C7 unsafe admission.

## 11. C6 — measured cassette-backed D2 reach campaign

Do not jump from a passing demo to `agent systems supported`. Define a reference profile and run a real campaign.

Minimum profile:

- Linux x86-64 and pinned CPython/tool dependencies;
- one-process/one-thread cooperative child using C5 transport;
- immutable fixture/world plus fresh writable area per run;
- at least 100 run-twice pairs over a declared reference suite;
- raw divergence numerator/denominator, every open channel, cassette history identity, and v2 bundles for findings;
- a leak battery that attempts time, entropy, real sockets/DNS, filesystem escape/order/metadata, proc/sys, inherited FDs, signals/timers, fork/thread, io_uring/IPC, ASLR/address output, and JIT/GC behavior, with every unsupported channel producing UNCHECKED rather than false CLEAN.

A reviewed in-repo cooperative agent fixture is sufficient for the safe D2 reference campaign. Executing a public checkout is a separate operator decision: the operator must name the repository/SHA and approve its license, data policy, execution plan, and disposable environment. `vh-sandbox` is not a hostile-code security boundary; never run an untrusted public harness directly on the operator workstation. Before the supervisor, only a pinned, code-reviewed cooperative fixture with no credentials, secrets, real-provider fallback, or ambient network authority may run locally. Without separate target authority, record criterion 4/live fire as blocked; do not silently substitute another target, credentials, or live paid calls.

For every candidate real bug, preserve source SHA, harness, world, exact bundle, independent reproduction request, human confirmation status, and whether it was previously known. `human-confirmed` is not granted by the building agent.

Any candidate corpus artifact remains corpus-track owned and lands through a separate K2 admission PR. The core C6 writer may prepare a reproduction bundle and interface request but may not edit `corpus/**`. A generic C6 harness is not the canonical `dharma_swarm` adapter receipt and does not close success criterion 7; report that criterion open unless the separately ratified adapter actually lands.

The four-week realism clock is not a C7 prerequisite. Start it only after a human-ratified target suite, cadence, eligible elapsed-time definition, admissible previously-unknown definition, and confirmation owner are recorded. If an earlier start is claimed, prove it with the exact merged artifact and sustained execution evidence.

**Acceptance:** the reference campaign publishes exact D2 divergence evidence; direct leak probes cannot produce false CLEAN; one child-visible agent harness is runnable from a v2 bundle under the supported profile.
**Kill criterion:** any silent leak found by the declared battery caps Tier 2 at experimental/UNCHECKED until that channel class is closed. Fewer than three admissible real bugs after the properly started four-week sustained-harvest window fires the existing realism kill and redirects work to fidelity, not more exploration.

## 12. C7 — separately audited unsafe-helper admission (human decision gate)

Do not write supervisor code under authority from C0-C6.

Prepare a small decision/admission PR that contains:

- exact supported target and non-goals;
- threat/channel model and syscall/effect support profile;
- preferred sibling-repository topology and excluded-workspace fallback;
- explicit files/repo, ownership, reviewers, and allowed unsafe ABI wrappers;
- length-prefixed versioned safe-core/helper protocol;
- helper content digest, capability/policy schema, kernel/arch, target/world, and tape fields required in receipts;
- host capability probes for Linux x86-64, kernel, ptrace policy, seccomp/user-notification availability, cgroup v2, namespaces, and two clean hosts;
- remaining budget and a no-unapproved-spend rule;
- ten-working-day start/end timestamps and stop rule;
- rollback: main workspace remains safe and cassette-backed D2 remains useful if spike dies.

C7 explicitly ratifies what zero-external-dependency means. If it does not allow a first-party separately built helper, D1 is killed at admission; do not smuggle an exception. It must name: topology; repository owner/organization; who creates the repository; visibility; license; branch protection; exact repo/path; allowed unsafe files/functions/ABI wrappers; build hosts; credentials policy; named independent unsafe/security reviewer; protocol reviewer; two evidence hosts; budget; and cleanup/archival owner. Never infer public versus private, organization, license, or permission to call a repository-creation API.

Preferred topology is a sibling `vibe-halt-supervisor-linux` repository. The human operator creates it, or separately and explicitly authorizes the agent to create that exact repository with the recorded settings. The fallback `tools/vh-supervisor-linux` requires an explicit root-workspace `exclude`/non-membership proof and separate build; it is not authorized merely because C7 mentions it. In both cases `crates/vh-sandbox` stays safe Rust and communicates over a minimal protocol.

Human merge of C7 and the separate explicit location/repository authorization are both required before helper code. Passing helper code cannot support D1 and cannot merge until the named non-author reviewer audits every unsafe block and the safe-core/helper protocol. If authority is absent, stop only the supervisor lane, finish C0-C6/CD/V1/K1, and return one decision-ready packet.

## 13. S1-S7 — ten-working-day Linux supervisor spike (conditional)

Start only after C7 authority. Partial rungs remain Tier 2 / D2 / UNCHECKED. D1 is admitted only after every channel in the declared supported profile is closed and the two-host gate passes.

### S1 — observation and protocol skeleton

- Safe core launches separately built helper through a versioned length-prefixed protocol.
- Exact signal/timeout/resource/process-tree outcomes; sealed capability receipt.
- Unknown message, syscall, effect, or protocol field fails closed.
- Helper digest and policy bind into every result.

### S2 — immutable world and single-process profile

- Bind executable/interpreter, scripts, dependencies, root filesystem, fixtures, environment, cassette, helper/policy, OS/kernel/arch, and writable layer.
- Admit exactly one process/thread. During partial work, unsupported `clone`, `clone3`, `fork`, `vfork`, further `exec`, shared memory, io_uring, async I/O, unmanaged signals/timers, and descendant escape may be reported while the run remains D2/UNCHECKED. For D1 admission, every reachable attempt must be synchronously blocked before side effects; merely not observing one is not closure.
- Enforce CPU, memory, file, process, and wall-budget cleanup without putting wall duration into replay identity.

### S3 — provider and network closure

- Allow only the C5 cassette broker/effect protocol for the supported fixture.
- Deny real sockets, DNS, and network namespace escape.
- Every request/response/stream/error enters the exact effect tape.

### S4 — virtual time, entropy, and host identity

- Interpose or cooperatively replace wall/monotonic/CPU clocks, sleeps, polling, deadlines, signals/timers, getrandom and entropy devices.
- Explicitly defeat, replace, or reject vDSO time and hardware time/RNG paths.
- Virtualize or reject PID/TID, hostname, uname, CPU count/feature queries, and proc/sys identity.
- Disable ASLR for the supported profile or reject address-derived observables; never normalize unknown address data silently.

### S5 — filesystem closure

- Content-addressed read-only root plus deterministic fresh writable layer.
- Deny escape and undeclared proc/sys/dev access.
- Normalize, model, or replay metadata, directory order, free space, locks, completion order, and declared fault outcomes.
- Pin or reject unmanaged JIT/GC/background behavior.

### S6 — exact tape and leak battery

- Version every effect request/response and consume the tape exactly.
- Missing, extra, duplicate, or reordered effects are DIVERGENT/UNCHECKED.
- Run the full C6 leak battery plus unknown-syscall and escape negatives; each must fail closed with the channel named.

### S7 — two-host D1 admission

On two clean Linux x86-64 hosts, the supported single-thread CPython/CLI fixture must produce 100/100 byte-identical world identity + complete effect tape + complete observation. The comparison artifact and both environments are independently identified. No checked-in self-report substitutes for execution.

At the exact helper commit and artifact digest, every unsafe wrapper receives an independent line-by-line audit, every required helper/protocol/security review and check is green, and no unresolved unsafe or security finding remains. Absence of an effect in 100 traces is never proof that its channel is closed; the capability policy and negative battery must synchronously close or reject it.

Only then may the exact profile be called Tier 2 / D1. Everything else remains D2. Never generalize from this fixture to arbitrary Python, multithreaded targets, other kernels, architectures, VMs, or hypervisor equivalence.

### Ten-day kill gate

The spike passes only if every S1-S7 acceptance is true within ten working days, every reachable channel in the declared profile is controlled or synchronously rejected, complete tape consumption and the leak battery pass, the independent unsafe audit is complete, and the two-host gate passes. Unknown syscalls, real network, time, and entropy are mandatory examples, not an exhaustive pass set. If it does not:

1. stop supervisor investment for v0.1;
2. publish the exact failed rung/channel and evidence;
3. keep the helper isolated or archive the spike branch without merging experimental unsafe into the main product;
4. ship the cassette-backed D2 profile honestly;
5. redirect the remaining runway to criterion 4 only if its realism kill remains open; otherwise redirect to the recorded fidelity bottleneck.

No deadline extension by relabeling partial closure D1.

## 14. Verification and PR protocol

For every package:

1. create a clean worktree from freshly fetched exact base;
2. announce branch, base SHA, track, owned files, and collision check;
3. write a failing adversarial regression before or with the fix;
4. make the smallest coherent change; files stay reviewable, normally under about 500 lines;
5. run before commit:

```bash
make onboard
make gate
cargo fmt --all --check
cargo clippy --workspace --all-targets --all-features --locked --offline -- -D warnings -F unsafe-code
cargo test --workspace --all-targets --all-features --locked --offline
RUSTDOCFLAGS=-Dwarnings cargo doc --workspace --all-features --no-deps --locked --offline
cargo run -q --locked --offline -p vh-cli -- doctor
```

For the separately admitted helper, use its own locked build, lint, test, sanitization/static-analysis, protocol-fuzz, and unsafe-audit commands defined in C7. Never pretend root `make gate` audits helper unsafe.

6. attempt at least one falsification of your green result;
7. push a draft PR with exact head SHA, changed surfaces, claim boundary, tests/outputs, kill disposition, and rollback;
8. inspect every CI job/step and unresolved review thread at that exact SHA; aggregate green alone is insufficient;
9. answer findings `FIXED`, `NOT REPRODUCIBLE` with evidence, or `DEFERRED` with owner/date/claim reduction;
10. the package writer stops feature mutation while awaiting human merge and performs only CI/review repairs on that PR; the coordinator moves to another dependency-independent, surface-disjoint package. After human merge, fetch, re-anchor, rerun onboard/gate, update the campaign-state comment, and clean the worktree.

Historical merged-PR threads are evidence, not automatic write targets. Do not reply to or resolve PR #17/#19/#20 threads without explicit operator authorization. Fix substantive debt in a new PR, cite the historical thread ID, and classify a stale marker as administrative only after checking the final merged code. Never mark a thread resolved merely because code moved or a bot marker became outdated.

## 15. Overnight autonomy and persistence

When a PR awaits human action, do not go idle if a dependency-independent package is safe:

- C1, C2-core/K1, C4, and CDa can occupy separate worktrees after C0 when their exact file locks do not overlap;
- read-only threat modeling and adversarial test design can continue without edit authority;
- CI failures are investigated immediately;
- reviews are polled and answered;
- no second writer enters the same surface;
- no more than one open implementation PR per package;
- no silent branch stacking.

At dawn, post one concise operator packet listing:

- current merged main SHA;
- each package: merged / open-ready / failing / killed / blocked;
- exact PR head and base, CI jobs, reviews, unresolved threads, and next merge order;
- all identity migrations and re-derived values;
- audit finding disposition matrix A.1-A.5, B.1-B.3, C.1-C.3 baseline, D.1-D.6, E.1-E.5, F.13/F.14, and G.1-G.6;
- current corpus exact-count table and criterion status;
- D2 reference divergence numerator/denominator if reached;
- supervisor admission/spike status and elapsed working days;
- one operator action, only if irreducible.

Do not end merely because context is long. Leave a resumable exact-state capsule in the active PR comment or durable campaign location permitted by repo law, then continue or hand off.

## 16. Completion contract

### Safe-phase completion

C0-C6, CDa/CDb, V1, and K1 are complete only when current merged main proves:

- A.1: oracle-consumed end state and full public observation are replay identity;
- A.2/A.4/A.5: address-derived observables fail closed or the D0 claim is explicitly narrowed; a 1,000-run SimRuntime full-observation artifact is byte-compared across the OS matrix while provenance remains separate; hash-only identity prose is corrected;
- B.1-B.3: exact recall/control gates and fail-closed oracles cover every corpus entry;
- C.1-C.3: the PR #23 narrow null, event-priority naming, and fail-closed bakeoff remain intact on the merged baseline;
- D.1-D.6: strict v2 bundles verify the complete observation and exact shrink lineage; stale output cannot mislead; evidence claims are accurately scoped;
- E.1-E.5: subprocess outcomes, world/capabilities, open channels, cassette/history, and measured divergence are typed and bound;
- E.2: a child—not the parent—performs cassette requests;
- F.13/F.14: the live risk ledger is evidence-based and supervisor status is `admitted`, `killed after an authorized run`, or `not run by operator decision` rather than implied;
- the D2 reference profile has executable evidence and no declared leak can return false CLEAN;
- doc doctrine matches code and the seven canonical success criteria; criterion 7 remains open unless the canonical dharma adapter receipt separately lands;
- every included finding is either FIXED on merged main or explicitly DEFERRED with owner, date/trigger, falsifier, and claim reduction; no BLOCKER named in the safe-phase contract may be silently deferred;
- all packages are independently reviewed, gate-green, and merged by the human.

### Full campaign completion

After safe-phase completion, exactly one of these must be true:

1. the explicitly supported Linux profile passes S7; the independently reviewed helper and safe-core adapter are human-merged at cited SHAs; both host comparison artifacts and the unsafe audit are identified; and the profile is admitted Tier 2 / D1 without generalization; or
2. the authorized ten-day spike runs and its kill fires; supervisor work is stopped/isolated, cassette-backed D2 remains truthful, and the program redirects to the recorded fidelity/target bottleneck; or
3. the human explicitly declines C7 location/unsafe authority; that decision is recorded, no helper code is created, cassette-backed D2 ships, and F.14 is reported `not run by operator decision`, never `killed` or `failed`.

The campaign is not complete because code exists on a branch, CI once passed, a checked-in receipt says it passed, or an agent reports confidence. Completion lives on current merged main plus the separately identified two-host evidence for any D1 claim.
