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

Harvested entry: counts toward the >=25 / >=80% real-recall acceptance
(corpus/SCHEMA.md law 3). Recall measured then pinned 2026-07-22.
