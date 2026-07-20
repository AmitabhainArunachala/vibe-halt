#!/usr/bin/env python3
"""Determinism deny-list gate (CI gate 0).

Kernel crates must be pure: no wall clock, no OS randomness, no hash-order
iteration, no threads, no I/O, no environment access, no global mutable
state, no unsafe code.

Honest scope statement (PR #1 hardening loops 1-2): this gate has two
layers with different strengths.

* SEMANTIC layer — structural manifest validation (Python `tomllib`, never
  substring matching): every dependency of every crate, in every table
  form (plain, dotted, target-specific, dev/build, workspace-inherited),
  must be a path dependency resolving inside this repository; `Cargo.lock`
  must contain only source-free local packages; and `unsafe_code =
  "forbid"` must be enforced by rustc via `[workspace.lints.rust]` in the
  root manifest inherited by `[lints] workspace = true` in EVERY crate —
  covering lib, bin, and test targets alike. A comment mentioning
  forbid(unsafe_code) satisfies nothing here (that spoof was reproduced
  against the old substring check).
* LINE-REGEX layer — DEFENSE-IN-DEPTH, not semantic enforcement: aliasing
  and macros can evade any regex. The frozen reference vectors and the
  divergence gate catch what the regex misses at the behavioral level. A
  type-aware lint is planned for Phase 2.

Fail-closed classification: EVERY .rs file under crates/ is scanned unless
it is explicitly listed as a boundary FILE (per-file, never per-crate —
PR #1 hardening-loop-2 GAP: the whole vh-cli crate, including its
kernel-grade demo workloads, used to be exempt). A new crate or file is
kernel-grade by default — nobody has to remember to register it.

Known, accepted regex miss (documented): float NaN/precision formatting —
`format!("{x}")` on an f64 is indistinguishable from other formatting at
line level.
"""

from __future__ import annotations

import re
import sys
import tomllib
from pathlib import Path

REPO = Path(__file__).resolve().parent.parent

# Boundary FILES exempt from the line scan (they own argv/exit codes or
# spawn the CLI binary under test). Everything else under crates/ is
# scanned — fail closed. Manifest and lint checks still apply to every
# crate including these files' crates.
BOUNDARY_FILES = {
    "crates/vh-cli/src/main.rs",
    "crates/vh-cli/tests/cli_contract.rs",
}

# Per-file, per-pattern exemptions (never whole-file).
# - vh-verify soak binary (PR #2 timing-boundary ruling): wall-clock upH
#   telemetry that stays outside replay inputs and trace hashes.
# - vh-cli workloads: NondetDemo is an explicit nondeterminism fixture the
#   divergence gate must catch; its global atomic is the point.
EXEMPT: dict[str, set[str]] = {
    "crates/vh-verify/src/main.rs": {r"std::time", r"Instant::now"},
    "crates/vh-cli/src/workloads.rs": {
        r"\bAtomic(Bool|Ptr|I8|I16|I32|I64|Isize|U8|U16|U32|U64|Usize)\b"
    },
}

# Pattern -> reason, applied to every scanned line.
DENYLIST: dict[str, str] = {
    r"std::time": "wall-clock time; use vh_core::VirtualClock",
    r"Instant::now": "wall-clock time; use vh_core::VirtualClock",
    r"SystemTime": "wall-clock time; use vh_core::VirtualClock",
    r"\bHashMap\b": "hash-order iteration; use BTreeMap",
    r"\bHashSet\b": "hash-order iteration; use BTreeSet",
    r"RandomState": "hash randomization; use BTree collections",
    r"thread::spawn": "threads in the kernel; parallelism lives at the multiverse boundary",
    r"std::thread": "threads in the kernel; parallelism lives at the multiverse boundary",
    r"thread_local!": "thread-local mutable state; per-universe state lives in UniverseCtx",
    r"static\s+mut\b": "global mutable state; per-universe state lives in UniverseCtx",
    r"\brand\b\s*::": "external RNG; use vh_core::Xoshiro256pp streams",
    r"getrandom": "OS randomness; use the seed tree",
    r"std::env": "environment leakage; config enters via typed parameters",
    r"\benv!\s*\(": "compile-time environment capture; varies per build environment",
    r"option_env!": "compile-time environment capture; varies per build environment",
    r"std::fs": "filesystem I/O in the kernel; Tier-2 I/O goes through the sandbox layer",
    r"std::io": "OS I/O in the kernel",
    r"std::os": "OS-specific escape hatches in the kernel",
    r"std::net": "network I/O in the kernel; simulated network lands in Phase 1",
    r"std::process": "process control in the kernel; subprocess universes live in the Tier-2 sandbox",
    r"\{:p\}": "pointer-address formatting; addresses vary per run (ASLR)",
    r"\b(global_)?asm!": "inline assembly escape hatch",
    # Root-module aliasing defeats every literal std::<mod> rule below
    # (`use std as host; host::time::Instant::now()`), so alias the root
    # and you trip this instead (PR #1 hardening-loop BLOCKER).
    r"use\s+(std|core|alloc)\s+as\s+": "std/core root aliasing defeats the deny-list",
    r"extern\s+crate\s+(std|core|alloc)\s+as\s+": "std/core root aliasing defeats the deny-list",
    # Grouped imports would otherwise dodge the literal std::<mod> rules.
    r"use\s+std::\{[^}]*\b(fs|env|net|process|io|os|thread|time)\b": (
        "grouped import of a denied std module"
    ),
}

# Applied to src/ only: adversarial TEST fixtures legitimately use global
# atomics to simulate nondeterminism; production kernel code never may.
SRC_ONLY_DENYLIST: dict[str, str] = {
    r"\bAtomic(Bool|Ptr|I8|I16|I32|I64|Isize|U8|U16|U32|U64|Usize)\b": (
        "global atomic state in production kernel code; per-universe state lives in UniverseCtx"
    ),
}

# --self-test regex corpus: (sample line, must_hit_in_src, must_hit_in_tests).
SELF_TEST: list[tuple[str, bool, bool]] = [
    ('format!("{:p}", ptr)', True, True),
    ("RandomState::new()", True, True),
    ("std::io::stdin().read_line(&mut s)", True, True),
    ("std::os::unix::fs::MetadataExt::ino(&m)", True, True),
    ("use std::{fs, env, net};", True, True),
    ('option_env!("TARGET")', True, True),
    ('env!("CARGO_PKG_VERSION")', True, True),
    ("thread_local! { static X: Cell<u64> = Cell::new(0); }", True, True),
    ("static mut COUNTER: u64 = 0;", True, True),
    ('asm!("nop")', True, True),
    ("Instant::now()", True, True),
    ("use std as host;", True, True),
    ("extern crate std as host;", True, True),
    ("static LEAK: AtomicU64 = AtomicU64::new(0);", True, False),
    ("let x: f64 = 0.5;", False, False),
    ("use std::collections::BTreeMap;", False, False),
    ("use std::sync::atomic::Ordering;", False, False),
    ("let zone = u64::MAX - (u64::MAX % n);", False, False),
]

# --self-test manifest corpus: (label, TOML text, expect_violation). Each
# fixture reproduces a bypass of the old substring-based manifest check
# (PR #1 hardening-loop-2 BLOCKER) and must be flagged by the tomllib
# validator.
MANIFEST_SELF_TEST: list[tuple[str, str, bool]] = [
    (
        "good path dep",
        '[package]\nname = "vh-x"\n[dependencies]\ngood = { path = "../vh-core" }\n'
        "[lints]\nworkspace = true\n",
        False,
    ),
    (
        "comment path= spoof on a git dep",
        '[package]\nname = "vh-x"\n[dependencies]\n'
        'evil = { git = "https://example.com/evil" } # path = "decoy"\n'
        "[lints]\nworkspace = true\n",
        True,
    ),
    (
        "dotted-table git dep",
        '[package]\nname = "vh-x"\n[dependencies.evil]\ngit = "https://example.com/evil"\n'
        "[lints]\nworkspace = true\n",
        True,
    ),
    (
        "target-specific external dep",
        '[package]\nname = "vh-x"\n[target.\'cfg(unix)\'.dependencies]\nevil = "1.0"\n'
        "[lints]\nworkspace = true\n",
        True,
    ),
    (
        "registry version-string dep",
        '[package]\nname = "vh-x"\n[dependencies]\nevil = "1.0"\n'
        "[lints]\nworkspace = true\n",
        True,
    ),
    (
        "escaping relative path dep",
        '[package]\nname = "vh-x"\n[dependencies]\n'
        'evil = { path = "../../../outside" }\n[lints]\nworkspace = true\n',
        True,
    ),
    (
        "absolute path dep",
        '[package]\nname = "vh-x"\n[dependencies]\nevil = { path = "/etc/evil" }\n'
        "[lints]\nworkspace = true\n",
        True,
    ),
    (
        "workspace-inherited dep with no validated root entry",
        '[package]\nname = "vh-x"\n[dependencies]\nevil = { workspace = true }\n'
        "[lints]\nworkspace = true\n",
        True,
    ),
    (
        "path dep that also names a registry version",
        '[package]\nname = "vh-x"\n[dependencies]\n'
        'evil = { path = "../vh-core", version = "1.0" }\n[lints]\nworkspace = true\n',
        True,
    ),
    (
        "missing lints inheritance",
        '[package]\nname = "vh-x"\n[dependencies]\ngood = { path = "../vh-core" }\n',
        True,
    ),
    (
        "lints inheritance disabled",
        '[package]\nname = "vh-x"\n[dependencies]\ngood = { path = "../vh-core" }\n'
        "[lints]\nworkspace = false\n",
        True,
    ),
]


def patterns_for(rel_path: str) -> dict[str, str]:
    pats = dict(DENYLIST)
    if "tests" not in Path(rel_path).parts:
        pats.update(SRC_ONLY_DENYLIST)
    for exempt_pattern in EXEMPT.get(rel_path, set()):
        pats.pop(exempt_pattern, None)
    return pats


def all_crates() -> list[Path]:
    crates_dir = REPO / "crates"
    return sorted(d for d in crates_dir.iterdir() if d.is_dir())


DEP_TABLE_KEYS = ("dependencies", "dev-dependencies", "build-dependencies")


def _dependency_tables(data: dict) -> list[tuple[str, dict]]:
    """Every dependency table in a manifest, in every form tomllib can
    represent: plain, dotted (parses identically), dev/build, and
    target-specific."""
    tables: list[tuple[str, dict]] = []
    for key in DEP_TABLE_KEYS:
        if isinstance(data.get(key), dict):
            tables.append((key, data[key]))
    for target, target_data in (data.get("target") or {}).items():
        if not isinstance(target_data, dict):
            continue
        for key in DEP_TABLE_KEYS:
            if isinstance(target_data.get(key), dict):
                tables.append((f"target.{target}.{key}", target_data[key]))
    return tables


def _validate_dep(
    label: str,
    name: str,
    spec: object,
    manifest_rel: str,
    manifest_dir: Path,
    root_path_deps: set[str],
) -> list[str]:
    where = f"{manifest_rel}: [{label}] {name}"
    if isinstance(spec, str):
        return [f"{where}: registry dependency '{spec}' in a hermetic workspace"]
    if not isinstance(spec, dict):
        return [f"{where}: unrecognized dependency form {spec!r} — fail closed"]
    if spec.get("workspace") is True:
        if name in root_path_deps:
            return []
        return [
            f"{where}: workspace-inherited dependency has no validated local "
            "path entry in [workspace.dependencies]"
        ]
    for forbidden in ("git", "registry", "registry-index", "version"):
        if forbidden in spec:
            return [
                f"{where}: '{forbidden}' key on a dependency in a hermetic "
                "workspace (only pure local path dependencies are allowed)"
            ]
    path = spec.get("path")
    if not isinstance(path, str):
        return [f"{where}: no path key — not a local path dependency"]
    if Path(path).is_absolute():
        return [f"{where}: absolute dependency path {path!r} is nonportable"]
    resolved = (manifest_dir / path).resolve()
    if not resolved.is_relative_to(REPO):
        return [f"{where}: dependency path {path!r} escapes the repository"]
    return []


def validate_manifest_data(
    data: dict,
    manifest_rel: str,
    manifest_dir: Path,
    root_path_deps: set[str],
    require_lints: bool = True,
) -> list[str]:
    """Structural (tomllib) validation of one crate manifest: hermetic
    dependencies in every table form, plus rustc-enforced unsafe-code
    lints inherited from the workspace."""
    violations: list[str] = []
    for label, table in _dependency_tables(data):
        for name, spec in table.items():
            violations.extend(
                _validate_dep(label, name, spec, manifest_rel, manifest_dir, root_path_deps)
            )
    if require_lints and (data.get("lints") or {}).get("workspace") is not True:
        violations.append(
            f"{manifest_rel}: missing `[lints] workspace = true` — the crate "
            "does not inherit the workspace unsafe_code = \"forbid\" lint"
        )
    return violations


def check_crate_manifest(crate_dir: Path, root_path_deps: set[str]) -> list[str]:
    manifest = crate_dir / "Cargo.toml"
    rel = str(manifest.relative_to(REPO))
    if not manifest.is_file():
        return [f"{rel}: missing Cargo.toml"]
    try:
        data = tomllib.loads(manifest.read_text(encoding="utf-8"))
    except tomllib.TOMLDecodeError as e:
        return [f"{rel}: unparseable manifest ({e}) — fail closed"]
    return validate_manifest_data(data, rel, crate_dir, root_path_deps)


def check_root_manifest() -> tuple[list[str], set[str]]:
    """Validate the workspace root: unsafe_code = "forbid" lint present,
    and any [workspace.dependencies] entries are themselves local path
    deps (returned so crate-level `workspace = true` can be resolved)."""
    manifest = REPO / "Cargo.toml"
    rel = "Cargo.toml"
    violations: list[str] = []
    root_path_deps: set[str] = set()
    try:
        data = tomllib.loads(manifest.read_text(encoding="utf-8"))
    except (OSError, tomllib.TOMLDecodeError) as e:
        return [f"{rel}: unreadable/unparseable root manifest ({e}) — fail closed"], set()
    workspace = data.get("workspace") or {}
    lints = ((workspace.get("lints") or {}).get("rust") or {})
    if lints.get("unsafe_code") != "forbid":
        violations.append(
            f'{rel}: [workspace.lints.rust] must set unsafe_code = "forbid" '
            "(the rustc-enforced semantic layer)"
        )
    for name, spec in (workspace.get("dependencies") or {}).items():
        dep_violations = _validate_dep(
            "workspace.dependencies", name, spec, rel, REPO, set()
        )
        if dep_violations:
            violations.extend(dep_violations)
        else:
            root_path_deps.add(name)
    return violations, root_path_deps


def check_lockfile(crate_names: set[str]) -> list[str]:
    """Cargo.lock must contain only source-free local packages whose names
    are workspace crates: a registry or git package carries a `source`
    key and is a hermeticity violation."""
    lock = REPO / "Cargo.lock"
    rel = "Cargo.lock"
    if not lock.is_file():
        return [f"{rel}: missing lockfile — hermetic builds require --locked"]
    try:
        data = tomllib.loads(lock.read_text(encoding="utf-8"))
    except tomllib.TOMLDecodeError as e:
        return [f"{rel}: unparseable lockfile ({e}) — fail closed"]
    violations = []
    for pkg in data.get("package", []):
        name = pkg.get("name", "<unnamed>")
        if "source" in pkg:
            violations.append(
                f"{rel}: package '{name}' has a source "
                f"({pkg['source']!r}) — external packages in a hermetic workspace"
            )
        if name not in crate_names:
            violations.append(
                f"{rel}: package '{name}' is not a workspace crate under crates/"
            )
    return violations


def crate_package_name(crate_dir: Path) -> str | None:
    manifest = crate_dir / "Cargo.toml"
    if not manifest.is_file():
        return None
    try:
        data = tomllib.loads(manifest.read_text(encoding="utf-8"))
    except tomllib.TOMLDecodeError:
        return None
    return (data.get("package") or {}).get("name")


def self_test() -> int:
    failures = 0
    for sample, expect_src, expect_test in SELF_TEST:
        for kind, expected in (("src", expect_src), ("tests", expect_test)):
            pseudo_path = f"crates/vh-x/{kind}/sample.rs"
            hit = any(re.search(p, sample) for p in patterns_for(pseudo_path))
            if hit != expected:
                failures += 1
                print(
                    f"self-test FAIL [{kind}] (expected {'HIT' if expected else 'MISS'}, "
                    f"got {'HIT' if hit else 'MISS'}): {sample}"
                )
    for label, toml_text, expect_violation in MANIFEST_SELF_TEST:
        data = tomllib.loads(toml_text)
        violations = validate_manifest_data(
            data,
            "crates/vh-x/Cargo.toml",
            REPO / "crates" / "vh-x",
            root_path_deps=set(),
        )
        if bool(violations) != expect_violation:
            failures += 1
            print(
                f"self-test FAIL [manifest] (expected "
                f"{'VIOLATION' if expect_violation else 'CLEAN'}, got {violations}): {label}"
            )
    # The scanner's own boundary contract: std::time allowed ONLY in the
    # vh-verify soak binary, still denied in its lib and tests.
    boundary_cases = [
        ("crates/vh-verify/src/main.rs", "Instant::now()", False),
        ("crates/vh-verify/src/lib.rs", "Instant::now()", True),
        ("crates/vh-verify/tests/replay.rs", "Instant::now()", True),
        ("crates/vh-cli/src/workloads.rs", "static L: AtomicU64 = AtomicU64::new(0);", False),
        ("crates/vh-cli/src/workloads.rs", "Instant::now()", True),
    ]
    for rel, sample, expected in boundary_cases:
        hit = any(re.search(p, sample) for p in patterns_for(rel))
        if hit != expected:
            failures += 1
            print(
                f"self-test FAIL [boundary] (expected {'HIT' if expected else 'MISS'}, "
                f"got {'HIT' if hit else 'MISS'}): {rel}: {sample}"
            )
    if failures:
        print(f"denylist self-test: {failures} failure(s)")
        return 1
    print(
        f"denylist self-test: PASS ({len(SELF_TEST)} regex samples x src/tests, "
        f"{len(MANIFEST_SELF_TEST)} manifest fixtures, {len(boundary_cases)} boundary cases)"
    )
    return 0


def main() -> int:
    if "--self-test" in sys.argv:
        return self_test()

    crates = all_crates()
    if not crates:
        print("denylist: no crates found under crates/ — fail closed", file=sys.stderr)
        return 2

    violations: list[str] = []
    root_violations, root_path_deps = check_root_manifest()
    violations.extend(root_violations)

    crate_names: set[str] = set()
    for crate_dir in crates:
        name = crate_package_name(crate_dir)
        if name:
            crate_names.add(name)
        violations.extend(check_crate_manifest(crate_dir, root_path_deps))
    violations.extend(check_lockfile(crate_names))

    scanned = 0
    skipped_boundary = 0
    for crate_dir in crates:
        for path in sorted(crate_dir.rglob("*.rs")):
            rel = str(path.relative_to(REPO))
            if rel in BOUNDARY_FILES:
                skipped_boundary += 1
                continue
            pats = patterns_for(rel)
            scanned += 1
            for lineno, line in enumerate(
                path.read_text(encoding="utf-8").splitlines(), start=1
            ):
                for pattern, reason in pats.items():
                    if re.search(pattern, line):
                        violations.append(
                            f"{rel}:{lineno}: [{pattern}] {reason}\n    {line.strip()}"
                        )

    if violations:
        print(f"determinism deny-list: {len(violations)} violation(s):")
        for v in violations:
            print(f"  {v}")
        return 1

    print(
        f"determinism deny-list: PASS ({len(crates)} crates, {scanned} files scanned, "
        f"{skipped_boundary} declared boundary files skipped; manifests "
        "tomllib-validated hermetic, lockfile source-free, workspace "
        'unsafe_code = "forbid" inherited by every crate)'
    )
    return 0


if __name__ == "__main__":
    sys.exit(main())
