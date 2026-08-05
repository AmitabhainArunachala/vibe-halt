#!/usr/bin/env python3
"""Plan or apply Vibe Halt's additive GitHub project structure.

The command is intentionally conservative: it creates missing labels, creates
or links the named project, and adds current open issues/PRs. It never deletes,
closes, archives, resolves, merges, or changes branch protection.
"""

from __future__ import annotations

import argparse
import json
import subprocess
import sys
from dataclasses import dataclass

OWNER = "AmitabhainArunachala"
REPO = "vibe-halt"
PROJECT_TITLE = "vibe-halt — Evidence to Reality"

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

PROJECT_VIEWS_QUERY = """
query($id: ID!) {
  node(id: $id) {
    ... on ProjectV2 {
      views(first: 50) {
        nodes {
          id
          name
          layout
          filter
          configuration {
            visibleFields(first: 50) {
              nodes {
                __typename
                ... on ProjectV2Field { id name }
                ... on ProjectV2SingleSelectField { id name }
                ... on ProjectV2IterationField { id name }
                ... on ProjectV2MultiSelectField { id name }
              }
            }
          }
        }
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


@dataclass
class Action:
    description: str
    command: list[str]


def required_options(options: str) -> tuple[str, ...]:
    return tuple(option.strip() for option in options.split(",") if option.strip())


def field_drift_diagnostics(fields: list[dict[str, object]]) -> list[str]:
    """Report managed field drift without replacing any existing field."""
    fields_by_name = {
        str(field.get("name")): field for field in fields if field.get("name")
    }
    diagnostics: list[str] = []
    for name, (data_type, options) in FIELDS.items():
        field = fields_by_name.get(name)
        if field is None or data_type != "SINGLE_SELECT":
            continue
        field_type = field.get("type")
        if field_type != "ProjectV2SingleSelectField":
            diagnostics.append(
                f"field {name!r} exists as {field_type or 'an unknown type'}; "
                "expected a single-select field and left it unchanged"
            )
            continue
        option_rows = field.get("options")
        if not isinstance(option_rows, list):
            option_rows = []
        actual_options = {
            str(option.get("name"))
            for option in option_rows
            if isinstance(option, dict) and option.get("name")
        }
        missing = [
            option
            for option in required_options(options)
            if option not in actual_options
        ]
        if missing:
            diagnostics.append(
                f"single-select field {name!r} is missing required option(s): "
                f"{', '.join(missing)}; existing options were left unchanged"
            )
    return diagnostics


def append_graphql_list(
    command: list[str], variable: str, values: list[str]
) -> list[str]:
    for value in values:
        command.extend(["-F", f"{variable}[]={value}"])
    return command


def supports_visible_fields(spec: ViewSpec) -> bool:
    return spec.layout != "ROADMAP_LAYOUT"


def create_view_command(
    project_id: str, spec: ViewSpec, visible_field_ids: list[str]
) -> list[str]:
    mutation = (
        CREATE_VIEW_MUTATION
        if supports_visible_fields(spec)
        else CREATE_ROADMAP_VIEW_MUTATION
    )
    command = [
        "gh",
        "api",
        "graphql",
        "-f",
        f"query={mutation}",
        "-F",
        f"projectId={project_id}",
        "-f",
        f"name={spec.name}",
        "-f",
        f"layout={spec.layout}",
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
        "gh",
        "api",
        "graphql",
        "-f",
        f"query={mutation}",
        "-F",
        f"viewId={view_id}",
        "-f",
        f"layout={spec.layout}",
        "-f",
        f"filter={spec.filter_query}",
    ]
    if not supports_visible_fields(spec):
        return command
    return append_graphql_list(command, "visibleFieldIds", visible_field_ids)


def unique(values: list[str]) -> list[str]:
    return list(dict.fromkeys(values))


def plan_view_actions(
    project_id: str,
    fields: list[dict[str, object]],
    views: list[dict[str, object]],
) -> list[Action]:
    """Plan convergence, preserving extra fields on layouts that support them."""
    field_ids_by_name = {
        str(field.get("name")): str(field.get("id"))
        for field in fields
        if field.get("name") and field.get("id")
    }
    views_by_name: dict[str, dict[str, object]] = {}
    for view in views:
        if view.get("name"):
            views_by_name.setdefault(str(view["name"]), view)

    actions: list[Action] = []
    for spec in VIEW_SPECS:
        required_ids = []
        if supports_visible_fields(spec):
            required_ids = [
                field_ids_by_name[name]
                for name in spec.visible_fields
                if name in field_ids_by_name
            ]
        view = views_by_name.get(spec.name)
        if view is None:
            actions.append(
                Action(
                    f"create project view {spec.name}",
                    create_view_command(project_id, spec, required_ids),
                )
            )
            continue

        current_ids: list[str] = []
        if supports_visible_fields(spec):
            configuration = view.get("configuration")
            if not isinstance(configuration, dict):
                configuration = {}
            visible_fields = configuration.get("visibleFields")
            if not isinstance(visible_fields, dict):
                visible_fields = {}
            nodes = visible_fields.get("nodes")
            if not isinstance(nodes, list):
                nodes = []
            current_ids = [
                str(node["id"])
                for node in nodes
                if isinstance(node, dict) and node.get("id")
            ]
        desired_ids = unique([*current_ids, *required_ids])

        drift: list[str] = []
        if view.get("layout") != spec.layout:
            drift.append("layout")
        if str(view.get("filter") or "") != spec.filter_query:
            drift.append("filter")
        if supports_visible_fields(spec) and any(
            field_id not in current_ids for field_id in required_ids
        ):
            drift.append("visible fields")
        if drift:
            actions.append(
                Action(
                    f"update project view {spec.name} ({', '.join(drift)})",
                    update_view_command(str(view["id"]), spec, desired_ids),
                )
            )
    return actions


def run(command: list[str], *, check: bool = True) -> subprocess.CompletedProcess[str]:
    return subprocess.run(command, text=True, capture_output=True, check=check)


def gh_json(*args: str) -> object:
    proc = run(["gh", *args])
    return json.loads(proc.stdout)


def plan(*, diagnostics: list[str] | None = None) -> list[Action]:
    actions: list[Action] = []
    auth = run(["gh", "auth", "status"], check=False)
    if auth.returncode != 0:
        raise SystemExit("gh is not authenticated; run `gh auth login` first")

    existing_labels = {
        item["name"]
        for item in gh_json("label", "list", "--limit", "200", "--json", "name,color")
    }
    for name, color in LABELS.items():
        if name not in existing_labels:
            actions.append(
                Action(
                    f"create label {name}",
                    ["gh", "label", "create", name, "--color", color],
                )
            )

    projects = gh_json(
        "project", "list", "--owner", OWNER, "--limit", "100", "--format", "json"
    )
    matches = [p for p in projects["projects"] if p["title"] == PROJECT_TITLE]
    if matches:
        project = matches[0]
        project_number = str(project["number"])
    else:
        actions.append(
            Action(
                f"create project {PROJECT_TITLE}",
                [
                    "gh",
                    "project",
                    "create",
                    "--owner",
                    OWNER,
                    "--title",
                    PROJECT_TITLE,
                ],
            )
        )
        project_number = "<new>"

    if project_number != "<new>":
        project_id = project["id"]
        fields = gh_json(
            "project",
            "field-list",
            project_number,
            "--owner",
            OWNER,
            "--limit",
            "100",
            "--format",
            "json",
        )
        field_rows = fields["fields"]
        if diagnostics is not None:
            diagnostics.extend(field_drift_diagnostics(field_rows))
        existing_fields = {field["name"] for field in field_rows}
        for name, (data_type, options) in FIELDS.items():
            if name in existing_fields:
                continue
            command = [
                "gh",
                "project",
                "field-create",
                project_number,
                "--owner",
                OWNER,
                "--name",
                name,
                "--data-type",
                data_type,
            ]
            if options:
                command.extend(["--single-select-options", options])
            actions.append(Action(f"create project field {name}", command))

        view_payload = gh_json(
            "api",
            "graphql",
            "-f",
            f"query={PROJECT_VIEWS_QUERY}",
            "-F",
            f"id={project_id}",
        )
        view_rows = view_payload["data"]["node"]["views"]["nodes"]
        actions.extend(plan_view_actions(project_id, field_rows, view_rows))

        query = (
            "query($id:ID!){node(id:$id){... on ProjectV2{"
            "repositories(first:100){nodes{nameWithOwner}}}}}"
        )
        links = gh_json(
            "api",
            "graphql",
            "-f",
            f"query={query}",
            "-F",
            f"id={project_id}",
        )
        linked_repositories = {
            node["nameWithOwner"]
            for node in links["data"]["node"]["repositories"]["nodes"]
        }
        if f"{OWNER}/{REPO}" not in linked_repositories:
            actions.append(
                Action(
                    "link repository to project",
                    [
                        "gh",
                        "project",
                        "link",
                        project_number,
                        "--owner",
                        OWNER,
                        "--repo",
                        REPO,
                    ],
                )
            )

        items = gh_json(
            "project",
            "item-list",
            project_number,
            "--owner",
            OWNER,
            "--limit",
            "500",
            "--format",
            "json",
        )
        known_urls = {
            item.get("content", {}).get("url")
            for item in items["items"]
            if item.get("content")
        }
        for kind in ("issue", "pr"):
            rows = gh_json(
                kind,
                "list",
                "--state",
                "open",
                "--limit",
                "200",
                "--json",
                "url",
            )
            for row in rows:
                if row["url"] not in known_urls:
                    actions.append(
                        Action(
                            f"add {row['url']} to project",
                            [
                                "gh",
                                "project",
                                "item-add",
                                project_number,
                                "--owner",
                                OWNER,
                                "--url",
                                row["url"],
                            ],
                        )
                    )
    return actions


def main() -> int:
    parser = argparse.ArgumentParser()
    group = parser.add_mutually_exclusive_group(required=True)
    group.add_argument("--plan", action="store_true")
    group.add_argument("--apply", action="store_true")
    args = parser.parse_args()

    if args.plan:
        diagnostics: list[str] = []
        actions = plan(diagnostics=diagnostics)
        for diagnostic in diagnostics:
            print(f"project-sync warning: {diagnostic}", file=sys.stderr)
        if not actions:
            print("project-sync: no additive changes required")
            return 0
        for action in actions:
            print(f"- {action.description}")
        print("project-sync: plan only; no writes performed")
        return 0

    applied = 0
    executed: set[tuple[str, ...]] = set()
    emitted_diagnostics: set[str] = set()
    for _ in range(5):
        diagnostics = []
        actions = plan(diagnostics=diagnostics)
        for diagnostic in diagnostics:
            if diagnostic not in emitted_diagnostics:
                print(f"project-sync warning: {diagnostic}", file=sys.stderr)
                emitted_diagnostics.add(diagnostic)
        if not actions:
            print(f"project-sync: applied {applied} additive action(s)")
            return 0
        fresh = [action for action in actions if tuple(action.command) not in executed]
        if not fresh:
            print(
                "project-sync: no new additive actions; GitHub may still be "
                "converging, rerun to confirm"
            )
            return 0
        for action in fresh:
            print(f"- {action.description}")
            run(action.command)
            executed.add(tuple(action.command))
            applied += 1
    raise SystemExit("project-sync: additive plan did not converge after five passes")


if __name__ == "__main__":
    try:
        raise SystemExit(main())
    except subprocess.CalledProcessError as error:
        sys.stderr.write(error.stderr)
        raise SystemExit(error.returncode)
