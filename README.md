# vibe-halt

**Mega Hyper Vibration Multiverse Halting Machine** — an AI-native adversarial
verification environment for whole repositories, applications, features, and
high-consequence workflows. Its strongest demonstrated engine today runs
modeled agent-shaped state machines across reproducible universes, injects
semantic faults, evaluates executable properties, and emits content-addressed,
fail-closed replay evidence with resource-bounded shrinking (runnable proof:
`make gate`). The broad product contract does not turn untested surfaces into
proof.

Product lock: [`docs/specs/PRODUCT_LOCK_V1.md`](docs/specs/PRODUCT_LOCK_V1.md)
· Vision: [`VISION.md`](VISION.md) · 12-week contract:
[`docs/plans/VIBE_HALT_BUILD_PLAN_2026-07-20.md`](docs/plans/VIBE_HALT_BUILD_PLAN_2026-07-20.md)
· Determinism doctrine:
[`docs/specs/DETERMINISM_TIERS.md`](docs/specs/DETERMINISM_TIERS.md)
· Current long-running goal:
[`docs/prompts/VIBE_HALT_REALITY_BRIDGE_LONG_RUNNING_GOAL_2026-07-29.md`](docs/prompts/VIBE_HALT_REALITY_BRIDGE_LONG_RUNNING_GOAL_2026-07-29.md)
· Delivery workflow:
[`docs/DEVELOPMENT_WORKFLOW.md`](docs/DEVELOPMENT_WORKFLOW.md)
· GitHub project:
[`vibe-halt — Evidence to Reality`](https://github.com/users/AmitabhainArunachala/projects/1)

## Current boundary

The product accepts a broad target envelope; the implementation currently
supports only the capabilities listed below. The planned target-level
`HALT`/`PROCEED`/`UNKNOWN` decision is separate from the existing engine-layer
`CLEAN`/`FINDINGS`/`UNCHECKED`/error outcomes and has not yet shipped
(`docs/specs/PRODUCT_LOCK_V1.md`, "Decision contract").

Demonstrated now:

- Tier 1/D0 engine-owned simulation with deterministic scheduling, network,
  disk, fault injection, properties, and complete-observation comparison
  (runnable proof: `make gate`);
- strict evidence bundles, standalone semantic replay, content-digest
  self-consistency checks, and exact-fingerprint shrinking for the currently
  capture-enabled demo workloads (`scripts/gate.sh`);
- a Tier 2/D2 subprocess harness with child-visible cassette replay and a
  published 100-pair reference campaign (`scripts/gate.sh`);
- eleven pinned regression-corpus entries (runnable count:
  `find corpus/entries -maxdepth 1 -type f -name 'VB-*' | wc -l`).

Not yet demonstrated:

- a production deployment, a real `dharma_swarm` adapter, or one independently
  confirmed foreign-target result (the local Python client exercises only the
  compiled demo/cooperative fixture registry; runnable state: `gh issue view 60`);
- arbitrary foreign repositories as deterministic targets
  (`docs/specs/DETERMINISM_TIERS.md:39-63`);
- D1 subprocess containment or Tier 3 hypervisor determinism
  (`docs/specs/DETERMINISM_TIERS.md:45-71`);
- any previously unknown, independently human-confirmed bug
  (`docs/audits/REACH_STRATEGY_DEBATE_PACKET_2026-07-25.md:64-71`);
- an independently curated, pre-registered acceptance holdout of at least 25
  provenance-qualified real defects — the criterion-3 denominator
  (`VISION.md`, "Honest external evaluation";
  `docs/plans/VIBE_HALT_BUILD_PLAN_2026-07-20.md:31`).

The D2 harness records all 29 capability channels as open. Its clean reference
campaign is useful evidence inside that stated boundary, not a D1 certificate
(`docs/specs/SANDBOX_CAPABILITY_ENVELOPE_V1.md:16-33`).
The local Python package exposes a narrow request/runner/typed-result surface.
It snapshots an explicitly configured engine, maps only strict Rust verifier
records, and treats Python outcomes as caller-process data rather than an
authority seal. A missing engine trust root prevents promotion: an otherwise
admissible record is exposed as `UNCHECKED`/`UNTRUSTED`, while invalid evidence
remains `ERROR`. This is a hardened local integration slice, not a production
or `dharma_swarm` bridge.

## Quickstart

The workspace uses the Rust toolchain pinned in
[`rust-toolchain.toml`](rust-toolchain.toml).

```bash
make onboard
make test
make gate
make review
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
| `clients/python` | strict, stdlib-only local Rust-backed adapter; no Python-side verdict authority or production/`dharma_swarm` claim |
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
   (`crates/vh-sandbox/src/lib.rs:771-802`). Either finite pair is a sampled
   falsifier. The Tier-1 claim rests on the D0 boundary, independent reference
   vectors, and cross-platform verification—not on replay agreement alone.

## Integration direction

The narrow local Python transport invokes an explicitly configured Rust engine,
binds the admitted request and observed engine image, and accepts only closed
Rust machine records after fresh replay. Its additive cooperative-v2 path also
queries a same-copy, digest-bound operation manifest, revalidates the complete
mandatory feature closure at execution, and binds a fresh Rust observation of
the staged fixture source into a versioned receipt. Its `BoundRequired`
operation succeeds only for an exact caller-supplied SHA-256 coordinate;
explicit `unknown` is a zero-execution typed refusal
(`crates/vh-cli/src/protocol.rs`; runnable check:
`cargo test -p vh-cli negotiated --locked --offline`). This remains local
Tier-2/D2 with an open observation-to-execution channel; legacy cooperative v1
remains explicitly unbound. The v2 `executions` field counts sandbox-run
attempts admitted immediately before the sandbox boundary: zero proves that
boundary was not invoked, while a positive value is only an upper bound and is
not spawn, loader, or child-start attestation. No `dharma_swarm` adapter or
foreign-target receipt is implemented. Any future `VibeHaltSandbox` must not
parse human-oriented stdout, infer a clean verdict, or claim a stronger tier
than the receipt supports.

## Governance

`make onboard` reconstructs the current session from
[`docs/governance/ACTIVE_TRACK.yaml`](docs/governance/ACTIVE_TRACK.yaml).
The project uses surface ownership, a three-track WIP limit,
citation-or-silence, one shared gate implementation, draft PRs, and human-only
merges. Green checks are evidence, not approval.

GitHub issues and the linked project board are the canonical delivery queue.
Every implementation starts from an accepted issue, publishes a draft PR,
runs `make review` and `make gate`, addresses review threads with replies, and
stops at human merge. See
[`docs/DEVELOPMENT_WORKFLOW.md`](docs/DEVELOPMENT_WORKFLOW.md).

Full name: **Mega Hyper Vibration Multiverse Halting Machine**. Short name:
`vibe-halt`. Binary: `vh`.
