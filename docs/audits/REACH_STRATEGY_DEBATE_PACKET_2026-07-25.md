# VIBE-HALT — Reach Strategy Decision Packet (2026-07-25)

**Status: ADVISORY ONLY.** Produced by the three-seat adversarial debate chartered in
PR #36 (`docs/prompts/VIBE_HALT_REACH_STRATEGY_DEBATE_2026-07-25.md`, PR head
`c0e78be`; the charter is itself an unmerged proposal and grants no authority —
neither does this packet). Nothing here creates execution, merge, spending, or
law-exception authority; every work package below requires its own human-merged
admission PR under existing governance. The sole active execution controller
remains `docs/prompts/VIBE_HALT_POST_AUDIT_TIER2_REACH_LONG_RUNNING_GOAL_2026-07-22.md`
("controller").

**Measurement basis.** Each seat independently re-derived live state at merged
`origin/main` = `c02d99e`: `make onboard` → READY; `make gate` → exit 0,
`== gate battery: ALL PASS ==`; frozen identities held (trace
`9ce6199f133f4d3c9dd0da0075e352d2`, 45 events; doctor
`669b4cdef41ede292761c5a47cd69f37`, `vh-doctor-observable-v4`). Seat-run workloads at seed `0xD1CE`,
100 universes: `corpus-dirty-read` 96, `corpus-crash-toctou` 38,
`corpus-stale-redispatch` 91, `demo-net` CLEAN 100/100.

**Process.** Four rounds as chartered: independent briefs; cross-examination
(every steelman confirmed or corrected by its advocate before the attack
counted); written mind-changing conditions; this synthesis, reviewed by all
three seats. §5 dissents are verbatim and unedited. Amended same day for the nine Codex
review findings on `414e2e9`; re-approved by all three seats.

## 0. What the debate established

1. **The published-number defect is real; every seat reproduced it.**
   `corpus/entries/VB-003-dirty-read.md:10` pins `found 83/100`; the pinned
   command measures **96/100**. `VB-004-crash-toctou.md:10` pins `21/100`;
   measures **38/100**. Gates stayed green because the affected recall gates
   assert only `"$fails" -lt 1` (`scripts/gate.sh:173` VB-003, `:186`
   VB-004; same pattern at `:147`) against a FAIL list the CLI truncates at ten
   (`crates/vh-cli/src/main.rs:410` `.take(10)`; observed
   `... and 86 more failing universes`). This is audit B.1 live on merged
   main; repair is owned by the unstarted C2b/K1 packages. Control: VB-007
   pins 91/100 and measures 91/100.
2. **Criterion 4 (≥3 previously unknown, human-confirmed bugs in real code —
   `docs/plans/VIBE_HALT_BUILD_PLAN_2026-07-20.md:109`) stands at zero on
   every seat's first milestone.** Seat A conceded its "SERVED" claim
   overclaimed: S7 admits D1 for the supported self-written fixture only
   (controller:454) and forbids generalization (controller:458). Seat B
   conceded its day-10 milestone executes zero foreign code. Seat C conceded
   forward confirmation (sim finding → real-target reproduction) has been
   executed zero times in this repository.
3. **On the only real-bug sample the project has, the demand census is 5–0
   fault-order over schedule-choice — with a standing filter caveat.** All
   five harvested entries (VB-007..011, provenance at each entry's line 7)
   recall under default FIFO with fault injection alone; the only
   `--schedule` flags in the battery belong to seeded VB-006
   (`scripts/gate.sh:455,473-474`). Seat B's unrefuted counter: a corpus
   admitted through a FIFO-plus-faults pipeline can only contain what that
   mechanism recalls (`corpus/SCHEMA.md:4-6`), so 0-of-5 measures the
   admission filter, not the world.
4. **No seat could name, today, one real foreign program its first milestone
   executes.** Seat A's S2 profile (one process/one thread, controller:423)
   fits 0 of the 5 harvested sources; Seat B's first foreign contact is a
   post-milestone dharma_swarm port; Seat C's confirmation step is
   unprecedented in-repo (finding 2).
5. **Each seat pre-declared a falsifiable abandonment trigger, and all three
   route through the same three cheap probes** (§1). The recommendation
   below is therefore an evidence sequence, not an average of the theses:
   each probe's kill rule was authored by the seat it can kill.
6. **Charter §7's question — is a project whose published numbers drift
   green ready to spend runway on new capability? — was answered no by all
   three seats.** The pin repair precedes everything.

## 1. Recommended sequence

Three ranked work packages, ~6 agent-days total, no law exceptions, no new
capability code. Each was proposed during the debate by the seat its outcome
can hurt most.

**WP1 — M0: exact recall-pin repair (the deferred C2b/K1 slice).**
Delivered as the controller's mandated split (controller:245): **K1** —
entries and contract prose, owner `vibe-bug-corpus-2026-07`
(`ACTIVE_TRACK.yaml:114-119` limits it to corpus artifacts) — then **C2b**
— the exact `scripts/gate.sh` assertions, one separate serialized C2-core
writer. Two exact-head PRs. Cost: **2 agent-days**.
Acceptance: every corpus entry's recall gate asserts exact equality of the
measured `always-failures` count against the entry's pinned count (replacing
`-lt 1`); VB-003 re-pinned at 96/100 and VB-004 at 38/100 with before/after
receipts in the PR body; the full battery passes twice consecutively with
exact asserts. Kill: any entry whose count is not bit-stable across two
consecutive runs at the pinned seed is quarantined `UNCHECKED`, never
silently re-pinned.

**WP2 — X-CONF: forward-confirmation probe (Seat C's abandonment trigger).**
Owner: `vibe-bug-corpus-2026-07`, plus controller:375 named-target
authorization granted against the exact recorded pre-fix repo/SHA pair,
license, and execution plan before execution (disposable environment, no
credentials, no live provider calls). Cost: **2 agent-days**, one per
target. Protocol: for VB-008 (langgraph #6491) and VB-010 (langgraph #7361)
at their pre-fix SHAs, attempt to force the *reduced mechanism's* trigger as
the rig would name it — for VB-008 the torn-write window meeting the
write-side validation gap (`corpus/entries/VB-008-unvalidated-checkpoint.md:24-34`),
not merely the published invalid-output manifestation — using conventional
test-level injection only (pytest, monkeypatch, mock provider, SIGKILL at a
hook). A success on a trigger the sim could not have named does not count. Acceptance is reaching the
pre-declared decision rule, not any particular outcome:
≥1/2 forced → the convergent Round-2 attack on M1's admissibility is
falsified and **amended M1** (below) is the funded discriminator;
0/2 forced → M1 is dead as budgeted; the funded discriminator is
**VH-PY-0-FIX** (below), Seat C's written flip to Seat B — unless S0
returns ≥3/10 in-profile, in which case Seat C's Round-3 redirect endorses
the supervisor lane instead.

**WP3 — S0: supervisor target census (Seat A's abandonment trigger).**
Owner: `vibe-bug-corpus-2026-07`; read-only; same disposable environment.
Cost: **2 agent-days**. Protocol: mechanically select **exactly ten**
eligible real, AI-authored, single-process CPython programs — the first ten
matching a selection rule recorded before reading any candidate (later
matches listed, non-binding) — submitted as a controller:375 named list for
approval before tracing; static import scan plus one syscall trace each;
publish a census table of S2-profile violations (threads/fork/exec,
C-extensions on the hot path, non-provider network, io_uring/IPC,
unmanaged signals/timers; controller:422-423). Decision rule: <2/10
in-profile → Seat A withdraws the C7 supervisor admission before any unsafe
spend and endorses Seat B (A's written trigger); ≥3/10 → the census becomes
the measured "exact supported target" input C7 requires (controller:391) and
supervisor admission stays live. (At exactly 2/10, A's trigger does not fire
but Seat C's endorsement of A does not engage; the operator sees the table
either way.)

**After the block (~day 6): fund exactly one discriminator, chosen by WP2's
rule, under its own pre-declared kills.**
— **Amended M1** (Seat C): three-target pre-registered port trial,
**15 agent-days**, corpus track (entries/provenance) plus a core-track
writer for the `crates/vh-cli` workloads (ACTIVE_TRACK.yaml:114-118);
M1-PC positive controls (each target ported
at the parent SHA of one known, later-fixed defect; the rig must recall it
or the target is disqualified and replaced from a pre-registered five-target
pool — at most two replacements, every attempt counted in the report, pool
exhaustion ending the trial as a porter-capability null distinct from an
engine null); a finding counts only if replayable, forced on the real
target, previously unknown per the controller's previously-known status and
human-confirmation contract (controller:377), and human-confirmed;
full-null kill → the recorded bleed-list picks A or B, classified per
target (provider/async → B; syscall-class → A; neither or model-fidelity →
no vote), majority of voting targets deciding — a tie or zero votes returns
to the operator with the recorded lists; a positive
(≥1 previously-unknown, forward-confirmed finding) fires Seat B's written
B→C trigger — Seat B endorses the port kit and no runtime lane is funded.
— **VH-PY-0-FIX** (Seat B): deterministic asyncio event loop over the C5
protocol, **10 agent-days**, core track — including its own minimal
protocol-v1 transport slice (2 of the 10 days, Seat B's Round-1 costing;
full C5 remains a separate package); fixture is a Python-level
re-reduction of harvested VB-011 (not an invented race); kills at day 10 on
any divergence unattributable to a receipt-declared `Open` channel or
no recall within 1,000 universes at two seeds — that failure is B's written
flip to putting C7/S1 before the operator.

Worst case to a premise verdict: 6 + 15 = **21 agent-days**, within the
ratified budget frame (`BUILD_PLAN:78-86`; balance recorded unknown, so no
package here carries spending authority).

## 2. The decisive experiment

**X-CONF (WP2) is the discriminator-selector, and it is the cheapest thing
that discriminates between the seats** — 2 agent-days against: M1's 15
(which it can kill), VH-PY-0-FIX's 10 (which it can crown), and the
supervisor lane's ~15 pre-spike days plus a law exception (which S0 can kill
in parallel). Pre-declared rules, written by the seats themselves in Round 3:

| X-CONF result | S0 result | Funded next |
|---|---|---|
| ≥1/2 forced | any | Amended M1 (15d, corpus); C7 admissibility per S0 |
| 0/2 forced | ≥3/10 in-profile | Supervisor lane (C5 → C7 decision → spike); census is C7's target input — Seat C's Round-3 redirect |
| 0/2 forced | exactly 2/10 | VH-PY-0-FIX (10d, core); census to the operator with no C7 recommendation (A's trigger does not fire; C's redirect does not engage) |
| 0/2 forced | <2/10 in-profile | VH-PY-0-FIX (10d, core); C7 withdrawn by A's own rule |

No outcome funds two capability lanes at once, and no outcome is
uninformative: even the double-negative row converts both rivals' theses
into measured evidence for Seat B's.

## 3. What we are giving up

- **Six days of zero capability progress, deliberately.** C5 — the audit's
  E.2 blocker and the first step of F.14's own sequence — does not start
  inside the block. The packet puts the audit's recommended sequence behind
  evidence gates; that is a real demotion of F.14, not a scheduling detail.
- **Criterion 3 (≥80% recall on ≥25 entries): no growth this cycle.** The
  corpus stays at 11 entries; the packet explicitly declines to count-farm
  reductions of published issues to 25.
- **Criterion 2 (Tier-2 divergence measured/published): no progress** during
  the block, nor under amended M1 if funded.
- **Criterion 7 (dharma_swarm adapter receipt): deferred** behind the
  discriminator outcome; the quarantined client stays quarantined.
- **Possibly D1 itself.** If S0 returns <2/10, the supervisor is withdrawn
  un-attempted, D2 becomes the recorded ceiling, and "code that knows
  nothing about vibe-halt" narrows permanently to "cooperative code". That
  withdrawal would fire Seat A's own trigger and A accepts it; A's residual
  objection below concerns the remaining rows — only the (0/2, ≥3/10) row
  funds the supervisor lane, so F.14's ten days can go unspent even in
  outcomes where S0 keeps that lane alive.
- **Speed.** Both capability seats pre-agreed to stand down for the block;
  if the operator already believes one thesis, the block costs six days of
  runway to learn what the operator would have assumed.

## 4. Law exceptions required

**NONE** for WP1–WP3 and for VH-PY-0-FIX: no `unsafe`, no external
dependency, no deny-list change, no frozen-format change, no Tier-3
hypervisor. X-CONF and S0 use the controller's existing operator target
gate (controller:375) — an authorization, not an exception. The single
exception that exists anywhere downstream is unchanged from the controller:
if and only if S0 keeps the supervisor lane alive and the operator later
ratifies C7, a first-party separately-built helper carrying audited
`unsafe` outside `crates/**` (controller §12:403). This packet neither
requests nor advances it.

## 5. Dissent (verbatim, unedited)

**Seat A — The Supervisor:**
> D2 forever means every verdict on foreign code carries open channels, and
> run-twice sampling can bless drift — this repository's own published
> recall numbers sat wrong for days while every gate stayed green. A D1
> two-host identity is the only verdict class immune to that rot, and it is
> cheap to price: ten working days, kill-gated, with the failure mode
> pre-accepted as an honest D2 ship. If the packet defers the supervisor
> indefinitely, it is not parking risk; it is deciding the product is
> chaos-testing with receipts, without ever having spent the ten days F.14
> budgeted to find out whether the ceiling was real.

**Seat B — The Hosted Runtime:**
> Every DST system that shipped got reach by replacing the substrate its
> targets run on; none got it by replaying faults at a process boundary.
> The 5-0 FIFO census the packet will lean on is the admission filter of a
> FIFO-only pipeline, not a measurement of the world. Criterion 7's only
> named target is an asyncio Python system, and virtual time for asyncio
> passes through the event loop whether or not anyone values interleaving
> exploration — C5 without loop ownership fails run-twice on its first
> asyncio.sleep. A packet that defers the hosted runtime defers the only
> ratified adapter and caps the project at replaying the past instead of
> exploring the possible. Pin the counts, land C5, then own the loop.

**Seat C — The Falsifier:**
> Every dollar spent so far has built a rig that has never once been pointed
> at reality, and both rivals' ten-day milestones still end at fixtures
> their own authors wrote. The published recall numbers drifted green for
> three days after the drift was announced, under a gate that counts to one.
> If this packet funds any capability before the one-day pin repair and the
> two-day forward-confirmation probe, it is repeating the same wager at
> higher stakes: capability first, truth later. A project that has never
> confirmed one known bug forward has no measured basis for building
> machinery to find unknown ones. Run the three cheap days first. Let the
> result — not the roadmap, not the audit's eloquence — spend the rest.

**Why the disagreement partially collapsed.** The theses did not converge —
the dissents above are live. What converged is the procedure: Round 3 forced
each seat to name the result that would flip it, and all three named cheap
probes (1–2 days each) rather than their rivals' full milestones. A debate
designed to prevent averaging instead produced three self-authored kill
switches; the packet funds the switches, not a compromise thesis.

## 6. The Antithesis answer

Our version of Antithesis is a deterministic multiverse bench for
agent-shaped state machines: target logic runs on an engine-owned substrate
(today the Tier-1 sim; after the evidence block, whichever reach lane
survives its own kill rules), gremlins inject the crash/retry/stream/
provider faults that account for every real bug the project has modeled,
properties adjudicate, and every finding ships as a one-command,
bit-reproducible bundle whose receipt names each channel it did not control.
It is explicitly **not**: a hypervisor, a security boundary, a chaos-testing
service for arbitrary hostile binaries, or an eval platform. The one
capability that would make an outside engineer choose it over writing more
tests: a bug report that replays byte-identically on their machine with the
trigger window named — a failing test they didn't have to write, for a
failure they couldn't schedule.

## 7. Operator gate

**One decision: authorize the 6-agent-day evidence block (WP1 K1→C2b → WP2
X-CONF → WP3 S0), with the §2 table binding the choice of the single funded
discriminator?**

- **YES** — the block runs; the only further operator touches are the
  controller:375 named-target approvals against exact recorded
  repo/SHA/license lists (X-CONF's two pre-fix checkouts; S0's ten
  programs) before those checkouts execute; the follow-on admission PRs
  arrive pre-bound to the §2 rules.
- **NO** — pick a seat directly and its chartered path runs instead:
  **A** (C5 → S0 → C7 decision → 10-day spike), **B** (C5 → VH-PY-0-FIX),
  or **C** (M0 → amended M1).
