//! `vh` — the vibe-halt CLI.
//!
//! This crate is the deterministic boundary: it may touch std::env and the
//! process exit code, and nothing inside the kernel crates may. Arg parsing
//! is manual to keep the workspace zero-dependency.

use vh_cli::workloads;
use vh_multiverse::{run_multiverse, run_universe, MultiverseConfig};

const DEFAULT_SEED: u64 = 0xD1CE;
const DEFAULT_UNIVERSES: u64 = 100;

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
    vh run [--workload NAME] [--seed N] [--universes N] [--universe K]
           [--no-divergence-check]
    vh doctor

WORKLOADS:
    demo         correct toy KV service (should pass)
    demo-buggy   ack-before-flush durability bug (rig must find it)
    demo-nondet  leaks global state (divergence detector must flag it)

`vh run` exits 0 only if the multiverse is clean: no always-failure,
no divergence, every declared sometimes reached.
";

struct RunArgs {
    workload: String,
    seed: u64,
    universes: u64,
    single_universe: Option<u64>,
    check_divergence: bool,
}

fn parse_run_args(args: &[String]) -> Result<RunArgs, String> {
    let mut out = RunArgs {
        workload: "demo".to_string(),
        seed: DEFAULT_SEED,
        universes: DEFAULT_UNIVERSES,
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
                out.universes = parse_u64(&value_for("--universes")?)?;
            }
            "--universe" => {
                out.single_universe = Some(parse_u64(&value_for("--universe")?)?);
            }
            "--no-divergence-check" => out.check_divergence = false,
            other => return Err(format!("unknown argument: {other}")),
        }
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

    // Single-universe verbose replay: the repro path.
    if let Some(universe_id) = run.single_universe {
        let result = run_universe(run.seed, universe_id, workload.as_ref());
        println!(
            "universe {universe_id} (seed 0x{:x}, workload {}): hash {} events {}",
            run.seed,
            workload.name(),
            result.trace_hash,
            result.trace_events
        );
        for f in &result.always_failures {
            println!("  ALWAYS-FAIL {}: {}", f.name, f.detail);
        }
        for (name, hit) in &result.sometimes {
            println!(
                "  sometimes {name}: {}",
                if *hit { "hit" } else { "not hit" }
            );
        }
        return if result.always_failures.is_empty() {
            0
        } else {
            1
        };
    }

    let cfg = MultiverseConfig {
        root_seed: run.seed,
        universes: run.universes,
        check_divergence: run.check_divergence,
    };
    let report = run_multiverse(&cfg, workload.as_ref());

    let failing = report.failing_universes();
    println!(
        "vibe-halt: workload={} seed=0x{:x} universes={} divergence-check={}",
        report.workload, report.root_seed, run.universes, run.check_divergence
    );
    println!(
        "  always-failures: {} universe(s); divergent: {}; sometimes unreached: {}",
        failing.len(),
        report.divergent_universes.len(),
        report.merged.unreached_sometimes().len()
    );

    for &u in failing.iter().take(10) {
        let r = &report.results[u as usize];
        for f in &r.always_failures {
            println!("  FAIL universe {u}: {} — {}", f.name, f.detail);
        }
        println!(
            "    repro: vh run --workload {} --seed 0x{:x} --universe {u}",
            report.workload, report.root_seed
        );
    }
    if failing.len() > 10 {
        println!("  ... and {} more failing universes", failing.len() - 10);
    }
    for &u in &report.divergent_universes {
        println!("  DIVERGENT universe {u}: two runs produced different trace hashes");
    }
    for name in report.merged.unreached_sometimes() {
        println!("  SOMETIMES-UNREACHED: {name} (dead path across the whole multiverse)");
    }

    if report.is_clean() {
        println!("  verdict: CLEAN");
        0
    } else {
        println!("  verdict: FINDINGS (see above)");
        1
    }
}

fn cmd_doctor() -> i32 {
    println!("vh {} — determinism self-check", env!("CARGO_PKG_VERSION"));
    let workload = workloads::by_name("demo").expect("demo workload exists");
    let a = run_universe(DEFAULT_SEED, 0, workload.as_ref());
    let b = run_universe(DEFAULT_SEED, 0, workload.as_ref());
    if a.trace_hash == b.trace_hash {
        println!("  replay check: OK (universe 0 hash {})", a.trace_hash);
        0
    } else {
        println!("  replay check: FAILED — kernel nondeterminism, do not trust results");
        1
    }
}
