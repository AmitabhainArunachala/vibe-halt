# VB-005 — fsync-lie-hole

| field | value |
|---|---|
| `id` | VB-005 |
| `class` | fsync-lie-hole |
| `source` | seeded |
| `workload` | `corpus-fsync-lie` |
| `expected_finding` | `oracle:wal_durability` |
| `recall` | found 21/100 at seed 0xD1CE, universe budget 100 |
| `repro` | `vh run --workload corpus-fsync-lie --seed 0xD1CE --universe 5` |
| `gate` | `corpus recall gate: corpus-fsync-lie` in `scripts/gate.sh` |
| `tier` | Tier 1 (engine-owned workload on the sim runtime) |

## Mechanism

The CORRECT paranoid WAL client (write -> flush -> fsync -> read-back verify -> ack) under lying hardware: an armed FsyncLie returns Ok while persisting nothing, and the verify read sees the OS cache, so the lie is invisible to any application-level defense. A later crash erases data an Ok fsync claimed durable. This is the class no app logic can close — the rig exists to expose it.

## The law

Acked implies durable-and-intact; when the hardware lies, only crash-testing the durability boundary reveals the hole.

Seeded entry: lower-bound evidence that the rig finds this class
(corpus/SCHEMA.md law 3). Recall pinned 2026-07-21.
