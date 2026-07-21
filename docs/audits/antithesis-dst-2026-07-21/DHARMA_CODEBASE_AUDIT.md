# vibe-halt Codebase Audit

> Layout note: this assessment audits **vibe-halt** (not dharma_swarm); this file fills the
> layout's local-codebase-audit slot for the repo under audit.
> Full lane detail: `lanes/LANE_A_LOCAL_AUDIT.md`. All command transcripts: `commands/`.

- **Audited tree**: `/Users/dhyana/vibe-halt` @ `3e2a5ed` (clean; branch `claude/vibe-halt-phase1-sandbox-goal`)
- **Pinned public commit**: `84f911e` (origin/main, 2026-07-21; differs by one docs file)
- **Repo age**: first commit 2026-07-20; ~44 commits; ~10k LOC Rust, 8 workspace crates, zero external dependencies (reproduced: `Cargo.lock` = 8 `vh-*` packages only)
- **Toolchain**: pinned `1.94.1` (`rust-toolchain.toml`)

## 1. Ground-truth system map

| crate | LOC (src) | role |
|---|---|---|
| `vh-core` | ~560 | PRNG (SplitMix64/xoshiro256++), seed tree, virtual clock, deterministic scheduler |
| `vh-trace` | 159 | append-only trace, chained FNV-1a-128 hash, injective length-prefixed framing |
| `vh-gremlin` | 265 | 9 `FaultKind`s, canonicalized deterministic `FaultPlan`s |
| `vh-props` | 209 | `always` / `sometimes` / `EndStateOracle`, multiverse merge |
| `vh-multiverse` | ~1970 | universe runner, divergence detector, SimNet/SimDisk runtime, fault-lifecycle ledger |
| `vh-cli` | ~1400 | `vh run` / `vh doctor`; demo + corpus workloads |
| `vh-shrink` | 846 + tests | bounded ddmin over fault plans (**library only**) |
| `vh-verify` | 1088 + tests | independent metamorphic re-verification battery + CI soak |

Other surfaces: `scripts/gate.sh` (single gate battery, run identically by `make gate` and CI), `scripts/check_determinism_denylist.py` (813 lines, two-layer + bypass self-test), `scripts/{onboard,check_governance}.py`, `.github/workflows/{ci,verify}.yml` (verify = cross-platform 200-run replay soak), `corpus/` (SCHEMA + PLAYBOOK + 5 seeded entries), `clients/python/` (quarantined stub), `docs/{specs,plans,governance,prompts}`.

## 2. Supported entry points and execution traces (all reproduced)

| command | result | transcript |
|---|---|---|
| `make onboard` (stock macOS python3 = 3.9.6) | **FAIL exit 2** — `tomllib` missing; NOT READY | `commands/make-onboard.txt` |
| `make onboard` (python3.11) | READY exit 0; deny-list self-test + governance self-test PASS | `commands/make-onboard-py311.txt` |
| `make test` | exit 0, all suites | `commands/make-test.txt` |
| `make gate` | exit 0 — full battery incl. 200-universe divergence gate, live CLEAN gates, exact-exit-code negative gates, corpus recall gates, Python-quarantine gate | `commands/make-gate.txt` |
| `vh run --workload demo --universes 200` | exit 0, CLEAN | `commands/cli-runs.txt` |
| `vh run --workload demo-buggy --universes 100` | exit 1, FINDINGS + one-command repros | `commands/cli-runs.txt` |
| replay of failing universe (`--universe 2`) | exit 1, identical violated keys + hash | `commands/determinism-runs.txt` |
| replay of non-failing universe | exit 3, UNCHECKED (documented semantics) | `commands/cli-runs.txt` |
| **same command, two separate processes** | **byte-identical output** (diff exit 0) | `commands/determinism-runs.txt` |
| seed sensitivity (`0xBEEF` vs `0xD1CE`) | different failing-universe sets — seeds genuinely change exploration | `commands/throughput-seed.txt` |
| corpus recall, 5 seeded entries | **29/76/83/21/21 per 100 universes — exact match to pinned claims** | `commands/corpus-recall.txt` |
| `vh doctor` | exit 0, frozen identity + observable fingerprint OK | `commands/cli-runs.txt` |
| `ls ~/.vibe-halt` after full gate + all runs | **does not exist** — nothing writes there | Lane A §3 |
| throughput, 200 universes ×2 (release) | 0.428s wall (~3.4M toy universe-executions/hr single-core) | `commands/throughput-seed.txt` |

## 3. Subsystem maturity table

| subsystem | maturity | key evidence |
|---|---|---|
| vh-core (PRNG/seed/clock/scheduler) | **operational** | frozen reference vectors from official algorithm transcriptions (`rng.rs:108-181`); total-order scheduler (`sched.rs:36-76`); double-run byte-identity reproduced |
| vh-trace | **operational** (in-memory only; disk spill Phase 2, disclosed) | injective framing + forgery regression tests (`vh-trace/src/lib.rs:132-158`) |
| vh-gremlin | **operational** (9 kinds; random palette frozen at 5 v0 kinds) | Phase-1 kinds explicit-construction only, pinned by test (`lib.rs:218-235`) |
| vh-props | **operational** (always/sometimes/oracle; no temporal properties) | fail-closed sometimes-declaration (`lib.rs:102-108`) |
| vh-multiverse runner + divergence | **operational** (sequential only) | non-adjacent two-pass replay (`lib.rs:840-879`); schedule-keyed-evasion regression test (`tests/divergence.rs:625`) |
| SimNet/SimDisk + fault lifecycle | **operational, documented limits** | Offered→Armed→Injected→Manifested→Recovered ledger (`evidence.rs`); limits: ClockSkew no-op (`runtime.rs:713-719`), network-wide partitions, single reorder slot, fixed 1µs base latency |
| vh-cli | **operational** (`run`, `doctor`) | exit semantics 0/1/2/3 reproduced |
| vh-shrink | **partial** — library + integration tests work; **not wired into CLI** | `vh-shrink/src/lib.rs:582-604`; no `shrink` in `main.rs` (reproduced grep) |
| vh-verify | **operational** | independently re-implemented PRNG/scheduler/seed-tree/trace tests + CI soak (`verify.yml:24`) |
| clients/python | **quarantined stub (honest)** | constructor raises; gate-held (`gate.sh:213-227`); docstring confesses prior fabricated `reproducibility_score=1.0` |
| Tier-2/D1 hermetic sandbox | **planned** (goal doc only, unstarted at pinned commit) | `docs/prompts/TRACK1_PHASE1_SANDBOX_GOAL_2026-07-21.md` |
| bug corpus | **operational as seeded harness; 0/25 real-bug acceptance** | 5/5 seeded recall reproduced; SCHEMA law 3 concedes lower-bound-only |
| `~/.vibe-halt/` receipts | **phantom** | referenced by CLAUDE.md:26-27 + .gitignore:16; no writer exists |
| DESIGN.md exploration stages 2–5, hierarchical shrinking, temporal properties, PRF seed derivation | **planned (labeled Draft spec), not in code** | Lane A defect D5 |

## 4. Distributed-systems and simulation semantics

- **Determinism (strongest area, adversarially verified):** frozen PRNG consumption including rejection sampling; doctor's complete-observable fingerprint; two-layer deny-list (semantic manifest/lockfile/`forbid(unsafe_code)` + regex defense-in-depth, per-file-per-pattern exemptions only); independent adversarial grep of kernel crates clean apart from two documented exemptions.
- **Scheduler:** fixed `(VirtualTime, seq)` FIFO, watermark-based, **no choice points** — same-time events always fire insertion-ordered; adversarial same-timestamp interleavings are never explored unless a fault reorders deliveries. DESIGN.md §3.3 `DecisionSource` unimplemented.
- **Fault model:** 9 kinds, 8 effective (ClockSkew offered-and-skipped, honestly trace-recorded, but drawn ~20% of generated plans — dilutes fault budgets). Partitions network-wide; no topology/bandwidth/per-node clocks; no provider/tool (LLM) fault family despite the agent-systems mission.
- **Legacy demo path caveat:** `demo`/`demo-buggy` drain their own fault plans and only `CrashRestart` takes effect; other recorded faults are inert there. Pinned, disclosed behavior (doctor asserts the demo stays legacy) — a compatibility fossil, not a hidden fake, but a naive reader could overestimate demo coverage.
- **Divergence detector epistemics:** pairwise replay agreement explicitly named a *sampled falsifier, never proof*, with a regression test demonstrating schedule-keyed evasion. Better stated than Antithesis's public docs state theirs.

## 5. ML / evaluation credibility

There is no ML in vibe-halt and none is claimed — stated directly per protocol. The evaluation machinery is the corpus recall gates: **real and exactly reproducible, but self-seeded** — the rig is measured against its own author's fault palettes (0 harvested real bugs vs a ≥25 acceptance metric). DESIGN.md's "multi-LLM sign-off ≥90%" artifacts are governance opinion honestly labeled Draft (2/7, one self-sign-off by the builder); they are not design validation and should never be cited as such. The anti-fabrication posture (private evidence fields closed at compile time, exact-exit-code negative gates, gate-held Python quarantine after a real prior fabrication) is genuine and load-bearing.

## 6. Code, tests, security, SRE/DevOps findings

Concrete defects (full detail Lane A §6):

- **D1 (medium)** — `make onboard` fails on stock macOS py3.9 (`tomllib` needs ≥3.11; `check_determinism_denylist.py:58`); no interpreter check; CI blind (ubuntu/py3.12).
- **D2 (low)** — CLAUDE.md architecture section stale: omits `vh-shrink`/`vh-verify`.
- **D3 (low)** — `~/.vibe-halt/` policy references a nonexistent writer; no durable evidence store exists (stdout only).
- **D4 (low, disclosed)** — DESIGN.md sign-off rule unmet at 2/7, one conflicted self-sign-off.
- **D5 (informational)** — DESIGN.md spec↔code gaps (PRF seed derivation, DecisionSource, temporal properties, exploration stages 2–5): aspiration, not description.
- **D6 (low, documented)** — ClockSkew no-op dilutes every generated plan's fault budget ~20%.
- **D7 (cosmetic)** — dead assignments in `workloads/corpus.rs:320,345`; drop-only `finish()` reads like a stub.

Security/supply-chain: zero external dependencies by design (lockfile binding, path-dep confinement, no `[patch]`/`.cargo`, enforced). Residual: deny-list regex layer is defense-in-depth by its own labeling; FNV-1a non-cryptographic (disclosed); all gates pin seed `0xD1CE` (single-seed determinism regressions would evade; mitigated by cross-platform verify soak). **Repo has no LICENSE file** (GitHub `licenseInfo: null`) — a real adoption/contribution blocker for an OSS-positioned rig.

SRE/DevOps: CI = `make gate` step-for-step (single implementation, verified `ci.yml:44-45`); verify.yml adds cross-platform replay soak with frozen receipts. No release packaging, no versioning of the CLI artifact, no dogfooded CI on PRs beyond the gate.

## 7. Dead, duplicated, superseded, or theatrical components

- **Phantom:** `~/.vibe-halt/` receipt store (policy without writer).
- **Orphaned:** `vh-shrink` (built, tested, unreachable from CLI).
- **Fossil:** legacy demo fault-application path (CrashRestart-only; pinned and disclosed, but misleading to demo viewers).
- **Theater risk (contained):** multi-LLM sign-off percentages — currently honest "Draft", one conflicted signature; must never become an admission gate.
- **Not theatrical (verified anti-theater):** Python client quarantine; empty-contract→UNCHECKED; exact-exit-code negative gates.

## 8. Capability gaps vs Antithesis-class DST (ranked, detailed in Lane A §8)

1. No coverage-/novelty-guided exploration (uniform-random Monte-Carlo only).
2. No schedule/interleaving search (fixed FIFO, no decision tape).
3. Target scope: in-process Rust only — mission targets untestable today.
4. Minimization unreachable (no `vh shrink`).
5. Corpus 100% self-seeded.
6. Fault model thin at edges (no topology/per-node clocks/provider faults; ClockSkew no-op).
7. No durable evidence store.
8. Environment fragility (`make onboard` broken on stock macOS).
9. Property language minimal (no temporal/at-most-once/reference-model).
10. Sequential single-machine runner (documented 2^20 universe bound, in-memory retention).

## 9. Commands and exact local evidence

Every command with exit codes and environment: `commands/` (make-onboard.txt, make-onboard-py311.txt, make-test.txt, make-gate.txt, cli-runs.txt, determinism-runs.txt, corpus-recall.txt, denylist-direct.txt, throughput-seed.txt). Lane A §3 carries the full table; `EVIDENCE_LEDGER.jsonl` carries per-claim modality/falsifiers.

## 10. Not verified by this audit

- CI green-ness at `84f911e` on GitHub runners (local equivalents all pass).
- Cross-platform bit-identity of frozen hashes (macOS arm64 only).
- vh-verify release soak receipt (`verify.yml:24` pinned; unit-tested in-crate; not run locally).
- Deny-list robustness against motivated macro/codegen adversaries beyond the script's own 18-sample bypass suite.
