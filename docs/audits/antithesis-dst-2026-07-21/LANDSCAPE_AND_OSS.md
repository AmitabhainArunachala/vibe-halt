# Landscape and Open-Source Prior Art

> Curated from `lanes/LANE_C_LANDSCAPE.md` (27-row matrix, all SHAs/licenses/activity fetched fresh 2026-07-21 via `gh api`; TigerBeetle, Stateright, Shuttle inspected at source level at pinned commits).
> Modality: observed = read at pinned commit / report page during audit; reported = project/vendor claim; inferred = analyst conclusion.

## 1. Discovery method and category map

Method: pin ~30 repos (SHA, license, push date) → source-level deep dives on the 3 strongest → GitHub searches for newer entrants (`deterministic simulation testing`, `LLM agent replay testing`, `agent trace replay`) → paper lineage (PCT, swarm testing, LDFI).

| class | question | candidates |
|---|---|---|
| A. Whole-system deterministic simulation | replay an entire system bit-for-bit from a seed? | FoundationDB Simulation, TigerBeetle VOPR, MadSim, Turmoil, Hiisi, OpenDST, **Antithesis** (commercial, hypervisor) |
| B. Concurrency/schedule exploration | did we try the breaking interleavings? | Loom, Shuttle (PCT), Coyote, Hermit |
| C. Black-box distributed correctness | does the real deployment violate its model? | Jepsen, Maelstrom, Chaos Mesh, Molly (LDFI) |
| D. Formal specification/model checking | is the *design* correct? | TLA+/Apalache, Alloy, P, Stateright, FizzBee |
| E. Coverage-guided fuzzing | what inputs drive new coverage? | AFL++, LibAFL, cargo-fuzz; swarm testing (Groce 2012 → TigerBeetle `fuzz.zig`) |
| F. Record/replay debugging | step back through this exact failure? | rr, Hermit, Shuttle ReplayScheduler |
| G. Agent/LLM-system testing | did the agent do the right thing? | promptfoo, deepeval, giskard, agentops — **none do deterministic simulation or re-executable replay** |

## 2. Open-source repository matrix

Top rows of the comparison matrix (full 27-row matrix in Lane C §3):

SHAs = default-branch heads 2026-07-21.

| name | repo @ SHA | license | model | exploration | replay artifact | activity | vibe-halt relevance |
|---|---|---|---|---|---|---|---|
| TigerBeetle VOPR | `tigerbeetle/tigerbeetle@97c7a8ef38` | Apache-2.0 | whole-cluster sim (VSR+storage), seed-driven | swarm testing (`random_enum_weights`), exponential fault params | seed+commit = repro | very high (pushed 2026-07-19, 16.6k★) | **top design donor** (fault params, swarm masks, checker lattice) |
| Stateright | `stateright/stateright@ab8c8be934` (v0.31.0) | MIT | Rust actor model + explicit-state checker; same actors over UDP | BFS/DFS/on-demand/random-sim | discovery **path** re-checkable via `assert_discovery` | moderate (2025-07, 1.8k★) | **top Rust shape donor** (Expectation DSL ≈ always/sometimes) |
| Shuttle | `awslabs/shuttle@c8a46d3965` | Apache-2.0 | in-process thread/task scheduler control | **PCT** (Burckhardt 2010, via Coyote), DFS, URW, random | schedule-string replay (`ReplayScheduler`) | high (2026-07-09, AWS) | PCT + schedule-tape design for the scheduler arm |
| FoundationDB Sim | `apple/foundationdb@3d64ad40be` | Apache-2.0 | actor-model (Flow) whole-cluster single-process sim | nightly random sims | seed rerun + trace logs | very high (16.5k★) | doctrine source: own the concurrency substrate |
| Turmoil | `tokio-rs/turmoil@684acc1a8e` | MIT | multi-host net/fs sim, single thread, seeded RNG | manual + seeded | seeded rerun | high (pushed 2026-07-21) | closest Rust analog to gremlin arm; bakeoff baseline |
| MadSim | `madsim-rs/madsim@519950efb4` | Apache-2.0 | tokio-API-compatible deterministic runtime | seeded chaos | seeded rerun | high (RisingWave production) | proves runtime-replacement DST at production scale |
| Loom | `tokio-rs/loom@948c8cc78b` | MIT | C11 permutation checker | exhaustive bounded | checkpoint/rewind | high (tokio standard) | tool for testing vibe-halt's *own* scheduler |
| Jepsen | `jepsen-io/jepsen@1b3780adf1` | EPL-1.0 | black-box histories + nemesis | random ops + nemesis | op history (non-deterministic) | very high (7.4k★) | history-as-evidence model for the cassette arm |
| Molly (LDFI) | `palvaro/molly@a3a6d79508` | none (research) | lineage-driven fault injection | **solver-guided** fault sets | provenance lineage | dead (2018) | strongest published "intelligent fault targeting" algorithm |
| OpenDST | `pingidentity/opendst@5d6ec3b3c6` | Apache-2.0 | Java DST via bytecode interposition | seeded scheduler | seed replay | new (2024–26, 21★) | emergent competitor; validates market timing |
| Antithesis | antithesis.com (closed) | proprietary | hypervisor-level sim of unmodified binaries | coverage+RL-guided tree fuzzing (reported) | full replay + time-travel | commercial, funded ($152M) | capability benchmark; see ANTITHESIS_DOSSIER.md |

Also pinned in Lane C: P, TLA+, Apalache, Alloy, FizzBee, Coyote (stale), Hermit, rr, AFL++ (AGPL — do not link), LibAFL, cargo-fuzz, Chaos Mesh, Chaos Monkey (stale), Hiisi, Maelstrom, promptfoo, deepeval, giskard, agentops, openai/evals.

## 3. Source-level deep dives (inspected at pinned commits, not READMEs)

**TigerBeetle** (`97c7a8ef38`, Apache-2.0):
- `src/testing/packet_simulator.zig`: `PacketSimulatorOptions` — per-path exponential one-way delays, loss/replay probabilities, partition mode+symmetry+**hysteresis stability minimums**, path capacity with drops, **path clogging** with durations. Battle-tested fault parameterization to copy nearly field-for-field.
- `src/testing/fuzz.zig`: `random_enum_weights` — *production swarm testing in ~20 lines* ("some variants are disabled completely, and the rest have wildly different probabilities").
- `src/testing/{state,storage,grid,manifest,journal}_checker.zig`: **checker lattice** — one specialized checker per invariant family, run continuously in-sim; assertions crash the sim. Identity = seed + git commit.

**Stateright** (`ab8c8be934`, MIT):
- `src/actor.rs`: `Actor` trait with pure effect emission via `Out<Self>` — exactly vibe-halt's "effects → future events" step. `Expectation::Always/Sometimes` property registration + `checker.assert_discovery(name, path)` — counterexamples as re-checkable values.
- `src/actor/spawn.rs`: same actor code deploys over real UDP — sim-check and live-run one implementation.
- ~13 deps (dashmap, parking_lot…) — **borrow trait shapes (200–400 lines), do not depend**.

**Shuttle** (`c8a46d3965`, Apache-2.0):
- `shuttle-schedulers/src/pct.rs`: **PCT** — with depth d, finds bugs needing ≤d preemptions with probability ≥ 1/n^(k·d) (reported, Burckhardt ASPLOS'10). The mathematically grounded version of vibe-halt's unimplemented "preemption-bounded perturbation" (DESIGN.md §5).
- `replay.rs`: a serialized schedule string *is* the replay artifact — validates the minimal form of a decision tape.
- Honest soundness/scalability positioning vs Loom — a precedent for vibe-halt's boundary honesty.

## 4. Direct commercial comparables and emerging/adjacent approaches

- **Direct commercial comparable:** Antithesis (closed, hypervisor-level, $152M disclosed funding) — the only serious commercial DST platform; full treatment in `ANTITHESIS_DOSSIER.md`. OpenDST (`pingidentity/opendst@5d6ec3b3c6`, Apache-2.0, Java bytecode interposition, 2024–26) is the emerging vendor-grade OSS signal that DST is spreading beyond database companies.
- **Adjacent agent-testing approaches (emerging but wrong shape):** promptfoo, deepeval, giskard, agentops — eval/observability only, no determinism grade, no seed tree, no re-executable replay (see the decisive finding below).

## 5. Research systems and formal methods line

- **Formal methods:** TLA+/TLC + Apalache (counterexample-as-artifact), Alloy, P (monitors + liveness), FizzBee (engineer-friendly UX, watch-list), Stateright (Rust-native, bridges spec→actor simulation — deep-dived above).
- **Research algorithms:** PCT (Burckhardt ASPLOS 2010 — probabilistic schedule-exploration guarantee; implemented in Shuttle/Coyote); swarm testing (Groce ISSTA 2012 — production implementation observed in TigerBeetle `fuzz.zig`); lineage-driven fault injection (Molly/LDFI, Alvaro et al. — solver-guided fault selection from good-outcome lineage; dead prototype, live idea).

## 6. The decisive landscape finding

**"DST for AI-generated code / agent systems" is unoccupied in OSS** (observed + inferred):
- GitHub searches (`LLM agent replay testing`, `agent trace replay`, 2026-07-21): zero relevant repos; only 0–2-star personal projects.
- The agent-testing category (promptfoo 23.5k★, deepeval 17k★, giskard, agentops) is entirely eval/observability: assertion matrices, LLM-as-judge, trace *viewing*. deepeval's "ConversationSimulator" is LLM role-play, not universe simulation. None has a determinism grade, seed tree, virtual clock, or re-executable replay artifact.
- LLM-call record/replay exists only as HTTP-cassette tooling (vcrpy-style) — orthogonal, unowned in Rust.
- Nearest occupants: Antithesis (closed, general-purpose, hypervisor) and agent-observability tools (no determinism). vibe-halt's claimed niche is real and empty.

## 7. Relevance and integration constraints for vibe-halt

**Borrow nearly verbatim (with attribution):**
1. TigerBeetle `PacketSimulatorOptions` field set (Zig→Rust rewrite of the parameterization, not the code).
2. TigerBeetle swarm-testing idiom: per-universe randomized fault-family masks (~20 lines).
3. Shuttle PCT + schedule-string replay design.
4. Stateright `Expectation` DSL + discovery-path API (reimplement, keep zero-dep).
5. FDB doctrine: own the concurrency substrate; sim crashes on any assertion; checker lattice; structured multi-step fault plans ("swizzle-clogging").

**Adapt (algorithm, not code):**
6. Molly/LDFI lineage-driven fault selection → bias faults toward events whose causal lineage feeds commits/receipts.
7. Jepsen history model → cassette arm: recorded provider/tool IO as checkable histories; Elle-style minimal-anomaly reporting as evidence-bundle quality bar.
8. Maelstrom stdin/stdout workload protocol → fixture format making LLM-written targets testable without bespoke adapters.
9. Antithesis test-template command algebra (7 prefixes) and causality-analysis algorithm (rewind + re-explore → bug-probability-over-time) — both published in enough detail to reimplement.

**Use as tools, don't embed:** Loom (test vibe-halt's own scheduler), cargo-fuzz (boundary fuzzing per DESIGN.md criterion 10), Turmoil/MadSim (bakeoff baselines).

**Avoid:** AFL++ code (AGPL-3.0); hypervisor/process-level determinism for v0.x (scoped out correctly by DETERMINISM_TIERS.md); Stateright/Turmoil/Shuttle *as dependencies* (breaks zero-dep law; shapes port in 200–400 lines); agent-observability framing (occupied, dilutes differentiation).

**Watch-list:** OpenDST (Java DST, validates + competes), FizzBee (spec UX for non-specialists), Turmoil crate-family refactor (may become composable), dhyve (OSS deterministic hypervisor — if it matures, the Tier-3 calculus changes).

**License note:** vibe-halt itself currently has **no LICENSE file** (GitHub `licenseInfo: null`, 2026-07-21) — blocks adoption, contribution, and any corporate use; fixing this is a prerequisite for the OSS-niche strategy.
