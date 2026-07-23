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
10. same-timestamp schedule policy, including PCT depth when present
11. decision-tape digest (absence vs. presence included)
12. `vh-end-state-observation-v1` canonical bytes for the exact ordered map
    consumed by end-state oracles — passing the same oracle does not erase a
    raw-state difference
13. `vh-complete-observation-v1` canonical bytes covering items 1–12

The trace hash alone is NOT identity: a replay can skip or reorder a
passing invariant while recording an identical trace (hardening-loop-3
GAP), or reach a different raw end state while the same coarse oracle passes.
That is why the transcript of item 4 is ordered and includes passes, and why
item 12 retains the oracle input. The kernel comparator is deliberately struct
equality
(`UniverseResult::observably_equal`, `crates/vh-multiverse/src/lib.rs:553-555`),
so adding an observable field automatically strengthens the divergence
check; this list must grow with the struct. Enforced by
`detector_flags_skipped_passing_invariants` and
`detector_flags_reordered_passing_check_transcripts`
(`crates/vh-multiverse/tests/divergence.rs`).

The item-12/13 false negative (post-audit finding A.1 — same trace, same
passing oracle, different unused raw end state) is closed the same way:
`detector_flags_different_raw_end_state_behind_same_passing_oracle`
(`crates/vh-multiverse/tests/divergence.rs:728-741`) proves two replays
with an identical trace and an identical passing `EndStateOracle` verdict
still diverge because their declared-but-unjudged end-state value differs;
`detector_flags_a_new_passing_oracle`
(`crates/vh-multiverse/tests/divergence.rs:777-788`) proves a transcript
that only gains a new PASSING oracle entry also diverges. Map-construction
order is explicitly NOT part of identity —
`reordered_map_construction_has_identical_canonical_state`
(`crates/vh-multiverse/tests/divergence.rs:814-824`) inserts the same two
`declare_end` keys in opposite order and asserts both the end-state and
complete-observation identities, and `observably_equal`, agree. The
getter-surface/observation-view compile-time ratchet is enforced by
`observation_view_matches_the_getter_surface_exhaustively`
(`crates/vh-multiverse/tests/divergence.rs:831-871`), which destructures
`UniverseResult` and `UniverseObservation` without `..` on either side.

### Canonical observation bytes

Both observation identities use algorithm tag
`vh-canonical-length-framing-v1`. Their envelope is:

```
magic[8] · le64(len(algorithm)) · algorithm
         · le64(len(schema)) · schema
         · le64(field_count)
         · repeated(field_name, kind_byte, le64(payload_len), payload)
```

Nested strings/bytes and sequence items are length-prefixed; integers are
fixed little-endian u64; booleans/options/enums use explicitly validated tag
bytes. Maps are encoded in strictly increasing key order. The strict decoder
rejects malformed UTF-8, duplicate/unknown/reordered fields, duplicate or
reordered map keys, invalid tags, truncation, and trailing bytes. No `Debug`
output, host address, locale, float display, panic text, or unordered
collection iteration feeds this encoding.

The canonical bytes themselves are replay identity, not a new hash. C3's
persisted evidence schema will apply its separately reviewed cryptographic
identity to these same bytes. Trace-v0 FNV-1a-128 and the doctor fingerprint
remain explicitly legacy/internal compatibility checks, never adversarial or
cross-party content authentication.

### What the two identities are and what they cover

Both are constructed only inside `crates/vh-multiverse/src/observation.rs`
and strictly decoded by `crates/vh-multiverse/src/observation/decode.rs`;
`crates/vh-cli/src/main.rs:701-713` is the one external caller, and it only
consumes the public `decode_end_state`/`validate_complete_observation`
decoders as a self-check inside `vh doctor` — it never constructs an
identity itself.

- **`vh-end-state-observation-v1`** (`EndStateIdentity`,
  `crates/vh-multiverse/src/observation.rs:58-88`) is the raw, unjudged
  state a workload declared via `ctx.declare_end` — exactly the map
  `EndStateOracle::check` reads, before any oracle verdict is taken. It is
  one canonical field, `state`, an ordered `key → value` string map
  (`crates/vh-multiverse/src/observation.rs:74-87`). This is what closes
  A.1: passing the same oracle can no longer hide a different raw value,
  because the raw value is itself part of identity (see the adversarial
  tests cited above).
- **`vh-complete-observation-v1`** (`CompleteObservationIdentity`,
  `crates/vh-multiverse/src/observation.rs:93-108`) covers all twelve
  fields listed in `COMPLETE_FIELDS`
  (`crates/vh-multiverse/src/observation.rs:35-48`), which is exactly the
  numbered "Observable identity" list above minus the schema-bytes item
  itself: universe ID, trace hash, trace event count, the ordered
  always-check transcript, ordered always-failures, the sometimes map,
  lifecycle (outcome + fault-plan retrieval discipline), the fault-plan
  digest, runtime evidence (the Offered→Armed→Injected→Manifested→Recovered
  ladder or `None`), schedule policy, the decision-tape digest (or `None`),
  and the end-state identity bytes themselves, nested whole
  (`crates/vh-multiverse/src/observation.rs:125-170`). `UniverseResult`
  constructs both and exposes them as read-only getters
  (`crates/vh-multiverse/src/lib.rs:449-462,537-548`); building them is
  internal to the runner, so no downstream caller can forge or omit a
  field.
- Both share the `vh-canonical-length-framing-v1` framing algorithm
  described above. The strict decoders
  (`decode_end_state`, `crates/vh-multiverse/src/observation/decode.rs:52-70`;
  `validate_complete_observation`,
  `crates/vh-multiverse/src/observation/decode.rs:75-99`) reject truncated,
  malformed-UTF-8, wrong-magic/algorithm/schema, duplicate/unknown/reordered
  field, wrong-kind, non-canonical-map, and trailing-byte input — every
  `DecodeError` variant is exercised by
  `strict_parser_rejects_duplicate_unknown_reordered_and_trailing_fields`
  and `strict_parser_rejects_duplicate_reordered_truncated_and_malformed_state`
  (`crates/vh-multiverse/src/observation/decode.rs:377-416,418-457`), plus an
  unstructured-fuzz smoke test that asserts only "no panic"
  (`malformed_probe_never_panics`,
  `crates/vh-multiverse/src/observation/decode.rs:459-475`).

### Decision tape

The decision tape (`DecisionTape`, `crates/vh-trace/src/lib.rs:44-52` for
the type, `:113-157` for its impl block) is a second, additive,
append-only event stream — schema `vh-decision-tape-v1`
(`crates/vh-trace/src/lib.rs:114`) — built on the same length-prefixed
FNV-1a-128 `Trace` primitive as the v0 execution trace, but as a wholly
separate instance with its own chain state. Recording a scheduler
same-timestamp choice (`record_decision(site_id, candidate_set_digest,
chosen_index, policy_id)`,
`crates/vh-trace/src/lib.rs:125-140`) never touches the execution trace or
its hash; this is deliberate, so scheduler-choice recording could land
without mutating `TRACE_FORMAT_V0.md`'s frozen surfaces
(`crates/vh-trace/src/lib.rs:44-47`). `digest_hex()`
(`crates/vh-trace/src/lib.rs:142-144`) is the tape's own chained hash,
distinct from any execution-trace hash
(`decision_tape_digest_is_not_the_empty_trace_hash`,
`crates/vh-trace/src/lib.rs:245-247`) and stable for a given decision
sequence (`decision_tape_has_stable_separate_digest`,
`crates/vh-trace/src/lib.rs:227-242`).

`UniverseResult::decision_tape_digest()` exposes this digest as
`Option<&str>` — `None` iff the universe never constructed the sim runtime
(the legacy workload-drained path)
(`crates/vh-multiverse/src/lib.rs:528-535`) — and it is item 11 of the
"Observable identity" list and the `decision-tape-digest` field of
`vh-complete-observation-v1`
(`crates/vh-multiverse/src/observation.rs:46,157-161`). Recording is
OPT-IN and costs measured wall time, so the default FIFO path pops
un-recorded and bit-for-bit; the sole runtime pop site records exactly one
`runtime.step` choice per pop when `record_tape` is on, for any of the
Fifo/Pct/Uniform strategies (`crates/vh-multiverse/src/runtime.rs:616-664`,
digest captured at `:953-954`). The tape is the replay/causality
substrate for scheduler-choice work, not itself a scheduler-guidance
claim — see `docs/governance/ACTIVE_TRACK.yaml`'s
`vibe-halt-1000x-exploration` disposition for the (falsified palette,
unmeasured-schedule-guidance) status. `SchedulePolicy::Pct` still means
the event-priority (PCT-inspired) strategy PR #23 narrowed to a measured
null on VB-006, not a validated thread-PCT guarantee — that disposition
is unchanged by C1 and is not reopened here.

### What remains UNCHECKED, not D0

The canonical encoder in `observation.rs` is a faithful, injective
transport of whatever typed values the runner already extracted (`u64`,
`String`, ordered maps, explicit tag bytes); by construction it cannot
itself invent a `Debug` rendering, a host address, or unordered iteration
(see "Canonical observation bytes" above). That does **not** prove every
`String` payload flowing INTO it is clean. If a workload's own code ever
formatted a raw pointer, an address-bearing value hidden behind a generic
`Debug`/`Hash` bound, a type-erased trait object, or a nested container,
and that text reached `ctx.declare_end`, `ctx.record`, or a failure
detail, the canonical bytes would faithfully — and misleadingly —
transport it as if it were ordinary deterministic content.

C1 hardened the determinism deny-list's line-regex layer to reject
*literal* raw-pointer syntax, `std::ptr`/`core::ptr` module and
provenance-API use, and `std::fmt::Pointer`/`{:p}` formatting in every
kernel crate
(`scripts/check_determinism_denylist.py:84-90,175-179`, exercised by the
self-test table added in the same change). It explicitly does **not**
close four type-hidden classes, documented in the scanner's own docstring
(`scripts/check_determinism_denylist.py:35-41,51-61`):

1. a generic `Debug`/`Hash` value instantiated with an address-bearing type;
2. a type-erased trait object whose concrete value carries an address;
3. a nested container whose derived formatter/hasher reaches such a value;
4. a user-defined formatter/hasher that emits address identity without any
   raw-pointer token at the call site.

Per `docs/specs/DETERMINISM_TIERS.md:19-23`, that class stays explicitly
`UNCHECKED` pending a separately reviewed type-aware gate — the whole
safe-Rust kernel surface is **not** claimed deterministic "by
construction" for it, and no doc in this repo should say `D0`/`CLEAN` for
this class until that gate lands. Float NaN/precision formatting is a
pre-existing, separately accepted regex-layer limitation of the same kind
(`scripts/check_determinism_denylist.py:59-61`).

### Changelog

- **2026-07-23 (post-audit C1):** raw oracle-consumed end state, schedule
  policy, decision-tape digest, and the versioned complete-observation bytes
  joined `UniverseResult` identity. The doctor fingerprint migrated
  `vh-doctor-observable-v3` (`1684e7c347e645f43a80a30abc46adb7`) →
  `vh-doctor-observable-v4` (`669b4cdef41ede292761c5a47cd69f37`). The
  v4 doctor hashes the complete canonical bytes using the existing
  legacy/internal trace hasher and an explicit schema-v4 XOR finalizer that
  preserves the already-published lost-package vector. The finalizer is a
  bijective compatibility transform, not cryptography or the persisted
  evidence digest. The trace identity
  (`9ce6199f133f4d3c9dd0da0075e352d2`, 45 events) is unchanged.

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

- FNV-1a is legacy/internal and not cryptographic. It is acceptable for trace
  compatibility and sampled divergence checks between runs we control; it is
  not an adversarial content identity. Persisted/cross-party evidence hashes
  the versioned canonical complete-observation bytes under its separately
  reviewed algorithm.
- Traces are in-memory only. Phase 2 adds spill-to-disk JSONL with the
  same hash chain, under `~/.vibe-halt/traces/`.
- The canonical observation encoding faithfully transports whatever typed
  values the runner extracted; it cannot prove those values were never
  derived from an address upstream (generic `Debug`/`Hash`, a type-erased
  trait object, or a nested container carrying such a value). See "What
  remains UNCHECKED, not D0" above — that class is not closed by C1.
