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

## Observable identity (Tier-1 doctrine)

Two universe runs are **identical** iff their COMPLETE public
`UniverseResult` observations match — every field, none privileged:

1. universe ID
2. trace hash
3. trace event count
4. the ordered always-check transcript, **passing checks included**
5. ordered always-failures with details
6. declared sometimes properties and their reached state
7. runner lifecycle evidence (typed completion outcome and fault-plan
   RETRIEVAL discipline — retrieval is all THAT ledger claims; the
   semantic ladder is item 9)
8. the canonical fault-plan digest (`vh-fault-plan-v1`) binding the
   replay input's identity into the result (hardening-loop-4 GAP)
9. the runner-owned semantic fault-lifecycle evidence (Phase-1 sim
   runtime): per planned injection, the measured
   `Offered → Armed → Injected → Manifested → Recovered` timestamps
   (`crates/vh-multiverse/src/evidence.rs` documents exactly what each
   stage measures). `None` for universes that never constructed the
   runtime — absence vs. presence is itself observable.

The trace hash alone is NOT identity: a replay can skip or reorder a
passing invariant while recording an identical trace (hardening-loop-3
GAP). That is why the transcript of item 4 is ordered and includes
passes. The kernel comparator is deliberately struct equality
(`UniverseResult::observably_equal` in `crates/vh-multiverse/src/lib.rs`),
so adding an observable field automatically strengthens the divergence
check; this list must grow with the struct. Enforced by
`detector_flags_skipped_passing_invariants` and
`detector_flags_reordered_passing_check_transcripts`
(`crates/vh-multiverse/tests/divergence.rs`).

### Changelog

- **2026-07-21 (Phase-1 sim runtime):** observable identity grew item 9
  (runner-owned semantic fault-lifecycle evidence — the runtime, not the
  workload, owns injection and measures Offered → Armed → Injected →
  Manifested → Recovered per injection; closes the loop-4 DEFERRED item
  ~4 weeks ahead of 2026-08-15). In the same migration, the demo's
  durability law was re-expressed as an END-STATE ORACLE (Phase-2 pulled
  early): the runner judges declared `acked:*`/`committed:*` end state
  post-run and records exactly one `oracle:durability` transcript entry
  per universe in place of the 32 inline per-key checks (per-key
  granularity lives on in the oracle's failure detail; oracles record no
  trace events by construction). The doctor observable fingerprint
  migrated `vh-doctor-observable-v2` (`cdb049391ddbacc06eb3faf3ea1cb43a`)
  → `vh-doctor-observable-v3` (`1684e7c347e645f43a80a30abc46adb7`),
  covering both causes; the v3 renderer records `runtime-evidence: none`
  for legacy-path universes and one `runtime-injection` line per
  injection otherwise, and doctor additionally asserts the frozen demo
  universe stays on the LEGACY path (its runtime evidence must be
  `None`) with the one-entry oracle transcript. The TRACE hash identity
  (`9ce6199f133f4d3c9dd0da0075e352d2`, 45 events) is unchanged — no
  recorded trace hash is invalidated. Additive in the same change:
  `FaultKind` gained TornWrite / FsyncLie / NetworkDuplicate /
  NetworkReorder (new `vh-fault-plan-v1` canonical strings; every
  previously recorded digest is over unchanged renderings and remains
  valid; `FaultPlan::generate`'s palette is untouched and pinned by
  regression).
- **2026-07-20 (hardening loop 4):** observable identity grew items 7-8
  (retrieval-honest lifecycle naming, fault-plan digest); the doctor
  observable fingerprint migrated `vh-doctor-observable-v1`
  (`462e803383be1b24594e76d5f9301be8`) → `vh-doctor-observable-v2`
  (`cdb049391ddbacc06eb3faf3ea1cb43a`) as an explicit version migration.
  The TRACE hash identity (`9ce6199f133f4d3c9dd0da0075e352d2`, 45
  events) is unchanged — no recorded trace hash is invalidated.
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
