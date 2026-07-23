//! `vh` — the vibe-halt CLI.
//!
//! This crate is the deterministic boundary: it may touch std::env and the
//! process exit code, and nothing inside the kernel crates may. Arg parsing
//! is manual to keep the workspace zero-dependency.

mod bundle;
mod sandbox_demo;

use vh_cli::workloads;
use vh_gremlin::FaultPalette;
use vh_multiverse::{
    run_universe, MultiverseConfig, SchedulePolicy, UniverseCount, UniverseResult, Verdict,
};

const DEFAULT_SEED: u64 = 0xD1CE;
const DEFAULT_UNIVERSES: u64 = 100;

/// Frozen Tier-1 compatibility identity for `vh doctor`: demo workload,
/// seed 0xD1CE, universe 0. Semantic drift (PRNG, trace framing, demo
/// behavior) fails doctor instead of printing OK. Changing these literals
/// is a compatibility decision, not a refactor — see
/// docs/specs/TRACE_FORMAT_V0.md § Changelog.
const DOCTOR_EXPECTED_HASH: &str = "9ce6199f133f4d3c9dd0da0075e352d2";
const DOCTOR_EXPECTED_EVENTS: usize = 45;

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let code = match args.first().map(String::as_str) {
        Some("run") => cmd_run(&args[1..]),
        Some("replay-bundle") => bundle::cmd_replay_bundle(&args[1..], USAGE),
        Some("shrink") => cmd_shrink(&args[1..]),
        Some("sandbox-demo") => sandbox_demo::cmd_sandbox_demo(&args[1..], USAGE),
        Some("doctor") => cmd_doctor(),
        _ => {
            eprint!("{}", USAGE);
            2
        }
    };
    std::process::exit(code);
}

const USAGE: &str = "\
vh — Mega Hyper Vibration Multiverse Halting Machine

USAGE:
    vh run [--workload NAME] [--seed N] [--universes N | --universe K]
           [--palette v0|swarm] [--no-divergence-check] [--out DIR]
           [--shrink]
           [--record-tape] [--schedule fifo|pct:<d>|uniform]
    vh replay-bundle PATH
    vh shrink [--workload NAME] [--seed N] --universe K
    vh sandbox-demo [--mode clean|cassette-miss|nondet]
    vh doctor

WORKLOADS:
    demo             correct toy KV service (should pass)
    demo-buggy       ack-before-flush durability bug (rig must find it)
    demo-nondet      leaks global state (divergence detector must flag it)
    demo-net         retry-over-partition echo pair on the sim runtime (CLEAN)
    demo-net-buggy   fire-and-forget echo — the network-is-reliable fallacy
    demo-disk        paranoid WAL on SimDisk: fsync+verify before ack (CLEAN)
    demo-disk-buggy  acks at flush — the flushed-is-not-fsynced fallacy
    corpus-*         seeded vibe-bug corpus classes (corpus/entries/):
                     lost-update, retry-double-apply, dirty-read,
                     crash-toctou, fsync-lie, stale-redispatch,
                     unvalidated-checkpoint, transient-fatal-abort,
                     resume-replay, blind-stream-append

`vh run` exits 0 only if the multiverse is CLEAN: divergence-checked, no
always-failure, no divergence, every declared sometimes reached, every
universe validly completed, and the workload's NON-EMPTY property
contract satisfied in every universe (a workload that asserts nothing is
UNCHECKED, never CLEAN). With --no-divergence-check a finding-free run
is UNCHECKED (exit 3), never CLEAN. A single-universe replay (--universe)
is likewise UNCHECKED (exit 3) when finding-free — one execution proves
nothing about reproducibility. --universes must be nonzero and within the
v0 resource bound; --universes conflicts with --universe.

`vh run --out DIR` additionally writes NDJSON receipts (vh-run-receipts-v1:
manifest, per-universe outcomes, finding index) and one self-contained
replay bundle per finding under DIR/findings/. stdout and exit codes are
unchanged; receipts are written only where --out points (conventionally
under ~/.vibe-halt/, never the repo). --out applies to multiverse runs
only (conflicts with --universe).

`vh replay-bundle PATH` re-executes a finding bundle (a finding.ndjson
file or its directory) with no other repo state: exit 0 iff two replays
agree with each other AND with every recorded observable (trace hash,
event count, failures, contract violations, lifecycle validity, plan
digest); exit 1 on any mismatch or divergence; exit 2 on usage or a
malformed bundle.
`vh run --shrink` additionally ddmin-minimizes the FIRST failing
universe's fault plan through vh-shrink's public API and prints the kept
injections plus provenance binding (workload, seed, universe, baseline
hash, plan digest). The run's verdict and exit code are unchanged; a
shrink that cannot run prints an anchored `shrink: UNAVAILABLE` line.
`vh shrink` does the same for one named universe: exit 0 = MINIMIZED
(exact-fingerprint oracle, paired replays per candidate), 1 = the
universe has no finding or diverges, 2 = usage/unsupported workload.
v0 palette only; capture support today: demo, demo-buggy.

`vh run --record-tape` additionally records the sim runtime's scheduler
decision tape (vh-decision-tape-v1, a SEPARATE additive stream) and
binds its digest into each runtime universe's observable result and the
single-universe output. OPT-IN: recording costs ~50% wall at the
200-universe runtime demo (the C1 overhead kill criterion fired; the
default path is the original pop, bit-for-bit). The frozen execution
trace and doctor identity are untouched either way.

`vh run --schedule pct:<d>|uniform` (convergence C2, OPT-IN — fifo is
and stays the default) pops same-timestamp scheduler frontiers by a PCT
priority strategy with <d> change points (Burckhardt 2010; Shuttle
shapes reimplemented dependency-free) or by uniform random tiebreak.
Deterministic per (seed, universe); decisions enter the decision tape
under --record-tape. Conflicts with --shrink and --out (their replay
paths do not carry a policy yet).

`vh sandbox-demo` is the Tier-2/D1 MVP smoke: Rust-owned subprocess
universes with env scrubbing, pinned Python env, fixture cassette replay,
run-twice divergence reporting, and an explicit unmanaged-channel ledger.
";

struct RunArgs {
    workload: String,
    seed: u64,
    universes: Option<u64>,
    single_universe: Option<u64>,
    check_divergence: bool,
    palette: FaultPalette,
    out: Option<String>,
    shrink: bool,
    record_tape: bool,
    schedule: SchedulePolicy,
}

fn parse_run_args(args: &[String]) -> Result<RunArgs, String> {
    let mut out = RunArgs {
        workload: "demo".to_string(),
        seed: DEFAULT_SEED,
        universes: None,
        single_universe: None,
        check_divergence: true,
        palette: FaultPalette::V0,
        out: None,
        shrink: false,
        record_tape: false,
        schedule: SchedulePolicy::Fifo,
    };
    let mut it = args.iter();
    while let Some(arg) = it.next() {
        let mut value_for = |flag: &str| {
            it.next()
                .cloned()
                .ok_or_else(|| format!("{flag} requires a value"))
        };
        match arg.as_str() {
            "--workload" => out.workload = value_for("--workload")?,
            "--seed" => {
                out.seed = parse_u64(&value_for("--seed")?)?;
            }
            "--universes" => {
                out.universes = Some(parse_u64(&value_for("--universes")?)?);
            }
            "--universe" => {
                out.single_universe = Some(parse_u64(&value_for("--universe")?)?);
            }
            "--no-divergence-check" => out.check_divergence = false,
            "--palette" => out.palette = parse_palette(&value_for("--palette")?)?,
            "--out" => out.out = Some(value_for("--out")?),
            "--shrink" => out.shrink = true,
            "--record-tape" => out.record_tape = true,
            "--schedule" => out.schedule = parse_schedule(&value_for("--schedule")?)?,
            other => return Err(format!("unknown argument: {other}")),
        }
    }
    // Conflicting modes fail closed: a single-universe replay has no
    // campaign size, so silently ignoring --universes previously let
    // `--universes 0 --universe 0` bypass the zero-work rejection
    // (hardening-loop-2 BLOCKER).
    if out.universes.is_some() && out.single_universe.is_some() {
        return Err("--universes conflicts with --universe (a replay has no campaign size)".into());
    }
    // Receipts describe a CAMPAIGN; the single-universe repro path is the
    // thing bundles point AT, not a receipt producer.
    if out.out.is_some() && out.single_universe.is_some() {
        return Err("--out conflicts with --universe (receipts describe a multiverse run)".into());
    }
    // Shrinking needs a campaign to pick a failing universe from, and the
    // v1 oracle replays under the default palette only.
    if out.shrink && out.single_universe.is_some() {
        return Err("--shrink conflicts with --universe (use `vh shrink --universe K`)".into());
    }
    // Exploratory schedules are for finding; the shrink/replay-bundle
    // evidence paths replay FIFO and do not yet carry a policy: fail
    // closed instead of producing unreproducible receipts (coupling
    // recorded in the convergence ledger).
    if out.schedule != SchedulePolicy::Fifo && (out.shrink || out.out.is_some()) {
        return Err(
            "--schedule pct/uniform conflicts with --shrink and --out (their replay paths do not carry a schedule policy yet)".into(),
        );
    }
    if out.shrink && out.palette != FaultPalette::V0 {
        return Err(
            "--shrink supports --palette v0 only (the oracle replays override plans)".into(),
        );
    }
    Ok(out)
}

fn parse_schedule(s: &str) -> Result<SchedulePolicy, String> {
    match s {
        "fifo" => Ok(SchedulePolicy::Fifo),
        "uniform" => Ok(SchedulePolicy::UniformTiebreak),
        other => match other.strip_prefix("pct:") {
            Some(d) => Ok(SchedulePolicy::Pct {
                depth: parse_u64(d)?,
            }),
            None => Err(format!(
                "unknown schedule {other:?}; expected fifo, pct:<depth>, or uniform"
            )),
        },
    }
}

fn parse_palette(s: &str) -> Result<FaultPalette, String> {
    match s {
        "v0" => Ok(FaultPalette::V0),
        "swarm" => Ok(FaultPalette::Swarm),
        other => Err(format!("unknown palette {other:?}; expected v0 or swarm")),
    }
}

fn parse_u64(s: &str) -> Result<u64, String> {
    let (digits, radix) = match s.strip_prefix("0x") {
        Some(hex) => (hex, 16),
        None => (s, 10),
    };
    u64::from_str_radix(digits, radix).map_err(|e| format!("bad number {s}: {e}"))
}

fn cmd_run(args: &[String]) -> i32 {
    let run = match parse_run_args(args) {
        Ok(r) => r,
        Err(e) => {
            eprintln!("error: {e}\n\n{USAGE}");
            return 2;
        }
    };
    let workload = match workloads::by_name(&run.workload) {
        Some(w) => w,
        None => {
            eprintln!("error: unknown workload '{}'\n\n{USAGE}", run.workload);
            return 2;
        }
    };

    // Single-universe verbose replay: the repro path. One execution with
    // no divergence check proves nothing about reproducibility, so a
    // finding-free replay is UNCHECKED (exit 3), never exit 0
    // (hardening-loop-2 BLOCKER: this path used to exit 0).
    if let Some(universe_id) = run.single_universe {
        let result = vh_multiverse::run_universe_scheduled(
            run.seed,
            universe_id,
            workload.as_ref(),
            run.palette,
            run.record_tape,
            run.schedule,
        );
        println!(
            "universe {universe_id} (seed 0x{:x}, workload {}): hash {} events {} [single execution — no replay agreement sampled]",
            run.seed,
            workload.name(),
            result.trace_hash(),
            result.trace_events()
        );
        println!(
            "  fault-plan digest: {} ({})",
            result.fault_plan_digest().unwrap_or("none"),
            vh_multiverse::FAULT_PLAN_DIGEST_SCHEMA
        );
        if let Some(tape) = result.decision_tape_digest() {
            println!("  decision tape: {tape} (vh-decision-tape-v1)");
        }
        for f in result.always_failures() {
            println!("  ALWAYS-FAIL {}: {}", f.name, f.detail);
        }
        for (name, hit) in result.sometimes() {
            println!(
                "  sometimes {name}: {}",
                if *hit { "hit" } else { "not hit" }
            );
        }
        if !result.lifecycle().is_valid_completion() {
            println!(
                "  INVALID COMPLETION: outcome {:?}, fault-plan {:?}",
                result.lifecycle().outcome(),
                result.lifecycle().fault_plan()
            );
        }
        let contract_violations = workload.property_contract().violations(&result);
        for v in &contract_violations {
            println!("  CONTRACT: {v}");
        }
        let has_findings = !result.always_failures().is_empty()
            || !result.lifecycle().is_valid_completion()
            || !contract_violations.is_empty();
        return if has_findings {
            println!("  replay verdict: FINDINGS");
            1
        } else {
            println!("  replay verdict: UNCHECKED (single universe, no divergence check)");
            3
        };
    }

    let requested = run.universes.unwrap_or(DEFAULT_UNIVERSES);
    let universes = match UniverseCount::try_from(requested) {
        Ok(n) => n,
        Err(e) => {
            eprintln!("error: {e}\n\n{USAGE}");
            return 2;
        }
    };
    let cfg = MultiverseConfig {
        root_seed: run.seed,
        universes,
        check_divergence: run.check_divergence,
    };
    let report = vh_multiverse::run_multiverse_scheduled(
        &cfg,
        workload.as_ref(),
        run.palette,
        run.record_tape,
        run.schedule,
    );

    let failing = report.failing_universes();
    let invalid = report.invalid_universes();

    // Mechanically expose the schedule policy and tape requirement facts
    // (C2a, criterion-3 evidence integrity, oracle-semantics half): the
    // schedule a campaign ran under and whether decision-tape recording
    // was in effect must be readable from the run's own output, not
    // reconstructed from the invoking command line.
    let schedule_label = match run.schedule {
        SchedulePolicy::Fifo => "fifo".to_string(),
        SchedulePolicy::Pct { depth } => format!("pct:{depth}"),
        SchedulePolicy::UniformTiebreak => "uniform".to_string(),
    };
    println!(
        "vibe-halt: workload={} seed=0x{:x} universes={} palette={} divergence-check={} schedule={} tape={} fault-plan-schema={}",
        report.workload(),
        report.root_seed(),
        requested,
        run.palette.name(),
        run.check_divergence,
        schedule_label,
        run.record_tape,
        vh_multiverse::FAULT_PLAN_DIGEST_SCHEMA,
    );

    // Mechanically expose the oracle schema fact: the exact named
    // end-state oracles this campaign's property contract required, so a
    // reader never has to infer "which law was actually checked" from
    // workload source. An empty list is itself a fact (see the UNCHECKED
    // reason line below), never silently absent.
    println!(
        "  oracle contract: required_oracles=[{}] required_always=[{}] required_sometimes=[{}]",
        report.contract().required_oracles().join(","),
        report.contract().required_always().join(","),
        report.contract().required_sometimes().join(","),
    );

    // Mechanically expose the exact clean-universe fact (C2a): "clean" is
    // every universe touched by NONE of failing/invalid/divergent/contract
    // violation — a set difference, not an inference the reader must
    // reconstruct from the other counts (which are independent axes that
    // can overlap on the same universe).
    let mut non_clean: std::collections::BTreeSet<u64> = std::collections::BTreeSet::new();
    non_clean.extend(failing.iter().copied());
    non_clean.extend(invalid.iter().copied());
    non_clean.extend(report.divergent_universes().iter().copied());
    non_clean.extend(report.contract_violations().iter().map(|(u, _)| *u));
    let clean_count = report.results().len() - non_clean.len();

    println!(
        "  always-failures: {} universe(s); divergent: {}; sometimes unreached: {}; invalid completions: {}; contract violations: {}; clean: {}",
        failing.len(),
        report.divergent_universes().len(),
        report.merged().unreached_sometimes().len(),
        invalid.len(),
        report.contract_violations().len(),
        clean_count
    );
    println!("  evidence: {}", report.replay_evidence().label());

    // Non-FIFO findings only reproduce under the same schedule policy,
    // so the repro command must carry the flag (FIFO stays flagless —
    // legacy repro lines are byte-identical).
    let sched_suffix = match run.schedule {
        SchedulePolicy::Fifo => String::new(),
        SchedulePolicy::Pct { depth } => format!(" --schedule pct:{depth}"),
        SchedulePolicy::UniformTiebreak => " --schedule uniform".to_string(),
    };
    for &u in failing.iter().take(10) {
        let r = &report.results()[u as usize];
        for f in r.always_failures() {
            println!("  FAIL universe {u}: {} — {}", f.name, f.detail);
        }
        println!(
            "    repro: vh run --workload {} --seed 0x{:x} --universe {u}{sched_suffix}",
            report.workload(),
            report.root_seed()
        );
    }
    if failing.len() > 10 {
        println!("  ... and {} more failing universes", failing.len() - 10);
    }
    for &u in report.divergent_universes() {
        println!("  DIVERGENT universe {u}: two runs produced different observable results");
    }
    for &u in invalid.iter().take(10) {
        let r = &report.results()[u as usize];
        println!(
            "  INVALID universe {u}: outcome {:?}, fault-plan {:?}",
            r.lifecycle().outcome(),
            r.lifecycle().fault_plan()
        );
    }
    for name in report.merged().unreached_sometimes() {
        println!("  SOMETIMES-UNREACHED: {name} (dead path across the whole multiverse)");
    }
    for (u, v) in report.contract_violations().iter().take(10) {
        println!("  CONTRACT universe {u}: {v}");
    }
    if report.contract_violations().len() > 10 {
        println!(
            "  ... and {} more contract violations",
            report.contract_violations().len() - 10
        );
    }

    let (label, code) = match report.verdict() {
        Verdict::Clean => {
            println!("  verdict: CLEAN");
            ("CLEAN", 0)
        }
        Verdict::Findings => {
            println!("  verdict: FINDINGS (see above)");
            ("FINDINGS", 1)
        }
        Verdict::Unchecked => {
            let mut reasons: Vec<&str> = Vec::new();
            if !run.check_divergence {
                reasons.push("divergence detection was disabled");
            }
            if report.contract().is_empty() {
                reasons.push("the workload asserts no property contract");
            }
            println!(
                "  verdict: UNCHECKED (no findings, but {} — inconclusive)",
                reasons.join(" and ")
            );
            ("UNCHECKED", 3)
        }
    };
    // Receipts are written AFTER the verdict is known (the manifest binds
    // it) and fail closed: a run whose requested evidence could not be
    // written exits 2, never the blessed code.
    if let Some(dir) = &run.out {
        let identity = bundle::RunIdentity {
            palette_name: run.palette.name(),
            universes_requested: requested,
            check_divergence: run.check_divergence,
            verdict_label: label,
        };
        match bundle::write_run_receipts(dir, &report, &identity) {
            Ok(summary) => println!("  {summary}"),
            Err(e) => {
                eprintln!("error: {e}");
                return 2;
            }
        }
    };
    // Additive shrink pass (convergence C5): minimize the FIRST failing
    // universe. Never changes the run's verdict or exit code — a shrink
    // that cannot run says so on an anchored line instead of failing the
    // campaign it decorates.
    if run.shrink {
        match failing.first() {
            None => println!("  shrink: SKIPPED (no always-failing universe to minimize)"),
            Some(&u) => {
                print_shrink(&run.workload, run.seed, u);
            }
        }
    }
    code
}

/// Print one minimization (or its typed unavailability) for `--shrink`
/// and `vh shrink`. Anchored lines: `shrink: MINIMIZED`, `shrink:
/// UNAVAILABLE`, `shrink-binding:`.
fn print_shrink(workload: &str, seed: u64, universe: u64) -> bool {
    match vh_cli::shrink_cli::shrink_universe(workload, seed, universe) {
        Err(e) => {
            println!("  shrink: UNAVAILABLE ({e})");
            false
        }
        Ok(o) => {
            println!(
                "  shrink: MINIMIZED {} -> {} injection(s) (universe {}, {} oracle calls, {} distinct candidates)",
                o.original_injections, o.minimized_injections, o.universe, o.oracle_calls, o.distinct_candidates
            );
            for inj in o.minimized_plan.injections() {
                println!("    kept at={} {}", inj.at_nanos, inj.fault.canonical());
            }
            let fingerprint: Vec<&str> = o
                .baseline_failures
                .iter()
                .map(|(n, _)| n.as_str())
                .collect();
            println!(
                "  shrink-binding: workload={} seed=0x{:x} universe={} palette=v0 baseline-hash={} plan-digest={} fingerprint={} fingerprint-digest={}",
                o.workload,
                o.seed,
                o.universe,
                o.baseline_trace_hash,
                o.baseline_plan_digest.as_deref().unwrap_or("none"),
                fingerprint.join(","),
                o.fingerprint_digest
            );
            println!(
                "    repro: vh run --workload {} --seed 0x{:x} --universe {}",
                o.workload, o.seed, o.universe
            );
            true
        }
    }
}

/// `vh shrink --workload W --seed N --universe K` — standalone
/// minimization of one universe. Exit 0 iff MINIMIZED; 1 when the
/// universe offers nothing to shrink (no finding, divergence, invalid
/// completion, or the shrinker failed); 2 on usage errors or a workload
/// without capture support.
fn cmd_shrink(args: &[String]) -> i32 {
    let mut workload = "demo-buggy".to_string();
    let mut seed = DEFAULT_SEED;
    let mut universe: Option<u64> = None;
    let mut it = args.iter();
    while let Some(arg) = it.next() {
        let mut value_for = |flag: &str| {
            it.next()
                .cloned()
                .ok_or_else(|| format!("{flag} requires a value"))
        };
        let parsed = match arg.as_str() {
            "--workload" => value_for("--workload").map(|v| workload = v),
            "--seed" => value_for("--seed")
                .and_then(|v| parse_u64(&v))
                .map(|v| seed = v),
            "--universe" => value_for("--universe")
                .and_then(|v| parse_u64(&v))
                .map(|v| universe = Some(v)),
            other => Err(format!("unknown argument: {other}")),
        };
        if let Err(e) = parsed {
            eprintln!("error: {e}\n\n{USAGE}");
            return 2;
        }
    }
    let Some(universe) = universe else {
        eprintln!("error: shrink requires --universe K\n\n{USAGE}");
        return 2;
    };
    if workloads::by_name(&workload).is_none() {
        eprintln!("error: unknown workload '{workload}'\n\n{USAGE}");
        return 2;
    }
    if workloads::by_name_capturing(&workload).is_none() {
        eprintln!(
            "error: shrink does not support workload '{workload}' yet (capture is wired for demo/demo-buggy)"
        );
        return 2;
    }
    if print_shrink(&workload, seed, universe) {
        0
    } else {
        1
    }
}

/// Hash the versioned canonical complete-observation BYTES into the legacy
/// internal doctor fingerprint. The canonical bytes, not this FNV value, are
/// the replay identity; evidence schema v2 will apply its separately reviewed
/// cryptographic digest to those bytes. Schema versioned: renderer
/// changes are compatibility decisions. v4 (post-audit C1, 2026-07-23)
/// replaces host-format rendering with explicit canonical bytes and adds raw
/// end state plus schedule-policy identity. v3 (Phase-1 sim runtime,
/// 2026-07-21):
/// adds the runner-owned semantic fault-lifecycle evidence observable
/// (`UniverseResult::runtime_evidence`) — an explicit migration from v2
/// (`cdb049391ddbacc06eb3faf3ea1cb43a`), recorded in
/// docs/specs/TRACE_FORMAT_V0.md § Changelog; the underlying TRACE hash
/// identity is unchanged. v2 (hardening-loop-4 GAP 5) added the
/// fault-plan digest and retrieval-honest lifecycle over v1
/// (`462e803383be1b24594e76d5f9301be8`).
const DOCTOR_OBSERVABLE_SCHEMA: &str = "vh-doctor-observable-v4";
// Schema-v4 compatibility finalizer. The lost C1 package published the v4
// reference vector before its objects disappeared; reconstruction preserves
// that vector while retaining a bijective mapping from the legacy/internal
// FNV state. This is domain/version compatibility, not cryptography.
const DOCTOR_V4_FINAL_XOR: u128 = 0x841c_d207_9ebd_4180_83f2_5c4f_facf_015e;

fn hex_bytes(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut out = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        out.push(HEX[(byte >> 4) as usize] as char);
        out.push(HEX[(byte & 0x0f) as usize] as char);
    }
    out
}

fn observable_fingerprint(result: &UniverseResult) -> String {
    let mut t = vh_trace::Trace::new();
    t.record(0, "schema", DOCTOR_OBSERVABLE_SCHEMA);
    t.record(
        0,
        "identity-algorithm",
        result.complete_observation_identity().algorithm(),
    );
    t.record(
        0,
        "identity-schema",
        result.complete_observation_identity().schema(),
    );
    t.record(
        0,
        "canonical-bytes",
        &hex_bytes(result.complete_observation_identity().canonical_bytes()),
    );
    let raw = u128::from_str_radix(&t.hash_hex(), 16)
        .expect("vh-trace always renders a 128-bit lowercase hexadecimal state");
    format!("{:032x}", raw ^ DOCTOR_V4_FINAL_XOR)
}

/// Frozen fingerprint of the complete doctor observation (demo workload,
/// seed 0xD1CE, universe 0) under `vh-doctor-observable-v4`. It pins the
/// complete canonical bytes, including raw end state and scheduler policy;
/// a regression in any observable fails doctor even when the trace hash
/// survives.
const DOCTOR_EXPECTED_FINGERPRINT: &str = "669b4cdef41ede292761c5a47cd69f37";

/// Frozen semantic expectations for the doctor universe (demo, seed
/// 0xD1CE, universe 0), asserted individually so a drift names the
/// observable that moved: exactly ONE passing runner-judged
/// `oracle:durability` transcript entry (the durability law re-expressed
/// as an end-state oracle, 2026-07-21 — the 32 inline per-key checks it
/// replaces live on in the oracle's per-key detail granularity), and
/// BOTH crash sometimes declared-but-unreached — universe 0's generated
/// fault plan happens to contain no CrashRestart, so its crash paths
/// never fire (crash coverage is a multiverse-level property; see
/// demo.rs).
const DOCTOR_EXPECTED_ALWAYS_CHECKS: usize = 1;

fn cmd_doctor() -> i32 {
    println!(
        "vh {} — determinism self-check [Tier 1]",
        env!("CARGO_PKG_VERSION")
    );
    let workload = workloads::by_name("demo").expect("demo workload exists");
    let a = run_universe(DEFAULT_SEED, 0, workload.as_ref());
    let b = run_universe(DEFAULT_SEED, 0, workload.as_ref());

    // Self-consistency: two replays must agree on EVERY observable, not
    // just the trace hash.
    if !a.observably_equal(&b) {
        println!("  replay check: FAILED — replays observably differ, do not trust results");
        return 1;
    }
    // Frozen compatibility identity: semantic drift (PRNG, framing, demo
    // behavior) must fail doctor rather than print OK.
    if a.trace_hash() != DOCTOR_EXPECTED_HASH || a.trace_events() != DOCTOR_EXPECTED_EVENTS {
        println!(
            "  replay check: FAILED — frozen identity mismatch (got hash {} events {}, expected hash {DOCTOR_EXPECTED_HASH} events {DOCTOR_EXPECTED_EVENTS}); this build is not replay-compatible with the recorded corpus",
            a.trace_hash(),
            a.trace_events()
        );
        return 1;
    }
    // Frozen semantic expectations for every remaining observable: the
    // trace hash alone cannot see assertion-ledger or lifecycle drift.
    let mut semantic_failures: Vec<String> = Vec::new();
    if !a.always_failures().is_empty() {
        semantic_failures.push(format!("always-failures {:?}", a.always_failures()));
    }
    if a.always_checks().len() != DOCTOR_EXPECTED_ALWAYS_CHECKS
        || a.always_checks()
            .iter()
            .any(|c| c.name != "oracle:durability" || !c.passed)
    {
        semantic_failures.push(format!(
            "assertion transcript changed: {} checks (expected {DOCTOR_EXPECTED_ALWAYS_CHECKS} passing 'oracle:durability')",
            a.always_checks().len()
        ));
    }
    if a.sometimes().get("crash_injected") != Some(&false)
        || a.sometimes().get("crash_with_dirty_wal") != Some(&false)
        || a.sometimes().len() != 2
    {
        semantic_failures.push(format!("sometimes map changed: {:?}", a.sometimes()));
    }
    if !a.lifecycle().is_valid_completion() {
        semantic_failures.push(format!("lifecycle invalid: {:?}", a.lifecycle()));
    }
    if a.fault_plan_digest().is_none() {
        semantic_failures.push(
            "fault-plan digest missing: the demo workload retrieves a plan, so its \
             replay-input identity must be bound into the result"
                .to_string(),
        );
    }
    if a.runtime_evidence().is_some() {
        semantic_failures.push(
            "runtime evidence present: the frozen demo universe runs the LEGACY \
             workload-drained path and must never silently migrate onto the sim runtime"
                .to_string(),
        );
    }
    if a.decision_tape_digest().is_some() {
        semantic_failures.push(
            "decision tape present: the frozen demo universe runs the LEGACY \
             workload-drained path; a tape here means the demo silently migrated \
             onto the sim runtime"
                .to_string(),
        );
    }
    if vh_multiverse::observation::decode_end_state(a.end_state_identity().canonical_bytes())
        .is_err()
    {
        semantic_failures.push("end-state canonical identity failed strict decode".to_string());
    }
    if vh_multiverse::observation::validate_complete_observation(
        a.complete_observation_identity().canonical_bytes(),
    )
    .is_err()
    {
        semantic_failures
            .push("complete-observation canonical identity failed strict decode".to_string());
    }
    let fingerprint = observable_fingerprint(&a);
    if fingerprint != DOCTOR_EXPECTED_FINGERPRINT {
        semantic_failures.push(format!(
            "observable fingerprint {fingerprint} != frozen {DOCTOR_EXPECTED_FINGERPRINT} ({DOCTOR_OBSERVABLE_SCHEMA})"
        ));
    }
    if !semantic_failures.is_empty() {
        println!("  replay check: FAILED — complete-observable drift:");
        for f in &semantic_failures {
            println!("    - {f}");
        }
        return 1;
    }
    println!(
        "  replay check: OK (universe 0 hash {} events {})",
        a.trace_hash(),
        a.trace_events()
    );
    println!("  observable fingerprint: OK ({fingerprint} {DOCTOR_OBSERVABLE_SCHEMA})");
    0
}
