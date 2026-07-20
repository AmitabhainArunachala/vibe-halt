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
le64(at_nanos) · 0x1F · kind · 0x1F · data · 0x1E
```

The separators (US `0x1F`, RS `0x1E`) make field and record boundaries
part of the hash — `("ab","c")` never collides with `("a","bc")` (tested:
`field_boundaries_matter`).

Two universe runs are **identical** iff their final hashes match. That
definition is the contract the divergence detector enforces.

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
