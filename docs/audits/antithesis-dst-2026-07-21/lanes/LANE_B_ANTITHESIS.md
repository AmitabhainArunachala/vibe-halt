# LANE B — Antithesis primary-source dossier

- Lane: B (Antithesis primary-source research)
- Audit: vibe-halt vs Antithesis DST comparison
- Access date (all sources): 2026-07-21
- Method: official site/docs/blog only as technical proof; customer stories published on antithesis.com marked as vendor-published customer evidence; market signals separated from technical proof.
- Modality legend: **observed** = directly read on a fetched primary page; **reported** = stated by Antithesis or a customer in a primary source (not independently verified); **inferred** = my synthesis from multiple primary sources. Nothing here is reverse-engineered from marketing into certainty; unknowns are marked UNKNOWN.

---

## 1. Identity, lineage, funding

| Claim | Modality | Source | Date | Falsifier |
|---|---|---|---|---|
| Antithesis was founded January 2018 by Will Wilson and Dave Scherer, both FoundationDB veterans (Scherer was FoundationDB's chief architect/co-founder; Wilson joined FDB April 2013 and built the deterministic simulation harness). | observed (company timeline) | https://antithesis.com/company/about/ ; https://antithesis.com/blog/is_something_bugging_you/ | about page undated; launch post 2024-02-13 | Corporate filings showing different incorporation date/founders. |
| FoundationDB lineage: FoundationDB began 2009 (Dave Scherer, Dave Rosenthal, Nick Lavezzo); acquired by Apple March 2015; its deterministic simulation framework is the direct ancestor of Antithesis. | observed | https://antithesis.com/company/about/ ; https://antithesis.com/blog/is_something_bugging_you/ | 2024-02-13 | Contradictory historical record. |
| ~5.5 years in stealth; emerged from stealth Feb 2024, announced $47M seed led by Amplify Partners, Tamarack Global, First in Ventures. | observed | https://antithesis.com/company/about/ ; https://antithesis.com/blog/is_something_bugging_you/ | 2024-02-13 | Funding databases showing different amounts/leads. |
| $105M Series A in December 2025, led by Jane Street (an existing customer — "when your customer leads your Series A"). | observed | https://antithesis.com/company/about/ ; https://antithesis.com/blog/2025/series_a/ | 2025-12-03 | Press/records contradicting amount or lead. |
| First customer began using Antithesis September 2019; single beta customer in 2020. | observed | https://antithesis.com/company/about/ ; https://antithesis.com/blog/2025/gradius/ | 2025-02-21 | — |
| Total known funding as of 2026-07: $47M seed + $105M Series A = ~$152M disclosed. No Series B or later found on official channels as of access date. | inferred | the two sources above | — | Announcement of a later round. |
| Current headcount, revenue, valuation: **UNKNOWN** (not published on official site; careers page exists at /company/careers/ but was not mined for counts). | — | — | — | — |

Notable internal-technology claims (reported, not verified):
- The Determinator hypervisor is "built with a pure functional language that not many people use" (employee quote on about page — consistent with Haskell; language not officially named). **reported**, falsifier: Antithesis publishing the actual implementation language.
- They wrote "our own strongly consistent hybrid analytic-operational datastore" because branching-timeline data models are unusual (Series A post footnote). **reported**.
- NixOS is used "literally everywhere at our company" (Madness blog). **reported**.

## 2. Deterministic execution substrate ("the Determinator")

Publicly established (high confidence, from the Mar 2024 hypervisor post + docs environment page):

| Claim | Modality | Source | Date | Falsifier |
|---|---|---|---|---|
| The substrate is a **custom deterministic hypervisor**, forked from FreeBSD's **bhyve**, with much standard functionality removed; runs on Intel server CPUs using VMX hardware virtualization. | observed | https://antithesis.com/blog/deterministic_hypervisor/ | 2024-03-20 | Antithesis publishing contradictory architecture docs. |
| The entire system under test (SUT) — all containers, client/test workload, checkers — runs inside **one deterministic VM**; the unit of reproducibility is the whole interconnected system state, not a single process. | observed | same; https://antithesis.com/docs/configuration/the_antithesis_environment/ | 2024-03-20 / undated docs | — |
| Each Determinator instance is pinned to **one physical CPU core**; parallelism comes from running many VMs per machine (one per core, e.g. 48–96), each exploring different branches. Inter-core nondeterminism is eliminated by isolation, not synchronization. | observed | https://antithesis.com/blog/deterministic_hypervisor/ | 2024-03-20 | — |
| **Virtual clock**: all time sources in the guest (TSC, HPET, etc.) return hypervisor-computed virtual time; time is a function of deterministic guest state/history. Intel PMC instructions-retired was found not-quite-deterministic (~1 in 1e12 instructions miscounted) and PMC threshold interrupts have variable delivery latency; they invented workarounds (details withheld). | observed (the problem), reported (the fix) | same | 2024-03-20 | Independent replication of PMC determinism claims; Antithesis publishing the workaround. |
| Guest I/O is via a **custom VMCALL-based hypercall**: guest emits logs/data and ingests commands/RNG seeds; each ingestion point is a potential **branch point in an input tree**. Interrupt injection added later to push inputs preemptively. | observed | same | 2024-03-20 | — |
| **Fast whole-VM snapshotting** exists and means replay never restarts from the beginning; enables rewind/time-travel and cheap branching. | observed | https://antithesis.com/blog/multiverse_debugging/ | 2024-09-10 | — |
| Guest OS: "mostly-stock 6.1 kernel with io_uring support"; simulated Intel x86-64 CPU with Skylake-era extensions; **x86-64 only**, no alternative architectures, **no nested hardware virtualization**. 10 GB RAM per machine default (increasable on request). | observed | https://antithesis.com/docs/configuration/the_antithesis_environment/ | undated (accessed 2026-07-21) | Docs update. |
| Controlled/fault-injected surfaces: **network** (latency, partitions, clogs/drops, restore; overlapping faults, most-aggressive-wins), **node** (hang, kill/stop, throttle; non-overlapping per target), **clock** (forward/backward jumps), **CPU** (speed modulation/"strobe", per-container CPU shares), **scheduler** (thread pausing/starvation — requires coverage instrumentation), plus **custom faults** (e.g. Tigris' regional faults) via forward-deployed engineers. | observed | https://antithesis.com/docs/concepts/fault_injection/ ; environment page | accessed 2026-07-21 | — |
| Entropy: guest /dev/random and /dev/urandom are replaced with devices whose entropy is platform-supplied (seeded, reproducible). SDKs also provide platform-guided randomness. | observed | environment page | — | — |
| **Fast-forward**: the simulation skips idle periods ("race-to-sleep" rewarded; busy-waiting on RDTSC/RDRAND is penalized). Time compression demonstrated (10 simulated minutes in seconds). | observed | environment page; https://antithesis.com/blog/multiverse_debugging/ | 2024-09-10 | — |
| Inside the guest: single linear history. Outside: the platform sees the full **branching tree of execution paths** and chooses where to explore next. | observed | hypervisor post | 2024-03-20 | — |

UNKNOWN / deliberately withheld:
- The snapshot/restore mechanism details, exploration-state bookkeeping, and "all the functionality that allows efficient state exploration and time-travel debugging" — explicitly omitted from the hypervisor post "for the sake of brevity and competitive edge". **UNKNOWN (proprietary).**
- The exact determinism fixes for PMC/interrupt issues. **UNKNOWN.**
- Storage-fault semantics at block level (bit rot, torn writes, fsync lies): the marketing page lists "storage faults" but the docs fault-type table lists only Network/Node/Clock/Other(thread pausing, CPU modulation, custom). Whether Antithesis injects fine-grained disk faults (à la FoundationDB's simulated disk corruption) is **UNKNOWN from public docs** as of access date. Falsifier: fault_types docs page enumerating storage faults.
- ARM/macOS support: absent (x86-64 Linux only) — observed.
- An independent open-source reimplementation exists: **dhyve**, "an open-source deterministic hypervisor built on bhyve and inspired by Antithesis", is name-checked in their own product FAQ. observed, https://antithesis.com/product/ (accessed 2026-07-21). This corroborates that the bhyve-fork approach is externally reproducible at some fidelity.

## 3. Workload generation & state-space search

| Claim | Modality | Source | Date | Falsifier |
|---|---|---|---|---|
| Exploration is split into **tactics** (input generation — e.g. random bytes, random "deltas"/button-state changes, correlated input sequences called "rollouts") and **strategies** (state evaluation — deciding which states deserve further exploration). | observed | https://antithesis.com/blog/2025/gradius/ | 2025-02-21 | — |
| The guidance component uses **RL** ("it uses RL, but you can tell your boss it's AI") to seek out new system states continuously. | observed | https://antithesis.com/docs/introduction/how_antithesis_works/ | accessed 2026-07-21 | Antithesis retracting; algorithm details unpublished → internals UNKNOWN. |
| **Coverage guidance**: language-specific instrumentation (LLVM sanitizer-coverage-style for C/C++/Rust/"other LLVM", plus Go/Java/JS/.NET instrumentors) sends continuous feedback of basic-block coverage; used to "find bugs faster", enables thread-pausing fault injection, and powers line-association in bug reports. | observed | https://antithesis.com/docs/reference/instrumentation/coverage_instrumentation/ | accessed 2026-07-21 | — |
| **Assertions double as exploration guidance** ("assertions are clues"): assertion evaluations hint which states to explore. `sometimes(...)` assertions make rare states more likely to be revisited. | observed | https://antithesis.com/docs/concepts/properties_assertions/assertions/ ; customer FAQ | accessed 2026-07-21 | — |
| "Tree fuzzing": the whole platform gives an arbitrary program **save slots reloadable at arbitrary points**, turning a one-shot search into incremental-progress search. Simple strategies suffice (FabiusStrategy: "maximize depth without dying" beat Gradius with 3 memory-byte predicates). | observed | https://antithesis.com/blog/2025/gradius/ | 2025-02-21 | — |
| Workloads are **test templates**: directories of executable **test commands** with 7 prefixes — `first_`, `parallel_driver_`, `serial_driver_`, `singleton_driver_`, `anytime_`, `eventually_`, `finally_`. Antithesis owns all selection/scheduling/parallelism of commands; finer-grained commands → better steering. `eventually_`/`finally_` halt fault injection and other commands for end-of-timeline validation. | observed | https://antithesis.com/docs/product/test_templates/test_composer_reference/ | accessed 2026-07-21 | — |
| Fault injection is autonomous and interleaved with workload; workloads can request quiet periods via `ANTITHESIS_STOP_FAULTS`; `setup_complete` lifecycle signal gates when faults start. | observed | https://antithesis.com/docs/concepts/fault_injection/ ; FAQ | — | — |
| Exploration-scale evidence: Tigris report — 20,373,041 total states explored, 211,006 executions, ~27.5k states/run average over 261 workload runs (July 2025–March 2026). "States" metric definition is Antithesis-internal. | reported (vendor-published customer report) | https://antithesis.com/blog/2026/tigris_report/ | 2026-04-21 | Definition of "state" disclosed and numbers independently recomputable. |
| Swarm-testing analog: docs recommend *many small granular commands* and properties catalogs; no explicit "swarm testing" (in the Groce sense) claim found. Exploration is guided, not blind-random config sampling. **inferred** — their swarm analog is the multiverse of VM branches per core. | inferred | test-template docs; FAQ | — | — |

Bug-finding claims (marketing-level, modality: reported):
- Homepage: "75+ severe bugs found that all other testing missed", "40x faster change verification", "10x faster time-to-release". Reported, unaudited. Falsifier: published methodology/counter-audit.
- NATS/Synadia customer story: first Antithesis experiment found a one-in-a-million Raft data-loss path (restart-during-recovery → state wipe → illegitimate leader → committed-data overwrite) that their own Jepsen-style chaos testing had masked for years. Published on antithesis.com as a customer story, authored by Synadia engineer (Marco Primi). 2025-03-18.
- Tigris report: 3 distinct cache-coherence bugs found (delete-then-read race on first Antithesis run), all in code paths their CI exercised but never under faults; led to eager-eviction + tombstone-barrier architecture changes; "no cache coherence bug since". 2026-04-21.

## 4. Assertion / invariant interfaces

| Claim | Modality | Source | Date |
|---|---|---|---|
| SDKs: **Go, Java, C, C++, JavaScript, Python, Rust, .NET**; plus a **low-level fallback API** (JSONL protocol) for other languages. | observed | https://antithesis.com/docs/reference/sdk/ | accessed 2026-07-21 |
| Assertion taxonomy: `always(...)`, `alwaysOrUnreachable(...)`, `reachable()`, `unreachable()`, `sometimes(...)`. Every assertion requires a human-readable `message`; **properties = unique messages**, aggregated across all assertions with that message; the triage report lists properties pass/fail. | observed | https://antithesis.com/docs/concepts/properties_assertions/assertions/ | — |
| Fallback schema: JSONL messages `antithesis_assert` with fields id/message/condition/display_type/hit/must_hit/assert_type∈{always,sometimes,reachability}/location/details; plus a startup **assertion catalog** message so never-hit assertions can be failed. | observed | https://antithesis.com/docs/reference/sdk/fallback/schema/ | — |
| SDK runtime behavior: outside Antithesis, calls become no-ops/logs/stdlib shims (safe to run in production, args still evaluated); can be compiled out entirely. Failed assertions **do not abort** the process (failures can escalate into more severe bugs). | observed | SDK docs; assertions doc | — |
| SDK categories: define test properties; generate randomness; manage test lifecycle (`setup_complete`); plus assertion cataloging and coverage instrumentation. | observed | SDK docs | — |
| Runtime integration: stub `libvoidstar.so` replaced with real implementation injected via `/etc/ld.so.preload` inside the environment (no LD_PRELOAD manipulation of customer containers). | observed | environment page | — |
| Property catalogs (KV systems, blockchains) and reliability-property guides published as onboarding resources; agent skills auto-generate property catalogs. | observed | https://antithesis.com/docs/resources/kv_property_catalog/ (listed); agent_skills blog | 2026-03-25 |

## 5. Failure artifacts, replay, minimization, debugging

| Claim | Modality | Source | Date | Falsifier |
|---|---|---|---|---|
| **Triage report** per run, emailed: sections Findings / Environment / Utilization / Properties; fault-injection events interleaved with app logs and assertion outcomes. Slack/Discord, issue-tracker (e.g. Linear), CI (GitHub Action `antithesis-trigger-action`), REST API, and webhook integrations. | observed | https://antithesis.com/docs/product/reports/ ; fault_injection docs; GitHub org | accessed 2026-07-21 | — |
| Perfect reproduction of every bug (deterministic replay from seed/snapshot) — the core guarantee. | observed | multiple; hypervisor post | 2024-03-20 | A customer-published case of non-reproduced Antithesis finding. |
| **Multiverse debugging**: from any log line/example, rewind to any moment of the bug timeline; get a bash terminal / run arbitrary scripts in any container at any simulated time (past or future); extract files/core dumps; attach gdb and retroactively set breakpoints; retroactively capture packets; enable profiling retroactively; **change the past** (e.g. disable fault injection on a subsystem) and see if the bug persists. Session ready in 10–30 min; SSO required. Advanced mode supports counterfactuals and external debuggers; browser-based reactive **Antithesis Notebook** UI (UI = f(code) over a deterministic world). | observed | https://antithesis.com/blog/multiverse_debugging/ ; https://antithesis.com/docs/product/debugging/simple_mvd/ | 2024-09-10; docs accessed 2026-07-21 | — |
| Scripted debugging workflows can be converted into **custom artifacts auto-attached** to future triage reports when a matching bug class is found (contact-support gated). | observed | simple_mvd docs | — | — |
| **Causality analysis** ("When did the bug start?"): take one buggy timeline, rewind to various moments, re-explore forward many times with different fuzzer behavior (faults, command choices, workload randomness, scheduling), compute **bug-probability-over-time graph**; sharp upward jumps localize causative events. Generated on demand per bug example; streams results. | observed | https://antithesis.com/docs/product/debugging/causality_analysis/ ; https://antithesis.com/blog/2026/causality_analysis/ | docs undated; blog 2026-05-11 | — |
| Automated **root cause analysis** claimed on homepage/product page ("automatically root causes every bug"). Causality analysis + line-association from coverage are the plausibly-referring features; full automation level **inferred partial**. | reported | https://antithesis.com/product/ | accessed 2026-07-21 | — |
| **Minimization**: no explicit "test-case minimization/shrinking" feature documented as of access date. The replay model (whole-timeline reproduction + probability analysis) substitutes for classical shrinking. **UNKNOWN** whether internal trace minimization exists. Falsifier: docs for a minimize/shrink feature. | inferred | absence across docs sitemap | — | — |
| Logs Explorer: unified, perfectly ordered cross-container event stream; search across thousands of execution histories. | observed | https://antithesis.com/docs/product/logs_explorer/ (listed); product page | — | — |

## 6. Deployment model, CI, pricing, tenancy/security

| Claim | Modality | Source | Date | Falsifier |
|---|---|---|---|---|
| Packaging: customer pushes **Linux container images** (x86-64) to a **per-customer Antithesis container registry**; orchestration via **Docker Compose** (config baked into a config image) or **Kubernetes** (raw manifests via `helm template`/kustomize, run on simulated single-node **k3s**). K8s support launched 2025-11-05. | observed | setup guide; https://antithesis.com/blog/2025/kubernetes_launch/ | 2025-11-05 | — |
| Execution location: Antithesis cloud (tenant data on ISO 27001-certified AWS/GCP). **Customer-VPC option**: ephemeral execution environments + registry run in the customer's AWS VPC; control plane stays in Antithesis's VPC, single-tenant compute+DB per customer. **No fully on-prem option.** | observed | https://antithesis.com/legal/security/ ; https://antithesis.com/product/ FAQ | security doc undated; product page accessed 2026-07-21 | — |
| Tenancy: per-Tenant **parallel copy of entire infrastructure**; minimal TCB (tenant-boundary components + infra-as-code declarations + auth'd APIs); no PII handled (contract-enforced); open-source bug onward-disclosure clause (30 days default). | observed | legal/security | — | — |
| CI: `basic_test` webhook trigger; official GitHub Action (`antithesishq/antithesis-trigger-action`, MIT); REST API; typical cadence nightly 6–8 h runs + ~30–60 min per-commit runs; "test hours" ≈ duration × cores provisioned. Slack/Discord/issue-tracker integrations. | observed | FAQ; GitHub org | accessed 2026-07-21 | — |
| Pricing: **no public pricing; no free tier** ("Antithesis is compute-intensive… infrastructure has to get paid for somehow"). Sold via sales/POC; **AWS Marketplace Private Offers** supported. Forward-deployed engineers attached to customers. 200 MB searchable log+image storage per simulation-hour, 6-month retention, included. | observed | product page FAQ; customer FAQ | accessed 2026-07-21 | Publication of a price list. |
| Local pre-flight: test commands validatable locally with `docker-compose up` offline + `docker exec`; "testing locally" docs. | observed | customer FAQ | — | — |
| Agent integration: open-source **antithesis-skills** repo (Apache-2.0) — `antithesis-research`, `antithesis-setup`, `antithesis-workload` skills that one-shot onboarding (architecture analysis → property catalog → topology → dockerfile/compose → test commands); demonstrated by bootstrapping rqlite. `snouty` CLI (Rust, Apache-2.0). | observed | https://antithesis.com/blog/2026/agent_skills/ ; GitHub org | 2026-03-25 | — |
| Onboarding time claim: "as little as a day" with agent skills; 2–3 weeks typical historically (Gradius post). | reported | product page; gradius post | — | — |

## 7. Customers / market evidence (market evidence, NOT technical proof)

Named on official channels as of 2026-07-21 (modality: reported, mostly vendor-published):
- **Jane Street** — customer, then led $105M Series A; tested a "critical message bus"; Ron Minsky video testimonial. (about, series_a, homepage)
- **MongoDB** — multi-year, core server + WiredTiger (launch post; two customer stories 2024-02-13, 2024-04-22).
- **Ethereum Foundation** — pre-Merge testing engagement (customer story 2024-02-13).
- **Palantir** — named in launch post.
- **Synadia/NATS** — Raft data-loss bug story (2025-03-18).
- **Formance** — "six months of production testing in an hour" (customer story 2025-05-01; homepage quote).
- **Mysten Labs (Sui)** — Mark Logan interview (2025-01-16); "3 weeks of debugging → 1 day".
- **Tigris Data** — full published report (2026-04-21); 3 cache-coherence bugs.
- **ParadeDB** — CEO quote: dropped Antithesis logs into Claude for end-to-end debug+fix (homepage).
- **Readyset** — caching bug customer story (2026-02-19).
- **PingThings** — CEO testimonial (homepage; 2025 story).
- **etcd/Kubernetes** — maintainer Marek Siarkowicz quote, "6 hrs in Antithesis > 100 engineers" (homepage; open-source program).
- Open-source giveaway program: $186k donated 2024 (blog 2024-09-16), 2025 pledge (blog 2025-10-16); free/discounted testing for OSS projects (etcd, Turso mentioned).
- Conference/community: Bug Bash conference (2025 US talks; **Bug Bash Europe, Sept 30 2026, Copenhagen**), 19-episode podcast, active docs/release notes. Discord support channel.
- Market context: DST adjacent ecosystem named in their own resources — TigerBeetle, Warpstream, Resonate, RisingWave as systems benefiting from DST (not necessarily customers).

## 8. Moat hypotheses (with falsifiers and contrary evidence)

- **H1: The Determinator (bare-metal deterministic hypervisor) is the moat.** Instruction-level determinism for *unmodified arbitrary x86 binaries* — no source rewrite, no pluggable-nondeterminism discipline (contrast FoundationDB-style in-process simulation, which vibe-halt follows). Years of CPU-errata-class debugging (PMC miscounts, APIC interrupt jitter, 50 GiB/run determinism forensics) encode tacit knowledge that's slow to re-derive.
  - Falsifier: an open-source rig reproducing whole-VM determinism with replay on commodity tooling. Contrary evidence: **dhyve** exists (bhyve-based, Antithesis-inspired, name-checked by Antithesis themselves); rr demonstrates process-level record/replay determinism (Antithesis podcast even hosted the rr story). The substrate is reproducible in principle; the *engineering depth* is what's scarce.
- **H2: The exploration engine (guided tree-fuzzing over branched VM snapshots) is the real moat, not determinism.** Determinism is table stakes; the RL-guided strategy/tactics split + coverage feedback + assertion-as-guidance is what finds bugs "in minutes".
  - Falsifier: blind random search over deterministic seeds matching Antithesis bug yield on a benchmark SUT. Contrary evidence: Gradius post shows a *very simple* strategy (maximize depth, avoid 3 byte-predicates) suffices for a nontrivial target — suggesting the snapshot/branch machinery matters more than strategy sophistication; Jepsen-style random fault injection also finds deep bugs without guidance, just slower.
- **H3: Debugging artifacts (multiverse debugger, causality analysis) are the stickiness moat.** Finding bugs is commoditizable; 10–30-minute rewind-to-any-moment sessions with retroactive breakpoints/packet captures are not.
  - Falsifier: users valuing only pass/fail gating (CI usage patterns in FAQ suggest many customers mostly run nightly/CI gates). Contrary evidence: Tigris report centers bug *finding*, not debugging sessions.
- **H4: Forward-deployed onboarding + property catalogs + agent skills are the go-to-market moat.** Tacit distributed-systems knowledge ("what properties should my system have?") is the real adoption blocker; skills automate it.
  - Falsifier: self-serve onboarding succeeding at scale (agent_skills post admits pre-skills onboarding was "brutal… crawling over broken glass" — the moat is also a cost).
- **H5 (contrary to all): determinism-as-a-service is a feature, not a company.** AWS/simulation-first internal tools (Al Vermeulen cited in their own DST explainer), FoundationDB's open simulator, TigerBeetle's VOPR, and dhyve show the technique diffuses. Antithesis's own explainer acknowledges simultaneous invention at FoundationDB and AWS ~2010.
  - Falsifier: no credible independent DST rig gaining production adoption over the next 2 years. (Relevant to vibe-halt: the technique is replicable; the platform depth is not, quickly.)

## 9. Public terminology (vibe-halt vocabulary overlap)

Antithesis-coined/popularized terms (observed): "**multiverse**" / "**multiverse debugging**" (2024-09-10 post; docs `simple_mvd`, `advanced_multiverse_debugging/*`), "**the Determinator**" (hypervisor), "**tree fuzzing**", "**tactics and strategies**", "**rollouts**", "**test template / test command**" prefixes, "**sometimes assertions**", "**triage report**", "**causality analysis**", "**findings**", "**test hours**", "**unknown unknowns**", "release with confidence / merge fearlessly".

vibe-halt overlap (observed in repo, 2026-07-21): crate `vh-multiverse` ("FoundationDB/Antithesis lineage: the runtime — not the workload — owns…", `crates/vh-multiverse/src/runtime.rs:4`), `vh-gremlin` (fault injection), `vh-shrink` (minimization — note: Antithesis has *no documented shrinker*), `vh-verify` divergence tests, `docs/specs/DETERMINISM_TIERS.md:39` explicitly scopes out "Antithesis-class whole-VM determinism", and `crates/vh-props/src/lib.rs:9` references "Antithesis-style check". The borrowing is acknowledged in-repo; the names "multiverse" and (conceptually) "gremlin/faults" map to Antithesis's "multiverse" and fault injector.

## 10. Top capabilities that matter for the vibe-halt comparison

1. Whole-system determinism for **unmodified binaries** (hypervisor, not cooperative simulation) — vibe-halt's explicit non-goal (Tier 3 scoped out).
2. **Snapshot/branch tree** ("multiverse") with rewind-to-any-moment; never replays from t=0.
3. Guided exploration: coverage feedback + RL guidance + assertions-as-clues; tactics/strategies split.
4. Fault model breadth: network (partition/clog/latency), node kill/hang/throttle, clock jumps, CPU strobe, thread pausing, custom/regional faults; pause/quiet-period API.
5. Property interface: always/sometimes/reachability assertions aggregated to named properties; JSONL fallback for any language; production-safe no-op SDKs.
6. Test-template command algebra (first/parallel/serial/singleton/anytime/eventually/finally) — a reusable scheduling contract vibe-halt could adopt wholesale.
7. Causality analysis: bug-probability-over-time via rewind+re-explore — a concrete, implementable algorithm.
8. Artifact pipeline: triage report (properties pass/fail + findings), logs explorer, custom artifacts, CI/webhook/API, agent skills.
9. Deployment pragmatics: container-in (compose/k3s), hermetic, x86-64 only, 10 GB default, fast-forward on idle — simple constraints to copy.
10. Commercial proof of demand: $105M Series A led by a *customer*; named database/infra customers finding multi-factorial bugs (NATS Raft, Tigris cache coherence) — validates the market for small DST rigs.

## 11. Key unknowns (explicit)

- Exploration algorithm internals (RL architecture, state representation, what a "state" is in the 20.3M-states metric). UNKNOWN.
- Snapshot/storage internals of the Determinator; deterministic disk-fault semantics. UNKNOWN (partially withheld deliberately).
- Implementation language of the hypervisor (Haskell inferred from "pure functional language" quote). INFERRED, unconfirmed.
- Test-case minimization/shrinking: no public evidence either way. UNKNOWN.
- Pricing, revenue, headcount, valuation. UNKNOWN (no public pricing; $152M disclosed funding only).
- Non-x86 (ARM) roadmap; nested virtualization support. UNKNOWN/absent.
- Independent (non-vendor-published) bug-yield benchmarks vs Jepsen/rr/foundationdb-simulation. UNKNOWN — no third-party benchmark found.
- Storage fault injection depth (bit rot, torn write, fsync reordering). UNKNOWN from public docs.

---

## SOURCE_MANIFEST

source_id,title,url,source_type,published_at,accessed_at,relevance,notes
S1,Antithesis homepage,https://antithesis.com/,official_site,undated,2026-07-21,positioning+claims,"75+ severe bugs; 40x/10x claims; customer quotes (Jane Street, Formance, Tigris, ParadeDB, etcd, PingThings, Mysten)"
S2,Product page,https://antithesis.com/product/,official_site,undated,2026-07-21,deployment+FAQ,"VPC option; AWS Marketplace; no free tier; dhyve mention; single-core-per-run; FAQ on languages/multicast"
S3,About,https://antithesis.com/company/about/,official_site,undated,2026-07-21,lineage+funding,"FDB 2009→Apple 2015→Antithesis Jan 2018; $47M seed Feb 2024; $105M Series A Dec 2025 Jane Street; Haskell hint"
S4,Is something bugging you?,https://antithesis.com/blog/is_something_bugging_you/,official_blog,2024-02-13,lineage+vision,"launch post; FDB simulation story; MongoDB/Ethereum/Palantir named; stealth exit"
S5,So you think you want to write a deterministic hypervisor?,https://antithesis.com/blog/deterministic_hypervisor/,official_blog,2024-03-20,substrate,"bhyve fork; VMX; 1 core/VM; virtual clock; PMC nondeterminism; VMCALL I/O; input tree; snapshot teased"
S6,Debugging in the Multiverse,https://antithesis.com/blog/multiverse_debugging/,official_blog,2024-09-10,debugging,"multiverse debugging coined; rewind/bash/gdb/pcap retroactively; fast-forward; notebook UI; snapshotting confirmed"
S7,When your customer leads your Series A,https://antithesis.com/blog/2025/series_a/,official_blog,2025-12-03,funding,"$105M Series A Dec 2025 led by Jane Street; custom datastore footnote"
S8,Depth is all you need: how Antithesis crushes Gradius,https://antithesis.com/blog/2025/gradius/,official_blog,2025-02-21,exploration,"tactics/strategies; tree fuzzing; FabiusStrategy; rollouts; 2-3 week onboarding claim"
S9,Hunting for one-in-a-million bugs in NATS,https://antithesis.com/blog/2025/synadia/,customer_story,2025-03-18,bug-finding evidence,"Synadia-authored; Raft state-wipe data-loss path found on first experiment"
S10,Antithesis report: Tigris Data,https://antithesis.com/blog/2026/tigris_report/,customer_report,2026-04-21,bug-finding evidence+scale,"3 cache-coherence bugs; 20.37M states/211k executions Jul 2025–Mar 2026; regional custom faults"
S11,Antithesis launches Kubernetes support,https://antithesis.com/blog/2025/kubernetes_launch/,official_blog,2025-11-05,deployment,"k3s single-node simulated cluster; raw manifests via helm template"
S12,Antithesis skills for agents,https://antithesis.com/blog/2026/agent_skills/,official_blog,2026-03-25,agent integration,"antithesis-research/setup/workload skills; rqlite demo; onboarding-pain admission"
S13,At the Mountains of Madness,https://antithesis.com/blog/madness/,official_blog,2024-07-10,engineering culture,"NixOS everywhere; open-sourced madness loader; ELF/ld-linux depth"
S14,Welcome to Antithesis (docs),https://antithesis.com/docs/introduction/welcome/,official_docs,undated,2026-07-21,mental model,"staging-environment framing; container registry on request; skills note"
S15,How Antithesis works (docs),https://antithesis.com/docs/introduction/how_antithesis_works/,official_docs,undated,2026-07-21,exploration,"RL guidance quote; multiverse framing; properties"
S16,The Antithesis environment (docs),https://antithesis.com/docs/configuration/the_antithesis_environment/,official_docs,undated,2026-07-21,substrate config,"6.1 kernel; Skylake x86-64 only; 10GB RAM; no nested virt; /dev/random replaced; ANTITHESIS_OUTPUT_DIR; libvoidstar ld.so.preload; fast-forward"
S17,Fault injection overview (docs),https://antithesis.com/docs/concepts/fault_injection/,official_docs,undated,2026-07-21,fault model,"network/node/clock/thread-pause/CPU/custom faults; overlap semantics; ANTITHESIS_STOP_FAULTS"
S18,SDKs (docs),https://antithesis.com/docs/reference/sdk/,official_docs,undated,2026-07-21,interfaces,"Go/Java/C/C++/JS/Python/Rust/.NET + fallback; no-op in prod; cataloging+instrumentation"
S19,Assertions in Antithesis (docs),https://antithesis.com/docs/concepts/properties_assertions/assertions/,official_docs,undated,2026-07-21,properties,"always/alwaysOrUnreachable/reachable/unreachable/sometimes; message=property; assertions are clues; non-fatal failures"
S20,Antithesis Assertion Schema (docs),https://antithesis.com/docs/reference/sdk/fallback/schema/,official_docs,undated,2026-07-21,fallback API,"JSONL antithesis_assert schema; assert_type enum; must_hit semantics"
S21,Test commands reference (docs),https://antithesis.com/docs/product/test_templates/test_composer_reference/,official_docs,undated,2026-07-21,workload model,"7 command prefixes with scheduling/fault semantics table"
S22,Test templates (docs),https://antithesis.com/docs/product/test_templates/,official_docs,undated,2026-07-21,workload model,"template=client+checker; granularity guidance"
S23,Coverage instrumentation (docs),https://antithesis.com/docs/reference/instrumentation/coverage_instrumentation/,official_docs,undated,2026-07-21,guidance,"basic-block callbacks; enables thread pausing; symbolization /symbols dir; line-association in reports"
S24,Simple Multiverse debugging (docs),https://antithesis.com/docs/product/debugging/simple_mvd/,official_docs,undated,2026-07-21,debugging workflow,"10-30min session setup; SSO; bash at any moment; file extraction; auto-artifacts; advanced mode"
S25,Causality analysis (docs),https://antithesis.com/docs/product/debugging/causality_analysis/,official_docs,undated,2026-07-21,root cause,"rewind+re-explore; bug-probability-over-time graph; coin-flip example"
S26,When did the bug start? (blog),https://antithesis.com/blog/2026/causality_analysis/,official_blog,2026-05-11,root cause,"blog announcement of causality analysis"
S27,Triage report (docs),https://antithesis.com/docs/product/reports/,official_docs,undated,2026-07-21,artifacts,"Findings/Environment/Utilization/Properties sections; emailed per run"
S28,Product FAQs (docs),https://antithesis.com/docs/faq/customer_faq/,official_docs,undated,2026-07-21,ops+pricing signals,"test hours=duration×cores; nightly 6-8h + 30-60min commit runs; 200MB/sim-hour logs 6mo; local validation; declarative-vs-imperative framing"
S29,Setup guide (docs),https://antithesis.com/docs/getting_started/setup_guide/,official_docs,undated,2026-07-21,packaging,"Docker Compose or Kubernetes; upload to Antithesis registry"
S30,Information security policy,https://antithesis.com/legal/security/,official_legal,undated,2026-07-21,tenancy/security,"per-tenant parallel infra; minimal TCB; AWS/GCP ISO27001; no PII; 30-day OSS bug disclosure"
S31,Deterministic simulation testing explainer,https://antithesis.com/docs/resources/deterministic_simulation_testing/,official_docs,undated,2026-07-21,DST context,"FDB+AWS ~2010 simultaneous invention; Al Vermeulen; Strange Loop 2014 talk; strengths/limits"
S32,Antithesis Learn index,https://antithesis.com/learn/,official_site,undated,2026-07-21,publication map,"full blog/podcast/talk chronology incl. Bug Bash Europe Sept 30 2026 Copenhagen"
S33,Antithesis sitemap,https://antithesis.com/sitemap-0.xml,official_site,undated,2026-07-21,coverage,"exhaustive docs/blog URL inventory; no shrink/minimize docs found"
S34,github.com/antithesishq,https://github.com/antithesishq,official_github,undated,2026-07-21,OSS surface,"49 repos; antithesis-trigger-action (MIT); antithesis-skills (Apache-2.0); snouty CLI; SDKs Java/Rust; Bombadil 1.3k★ Rust UI PBT; madness; proptest fork"
S35,vibe-halt repository (local),/Users/dhyana/vibe-halt,local_repo,2026-07-21,2026-07-21,vocabulary overlap,"vh-multiverse crate cites FoundationDB/Antithesis lineage; DETERMINISM_TIERS scopes out Antithesis-class hypervisor; vh-shrink exists where Antithesis documents none"
