# Codex Audit Charter — post-convergence full-repo audit + 12-week plan risk review (2026-07-22)

**To: Codex (verifier track).** You are the independent auditor for a
post-convergence full-repository audit. This charter follows the
established protocol (`docs/prompts/CODEX_PR2_MASTER_SPEC_2026-07-20.md`
precedent: committed prompt charter + GitHub threads, no side channels).
This file mirrors GitHub issue #24; deliver findings as replies on that
issue and/or line comments on the cited PRs.

## Context you must load first

- `AGENTS.md` (root), `docs/governance/ACTIVE_TRACK.yaml` (two-key law;
  surface ownership)
- The audit corpus: `docs/audits/antithesis-dst-2026-07-21/` — especially
  `EXECUTIVE_VERDICT.md`, `FIRST_PRINCIPLES_AND_TARGET_ARCHITECTURE.md`
  (rejections R1/R2/R3), `INTEGRATION_ROADMAP.md` (recommendations
  R0–R8), `EVIDENCE_LEDGER.jsonl`, `CONVERGENCE_LEDGER_2026-07-22.md`
  (campaign closeout)
- The convergence campaign: PRs #16, #17, #18, #19, #20, #21, #22
  (merged) and #23 (draft, C2)
- Founding docs: `DESIGN.md`, `docs/plans/VIBE_HALT_BUILD_PLAN_2026-07-20.md`,
  `docs/specs/DETERMINISM_TIERS.md`, `docs/specs/TRACE_FORMAT_V0.md`

## Standing law (binding on this audit)

Citation-or-silence: every finding carries `file:line` or a runnable
command + output. No edits to any surface outside your owned crates
(`crates/vh-verify/**`, `crates/vh-shrink/**`, `verify.yml`, `AGENTS.md`)
without a ratified charter — this audit is read-and-report. Draft PRs
only if you build anything; the operator alone merges. Rank findings
BLOCKER / GAP / NIT.

## Audit areas

### A. Determinism soundness (the load-bearing claim)

1. Hunt hidden nondeterminism sources the deny-list scanner
   (`scripts/check_determinism_denylist.py`) cannot see: hash-order
   iteration, allocator-address leaks into observables, float
   formatting, `sort` stability assumptions, panic-message content in
   traces.
2. The trace identity is FNV-1a-128 (`crates/vh-trace/src/lib.rs` —
   explicitly non-cryptographic). Assess collision/gaming risk now that
   trace hashes are merge-gating identities and bundle-verification
   anchors (C4/C5). Is the planned SHA-256 upgrade urgent or
   deferrable?
3. Cross-machine bit-identity (BUILD_PLAN Success criterion 1):
   `verify.yml` runs a 3-OS matrix — verify whether it actually asserts
   hash *equality across* OSes or only per-OS self-consistency. If the
   latter, that criterion is weaker than it looks; propose the minimal
   cross-OS assertion.

### B. Oracle strength and recall-gate integrity

4. Audit all 11 corpus workload oracles
   (`crates/vh-cli/src/workloads/corpus.rs`) for tautology: does any
   oracle merely re-assert what the workload's own control flow
   guarantees, rather than an independent correctness law?
5. Anti-gaming: recall pins (29–96/100) are frozen in
   `scripts/gate.sh`. Can a future change quietly widen a fault palette
   or budget to inflate recall without tripping a gate? Is the VB-010
   anti-gaming precedent (`corpus/entries/VB-010-resume-replay.md`)
   generalized or one-off?

### C. C2 bakeoff methodology (PR #23) — adversarial review of the null result

6. The PCT-vs-uniform null (`scripts/track2_pct_bakeoff.py`, receipt
   `docs/audits/antithesis-dst-2026-07-21/commands/convergence-c2-pct.txt`):
   the 32 seeds are **consecutive integers** (0xD1CE..0xD1ED). Assess
   seed-correlation risk through `Xoshiro256pp::from_seed` mixing —
   could correlated seeds mask a real PCT advantage?
7. Metric critique: `first failing universe` with budget 1000 on a bug
   both strategies find at universe 0–2 has near-zero discriminating
   power. Is the null "PCT is not faster" or "this corpus cannot
   measure schedule-strategy differences"? Distinguish precisely — the
   published claim's strength depends on it.
8. VB-006 is depth-1 by construction. State what the *minimal* corpus
   entry proving/disproving PCT's value would look like (depth≥2,
   ordered causal chain), without building it.

### D. Evidence integrity (C4/C5 surfaces — your lineage)

9. `vh-run-receipts-v1` / `vh-finding-bundle-v1` and `vh replay-bundle`:
   attack the tamper story. Any field not covered by verification? Note
   that bundles do not record a schedule policy (the C2 fail-closed
   conflict) — assess whether the bundle format should version now
   rather than retrofit.
10. The shrink provenance binding (`fingerprint-digest`,
    vh-shrink-fingerprint-v1): can a shrunk plan bind to the wrong
    baseline undetected?

### E. Tier-2 sandbox honesty (PR #11, your crate's neighbor)

11. `crates/vh-sandbox`: audit the `HonestyChannel` ledger for
    completeness — which uncontrolled channels are NOT yet declared
    (vdso time, ASLR, `/proc` reads, io_uring, GC timing in target
    runtimes)? An honesty ledger that under-declares is worse than
    none.
12. The cassette layer (`vh-llm-request-v1` exact-digest replay): assess
    the gap between this and a usable agent-testing surface (HTTP
    interposition, streaming, fuzzy matching) — rank what is
    load-bearing vs deferrable.

### F. 12-week plan risk ranking + the strategic question

13. Rank the 7 BUILD_PLAN Success criteria
    (`docs/plans/VIBE_HALT_BUILD_PLAN_2026-07-20.md:87-102`) by current
    risk, with evidence. Note criterion 4 (≥3 previously unknown
    human-confirmed real bugs) has zero progress and hard-depends on
    reach (Tier-2).
14. **The strategic question, answered with your independent judgment:**
    given the zero-external-dependency law, `unsafe_code = forbid`, and
    the $10k/3-month constraint — is a deterministic *process
    supervisor* (seccomp+ptrace syscall interposition, virtual time,
    single-threaded targets first; DetTrace-lite) the correct "our
    version of the hypervisor," or should Tier-2 stay at env-scrubbing
    + cassette fidelity? If the supervisor: specify the minimal
    channel-closure ladder to reach D1 and where the audited-unsafe
    boundary crate (or helper binary) must live to respect the
    workspace unsafe ban.

### G. Doc integrity (small but compounding)

15. Two colliding "R" numbering schemes exist (rejections R1–R3 in
    `FIRST_PRINCIPLES_AND_TARGET_ARCHITECTURE.md:18-22` vs
    recommendations R0–R8 in `INTEGRATION_ROADMAP.md`), and
    `docs/prompts/CONVERGENCE_CAMPAIGN_EXECUTOR_2026-07-22.md:320`
    mislabels the RL rejection as R2 (canonically R3). Propose the
    one-line erratum convention.
16. `DESIGN.md:212-224` (11 criteria) vs `BUILD_PLAN:87-102`
    (7 criteria) disagree on numbers (10k vs 1k universes/hr; ≥70% vs
    ≥90% shrink). Recommend which list is canonical and how to mark the
    other.

## Deliverable

One structured report as a reply on issue #24: findings ranked
BLOCKER/GAP/NIT per area A–G, each with file:line or command, plus your
answer to F.14. No fixes in this pass — findings first, the operator
routes the work.
