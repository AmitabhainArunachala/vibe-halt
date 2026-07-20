# VB-003 — dirty-read

| field | value |
|---|---|
| `id` | VB-003 |
| `class` | dirty-read |
| `source` | seeded |
| `workload` | `corpus-dirty-read` |
| `expected_finding` | `oracle:published_implies_durable` |
| `recall` | found 83/100 at seed 0xD1CE, universe budget 100 |
| `repro` | `vh run --workload corpus-dirty-read --seed 0xD1CE --universe 0` |
| `gate` | `corpus recall gate: corpus-dirty-read` in `scripts/gate.sh` |
| `tier` | Tier 1 (engine-owned workload on the sim runtime) |

## Mechanism

A reporter publishes values read from the FULL disk view — application buffer and OS cache included — as settled facts. A crash erases the volatile layers; the published values never existed durably. Crash-free universes pass (final shutdown persists everything), so the finding always names a real dirty read.

## The law

Downstream publication must read only committed (fsynced) state.

Seeded entry: lower-bound evidence that the rig finds this class
(corpus/SCHEMA.md law 3). Recall pinned 2026-07-21.
