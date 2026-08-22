# Path to Antithesis-grade real-target testing

Research resolution for [Define the path to Antithesis-grade real-target testing](https://github.com/AmitabhainArunachala/vibe-halt/issues/107).

## Verdict

“Antithesis-grade” must be an **external benchmark vector**, not a badge or architecture analogy. Vibe Halt reaches that grade for a declared target profile only when it can:

1. run the real multi-process target and its declared dependencies;
2. control every claim-relevant source of nondeterminism or type it `Open`;
3. generate workloads and environmental faults across meaningful choice points;
4. use measured feedback to explore more discriminating states than frozen null strategies;
5. replay and minimize a failure independently from a signed, exact world identity;
6. model failure, recovery, safety, and liveness across a whole declared topology;
7. sustain enough useful executions per cost unit to find real faults; and
8. beat preregistered baselines on independently confirmed severity-weighted yield.

This is a staged capability ladder. It is not a prescription to clone Antithesis's proprietary implementation, and it never makes finite testing proof. The public Antithesis material establishes capabilities and useful product shapes; its internal guidance algorithms, infrastructure costs, and independent real-fault yield remain unknown.

The near-term order is **reach → exact control → replay/minimization → discriminating search → economic yield**. Search before reach optimizes fixtures. Hypervisor-scale investment before an exact real-target campaign earns its keep only if the narrower native-interposition profile passes its revival falsifier.

## Current accepted-main position

At accepted `main` `d19ba9e1198c0ab7ef4bb1bdf69a299d056f9754`, Vibe Halt has a strong evidence kernel but has not crossed the real-target capability threshold:

- Product Lock v1 already makes Antithesis-adjacent verification the conditional north star and requires exact revision binding, a target map, coverage plan, evidence, and `HALT | PROCEED | UNKNOWN` ([Product Lock lines 11–44](https://github.com/AmitabhainArunachala/vibe-halt/blob/d19ba9e1198c0ab7ef4bb1bdf69a299d056f9754/docs/specs/PRODUCT_LOCK_V1.md#L11-L44)).
- Current subprocess evidence is D2: all 29 capability channels remain `Open`, so a successful cooperative run cannot construct D1 ([sandbox capability envelope](https://github.com/AmitabhainArunachala/vibe-halt/blob/d19ba9e1198c0ab7ef4bb1bdf69a299d056f9754/docs/specs/SANDBOX_CAPABILITY_ENVELOPE_V1.md#L16-L33)).
- Current receipts are content-addressed self-consistency, not signed/authenticated provenance ([`receipts_v2.rs` lines 20–31](https://github.com/AmitabhainArunachala/vibe-halt/blob/d19ba9e1198c0ab7ef4bb1bdf69a299d056f9754/crates/vh-cli/src/receipts_v2.rs#L20-L31)).
- Current fault-plan shrinking is real but bounded to capture-enabled registered workloads; there is no admitted real Dharma target result ([reality-bridge starting ledger](https://github.com/AmitabhainArunachala/vibe-halt/blob/d19ba9e1198c0ab7ef4bb1bdf69a299d056f9754/docs/prompts/VIBE_HALT_REALITY_BRIDGE_LONG_RUNNING_GOAL_2026-07-29.md#L113-L147)).
- Guided exploration has two published nulls: the first swarm palette passed 0/5 pinned classes; event-priority scheduling beat uniform on 0/32 seeds and lost 8, but that schedule comparison had a floor effect and is explicitly narrow ([evidence ledger entries](https://github.com/AmitabhainArunachala/vibe-halt/blob/d19ba9e1198c0ab7ef4bb1bdf69a299d056f9754/docs/audits/antithesis-dst-2026-07-21/EVIDENCE_LEDGER.jsonl#L29-L30)).
- The product benchmark is already correct: unique independently confirmed severity-weighted faults per spend and wall-clock, compared under frozen equal budgets ([Product Lock lines 87–108](https://github.com/AmitabhainArunachala/vibe-halt/blob/d19ba9e1198c0ab7ef4bb1bdf69a299d056f9754/docs/specs/PRODUCT_LOCK_V1.md#L87-L108)).

Historical audit claims such as “no reach” or “no depth” were accurate at their pinned July commit but are not copied forward as current facts. Accepted main has since added D2 subprocess observation, replay identities, corpus wiring, shrink integration, and typed admission. The open frontier is whether those assets can govern a real target with a closed-enough execution envelope and demonstrate yield.

## What the external precedents actually establish

### Antithesis: public capability boundary

[Antithesis's public architecture](https://antithesis.com/docs/introduction/how_antithesis_works/) describes containerized software and dependencies inside a deterministic environment, branching timelines across input and fault choices, and an intelligent guidance component seeking new states. Its [coverage instrumentation documentation](https://antithesis.com/docs/instrumentation/coverage_instrumentation/) says basic-block callbacks continuously feed the search, enable thread pausing, and enrich causal reports. Its [fault catalogue](https://antithesis.com/docs/environment/fault_injection/) includes network, node, clock, thread, and CPU faults, including overlapping faults. Its [test-command algebra](https://antithesis.com/docs/product/test_templates/test_composer_reference/) separates setup, concurrent and serial drivers, continuous checks, and quiescent recovery checks.

Those sources support a capability comparison. They do not publish an independently reproducible throughput benchmark, exact guidance algorithm, cost model, complete determinism TCB, or neutral head-to-head fault-yield study. Vibe Halt must therefore measure its own vector and say `UNKNOWN` on proprietary dimensions.

### FoundationDB: designed-in simulation and its boundary

The [FoundationDB paper](https://www.foundationdb.org/files/fdb-paper.pdf) describes running the real database code, randomized workloads, and fault injection in a deterministic discrete-event simulation with network, disk, time, randomness, and actor scheduling abstracted. It also states the boundary: the simulator cannot reliably test performance, third-party libraries, or code outside its deterministic Flow world. This is the reason real-target interposition and exact dependency identity are load-bearing for Vibe Halt rather than cosmetic fidelity.

### Coverage guidance: useful signal, not truth

[LLVM's libFuzzer documentation](https://llvm.org/docs/LibFuzzer.html) shows the standard feedback loop: retain inputs that add execution features, evolve the corpus, minimize it while preserving coverage, and optionally use value profiles. It also notes structured inputs need good seeds and that coverage is only a practical quality signal. Vibe Halt should treat coverage and novelty as search rewards, never admission evidence: a new edge is not a new consequential behavior, and full measured edge coverage is not proof of a property.

### Schedule exploration: only where the model fits

The original [PCT paper](https://www.microsoft.com/en-us/research/wp-content/uploads/2016/02/asplos277-pct.pdf) defines bug depth as the minimum number of scheduling constraints needed to expose a bug and gives per-run probability bounds under explicit thread/step assumptions. Current Vibe Halt's event-priority strategy operates over event frontiers, not native threads, and correctly disclaims those guarantees. A PCT-shaped strategy earns renewed investment only on a preregistered depth-at-least-two instrument with low uniform hit probability; it cannot inherit a theorem from a different scheduler model.

### Replay and reduction: complementary obligations

[rr's documented boundary](https://rr-project.org/) demonstrates exact replay for supported Linux process trees by recording kernel inputs and nondeterministic CPU effects, while exposing real limitations: single-core execution, unsupported syscalls, kernel/CPU dependencies, and shared-memory boundaries. This is precedent for exact native replay identity, not whole-system fault exploration.

[Zeller and Hildebrandt's delta-debugging result](https://www.st.cs.uni-saarland.de/papers/tse2002/) establishes 1-minimal failure-inducing input reduction under a test predicate. It does not prove the reduced trace has the same root cause. Vibe Halt must preserve an exact finding fingerprint or a separately verified semantic equivalence claim during shrink.

## Capability ladder

Each rung is a claim boundary. A higher rung requires fresh evidence; it does not relabel evidence from a lower rung.

| Rung | Capability claim | Admission evidence | Explicit ceiling |
|---|---|---|---|
| **A0 — Truth kernel** | Seeded Tier-1 universes, typed evidence, exact local replay, registered properties, and bounded shrink work for the in-process engine. | Accepted-main gates and frozen reference vectors. | No real-target or arbitrary-process claim. |
| **A1 — Real-target observatory** | One exact native Dharma mutation-to-review path executes end to end under `CooperativeD2`; target map, path coverage, property opportunities, open-channel ledger, findings, and signed replay artifacts agree. | Pinned subject/manifest; exact base/candidate/diff; at least one known mutant; independent verifier; every open channel preserved. | This is only the reach/observatory component. It does not by itself prove treatment re-entry or admission binding, and `PROCEED` is forbidden while a mandatory property depends on an open channel. |
| **A2 — Native deterministic profile** | One declared unmodified CPython/Linux profile runs under `NativeInterposed`; every reachable effect is denied, virtualized, or exactly replayed; unsupported effects fail before side effects. | Complete channel inventory; leak battery; effect-tape exhaustion; 100/100 byte-identical replay on each of two named clean hosts; independent helper/protocol audit. | Claim applies only to the exact artifact, kernel/CPU profile, controller set, topology, and campaign. Not yet a security boundary unless separately audited. |
| **A3 — Whole-topology DST** | The declared multi-process service topology, test drivers, checkers, dependencies, and recovery phases all run inside one controlled world. Network, node, disk, clock, process, and schedule faults are explicit choices. | Topology digest; controlled service discovery; no ambient network; overlapping-fault probes; quiet-period recovery checks; liveness opportunity ledger; full-world replay. | Undeclared external services and performance properties remain `UNKNOWN`; containment is separately graded. |
| **A4 — Finding depth** | Each admitted finding carries an independently runnable minimal replay and enough causal structure to make repair testable. | Exact full-world checkpoint/decision tape; shrink predicate; 1-minimality under declared atoms; fingerprint/root-cause-equivalence check; fresh replay outside discoverer; bounded time-to-repro. | Minimal does not mean causal proof; causal statements retain their evidence grade. |
| **A5 — Measured autonomous search** | Search feedback increases consequential state discovery or fault yield over frozen uniform/null strategies at equal cost. | Preregistered discriminating corpus with hidden cases; decorrelated algorithms; coverage/novelty/opportunity signals; equal compute; confidence intervals and retained nulls. | Feedback signals cannot lift a verdict or property modality. No “AI-guided” claim without measured advantage. |
| **A6 — Real-fault advantage** | On supported real targets, Vibe Halt finds more important independently confirmed faults per fixed budget than frontier-AI review alone. | Product Lock tournament: same pinned target, at least three decorrelated reviewers, frozen prompts/tools/budgets/severity/deduplication, human confirmation, and misses retained. | No generalization beyond tested target classes or mechanisms. |
| **A7 — Antithesis-adjacent operating grade** | A repeatable service sustains A2–A6 across multiple real distributed targets with usable throughput, replay reliability, and repair value. | Multiple target classes; published failure and cost distribution; SLOs; independently audited provenance; longitudinal escaped-defect and decision-change data. | “Adjacent” remains a measured vector. It is never equivalence to proprietary internals or a proof certificate. |

## The measurement vector

Do not collapse this into one score. Publish the raw vector for every target profile:

```text
RealTargetGrade {
  target_fidelity: exact subject + dependency/loader closure,
  controlled_channels: closed / applicable,
  uncontrolled_channels: named Open obligations,
  topology_coverage: declared services and consequential paths exercised,
  property_opportunity: reached / required for every property,
  exploration_rate: unique consequential states per CPU-hour and USD,
  search_advantage: paired delta against uniform and other frozen strategies,
  schedule_advantage: paired delta on discriminating schedule corpus,
  replay_success: independent exact replays / admitted findings,
  shrink_success: independently replayable minimal cases / admitted findings,
  median_and_p95_time_to_first_confirmed_fault,
  confirmed_yield,
  invalid_claim_rate,
  baseline_unique_misses,
  real_decision_changes,
}
```

“Consequential state” must be frozen before the experiment as a domain event, property opportunity, dataflow boundary, or effect transition. Raw edge coverage may be reported alongside it but cannot substitute for it.

Throughput is likewise typed. Report at least:

- controlled virtual-time advanced per wall-clock hour;
- completed universes and choice points per CPU-hour;
- distinct property opportunities per CPU-hour;
- evidence bytes and replay time per finding;
- p50/p95 slowdown against the same native workflow; and
- confirmed yield per USD and wall-clock minute.

No universal threshold is defensible before the first target. Each campaign
freezes thresholds before execution. Product Lock's six-week victory and
three-tournament reorientation rule are current law. The C7 packet's proposed
10-working-day profile and two-host 100/100 replay are useful candidate
thresholds from a document explicitly marked `PROPOSAL` that “grants nothing”;
issues #113 and #117 must ratify, amend, or reject them
([C7 proposal boundary](https://github.com/AmitabhainArunachala/vibe-halt/blob/d19ba9e1198c0ab7ef4bb1bdf69a299d056f9754/docs/audits/C7_SUPERVISOR_ADMISSION_PACKET_2026-07-25.md#L1-L14)).

## Search portfolio and anti-Goodhart rules

Only search hypotheses with explicit nulls enter the portfolio:

1. **Uniform seeded exploration** remains the control.
2. **Coverage/value feedback** is eligible when target instrumentation is complete enough that the missing callbacks are themselves ledgered.
3. **Property-opportunity feedback** rewards reaching a predeclared oracle opportunity, not merely executing the assertion line.
4. **Fault-outcome novelty** rewards new typed effect or recovery states, not log-string variation.
5. **Schedule exploration** is a separate axis and must use a scheduler model whose theorem/heuristic assumptions match the controlled target.
6. **Lineage or causal branching** is research-only until it beats simpler controls on preserved targets.

Freeze reward definitions, corpus, budgets, and stopping rules before revealing holdouts. Retain losing strategies and null results. Never let the treatment arm see hidden seeds or mutate reward/oracle instrumentation. If guidance improves coverage but not opportunity, fault recall, or confirmed yield, report precisely that—and do not call it better testing.

## Kill, pivot, and revival criteria

| Hypothesis | Kill or pivot | Revival event |
|---|---|---|
| Cooperative execution can support admission | Kill `CooperativeD2` as a `PROCEED` lane whenever any mandatory property depends on an open channel. Keep it for MAP, diagnosis, and counterexamples. | Fresh evidence under `NativeInterposed`, or a new capability-shaped artifact under `CapabilityClosed`; never relabel D2. |
| The first native interposition profile is viable | If #113/#117 ratify the C7-shaped candidate, kill after its 10-working-day timebox when mandatory workflow compatibility, complete effect control, synchronous rejection, leak probes, independent audit, two-host 100/100 replay, or frozen p95 slowdown ceiling fails. Until then the numbers are proposed, not inherited. | A specific failed primitive or host constraint changes and the preserved workflow/probe passes the ratified thresholds in a new timebox. Narrowing scope creates a new profile. |
| Swarm palette is better than v0 | The existing 0/5 result has already killed that implementation for its tested classes. | A materially new strategy, frozen discriminating corpus including hidden cases, and preregistered improvement—not a rerun until lucky. |
| Event-priority guidance is better than uniform | Preserve the 0/32 wins, 8 losses narrow null; do not invest on VB-006 or inherit PCT guarantees. | A depth-at-least-two controlled instrument with uniform per-universe hit probability below roughly 10%, where median discovery cost improves across the frozen seed set. |
| Coverage guidance improves bug finding | Kill the strategy for the target class if it raises raw coverage but not property opportunities, known-fault recall, time-to-first fault, or confirmed yield at equal cost across the preregistered tournament. | A changed feedback signal or target instrumentation closes the measured disconnect and passes the same held-out evaluation. |
| Whole-topology DST is economically usable | Pivot to narrower workflow/profile testing if p95 slowdown, setup burden, or evidence volume breaches the frozen ceiling on two tenants, even if deterministic replay works. | A named mechanism reduces the failed cost while preserving replay/control thresholds. |
| Finding minimization creates usable depth | Kill automatic “minimal replay” claims if fewer than the preregistered share independently reproduce, if shrink changes the finding identity, or if time-to-minimal breaches the budget. Preserve the original replay. | A corrected atomization or semantic fingerprint passes the old failures and held-out cases. |
| Vibe Halt has product advantage | Product Lock already requires a pause after three mechanism-eligible equal-budget target tournaments with no confirmed severity-weighted yield advantage. Preserve the evidence kernel and diagnose which mechanisms baselines won. | A new mechanism hypothesis, new preregistered tournaments, and a new human product-scope decision. |
| Hypervisor-scale expansion is justified | Kill expansion if A2–A3 have not produced an important real fault missed by baselines, or if the native profile cannot close its declared channels at viable cost. | Repeated paid or high-consequence tenant demand plus measured yield on the narrower profile, and a separately ratified capital/security plan. |

## What this path refuses

- no claim that Vibe Halt is Antithesis because both use seeds or simulated faults;
- no imported assurance from Antithesis marketing or FoundationDB's designed-for-simulation codebase;
- no coverage percentage as a safety percentage;
- no PCT theorem on Vibe Halt event priorities or uncontrolled native threads;
- no deterministic claim that omits loader, dependencies, topology, controller set, and external services;
- no security claim from determinism alone;
- no repair success unless an independent, hidden re-shake treats the patch as hostile new input;
- no known seeded fixture counted as previously unknown real-fault yield;
- no hypervisor program whose first proof is only that a hypervisor was built.

## Decision for the map

Adopt A0–A7 as the capability ladder and the raw measurement vector as the
only meaning of “Antithesis-grade.” A1 is the first reach/observatory component,
not the full constitutional proof. The first Dharma campaign must close A1 plus
the identity-coupled treatment/admission proof from research #101: the same
`FaultClassId`/`FindingId` is found and shrunk, the exact `RepairClaimId`
candidate survives replay and hidden revalidation, and the same-path rebind is
rejected before consequence (R0 + R1). A timeboxed A2 exact-native profile
follows only if the campaign's mandatory claims require D1. Run search
experiments only after there is a real, controlled subject and a discriminating
held-out corpus. Require A6 real-fault advantage before capital-intensive
platform expansion.

The durable product sentence is:

> Vibe Halt is Antithesis for supported real targets only when it can control, explore, replay, minimize, and economically outperform on those targets. Until then it is an honest rung on that path, and its verdict must say which rung.
