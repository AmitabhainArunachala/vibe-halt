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
| `root_seed` | `0xD1CE` |
| `universe_budget` | 10000 (FIFO invisibility control) / 100 (PCT d=3 recall) |
| `oracle_contract` | `required_oracles=[init_before_commit] required_always=[] required_sometimes=[]` (CLI-printed; a missing required oracle counts as a contract violation, pinned 0) |
| `generator` | palette `v0`, fault-plan schema `vh-fault-plan-v1` (CLI banner); universe-0 fault-plan digest `b8dfda3b7737d29c93d2b74c7ae65d67` — no faults are injected, the race is pure scheduling, so the digest is identical under both schedules |
| `schedule` | control: `fifo`, `tape=false`; recall: `pct:3` with `--record-tape` (`tape=true`), universe-0 decision-tape digest `4fac47fe998a6b61b690b3564a9e4940` (`vh-decision-tape-v1`) |
| `divergence_check` | enabled for both campaigns (`divergence-check=true`); evidence: `pairwise replay agreement (sampled falsifier — not proof; Tier-1 claim rests on the D0 boundary)` |
| `counts` | FIFO 10000: always-failures **0**; clean **10000**; divergent 0; sometimes unreached 0; invalid completions 0; contract violations 0. PCT d=3 100: always-failures **76**; clean **24**; divergent 0; sometimes unreached 0; invalid completions 0; contract violations 0 |
| `expected_exit` | FIFO control: exit 0, `verdict: CLEAN`. PCT d=3: exit 1, `verdict: FINDINGS (see above)` |
| `control` | the FIFO run is the fault-free control (0/10000, exit 0 — invisibility by construction); pinned clean universe 0 under FIFO: `vh run --workload corpus-same-timestamp-race --seed 0xD1CE --universe 0` -> no finding, exit 3 (single-replay UNCHECKED policy) |
| `required_facts` | every commit must observe its init: `commit_base:<round>` must equal "ok"; the failure detail names every round whose commit ran against a missing base |

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

Over 32 seeds at budget 1000 (`scripts/track2_pct_bakeoff.py`):
event-priority (PCT-inspired) d=3 first-finding median 0,
uniform-with-random-tiebreak median 0; wins 0, losses 8, ties 24 — not
faster than uniform. Per the charter the strategy is DROPPED as a
guided-exploration bet (it remains in-tree, opt-in, as the reproducible
falsification harness — the W1 swarm-palette precedent); the decision
tape stays (replay/causality substrate).

**Claim scope (narrowed 2026-07-22, Codex audit C.1 / issue #24):** this
bakeoff is a floor effect — VB-006's 6 independent two-way races give
uniform a ~98.4% per-universe hit rate (observed 96/100), saturating the
metric at 0/0. It supports only the NARROW null (no advantage on this
workload/metric), not a general falsification of guided exploration.
What VB-006 needed was ANY same-timestamp diversity, not guidance; a
future depth->=2 entry with a low uniform hit rate is the instrument
that could actually discriminate.
Evidence: `docs/audits/antithesis-dst-2026-07-21/commands/convergence-c2-pct.txt`.

Seeded entry: lower-bound evidence only (corpus/SCHEMA.md law 3).
Recall pinned 2026-07-22.

## Contract freeze (K1, 2026-07-25)

All counts measured twice consecutively at engine head `ca6b37f`,
byte-identical summaries both passes (corpus/** edits never touch
the engine, so this entry's PR does not move them):

```
$ vh run --workload corpus-same-timestamp-race --seed 0xD1CE --universes 10000
always-failures: 0 universe(s); divergent: 0; sometimes unreached: 0; invalid completions: 0; contract violations: 0; clean: 10000
verdict: CLEAN

$ vh run --workload corpus-same-timestamp-race --seed 0xD1CE --universes 100 --schedule pct:3 --record-tape
always-failures: 76 universe(s); divergent: 0; sometimes unreached: 0; invalid completions: 0; contract violations: 0; clean: 24
verdict: FINDINGS (see above)
```

Failing-repro receipt (universe 0, `pct:3`, taped): trace hash
`6f82d84d6d634ba9f885e0dc17db82dd` (31 events), fault-plan digest
`b8dfda3b7737d29c93d2b74c7ae65d67` (`vh-fault-plan-v1`), decision tape
`4fac47fe998a6b61b690b3564a9e4940` (`vh-decision-tape-v1`), exit 1.
Clean-control receipt (universe 0, FIFO): trace hash
`c260cb0353bec0ee1e0aab391d48d6ef` (31 events), same fault-plan digest,
exit 3 (`UNCHECKED`, no finding).

Drift law: a future measurement differing from these pins — in
EITHER direction — is a finding to explain and re-pin with its
semantic cause, never a tolerance band. A count that is not
identical across two consecutive runs at one head makes this
entry's claim UNCHECKED and files an identity defect
(controller kill rule). The exact PCT gate and the narrow FIFO
null recorded above are preserved unchanged (controller law).
