# Determinism Tiers — the honesty doctrine

The single most important design decision in vibe-halt: there are three
tiers of determinism, we ship two, and every verdict states which tier
produced it. A "deterministic" simulator that quietly isn't is worse than
none — it manufactures false confidence.

## Tier 1 — Full determinism (shipped: kernel; Phase 1: sim runtime)

Code that runs against the simulated runtime: virtual clock
(`crates/vh-core/src/clock.rs`), named PRNG streams from the seed tree
(`crates/vh-core/src/seed.rs`), deterministic scheduler
(`crates/vh-core/src/sched.rs`), simulated network/disk (Phase 1).

Guarantee: same root seed ⇒ bit-identical trace hash, forever, on any
machine with the pinned toolchain. Enforced by the frozen PRNG reference
vector test and the CI divergence gate.

## Tier 2 — Hermetic reproducibility (Phase 1-2)

Arbitrary code (including AI-generated Python) in a hermetic subprocess
sandbox: fixed seeds, virtual/faked clock, recorded-replay LLM cassettes,
fault-injecting network and filesystem interposition.

Guarantee: the *environment* is deterministic; interpreter scheduling is
not. So every universe runs twice and trace hashes are compared — the
divergence detector (`crates/vh-multiverse/src/lib.rs`) reports the
divergence rate instead of hiding it. A Tier-2 verdict always carries
that rate.

## Tier 3 — Hypervisor determinism (explicit non-goal)

Antithesis-class whole-VM determinism. Out of scope for the 12-week
build: it is a multi-year effort at any quality level. The trace/oracle/
property layers are substrate-agnostic so a hypervisor (or rr-based
record-replay) backend can slot underneath later without touching the
property system.

## The rule

Every report, receipt, and PR that cites a vibe-halt result names the
tier. "Deterministic" without a tier number is an uncited claim.
