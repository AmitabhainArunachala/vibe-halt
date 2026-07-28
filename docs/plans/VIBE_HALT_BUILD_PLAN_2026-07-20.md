# vibe-halt Build Plan — 12 weeks / $10k

Ratified 2026-07-20. Constraints: $10,000 total, 3 months, Rust primary,
demonstrable by end of month 3. Acceptance criteria are mirrored in
`docs/governance/ACTIVE_TRACK.yaml` (track `vibe-halt-core-2026-07`).

**Post-audit canonical status (2026-07-23, CDa):** this file's seven
week-12 success criteria and Budget table are the canonical criteria and
allocation. Conflicting historical or aspirational values in `DESIGN.md` are
superseded. The remaining balance is unknown; no executor has spending
authority. C7 must refresh and ratify the balance before any spend or
allocation change.

## 2026-07-29 rebaseline — merged `main` at `ab259c07`

This ledger is intentionally narrower than a percent-complete estimate. It
changes sequencing but not the 12-week budget. It also proposes one material
measurement-contract amendment for criterion 3: preserve the numeric threshold
(at least 80% over at least 25 real defects) while replacing its circular
admitted-corpus denominator with a pre-registered holdout that retains misses.
The target population remains real defects introduced by a disclosed
AI-generated or coding-agent-authored PR/commit; mere later agent maintenance
does not qualify an otherwise conventional defect. Human merge of this change
ratifies the measurement amendment; until then, it is a proposal rather than
current law.

| # | week-12 criterion | current evidence | status |
|---|---|---|---|
| 1 | Tier-1 identity across 1,000 runs and two machines | workflow contract at `.github/workflows/verify.yml:386-493`; [Verify #129](https://github.com/AmitabhainArunachala/vibe-halt/actions/runs/30365776537) passed on PR #53's synthetic merge, not an independently retained exact-`ab259c07` push run | **MET for the pinned reference workload; recertify every combined head** |
| 2 | publish Tier-2 divergence below 5% | current-tree command `make gate`; exact C5/C6 assertions at `scripts/gate.sh:104-190`; all-open ledger at `docs/specs/SANDBOX_CAPABILITY_ENVELOPE_V1.md:16-33` | **MET narrowly at D2; never D1** |
| 3 | at least 80% recall on at least 25 real defects | eleven regression entries exist (run `find corpus/entries -maxdepth 1 -type f -name 'VB-*' | sort`); no pre-registered holdout exists | **OPEN** |
| 4 | three previously unknown, human-confirmed bugs | `docs/audits/REACH_STRATEGY_DEBATE_PACKET_2026-07-25.md:64-71` records zero | **OPEN — 0/3** |
| 5 | one-command replay and median shrink at least 90% | bundle replay and capture-enabled demo shrink gates are `scripts/gate.sh:397-479`; representative median is not measured | **PARTIAL** |
| 6 | at least 1,000 Tier-1 universes/hour on the build box | no named build-box benchmark receipt certifies this threshold | **OPEN** |
| 7 | one end-to-end `dharma_swarm` adapter receipt | execution refuses in `clients/python/vibe_halt/core/runner.py:10-30`, while `clients/python/vibe_halt/__init__.py:5-9` still exports caller-constructible evidence | **OPEN** |

The load-bearing next move is therefore an external-utility bridge, not more
internal simulation machinery. The current execution scope is
`docs/prompts/VIBE_HALT_REALITY_BRIDGE_LONG_RUNNING_GOAL_2026-07-29.md`.
The separately proposed C7 supervisor remains deferred.

### Criterion 3 measurement-contract amendment

The original phrase "seeded corpus" allowed a circular reading: the corpus
schema admits an entry only after the rig detects it, so recall over admitted
entries cannot measure misses. The acceptance denominator is now:

- a **pre-registered holdout of at least 25 provenance-qualified real defects
  introduced by disclosed AI-generated or coding-agent-authored changes**,
  separate from the regression/training corpus;
- every candidate retained in the denominator, including misses;
- an independent curator—not the engine or adapter builder—owning candidate
  identity, eligibility, and ground-truth oracles;
- the exact engine/adapter commits, executable digest, protocol schemas,
  evaluation policy, independent oracles, and fixed per-target universe
  budgets plus a curator-authenticated opaque cohort commitment and frozen
  sampling-frame/cutoff/selection/dedupe/diversity policy published in a
  candidate-secret artifact human-merged to public `main` before a separately
  authenticated curator reveal that opens the commitment; the cohort spans at
  least five repositories and five root-cause/mechanism clusters, with no
  repository or cluster contributing more than `ceil(0.2 * N)`; and
- for a frozen cohort of size `N`, at least `ceil(0.8 * N)` detected at least
  once within those budgets (at least 20 only when `N = 25`).

Per-entry manifestation frequency remains diagnostic and regression-pinned;
it is not the aggregate recall percentage. The existing eleven entries remain
valuable regression assets but provide no holdout credit. Any implementation
or protocol change may rerun the whole revealed cohort only as non-credit
calibration while preserving all prior results. Only the first complete
execution under the pre-reveal freeze may earn criterion-3 credit; another
acceptance attempt requires a newly curated unseen holdout and a new
freeze→authenticated-reveal sequence.

## Scope decision (the one that matters)

Three determinism tiers (`docs/specs/DETERMINISM_TIERS.md`): ship Tier 1
(full determinism for code on the simulated runtime) and a Tier-2 D2
subprocess harness that measures and publishes divergence without claiming a
deterministic environment. Hermetic D1 remains a future target. Tier 3
(Antithesis-class hypervisor) is an explicit non-goal — multi-year at any
quality. The trace/oracle/property layer stays substrate-agnostic so a
hypervisor or rr-based backend can slot in later.

## Phases

### Phase 0 — Foundations (weeks 1-2) — DONE at scaffold
- Deterministic kernel: seed tree with name-independent streams, virtual
  clock, deterministic scheduler (`crates/vh-core`).
- Trace format frozen first (`docs/specs/TRACE_FORMAT_V0.md`); chained
  hash ledger (`crates/vh-trace`).
- Divergence detector as CI gate #1 (`crates/vh-multiverse`): every
  universe runs twice, hashes compared, mismatch fails loudly.
- Deny-list gate #0 (`scripts/check_determinism_denylist.py`): no wall
  clock / OS randomness / hash iteration / threads / I/O in kernel crates.
- Proof-of-life: seeded ack-before-flush durability bug caught with
  one-command repro (`crates/vh-cli/tests/demo.rs`).

### Phase 1 — Universe runner + gremlins (weeks 3-5)
- Tier-1 sim runtime on the scheduler: simulated network (partition,
  delay, reorder, duplicate), disk (torn write, ENOSPC, fsync lies),
  process crash/restart, and observable clock skew wired into workload
  execution. Network and disk controls/buggies plus the corpus gates exercise
  the shipped boundary; fault-family breadth beyond those gates is not implied.
- Tier-2 D1 target-state sandbox: subprocess universes under cgroups + netns,
  fault-injecting proxy, clock control, **LLM record/replay cassettes**
  (for agent systems the LLM call is the dominant nondeterminism source).
  **2026-07-29 disposition:** the shipped boundary is a **Tier-2 D2
  subprocess harness; D1 is a future backend**. Child-visible ordered
  cassette replay and a strict capability ledger now exist, but all 29
  channels remain open. Cgroups, netns, fault proxy, clock control, and
  controller-proven channel closure remain unimplemented. Any stronger claim
  requires separate ratification and evidence.
- Targeted scheduling was tested in bounded opt-in experiments. Palette
  guidance lost its bakeoff; the narrow schedule instrument did not establish
  an advantage. Investment remains stopped pending a new discriminating,
  predeclared experiment. FIFO remains the default.

### Phase 2 — Property system depth (weeks 6-8)
- End-state oracles (data integrity across crash/restart) joining
  always/sometimes (`crates/vh-props`).
- Fault-plan shrinker: minimize a failing universe's injections; target
  median >=90% of events removed. Exact-fingerprint resource-bounded shrink
  and lineage receipts have shipped; the representative median is still open.
- Vibe-bug corpus: eleven regression entries have shipped. The acceptance
  benchmark is the separate pre-registered holdout defined above, so misses
  cannot disappear through admission and generic conventional-code defects
  cannot broaden the target population.

### Phase 3 — Multiverse explorer (weeks 9-10)
- Parallel fan-out across cores that matches the sequential runner
  hash-for-hash (the sequential baseline is the reference).
- Bandit seed scheduling over fault-family × workload space; failure
  fingerprinting and dedup (500 failing universes → 4 distinct bugs).

**2026-07-29 disposition:** guided-exploration investment is stopped after
the published null and non-discriminating schedule experiment. Parallel
fan-out and adaptive search are downstream of external utility proof, not the
current critical path.

### Phase 4 — Integration + live fire (weeks 11-12)
- gRPC + CLI surface; thin Python client (`clients/python/`).
- dharma_swarm adapter: a `VibeHaltSandbox` implementing the `Sandbox`
  ABC (dharma_swarm `dharma_swarm/sandbox.py:37-54`), and a diff-verdict
  hook beside `diff_applier.py`/`build_engine.py`. Receipts under
  `~/.dharma/` per dharma_swarm's rules; tier named in every receipt.
- Live-fire demo on real vibe-coded repos.

**2026-07-29 disposition:** this is the current frontier. The first slice is a
strict Python-to-Rust evidence transport, followed by one exactly authorized,
version-pinned foreign-target confirmation attempt. No live target, provider,
credential, or sibling-repository write is implied by this plan.

## Budget

| item                | amount | notes                                        |
|---------------------|--------|----------------------------------------------|
| AI inference        | ~$5.5k | the engineering payroll; 12 weeks of sessions |
| compute             | ~$2k   | one 16-32 core box for fan-out + minimal CI  |
| human expert review | ~$1.5k | DST-experienced reviewer at wk 3 and wk 10, determinism kernel only |
| contingency/corpus  | ~$1k   | corpus bounties or final-soak compute        |

## Risks

1. **Determinism holes** — mitigated by gate #0 (deny-list) and gate #1
   (divergence, run-twice) live in CI from day 0, plus frozen PRNG/trace
   reference vectors.
2. **Tier-3 scope creep** — contractually out of scope; tiers doc is law.
3. **The tool is itself vibe-coded** — vibe-halt tests itself (the gate
   battery runs the rig against seeded bugs and a seeded nondeterminism
   leak on every commit); expert budget goes to the kernel.
4. **Demo-overfitting** — acceptance recall is measured on the independently
   curated frozen holdout, with engine/adapter identity frozen before reveal
   and no candidate-specific post-reveal tuning.
5. **Operator bus factor** — every session ends with committed state;
   `make onboard` reconstructs context.

## Original 2026-07-20 success wording (criterion 3 amended below)

1. Tier 1: same seed ⇒ bit-identical trace hash across 1,000 runs and two
   machines.
2. Tier 2: divergence rate measured and published (<5% target), never
   hidden.
3. **Historical wording, superseded when the 2026-07-29 amendment is
   human-merged:** >=80% recall on the >=25-bug seeded corpus within a fixed
   universe budget.
4. >=3 previously unknown, human-confirmed bugs in real code.
5. Every failure ships a one-command deterministic repro; median shrink
   >=90%.
6. >=1,000 Tier-1 universes/hour on the build box (reference workload).
7. One end-to-end dharma_swarm receipt via the adapter.

**Proposed 2026-07-29 criterion-3 measurement amendment:** human merge of the
amendment redefines item 3 operationally as >=80% aggregate recall on an
independently curated, pre-registered holdout of `N >= 25`
provenance-qualified real defects introduced by disclosed AI-generated or
coding-agent-authored changes: at least `ceil(0.8 * N)` detections, every
non-detection retained, with the independent curator, exact engine/adapter
commits, executable digest, protocol schemas, evaluation policy, independent
oracles, fixed per-target budgets, frozen sampling/selection/diversity policy,
and opaque cohort commitment published in a candidate-secret artifact
human-merged to public `main` before a separately authenticated curator reveal
opens that commitment. The cohort spans at least five repositories and five
root-cause/mechanism clusters; neither one repository nor one cluster may
contribute more than `ceil(0.2 * N)`. Only that pre-frozen implementation's
first complete run may earn acceptance credit. Later implementations may
rerun the revealed cohort only as non-credit calibration; a new acceptance
attempt requires a newly curated unseen cohort. The original 2026-07-20
wording remains above as history.

Failing (1) means the project failed regardless of the rest: false
confidence is the disease this machine exists to cure.
