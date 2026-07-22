# VIBE-HALT — CONVERGENCE CAMPAIGN EXECUTOR (long-running, adaptive)

> **Superseded for core execution upon human merge of C0 by `docs/prompts/VIBE_HALT_POST_AUDIT_TIER2_REACH_LONG_RUNNING_GOAL_2026-07-22.md`.** Retained as historical evidence; this supersedes execution authority only and grants no current core execution authority.

Authored 2026-07-22 against post-merge `main` (`2e47386`, PRs #12/#13/#14
landed), grounded in the 2026-07-22 whole-repo review and the audit at
`docs/audits/antithesis-dst-2026-07-21/`. You are a fresh, long-running
executor session on `AmitabhainArunachala/vibe-halt` — assume ZERO
memory. Every moving fact below (line numbers especially) must be
re-verified before you rely on it; this spec is the charter, the code is
the truth. The short `/goal` that dispatched you cites this file by path
and never overrides it.

This campaign CONTINUES the existing active tracks — Track 2
(`vibe-halt-1000x-exploration`) and the corpus track
(`vibe-bug-corpus-2026-07`) — it does not open a fourth track. The
human operator merging this spec ratifies the campaign charter; every
concrete ownership change still lands as an explicit
`docs/governance/ACTIVE_TRACK.yaml` diff in C0, human-merged (two-key).

## 0. State of the world (verify each; re-anchor and note drift if moved)

Run first: `make onboard`, then `make gate` (both must be green before
any edit; a red gate is a finding — report it, never route around).

- `main` = `2e47386` or later. `git log --oneline -6 origin/main` shows
  the #13/#14 merges; note drift in your first receipt.
- Frozen identities (verified 2026-07-22 post-merge, `make gate` exit 0):
  doctor trace `9ce6199f133f4d3c9dd0da0075e352d2` / 45 events / seed
  0xD1CE; observable fingerprint `1684e7c347e645f43a80a30abc46adb7`
  (`vh-doctor-observable-v3`). Unexplained drift = STOP, bisect, report.
- W1 kill criterion FIRED and is published: the swarm palette lost the
  bakeoff 0/5 classes over 16 seeds (PR #13; `corpus/PLAYBOOK.md`;
  `docs/audits/antithesis-dst-2026-07-21/EVIDENCE_LEDGER.jsonl`). All
  "guided exploration" claims are currently DEMOTED to unproven.
  `--palette v0` is default and must remain forever available.
- W2 substrate is merged but NOT wired: the sole live pop site
  `crates/vh-multiverse/src/runtime.rs:547` still reads
  `self.sched.pop()?`. The substrate exists:
  `vh_core::Scheduler::pop_recorded` (`crates/vh-core/src/sched.rs:121`)
  and `vh_trace::DecisionTape` (`crates/vh-trace/src/lib.rs:44`, schema
  `vh-decision-tape-v1` at `:109`, `digest_hex` at `:137`). The exact
  wiring is specified in the standing INTERFACE REQUEST:
  https://github.com/AmitabhainArunachala/vibe-halt/pull/13#issuecomment-5040630656
- ClockSkew is a no-op diluting fault budgets: offered-and-skipped by
  the v1 runtime (`crates/vh-multiverse/src/runtime.rs:37,713`),
  generated at `crates/vh-gremlin/src/lib.rs:315` (audit D6).
- `~/.vibe-halt/` is a phantom: `CLAUDE.md:27` declares it; no writer
  exists anywhere in the tree (audit finding; grep to confirm).
- Corpus: 5 seeded entries (`corpus/entries/VB-001..005`), recall gates
  anchored in `scripts/gate.sh:140-203`. ZERO harvested real bugs;
  the track's acceptance is ≥25 harvested from real AI-generated code.
- Governance: 3 ACTIVE tracks at `wip_max 3` (`vibe-halt-verify-2026-07`,
  `vibe-bug-corpus-2026-07`, `vibe-halt-1000x-exploration`); the broad
  core track is PAUSED (`docs/governance/ACTIVE_TRACK.yaml`).
- Radioactive surfaces: `crates/vh-verify/**` and `crates/vh-shrink/**`
  belong to the Codex verifier track. NEVER edit them. Cross-boundary
  needs are `INTERFACE REQUEST:` PR comments with exact signatures and
  cited call sites. Calling their PUBLIC API from your own surfaces is
  not an edit (see C5).
- `DESIGN.md:1-25` still demands 7-LLM ≥90% sign-offs; the ratified
  audit (D4) and Track-2 standing law §7 reject sign-off percentages as
  evidence. Doctrine conflict, unresolved (C7).

## 1. Mission

Converge every open thread from the 2026-07-22 review into merged,
gate-protected state: wire the decision tape (C1), run the surviving
half of the guided-exploration bet (C2), close the honesty gaps (C3,
C7), make findings durable artifacts (C4), wire the shrinker from the
boundary side (C5), and point the rig at real code (C6). You are
long-running and multi-PR: the campaign ends when every package is
merged, killed-with-published-disposition, or escalated-with-evidence —
not when the context gets long.

The strategic bet you are testing, not assuming: schedule diversity
(PCT over tape choice points) finds bugs that uniform FIFO cannot.
Palette diversity already lost its bakeoff; if schedule diversity loses
too, that null result — published — completes the falsification of the
audit's guided-exploration thesis and is worth as much as a win.

## 2. Standing law (binding all session, restated from Track 2)

1. Human-only merge authority. Never merge, never self-approve, never
   push to main. Push only to `claude/convergence-…` branches; draft PRs.
2. Citation-or-silence: every claim in receipts/PRs carries `file:line`
   or a runnable command + observed output. Uncited claims carry zero
   weight regardless of fluency.
3. `make gate` before every commit. Red gate = finding, never obstacle.
4. Determinism deny-list is the #1 law. Kernel crates stay pure. Any
   collision = design change or same-PR deny-list amendment with
   rationale, never a quiet workaround.
5. Frozen surfaces (`vh-core/src/rng.rs` output, `TRACE_FORMAT_V0.md`)
   are untouchable. Identity extensions go through NEW streams/fields.
6. Evidence epistemics: nothing you ship may make an unchecked thing
   look checked. Fallbacks (`--palette v0`, FIFO scheduling, stdout
   reports) remain forever available; new mechanisms are opt-in until
   their bakeoff/acceptance gate passes.
7. No multi-LLM sign-off percentages anywhere as evidence (audit D4).
8. One draft PR per work package; re-anchor on main after every human
   merge; never stack unmerged packages unless dependency-forced
   (C1→C2), and say so in the PR body.

## 3. Operating intelligence (how to think, not just what to do)

These are binding operating directives, in priority order:

- **Re-derive, never recall.** Every line number and identity in this
  spec is a snapshot; verify each at point of use. When this spec and
  the code disagree, the code wins and the drift goes in your receipt.
- **Connection-seeking mandate.** Before starting any package, read §5
  (standing couplings) and ask: what existing mechanism already carries
  half of this? What does this package unlock or invalidate elsewhere?
  A discovered coupling not listed in §5 is a first-class result —
  record it in the campaign ledger and exploit it instead of building a
  parallel mechanism. The repo's whole design (tape → replay → shrink →
  bundle → corpus) compounds; work WITH the compounding.
- **Adversarial self-check.** For every green result, attempt at least
  one falsification before claiming it: different process, different
  seed, reordered steps, deleted cache. Agreement you did not try to
  break is decoration, not evidence (the divergence detector's own
  epistemics — sampled falsifier, never proof — apply to you too).
- **Adaptive scheduling, never idle-blocking.** The dependency spine is
  C0 → C1 → C2; C3, C4, C5, C7 are independent of it; C6 runs whenever
  anything else is blocked. If a package waits on a human merge or an
  interface-request answer, switch to the highest-leverage unblocked
  package and record the switch in the ledger. There is always legal
  work: C6 harvesting is never blocked.
- **Negative results are first-class deliverables.** W1 set the
  precedent: a fired kill criterion is published, not routed around.
  Every kill firing gets a counterevidence entry in
  `docs/audits/antithesis-dst-2026-07-21/EVIDENCE_LEDGER.jsonl` in the
  same PR.
- **Escalate ambiguity, don't absorb it.** An architecturally
  significant fork (two defensible designs, different long-term costs)
  goes in the PR body as `OPEN QUESTION:` with your recommendation and
  a reversible default taken. Small ambiguity you resolve and cite.
- **Campaign ledger.** Maintain
  `docs/audits/antithesis-dst-2026-07-21/CONVERGENCE_LEDGER_2026-07-22.md`
  (append-only): per package — state, PR, evidence pointers, kill
  status, couplings discovered, switches taken. Per-package command
  receipts go beside the existing ones as
  `docs/audits/antithesis-dst-2026-07-21/commands/convergence-c<N>-<slug>.txt`.

## 4. Work packages

### C0 — Re-anchor + governance registration (hours; FIRST)
Verify every §0 fact mechanically. Then one PR against
`docs/governance/ACTIVE_TRACK.yaml`:
- Extend `vibe-halt-1000x-exploration` `next:` with C1–C3 and add the
  narrow surface `crates/vh-multiverse/src/runtime.rs` (pop-site wiring
  only), citing the standing interface request as the basis. This is
  the operator's answer to that request; comment on the interface
  request thread linking the PR so the loop closes in one place.
- Extend `vibe-bug-corpus-2026-07` `next:` with the C6 sprint target.
- Register the campaign ledger + receipt paths.
Acceptance: `make onboard` READY; governance self-test PASS; ownership
overlap gate clean; human merge ratifies.
Kill criterion: none (bookkeeping). If the human rejects the
`runtime.rs` grant, C1 stays an interface request and the spine
re-routes: C3/C4/C5/C6/C7 proceed, C2 blocks, say so in the ledger.

### C1 — Wire the decision tape (W2 completion; audit RFC-003; ~2 days)
Conditional on C0 merge. Implement exactly the posted interface
request: `runtime.rs:547` pop → `pop_recorded("runtime.step",
"fifo-v0", …)` recording into a `DecisionTape` carried on the universe
result path; tape digest surfaced as a SEPARATE printed line and
observable. FIFO order preserved bit-for-bit.
Acceptance (Track 2 W2): two processes, same seed → same tape digest;
doctor trace hash stays `9ce6199f…` and fingerprint stays `1684e7c3…`
(gate asserts both); gate gains an anchored tape-digest leak test;
runtime overhead ≤5% at the 200-universe demo (measure at the boundary
in `gate.sh` — wall clock never enters kernel crates).
Kill criterion: any frozen-identity drift or >5% overhead → tape goes
behind `--record-tape`, overhead numbers published, C2 proceeds with
the flag on.

### C2 — PCT + VB-006 (W3; requires C1; ~1 week)
Priority-permutation scheduling with change-point budget d over
same-timestamp choice points (Burckhardt ASPLOS 2010; reimplement,
attribute Shuttle `shuttle-schedulers/src/pct.rs@c8a46d3965` — shapes
only, zero dependencies). Build seeded class `VB-006 same-timestamp
race`: a bug INVISIBLE to FIFO v0 by construction — verify red on v0
FIRST and cite the run (v0 misses in 10,000 universes at pinned seed).
Acceptance: PCT d=3 finds VB-006 in ≤100 universes at pinned seed;
the finding replays byte-identically from `(seed, tape_digest)`; all
frozen identities unchanged; PCT is opt-in, FIFO stays default.
Kill criterion: PCT no faster than uniform-with-random-tiebreak over
32 seeds → drop PCT, KEEP the tape (it pays for itself as the
replay/causality substrate), publish the null result — this completes
the guided-exploration falsification and must be stated that way in
the closeout, PLAYBOOK, and evidence ledger.

### C3 — ClockSkew: implement or stop generating (W4/audit D6; small; independent)
Choose the smaller honest option: remove ClockSkew from
`FaultPlan::generate` offers (`crates/vh-gremlin/src/lib.rs:315`) OR
implement real observable virtual-clock divergence in the runtime.
Either way the fault-lifecycle ledger stops recording skips as budget
spent.
COUPLING WARNING (do not skip): changing generation changes every
fault plan drawn after the change point, so the five pinned corpus
recall behaviors (`scripts/gate.sh:140-203`) and any captured failure
fingerprints may shift. Re-run all five; if any pin moves, re-pin in
the same PR with before/after numbers and rationale — never silently.
Acceptance: generated plans contain no offered-and-skipped kinds OR
skew manifests in trace with measurable virtual-clock divergence;
corpus gates green (re-pinned and receipted if moved).
Kill criterion: none (honesty repair).

### C4 — Evidence store + replay bundles (audit R4; 3–5 days; independent)
`vh run --out <dir>` writes NDJSON receipts (run manifest, per-universe
outcomes, findings, trace hashes, tape digests when present);
`vh replay-bundle <dir>/<finding>` re-executes from the bundle alone;
CI replays a pinned bundle set. All I/O stays in boundary code
(`vh-cli`) — the deny-list is untouched. Retire the `~/.vibe-halt/`
phantom: implement the writer or amend `CLAUDE.md:27` in the same PR —
never both silent.
Acceptance (audit R4): copy a bundle out, delete the out dir parent,
`vh replay-bundle` reproduces the exact finding hash with no other
repo state; bundle digests stable across two runs; gate asserts it;
stdout remains the default.
Kill criterion: none (foundational); descope to stdout + artifact flag
only if review finds leak risks.

### C5 — Shrink wiring from the boundary side (audit R1 / Track-2 W5; 1–2 days)
The key connection Track 2's W5 missed: `vh-shrink`'s PUBLIC API
(`shrink`, `try_shrink`, `try_shrink_with_config`, `ShrinkReport` —
`crates/vh-shrink/src/lib.rs:582-651,126`) is callable from `vh-cli`
without editing a radioactive surface. Implement `vh run --shrink` and
`vh shrink --seed S --universe U` in `vh-cli` against the public API
only; the Cargo dependency addition rides the shared append-only
manifests. Post a courtesy comment to the verifier track that its
public API gained a consumer; anything requiring changes INSIDE
`vh-shrink/**` or `vh-verify/**` remains an `INTERFACE REQUEST:` with
exact signatures — never an edit.
Bind provenance: `ShrinkReport` deliberately carries no
source/build/workload/seed binding (PR #2's honest open contract) —
the C4 bundle manifest is the natural home. If C4 has landed, bind
shrink evidence into bundles and note the contract closure in the
ledger; if not, print the binding fields at the CLI boundary.
Acceptance (audit R1): `vh run --workload demo-buggy --seed 0xD1CE
--universes 100 --shrink` exits 1 and prints a shrunk plan with
strictly fewer injections that still replays to the SAME oracle
violation (exact captured fingerprint, not any-failure — cause
switching is a documented shrink hazard); gate gains an anchored
shrink line; frozen identities unchanged.
Kill criterion (audit R1): median shrink >60s at 100 universes → ship
the CLI without `--shrink` default, publish the bound, propose ddmin
budget changes as an interface request.

### C6 — Real-bug harvest sprint (corpus track; background thread, never blocked)
The rig's spine objective is `real-bugs-found`: it "earns its keep by
finding real defects in real (vibe-coded) code, not by passing its own
demos" (`docs/governance/ACTIVE_TRACK.yaml:23-26`). Corpus sits at
5/25, all seeded, zero harvested. Per `corpus/PLAYBOOK.md` +
`corpus/SCHEMA.md` typed admission: harvest candidate bugs from public
AI-generated code (repos, merged AI PRs, agent-framework issue
trackers), model each as a seeded workload + oracle, and admit ≥5 new
entries this campaign, each with a pinned one-command repro and an
anchored recall gate in `scripts/gate.sh`, each citing provenance
(source repo, PR/commit, defect description).
Whenever any other package blocks on a human, work here.
Acceptance: each entry passes corpus admission; its recall gate is
green; provenance cited; entries citing C4 bundles once C4 lands.
Kill criterion (audit R7): sustained harvesting yields <3 admissible
real bugs in 4 weeks → the rig's REALISM is falsified, not the corpus
process; STOP exploration work, report, and propose fidelity fixes
before any further guidance/exploration investment.

### C7 — Doctrine reconciliation (docs-only; anytime; small)
`DESIGN.md:1-25` (7-LLM ≥90% sign-off requirement) contradicts audit
D4 and standing law §7. PR: annotate the section as historical —
superseded by the audit's evidence doctrine — citing
`docs/audits/antithesis-dst-2026-07-21/` (do not delete the history;
do not touch the recorded sign-offs themselves).
`OPEN QUESTION:` for the operator in the PR body: annotate-in-place
(recommended) vs move to an archive file.
Kill criterion: none.

## 5. Standing couplings to exploit (the connection clause, made concrete)

- **Tape (C1) is the campaign's keystone**: C2 consumes its choice
  points; C4 binds its digests into bundles; the audit's R8 causality
  work rewinds on it later. Design every C1 surface with those three
  consumers in view — but ship only C1's acceptance.
- **C4 bundles ↔ C6 corpus**: audit R7's acceptance wants corpus
  `evidence` fields to be bundle digests that replay green in CI.
  Land C4 early and every C6 entry gets durable evidence for free.
- **C2's VB-006 ↔ C6**: a found same-timestamp race is itself a new
  corpus entry through the same admission gate — one artifact, two
  acceptance criteria.
- **C3 palette change ↔ corpus pins ↔ C5 fingerprints**: generation
  changes move fault plans, which can move pinned recalls AND captured
  failure fingerprints that C5's shrink oracle matches exactly.
  Sequence C3 before C5's fingerprint captures, or re-derive — never
  copy — any fingerprint that predates C3.
- **C5 ↔ PR #2's open contract**: `ShrinkReport`-lacks-provenance was
  declared honestly; C4+C5 together can close it. Closing a declared
  open contract is ledger-worthy.
- **Every kill firing ↔ EVIDENCE_LEDGER.jsonl**: same-PR
  counterevidence entry, Track-2 §5 precedent.

## 6. Cadence and closeout

- Branch prefix `claude/convergence-c<N>-<slug>`; draft PRs; human
  merges; re-anchor after each.
- After each package: update the campaign ledger, write the command
  receipt, run `make gate`, attempt one falsification of your own
  green result.
- Campaign closeout receipt (final ledger entry) answers with
  evidence: (a) per-package disposition — merged / killed-published /
  escalated, with numbers, not adjectives; (b) all frozen identities
  intact — paste the doctor output; (c) every kill criterion fired and
  what you did about each; (d) couplings discovered beyond §5;
  (e) the final status of the guided-exploration thesis after C2 —
  promoted, or falsification completed; (f) corpus count N/25 and the
  realism verdict; (g) updated maturity rows for every crate touched.

## 7. Explicitly out of scope

Hypervisor/process-level determinism (audit R1 rejection); RL-guided
exploration (R2 rejection); any external dependency (reimplement
shapes with attribution, 200–400 lines); eval dashboards; sign-off
percentages as evidence; Tier-2 sandbox expansion (Track 1's mission —
coordinate, don't collide); edits to `crates/vh-verify/**`,
`crates/vh-shrink/**`, or any radioactive surface; merging,
self-approval, or pushing to main.
