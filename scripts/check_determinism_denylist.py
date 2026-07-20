#!/usr/bin/env python3
"""Determinism deny-list gate (CI gate 0).

Kernel crates must be pure: no wall clock, no OS randomness, no hash-order
iteration, no threads, no I/O, no environment access, no global mutable
state, no unsafe code.

Honest scope statement (PR #1 hardening loop): the line-regex layer is
DEFENSE-IN-DEPTH, not semantic enforcement — aliasing and macros can evade
any regex. The semantic layer is `#![forbid(unsafe_code)]` (checked below,
enforced by rustc) plus the frozen reference vectors and the divergence
gate, which catch what the regex misses at the behavioral level. A
type-aware lint is planned for Phase 2.

Fail-closed classification: EVERY crate under crates/ is scanned unless it
is explicitly listed as a boundary crate. A new crate is kernel-grade by
default — nobody has to remember to register it.

Known, accepted regex miss (documented): float NaN/precision formatting —
`format!("{x}")` on an f64 is indistinguishable from other formatting at
line level.
"""

from __future__ import annotations

import re
import sys
from pathlib import Path

REPO = Path(__file__).resolve().parent.parent

# Boundary crates are exempt from the line scan (they own argv/exit codes).
# Everything else under crates/ is scanned — fail closed.
BOUNDARY_CRATES = {
    "crates/vh-cli",
}

# Per-file, per-pattern exemptions (never whole-file). PR #2 timing-boundary
# ruling: the vh-verify soak binary may use wall-clock time for telemetry
# that stays outside replay inputs and trace hashes; nothing else.
EXEMPT: dict[str, set[str]] = {
    "crates/vh-verify/src/main.rs": {r"std::time", r"Instant::now"},
}

# Pattern -> reason, applied to every scanned line.
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
    # Root-module aliasing defeats every literal std::<mod> rule below
    # (`use std as host; host::time::Instant::now()`), so alias the root
    # and you trip this instead (PR #1 hardening-loop BLOCKER).
    r"use\s+(std|core|alloc)\s+as\s+": "std/core root aliasing defeats the deny-list",
    r"extern\s+crate\s+(std|core|alloc)\s+as\s+": "std/core root aliasing defeats the deny-list",
    # Grouped imports would otherwise dodge the literal std::<mod> rules.
    r"use\s+std::\{[^}]*\b(fs|env|net|process|io|os|thread|time)\b": (
        "grouped import of a denied std module"
    ),
}

# Applied to src/ only: adversarial TEST fixtures legitimately use global
# atomics to simulate nondeterminism; production kernel code never may.
SRC_ONLY_DENYLIST: dict[str, str] = {
    r"\bAtomic(Bool|Ptr|I8|I16|I32|I64|Isize|U8|U16|U32|U64|Usize)\b": (
        "global atomic state in production kernel code; per-universe state lives in UniverseCtx"
    ),
}

# --self-test corpus: (sample line, must_hit_in_src, must_hit_in_tests).
SELF_TEST: list[tuple[str, bool, bool]] = [
    ('format!("{:p}", ptr)', True, True),
    ("RandomState::new()", True, True),
    ("std::io::stdin().read_line(&mut s)", True, True),
    ("std::os::unix::fs::MetadataExt::ino(&m)", True, True),
    ("use std::{fs, env, net};", True, True),
    ('option_env!("TARGET")', True, True),
    ('env!("CARGO_PKG_VERSION")', True, True),
    ("thread_local! { static X: Cell<u64> = Cell::new(0); }", True, True),
    ("static mut COUNTER: u64 = 0;", True, True),
    ('asm!("nop")', True, True),
    ("Instant::now()", True, True),
    ("use std as host;", True, True),
    ("extern crate std as host;", True, True),
    ("static LEAK: AtomicU64 = AtomicU64::new(0);", True, False),
    ("let x: f64 = 0.5;", False, False),
    ("use std::collections::BTreeMap;", False, False),
    ("use std::sync::atomic::Ordering;", False, False),
    ("let zone = u64::MAX - (u64::MAX % n);", False, False),
]


def patterns_for(rel_path: str) -> dict[str, str]:
    pats = dict(DENYLIST)
    if "tests" not in Path(rel_path).parts:
        pats.update(SRC_ONLY_DENYLIST)
    for exempt_pattern in EXEMPT.get(rel_path, set()):
        pats.pop(exempt_pattern, None)
    return pats


def kernel_crates() -> list[Path]:
    crates_dir = REPO / "crates"
    return sorted(
        d
        for d in crates_dir.iterdir()
        if d.is_dir() and f"crates/{d.name}" not in BOUNDARY_CRATES
    )


def check_manifest(crate_dir: Path) -> list[str]:
    """Mechanically reject external dependencies: every dependency of every
    crate must be a workspace-local path dependency."""
    violations = []
    manifest = crate_dir / "Cargo.toml"
    if not manifest.is_file():
        return [f"{manifest.relative_to(REPO)}: missing Cargo.toml"]
    in_deps = False
    for lineno, line in enumerate(manifest.read_text().splitlines(), start=1):
        stripped = line.strip()
        if stripped.startswith("["):
            in_deps = stripped in ("[dependencies]", "[dev-dependencies]", "[build-dependencies]")
            continue
        if in_deps and stripped and not stripped.startswith("#"):
            if "path =" not in stripped and "path=" not in stripped:
                violations.append(
                    f"{manifest.relative_to(REPO)}:{lineno}: external dependency "
                    f"in a hermetic workspace: {stripped}"
                )
    return violations


def check_forbid_unsafe(crate_dir: Path) -> list[str]:
    """The semantic layer: kernel crate roots must carry forbid(unsafe_code)."""
    lib = crate_dir / "src" / "lib.rs"
    if not lib.is_file():
        return []  # binary-only crate; bins are covered by the line scan
    if "#![forbid(unsafe_code)]" not in lib.read_text():
        return [f"{lib.relative_to(REPO)}: missing #![forbid(unsafe_code)]"]
    return []


def self_test() -> int:
    failures = 0
    for sample, expect_src, expect_test in SELF_TEST:
        for kind, expected in (("src", expect_src), ("tests", expect_test)):
            pseudo_path = f"crates/vh-x/{kind}/sample.rs"
            hit = any(re.search(p, sample) for p in patterns_for(pseudo_path))
            if hit != expected:
                failures += 1
                print(
                    f"self-test FAIL [{kind}] (expected {'HIT' if expected else 'MISS'}, "
                    f"got {'HIT' if hit else 'MISS'}): {sample}"
                )
    if failures:
        print(f"denylist self-test: {failures} failure(s)")
        return 1
    print(f"denylist self-test: PASS ({len(SELF_TEST)} samples x src/tests contexts)")
    return 0


def main() -> int:
    if "--self-test" in sys.argv:
        return self_test()

    crates = kernel_crates()
    if not crates:
        print("denylist: no kernel crates found under crates/ — fail closed", file=sys.stderr)
        return 2

    violations: list[str] = []
    scanned = 0
    for crate_dir in crates:
        violations.extend(check_manifest(crate_dir))
        violations.extend(check_forbid_unsafe(crate_dir))
        for path in sorted(crate_dir.rglob("*.rs")):
            rel = str(path.relative_to(REPO))
            pats = patterns_for(rel)
            scanned += 1
            for lineno, line in enumerate(
                path.read_text(encoding="utf-8").splitlines(), start=1
            ):
                for pattern, reason in pats.items():
                    if re.search(pattern, line):
                        violations.append(
                            f"{rel}:{lineno}: [{pattern}] {reason}\n    {line.strip()}"
                        )
    # Boundary crates still get the manifest check: zero external deps is
    # a workspace-wide doctrine, not a kernel-only one.
    for name in sorted(BOUNDARY_CRATES):
        violations.extend(check_manifest(REPO / name))

    if violations:
        print(f"determinism deny-list: {len(violations)} violation(s):")
        for v in violations:
            print(f"  {v}")
        return 1

    print(
        f"determinism deny-list: PASS ({len(crates)} kernel crates, {scanned} files, "
        "manifests hermetic, unsafe forbidden)"
    )
    return 0


if __name__ == "__main__":
    sys.exit(main())
