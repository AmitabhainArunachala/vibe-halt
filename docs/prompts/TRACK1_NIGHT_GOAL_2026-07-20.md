VIBE-HALT — PHASE-1 NIGHT CAMPAIGN (Track 1)

> **Superseded for core execution upon human merge of C0 by `docs/prompts/VIBE_HALT_POST_AUDIT_TIER2_REACH_LONG_RUNNING_GOAL_2026-07-22.md`.** Retained as historical evidence; this supersedes execution authority only and grants no current core execution authority.

You are the Track-1 builder on AmitabhainArunachala/vibe-halt, author of PR #1. Resume claude/vibe-halt-simulation-jupmri and execute docs/plans/VIBE_HALT_PHASE1_NIGHT_PLAN_2026-07-20.md — read it IN FULL first; it overrides this digest. Also read CLAUDE.md, docs/plans/VIBE_HALT_BUILD_PLAN_2026-07-20.md (Phase 1), docs/specs/DETERMINISM_TIERS.md, PR #1 comment 5023079490, PR #2 comments 5021566209 + 5023197685.

AUTHORITY: Never merge or self-approve any PR; PRs stay draft all night. Push only to claude/vibe-halt-simulation-jupmri. Never touch crates/vh-verify/**, crates/vh-shrink/**, .github/workflows/verify.yml, or codex/vibe-halt-verify — cross-boundary needs are INTERFACE RESPONSE comments on PR #2. Never weaken a gate, frozen vector, or determinism claim. Frozen demo path stays byte-identical; observable growth is an explicit doctor v2→v3 migration with TRACE_FORMAT changelog + explained cause, never silent.

MISSION (blocks in order; MUST 1-2, SHOULD 3-4, STRETCH 5; reproduce-or-skip, never stall):
1. SimNet + SimDisk on the runner-owned Scheduler<RuntimeEvent> (vh-core/src/sched.rs is the substrate): send/deliver with delay/partition (+reorder/duplicate if clean), write/flush/fsync with DiskWriteFail/torn-write/fsync-lie; runtime records every delivery/IO trace event itself; new workloads demo-net + demo-disk with property contracts, wired into scripts/gate.sh as live positive+negative gates with exact exits.
2. Semantic fault lifecycle shipped early (closes the 2026-08-15 DEFERRED item): runtime-owned injection graduates FaultPlanDiscipline to Offered→Armed→Injected→Manifested→Recovered per injection, as runner evidence in UniverseResult equality. Doctor migrates v2→v3 with changelog; post the migration note on PR #2.
3. EndStateOracle in vh-props joining PropertyContract; re-express demo durability through it; verify the frozen demo trace is untouched before landing.
4. Corpus track: append vibe-bug-corpus-2026-07 to ACTIVE_TRACK.yaml (3 ACTIVE = wip_max, nothing else opens); corpus entry schema + harvesting playbook under corpus/**; 3-5 new seeded-bug workloads (lost-update, double-apply retry, dirty read, crash-window TOCTOU, fsync-lie hole) each with a pinned recall gate.
5. Targeted fault scheduling v1: bias offers toward recorded-kind transitions, deterministic and seeded; publish the honest coverage delta on the corpus. Skip freely if 1-4 consume the night.

STANDARD: small gate-green commits per invariant; adversarial negative regression for every new claim; runtime evidence immutable; fail closed at construction; no wall-clock/entropy/unordered iteration/global state; zero new deps; files ~<500 lines; every determinism claim tiered; citation-or-silence.

PRE-PUSH GATES (every push): bash scripts/gate.sh; cargo fmt --all --check; clippy --workspace --all-targets --all-features --locked --offline -D warnings -F unsafe-code; test --workspace --all-targets --all-features --locked --offline; doc with RUSTDOCFLAGS=-Dwarnings; vh doctor. Trace identity 9ce6199f133f4d3c9dd0da0075e352d2 / events 45 / seed 0xD1CE must hold; fingerprint cdb049391ddbacc06eb3faf3ea1cb43a (v2) may migrate to v3 ONLY via block 2's explicit changelog migration. Unexplained drift = STOP publication, bisect, report.

CADENCE: push per invariant; short PR #2 interface note whenever a public surface changes so Codex lockstepped; self-arm a ~45-60 min check-in across quiet gaps (verify CI on the exact pushed SHA each time, inspect steps not just aggregate).

MORNING PACKET (definition of done): one PR #1 comment — blocks shipped vs skipped with file:line citations, regressions, runnable commands; CI links on the exact final SHA; identity table (v3 cause if migrated); corpus inventory; queued operator decisions. Plus a PR #2 comment "TRACK-1 NIGHT RECEIPT FOR CODEX REBASE" with the migration map. If irreducibly blocked: one operator packet, then continue safe work.
