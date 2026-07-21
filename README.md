# vibe-halt

**Mega Hyper Vibration Multiverse Halting Machine** — a deterministic
simulation testing (DST) rig for vibe-coded (AI-generated) repositories
and agent systems. An electrodynamic shaker table + HALT rig + multiverse
explorer for code: run it across thousands of reproducible universes,
inject targeted gremlins, enforce integrity properties, and emit findings
with one-command deterministic repros.

Vision: [`VISION.md`](VISION.md) · 12-week plan:
[`docs/plans/VIBE_HALT_BUILD_PLAN_2026-07-20.md`](docs/plans/VIBE_HALT_BUILD_PLAN_2026-07-20.md)
· Determinism doctrine:
[`docs/specs/DETERMINISM_TIERS.md`](docs/specs/DETERMINISM_TIERS.md)

## Quickstart

```bash
make onboard                 # session status — run first
make test                    # full workspace test suite
make gate                    # the whole gate battery
make demo                    # watch the rig catch a seeded durability bug

cargo run -p vh-cli -- run --workload demo --universes 200        # clean
cargo run -p vh-cli -- run --workload demo-buggy --universes 100  # findings + repros
cargo run -p vh-cli -- run --workload demo-buggy --seed 0xD1CE --universe 0  # replay one universe
cargo run -p vh-cli -- doctor
```

`vh run` exits 0 only if the multiverse is clean: no always-failure, no
divergence between the two runs of each universe, every declared
sometimes-assertion reached, every universe validly completed, and the
workload's non-empty property contract satisfied everywhere (a workload
that asserts nothing is UNCHECKED, never CLEAN).

## Layout

| path | what |
|------|------|
| `crates/vh-core` | determinism kernel: seed tree, PRNG streams, virtual clock, deterministic scheduler |
| `crates/vh-trace` | append-only, chain-hashed event trace ([format spec](docs/specs/TRACE_FORMAT_V0.md)) |
| `crates/vh-gremlin` | fault model: gremlin kinds + deterministic fault plans |
| `crates/vh-props` | always-invariants + sometimes-reachability assertions |
| `crates/vh-multiverse` | universe runner, multiverse fan-out, divergence detector |
| `crates/vh-cli` | the `vh` binary + demo workloads |
| `clients/python` | integration client stub (dharma_swarm et al., Phase 4) |
| `docs/governance` | active track portfolio (lean dharma_swarm governance import) |
| `scripts` | onboard + determinism deny-list gate |

The workspace has **zero external dependencies** by design: hermetic,
offline, bit-stable builds on the pinned toolchain
([`rust-toolchain.toml`](rust-toolchain.toml)).

## The two laws

1. **Determinism deny-list** — kernel crates use no wall clock, no OS
   randomness, no hash-order iteration, no threads, no I/O. Enforced
   mechanically in CI (`scripts/check_determinism_denylist.py`).
2. **Divergence honesty** — every universe runs twice, in two
   non-adjacent passes; complete observable results must match or the
   report says DIVERGENT. That replay pair is a **sampled falsifier**:
   agreement is evidence, never proof of determinism (a workload keyed to
   the execution schedule can agree with itself), so the report names its
   evidence "pairwise replay agreement" and the deterministic-substrate
   claim rests on the D0 boundary — gate 0 plus the frozen reference
   vectors — not on the sample.

## Integration with Dharma Swarm

Phase 4 wires vibe-halt into dharma_swarm as a `VibeHaltSandbox`
(implementing the swarm's `Sandbox` ABC) and a diff-verdict gate beside
its build/diff pipeline, emitting tier-labeled receipts. The
`clients/python/` package is that adapter's home.

## Governance

Lean import of [dharma_swarm](https://github.com/AmitabhainArunachala/dharma_swarm)
governance: `make onboard` session status, a WIP-limited surface-owned
track portfolio (`docs/governance/ACTIVE_TRACK.yaml`), citation-or-silence
for all claims, and mechanical gates over reviewer vigilance. Agent
configuration lives in [`CLAUDE.md`](CLAUDE.md).

## Name

Full: **Mega Hyper Vibration Multiverse Halting Machine**.
Short: `vibe-halt`. The binary is `vh`.
