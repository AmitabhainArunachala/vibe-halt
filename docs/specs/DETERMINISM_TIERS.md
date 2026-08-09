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
machine with the pinned toolchain. Gate 0 enforces structural manifest and
rustc unsafe-code rules plus a syntactic deny-list, while the frozen PRNG
vectors, doctor identity, and pairwise replay-agreement gate FALSIFY BY
SAMPLING. The line scanner is not type-aware: generic, trait-object, and
nested-container formatting/hashing can hide address-bearing values. That
class remains explicitly UNCHECKED pending a separately reviewed type-aware
gate, so the whole safe-Rust surface is not claimed deterministic "by
construction." A finite replay sample can refute the claim, never prove it —
reports therefore carry the evidence name
"pairwise replay agreement", not a tier proof (hardening-loop-4
BLOCKER 2).

## Tier 2 — subprocess evidence (D2 shipped; hermetic D1 future)

**D1 target state:** arbitrary code (including AI-generated Python) in a hermetic
subprocess sandbox with fixed seeds, virtual/faked clock, recorded-replay LLM
cassettes, and fault-injecting network and filesystem interposition.

Only after those controls and every relevant capability channel are
mechanically proven closed may a D1 receipt claim exact replay of controlled
effects. Interpreter scheduling still remains outside that guarantee unless
it is separately controlled.

**Current D2 guarantee:** no deterministic environment is claimed. The
controller bounds execution, records the declared world and all observed
outcomes, lists every uncontrolled capability channel, runs the subprocess
twice, and publishes divergence instead of hiding it. A D2 pair that agrees is
evidence inside that exact boundary, never a determinism certificate.

**Current implementation status (2026-07-29, `ab259c07`):** the shipped
boundary is the **Tier-2 D2 subprocess harness; D1 is a future backend**.
`vh-cli` and `vh-sandbox` are the boundary crates; the deterministic kernel
crates remain pure.

The child can now consume an ordered, identity-bound cassette through the C5
cooperative Python fixture. Cassette miss and unconsumed history taint the run
`UNCHECKED`. The C6 reference campaign publishes raw run-twice counts and
records 0 divergent pairs in 100; its leak battery proves that probed leaks
diverge and unprobed channels cannot become false `CLEAN` results
(`scripts/gate.sh:104-190`).

That transport closes no capability channel by itself. Every one of the 29
channels in `vh-sandbox-capability-v1` remains `Open`, so D1 is unreachable
through the public API. Cgroups, netns, fault proxy, clock control, process
tree enforcement, and controller-proven closure remain unimplemented.
The separately proposed C7 supervisor is deferred at 2/14 admission
decisions (`docs/audits/C7_ADMISSION_LEDGER_2026-07-25.md:9-14`). Neither the
cassette result nor a low D2 divergence rate implies D1 coverage.

## Tier 3 — Hypervisor determinism (conditional north star; not v0.1)

Antithesis-class whole-VM determinism. Out of scope for the 12-week
build and the present execution grant: it is a multi-year effort at any quality
level. Product Lock v1 retains a Vibe Halt hypervisor as a conditional
long-term direction only if external fault-yield evidence, resources, and
economics justify it. The trace/oracle/property layers are substrate-agnostic
so a hypervisor (or rr-based record-replay) backend can slot underneath later
without touching the property system.

## Crosswalk to the D-grade vocabulary (DESIGN.md)

The merged master spec (`DESIGN.md` §2, landed on main 2026-07-20) uses
determinism grades D0/D1/D2 for *campaign and evidence claims*. The tiers
in this file are the *engine implementation doctrine*. One taxonomy does
not replace the other; this crosswalk is the canonical mapping:

| DESIGN.md grade | This doc | Meaning |
|---|---|---|
| D0 Closed Simulation | Tier 1 | engine-owned actors on the simulated runtime; bit-identical replay |
| D1 Cooperative/Hermetic | Tier 2 (strong) | instrumented target, controlled effects replayed exactly, unmanaged entropy tainted |
| D2 Opaque Process | Tier 2 (weak) | repeatable workload + fault plan only; chaos testing, never certified deterministic |
| — (no D-grade) | Tier 3 | hypervisor substrate; outside v0.1/current execution, see above |

Evidence bundles cite the D-grade; engine code and receipts cite the
tier; either alone is incomplete for a cross-boundary claim.

## The rule

Every report, receipt, and PR that cites a vibe-halt result names the
tier. "Deterministic" without a tier number is an uncited claim.
