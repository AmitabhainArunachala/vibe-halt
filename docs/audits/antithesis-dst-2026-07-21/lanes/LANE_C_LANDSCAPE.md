# Lane C — Adjacent Landscape & OSS Research

**Audit:** vibe-halt vs. state of the art in deterministic simulation testing (DST)
**Date:** 2026-07-21 (all SHAs, activity dates, and license data fetched fresh on this date)
**Author:** Lane C (subagent), via GitHub API (`gh api`), raw source fetches at pinned commits, and project docs
**Scope note:** vibe-halt = zero-dependency in-process Rust DST rig (deterministic scheduler + PRNG seed tree + virtual clock + fault gremlins + always/sometimes properties + multiverse fan-out + chain-hashed trace) for AI-generated code and agent systems. This lane maps the *problem-class* landscape and evaluates what to borrow.

## Evidence modality legend

- **observed** — read in source/docs at the pinned commit during this audit
- **reported** — stated by the project/vendor in README/docs/talks, not independently re-verified
- **inferred** — my conclusion from the evidence

---

## 1. Discovery method

1. Pinned default-branch commits + licenses + push activity for ~30 repos via `gh api repos/<org>/<repo>` and `gh api .../commits?per_page=1`.
2. Read actual source at pinned commits (raw.githubusercontent.com) for the three most relevant candidates: TigerBeetle (`src/vopr.zig`, `src/testing/*`), Stateright (`src/actor.rs`, `src/checker.rs`, `src/actor/spawn.rs`), Shuttle (`shuttle-schedulers/src/*`).
3. GitHub repo search for newer entrants: `deterministic simulation testing`, `LLM agent replay testing`, `agent trace replay`.
4. Fetched FoundationDB's official testing documentation; cross-checked paper lineages (PCT, swarm testing, LDFI) from citations observed in source.

Key finding up front: **the "DST for AI-generated code / agent systems" niche is essentially unoccupied in OSS** (§6). The strongest prior art is database/infrastructure DST (FDB → TigerBeetle → madsim/Turmoil lineage) and concurrency-schedule exploration (Loom/Shuttle/Coyote/PCT).

---

## 2. Category map (by problem class, not vendor)

| Problem class | Core question | Candidates |
|---|---|---|
| **A. Whole-system deterministic simulation** (single-threaded sim of a full distributed system, all nondeterminism stubbed) | "Can we replay an entire system's execution bit-for-bit from a seed?" | FoundationDB Simulation, TigerBeetle VOPR, MadSim, Turmoil, Hiisi (PoC), OpenDST (Java), Antithesis (commercial hypervisor-level) |
| **B. Concurrency/schedule exploration** (interleaving search over shared-memory or async tasks) | "Did we try the interleavings that break it?" | Loom (exhaustive, bounded), Shuttle (randomized PCT/DFS/URW), Coyote (.NET, PCT), Hermit (process-level det. replay) |
| **C. Black-box distributed correctness** (no determinism; real binaries, real faults, checker over histories) | "Does the deployed system violate its consistency model?" | Jepsen, Maelstrom, Chaos Mesh, Chaos Monkey, Molly (LDFI research prototype) |
| **D. Formal specification & model checking** (exhaustive/bounded proof over abstract model) | "Is the *design* correct?" | TLA+/TLC + Apalache, Alloy, P, Stateright (Rust-native, bridges to D via `actor` spawn), FizzBee |
| **E. Coverage-guided fuzzing** (input-space mutation with feedback) | "What inputs drive new coverage?" | AFL++, LibAFL, cargo-fuzz; swarm-testing idea (Groce et al. 2012) implemented concretely in TigerBeetle `fuzz.zig` |
| **F. Record/replay debugging** (capture real nondeterminism, replay later) | "Can I step back through this exact failure?" | rr, Hermit, Shuttle `ReplayScheduler` (schedule-string replay), FDB trace logs |
| **G. Agent/LLM-system testing** (eval harnesses, trace capture, provider mocking) | "Did the agent do the right thing?" | promptfoo, deepeval, giskard, agentops, openai/evals — **none do deterministic simulation or seed-replayable multiverse exploration** |

---

## 3. Comparison matrix

SHAs are default-branch heads as of 2026-07-21. "Replay" = artifact quality for reproducing a failure.

| Name | Repo @ pinned SHA | License | Model | Exploration | Minimization | Replay | Maturity (activity) | vibe-halt relevance |
|---|---|---|---|---|---|---|---|---|
| FoundationDB Simulation | `apple/foundationdb@3d64ad40be` | Apache-2.0 | Actor-model (Flow) whole-cluster sim, single process, virtual time | Nightly random sims, ~"trillion CPU-hours" claimed (reported) | None automated (human + trace logs) | Deterministic rerun from random seed; trace-event logs | Extremely high; pushed 2026-07-21, 16.5k★ | Intellectual parent; **C++/Flow — not borrowable as code**, only doctrine |
| TigerBeetle VOPR | `tigerbeetle/tigerbeetle@97c7a8ef38` | Apache-2.0 | Whole-cluster sim (VSR consensus + storage), seed-driven | Seeded swarm testing (`random_enum_weights`), exponential-distribution fault params, CI fuzzing | Seed + git commit = repro; no auto-shrinker (inferred) | Seed replay; structured checker lattice; sim.tigerbeetle.com live demo | Very high; pushed 2026-07-19, 16.6k★ | **Top design donor.** Zig — borrow architecture, not code |
| Stateright | `stateright/stateright@ab8c8be934` (v0.31.0) | MIT | Rust actor model + explicit-state model checker; same actors runnable over UDP | BFS, DFS, on-demand, random-simulation (`UniformChooser`) | Checker returns discovery **path** (counterexample trace) | Path = sequence of `ActorModelAction`s; exact re-check via `assert_discovery` | Moderate; last push 2025-07-27, 1.8k★ | **Top Rust donor.** Expectation DSL + checker trait shape; embeddable or forkable |
| Shuttle | `awslabs/shuttle@c8a46d3965` | Apache-2.0 | In-process thread/task scheduler control for std/tokio sync | **PCT** (Burckhardt ASPLOS'10, follows Coyote impl), DFS, random, round-robin, URW | None built-in (inferred); schedules serializable | `ReplayScheduler` replays encoded schedule string/file | High; pushed 2026-07-09, 1k★, AWS-maintained, recently refactored into crate family | Borrow PCT + schedule-tape replay design for the scheduler arm |
| Loom | `tokio-rs/loom@948c8cc78b` | MIT | C11-memory-model permutation checker over `loom::sync` mocks | Exhaustive bounded (preemption bound, state reduction) | n/a (exhaustive within bound) | Checkpoint/rewind execution; failing permutation reproducible | High; pushed 2026-02-20, 2.8k★, tokio ecosystem standard | Wrong granularity for whole-system sim; right tool for testing vibe-halt's **own** concurrency |
| Turmoil | `tokio-rs/turmoil@684acc1a8e` | MIT | Multi-host network sim, single thread, seeded RNG; drop-in `tokio::net`/`std::fs`/`io_uring` replacements | Manual fault control + seeded RNG; latency/drops/partitions/crashes/torn writes | None automated (inferred) | Seeded rerun | High & rising; pushed 2026-07-21, 1.2k★; recently restructured into crate family (turmoil-net/fs/io-uring) | Closest Rust analog to vibe-halt's gremlin arm; **bakeoff candidate already in DESIGN.md week 1–2** |
| MadSim | `madsim-rs/madsim@519950efb4` | Apache-2.0 | tokio API-compatible deterministic runtime; sim net/time/fs/rpc | Seeded chaos + fault injection | None automated (inferred) | Seeded rerun | High (RisingWave production use, reported); pushed 2026-02-16, 1.1k★ | Bakeoff candidate; proves "replace the runtime" path at production scale |
| Jepsen | `jepsen-io/jepsen@1b3780adf1` | EPL-1.0 (observed in `jepsen/project.clj`) | Black-box: real nodes, nemesis faults, history → checker (Knossos/Elle) | Random op streams + nemesis schedules | Automatic history minimization limited; Elle finds minimal anomalies (reported) | Full op history + nemesis timeline; **not** deterministic replay | Very high; pushed 2026-07-20, 7.4k★; industry gold standard for consistency | Different arm (D2/black-box). Borrow: history-as-evidence model, nemesis taxonomy |
| Maelstrom | `jepsen-io/maelstrom@480a819702` | EPL-1.0 | Black-box workloads over stdin/stdout protocol + Jepsen checkers | Guided challenge workloads | n/a | Test-run artifacts | High; 3.6k★, used in fly.io "Gossip Glomers" challenges | Good target-workload format inspiration for AI-generated distributed code |
| P | `p-org/P@099c8b7dae` | MIT | State-machine spec language with systematic + random exploration, monitors for safety/liveness | Systematic + random schedule exploration of machine interleavings | Trace simplification (reported) | Replayable schedules; bug traces | High; pushed 2026-07-21, 3.6k★; used at AWS (reported) | Monitor/state-machine property style worth borrowing; language itself out of scope |
| Stateright→(see above) | — | — | — | — | — | — | — | — |
| TLA+ / TLC | `tlaplus/tlaplus@30cc360132` | MIT | Formal spec + explicit-state model checker | Exhaustive (bounded) / simulation mode | Counterexample is minimal-length by BFS construction | Counterexample trace (states) | Very high; pushed 2026-07-18, 3k★ | Design-stage complement, not runtime; borrow "counterexample as first-class artifact" |
| Apalache | `apalache-mc/apalache@d9f0633ebd` | Apache-2.0 | Symbolic (SMT) model checker for TLA+ | Symbolic bounded | Minimal-length traces | Symbolic counterexample | Moderate; 585★ | Same as TLA+ |
| Alloy | `AlloyTools/org.alloytools.alloy@ed89fdb16c` | MIT (repo LICENSE notes "CURRENTLY CODE IS UNDER MIT LICENSE"; GitHub API: NOASSERTION) | Relational logic + SAT | Bounded exhaustive within scope | SAT-minimal instances | Instance/viz | High (long-lived); 853★ | Out of scope; mention only |
| FizzBee | `FizzBee-io/fizzbee@0f2c824854` | Apache-2.0 | Pythonic (Starlark-like) spec + model checker targeting engineers | BFS/DFS + simulation | Trace output | Counterexample paths | Young; 338★, pushed 2026-07-08 | Watch-list; lowers formal-methods barrier for vibe-coders |
| Coyote | `microsoft/coyote@f2c135d201` | MIT (repo LICENSE; GitHub API: NOASSERTION) | .NET async task scheduler control | PCT + systematic strategies (Shuttle's PCT cites Coyote) | None automated (inferred) | Replayable schedules | **Stale**: last push 2024-12-11, 1.6k★ | Proof PCT transfers across runtimes; otherwise superseded by Shuttle for Rust |
| Hermit | `facebookexperimental/hermit@2d67d22eb7` | BSD-style (per-file notices; GitHub: NOASSERTION) | Process-level deterministic execution/replay of unmodified binaries | Re-run determinism, not exploration | n/a | Whole-process record/replay | Active (Meta); pushed 2026-07-21, 1.4k★ | Relevant to vibe-halt **Tier-2/D1** sandboxing ideas; heavy lift |
| rr | `rr-debugger/rr@fb97ee8f83` | MIT (LICENSE observed; GitHub: NOASSERTION) | Record & replay real Linux processes | n/a | Reverse-continue debugging | Full syscall-level recording | Very high; 10.6k★, pushed 2026-07-20 | D2 debugging aid; not DST |
| AFL++ | `AFLplusplus/AFLplusplus@ad5304010a` (stable) | AGPL-3.0 | Coverage-guided binary fuzzing | Mutation + coverage feedback | afl-tmin testcase minimization | Corpus + crashing inputs | Very high; 6.7k★, pushed 2026-07-21 | Inspiration for novelty-guided universe mutation; AGPL = do not link |
| LibAFL | `AFLplusplus/LibAFL@f749dbf8aa` | Apache-2.0 OR MIT (LICENSE-APACHE + LICENSE-MIT observed) | Rust fuzzing framework (modular) | Pluggable mutators/schedulers | Pluggable minimizers | Corpus artifacts | High; 2.6k★ | Possible future engine if vibe-halt wants coverage-guided decision-tape mutation without writing it from scratch |
| cargo-fuzz | `rust-fuzz/cargo-fuzz@bf2fc668da` | Apache-2.0 | libFuzzer harness for Rust | Coverage-guided | libFuzzer `-minimize_crash` | Corpus + artifacts | High; 1.9k★ | Already in DESIGN.md success criteria ("fuzzed at boundaries") |
| Chaos Mesh | `chaos-mesh/chaos-mesh@37ef27f7e7` | Apache-2.0 | K8s runtime fault injection (real systems) | Manual/scheduled experiments | n/a | Experiment records, no determinism | Very high; 7.8k★ | Gremlin taxonomy cross-check for D2 arm only |
| Chaos Monkey | `Netflix/chaosmonkey@eaa28fb761` | Apache-2.0 | Random instance termination in prod | Random | n/a | None | **Stale** (2025-01), 17k★ (legacy fame) | Historical lineage (Netflix chaos → FIT/ChAP lineage-driven FI, not OSS) |
| Molly (LDFI) | `palvaro/molly@a3a6d79508` | none (research) | Lineage-driven fault injection on Dedalus programs | **Solver-guided**: use lineage of good outcomes to pick faults | Fault sets are minimal by construction | Provenance lineage | Dead research prototype (2018), 128★ | **Conceptually the strongest match to vibe-halt's "intelligent fault targeting"** — borrow the lineage→fault-set idea, not the code |
| OpenDST | `pingidentity/opendst@5d6ec3b3c6` | Apache-2.0 | Java DST via bytecode instrumentation (virtual threads, virtual time, sim network) | Seeded deterministic scheduler | n/a | Seed replay | Very new (2024–2026), 21★, Ping Identity | Signals vendor-grade DST spreading beyond databases; validates market timing |
| Hiisi | `penberg/hiisi@fe72bfd90f` | MIT | Rust PoC DB server "TigerBeetle-style with DST" | Seeded sim | n/a | Seed replay | PoC, stale 2024-08, 126★ (Pekka Enberg) | Existence proof a *small* Rust DST rig is buildable; closest spiritual sibling |
| Antithesis | antithesis.com (closed) | proprietary | Hypervisor-level deterministic sim of unmodified binaries + SDK properties, coverage-guided exploration | Feedback-driven universe search (reported) | Auto-minimized repro (reported) | Full replay + time-travel debug | Commercial, production customers | The benchmark vibe-halt positions against; keep as capability checklist |
| promptfoo | `promptfoo/promptfoo@7f6cdd8851` | MIT | LLM eval + red-team harness (YAML assertions, graders) | Prompt/test-case matrices | n/a | Eval result stores | Very high; 23.5k★, pushed 2026-07-21 | Agent-eval **interface** expectations; no determinism story |
| deepeval | `confident-ai/deepeval@c6293c1201` | Apache-2.0 | LLM eval metrics; includes `conversation_simulator.py` (observed) — simulated multi-turn user conversations | Scenario simulation of conversations | n/a | Eval reports | Very high; 17k★ | "Simulator" here = LLM role-play, not DST — note the term collision |
| giskard | `giskard-ai/giskard@b6098b395e` | Apache-2.0 | LLM/agent scan + test generation | Vulnerability scan heuristics | n/a | Reports | High; 5.7k★ | Same category |
| agentops | `agentops-ai/agentops@f8e907b92d` | MIT | Agent trace/observability (session replay UI) | n/a | n/a | **Trace capture/replay viewing** (not re-execution) | High; 5.7k★ | Closest to "agent trace replay" — but it's observability, not deterministic re-execution |
| openai/evals | `openai/evals@8eac7a7de5` | MIT-style (GitHub: NOASSERTION); not archived | Eval registry for models | n/a | n/a | Logs | Legacy; pushed 2026-04-14, 19k★ | Marginal |

---

## 4. Per-candidate evidence (key entries)

### 4.1 FoundationDB — the intellectual parent
- Official docs (observed, fetched 2026-07-21): "Simulation is able to conduct a deterministic simulation of an entire FoundationDB cluster within a single-threaded process… deterministic… perfect repeatability… 10-1 real-to-simulated time factor." Fault folklore includes "swizzle-clogging" (staggered clog/unclog of random node subsets) — a fault *family with internal ordering structure*, directly analogous to vibe-halt's typed fault families with preconditions.
- Simulation enabled by **Flow**, FDB's actor-model language extension — i.e. determinism was achieved by *owning the concurrency substrate* (observed). This is the single most important doctrine for vibe-halt: the D0 grade requires owning scheduling/time/IO, exactly what vibe-halt's engine-owned actors do.
- Simulator implementation present in tree: `fdbrpc/sim2.cpp`, `fdbserver/SimulatedCluster.cpp` (observed at `3d64ad40be`).
- Will Wilson's 2014 StrangeLoop talk ("Testing FoundationDB", youtu.be/4fFDFbi3toc) is the canonical write-up (reported; cited by MadSim README observed). Wilson later co-founded **Antithesis** — the commercial generalization.

### 4.2 TigerBeetle VOPR — strongest architecture donor (DEEP DIVE, source inspected at `97c7a8ef38`)
- `src/vopr.zig` (observed): standalone simulator binary. CLI takes `--seed`, `--lite` (small cluster, crash-only), `--performance`; fault overrides like `--packet_loss_ratio`. Identity = **seed + git commit** (`git_commit` baked via build options). GPA leak detection on exit — sim crashes on any assertion or leak: "assertions are a force multiplier… if any assertion is broken, the simulation crashes" (docs/internals/vopr.md, observed).
- `src/testing/packet_simulator.zig` (observed): `PacketSimulatorOptions` — per-path link state with `one_way_delay_mean/min` (exponential forward delay), `packet_loss_probability`, `packet_replay_probability` (duplicates), `partition_mode` (none/…) + `partition_symmetry` (symmetric/asymmetric) + `partition_probability`/`unpartition_probability` per tick + **stability minimums** (hysteresis so partitions flap at controlled rates), `path_maximum_capacity` with random drops when full, and **path clogging** (`path_clog_probability`, `path_clog_duration_mean`). This is a battle-tested fault-family parameterization vibe-halt's gremlins should copy nearly field-for-field.
- `src/testing/fuzz.zig` (observed): `random_enum_weights` — *"This is swarm testing: some variants are disabled completely, and the rest have wildly different probabilities"* — i.e. Groce et al. swarm testing implemented in 20 lines; `random_int_exponential` for fault magnitudes; `random_id` hot/cold key distributions.
- `src/testing/cluster/` (observed): checker lattice — `state_checker.zig` (commit history consistency), `storage_checker.zig`, `grid_checker.zig` (block-level byte-identical replicas), `manifest_checker.zig`, `journal_checker.zig`. Separately `src/testing/exhaustigen.zig` for exhaustive small-case generation. **Design lesson: properties as a lattice of specialized checkers, each owning one invariant family, run continuously inside the sim loop** — maps 1:1 onto vibe-halt's property monitors.
- Also present: `src/vortex.zig` (secondary supervisor/fuzzer) and `docs/internals/{vopr,testing}.md`. Live interactive sim at sim.tigerbeetle.com (reported in docs).
- Not present (inferred): no automated failure minimizer; reproduction = rerun seed at pinned commit. VOPR Club = their distributed-CI seed-farming practice (reported in talks/blog).

### 4.3 Stateright — strongest *Rust code* donor (DEEP DIVE, source inspected at `ab8c8be934`)
- `src/actor.rs` (observed): `Actor` trait — `type Msg/State/Timer/Random/Storage`; `on_start(id, &storage, &mut Out) -> State`, `on_msg(id, &mut Cow<State>, src, msg, &mut Out)`. Actors send via `Out<Self>` — **pure effect emission, exactly vibe-halt's "convert effects to future events" step**. Doc example shows `ActorModel::new((),()).actor(...).property(Expectation::Always, "less than max", |_, state| ...).checker().spawn_bfs().join()` and `checker.assert_discovery(name, vec![ActorModelAction::Deliver{...}])` — property registration + counterexample-as-value, directly matching vibe-halt's `always(P)`/`sometimes(P)` requirement.
- `src/checker.rs` + `src/checker/` (observed): BFS, DFS, on-demand, representative-path, **rewrite/rewrite_plan** (trace rewriting), and `simulation.rs` exposing `Chooser`/`UniformChooser` — a *random simulation mode inside a model checker*, which is precisely vibe-halt's multiverse fan-out over an actor model.
- `src/actor/spawn.rs` (observed): the **same actor code spawns over real UDP sockets** (Id ↔ SocketAddrV4 mapping) — model-check and deploy one implementation. For vibe-halt this suggests a path where D0-simulated agent code can also run live.
- v0.31.0, MIT, ~13 direct deps (dashmap, parking_lot, tiny_http…). Last push **2025-07-27** — quiet but stable. Integration cost for zero-dep vibe-halt: taking it as a dependency breaks the zero-dep goal and drags in dashmap/parking_lot; **borrowing the trait shapes (Actor/Out/Expectation/Checker discovery path) is a 200–400-line re-implementation, not a port** (inferred).
- Examples dir (observed): `paxos.rs`, `raft.rs`, `2pc.rs`, `linearizable-register.rs` — canonical oracle implementations worth stealing as test fixtures.

### 4.4 Shuttle — strongest scheduler-strategy donor (DEEP DIVE, source inspected at `c8a46d3965`)
- Recently refactored (May–Jun 2026) into a workspace: `shuttle-core`, `shuttle`, `shuttle-std`, `shuttle-schedulers`, `shuttle-explorer`, `wrappers` (observed tree).
- `shuttle-schedulers/src/lib.rs` (observed) exports: `PctScheduler`, `DfsScheduler`, `RandomScheduler`, `ReplayScheduler`, `RoundRobinScheduler`, `UrwRandomScheduler`, `AnnotationScheduler`, `UncontrolledNondeterminismCheckScheduler`.
- `pct.rs` (observed header): implements **PCT (Burckhardt et al., ASPLOS 2010)** "following the one in Coyote… supports dynamically determining the bound" — priority-permutation with `change_points` of length `max_depth-1`, seeded `Pcg64Mcg`, `SHUTTLE_RANDOM_SEED` env override. This is the mathematically grounded version of vibe-halt's "preemption-bounded schedule perturbation" (DESIGN.md §5, item 5): PCT with depth d finds bugs needing ≤d preemptions with probability ≥ 1/n^(k·d) (reported, paper) — a real *exploration guarantee* vibe-halt currently lacks.
- `replay.rs` (observed): `ReplayScheduler::new_from_encoded(&str)` / `new_from_file(path)` — **a schedule serialized to a string is the entire replay artifact**. vibe-halt's decision tape is a strict superset of this; Shuttle validates the minimal viable form.
- README (observed): explicit soundness/scalability trade-off positioning vs Loom ("randomized testing finds most concurrency bugs, which tend not to be adversarial") — a useful honesty precedent matching vibe-halt's "radical honesty about determinism boundaries".
- Integration cost (inferred): Shuttle controls *threads and sync primitives*, not a virtual-clock actor event loop; adopting it wholesale would invert vibe-halt's architecture. Borrow PCT + URW as exploration strategies over vibe-halt's existing decision points instead.

### 4.5 Turmoil & MadSim — the runtime-replacement Rust DST pair
- Turmoil README (observed at `684acc1a8e`, fetched 2026-07-21): "a family of crates for deterministic simulation testing… multiple concurrent hosts within a single thread… latency, drops, partitions, crashes, torn writes… under manual control or a seeded RNG." Now ships `turmoil-net` / `turmoil-fs` / `turmoil-io-uring` as drop-in `tokio`/`std` replacements — same interposition strategy as MadSim, crate-modular. Under active development (pushed today).
- MadSim README (observed): API-compatible `madsim-tokio`, `madsim-tonic`, `madsim-etcd-client`, `madsim-rdkafka`, `madsim-aws-sdk-s3`; RisingWave's two-part DST writeup linked; explicitly "borrowed from FoundationDB and sled simulation guide". Proves the approach sustains a production database company.
- For vibe-halt (inferred): both prove Rust DST works, but both require the *target to link their runtime* — appropriate for Rust-database targets, wrong shape for testing arbitrary AI-generated code and Python/JS agents. They are the week 1–2 bakeoff baseline, not the destination.

### 4.6 Jepsen / Maelstrom — the black-box arm
- Jepsen: Clojure; real clusters + nemesis; linearizability checkers (Knossos/Elle). License EPL-1.0 (observed in `project.clj`; GitHub API reports null). Relevance: property checking over *histories* rather than in-sim monitors — matches vibe-halt's D2/recorded-cassette arm; Elle's "minimal anomaly" reports are the bar for evidence-bundle quality (reported).
- Maelstrom: standardized stdin/stdout workload protocol with demo workloads (broadcast, CRDTs, Kafka-style log, txn). An AI-coding-agent-friendly target format — literally used to teach distributed systems to humans and LLMs today (reported/inferred).

### 4.7 Formal methods line (TLA+/Apalache, Alloy, P, FizzBee, Stateright)
- All pin-checked (matrix). P's monitors + explicit liveness properties, and TLA+'s counterexample-trace-as-artifact, are the two ideas worth absorbing into vibe-halt's property system. FizzBee is the 2024-era attempt to make model checking accessible to non-specialists — watch it for UX ideas; its Pythonic spec surface overlaps the vibe-coder demographic (inferred).

### 4.8 Fuzzing lineage & swarm testing
- TigerBeetle's `fuzz.zig` (observed) is the only production-quality *swarm testing* implementation located in this audit — 20 lines, huge effect (each seed gets a disjoint random feature/probability mask, multiplying coverage diversity across the multiverse). Direct precedent for vibe-halt's multiverse fan-out: **give each universe a randomized *configuration mask*, not just a random draw stream**.
- Groce et al., "Swarm Testing" (ISSTA 2012) — the paper behind it (reported, cited here as lineage).
- AFL++ (AGPL — avoid linking), LibAFL (Apache/MIT, Rust, modular) and cargo-fuzz cover boundary fuzzing; DESIGN.md already commits to "fuzzed at boundaries".

### 4.9 Chaos engineering & lineage-driven fault injection
- Netflix's FIT/ChAP lineage work is **not OSS** (reported); OSS residue is Chaos Monkey (stale) and Chaos Mesh (K8s runtime faults, no determinism). The research core is **Molly/LDFI** (Alvaro et al.): use data lineage of *successful* outcomes to compute minimal fault sets that could have prevented them — solver-guided instead of random gremlins (reported; repo pinned, dead since 2018). This is the strongest published answer to vibe-halt's "intelligent targeting: bias toward state transitions, retry/ack/commit paths" ambition.

---

## 5. Section 8 question: does "DST for AI-generated code / agent systems" already exist?

**No serious instance found (observed + inferred).**
- GitHub search `LLM agent replay testing` → zero relevant repos; `agent trace replay` → only 0–2-star personal projects (observed via `gh search repos`, 2026-07-21).
- The agent-testing category (promptfoo 23.5k★, deepeval 17k★, giskard, agentops) is entirely **eval/observability**: assertion matrices, LLM-as-judge, session trace capture. agentops "session replay" replays *traces to a viewer*, not executions. deepeval's "ConversationSimulator" simulates *users via LLM role-play*, not universes. None has a determinism grade, seed tree, virtual clock, or replay artifact that re-executes (observed from repos/docs).
- Record/replay for LLM calls exists only as HTTP-cassette tooling (vcrpy/Polly-style; DESIGN.md already plans cassettes) — orthogonal and unowned-in-Rust (inferred).
- Conclusion: vibe-halt's claimed niche — **deterministic multiverse simulation of agent harnesses with typed fault families and replayable evidence** — is genuinely unoccupied in OSS; the nearest occupants are Antithesis (commercial, general-purpose) and the agent-observability tools (no determinism).

---

## 6. What to build / borrow / adapt / avoid for vibe-halt

**Borrow (nearly verbatim, with attribution):**
1. **TigerBeetle `PacketSimulatorOptions` field set** (Apache-2.0, Zig→Rust rewrite): per-path exponential delays, loss/replay ratios, partition mode+symmetry+hysteresis stability minimums, path capacity + clog durations. Highest-density fault-design prior art found.
2. **TigerBeetle swarm-testing idiom** (`random_enum_weights`): per-universe randomized fault-family masks. 20 lines; multiplies multiverse diversity.
3. **Shuttle's PCT scheduler** (Burckhardt 2010; Coyote-derived implementation notes in `pct.rs`): gives the "preemption-bounded schedule perturbation" roadmap item an actual probabilistic guarantee. Also `ReplayScheduler`'s schedule-string-as-artifact minimalism.
4. **Stateright's `Expectation` DSL + discovery-path API**: `Always`/`Sometimes`/`Eventually` + `assert_discovery(path)` — the exact always/sometimes property shape, with counterexamples as re-checkable values. Reimplement trait-shape only (keeps zero-dep).
5. **FDB doctrine**: own the concurrency substrate; sim crashes on any assertion; checker lattice over one invariant family each; "swizzle-clog"-style structured multi-step fault sequences as first-class fault plans.

**Adapt (same idea, different shape):**
6. **Molly/LDFI lineage-driven fault selection** → vibe-halt's "intelligent targeting": record causal lineage of successful universe outcomes; bias faults toward events whose lineage feeds receipts/commits. No code to take (dead research prototype) — it's an algorithm.
7. **Jepsen history model** → D2/cassette arm: an agent run's recorded provider/tool IO is a "history"; checkers (at-most-once, conservation) run over histories even when replay isn't certified. Elle-style minimal-anomaly reporting as evidence-bundle quality bar.
8. **Maelstrom workload protocol** → fixture format for AI-generated target workloads (stdin/stdout contracts make LLM-written targets testable without bespoke adapters).

**Use as tools, don't embed:**
9. **Loom** — test vibe-halt's *own* scheduler/kernel concurrency (exhaustive within bound).
10. **cargo-fuzz** (+ optionally LibAFL later) — boundary fuzzing of trace parsers/cassette decoders per success criterion 10.
11. **Turmoil/MadSim** — bakeoff baseline (already scheduled week 1–2); also candidates if vibe-halt ever ships an optional "link-this-runtime" fast path for Rust targets.

**Avoid:**
12. **AFL++ code** (AGPL-3.0) — ideas fine, linkage no.
13. **Hypervisor/process-level determinism (Antithesis-style, Hermit, rr) for v0.1** — DESIGN.md already scopes this out; Hermit/rr are debugging aids for D2, not the D0 core.
14. **Taking Stateright/Turmoil as dependencies** — breaks zero-dep; their value here is design, and both are small enough that the shapes port cleanly.
15. **Agent-observability framing (agentops/promptfoo)** — different problem; do not dilute the determinism story into "eval dashboards".

**Watch-list:** OpenDST (vendor-grade Java DST, bytecode interposition — if it matures, it validates and competes), FizzBee (spec UX for non-specialists), Turmoil's crate-family refactor (may become directly composable).

---

## 7. SOURCE_MANIFEST

```csv
source_id,title,url,source_type,published_at,accessed_at,relevance,notes
C01,TigerBeetle repo @97c7a8ef38,https://github.com/tigerbeetle/tigerbeetle/tree/97c7a8ef38,repo,2026-07-19,2026-07-21,critical,"Apache-2.0; src/vopr.zig + src/testing/* inspected; fault params + swarm testing + checker lattice"
C02,TigerBeetle DST docs (vopr.md),https://github.com/tigerbeetle/tigerbeetle/blob/97c7a8ef38/docs/internals/vopr.md,doc,2026,2026-07-21,critical,"Seed+commit replay identity; assertions-as-force-multiplier doctrine; cites FDB+Antithesis"
C03,packet_simulator.zig source,https://raw.githubusercontent.com/tigerbeetle/tigerbeetle/97c7a8ef38/src/testing/packet_simulator.zig,code,2026,2026-07-21,critical,"PacketSimulatorOptions: delay/loss/replay/partition hysteresis/clog fields"
C04,fuzz.zig source,https://raw.githubusercontent.com/tigerbeetle/tigerbeetle/97c7a8ef38/src/testing/fuzz.zig,code,2026,2026-07-21,high,"random_enum_weights = production swarm testing"
C05,Stateright repo @ab8c8be934,https://github.com/stateright/stateright/tree/ab8c8be934,repo,2025-07-27,2026-07-21,critical,"MIT; v0.31.0; actor.rs/checker.rs/spawn.rs inspected; Expectation DSL + discovery paths + UDP spawn"
C06,Shuttle repo @c8a46d3965,https://github.com/awslabs/shuttle/tree/c8a46d3965,repo,2026-06-16,2026-07-21,critical,"Apache-2.0; shuttle-schedulers/{pct,replay,dfs,urw}.rs inspected; PCT per Burckhardt 2010 via Coyote"
C07,Burckhardt et al. PCT paper (ASPLOS 2010),https://www.microsoft.com/en-us/research/wp-content/uploads/2016/02/asplos277-pct.pdf,paper,2010,2026-07-21,high,"Probabilistic bug-finding guarantee; cited in Shuttle pct.rs"
C08,FoundationDB testing docs,https://apple.github.io/foundationdb/testing.html,doc,live,2026-07-21,critical,"Single-thread deterministic cluster sim; swizzle-clogging; trillion CPU-hours claim (reported)"
C09,apple/foundationdb @3d64ad40be,https://github.com/apple/foundationdb/tree/3d64ad40be,repo,2026-07-21,2026-07-21,high,"Apache-2.0; fdbrpc/sim2.cpp + fdbserver/SimulatedCluster.cpp present"
C10,Will Wilson StrangeLoop 2014 talk,https://www.youtube.com/watch?v=4fFDFbi3toc,talk,2014,2026-07-21,high,"Canonical FDB simulation talk; cited by MadSim README"
C11,Antithesis,https://www.antithesis.com/,vendor,live,2026-07-21,high,"Commercial hypervisor-level DST; capability benchmark; not OSS"
C12,Loom @948c8cc78b,https://github.com/tokio-rs/loom/tree/948c8cc78b,repo,2026-02-20,2026-07-21,high,"MIT; C11 permutation checker; src/lib.rs docs observed"
C13,Turmoil @684acc1a8e,https://github.com/tokio-rs/turmoil/tree/684acc1a8e,repo,2026-07-21,2026-07-21,high,"MIT; crate family turmoil-net/fs/io-uring; seeded RNG fault control"
C14,MadSim @519950efb4,https://github.com/madsim-rs/madsim/tree/519950efb4,repo,2026-02-16,2026-07-21,high,"Apache-2.0; tokio-compatible sim runtime; RisingWave production use"
C15,RisingWave DST writeup part 1,https://www.risingwave.com/blog/deterministic-simulation-a-new-era-of-distributed-system-testing/,blog,2022,2026-07-21,medium,"Linked from MadSim README"
C16,Jepsen @1b3780adf1,https://github.com/jepsen-io/jepsen/tree/1b3780adf1,repo,2026-07-20,2026-07-21,high,"EPL-1.0 (observed in project.clj); black-box histories + nemesis"
C17,Maelstrom @480a819702,https://github.com/jepsen-io/maelstrom/tree/480a819702,repo,2026-07-10,2026-07-21,medium,"EPL-1.0; stdin/stdout workload protocol"
C18,P @099c8b7dae,https://github.com/p-org/P/tree/099c8b7dae,repo,2026-07-21,2026-07-21,medium,"MIT; state-machine monitors; AWS usage reported"
C19,TLA+ tools @30cc360132,https://github.com/tlaplus/tlaplus/tree/30cc360132,repo,2026-07-18,2026-07-21,medium,"MIT; TLC model checker"
C20,Apalache @d9f0633ebd,https://github.com/apalache-mc/apalache/tree/d9f0633ebd,repo,2026-07-10,2026-07-21,low,"Apache-2.0; symbolic TLA+"
C21,Alloy @ed89fdb16c,https://github.com/AlloyTools/org.alloytools.alloy/tree/ed89fdb16c,repo,2026-06-11,2026-07-21,low,"MIT per repo LICENSE note; GitHub API NOASSERTION"
C22,FizzBee @0f2c824854,https://github.com/FizzBee-io/fizzbee/tree/0f2c824854,repo,2026-07-08,2026-07-21,medium,"Apache-2.0; 2024-era engineer-friendly model checker; watch-list"
C23,Coyote @f2c135d201,https://github.com/microsoft/coyote/tree/f2c135d201,repo,2024-12-11,2026-07-21,medium,"MIT; stale; PCT implementation Shuttle cites"
C24,Hermit @2d67d22eb7,https://github.com/facebookexperimental/hermit/tree/2d67d22eb7,repo,2026-07-21,2026-07-21,medium,"BSD-style per-file; process-level deterministic replay"
C25,rr @fb97ee8f83,https://github.com/rr-debugger/rr/tree/fb97ee8f83,repo,2026-07-20,2026-07-21,medium,"MIT; record/replay debugger"
C26,AFL++ @ad5304010a,https://github.com/AFLplusplus/AFLplusplus/tree/ad5304010a,repo,2026-07-21,2026-07-21,medium,"AGPL-3.0 — avoid linking"
C27,LibAFL @f749dbf8aa,https://github.com/AFLplusplus/LibAFL/tree/f749dbf8aa,repo,2026-07-12,2026-07-21,medium,"Apache-2.0 OR MIT observed; modular Rust fuzzing"
C28,cargo-fuzz @bf2fc668da,https://github.com/rust-fuzz/cargo-fuzz/tree/bf2fc668da,repo,2026-07-20,2026-07-21,medium,"Apache-2.0; boundary fuzzing per DESIGN.md criterion 10"
C29,Groce et al. Swarm Testing (ISSTA 2012),https://dl.acm.org/doi/10.1145/2338965.2336763,paper,2012,2026-07-21,high,"Feature-mask diversity idea; production impl observed in TigerBeetle fuzz.zig"
C30,Chaos Mesh @37ef27f7e7,https://github.com/chaos-mesh/chaos-mesh/tree/37ef27f7e7,repo,2026-07-21,2026-07-21,low,"Apache-2.0; K8s runtime chaos, no determinism"
C31,Chaos Monkey @eaa28fb761,https://github.com/Netflix/chaosmonkey/tree/eaa28fb761,repo,2025-01-06,2026-07-21,low,"Apache-2.0; stale; Netflix chaos lineage (FIT/ChAP not OSS)"
C32,Molly (LDFI) @a3a6d79508,https://github.com/palvaro/molly/tree/a3a6d79508,repo,2018-11-04,2026-07-21,high,"No license; dead research prototype; lineage-driven fault injection concept"
C33,OpenDST @5d6ec3b3c6,https://github.com/pingidentity/opendst/tree/5d6ec3b3c6,repo,2026-07-16,2026-07-21,medium,"Apache-2.0; Java DST via bytecode instrumentation; new entrant validating market"
C34,Hiisi @fe72bfd90f,https://github.com/penberg/hiisi/tree/fe72bfd90f,repo,2024-08-12,2026-07-21,medium,"MIT; small Rust TigerBeetle-style DST PoC; spiritual sibling"
C35,awesome-deterministic-simulation-testing @b4c2732880,https://github.com/ivanyu/awesome-deterministic-simulation-testing/tree/b4c2732880,list,2026-04-29,2026-07-21,medium,"CC-BY-4.0 curated DST resource list"
C36,promptfoo @7f6cdd8851,https://github.com/promptfoo/promptfoo/tree/7f6cdd8851,repo,2026-07-21,2026-07-21,medium,"MIT; agent eval/red-team; no determinism"
C37,deepeval @c6293c1201,https://github.com/confident-ai/deepeval/tree/c6293c1201,repo,2026-07-21,2026-07-21,medium,"Apache-2.0; conversation_simulator.py observed = LLM role-play sim, not DST"
C38,giskard @b6098b395e,https://github.com/giskard-ai/giskard/tree/b6098b395e,repo,2026-07-21,2026-07-21,low,"Apache-2.0; LLM scan"
C39,agentops @f8e907b92d,https://github.com/agentops-ai/agentops/tree/f8e907b92d,repo,2026-06-25,2026-07-21,medium,"MIT; agent trace observability; 'replay' = viewing, not re-execution"
C40,openai/evals @8eac7a7de5,https://github.com/openai/evals/tree/8eac7a7de5,repo,2026-04-14,2026-07-21,low,"Not archived; legacy eval registry"
C41,vibe-halt DESIGN.md,/Users/dhyana/vibe-halt/DESIGN.md,doc,2026-07-20,2026-07-21,internal,"D0/D1/D2 grades, decision tape, seed tree, gremlins, multiverse + shrinker spec this lane serves"
```
