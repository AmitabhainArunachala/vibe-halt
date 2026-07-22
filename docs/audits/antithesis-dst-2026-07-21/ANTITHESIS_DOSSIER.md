# Antithesis Dossier

> Curated from `lanes/LANE_B_ANTITHESIS.md` (35 sources, primary-source-first; full claim/modality/falsifier tables there).
> Access date for all sources: 2026-07-21. Modality: **observed** = read on fetched primary page; **reported** = stated by Antithesis/customer, not independently verified; **inferred** = analyst synthesis. Proprietary internals are marked UNKNOWN, not reverse-engineered from marketing.

## 1. Identity, history, technical lineage, current status

- Founded **January 2018** by **Will Wilson** (built FoundationDB's deterministic simulation harness from 2013) and **Dave Scherer** (FoundationDB co-founder/chief architect). FoundationDB (2009) → Apple acquisition (March 2015) → Antithesis. The FDB simulator is the direct ancestor. (observed: about page, launch post 2024-02-13)
- ~5.5 years stealth; emerged **Feb 2024** with **$47M seed** (Amplify/Tamarack/First in). (observed)
- **$105M Series A, December 2025, led by Jane Street — an existing customer.** ~$152M disclosed total. (observed: about, Series A post 2025-12-03)
- First customer Sept 2019. Revenue, headcount, valuation: **UNKNOWN** (no public pricing).
- Reported internal tech (unverified): hypervisor "built with a pure functional language" (Haskell inferred, unconfirmed); custom branching-timeline datastore; NixOS everywhere.

## 2. Product and developer workflow

1. Customer pushes **Linux x86-64 container images** to a per-customer Antithesis registry; orchestration via **Docker Compose** (config image) or **Kubernetes** (raw manifests on simulated single-node k3s; launched 2025-11-05).
2. Workloads are **test templates**: directories of executable **test commands** with 7 prefixes — `first_`, `parallel_driver_`, `serial_driver_`, `singleton_driver_`, `anytime_`, `eventually_`, `finally_`. Antithesis owns selection/scheduling/parallelism; `eventually_`/`finally_` halt fault injection for end-of-timeline validation. *A clean, directly copyable command algebra.* (observed: test_composer docs)
3. Assertions via **SDKs (Go/Java/C/C++/JS/Python/Rust/.NET)** or a **JSONL fallback protocol** for any language; SDKs are no-ops outside the platform (production-safe); failed assertions do not abort. Assertion catalog declared at startup so never-hit assertions fail. (observed)
4. Typical cadence: nightly 6–8h runs + 30–60min per-commit runs; CI via official GitHub Action / REST API / webhooks; per-run **triage report** (Findings/Environment/Utilization/Properties). (observed: FAQ, reports docs)
5. Onboarding historically 2–3 weeks ("brutal"); open-source **antithesis-skills** agent skills (Apache-2.0, 2026-03-25) now claim one-shot onboarding ("as little as a day"). (observed/reported)

## 3. Publicly established architecture

Substrate: **"the Determinator"** — a custom deterministic **hypervisor forked from FreeBSD bhyve**, Intel VMX, **one VM pinned per physical core** (48–96 VMs/machine exploring different branches). The **entire SUT (containers + workload + checkers) runs inside one deterministic guest** — the unit of reproducibility is whole interconnected system state. (observed: hypervisor post 2024-03-20, environment docs)

- **Virtual clock**: all guest time sources hypervisor-computed. Intel PMC found not-quite-deterministic (~1/1e12 instructions miscounted); workarounds invented, details withheld. (observed problem / reported fix)
- **I/O**: custom VMCALL hypercall; each input ingestion is a **branch point in an "input tree"**. Guest: mostly-stock Linux 6.1, x86-64 Skylake only, 10GB RAM default, no nested virt; /dev/(u)random replaced with seeded devices.
- **Fast whole-VM snapshotting** — replay never restarts from t=0; enables rewind and cheap branching. **Fast-forward** through idle (race-to-sleep rewarded).
- Inside guest: single linear history. Outside: the platform sees the **branching tree** and chooses where to explore next.
- An open-source bhyve-based deterministic hypervisor, **dhyve**, exists and is name-checked in Antithesis's own FAQ — the substrate approach is externally reproducible at some fidelity. (observed)

## 4. Determinism, world model, exploration, invariants, replay, minimization

- **Fault model** (observed: fault_injection docs): network (latency, partitions, clogs/drops, overlapping most-aggressive-wins), node (hang/kill/throttle), clock jumps, CPU strobe, **thread pausing** (requires coverage instrumentation), custom/regional faults via forward-deployed engineers. `ANTITHESIS_STOP_FAULTS` quiet periods; `setup_complete` gates fault start. **Fine-grained storage faults (bit rot, torn writes, fsync lies): UNKNOWN from public docs.**
- **Exploration**: split into **tactics** (input generation: random bytes, deltas, correlated "rollouts") and **strategies** (state evaluation). Guidance uses **RL** + **language-specific basic-block coverage instrumentation** (LLVM-style for C/C++/Rust, plus Go/Java/JS/.NET) + **assertions-as-clues** (`sometimes(...)` states revisited more). Internals UNKNOWN. "Tree fuzzing": save slots reloadable at arbitrary points turn one-shot search into incremental search — and the Gradius post shows a *very simple* strategy (maximize depth, avoid 3 byte-predicates) suffices on a nontrivial target, suggesting snapshot/branch machinery matters more than strategy sophistication. (observed)
- **Scale evidence**: Tigris report — 20,373,041 states / 211,006 executions (Jul 2025–Mar 2026); "state" definition internal. (reported, vendor-published)
- **Invariants**: `always / alwaysOrUnreachable / reachable / unreachable / sometimes`; properties = unique messages aggregated across assertions; triage report lists properties pass/fail. (observed)
- **Replay**: perfect deterministic reproduction is the core guarantee. **Multiverse debugging** (term coined 2024-09-10): rewind to any moment, bash/gdb/pcap/file-extraction retroactively, **change the past** (disable a fault class) and re-run; 10–30min session setup. (observed)
- **Causality analysis** ("When did the bug start?", 2026-05-11): rewind to various moments, re-explore forward many times with perturbed fuzzer behavior, compute **bug-probability-over-time**; sharp jumps localize causative events. *A concrete, implementable algorithm.* (observed)
- **Minimization**: **no documented shrinking/minimization feature** — the whole-timeline replay + probability model substitutes for it. UNKNOWN whether internal trace minimization exists. (Notably: vibe-halt has `vh-shrink` where Antithesis documents none.)

## 5. Deployment, security, integrations, market evidence

- Antithesis cloud (ISO 27001 AWS/GCP); **customer-VPC option** (ephemeral execution + registry in customer AWS VPC, Antithesis control plane); **no on-prem**. Per-tenant parallel infra copy, minimal TCB, no PII (contract), 30-day OSS bug disclosure clause. (observed: legal/security)
- **No public pricing, no free tier**; AWS Marketplace Private Offers; "test hours" = duration × cores; forward-deployed engineers. (observed: FAQ)
- Named customers/channels (reported, mostly vendor-published): **Jane Street** (customer → Series A lead), **MongoDB**, **Ethereum Foundation**, **Palantir**, **Synadia/NATS**, **Formance**, **Mysten/Sui**, **Tigris**, **Readyset**, **PingThings**, **etcd/Kubernetes** (OSS program), **ParadeDB**. OSS giveaway program ($186k donated 2024). Bug Bash Europe Sept 30 2026 Copenhagen.
- Bug stories (customer-authored but vendor-hosted): NATS Raft one-in-a-million data-loss path found on first experiment (2025-03-18); Tigris 3 cache-coherence bugs (2026-04-21). Marketing numbers ("75+ severe bugs", "40x") unaudited.

## 6. Moat hypotheses (with falsifiers and contrary evidence)

- **H1 — the Determinator is the moat.** Instruction-level determinism for *unmodified* x86 binaries; years of CPU-errata-class tacit knowledge. *Contrary:* dhyve exists; rr demonstrates process-level record/replay; substrate reproducible in principle — the engineering depth is what's scarce.
- **H2 — the exploration engine is the real moat.** Determinism is table stakes; RL+coverage+assertion guidance finds bugs "in minutes". *Contrary:* Gradius shows trivial strategies suffice given snapshot/branch machinery; Jepsen-style random faults also find deep bugs, slower.
- **H3 — debugging artifacts are the stickiness moat.** Multiverse debugger + causality analysis are hard to commoditize. *Contrary:* much customer usage is nightly CI gating (FAQ); Tigris report centers finding, not debugging.
- **H4 — forward-deployed onboarding + property catalogs + agent skills are the GTM moat.** "What properties should my system have?" is the real adoption blocker. *Contrary:* the moat is also a cost (pre-skills onboarding was "crawling over broken glass").
- **H5 (contrary to all) — determinism-as-a-service is a feature, not a company.** FDB's open simulator, TigerBeetle VOPR, dhyve, simultaneous invention at AWS ~2010. *Falsifier:* no credible independent DST rig gaining production adoption within 2 years. *For vibe-halt: the technique is replicable; the platform depth is not, quickly.*

## 7. Proprietary boundary and unresolved unknowns

- Exploration internals (RL architecture, state representation, "state" metric). UNKNOWN.
- Determinator snapshot/storage internals; PMC/interrupt determinism fixes. UNKNOWN (deliberately withheld "for competitive edge").
- Hypervisor implementation language (Haskell inferred from employee quote). INFERRED.
- Test-case minimization: no public evidence either way. UNKNOWN.
- Pricing/revenue/headcount/valuation; ARM roadmap; nested virt. UNKNOWN/absent.
- Independent (non-vendor) bug-yield benchmarks vs Jepsen/rr/FDB-sim. UNKNOWN — none found.
- Fine-grained storage-fault semantics. UNKNOWN from public docs.

## 8. Vocabulary overlap with vibe-halt

Antithesis-coined: "multiverse (debugging)", "the Determinator", "tree fuzzing", "tactics/strategies", "rollouts", test-command prefixes, "sometimes assertions", "triage report", "causality analysis", "test hours". vibe-halt's borrowing is acknowledged in-repo (`vh-multiverse` runtime.rs:4 cites FoundationDB/Antithesis lineage; `DETERMINISM_TIERS.md:39` scopes out Antithesis-class whole-VM determinism; `vh-props/lib.rs:9` references "Antithesis-style check"). vibe-halt uniquely has a **shrinker** and a **fault-lifecycle evidence ledger** (Offered→…→Recovered) with no public Antithesis analog.
