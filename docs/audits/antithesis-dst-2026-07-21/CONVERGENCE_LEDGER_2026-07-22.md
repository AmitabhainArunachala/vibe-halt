# Convergence Campaign Ledger — 2026-07-22 (append-only)

Charter: `docs/prompts/CONVERGENCE_CAMPAIGN_EXECUTOR_2026-07-22.md` (merged
via PR #15, `e36b15b`). Executor: long-running Claude session under Track 2
(`vibe-halt-1000x-exploration`) + corpus track (`vibe-bug-corpus-2026-07`).
Standing law: human-only merge; citation-or-silence; `make gate` before
every commit; determinism deny-list; frozen surfaces untouchable.

Per-package receipts: `commands/convergence-c<N>-<slug>.txt` beside this file.

---

## 2026-07-22 — Campaign open: §0 state-of-the-world verification

All §0 facts re-derived mechanically at point of use. Receipt:
`commands/convergence-c0-reanchor.txt`.

| §0 fact | Verified | Evidence |
|---|---|---|
| main ≥ `2e47386`, #13/#14 merged | YES, drift noted | `git log --oneline -8 origin/main`: head `e36b15b` (#15 = this campaign's charter merge), `2e47386` (#14), `e2db597` (#13) |
| Frozen doctor trace `9ce6199f133f4d3c9dd0da0075e352d2` / 45 events / seed 0xD1CE | YES | `make gate` (exit 0): "replay check: OK (universe 0 hash 9ce6199f133f4d3c9dd0da0075e352d2 events 45)" |
| Observable fingerprint `1684e7c347e645f43a80a30abc46adb7` (`vh-doctor-observable-v3`) | YES | same `make gate` run: "observable fingerprint: OK" |
| W1 kill fired & published, palette claims DEMOTED | YES | `docs/audits/antithesis-dst-2026-07-21/EVIDENCE_LEDGER.jsonl` claim `VH-TRACK2-NULL-001` (0/5 classes, 16 seeds); `--palette v0` default at `crates/vh-cli/src/main.rs:93` (`palette: FaultPalette::V0`) |
| W2 substrate merged, NOT wired | YES | `crates/vh-multiverse/src/runtime.rs:547` = `let (at, ev) = self.sched.pop()?;`; `pop_recorded` at `crates/vh-core/src/sched.rs:121`; `DecisionTape` at `crates/vh-trace/src/lib.rs:44`, schema `vh-decision-tape-v1` at `:109`, `digest_hex` at `:137` |
| Standing INTERFACE REQUEST | YES | PR #13 comment 5040630656 (2026-07-22T00:57:49Z, OWNER) — exact `pop_recorded("runtime.step", "fifo-v0", …)` wiring |
| ClockSkew offered-and-skipped | YES | `crates/vh-multiverse/src/runtime.rs:37` (doc), `:713` (`FaultKind::ClockSkew` → `fault.skipped`); generated at `crates/vh-gremlin/src/lib.rs:315` |
| `~/.vibe-halt/` phantom | YES | declared `CLAUDE.md:27`; `grep -rn "\.vibe-halt"` over `*.rs,*.py,*.sh,Makefile` → zero writers |
| Corpus 5 seeded / 0 harvested | YES | `corpus/entries/VB-001..VB-005`; recall gates `scripts/gate.sh:140-203` |
| Governance: 3 ACTIVE, wip_max 3 | YES | `make onboard`: READY; self-test PASS (4 bypass reproductions, 10 schema cases) |
| DESIGN.md doctrine conflict | YES | `DESIGN.md` "Review & Sign-off": ≥7 frontier LLMs, ≥90% confidence — contradicts audit D4 / standing law §7 (→ C7) |

**Drift notes:** (1) main advanced from `2e47386` to `e36b15b` — the delta
is PR #15 (this campaign's own charter), no code drift. (2) All charter
line anchors re-verified exact; none moved.

---

## C0 — Re-anchor + governance registration

- **State:** PR open (draft), awaiting human merge.
- **Diff:** `docs/governance/ACTIVE_TRACK.yaml` — (a) Track 2 `next:`
  extended with C1/C2/C3 + campaign ledger/receipt registration;
  (b) Track 2 `owned_surfaces:` gains `crates/vh-multiverse/src/runtime.rs`
  (pop-site wiring only), citing the standing interface request as basis;
  (c) corpus track `next:` extended with the C6 sprint target + R7 kill.
- **Evidence:** receipt `commands/convergence-c0-reanchor.txt`; `make
  onboard` READY + governance self-test PASS + ownership-overlap gate clean
  on the branch; `make gate` ALL PASS pre-commit.
- **Kill status:** none defined (bookkeeping). If the human rejects the
  `runtime.rs` grant: C1 stays an interface request, C2 blocks, C3/C4/C5/C6/C7
  proceed.
- **Couplings discovered:** none beyond §5 yet.
- **Loop closure:** PR link posted as a reply on the interface-request
  thread (PR #13 comment 5040630656) so the request and its answer live in
  one place.

---

## 2026-07-22 — Wave-1 closeout: every package executed to its two-key limit

All solo-executable work is DONE; every package is now merged-pending
(draft PR escalated to the operator with evidence) or blocked by the
two-key law itself. Per-package disposition, numbers not adjectives:

| Pkg | Disposition | PR | Evidence |
|---|---|---|---|
| C0 | escalated-with-evidence (draft, CI 6/6 green) | #16 | this ledger; receipt convergence-c0-reanchor.txt; onboard READY; overlap gate clean |
| C1 | blocked-by-design: needs #16's ratified runtime.rs grant (two-key). Spec ready: PR #13 comment 5040630656 | — | interface-request thread carries the C0 answer link |
| C2 | blocked on C1 (charter spine C0→C1→C2) | — | — |
| C3 | escalated-with-evidence (draft, CI 6/6 green) | #18 | implement-not-remove derivation; 0 pins moved; falsifier: 5-workload byte-identical stdout vs main; receipt convergence-c3-clockskew.txt |
| C4 | escalated-with-evidence (draft) | #19 | delete-parent standalone replay REPRODUCED; tamper-negative exit 1; byte-stable receipts (diff -r); receipt convergence-c4-evidence-store.txt |
| C5 | escalated-with-evidence (draft) | #20 | MINIMIZED 3->1 @ demo-buggy 0xD1CE; kill margin ~60x (0.4s vs 60s); verifier-track courtesy comment 5041929439; receipt convergence-c5-shrink-boundary.txt |
| C6 | ACCEPTANCE MET, escalated-with-evidence (draft) | #21 | 5/5 harvested (VB-007..011: recalls 91,96,79,70,58 of 100 @0xD1CE); corpus 10/25; five distinct fault families; R7 kill NOT fired; receipt convergence-c6-harvest-1.txt |
| C7 | escalated-with-evidence (draft) | #17 | annotate-in-place; OPEN QUESTION in PR body; receipt convergence-c7-doctrine.txt |

Frozen identities on every branch, every gate run: doctor
9ce6199f133f4d3c9dd0da0075e352d2 / 45 events; fingerprint
1684e7c347e645f43a80a30abc46adb7 (vh-doctor-observable-v3).

Kill criteria fired this wave: VB-010's first palette (guaranteed >=1
crash, 100/100) tripped the anti-gaming rule -> widened 0..=2,
re-pinned 70/100, published in the entry + PR #21 (same-PR
counterevidence discipline). C5 and C6 kill criteria measured and NOT
fired. No frozen-identity drift anywhere.

Couplings discovered beyond §5 (first-class results):
1. Runtime-path workloads (disk/net/corpus) retrieve their fault plan
   INSIDE UniverseCtx::runtime and never hold it — boundary-side shrink
   capture (C5) cannot reach them without a small kernel API (a
   plan-capture hook or a runtime() variant returning the retrieved
   plan). Future INTERFACE REQUEST; typed exit-2 diagnostic meanwhile.
2. The held-reorder expiry semantic (a reorder with no following send
   expires its captive) makes NetworkReorder a LOSSY fault at
   stream-end — workloads wanting loss-free reorder palettes need an
   eos-trailer idiom (VB-011 implements it; reusable pattern).
3. demo-buggy universe 0 shares trace hash 9ce6199f… with demo's
   doctor universe (no crash drawn -> identical trace) — a useful
   cross-workload identity witness, and a reminder that trace hashes
   identify EXECUTIONS, not workload variants.

Switches taken (adaptive scheduling): C0 wait -> C7 -> C3 -> C4 -> C5
-> C6 (increments 1-5). C6 was never blocked, as the charter demands.

Next wave (auto-resumes on merge events / armed check-in): re-anchor,
rebase later-merging PRs (known trivial conflicts: ACTIVE_TRACK.yaml
#16/#18; gate.sh + cli_contract.rs + workloads/mod.rs among
#19/#20/#21), then C1 the moment the grant is ratified, then C2 — the
guided-exploration thesis' final disposition (promoted or
falsification-completed) rides on C2's bakeoff.

---

## 2026-07-22 — Merge wave 1 + C1 (decision tape) executed

- MERGED by operator: C0 (#16, 55e806d — the runtime.rs grant is
  ratified) and C7 (#17, 947dba9). Dispositions upgraded merged.
- #18 (C3) rebased onto post-merge main (ACTIVE_TRACK.yaml conflict
  resolved, runtime.rs surface line deduped; C3 grant comment now
  says EXTENDS the C0 grant) and hardened per two independent review
  findings (Devin + Codex, same defect): a generated zero-magnitude
  ClockSkew could claim Injected+Manifested with no divergence —
  fixed in df10304 (zero skews honestly stay Armed), regression test
  added, reviewer confirmed resolved.
- C1 EXECUTED on the ratified grant (branch
  claude/convergence-c1-decision-tape): pop site wired exactly per the
  interface request; digest bound as a new observable through the
  UniverseObservation compile-time ratchet (which fired as designed).
  KILL CRITERION FIRED: recording costs ~+48% release / ~+90% debug at
  the 200-universe demo-net demo (>5%); per the charter the tape went
  behind `vh run --record-tape`, the default arm is the original pop
  bit-for-bit (release default 12ms vs main 13-15ms — no measurable
  overhead), numbers published in
  commands/convergence-c1-decision-tape.txt. C2 proceeds with the flag
  on. Gate gained the two-process tape-agreement gate and the
  tape-leak negative (default + legacy runs must print no tape;
  frozen demo identity asserted under the flag).
- Coupling note for C2/C4: the tape digest rides UniverseResult now;
  C4's run.ndjson/bundles gain the field additively at their rebase.

---

## 2026-07-22 — C2 (PCT + VB-006) executed: KILL CRITERION FIRED, null published

Branch `claude/convergence-c2-pct` off fb0ca58 (#22 merged — C1→C2 is
the campaign's only sanctioned stack, dissolved by merge). Full
numbers: commands/convergence-c2-pct.txt.

- Wired: `Scheduler::pop_chosen` (vh-core sched.rs) + `PctStrategy` /
  `UniformTiebreakStrategy` (vh-core strategy.rs, NEW — Burckhardt
  ASPLOS 2010; Shuttle pct.rs@c8a46d3965 shapes only, zero deps);
  `SchedulePolicy` threading through vh-multiverse; CLI
  `--schedule fifo|pct:<d>|uniform` (opt-in, fifo default byte-for-byte
  — falsified explicitly: default vs `--schedule fifo` on demo/50u is
  byte-identical). Fail-closed: non-FIFO `--schedule` conflicts with
  `--shrink`/`--out` (their replay paths carry no schedule policy).
- VB-006 `corpus-same-timestamp-race` seeded red-by-construction:
  invisible to FIFO v0 over 10,000 universes (CLEAN, exit 0); PCT d=3
  finds it at universe 0, 76/100 failing, divergent=0.
- Repro-honesty defect found+fixed in-package: printed repro lines
  omitted the schedule flag — a PCT finding's repro silently replayed
  CLEAN under FIFO. Repro now carries `--schedule …`; pinned by a
  contract test that EXECUTES the printed repro (exit 1 + FINDINGS).
- KILL FIRED: 32-seed bakeoff (budget 1000/seed, first-failing-universe
  metric, run twice, identical): median_pct=0.0 median_uniform=0.0
  pct_wins=0 losses=8 ties=24 → `pct_faster_than_uniform=false`.
  Cross-check: uniform alone finds VB-006 in 96/100 universes vs PCT's
  76 — PCT's change points sometimes RESTORE FIFO-like order on
  depth-1 bugs. Disposition per charter: PCT-inspired scheduling
  dropped as investment, kept in-tree opt-in as the reproducible
  falsification harness (W1 precedent); tape kept; null published in
  corpus/PLAYBOOK.md + EVIDENCE_LEDGER VH-TRACK2-NULL-002 (same PR).
  CLAIM NARROWED post-review (Codex audit C.1, issue #24): the metric
  was saturated (uniform hits VB-006 at ~98.4%/universe — a floor
  effect), so the null is NARROW (no advantage on this
  workload/metric), not a general falsification; revival falsifier: a
  depth≥2 class with low uniform hit rate where the metric can
  discriminate.
- Gates added (scripts/gate.sh): VB-006-invisible-to-FIFO (exit 0,
  CLEAN, 2000u), PCT-finds-within-100 (exit 1, anchored oracle),
  PCT-replay-byte-identical across two processes. `make gate` ALL
  PASS; `make test` 32 binaries 0 failed; clippy -D warnings clean.

---

## 2026-07-22 — CAMPAIGN CLOSEOUT (charter §6)

### (a) Per-package disposition — numbers, not adjectives

| Pkg | Disposition | PR | Numbers |
|---|---|---|---|
| C0 | MERGED (55e806d) | #16 | portfolio re-anchored; runtime.rs pop-site grant ratified by merge; onboard READY; overlap gate clean |
| C1 | MERGED (fb0ca58) | #22 | tape wired at the sole pop site; KILL fired (+48% release / +90% debug at 200u demo-net) → opt-in `--record-tape`; default arm 12ms vs main 13–15ms; two-process digest agreement gated (639f6e14d27806cf5c9094d04ebb3fe9) |
| C2 | EXECUTED, this PR (draft) | — | KILL fired: pct_wins=0/32 (losses 8, ties 24); VB-006 0/10000 on FIFO, 76/100 first=0 under PCT d=3; replay identity 6f82d84d6d634ba9f885e0dc17db82dd / tape 4fac47fe998a6b61b690b3564a9e4940 across processes; null published (PLAYBOOK + VH-TRACK2-NULL-002) |
| C3 | MERGED | #18 | ClockSkew implemented as observable virtual-clock divergence; zero-magnitude guard (df10304) after 2 independent reviews; 0 of 5 corpus pins moved; 5-workload stdout byte-identical vs main |
| C4 | MERGED | #19 | vh-run-receipts-v1 / vh-finding-bundle-v1; standalone bundle replay REPRODUCED after out-dir deletion; tampered bundle exit 1 MISMATCH; receipts byte-deterministic (diff -r) |
| C5 | MERGED | #20 | `--shrink` MINIMIZED 3→1 injection @ demo-buggy 0xD1CE; kill margin ~60x under threshold (0.4s vs 60s); provenance binding incl. fingerprint-digest (vh-shrink-fingerprint-v1); PR #2 open contract closed |
| C6 | MERGED | #21 | 5/5 harvested (VB-007..011; recalls 91, 96, 79, 70, 58 /100 @0xD1CE); five distinct fault families; R7 kill NOT fired |
| C7 | MERGED (947dba9) | #17 | DESIGN.md:1-25 annotated historical-in-place; OPEN QUESTION escalated in PR body |

### (b) Frozen identities — doctor output, this branch, post-C2

```
vh 0.1.0 — determinism self-check [Tier 1]
  replay check: OK (universe 0 hash 9ce6199f133f4d3c9dd0da0075e352d2 events 45)
  observable fingerprint: OK (1684e7c347e645f43a80a30abc46adb7 vh-doctor-observable-v3)
```
Exit 0. Intact on every branch at every gate run of the campaign.

### (c) Kill criteria fired, and what was done

1. **C1 tape overhead** (>5%): fired at +48% release → tape moved
   behind `--record-tape`, default arm is the original pop
   bit-for-bit, numbers published (convergence-c1-decision-tape.txt).
2. **C2 PCT-vs-uniform** (32 seeds): fired at pct_wins=0 → PCT dropped
   as investment, kept as opt-in falsification harness, null published
   same-PR (PLAYBOOK + VH-TRACK2-NULL-002).
3. **VB-010 anti-gaming** (guaranteed-crash palette, 100/100 recall):
   fired → palette widened to 0..=2 crashes, re-pinned 70/100,
   counterevidence published in the entry + PR #21.
4. C5 (shrink >60s) and C6 (R7 realism) measured and NOT fired.

### (d) Couplings discovered beyond charter §5

1. Runtime-path workloads retrieve fault plans inside
   `UniverseCtx::runtime` — boundary-side shrink capture can't reach
   them without a kernel API (typed exit-2 meanwhile; future
   INTERFACE REQUEST).
2. Held-reorder expiry makes NetworkReorder lossy at stream-end —
   eos-trailer idiom (VB-011) is the reusable fix.
3. demo-buggy u0 shares trace hash 9ce6199f… with the doctor universe:
   trace hashes identify EXECUTIONS, not workload variants.
4. Tape digest rides UniverseResult → C4 receipts/bundles gained the
   field additively at rebase.
5. **New (C2):** printed repro lines must carry the schedule policy or
   non-FIFO findings replay CLEAN — repro honesty is policy-coupled
   (fixed + contract-tested this PR). Same coupling fail-closes
   `--schedule` against `--shrink`/`--out` until their replay formats
   record a policy.

### (e) Guided-exploration thesis — final status: UNPROVEN; INVESTMENT STOPS
### (revised 2026-07-22 per Codex audit C.1, issue #24 — was "FALSIFICATION COMPLETED")

W1 swarm palette: 0/5 corpus wins vs v0 (merged wave 1) over a
DISCRIMINATING instrument (real spread in the medians) — that null
stands as measured falsification on its axis. C2 event-priority
(PCT-inspired) d=3: 0/32 seeds faster than uniform tiebreak, but the
instrument was SATURATED (VB-006 floor effect: 6 independent two-way
races give uniform a ~98.4% per-universe hit rate, observed 96/100) —
that null is NARROW: no advantage shown; generalization not supported.
Honest joint verdict: the 1000x guided-exploration thesis is UNPROVEN
on this corpus — falsified on the palette axis, unmeasurable on the
schedule axis pending a depth≥2 instrument. Exploration mechanisms
stay in-tree, opt-in, as reproducible harnesses; investment stops.
Recorded revival falsifiers: a depth≥2 ordering class with low uniform
hit rate where the metric can discriminate; corpus entries whose
faults live outside v0's families.

### (f) Corpus count and realism verdict

11/25 (VB-001..005 seeded Phase-1 classes; VB-007..011 harvested from
real AI-generated code; VB-006 seeded-by-construction for C2's
falsification). R7 realism kill NOT fired: 5 admissible real-code
entries landed in one sprint, recalls 58–96/100 within pinned budgets.

### (g) Crate maturity after the campaign

| Crate | Before | After | Evidence |
|---|---|---|---|
| vh-core | scheduler FIFO-only | choice-point substrate: `pop_recorded` + `pop_chosen`, strategies module (PCT, uniform), pure/deny-listed | sched.rs, strategy.rs; 24 unit tests |
| vh-multiverse | single implicit schedule | policy-parameterized runner (`run_*_scheduled`), tape + digest observable, observable ClockSkew, compile-time observation ratchet exercised twice | lib.rs, runtime.rs, evidence.rs |
| vh-cli | run/replay/doctor | + `--record-tape`, `--schedule`, `--out` receipts+bundles, `--shrink`, `replay-bundle`, 11 corpus workloads; 21-test process contract | main.rs, workloads/, cli_contract.rs |
| vh-trace | trace + tape (W2) | tape digest is a two-process-stable identity (gated) | C1 gate |
| vh-gremlin | ClockSkew phantom | every generated fault has observable manifestation or honestly stays Armed | #18 |

Campaign verdict: 8/8 packages executed to disposition — 7 merged, 1
(C2) escalated as draft with its kill criterion fired and published.
No frozen-identity drift. No radioactive surface touched. Two-key law
held throughout: every merge was the operator's.
