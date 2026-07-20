#!/usr/bin/env python3
"""Session onboarding — truthful status of this checkout, nothing more.

Lean import of dharma_swarm's `make onboard`. What a READY verdict proves:
the local checkout builds a coherent session picture. What it does NOT
prove: CI admission, merge approval, or that any acceptance criterion in
ACTIVE_TRACK.yaml currently holds. Declared intent lives in the YAML;
runtime truth comes only from running the gates.
"""

from __future__ import annotations

import subprocess
import sys
from pathlib import Path

REPO = Path(__file__).resolve().parent.parent


def run(cmd: list[str]) -> tuple[int, str]:
    try:
        proc = subprocess.run(
            cmd, cwd=REPO, capture_output=True, text=True, timeout=120
        )
        return proc.returncode, (proc.stdout + proc.stderr).strip()
    except FileNotFoundError:
        return 127, f"{cmd[0]}: not found"
    except subprocess.TimeoutExpired:
        return 124, f"{' '.join(cmd)}: timeout"


def main() -> int:
    print("=== vibe-halt onboard ===\n")

    print("Checkout:")
    for label, cmd in [
        ("branch", ["git", "rev-parse", "--abbrev-ref", "HEAD"]),
        ("head", ["git", "rev-parse", "--short", "HEAD"]),
        ("dirty", ["git", "status", "--porcelain"]),
    ]:
        code, out = run(cmd)
        if label == "dirty":
            print(f"  dirty files: {len(out.splitlines()) if out else 0}")
        else:
            print(f"  {label}: {out if code == 0 else '(unavailable)'}")

    print("\nToolchain:")
    for cmd in (["rustc", "--version"], ["cargo", "--version"], ["python3", "--version"]):
        code, out = run(cmd)
        print(f"  {out if code == 0 else cmd[0] + ': MISSING'}")

    print("\nDeclared intent (docs/governance/ACTIVE_TRACK.yaml — not runtime truth):")
    track_file = REPO / "docs" / "governance" / "ACTIVE_TRACK.yaml"
    if track_file.is_file():
        interesting = ("- id:", "title:", "status:", "serves:")
        for line in track_file.read_text(encoding="utf-8").splitlines():
            stripped = line.strip()
            if stripped.startswith(interesting) and "spine" not in stripped:
                print(f"  {stripped}")
    else:
        print("  MISSING — governance file absent, session is NOT READY")
        return 1

    print("\nGates (run now, mechanically):")
    code, out = run(["python3", "scripts/check_determinism_denylist.py"])
    print(f"  deny-list: {'PASS' if code == 0 else 'FAIL'}")
    denylist_ok = code == 0
    if not denylist_ok:
        print(f"{out}\n")

    print("\nVerdict:", "READY" if denylist_ok else "NOT READY (fix gates above)")
    print("Next: cargo test --workspace && make gate   (full truth, not this digest)")
    return 0 if denylist_ok else 1


if __name__ == "__main__":
    sys.exit(main())
