# Trace Format v0

The trace is the spine of vibe-halt: replay, shrinking, divergence
detection, and evidence all hang off it. Implementation:
`crates/vh-trace/src/lib.rs`.

## Event

| field      | type   | meaning                                          |
|------------|--------|--------------------------------------------------|
| `at_nanos` | u64    | virtual-time nanos at record time                |
| `kind`     | string | short machine-readable kind (`put`, `crash`, …)  |
| `data`     | string | deterministic payload                            |

Events are append-only. There is no mutation and no removal; a different
history is a different universe.

## Hash chain

Running FNV-1a 128 over every event in order. Per event, absorbed bytes:

```
le64(at_nanos) · le64(len(kind)) · kind · le64(len(data)) · data
```

Framing is length-prefixed: every field is fixed-width or preceded by its
little-endian byte length, so the absorbed stream decodes to exactly one
event sequence for ANY payload content — framing is injective by
construction (tested: `field_boundaries_matter`,
`separator_bytes_in_payload_cannot_forge_event_boundaries`,
`event_count_is_part_of_framing`).

Two universe runs are **identical** iff their complete observable results
match: trace hash, event count, always-failures, and sometimes map. That
definition is the contract the divergence detector enforces
(`UniverseResult::observably_equal`).

### Changelog

- **2026-07-20 (pre-release repair):** original v0 framing used US/RS
  separator bytes (`0x1F`/`0x1E`), which was non-injective — payloads
  containing separator bytes could forge event boundaries (found by the
  PR #1 adversarial review, with a byte-identical two-events/one-event
  collision). No corpus or release existed, so v0 is redefined to
  length-prefixed framing rather than bumping to v1. All previously
  quoted reference hashes are invalidated by this repair.

## Frozen surfaces

The hash constants, absorption order, and separators are frozen. So is
the xoshiro256++/SplitMix64 PRNG output (`frozen_reference_vector` test in
`crates/vh-core/src/rng.rs`). Changing any of these invalidates every
recorded trace hash in every corpus: it is a format version bump (v1),
never a refactor.

## Known v0 limits (accepted, documented)

- FNV-1a is not cryptographic. Fine for divergence detection between
  runs we control; v1 moves to SHA-256 when traces become cross-party
  evidence (e.g., receipts consumed by dharma_swarm gates).
- Traces are in-memory only. Phase 2 adds spill-to-disk JSONL with the
  same hash chain, under `~/.vibe-halt/traces/`.
