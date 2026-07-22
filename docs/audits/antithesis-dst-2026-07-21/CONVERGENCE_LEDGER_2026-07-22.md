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
