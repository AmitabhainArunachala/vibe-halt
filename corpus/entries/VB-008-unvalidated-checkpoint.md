# VB-008 — unvalidated checkpoint (harvested)

| field | value |
|---|---|
| `id` | VB-008 |
| `class` | dirty-read |
| `source` | HARVESTED: langchain-ai/langgraph issue #6491 ("Invalid state saved to checkpoint without validation, causing permanent corruption", reported 2025-11-24) — https://github.com/langchain-ai/langgraph/issues/6491 |
| `workload` | `corpus-unvalidated-checkpoint` |
| `expected_finding` | `oracle:checkpoint_recoverable` |
| `recall` | found 96/100 at seed 0xD1CE, universe budget 100 |
| `repro` | `vh run --workload corpus-unvalidated-checkpoint --seed 0xD1CE --universe 0` |
| `gate` | `corpus recall gate: corpus-unvalidated-checkpoint` in `scripts/gate.sh` |
| `tier` | Tier 1 (engine-owned workload on the sim runtime) |

## Provenance (the real bug)

LangGraph validates node INPUT (when preparing the next task) but not
node OUTPUT (after execution completes): a node returning invalid state
(e.g. `None` in a `List[str]` field) is checkpointed successfully, and
the corruption only surfaces later — `get_state_history()` re-validates
on retrieval, raises `ValidationError`, and the checkpoint is
permanently unrecoverable. Write-side accepts what read-side rejects.

## Mechanism (reduced)

A checkpointer persists framed records (`ckpt:<id>:<payload>#end`) and
acknowledges after write → flush → fsync succeed — WITHOUT validating
or reading back what it wrote; validation lives only on the retrieval
path. The palette is torn-writes-only: every write returns Ok and every
record is durably fsynced, so the only way an acknowledged checkpoint
can be unrecoverable is the write-side validation gap meeting a tear —
half a record persists, the terminator is gone, and retrieval rejects
it. Contrast `demo-disk`'s paranoid WAL, whose read-back verify after
fsync closes exactly this window.

## The law

Every acknowledged checkpoint must be recoverable at retrieval:
`acked:<id>` ⇒ `recovered:<id>` (the exact framed record validates on
the way out). The failure detail names each acknowledged-but-
unrecoverable checkpoint.

Harvested entry: counts toward the >=25 / >=80% real-recall acceptance
(corpus/SCHEMA.md law 3). Recall measured then pinned 2026-07-22.
