#!/usr/bin/env python3
"""Determinism deny-list gate (CI gate 0).

Kernel crates must be pure: no wall clock, no OS randomness, no hash-order
iteration, no threads, no I/O, no environment access, no global mutable
state, no unsafe code.

Honest scope statement (PR #1 hardening loops 1-4): this gate has two
layers with different strengths.

* SEMANTIC layer — structural manifest validation (Python `tomllib`, never
  substring matching):
  - every dependency of every crate, in every table form (plain, dotted,
    target-specific, dev/build, workspace-inherited), must be a path
    dependency resolving inside this repository AND bound to the target
    manifest by name (a path dep whose target manifest declares a
    different package name is rejected);
  - `Cargo.lock` must contain exactly the workspace crates, bound by
    name AND version, all source-free;
  - every compiled source must live under a scanned package root: the
    workspace member list is bound to `crates/*` exactly (no globs, no
    outside members, no root `[package]`), and explicit `build` /
    `lib.path` / `bin`/`test`/`bench`/`example` path targets must resolve
    inside their crate directory — a `build = "../../support/build.rs"`
    that Cargo compiles but the scanner never reads is rejected
    (hardening-loop-4 BLOCKER);
  - `[patch]` and `[replace]` tables and in-repo `.cargo/config*` files
    are rejected outright;
  - `unsafe_code = "forbid"` must be enforced by rustc via
    `[workspace.lints.rust]` in the root manifest inherited by `[lints]
    workspace = true` in EVERY crate — covering lib, bin, and test
    targets alike. A comment mentioning forbid(unsafe_code) satisfies
    nothing here (that spoof was reproduced against the old substring
    check).
* LINE-REGEX layer — DEFENSE-IN-DEPTH, not semantic enforcement: aliasing
  and macros can evade any regex. The frozen reference vectors and the
  divergence gate catch what the regex misses at the behavioral level. A
  type-aware lint is planned for Phase 2.

Fail-closed classification: EVERY .rs file under crates/ is scanned with
the FULL pattern set unless it is explicitly listed as a boundary FILE or
carries an explicit per-file per-pattern exemption. There is no blanket
`tests` path heuristic — a `src/tests/mod.rs` module compiled into
production used to silently drop the global-atomic rule
(hardening-loop-4 BLOCKER); now an adversarial test fixture that needs an
atomic must register its exact file here.

Known, accepted regex miss (documented): float NaN/precision formatting —
`format!("{x}")` on an f64 is indistinguishable from other formatting at
line level.
"""

from __future__ import annotations

import os
import re
import sys

if sys.version_info < (3, 11):
    print(
        "Python >= 3.11 required: scripts/check_determinism_denylist.py "
        "uses the standard-library tomllib module",
        file=sys.stderr,
    )
    sys.exit(2)

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

ATOMIC_PATTERN = r"\bAtomic(Bool|Ptr|I8|I16|I32|I64|Isize|U8|U16|U32|U64|Usize)\b"

# Per-file, per-pattern exemptions (never whole-file, never whole-directory).
# - vh-verify soak binary (PR #2 timing-boundary ruling): wall-clock upH
#   telemetry that stays outside replay inputs and trace hashes.
# - vh-cli workloads: NondetDemo is an explicit nondeterminism fixture the
#   divergence gate must catch; its global atomic is the point.
# - vh-multiverse divergence tests: adversarial fixtures that use global
#   atomics to SIMULATE nondeterminism the detector must catch. This
#   replaces the old blanket "any path component named `tests`" heuristic,
#   which also suppressed the rule for production `src/tests/` modules
#   (hardening-loop-4 BLOCKER). A new adversarial test file must register
#   itself here explicitly.
EXEMPT: dict[str, set[str]] = {
    "crates/vh-verify/src/main.rs": {r"std::time", r"Instant::now"},
    "crates/vh-cli/src/workloads/mod.rs": {ATOMIC_PATTERN},
    "crates/vh-multiverse/tests/divergence.rs": {ATOMIC_PATTERN},
    # vh-verify's independent skip-vs-passing-invariant divergence
    # regressions simulate nondeterminism with global atomics on purpose
    # (registered during the operator-authorized restack; the fixture
    # predates the per-file exemption regime).
    "crates/vh-verify/tests/divergence.rs": {ATOMIC_PATTERN},
    # Tier-2 sandbox boundary crate (crates/vh-sandbox): it OWNS subprocess
    # execution, host filesystem I/O, and boundary wall-time telemetry that
    # never enters any identity digest. Exempted per-pattern rather than
    # whole-file (Lane-B review advisory 2026-07-22): every OTHER denied
    # pattern — HashMap/HashSet, threads, pointer-format, OS randomness,
    # net, os escape hatches — is still enforced on this crate, so an
    # accidental nondeterminism source that is NOT part of the declared
    # boundary is still caught.
    "crates/vh-sandbox/src/lib.rs": {
        r"std::process",
        r"std::time",
        r"Instant::now",
        r"std::io",
        r"std::fs",
    },
    # Sandbox unit tests: host tempdirs plus a process-global counter that
    # hands each test a unique workspace (adversarial-fixture style; the
    # atomic is the point). Clock/net/process rules still apply.
    "crates/vh-sandbox/src/tests.rs": {r"std::env", ATOMIC_PATTERN},
    # CLI sandbox-demo boundary: host tempdir setup for the run-twice smoke
    # campaign only. It spawns nothing directly (the crate does) and stays
    # under the full pattern set for everything else.
    "crates/vh-cli/src/sandbox_demo.rs": {r"std::env", r"std::fs"},
    # Evidence-store boundary (convergence C4, audit R4): receipt/bundle
    # file I/O for `vh run --out` and `vh replay-bundle`. Receipt CONTENT
    # is built and parsed by the pure vh_cli::receipts module (fully
    # deny-listed); this file only reads/writes those bytes, so clock,
    # env, net, process, and hash-order rules all still bind here.
    "crates/vh-cli/src/bundle.rs": {r"std::fs"},
}

# Pattern -> reason, applied to every scanned line of every scanned file.
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
    # Process-global atomic state: production kernel code never may; an
    # adversarial nondeterminism fixture must carry an explicit per-file
    # exemption above (no blanket tests-directory relaxation).
    ATOMIC_PATTERN: (
        "process-global atomic state; per-universe state lives in UniverseCtx "
        "(adversarial fixtures need an explicit per-file exemption)"
    ),
}

# --self-test regex corpus: (sample line, must_hit_in_src, must_hit_in_tests).
# src/tests distinction retained to PROVE there is no directory-based
# relaxation left: expectations are identical in both locations.
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
    ("static LEAK: AtomicU64 = AtomicU64::new(0);", True, True),
    ("let x: f64 = 0.5;", False, False),
    ("use std::collections::BTreeMap;", False, False),
    ("use std::sync::atomic::Ordering;", False, False),
    ("let zone = u64::MAX - (u64::MAX % n);", False, False),
]

# --self-test manifest corpus: (label, TOML text, expect_violation). Each
# fixture reproduces a real bypass (hardening loops 2 and 4) and must be
# flagged by the tomllib validator.
MANIFEST_SELF_TEST: list[tuple[str, str, bool]] = [
    (
        "good path dep, name-bound to the target manifest",
        '[package]\nname = "vh-x"\n[dependencies]\nvh-core = { path = "../vh-core" }\n'
        "[lints]\nworkspace = true\n",
        False,
    ),
    (
        "path dep name not matching the target manifest package name",
        '[package]\nname = "vh-x"\n[dependencies]\ngood = { path = "../vh-core" }\n'
        "[lints]\nworkspace = true\n",
        True,
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
        '[package]\nname = "vh-x"\n[dependencies]\nvh-core = { path = "../vh-core" }\n',
        True,
    ),
    (
        "lints inheritance disabled",
        '[package]\nname = "vh-x"\n[dependencies]\nvh-core = { path = "../vh-core" }\n'
        "[lints]\nworkspace = false\n",
        True,
    ),
    # Hardening-loop-4 BLOCKER: Cargo compiles/executes these, the line
    # scanner never reads them. All must be structural violations.
    (
        "build script outside the crate root (the reproduced bypass)",
        '[package]\nname = "vh-x"\nbuild = "../../support/build.rs"\n'
        "[lints]\nworkspace = true\n",
        True,
    ),
    (
        "build script inside the crate root is scanned, hence allowed",
        '[package]\nname = "vh-x"\nbuild = "build.rs"\n[lints]\nworkspace = true\n',
        False,
    ),
    (
        "absolute build script path",
        '[package]\nname = "vh-x"\nbuild = "/tmp/evil-build.rs"\n'
        "[lints]\nworkspace = true\n",
        True,
    ),
    (
        "lib target path escaping the crate root",
        '[package]\nname = "vh-x"\n[lib]\npath = "../../support/lib.rs"\n'
        "[lints]\nworkspace = true\n",
        True,
    ),
    (
        "bin target path escaping the crate root",
        '[package]\nname = "vh-x"\n[[bin]]\nname = "x"\npath = "../../support/main.rs"\n'
        "[lints]\nworkspace = true\n",
        True,
    ),
    (
        "test target path escaping the crate root",
        '[package]\nname = "vh-x"\n[[test]]\nname = "t"\npath = "../outside/t.rs"\n'
        "[lints]\nworkspace = true\n",
        True,
    ),
    (
        "in-crate explicit target paths are fine",
        '[package]\nname = "vh-x"\n[lib]\npath = "src/lib.rs"\n'
        '[[bin]]\nname = "x"\npath = "src/main.rs"\n[lints]\nworkspace = true\n',
        False,
    ),
    (
        "crate-level [patch] table",
        '[package]\nname = "vh-x"\n[lints]\nworkspace = true\n'
        '[patch.crates-io]\nserde = { path = "../evil" }\n',
        True,
    ),
    (
        "crate-level [replace] table",
        '[package]\nname = "vh-x"\n[lints]\nworkspace = true\n'
        '[replace]\n"foo:1.0.0" = { path = "../evil" }\n',
        True,
    ),
]

# --self-test root-manifest corpus: (label, TOML text, expect_violation).
# The scan root is only meaningful if the workspace member list is bound
# to it (hardening-loop-4 BLOCKER: a member outside crates/, a member
# glob, or a root [package] would compile sources the scanner never
# walks).
ROOT_SELF_TEST: list[tuple[str, str, bool]] = [
    (
        "good virtual root",
        '[workspace]\nresolver = "2"\nmembers = ["crates/vh-core"]\n'
        '[workspace.lints.rust]\nunsafe_code = "forbid"\n',
        False,
    ),
    (
        "member outside crates/",
        '[workspace]\nmembers = ["crates/vh-core", "support/helper"]\n'
        '[workspace.lints.rust]\nunsafe_code = "forbid"\n',
        True,
    ),
    (
        "member glob defeats explicit scan-root binding",
        '[workspace]\nmembers = ["crates/*"]\n'
        '[workspace.lints.rust]\nunsafe_code = "forbid"\n',
        True,
    ),
    (
        "root [package] compiles sources outside crates/",
        '[workspace]\nmembers = ["crates/vh-core"]\n'
        '[workspace.lints.rust]\nunsafe_code = "forbid"\n'
        '[package]\nname = "root"\nversion = "0.0.1"\n',
        True,
    ),
    (
        "root [patch] table",
        '[workspace]\nmembers = ["crates/vh-core"]\n'
        '[workspace.lints.rust]\nunsafe_code = "forbid"\n'
        '[patch.crates-io]\nserde = { path = "evil" }\n',
        True,
    ),
    (
        "root [replace] table",
        '[workspace]\nmembers = ["crates/vh-core"]\n'
        '[workspace.lints.rust]\nunsafe_code = "forbid"\n'
        '[replace]\n"foo:1.0.0" = { path = "evil" }\n',
        True,
    ),
    (
        "missing unsafe_code forbid",
        '[workspace]\nmembers = ["crates/vh-core"]\n',
        True,
    ),
]


def patterns_for(rel_path: str) -> dict[str, str]:
    """The full deny-list minus this exact file's explicit per-pattern
    exemptions. No directory heuristics: `src/tests/mod.rs` and an
    unregistered `tests/*.rs` file get the full set."""
    pats = dict(DENYLIST)
    for exempt_pattern in EXEMPT.get(rel_path, set()):
        pats.pop(exempt_pattern, None)
    return pats


def all_crates() -> list[Path]:
    crates_dir = REPO / "crates"
    return sorted(d for d in crates_dir.iterdir() if d.is_dir())


DEP_TABLE_KEYS = ("dependencies", "dev-dependencies", "build-dependencies")

# Cargo target tables whose explicit `path` keys point at compiled sources.
TARGET_TABLE_KEYS = ("bin", "test", "bench", "example")


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


def _resolves_inside(base: Path, candidate: str, boundary: Path) -> bool:
    if Path(candidate).is_absolute():
        return False
    return (base / candidate).resolve().is_relative_to(boundary)


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
    # Name binding (hardening-loop-4 BLOCKER: the lock check used to bind
    # by name alone): the target manifest must declare exactly the package
    # name this dependency claims — `package = "..."` renames included.
    expected = spec.get("package", name)
    target_manifest = resolved / "Cargo.toml"
    try:
        target_data = tomllib.loads(target_manifest.read_text(encoding="utf-8"))
    except (OSError, tomllib.TOMLDecodeError) as e:
        return [f"{where}: unreadable target manifest {path!r}/Cargo.toml ({e}) — fail closed"]
    actual = (target_data.get("package") or {}).get("name")
    if actual != expected:
        return [
            f"{where}: target manifest declares package {actual!r}, "
            f"dependency claims {expected!r} — name binding violated"
        ]
    return []


def _validate_target_paths(data: dict, manifest_rel: str, manifest_dir: Path) -> list[str]:
    """Every explicit compiled-source path in a manifest — `build`,
    `lib.path`, and `[[bin]]`/`[[test]]`/`[[bench]]`/`[[example]]` paths —
    must resolve INSIDE the crate directory, or Cargo compiles (and for
    build scripts, EXECUTES) source the scanner never reads
    (hardening-loop-4 BLOCKER; reproduced with
    `build = "../../support/build.rs"`)."""
    violations: list[str] = []
    build = (data.get("package") or {}).get("build")
    if isinstance(build, str) and not _resolves_inside(manifest_dir, build, manifest_dir):
        violations.append(
            f"{manifest_rel}: build script {build!r} lives outside the crate "
            "root — compiled+executed but never scanned"
        )
    lib_path = (data.get("lib") or {}).get("path")
    if isinstance(lib_path, str) and not _resolves_inside(manifest_dir, lib_path, manifest_dir):
        violations.append(
            f"{manifest_rel}: [lib] path {lib_path!r} lives outside the crate root"
        )
    for key in TARGET_TABLE_KEYS:
        for i, entry in enumerate(data.get(key) or []):
            if not isinstance(entry, dict):
                continue
            p = entry.get("path")
            if isinstance(p, str) and not _resolves_inside(manifest_dir, p, manifest_dir):
                violations.append(
                    f"{manifest_rel}: [[{key}]] #{i} path {p!r} lives outside the crate root"
                )
    return violations


def _validate_no_patch_replace(data: dict, manifest_rel: str) -> list[str]:
    violations = []
    for key in ("patch", "replace"):
        if key in data:
            violations.append(
                f"{manifest_rel}: [{key}] table redirects dependency resolution "
                "in a hermetic workspace — rejected"
            )
    return violations


def validate_manifest_data(
    data: dict,
    manifest_rel: str,
    manifest_dir: Path,
    root_path_deps: set[str],
    require_lints: bool = True,
) -> list[str]:
    """Structural (tomllib) validation of one crate manifest: hermetic
    name-bound dependencies in every table form, compiled-source paths
    confined to the crate root, no [patch]/[replace], plus rustc-enforced
    unsafe-code lints inherited from the workspace."""
    violations: list[str] = []
    for label, table in _dependency_tables(data):
        for name, spec in table.items():
            violations.extend(
                _validate_dep(label, name, spec, manifest_rel, manifest_dir, root_path_deps)
            )
    violations.extend(_validate_target_paths(data, manifest_rel, manifest_dir))
    violations.extend(_validate_no_patch_replace(data, manifest_rel))
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


def validate_root_data(data: dict) -> list[str]:
    """Structural validation of the workspace root manifest: virtual root
    (no [package]), members bound exactly to crates/ with no globs, no
    [patch]/[replace], and the rustc-enforced unsafe_code lint."""
    rel = "Cargo.toml"
    violations: list[str] = []
    if "package" in data:
        violations.append(
            f"{rel}: root [package] would compile sources outside the "
            "scanned crates/ roots — the root must stay a virtual manifest"
        )
    violations.extend(_validate_no_patch_replace(data, rel))
    workspace = data.get("workspace") or {}
    lints = (workspace.get("lints") or {}).get("rust") or {}
    if lints.get("unsafe_code") != "forbid":
        violations.append(
            f'{rel}: [workspace.lints.rust] must set unsafe_code = "forbid" '
            "(the rustc-enforced semantic layer)"
        )
    for member in workspace.get("members") or []:
        if not isinstance(member, str) or any(c in member for c in "*?["):
            violations.append(
                f"{rel}: workspace member {member!r} is a glob — members must "
                "be enumerated exactly so every compiled package root is scanned"
            )
            continue
        if not _resolves_inside(REPO, member, REPO / "crates"):
            violations.append(
                f"{rel}: workspace member {member!r} lives outside crates/ — "
                "its sources would never be scanned"
            )
    return violations


def check_root_manifest() -> tuple[list[str], set[str]]:
    """Validate the workspace root and return [workspace.dependencies]
    entries that are themselves valid local path deps (so crate-level
    `workspace = true` can be resolved)."""
    manifest = REPO / "Cargo.toml"
    rel = "Cargo.toml"
    try:
        data = tomllib.loads(manifest.read_text(encoding="utf-8"))
    except (OSError, tomllib.TOMLDecodeError) as e:
        return [f"{rel}: unreadable/unparseable root manifest ({e}) — fail closed"], set()
    violations = validate_root_data(data)
    root_path_deps: set[str] = set()
    workspace = data.get("workspace") or {}
    for name, spec in (workspace.get("dependencies") or {}).items():
        dep_violations = _validate_dep(
            "workspace.dependencies", name, spec, rel, REPO, set()
        )
        if dep_violations:
            violations.extend(dep_violations)
        else:
            root_path_deps.add(name)
    return violations, root_path_deps


def check_cargo_configs() -> list[str]:
    """In-repo `.cargo/config*` files can redirect registries, patch
    sources, and inject rustflags — reject them anywhere in the repo
    (hardening-loop-4 BLOCKER). Walk skips build/VCS artifact dirs."""
    violations = []
    skip = {".git", "target", "node_modules"}
    for dirpath, dirnames, filenames in os.walk(REPO):
        dirnames[:] = [d for d in dirnames if d not in skip]
        if Path(dirpath).name == ".cargo":
            for f in filenames:
                if f in ("config", "config.toml"):
                    rel = str((Path(dirpath) / f).relative_to(REPO))
                    violations.append(
                        f"{rel}: in-repo cargo config redirects resolution — rejected"
                    )
    return violations


def check_lockfile(expected_versions: dict[str, str]) -> list[str]:
    """Cargo.lock must contain exactly the workspace crates, bound by name
    AND version, all source-free. Name-only membership let a lock entry
    shadow a workspace crate at a different version
    (hardening-loop-4 BLOCKER: bind name+version+path, not name alone)."""
    lock = REPO / "Cargo.lock"
    rel = "Cargo.lock"
    if not lock.is_file():
        return [f"{rel}: missing lockfile — hermetic builds require --locked"]
    try:
        data = tomllib.loads(lock.read_text(encoding="utf-8"))
    except tomllib.TOMLDecodeError as e:
        return [f"{rel}: unparseable lockfile ({e}) — fail closed"]
    violations = []
    seen: set[str] = set()
    for pkg in data.get("package", []):
        name = pkg.get("name", "<unnamed>")
        version = pkg.get("version")
        if "source" in pkg:
            violations.append(
                f"{rel}: package '{name}' has a source "
                f"({pkg['source']!r}) — external packages in a hermetic workspace"
            )
        if name not in expected_versions:
            violations.append(
                f"{rel}: package '{name}' is not a workspace crate under crates/"
            )
        elif version != expected_versions[name]:
            violations.append(
                f"{rel}: package '{name}' locked at version {version!r} but the "
                f"workspace crate declares {expected_versions[name]!r}"
            )
        seen.add(name)
    for name in sorted(set(expected_versions) - seen):
        violations.append(
            f"{rel}: workspace crate '{name}' missing from the lockfile"
        )
    return violations


def crate_name_version(crate_dir: Path, workspace_version: str | None) -> tuple[str, str] | None:
    manifest = crate_dir / "Cargo.toml"
    if not manifest.is_file():
        return None
    try:
        data = tomllib.loads(manifest.read_text(encoding="utf-8"))
    except tomllib.TOMLDecodeError:
        return None
    package = data.get("package") or {}
    name = package.get("name")
    version = package.get("version")
    if isinstance(version, dict) and version.get("workspace") is True:
        version = workspace_version
    if not isinstance(name, str) or not isinstance(version, str):
        return None
    return name, version


def workspace_package_version() -> str | None:
    try:
        data = tomllib.loads((REPO / "Cargo.toml").read_text(encoding="utf-8"))
    except (OSError, tomllib.TOMLDecodeError):
        return None
    return ((data.get("workspace") or {}).get("package") or {}).get("version")


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
    for label, toml_text, expect_violation in ROOT_SELF_TEST:
        violations = validate_root_data(tomllib.loads(toml_text))
        if bool(violations) != expect_violation:
            failures += 1
            print(
                f"self-test FAIL [root] (expected "
                f"{'VIOLATION' if expect_violation else 'CLEAN'}, got {violations}): {label}"
            )
    # The scanner's own boundary contract: exemptions are per-file AND
    # per-pattern; unregistered files — src/tests modules included — get
    # the full set (hardening-loop-4 BLOCKER mutants).
    boundary_cases = [
        ("crates/vh-verify/src/main.rs", "Instant::now()", False),
        ("crates/vh-verify/src/lib.rs", "Instant::now()", True),
        ("crates/vh-verify/tests/replay.rs", "Instant::now()", True),
        # workloads.rs became workloads/mod.rs when the Phase-1 sim-runtime
        # workloads split the module (2026-07-21); the exemption moved with
        # it and the OLD path must now hit the full pattern set.
        ("crates/vh-cli/src/workloads/mod.rs", "static L: AtomicU64 = AtomicU64::new(0);", False),
        ("crates/vh-cli/src/workloads/mod.rs", "Instant::now()", True),
        ("crates/vh-cli/src/workloads.rs", "static L: AtomicU64 = AtomicU64::new(0);", True),
        ("crates/vh-cli/src/workloads/net.rs", "static L: AtomicU64 = AtomicU64::new(0);", True),
        ("crates/vh-cli/src/workloads/disk.rs", "static L: AtomicU64 = AtomicU64::new(0);", True),
        # The reproduced mutant: a production `src/tests/` module with a
        # process-global atomic used to pass under the blanket heuristic.
        ("crates/vh-core/src/tests/mod.rs", "static L: AtomicU64 = AtomicU64::new(0);", True),
        # Registered adversarial fixture file: atomic allowed, clock still denied.
        (
            "crates/vh-multiverse/tests/divergence.rs",
            "static L: AtomicU64 = AtomicU64::new(0);",
            False,
        ),
        ("crates/vh-multiverse/tests/divergence.rs", "Instant::now()", True),
        # An unregistered integration-test file fails closed.
        ("crates/vh-core/tests/adversarial.rs", "static L: AtomicU64 = AtomicU64::new(0);", True),
        # Tier-2 sandbox boundary crate: the DECLARED boundary patterns are
        # exempt, but every other denied pattern stays enforced — the
        # exemption is per-pattern, not whole-file (Lane-B advisory).
        ("crates/vh-sandbox/src/lib.rs", "let mut c = Command::new(&argv[0]);", False),
        ("crates/vh-sandbox/src/lib.rs", "let started = Instant::now();", False),
        ("crates/vh-sandbox/src/lib.rs", "let bytes = std::fs::read(&path)?;", False),
        ("crates/vh-sandbox/src/lib.rs", "let m: HashMap<u8, u8> = HashMap::new();", True),
        ("crates/vh-sandbox/src/lib.rs", "thread::spawn(|| {});", True),
        ("crates/vh-sandbox/src/tests.rs", "std::env::temp_dir()", False),
        ("crates/vh-sandbox/src/tests.rs", "static N: AtomicU64 = AtomicU64::new(0);", False),
        ("crates/vh-sandbox/src/tests.rs", "Instant::now()", True),
        ("crates/vh-cli/src/sandbox_demo.rs", "std::fs::create_dir_all(&p)?;", False),
        ("crates/vh-cli/src/sandbox_demo.rs", "let m: HashMap<u8, u8> = HashMap::new();", True),
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
        f"{len(MANIFEST_SELF_TEST)} manifest fixtures, {len(ROOT_SELF_TEST)} root "
        f"fixtures, {len(boundary_cases)} boundary cases)"
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
    violations.extend(check_cargo_configs())

    ws_version = workspace_package_version()
    expected_versions: dict[str, str] = {}
    for crate_dir in crates:
        nv = crate_name_version(crate_dir, ws_version)
        if nv is None:
            violations.append(
                f"{crate_dir.relative_to(REPO)}/Cargo.toml: cannot resolve "
                "package name+version — fail closed"
            )
        else:
            expected_versions[nv[0]] = nv[1]
        violations.extend(check_crate_manifest(crate_dir, root_path_deps))
    violations.extend(check_lockfile(expected_versions))

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
        "tomllib-validated hermetic with name+version-bound deps and "
        "crate-confined build/target paths, members bound to crates/, no "
        "[patch]/[replace]/.cargo-config, lockfile name+version-bound "
        'source-free, workspace unsafe_code = "forbid" inherited by every crate)'
    )
    return 0


if __name__ == "__main__":
    sys.exit(main())
