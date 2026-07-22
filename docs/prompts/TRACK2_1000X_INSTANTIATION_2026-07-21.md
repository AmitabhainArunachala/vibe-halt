# VIBE-HALT — TRACK 2: THE 1000x INSTANTIATION (guided exploration + finding depth)

> **Superseded for core execution upon human merge of C0 by `docs/prompts/VIBE_HALT_POST_AUDIT_TIER2_REACH_LONG_RUNNING_GOAL_2026-07-22.md`.** Retained as historical evidence; this supersedes execution authority only and grants no current core execution authority.

Authored 2026-07-21 by the Grand Orchestrator, grounded in the full
Antithesis-class audit at `docs/audits/antithesis-dst-2026-07-21/`
(pinned public commit `84f911e`, validator gate: 0 errors, 0 warnings).
You are a fresh Track-2 builder session on `AmitabhainArunachala/vibe-halt`
— assume ZERO memory; every moving fact below must be re-verified before
you rely on it. This document is the full spec; the short `/goal` that
dispatched you cites it by path and never overrides it.

## 0. State of the world (verify each; re-anchor if moved)

- `main` = `84f911e585b11e366c4185b5c301de6e088d07b7` or later. Run
  `git log --oneline -3 origin/main` and note drift in your first receipt.
- Frozen identities: doctor trace `9ce6199f133f4d3c9dd0da0075e352d2` /
  45 events / seed 0xD1CE; observable fingerprint
  `1684e7c347e645f43a80a30abc46adb7`. Unexplained drift = STOP, bisect,
  report. Your work must NOT change these (see §3 W2 for the one planned,
  separately-gated exception).
- The audit (`docs/audits/antithesis-dst-2026-07-21/`) reproduced on
  2026-07-21: `make gate` exit 0; byte-identical cross-process reruns;
  corpus recall 29/76/83/21/21 exact. Its `INTEGRATION_ROADMAP.md` is the
  source of every acceptance test and kill criterion below — read it in
  full before writing code. Do not re-litigate it; falsify it by building.
- Concurrent tracks are radioactive to you. Before touching anything,
  read `docs/governance/ACTIVE_TRACK.yaml` and register Track 2
  (`vibe-halt-1000x-exploration`) per wip_max law. Known radioactivity:
  `crates/vh-verify/**` and `crates/vh-shrink/**` are the Codex verifier
  track's surfaces (Track-1 prompt §0); `docs/prompts/TRACK1_*` sandbox
  work may own `crates/vh-multiverse/src/runtime.rs`. Cross-boundary needs
  are `INTERFACE REQUEST:` PR comments with exact signatures and cited
  call sites — never edits.
- Known live defects you inherit (audit `DHARMA_CODEBASE_AUDIT.md` §6):
  D1 `make onboard` broken on stock macOS py3.9; D6 ClockSkew no-op
  diluting ~20% of generated fault budgets; `~/.vibe-halt/` is a phantom
  (no writer exists). Do not "fix" D6 silently — it is W4.

## 1. Mission

Convert vibe-halt's exploration from uniform Monte-Carlo into guided
search, and its findings from stdout text into minimized artifacts —
the first two multipliers of the 1000x path — without breaking a single
frozen identity, the zero-dependency law, or the deny-list. You are NOT
building a hypervisor, an eval dashboard, or an RL system (audit
FIRST_PRINCIPLES §1 rejections R1–R3 are binding).

The strategic bet you are testing, not assuming: swarm masks + schedule
choice points beat uniform random ON THIS RIG'S OWN CORPUS. If the
bakeoff says no, that is a publishable finding, not a failure to route
around.

## 2. Standing law (restated; binding all session)

1. Human-only merge authority. Never merge, never self-approve, never
   push to main. Push only to `claude/track2-…` branches; draft PRs.
2. Citation-or-silence: every claim in receipts/PRs carries `file:line`
   or a runnable command + observed output. Uncited claims carry zero
   weight.
3. `make gate` before every commit. A red gate is a finding — report,
   never route around.
4. Determinism deny-list is the #1 law. Kernel crates stay pure. Any
   collision = design change or same-PR deny-list amendment with
   rationale, never a quiet workaround.
5. Frozen surfaces (`vh-core/src/rng.rs` output, `TRACE_FORMAT_V0.md`)
   are untouchable. Universe-identity changes go through NEW streams/
   fields, never edits to v0 formats.
6. Evidence epistemics: nothing you ship may make an unchecked thing
   look checked. Uniform-random fallback (`--palette v0`) must remain
   forever available; guidance is opt-in until its bakeoff gate passes.
7. No multi-LLM sign-off percentages anywhere as evidence. They are
   labeled opinion at best (audit D4).

## 3. Work packages (in order; each ships behind its own gate)

### W0 — Housekeeping (audit R0; hours)
- Propose LICENSE via PR (Apache-2.0 recommended — matches every donor;
  audit found `licenseInfo: null`). Decision is the human's; your PR
  presents it, does not merge it.
- Fix D1: `scripts/check_determinism_denylist.py` and `scripts/onboard.py`
  fail fast with "Python >= 3.11 required" on older interpreters.
- Fix D2: CLAUDE.md crate map gains `vh-shrink` + `vh-verify`.
Acceptance test: `make onboard` on a py3.9 host exits 2 with the
explicit message (or 0); `gh repo view --json licenseInfo` non-null
after human merges LICENSE.
Kill criterion: none (defect repair).

### W1 — Swarm palette (audit R2; ~1 day + bakeoff; FIRST TOUCH: `crates/vh-gremlin`)
`FaultPlan::generate` gains an opt-in `--palette swarm`: per-universe
mask drawn from `universe_seed ^ fnv1a64("palette")` disabling a random
subset of fault kinds and wildly reweighting the rest — TigerBeetle
`random_enum_weights` idiom (attribute in comment:
`tigerbeetle@97c7a8ef38 src/testing/fuzz.zig`). `--palette v0` stays
default and bit-identical.
Acceptance test: seeded A/B harness in `scripts/` (initially
non-blocking): `--palette swarm` reaches each pinned corpus recall
(29/76/83/21/21 ± tolerance) in ≤25% of the universe-executions of
`--palette v0` on ≥4/5 seeded classes, first-detection universe index,
16 seeds. Only after it passes may a separate PR flip the default.
Kill criterion: swarm fails to beat v0 on ≥3/5 classes over 16 seeds →
keep v0 default, publish the negative result in `corpus/PLAYBOOK.md`.
A null result here demotes ALL "guided exploration" claims in the audit
roadmap to unproven — say so in your closeout receipt.

### W2 — Decision tape (audit RFC-003; ~1 week; touches `vh-core` scheduler + `vh-trace`)
Every scheduler pop site becomes a named decision point recording
`(site_id, candidate_set_digest, chosen_index, policy_id)` onto a NEW
trace stream (TRACE_FORMAT stays v0 — additive stream, not a format
change). FIFO v0 = tape with constant policy. Replay verifies
tape-vs-execution equivalence. Universe identity extends to
`(seed, tape_digest)` as a SEPARATE printed line; existing doctor
fingerprint and frozen identities MUST NOT change (gate asserts this).
Acceptance test: two processes, same seed → same tape digest; doctor
trace hash stays `9ce6199f…` and fingerprint stays `1684e7c3…`; leak
test in gate.
Kill criterion: tape recording changes any frozen identity or adds >5%
runtime overhead at 200-universe demo → flag-off, ship recording-only
behind `--record-tape`, report overhead numbers.

### W3 — PCT schedule perturbation (audit R3; ~1 week; requires W2)
Priority-permutation strategy with change-point budget d over
same-timestamp choice points (Burckhardt ASPLOS 2010, via Shuttle
`shuttle-schedulers/src/pct.rs@c8a46d3965`, Coyote-derived — attribute).
New seeded bug class `VB-006 same-timestamp race`: a workload whose bug
is INVISIBLE to FIFO v0 by construction (verify red on v0 first, cite
the run).
Acceptance test: PCT d=3 finds VB-006 in ≤100 universes at pinned seed
(verify v0 does not in 10,000); finding replays byte-identically from
`(seed, tape_digest)`.
Kill criterion: PCT no faster than uniform-with-random-tiebreak over 32
seeds → drop PCT, keep the tape (it pays for itself as replay/causality
substrate), document the null result.

### W4 — ClockSkew: implement or stop generating (audit D6; small)
Either implement real per-node clock offsets in the v1 runtime or remove
ClockSkew from `FaultPlan::generate`'s palette (it currently no-ops at
~20% draw — `runtime.rs:713-719`, `vh-gremlin/src/lib.rs:124`). Whichever
you choose, the fault-lifecycle ledger must stop recording skips as if
they were budget spent. Choose the smaller honest option.
Acceptance test: generated plans contain no offered-and-skipped kinds,
OR ClockSkew manifests in trace with measurable virtual-clock divergence.

### W5 — Shrink wiring (audit R1) — INTERFACE REQUEST ONLY
`crates/vh-shrink/**` is the Codex track's surface. Post an
`INTERFACE REQUEST:` PR comment proposing `vh shrink --seed S --universe U`
+ `vh run --shrink` with the exact acceptance test from audit R1 (shrunk
plan, strictly fewer injections, same oracle violation at
`--seed 0xD1CE --universe 2`). Do NOT edit vh-shrink yourself unless the
human reassigns ownership in ACTIVE_TRACK.yaml.

## 4. Explicitly out of scope (audit "what not to build")

Hypervisor/process-level determinism; RL-guided exploration; dependencies
on Stateright/Turmoil/Shuttle (shapes only, 200–400 line reimplementations
WITH attribution); agent-eval dashboards; Tier-2 sandbox code (Track 1's
mission — coordinate, don't collide); corpus harvesting (blocked on W2+).

## 5. Receipts and closeout

- One receipt per work package under the repo's receipt convention:
  commands + exit codes + anchored output lines, every claim cited.
- Final closeout receipt answers, with evidence: (a) did swarm beat v0 —
  numbers, not adjectives; (b) are all frozen identities intact — show
  the doctor output; (c) which kill criteria fired and what you did about
  each; (d) updated maturity table rows for vh-gremlin/vh-core/vh-trace.
- If you falsify any audit claim while building, that is a FIRST-CLASS
  result: amend `docs/audits/antithesis-dst-2026-07-21/EVIDENCE_LEDGER.jsonl`
  with a counterevidence entry in the same PR.
