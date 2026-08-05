#!/usr/bin/env python3
"""Mechanical acceptance evaluator for the vibe-halt GitHub Project.

THE RULE: acceptance terminals are DERIVED from content state plus board
values, never stored, and lifecycle Status=Done NEVER implies Accepted.

Terminals:
  Accepted          content CLOSED/MERGED + Gate state=Satisfied + non-empty
                    Acceptance record + zero violations; always scoped to the
                    item's declared Claim boundary — acceptance is of the
                    bounded claim only
  NullResult        content CLOSED + Acceptance record beginning "null:" —
                    the predeclared honorable null
  ClosedUnaccepted  content CLOSED/MERGED with Gate state != Satisfied — the
                    honest disposition for parked/rejected/superseded work,
                    never an error by itself
  Open              content OPEN
  Violation         any V code below; dominates every other terminal

Violation codes:
  V1  gate-pending flavor mismatches the Authority class (Human pending ->
      Human merge, Operator pending -> Operator approval, External pending ->
      External confirmation; other gate values exempt)
  V2  Gate state=Ready while Blocked-by holds a live #NN ref to an item OPEN
      in the export (refs in `constraint:`/`decides:` segments and refs
      annotated `(satisfied YYYY-MM-DD)` are ignored)
  V3  Gate state=Satisfied without a non-empty Acceptance record
  V4  Evidence=CI green without a non-empty Acceptance record (an
      `excluded:` record counts only when it also carries a resolvable
      run/SHA/URL reference)
  V5  Evidence=External proof whose record cannot witness an artifact outside
      the single-account lineage. FAIL-CLOSED HEURISTIC: the record must
      carry an `external:` segment marker or a URL whose host is not
      github.com; an empty record, bare prose, or a record made solely of
      github.com/AmitabhainArunachala URLs/SHAs fails
  V6  Gate state=Satisfied under Authority=Operator approval / External
      confirmation without the matching `operator:` / `external:` record
      marker — CI-run/merge references never satisfy these authority classes
  V7  boundary footprint (--diff-paths + --item only, under Claim
      boundary=Governance): docs/**, top-level *.md, LICENSE, and
      .github/ISSUE_TEMPLATE/** are clean; scripts/*.py, Makefile, and
      .github/workflows/** are a W-boundary WARNING; crates/**, clients/**,
      corpus/** are a VIOLATION; any other path fails closed to VIOLATION

Acceptance-record prefix grammar (four typed prefixes):
  null:      predeclared null publication -> NullResult on closed content
  excluded:  disposition note on an unaccepted closure -> ClosedUnaccepted on
             closed content; counts as a record for V4 only when it also
             carries a resolvable run/SHA/URL reference; NEVER counts toward
             Accepted, the `operator:`/`external:` authority classes, or
             V5's external-artifact requirement
  operator:  typed Operator-approval authority marker required by V6
  external:  typed External-confirmation authority marker required by V6/V5

Warnings (non-fatal): W-stale (Blocked-by ref to a CLOSED/MERGED item
lacking the `(satisfied ...)` marker), W-cycle (naive-parse pairwise A<->B
dependency cycles), W-boundary (V7 middle tier).

LIMITATION: authority records are checked for presence, shape, and location
only; single-account estates cannot mechanically authenticate WHO authored a
record — authorship attestation is out-of-band.

Exit codes: 0 no violations (warnings allowed), 1 self-test failure,
2 violations present, 3 input/transport error. --live performs one read-only
GraphQL query via gh; this script never writes to GitHub. --self-test runs
embedded in-memory fixtures only (hermetic, no network, no filesystem).
"""

from __future__ import annotations

import argparse
import json
import re
import subprocess
import sys
from collections import Counter
from dataclasses import dataclass
from pathlib import Path

OWNER = "AmitabhainArunachala"
PROJECT_NUMBER = 1

PENDING_AUTHORITY = {
    "Human pending": "Human merge",
    "Operator pending": "Operator approval",
    "External pending": "External confirmation",
}

AUTHORITY_MARKERS = {
    "Operator approval": "operator:",
    "External confirmation": "external:",
}

REF_PATTERN = re.compile(r"#(\d+)(\s*\(satisfied\s+\d{4}-\d{2}-\d{2}\))?")
URL_HOST_PATTERN = re.compile(r"https?://([^/\s]+)")
SCRIPTS_PY_PATTERN = re.compile(r"scripts/[^/]+\.py")
RESOLVABLE_REF_PATTERN = re.compile(r"\b(?:[0-9a-f]{7,40}|\d{6,})\b|https?://")

LIMITATION = (
    "authority records are checked for presence, shape, and location only; "
    "single-account estates cannot mechanically authenticate WHO authored a "
    "record — authorship attestation is out-of-band."
)

GRAPHQL_QUERY = """\
query($owner: String!, $number: Int!) {
  user(login: $owner) {
    projectV2(number: $number) {
      id
      title
      fields(first: 50) {
        pageInfo { hasNextPage }
        nodes {
          __typename
          ... on ProjectV2FieldCommon { name dataType }
          ... on ProjectV2SingleSelectField { options { name } }
        }
      }
      items(first: 100) {
        pageInfo { hasNextPage }
        nodes {
          content {
            __typename
            ... on Issue { number state title }
            ... on PullRequest { number state title merged }
            ... on DraftIssue { title }
          }
          fieldValues(first: 50) {
            pageInfo { hasNextPage }
            nodes {
              __typename
              ... on ProjectV2ItemFieldTextValue {
                text
                field { ... on ProjectV2FieldCommon { name } }
              }
              ... on ProjectV2ItemFieldSingleSelectValue {
                name
                field { ... on ProjectV2FieldCommon { name } }
              }
              ... on ProjectV2ItemFieldDateValue {
                date
                field { ... on ProjectV2FieldCommon { name } }
              }
            }
          }
        }
      }
    }
  }
}
"""


class InputError(Exception):
    pass


@dataclass
class ItemResult:
    n: int
    state: str
    status: str
    terminal: str
    violations: list[tuple[str, str]]
    warnings: list[tuple[str, str]]


def _val(values: dict, key: str) -> str:
    v = values.get(key)
    return v.strip() if isinstance(v, str) else ""


def parse_refs(blocked_by: str) -> list[tuple[int, bool]]:
    """(#NN, satisfied?) pairs. `constraint:`/`decides:` segments carry
    scope/disposition notes, not dependencies, so their refs never count."""
    refs: list[tuple[int, bool]] = []
    for segment in blocked_by.split(";"):
        if segment.strip().lower().startswith(("constraint:", "decides:")):
            continue
        for m in REF_PATTERN.finditer(segment):
            refs.append((int(m.group(1)), m.group(2) is not None))
    return refs


def _is_excluded(record: str) -> bool:
    return record.lower().startswith("excluded:")


def _has_marker(record: str, marker: str) -> bool:
    if _is_excluded(record):
        return False
    return any(seg.strip().lower().startswith(marker) for seg in record.split(";"))


def _references_external(record: str) -> bool:
    if _is_excluded(record):
        return False
    if _has_marker(record, "external:"):
        return True
    for m in URL_HOST_PATTERN.finditer(record):
        host = m.group(1).lower()
        if host != "github.com" and not host.endswith(".github.com"):
            return True
    return False


def _classify_path(path: str) -> str:
    p = path.strip().removeprefix("./")
    if (
        p.startswith(("docs/", ".github/ISSUE_TEMPLATE/"))
        or p == "LICENSE"
        or ("/" not in p and p.endswith(".md"))
    ):
        return "prose"
    if (
        p == "Makefile"
        or p.startswith(".github/workflows/")
        or SCRIPTS_PY_PATTERN.fullmatch(p)
    ):
        return "coordination"
    if p.startswith(("crates/", "clients/", "corpus/")):
        return "kernel"
    return "unclassified"


def evaluate_item(
    item: dict, states: dict[int, str], diff_paths: list[str] | None
) -> ItemResult:
    values = item.get("values") or {}
    n = item["n"]
    state = item["state"]
    gate = _val(values, "Gate state")
    evidence = _val(values, "Evidence")
    authority = _val(values, "Authority")
    status = _val(values, "Status")
    record = _val(values, "Acceptance record")
    boundary = _val(values, "Claim boundary")

    violations: list[tuple[str, str]] = []
    warnings: list[tuple[str, str]] = []

    expected_authority = PENDING_AUTHORITY.get(gate)
    if expected_authority is not None and authority != expected_authority:
        violations.append(
            (
                "V1",
                f"Gate state={gate!r} requires Authority={expected_authority!r}, "
                f"found {authority!r}",
            )
        )

    for ref, satisfied in parse_refs(_val(values, "Blocked by")):
        ref_state = states.get(ref)
        if satisfied or ref_state is None:
            continue
        if ref_state == "OPEN":
            if gate == "Ready":
                violations.append(
                    (
                        "V2",
                        f"Gate state=Ready with live dependency #{ref} OPEN in the export",
                    )
                )
        else:
            warnings.append(
                (
                    "W-stale",
                    f"Blocked-by ref #{ref} is {ref_state} but lacks a "
                    "`(satisfied YYYY-MM-DD)` marker",
                )
            )

    if gate == "Satisfied" and not record:
        violations.append(("V3", "Gate state=Satisfied with no Acceptance record"))
    if evidence == "CI green":
        if not record:
            violations.append(("V4", "Evidence=CI green with no Acceptance record"))
        elif _is_excluded(record) and not RESOLVABLE_REF_PATTERN.search(record):
            violations.append(
                (
                    "V4",
                    "Evidence=CI green with an `excluded:` record lacking a "
                    "resolvable run/SHA/URL reference",
                )
            )
    if evidence == "External proof" and not _references_external(record):
        violations.append(
            (
                "V5",
                "Evidence=External proof but the Acceptance record carries no "
                "`external:` marker and no non-github.com URL "
                "(single-account lineage — fail closed)",
            )
        )
    marker = AUTHORITY_MARKERS.get(authority)
    if gate == "Satisfied" and marker is not None and not _has_marker(record, marker):
        violations.append(
            (
                "V6",
                f"Gate state=Satisfied under Authority={authority} without the "
                f"`{marker}` record marker (CI-run/merge references never satisfy "
                "this authority class)",
            )
        )

    if diff_paths is not None and boundary == "Governance":
        for path in diff_paths:
            tier = _classify_path(path)
            if tier == "coordination":
                warnings.append(
                    (
                        "W-boundary",
                        f"changed path {path!r} is executable-coordination tier "
                        "under a Governance claim",
                    )
                )
            elif tier == "kernel":
                violations.append(
                    (
                        "V7",
                        f"changed path {path!r} is kernel/product tier under a "
                        "Governance claim",
                    )
                )
            elif tier == "unclassified":
                violations.append(
                    (
                        "V7",
                        f"changed path {path!r} matches no Governance footprint "
                        "tier — fail closed",
                    )
                )

    if violations:
        terminal = "Violation"
    elif state == "OPEN":
        terminal = "Open"
    elif state == "CLOSED" and record.lower().startswith("null:"):
        terminal = "NullResult"
    elif _is_excluded(record):
        terminal = "ClosedUnaccepted"
    elif gate == "Satisfied" and record:
        terminal = f"Accepted ({boundary or 'undeclared boundary'}-bounded)"
    else:
        terminal = "ClosedUnaccepted"

    return ItemResult(n, state, status, terminal, violations, warnings)


def _cycle_pairs(items: list[dict]) -> list[tuple[int, int]]:
    deps: dict[int, set[int]] = {}
    for item in items:
        refs = parse_refs(_val(item.get("values") or {}, "Blocked by"))
        deps[item["n"]] = {ref for ref, satisfied in refs if not satisfied}
    pairs: list[tuple[int, int]] = []
    for a in sorted(deps):
        for b in sorted(deps[a]):
            if b > a and a in deps.get(b, set()):
                pairs.append((a, b))
    return pairs


def evaluate_export(
    export: dict,
    diff_item: int | None = None,
    diff_paths: list[str] | None = None,
) -> list[ItemResult]:
    items = export.get("items") or []
    states = {item["n"]: item["state"] for item in items}
    results = []
    for item in sorted(items, key=lambda i: i["n"]):
        paths = diff_paths if diff_item == item["n"] else None
        results.append(evaluate_item(item, states, paths))
    by_n = {r.n: r for r in results}
    for a, b in _cycle_pairs(items):
        by_n[a].warnings.append(
            ("W-cycle", f"pairwise Blocked-by cycle #{a}<->#{b} (naive parse)")
        )
    return results


def print_report(results: list[ItemResult], source: str) -> bool:
    print(f"project-acceptance: {len(results)} item(s) evaluated from {source}")
    print(
        "rule: terminal states are derived, never stored; "
        "Status=Done never implies Accepted"
    )
    print(f"limitation: {LIMITATION}")
    print()
    for r in results:
        print(f"#{r.n} [{r.state}] Status={r.status or '-'} -> {r.terminal}")
    violations = [(r.n, c, m) for r in results for c, m in r.violations]
    warnings = [(r.n, c, m) for r in results for c, m in r.warnings]
    if violations:
        print()
        for n, code, msg in violations:
            print(f"VIOLATION {code} item #{n}: {msg}")
    if warnings:
        print()
        for n, code, msg in warnings:
            print(f"WARNING {code} item #{n}: {msg}")
    counts = Counter(r.terminal.split(" (")[0] for r in results)
    tally = ", ".join(f"{counts[t]} {t}" for t in sorted(counts))
    print()
    print(
        f"summary: {tally}; {len(violations)} violation(s), {len(warnings)} warning(s)"
    )
    return bool(violations)


def validate_export(data: object) -> dict:
    if not isinstance(data, dict) or not isinstance(data.get("items"), list):
        raise InputError("export must be an object with an `items` list")
    if not data["items"]:
        raise InputError("export contains zero items — fail closed")
    for item in data["items"]:
        if not isinstance(item, dict) or not isinstance(item.get("n"), int):
            raise InputError(f"item without integer `n`: {item!r}")
        if item.get("state") not in ("OPEN", "CLOSED", "MERGED"):
            raise InputError(
                f"item #{item['n']}: state {item.get('state')!r} not in "
                "OPEN/CLOSED/MERGED — fail closed"
            )
        if not isinstance(item.get("values"), dict):
            raise InputError(f"item #{item['n']}: missing `values` object")
    return data


def load_export(path: str) -> dict:
    try:
        raw = Path(path).read_text(encoding="utf-8")
    except OSError as e:
        raise InputError(f"cannot read export {path}: {e}") from e
    try:
        data = json.loads(raw)
    except json.JSONDecodeError as e:
        raise InputError(f"export {path} is not valid JSON: {e}") from e
    return validate_export(data)


def _require_complete(connection: dict, label: str) -> None:
    if ((connection.get("pageInfo") or {}).get("hasNextPage")) is not False:
        raise InputError(
            f"{label} page truncated or pageInfo missing — refusing a partial export"
        )


def load_live() -> dict:
    cmd = [
        "gh",
        "api",
        "graphql",
        "-f",
        f"query={GRAPHQL_QUERY}",
        "-F",
        f"owner={OWNER}",
        "-F",
        f"number={PROJECT_NUMBER}",
    ]
    try:
        proc = subprocess.run(cmd, capture_output=True, text=True, timeout=120)
    except (OSError, subprocess.TimeoutExpired) as e:
        raise InputError(f"gh transport failure: {e}") from e
    if proc.returncode != 0:
        raise InputError(
            f"gh api graphql exited {proc.returncode}: {proc.stderr.strip()}"
        )
    try:
        payload = json.loads(proc.stdout)
    except json.JSONDecodeError as e:
        raise InputError(f"gh returned non-JSON: {e}") from e
    project = ((payload.get("data") or {}).get("user") or {}).get("projectV2")
    if not project:
        raise InputError("GraphQL response has no projectV2 (auth or permissions?)")
    _require_complete(project.get("fields") or {}, "fields")
    _require_complete(project.get("items") or {}, "items")
    fields = []
    for fnode in (project["fields"].get("nodes") or []):
        options = fnode.get("options")
        fields.append(
            {
                "name": fnode.get("name"),
                "type": fnode.get("__typename"),
                "dataType": fnode.get("dataType"),
                "options": [o["name"] for o in options] if options else None,
            }
        )
    items = []
    for node in (project["items"].get("nodes") or []):
        _require_complete(node.get("fieldValues") or {}, "item fieldValues")
        content = node.get("content") or {}
        typename = content.get("__typename")
        if typename not in ("Issue", "PullRequest"):
            print(
                f"note: skipping non-issue/PR item ({typename or 'no content'})",
                file=sys.stderr,
            )
            continue
        values: dict[str, object] = {}
        for fv in node["fieldValues"].get("nodes") or []:
            fname = (fv.get("field") or {}).get("name")
            if not fname:
                continue
            values[fname] = fv.get("text") or fv.get("name") or fv.get("date")
        items.append(
            {
                "n": content.get("number"),
                "kind": typename,
                "state": content.get("state"),
                "merged": content.get("merged"),
                "title": content.get("title"),
                "values": values,
            }
        )
    return validate_export({"fields": fields, "items": items})


# ---------------------------------------------------------------------------
# Self-test: embedded in-memory fixtures only (negative controls first).


def _fx(n: int, state: str, values: dict, kind: str = "Issue") -> dict:
    return {
        "n": n,
        "kind": kind,
        "state": state,
        "merged": state == "MERGED",
        "title": f"fixture #{n}",
        "values": values,
    }


def self_test() -> int:
    failures = 0
    cases = 0

    def check(label: str, cond: bool, detail: object = "") -> None:
        nonlocal failures
        if not cond:
            failures += 1
            print(f"project-acceptance self-test FAIL: {label} {detail}")

    def run(
        items: list[dict],
        diff_item: int | None = None,
        diff_paths: list[str] | None = None,
    ) -> dict[int, ItemResult]:
        return {r.n: r for r in evaluate_export({"items": items}, diff_item, diff_paths)}

    def codes(result: ItemResult) -> list[str]:
        return [c for c, _ in result.violations]

    def warning_codes(result: ItemResult) -> list[str]:
        return [c for c, _ in result.warnings]

    # N1: closed unmerged PR, Status=Done, CI green, record present — the
    # honest parked disposition must never read as acceptance.
    cases += 1
    r = run(
        [
            _fx(
                101,
                "CLOSED",
                {
                    "Status": "Done",
                    "Evidence": "CI green",
                    "Gate state": "Dependency blocked",
                    "Authority": "Human merge",
                    "Claim boundary": "Governance",
                    "Horizon": "Parked",
                    "Acceptance record": (
                        "branch-time gate run (PR closed unmerged); claims unaccepted"
                    ),
                    "Blocked by": "None",
                },
                kind="PullRequest",
            )
        ]
    )[101]
    check("N1 closed unmerged PR is ClosedUnaccepted", r.terminal == "ClosedUnaccepted", r.terminal)
    check("N1 is not Accepted", not r.terminal.startswith("Accepted"), r.terminal)
    check("N1 raises no violations", not r.violations, r.violations)

    # E1 (council Round 1 addendum): an `excluded:` disposition record with a
    # resolvable run/SHA reference satisfies V4 yet never reads as acceptance.
    cases += 1
    e1 = _fx(
        102,
        "CLOSED",
        {
            "Status": "Done",
            "Evidence": "CI green",
            "Gate state": "Dependency blocked",
            "Authority": "Human merge",
            "Claim boundary": "Governance",
            "Acceptance record": (
                "excluded: closed unmerged, claims unaccepted; "
                "branch ci run 29903491772 @ 189a68f0"
            ),
            "Blocked by": "None",
        },
        kind="PullRequest",
    )
    r = run([e1])[102]
    check("E1 excluded record derives ClosedUnaccepted", r.terminal == "ClosedUnaccepted", r.terminal)
    check("E1 excluded record with run/SHA ref raises no V4", not r.violations, r.violations)
    check("E1 excluded record is never Accepted", not r.terminal.startswith("Accepted"), r.terminal)

    # E2: without a resolvable reference the excluded record cannot stand in
    # for V4, and even Gate=Satisfied on merged content cannot promote it.
    cases += 1
    r = run(
        [
            _fx(
                103,
                "CLOSED",
                {
                    **e1["values"],
                    "Acceptance record": "excluded: closed unmerged, claims unaccepted",
                },
                kind="PullRequest",
            )
        ]
    )[103]
    check("E2 excluded record without run/SHA ref is V4", codes(r) == ["V4"], codes(r))
    r = run(
        [
            _fx(
                104,
                "MERGED",
                {**e1["values"], "Gate state": "Satisfied"},
                kind="PullRequest",
            )
        ]
    )[104]
    check(
        "E2 excluded record never counts toward Accepted even at Gate=Satisfied",
        r.terminal == "ClosedUnaccepted",
        r.terminal,
    )

    # N2 (#60 shape): a CI-run URL can never satisfy Operator approval.
    cases += 1
    n2 = _fx(
        60,
        "OPEN",
        {
            "Status": "Todo",
            "Evidence": "CI green",
            "Gate state": "Satisfied",
            "Authority": "Operator approval",
            "Acceptance record": (
                "https://github.com/AmitabhainArunachala/vibe-halt/actions/runs/31002567065"
            ),
            "Blocked by": "None",
        },
    )
    r = run([n2])[60]
    check("N2 CI-run record cannot satisfy Operator approval", codes(r) == ["V6"], codes(r))
    check("N2 terminal is Violation", r.terminal == "Violation", r.terminal)

    # N3 (#80 shape): same for External confirmation.
    cases += 1
    n3 = _fx(80, "OPEN", {**n2["values"], "Authority": "External confirmation"})
    r = run([n3])[80]
    check("N3 CI-run record cannot satisfy External confirmation", codes(r) == ["V6"], codes(r))
    check("N3 terminal is Violation", r.terminal == "Violation", r.terminal)

    # N4: merged governance PR under the two-tier footprint check.
    n4 = _fx(
        66,
        "MERGED",
        {
            "Status": "Done",
            "Evidence": "CI green",
            "Gate state": "Satisfied",
            "Authority": "Human merge",
            "Claim boundary": "Governance",
            "Acceptance record": "merge ed32f1dd; verify 31002567065; ci 31002567030",
            "Blocked by": "None",
        },
        kind="PullRequest",
    )
    cases += 1
    r = run(
        [n4],
        66,
        [
            "docs/DEVELOPMENT_WORKFLOW.md",
            "README.md",
            "LICENSE",
            ".github/ISSUE_TEMPLATE/work_packet.md",
        ],
    )[66]
    check(
        "N4 docs-only merged governance PR is Accepted(Governance-bounded)",
        r.terminal == "Accepted (Governance-bounded)",
        r.terminal,
    )
    check("N4 docs-only raises no warnings", not r.warnings, r.warnings)

    cases += 1
    r = run([n4], 66, ["docs/DEVELOPMENT_WORKFLOW.md", "crates/vh-core/src/lib.rs"])[66]
    check("N4 crates path under Governance claim is V7", codes(r) == ["V7"], codes(r))
    check("N4 crates path terminal is Violation", r.terminal == "Violation", r.terminal)

    cases += 1
    r = run([n4], 66, ["docs/DEVELOPMENT_WORKFLOW.md", "scripts/sync_github_project.py"])[66]
    check("N4 scripts path warns W-boundary", warning_codes(r) == ["W-boundary"], r.warnings)
    check(
        "N4 scripts path stays Accepted-eligible",
        r.terminal == "Accepted (Governance-bounded)",
        r.terminal,
    )

    # N5: fabricated consensus/signature prose manufactures no grade; the
    # V3/V4/V5 conjuncts still fire on the absent resolvable record.
    cases += 1
    r = run(
        [
            _fx(
                105,
                "CLOSED",
                {
                    "Status": "Done",
                    "Evidence": "CI green",
                    "Gate state": "Satisfied",
                    "Authority": "Human merge",
                    "Owner lane": "consensus: 7 models agree, signature: xyz",
                    "Blocked by": "None",
                },
            )
        ]
    )[105]
    check("N5 consensus prose in Owner lane promotes nothing", codes(r) == ["V3", "V4"], codes(r))

    cases += 1
    r = run(
        [
            _fx(
                106,
                "OPEN",
                {
                    "Status": "Todo",
                    "Evidence": "External proof",
                    "Gate state": "External pending",
                    "Authority": "External confirmation",
                    "Acceptance record": "consensus: 7 models agree, signature: xyz",
                    "Blocked by": "None",
                },
            )
        ]
    )[106]
    check("N5 consensus prose is not an external artifact", codes(r) == ["V5"], codes(r))

    # P1: the predeclared null is a first-class terminal, not a failure.
    cases += 1
    r = run(
        [
            _fx(
                107,
                "CLOSED",
                {
                    "Status": "Done",
                    "Evidence": "Not run",
                    "Gate state": "Operator pending",
                    "Authority": "Operator approval",
                    "Acceptance record": "null:docs/nulls/2026-08-05_r4_predeclared_null.md",
                    "Blocked by": "None",
                },
            )
        ]
    )[107]
    check("P1 predeclared null is NullResult", r.terminal == "NullResult", r.terminal)
    check("P1 raises no violations (exit 0 path)", not r.violations, r.violations)

    # P2: V1 pending-flavor/authority mapping.
    cases += 1
    r = run(
        [
            _fx(
                108,
                "OPEN",
                {
                    "Status": "Todo",
                    "Evidence": "Not run",
                    "Gate state": "Operator pending",
                    "Authority": "Human merge",
                    "Blocked by": "None",
                },
            )
        ]
    )[108]
    check("P2 pending flavor vs Authority mismatch is V1", codes(r) == ["V1"], codes(r))

    # P3 (#81 shape): Ready with a live open dependency.
    cases += 1
    fx81 = _fx(
        81,
        "OPEN",
        {
            "Status": "Todo",
            "Evidence": "Not run",
            "Gate state": "Ready",
            "Authority": "Human merge",
            "Blocked by": "#65 audit coordination",
        },
    )
    fx65 = _fx(
        65,
        "OPEN",
        {
            "Status": "Todo",
            "Evidence": "Not run",
            "Gate state": "Human pending",
            "Authority": "Human merge",
            "Blocked by": "None",
        },
    )
    r = run([fx81, fx65])[81]
    check("P3 Ready with live open dep is V2", codes(r) == ["V2"], codes(r))

    # P4: stale closed ref warns; the `(satisfied ...)` marker silences it.
    cases += 1
    fx_open = _fx(
        109,
        "OPEN",
        {
            "Status": "Todo",
            "Evidence": "Not run",
            "Gate state": "Dependency blocked",
            "Authority": "Human merge",
            "Blocked by": "#110 workflow contract",
        },
    )
    fx_closed = _fx(
        110,
        "CLOSED",
        {
            "Status": "Done",
            "Evidence": "Not run",
            "Gate state": "Dependency blocked",
            "Authority": "Human merge",
            "Blocked by": "None",
        },
    )
    r = run([fx_open, fx_closed])[109]
    check("P4 stale closed ref warns W-stale", warning_codes(r) == ["W-stale"], r.warnings)
    check("P4 stale ref is not a violation", not r.violations, r.violations)

    cases += 1
    fx_marked = _fx(
        109, "OPEN", {**fx_open["values"], "Blocked by": "#110 (satisfied 2026-08-05)"}
    )
    r = run([fx_marked, fx_closed])[109]
    check(
        "P4 satisfied-marked ref stays silent",
        not r.warnings and not r.violations,
        (r.violations, r.warnings),
    )

    # P5 (58<->62 shape): naive-parse pairwise cycle warns, never errors.
    cases += 1
    fx58 = _fx(
        58,
        "OPEN",
        {
            "Status": "Todo",
            "Evidence": "Blocked",
            "Gate state": "Dependency blocked",
            "Authority": "Human merge",
            "Blocked by": "#67 terminal; #62 corrections and admission",
        },
    )
    fx62 = _fx(
        62,
        "OPEN",
        {
            "Status": "Todo",
            "Evidence": "Blocked",
            "Gate state": "Dependency blocked",
            "Authority": "Human merge",
            "Blocked by": "#67 terminal; PR #58 correction and admission",
        },
    )
    results = run([fx58, fx62])
    check("P5 58<->62 naive cycle warns once", warning_codes(results[58]) == ["W-cycle"], results[58].warnings)
    check(
        "P5 cycle is not a violation",
        not results[58].violations and not results[62].violations,
        (results[58].violations, results[62].violations),
    )

    # P6: refs inside `constraint:`/`decides:` segments never count as deps.
    cases += 1
    fx_ready = _fx(
        111,
        "OPEN",
        {
            "Status": "Todo",
            "Evidence": "Not run",
            "Gate state": "Ready",
            "Authority": "Human merge",
            "Blocked by": "None; constraint: retained debts live in #112 #113",
        },
    )
    fx_decides = _fx(
        114,
        "OPEN",
        {
            "Status": "Todo",
            "Evidence": "Not run",
            "Gate state": "Ready",
            "Authority": "Human merge",
            "Blocked by": "decides: #112 disposition",
        },
    )
    fx_live_a = _fx(
        112,
        "OPEN",
        {
            "Status": "Todo",
            "Evidence": "Not run",
            "Gate state": "Ready",
            "Authority": "Human merge",
            "Blocked by": "None",
        },
    )
    fx_live_b = _fx(
        113,
        "OPEN",
        {
            "Status": "Todo",
            "Evidence": "Not run",
            "Gate state": "Ready",
            "Authority": "Human merge",
            "Blocked by": "None",
        },
    )
    results = run([fx_ready, fx_decides, fx_live_a, fx_live_b])
    check("P6 constraint-segment refs never make V2", not results[111].violations, results[111].violations)
    check("P6 decides-segment refs never make V2", not results[114].violations, results[114].violations)
    check(
        "P6 ignored refs raise no warnings",
        not results[111].warnings and not results[114].warnings,
        (results[111].warnings, results[114].warnings),
    )

    if failures:
        print(f"project-acceptance self-test: {failures} failure(s)")
        return 1
    print(f"project-acceptance self-test: PASS ({cases} cases)")
    return 0


def main() -> int:
    parser = argparse.ArgumentParser(
        description="Derive per-item acceptance terminals for the vibe-halt project board."
    )
    parser.add_argument("--export", help="path to a compact-shape JSON export")
    parser.add_argument(
        "--live",
        action="store_true",
        help="read the board via one read-only gh GraphQL query",
    )
    parser.add_argument(
        "--self-test", action="store_true", dest="self_test", help="run embedded fixtures"
    )
    parser.add_argument(
        "--diff-paths", help="newline list of changed paths for the --item footprint check"
    )
    parser.add_argument("--item", type=int, help="item number the --diff-paths belong to")
    args = parser.parse_args()

    if args.self_test:
        return self_test()
    if bool(args.export) == args.live:
        print("exactly one of --export PATH or --live is required", file=sys.stderr)
        return 3
    if (args.diff_paths is None) != (args.item is None):
        print("--diff-paths and --item are required together", file=sys.stderr)
        return 3

    try:
        export = load_live() if args.live else load_export(args.export)
        diff_paths = None
        if args.diff_paths is not None:
            try:
                text = Path(args.diff_paths).read_text(encoding="utf-8")
            except OSError as e:
                raise InputError(f"cannot read --diff-paths {args.diff_paths}: {e}") from e
            diff_paths = [line.strip() for line in text.splitlines() if line.strip()]
            if args.item not in {item["n"] for item in export["items"]}:
                raise InputError(f"--item {args.item} not present in the export")
    except InputError as e:
        print(f"input error: {e}", file=sys.stderr)
        return 3

    results = evaluate_export(export, args.item, diff_paths)
    source = args.export if args.export else "live GraphQL export"
    if args.item is not None:
        source += f" (footprint check on item #{args.item})"
    return 2 if print_report(results, source) else 0


if __name__ == "__main__":
    sys.exit(main())
