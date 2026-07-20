//! vh-multiverse — runs workloads across universes and detects divergence.
//!
//! CI gate #1 lives here: with `check_divergence` on, every universe is run
//! TWICE and the trace hashes must match bit-for-bit. A mismatch means
//! nondeterminism leaked into the kernel or the workload, and the report
//! says so loudly instead of pretending the run was reproducible.

use std::collections::BTreeMap;

use vh_core::{SeedTree, VirtualClock, VirtualTime, Xoshiro256pp};
use vh_props::{AlwaysFailure, MergedProperties, Properties};
use vh_trace::Trace;

/// Everything a workload may touch inside one universe. All randomness comes
/// from named streams; all time comes from the virtual clock; all observable
/// behavior goes into the trace.
pub struct UniverseCtx {
    pub universe_id: u64,
    seed_tree: SeedTree,
    pub clock: VirtualClock,
    pub trace: Trace,
    pub props: Properties,
}

impl UniverseCtx {
    fn new(root_seed: u64, universe_id: u64) -> Self {
        Self {
            universe_id,
            seed_tree: SeedTree::new(root_seed),
            clock: VirtualClock::new(),
            trace: Trace::new(),
            props: Properties::new(),
        }
    }

    /// A named, independent PRNG stream for this universe.
    pub fn stream(&self, name: &str) -> Xoshiro256pp {
        self.seed_tree.stream(self.universe_id, name)
    }

    /// Record a trace event stamped with the current virtual time.
    pub fn record(&mut self, kind: &str, data: &str) {
        let at = self.clock.now().nanos();
        self.trace.record(at, kind, data);
    }

    pub fn advance_to(&mut self, nanos: u64) {
        self.clock.advance_to(VirtualTime(nanos));
    }
}

pub trait Workload {
    fn name(&self) -> &str;
    fn run(&self, ctx: &mut UniverseCtx);
}

#[derive(Debug, Clone)]
pub struct UniverseResult {
    pub universe_id: u64,
    pub trace_hash: String,
    pub trace_events: usize,
    pub always_failures: Vec<AlwaysFailure>,
    pub sometimes: BTreeMap<String, bool>,
}

pub fn run_universe(root_seed: u64, universe_id: u64, workload: &dyn Workload) -> UniverseResult {
    let mut ctx = UniverseCtx::new(root_seed, universe_id);
    workload.run(&mut ctx);
    UniverseResult {
        universe_id,
        trace_hash: ctx.trace.hash_hex(),
        trace_events: ctx.trace.len(),
        always_failures: ctx.props.always_failures().to_vec(),
        sometimes: ctx.props.sometimes_map().clone(),
    }
}

#[derive(Debug, Clone)]
pub struct MultiverseConfig {
    pub root_seed: u64,
    pub universes: u64,
    /// Run every universe twice and compare trace hashes.
    pub check_divergence: bool,
}

#[derive(Debug, Clone)]
pub struct MultiverseReport {
    pub root_seed: u64,
    pub workload: String,
    pub results: Vec<UniverseResult>,
    /// Universe ids whose two runs produced different trace hashes.
    pub divergent_universes: Vec<u64>,
    pub merged: MergedProperties,
}

impl MultiverseReport {
    /// Universe ids with at least one always-failure.
    pub fn failing_universes(&self) -> Vec<u64> {
        self.results
            .iter()
            .filter(|r| !r.always_failures.is_empty())
            .map(|r| r.universe_id)
            .collect()
    }

    /// The run is clean iff: no always-failure, no divergence, and every
    /// declared sometimes was reached somewhere in the multiverse.
    pub fn is_clean(&self) -> bool {
        self.failing_universes().is_empty()
            && self.divergent_universes.is_empty()
            && self.merged.unreached_sometimes().is_empty()
    }
}

/// v0 runs universes sequentially; Phase 3 fans out across cores. The
/// sequential baseline is also the reference implementation the parallel
/// runner must match hash-for-hash.
pub fn run_multiverse(cfg: &MultiverseConfig, workload: &dyn Workload) -> MultiverseReport {
    let mut results = Vec::with_capacity(cfg.universes as usize);
    let mut divergent = Vec::new();
    let mut merged = MergedProperties::default();

    for universe_id in 0..cfg.universes {
        let first = run_universe(cfg.root_seed, universe_id, workload);
        if cfg.check_divergence {
            let second = run_universe(cfg.root_seed, universe_id, workload);
            if second.trace_hash != first.trace_hash {
                divergent.push(universe_id);
            }
        }
        merged.absorb(universe_id, &props_of(&first));
        results.push(first);
    }

    MultiverseReport {
        root_seed: cfg.root_seed,
        workload: workload.name().to_string(),
        results,
        divergent_universes: divergent,
        merged,
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
