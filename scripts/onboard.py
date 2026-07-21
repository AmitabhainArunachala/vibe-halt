#!/usr/bin/env python3
"""Session onboarding — truthful status of this checkout, nothing more.

READY is an aggregate verdict (PR #1 hardening loop): pinned toolchain
present and matching, deny-list self-test and scan green, governance file
parseable, WIP within limit, no ownership overlap between tracks. Any
failure is NOT READY — a session must not start on a broken floor.

What READY still does NOT prove: CI admission, merge approval, or that any
acceptance criterion in ACTIVE_TRACK.yaml currently holds. Declared intent
lives in the YAML; runtime truth comes only from running the gates
(`make gate`).
"""

from __future__ import annotations

import re
import subprocess
import sys
from pathlib import Path

# The strict-schema governance checker is the single implementation
# (hardening-loop-4 BLOCKER 4: the onboard-embedded yaml-lite parser
# absorbed nested metadata and compared uncanonicalized paths; it is
# gone). scripts/ is sys.path[0] when this file runs as a script.
import check_governance

REPO = Path(__file__).resolve().parent.parent


def run(cmd: list[str]) -> tuple[int, str]:
    try:
        proc = subprocess.run(cmd, cwd=REPO, capture_output=True, text=True, timeout=300)
        return proc.returncode, (proc.stdout + proc.stderr).strip()
    except FileNotFoundError:
        return 127, f"{cmd[0]}: not found"
    except subprocess.TimeoutExpired:
        return 124, f"{' '.join(cmd)}: timeout"


def pinned_channel() -> str | None:
    toolchain = REPO / "rust-toolchain.toml"
    if not toolchain.is_file():
        return None
    m = re.search(r'channel\s*=\s*"([^"]+)"', toolchain.read_text())
    return m.group(1) if m else None


def check_toolchain(problems: list[str]) -> None:
    channel = pinned_channel()
    if channel is None:
        problems.append("rust-toolchain.toml missing or has no channel pin")
        return
    code, out = run(["rustc", "--version"])
    if code != 0:
        problems.append(f"rustc unavailable ({out})")
    elif channel not in out:
        problems.append(f"rustc is not the pinned {channel}: {out}")
    else:
        print(f"  {out} (pinned {channel}: OK)")
    code, out = run(["cargo", "--version"])
    if code != 0:
        problems.append(f"cargo unavailable ({out})")
    else:
        print(f"  {out}")


def check_governance_status(problems: list[str]) -> None:
    """Delegate to the strict-schema checker and render the digest. A
    governance self-test failure is a broken floor, same as a parse
    problem."""
    if check_governance.self_test() != 0:
        problems.append("governance self-test failed")
    track_file = REPO / "docs" / "governance" / "ACTIVE_TRACK.yaml"
    if not track_file.is_file():
        problems.append("docs/governance/ACTIVE_TRACK.yaml missing")
        return
    wip_max, _, tracks, gov_problems = check_governance.validate(
        track_file.read_text(encoding="utf-8")
    )
    problems.extend(gov_problems)
    active = [t for t in tracks if t.status == "ACTIVE"]
    print(f"  tracks: {len(active)} ACTIVE (wip_max {wip_max})")
    for t in active:
        print(f"    - {t.track_id}")


def main() -> int:
    print("=== vibe-halt onboard ===\n")
    problems: list[str] = []

    # Checkout failures and detached HEAD are verdict problems, not just
    # printed curiosities (PR #1 hardening-loop-2 GAP): a session must not
    # start READY on a checkout it cannot even describe.
    print("Checkout:")
    branch = ""
    for label, cmd in [
        ("branch", ["git", "rev-parse", "--abbrev-ref", "HEAD"]),
        ("head", ["git", "rev-parse", "--short", "HEAD"]),
    ]:
        code, out = run(cmd)
        print(f"  {label}: {out if code == 0 else '(unavailable)'}")
        if code != 0:
            problems.append(f"git {label} unavailable ({out})")
        elif label == "branch":
            branch = out
    if branch == "HEAD":
        problems.append("detached HEAD — onboard requires a branch checkout")
    code, out = run(["git", "status", "--porcelain"])
    if code != 0:
        problems.append(f"git status unavailable ({out})")
    print(f"  dirty files: {len(out.splitlines()) if code == 0 and out else 0}")

    print("\nToolchain (pinned):")
    check_toolchain(problems)

    print("\nGovernance (docs/governance/ACTIVE_TRACK.yaml — declared intent):")
    check_governance_status(problems)

    print("\nGates (run now, mechanically):")
    code, out = run([sys.executable, "scripts/check_determinism_denylist.py", "--self-test"])
    print(f"  deny-list self-test: {'PASS' if code == 0 else 'FAIL'}")
    if code != 0:
        problems.append("deny-list self-test failed")
        print(out)
    code, out = run([sys.executable, "scripts/check_determinism_denylist.py"])
    print(f"  deny-list scan: {'PASS' if code == 0 else 'FAIL'}")
    if code != 0:
        problems.append("deny-list scan failed")
        print(out)

    print()
    if problems:
        print("Verdict: NOT READY")
        for p in problems:
            print(f"  - {p}")
        return 1
    print("Verdict: READY")
    print("Next: make gate   (full truth, not this digest)")
    return 0


if __name__ == "__main__":
    sys.exit(main())
