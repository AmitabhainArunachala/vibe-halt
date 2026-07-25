# VIBE-HALT — Reach Strategy Debate (three-seat adversarial decision charter)

**Artifact type:** debate charter for three independent agents with repository access
**Authored:** 2026-07-25
**Status:** PROPOSAL / DEBATE ONLY — **this file is not an execution controller**

The sole active core execution controller remains
`docs/prompts/VIBE_HALT_POST_AUDIT_TIER2_REACH_LONG_RUNNING_GOAL_2026-07-22.md`.
This charter supersedes nothing, grants no ownership, and authorizes no code
change, no merge, no dependency, no unsafe code, no spending, no repository
creation, and no public-target execution. Its only output is a decision packet
for the human operator. Any work that follows requires a separate human-merged
admission PR under the existing governance.

---

## 0. Why this debate exists

The engine works. The reach does not. Those two sentences are the whole
situation, and every seat must begin by verifying them rather than believing
them.

At authorship, `origin/main` was `35a7fc7674e72196f378df9e66f25a40bbbf3cf7`.
Measured at that head:

- `make gate` exits 0; the full battery passes; frozen Tier-1 identities hold
  (trace `9ce6199f133f4d3c9dd0da0075e352d2` / 45 events; doctor
  `669b4cdef41ede292761c5a47cd69f37` `vh-doctor-observable-v4`).
- Ten corpus workloads are recalled within a 100-universe budget at seed
  `0xD1CE`: 29, 76, 96, 38, 21, 91, 96, 79, 70, 58.
- Success criterion 4 — three previously unknown, human-confirmed bugs in real
  code (`docs/plans/VIBE_HALT_BUILD_PLAN_2026-07-20.md:109`) — stands at
  **zero**, and no mechanism currently exists that could move it.
- The Tier-2 surface is an environment-scrubbed subprocess runner. All 29
  declared capability channels are honestly `Open`/D2
  (`crates/vh-sandbox/src/capability.rs`).
- The cassette does not reach the child process; the parent performs the
  provider call and interpolates bytes into generated Python
  (audit finding E.2, issue #24 comment `5043766546`).

**Every one of those facts is stale the moment you read it.** Re-derive each at
current merged `origin/main` before citing it. A seat that argues from this
section without re-verification has already lost the round.

---

## 1. The question under debate

> What is the shortest credible path from what vibe-halt is today to a tool
> that finds bugs nobody planted, in code its authors did not write for it —
> and what, concretely, is *our* version of Antithesis?

Sub-questions every seat must answer explicitly, not by implication:

1. What runs on the rig six months from now, who runs it, and what do they type?
2. Which of the seven ratified week-12 success criteria does your path serve,
   and which does it knowingly sacrifice?
3. What is the first experiment on your path that could return a null, and what
   would you do if it did?
4. Does your path require an exception to a standing law (zero external
   dependency, `unsafe_code = forbid` in `crates/**`, deny-list purity, no
   Tier-3 hypervisor)? Name it exactly. Do not smuggle it.

---

## 2. The three seats

Each seat is dispatched as a separate agent session with repository access.
Seats argue their assigned thesis **at full strength** — this is assigned
advocacy, not preference. A seat that hedges into the middle has failed its
function. Convergence, if it happens, must be earned in Round 3 against
evidence, never assumed in Round 1.

### SEAT A — The Supervisor

**Thesis:** reach means executing arbitrary target code under controlled
conditions. Finish the child-visible cassette transport (C5), then run the
capped Linux single-process deterministic supervisor spike (C7/S1–S7) and close
the channel ladder to D1. This is the audit's own F.14 verdict and the only
path that makes vibe-halt work on code that knows nothing about vibe-halt.

**You must confront:** the supervisor needs audited `unsafe` and a
first-party helper binary, both of which the project's standing law forbids
inside `crates/**`; the honest closure ladder is seven rungs
(issue #24, F.14) and partial closure buys nothing — every incomplete rung is
still D2; the spike's own kill gate is ten working days; and CPython under a
syscall supervisor is a target profile so narrow it may not include any real
agent system anyone runs.

### SEAT B — The Hosted Runtime

**Thesis:** reach does not require syscall interposition. It requires the
*target* to run on a deterministic runtime. That is exactly how every
successful DST system got there — FoundationDB, TigerBeetle's VOPR, Shuttle,
madsim, Loom — none of them supervise arbitrary binaries; they replace the
concurrency/IO substrate the program is written against. Build a deterministic
runtime shim for one hosted language (Python `asyncio` or Node) that maps
scheduling, timers, sleeps, randomness, IO and provider calls onto the existing
`vh-core` scheduler and virtual clock over a small versioned protocol. Real
agent code runs with a modest, mechanical port — no `unsafe`, no ptrace, no
seccomp, no law exception.

**You must confront:** a shim only controls what the target actually routes
through it — one direct `time.time()`, one thread, one C extension, one
subprocess and determinism is gone, so the honest grade may be D2 forever;
"modest mechanical port" is a claim requiring evidence, not assertion; this
route re-introduces a cross-language boundary that must be identity-bound and
verified; and it risks becoming a general-purpose async framework, which is
scope death.

### SEAT C — The Falsifier

**Thesis:** both of the above are capability-building on an unvalidated premise.
The single unproven claim in the entire project is that this rig finds *unknown*
bugs in *real* code. Nothing built before that is proven can be justified by
anything except hope. Run the cheapest decisive experiment now: hand-port the
core state machine of one real AI-generated repository into a `vh` workload
with an independent oracle, and see whether anything falls out. Then let the
result — not the roadmap — pick between Seat A and Seat B. Argue also that the
existing corpus may already be measuring the wrong thing: the harvested entries
reduce already-published issues, so none is previously unknown
(issue #24, F table, criterion 4).

**You must confront:** hand-porting is manual and does not scale, so a positive
result proves the *engine's* value but not the *product's*; a single negative
result may be a bad target choice rather than a real null, so you must
pre-declare how many targets and what counts as a fair trial; and the
four-week realism kill (R7) already exists in governance
(`docs/governance/ACTIVE_TRACK.yaml`, corpus track) — explain why your
experiment is not simply that kill criterion restated.

---

## 3. Rules of engagement (binding on all seats)

1. **Citation-or-silence.** Every factual claim carries a current `file:line`,
   a GitHub item, or a runnable command with its observed output. Uncited
   claims are struck from the record regardless of how well written they are.
2. **No adjectives where a number belongs.** "Fast", "robust", "significant",
   "close to Antithesis" are not findings. Measure or say UNKNOWN.
3. **Every claim names its falsifier.** If nothing could show you wrong, you
   have stated a preference, not a position.
4. **Steelman before you strike.** In Round 2 you must restate the position you
   are attacking in a form its own advocate would accept, and get that
   confirmed, before attacking it.
5. **Forbidden moves:** appeal to industry practice without a cited system and
   its actual constraints; "we can do both" as a way to avoid ranking; reciting
   the audit back as if quotation were analysis; attacking a seat's assigned
   thesis as though the agent chose it; any claim about what Antithesis does
   internally that is not from public material.
6. **Read-only.** No seat edits, commits, pushes, opens a PR, comments on
   GitHub, or runs anything that writes outside a scratch directory. Running
   `make onboard`, `make gate`, `cargo test`, and `vh run` locally is expected
   and encouraged — measure, do not speculate.
7. **Standing law is not up for debate**, only for exception requests: human-only
   merge; zero external runtime dependencies; `unsafe_code = forbid` in
   `crates/**`; determinism deny-list; frozen PRNG and trace formats;
   receipts never enter git; no Tier-3 hypervisor. A seat may *request* a named
   exception with rationale; no seat may assume one.
8. **Budget is real.** $10,000 and 12 weeks total, ratified
   (`docs/plans/VIBE_HALT_BUILD_PLAN_2026-07-20.md:78-86`); remaining balance is
   recorded as unknown and no agent has spending authority. Estimate every
   proposal in agent-days and name what it displaces.

---

## 4. Rounds

**Round 1 — Independent opening brief.** Each seat works alone and must not
read the others' output. Refresh live state, run the gate, run the rig against
at least three workloads yourself, then produce (max 1,200 words):

- your thesis in one paragraph a non-engineer could act on;
- the three strongest pieces of evidence for it, each cited;
- your six-month product picture: the exact command a user types, on what code;
- criteria served / criteria sacrificed;
- your first falsifiable milestone, its cost in agent-days, and its kill rule;
- the single strongest argument *against* your own position.

**Round 2 — Adversarial cross-examination.** Each seat reads the other two
briefs and must, for each: state the opposing thesis in its strongest form,
then attack its single most load-bearing claim with evidence. Vague dissent does
not count — name the file, the number, or the missing mechanism. Each seat
answers the attacks made on it: `CONCEDED`, `REFUTED` (with evidence), or
`UNRESOLVED — needs experiment X`.

**Round 3 — Mind-changing conditions.** Each seat states, in writing: *the
specific result that would make me abandon my own position and endorse a named
rival.* If a seat cannot name one, that is recorded as a finding about the
seat's position, not a strength. Then each seat estimates the cost of running
that decisive experiment.

**Round 4 — Joint decision packet.** One synthesis document. It may be written
by any seat or a coordinator, but it must be reviewed by all three and it must
preserve dissent verbatim — averaging the positions into a compromise is the
failure mode this whole structure exists to prevent.

---

## 5. Required deliverable

A single decision packet containing:

1. **The recommended sequence** — an ordered list of the next three work
   packages, each with owner track, cost in agent-days, the exact acceptance
   test, and the kill criterion that stops it.
2. **The decisive experiment** — the cheapest thing that discriminates between
   the seats, with its cost and its pre-declared success/failure rule. Run
   before the expensive commitment, not after.
3. **What we are giving up** — named criteria and capabilities the recommended
   sequence sacrifices, stated plainly. A packet that sacrifices nothing is
   a packet that ranked nothing.
4. **Law exceptions required** — exact, with rationale, or `NONE`.
5. **Dissent section** — each seat's remaining objection in its own words,
   unedited. If all three agree, say so and explain why the disagreement
   collapsed, because that is a surprising outcome and needs an explanation.
6. **The Antithesis answer** — one paragraph: what our version is, what it is
   explicitly *not*, and the one capability that would make an outside engineer
   choose it over writing more tests.
7. **Operator gate** — the single decision only the human can make, phrased so
   it can be answered yes/no or A/B/C without further reading.

Cap the packet at 2,500 words. Length is not evidence.

---

## 6. Explicitly out of scope for this debate

- The evidence-locked evolution forge (PR #29). A third thesis may not open
  while the first is unproven and the second returned null.
- Reviving guided exploration — palette guidance was falsified, schedule
  guidance is unmeasured pending a depth≥2 instrument
  (`docs/audits/antithesis-dst-2026-07-21/CONVERGENCE_LEDGER_2026-07-22.md`
  §(e)). Neither is on the table here.
- Any dashboard, eval platform, UI, or new evaluator framework.
- Re-litigating merged work. Historical PRs are evidence, not targets.

## 7. Known live defect the seats should account for

At `35a7fc7`, two published corpus recall pins contradict measured behavior at
the same seed: `corpus/entries/VB-003-dirty-read.md:10` claims `found 83/100`
(measures 96/100) and `corpus/entries/VB-004-crash-toctou.md:10` claims
`found 21/100` (measures 38/100). The oracle repairs that moved them merged in
PR #32; the pins were deferred to the unstarted C2b/K1 packages. The recall
gates stayed green because each asserts only `fails -lt 1`
(`scripts/gate.sh:147`) against a `FAIL` line list the CLI truncates at ten.

This is audit finding B.1 demonstrating itself on merged main. Any seat that
argues from corpus recall numbers must reconcile this first — and each seat
should say whether a project whose published numbers can drift green is ready
to spend its remaining runway on new capability at all.
