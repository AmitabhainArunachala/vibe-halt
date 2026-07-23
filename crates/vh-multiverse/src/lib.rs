//! vh-multiverse — runs workloads across universes and detects divergence.
//!
//! CI gate #1 lives here: with `check_divergence` on, every universe is run
//! TWICE — in two non-adjacent passes — and the complete observable results
//! — including raw end state, transcript, lifecycle/runtime evidence,
//! schedule policy, plan, and tape identities — must match exactly. A
//! mismatch means nondeterminism leaked into the kernel or the workload,
//! and the report says so loudly instead of pretending the run was
//! reproducible. Agreement is the SAMPLED-FALSIFIER evidence class
//! ([`ReplayEvidence::PairwiseReplayAgreement`]): it can refute
//! determinism, never prove it — the deterministic-substrate claim rests
//! on the reviewed D0 boundary, not on this sample. Gate 0 combines semantic
//! manifest/lint enforcement with a regex defense; generic/type-erased
//! address formatting remains explicitly UNCHECKED until a type-aware gate
//! lands, so the regex layer is not described as proof by construction
//! (hardening-loop-4 BLOCKER 2).
//!
//! Evidence integrity: workloads interact with the evidence ledger ONLY
//! through capability methods (`record`, `always`, `sometimes`, ...) —
//! the trace, properties, clock, and identity fields are private, so a
//! safe-Rust workload cannot erase, replace, or re-attribute evidence
//! (PR #1 hardening-loop BLOCKER). The same applies downstream: every
//! `UniverseResult` and `MultiverseReport` evidence field is private with
//! read-only getters and internal construction, so safe downstream code
//! cannot forge an empty divergence-checked report or flip its flags
//! (hardening-loop-2 BLOCKER; the privacy is enforced by rustc — the
//! pre-repair forgery repro no longer compiles). This is an API guarantee
//! for safe code in-process, not a sandbox: untrusted code belongs in
//! Tier-2 subprocess universes, never linked into the runner.

#![forbid(unsafe_code)]

pub mod evidence;
pub mod observation;
pub mod runtime;

use std::collections::BTreeMap;
use std::fmt;
use std::num::NonZeroU64;

use vh_core::{SeedTree, VirtualClock, VirtualTime, Xoshiro256pp};
use vh_gremlin::{FaultPalette, FaultPlan};
use vh_props::{AlwaysCheck, AlwaysFailure, MergedProperties, Properties};
use vh_trace::Trace;

pub use evidence::{InjectionOutcome, RuntimeEvidence};
pub use observation::{CompleteObservationIdentity, EndStateIdentity};
pub use runtime::{DeliveryNote, DiskError, NodeId, SimRuntime, StepEvent};
pub use vh_props::{EndState, EndStateOracle};

/// Everything a workload may touch inside one universe. All randomness comes
/// from named streams; all time comes from the virtual clock; all observable
/// behavior goes into the trace — through capability methods only.
pub struct UniverseCtx {
    universe_id: u64,
    seed_tree: SeedTree,
    // pub(crate): the sim runtime (`runtime.rs`) records deliveries, IO,
    // and lifecycle transitions itself. Crate-internal visibility keeps
    // the evidence-integrity guarantee against DOWNSTREAM safe code
    // intact — external workloads still reach these only through
    // capability methods.
    pub(crate) clock: VirtualClock,
    pub(crate) trace: Trace,
    pub(crate) props: Properties,
    fault_plan_override: Option<FaultPlan>,
    /// Runner-owned ledger: how many times the workload RETRIEVED its
    /// fault plan through [`UniverseCtx::fault_plan_or`]. Finalized into
    /// [`FaultPlanDiscipline`] — a workload cannot edit it
    /// (hardening-loop-2 BLOCKER). Retrieval is all this ledger can
    /// truthfully claim; see [`FaultPlanDiscipline`] for what it does
    /// NOT claim (hardening-loop-4 GAP 5).
    fault_plan_retrievals: u64,
    /// Runner-owned canonical digest ledger of every retrieved plan, in
    /// retrieval order, under [`FAULT_PLAN_DIGEST_SCHEMA`]. Bound into
    /// [`UniverseResult`] so replay evidence carries its input identity.
    plan_digest_trace: Trace,
    /// Runner-owned semantic fault-lifecycle ledger, finalized by the
    /// Phase-1 sim runtime ([`UniverseCtx::runtime`]); `None` for
    /// universes that never constructed the runtime.
    pub(crate) runtime_evidence: Option<RuntimeEvidence>,
    /// Digest of the runtime's decision tape (`vh-decision-tape-v1` —
    /// a SEPARATE additive stream; never enters the frozen execution
    /// trace or its hash), finalized by the sim runtime alongside the
    /// evidence ledger; `None` for universes that never constructed the
    /// runtime (convergence C1, Track-2 W2 / RFC-003).
    pub(crate) decision_tape_digest: Option<String>,
    /// Same-timestamp schedule policy the sim runtime pops under
    /// (convergence C2). Fifo unless an exploratory entry point set it.
    pub(crate) schedule_policy: SchedulePolicy,
    /// Whether the sim runtime records its scheduler decision tape.
    /// OFF by default: the tape's per-pop candidate digest costs ~50%
    /// wall at the 200-universe runtime demo (convergence C1 kill
    /// criterion fired; numbers in the C1 receipt), so recording is
    /// opt-in until a cheaper substrate lands.
    pub(crate) record_tape: bool,
    /// The workload's declared end state, judged post-run by the
    /// workload's [`EndStateOracle`]s. Oracles read state and never
    /// record trace events; their verdicts enter the always transcript
    /// as `oracle:<name>` entries.
    pub(crate) end_state: EndState,
    fault_palette: FaultPalette,
}

impl UniverseCtx {
    fn new(
        root_seed: u64,
        universe_id: u64,
        fault_plan_override: Option<FaultPlan>,
        fault_palette: FaultPalette,
    ) -> Self {
        let mut plan_digest_trace = Trace::new();
        plan_digest_trace.record(0, "schema", FAULT_PLAN_DIGEST_SCHEMA);
        Self {
            universe_id,
            seed_tree: SeedTree::new(root_seed),
            clock: VirtualClock::new(),
            trace: Trace::new(),
            props: Properties::new(),
            fault_plan_override,
            fault_plan_retrievals: 0,
            plan_digest_trace,
            runtime_evidence: None,
            decision_tape_digest: None,
            schedule_policy: SchedulePolicy::Fifo,
            record_tape: false,
            end_state: EndState::new(),
            fault_palette,
        }
    }

    /// This universe's identity (read-only).
    pub fn universe_id(&self) -> u64 {
        self.universe_id
    }

    /// Current virtual time in nanos (read-only).
    pub fn now_nanos(&self) -> u64 {
        self.clock.now().nanos()
    }

    /// The fault plan for this universe: the externally supplied override
    /// (shrinker/replay path via [`run_universe_with_fault_plan`]) if one
    /// exists, else the plan the workload generates itself. Workloads MUST
    /// route their plan through this so a shrunk plan replays through the
    /// exact same code path as the original.
    ///
    /// The generator closure is ALWAYS evaluated — override present or not
    /// — so its effects (PRNG stream draws) happen identically on both
    /// paths. Before this, an override skipped generation, and any workload
    /// whose generator shared a stream with later draws consumed a
    /// different number of words under replay, contradicting the
    /// identical-path claim (hardening-loop-2 BLOCKER). Each RETRIEVAL is
    /// counted in a runner-owned ledger and the retrieved plan's canonical
    /// digest is bound into the universe's evidence; see
    /// [`FaultPlanDiscipline`] and [`UniverseResult::fault_plan_digest`].
    pub fn fault_plan_or(&mut self, generate: impl FnOnce() -> FaultPlan) -> FaultPlan {
        let generated = generate();
        self.fault_plan_retrievals += 1;
        let effective = match &self.fault_plan_override {
            Some(plan) => plan.clone(),
            None => generated,
        };
        self.plan_digest_trace
            .record(self.fault_plan_retrievals, "retrieval", "");
        for inj in effective.injections() {
            self.plan_digest_trace
                .record(inj.at_nanos, "injection", &inj.fault.canonical());
        }
        effective
    }

    /// A named, independent PRNG stream for this universe.
    pub fn stream(&self, name: &str) -> Xoshiro256pp {
        self.seed_tree.stream(self.universe_id, name)
    }

    /// Deterministic per-universe seed. Palette masks and future schedule
    /// choice streams derive from this without perturbing existing named
    /// streams.
    pub fn universe_seed(&self) -> u64 {
        self.seed_tree.universe_seed(self.universe_id)
    }

    /// Active fault-plan palette for this universe. v0 remains the default.
    pub fn fault_palette(&self) -> FaultPalette {
        self.fault_palette
    }

    /// Record a trace event stamped with the current virtual time.
    pub fn record(&mut self, kind: &str, data: &str) {
        let at = self.clock.now().nanos();
        self.trace.record(at, kind, data);
    }

    /// Advance virtual time (monotonic; panics on backwards time).
    pub fn advance_to(&mut self, nanos: u64) {
        self.clock.advance_to(VirtualTime(nanos));
    }

    /// Check an invariant; every evaluation enters the assertion transcript.
    pub fn always<F: FnOnce() -> String>(&mut self, name: &str, condition: bool, detail: F) {
        self.props.always(name, condition, detail);
    }

    /// Declare a sometimes-assertion up front (unreached ⇒ finding).
    pub fn declare_sometimes(&mut self, name: &str) {
        self.props.declare_sometimes(name);
    }

    /// Mark a sometimes-assertion as reached in this universe. Panics if
    /// the name was never declared (fail-closed declaration discipline).
    pub fn sometimes(&mut self, name: &str) {
        self.props.sometimes(name);
    }

    /// Declare one end-state fact for post-run oracle judgment. Last
    /// write per key wins (deterministic; keys iterate in BTreeMap
    /// order). Declaring end state records NO trace event — oracles are
    /// read-only over state, so re-expressing an inline check as an
    /// oracle leaves the frozen trace identity untouched.
    pub fn declare_end(&mut self, key: &str, value: &str) {
        self.end_state.insert(key.to_string(), value.to_string());
    }
}

/// Typed completion outcome a workload must return (hardening-loop-2
/// BLOCKER): a report can only be CLEAN when every universe's workload
/// AFFIRMATIVELY completed. Before this, `run` returned nothing, so a
/// workload that silently did no work reached CLEAN through an empty
/// finding ledger.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RunOutcome {
    /// The workload ran to its intended end; its evidence is a valid basis
    /// for a verdict.
    Completed,
    /// A precondition of the workload did not hold; the universe proves
    /// nothing about the property space. Never CLEAN.
    InvalidAssumption(String),
    /// The workload hit an error path it could not absorb. Never CLEAN.
    ExecutionError(String),
}

/// Versioned schema of the canonical fault-plan digest carried by
/// [`UniverseResult::fault_plan_digest`]: the frozen trace hasher over a
/// schema record plus, per retrieval, a `retrieval` marker and one
/// `injection` record per canonical injection
/// ([`vh_gremlin::FaultKind::canonical`]). Changing the rendering is a
/// schema bump (v2), never a refactor.
pub const FAULT_PLAN_DIGEST_SCHEMA: &str = "vh-fault-plan-v1";

/// Runner-derived fault-plan RETRIEVAL discipline for one universe.
/// Produced from the private retrieval ledger after the workload
/// returns; a workload cannot construct or edit it.
///
/// Truthful scope (hardening-loop-4 GAP 5): this ledger records that the
/// workload RETRIEVED a plan through [`UniverseCtx::fault_plan_or`] —
/// nothing more. It does NOT claim the plan was applied, that any
/// injection became eligible, or that a fault manifested; a workload can
/// retrieve a plan and discard it while still reporting `Completed`. The
/// semantic lifecycle (offered → retrieved → armed → manifested →
/// effect-observed) requires the runtime to own fault scheduling, which
/// lands with the Phase-1 sim runtime — until then the names here
/// promise only what is measured. DEFERRED: owner
/// vibe-halt-core-2026-07 (Claude), due with Phase 1 (2026-08-15).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FaultPlanDiscipline {
    /// No override was supplied; the workload retrieved a self-generated
    /// plan `retrievals` times through [`UniverseCtx::fault_plan_or`]
    /// (zero is legal — not every workload uses fault plans).
    SelfGenerated { retrievals: u64 },
    /// An override was supplied and retrieved exactly once: the replay
    /// input reached the workload (whether it was honored is a
    /// divergence-detector question, not a ledger claim).
    OverrideRetrieved,
    /// An override was supplied but never retrieved — the replacement
    /// plan cannot have influenced the run, so any replay claim would be
    /// false. Fails closed: never a valid completion.
    OverrideNeverRetrieved,
    /// An override was supplied and retrieved more than once — ambiguous
    /// replay. Fails closed: never a valid completion.
    OverrideRetrievedMultiply { retrievals: u64 },
}

/// Same-timestamp schedule policy for the sim runtime's pop site
/// (convergence C2, Track-2 W3). `Fifo` is the frozen v0 default —
/// bit-for-bit the original pop. The exploratory strategies are OPT-IN
/// and deterministic per `(root seed, universe)`; their choices are
/// witnessed by the decision tape when recording is on.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SchedulePolicy {
    Fifo,
    /// PCT with `depth` change points (Burckhardt 2010; Shuttle shapes,
    /// reimplemented dependency-free in `vh_core::strategy`).
    Pct {
        depth: u64,
    },
    /// Uniform-with-random-tiebreak: the null hypothesis PCT must beat
    /// (the C2 kill criterion's comparator).
    UniformTiebreak,
}

/// Runner-owned lifecycle evidence for one universe: the workload's typed
/// outcome plus the fault-plan discipline. Part of the observable result
/// and its equality (Tier-1 identity).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UniverseLifecycle {
    outcome: RunOutcome,
    fault_plan: FaultPlanDiscipline,
}

impl UniverseLifecycle {
    /// The workload's typed completion outcome.
    pub fn outcome(&self) -> &RunOutcome {
        &self.outcome
    }

    /// The runner-derived fault-plan discipline.
    pub fn fault_plan(&self) -> &FaultPlanDiscipline {
        &self.fault_plan
    }

    /// A universe is validly complete iff the workload affirmatively
    /// completed AND the fault-plan retrieval discipline held. CLEAN
    /// requires this for every universe.
    pub fn is_valid_completion(&self) -> bool {
        self.outcome == RunOutcome::Completed
            && matches!(
                self.fault_plan,
                FaultPlanDiscipline::SelfGenerated { .. } | FaultPlanDiscipline::OverrideRetrieved
            )
    }
}

/// Runner-owned property contract (hardening-loop-4 GAP 5): what a
/// workload COMMITS to asserting in every universe. Before this, a no-op
/// workload could return `Completed` with no properties and reach CLEAN
/// through an empty finding ledger. The runner verifies the contract
/// against each universe's transcript; an EMPTY contract means the
/// campaign asserted nothing and can never be CLEAN (it is UNCHECKED,
/// the honest tri-state).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct PropertyContract {
    required_always: Vec<String>,
    required_sometimes: Vec<String>,
    required_oracles: Vec<String>,
}

impl PropertyContract {
    pub fn new(required_always: &[&str], required_sometimes: &[&str]) -> Self {
        Self {
            required_always: required_always.iter().map(|s| s.to_string()).collect(),
            required_sometimes: required_sometimes.iter().map(|s| s.to_string()).collect(),
            required_oracles: Vec::new(),
        }
    }

    /// Require named [`EndStateOracle`]s to have been evaluated in every
    /// universe (their verdicts appear as `oracle:<name>` transcript
    /// entries). A workload whose oracle list omits a required name is a
    /// contract violation — belt and suspenders against the two lists
    /// drifting apart.
    pub fn with_oracles(mut self, names: &[&str]) -> Self {
        self.required_oracles = names.iter().map(|s| s.to_string()).collect();
        self
    }

    /// End-state oracle names that must be judged in every universe.
    pub fn required_oracles(&self) -> &[String] {
        &self.required_oracles
    }

    /// Always-invariant names that must be evaluated at least once in
    /// every universe.
    pub fn required_always(&self) -> &[String] {
        &self.required_always
    }

    /// Sometimes-property names that must be declared in every universe.
    pub fn required_sometimes(&self) -> &[String] {
        &self.required_sometimes
    }

    pub fn is_empty(&self) -> bool {
        self.required_always.is_empty()
            && self.required_sometimes.is_empty()
            && self.required_oracles.is_empty()
    }

    /// Contract violations for one universe's transcript. Runner-owned:
    /// evaluated against the immutable result, never workload state.
    pub fn violations(&self, result: &UniverseResult) -> Vec<String> {
        let mut out = Vec::new();
        for name in &self.required_always {
            if !result.always_checks().iter().any(|c| &c.name == name) {
                out.push(format!(
                    "required always property '{name}' was never evaluated"
                ));
            }
        }
        for name in &self.required_sometimes {
            if !result.sometimes().contains_key(name) {
                out.push(format!(
                    "required sometimes property '{name}' was never declared"
                ));
            }
        }
        for name in &self.required_oracles {
            let entry = format!("oracle:{name}");
            if !result.always_checks().iter().any(|c| c.name == entry) {
                out.push(format!(
                    "required end-state oracle '{name}' was never judged"
                ));
            }
        }
        out
    }
}

pub trait Workload {
    fn name(&self) -> &str;
    fn run(&self, ctx: &mut UniverseCtx) -> RunOutcome;

    /// The properties this workload commits to asserting in every
    /// universe. Default is the EMPTY contract, which fails closed: a
    /// campaign that asserts nothing is UNCHECKED, never CLEAN
    /// (hardening-loop-4 GAP 5).
    fn property_contract(&self) -> PropertyContract {
        PropertyContract::default()
    }

    /// Typed post-run assertions over the declared end state. The runner
    /// judges each oracle ONCE per universe after `run` returns and
    /// records the verdict as an `oracle:<name>` entry in the always
    /// transcript (in list order, after every inline check). Oracles
    /// read state only — they cannot record trace events, so the frozen
    /// trace identity of a re-expressed check is untouched by
    /// construction.
    fn end_state_oracles(&self) -> Vec<EndStateOracle> {
        Vec::new()
    }
}

/// The complete public observation of one universe execution. All fields
/// are private with read-only getters; construction is internal to the
/// runner, so safe downstream code cannot forge or edit evidence
/// (hardening-loop-2 BLOCKER). Tier-1 identity is the WHOLE struct — see
/// `observably_equal`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UniverseResult {
    universe_id: u64,
    trace_hash: String,
    trace_events: usize,
    always_checks: Vec<AlwaysCheck>,
    always_failures: Vec<AlwaysFailure>,
    sometimes: BTreeMap<String, bool>,
    lifecycle: UniverseLifecycle,
    fault_plan_digest: Option<String>,
    runtime_evidence: Option<RuntimeEvidence>,
    schedule_policy: SchedulePolicy,
    decision_tape_digest: Option<String>,
    end_state_identity: EndStateIdentity,
    complete_observation_identity: CompleteObservationIdentity,
}

impl UniverseResult {
    /// This universe's identity.
    pub fn universe_id(&self) -> u64 {
        self.universe_id
    }

    /// Chain hash of the trace (docs/specs/TRACE_FORMAT_V0.md).
    pub fn trace_hash(&self) -> &str {
        &self.trace_hash
    }

    /// Number of trace events.
    pub fn trace_events(&self) -> usize {
        self.trace_events
    }

    /// Full assertion transcript in invocation order — passing checks
    /// included, so a replay that skips a passing invariant is observably
    /// different (PR #1 hardening-loop BLOCKER).
    pub fn always_checks(&self) -> &[AlwaysCheck] {
        &self.always_checks
    }

    /// Ordered always-failures with details.
    pub fn always_failures(&self) -> &[AlwaysFailure] {
        &self.always_failures
    }

    /// Declared sometimes properties and their reached state.
    pub fn sometimes(&self) -> &BTreeMap<String, bool> {
        &self.sometimes
    }

    /// Runner-owned lifecycle evidence (typed outcome + fault-plan
    /// retrieval discipline).
    pub fn lifecycle(&self) -> &UniverseLifecycle {
        &self.lifecycle
    }

    /// Canonical digest of every fault plan the workload retrieved, in
    /// retrieval order, under [`FAULT_PLAN_DIGEST_SCHEMA`] — the replay
    /// input's identity, bound into the observable result
    /// (hardening-loop-4 GAP 5). `None` iff no plan was ever retrieved.
    pub fn fault_plan_digest(&self) -> Option<&str> {
        self.fault_plan_digest.as_deref()
    }

    /// Runner-owned semantic fault-lifecycle evidence from the Phase-1
    /// sim runtime (Offered → Armed → Injected → Manifested → Recovered
    /// per injection; see `evidence.rs` for the measured-stage doctrine).
    /// `None` iff this universe never constructed the runtime — the
    /// legacy workload-drained path, whose lifecycle claims remain
    /// retrieval-only. In observable equality.
    pub fn runtime_evidence(&self) -> Option<&RuntimeEvidence> {
        self.runtime_evidence.as_ref()
    }

    /// Same-timestamp scheduler policy used for this universe. Policy is an
    /// observable replay input even when no choice point was reached.
    pub fn schedule_policy(&self) -> SchedulePolicy {
        self.schedule_policy
    }

    /// Digest of the sim runtime's scheduler decision tape
    /// (`vh-decision-tape-v1`) — the replay/causality substrate's
    /// identity, bound into the observable result as a SEPARATE stream
    /// so the frozen execution-trace hash is untouched. `None` iff this
    /// universe never constructed the runtime. In observable equality.
    pub fn decision_tape_digest(&self) -> Option<&str> {
        self.decision_tape_digest.as_deref()
    }

    /// Versioned canonical bytes for the exact raw map consumed by end-state
    /// oracles. This closes the false-negative where two different states
    /// produced the same passing oracle transcript.
    pub fn end_state_identity(&self) -> &EndStateIdentity {
        &self.end_state_identity
    }

    /// Versioned canonical bytes over every public universe observable.
    /// This is an injective representation, not a second digest.
    pub fn complete_observation_identity(&self) -> &CompleteObservationIdentity {
        &self.complete_observation_identity
    }

    /// Two replays are the same run iff EVERY observable agrees. Struct
    /// equality is the definition on purpose: adding an observable field
    /// automatically strengthens the divergence check.
    pub fn observably_equal(&self, other: &UniverseResult) -> bool {
        self == other
    }

    /// Borrowed, read-only view of the COMPLETE observation — the
    /// compile-time schema ratchet requested by the verifier track
    /// (PR #2 interface request 5021566209). The implementation
    /// destructures `UniverseResult` WITHOUT `..`, so adding a private
    /// result field fails compilation here until the observation
    /// doctrine decides how it is exposed; downstream verifiers
    /// destructure this view without `..` to inherit the same ratchet.
    /// Plain borrowed data only: no mutation path and no Track-1-owned
    /// canonical hash, so independent downstream framings stay
    /// independent. `observably_equal` remains whole-struct equality of
    /// `UniverseResult`, never equality of this view.
    pub fn observation(&self) -> UniverseObservation<'_> {
        let UniverseResult {
            universe_id,
            trace_hash,
            trace_events,
            always_checks,
            always_failures,
            sometimes,
            lifecycle,
            fault_plan_digest,
            runtime_evidence,
            schedule_policy,
            decision_tape_digest,
            end_state_identity,
            complete_observation_identity,
        } = self;
        UniverseObservation {
            universe_id: *universe_id,
            trace_hash,
            trace_events: *trace_events,
            always_checks,
            always_failures,
            sometimes,
            lifecycle,
            fault_plan_digest: fault_plan_digest.as_deref(),
            runtime_evidence: runtime_evidence.as_ref(),
            schedule_policy: *schedule_policy,
            decision_tape_digest: decision_tape_digest.as_deref(),
            end_state_identity,
            complete_observation_identity,
        }
    }
}

/// The complete public observation of a [`UniverseResult`], as plain
/// borrowed data. See [`UniverseResult::observation`] for the ratchet
/// contract this view carries.
#[derive(Debug, Clone, Copy)]
pub struct UniverseObservation<'a> {
    pub universe_id: u64,
    pub trace_hash: &'a str,
    pub trace_events: usize,
    pub always_checks: &'a [AlwaysCheck],
    pub always_failures: &'a [AlwaysFailure],
    pub sometimes: &'a BTreeMap<String, bool>,
    pub lifecycle: &'a UniverseLifecycle,
    pub fault_plan_digest: Option<&'a str>,
    pub runtime_evidence: Option<&'a RuntimeEvidence>,
    pub schedule_policy: SchedulePolicy,
    pub decision_tape_digest: Option<&'a str>,
    pub end_state_identity: &'a EndStateIdentity,
    pub complete_observation_identity: &'a CompleteObservationIdentity,
}

pub fn run_universe(root_seed: u64, universe_id: u64, workload: &dyn Workload) -> UniverseResult {
    run_universe_with_palette(root_seed, universe_id, workload, FaultPalette::V0)
}

/// Run one universe with an explicit fault palette.
pub fn run_universe_with_palette(
    root_seed: u64,
    universe_id: u64,
    workload: &dyn Workload,
    fault_palette: FaultPalette,
) -> UniverseResult {
    run_universe_inner(root_seed, universe_id, workload, None, fault_palette)
}

/// Replay a universe with an externally supplied fault plan instead of the
/// workload-generated one. This is the shrinker's oracle surface: minimize
/// a failing plan by replaying candidate sub-plans through the identical
/// workload path.
///
/// Tier honesty: identical (seed, universe, workload, plan) inputs produce
/// identical observable results — Tier 1 — PROVIDED the workload draws all
/// nondeterminism from its `UniverseCtx` and its lifecycle reports
/// `OverrideRetrieved`. A result whose fault-plan discipline is
/// `OverrideNeverRetrieved` or `OverrideRetrievedMultiply` is not a valid
/// replay and never a valid completion; the divergence detector remains
/// the mechanical check for workload purity, and retrieval is all the
/// ledger claims (see [`FaultPlanDiscipline`]).
pub fn run_universe_with_fault_plan(
    root_seed: u64,
    universe_id: u64,
    workload: &dyn Workload,
    plan: FaultPlan,
) -> UniverseResult {
    run_universe_inner(
        root_seed,
        universe_id,
        workload,
        Some(plan),
        FaultPalette::V0,
    )
}

/// Run one universe with an explicit palette AND the decision-tape
/// recorder toggled (convergence C1). `record_tape: false` is
/// bit-identical to [`run_universe_with_palette`]; `true` additionally
/// binds the `vh-decision-tape-v1` digest into the observable result
/// for universes that construct the sim runtime. Opt-in because the C1
/// overhead kill criterion fired (>5% at the 200-universe runtime
/// demo); the frozen execution trace is untouched either way.
pub fn run_universe_recorded(
    root_seed: u64,
    universe_id: u64,
    workload: &dyn Workload,
    fault_palette: FaultPalette,
    record_tape: bool,
) -> UniverseResult {
    run_universe_inner_recorded(
        root_seed,
        universe_id,
        workload,
        None,
        fault_palette,
        record_tape,
    )
}

fn run_universe_inner(
    root_seed: u64,
    universe_id: u64,
    workload: &dyn Workload,
    fault_plan_override: Option<FaultPlan>,
    fault_palette: FaultPalette,
) -> UniverseResult {
    run_universe_inner_recorded(
        root_seed,
        universe_id,
        workload,
        fault_plan_override,
        fault_palette,
        false,
    )
}

/// [`run_universe_recorded`] with an explicit same-timestamp schedule
/// policy (convergence C2). `SchedulePolicy::Fifo` is bit-identical to
/// [`run_universe_recorded`]; exploratory policies are opt-in and
/// deterministic per `(root_seed, universe_id)`.
pub fn run_universe_scheduled(
    root_seed: u64,
    universe_id: u64,
    workload: &dyn Workload,
    fault_palette: FaultPalette,
    record_tape: bool,
    policy: SchedulePolicy,
) -> UniverseResult {
    run_universe_inner_scheduled(
        root_seed,
        universe_id,
        workload,
        None,
        fault_palette,
        record_tape,
        policy,
    )
}

fn run_universe_inner_recorded(
    root_seed: u64,
    universe_id: u64,
    workload: &dyn Workload,
    fault_plan_override: Option<FaultPlan>,
    fault_palette: FaultPalette,
    record_tape: bool,
) -> UniverseResult {
    run_universe_inner_scheduled(
        root_seed,
        universe_id,
        workload,
        fault_plan_override,
        fault_palette,
        record_tape,
        SchedulePolicy::Fifo,
    )
}

fn run_universe_inner_scheduled(
    root_seed: u64,
    universe_id: u64,
    workload: &dyn Workload,
    fault_plan_override: Option<FaultPlan>,
    fault_palette: FaultPalette,
    record_tape: bool,
    policy: SchedulePolicy,
) -> UniverseResult {
    let override_supplied = fault_plan_override.is_some();
    let mut ctx = UniverseCtx::new(root_seed, universe_id, fault_plan_override, fault_palette);
    ctx.record_tape = record_tape;
    ctx.schedule_policy = policy;
    let outcome = workload.run(&mut ctx);
    // Post-run oracle judgment: runner-owned, over the immutable declared
    // end state, one transcript entry per oracle in list order. Judged
    // unconditionally (an errored universe's oracle verdicts are evidence
    // too; invalid completions already bar CLEAN).
    for oracle in workload.end_state_oracles() {
        let name = format!("oracle:{}", oracle.name);
        match (oracle.check)(&ctx.end_state) {
            Ok(()) => ctx.props.always(&name, true, String::new),
            Err(detail) => ctx.props.always(&name, false, || detail.clone()),
        }
    }
    let fault_plan = match (override_supplied, ctx.fault_plan_retrievals) {
        (false, n) => FaultPlanDiscipline::SelfGenerated { retrievals: n },
        (true, 1) => FaultPlanDiscipline::OverrideRetrieved,
        (true, 0) => FaultPlanDiscipline::OverrideNeverRetrieved,
        (true, n) => FaultPlanDiscipline::OverrideRetrievedMultiply { retrievals: n },
    };
    let fault_plan_digest = if ctx.fault_plan_retrievals > 0 {
        Some(ctx.plan_digest_trace.hash_hex())
    } else {
        None
    };
    let universe_id = ctx.universe_id;
    let trace_hash = ctx.trace.hash_hex();
    let trace_events = ctx.trace.len();
    let always_checks = ctx.props.always_checks().to_vec();
    let always_failures = ctx.props.always_failures().to_vec();
    let sometimes = ctx.props.sometimes_map().clone();
    let lifecycle = UniverseLifecycle {
        outcome,
        fault_plan,
    };
    let runtime_evidence = ctx.runtime_evidence;
    let schedule_policy = ctx.schedule_policy;
    let decision_tape_digest = ctx.decision_tape_digest;
    let end_state_identity = EndStateIdentity::from_end_state(&ctx.end_state);
    let complete_observation_identity =
        observation::complete_identity(observation::ObservationSource {
            universe_id,
            trace_hash: &trace_hash,
            trace_events,
            always_checks: &always_checks,
            always_failures: &always_failures,
            sometimes: &sometimes,
            lifecycle: &lifecycle,
            fault_plan_digest: fault_plan_digest.as_deref(),
            runtime_evidence: runtime_evidence.as_ref(),
            schedule_policy,
            decision_tape_digest: decision_tape_digest.as_deref(),
            end_state_identity: &end_state_identity,
        });
    UniverseResult {
        universe_id,
        trace_hash,
        trace_events,
        always_checks,
        always_failures,
        sometimes,
        lifecycle,
        fault_plan_digest,
        runtime_evidence,
        schedule_policy,
        decision_tape_digest,
        end_state_identity,
        complete_observation_identity,
    }
}

/// Typed universe count: nonzero AND bounded. Zero work is never certified
/// (hardening-loop-1 BLOCKER); an absurd count is a typed configuration
/// error instead of an allocation abort — `--universes u64::MAX` used to
/// exit 101 through `Vec::with_capacity` (hardening-loop-2 GAP).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct UniverseCount(NonZeroU64);

impl UniverseCount {
    /// v0 resource bound: the sequential runner refuses campaigns beyond
    /// this rather than attempting them. Raising it is a resourcing
    /// decision, not a refactor.
    pub const MAX: u64 = 1 << 20;

    pub fn get(self) -> u64 {
        self.0.get()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MultiverseConfigError {
    /// Zero universes: an empty campaign can never be certified.
    ZeroUniverses,
    /// More universes than the v0 resource bound.
    TooManyUniverses { requested: u64, max: u64 },
}

impl fmt::Display for MultiverseConfigError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            MultiverseConfigError::ZeroUniverses => {
                write!(
                    f,
                    "--universes must be nonzero — zero work is never certified"
                )
            }
            MultiverseConfigError::TooManyUniverses { requested, max } => {
                write!(
                    f,
                    "--universes {requested} exceeds the v0 resource bound ({max}) — refusing the allocation instead of aborting"
                )
            }
        }
    }
}

impl std::error::Error for MultiverseConfigError {}

impl TryFrom<u64> for UniverseCount {
    type Error = MultiverseConfigError;

    fn try_from(n: u64) -> Result<Self, Self::Error> {
        match NonZeroU64::new(n) {
            None => Err(MultiverseConfigError::ZeroUniverses),
            Some(_) if n > Self::MAX => Err(MultiverseConfigError::TooManyUniverses {
                requested: n,
                max: Self::MAX,
            }),
            Some(nz) => Ok(Self(nz)),
        }
    }
}

#[derive(Debug, Clone)]
pub struct MultiverseConfig {
    pub root_seed: u64,
    /// Typed nonzero-and-bounded: an empty or absurd multiverse cannot be
    /// constructed (hardening loops 1 and 2).
    pub universes: UniverseCount,
    /// Run every universe twice and compare complete observable results.
    /// When false the report's verdict is UNCHECKED, never CLEAN.
    pub check_divergence: bool,
}

/// The report's tri-state verdict. `Unchecked` exists so that a run with
/// divergence detection disabled can never share the CLEAN verdict path.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Verdict {
    /// Divergence-checked, finding-free, every universe validly completed.
    Clean,
    /// At least one always-failure, divergence, unreached sometimes,
    /// invalid completion, or report-integrity violation.
    Findings,
    /// Finding-free but divergence detection was disabled: inconclusive.
    Unchecked,
}

/// Typed replay-evidence quality, orthogonal to finding status
/// (hardening-loop-2 GAP), named as the exact fact it is
/// (hardening-loop-4 BLOCKER 2): a finite replay sample is a
/// FALSIFIER of determinism, never a proof. The old name
/// (`Tier1DivergenceChecked`) promoted pairwise agreement into a tier
/// claim; the Tier-1 claim actually rests on the separately enforced D0
/// boundary (gate 0: deny-list + rustc lints; docs/specs/
/// DETERMINISM_TIERS.md), and this evidence only reports whether that
/// claim survived a sampled falsification attempt.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReplayEvidence {
    /// Every universe ran twice — in two NON-ADJACENT passes, see
    /// [`run_multiverse`] — and complete observations agreed pairwise.
    /// Agreement is evidence, not proof: a workload keyed to the
    /// execution schedule can agree with itself while being
    /// nondeterministic (regression-documented in
    /// `tests/divergence.rs`). Divergences, if any, are findings —
    /// sampled ≠ clean.
    PairwiseReplayAgreement,
    /// Single executions only: no replay agreement was even sampled in
    /// this run.
    SingleRunUnchecked,
}

impl ReplayEvidence {
    pub fn label(self) -> &'static str {
        match self {
            ReplayEvidence::PairwiseReplayAgreement => {
                "pairwise replay agreement (sampled falsifier — not proof; Tier-1 claim rests on the D0 boundary)"
            }
            ReplayEvidence::SingleRunUnchecked => {
                "single execution (no replay agreement — divergence check disabled)"
            }
        }
    }
}

/// Campaign evidence. All fields private with read-only getters and
/// internal construction: safe downstream code cannot build an empty
/// `divergence_checked` report or flip its flags, and `verdict()` only
/// ever reads runner-produced state (hardening-loop-2 BLOCKER).
#[derive(Debug, Clone)]
pub struct MultiverseReport {
    root_seed: u64,
    workload: String,
    divergence_checked: bool,
    universes_requested: u64,
    results: Vec<UniverseResult>,
    divergent_universes: Vec<u64>,
    merged: MergedProperties,
    contract: PropertyContract,
    contract_violations: Vec<(u64, String)>,
}

impl MultiverseReport {
    pub fn root_seed(&self) -> u64 {
        self.root_seed
    }

    pub fn workload(&self) -> &str {
        &self.workload
    }

    /// Whether every universe was replayed and compared.
    pub fn divergence_checked(&self) -> bool {
        self.divergence_checked
    }

    /// The campaign size that was requested — stored so the verdict can
    /// cross-check result cardinality instead of trusting `results` alone.
    pub fn universes_requested(&self) -> u64 {
        self.universes_requested
    }

    pub fn results(&self) -> &[UniverseResult] {
        &self.results
    }

    /// Universe ids whose two runs produced different observable results.
    pub fn divergent_universes(&self) -> &[u64] {
        &self.divergent_universes
    }

    pub fn merged(&self) -> &MergedProperties {
        &self.merged
    }

    /// Universe ids with at least one always-failure.
    pub fn failing_universes(&self) -> Vec<u64> {
        self.results
            .iter()
            .filter(|r| !r.always_failures.is_empty())
            .map(|r| r.universe_id)
            .collect()
    }

    /// Universe ids whose lifecycle is not a valid completion (workload
    /// outcome not `Completed`, or fault-plan discipline violated).
    pub fn invalid_universes(&self) -> Vec<u64> {
        self.results
            .iter()
            .filter(|r| !r.lifecycle.is_valid_completion())
            .map(|r| r.universe_id)
            .collect()
    }

    /// Typed replay-evidence quality (orthogonal to findings).
    pub fn replay_evidence(&self) -> ReplayEvidence {
        if self.divergence_checked {
            ReplayEvidence::PairwiseReplayAgreement
        } else {
            ReplayEvidence::SingleRunUnchecked
        }
    }

    /// The runner-verified property contract this campaign ran under.
    pub fn contract(&self) -> &PropertyContract {
        &self.contract
    }

    /// Per-universe contract violations (universe id, description).
    pub fn contract_violations(&self) -> &[(u64, String)] {
        &self.contract_violations
    }

    fn has_findings(&self) -> bool {
        !self.failing_universes().is_empty()
            || !self.invalid_universes().is_empty()
            || !self.divergent_universes.is_empty()
            || !self.merged.unreached_sometimes().is_empty()
            || !self.contract_violations.is_empty()
    }

    /// Report integrity: exactly the requested number of universes must
    /// have produced results. Internal construction should make a mismatch
    /// impossible; the check fails closed against internal bugs anyway.
    fn cardinality_ok(&self) -> bool {
        self.results.len() as u64 == self.universes_requested
    }

    /// Tri-state verdict; CLEAN requires divergence checking to have run,
    /// every universe to have validly completed under a NON-EMPTY,
    /// satisfied property contract, and result cardinality to match the
    /// requested campaign size. An empty contract is UNCHECKED — a
    /// campaign that asserted nothing proved nothing
    /// (hardening-loop-4 GAP 5).
    pub fn verdict(&self) -> Verdict {
        if !self.cardinality_ok() || self.has_findings() {
            Verdict::Findings
        } else if self.divergence_checked && !self.contract.is_empty() {
            Verdict::Clean
        } else {
            Verdict::Unchecked
        }
    }

    /// True only for a divergence-checked, finding-free, validly completed
    /// run.
    pub fn is_clean(&self) -> bool {
        self.verdict() == Verdict::Clean
    }
}

/// v0 runs universes sequentially; Phase 3 fans out across cores. The
/// sequential baseline is also the reference implementation the parallel
/// runner must match hash-for-hash.
///
/// Replay pairing is NON-ADJACENT (hardening-loop-4 BLOCKER 2): pass 1
/// runs every universe once, pass 2 replays them all. The old adjacent
/// pairing let a process-global counter divided by 2 agree with itself
/// inside every pair (`A,A` then `B,B`) and be reported divergence-free;
/// separating the passes catches that exact class. It remains a SAMPLED
/// falsifier: a workload keyed to the full execution schedule can still
/// agree with itself (documented by
/// `schedule_keyed_nondeterminism_still_evades_sampled_replay_agreement`
/// in `tests/divergence.rs`), which is why the evidence is named
/// [`ReplayEvidence::PairwiseReplayAgreement`], never "proof".
pub fn run_multiverse(cfg: &MultiverseConfig, workload: &dyn Workload) -> MultiverseReport {
    run_multiverse_with_palette(cfg, workload, FaultPalette::V0)
}

/// Run a campaign with an explicit fault palette. v0 is the default path
/// used by [`run_multiverse`]; swarm is opt-in for the Track-2 bakeoff.
pub fn run_multiverse_with_palette(
    cfg: &MultiverseConfig,
    workload: &dyn Workload,
    fault_palette: FaultPalette,
) -> MultiverseReport {
    run_multiverse_recorded(cfg, workload, fault_palette, false)
}

/// [`run_multiverse_with_palette`] with the decision-tape recorder
/// toggled (convergence C1; opt-in — see [`run_universe_recorded`]).
pub fn run_multiverse_recorded(
    cfg: &MultiverseConfig,
    workload: &dyn Workload,
    fault_palette: FaultPalette,
    record_tape: bool,
) -> MultiverseReport {
    run_multiverse_scheduled(
        cfg,
        workload,
        fault_palette,
        record_tape,
        SchedulePolicy::Fifo,
    )
}

/// [`run_multiverse_recorded`] with an explicit schedule policy
/// (convergence C2; opt-in, Fifo default).
pub fn run_multiverse_scheduled(
    cfg: &MultiverseConfig,
    workload: &dyn Workload,
    fault_palette: FaultPalette,
    record_tape: bool,
    policy: SchedulePolicy,
) -> MultiverseReport {
    let universes = cfg.universes.get();
    // UniverseCount::MAX bounds this preallocation; the typed constructor
    // rejects anything larger before we get here.
    let mut results = Vec::with_capacity(universes as usize);
    let mut divergent = Vec::new();
    let mut merged = MergedProperties::default();
    let contract = workload.property_contract();
    let mut contract_violations: Vec<(u64, String)> = Vec::new();

    for universe_id in 0..universes {
        let first = run_universe_scheduled(
            cfg.root_seed,
            universe_id,
            workload,
            fault_palette,
            record_tape,
            policy,
        );
        merged.absorb(universe_id, &props_of(&first));
        for v in contract.violations(&first) {
            contract_violations.push((universe_id, v));
        }
        results.push(first);
    }
    if cfg.check_divergence {
        for universe_id in 0..universes {
            let second = run_universe_scheduled(
                cfg.root_seed,
                universe_id,
                workload,
                fault_palette,
                record_tape,
                policy,
            );
            if !second.observably_equal(&results[universe_id as usize]) {
                divergent.push(universe_id);
            }
        }
    }

    MultiverseReport {
        root_seed: cfg.root_seed,
        workload: workload.name().to_string(),
        divergence_checked: cfg.check_divergence,
        universes_requested: universes,
        results,
        divergent_universes: divergent,
        merged,
        contract,
        contract_violations,
    }
}

fn props_of(result: &UniverseResult) -> Properties {
    let mut p = Properties::new();
    for f in &result.always_failures {
        p.always(&f.name, false, || f.detail.clone());
    }
    for (name, hit) in &result.sometimes {
        p.declare_sometimes(name);
        if *hit {
            p.sometimes(name);
        }
    }
    p
}
