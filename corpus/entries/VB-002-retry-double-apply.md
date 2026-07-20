# VB-002 — retry-double-apply

| field | value |
|---|---|
| `id` | VB-002 |
| `class` | retry-double-apply |
| `source` | seeded |
| `workload` | `corpus-retry-double-apply` |
| `expected_finding` | `oracle:exactly_once` |
| `recall` | found 76/100 at seed 0xD1CE, universe budget 100 |
| `repro` | `vh run --workload corpus-retry-double-apply --seed 0xD1CE --universe 1` |
| `gate` | `corpus recall gate: corpus-retry-double-apply` in `scripts/gate.sh` |
| `tier` | Tier 1 (engine-owned workload on the sim runtime) |

## Mechanism

The client retries un-acked appends (correct — the network is lossy) but the server applies every receipt with no idempotency key. A duplicated delivery, an append racing its own retry, or a partition-eaten ack turns one logical append into two applications. The palette's blackout budget (<=200k) sits under the retry budget (240k), so under-application is impossible: every violation is the over-apply bug.

## The law

Retries demand idempotency keys on the apply path: each requested item applied exactly once.

Seeded entry: lower-bound evidence that the rig finds this class
(corpus/SCHEMA.md law 3). Recall pinned 2026-07-21.
