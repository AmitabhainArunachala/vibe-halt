# First Principles and Target Architecture

Synthesis of Lanes A/B/C. Every retained invariant is mapped `precedent → evidence → vibe-halt failure mode → proposed boundary → verifier → rollback`. "1000x" is treated as four independent multipliers on the existing kernel, not as platform cosplay.

Identifier convention: audit rejections are `REJ-R1`–`REJ-R3`; roadmap
recommendations are `REC-R0`–`REC-R8`. Legacy bare `R#` references are
noncanonical because the two namespaces overlap.

## 1. Derived invariants (tested against the actual system)

**Retained:**

- **I1 — A run identifies its world.** Code, config, seed, fixtures, dependency state. *vibe-halt already holds this* (frozen PRNG/trace identities, seed tree, doctor fingerprint) — verified byte-identical cross-process reruns. Keep; extend to workload-fixture identity (cassette digests) as Tier-2 lands.
- **I2 — Important nondeterminism is explicit, captured, controlled, or bounded.** Holds for the kernel (deny-list, two documented exemptions). **Fails at the mission boundary**: an LLM provider call or a subprocess is uncontrolled nondeterminism — hence Tier-2/cassettes are not optional features, they are the invariant's completion.
- **I3 — Faults enter through a defined world model, not scattered mocks.** Holds (runner-owned SimNet/SimDisk with lifecycle ledger — a genuine differentiator; few systems attest per-injection outcomes). Fails at edges: ClockSkew no-op, network-wide partitions, no provider/tool fault family.
- **I4 — Safety/correctness properties are executable, and "nothing asserted" is never "clean".** Holds and is *better than Antithesis's documented default* (empty-contract→UNCHECKED by construction; Antithesis approximates this with the assertion catalog + `must_hit`).
- **I5 — Failures yield durable, replayable, minimized counterexamples.** **Partially held**: replay yes (verified), durable no (stdout only, `~/.vibe-halt/` phantom), minimized no (shrinker orphaned). This is the weakest-held core invariant.
- **I6 — Exploration coverage is measurable and fed back.** **Absent.** Uniform-random Monte-Carlo; no coverage, no novelty, no schedule search. The single largest gap vs Antithesis (coverage+RL+assertions-as-clues) and vs TigerBeetle (swarm masks).
- **I7 — Evidence is distinct from report, inference, consensus, and speculation.** Holds unusually well (modality discipline, sampled-falsifier naming, gate-held Python quarantine). Extend it *into the type system* (§5) so promotion is checked by the evaluator, not by reviewer vigilance.
- **I8 — Test worlds are hermetic enough to reproduce and realistic enough to matter.** Hermetic: yes (verified). Realistic: **no** — only in-process Rust toys; zero harvested real bugs. Realism is now the binding constraint, not determinism.

**Rejected / descoped:**

- **REJ-R1 — Whole-machine determinism for unmodified binaries (hypervisor).** Rejected for v0.x: DETERMINISM_TIERS.md scopes it out correctly; multi-year, x86-only, and the agent-systems mission is served at the process/protocol boundary (I2's cassette/subprocess path), not the CPU boundary. Revisit only if dhyve-class OSS substrates mature. *Falsifier of this rejection:* Tier-2 subprocess determinism proves unachievable at acceptable fidelity in the current spike.
- **REJ-R2 — Exhaustive model checking as the primary engine.** Rejected: Loom-style exhaustiveness is the wrong granularity for whole workloads; Shuttle's own README (observed) positions randomized PCT as finding most concurrency bugs. Use Loom only on vibe-halt's *own* scheduler.
- **REJ-R3 — ML/RL-guided search as a near-term dependency.** Antithesis uses RL (reported), but the Gradius evidence shows trivial strategies suffice given branching machinery; swarm masks + PCT + coverage-from-assertions buy most of the yield at zero ML-risk. RL is a later optimization, never a foundation.

## 2. Gap → concrete failure modes map

| gap (evidence) | concrete failure mode it causes |
|---|---|
| Uniform-random exploration (`vh-gremlin/src/lib.rs:110-132`) | Rare multi-fault interactions found only by seed luck; recall plateaus at hand-tuned palettes; 100k universes ≈ 100k correlated samples |
| Fixed FIFO scheduler, no choice points (`vh-core/src/sched.rs:71-76`) | Same-timestamp race bugs are *invisible by construction* — the divergence detector even documents a schedule-keyed evasion |
| In-process Rust targets only (`vh-multiverse/src/lib.rs:359-381`) | The stated mission (vibe-coded repos, agent systems) is untestable; corpus stays self-seeded; recall numbers stay self-referential |
| No `vh shrink` CLI | Findings land as full fault plans; humans do minimization by hand; corpus entries bloat and lose falsifiability |
| No durable evidence store | Findings evaporate at process exit; no regression corpus, no triage history, no CI trend line |
| No provider/tool fault family | LLM 429s, malformed tool responses, context truncation — the actual fault surface of agent systems — uninjectable |
| No per-universe config diversity | Every universe draws from the same palette (TigerBeetle's swarm lesson unlearned) |
| No LICENSE | The empty OSS niche cannot be occupied legally by anyone but the author |

## 3. Build / borrow / adapt / partner / avoid

| decision | what | precedent & evidence |
|---|---|---|
| **Build** | Decision tape + PCT schedule perturbation; per-universe swarm masks; `vh shrink` CLI; evidence store (NDJSON receipts under a real `--out` dir); provider/tool gremlin family (LLM 429/malformed/truncation); Tier-2 subprocess+cassette boundary | Shuttle pct.rs/replay.rs (observed); TigerBeetle fuzz.zig (observed); vh-shrink exists (tested); I2/I5/I6 |
| **Borrow (rewrite shapes, ~200–400 lines each)** | Stateright Expectation/discovery-path API; TigerBeetle PacketSimulatorOptions parameterization; Antithesis 7-prefix test-command algebra | Lane C deep dives; Lane B test_composer docs (observed) |
| **Adapt (algorithm)** | Molly/LDFI lineage-driven fault targeting; Antithesis causality analysis (rewind + re-explore → bug-probability-over-time); Jepsen history-checking over cassettes | Lane C §4.9; Lane B §5 |
| **Partner** | None now. Later: dogfood on dharma_swarm (`clients/python` Phase-4 hook already planned); OSS program à la Antithesis etcd once corpus is real | — |
| **Avoid** | Hypervisor; RL exploration near-term; Stateright/Turmoil/Shuttle as dependencies; agent-eval dashboards; multi-LLM sign-offs as gates; AFL++ linkage (AGPL) | §1 REJ-R1–REJ-R3 |

## 4. Target architecture

Four multipliers on the existing kernel, in dependency order. Each ships independently behind the existing gate battery.

```
                        ┌─────────────────────────────────────────────┐
                        │                 vh campaign                  │
                        │  (parallel universes, budget allocation,     │
                        │   evidence store, regression corpus)         │
                        └───────┬─────────────────────┬───────────────┘
                                │ findings             │ replay bundles
              ┌─────────────────▼───────┐   ┌──────────▼──────────────┐
              │ M4: Finding depth        │   │ M3: Target reach         │
              │ vh shrink (CLI)          │   │ Tier-2 subprocess univ.  │
              │ decision tape → causality│   │ cassettes (LLM/tool IO)  │
              │ bug-probability-over-time│   │ provider/tool gremlins   │
              └─────────────────▲───────┘   │ Maelstrom-style fixtures │
                                │           └──────────▲──────────────┘
              ┌─────────────────┴───────┐              │
              │ M2: Guided exploration   │   ┌──────────┴──────────────┐
              │ swarm masks (TB idiom)   │   │ M1: Kernel hardening     │
              │ decision tape + PCT      │   │ (exists; extend: per-node│
              │ coverage from sometimes- │   │ clocks, topology, TB     │
              │ hit novelty              │   │ fault params, LICENSE)   │
              └─────────────────▲───────┘   └──────────────────────────┘
                                │
              ┌─────────────────┴───────────────────────────────────┐
              │ EXISTING VERIFIED KERNEL: vh-core/trace/gremlin/props│
              │ /multiverse + gate battery + verdict epistemics      │
              └─────────────────────────────────────────────────────┘
```

**M1 — Kernel hardening (days).** Add LICENSE; fix `make onboard` py≥3.11 check (D1); implement ClockSkew or stop generating it (D6); per-node clocks + link-level partitions/clogs with hysteresis using the TigerBeetle field set. *Boundary:* kernel crates stay deny-list pure.

**M2 — Guided exploration (weeks; the 10–100x multiplier).**
- *Decision tape*: every scheduler pop and fault-plan choice records `(decision_point, chosen, alternatives)` into the chain-hashed trace. Replay = re-execute tape. Superset of Shuttle's schedule string.
- *Swarm masks*: each universe draws a randomized fault-family/palette mask (TigerBeetle `random_enum_weights` idiom) — disjoint feature subsets per universe; ~20 lines in vh-gremlin, immediate diversity multiplier.
- *PCT perturbation*: priority-permutation scheduler strategy with change-point budget d over same-timestamp choice points; probabilistic guarantee (≥1/n^(k·d) for ≤d-preemption bugs) instead of hope.
- *Assertion-novelty feedback*: `sometimes`-hit bitmaps and property-violation edges feed a novelty score; universes branching from novel prefixes get budget (Antithesis "assertions are clues", observed). No RL.
- *Verifier:* seeded bakeoff vs Turmoil/MadSim baseline and vs uniform-random ablation on the 5-bug corpus + 5 new seeded classes: guided must reach pinned recall with ≥10x fewer universe-executions, else the guidance is theater (kill criterion).

**M3 — Target reach (weeks–months; the mission multiplier).**
- *Cassette boundary (D1 grade)*: LLM/tool/provider IO recorded as digested cassettes; universes replay against cassettes with fault injection on replay (429, malformed, truncation, latency). Checker runs Jepsen-style over histories (at-most-once, conservation). This makes *agent harnesses* testable without a live provider.
- *Tier-2 subprocess universes*: the active goal doc — child process with controlled stdin/stdout/fs/clock via a preload/shim layer, seeded entropy; honesty rule: if a boundary leaks wall-clock/OS entropy, the run is UNCHECKED, never CLEAN.
- *Fixture format*: Maelstrom-style stdin/stdout workload contracts so an LLM-written target repo gets a harness without bespoke adapters.

**M4 — Finding depth (weeks; the trust multiplier).**
- `vh shrink <finding>` wiring the existing ddmin (+ occurrence→argument hierarchy levels 2–3).
- *Causality analysis*: rewind on the decision tape to prefix P, re-explore forward N times with perturbed tapes, emit bug-probability-over-time — Antithesis's published algorithm on vibe-halt's artifact.
- *Evidence store*: NDJSON receipts + replay bundles (seed + tape + trace hash + workload digest) in a real `--out` directory; regression corpus replays bundles in CI. Retires the `~/.vibe-halt/` phantom by making the policy true.

**Data flow (steady state):** campaign draws swarm masks → universes run (kernel) → traces + tapes + property outcomes stream to evidence store → novelty feedback biases next mask batch → findings auto-shrunk → causality analysis on demand → minimized bundles enter regression corpus → corpus entries cite bundle digests (promotable only per §5).

### Interfaces, schemas, and lifecycle

- **Decision tape schema**: append-only records `(site_id, candidate_set_digest, chosen_index, policy_id)` on a new trace stream; tape digest joins universe identity `(seed, tape_digest)`; replay verifies tape-vs-execution equivalence. Lifecycle: recorded per universe → stored in bundle → consumed by shrinker (M4) and causality (REC-R8) → frozen per trace-format law (new stream, not a v0 format change).
- **Replay bundle schema**: `{seed, tape_digest, trace_hash, workload_digest, palette_mask, finding_refs}` as NDJSON; bundle alone must reproduce the finding (REC-R4 acceptance). Rollback: evidence store is additive; stdout remains default until gate-proven.
- **Cassette schema**: `(request_digest, response, recorded_at, provider_meta)`; digest mismatch → UNCHECKED, never CLEAN. Lifecycle: record (live) → replay (sim) → staleness declared in run manifest.
- **Property/monitors interface**: Stateright-shaped `Expectation::{Always, Sometimes}` + `assert_discovery(path)` re-checkable counterexamples; existing `EndStateOracle` remains the end-of-timeline hook. Failure semantics: any monitor violation = FINDINGS; any monitor silence = UNCHECKED contribution.
- **Admission interface (typed construct, §5)**: corpus entries accept only `Claim<_, Reproduced, _>` with a `PromotionProof`; all other modalities type-error at the gate.

## 5. Typed epistemic/authority construct

typed-contribution-id: VH-EPI-CLAIM-001

The corpus and verdict surfaces already practice modality discipline in prose; this construct makes it **evaluator semantics** so promotion cannot be smuggled past by fluency — the exact failure the Python client committed before quarantine.

```rust
/// A claim is a value plus its epistemic provenance. Modality and authority
/// are type parameters: promotion rules are typechecker rules, not conventions.
enum Modality { Speculative, Reported, Observed, Reproduced }
enum Authority { SelfBuilt, VendorPublished, IndependentVerifier, RuntimeExperiment }

struct Claim<T, const M: Modality, const A: Authority> {
    value: T,
    scope: Scope,                 // seed set, commit, workload digest, env
    evidence: Vec<EvidenceRef>,   // artifact:// bundle digests only
    falsifier: Predicate<T>,      // the observation that would defeat the claim
}

/// Promotion obligation: what a gate may consume.
trait Admits<T> { fn require() -> (Modality, Authority); }

/// Example gate: a corpus recall entry may only cite a claim that is at least
/// Reproduced by an IndependentVerifier (vh-verify) or RuntimeExperiment.
impl Admits<RecallRate> for CorpusAdmission {
    fn require() -> (Modality, Authority) {
        (Modality::Reproduced, Authority::IndependentVerifier)
    }
}

// Type-level rule: Claim<RecallRate, Reported, SelfBuilt> does NOT implement
// Satisfies<CorpusAdmission>; only claims produced by a Promotion Proof do:
struct PromotionProof<T> {
    from: ClaimDigest,          // the weaker claim
    command: VerbatimCommand,   // the discriminating check that was run
    transcript: ArtifactRef,    // recorded output + exit code
    at: PinnedCommit,
}
fn promote<T, const A: Authority>(
    c: Claim<T, Reported, A>,
    proof: PromotionProof<T>,
) -> Result<Claim<T, Reproduced, Authority::RuntimeExperiment>, PromotionError>;
```

Evaluator rule (challengeable): **`Claim<T, Reported, SelfBuilt>` can never satisfy an admission obligation** — not with more signatures, not with higher stated confidence, not with model consensus. Only `promote()` with a recorded discriminating command changes modality, and `promote` is the *only* constructor of `Reproduced` claims outside the runner. This is the typechecker version of the repo's existing "citation-or-silence" law; the challenge to defend is whether `IndependentVerifier` (vh-verify re-implementation) is genuinely independent given shared authorship — if not, the authority lattice needs a fifth variant for "same-author reimplementation".

### Counterexample fixture

`corpus/fixtures/epistemic/reject_self_reported_recall.json` — a fixture the admission evaluator must reject:

```json
{
  "claim": "fsync-lie recall is 21/100 at seed 0xD1CE",
  "modality": "reported",
  "authority": "self_built",
  "evidence": [{"kind": "prose", "locator": "DESIGN.md sign-off: Claude 91%"}],
  "expected_verdict": "REJECT",
  "reason": "Reported+SelfBuilt cannot satisfy CorpusAdmission; a multi-LLM sign-off percentage is not a PromotionProof. Only corpus-recall.txt (reproduced command transcript at pinned commit) promotes this claim.",
  "historical_basis": "clients/python fabricated reproducibility_score=1.0 for nonexistent repos before quarantine — exactly this promotion attempted in prose"
}
```

And the positive control `accept_reproduced_recall.json`: same claim promoted via `PromotionProof { command: "cargo run -p vh-cli -- run --workload corpus-fsync-lie --seed 0xD1CE --universes 100", transcript: artifact://commands/corpus-recall.txt, at: 84f911e }` → ACCEPT.

## 6. Why this is 1000x and not 10x

- M2 multiplies *exploration yield per universe-execution* (swarm × PCT × novelty feedback; TigerBeetle/Shuttle/Antithesis precedents suggest order(s) of magnitude over uniform Monte-Carlo — but this is a hypothesis with a seeded bakeoff as its falsifier, not a promise).
- M3 multiplies *target space* from "in-process Rust toys" to "any agent harness with cassetteable IO" — the actual mission; also unlocks corpus harvesting (0→25+ real bugs), converting recall from self-referential to evidential.
- M4 multiplies *finding value*: a minimized, causality-localized, durably stored bundle is actionable; a raw fault plan on stdout is not.
- The kernel's verified determinism and ungameable verdicts are the multiplicand that makes each multiplier meaningful — Antithesis's $152M and customer-led Series A (Lane B) is the market proof that this exact compound is valuable.
- What is deliberately *not* in the 1000x: hypervisors, RL, dashboards, and any claim promotion without a PromotionProof.
