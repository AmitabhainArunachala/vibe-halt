# VB-009 — transient-fatal abort (harvested)

| field | value |
|---|---|
| `id` | VB-009 |
| `class` | transient-fatal-abort |
| `source` | HARVESTED: OpenHands/OpenHands issue #12064 ("Bad Gateway in LiteLLM proxy causes agent to crash", reported 2025-12-16; fixed by PR #12117) — https://github.com/OpenHands/OpenHands/issues/12064 |
| `workload` | `corpus-transient-fatal-abort` |
| `expected_finding` | `oracle:session_complete` |
| `recall` | found 79/100 at seed 0xD1CE, universe budget 100 |
| `repro` | `vh run --workload corpus-transient-fatal-abort --seed 0xD1CE --universe 0` |
| `gate` | `corpus recall gate: corpus-transient-fatal-abort` in `scripts/gate.sh` |
| `tier` | Tier 1 (engine-owned workload on the sim runtime) |

## Provenance (the real bug)

A LiteLLM-proxy 502 Bad Gateway surfaces as `litellm.APIError`, which is
missing from `LLM_RETRY_EXCEPTIONS` in `openhands/llm/llm.py`. The retry
logic does not recognize the transient error, the agent controller
catches the unhandled exception, and the agent crashes mid-session —
abandoning the conversation and every remaining accepted task. The fix
(PR #12117) simply adds the error to the retriable set: the failure was
transient all along.

## Mechanism (reduced)

A client accepts a session of tasks and awaits each backend reply under
a deadline. The palette is transient-only (partitions that heal, delays
that deliver), so a retrying client would always finish the session. On
a missed deadline the buggy client classifies the failure as FATAL (the
missing retriable entry) and aborts the ENTIRE session: later dispatch
timers step nothing, late replies fall on the floor, and every
remaining accepted task is abandoned.

Distinct from `demo-net-buggy` (fire-and-forget never LEARNS of a
failure): this client learns, misclassifies, and takes the whole
session down with it — the defect is blast radius, not blindness.

## The law

Accepted work must complete once the transient fault clears:
`completed:<task>` == true for every accepted task. The failure detail
names every abandoned task.

Harvested entry: counts toward the >=25 / >=80% real-recall acceptance
(corpus/SCHEMA.md law 3). Recall measured then pinned 2026-07-22.
