# VIBE-HALT — PHASE-1 REMAINDER: TIER-2/D1 HERMETIC SANDBOX MVP (Track 1)

> **Superseded for core execution upon human merge of C0 by `docs/prompts/VIBE_HALT_POST_AUDIT_TIER2_REACH_LONG_RUNNING_GOAL_2026-07-22.md`.** Retained as historical evidence; this supersedes execution authority only and grants no current core execution authority.

Authored 2026-07-21 by the Grand Orchestrator. You are a fresh Track-1
builder session on `AmitabhainArunachala/vibe-halt` — assume ZERO memory;
every moving fact below must be re-verified before you rely on it. This
document is the full spec; the short `/goal` that dispatched you cites it
by path and never overrides it.

## 0. State of the world (verify each)

- `main` = `41f741ceb0afb3dda723a2da93650c3ea6bc7f92` (tree
  `1bf0cf531ebf3c8cf1ed3a9f71782d84671056f2`), both workflows green at
  that exact head (runs 29816008564 ci, 29816008538 verify — job logs
  reproduce the identities below). If main has moved, re-anchor and note
  the diff in your first receipt.
- Frozen identities on main: doctor trace
  `9ce6199f133f4d3c9dd0da0075e352d2` / 45 events / seed 0xD1CE;
  observable fingerprint `1684e7c347e645f43a80a30abc46adb7`
  (`vh-doctor-observable-v3`). Codex soak:
  `eafa30e8a7a6c82939ea3f755bc866ab` / 33 events
  (`vh-verify-observable-v2`, `39f727ed5d8a949c1f5a1243bd6d1d10`).
  Unexplained drift in ANY of these = STOP publication, bisect, report.
- The verifier track (Codex) is concurrently building its supersede PR
  (`codex/vibe-halt-verify-main`; instructions in PR #8 comment
  5032050846). Its surfaces are radioactive to you: `crates/vh-verify/**`,
  `crates/vh-shrink/**`, `.github/workflows/verify.yml`, `AGENTS.md`.
  Cross-boundary needs are `INTERFACE REQUEST:` PR comments with exact
  signatures and cited call sites — never edits.
- Read IN FULL before writing code: `CLAUDE.md`,
  `docs/plans/VIBE_HALT_BUILD_PLAN_2026-07-20.md` (Phase 1),
  `docs/specs/DETERMINISM_TIERS.md`, `docs/specs/TRACE_FORMAT_V0.md`,
  `DESIGN.md` §2 + the Claude sign-off caveats (DESIGN.md:43-56),
  PR #1 comments 5023079490 + 5024743220, `scripts/gate.sh`.

## 1. Mission

Ship the de-scoped Tier-2/D1 hermetic sandbox MVP — the Phase-1
remainder. Scope is fixed by the ratified plan and the signed de-scope:

- Plan: "Tier-2 hermetic sandbox: subprocess universes … **LLM
  record/replay cassettes**" (docs/plans/VIBE_HALT_BUILD_PLAN_2026-07-20.md:35-37).
- Signed de-scope: "subprocess + pinned env + cassettes; defer
  cgroups/netns hardening … full isolation as Phase-3 work"
  (DESIGN.md:47).
- Cassette law: "Live LLM calls are always outside the deterministic
  core. Use record/replay cassettes + mutation." (DESIGN.md:189).
- Epistemics: Tier 2 guarantees a deterministic *environment*, not
  deterministic execution; every universe runs twice and the divergence
  rate is REPORTED, never hidden (docs/specs/DETERMINISM_TIERS.md:25-36).
  D1 = controlled effects replayed exactly, unmanaged entropy TAINTED;
  anything less is D2 and must say so (docs/specs/DETERMINISM_TIERS.md:55-56).

This MVP runs arbitrary code (first target: Python) in subprocess
universes with a pinned environment and cassette-replayed LLM calls, and
tells the truth about everything it does not control.

## 2. Standing law (restated; binding all session)

1. Human-only merge authority. Never merge, never self-approve, never
   push to main. Push only to `claude/…` branches; PRs stay draft until
   green at their exact head.
2. `bash scripts/gate.sh` green before EVERY push, plus
   `cargo fmt --all --check`; `cargo clippy --workspace --all-targets
   --all-features --locked --offline -- -D warnings -F unsafe-code`;
   `cargo test --workspace --all-targets --all-features --locked
   --offline`; `RUSTDOCFLAGS=-Dwarnings cargo doc --workspace
   --all-features --no-deps --locked --offline`;
   `cargo run -q --locked --offline -p vh-cli -- doctor`.
3. Citation-or-silence; every determinism claim names its tier AND
   D-grade for cross-boundary claims (docs/specs/DETERMINISM_TIERS.md:59-61).
4. Frozen surfaces move only by explicit versioned migration with a
   TRACE_FORMAT_V0.md changelog entry and an independently explained
   semantic cause.
5. The Python-client quarantine stays closed (DESIGN.md:51, sign-off
   caveat 5; negative gate in scripts/gate.sh § python quarantine). The
   sandbox is Rust-owned; nothing in this goal revives
   `clients/python/` execution paths.
6. Zero new external dependencies without a stated cause in the commit
   body; files ~<500 lines; no wall-clock/entropy/unordered iteration in
   anything that feeds an identity.
7. Work belongs to track `vibe-halt-core-2026-07`
   (docs/governance/ACTIVE_TRACK.yaml); if surface additions are needed,
   append-only, and `python3 scripts/check_governance.py` must pass.

## 3. Blocks (in order; MUST 1-3, SHOULD 4, STRETCH 5)

### Block 1 (MUST) — `crates/vh-sandbox`: subprocess universes, pinned env
- `SandboxSpec`: explicit env allowlist (everything else scrubbed),
  pinned `PYTHONHASHSEED`, `TZ=UTC`, `LC_ALL=C`, fixed cwd inside a
  per-universe workspace dir, argv, stdin bytes. Deterministic,
  versioned serialization of the spec (BTree order) digested into a
  `vh-sandbox-spec-v1` identity.
- Execution: spawn, capture exit status + stdout/stderr digests +
  declared artifact files (path allowlist), wall-time as BOUNDARY
  TELEMETRY only — never inside any identity.
- Honesty ledger: a structured list of channels the MVP does NOT
  control (real network, real clock syscalls, filesystem outside the
  workspace, thread scheduling). This ledger is part of the run record
  and is what downgrades a verdict D1→D2. Fail closed: an empty ledger
  claim is a construction error, not a default.
- No cgroups, no netns, no proxy — deferred per DESIGN.md:47. Do not
  build partial versions of them.

### Block 2 (MUST) — LLM record/replay cassettes
- Versioned cassette schema `vh-cassette-v1`: entries keyed by a
  canonical request digest (BTree-ordered serialization of
  provider/model/messages/params); value = recorded response bytes +
  boundary telemetry. Deterministic file layout under the universe
  workspace.
- Record mode: pass-through is OUT OF SCOPE for this MVP (no live LLM
  calls from the rig); recording happens via an offline capture tool fed
  by fixtures. Replay mode: exact-digest match serves the response;
  ANY miss is a fail-closed taint verdict (named finding), never a
  silent live call and never a fuzzy match (DESIGN.md:189).
- Mutation hooks: schema field reserved, implementation deferred
  (STRETCH at most; say so explicitly if touched).

### Block 3 (MUST) — Tier-2 run-twice divergence measurement
- Every Tier-2 universe runs twice; observable records compared;
  divergence RATE published in the campaign summary line exactly as the
  tiers doc demands (docs/specs/DETERMINISM_TIERS.md:30-36). A Tier-2
  verdict line always carries: tier, D-grade (D1 only when every
  effect channel is controlled+replayed and the honesty ledger is
  empty of unmanaged channels; else D2), divergence rate, and the
  evidence name "run-twice agreement (sampled falsifier — not proof)".
- Tier-2 machinery must not touch Tier-1 identity surfaces: the frozen
  doctor path stays byte-identical (regression proving it), and no
  Tier-2 code appears in the deny-list-scanned kernel crates unless the
  scanner's rules are satisfied without new blanket exemptions.

### Block 4 (SHOULD) — honest demo + gates
- `demo-sandbox`: a small fixture Python script with one injected
  nondeterminism source and one cassette-served fake-LLM call. Positive
  gate: replay-clean run with published divergence rate and exact exit 0.
  Negative gates: cassette-miss taints with exact exit 1; injected
  nondeterminism is CAUGHT by run-twice with exact exit 1. All three
  wired into `scripts/gate.sh` as named sections.
- Fixture scripts live under an explicitly exempted fixtures path with
  per-file scanner exemptions (never a blanket relaxation).

### Block 5 (STRETCH) — entropy audit report
- A `vh sandbox audit` subcommand that prints the honesty ledger +
  divergence statistics for a spec, as a receipt-ready block. Skip
  freely if Blocks 1-4 consume the session.

## 4. Cadence & receipts

- Small gate-green commits per invariant; adversarial negative
  regression for every new claim; push per invariant and verify CI on
  the exact pushed SHA (inspect job steps, never aggregate green).
- If any public surface Codex projects from grows (e.g. new
  `UniverseResult` observables), STOP: that is a doctor-migration-class
  event — post the interface note on the newest verifier PR before
  landing it, mirroring PR #2 comment 5024538998's pattern.
- Definition of done: Blocks 1-3 shipped with regressions; gate battery
  green locally AND on the exact final SHA in CI; frozen identity table
  re-verified byte-identical; one closing receipt comment on your PR in
  the house style (blocks shipped vs skipped, file:line citations,
  runnable commands, CI links, queued operator decisions).
- If irreducibly blocked: ONE operator packet (exact blocker, evidence,
  attempts, smallest decision, consequences), then continue safe work.
