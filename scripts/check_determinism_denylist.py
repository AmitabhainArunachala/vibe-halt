#!/usr/bin/env python3
"""Determinism deny-list gate (CI gate #0).

Kernel crates must be pure: no wall clock, no OS randomness, no hash-order
iteration, no threads, no I/O, no environment access. This check is
mechanical and uncharmable — a hit fails the build regardless of how good
the surrounding prose sounds.

vh-cli is the deterministic boundary and is exempt (it owns argv/exit codes).
Test code is scanned too: `std::sync::atomic` is allowed (single-threaded
determinism is unaffected); everything below is not.
"""

from __future__ import annotations

import re
import sys
from pathlib import Path

REPO = Path(__file__).resolve().parent.parent

KERNEL_CRATES = [
    "crates/vh-core",
    "crates/vh-trace",
    "crates/vh-gremlin",
    "crates/vh-props",
    "crates/vh-multiverse",
]

# Pattern -> reason. Keep patterns coarse: false positives are cheap to
# rename around; false negatives silently rot determinism.
# Known, accepted miss (documented, PR #1 review): float NaN/precision
# formatting cannot be caught by a line regex — `format!("{x}")` on an f64
# is indistinguishable from any other formatting. An AST/type-aware lint is
# planned for Phase 2; until then float formatting in kernel crates is a
# review-time concern.
DENYLIST: dict[str, str] = {
    r"std::time": "wall-clock time; use vh_core::VirtualClock",
    r"Instant::now": "wall-clock time; use vh_core::VirtualClock",
    r"SystemTime": "wall-clock time; use vh_core::VirtualClock",
    r"\bHashMap\b": "hash-order iteration; use BTreeMap",
    r"\bHashSet\b": "hash-order iteration; use BTreeSet",
    r"RandomState": "hash randomization; use BTree collections",
    r"thread::spawn": "threads in the kernel; parallelism lives at the multiverse boundary",
    r"std::thread": "threads in the kernel; parallelism lives at the multiverse boundary",
    r"thread_local!": "thread-local mutable state; per-universe state lives in UniverseCtx",
    r"static\s+mut\b": "global mutable state; per-universe state lives in UniverseCtx",
    r"\brand\b\s*::": "external RNG; use vh_core::Xoshiro256pp streams",
    r"getrandom": "OS randomness; use the seed tree",
    r"std::env": "environment leakage; config enters via typed parameters",
    r"\benv!\s*\(": "compile-time environment capture; varies per build environment",
    r"option_env!": "compile-time environment capture; varies per build environment",
    r"std::fs": "filesystem I/O in the kernel; Tier-2 I/O goes through the sandbox layer",
    r"std::io": "OS I/O in the kernel",
    r"std::os": "OS-specific escape hatches in the kernel",
    r"std::net": "network I/O in the kernel; simulated network lands in Phase 1",
    r"std::process": "process control in the kernel; subprocess universes live in the Tier-2 sandbox",
    r"\{:p\}": "pointer-address formatting; addresses vary per run (ASLR)",
    r"\b(global_)?asm!": "inline assembly escape hatch",
    # Grouped imports would otherwise dodge the literal std::<mod> rules
    # (`use std::{fs, env};` contains no "std::fs").
    r"use\s+std::\{[^}]*\b(fs|env|net|process|io|os|thread|time)\b": (
        "grouped import of a denied std module"
    ),
}

# --self-test corpus: (sample line, must_hit). The HIT samples are the
# bypasses found in the PR #1 review; the MISS samples guard against the
# gate over-reaching into legitimate kernel code.
SELF_TEST: list[tuple[str, bool]] = [
    ('format!("{:p}", ptr)', True),
    ("RandomState::new()", True),
    ("std::io::stdin().read_line(&mut s)", True),
    ("std::os::unix::fs::MetadataExt::ino(&m)", True),
    ("use std::{fs, env, net};", True),
    ('option_env!("TARGET")', True),
    ('env!("CARGO_PKG_VERSION")', True),
    ("thread_local! { static X: Cell<u64> = Cell::new(0); }", True),
    ("static mut COUNTER: u64 = 0;", True),
    ('asm!("nop")', True),
    ("Instant::now()", True),
    ("let x: f64 = 0.5;", False),
    ("use std::collections::BTreeMap;", False),
    ("use std::sync::atomic::{AtomicU64, Ordering};", False),
    ("static LEAK: AtomicU64 = AtomicU64::new(0);", False),
    ("let zone = u64::MAX - (u64::MAX % n);", False),
]


def self_test() -> int:
    failures = 0
    for sample, must_hit in SELF_TEST:
        hit = any(re.search(p, sample) for p in DENYLIST)
        if hit != must_hit:
            failures += 1
            expected = "HIT" if must_hit else "MISS"
            actual = "HIT" if hit else "MISS"
            print(f"self-test FAIL (expected {expected}, got {actual}): {sample}")
    if failures:
        print(f"denylist self-test: {failures} failure(s)")
        return 1
    print(f"denylist self-test: PASS ({len(SELF_TEST)} samples)")
    return 0


def main() -> int:
    if "--self-test" in sys.argv:
        return self_test()

    violations: list[str] = []
    scanned = 0
    for crate in KERNEL_CRATES:
        crate_dir = REPO / crate
        if not crate_dir.is_dir():
            print(f"denylist: missing kernel crate {crate}", file=sys.stderr)
            return 2
        for path in sorted(crate_dir.rglob("*.rs")):
            scanned += 1
            text = path.read_text(encoding="utf-8")
            for lineno, line in enumerate(text.splitlines(), start=1):
                for pattern, reason in DENYLIST.items():
                    if re.search(pattern, line):
                        rel = path.relative_to(REPO)
                        violations.append(f"{rel}:{lineno}: [{pattern}] {reason}\n    {line.strip()}")

    if violations:
        print(f"determinism deny-list: {len(violations)} violation(s) in {scanned} file(s):")
        for v in violations:
            print(f"  {v}")
        return 1

    print(f"determinism deny-list: PASS ({scanned} kernel files scanned, 0 violations)")
    return 0


if __name__ == "__main__":
    sys.exit(main())
