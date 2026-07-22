# VB-010 — resume-becomes-replay (harvested)

| field | value |
|---|---|
| `id` | VB-010 |
| `class` | retry-double-apply |
| `source` | HARVESTED: langchain-ai/langgraph issue #7361 ("When resume from a specific checkpoint_id, it becomes replay", reported 2026-03-31; regression in 1.1.x, workarounds: downgrade to 1.0.x / drop checkpoint_id / PR #7126) — https://github.com/langchain-ai/langgraph/issues/7361 |
| `workload` | `corpus-resume-replay` |
| `expected_finding` | `oracle:resume_at_most_once` |
| `recall` | found 70/100 at seed 0xD1CE, universe budget 100 |
| `repro` | `vh run --workload corpus-resume-replay --seed 0xD1CE --universe 1` |
| `gate` | `corpus recall gate: corpus-resume-replay` in `scripts/gate.sh` |
| `tier` | Tier 1 (engine-owned workload on the sim runtime) |

## Provenance (the real bug)

Resuming a LangGraph graph from a specific `checkpoint_id` re-executes
from the beginning instead of continuing at the interrupt point — "the
second run for resume still run from the beginning of the graph, not
interrupt trigger point." The checkpoint with the progress exists and
is readable (dropping `checkpoint_id` from the config is a workaround —
the data was fine); the resume path misuses it. Regression in 1.1.x.

## Mechanism (reduced)

A pipeline applies side-effecting steps (the outside world: tools
invoked, emails sent — crash-proof by nature) and durably fsyncs a
progress cursor after each. On crash-recovery it reads the cursor back
from durable state — and then schedules the pipeline from step 0
anyway. Every step completed before the crash lands a second time. The
palette is crashes-only (0..=2): the cursor is always on disk before a
crash can matter, so every duplicate application is the resume path
ignoring truth it demonstrably held; crash-free universes PASS.

Distinct from VB-007 (stale sweep re-dispatching one in-flight call)
and VB-002 (lossy-network retry without server dedupe): here the
duplication engine is CRASH-RECOVERY itself replaying finished work.

## The law

Recovery must resume, not replay: each step's side effect lands at most
once across all crash epochs (`applied:<step>` <= 1). The failure detail
names each replayed step and its count.

Harvested entry: counts toward the >=25 / >=80% real-recall acceptance
(corpus/SCHEMA.md law 3). Recall measured then pinned 2026-07-22.
