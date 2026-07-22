#!/usr/bin/env python3
"""Track-2 non-blocking A/B harness for `--palette swarm`.

Measures the R2 acceptance shape from INTEGRATION_ROADMAP.md:
for each seeded corpus class, compare the smallest universe budget at
which v0 vs swarm reaches the pinned recall count. By default this is a
report-only harness; pass --enforce to make the acceptance threshold an
exit-code gate after the numbers justify doing so.
"""

from __future__ import annotations

import argparse
import math
import re
import subprocess
import sys
from dataclasses import dataclass
from pathlib import Path

REPO = Path(__file__).resolve().parent.parent
VH_BIN = REPO / "target" / "debug" / "vh"

PINNED = [
    ("corpus-lost-update", 29),
    ("corpus-retry-double-apply", 76),
    ("corpus-dirty-read", 83),
    ("corpus-crash-toctou", 21),
    ("corpus-fsync-lie", 21),
]

SUMMARY_RE = re.compile(r"always-failures:\s+(\d+)\s+universe")
FAIL_RE = re.compile(r"FAIL universe\s+(\d+):")


@dataclass(frozen=True)
class Measurement:
    failures_at_max: int
    first_detection: int | None
    budget_to_target: int | None


def parse_seed(text: str) -> int:
    return int(text, 16) if text.startswith("0x") else int(text)


def run(cmd: list[str]) -> subprocess.CompletedProcess[str]:
    return subprocess.run(cmd, cwd=REPO, text=True, capture_output=True, check=False)


def ensure_binary() -> None:
    proc = run(["cargo", "build", "-q", "--locked", "--offline", "-p", "vh-cli"])
    if proc.returncode != 0:
        sys.stderr.write(proc.stdout)
        sys.stderr.write(proc.stderr)
        raise SystemExit(proc.returncode)


def failures_for(
    workload: str,
    seed: int,
    palette: str,
    universes: int,
    divergence_check: bool,
) -> tuple[int, list[int]]:
    cmd = [
        str(VH_BIN),
        "run",
        "--workload",
        workload,
        "--seed",
        f"0x{seed:x}",
        "--universes",
        str(universes),
        "--palette",
        palette,
    ]
    if not divergence_check:
        cmd.append("--no-divergence-check")
    proc = run(cmd)
    if proc.returncode not in (0, 1, 3):
        sys.stderr.write(proc.stdout)
        sys.stderr.write(proc.stderr)
        raise RuntimeError(f"{' '.join(cmd)} exited {proc.returncode}")
    m = SUMMARY_RE.search(proc.stdout)
    if not m:
        raise RuntimeError(f"missing always-failures summary in:\n{proc.stdout}")
    listed = [int(x) for x in FAIL_RE.findall(proc.stdout)]
    return int(m.group(1)), listed


def measure(
    workload: str,
    seed: int,
    palette: str,
    target: int,
    max_budget: int,
    divergence_check: bool,
) -> Measurement:
    failures, listed = failures_for(workload, seed, palette, max_budget, divergence_check)
    first_detection = min(listed) + 1 if listed else None
    if failures < target:
        return Measurement(failures, first_detection, None)

    lo, hi = 1, max_budget
    while lo < hi:
        mid = (lo + hi) // 2
        got, _ = failures_for(workload, seed, palette, mid, divergence_check)
        if got >= target:
            hi = mid
        else:
            lo = mid + 1
    return Measurement(failures, first_detection, lo)


def median(values: list[float]) -> float | None:
    if not values:
        return None
    xs = sorted(values)
    mid = len(xs) // 2
    if len(xs) % 2:
        return xs[mid]
    return (xs[mid - 1] + xs[mid]) / 2.0


def main() -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument("--seeds", type=int, default=16)
    ap.add_argument("--seed0", type=parse_seed, default=0xD1CE)
    ap.add_argument("--max-budget", type=int, default=100)
    ap.add_argument("--speedup", type=float, default=0.25)
    ap.add_argument("--classes-required", type=int, default=4)
    ap.add_argument("--enforce", action="store_true")
    ap.add_argument(
        "--no-divergence-check",
        action="store_true",
        help="faster exploratory report; acceptance runs should omit this",
    )
    args = ap.parse_args()

    if args.seeds <= 0 or args.max_budget <= 0:
        raise SystemExit("--seeds and --max-budget must be positive")

    ensure_binary()
    divergence_check = not args.no_divergence_check
    class_passes = 0
    print(
        "workload\tseed\tpalette\ttarget\tfirst_detection\tbudget_to_target\tfailures_at_max"
    )
    for workload, target in PINNED:
        ratios: list[float] = []
        wins = 0
        comparable = 0
        for offset in range(args.seeds):
            seed = args.seed0 + offset
            v0 = measure(workload, seed, "v0", target, args.max_budget, divergence_check)
            swarm = measure(workload, seed, "swarm", target, args.max_budget, divergence_check)
            for palette, m in (("v0", v0), ("swarm", swarm)):
                print(
                    f"{workload}\t0x{seed:x}\t{palette}\t{target}\t"
                    f"{m.first_detection or 'NA'}\t{m.budget_to_target or 'NA'}\t"
                    f"{m.failures_at_max}"
                )
            if v0.budget_to_target is not None:
                comparable += 1
                if swarm.budget_to_target is not None:
                    ratio = swarm.budget_to_target / v0.budget_to_target
                    ratios.append(ratio)
                    if swarm.budget_to_target <= math.ceil(v0.budget_to_target * args.speedup):
                        wins += 1
        med = median(ratios)
        # Class pass: median comparable seed reaches the configured speedup.
        class_pass = med is not None and med <= args.speedup
        if class_pass:
            class_passes += 1
        print(
            f"SUMMARY\t{workload}\tclass_pass={class_pass}\t"
            f"median_swarm_over_v0={med if med is not None else 'NA'}\t"
            f"wins={wins}/{comparable}\tspeedup_threshold={args.speedup}"
        )
    print(
        f"OVERALL\tclass_passes={class_passes}/{len(PINNED)}\t"
        f"required={args.classes_required}\tenforced={args.enforce}"
    )
    if args.enforce and class_passes < args.classes_required:
        return 1
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
