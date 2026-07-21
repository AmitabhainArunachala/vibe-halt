# vibe-halt Build Plan — 12 weeks / $10k

Ratified 2026-07-20. Constraints: $10,000 total, 3 months, Rust primary,
demonstrable by end of month 3. Acceptance criteria are mirrored in
`docs/governance/ACTIVE_TRACK.yaml` (track `vibe-halt-core-2026-07`).

## Scope decision (the one that matters)

Three determinism tiers (`docs/specs/DETERMINISM_TIERS.md`): ship Tier 1
(full determinism for code on the simulated runtime) and Tier 2 (hermetic
reproducibility for arbitrary code, divergence rate measured and
published). Tier 3 (Antithesis-class hypervisor) is an explicit non-goal —
multi-year at any quality. The trace/oracle/property layer stays
substrate-agnostic so a hypervisor or rr-based backend can slot in later.

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
  process crash/restart wired into workload execution (today only
  CrashRestart is exercised by the demo; kinds exist in `vh-gremlin`).
- Tier-2 hermetic sandbox: subprocess universes under cgroups + netns,
  fault-injecting proxy, clock control, **LLM record/replay cassettes**
  (for agent systems the LLM call is the dominant nondeterminism source).
- Targeted fault scheduling: bias toward state-transition edges and novel
  coverage, replacing v0's uniform draw (`FaultPlan::generate`).

### Phase 2 — Property system depth (weeks 6-8)
- End-state oracles (data integrity across crash/restart) joining
  always/sometimes (`crates/vh-props`).
- Fault-plan shrinker: minimize a failing universe's injections; target
  median >=90% of events removed. Plans are already plain data for this.
- Vibe-bug corpus: >=25 real bugs harvested from AI-generated PRs; the
  recall benchmark. Evaluate rr for deterministic replay of divergent
  Tier-2 universes (buy, don't build).

### Phase 3 — Multiverse explorer (weeks 9-10)
- Parallel fan-out across cores that matches the sequential runner
  hash-for-hash (the sequential baseline is the reference).
- Bandit seed scheduling over fault-family × workload space; failure
  fingerprinting and dedup (500 failing universes → 4 distinct bugs).

### Phase 4 — Integration + live fire (weeks 11-12)
- gRPC + CLI surface; thin Python client (`clients/python/`).
- dharma_swarm adapter: a `VibeHaltSandbox` implementing the `Sandbox`
  ABC (dharma_swarm `dharma_swarm/sandbox.py:37-54`), and a diff-verdict
  hook beside `diff_applier.py`/`build_engine.py`. Receipts under
  `~/.dharma/` per dharma_swarm's rules; tier named in every receipt.
- Live-fire demo on real vibe-coded repos.

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
4. **Demo-overfitting** — corpus recall measured on unseeded real code
   before anything is called done.
5. **Operator bus factor** — every session ends with committed state;
   `make onboard` reconstructs context.

## Success at week 12 (measurable)

1. Tier 1: same seed ⇒ bit-identical trace hash across 1,000 runs and two
   machines.
2. Tier 2: divergence rate measured and published (<5% target), never
   hidden.
3. >=80% recall on the >=25-bug seeded corpus within a fixed universe
   budget.
4. >=3 previously unknown, human-confirmed bugs in real code.
5. Every failure ships a one-command deterministic repro; median shrink
   >=90%.
6. >=1,000 Tier-1 universes/hour on the build box (reference workload).
7. One end-to-end dharma_swarm receipt via the adapter.

Failing (1) means the project failed regardless of the rest: false
confidence is the disease this machine exists to cure.
