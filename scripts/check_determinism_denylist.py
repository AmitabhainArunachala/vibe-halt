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
DENYLIST: dict[str, str] = {
    r"std::time": "wall-clock time; use vh_core::VirtualClock",
    r"Instant::now": "wall-clock time; use vh_core::VirtualClock",
    r"SystemTime": "wall-clock time; use vh_core::VirtualClock",
    r"\bHashMap\b": "hash-order iteration; use BTreeMap",
    r"\bHashSet\b": "hash-order iteration; use BTreeSet",
    r"thread::spawn": "threads in the kernel; parallelism lives at the multiverse boundary",
    r"std::thread": "threads in the kernel; parallelism lives at the multiverse boundary",
    r"\brand\b\s*::": "external RNG; use vh_core::Xoshiro256pp streams",
    r"getrandom": "OS randomness; use the seed tree",
    r"std::env": "environment leakage; config enters via typed parameters",
    r"std::fs": "filesystem I/O in the kernel; Tier-2 I/O goes through the sandbox layer",
    r"std::net": "network I/O in the kernel; simulated network lands in Phase 1",
    r"std::process": "process control in the kernel; subprocess universes live in the Tier-2 sandbox",
}


def main() -> int:
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
