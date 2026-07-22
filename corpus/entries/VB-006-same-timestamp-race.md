# VB-006 — same-timestamp race (seeded; the C2 PCT bet's instrument)

| field | value |
|---|---|
| `id` | VB-006 |
| `class` | same-timestamp-race |
| `source` | seeded (convergence C2 / Track-2 W3 — reserved since the campaign charter) |
| `workload` | `corpus-same-timestamp-race` |
| `expected_finding` | `oracle:init_before_commit` |
| `recall` | FIFO v0: found 0/10000 at seed 0xD1CE (invisible by construction). PCT d=3: found 76/100 at seed 0xD1CE, first at universe 0. |
| `repro` | `vh run --workload corpus-same-timestamp-race --seed 0xD1CE --universe 0 --schedule pct:3 --record-tape` |
| `gate` | `C2 gate: VB-006 invisible to FIFO v0` + `C2 gate: PCT d=3 finds VB-006` in `scripts/gate.sh` |
| `tier` | Tier 1 (engine-owned workload; schedule strategies deterministic per (seed, universe), witnessed by the decision tape) |

## Mechanism

Each round the writer sends `init` then `commit` back-to-back; both
arrive at the SAME virtual time — a same-timestamp scheduler frontier
of exactly two. The store applies `commit` without checking that `init`
arrived (the bug: an ordering assumption with no guard). No faults are
injected at all: the race is pure scheduling.

Under FIFO v0 the insertion-order tiebreak always delivers `init`
first, so the bug is invisible by construction — 0 findings in 10,000
universes at the pinned seed. Any same-timestamp strategy (PCT or
uniform tiebreak) can flip the pair and expose it.

## The law

A commit must observe its init (`commit_base:<round>` == "ok"); the
failure detail names every round whose commit ran against a missing
base.

## Bakeoff disposition (C2 kill criterion — FIRED)

Over 32 seeds at budget 1000 (`scripts/track2_pct_bakeoff.py`): PCT d=3
first-finding median 0, uniform-with-random-tiebreak median 0; PCT wins
0, losses 8, ties 24 — PCT is NOT faster than uniform. Per the charter,
PCT is DROPPED as a guided-exploration bet (it remains in-tree, opt-in,
as the reproducible falsification harness — the W1 swarm-palette
precedent); the decision tape stays (replay/causality substrate). This
completes the falsification of the audit's guided-exploration thesis:
what VB-006 needed was ANY same-timestamp diversity, not guidance.
Evidence: `docs/audits/antithesis-dst-2026-07-21/commands/convergence-c2-pct.txt`.

Seeded entry: lower-bound evidence only (corpus/SCHEMA.md law 3).
Recall pinned 2026-07-22.
