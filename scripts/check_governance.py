#!/usr/bin/env python3
"""Strict-schema governance checker for docs/governance/ACTIVE_TRACK.yaml.

Hardening-loop-4 BLOCKER 4: the onboard-embedded parser matched stripped
field text at any indentation, so a nested `wip_max:`/`status:` inside a
track's metadata silently overrode the top-level declaration, and surface
overlap compared uncanonicalized lexical text (`crates/foo/**` vs
`./crates/foo/**` did not overlap). Governance was also invoked only by
onboarding — never by `make gate` or CI — so a PR could violate
ownership/WIP and stay green.

This checker is the single implementation (onboard.py imports it; the
central gate battery executes it):

* EXACT-INDENT schema: top-level keys at column 0, track entries at
  `  - id:`, track fields at indent 4, list items at indent 6. A field at
  any other indent is never absorbed — and unknown keys or unexpected
  nesting are rejected outright (fail closed), not ignored.
* CANONICAL surfaces: every surface is normalized to a repo-relative
  POSIX path (leading `./` stripped, `//` collapsed) before any overlap
  or shared-list comparison; absolute paths, `..` segments, and
  backslashes are rejected.
* Adversarial fixtures run under `--self-test`, including the exact
  reproduced bypasses.
"""

from __future__ import annotations

import posixpath
import sys
from pathlib import Path

REPO = Path(__file__).resolve().parent.parent
TRACK_FILE = REPO / "docs" / "governance" / "ACTIVE_TRACK.yaml"

KNOWN_STATUSES = {"ACTIVE", "PAUSED", "SHIPPED", "RETIRED"}

TOP_SCALAR_KEYS = {"version", "wip_warn", "wip_max"}
TOP_LIST_KEYS = {"shared_surfaces"}
TOP_ENTRY_KEYS = {"spine_objectives", "tracks"}
TRACK_SCALAR_FIELDS = {"id", "title", "status", "serves", "opened"}
TRACK_LIST_FIELDS = {"owned_surfaces", "acceptance", "next", "non_goals"}
SPINE_FIELDS = {"id", "summary"}
BLOCK_SCALAR_MARKERS = {">", "|", ">-", "|-", ">+", "|+"}


class Track:
    def __init__(self, track_id: str) -> None:
        self.track_id = track_id
        self.status = ""
        self.surfaces: list[str] = []


def _unquote(value: str) -> str:
    if len(value) >= 2 and value[0] == value[-1] and value[0] in ("'", '"'):
        return value[1:-1]
    return value


def _scalar(after_colon: str) -> str:
    return _unquote(after_colon.split("#", 1)[0].strip())


def _indent(line: str) -> int:
    return len(line) - len(line.lstrip(" "))


def canonicalize_surface(surface: str) -> tuple[str | None, str | None]:
    """Return (canonical_form, problem). Canonical form is a normalized
    repo-relative POSIX path, with a trailing `/**` marker preserved.
    Rejections fail closed: an unrepresentable surface must never enter
    the overlap check as inert text."""
    is_glob = surface.endswith("/**")
    body = surface[:-3] if is_glob else surface
    if body == "":
        return None, f"empty surface {surface!r}"
    if "\\" in body:
        return None, f"backslash in surface {surface!r} — POSIX paths only"
    if any(c in body for c in "*?[]"):
        return None, f"unsupported glob {surface!r} (exact path or trailing /** only)"
    if posixpath.isabs(body):
        return None, f"absolute surface path {surface!r} — repo-relative only"
    norm = posixpath.normpath(body)
    if norm == "." or norm == ".." or norm.startswith("../"):
        return None, f"surface {surface!r} escapes or names the repo root"
    return norm + ("/**" if is_glob else ""), None


def _normalize(surface: str) -> str:
    return surface[:-3] if surface.endswith("/**") else surface


def overlaps(a: str, b: str) -> bool:
    na, nb = _normalize(a), _normalize(b)
    return na == nb or na.startswith(nb + "/") or nb.startswith(na + "/")


def parse(text: str) -> tuple[int, list[str], list[Track], list[str]]:
    """Exact-indent parse of ACTIVE_TRACK.yaml. Returns
    (wip_max, shared_surfaces, tracks, problems). Every structural
    surprise is a problem, never a silent skip."""
    wip_max = 0
    shared: list[str] = []
    tracks: list[Track] = []
    problems: list[str] = []

    top_key = ""  # current top-level section
    track_list_field = ""  # open indent-6 list field of the current track
    block_scalar_indent: int | None = None  # content indent bound of an open block scalar

    def problem(lineno: int, msg: str) -> None:
        problems.append(f"ACTIVE_TRACK.yaml:{lineno}: {msg}")

    for lineno, raw in enumerate(text.splitlines(), start=1):
        line = raw.rstrip("\n")
        stripped = line.strip()
        indent = _indent(line)

        # Block scalar content is consumed purely by indentation.
        if block_scalar_indent is not None:
            if not stripped or indent > block_scalar_indent:
                continue
            block_scalar_indent = None

        if not stripped or stripped.startswith("#"):
            continue

        if indent == 0:
            track_list_field = ""
            if ":" not in stripped:
                problem(lineno, f"unrecognized top-level line {stripped!r}")
                top_key = ""
                continue
            key, _, rest = stripped.partition(":")
            top_key = key.strip()
            value = _scalar(rest)
            if top_key in TOP_SCALAR_KEYS:
                if top_key == "wip_max":
                    try:
                        wip_max = int(value)
                    except ValueError:
                        problem(lineno, f"unparseable wip_max {value!r}")
            elif top_key in TOP_LIST_KEYS or top_key in TOP_ENTRY_KEYS:
                if value:
                    problem(lineno, f"{top_key} must introduce a block, got {value!r}")
            else:
                problem(lineno, f"unknown top-level key {top_key!r} — strict schema")
            continue

        if top_key == "shared_surfaces":
            if indent == 2 and stripped.startswith("- "):
                shared.append(_unquote(stripped[2:].split("#", 1)[0].strip()))
            else:
                problem(lineno, f"unexpected line in shared_surfaces: {stripped!r}")
            continue

        if top_key == "spine_objectives":
            if indent == 2 and stripped.startswith("- id:"):
                continue
            if indent == 4 and ":" in stripped and not stripped.startswith("- "):
                key, _, rest = stripped.partition(":")
                if key.strip() not in SPINE_FIELDS:
                    problem(lineno, f"unknown spine_objectives field {key.strip()!r}")
                if _scalar(rest) in BLOCK_SCALAR_MARKERS:
                    block_scalar_indent = indent
                continue
            problem(lineno, f"unexpected line in spine_objectives: {stripped!r}")
            continue

        if top_key == "tracks":
            if indent == 2 and stripped.startswith("- "):
                track_list_field = ""
                if stripped.startswith("- id:"):
                    tracks.append(Track(_scalar(stripped[2:].partition(":")[2])))
                else:
                    problem(lineno, f"track entry must start with '- id:', got {stripped!r}")
                    tracks.append(Track(""))
                continue
            if not tracks:
                problem(lineno, f"track field before any '- id:' entry: {stripped!r}")
                continue
            if indent == 4 and ":" in stripped and not stripped.startswith("- "):
                key, _, rest = stripped.partition(":")
                key = key.strip()
                value = _scalar(rest)
                track_list_field = ""
                if key in TRACK_LIST_FIELDS:
                    if value:
                        problem(lineno, f"{key} must introduce a list, got {value!r}")
                    track_list_field = key
                elif key in TRACK_SCALAR_FIELDS:
                    if value in BLOCK_SCALAR_MARKERS:
                        block_scalar_indent = indent
                    elif key == "status":
                        tracks[-1].status = value
                else:
                    problem(
                        lineno,
                        f"unknown track field {key!r} at indent 4 — strict "
                        "schema (nested metadata is rejected, never absorbed)",
                    )
                continue
            if indent == 6 and stripped.startswith("- ") and track_list_field:
                if track_list_field == "owned_surfaces":
                    tracks[-1].surfaces.append(
                        _unquote(stripped[2:].split("#", 1)[0].strip())
                    )
                continue
            problem(
                lineno,
                f"unexpected structure in tracks at indent {indent}: {stripped!r} "
                "— strict schema rejects unmodeled nesting",
            )
            continue

        problem(lineno, f"unexpected line under {top_key!r}: {stripped!r}")

    return wip_max, shared, tracks, problems


def validate(text: str) -> tuple[int, list[str], list[Track], list[str]]:
    """Parse + semantic validation. Surfaces in returned tracks/shared are
    canonicalized. Problems cover schema, vocabulary, WIP, and canonical
    ownership overlap."""
    wip_max, shared_raw, tracks, problems = parse(text)

    shared: list[str] = []
    for s in shared_raw:
        canon, err = canonicalize_surface(s)
        if err:
            problems.append(f"ACTIVE_TRACK.yaml: shared surface: {err}")
        else:
            shared.append(canon)

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
        canonical_surfaces: list[str] = []
        for s in t.surfaces:
            canon, err = canonicalize_surface(s)
            if err:
                problems.append(f"ACTIVE_TRACK.yaml: track {t.track_id}: {err}")
            else:
                canonical_surfaces.append(canon)
        t.surfaces = canonical_surfaces
        if t.status == "ACTIVE" and not t.surfaces:
            problems.append(
                f"ACTIVE_TRACK.yaml: ACTIVE track {t.track_id} owns no surfaces"
            )

    if wip_max <= 0:
        problems.append("ACTIVE_TRACK.yaml: missing or nonpositive top-level wip_max")
    if not tracks:
        problems.append("ACTIVE_TRACK.yaml: no tracks parsed")

    active = [t for t in tracks if t.status == "ACTIVE"]
    if wip_max > 0 and len(active) > wip_max:
        problems.append(
            f"WIP overflow: {len(active)} ACTIVE tracks > wip_max {wip_max}"
        )
    shared_set = set(shared)
    for i, ta in enumerate(active):
        for tb in active[i + 1 :]:
            for sa in ta.surfaces:
                if sa in shared_set:
                    continue
                for sb in tb.surfaces:
                    if sb in shared_set:
                        continue
                    if overlaps(sa, sb):
                        problems.append(
                            f"ownership overlap: {ta.track_id}:{sa} vs {tb.track_id}:{sb}"
                        )
    return wip_max, shared, tracks, problems


# ---------------------------------------------------------------------------
# Self-test: adversarial fixtures, including the exact reproduced bypasses.

GOOD = """\
version: 1
wip_max: 2
shared_surfaces:
  - Cargo.toml
spine_objectives:
  - id: truth
    summary: >
      block scalar content is skipped, even lines like
      status: RETIRED
      wip_max: 99
tracks:
  - id: a
    title: "track a"
    status: ACTIVE
    owned_surfaces:
      - crates/foo/**
    acceptance:
      - "criterion: with a colon"
  - id: b
    status: ACTIVE
    owned_surfaces:
      - crates/bar/**
"""

NESTED_OVERRIDE = """\
version: 1
wip_max: 1
tracks:
  - id: a
    status: ACTIVE
    metadata:
      wip_max: 99
      status: RETIRED
    owned_surfaces:
      - crates/foo/**
"""

DOTTED_ALIAS_OVERLAP = """\
version: 1
wip_max: 3
tracks:
  - id: a
    status: ACTIVE
    owned_surfaces:
      - crates/foo/**
  - id: b
    status: ACTIVE
    owned_surfaces:
      - ./crates/foo/**
"""

DOUBLE_SLASH_OVERLAP = """\
version: 1
wip_max: 3
tracks:
  - id: a
    status: ACTIVE
    owned_surfaces:
      - crates//foo
  - id: b
    status: ACTIVE
    owned_surfaces:
      - crates/foo/**
"""


def _fixture(text: str, old: str, new: str) -> str:
    return text.replace(old, new)


def self_test() -> int:
    failures = 0

    def check(label: str, cond: bool, detail: object = "") -> None:
        nonlocal failures
        if not cond:
            failures += 1
            print(f"governance self-test FAIL: {label} {detail}")

    wip_max, shared, tracks, problems = validate(GOOD)
    check("good fixture parses clean", not problems, problems)
    check("good fixture wip_max", wip_max == 2, wip_max)
    check("good fixture shared canonical", shared == ["Cargo.toml"], shared)
    check("good fixture track count", len(tracks) == 2, len(tracks))

    # Reproduced bypass: nested metadata must be REJECTED, and must never
    # override the top-level wip_max or the track's declared status.
    wip_max, _, tracks, problems = validate(NESTED_OVERRIDE)
    check("nested metadata is rejected", bool(problems), "no problems raised")
    check("nested wip_max not absorbed", wip_max == 1, wip_max)
    check(
        "nested status not absorbed",
        tracks and tracks[0].status == "ACTIVE",
        tracks[0].status if tracks else "no tracks",
    )

    # Reproduced bypass: ./-prefixed and //-doubled paths must overlap
    # after canonicalization.
    for label, fixture in (
        ("./ alias overlap detected", DOTTED_ALIAS_OVERLAP),
        ("// collapse overlap detected", DOUBLE_SLASH_OVERLAP),
    ):
        _, _, _, problems = validate(fixture)
        check(label, any("ownership overlap" in p for p in problems), problems)

    # Vocabulary, WIP, and form checks stay fail-closed.
    cases = [
        ("quoted ACTIVE recognized", _fixture(GOOD, "status: ACTIVE", 'status: "ACTIVE"'), False),
        ("typo status rejected", _fixture(GOOD, "status: ACTIVE", "status: ACTIV"), True),
        ("duplicate id rejected", _fixture(GOOD, "- id: b", "- id: a"), True),
        ("wildcard surface rejected", _fixture(GOOD, "crates/foo/**", "crates/*"), True),
        ("absolute surface rejected", _fixture(GOOD, "crates/foo/**", "/etc/foo"), True),
        ("escaping surface rejected", _fixture(GOOD, "crates/foo/**", "../outside"), True),
        ("wip overflow rejected", _fixture(GOOD, "wip_max: 2", "wip_max: 1"), True),
        ("unknown top-level key rejected", GOOD + "surprise: 1\n", True),
        (
            "empty ACTIVE ownership rejected",
            GOOD.replace("      - crates/bar/**\n", ""),
            True,
        ),
        (
            "unquoted-typo'd nested list rejected",
            _fixture(GOOD, "    owned_surfaces:", "    owned_surface:"),
            True,
        ),
    ]
    for label, fixture, expect_problems in cases:
        _, _, _, problems = validate(fixture)
        check(label, bool(problems) == expect_problems, problems)

    if failures:
        print(f"governance self-test: {failures} failure(s)")
        return 1
    print("governance self-test: PASS (4 bypass reproductions, 10 schema cases)")
    return 0


def main() -> int:
    if "--self-test" in sys.argv:
        return self_test()
    if not TRACK_FILE.is_file():
        print("governance: docs/governance/ACTIVE_TRACK.yaml missing — fail closed")
        return 1
    _, _, tracks, problems = validate(TRACK_FILE.read_text(encoding="utf-8"))
    if problems:
        print(f"governance: {len(problems)} problem(s):")
        for p in problems:
            print(f"  {p}")
        return 1
    active = sum(1 for t in tracks if t.status == "ACTIVE")
    print(
        f"governance: PASS ({len(tracks)} tracks, {active} ACTIVE, strict "
        "schema, canonical ownership, WIP within limit)"
    )
    return 0


if __name__ == "__main__":
    sys.exit(main())
