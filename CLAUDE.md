# vibe-halt — Claude Code Configuration

Lean governance, imported from dharma_swarm and cut to what a small
single-purpose repo needs. When this file and the code disagree, the code
is the truth.

## Before Anything Else

Run `make onboard` before any non-trivial Read, Grep, or Edit. It reports
checkout state, toolchain, the declared track, and runs the deny-list gate.
READY is evidence about this local session only — it is not CI admission,
merge approval, or proof any acceptance criterion holds.

## Behavioral Rules (Always Enforced)

- Do what has been asked; nothing more, nothing less.
- ALWAYS read a file before editing it.
- NEVER create files unless necessary; prefer editing existing files.
- NEVER proactively create documentation/README files unless asked.
- NEVER save working files or tests to the root folder.
- NEVER commit secrets, credentials, or .env files.
- **Citation-or-silence.** Every factual claim in a spec, PR body, report,
  or conclusion carries a `file:line` citation or a runnable command.
  Uncited claims carry zero weight regardless of fluency. Prefer
  uncharmable mechanical checks over reviewer vigilance.
- **Runtime receipts never enter git.** Universe reports and corpus runs go
  under `~/.vibe-halt/`, not the repo.

## The Determinism Deny-List (this repo's #1 law)

Kernel crates (`vh-core`, `vh-trace`, `vh-gremlin`, `vh-props`,
`vh-multiverse`) must be pure: no wall clock, no OS randomness, no
HashMap/HashSet iteration, no threads, no I/O, no env access. Enforced
mechanically by `scripts/check_determinism_denylist.py` (CI gate 0).
`vh-cli` is the boundary crate and is exempt. If a legitimate need
collides with the deny-list, the answer is a design change or a
deny-list amendment in the same PR with rationale — never a quiet
workaround.

Two frozen surfaces whose change invalidates every recorded trace hash:
the PRNG output (`crates/vh-core/src/rng.rs` — see
`frozen_reference_vector` test) and the trace hash format
(`docs/specs/TRACE_FORMAT_V0.md`). Changing either is a format version
bump, not a refactor.

## Active Track

Declared intent lives in `docs/governance/ACTIVE_TRACK.yaml` (rendered by
`make onboard`). Before editing a file, check it against the track's
`owned_surfaces`. New project = new track in the YAML, up to `wip_max`.

## Architecture

- Rust workspace, zero external dependencies by design (hermetic builds).
  Adding a dependency to a kernel crate requires a determinism review.
- `crates/vh-core` — seed tree, PRNG streams, virtual clock, deterministic
  scheduler. `crates/vh-trace` — chain-hashed append-only trace.
  `crates/vh-gremlin` — fault plans. `crates/vh-props` — always/sometimes
  properties. `crates/vh-multiverse` — universe runner + divergence
  detector. `crates/vh-cli` — the `vh` binary and demo workloads.
- `clients/python/` — future integration client (dharma_swarm et al.).
  Currently a stub; the Rust core is the only engine.
- Determinism doctrine: `docs/specs/DETERMINISM_TIERS.md`. Build plan:
  `docs/plans/VIBE_HALT_BUILD_PLAN_2026-07-20.md`.

## Build & Test

```bash
make onboard    # session status (run first)
make test       # cargo test --workspace
make gate       # deny-list + tests + live divergence/seeded-bug/detector gates
make demo       # watch the rig catch the ack-before-flush bug
cargo run -p vh-cli -- run --workload demo --universes 200
```

ALWAYS run `make gate` before committing. A red gate is a finding, not an
obstacle: report it, don't route around it.
