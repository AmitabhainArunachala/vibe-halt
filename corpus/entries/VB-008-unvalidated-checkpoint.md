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
| `root_seed` | `0xD1CE` |
| `universe_budget` | 100 |
| `oracle_contract` | `required_oracles=[checkpoint_recoverable] required_always=[] required_sometimes=[]` (CLI-printed; a missing required oracle counts as a contract violation, pinned 0) |
| `generator` | palette `v0`, fault-plan schema `vh-fault-plan-v1` (CLI banner); failing-repro fault-plan digest `3ea7abf11207f577e931a3d8444c4266` |
| `schedule` | `fifo`, no decision tape (`tape=false`) |
| `counts` | always-failures **96**; clean **4**; divergent 0; sometimes unreached 0; invalid completions 0; contract violations 0 |
| `expected_exit` | exit 1, `verdict: FINDINGS` |
| `control` | fault-free/harmless universes must PASS: clean = 4 exactly (>=1) at the pinned budget; pinned clean universe 17: `vh run --workload corpus-unvalidated-checkpoint --seed 0xD1CE --universe 17` -> no finding, exit 3 (single-replay UNCHECKED policy) |
| `required_facts` | the oracle independently re-derives checkpoint membership from the raw durable dump plus each checkpoint's expected framed record; it never trusts a workload-precomputed `recovered:<ckpt>` Boolean (PR #32). |

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

## Contract freeze (K1, 2026-07-25)

All counts measured twice consecutively at engine head `ca6b37f`,
byte-identical summaries both passes (corpus/** edits never touch
the engine, so this entry's PR does not move them):

```
$ vh run --workload corpus-unvalidated-checkpoint --seed 0xD1CE --universes 100
always-failures: 96 universe(s); divergent: 0; sometimes unreached: 0; invalid completions: 0; contract violations: 0; clean: 4
verdict: FINDINGS   (exit 1)
```

Failing-repro receipt (universe 0): trace hash
`cc9068874514d123d0bb370bb3c11ce4` (31 events), fault-plan digest
`3ea7abf11207f577e931a3d8444c4266` (`vh-fault-plan-v1`), exit 1.
Clean-control receipt (universe 17): trace hash
`efd8c87d5f91817ff5ea0a6232c4654c` (28 events), fault-plan digest
`eb0e27583fbc63a5ccbaa7a260102fc4`, exit 3 (`UNCHECKED`, no finding).

Drift law: a future measurement differing from these pins — in
EITHER direction — is a finding to explain and re-pin with its
semantic cause, never a tolerance band. A count that is not
identical across two consecutive runs at one head makes this
entry's claim UNCHECKED and files an identity defect
(controller kill rule).
