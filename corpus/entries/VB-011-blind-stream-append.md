# VB-011 — blind stream append (harvested)

| field | value |
|---|---|
| `id` | VB-011 |
| `class` | duplicate-delivery |
| `source` | HARVESTED: langchain-ai/langchain issue #22227 ("astream_events (V1 and V2) gives duplicate content in on_chat_model_stream", reported 2024-05-28, closed) — https://github.com/langchain-ai/langchain/issues/22227 |
| `workload` | `corpus-blind-stream-append` |
| `expected_finding` | `oracle:stream_integrity` |
| `recall` | found 58/100 at seed 0xD1CE, universe budget 100 |
| `repro` | `vh run --workload corpus-blind-stream-append --seed 0xD1CE --universe 2` |
| `gate` | `corpus recall gate: corpus-blind-stream-append` in `scripts/gate.sh` |
| `tier` | Tier 1 (engine-owned workload on the sim runtime) |
| `root_seed` | `0xD1CE` |
| `universe_budget` | 100 |
| `oracle_contract` | `required_oracles=[stream_integrity] required_always=[] required_sometimes=[]` (CLI-printed; a missing required oracle counts as a contract violation, pinned 0) |
| `generator` | palette `v0`, fault-plan schema `vh-fault-plan-v1` (CLI banner); failing-repro fault-plan digest `200dd1d663ba5636e472e0888bad10d0` |
| `schedule` | `fifo`, no decision tape (`tape=false`) |
| `divergence_check` | enabled (`divergence-check=true`); evidence: `pairwise replay agreement (sampled falsifier — not proof; Tier-1 claim rests on the D0 boundary)` |
| `counts` | always-failures **58**; clean **42**; divergent 0; sometimes unreached 0; invalid completions 0; contract violations 0 |
| `expected_exit` | exit 1, `verdict: FINDINGS (see above)` |
| `control` | fault-free/harmless universes must PASS: clean = 42 exactly (>=1) at the pinned budget; pinned clean universe 0: `vh run --workload corpus-blind-stream-append --seed 0xD1CE --universe 0` -> no finding, exit 3 (single-replay UNCHECKED policy) |
| `required_facts` | the assembled-stream and sent-stream fact pair must be present and equal; a missing/malformed pair is a hard failure, never a vacuous match (PR #32). |

## Provenance (the real bug)

`astream_events` delivers duplicate content in `on_chat_model_stream`:
nested callback/streaming layers re-emit the same chunk, and consumers
see every token twice — "Books| Books|", "1|1|.|.|" — across both V1
and V2 of the streaming API. The consumer-side defect this harvests is
the invariant those consumers relied on: a stream assembled by BLIND
APPEND, trusting the event stream to be exactly-once-in-order, with no
sequence numbers consulted, no deduplication, no reorder handling.

## Mechanism (reduced)

A producer streams uniquely-numbered chunks (`chunk:<seq>:<token>`);
the consumer appends every delivery in arrival order — the sequence
number is RIGHT THERE in the payload and is ignored (the bug). The
palette is duplicates-and-pairwise-reorders only (0..=2 injections); an
end-of-stream trailer guarantees a held reorder always releases its
captive, so no content chunk is ever lost — the assembled document can
differ from the sent stream only through the consumer's missing
sequence discipline meeting a shaped delivery. Fault-free universes
PASS.

## The law

The assembled stream must equal the sent stream exactly (`assembled` ==
`expected`); the failure detail prints both, exposing the duplicated or
transposed tokens.

Harvested regression entry: its real-issue provenance and pinned
manifestation count test the reduced mechanism, but it receives no
retrospective holdout credit (`corpus/SCHEMA.md` laws 3–4). Recall measured
then pinned 2026-07-22.

## Contract freeze (K1, 2026-07-25)

All counts measured twice consecutively at engine head `ca6b37f`,
byte-identical summaries both passes (corpus/** edits never touch
the engine, so this entry's PR does not move them):

```
$ vh run --workload corpus-blind-stream-append --seed 0xD1CE --universes 100
always-failures: 58 universe(s); divergent: 0; sometimes unreached: 0; invalid completions: 0; contract violations: 0; clean: 42
verdict: FINDINGS (see above)
```

Failing-repro receipt (universe 2): trace hash
`6a429b6799782a78290155c120cdda99` (35 events), fault-plan digest
`200dd1d663ba5636e472e0888bad10d0` (`vh-fault-plan-v1`), exit 1.
Clean-control receipt (universe 0): trace hash
`c86c5a7b59e6112a30f173e9dfc5cda9` (28 events), fault-plan digest
`b8dfda3b7737d29c93d2b74c7ae65d67`, exit 3 (`UNCHECKED`, no finding).

Drift law: a future measurement differing from these pins — in
EITHER direction — is a finding to explain and re-pin with its
semantic cause, never a tolerance band. A count that is not
identical across two consecutive runs at one head makes this
entry's claim UNCHECKED and files an identity defect
(controller kill rule).
