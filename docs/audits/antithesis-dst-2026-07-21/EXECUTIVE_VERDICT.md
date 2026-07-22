# Executive Verdict — vibe-halt vs Antithesis-class DST

## Scope, repositories, commits, and cutoff date

- **Date / cutoff**: 2026-07-21 (all external sources accessed this date)
- **Repository under audit**: `AmitabhainArunachala/vibe-halt`, pinned public commit `84f911e` (origin/main). Audited tree: `/Users/dhyana/vibe-halt` @ `3e2a5ed` (branch `claude/vibe-halt-phase1-sandbox-goal`; differs from `84f911e` by one docs file, PR #10).
- **Comparison target**: Antithesis (antithesis.com) — deterministic software testing company, FoundationDB lineage; primary sources only, proprietary internals marked UNKNOWN.
- **Method**: three independent evidence lanes (local code + runtime reproduction; Antithesis primary-source dossier; adjacent OSS/research landscape), cross-falsified. Full evidence: `lanes/`, `commands/`, `EVIDENCE_LEDGER.jsonl`, `SOURCE_MANIFEST.csv`.

## Unvarnished verdict

vibe-halt is a **real, working, and unusually honest Tier-1 deterministic simulation kernel** — every load-bearing claim we tested (zero dependencies, frozen PRNG, deny-list gate, exit-code semantics, all five pinned corpus recall numbers, byte-identical cross-process reruns) **reproduced exactly**. Its evidence epistemics (tri-state verdicts that cannot be gamed into CLEAN, sampled-falsifier labeling, compile-time-closed evidence forgery, a gate-held quarantine of a previously fabricating Python client) are in places *better stated than Antithesis's own public documentation*.

It is **not Antithesis-class in capability, and is roughly three large steps away**:

1. **No search.** Exploration is uniform-random fault plans over independent seeds — Monte-Carlo, not state-space exploration. No coverage feedback, no novelty, no schedule choice points, no branching from interesting states. The code itself admits the targeted-generation phase "did not land."
2. **No reach.** Only in-process Rust `Workload` implementations are testable. The stated mission — testing *vibe-coded repositories and agent systems* — cannot be executed at all today; the Tier-2 subprocess sandbox is an unstarted goal doc at the pinned commit.
3. **No depth on findings.** The ddmin shrinker exists but is unreachable from the CLI; there is no decision tape, no causality analysis, no durable evidence store (`~/.vibe-halt/` is a phantom — nothing writes there).

Honest comparison: a two-day-old, evidence-hardened FoundationDB-style simulator in the pre-swarm era — MadSim/Turmoil with far stronger verdict governance and a far smaller runtime — not Antithesis.

The strategic finding cuts the other way: **the "DST for AI-generated code / agent systems" niche is unoccupied in open source** (Lane C §5). Agent tooling (promptfoo, deepeval, agentops) does eval/observability with zero determinism; Antithesis is closed, hypervisor-level, and general-purpose. vibe-halt's determinism + epistemics foundation is genuinely differentiated — the gap is exploration intelligence and target reach, both of which have concrete, borrowable prior art (TigerBeetle swarm testing, Shuttle PCT, Stateright property/checker shapes, Antithesis's published test-template algebra and causality-analysis algorithm).

"1000x" is not a hypervisor. It is: **guided exploration × real target reach × finding-depth artifacts × a harvested (not self-seeded) corpus** — each independently multiplicative, all four buildable on the existing kernel without breaking zero-dependency Tier-1 determinism.

## Ten highest-consequence findings

1. **Core claims all verified.** `make test` and `make gate` exit 0; same `demo-buggy` command in two separate processes → byte-identical output; replay of a failing universe reproduces the exact finding; corpus recall 29/76/83/21/21 matches pinned entries exactly. (reproduced, `commands/`)
2. **Exploration is blind.** `FaultPlan::generate` is uniform random (`vh-gremlin/src/lib.rs:110-132`); no coverage instrumentation exists anywhere; scheduler is fixed `(time, seq)` FIFO with no choice points (`vh-core/src/sched.rs:71-76`). Largest capability gap vs Antithesis's coverage+RL-guided tree fuzzing.
3. **Mission/target mismatch.** Only in-process Rust workloads are testable (`vh-multiverse/src/lib.rs:359-381`); zero vibe-coded repos or agent systems can be exercised today. Tier-2/D1 sandbox is planned, not started.
4. **Shrinker is orphaned.** `vh-shrink` (846 LOC bounded ddmin, integration-tested) has no CLI wiring — users cannot minimize a finding without writing Rust.
5. **Corpus measures the rig against itself.** 5/5 seeded entries reproduce; 0 harvested real bugs against a ≥25 acceptance metric (SCHEMA law 3 concedes seeded recall is a lower bound only). Recall numbers must not be cited as bug-finding power.
6. **`make onboard` fails on stock macOS** (exit 2: `tomllib` requires Python ≥3.11; system python3 is 3.9.6; no version check). The repo's own "run onboard first" law breaks at first contact on this platform; CI is blind to it (ubuntu/py3.12).
7. **`~/.vibe-halt/` is a phantom surface.** CLAUDE.md and .gitignore reference runtime receipts; no code writes there. Evidence today leaves the process only via stdout — there is no durable local evidence store.
8. **DESIGN.md sign-off rule is met at 2/7 with one conflicted signature** (Claude 91% is a self-sign-off by the Track-1 builder). Honestly labeled "Draft" — but the multi-LLM percentage theater should not be cited as design validation.
9. **The niche is empty.** No OSS system does deterministic multiverse simulation of agent harnesses with replayable evidence (GitHub searches, agent-eval repo inspection, 2026-07-21). Antithesis is the nearest occupant and is closed/commercial.
10. **The best next capabilities are all borrowable, not inventable.** TigerBeetle's 20-line swarm-testing idiom and battle-tested fault parameterization; Shuttle's PCT (probabilistic bug-finding guarantee); Stateright's Expectation/discovery-path shapes; Antithesis's published 7-prefix test-command algebra and rewind+re-explore causality algorithm. No hypervisor, no new dependencies required.

## Strongest real asset

**Verified determinism + ungameable verdict semantics.** Byte-identical cross-process reruns, frozen PRNG/trace identities, a semantic + regex deny-list with bypass reproductions, tri-state verdicts closed against the "certified nothing" hole by construction, and an anti-fabrication posture that is gate-enforced rather than aspirational. This is the substrate everything else compiles into — and the property Antithesis spent 5.5 stealth years on at the hypervisor layer.

## Most dangerous unsupported belief

That **pinned recall numbers against a self-seeded corpus demonstrate bug-finding capability**. They measure the rig against its own author's fault palettes. Until the corpus contains harvested real-world bugs and exploration is guided rather than uniform, "we catch X% of bugs" is a claim about five known bugs, not about unknown unknowns — the exact epistemic failure the repo's own governance exists to prevent.

## Highest-leverage next move

**Convert Monte-Carlo into search, this week, in ~a day of work:** per-universe swarm-testing configuration masks (TigerBeetle `random_enum_weights` idiom) + a recorded decision tape + `vh shrink` CLI wiring. All three reuse existing, tested code paths (vh-gremlin, vh-core scheduler, vh-shrink) and immediately multiply both exploration diversity and finding usability. Then PCT schedule perturbation (Shuttle design) as the first exploration strategy with a real probabilistic guarantee. Full sequencing in `INTEGRATION_ROADMAP.md`.

## What not to build

- **A hypervisor / process-level determinism layer** (Antithesis's Determinator, Hermit, rr). DETERMINISM_TIERS.md already scopes Tier-3 out correctly; it is a multi-year, x86-only, capital-intensive path that does not serve the agent-systems mission.
- **Agent-eval dashboards / observability framing** (promptfoo/agentops territory). Occupied, commoditized, and dilutes the determinism story that is the entire differentiation.
- **Dependencies on Stateright/Turmoil/Shuttle.** Their value is design shapes (each is a 200–400-line re-implementation); taking them as deps breaks the zero-dep determinism law for no capability gain.
- **Multi-LLM sign-off ceremonies as evidence.** Keep them as labeled opinion; never as admission gates.

## Confidence and decisive unknowns

- **High confidence**: all local reproduced claims (Lane A ran them); Antithesis architecture facts from primary docs/blog (Lane B); niche vacancy (Lane C searches + repo inspections).
- **Medium confidence**: cross-platform bit-identity (single-machine evidence; CI verify matrix reported, not re-executed); Antithesis exploration internals (deliberately proprietary — UNKNOWN).
- **Decisive unknowns**: (1) whether guided exploration actually outperforms uniform Monte-Carlo on *agent-system* workloads specifically — no public benchmark exists; this is testable and should be the first experiment, not an assumption; (2) whether Tier-2 subprocess determinism can be achieved without OS-level interposition at acceptable fidelity — the active goal doc's core risk; (3) Antithesis bug-yield vs Jepsen-style black-box testing — no independent third-party benchmark found, all bug stories are vendor-published.
