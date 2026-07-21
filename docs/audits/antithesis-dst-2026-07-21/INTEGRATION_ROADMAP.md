# Integration Roadmap

Sequenced by leverage-per-effort on the verified kernel. Scores: impact/confidence/effort/risk/reversibility, each explained — no weighted-total false precision. Every major item carries an executable acceptance test and a kill criterion.

## Immediate (this week)

### R0 — Housekeeping that unblocks everything
- Add a LICENSE (Apache-2.0 or MIT — matches every donor project; currently none, blocking the OSS-niche strategy).
- Fix D1: `scripts/check_determinism_denylist.py` + `scripts/onboard.py` fail fast with a clear message on Python <3.11 (or vendor a tomllib fallback).
- Fix D2: CLAUDE.md crate map adds `vh-shrink`/`vh-verify`.
- Impact: medium (removes first-contact failure + legal blocker). Confidence: high (all reproduced). Effort: hours. Risk: none. Reversibility: total.
Acceptance test: `make onboard` exits 0 on stock macOS python3.9 host OR exits 2 with an explicit "Python >= 3.11 required" message; `gh repo view --json licenseInfo` non-null.
Kill criterion: none (pure defect repair).

### R1 — `vh shrink` CLI wiring (M4a)
Wire existing, tested `vh-shrink` ddmin into the CLI: `vh run --shrink` auto-shrinks each finding; `vh shrink --seed S --universe U` minimizes a replay.
- Impact: high (I5 completion; findings become actionable). Confidence: high (library + integration tests exist and pass). Effort: 1–2 days (CLI + report plumbing). Risk: low (no kernel change). Reversibility: flag-off.
Acceptance test: `cargo run -p vh-cli -- run --workload demo-buggy --seed 0xD1CE --universes 100 --shrink` exits 1 and prints a shrunk fault plan with strictly fewer injections that still replays to the same oracle violation (`--seed 0xD1CE --universe 2` path); gate gains an anchored shrink-gate line.
Kill criterion: shrinking any corpus finding takes >60s median at 100 universes → ship CLI without `--shrink` default and reassess ddmin bounds.

### R2 — Swarm masks (M2a, the 20-line multiplier)
Per-universe randomized fault-family/palette mask in `vh-gremlin::FaultPlan::generate` (TigerBeetle `random_enum_weights` idiom, attributed): each universe disables a random subset of fault kinds and wildly reweights the rest. Palette stays frozen-compatible via an opt-in flag first, default after gate bakeoff.
- Impact: high (exploration diversity ×; zero new concepts). Confidence: high (production precedent observed in TigerBeetle `fuzz.zig`). Effort: ~1 day + gates. Risk: medium (changes default exploration — freeze old behavior behind `--palette v0`). Reversibility: flag.
Acceptance test: seeded A/B in CI: `vh run --workload corpus-* --palette swarm` reaches each pinned recall (29/76/83/21/21 ± tolerance) in ≤25% of the universe-executions of `--palette v0` on at least 4/5 seeded classes, measured by first-detection universe index over 16 seeds.
Kill criterion: swarm palette fails to beat v0 on ≥3/5 classes over 16 seeds → revert to v0 default, publish the negative result in corpus PLAYBOOK (a real finding about guidance on this workload class).

## 30 days

### R3 — Decision tape + PCT schedule perturbation (M2b)
Scheduler gains choice points at same-timestamp events; every choice recorded on a chain-hashed decision tape; replay re-executes the tape; a PCT strategy (Burckhardt 2010, via Shuttle's Coyote-derived implementation notes, attributed) perturbs priorities with change-point budget d. Universe identity extends to `(seed, tape digest)`.
- Impact: very high (first real schedule search; converts the documented schedule-keyed evasion from a blind spot into explored space). Confidence: medium-high (PCT guarantee is published math; integration with watermark scheduler is the unknown). Effort: 1–2 weeks. Risk: medium (touches frozen-adjacent surfaces; trace format stays v0 — tape is a new stream, not a format change). Reversibility: strategy flag; FIFO remains default until bakeoff.
Acceptance test: a new seeded bug class `VB-006 same-timestamp race` (invisible to FIFO v0 by construction, verified red on v0) is found by PCT at d=3 in ≤100 universes at pinned seed; tape replay of the finding is byte-identical; doctor fingerprint gains tape digest without changing existing frozen identities.
Kill criterion: PCT finds VB-006 no faster than uniform-with-random-tiebreak over 32 seeds → drop PCT, keep the decision tape (it pays for itself as the replay/causality substrate), document the null result.

### R4 — Evidence store + regression corpus (M4c)
Real `--out <dir>` NDJSON receipts (run manifest, per-universe outcomes, findings, trace hashes, tape digests); `vh replay-bundle <dir>/<finding>` re-executes from the bundle alone; CI replays a pinned bundle set. Retires the `~/.vibe-halt/` phantom by making the policy true (or amends CLAUDE.md — never both silent).
- Impact: high (durability = I5; enables corpus harvesting and trend lines). Confidence: high (trace format already injective; pure addition). Effort: 3–5 days. Risk: low. Reversibility: stdout remains default.
Acceptance test: `rm -rf` the out dir parent, then `vh replay-bundle` from a copied bundle reproduces the exact finding hash with no other repo state; gate asserts bundle digests are stable across two runs.
Kill criterion: none (foundational plumbing); descope to stdout+artifact flag only if review finds leak risks.

### R5 — Provider/tool gremlin family + cassette boundary spike (M3a)
New fault kinds for the agent mission: LLM 429/500, malformed JSON tool response, truncated context, provider latency spikes — injected at a cassette replay boundary. Cassettes: recorded request/response pairs with digest identity; checker runs at-most-once/conservation over the history (Jepsen-style). Spike scope: one demo agent workload with a cassette-backed fake provider.
- Impact: very high (first mission-relevant target class; unblocks real corpus harvesting). Confidence: medium (cassette determinism is well-precedented — vcrpy et al.; the honesty boundary — when is a cassette stale relative to a live provider — is the real design problem). Effort: 2 weeks spike. Risk: medium-high (scope creep into "eval framework" — guard: no LLM-as-judge, ever). Reversibility: separate crate `vh-cassette`.
Acceptance test: seeded agent bug `VB-007 retry-double-apply-under-429`: a demo agent that double-executes a tool call when the provider 429s mid-retry is caught at pinned seed with the provider gremlin enabled and never without it, across 16 seeds; cassette digest mismatch → UNCHECKED (not CLEAN, not FINDINGS).
Kill criterion: cassette determinism requires giving up deny-list purity in a kernel crate, or the fake-provider fidelity debate has no resolution after 2 weeks → stop; agent-arm becomes black-box history checking over recorded real runs only (Jepsen arm), no simulation claim.

## 90 days

### R6 — Tier-2 subprocess universes (the active goal doc, executed)
Per `docs/prompts/TRACK1_PHASE1_SANDBOX_GOAL_2026-07-21.md`: child-process universes with controlled stdin/stdout/fs/clock and seeded entropy; leak → UNCHECKED fail-closed. This audit adds two requirements the goal doc should absorb: (a) decision-tape equivalence at the shim boundary, or nondeterminism is declared, not hidden; (b) a documented fidelity matrix (what is simulated vs real) shipped with every Tier-2 run manifest.
- Impact: very high (mission reach). Confidence: medium (process-boundary determinism without interposition is the core research risk — R1 rejection's falsifier lives here). Effort: 4–8 weeks. Risk: high (silent leaks = false CLEAN — the worst outcome this repo's epistemics exist to prevent). Reversibility: Tier-1 unaffected.
Acceptance test: leak-detection battery (spawn a workload that reads wall clock, /dev/urandom, unseeded HashMap iteration, env, real socket) is detected and reported UNCHECKED with the leaking boundary named; a compliant demo subprocess workload reproduces byte-identically across two processes.
Kill criterion: any silent nondeterminism leak found post-gate in the leak battery → Tier-2 marked experimental-only, verdict capped at UNCHECKED until the boundary class is closed.

### R7 — Corpus harvesting pipeline
Harvest ≥10 (toward ≥25) real bugs: dogfood on dharma_swarm (`clients/python` Phase-4 hook) and 2–3 OSS vibe-coded repos; entries cite replay bundles (R4) and pass the §5 typed admission gate (Reproduced + PromotionProof only).
Acceptance test: `python3 corpus/validate.py` passes with ≥10 entries whose `evidence` fields are bundle digests that replay green in CI.
Kill criterion: 4 weeks of harvesting yields <3 real bugs → the rig's realism is falsified, not the corpus process; revisit M3 fidelity before further exploration work.

### R8 — Causality analysis (M4b)
Rewind on decision tape to prefix P, re-explore forward N times with perturbed tapes, emit bug-probability-over-time per finding (Antithesis's published algorithm on vibe-halt's artifact; attribution in docs).
Acceptance test: on `VB-001 lost-update`, the causality graph shows a sharp probability jump at the fault-plan injection that the shrinker retains (cross-validation of two independent mechanisms); artifact emitted to evidence store.
Kill criterion: causality output disagrees with shrinker minimality on >50% of corpus findings → one of the two is wrong; stop and find out which before shipping either as "root cause".

## Longer horizon (post-90d, gated on R5–R7 evidence)

- **Parallel campaigns** (multi-core fan-out, budget allocation by novelty) — current runner is fast but sequential; DESIGN.md §7's 100k soak.
- **Assertion-novelty guidance** (sometimes-hit bitmaps feeding mask allocation) — only after R2/R3 ablations show guidance headroom remains.
- **Temporal property DSL** (`after(A).within(t,B)`, at-most-once, reference-model) — Stateright Expectation shapes; only when harvested corpus demands them.
- **OSS positioning**: LICENSE + public corpus + honest bakeoff vs Turmoil/MadSim published; Antithesis-style OSS free-tier analog is premature until R7.
- **Not on any horizon**: hypervisor (R1 rejection), RL exploration (R3 rejection), eval dashboards, multi-LLM sign-off gates.

## First three implementation-ready spike specs

### RFC-001 — `vh shrink` CLI (R1)
- Surface: `vh run --shrink[=budget]`; `vh shrink --seed S --universe U [--max-oracle-calls N]`.
- Internals: call `vh_shrink::shrink` per finding via the existing `run_universe_with_fault_plan` replay hook (`tests/multiverse_integration.rs:87` pattern); report prints original→shrunk injection counts + replay hash equality.
- Gates: anchored shrink-gate in `gate.sh` (exit 1 + `shrunk: a→b injections` line); frozen identities unchanged (shrinker consumes the tape, doesn't alter it).
- Migration: none. Rollback: remove flag.

### RFC-002 — Swarm palette (R2)
- Surface: `vh run --palette {v0,swarm}`; default v0 until bakeoff gate passes, then flip default in one PR.
- Internals: `FaultPlan::generate` gains `PaletteMask` drawn from the universe seed stream `universe_seed ^ fnv1a64("palette")`; mask = random subset disable + Dirichlet-ish reweight of remaining kinds (TigerBeetle `random_enum_weights` semantics, attributed in comment).
- Gates: A/B bakeoff harness in `scripts/` with the R2 acceptance test as a CI-reportable (initially non-blocking) job; frozen palette v0 untouched.
- Rollback: default flag flip.

### RFC-003 — Decision tape (R3a, prerequisite of PCT and causality)
- Surface: trace gains a `decision` record kind (new stream, TRACE_FORMAT stays v0 — tape is additive); `vh run --record-tape`; replay verifies tape-vs-execution equivalence.
- Internals: scheduler pop sites become named decision points `(site_id, candidates)`; FIFO v0 = tape with constant policy; divergence detector cross-checks tape digests.
- Gates: byte-identity of existing doctor fingerprint preserved; new tape digest added as a separate printed line; leak test: two processes, same seed, same tape digest.
- Rollback: tape recording flag-off; replay path ignores tapes.

## Dependency order

R0 → R1, R2 (parallel) → R4 → R3 (tape before PCT) → R5 (cassette spike, parallel with R3) → R6, R7 → R8. R2/R3 ablation results gate all "guided exploration" claims; R5/R6 results gate all "agent-system testing" claims; R7 results gate the OSS-positioning story.

## Acceptance test and kill criterion index

Every item above carries a machine-checkable `Acceptance test:` line and a `Kill criterion:` line at line start (R0–R8). The three implementation-ready spikes (RFC-001/002/003) inherit the acceptance tests of their parent items (R1, R2, R3).
