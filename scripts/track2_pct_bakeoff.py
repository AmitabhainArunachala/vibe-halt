#!/usr/bin/env python3
"""C2 kill-criterion bakeoff: PCT d=3 vs uniform-with-random-tiebreak.

Charter (docs/prompts/CONVERGENCE_CAMPAIGN_EXECUTOR_2026-07-22.md §4/C2):
kill PCT if it is no faster than uniform-with-random-tiebreak over 32
seeds at finding VB-006 (corpus-same-timestamp-race). Metric:
universes-to-first-finding (the index of the first failing universe)
within the budget. Lower is faster. The verdict line is machine-anchored;
this harness only measures and reports — it never blesses.
"""

import argparse
import re
import statistics
import subprocess
import sys

FAIL_RE = re.compile(r"^  FAIL universe (\d+):", re.M)


def first_finding(seed: int, schedule: str, budget: int) -> int | None:
    cmd = [
        "cargo", "run", "-q", "--locked", "--offline", "--release", "-p", "vh-cli", "--",
        "run", "--workload", "corpus-same-timestamp-race",
        "--seed", hex(seed), "--universes", str(budget),
        "--schedule", schedule,
    ]
    out = subprocess.run(cmd, capture_output=True, text=True).stdout
    hits = FAIL_RE.findall(out)
    return int(hits[0]) if hits else None


def main() -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument("--seeds", type=int, default=32)
    ap.add_argument("--budget", type=int, default=1000)
    ap.add_argument("--depth", type=int, default=3)
    args = ap.parse_args()

    base = 0xD1CE
    rows = []
    for i in range(args.seeds):
        seed = base + i
        pct = first_finding(seed, f"pct:{args.depth}", args.budget)
        uni = first_finding(seed, "uniform", args.budget)
        rows.append((seed, pct, uni))
        print(f"seed 0x{seed:x}: pct:{args.depth} first={pct} uniform first={uni}", flush=True)

    miss = args.budget  # censored value for not-found within budget
    pcts = [r[1] if r[1] is not None else miss for r in rows]
    unis = [r[2] if r[2] is not None else miss for r in rows]
    med_pct = statistics.median(pcts)
    med_uni = statistics.median(unis)
    wins = sum(1 for p, u in zip(pcts, unis) if p < u)
    losses = sum(1 for p, u in zip(pcts, unis) if p > u)
    ties = args.seeds - wins - losses
    print(
        f"OVERALL seeds={args.seeds} budget={args.budget} "
        f"median_pct={med_pct} median_uniform={med_uni} "
        f"pct_wins={wins} losses={losses} ties={ties}"
    )
    faster = med_pct < med_uni
    print(f"verdict: pct_faster_than_uniform={str(faster).lower()} (kill fires when false)")
    return 0


if __name__ == "__main__":
    sys.exit(main())
