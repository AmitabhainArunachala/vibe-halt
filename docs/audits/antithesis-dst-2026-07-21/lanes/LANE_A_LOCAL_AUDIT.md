# Lane A — Local Code Audit: vibe-halt vs Antithesis-class DST

- **Date**: 2026-07-21
- **Auditor lane**: A (local code audit), adversarial technical assessment
- **Repo**: `/Users/dhyana/vibe-halt` (GitHub: AmitabhainArunachala/vibe-halt)
- **Audited tree**: working tree at `3e2a5ed` (branch `claude/vibe-halt-phase1-sandbox-goal`), clean
- **Pinned public commit**: `84f911e` (origin/main; differs from audited tree only by one docs file, PR #10)
- **Evidence modality per claim**: observed (code inspected) / reproduced (command run, transcript in `../commands/`) / reported (docs claim only) / inferred
- **Rule**: no repo file was modified; all outputs live under `docs/audits/antithesis-dst-2026-07-21/`

---

## 1. Executive verdict

vibe-halt is a **real, working, remarkably honest Tier-1 deterministic simulation
kernel** — two days old (first commit 2026-07-20, 44 commits at audit time) —
with evidence discipline that is genuinely Antithesis-*inspired* and in places
Antithesis-*grade* (tri-state verdicts that cannot be gamed into CLEAN, frozen
identities, sampled-falsifier epistemics named as such, runner-owned evidence
ledgers). Every load-bearing claim in CLAUDE.md/README that I tested
**reproduced exactly**: zero external dependencies, the deny-list gate, frozen
PRNG vectors, chain-hashed traces, exit-0-only-if-clean semantics, and all five
pinned corpus recall numbers.

It is **not** an Antithesis-class system in capability. There is **no
state-space search, no coverage-guided exploration, no schedule-choice
exploration** — exploration is independent random seeds with uniform-random
fault plans. The scheduler is a fixed `(virtual_time, seq)` FIFO, not an
adversarial interleaving explorer. Minimization (ddmin over fault plans) exists
as a **library only, unreachable from the CLI**. And the only testable targets
today are **in-process Rust workloads implementing the `Workload` trait** — no
subprocess, no external services, no record/replay cassettes (Tier-2 is the
active, unstarted build goal at this HEAD). DESIGN.md's own tier doctrine
concedes Tier-3 (Antithesis's actual substrate) is an explicit multi-year
non-goal (`docs/specs/DETERMINISM_TIERS.md:37-43`).

Nearest honest comparison: a two-day-old, evidence-hardened FoundationDB-style
deterministic simulator (pre-swarm era), or MadSim/Turmoil with much stronger
verdict/epistemics governance and a much smaller runtime — not Antithesis.

---

## 2. Ground-truth system map

Rust workspace, 8 crates, zero external dependencies
(reproduced: `Cargo.lock` contains exactly 8 packages, all `vh-*` —
`commands/denylist-direct.txt`; observed: `Cargo.toml` workspace manifest).
Toolchain pinned `1.94.1` (`rust-toolchain.toml`). ~10k lines of Rust total
(reproduced: `wc -l` per crate, below).

| crate | files / LOC (src) | role |
|---|---|---|
| `vh-core` | rng 220, seed 127, clock 62, sched 134 (+lib) | PRNG (SplitMix64/xoshiro256++), seed tree, virtual clock, deterministic scheduler |
| `vh-trace` | 159 | append-only trace, chained FNV-1a-128 hash, length-prefixed injective framing |
| `vh-gremlin` | 265 | 9 `FaultKind`s, `FaultPlan` (canonicalized, deterministic) |
| `vh-props` | 209 | `always` / `sometimes` / `EndStateOracle`, multiverse merge |
| `vh-multiverse` | lib 906, runtime 830, evidence 235 | universe runner, divergence detector, SimNet/SimDisk runtime, fault-lifecycle ledger |
| `vh-cli` | main 437, workloads 964 | `vh run` / `vh doctor`, demo + corpus workloads |
| `vh-shrink` | 846 + 2 test files | ddmin over fault plans (library) |
| `vh-verify` | 1088 + 6 test files | independent metamorphic re-verification battery + CI soak binary |

Other surfaces:

- `scripts/gate.sh` (229 lines) — the single gate battery implementation, run
  identically by `make gate` and CI (`observed`: `.github/workflows/ci.yml:44-45`).
- `scripts/check_determinism_denylist.py` (813 lines) — two-layer gate
  (structural manifest/lint semantics + line-regex defense-in-depth) with a
  self-test including bypass reproductions.
- `scripts/check_governance.py`, `scripts/onboard.py` — track/WIP/ownership
  admission governance.
- `.github/workflows/verify.yml` (384 lines) — cross-platform
  (ubuntu/macos/windows) 200-run replay soak with byte-shaped machine-readable
  receipts, release-mode shrinker gate, pinned-SHA actions.
- `corpus/` — PLAYBOOK + SCHEMA + 5 seeded entries (VB-001..005).
- `clients/python/` — quarantined stub (see §7).
- `docs/specs/{DETERMINISM_TIERS,TRACE_FORMAT_V0}.md`, `docs/plans/`,
  `docs/governance/ACTIVE_TRACK.yaml` (3 ACTIVE tracks, wip_max 3),
  `docs/prompts/` (builder dispatch specs).

---

## 3. Reproduced command evidence

All transcripts in `docs/audits/antithesis-dst-2026-07-21/commands/`.

| command | result | transcript |
|---|---|---|
| `make onboard` (stock `python3` = 3.9.6) | **FAIL, exit 2** — `ModuleNotFoundError: No module named 'tomllib'`; verdict NOT READY | `make-onboard.txt` |
| `make onboard` (python3.11 shim) | READY, exit 0; deny-list self-test + scan PASS; governance self-test PASS (4 bypass reproductions, 10 schema cases) | `make-onboard-py311.txt` |
| `make test` | exit 0; all suites ok (largest: 29 tests vh-multiverse; totals across 30 result lines) | `make-test.txt` |
| `make gate` | exit 0; full battery: deny-list, governance, fmt, strict clippy, tests, doctor, 200-universe divergence gate, 2 live CLEAN gates, 6 negative FINDINGS gates (exact exit 1 + anchored oracle lines), 5 corpus recall gates, nondet DIVERGENT gate, zero-universe exit-2 gate, replay-UNCHECKED exit-3 gate, Python-quarantine gate | `make-gate.txt` |
| `vh run --workload demo --universes 200` | exit 0, `verdict: CLEAN` | `cli-runs.txt` |
| `vh run --workload demo-buggy --universes 100` | exit 1, `verdict: FINDINGS`, per-universe `oracle:durability` failures with one-command repros | `cli-runs.txt` |
| `vh run --workload demo-buggy --seed 0xD1CE --universe 2` (failing universe) | exit 1, `replay verdict: FINDINGS`, same violated keys as the batch run (k8=v172, k9=v989), hash `8f1db0c9…` | `determinism-runs.txt` |
| `vh run --workload demo-buggy --seed 0xD1CE --universe 0` (non-failing) | exit 3, `replay verdict: UNCHECKED` — matches documented semantics | `cli-runs.txt` |
| `vh doctor` | exit 0, frozen identity `9ce6199f133f4d3c9dd0da0075e352d2` / 45 events + observable fingerprint OK | `cli-runs.txt` |
| **Adversarial determinism**: same `demo-buggy --seed 0xD1CE --universes 100` run twice as separate processes | `diff` of full outputs: **identical** (diff exit 0; both runs exit 1) | `determinism-runs.txt` |
| Seed sensitivity: `demo-buggy --seed 0xBEEF` | 38/100 failing universes vs 0xD1CE's different set — seeds genuinely change exploration | `throughput-seed.txt` |
| Corpus recall (seed 0xD1CE, 100 universes) | lost-update **29**, retry-double-apply **76**, dirty-read **83**, crash-toctou **21**, fsync-lie **21** — **all five match the pinned entry claims exactly** | `corpus-recall.txt` |
| Entry repro commands (VB-001 u1, VB-004 u9, VB-005 u5) | each exit 1 with the pinned oracle finding | `corpus-recall.txt` |
| Deny-list direct (py3.11): self-test + scan | PASS (18 regex samples, 21 manifest fixtures, 7 root fixtures, 12 boundary cases); 8 crates/30 files scanned | `denylist-direct.txt` |
| Throughput: 200 universes ×2 (divergence) demo, release | 0.428s wall — ~3.4M universe-executions/hour single-core on this toy workload | `throughput-seed.txt` |
| `~/.vibe-halt/` after full gate + all runs | **does not exist** — nothing writes there | reproduced (`ls`, exit 1) |

Caveat noted in-file: the `EXIT=0` lines in the first half of
`corpus-recall.txt` are the pipeline's grep exit, not `vh`'s; `vh`'s exit 1 for
those workloads is pinned by the anchored negative gates in `make-gate.txt`.

---

## 4. Subsystem maturity

| subsystem | maturity | evidence |
|---|---|---|
| vh-core PRNG/seed tree/clock/scheduler | **operational** | frozen reference vectors derived from official algorithms, not self-comparison (`rng.rs:108-150`); rejection-sampling consumption frozen (`rng.rs:156-181`); total-order scheduler with watermark (`sched.rs:36-76`); reproduced via doctor + double-run diff |
| vh-trace | **operational** (in-memory only; disk spill = Phase 2, disclosed `TRACE_FORMAT_V0.md:122-123`) | injective length-prefixed framing with forgery regression tests (`vh-trace/src/lib.rs:132-158`) |
| vh-gremlin fault model | **operational**, 9 kinds; `generate()` palette frozen at 5 v0 kinds | `vh-gremlin/src/lib.rs:12-35`; Phase-1 kinds (torn write, fsync lie, duplicate, reorder) never randomly generated — explicit-construction only, pinned by test (`lib.rs:218-235`) |
| vh-props | **operational** (always/sometimes/oracle only; no temporal properties) | `vh-props/src/lib.rs`; fail-closed sometimes-declaration (`:102-108`) |
| vh-multiverse runner + divergence detector | **operational** (sequential only) | non-adjacent two-pass replay (`lib.rs:840-879`); struct-equality observable identity (`:461-463`); schedule-keyed evasion documented + regression-tested (`tests/divergence.rs:625`) |
| SimNet/SimDisk runtime | **operational with documented limits** | runner-owned injection + lifecycle ledger (`runtime.rs`); limits: ClockSkew no-op (`:713-719`), network-wide partitions (`:688-692`), single held reorder slot (`:146`, `:360-387`), fixed 1µs base latency (`:51`), one runtime per universe (`:174-181`) |
| Fault-lifecycle evidence (Offered→Armed→Injected→Manifested→Recovered) | **operational** | `evidence.rs:1-37`; every stage transition trace-recorded (`runtime.rs:727-749`) |
| vh-cli | **operational** — `run`, `doctor` only | `main.rs:23-34`; exit semantics 0/1/2/3 reproduced |
| vh-shrink (ddmin) | **partial** — library + integration tests operational, **not wired into the CLI** | `vh-shrink/src/lib.rs:582-604`; integration via `run_universe_with_fault_plan` (`tests/multiverse_integration.rs:87`); reproduced: no `shrink` in `main.rs` (grep empty) |
| vh-verify (independent verifier) | **operational** | own re-implemented PRNG/scheduler/seed-tree/trace-framing tests (`crates/vh-verify/tests/`); CI soak with frozen receipts (`verify.yml:24`) |
| Python client | **quarantined stub** (honest) | `clients/python/vibe_halt/core/runner.py:1-28` — constructor raises; gate holds it closed (`gate.sh:213-227`); docstring confesses the previous version **fabricated success** (reproducibility_score=1.0 for nonexistent repos) |
| Tier-2/D1 hermetic sandbox | **planned — not started at this HEAD** | the audited commit *is* the goal doc (`docs/prompts/TRACK1_PHASE1_SANDBOX_GOAL_2026-07-21.md`); no sandbox code exists in `crates/` |
| Bug corpus | **operational as seeded harness; 0/25 toward the real-bug acceptance metric** | 5/5 seeded entries with reproduced pinned recall; SCHEMA law 3 concedes seeded = lower-bound only (`corpus/SCHEMA.md`); no harvested entries |
| `~/.vibe-halt/` receipts | **phantom surface** | referenced by `CLAUDE.md:26-27`, `.gitignore:16`, `TRACE_FORMAT_V0.md:123`; **no code writes there** (reproduced: directory absent after full gate); honestly disclosed as Phase-2 in the spec |
| DESIGN.md exploration stages 2–5, hierarchical shrinking, temporal properties, PRF seed derivation | **planned (spec), not in code** | see §6 defects D5 |

---

## 5. Capability assessment vs Antithesis-class DST

### 5.1 Exploration: independent random seeds only — no search

- Universe seeds derive from `root + golden_ratio*(id+1)` via SplitMix64
  (`vh-core/src/seed.rs:47-52`); per-universe fault plans are drawn
  **uniformly at random** over kind and time (`vh-gremlin/src/lib.rs:110-132`;
  the demo/corpus palettes in `workloads/*.rs` are likewise uniform draws).
- The code itself concedes this: "`v0` draws fault kinds uniformly; Phase 1
  replaces this with targeted scheduling biased toward state-transition edges
  and novel coverage" (`vh-gremlin/src/lib.rs:108-109`) — Phase-1 runtime
  landed, but the targeted/coverage-biased generation **did not** (palette
  frozen for compatibility, `lib.rs:211-235`).
- **No coverage instrumentation exists anywhere** (reproduced: grep for
  coverage finds only sometimes-assertion marking). No feedback from run N to
  run N+1, no novelty search, no swarm/budget allocation. Antithesis's core
  differentiator — coverage-guided multiverse exploration branching from
  interesting states — is entirely absent.
- Scheduler exploration: none. `Scheduler::pop` fires a fixed total order
  `(VirtualTime, seq)` (`vh-core/src/sched.rs:30-34, 71-76`). There are **no
  schedule choice points**; all timing diversity comes from fault-plan draws
  and message latencies. Same-time events always fire in insertion order, so
  workloads never experience adversarial same-timestamp orderings unless a
  fault reorders deliveries. DESIGN.md §3.3's "Select via recorded
  DecisionSource" (`DESIGN.md:124`) is not implemented.
- Verdict: exploration = **Monte-Carlo over fault plans**, not state-space
  search. Finding new bug classes depends on seed luck and hand-tuned workload
  palettes (which are honestly budgeted by construction, e.g.
  `workloads/net.rs:41-45` blackout-budget proof).

### 5.2 Determinism: the strongest part, verified adversarially

- Same command, two separate processes → byte-identical output (reproduced,
  `determinism-runs.txt`, diff exit 0).
- Frozen surfaces: PRNG vectors from independent transcriptions of the
  reference algorithms (`rng.rs:108-150`); trace hash framing
  (`TRACE_FORMAT_V0.md`); doctor's complete-observable fingerprint
  (`main.rs:299-351`); CI verify soak's frozen receipt regex (`verify.yml:24`).
- Deny-list is two-layer: **semantic** (tomllib manifest validation, path-dep
  confinement, lockfile binding, no `[patch]`/`.cargo`, rustc-enforced
  `unsafe_code = forbid` inherited by all 8 crates — reproduced,
  `denylist-direct.txt`) + line-regex defense-in-depth, with per-file
  per-pattern exemptions only (`check_determinism_denylist.py:67-94`).
- My own adversarial grep of kernel crates for
  HashMap/HashSet/SystemTime/Instant/thread/env/fs/unsafe: clean except the
  documented `vh-verify/src/main.rs` Instant exemption (upH telemetry,
  outside replay inputs) and the intentional `demo-nondet` atomic fixture —
  both registered in the script's exemption table (`:85-94`).
- The divergence detector's epistemics are stated better than Antithesis's
  public docs state theirs: pairwise replay agreement is named a **sampled
  falsifier, never proof**, with a regression test demonstrating
  schedule-keyed nondeterminism evading it
  (`tests/divergence.rs:625`, `lib.rs:677-712`).
- Residual risks: FNV-1a is non-cryptographic (disclosed,
  `TRACE_FORMAT_V0.md:117-121`; FNV-1a-64 stream-name collisions feasible for
  adversarially chosen names, disclosed `seed.rs:8-15`); all gates pin seed
  `0xD1CE`, so a determinism regression visible only at other seeds would
  evade the gate (mitigated by the verify soak's cross-platform matrix on a
  fixed seed, and by unit tests over other seeds).

### 5.3 Minimization: exists, but library-only and one level deep

- `vh-shrink` is a bounded ddmin over fault-plan injections with an exact
  cache, oracle-call/bytes/injection caps, and two verification modes
  (`vh-shrink/src/lib.rs:1-119`). Integration tests shrink a real failing
  plan through the public replay hook
  (`tests/multiverse_integration.rs:87`).
- **Not reachable from the CLI** (reproduced: `vh run`/`vh doctor` are the
  only subcommands). A user who gets a finding cannot shrink it without
  writing Rust. DESIGN.md's 5-level hierarchical shrinking (actions → fault
  families → occurrences → arguments → scheduling decisions,
  `DESIGN.md:173-180`) is 1-of-5 implemented (occurrences only).

### 5.4 What can actually be tested

- **Only in-process Rust code implementing `vh_multiverse::Workload`**
  (`lib.rs:359-381`), drawing all entropy from `UniverseCtx`. No subprocess
  universes, no interposition, no external processes or network services, no
  LLM cassettes. The runner's own docs mark untrusted code as belonging to
  "Tier-2 subprocess universes" (`lib.rs:25-26`) — which do not exist yet
  (goal doc at this HEAD). D2-style opaque-process chaos testing: absent.
- SimNet/SimDisk are real but minimal: fixed base latency, no topology
  (partition is network-wide), no bandwidth, one outstanding reorder hold, no
  per-node clocks (ClockSkew offered-and-skipped, honestly recorded).
  Semantic fault lifecycle measurement (Offered→…→Recovered) is a genuine
  strength — few DST systems attest per-injection outcomes this carefully.
- Legacy demo path caveat: `demo`/`demo-buggy` drain their own fault plans
  and only `CrashRestart` has any effect (`workloads/mod.rs:88-105`);
  NetworkDelay/Partition/DiskWriteFail/ClockSkew are recorded but inert
  there. This is pinned behavior (frozen doctor identity; doctor asserts the
  demo stays on the legacy path, `main.rs:410-416`), so it is a disclosed
  compatibility fossil, not a hidden fake — but a naive reader of the demo
  could overestimate what the demo exercises.

### 5.5 Verdict semantics: verified, ungameable-by-construction

- Tri-state CLEAN/FINDINGS/UNCHECKED; CLEAN requires divergence-checked AND
  non-empty property contract AND cardinality AND valid completions
  (`lib.rs:802-837`). Reproduced live: exit 0 (demo), 1 (buggy), 2 (usage /
  zero universes), 3 (single-replay / `--no-divergence-check`). The empty-
  contract→UNCHECKED and no-op-workload→never-CLEAN rules close the classic
  "certified nothing" hole by construction (`lib.rs:184-199, 274-357`).

---

## 6. Concrete defects and findings

**D1 — `make onboard` fails on stock macOS (medium).**
`Makefile:6` invokes `python3`; `scripts/check_determinism_denylist.py:58`
requires `tomllib` (Python ≥3.11); macOS system python3 is 3.9.6 →
`ModuleNotFoundError`, verdict NOT READY, exit 2 (reproduced,
`commands/make-onboard.txt`). No interpreter version check or pin anywhere.
CI (ubuntu-24.04, py3.12) is unaffected, so the failure is invisible to CI.
The same latent dependency affects `gate.sh:25-30` on any py<3.11 host.

**D2 — CLAUDE.md architecture section is stale (low).**
`CLAUDE.md:59-65` lists 6 crates and omits `vh-shrink` and `vh-verify`, which
are first-class workspace members with their own CI workflow (`verify.yml`).
Onboard renders the truth, but the governance file's own map is incomplete.

**D3 — `~/.vibe-halt/` receipts: policy references a nonexistent writer (low).**
`CLAUDE.md:26-27` and `.gitignore:16` speak of receipts going under
`~/.vibe-halt/`; no code path creates or writes it (reproduced: directory
absent after the full gate battery and all CLI runs). The spec is honest
(`TRACE_FORMAT_V0.md:122-123` marks spill-to-disk as Phase 2), so this is a
governance-file overstatement, not fabricated evidence — nothing claims a
receipt that wasn't produced. Still: today, evidence leaves the process only
via stdout; there is no durable local evidence store at all.

**D4 — DESIGN.md sign-off requirement unmet (low, honestly labeled).**
`DESIGN.md:5` requires ≥7 frontier-LLM sign-offs at ≥90% confidence; exactly
2 exist (`DESIGN.md:27-56`): Grok 92%, and Claude 91% — the latter a
**self-sign-off by the Track-1 builder of the very PRs being reviewed**.
Status line says "Draft" (`DESIGN.md:3`) and commit `a541579` says "2/7
signed", so the gap is disclosed; but the spec's own admission rule is
currently satisfied at 2/7 with one conflicted signature.

**D5 — DESIGN.md spec↔code gaps (informational; spec is a labeled Draft).**
Observed mismatches between the merged master spec and the code:
- §3.2 seed derivation "via PRF from universe_id + decision_kind +
  stable_site_id + occurrence + enabled_set_digest" (`DESIGN.md:111-116`) vs
  actual `universe_seed ^ fnv1a64(name)` (`seed.rs:55-58`) — no occurrence
  counters, no enabled-set digests; stream independence is name-keyed only.
- §3.3 scheduler "Select via recorded DecisionSource" (`DESIGN.md:124`) vs
  deterministic FIFO pop (`sched.rs:71-76`) — no decision recording.
- §4 property styles (never, at_most_once, after().within(), conservation,
  metamorphic, reference-model) (`DESIGN.md:149-160`) vs always/sometimes/
  EndStateOracle only (`vh-props/src/lib.rs`).
- §5 exploration stages 2–5 (`DESIGN.md:164-172`) and hierarchical shrinking
  (`DESIGN.md:173-180`) — not implemented (see §5.1, §5.3).
None of these are claimed as shipped anywhere in README/CLAUDE.md; the risk
is a reader treating DESIGN.md as description rather than aspiration.

**D6 — ClockSkew is a silent no-op inside generated plans (low, documented).**
`FaultPlan::generate` emits ClockSkew with weight 1/5
(`vh-gremlin/src/lib.rs:124`), but the v1 runtime offers-and-skips it
(`runtime.rs:713-719`). In the legacy demo path it is recorded with zero
effect (`workloads/mod.rs:88-105`). Documented in `runtime.rs:38` and
`evidence.rs:16-18`; the trace records the skip. Honest but dilutes every
generated plan's fault budget by ~20%.

**D7 — Cosmetic: dead assignments / drop-only finish (cosmetic).**
`workloads/corpus.rs:320,345` (`next_op` assigned, only `let _ =`'d —
DirtyRead, unlike WalDemo which uses it); `runtime.rs:634`
(`pub fn finish(self) {}` — works via `Drop`, reads like a stub).

**Non-defects verified (fabricated-success probes):**
- Python client cannot manufacture success: construction raises, gate pins it
  (`gate.sh:213-227`; reproduced in `make-gate.txt`).
- No empty-assertion CLEAN: `PropertyContract` + `RunOutcome` + cardinality
  checks (`lib.rs:802-837`); a workload returning `Completed` with no
  properties is UNCHECKED, verified by the demo-nondet replay gate (exit 3).
- No panic-blessing: negative gates require exact exit codes (1, not 101)
  plus anchored verdict lines (`gate.sh:59-200`).
- Evidence fields are private with runner-internal construction; the
  forgery-repro class is closed at compile time (`lib.rs:15-26`).

---

## 7. Honesty audit: claims vs reality

| claim (source) | verdict | evidence |
|---|---|---|
| Zero external dependencies, hermetic builds (CLAUDE.md:57-58, README:49-51) | **VERIFIED** | Cargo.lock = 8 workspace packages only (reproduced); `--locked --offline` throughout gate.sh |
| Deny-list enforcement layered, semantic + regex (CLAUDE.md:29-41) | **VERIFIED** | self-test + scan PASS (reproduced); my independent grep clean; exemptions per-file per-pattern (observed `:85-94`) |
| Frozen PRNG surface, `frozen_reference_vector` (CLAUDE.md:43-47) | **VERIFIED** | vectors derived from reference algorithms, incl. rejection-consumption freezing (observed `rng.rs:108-181`); test passes in `make test` |
| Chain-hashed trace, injective framing (TRACE_FORMAT_V0.md) | **VERIFIED** | observed `vh-trace/src/lib.rs:53-72`; forgery regressions exist |
| `vh run` exit-0-only-if-clean, incl. UNCHECKED semantics (README:29-33, CLAUDE.md) | **VERIFIED** | reproduced exits 0/1/2/3 across 8 CLI invocations |
| Every universe run twice, non-adjacent passes (README:58-65) | **VERIFIED** | observed `lib.rs:864-879` |
| Corpus: "five seeded bug classes with measured, pinned recall gates" (commit 6760e99) | **VERIFIED** | 29/76/83/21/21 reproduced exactly; gates anchored on oracle names |
| CI = `make gate` step-for-step (Makefile comment, ci.yml) | **VERIFIED** | ci.yml:44 runs `scripts/gate.sh`; single implementation (observed) |
| Runtime receipts under `~/.vibe-halt/` (CLAUDE.md:26-27) | **UNSUPPORTED as stated** | nothing writes there (reproduced); Phase-2 per spec |
| Python client "currently a stub" (CLAUDE.md:64-65) | **VERIFIED** (and the quarantine docstring's confession of prior fabrication is credible) | observed; gate-held |
| "7 frontier LLM sign-offs ≥90%" (DESIGN.md:5) | **UNMET** (2/7, one self-sign-off; labeled Draft) | observed DESIGN.md:25-56 |
| Tier-1 "bit-identical trace hash forever on any machine with the pinned toolchain" (DETERMINISM_TIERS.md:15-23) | **PLAUSIBLE, partially reproduced** (double-run diff identical; cross-platform claim rests on CI verify matrix — not re-run by me) | modality: reproduced locally, reported for other OSes |
| ≥10,000 Tier-1 universes/hour target (DESIGN.md:209) | **EXCEEDED on toy workloads** (~3.4M/hr single-core measured) — but workloads are microscopic | reproduced `throughput-seed.txt` |
| ≥12 fault families (DESIGN.md:206) | **UNMET** — 9 `FaultKind`s, 8 effective (ClockSkew no-op) | observed `vh-gremlin/src/lib.rs:12-35` |
| ≥25 real bugs / ≥80% recall acceptance (SCHEMA law 3, ACTIVE_TRACK) | **UNMET** — 5 seeded, 0 harvested | observed `corpus/entries/` |
| Shrinker median ≥70% reduction (DESIGN.md:208) | **UNVERIFIED** — no such measurement exists in repo | inferred (no bench/gate for it) |

---

## 8. Top gaps vs Antithesis-class DST (ranked)

1. **No coverage-guided or novelty-guided exploration** — exploration is
   uniform-random fault plans over independent seeds; no feedback loop, no
   branching from interesting states. The single largest capability gap.
2. **No schedule/interleaving search** — fixed `(time, seq)` FIFO scheduler;
   no choice-point recording or perturbation (DESIGN.md §3.3/§5 unimplemented).
3. **Target scope: in-process Rust only** — no Tier-2 subprocess sandbox, no
   interposition, no external services, no cassettes; cannot yet test the
   "vibe-coded repositories" that are the stated mission.
4. **Minimization unreachable** — ddmin exists but is library-only, one
   hierarchy level deep; no `vh shrink`.
5. **Corpus is 100% self-seeded** — zero harvested real-world bugs; recall
   numbers, though exactly reproducible, measure the rig against its own
   author (SCHEMA law 3 concedes this).
6. **Fault model thin at the edges** — 8 effective kinds; ClockSkew no-op;
   network-wide partitions only; no per-node clocks, bandwidth, or topology;
   no provider/tool fault family (LLM 429s, malformed responses) despite the
   agent-systems mission.
7. **No durable evidence store** — stdout only; `~/.vibe-halt/` phantom;
   trace spill-to-disk unimplemented.
8. **Environment fragility** — `make onboard` broken on stock macOS python3
   (D1); the repo's own "run `make onboard` first" law fails at first contact
   on the platform this audit ran on.
9. **Property language minimal** — no temporal/bounded-response properties
   (`after(A).within(t,B)`), no at-most-once, no reference-model equivalence;
   EndStateOracle is a good seed but the DSL of DESIGN.md §4 is absent.
10. **Sequential runner, single machine** — fine at current scale (and fast),
    but the 100k-universe soak / parallel-campaign items (DESIGN.md §7) are
    unbuilt; `UniverseCount::MAX` (2^20) and in-memory result retention are
    the documented bounds.

---

## 9. What this lane did NOT verify

- CI runs on GitHub runners (ci.yml / verify.yml green-ness at `84f911e`):
  reported by repo docs only; not re-executed here. The local equivalents
  (`make gate`, deny-list, doctor) all pass.
- Cross-platform (Windows/Linux) bit-identity of the frozen hashes:
  single-machine (macOS arm64) evidence only.
- vh-verify soak receipt `hash=eafa30e8…` / `upH` telemetry: pinned in
  verify.yml:24 and unit-tested in-crate; I did not run the release soak
  binary locally.
- Security of the deny-list against a motivated adversary with macro/codegen
  tricks beyond the script's own 18-sample bypass suite: the script itself
  labels the regex layer defense-in-depth (`check_determinism_denylist.py:34-39`).
