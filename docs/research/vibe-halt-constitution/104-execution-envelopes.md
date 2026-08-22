# Execution envelopes without evidence laundering

**Issue:** [#104 — Compare the execution envelopes without laundering evidence](https://github.com/AmitabhainArunachala/vibe-halt/issues/104)

**Repository basis:** accepted Vibe Halt `main` at `d19ba9e1198c0ab7ef4bb1bdf69a299d056f9754`

**Research date:** 2026-08-22

**Scope:** decision research only; no implementation or execution

## Verdict

Lock all three envelopes, but do **not** make them rungs on one ladder:

- `CooperativeD2` is the bootstrap and diagnostic envelope. It touches the real native target with the least porting, but it cannot support admission-grade `PROCEED` while relevant channels remain open.
- `NativeInterposed` is the reach envelope for claims about the existing native artifact. It is the primary first-campaign candidate when exact CPython/Dharma fidelity matters. Determinization and containment remain separate claims.
- `CapabilityClosed` is the closure envelope for a deliberately recompiled or distilled artifact. It can provide the strongest capability boundary, but a WASI component is a **different subject** from the native CPython artifact, and capability closure is not determinism by default.

The constitutional rule is:

> Evidence is indexed by `(artifact, execution envelope, controller set, campaign)`. There is no cast from evidence in one envelope to evidence in another. Cross-envelope results may create a typed corroboration or equivalence claim that retains both identities; they never inherit the stronger grade of either side.

For the already-locked sequence, use `CooperativeD2` to map and bootstrap the real Dharma promotion path, attempt `NativeInterposed` to admit claims about that same path, and reserve `CapabilityClosed` for the extracted `PromotionPermit` residue or another intentionally capability-shaped cut. WASI is not a substitute result for native Dharma.

**Evidence status:** only `CooperativeD2` is observed in accepted Vibe Halt. `NativeInterposed` is a specified but unimplemented Vibe profile; Hermit is mechanism precedent, not evidence that Vibe's profile works. `CapabilityClosed` is an external substrate option with no Vibe backend at this revision. Every positive statement about either future envelope below is conditional on its admission gates.

## Accepted-main ground truth

The comparison begins from five laws already present at `d19ba9e`:

1. A broad target front door never creates a broad proof claim; unsupported or uncontrolled coverage is `UNKNOWN`, and every source artifact is bound to an exact observed revision ([Product Lock v1, lines 17–29](https://github.com/AmitabhainArunachala/vibe-halt/blob/d19ba9e1198c0ab7ef4bb1bdf69a299d056f9754/docs/specs/PRODUCT_LOCK_V1.md#L17-L29)).
2. Current subprocess evidence makes no deterministic-environment claim: it runs twice, records divergence, and keeps the claim inside that exact boundary ([Determinism Tiers, lines 28–43](https://github.com/AmitabhainArunachala/vibe-halt/blob/d19ba9e1198c0ab7ef4bb1bdf69a299d056f9754/docs/specs/DETERMINISM_TIERS.md#L28-L43)).
3. All 29 current subprocess capability channels remain `Open`; cassette transport alone closes none ([Determinism Tiers, lines 50–63](https://github.com/AmitabhainArunachala/vibe-halt/blob/d19ba9e1198c0ab7ef4bb1bdf69a299d056f9754/docs/specs/DETERMINISM_TIERS.md#L50-L63)). The sealed receipt makes D1 constructible only when every channel is controller-closed ([`capability.rs`, lines 139–225](https://github.com/AmitabhainArunachala/vibe-halt/blob/d19ba9e1198c0ab7ef4bb1bdf69a299d056f9754/crates/vh-sandbox/src/capability.rs#L139-L225)).
4. Current world identity is useful but not sufficient for settlement: it observes executable bytes immediately before spawn, cannot make observation-to-exec atomic, and leaves loader/filesystem channels open ([Sandbox Capability Envelope v1, lines 130–145](https://github.com/AmitabhainArunachala/vibe-halt/blob/d19ba9e1198c0ab7ef4bb1bdf69a299d056f9754/docs/specs/SANDBOX_CAPABILITY_ENVELOPE_V1.md#L130-L145)). The sandbox digest is explicitly deterministic FNV-1a-128, not cryptographic ([`vh-sandbox/src/lib.rs`, lines 1678–1688](https://github.com/AmitabhainArunachala/vibe-halt/blob/d19ba9e1198c0ab7ef4bb1bdf69a299d056f9754/crates/vh-sandbox/src/lib.rs#L1678-L1688)).
5. Authority and modality are orthogonal; a stronger authority cannot manufacture a stronger observation, replay, or proof ([`modality.rs`, lines 1–56](https://github.com/AmitabhainArunachala/vibe-halt/blob/d19ba9e1198c0ab7ef4bb1bdf69a299d056f9754/crates/vh-cli/src/modality.rs#L1-L56)). Envelope identity must obey the same rule: changing where code ran cannot silently lift what was learned.

## Terms that must not collapse

- **Artifact fidelity:** which exact bytes are the subject of the claim. A source revision, native executable plus interpreter/loader closure, and WASM component are three different identities.
- **Capability closure:** whether an effect is synchronously denied, virtualized, exactly replayed, or structurally unavailable at the guest boundary. “Not seen in this run” is not closure.
- **Determinism:** whether all claim-relevant choices and effects are controlled sufficiently for the declared replay statement. A security sandbox is not automatically deterministic.
- **Security boundary:** whether hostile guest code is contained from the host. A deterministic supervisor is not automatically a security boundary.
- **Behavioral correspondence:** a separately tested relation between two artifacts. Shared source or matching outputs does not make the artifacts identical.

The current two-state `Open | Closed` ledger should eventually gain a controller-only `Absent { boundary_witness }` disposition for capability-shaped guests. `Absent` must be evidenced against a versioned guest interface and host link graph; omission from a receipt can never mean absence.

## Decision matrix

| Dimension | `CooperativeD2` | `NativeInterposed` | `CapabilityClosed` |
|---|---|---|---|
| Claim subject | Native command/script/interpreter as observed by the runner. Current loader binding is pre-spawn and raceable, so “exact loaded image” remains open. | The existing native ELF/script/interpreter and its declared loader closure. Highest fidelity to the artifact users actually run, provided the supervisor binds the bytes, dependencies, kernel ABI, and policy it executed. | A separately built core Wasm module or component. Bind source revision and build provenance, but the executable subject is the WASM digest, never the native digest. CPython itself reports its WASI build as `sys.platform == "wasi"` and `machine == "wasm32"` ([CPython WASI README](https://github.com/python/cpython/blob/999a046b24cff4ba0e72b574196721f66bd08237/Platforms/WASI/README.md)). |
| Control mechanism | Voluntary cassette/API use, allowlisted environment, bounded subprocess observation, repeated executions. A direct clock/network/filesystem/subprocess path can bypass cooperation. | ptrace/seccomp/namespaces/virtual time/scheduling and default-deny effect policy, but only for the named supported profile. Hermit demonstrates that an unmodified x86-64 Linux guest can have scheduling, time, random data, CPUID, and selected metadata controlled ([Hermit README](https://github.com/facebookexperimental/hermit/blob/e407311a3c1ab1da7d41220696929dc8235ec925/README.md#hermit)). | Wasm has no raw system-call access; all outside interaction occurs through linked imports. WASI is capability-oriented, has no ambient authority, and uses explicit handles/link-time capabilities ([WASI design principles](https://github.com/WebAssembly/WASI/blob/3071db04c857b3a2c047d3d1ac694bc41f021796/docs/DesignPrinciples.md)). Granted imports still require controlled host implementations. |
| Current/possible channel disposition | At `d19ba9e`: 29/29 `Open`; D2 only. | D1 is possible only if every reachable channel is denied, virtualized, or replayed with controller evidence; unknown syscalls/effects must fail closed. Vibe Halt's existing C7 proposal already requires that shape ([C7 packet, lines 61–84](https://github.com/AmitabhainArunachala/vibe-halt/blob/d19ba9e1198c0ab7ef4bb1bdf69a299d056f9754/docs/audits/C7_SUPERVISOR_ADMISSION_PACKET_2026-07-25.md#L61-L84)). | Many native channels can be controller-proved `Absent` at the guest boundary; resource access is limited to imports/grants. This still does not close time/randomness automatically: Wasmtime's default `WasiCtx` uses host clocks and randomly initialized RNGs, while filesystem and network grants are separately configurable ([`WasiCtxBuilder`](https://docs.wasmtime.dev/api/wasmtime_wasi/struct.WasiCtxBuilder.html)). |
| Strongest honest claim | “This exact declared D2 run observed X; these repetitions agreed/diverged; these channels remained open.” It may support a replayed concrete `HALT`, or `UNKNOWN`; not admission-grade `PROCEED`. | “For this exact native artifact, host profile, supervisor/policy, controller set, and campaign, controlled effects replayed exactly.” May support bounded `HALT` or `PROCEED`; never whole-repository safety. | “This exact WASM artifact was confined to this import/grant world and produced this result.” It supports deterministic replay only if every granted nondeterministic service and scheduling source is also controlled. It says nothing directly about the native artifact. |
| Evidence TCB | Vibe runner and receipt code; host OS/filesystem/loader; native interpreter and dependencies; target cooperation; every external service behind an open channel. | Supervisor/helper and protocol; Linux kernel and enabled namespace/seccomp/ptrace features; CPU/PMU and feature masking; loader/libc/interpreter; input snapshot/effect store; Vibe verifier. | Source-to-Wasm compiler and build pipeline; Wasmtime/Cranelift or named engine; exact WASI version/WIT world; host adapter implementations and grants; host OS/hardware; Vibe verifier. |
| Host requirements | Any supported host that can run the native command; fidelity and behavior remain host-specific. | Narrow. Hermit currently requires x86-64 Linux, ptrace/seccomp, namespaces, and optionally PMU access for precise preemption; containers/VMs often block some of these ([Hermit requirements](https://github.com/facebookexperimental/hermit/blob/e407311a3c1ab1da7d41220696929dc8235ec925/README.md#requirements)). | Potentially broad engine portability, but portability is API-specific and engines need not implement every WASI API ([WASI portability principle](https://github.com/WebAssembly/WASI/blob/3071db04c857b3a2c047d3d1ac694bc41f021796/docs/DesignPrinciples.md#portability)). The exact runtime and WIT/API versions remain identity. |
| Existing CPython compatibility | Highest: runs the host CPython and installed native extensions, subject to open channels. | Medium and profile-dependent. Hermit reports simple Python/file/JSON runs working but complex imports and subprocess record/replay limited; its documentation warns overall Linux compatibility is incomplete ([Hermit compatibility](https://github.com/facebookexperimental/hermit/blob/e407311a3c1ab1da7d41220696929dc8235ec925/README.md#compatibility)). A Vibe Halt supervisor must measure its own profile rather than inherit Hermit's results. | A real CPython WASI build exists at Tier 2, but Python documents a subset of POSIX: process APIs fail, filesystem/permissions and networking differ, and modules using processes, threads, signals, or IPC may be absent; `asyncio` is explicitly unavailable on WASI ([Python WebAssembly limitations](https://docs.python.org/3.14/library/intro.html#webassembly-platforms), [`asyncio` availability](https://docs.python.org/3.14/library/asyncio.html)). Compatibility with a Dharma path is therefore an experiment, never assumed from “Python runs.” |
| Portability | Runner may be portable; the native artifact/result is not portable beyond its bound host envelope. | Low: initial Vibe profile is Linux/x86-64/single-process/single-thread, and no result generalizes to another kernel, architecture, or profile. | Highest for the exact WASM artifact across engines that implement the exact world, but host adapters can differ. Portability does not imply native equivalence or cross-engine deterministic identity. |
| Performance budget | At least two executions for current run-twice evidence, plus capture/verification. No fixed slowdown claim should be inferred. | Hermit's current ptrace planning range is roughly 3–6× native and workload-sensitive ([Hermit performance](https://github.com/facebookexperimental/hermit/blob/e407311a3c1ab1da7d41220696929dc8235ec925/README.md#performance)); Vibe Halt must preregister and measure its own p95 ceiling. | No universal factor. Compilation mode, engine, host calls, granted I/O, and CPython-on-Wasm dominate; benchmark the exact component and campaign against a preregistered ceiling. |
| Security boundary | None. The current subprocess harness is not authorized for hostile foreign code. | **Unclaimed by default.** Hermit explicitly says it is not a security boundary, and Linux says seccomp filtering is a tool for sandbox builders, not a sandbox itself ([Hermit warning](https://github.com/facebookexperimental/hermit/blob/e407311a3c1ab1da7d41220696929dc8235ec925/README.md#hermit), [Linux seccomp documentation](https://www.kernel.org/doc/html/latest/userspace-api/seccomp_filter.html#what-it-isn-t)). A separate containment profile and audit are required. | Wasmtime explicitly treats WebAssembly execution as a sandbox: no raw I/O/syscalls, bounds-checked linear memory, typed control flow, and imports/exports as the only outside interface ([Wasmtime security model](https://docs.wasmtime.dev/security.html)). The engine, embedder, granted imports, resource limits, and host functions remain trusted boundary code. |

## Selection rule

Choose the envelope by the claim's subject and required strength, in this order:

1. **Name the artifact.** If the claim must apply to the existing native CPython/Dharma path, `CapabilityClosed` is ineligible as primary evidence because it changes the executable subject. If a new distilled residue is the subject, it is eligible.
2. **Name the verdict ceiling.** If admission-grade `PROCEED` is required, `CooperativeD2` is ineligible while any property-relevant channel is open. It remains valuable for mapping, counterexamples, and honest `UNKNOWN`.
3. **Require compatibility before capability work.** `NativeInterposed` is selectable only if the exact mandatory workflow fits the declared syscall/process/thread profile on two named hosts. `CapabilityClosed` is selectable only if the exact mandatory workflow compiles, instantiates, and runs using a closed, versioned import/grant manifest.
4. **Separate determinism from security.** A native supervisor must carry a distinct `security_boundary = Unclaimed | Audited(profile)` field. A capability-closed guest must carry a distinct `determinism_grade`; the Wasm sandbox cannot mint D1 by itself.
5. **Bind cost before execution.** Every non-bootstrap selection must preregister a timebox, mandatory-workflow pass threshold, replay count, host count, and p95 slowdown ceiling. An omitted threshold is a selection failure, not freedom to rationalize later.

For the first campaign, this yields:

```text
native Dharma promotion path
  -> CooperativeD2: MAP + baseline + counterexamples; PROCEED ceiling = forbidden
  -> NativeInterposed: primary exact-artifact admission experiment

distilled PromotionPermit residue
  -> CapabilityClosed: primary capability-confinement candidate
  -> proof court: separate proof claim over the residue, not inherited from WASI
```

## Measurable kill and revival falsifiers

A **kill** stops selecting an envelope for the named claim/profile. It does not erase useful evidence or pronounce the mechanism universally impossible. A **revival** requires the previously failed variable to change and the preserved probe to cross the same preregistered threshold. New code, a new maintainer, or a more persuasive narrative is not a revival event.

| Envelope | Kill condition | Revival falsifier |
|---|---|---|
| `CooperativeD2` | Kill it as an **admission lane** immediately if any mandatory consequential property can be affected through an `Open` channel, if a declared leak probe can return false `CLEAN`/`PROCEED`, or if anyone proposes translating run-twice agreement into D1. At current `d19ba9e`, 29/29 open means the first admission campaign must end `HALT` on a directly replayed blocking violation or `UNKNOWN`, never `PROCEED`. | There is no relabel-based revival. The same native subject must be freshly executed under a new `NativeInterposed` envelope/controller-set identity. A `CapabilityClosed` run necessarily has a new artifact identity and can only be related through an explicit correspondence claim. `CooperativeD2` itself may be reused for a newly declared diagnostic claim after its leak battery passes; that does not lift old evidence. |
| `NativeInterposed` | Treat the **unratified** C7 packet as a candidate falsifier, not inherited law: it proposes a 10-working-day stop unless (a) 100% of mandatory workflow tests pass; (b) every reachable channel in the versioned inventory is controller-proved denied/virtualized/replayed; (c) unsupported effects are synchronously rejected before side effects; (d) the leak battery and complete effect-tape consumption pass; (e) an independent unsafe/protocol audit passes; and (f) **100/100** runs on each of two clean named hosts produce byte-identical world identity, effect tape, and complete observation ([C7 proposal and stop rule](https://github.com/AmitabhainArunachala/vibe-halt/blob/d19ba9e1198c0ab7ef4bb1bdf69a299d056f9754/docs/audits/C7_SUPERVISOR_ADMISSION_PACKET_2026-07-25.md#L1-L14)). Also propose killing when measured p95 slowdown exceeds the frozen campaign ceiling. Issues #113 and #117 must ratify, amend, or reject these thresholds before use. | Revive only when the receipt names the failed rung and a specific mechanism/host capability changed—for example, the previously unsupported syscall is now synchronously denied, or the named kernel/PMU limitation is removed—and the **same preserved workflow and leak probe** passes every ratified threshold within a new fixed timebox. Narrowing to one thread or removing a dependency creates a new profile; it cannot repair evidence for the killed broader profile. |
| `CapabilityClosed` | Within a preregistered feasibility timebox, kill it for the named target if any mandatory workflow cannot compile/instantiate/run, any required dependency demands an unavailable API, any undeclared import or ambient host escape exists, the import/grant graph cannot be exhaustively hashed, or p95 slowdown exceeds the committed ceiling. For D1, additionally kill unless two-host 100/100 replay identity holds with controlled clocks, RNG, scheduling, files, and network—not Wasmtime's host-backed defaults. If correspondence to native is claimed, kill that correspondence if any mandatory property differs in the preregistered native-vs-Wasm differential suite. | Revive only when the exact missing primitive changes (named CPython/WASI/engine version, dependency port, or host adapter), or when a newly distilled residue no longer requires it, then rebuild and rerun every compatibility, capability, determinism, and differential gate. The rebuilt WASM digest is a new `ArtifactId`; success cannot back-promote an old native result. |
| Cross-envelope policy | Kill the constitution or evaluator if it accepts `Evidence<A, E1, C>` where `Evidence<A, E2, C>` is required, chooses the maximum grade across envelopes, treats a shared source revision as artifact equality, or omits the envelope/controller set from a signature. One fixture that obtains `PROCEED` by swapping only the envelope tag is a zero-tolerance failure. | Revival requires a regression fixture proving the cast is unconstructible plus an explicit `Corroboration<E1,E2>` or `BehavioralEquivalence<Artifact1,Artifact2>` object that retains both complete identities and its own witness. Even successful equivalence does not merge the evidence objects or transfer security/determinism grades. |

## Minimal typed evidence identity

A single revision string is not enough. Observation and admission must form an
acyclic graph so a runner never signs the verdict derived from its own run:

```text
ObservationClosureId<Role> = (
  SubjectId,
  EnvelopeId,
  ControllerSetId,
  CampaignId,
  RoleObservationSetId,
)

ObservationAttestation<Role> = Sign<Role>(ObservationClosureId<Role>)

EvidenceClosureId = (
  OrderedObservationClosureIds,
  OrderedObservationAttestations,
  RequiredStatementAndBlobManifest,
)

AdmissionPayloadId = (
  EvidenceClosureId,
  PolicyId,
  Action,
  Assessment<Action>,
  GovernabilityProjectionPayloadId,
  GovernabilityGateDecision<Action, Scope, Policy>,
  AdmissionQuorumPolicyId,
)

JudgeAttestation<Judge> = Sign<Judge>(AdmissionPayloadId)

AdmissionRecordId = (
  AdmissionPayloadId,
  OrderedJudgeAttestations,
)

AdmissionQuorumWitness<Policy> = VerifyQuorum(AdmissionRecordId, Policy)

VerifyGovernabilityGate(AdmissionRecordId, Policy) ->
  Result<GovernabilityGateWitness<Action, Scope, Policy>, GateUnsatisfied>
```

with the following required content:

```text
SubjectId {
  source_revision,
  artifact_kind: Native | WasmModule | WasmComponent,
  artifact_sha256,
  materialization_receipt:
    Bound(MaterializationReceiptId) | Unresolved(reason),
  build_provenance: Bound(digest) | Unresolved(reason),
  interpreter_or_loader_closure:
    NotApplicable | Bound(digest) | Unresolved(reason),
}

EnvelopeId =
  CooperativeD2 {
    runner_sha256, host_profile_digest
  }
| NativeInterposed {
    supervisor_sha256, protocol_digest, policy_digest,
    kernel_abi, cpu_profile_digest, host_profile_digest,
    security_boundary
  }
| CapabilityClosed {
    engine_sha256, compiler_toolchain_digest,
    wasi_version, interface_abi_digest, import_set_digest,
    grants_digest,
    host_adapter_digest, host_profile_digest,
    security_boundary
  }

ControllerSetId {
  capability_schema,
  ordered_channel_dispositions: [
    Open(reason)
    | Denied(witness_digest)
    | Virtualized(witness_digest)
    | Replayed(witness_digest)
    | Absent(boundary_witness_digest)
  ],
  determinism_grade,
}

CampaignSpecId {
  manifest_digest,
  property_oracle_contract_digest,
  fault_palette_digest,
  seed_domain_policy_digest,
  input_cassette_digest,
  effect_tape_schema_and_controller_policy_digest,
  budget_and_thresholds_digest,
  required_evidence_contract_and_schema_digest,
}

HoldoutCommitmentSetId {
  ordered_commitment_statement_digests_bound_to_campaign_spec,
}

CampaignId = H(CampaignSpecId, HoldoutCommitmentSetId)

RoleObservationSetId {
  scope_digest,
  per_observation_modality_and_authority,
  produced_effect_tape_digest,
  role_owned_observation_or_replay_digest,
}

AttestationId<Role> {
  statement_payload_sha256,
  payload_type,
  signer_role: Role,
  signature_scheme,
  key_id,
  signature,
}
```

Each runner or independent replayer signs only its own
`ObservationClosureId<Role>`. `EvidenceClosureId` later binds those closures,
their role attestations, and the required blob manifest. The ratified admission
judges verify that evidence closure, apply `PolicyId`, and each sign identical
`AdmissionPayloadId` bytes. `AdmissionRecordId` is constructed only afterward
from that payload plus the ordered attestations; verifying it under the frozen
quorum policy derives `AdmissionQuorumWitness<Policy>`. No signature is included
inside the payload it signs. `CampaignSpecId` is frozen first without a
holdout commitment. Every holdout commitment binds that spec; only then is
`CampaignId` derived from the spec plus ordered commitment statements. It binds
the frozen evidence contract, not future produced evidence. No observation
closure contains its own assessment or transparency proof; later graph nodes
point back to it.

`GovernabilityProjectionPayloadId` is a pure content identity over the map,
accounting, blind spots, and boundary witnesses; it contains no admission
signature or quorum witness. Its action- and policy-indexed gate decision is
inside the bytes judges sign, and only a completed `AdmissionRecordId` whose
decision satisfies policy can derive `GovernabilityGateWitness`.
`RequiredButUngoverned` remains a signed fail-closed admission and returns
`GateUnsatisfied`. The evidence closure's required manifest
must include every signed `MaterializationReceiptId` referenced by a subject.
A filesystem pathname is not part of review authority.

Every apparent optional fact is encoded as `NotApplicable` or `Unresolved(reason)`, never omitted. In particular, an unknown native loader closure remains a typed open boundary rather than disappearing from identity.

This follows the useful minimum of the in-toto Statement model: an attestation binds immutable subjects by digest and identifies the predicate type ([in-toto Statement v1](https://github.com/in-toto/attestation/blob/051624ce466deaed4c5a66e66877f69b471fccbe/spec/v1/statement.md)). The outer signature must authenticate both payload bytes and payload type; DSSE exists specifically to prevent type-confusion and verify-before-parse failures ([DSSE rationale](https://github.com/secure-systems-lab/dsse/blob/1d3370f62565bca041e97c8310b873ac340edc2e/background.md), [in-toto envelope requirements](https://github.com/in-toto/attestation/blob/051624ce466deaed4c5a66e66877f69b471fccbe/spec/v1/envelope.md)). Key authorization and revocation remain external authority policy; a valid signature attributes a claim but cannot lift its modality.

At the type level, no API should exist for:

```text
Evidence<A, CooperativeD2, C> -> Evidence<A, NativeInterposed, C>
Evidence<NativeArtifact, E, C> -> Evidence<WasmArtifact, E, C>
CapabilityClosed -> Deterministic
NativeInterposed -> SecurityBoundary
```

The only lawful cross-envelope constructor is relational:

```text
corroborate(
  Evidence<A1, E1, C1>,
  Evidence<A2, E2, C2>,
  DifferentialWitness
) -> Corroboration<(A1,E1,C1), (A2,E2,C2)>
```

That object may strengthen confidence in a named behavioral proposition. It may not rewrite either subject, envelope, controller ledger, grade, verdict, modality, or authority.

## Constitutional decision to carry forward

Adopt the three-envelope architecture with these fixed roles:

> `NativeInterposed` shakes the existing forest. `CapabilityClosed` grows and confines the residue. `CooperativeD2` reaches the real target early while refusing to impersonate proof. Evidence belongs forever to the exact artifact and envelope that produced it.

This is not a compromise between ptrace and WASI. They answer different identity questions. The failure mode is not choosing the “wrong” one; it is allowing evidence about one subject under one trust shape to authorize consequence for another.
