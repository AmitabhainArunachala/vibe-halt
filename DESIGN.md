# Mega Hyper Vibration Multiverse Halting Machine (vibe-halt)

**Merged Master Spec v0.1**  
Synthesized from frontier model responses (Claude + Codex) + Grok synthesis  
Date: 2026-07-20

## 1. Project Vision

Build a high-stress, deterministic simulation testing system for **vibe-coded (AI-generated) code and agent systems**.

It acts as a software equivalent of an electrodynamic shaker table + HALT rig + multiverse explorer:
- Execute targets across thousands of reproducible universes
- Inject targeted, intelligent faults at semantic boundaries
- Enforce strong integrity properties
- Produce rich, minimal, reproducible evidence
- Integrate cleanly with existing multi-agent systems (e.g. Dharma Swarm)

**Core Principle**: Radical honesty about determinism boundaries.

## 2. Determinism Model (Graded Claims)

We use explicit determinism grades instead of pretending full determinism is possible for arbitrary code.

| Grade | Name                    | Boundary                                      | Valid Claim                                      | Notes |
|-------|-------------------------|-----------------------------------------------|--------------------------------------------------|-------|
| D0    | Closed Simulation      | Engine-owned actors + modeled effects        | Bit-identical replay for pinned engine/target   | Gold standard for instrumented code |
| D1    | Cooperative / Hermetic | Instrumented target + explicit SDK boundaries| Exact replay of controlled effects; taints on unmanaged entropy | Primary target for v0.1 |
| D2    | Opaque Process         | Arbitrary command / container                | Repeatable workloads + fault plans only         | Chaos testing only; never certified as deterministic |

**Key Rules**:
- Every campaign and evidence bundle must declare its determinism grade.
- Uncontrolled entropy must be rejected or explicitly **tainted** (never silently certified).
- Divergence detector runs every universe twice by default in CI.

## 3. Core Architecture

### 3.1 Decision Tape (Source of Truth)

A normalized, append-only, content-hashed event trace is the single source of truth for every universe.

Each record contains:
- Stable semantic event ID
- Virtual timestamp
- Decision kind + site
- Enabled set digest
- Fault opportunity taken
- Normalized effect
- Property state deltas

Replay validation checks engine, target, configuration, toolchain, decision tape, fault tape, and final state digest.

### 3.2 Domain-Separated Randomness (Seed Tree)

Randomness is derived via PRF from:
`universe_id + decision_kind + stable_site_id + occurrence + enabled_set_digest`

This prevents unrelated new random draws from perturbing reproducibility across versions or fault additions.

### 3.3 Virtual Time + Deterministic Scheduler

Single logical thread per universe.
At each step:
1. Collect enabled events at earliest virtual time
2. Sort by stable semantic ID
3. Select via recorded DecisionSource
4. Execute actor to next yield boundary
5. Convert effects to future events
6. Allow fault engine to modify effects
7. Update properties and coverage
8. Append normalized trace record

### 3.4 Fault Injection

Faults operate at **semantic boundaries**, not arbitrary bit flips or sleeps.

Initial families:
- Messaging (delay, drop, duplicate, reorder, partition, lost ack)
- Process (crash, restart, hang, cancellation)
- Storage (failed read/write, torn write, stale read)
- Time (offset, drift, deadline races)
- Provider/Tool (timeout, 429, malformed, truncated, duplicate/late response)
- Resource & Integrity (quota, credential expiry, receipt mismatch)

Faults are typed, have preconditions, budgets, and recovery semantics.

Intelligent targeting: bias toward state transitions, retry/ack/commit paths, and novel coverage.

## 4. Property System

Properties are stateful monitors evaluated after relevant transitions.

Supported styles:
- `always(P)` / `never(P)`
- `at_most_once(key)`
- `after(A).within(ticks, B)`
- Conservation / monotonicity
- Reachability / sometimes(P)
- Metamorphic relations
- Reference model equivalence

Verdict types: Violated, NotFalsified, Inconclusive, InvalidAssumption, NonReplayable.

Properties can be extended to accept Dharma Swarm **telos gates** as first-class oracles in later versions.

## 5. Multiverse Exploration & Shrinking

Exploration order (v0.1):
1. Independent seeded universes
2. Single-fault sweeps
3. Bounded pairwise combinations
4. Novelty-guided decision-tape mutation
5. Preemption-bounded schedule perturbation

**Shrinking** is hierarchical:
1. Remove workload actions
2. Remove fault families
3. Remove individual fault occurrences
4. Reduce arguments and delays
5. Remove scheduling decisions

Every shrunk candidate is re-validated by replay.

## 6. Integration with Dharma Swarm

Integration is narrow and high-value:
- `VibeHaltSandbox` implementation of the existing sandbox interface
- Verdict + minimal repro receipt written under `~/.dharma/`
- Can feed `DarwinEngine.gate_check` as a mechanical, uncharmable check

Live LLM calls are always outside the deterministic core. Use record/replay cassettes + mutation.

## 7. 12-Week Phased Roadmap

**Weeks 1–2**: Architecture freeze, determinism grades, threat model, trace schema, bakeoff (owned kernel vs MadSim/Turmoil).
**Weeks 3–5**: Core execution engine + virtual time + messaging/storage/time/provider faults.
**Weeks 6–8**: Property system + mutation corpus + hierarchical shrinker.
**Weeks 9–10**: Parallel campaign runner + adaptive exploration + Python protocol/adapter.
**Weeks 11–12**: Dharma integration slice, independent reviews, 100k-universe soak, release.

Feature freeze at end of Week 9.

## 8. Measurable Success Criteria (Week 12)

1. Determinism: ≥1,000 consecutive D0 runs produce identical normalized trace hashes across machines.
2. Every retained failure replays 100/100 times.
3. Divergence rate on reference D1 workloads is measured and published (<5% target).
4. ≥12 meaningful fault families implemented across messaging/process/storage/time/provider/integrity.
5. ≥12 realistic AI/agent defects in corpus achieve ≥10/12 kill rate within fixed universe budget.
6. Shrinker reduces representative failures by median ≥70% in event/fault count.
7. Throughput: ≥10,000 Tier-1 universes / hour on 8-core commodity hardware.
8. One real pinned Dharma control-plane slice demonstrates lost acks, duplication, restart, provider faults, and receipt/authority checks.
9. Every evidence bundle binds source, build, engine, configuration, decision tape, fault tape, trace, verdict, and minimization lineage.
10. Core is lint-clean, dependency-audited, fuzzed at boundaries, and contains no unaudited unsafe.
11. Independent reviewer can clone, run campaign, reproduce finding, and verify evidence in ≤30 minutes.

## 9. Risks & Mitigations

- **Determinism holes**: Divergence detector is CI gate #1 from Week 2. Deny-lists for nondeterministic APIs.
- **Scope creep to hypervisor**: Explicitly out of scope for v0.1. Trace/property layer designed to be substrate-agnostic for future backends.
- **Weak properties**: Mutation corpus + independent property review.
- **AI-generated implementation debt**: Small reviewable changes + human merges + external reviews at weeks 1, 6, and 11.
- **Operator bus factor**: Every session ends with committed state doc.

## 10. Budget Allocation ($10,000)

- Senior Rust/distributed-systems review (architecture + midpoint + release): $3,500
- Independent security + reproducibility review: $1,500
- Frontier model inference: $2,000
- Compute (multi-core runner + fuzzing): $1,200
- Contingency + corpus: $1,800

Spending gates protect the determinism kernel and final review.

## 11. Out of Scope for v0.1

- Full deterministic hypervisor (bhyve / KVM / Firecracker level)
- Arbitrary unmodified binaries or containers as D0/D1
- Distributed multi-node simulation
- Time-travel debugging or general VM snapshots
- Broad black-box testing of live systems

## 12. Governance

This is a living document. All major design decisions must be recorded here with rationale and date. Changes after Week 2 require explicit justification and ratcheting.

**Primary sources synthesized**:
- Claude response (2026-07-20)
- Codex response (2026-07-20)
- Grok synthesis

---

*This machine does not solve the halting problem. It can halt a bounded campaign on violation, quiescence, timeout, resource budget, or exploration exhaustion.*