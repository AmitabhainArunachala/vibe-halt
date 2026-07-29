# TRUTH KERNEL — Harness-Agnostic Admission Layer, First Tenant dharma_swarm

- **Artifact type:** resumable long-running `/goal` controller (admission
  proposal)
- **Authored:** 2026-07-29
- **Repository:** `AmitabhainArunachala/vibe-halt` (primary);
  `AmitabhainArunachala/dharma_swarm` (first tenant, separate admission)
- **Last observed merged `main`:** `3a77000` (vibe-halt; Reality Bridge Wave A
  R0/R1 merged via PR #55)
- **Target duration:** 4 resumable waves, ~40–80 focused implementation hours
  plus human merge/authorization pauses; expected wall-clock span: weeks
- **Status:** admission proposal; implementation authority begins only after
  the PR containing this controller is human-merged, and only for the waves
  whose entry gates are true

## Operator use

Give this whole file to one long-running coding agent per wave. It is the
strategic successor to
`docs/prompts/VIBE_HALT_REALITY_BRIDGE_LONG_RUNNING_GOAL_2026-07-29.md`
(the "Reality Bridge controller"): the Reality Bridge remains the current
critical-path controller and this controller MUST NOT preempt, weaken, or
fork it. Wave K1 below may run concurrently with Reality Bridge Waves B/C
only where the surface table in §4 says so; every later wave gates on
Reality Bridge state.

Human merge of this controller authorizes only the repository-local packages
described below. It grants no merge, self-approval, spending, credential,
paid-provider, deployment, unsafe-code, or foreign-target authority. Work in
`dharma_swarm` requires a separate admission PR in that repository under its
own governance (`CLAUDE.md`, `ACTIVE_TRACK.yaml`, hot-path acknowledgement);
this controller only defines the cross-repo contract those PRs implement.

Treat every recorded SHA, PR state, line number, and criterion status as
stale until refreshed.

---

# `/goal` TRUTH KERNEL

## 0. Thesis (why this campaign, why now)

Late-July 2026: agent harnesses are commoditizing (Verve's Next.js harness
and peers ship weekly). Code generation is effectively free; verification of
unread code is the scarce resource, and it becomes MORE scarce as harnesses
accelerate. The durable position is therefore not a faster harness but the
**admission layer harnesses plug into**: a deterministic, fail-closed kernel
that takes any agent's proposed change, executes it against a property
contract under adversarial fault injection, and emits a content-addressed
evidence bundle — or rejects it. Harnesses compete on speed; the kernel is
the referee, and referees don't get commoditized.

The two halves already exist and have not been fused:

- **vibe-halt** proves what code *does*: deterministic multiverse execution,
  semantic fault injection, exact-fingerprint shrinking, chained-hash traces,
  v2 evidence bundles (`crates/vh-cli/src/receipts_v2.rs`), and a strict
  Python adapter whose verdicts are Rust-owned (Reality Bridge R0/R1).
- **dharma_swarm** proves what a *claim* is backed by: EvidenceReceipt
  dispatch spine (`dharma_swarm/spine/receipt.py`), telos gates,
  citation-or-silence, and fail-closed merge authority (Merge Master Mike).

The Truth Kernel campaign fuses them: vibe-halt becomes a harness-agnostic
verification substrate with a stable tenancy contract, and dharma_swarm
becomes its first paying-in-discipline tenant — swarm PRs that touch
kernel-covered surfaces must carry a kernel-verified evidence bundle to
merge. Dogfooding until the kernel is the product and the swarm is its
noisiest customer.

## 1. Enduring laws (inherited, non-negotiable)

All six enduring laws of `VISION.md` apply unchanged. In particular:

1. The engine owns truth; no tenant, harness, or client may mint a verdict,
   grade, digest, or receipt.
2. Every claim names its boundary (Tier 1/D0 vs Tier 2/D2); agreement is a
   sampled falsifier, never proof.
3. Evidence fails closed: missing/malformed/stale/ambiguous/tainted is
   `UNCHECKED` or an error, never `CLEAN`.
4. Real utility outranks self-demonstration; tenancy receipts from
   dharma_swarm count as utility only when they gate a real merge decision.
5. A negative result is a result; tenancy nulls stay visible.
6. Humans merge and confirm; green automation is evidence, not approval.

## 2. Mission and completion contract

Deliver four outcomes across four waves:

1. **K1 — Tenancy contract (spec only).** A versioned, harness-agnostic
   admission contract: how any external system submits a verification
   request, what workload/property/fault-model descriptions it may carry,
   and what evidence bundle it gets back. Builds strictly on the merged
   Reality Bridge R0 machine contract; no second wire format.
2. **K2 — Tenant SDK surface.** The strict Python adapter (Reality Bridge
   R1) generalized from "vibe-halt's own client" to "any tenant's client":
   stable request/result/verdict types, engine-identity pinning, and a
   documented capability statement (what the kernel can and cannot check).
   Still stdlib-only, still fail-closed, still no Python-side truth.
3. **K3 — First tenant: dharma_swarm merge gate (separate repo admission).**
   A dharma_swarm-side adapter that (a) wraps a nominated deterministic
   workload (first candidate: DharmaGraph checkpoint/replay semantics,
   `dharma_swarm/graph/**` + `dharma_swarm/checkpoint.py`) as a vh workload,
   (b) binds the resulting Rust-owned receipt digest into an
   EvidenceReceipt `attributes` entry, and (c) adds an advisory (not
   required) CI check on dharma_swarm PRs touching the nominated surfaces:
   present-and-verified bundle → green; absent/unverifiable → advisory red.
   Promotion from advisory to required is a human governance decision,
   never this controller's.
4. **K4 — Second tenant or public tenancy dossier.** Either a second real
   tenant (another repo/harness output verified through the same contract)
   or a decision-ready public dossier for one, proving the contract is not
   dharma_swarm-shaped. Entry requires K3's advisory gate to have produced
   at least 10 real PR verdicts (any mix of green/red/unchecked) on
   dharma_swarm main-bound PRs.

Terminal completion: current merged `origin/main` (vibe-halt) contains K1–K2;
dharma_swarm merged main contains K3 with its advisory check live; and either
K4 is merged or a truthful `TENANCY_READY_SECOND_TENANT_REQUIRED` pause is
declared with a decision-ready packet.

Explicit non-goals (kill on sight):

- replacing or re-implementing dharma_swarm's spine, telos gates, or Merge
  Master Mike inside vibe-halt;
- any "Rust migration of dharma_swarm" — the kernel verifies tenants, it
  does not absorb them;
- claiming the kernel checks properties it cannot (LLM output quality,
  semantic correctness of prose, security of arbitrary code); the capability
  statement in K2 must say exactly what is checked;
- upgrading D2 evidence to D1, or letting a tenant's cooperative behavior be
  described as containment;
- a hosted service, billing, credentials, or any live-provider execution.

## 3. Truth at admission

Refresh, never assume:

- Reality Bridge Wave A (R0 red-matrix contract + R1 strict adapter) merged
  at `3a77000` via PR #55; Wave B (R2 cassette transport, R3 holdout law)
  and Wave C (R4) state must be read from live `origin/main` and open PRs;
- vibe-halt tracks: 3 ACTIVE at `wip_max: 3`
  (`docs/governance/ACTIVE_TRACK.yaml`) — this campaign therefore runs
  under `vibe-halt-core-2026-07` next-items, not a fourth track;
- dharma_swarm spine: `EvidenceReceipt` at
  `dharma_swarm/spine/receipt.py:41` is frozen-dataclass, OTel-exportable,
  with free-form `attributes` — the natural mount point for a
  `vibe_halt.receipt_digest` binding without schema surgery;
- dharma_swarm active portfolio is at `wip_max: 10` with 10 ACTIVE tracks;
  K3 admission there must either serve `dharmagraph-engine-2026-07` (whose
  owned surfaces include `dharma_swarm/graph/**`, `checkpoint.py`) or a
  successor track, decided in that repo's admission PR;
- the vibe-halt criterion "one end-to-end dharma_swarm receipt via a
  VibeHaltSandbox adapter" already exists in
  `docs/governance/ACTIVE_TRACK.yaml` acceptance — K3 is its honest
  fulfillment path.

If any statement has changed, record exact evidence and re-plan before
editing.

## 4. Governance, ownership, and sequencing

| wave | repo | runs under | primary surfaces | entry gate |
|---|---|---|---|---|
| K1 spec | vibe-halt | `vibe-halt-core-2026-07` | `docs/specs/TENANCY_CONTRACT_V0.md` (new), `docs/prompts/**` | this controller human-merged; Reality Bridge Wave A merged (true at authorship) |
| K2 SDK | vibe-halt | `vibe-halt-core-2026-07` | `clients/python/**`, `crates/vh-cli/**` | K1 spec human-merged; no open Reality Bridge R1-surface PR in flight (serialize with Wave B on `clients/python/**`) |
| K3 tenant | dharma_swarm | that repo's admission PR | `dharma_swarm/graph/**` adapter module, one CI workflow, tests | K2 merged on vibe-halt main; dharma_swarm admission PR human-merged |
| K4 second tenant | tbd | new admission | tbd | K3 advisory gate has ≥10 real PR verdicts |

Rules:

- One writer per file; vibe-halt shared surfaces (`scripts/gate.sh`,
  `Cargo.toml`, `Cargo.lock`, `ACTIVE_TRACK.yaml`) stay under the existing
  single-integration-writer protocol.
- K1/K2 must not touch Reality Bridge Wave B/C surfaces
  (`crates/vh-sandbox/**`, `corpus/**`) except through an exact interface
  request; if Wave B is mid-flight on `clients/python/**`, K2 waits.
- Nothing in this controller mutates `ACTIVE_TRACK.yaml` acceptance
  criteria; the admission PR may append one `next` item to
  `vibe-halt-core-2026-07` referencing this file, and nothing else.
- Every PR starts draft, names exact base/head SHA, cites this controller,
  includes test receipts and rollback notes. Humans mark ready and merge.

## 5. Work packages

### K1 — Tenancy contract v0 (spec only, no code)

Write `docs/specs/TENANCY_CONTRACT_V0.md` defining, on top of the merged R0
machine contract (do not fork it):

- **Tenant identity:** a tenant is a named external system submitting
  verification requests; the kernel records tenant id as untrusted request
  metadata, never as authority.
- **Workload description:** what a tenant may submit — today only workloads
  compiled into `vh` (registered workload id + parameters). The contract
  must state plainly that arbitrary tenant code runs only via the Tier-2/D2
  sandbox path with all capability-channel caveats, and via no other path.
- **Property contract:** how a tenant declares Always/Sometimes properties
  for a registered workload, versioned, hash-bound into the request.
- **Evidence bundle:** exactly the v2 bundle plus a tenancy envelope
  (request identity, tenant id, contract version, engine digest, receipt
  digest) — content-addressed, atomically emitted, Rust-verified.
- **Verdict semantics for tenants:** `CLEAN`/`FINDINGS`/`UNCHECKED`/errors
  mean exactly what R0 says; a tenant CI gate must map `UNCHECKED` and every
  error to not-green. Write the mapping table into the spec.
- **Capability statement:** an explicit enumerated list of what a kernel
  verdict does and does not attest, including the D2/29-open-channels
  boundary, so no tenant can honestly over-claim.

Acceptance: spec merged; a negative-space section lists at least five
things the contract refuses to promise; no code changed.

### K2 — Tenant SDK surface

Generalize the R1 adapter into the tenant client without adding a second
truth path:

- expose request construction for any registered workload + property
  contract id (today: the built-in demo workloads and the K3 workload once
  registered), preserving the closed typed outcome set;
- pin engine identity per K1 (engine digest verified before any verdict is
  returned — already R1 law; extend to the tenancy envelope);
- add `verify_bundle(path)`-shaped re-verification so a tenant CI job can
  check a bundle produced elsewhere (Rust does the verification; Python
  relays the typed result);
- ship the capability statement as data the client can print, sourced from
  the Rust-published operation/feature set, not hand-maintained prose;
- stdlib-only, fail-closed, all R0 negative-matrix behaviors preserved; the
  full R1 hostile-construction test suite must still pass unweakened.

Acceptance: direct CLI and tenant-client paths bind identical receipt
digests for identical requests; a bundle tampered in any single byte fails
re-verification; `make gate` green twice from clean state on the exact head.

### K3 — dharma_swarm as first tenant (separate repo admission)

In dharma_swarm, via its own admission PR:

- select and freeze the nominated workload: DharmaGraph checkpoint/replay
  determinism (crash-during-checkpoint, resume-exactly-once, no lost or
  doubled node effects) is the first candidate because it is genuinely
  deterministic-checkable and owned by an ACTIVE track;
- implement the adapter that drives it through the K2 client — the
  vh-registered workload models the graph-engine state machine; property
  contract encodes the invariants above;
- bind the Rust receipt digest into the dispatch spine:
  `EvidenceReceipt.attributes["vibe_halt.receipt_digest"]` (plus contract
  version and engine digest) on the runs that produced it — additive, no
  spine schema change;
- add one advisory CI workflow: on PRs touching the nominated surfaces,
  require a fresh evidence bundle in the PR's artifact space, re-verify it
  with `verify_bundle`, map verdicts per the K1 table; advisory only;
- write the promotion memo (advisory → required) as a governance doc for a
  human decision — do not flip it.

Acceptance: at least one real dharma_swarm PR shows the advisory check
producing a verified verdict end-to-end; the vibe-halt acceptance criterion
"one end-to-end dharma_swarm receipt" is satisfied by citation to that PR
and bundle digest; a deliberately broken mutant of the workload turns the
advisory check red with a one-command deterministic repro.

### K4 — Second tenant or public tenancy dossier

Entry gate: ≥10 real advisory verdicts on dharma_swarm main-bound PRs, with
the miss/null/unchecked counts published, not just the greens.

Then either onboard a second tenant through the unchanged K1/K2 contract
(candidate classes: another repo in the org; the output of a commodity
harness run, e.g. a Verve/Next.js-harness-generated service's state machine,
modeled as a registered workload), or publish a decision-ready dossier for
one, including exactly why it can or cannot be checked honestly today. A
truthful "the contract is dharma_swarm-shaped in these three ways" finding
is a valid K4 result and feeds a v1 contract revision.

## 6. Mandatory gate matrix

| property | required proof |
|---|---|
| Reality Bridge unharmed | its controller file, tracks, and merged surfaces byte-identical except where a human-merged Wave B/C PR changed them |
| single truth path | no new wire format; tenancy envelope wraps R0, never replaces it; grep-level proof no Python constructs verdicts |
| fail-closed tenancy | tampered/absent/stale bundle in the K3 advisory check never yields green |
| capability honesty | capability statement generated from Rust-published data; D2 and 29-open-channels language present verbatim |
| tenant reality | K3 verdicts cited to real PR numbers and bundle digests, misses included |
| no scope creep | zero edits to dharma_swarm spine schema, telos gates, or merge authority beyond the additive attributes binding |
| full integration | `make gate` twice clean on each exact vibe-halt head; dharma_swarm `make onboard` + targeted tests green on each exact head |

## 7. Kill and stop rules

Stop the affected lane when:

- Reality Bridge Wave B/C work collides on `clients/python/**` or
  `crates/vh-cli/**` without an agreed serialization;
- the tenancy contract would need a second truth authority, stdout parsing,
  or a caller-suppliable verdict to be "useful";
- the K3 workload cannot be made honestly deterministic (then publish the
  exact nondeterminism finding — that is a result, likely a real bug);
- dharma_swarm governance declines the admission PR (respect it; produce a
  decision packet, do not route around Merge Master Mike);
- anyone proposes flipping the advisory check to required inside this
  campaign;
- two repair loops fail for the same cause without new evidence.

At a stop, emit one packet: exact blocker, evidence, smallest safe options,
recommendation, consequence of waiting.

## 8. Terminal report labels

- `TRUTH_KERNEL_K1_SPEC_READY_FOR_HUMAN_REVIEW`
- `TRUTH_KERNEL_K2_SDK_READY_FOR_HUMAN_REVIEW`
- `TRUTH_KERNEL_K3_TENANT_LIVE_ADVISORY`
- `TRUTH_KERNEL_COMPLETE_SECOND_TENANT`
- `TRUTH_KERNEL_TENANCY_READY_SECOND_TENANT_REQUIRED`
- `TRUTH_KERNEL_BLOCKED_<precise_reason>`

Anything else is an interim checkpoint, not completion.

---

The campaign succeeds when a system that vibe-halt did not write, run by an
operator vibe-halt does not control, treats a kernel evidence bundle as the
reason a change merges — and when every verdict in that chain can be
replayed, byte-for-byte, by anyone who doubts it.
