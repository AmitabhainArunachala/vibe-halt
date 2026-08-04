#!/usr/bin/env python3
"""Create one deterministic Vibe Halt release archive.

The script accepts only explicit inputs, uses a fixed tar member order,
timestamp, uid/gid, and gzip header, and asks the Rust release-metadata binary
to compute the executable digest. It never tags, publishes, or contacts
GitHub.
"""

from __future__ import annotations

import argparse
import gzip
import hashlib
import json
import os
import shutil
import subprocess
import tarfile
import tempfile
from pathlib import Path

REPO = Path(__file__).resolve().parent.parent
FIXED_MTIME = 0


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser()
    parser.add_argument("--version", required=True)
    parser.add_argument("--binary", required=True, type=Path)
    parser.add_argument("--target", required=True)
    parser.add_argument("--out", required=True, type=Path)
    return parser.parse_args()


def validate_label(value: str, label: str) -> str:
    allowed = "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789._-"
    if not value or any(c not in allowed for c in value):
        raise SystemExit(f"{label} contains unsupported characters: {value!r}")
    return value


def release_metadata(binary: Path, version: str, target: str) -> dict[str, object]:
    cmd = [
        "cargo",
        "run",
        "--quiet",
        "--locked",
        "--offline",
        "-p",
        "vh-cli",
        "--bin",
        "release_metadata",
        "--",
        "--binary",
        str(binary),
        "--version",
        version,
        "--target",
        target,
    ]
    proc = subprocess.run(cmd, cwd=REPO, check=True, capture_output=True, text=True)
    return json.loads(proc.stdout)


def mode_for(source: Path) -> int:
    return 0o755 if source.stat().st_mode & 0o111 else 0o644


def add_file(tar: tarfile.TarFile, source: Path, arcname: str, mode: int) -> None:
    info = tar.gettarinfo(str(source), arcname=arcname)
    info.mtime = FIXED_MTIME
    info.uid = 0
    info.gid = 0
    info.uname = ""
    info.gname = ""
    info.mode = mode
    with source.open("rb") as handle:
        tar.addfile(info, handle)


def sha256(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as handle:
        for chunk in iter(lambda: handle.read(1024 * 1024), b""):
            digest.update(chunk)
    return digest.hexdigest()


def main() -> int:
    args = parse_args()
    version = validate_label(args.version, "version")
    target = validate_label(args.target, "target")
    binary = args.binary.resolve()
    if not binary.is_file():
        raise SystemExit(f"release binary does not exist: {binary}")

    if args.out.exists():
        if not args.out.is_dir():
            raise SystemExit(f"release output is not a directory: {args.out}")
        if any(args.out.iterdir()):
            raise SystemExit(f"release output is not empty: {args.out}")
    else:
        args.out.mkdir(parents=True, exist_ok=False)
    archive_name = f"vh-{version}-{target}.tar.gz"
    archive_path = (args.out / archive_name).resolve()
    package_root = f"vh-{version}-{target}"

    metadata = release_metadata(binary, version, target)
    metadata_bytes = (
        json.dumps(metadata, sort_keys=True, separators=(",", ":")) + "\n"
    ).encode("utf-8")

    with tempfile.TemporaryDirectory(prefix="vh-release-") as tmp:
        tmpdir = Path(tmp)
        staged_binary = tmpdir / ("vh.exe" if binary.suffix == ".exe" else "vh")
        shutil.copyfile(binary, staged_binary)
        os.chmod(staged_binary, 0o755)
        metadata_path = tmpdir / "release-metadata.json"
        metadata_path.write_bytes(metadata_bytes)
        source_files: dict[str, Path] = {}
        for directory in (".github", "clients", "corpus", "crates", "docs", "scripts"):
            for source in (REPO / directory).rglob("*"):
                if source.is_file() and "__pycache__" not in source.parts:
                    source_files[source.relative_to(REPO).as_posix()] = source
        for name in (
            ".gitattributes",
            ".gitignore",
            "AGENTS.md",
            "CLAUDE.md",
            "Cargo.lock",
            "Cargo.toml",
            "DESIGN.md",
            "LICENSE",
            "Makefile",
            "README.md",
            "VISION.md",
            "rust-toolchain.toml",
        ):
            source_files[name] = REPO / name

        with archive_path.open("wb") as raw:
            with gzip.GzipFile(filename="", mode="wb", fileobj=raw, mtime=FIXED_MTIME) as zipped:
                with tarfile.open(fileobj=zipped, mode="w", format=tarfile.PAX_FORMAT) as tar:
                    add_file(
                        tar,
                        staged_binary,
                        f"{package_root}/bin/{staged_binary.name}",
                        0o755,
                    )
                    add_file(
                        tar,
                        metadata_path,
                        f"{package_root}/release-metadata.json",
                        0o644,
                    )
                    for relative in sorted(source_files):
                        source = source_files[relative]
                        add_file(
                            tar,
                            source,
                            f"{package_root}/source/{relative}",
                            mode_for(source),
                        )

    checksum_path = (args.out / f"{archive_name}.sha256").resolve()
    checksum_path.write_text(f"{sha256(archive_path)}  {archive_name}\n", encoding="utf-8")
    print(archive_path)
    print(checksum_path)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
