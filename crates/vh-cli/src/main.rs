//! `vh` — the vibe-halt CLI.
//!
//! This crate is the deterministic boundary: it may touch std::env and the
//! process exit code, and nothing inside the kernel crates may. Arg parsing
//! is manual to keep the workspace zero-dependency.

use vh_cli::workloads;
use vh_multiverse::{
    run_multiverse, run_universe, MultiverseConfig, UniverseCount, UniverseResult, Verdict,
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
           [--no-divergence-check]
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
                     crash-toctou, fsync-lie

`vh run` exits 0 only if the multiverse is CLEAN: divergence-checked, no
always-failure, no divergence, every declared sometimes reached, every
universe validly completed, and the workload's NON-EMPTY property
contract satisfied in every universe (a workload that asserts nothing is
UNCHECKED, never CLEAN). With --no-divergence-check a finding-free run
is UNCHECKED (exit 3), never CLEAN. A single-universe replay (--universe)
is likewise UNCHECKED (exit 3) when finding-free — one execution proves
nothing about reproducibility. --universes must be nonzero and within the
v0 resource bound; --universes conflicts with --universe.
";

struct RunArgs {
    workload: String,
    seed: u64,
    universes: Option<u64>,
    single_universe: Option<u64>,
    check_divergence: bool,
}

fn parse_run_args(args: &[String]) -> Result<RunArgs, String> {
    let mut out = RunArgs {
        workload: "demo".to_string(),
        seed: DEFAULT_SEED,
        universes: None,
        single_universe: None,
        check_divergence: true,
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
    Ok(out)
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
        let result = run_universe(run.seed, universe_id, workload.as_ref());
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
    let report = run_multiverse(&cfg, workload.as_ref());

    let failing = report.failing_universes();
    let invalid = report.invalid_universes();
    println!(
        "vibe-halt: workload={} seed=0x{:x} universes={} divergence-check={}",
        report.workload(),
        report.root_seed(),
        requested,
        run.check_divergence
    );
    println!(
        "  always-failures: {} universe(s); divergent: {}; sometimes unreached: {}; invalid completions: {}; contract violations: {}",
        failing.len(),
        report.divergent_universes().len(),
        report.merged().unreached_sometimes().len(),
        invalid.len(),
        report.contract_violations().len()
    );
    println!("  evidence: {}", report.replay_evidence().label());

    for &u in failing.iter().take(10) {
        let r = &report.results()[u as usize];
        for f in r.always_failures() {
            println!("  FAIL universe {u}: {} — {}", f.name, f.detail);
        }
        println!(
            "    repro: vh run --workload {} --seed 0x{:x} --universe {u}",
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

    match report.verdict() {
        Verdict::Clean => {
            println!("  verdict: CLEAN");
            0
        }
        Verdict::Findings => {
            println!("  verdict: FINDINGS (see above)");
            1
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
            3
        }
    }
}

/// Render the COMPLETE public observation of a universe result into a
/// fresh trace and hash it: one canonical fingerprint over every
/// observable field, reusing the frozen trace-hash machinery
/// (docs/specs/TRACE_FORMAT_V0.md). Schema versioned: renderer changes
/// are compatibility decisions. v3 (Phase-1 sim runtime, 2026-07-21):
/// adds the runner-owned semantic fault-lifecycle evidence observable
/// (`UniverseResult::runtime_evidence`) — an explicit migration from v2
/// (`cdb049391ddbacc06eb3faf3ea1cb43a`), recorded in
/// docs/specs/TRACE_FORMAT_V0.md § Changelog; the underlying TRACE hash
/// identity is unchanged. v2 (hardening-loop-4 GAP 5) added the
/// fault-plan digest and retrieval-honest lifecycle over v1
/// (`462e803383be1b24594e76d5f9301be8`).
const DOCTOR_OBSERVABLE_SCHEMA: &str = "vh-doctor-observable-v3";

fn observable_fingerprint(result: &UniverseResult) -> String {
    let mut t = vh_trace::Trace::new();
    t.record(0, "schema", DOCTOR_OBSERVABLE_SCHEMA);
    t.record(0, "universe-id", &result.universe_id().to_string());
    t.record(0, "trace-hash", result.trace_hash());
    t.record(0, "trace-events", &result.trace_events().to_string());
    for c in result.always_checks() {
        t.record(0, "always-check", &format!("{}={}", c.name, c.passed));
    }
    for f in result.always_failures() {
        t.record(0, "always-failure", &format!("{}={}", f.name, f.detail));
    }
    for (name, hit) in result.sometimes() {
        t.record(0, "sometimes", &format!("{name}={hit}"));
    }
    t.record(0, "lifecycle", &format!("{:?}", result.lifecycle()));
    t.record(
        0,
        "fault-plan-digest",
        result.fault_plan_digest().unwrap_or("none"),
    );
    match result.runtime_evidence() {
        None => t.record(0, "runtime-evidence", "none"),
        Some(ev) => {
            for inj in ev.injections() {
                t.record(0, "runtime-injection", &inj.canonical());
            }
        }
    }
    t.hash_hex()
}

/// Frozen fingerprint of the complete doctor observation (demo workload,
/// seed 0xD1CE, universe 0) under `vh-doctor-observable-v3`. Unlike the
/// trace hash alone, this pins the assertion transcript, failures,
/// sometimes states, lifecycle, fault-plan digest, and runtime evidence
/// — a regression in any observable fails doctor even when the trace
/// hash survives (hardening-loop-2 GAP).
const DOCTOR_EXPECTED_FINGERPRINT: &str = "1684e7c347e645f43a80a30abc46adb7";

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
