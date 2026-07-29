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
| `root_seed` | `0xD1CE` |
| `universe_budget` | 100 |
| `oracle_contract` | `required_oracles=[session_complete] required_always=[] required_sometimes=[session_aborted]` (CLI-printed; a missing required oracle counts as a contract violation, pinned 0) |
| `generator` | palette `v0`, fault-plan schema `vh-fault-plan-v1` (CLI banner); failing-repro fault-plan digest `8b20a2ce772578b9a826220b865f64cc` |
| `schedule` | `fifo`, no decision tape (`tape=false`) |
| `divergence_check` | enabled (`divergence-check=true`); evidence: `pairwise replay agreement (sampled falsifier — not proof; Tier-1 claim rests on the D0 boundary)` |
| `counts` | always-failures **79**; clean **21**; divergent 0; sometimes unreached 0; invalid completions 0; contract violations 0 |
| `expected_exit` | exit 1, `verdict: FINDINGS (see above)` |
| `control` | fault-free/harmless universes must PASS: clean = 21 exactly (>=1) at the pinned budget; pinned clean universe 8: `vh run --workload corpus-transient-fatal-abort --seed 0xD1CE --universe 8` -> no finding, exit 3 (single-replay UNCHECKED policy) |
| `required_facts` | every accepted task must reach completion; the `session_aborted` sometimes-property must be reached within the budget (`sometimes unreached` pinned 0), so the abort window is provably exercised. |

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

Harvested regression entry: its real-issue provenance and pinned
manifestation count test the reduced mechanism, but it receives no
retrospective holdout credit (`corpus/SCHEMA.md` laws 3–4). Recall measured
then pinned 2026-07-22.

## Contract freeze (K1, 2026-07-25)

All counts measured twice consecutively at engine head `ca6b37f`,
byte-identical summaries both passes (corpus/** edits never touch
the engine, so this entry's PR does not move them):

```
$ vh run --workload corpus-transient-fatal-abort --seed 0xD1CE --universes 100
always-failures: 79 universe(s); divergent: 0; sometimes unreached: 0; invalid completions: 0; contract violations: 0; clean: 21
verdict: FINDINGS (see above)
```

Failing-repro receipt (universe 0): trace hash
`66d50c1e004c48b6ab664a302feceae5` (29 events), fault-plan digest
`8b20a2ce772578b9a826220b865f64cc` (`vh-fault-plan-v1`), exit 1.
Clean-control receipt (universe 8): trace hash
`160acd3043b811e77506b945466151e6` (45 events), fault-plan digest
`4c7d200e07670f2eb9e0ae8788f9961b`, exit 3 (`UNCHECKED`, no finding).

Drift law: a future measurement differing from these pins — in
EITHER direction — is a finding to explain and re-pin with its
semantic cause, never a tolerance band. A count that is not
identical across two consecutive runs at one head makes this
entry's claim UNCHECKED and files an identity defect
(controller kill rule).
