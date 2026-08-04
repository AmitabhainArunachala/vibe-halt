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
    "Target date": ("DATE", ""),
    "Owner lane": ("TEXT", ""),
}


@dataclass
class Action:
    description: str
    command: list[str]


def run(command: list[str], *, check: bool = True) -> subprocess.CompletedProcess[str]:
    return subprocess.run(command, text=True, capture_output=True, check=check)


def gh_json(*args: str) -> object:
    proc = run(["gh", *args])
    return json.loads(proc.stdout)


def plan() -> list[Action]:
    actions: list[Action] = []
    auth = run(["gh", "auth", "status"], check=False)
    if auth.returncode != 0:
        raise SystemExit("gh is not authenticated; run `gh auth login` first")

    existing_labels = {
        item["name"]
        for item in gh_json(
            "label", "list", "--limit", "200", "--json", "name", "color"
        )
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
            "--format",
            "json",
        )
        existing_fields = {field["name"] for field in fields["fields"]}
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
        actions = plan()
        if not actions:
            print("project-sync: no additive changes required")
            return 0
        for action in actions:
            print(f"- {action.description}")
        print("project-sync: plan only; no writes performed")
        return 0

    applied = 0
    for _ in range(5):
        actions = plan()
        if not actions:
            print(f"project-sync: applied {applied} additive action(s)")
            return 0
        for action in actions:
            print(f"- {action.description}")
            run(action.command)
            applied += 1
    raise SystemExit("project-sync: additive plan did not converge after five passes")


if __name__ == "__main__":
    try:
        raise SystemExit(main())
    except subprocess.CalledProcessError as error:
        sys.stderr.write(error.stderr)
        raise SystemExit(error.returncode)
