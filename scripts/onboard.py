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


class Track:
    def __init__(self, track_id: str) -> None:
        self.track_id = track_id
        self.status = ""
        self.surfaces: list[str] = []


# Fail-closed status vocabulary: anything else is a governance problem,
# not a silent deactivation (PR #1 hardening-loop-2 GAP: `status:
# "ACTIVE"` and typo'd statuses used to silently drop a track from every
# WIP/overlap check).
KNOWN_STATUSES = {"ACTIVE", "PAUSED", "SHIPPED", "RETIRED"}


def _unquote(value: str) -> str:
    """YAML semantics for the scalar forms this file uses: matching
    single/double quotes wrap the same string."""
    if len(value) >= 2 and value[0] == value[-1] and value[0] in ("'", '"'):
        return value[1:-1]
    return value


def _scalar(stripped: str) -> str:
    return _unquote(stripped.split(":", 1)[1].split("#", 1)[0].strip())


def _surface_form_ok(surface: str) -> bool:
    """Only exact paths and a trailing '/**' are supported. Any other glob
    metacharacter would silently defeat the overlap check, so it is
    rejected instead of ignored."""
    body = surface[:-3] if surface.endswith("/**") else surface
    return body != "" and not any(c in body for c in "*?[]")


def parse_tracks(text: str) -> tuple[int, list[str], list[Track], list[str]]:
    """yaml-lite parse of ACTIVE_TRACK.yaml: wip_max, shared_surfaces,
    per-track id/status/owned_surfaces, plus fail-closed parse problems.
    Intentionally simple; the file's structure is owned by this repo."""
    wip_max = 0
    shared: list[str] = []
    tracks: list[Track] = []
    problems: list[str] = []
    top_key = ""
    section = ""
    for raw in text.splitlines():
        line = raw.rstrip()
        stripped = line.strip()
        if not stripped or stripped.startswith("#"):
            continue
        # Track the current top-level key so list entries are attributed to
        # the section they are actually in. The previous parser treated
        # EVERY '- id:' at two-space indent as a track, silently absorbing
        # spine_objectives entries as empty-status tracks (fail-open,
        # masked until statuses were validated).
        if not line[0].isspace():
            top_key = stripped.split(":", 1)[0]
            section = "shared" if top_key == "shared_surfaces" else ""
        if stripped.startswith("wip_max:"):
            try:
                wip_max = int(_scalar(stripped))
            except ValueError:
                problems.append(f"ACTIVE_TRACK.yaml: unparseable wip_max: {stripped!r}")
        elif section == "shared" and stripped.startswith("- "):
            shared.append(_unquote(stripped[2:].split("#", 1)[0].strip()))
        elif top_key == "tracks" and stripped.startswith("- id:") and line.startswith("  -"):
            tracks.append(Track(_scalar(stripped[2:])))
            section = "track"
        elif top_key == "tracks" and tracks and stripped.startswith("status:"):
            tracks[-1].status = _scalar(stripped)
        elif top_key == "tracks" and tracks and stripped == "owned_surfaces:":
            section = "surfaces"
        elif section == "surfaces" and stripped.startswith("- "):
            tracks[-1].surfaces.append(_unquote(stripped[2:].split("#", 1)[0].strip()))
        elif section == "surfaces" and not stripped.startswith(("- ", "#")):
            section = "track"

    seen_ids: set[str] = set()
    for t in tracks:
        if not t.track_id:
            problems.append("ACTIVE_TRACK.yaml: track with empty id")
        elif t.track_id in seen_ids:
            problems.append(f"ACTIVE_TRACK.yaml: duplicate track id {t.track_id}")
        seen_ids.add(t.track_id)
        if t.status not in KNOWN_STATUSES:
            problems.append(
                f"ACTIVE_TRACK.yaml: track {t.track_id}: unknown status "
                f"{t.status!r} (known: {sorted(KNOWN_STATUSES)})"
            )
        if t.status == "ACTIVE" and not t.surfaces:
            problems.append(
                f"ACTIVE_TRACK.yaml: ACTIVE track {t.track_id} owns no surfaces"
            )
        for s in t.surfaces:
            if not _surface_form_ok(s):
                problems.append(
                    f"ACTIVE_TRACK.yaml: track {t.track_id}: unsupported surface "
                    f"glob {s!r} (exact path or trailing /** only)"
                )
    for s in shared:
        if not _surface_form_ok(s):
            problems.append(
                f"ACTIVE_TRACK.yaml: unsupported shared surface glob {s!r}"
            )
    return wip_max, shared, tracks, problems


def normalize(surface: str) -> str:
    return surface[:-3] if surface.endswith("/**") else surface


def overlaps(a: str, b: str) -> bool:
    na, nb = normalize(a), normalize(b)
    return na == nb or na.startswith(nb + "/") or nb.startswith(na + "/")


def check_governance(problems: list[str]) -> None:
    track_file = REPO / "docs" / "governance" / "ACTIVE_TRACK.yaml"
    if not track_file.is_file():
        problems.append("docs/governance/ACTIVE_TRACK.yaml missing")
        return
    wip_max, shared, tracks, parse_problems = parse_tracks(
        track_file.read_text(encoding="utf-8")
    )
    problems.extend(parse_problems)
    if wip_max <= 0 or not tracks:
        problems.append("ACTIVE_TRACK.yaml unparseable (no wip_max or no tracks)")
        return
    active = [t for t in tracks if t.status == "ACTIVE"]
    print(f"  tracks: {len(active)} ACTIVE (wip_max {wip_max})")
    for t in active:
        print(f"    - {t.track_id}")
    if len(active) > wip_max:
        problems.append(f"WIP overflow: {len(active)} ACTIVE tracks > wip_max {wip_max}")
    for i, ta in enumerate(active):
        for tb in active[i + 1 :]:
            for sa in ta.surfaces:
                if sa in shared:
                    continue
                for sb in tb.surfaces:
                    if sb in shared:
                        continue
                    if overlaps(sa, sb):
                        problems.append(
                            f"ownership overlap: {ta.track_id}:{sa} vs {tb.track_id}:{sb}"
                        )


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
    check_governance(problems)

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
