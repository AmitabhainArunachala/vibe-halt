# Codex Master Spec — PR 1 Review + PR 2 Build (vibe-halt)

Ratified 2026-07-20. You (Codex) are the second builder on vibe-halt,
working in sync with Claude (first builder, author of PR #1). This file is
the contract for a long-running task: adversarially review PR #1, then
build PR #2. Read it fully before acting.

Repo: `AmitabhainArunachala/vibe-halt`. Claude's branch (PR #1):
`claude/vibe-halt-simulation-jupmri`. Your branch (PR #2):
`codex/vibe-halt-verify` — create it from the TIP of Claude's branch.

## 0. Ground rules (non-negotiable)

1. **Onboard first.** `make onboard` before any non-trivial read or edit.
   Then read `CLAUDE.md` (governance SSOT — it binds you too),
   `docs/specs/DETERMINISM_TIERS.md`, `docs/specs/TRACE_FORMAT_V0.md`,
   `docs/plans/VIBE_HALT_BUILD_PLAN_2026-07-20.md`.
2. **Citation-or-silence.** Every factual claim you write (review comment,
   PR body, report) carries a `file:line` citation or a runnable command.
   Uncited claims carry zero weight.
3. **Gates are law.** `make gate` green before every push. Never weaken a
   gate, a frozen test vector, or the deny-list to make something pass —
   a red gate is a finding to report, not an obstacle to route around.
4. **Frozen surfaces.** The PRNG output (`crates/vh-core/src/rng.rs`,
   `frozen_reference_vector` test) and the trace hash format
   (`docs/specs/TRACE_FORMAT_V0.md`) are frozen. If your review finds a
   defect in either, that is a BLOCKER finding on PR #1 — report it; do
   not fix it yourself (format changes are version bumps owned by track 1).
5. **Zero new dependencies** in any crate you touch without written
   justification in the PR body. The workspace is hermetic by design
   (`Cargo.toml` workspace comment).
6. **Tier honesty.** Any determinism claim you make names its tier
   (`docs/specs/DETERMINISM_TIERS.md`).

## 1. Surface ownership (the sync mechanism)

You own (create, edit freely):
- `crates/vh-verify/**` (new)
- `crates/vh-shrink/**` (new)
- `.github/workflows/verify.yml` (new)
- `AGENTS.md` (new — thin pointer to CLAUDE.md so future Codex sessions
  auto-onboard)

Append-only edits allowed (flag each in the PR body):
- `docs/governance/ACTIVE_TRACK.yaml` — append your track (§3.1)
- `Cargo.toml` — append your two crates to `members`
- `scripts/check_determinism_denylist.py` — append `crates/vh-shrink` and
  `crates/vh-verify` to `KERNEL_CRATES` (your crates are kernel-grade)

Off-limits (Claude's surfaces; changes flow through §4 protocol):
- `crates/vh-core/**`, `crates/vh-trace/**`, `crates/vh-gremlin/**`,
  `crates/vh-props/**`, `crates/vh-multiverse/**`, `crates/vh-cli/**`
- `Makefile`, `.github/workflows/ci.yml`, `CLAUDE.md`, `README.md`,
  `docs/specs/**`, `docs/plans/**`, `clients/**`

## 2. Phase A — Adversarial review of PR #1

Goal: try to BREAK the determinism claims, not to polish prose. Post one
GitHub review on PR #1 with line-anchored comments. Do not push to
Claude's branch.

Checklist (run everything yourself; cite command + output):
1. `make onboard`, `make gate`, `cargo test --workspace` — do they pass on
   your machine? Any platform-dependent behavior is a finding.
2. **Cross-machine determinism (acceptance criterion #1).** Run
   `cargo run -p vh-cli -- doctor`. On Claude's machine it prints
   universe-0 demo hash `9ce6199f133f4d3c9dd0da0075e352d2` (seed 0xD1CE).
   (Updated 2026-07-20: the trace-framing repair from this review's own
   BLOCKER finding invalidated the original hash
   `7de48a539478aa12bae04f9afca745fc` — see
   `docs/specs/TRACE_FORMAT_V0.md` § Changelog. The first exchange
   MATCHED on the old framing; re-verify on the new one after rebase.)
   If your machine prints anything else, that is the single most important
   finding this review can produce — report it with your exact toolchain.
   Also record hashes for seeds {1, 2, 42, 0xD1CE} × universes {0, 1, 7}
   in your review so future machines can compare.
3. **PRNG correctness.** Verify xoshiro256++ and SplitMix64
   (`crates/vh-core/src/rng.rs`) against the reference implementations
   (Blackman & Vigna reference C, and the canonical SplitMix64). Derive
   expected outputs independently; do not trust the in-repo tests to
   check themselves.
4. **Rejection sampling.** Audit `next_below` for bias/edge cases (n=1,
   n=2^63, n near u64::MAX).
5. **Seed tree.** Hunt for stream-name collisions (FNV-1a 64 on names —
   is accidental collision plausible for realistic name sets?) and for
   universe-seed correlations (adjacent universe ids).
6. **Trace framing.** Try to construct two different event sequences with
   equal hashes given the separator scheme in
   `docs/specs/TRACE_FORMAT_V0.md` (e.g., kind/data containing 0x1F/0x1E
   bytes). If you find one, BLOCKER.
7. **Scheduler.** Confirm same-time determinism claims hold under
   interleaved schedule/pop patterns, not just the batch patterns in
   `crates/vh-core/src/sched.rs` tests.
8. **Divergence detector.** Look for false-negative paths: can a workload
   be nondeterministic in behavior but deterministic in trace hash (i.e.,
   under-recording)? That is a doctrine gap worth a finding.
9. **Deny-list gaps.** Read `scripts/check_determinism_denylist.py` — what
   nondeterminism sources does it miss (e.g., float NaN formatting, ptr
   formatting via `{:p}`, `std::collections::hash_map::RandomState`,
   iterator order of `std::env`-free but OS-touched APIs)? Propose
   additions as review comments; Claude lands them.
10. **Demo honesty.** In `crates/vh-cli/src/workloads.rs`, the buggy
    variant can fail durability even without a crash (unflushed acked
    writes at end-of-run). Assess whether that weakens the demo's claim
    ("crash gremlins expose the bug") and propose the crisper design if so.

Severity labels: `BLOCKER` (kernel correctness/determinism defect),
`GAP` (missing coverage/doctrine hole), `NIT`. Claude fixes findings on
PR #1; you do not wait for fixes to start Phase B (your surfaces are
disjoint).

## 3. Phase B — Build PR #2: the verification battery + shrinker

Branch `codex/vibe-halt-verify` from the tip of
`claude/vibe-halt-simulation-jupmri`. Open PR #2 as draft targeting
`main`, marked "stacked on #1" in the body; after #1 merges, rebase onto
`main`. Small commits, gate-green each.

### 3.1 Track registration
Append to `docs/governance/ACTIVE_TRACK.yaml`:
`id: vibe-halt-verify-2026-07`, serves `deterministic-truth`, owned
surfaces = §1 list, acceptance = D1–D4 below, status ACTIVE.

### 3.2 D1 — `crates/vh-verify`: the metamorphic battery
A crate of integration tests + one binary (`vh-verify`) that attack the
kernel from outside, depending only on the public APIs of the kernel
crates:
- PRNG reference vectors from the official algorithms (independent of
  vh-core's own tests).
- Seed-tree metamorphic properties: stream independence under stream-set
  changes; no adjacent-universe correlation (statistical smoke test with
  fixed seeds, deterministic thresholds).
- Trace framing adversarial tests (separator-byte payloads, empty
  fields, long payloads).
- Scheduler permutation tests: N events scheduled in every order at equal
  times fire in insertion order.
- **Replay soak** (binary): 1,000 sequential replays of reference
  universes; assert one hash; measure and print universes/hour. Output is
  a machine-readable line (`soak: runs=1000 hash=<h> upH=<n>`), tier
  labeled Tier 1. This is the standing evidence for acceptance criterion
  #1 (one-machine half; the cross-machine half is the §2.2 exchange).

### 3.3 D2 — `crates/vh-shrink`: generic fault-plan minimizer
Delta-debugging (ddmin) over `Vec<FaultInjection>` with a caller-supplied
oracle: `fn shrink(plan, oracle: impl FnMut(&FaultPlan) -> bool) -> FaultPlan`
(oracle returns "still fails"). Deterministic: no randomness beyond what
the caller passes; identical inputs ⇒ identical minimal plan. Include
worked tests using synthetic oracles. Do NOT wire it into
`vh-multiverse`/`vh-cli` — the plan-override hook in `UniverseCtx` is
Claude's surface; request it via §4 and design your API so the hook can
adopt it unchanged.

### 3.4 D3 — `verify.yml` workflow
Runs the battery + a 200-replay soak on push/PR/workflow_dispatch. Keep
it independent of `ci.yml` (Claude's file).

### 3.5 D4 — `AGENTS.md`
Root file, <30 lines: "Read CLAUDE.md — it is the governance SSOT for all
agents. Run `make onboard` first. Your track and surfaces live in
docs/governance/ACTIVE_TRACK.yaml." Nothing that duplicates CLAUDE.md
content (duplication rots).

## 4. Sync protocol with Claude

- **Channel:** GitHub PR threads only (PR #1 for review findings, PR #2
  for build discussion). No side channels; every message cited.
- **Interface changes:** if you need anything from a Claude-owned crate
  (e.g., the fault-plan override hook), post a comment on PR #2 titled
  `INTERFACE REQUEST:` with the exact proposed signature and a citation to
  your call site. Claude implements it on track-1 surfaces. Never
  implement it yourself in an off-limits crate.
- **Rebase rule:** you rebase onto Claude's branch tip (or main after
  merge); Claude never rebases onto yours. If a rebase breaks you, that's
  an `INTERFACE REQUEST`, not a reason to fork behavior.
- **Conflict rule:** if you find yourself editing a file outside §1, stop
  — that is the signal the split is wrong; raise it in the PR thread.
- **Hash exchange:** publish your machine's reference hashes (§2.2) in
  the PR #2 body and keep them updated after any rebase that touches the
  kernel.

## 5. Definition of done

- Phase A review posted on PR #1: every checklist item addressed with
  evidence, findings labeled BLOCKER/GAP/NIT.
- PR #2 open (draft), gate-green, containing D1–D4, zero edits outside §1
  surfaces except the three flagged append-only files.
- Cross-machine hash comparison result stated explicitly in the PR #2
  body — match or mismatch, with toolchain details.
- Every claim in both PR bodies cited. Every determinism claim tiered.
