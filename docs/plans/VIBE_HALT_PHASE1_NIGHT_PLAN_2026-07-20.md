# Phase-1 Night Campaign — 2026-07-20 → 07-21

One overnight, two autonomous tracks, one goal: convert the hardened
Phase-0 skeleton into a real Phase-1 simulation runtime — runtime-owned
fault injection across all five gremlin channels, the semantic fault
lifecycle shipped early (it is DEFERRED to 2026-08-15 in the loop-4
receipt; tonight beats the date), end-state oracles, and the first
vibe-bug corpus entries. Everything lands as gate-green commits on the
existing draft PRs. **No merges overnight — human-only authority is
unchanged.**

Baseline at planning time: Track-1 head `f6b5e929` (loops 1–4 closed,
CI green, receipts posted); Codex at `42ae5fce` pending rebase; doctor
identity `9ce6199f133f4d3c9dd0da0075e352d2` / 45 events / seed 0xD1CE,
observable fingerprint `cdb049391ddbacc06eb3faf3ea1cb43a`
(`vh-doctor-observable-v2`).

## Prior art this plan borrows deliberately

- **FoundationDB simulation / Antithesis lineage**: the runtime — not
  the workload — owns fault scheduling; workloads only declare
  interaction points. Tonight's W1/W2 move vibe-halt onto that model.
- **TigerBeetle VOPR**: seeded, replayable state-machine fuzzing with
  end-state assertions — the shape of W3's oracles and W4's corpus
  recall tests.
- **madsim / turmoil (Rust)**: simulated network APIs on a deterministic
  scheduler with per-link partitions and message reordering — W1's
  SimNet surface is the zero-dependency equivalent on `vh-core`'s
  `Scheduler<E>` (`crates/vh-core/src/sched.rs` — total order by
  `(VirtualTime, seq)` with a watermark; exactly what deterministic
  delivery needs).

## Non-negotiable guardrails (verbatim from standing law)

1. Frozen surfaces: PRNG output and trace framing NEVER change. The
   frozen demo path stays byte-identical — all new runtime is ADDITIVE
   (new APIs + new workloads). If an observable schema grows, that is an
   explicit doctor v2→v3 migration with a TRACE_FORMAT changelog entry
   and an independently explained cause — never a silent update.
2. Ownership: no edits to `crates/vh-verify/**`, `crates/vh-shrink/**`,
   `.github/workflows/verify.yml`, or Codex's branch. Cross-boundary
   needs are INTERFACE RESPONSE comments on PR #2.
3. `bash scripts/gate.sh` green before every push; small commits grouped
   by invariant; citation-or-silence; every determinism claim tiered;
   files under ~500 lines; zero new dependencies.
4. WIP: opening the corpus track makes 3 ACTIVE tracks — exactly
   `wip_max`. Nothing else opens.

## Workstream W1 — SimNet + SimDisk on the scheduler (MUST)

New module(s) in `crates/vh-multiverse` (or a new kernel crate
`crates/vh-runtime` if size demands — scanner covers new crates fail-
closed automatically):

- `SimNet`: `send(from, to, payload)` / typed delivery events through a
  runner-owned `Scheduler<RuntimeEvent>`; per-link state driven by
  `FaultKind::{NetworkDelay, NetworkPartition}` plus new
  reorder/duplicate faults if cleanly expressible (fault-kind additions
  change no frozen surface — `FaultKind` is plain data).
- `SimDisk`: `write/flush/fsync` with `DiskWriteFail`, torn-write, and
  fsync-lie semantics; volatile vs durable state owned by the runtime so
  crash faults erase exactly the volatile layer.
- Every delivery/IO completion is a trace event recorded by the RUNTIME
  (workloads cannot under-record runtime effects — closes a loop-4
  epistemics thread mechanically).
- New workloads `demo-net` (retry-over-partition echo pair) and
  `demo-disk` (WAL on SimDisk) with property contracts; both wired into
  gate.sh as live gates (clean variant CLEAN, seeded-bug variant
  FINDINGS with exact exit codes).

## Workstream W2 — Semantic fault lifecycle, shipped early (MUST)

With the runtime owning injection, graduate `FaultPlanDiscipline`'s
retrieval vocabulary to the truthful ladder recorded per injection:
`Offered → Armed → Injected → Manifested → Recovered`, as runner
evidence in `UniverseResult` (observable, in equality). This closes the
loop-4 DEFERRED item ~4 weeks ahead of its 2026-08-15 due date. Doctor
migrates v2→v3 with changelog; PR #2 gets the migration note (their
observation-view ratchet makes their side mechanical).

## Workstream W3 — End-state oracles (SHOULD)

`EndStateOracle` in `vh-props`: typed post-run assertions over declared
final state (data integrity across crash/restart), joining
`PropertyContract` so CLEAN can require oracle satisfaction. Re-express
the demo's hand-rolled durability check through it; the frozen demo
trace is unaffected (oracles read state, they do not record events —
verify this explicitly before landing).

## Workstream W4 — Corpus track opening (SHOULD)

Append track `vibe-bug-corpus-2026-07` to ACTIVE_TRACK.yaml (serves
`real-bugs-found`; owned surfaces `corpus/**`,
`crates/vh-cli/src/workloads/**` if split). Deliverables tonight:
corpus entry schema (bug id, class, source, workload, expected finding,
repro command), harvesting playbook, and 3–5 NEW seeded bug workloads
beyond ack-before-flush (candidates: lost-update on concurrent WAL
replay, non-idempotent retry double-apply, read-after-unflushed-write
dirty read, crash-window TOCTOU, fsync-lie durability hole — the last
lands with SimDisk). Each with a recall test: the rig MUST find it
within N universes, exact exit/verdict pinned in gate.sh.

## Workstream W5 — Targeted fault scheduling v1 (STRETCH)

Bias injection times toward state-transition edges: runtime observes
recorded event kinds and concentrates fault offers near kind
transitions (deterministic, seeded, documented). Publish the coverage
delta honestly (found-bug latency on the corpus, before/after). Skip
without guilt if blocks 1–4 consume the night.

## Codex's night (their branch, their charter)

Rebase onto the Track-1 tip; independently re-derive v3 fingerprints;
adopt the `observation()` ratchet with `..`-free destructuring; append
the promised invalid-input `next_bool` vectors; build the evidence
manifest (their open contract); add verifier vectors per new fault
channel as W1/W2 land — the PR #2 thread is the sync bus, as today.
At dawn: hardening loop 5 against the night's work.

## Cadence & failure policy

Commit/push per invariant; post a short PR #2 interface note whenever a
public surface changes so Codex can lockstep. Self-arm a check-in
(~45–60 min) across quiet gaps. If a block stalls: reproduce-or-skip —
never weaken a gate to keep moving; record the skip in the morning
packet. If doctor identity moves unexplained: STOP publication, bisect,
report.

## Morning packet (definition of done for the night)

One PR #1 comment: blocks shipped vs skipped, per-block citations +
regressions + runnable commands, CI links on the exact final SHA,
updated identity table (doctor v3 if migrated, with cause), corpus
inventory, and the precise operator decisions queued for the morning
(foremost: merge PR #1, then PR #2 after Codex's rebase receipt).
