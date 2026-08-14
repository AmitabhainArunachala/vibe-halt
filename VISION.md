# Mega Hyper Vibration Multiverse Halting Machine — Vision

`vibe-halt` is an AI-native adversarial verification environment for vibe
coders and serious builders who cannot safely trust AI-generated or AI-modified
software. A target may be a whole repository, application, feature, agentic
system, trading system, or high-level fintech workflow. The canonical product
boundary and decision contract live in
[`docs/specs/PRODUCT_LOCK_V1.md`](docs/specs/PRODUCT_LOCK_V1.md).

Its strongest demonstrated engine today is a deterministic multiverse bench
for agent-shaped state machines: engine-owned execution, semantic fault
injection, executable properties, content-addressed fail-closed replay
evidence, and capture-enabled fault-plan minimization whose receipt states
every uncontrolled channel. That engine is the first deep attack engine and
evidence spine, not the product boundary.

The physical metaphor is an electrodynamic shaker table plus HALT rig. The
software is stressed across many reproducible worlds so that weak assumptions
fail quickly and every retained finding has inspectable evidence.

## Why it exists

AI-generated and agent-maintained systems concentrate defects at boundaries:
retries, acknowledgements, persistence, checkpoints, cancellation, tool calls,
and cross-file state. Ordinary happy-path tests often miss those defects.
`vibe-halt` should make the relevant orderings and failures cheap to explore
without pretending that an opaque process is deterministic.

The product front door is intentionally broader than current engine reach. A
whole target may enter, but every run must bind the exact revision and report
tested, unsupported, unreachable, uncontrolled, and unknown surfaces. A broad
input envelope never permits a broad safety claim.

## Enduring laws

1. **The engine owns truth.** A client may request work; it cannot manufacture
   a clean verdict, capability grade, digest, or receipt.
2. **Every claim names its boundary.** Tier 1/D0 is closed simulation. The
   present subprocess path is Tier 2/D2 unless every relevant channel is
   mechanically closed. Agreement from two runs is a sampled falsifier, never
   a proof.
3. **Evidence fails closed.** Missing, malformed, stale, ambiguous, tainted, or
   unverifiable evidence is `UNCHECKED` or an error, never `CLEAN`.
4. **Real utility outranks self-demonstration.** Seeded fixtures and reductions
   are regression assets. They do not count as previously unknown,
   human-confirmed bugs.
5. **A negative result is a result.** Predeclared nulls, stopped investments,
   and unreachable grades remain visible.
6. **Humans merge and confirm.** Green automation is evidence, not approval.

## Proven boundary in the current tree

Merged `main` commit `63ccd32` established the inherited Reality Bridge slice;
the current tree adds only the local negotiated cooperative-v2 contract
described below. Recheck the complete current boundary with `git rev-parse
HEAD`, `cargo test -p vh-cli negotiated --locked --offline`, and `make gate`.
The earlier exact merged commit passed
[CI run 30965578160](https://github.com/AmitabhainArunachala/vibe-halt/actions/runs/30965578160)
and [Verify run 30965578156](https://github.com/AmitabhainArunachala/vibe-halt/actions/runs/30965578156).

- a dependency-free Rust determinism kernel, simulated network and disk,
  semantic gremlins, executable properties, and multiverse execution
  (`scripts/check_determinism_denylist.py`; runnable check: `make gate`);
- frozen Tier-1 identities plus a 1,000-replay complete-observation job on
  Linux, macOS, and Windows (`.github/workflows/verify.yml:386-493`;
  [Verify #129](https://github.com/AmitabhainArunachala/vibe-halt/actions/runs/30365776537));
- strict v2 evidence bundles, standalone semantic replay,
  content-digest self-consistency checks, and exact-fingerprint fault-plan
  shrinking for the currently capture-enabled demo workloads
  (`crates/vh-cli/src/receipts_v2.rs:26-31`, `scripts/gate.sh`);
- a child-visible ordered cassette transport and a reference Tier-2 campaign
  with 0 divergent pairs in 100, while all 29 capability channels remain open
  (`scripts/gate.sh` and
  `docs/specs/SANDBOX_CAPABILITY_ENVELOPE_V1.md:16-33`); this is D2, never D1;
- a strict stdlib-only Python adapter that copies and optionally digest-pins the
  Rust executable, consumes only Rust machine records, and re-verifies the run
  receipt before returning a typed outcome
  (`clients/python/vibe_halt/core/runner.py`);
- an additive local cooperative-v2 protocol whose Rust-owned closed registry
  publishes a same-engine manifest, refuses unsupported operations/features
  and an `unknown` requested revision before child execution, and binds a fresh
  SHA-256 observation of the staged fixture source to the caller's exact
  coordinate in a revision-bound receipt while naming the execution
  ceiling `staged-d2` and its observation-to-exec channel `open`
  (`crates/vh-cli/src/protocol.rs`; runnable check:
  `cargo test -p vh-cli negotiated --locked --offline`); its `executions`
  counter records admitted sandbox-run attempts, so zero proves no sandbox
  boundary invocation while a positive count does not attest a child start;
- a versioned holdout/dossier contract plus two calibration fixtures, with
  misses and authority-blocked states retained
  (`docs/specs/HOLDOUT_CONTRACT_V1.md`, `corpus/calibration/`);
- eleven regression-corpus entries: six seeded instruments and five reductions
  of already-published real issues (run
  `find corpus/entries -maxdepth 1 -type f -name 'VB-*' | sort`).

The project has **not** yet demonstrated:

- a production or `dharma_swarm` adapter receipt; the negotiated,
  revision-bound cooperative-v2 path covers only the Rust-owned local fixture
  and does not close either the target-byte or copied-engine
  observation-to-execution channel
  (`clients/python/vibe_halt/core/runner.py`,
  `crates/vh-cli/src/cooperative.rs`);
- a real foreign target executed through that adapter;
- an independently curated frozen `N >= 25` acceptance holdout;
- any previously unknown, independently human-confirmed bug
  (`docs/audits/REACH_STRATEGY_DEBATE_PACKET_2026-07-25.md:64-71`);
- D1 subprocess containment or Tier 3 hypervisor determinism.

The Python package exposes only a strict local request/result/runner client. It
snapshots an explicitly configured Rust engine, strictly consumes its
same-copy protocol manifest, validates closed machine records after fresh Rust
replay, and trust-qualifies public checked outcomes. Cooperative v2 now carries
versioned operation/feature negotiation plus a verified observed fixture-source
revision; Python can expose those Rust values as data but cannot construct a
fresh or verified revision (`clients/python/vibe_halt/core/runner.py`,
`clients/python/tests/test_transport.py`). It does not implement the
`dharma_swarm` sandbox ABC or a real foreign-target receipt. Criterion 7
therefore remains OPEN.

## Current technical frontier — the reality bridge

The next technical proof milestone is:

> Run one operator-authorized, version-pinned foreign target through a strict
> Rust-backed adapter and either produce one independently forward-confirmed
> candidate or publish the predeclared null.

The campaign remains scoped in
[`docs/prompts/VIBE_HALT_REALITY_BRIDGE_LONG_RUNNING_GOAL_2026-07-29.md`](docs/prompts/VIBE_HALT_REALITY_BRIDGE_LONG_RUNNING_GOAL_2026-07-29.md).
The local fail-closed transport to the existing Rust evidence authority is now
implemented. The remaining bridge prepares exact target-admission packets and
runs foreign code only after the operator approves the named repository,
revision, license, data boundary, oracle, fixed budget, and disposable
environment in issue #60.

Known published defects may validate transfer from a reduced model to a real
target, but they do not satisfy the unknown-bug criterion. A candidate counts
there only after an independent human confirms it was previously unknown and
real.

## Rebased 12-week path

### 0. Whole-target product envelope — ratified, not yet implemented

Implement a bounded target manifest, target map, coverage plan, coverage
ledger, and target-level `HALT`/`PROCEED`/`UNKNOWN` receipt without weakening
the engine-layer truth states. The product benchmark is confirmed,
severity-weighted, unique fault yield under equal time and monetary budgets
against at least three decorrelated frontier-AI reviewers
(`docs/specs/PRODUCT_LOCK_V1.md`).

### 1. Truth substrate — proven, keep ratcheting

Maintain the deterministic kernel, complete-observation identity, evidence
schemas, replay, shrink, D2 capability ledger, and exact regression gates.
Never weaken these to accelerate reach.

### 2. Reality bridge — current

Preserve and subject the local versioned operation/feature and
observed-fixture-revision contract, then validate one operator-authorized,
version-pinned foreign target or publish the predeclared null. Preserve typed
`CLEAN`/`FINDINGS`/`UNCHECKED`/error outcomes throughout; do not promote the
local staged-byte equality into causal execution or external-target evidence.

### 3. Honest external evaluation

Create a pre-registered holdout of at least 25 provenance-qualified real
defects introduced by disclosed AI-generated or coding-agent-authored
PRs/commits. Mere later agent maintenance and generic conventional-code
defects are ineligible. An independent curator—not the engine or adapter
builder—owns candidate identity, eligibility, and the ground-truth oracle.
Freeze the exact engine and adapter commits, executable digest, protocol
schemas, evaluation policy, sampling frame/cutoff/selection/dedupe/diversity
rules, and a curator-authenticated opaque cohort commitment before reveal by
human-merging a candidate-secret artifact to public `main`; bind a later
curator-authenticated reveal to that commit/digest and require its canonical
bytes to open the commitment. Require at least five repositories and five
root-cause/mechanism clusters, with no repository or cluster contributing more
than `ceil(0.2 * N)`. Misses stay in the denominator. For a frozen cohort of
size `N`, detect at least `ceil(0.8 * N)` within fixed, predeclared per-target
budgets and independently fixed oracles (at least 20 only when `N = 25`).
Only the first complete execution under that pre-reveal freeze earns
acceptance credit. Later implementations may rerun the revealed cohort only as
non-credit calibration; a new acceptance attempt requires a newly curated
unseen cohort. The existing eleven entries remain the regression/training
corpus, not the acceptance denominator.

Pursue three previously unknown candidates, but award no credit without
independent human confirmation.

### 4. Integration, scale, and release

Produce one end-to-end `dharma_swarm` receipt through the strict adapter,
certify at least 1,000 Tier-1 universes/hour on a named build box, measure
representative median shrink of at least 90%, and package an independently
reproducible release.

D1 supervisor work remains a separately admitted option. It is not a
prerequisite for proving external utility, and it cannot begin from this
vision document.

## Claim boundary

A passing `vibe-halt` campaign is not a general safety certificate and does
not solve the halting problem. It increases confidence only for the named
target, revision, workload, properties, fault model, universe budget,
toolchain, evidence tier, and open-channel ledger recorded in its receipt.

The week-12 measurable contract and current criterion ledger live in
[`docs/plans/VIBE_HALT_BUILD_PLAN_2026-07-20.md`](docs/plans/VIBE_HALT_BUILD_PLAN_2026-07-20.md).
