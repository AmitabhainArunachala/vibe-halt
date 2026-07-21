# VB-001 — lost-update

| field | value |
|---|---|
| `id` | VB-001 |
| `class` | lost-update |
| `source` | seeded |
| `workload` | `corpus-lost-update` |
| `expected_finding` | `oracle:no_lost_updates` |
| `recall` | found 29/100 at seed 0xD1CE, universe budget 100 |
| `repro` | `vh run --workload corpus-lost-update --seed 0xD1CE --universe 1` |
| `gate` | `corpus recall gate: corpus-lost-update` in `scripts/gate.sh` |
| `tier` | Tier 1 (engine-owned workload on the sim runtime) |

## Mechanism

Two writers increment a shared counter via read-modify-write messages; the store applies blind last-write-wins sets with no compare-and-swap. A delayed read reply overlaps the writers' cycles: both read the same value, both write value+1, and an increment vanishes (or a stale write rolls the counter back).

## The law

The store must apply increments atomically (CAS / version check); the workload's client protocol is the bug, the store contract is the law: final counter == requested increments.

Seeded entry: lower-bound evidence that the rig finds this class
(corpus/SCHEMA.md law 3). Recall pinned 2026-07-21.
