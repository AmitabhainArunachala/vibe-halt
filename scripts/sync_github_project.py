#!/usr/bin/env python3
"""Plan or apply Vibe Halt's additive GitHub project structure, fail-closed.

Additive contract: creates missing labels, fields, views, repository links,
and open issue/PR items; never deletes, closes, archives, merges, or replaces
existing structure. Fail-closed per issue #86: wrong managed field dataType,
missing managed select options, view-ownership conflicts, duplicate view
names, pagination truncation, and concurrent view edits all abort with typed
errors instead of writing. Fields are read via GraphQL dataType (gh
field-list cannot distinguish DATE/TEXT). The project is resolved by owner +
number, never by mutable title. View ownership lives in
scripts/project_sync_views.lock.json; --adopt-view NAME adopts a live view
into it. Apply re-reads each view immediately before writing, merges visible
columns from that fresh read, and exits 0 only after a fresh empty plan.
Apply prints a JSON receipt (executed argvs, prior values of overwritten
views); --receipt PATH writes it outside the repository tree only.

Exit codes: 0 converged / no changes; 1 plan has pending actions; 2 typed
drift, conflict, or transport error; 3 apply did not converge.
"""

from __future__ import annotations

import argparse
import json
import subprocess
import sys
import tempfile
from collections.abc import Callable
from dataclasses import dataclass
from pathlib import Path
from typing import Protocol

OWNER = "AmitabhainArunachala"
REPO = "vibe-halt"
REPO_SLUG = f"{OWNER}/{REPO}"
PROJECT_TITLE = "vibe-halt — Evidence to Reality"
PROJECT_NUMBER = 1

REPO_ROOT = Path(__file__).resolve().parent.parent
VIEW_REGISTRY_PATH = Path(__file__).resolve().parent / "project_sync_views.lock.json"

LABEL_LIST_LIMIT = 200
OPEN_LIST_LIMIT = 200
ITEM_LIST_LIMIT = 500
MAX_APPLY_PASSES = 3

LABELS = {
    "triage": "5319e7",
    "priority:P0": "b60205",
    "priority:P1": "d93f0b",
    "priority:P2": "fbca04",
    "priority:P3": "c2e0c6",
    "area:reality-bridge": "1d76db",
    "area:evidence": "0e8a16",
    "area:release": "0052cc",
    "area:governance": "6f42c1",
    "area:corpus": "7057ff",
    "agent-ready": "a2eeef",
    "human-gate": "8b5cf6",
    "blocked": "bfdadc",
}

FIELDS = {
    "Priority": ("SINGLE_SELECT", "P0,P1,P2,P3"),
    "Phase": (
        "SINGLE_SELECT",
        "Reality Bridge,External Proof,Productization,Truth Kernel,Maintenance",
    ),
    "Evidence": (
        "SINGLE_SELECT",
        "Not run,Local green,CI green,External proof,Blocked",
    ),
    "Authority": (
        "SINGLE_SELECT",
        "Agent executable,Human merge,Operator approval,External confirmation",
    ),
    "Type": (
        "SINGLE_SELECT",
        "Epic,Work packet,Proof,Experiment,Governance,Maintenance",
    ),
    "Outcome": (
        "SINGLE_SELECT",
        "C1 Tier-1 identity,C2 Tier-2 divergence,C3 Holdout recall,"
        "C4 Unknown bugs,C5 Replay and shrink,C6 Throughput,"
        "C7 Dharma receipt,Enabler,Research",
    ),
    "Claim boundary": (
        "SINGLE_SELECT",
        "Tier 1 / D0,Tier 2 / D2,Governance,Measurement,"
        "External confirmation,Unknown / fail-closed",
    ),
    "Gate state": (
        "SINGLE_SELECT",
        "Ready,Dependency blocked,Human pending,Operator pending,"
        "External pending,Satisfied",
    ),
    "Risk": ("SINGLE_SELECT", "Low,Medium,High,Critical"),
    "Horizon": ("SINGLE_SELECT", "Now,Next,Later,Parked"),
    "Target date": ("DATE", ""),
    "Owner lane": ("TEXT", ""),
    "Blocked by": ("TEXT", ""),
    "Acceptance record": ("TEXT", ""),
}


@dataclass(frozen=True)
class ViewSpec:
    name: str
    layout: str
    filter_query: str
    visible_fields: tuple[str, ...]


VIEW_SPECS = (
    ViewSpec(
        "All Work",
        "TABLE_LAYOUT",
        "",
        (
            "Title",
            "Status",
            "Priority",
            "Type",
            "Outcome",
            "Claim boundary",
            "Evidence",
            "Authority",
            "Gate state",
            "Risk",
            "Horizon",
            "Blocked by",
            "Owner lane",
            "Target date",
            "Assignees",
            "Repository",
        ),
    ),
    ViewSpec(
        "Delivery Board",
        "BOARD_LAYOUT",
        "",
        (
            "Title",
            "Status",
            "Priority",
            "Type",
            "Outcome",
            "Gate state",
            "Risk",
            "Horizon",
            "Blocked by",
            "Evidence",
            "Authority",
            "Claim boundary",
            "Owner lane",
            "Target date",
        ),
    ),
    ViewSpec(
        "Critical Path",
        "TABLE_LAYOUT",
        "is:open priority:P0",
        (
            "Title",
            "Status",
            "Priority",
            "Type",
            "Outcome",
            "Claim boundary",
            "Gate state",
            "Risk",
            "Evidence",
            "Authority",
            "Blocked by",
            "Owner lane",
            "Target date",
        ),
    ),
    ViewSpec(
        "Evidence & Authority",
        "TABLE_LAYOUT",
        "is:open",
        (
            "Title",
            "Status",
            "Outcome",
            "Claim boundary",
            "Evidence",
            "Authority",
            "Gate state",
            "Risk",
            "Blocked by",
            "Owner lane",
        ),
    ),
    ViewSpec(
        "12-Week Roadmap",
        "ROADMAP_LAYOUT",
        "is:open",
        (),
    ),
    ViewSpec(
        "Human Gates",
        "TABLE_LAYOUT",
        'is:open -authority:"Agent executable"',
        (
            "Title",
            "Status",
            "Priority",
            "Type",
            "Outcome",
            "Claim boundary",
            "Authority",
            "Gate state",
            "Evidence",
            "Risk",
            "Blocked by",
            "Owner lane",
            "Target date",
        ),
    ),
    ViewSpec(
        "Parked / Research",
        "TABLE_LAYOUT",
        "horizon:Parked",
        (
            "Title",
            "Status",
            "Priority",
            "Type",
            "Outcome",
            "Claim boundary",
            "Horizon",
            "Risk",
            "Evidence",
            "Authority",
            "Gate state",
            "Blocked by",
            "Owner lane",
        ),
    ),
)

SPEC_VIEW_NAMES = tuple(spec.name for spec in VIEW_SPECS)

PROJECT_RESOLVE_QUERY = """
query($login: String!, $number: Int!) {
  user(login: $login) {
    projectV2(number: $number) { id title number }
  }
}
""".strip()

PROJECT_FIELDS_QUERY = """
query($login: String!, $number: Int!, $cursor: String) {
  user(login: $login) {
    projectV2(number: $number) {
      fields(first: 100, after: $cursor) {
        pageInfo { hasNextPage endCursor }
        nodes {
          __typename
          ... on ProjectV2FieldCommon { id name dataType }
          ... on ProjectV2SingleSelectField { options { id name } }
        }
      }
    }
  }
}
""".strip()

PROJECT_VIEWS_QUERY = """
query($login: String!, $number: Int!, $cursor: String) {
  user(login: $login) {
    projectV2(number: $number) {
      views(first: 50, after: $cursor) {
        pageInfo { hasNextPage endCursor }
        nodes {
          id name layout filter
          configuration {
            visibleFields(first: 100) {
              pageInfo { hasNextPage endCursor }
              nodes { __typename ... on ProjectV2FieldCommon { id name } }
            }
          }
        }
      }
    }
  }
}
""".strip()

VIEW_NODE_QUERY = """
query($id: ID!, $cursor: String) {
  node(id: $id) {
    __typename
    ... on ProjectV2View {
      id name layout filter
      configuration {
        visibleFields(first: 100, after: $cursor) {
          pageInfo { hasNextPage endCursor }
          nodes { __typename ... on ProjectV2FieldCommon { id name } }
        }
      }
    }
  }
}
""".strip()

PROJECT_REPOSITORIES_QUERY = """
query($id: ID!, $cursor: String) {
  node(id: $id) {
    ... on ProjectV2 {
      repositories(first: 100, after: $cursor) {
        pageInfo { hasNextPage endCursor }
        nodes { nameWithOwner }
      }
    }
  }
}
""".strip()

CREATE_VIEW_MUTATION = """
mutation(
  $projectId: ID!
  $name: String!
  $layout: ProjectV2ViewLayout!
  $visibleFieldIds: [ID!]
) {
  createProjectV2View(input: {
    projectId: $projectId
    name: $name
    layout: $layout
    configuration: {visibleFieldIds: $visibleFieldIds}
  }) {
    projectV2View { id name layout filter }
  }
}
""".strip()

UPDATE_VIEW_MUTATION = """
mutation(
  $viewId: ID!
  $layout: ProjectV2ViewLayout!
  $filter: String!
  $visibleFieldIds: [ID!]
) {
  updateProjectV2View(input: {
    viewId: $viewId
    layout: $layout
    filter: $filter
    configuration: {visibleFieldIds: $visibleFieldIds}
  }) {
    projectV2View { id name layout filter }
  }
}
""".strip()

CREATE_ROADMAP_VIEW_MUTATION = """
mutation(
  $projectId: ID!
  $name: String!
  $layout: ProjectV2ViewLayout!
) {
  createProjectV2View(input: {
    projectId: $projectId
    name: $name
    layout: $layout
  }) {
    projectV2View { id name layout filter }
  }
}
""".strip()

UPDATE_ROADMAP_VIEW_MUTATION = """
mutation(
  $viewId: ID!
  $layout: ProjectV2ViewLayout!
  $filter: String!
) {
  updateProjectV2View(input: {
    viewId: $viewId
    layout: $layout
    filter: $filter
  }) {
    projectV2View { id name layout filter }
  }
}
""".strip()


class SyncError(Exception):
    def __init__(self, *diagnostics: str) -> None:
        super().__init__("\n".join(diagnostics))
        self.diagnostics = list(diagnostics)


class Transport(Protocol):
    def run(
        self, command: list[str], *, check: bool = True
    ) -> subprocess.CompletedProcess[str]: ...


class GhTransport:
    def run(
        self, command: list[str], *, check: bool = True
    ) -> subprocess.CompletedProcess[str]:
        return subprocess.run(command, text=True, capture_output=True, check=check)


@dataclass(frozen=True)
class ViewState:
    view_id: str
    name: str
    layout: str
    filter_query: str
    visible_field_ids: tuple[str, ...]


@dataclass(frozen=True)
class ViewUpdate:
    spec: ViewSpec
    plan_state: ViewState
    required_ids: tuple[str, ...]


@dataclass
class Action:
    description: str
    command: list[str]
    view_update: ViewUpdate | None = None
    creates_view: str | None = None


def required_options(options: str) -> tuple[str, ...]:
    return tuple(option.strip() for option in options.split(",") if option.strip())


def unique(values: list[str]) -> list[str]:
    return list(dict.fromkeys(values))


def supports_visible_fields(spec: ViewSpec) -> bool:
    return spec.layout != "ROADMAP_LAYOUT"


def label_list_command() -> list[str]:
    return [
        "gh", "label", "list", "-R", REPO_SLUG,
        "--limit", str(LABEL_LIST_LIMIT), "--json", "name,color",
    ]


def open_list_command(kind: str) -> list[str]:
    return [
        "gh", kind, "list", "-R", REPO_SLUG, "--state", "open",
        "--limit", str(OPEN_LIST_LIMIT), "--json", "url",
    ]


def item_list_command() -> list[str]:
    return [
        "gh", "project", "item-list", str(PROJECT_NUMBER), "--owner", OWNER,
        "--limit", str(ITEM_LIST_LIMIT), "--format", "json",
    ]


def graphql_command(query: str, variables: dict[str, object]) -> list[str]:
    command = ["gh", "api", "graphql", "-f", f"query={query}"]
    for name, value in variables.items():
        if value is None:
            continue
        flag = "-f" if isinstance(value, str) else "-F"
        command.extend([flag, f"{name}={value}"])
    return command


def gh_json(transport: Transport, command: list[str]) -> object:
    proc = transport.run(command)
    try:
        return json.loads(proc.stdout)
    except json.JSONDecodeError as error:
        raise SyncError(
            f"unparseable gh output: {' '.join(command[:4])} ...: {error}: "
            f"{proc.stdout[:200]!r}"
        ) from error


def gh_list(
    transport: Transport, command: list[str], limit: int, label: str
) -> list[dict[str, object]]:
    rows = gh_json(transport, command)
    if not isinstance(rows, list):
        raise SyncError(f"{label}: expected a JSON list, got {type(rows).__name__}")
    if len(rows) >= limit:
        raise SyncError(
            f"{label}: returned {len(rows)} rows at the requested limit {limit}; "
            "possible truncation — raise the limit or paginate"
        )
    return rows


def graphql(
    transport: Transport, query: str, variables: dict[str, object]
) -> dict[str, object]:
    payload = gh_json(transport, graphql_command(query, variables))
    if not isinstance(payload, dict) or not isinstance(payload.get("data"), dict):
        raise SyncError(
            f"GraphQL response without a data object: {str(payload)[:200]!r}"
        )
    return payload["data"]


def follow(
    transport: Transport,
    query: str,
    variables: dict[str, object],
    path: tuple[str, ...],
) -> list[dict[str, object]]:
    """Cursor-follow one GraphQL connection to exhaustion."""
    nodes: list[dict[str, object]] = []
    cursor: str | None = None
    while True:
        data: object = graphql(transport, query, {**variables, "cursor": cursor})
        connection: object = data
        for key in path:
            connection = connection.get(key) if isinstance(connection, dict) else None
        if not isinstance(connection, dict):
            raise SyncError(f"missing {'.'.join(path)} in GraphQL response")
        nodes.extend(n for n in connection.get("nodes") or [] if isinstance(n, dict))
        page = connection.get("pageInfo")
        if not isinstance(page, dict) or not page.get("hasNextPage"):
            return nodes
        cursor = str(page.get("endCursor"))


def resolve_project(transport: Transport) -> dict[str, object] | None:
    data = graphql(
        transport,
        PROJECT_RESOLVE_QUERY,
        {"login": OWNER, "number": PROJECT_NUMBER},
    )
    user = data.get("user")
    if not isinstance(user, dict):
        raise SyncError(f"GraphQL user({OWNER!r}) resolved to nothing")
    project = user.get("projectV2")
    return project if isinstance(project, dict) else None


def read_fields(transport: Transport) -> list[dict[str, object]]:
    return follow(
        transport,
        PROJECT_FIELDS_QUERY,
        {"login": OWNER, "number": PROJECT_NUMBER},
        ("user", "projectV2", "fields"),
    )


def _visible_ids(
    connection: object, view_label: str
) -> tuple[list[str], dict[str, object]]:
    if not isinstance(connection, dict):
        raise SyncError(f"view {view_label}: missing visibleFields connection")
    ids: list[str] = []
    for node in connection.get("nodes") or []:
        if not isinstance(node, dict) or not node.get("id"):
            raise SyncError(
                f"view {view_label}: visible field node without an id ({node!r}); "
                "refusing to plan an update that could drop a column"
            )
        ids.append(str(node["id"]))
    page = connection.get("pageInfo")
    return ids, page if isinstance(page, dict) else {}


def _view_state(transport: Transport, node: dict[str, object]) -> ViewState:
    view_id = str(node.get("id") or "")
    label = f"{node.get('name')!r} ({view_id})"
    configuration = node.get("configuration")
    connection = (
        configuration.get("visibleFields") if isinstance(configuration, dict) else None
    )
    ids, page = _visible_ids(connection, label)
    while page.get("hasNextPage"):
        data = graphql(
            transport,
            VIEW_NODE_QUERY,
            {"id": view_id, "cursor": str(page.get("endCursor"))},
        )
        more_node = data.get("node")
        if not isinstance(more_node, dict):
            raise SyncError(f"view {label}: vanished while paginating visible fields")
        configuration = more_node.get("configuration")
        connection = (
            configuration.get("visibleFields")
            if isinstance(configuration, dict)
            else None
        )
        more, page = _visible_ids(connection, label)
        ids.extend(more)
    return ViewState(
        view_id=view_id,
        name=str(node.get("name") or ""),
        layout=str(node.get("layout") or ""),
        filter_query=str(node.get("filter") or ""),
        visible_field_ids=tuple(ids),
    )


def read_view_states(transport: Transport) -> list[ViewState]:
    nodes = follow(
        transport,
        PROJECT_VIEWS_QUERY,
        {"login": OWNER, "number": PROJECT_NUMBER},
        ("user", "projectV2", "views"),
    )
    return [_view_state(transport, node) for node in nodes]


def read_view_state(transport: Transport, view_id: str) -> ViewState:
    data = graphql(transport, VIEW_NODE_QUERY, {"id": view_id, "cursor": None})
    node = data.get("node")
    if not isinstance(node, dict) or node.get("__typename") != "ProjectV2View":
        raise SyncError(
            f"registered view {view_id} no longer exists live — deleted or "
            "recreated by a human; re-adopt explicitly with --adopt-view"
        )
    return _view_state(transport, node)


def load_view_registry(path: Path) -> dict[str, str]:
    try:
        raw = path.read_text(encoding="utf-8")
    except OSError as error:
        raise SyncError(
            f"view ownership registry {path} is unreadable ({error}); it is "
            "committed repo state — restore it from git"
        ) from error
    try:
        data = json.loads(raw)
    except json.JSONDecodeError as error:
        raise SyncError(f"view ownership registry {path} is not JSON: {error}") from error
    if not isinstance(data, dict) or not all(
        isinstance(k, str) and isinstance(v, str) for k, v in data.items()
    ):
        raise SyncError(f"view ownership registry {path} must map view name → view id")
    unknown = sorted(set(data) - set(SPEC_VIEW_NAMES))
    if unknown:
        raise SyncError(
            f"view ownership registry {path} contains non-managed view name(s): "
            f"{', '.join(repr(u) for u in unknown)} — only managed spec views may "
            "be registered"
        )
    return dict(data)


def save_view_registry(path: Path, registry: dict[str, str]) -> None:
    path.write_text(
        json.dumps(registry, indent=2, ensure_ascii=False, sort_keys=True) + "\n",
        encoding="utf-8",
    )


def field_drift(fields: list[dict[str, object]]) -> list[str]:
    by_name: dict[str, list[dict[str, object]]] = {}
    for node in fields:
        name = node.get("name")
        if isinstance(name, str) and name:
            by_name.setdefault(name, []).append(node)
    diagnostics: list[str] = []
    for name, (data_type, options) in FIELDS.items():
        rows = by_name.get(name, [])
        if not rows:
            continue
        if len(rows) > 1:
            observed = ", ".join(str(row.get("dataType")) for row in rows)
            diagnostics.append(
                f"{len(rows)} fields named {name!r} exist live (dataType: {observed}); "
                f"expected exactly one {data_type} field — resolve manually"
            )
            continue
        observed_type = str(rows[0].get("dataType"))
        if observed_type != data_type:
            diagnostics.append(
                f"field {name!r} exists with dataType {observed_type}; expected "
                f"{data_type} — a wrong-typed field is squatting the managed name"
            )
            continue
        if data_type == "SINGLE_SELECT":
            option_rows = rows[0].get("options")
            actual = {
                str(option.get("name"))
                for option in (option_rows if isinstance(option_rows, list) else [])
                if isinstance(option, dict) and option.get("name")
            }
            missing = [o for o in required_options(options) if o not in actual]
            if missing:
                diagnostics.append(
                    f"single-select field {name!r} is missing required option(s): "
                    f"{', '.join(missing)} — existing options were left unchanged"
                )
    return diagnostics


def view_ownership_conflicts(
    registry: dict[str, str], views: list[ViewState]
) -> list[str]:
    conflicts: list[str] = []
    live_by_id = {view.view_id: view for view in views}
    by_name: dict[str, list[ViewState]] = {}
    for view in views:
        by_name.setdefault(view.name, []).append(view)
    for name in SPEC_VIEW_NAMES:
        matches = by_name.get(name, [])
        if len(matches) > 1:
            ids = ", ".join(view.view_id for view in matches)
            conflicts.append(
                f"duplicate live views named {name!r}: {ids} — rename or remove "
                "the extras manually; refusing to guess which one is managed"
            )
            continue
        registered = registry.get(name)
        live = matches[0] if matches else None
        if registered is None:
            if live is not None:
                conflicts.append(
                    f"live view {name!r} ({live.view_id}) is not in the ownership "
                    f"registry — a name match alone never authorizes overwrite; "
                    f"pass --adopt-view {name!r} to adopt it (then commit "
                    f"{VIEW_REGISTRY_PATH.name})"
                )
            continue
        registered_live = live_by_id.get(registered)
        if live is not None and live.view_id == registered:
            continue
        if live is not None:
            conflicts.append(
                f"view {name!r}: registry owns {registered} but the live view "
                f"named {name!r} is {live.view_id} — recreated by a human; "
                f"resolve, then --adopt-view {name!r} to adopt the live view"
            )
        elif registered_live is not None:
            conflicts.append(
                f"registered view {name!r} ({registered}) is now named "
                f"{registered_live.name!r} live — renamed by a human; resolve "
                f"manually, then re-adopt explicitly"
            )
        else:
            conflicts.append(
                f"registered view {name!r} ({registered}) no longer exists live — "
                f"view deleted or recreated by a human; re-adopt explicitly with "
                f"--adopt-view {name!r}"
            )
    return conflicts


def append_graphql_list(
    command: list[str], variable: str, values: list[str]
) -> list[str]:
    for value in values:
        command.extend(["-F", f"{variable}[]={value}"])
    return command


def create_view_command(
    project_id: str, spec: ViewSpec, visible_field_ids: list[str]
) -> list[str]:
    mutation = (
        CREATE_VIEW_MUTATION
        if supports_visible_fields(spec)
        else CREATE_ROADMAP_VIEW_MUTATION
    )
    command = [
        "gh", "api", "graphql",
        "-f", f"query={mutation}",
        "-F", f"projectId={project_id}",
        "-f", f"name={spec.name}",
        "-f", f"layout={spec.layout}",
    ]
    if not supports_visible_fields(spec):
        return command
    return append_graphql_list(command, "visibleFieldIds", visible_field_ids)


def update_view_command(
    view_id: str, spec: ViewSpec, visible_field_ids: list[str]
) -> list[str]:
    mutation = (
        UPDATE_VIEW_MUTATION
        if supports_visible_fields(spec)
        else UPDATE_ROADMAP_VIEW_MUTATION
    )
    command = [
        "gh", "api", "graphql",
        "-f", f"query={mutation}",
        "-F", f"viewId={view_id}",
        "-f", f"layout={spec.layout}",
        "-f", f"filter={spec.filter_query}",
    ]
    if not supports_visible_fields(spec):
        return command
    return append_graphql_list(command, "visibleFieldIds", visible_field_ids)


def plan_view_actions(
    project_id: str,
    field_ids_by_name: dict[str, str],
    views_by_name: dict[str, ViewState],
) -> list[Action]:
    actions: list[Action] = []
    for spec in VIEW_SPECS:
        required_ids: list[str] = []
        if supports_visible_fields(spec):
            required_ids = [
                field_ids_by_name[name]
                for name in spec.visible_fields
                if name in field_ids_by_name
            ]
        state = views_by_name.get(spec.name)
        if state is None:
            actions.append(
                Action(
                    f"create project view {spec.name}",
                    create_view_command(project_id, spec, required_ids),
                    creates_view=spec.name,
                )
            )
            continue
        drift: list[str] = []
        if state.layout != spec.layout:
            drift.append("layout")
        if state.filter_query != spec.filter_query:
            drift.append("filter")
        if supports_visible_fields(spec) and any(
            field_id not in state.visible_field_ids for field_id in required_ids
        ):
            drift.append("visible fields")
        if drift:
            desired = unique([*state.visible_field_ids, *required_ids])
            actions.append(
                Action(
                    f"update project view {spec.name} ({', '.join(drift)})",
                    update_view_command(state.view_id, spec, desired),
                    view_update=ViewUpdate(
                        spec=spec,
                        plan_state=state,
                        required_ids=tuple(required_ids),
                    ),
                )
            )
    return actions


def build_plan(
    transport: Transport,
    registry: dict[str, str],
    *,
    allow_create_project: bool = False,
) -> list[Action]:
    actions: list[Action] = []

    labels = gh_list(transport, label_list_command(), LABEL_LIST_LIMIT, "gh label list")
    existing_labels = {str(row.get("name")) for row in labels}
    for name, color in LABELS.items():
        if name not in existing_labels:
            actions.append(
                Action(
                    f"create label {name}",
                    ["gh", "label", "create", name, "-R", REPO_SLUG, "--color", color],
                )
            )

    project = resolve_project(transport)
    if project is None:
        if not allow_create_project:
            raise SyncError(
                f"project number {PROJECT_NUMBER} not found for owner {OWNER}; "
                f"create it manually (title {PROJECT_TITLE!r}) or pass "
                "--allow-create-project — after creation, rerun so the sync "
                "adopts the project number (projects are never resolved by title)"
            )
        actions.append(
            Action(
                f"create project {PROJECT_TITLE!r} (rerun required afterwards to "
                "adopt the project number)",
                ["gh", "project", "create", "--owner", OWNER, "--title", PROJECT_TITLE],
            )
        )
        return actions
    project_id = str(project.get("id"))

    fields = read_fields(transport)
    views = read_view_states(transport)
    diagnostics = field_drift(fields) + view_ownership_conflicts(registry, views)
    if diagnostics:
        raise SyncError(*diagnostics)

    existing_fields = {
        str(node.get("name")) for node in fields if node.get("name")
    }
    for name, (data_type, options) in FIELDS.items():
        if name in existing_fields:
            continue
        command = [
            "gh", "project", "field-create", str(PROJECT_NUMBER),
            "--owner", OWNER, "--name", name, "--data-type", data_type,
        ]
        if options:
            command.extend(["--single-select-options", options])
        actions.append(Action(f"create project field {name}", command))

    field_ids_by_name = {
        str(node.get("name")): str(node.get("id"))
        for node in fields
        if node.get("name") and node.get("id")
    }
    views_by_name = {view.name: view for view in views}
    actions.extend(plan_view_actions(project_id, field_ids_by_name, views_by_name))

    linked = follow(
        transport,
        PROJECT_REPOSITORIES_QUERY,
        {"id": project_id},
        ("node", "repositories"),
    )
    if REPO_SLUG not in {str(node.get("nameWithOwner")) for node in linked}:
        actions.append(
            Action(
                "link repository to project",
                [
                    "gh", "project", "link", str(PROJECT_NUMBER),
                    "--owner", OWNER, "--repo", REPO,
                ],
            )
        )

    items_payload = gh_json(transport, item_list_command())
    if not isinstance(items_payload, dict) or not isinstance(
        items_payload.get("items"), list
    ):
        raise SyncError("gh project item-list: expected an object with an items list")
    items = items_payload["items"]
    if len(items) >= ITEM_LIST_LIMIT:
        raise SyncError(
            f"gh project item-list: returned {len(items)} items at the requested "
            f"limit {ITEM_LIST_LIMIT}; possible truncation — raise the limit or paginate"
        )
    known_urls = {
        item["content"].get("url")
        for item in items
        if isinstance(item, dict) and isinstance(item.get("content"), dict)
    }
    for kind in ("issue", "pr"):
        rows = gh_list(
            transport, open_list_command(kind), OPEN_LIST_LIMIT, f"gh {kind} list"
        )
        for row in rows:
            url = row.get("url")
            if url and url not in known_urls:
                actions.append(
                    Action(
                        f"add {url} to project",
                        [
                            "gh", "project", "item-add", str(PROJECT_NUMBER),
                            "--owner", OWNER, "--url", str(url),
                        ],
                    )
                )
    return actions


def run_plan(
    transport: Transport,
    registry: dict[str, str],
    *,
    allow_create_project: bool = False,
    echo: Callable[[str], object] = print,
) -> int:
    actions = build_plan(
        transport, registry, allow_create_project=allow_create_project
    )
    if not actions:
        echo("project-sync: no additive changes required")
        return 0
    for action in actions:
        echo(f"- {action.description}")
        echo(f"  argv: {json.dumps(action.command, ensure_ascii=False)}")
        vu = action.view_update
        if vu is not None:
            if vu.plan_state.filter_query != vu.spec.filter_query:
                echo(
                    f"  would overwrite filter: '{vu.plan_state.filter_query}' → "
                    f"'{vu.spec.filter_query}'"
                )
            if vu.plan_state.layout != vu.spec.layout:
                echo(
                    f"  would overwrite layout: '{vu.plan_state.layout}' → "
                    f"'{vu.spec.layout}'"
                )
    echo(
        f"project-sync: {len(actions)} additive action(s) pending; plan only, "
        "no writes performed"
    )
    return 1


def _receipt(
    executed: list[list[str]],
    overwritten: list[dict[str, str]],
    result: str,
    residual: list[list[str]],
    failures: list[str],
) -> dict[str, object]:
    return {
        "executed": executed,
        "overwritten_views": overwritten,
        "result": result,
        "residual": residual,
        "failures": failures,
    }


def _apply_view_update(
    transport: Transport,
    action: Action,
    executed: list[list[str]],
    executed_keys: set[tuple[str, ...]],
    overwritten: list[dict[str, str]],
    echo: Callable[[str], object],
) -> str | None:
    vu = action.view_update
    assert vu is not None
    plan_state = vu.plan_state
    fresh = read_view_state(transport, plan_state.view_id)
    if (fresh.layout, fresh.filter_query, fresh.visible_field_ids) != (
        plan_state.layout,
        plan_state.filter_query,
        plan_state.visible_field_ids,
    ):
        return (
            f"concurrent edit on view {plan_state.name!r} ({plan_state.view_id}): "
            f"plan-time (layout={plan_state.layout}, "
            f"filter={plan_state.filter_query!r}, "
            f"{len(plan_state.visible_field_ids)} columns) != pre-write "
            f"(layout={fresh.layout}, filter={fresh.filter_query!r}, "
            f"{len(fresh.visible_field_ids)} columns) — no write performed"
        )
    desired = unique([*fresh.visible_field_ids, *vu.required_ids])
    command = update_view_command(plan_state.view_id, vu.spec, desired)
    echo(f"- {action.description}")
    transport.run(command)
    executed.append(list(command))
    executed_keys.add(tuple(command))
    overwritten.append(
        {
            "name": plan_state.name,
            "prior_filter": fresh.filter_query,
            "prior_layout": fresh.layout,
        }
    )
    return None


def _adopt_created_view(
    transport: Transport,
    registry: dict[str, str],
    registry_path: Path | None,
    name: str,
    echo: Callable[[str], object],
) -> str | None:
    matches = [view for view in read_view_states(transport) if view.name == name]
    if len(matches) != 1:
        return (
            f"created view {name!r} not uniquely visible on re-read "
            f"({len(matches)} matches); resolve and --adopt-view explicitly"
        )
    registry[name] = matches[0].view_id
    if registry_path is not None:
        save_view_registry(registry_path, registry)
        echo(
            f"project-sync: adopted created view {name!r} → {matches[0].view_id}; "
            f"commit {registry_path.name}"
        )
    return None


def run_apply(
    transport: Transport,
    registry: dict[str, str],
    *,
    registry_path: Path | None = None,
    allow_create_project: bool = False,
    echo: Callable[[str], object] = print,
) -> tuple[int, dict[str, object]]:
    executed: list[list[str]] = []
    executed_keys: set[tuple[str, ...]] = set()
    overwritten: list[dict[str, str]] = []
    failures: list[str] = []
    residual: list[list[str]] = []
    converged = False
    try:
        for _ in range(MAX_APPLY_PASSES):
            actions = build_plan(
                transport, registry, allow_create_project=allow_create_project
            )
            if not actions:
                converged = True
                break
            repeated = [a for a in actions if tuple(a.command) in executed_keys]
            if repeated:
                for action in repeated:
                    failures.append(
                        f"non-convergence: {action.description} replanned after "
                        "execution — the write is not becoming visible to re-reads"
                    )
                residual = [action.command for action in actions]
                break
            rerun_required = False
            for action in actions:
                if action.view_update is not None:
                    conflict = _apply_view_update(
                        transport, action, executed, executed_keys, overwritten, echo
                    )
                    if conflict is not None:
                        failures.append(conflict)
                    continue
                echo(f"- {action.description}")
                transport.run(action.command)
                executed.append(list(action.command))
                executed_keys.add(tuple(action.command))
                if action.creates_view is not None:
                    adopt_failure = _adopt_created_view(
                        transport, registry, registry_path, action.creates_view, echo
                    )
                    if adopt_failure is not None:
                        failures.append(adopt_failure)
                if action.command[:3] == ["gh", "project", "create"]:
                    rerun_required = True
                    failures.append(
                        f"project {PROJECT_TITLE!r} created; rerun required so the "
                        f"sync adopts project number {PROJECT_NUMBER} (projects are "
                        "never resolved by title)"
                    )
            if failures:
                if not rerun_required:
                    residual = [
                        action.command
                        for action in build_plan(
                            transport,
                            registry,
                            allow_create_project=allow_create_project,
                        )
                    ]
                break
        if not converged and not failures:
            remaining = build_plan(
                transport, registry, allow_create_project=allow_create_project
            )
            if remaining:
                failures.append(
                    f"non-convergence: plan still holds {len(remaining)} action(s) "
                    f"after {MAX_APPLY_PASSES} passes"
                )
                residual = [action.command for action in remaining]
            else:
                converged = True
    except SyncError as error:
        if not executed and not overwritten:
            raise
        failures.extend(error.diagnostics)
        return 2, _receipt(executed, overwritten, "failed", residual, failures)
    except subprocess.CalledProcessError as error:
        if not executed and not overwritten:
            raise
        command = error.cmd if isinstance(error.cmd, list) else [str(error.cmd)]
        failures.append(
            f"command failed ({error.returncode}): "
            f"{json.dumps(command, ensure_ascii=False)}: {(error.stderr or '').strip()}"
        )
        return 3, _receipt(executed, overwritten, "failed", residual, failures)
    result = "converged" if converged and not failures else "failed"
    code = 0 if result == "converged" else 3
    return code, _receipt(executed, overwritten, result, residual, failures)


def run_adopt_view(
    transport: Transport,
    name: str,
    registry_path: Path,
    *,
    echo: Callable[[str], object] = print,
) -> int:
    if name not in SPEC_VIEW_NAMES:
        raise SyncError(
            f"{name!r} is not a managed view name; only managed spec views may "
            f"enter the ownership registry (managed: {', '.join(SPEC_VIEW_NAMES)})"
        )
    registry = load_view_registry(registry_path)
    matches = [view for view in read_view_states(transport) if view.name == name]
    if not matches:
        raise SyncError(f"no live view named {name!r} exists to adopt")
    if len(matches) > 1:
        ids = ", ".join(view.view_id for view in matches)
        raise SyncError(
            f"duplicate live views named {name!r}: {ids} — rename or remove the "
            "extras manually before adopting"
        )
    registry[name] = matches[0].view_id
    save_view_registry(registry_path, registry)
    echo(
        f"project-sync: adopted view {name!r} → {matches[0].view_id} in "
        f"{registry_path}; commit this change"
    )
    return 0


def validate_receipt_path(raw: str) -> Path:
    path = Path(raw).expanduser()
    if not path.is_absolute():
        path = Path.cwd() / path
    resolved = path.resolve()
    if resolved.is_relative_to(REPO_ROOT):
        raise SyncError(
            f"receipt path {resolved} is inside the repository tree {REPO_ROOT}; "
            "receipts never enter git — choose a path outside the repo"
        )
    return resolved


class FakeTransport:
    """In-memory transport: argv tuple → queue of canned responses."""

    def __init__(self, responses: dict[tuple[str, ...], list[object]]) -> None:
        self.responses = {key: list(value) for key, value in responses.items()}
        self.calls: list[list[str]] = []

    def run(
        self, command: list[str], *, check: bool = True
    ) -> subprocess.CompletedProcess[str]:
        self.calls.append(list(command))
        key = tuple(command)
        if key not in self.responses:
            raise AssertionError(f"unexpected command: {command}")
        queue = self.responses[key]
        payload = queue.pop(0) if len(queue) > 1 else queue[0]
        stdout = payload if isinstance(payload, str) else json.dumps(payload)
        return subprocess.CompletedProcess(command, 0, stdout=stdout, stderr="")


def _silent(_: str) -> None:
    return None


def _page(
    nodes: list[dict[str, object]], *, has_next: bool = False, cursor: str | None = None
) -> dict[str, object]:
    return {"pageInfo": {"hasNextPage": has_next, "endCursor": cursor}, "nodes": nodes}


def _field_node(
    name: str, data_type: str, options: tuple[str, ...] = ()
) -> dict[str, object]:
    node: dict[str, object] = {
        "__typename": (
            "ProjectV2SingleSelectField"
            if data_type == "SINGLE_SELECT"
            else "ProjectV2Field"
        ),
        "id": f"F_{name}",
        "name": name,
        "dataType": data_type,
    }
    if data_type == "SINGLE_SELECT":
        node["options"] = [
            {"id": f"O_{name}_{i}", "name": option} for i, option in enumerate(options)
        ]
    return node


def _default_fields() -> list[dict[str, object]]:
    nodes = [
        _field_node("Title", "TITLE"),
        _field_node("Status", "SINGLE_SELECT", ("Todo", "In Progress", "Done")),
        _field_node("Assignees", "ASSIGNEES"),
        _field_node("Repository", "REPOSITORY"),
    ]
    for name, (data_type, options) in FIELDS.items():
        nodes.append(_field_node(name, data_type, required_options(options)))
    return nodes


def _spec_required_ids(spec: ViewSpec, field_nodes: list[dict[str, object]]) -> list[str]:
    ids = {str(node["name"]): str(node["id"]) for node in field_nodes}
    return [ids[name] for name in spec.visible_fields if name in ids]


def _view_node(
    spec: ViewSpec,
    field_nodes: list[dict[str, object]],
    *,
    view_id: str | None = None,
    filter_override: str | None = None,
    layout_override: str | None = None,
) -> dict[str, object]:
    visible = [
        {"__typename": "ProjectV2Field", "id": field_id, "name": ""}
        for field_id in _spec_required_ids(spec, field_nodes)
    ]
    return {
        "id": view_id or f"V_{spec.name}",
        "name": spec.name,
        "layout": layout_override or spec.layout,
        "filter": spec.filter_query if filter_override is None else filter_override,
        "configuration": {"visibleFields": _page(visible)},
    }


def _view_node_response(node: dict[str, object]) -> dict[str, object]:
    return {"data": {"node": {"__typename": "ProjectV2View", **node}}}


def _resolve_response(present: bool = True) -> dict[str, object]:
    project = (
        {"id": "P_1", "title": PROJECT_TITLE, "number": PROJECT_NUMBER}
        if present
        else None
    )
    return {"data": {"user": {"projectV2": project}}}


def _fields_response(page: dict[str, object]) -> dict[str, object]:
    return {"data": {"user": {"projectV2": {"fields": page}}}}


def _views_response(view_nodes: list[dict[str, object]]) -> dict[str, object]:
    return {"data": {"user": {"projectV2": {"views": _page(view_nodes)}}}}


def _repos_response() -> dict[str, object]:
    return {
        "data": {
            "node": {"repositories": _page([{"nameWithOwner": REPO_SLUG}])}
        }
    }


def _default_registry() -> dict[str, str]:
    return {spec.name: f"V_{spec.name}" for spec in VIEW_SPECS}


_ISSUE_URL = f"https://github.com/{REPO_SLUG}/issues/1"


def _board_responses(
    *,
    fields: list[dict[str, object]] | None = None,
    views: list[dict[str, object]] | None = None,
    labels: list[dict[str, object]] | None = None,
    items: list[str] | None = None,
    issues: list[str] | None = None,
    prs: list[str] | None = None,
    project_present: bool = True,
) -> dict[tuple[str, ...], list[object]]:
    field_nodes = fields if fields is not None else _default_fields()
    view_nodes = (
        views
        if views is not None
        else [_view_node(spec, field_nodes) for spec in VIEW_SPECS]
    )
    label_rows = (
        labels
        if labels is not None
        else [{"name": name, "color": color} for name, color in LABELS.items()]
    )
    item_urls = items if items is not None else [_ISSUE_URL]
    issue_urls = issues if issues is not None else [_ISSUE_URL]
    pr_urls = prs if prs is not None else []
    login_vars: dict[str, object] = {"login": OWNER, "number": PROJECT_NUMBER}
    return {
        tuple(graphql_command(PROJECT_RESOLVE_QUERY, login_vars)): [
            _resolve_response(project_present)
        ],
        tuple(graphql_command(PROJECT_FIELDS_QUERY, login_vars)): [
            _fields_response(_page(field_nodes))
        ],
        tuple(graphql_command(PROJECT_VIEWS_QUERY, login_vars)): [
            _views_response(view_nodes)
        ],
        tuple(graphql_command(PROJECT_REPOSITORIES_QUERY, {"id": "P_1"})): [
            _repos_response()
        ],
        tuple(item_list_command()): [
            {"items": [{"content": {"url": url}} for url in item_urls]}
        ],
        tuple(label_list_command()): [label_rows],
        tuple(open_list_command("issue")): [[{"url": url} for url in issue_urls]],
        tuple(open_list_command("pr")): [[{"url": url} for url in pr_urls]],
    }


def _expect_drift(
    responses: dict[tuple[str, ...], list[object]],
    registry: dict[str, str],
    *needles: str,
) -> None:
    transport = FakeTransport(responses)
    try:
        build_plan(transport, registry)
    except SyncError as error:
        message = str(error)
        for needle in needles:
            assert needle in message, f"missing {needle!r} in: {message}"
        return
    raise AssertionError(f"expected SyncError mentioning {needles}, plan succeeded")


def _case_wrong_type_squat() -> None:
    fields = [n for n in _default_fields() if n["name"] != "Priority"]
    fields.append(_field_node("Priority", "TEXT"))
    _expect_drift(
        _board_responses(fields=fields),
        _default_registry(),
        "Priority",
        "TEXT",
        "SINGLE_SELECT",
    )


def _case_duplicate_target_date() -> None:
    fields = _default_fields()
    fields.append(
        {
            "__typename": "ProjectV2Field",
            "id": "F_TargetDate2",
            "name": "Target date",
            "dataType": "TEXT",
        }
    )
    _expect_drift(
        _board_responses(fields=fields),
        _default_registry(),
        "Target date",
        "DATE",
        "TEXT",
    )


def _case_missing_option() -> None:
    fields = [n for n in _default_fields() if n["name"] != "Priority"]
    fields.append(_field_node("Priority", "SINGLE_SELECT", ("P0", "P1", "P2")))
    _expect_drift(
        _board_responses(fields=fields), _default_registry(), "Priority", "P3"
    )


def _case_list_truncation() -> None:
    labels = [
        {"name": f"label-{i}", "color": "ffffff"} for i in range(LABEL_LIST_LIMIT)
    ]
    _expect_drift(
        _board_responses(labels=labels),
        _default_registry(),
        "truncation",
        str(LABEL_LIST_LIMIT),
    )


def _case_fields_pagination() -> None:
    field_nodes = _default_fields()
    first, second = field_nodes[:5], field_nodes[5:]
    responses = _board_responses()
    login_vars: dict[str, object] = {"login": OWNER, "number": PROJECT_NUMBER}
    responses[tuple(graphql_command(PROJECT_FIELDS_QUERY, login_vars))] = [
        _fields_response(_page(first, has_next=True, cursor="C1"))
    ]
    page2_key = tuple(
        graphql_command(PROJECT_FIELDS_QUERY, {**login_vars, "cursor": "C1"})
    )
    responses[page2_key] = [_fields_response(_page(second))]
    transport = FakeTransport(responses)
    actions = build_plan(transport, _default_registry())
    assert actions == [], f"page-2 fields were not seen: {[a.description for a in actions]}"
    assert list(page2_key) in transport.calls, "cursor page was never requested"


def _case_project_number_miss() -> None:
    _expect_drift(
        _board_responses(project_present=False),
        _default_registry(),
        f"project number {PROJECT_NUMBER} not found",
        "--allow-create-project",
    )


def _case_unregistered_view() -> None:
    registry = _default_registry()
    del registry["Critical Path"]
    _expect_drift(
        _board_responses(),
        registry,
        "V_Critical Path",
        "--adopt-view",
    )
    registry = _default_registry()
    registry["Critical Path"] = "V_old"
    _expect_drift(
        _board_responses(),
        registry,
        "V_old",
        "V_Critical Path",
        "--adopt-view",
    )


def _case_duplicate_view_names() -> None:
    field_nodes = _default_fields()
    views = [_view_node(spec, field_nodes) for spec in VIEW_SPECS]
    spec = next(s for s in VIEW_SPECS if s.name == "Critical Path")
    views.append(_view_node(spec, field_nodes, view_id="V_dup"))
    _expect_drift(
        _board_responses(views=views),
        _default_registry(),
        "duplicate live views",
        "V_Critical Path",
        "V_dup",
    )


def _case_concurrent_view_edit() -> None:
    field_nodes = _default_fields()
    spec = next(s for s in VIEW_SPECS if s.name == "Critical Path")
    views = [
        _view_node(s, field_nodes)
        if s.name != "Critical Path"
        else _view_node(s, field_nodes, filter_override="human filter")
        for s in VIEW_SPECS
    ]
    responses = _board_responses(views=views)
    fresh_node = _view_node(spec, field_nodes, filter_override="human filter edited again")
    responses[
        tuple(graphql_command(VIEW_NODE_QUERY, {"id": "V_Critical Path"}))
    ] = [_view_node_response(fresh_node)]
    transport = FakeTransport(responses)
    code, receipt = run_apply(transport, _default_registry(), echo=_silent)
    assert code == 3, f"expected exit 3, got {code}"
    assert receipt["result"] == "failed"
    assert receipt["executed"] == [], f"a write was recorded: {receipt['executed']}"
    failures = receipt["failures"]
    assert isinstance(failures, list) and any(
        "concurrent edit" in str(f) for f in failures
    ), failures
    for call in transport.calls:
        assert not any("updateProjectV2View" in part for part in call), (
            f"a view write was issued despite the conflict: {call}"
        )


def _acceptance_record_create_command() -> list[str]:
    return [
        "gh", "project", "field-create", str(PROJECT_NUMBER),
        "--owner", OWNER, "--name", "Acceptance record", "--data-type", "TEXT",
    ]


def _case_convergence_success() -> None:
    fields_without = [
        n for n in _default_fields() if n["name"] != "Acceptance record"
    ]
    fields_with = _default_fields()
    responses = _board_responses(fields=fields_without)
    login_vars: dict[str, object] = {"login": OWNER, "number": PROJECT_NUMBER}
    responses[tuple(graphql_command(PROJECT_FIELDS_QUERY, login_vars))] = [
        _fields_response(_page(fields_without)),
        _fields_response(_page(fields_with)),
    ]
    create = _acceptance_record_create_command()
    responses[tuple(create)] = [""]
    transport = FakeTransport(responses)
    code, receipt = run_apply(transport, _default_registry(), echo=_silent)
    assert code == 0, f"expected exit 0, got {code}: {receipt}"
    assert receipt["result"] == "converged"
    assert receipt["executed"] == [create], receipt["executed"]
    assert receipt["residual"] == [] and receipt["failures"] == []


def _case_non_convergence() -> None:
    fields_without = [
        n for n in _default_fields() if n["name"] != "Acceptance record"
    ]
    responses = _board_responses(fields=fields_without)
    create = _acceptance_record_create_command()
    responses[tuple(create)] = [""]
    transport = FakeTransport(responses)
    code, receipt = run_apply(transport, _default_registry(), echo=_silent)
    assert code == 3, f"expected exit 3, got {code}"
    assert receipt["result"] == "failed"
    failures = receipt["failures"]
    assert isinstance(failures, list) and any(
        "non-convergence" in str(f) for f in failures
    ), failures
    executions = [call for call in transport.calls if call == create]
    assert len(executions) == 1, f"create ran {len(executions)} times"


def _case_malformed_json() -> None:
    responses = _board_responses()
    responses[tuple(label_list_command())] = ["{not json"]
    _expect_drift(responses, _default_registry(), "unparseable gh output")


def _case_repo_scoped_argv() -> None:
    labels = [
        {"name": name, "color": color}
        for name, color in LABELS.items()
        if name != "triage"
    ]
    second_issue = f"https://github.com/{REPO_SLUG}/issues/2"
    responses = _board_responses(
        labels=labels, issues=[_ISSUE_URL, second_issue], items=[_ISSUE_URL]
    )
    transport = FakeTransport(responses)
    actions = build_plan(transport, _default_registry())
    label_creates = [a for a in actions if a.command[:3] == ["gh", "label", "create"]]
    assert label_creates, "missing label create action"
    for action in label_creates:
        index = action.command.index("-R")
        assert action.command[index + 1] == REPO_SLUG, action.command
    assert any(
        a.command[:4] == ["gh", "project", "item-add", str(PROJECT_NUMBER)]
        for a in actions
    ), "missing item-add action"
    for call in transport.calls:
        if len(call) > 1 and call[1] in ("label", "issue", "pr"):
            index = call.index("-R")
            assert call[index + 1] == REPO_SLUG, call


def _case_registry_round_trip() -> None:
    with tempfile.TemporaryDirectory() as tmp:
        path = Path(tmp) / "project_sync_views.lock.json"
        seed = _default_registry()
        save_view_registry(path, seed)
        assert load_view_registry(path) == seed
        field_nodes = _default_fields()
        views = [
            _view_node(s, field_nodes)
            if s.name != "Critical Path"
            else _view_node(s, field_nodes, view_id="V_new")
            for s in VIEW_SPECS
        ]
        transport = FakeTransport(_board_responses(views=views))
        code = run_adopt_view(transport, "Critical Path", path, echo=_silent)
        assert code == 0
        updated = load_view_registry(path)
        assert updated["Critical Path"] == "V_new", updated
        assert set(updated) == set(seed)
        try:
            run_adopt_view(transport, "Review Queue", path, echo=_silent)
        except SyncError:
            pass
        else:
            raise AssertionError("adopting the human view 'Review Queue' succeeded")
        assert "Review Queue" not in load_view_registry(path)


def self_test() -> int:
    cases: list[tuple[str, Callable[[], None]]] = [
        ("a: wrong-type squat (TEXT field named Priority)", _case_wrong_type_squat),
        ("b: duplicate Target date (DATE + TEXT)", _case_duplicate_target_date),
        ("c: missing single-select option", _case_missing_option),
        ("d: gh list result at limit → truncation error", _case_list_truncation),
        ("e: GraphQL hasNextPage followed across 2 pages", _case_fields_pagination),
        ("f: project number miss → typed error", _case_project_number_miss),
        ("g: unregistered same-name view → adopt required", _case_unregistered_view),
        ("h: duplicate live view names → error", _case_duplicate_view_names),
        ("i: concurrent view edit → conflict, no write", _case_concurrent_view_edit),
        ("j: apply convergence → empty fresh plan, exit 0", _case_convergence_success),
        ("k: non-convergence → exit 3", _case_non_convergence),
        ("l: malformed transport JSON → typed error", _case_malformed_json),
        ("m: repo-scoped argvs carry -R owner/repo", _case_repo_scoped_argv),
        ("n: registry lock round-trip incl. --adopt-view", _case_registry_round_trip),
    ]
    failures = 0
    for label, case in cases:
        try:
            case()
        except Exception as error:  # noqa: BLE001 — report every fixture failure
            failures += 1
            print(f"project-sync self-test FAIL: {label}: {error!r}")
    if failures:
        print(f"project-sync self-test: {failures} failure(s)")
        return 1
    print(f"project-sync self-test: PASS ({len(cases)} cases)")
    return 0


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    group = parser.add_mutually_exclusive_group(required=True)
    group.add_argument("--plan", action="store_true")
    group.add_argument("--apply", action="store_true")
    group.add_argument("--adopt-view", metavar="NAME")
    group.add_argument("--self-test", action="store_true")
    parser.add_argument("--allow-create-project", action="store_true")
    parser.add_argument("--receipt", metavar="PATH")
    args = parser.parse_args()

    if args.self_test:
        return self_test()

    try:
        if args.receipt and not args.apply:
            raise SyncError("--receipt requires --apply")
        receipt_path = validate_receipt_path(args.receipt) if args.receipt else None

        transport = GhTransport()
        auth = transport.run(["gh", "auth", "status"], check=False)
        if auth.returncode != 0:
            raise SyncError("gh is not authenticated; run `gh auth login` first")

        if args.adopt_view:
            return run_adopt_view(transport, args.adopt_view, VIEW_REGISTRY_PATH)

        registry = load_view_registry(VIEW_REGISTRY_PATH)
        if args.plan:
            return run_plan(
                transport, registry, allow_create_project=args.allow_create_project
            )

        code, receipt = run_apply(
            transport,
            registry,
            registry_path=VIEW_REGISTRY_PATH,
            allow_create_project=args.allow_create_project,
        )
        rendered = json.dumps(receipt, indent=2, ensure_ascii=False)
        print(rendered)
        if receipt_path is not None:
            try:
                receipt_path.write_text(rendered + "\n", encoding="utf-8")
            except OSError as error:
                print(
                    f"project-sync error: cannot write receipt to {receipt_path}: "
                    f"{error}",
                    file=sys.stderr,
                )
                return max(code, 3)
        return code
    except SyncError as error:
        for line in error.diagnostics:
            print(f"project-sync error: {line}", file=sys.stderr)
        return 2


if __name__ == "__main__":
    try:
        raise SystemExit(main())
    except subprocess.CalledProcessError as error:
        sys.stderr.write(error.stderr or "")
        raise SystemExit(error.returncode)
