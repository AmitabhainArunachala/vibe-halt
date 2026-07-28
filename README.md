# vibe-halt

**Mega Hyper Vibration Multiverse Halting Machine** — a deterministic
simulation-testing rig for agent-shaped state machines. It runs modeled
workloads across reproducible universes, injects semantic faults, evaluates
executable properties, and emits content-addressed, fail-closed replay
evidence with resource-bounded shrinking (runnable proof: `make gate`).

Vision: [`VISION.md`](VISION.md) · 12-week contract:
[`docs/plans/VIBE_HALT_BUILD_PLAN_2026-07-20.md`](docs/plans/VIBE_HALT_BUILD_PLAN_2026-07-20.md)
· Determinism doctrine:
[`docs/specs/DETERMINISM_TIERS.md`](docs/specs/DETERMINISM_TIERS.md)
· Current long-running goal:
[`docs/prompts/VIBE_HALT_REALITY_BRIDGE_LONG_RUNNING_GOAL_2026-07-29.md`](docs/prompts/VIBE_HALT_REALITY_BRIDGE_LONG_RUNNING_GOAL_2026-07-29.md)

## Current boundary

Demonstrated now:

- Tier 1/D0 engine-owned simulation with deterministic scheduling, network,
  disk, fault injection, properties, and complete-observation comparison
  (runnable proof: `make gate`);
- strict evidence bundles, standalone semantic replay, content-digest
  self-consistency checks, and exact-fingerprint shrinking for the currently
  capture-enabled demo workloads (`scripts/gate.sh:397-479`);
- a Tier 2/D2 subprocess harness with child-visible cassette replay and a
  published 100-pair reference campaign (`scripts/gate.sh:104-190`);
- eleven pinned regression-corpus entries (runnable count:
  `find corpus/entries -maxdepth 1 -type f -name 'VB-*' | wc -l`).

Not yet demonstrated:

- a production Python or `dharma_swarm` adapter
  (`clients/python/vibe_halt/core/runner.py:10-30`);
- arbitrary foreign repositories as deterministic targets
  (`docs/specs/DETERMINISM_TIERS.md:39-63`);
- D1 subprocess containment or Tier 3 hypervisor determinism
  (`docs/specs/DETERMINISM_TIERS.md:45-71`);
- any previously unknown, independently human-confirmed bug
  (`docs/audits/REACH_STRATEGY_DEBATE_PACKET_2026-07-25.md:64-71`).

The D2 harness records all 29 capability channels as open. Its clean reference
campaign is useful evidence inside that stated boundary, not a D1 certificate
(`docs/specs/SANDBOX_CAPABILITY_ENVELOPE_V1.md:16-33`).
The Python runner refuses execution, but the package still exports legacy
Python-side simulator and caller-constructible evidence surfaces; none is a
trusted engine or result path. Wave A removes or package-excludes them while
building the strict Rust-backed adapter
(`clients/python/vibe_halt/core/runner.py:10-30`,
`clients/python/vibe_halt/__init__.py:5-9`,
`clients/python/vibe_halt/core/evidence.py:24-62`).

## Quickstart

The workspace uses the Rust toolchain pinned in
[`rust-toolchain.toml`](rust-toolchain.toml).

```bash
make onboard
make test
make gate
make demo

cargo run -p vh-cli -- doctor
cargo run -p vh-cli -- run --workload demo --universes 200
cargo run -p vh-cli -- run --workload demo-buggy --universes 100 --shrink
cargo run -p vh-cli -- run --workload demo-buggy --universes 100 --out /tmp/vh-evidence
finding_path=$(find /tmp/vh-evidence/findings -name finding.ndjson -print -quit)
cargo run -p vh-cli -- replay-bundle "$finding_path"
cargo run -p vh-cli -- sandbox-campaign --mode reference --pairs 100
```

`vh run` exits 0 only when the multiverse is divergence-checked, every
universe validly completes, the non-empty property contract is satisfied, and
there are no findings. A finding-free single replay or a run without the
divergence check is `UNCHECKED` (exit 3), never `CLEAN`.

Evidence output directories must be new or empty. Replay verifies the recorded
schema, content-digest self-consistency, lineage, execution observations, and
finding identity before it reports `REPRODUCED`. The bundles are not signed
and do not authenticate source provenance
(`crates/vh-cli/src/receipts_v2.rs:26-31`).

## Layout

| path | responsibility |
|---|---|
| `crates/vh-core` | seed tree, PRNG streams, virtual clock, deterministic scheduler |
| `crates/vh-trace` | append-only chain-hashed event trace |
| `crates/vh-gremlin` | semantic fault types and deterministic fault plans |
| `crates/vh-props` | always/sometimes property monitors |
| `crates/vh-multiverse` | simulated runtime, universe runner, complete-observation divergence detector |
| `crates/vh-sandbox` | bounded subprocess execution, capability ledger, cassette protocol |
| `crates/vh-digest` | dependency-free SHA-256 evidence primitive |
| `crates/vh-shrink` | resource-bounded exact-fingerprint fault-plan shrinking |
| `crates/vh-verify` | independent vectors, models, replay soak, and cross-platform verification |
| `crates/vh-cli` | `vh` commands, workloads, receipts, bundles, and sandbox campaigns |
| `clients/python` | quarantined integration surface; strict Rust-backed adapter is the next milestone |
| `corpus` | regression entries, provenance, schemas, and harvesting playbook |
| `docs/governance` | WIP-limited, surface-owned active-track portfolio |
| `scripts` | onboarding, governance, deny-list, and the single gate battery |

The Rust workspace has zero external crate dependencies by design: builds are
offline and toolchain-pinned (runnable proof: `make gate`).

## The two laws

1. **Determinism boundary honesty.** Kernel crates exclude wall clock, OS
   randomness, hash-order iteration, threads, and I/O. Every cross-boundary
   receipt names its tier and uncontrolled channels.
2. **Divergence honesty.** Tier-1 checked universes run in two non-adjacent
   passes and complete observations must agree. Tier 2/D2 uses an adjacent
   bounded run-twice pair and publishes divergence
   (`crates/vh-sandbox/src/lib.rs:439-447`). Either finite pair is a sampled
   falsifier. The Tier-1 claim rests on the D0 boundary, independent reference
   vectors, and cross-platform verification—not on replay agreement alone.

## Integration direction

The next integration is a narrow Python transport that invokes an explicitly
configured Rust engine whose exact artifact identity must be verified, and
accepts only engine-verified receipts. A future
`VibeHaltSandbox` may implement the `dharma_swarm` sandbox interface, but it
must not parse human-oriented stdout, infer a clean verdict, or claim a
stronger tier than the receipt supports.

## Governance

`make onboard` reconstructs the current session from
[`docs/governance/ACTIVE_TRACK.yaml`](docs/governance/ACTIVE_TRACK.yaml).
The project uses surface ownership, a three-track WIP limit,
citation-or-silence, one shared gate implementation, draft PRs, and human-only
merges. Green checks are evidence, not approval.

Full name: **Mega Hyper Vibration Multiverse Halting Machine**. Short name:
`vibe-halt`. Binary: `vh`.
