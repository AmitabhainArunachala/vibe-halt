"""Immutable request data and engine policy for the strict adapter."""

import os
from dataclasses import dataclass
from pathlib import Path
from typing import Optional


@dataclass(frozen=True)
class EnginePolicy:
    """Approved engine path and optional content digest trust root.

    The path must be absolute. If `expected_digest` is supplied the
    adapter copies the executable into the private invocation directory,
    verifies the copy's SHA-256, and runs that exact copy. If it is
    omitted the adapter still copies the binary but cannot claim a
    checked production verdict; `Grade.UNTRUSTED` is reported instead.
    """

    path: str
    expected_digest: Optional[str] = None

    def __post_init__(self):
        if not os.path.isabs(self.path):
            raise ValueError(f"engine path must be absolute: {self.path!r}")
        if self.expected_digest is not None:
            d = self.expected_digest
            if len(d) != 64 or any(c not in "0123456789abcdefABCDEF" for c in d):
                raise ValueError(
                    f"expected_digest must be a 64-character hex SHA-256: {d!r}"
                )


@dataclass(frozen=True)
class RunRequest:
    """Canonical immutable request to the Rust engine.

    `output_root` and `invocation_id` are invocation context, not part of
    the logical request; they are excluded from `request_digest`.
    """

    workload: str
    universes: int
    seed: int = 0xD1CE
    palette: str = "v0"
    schedule: str = "fifo"
    check_divergence: bool = True
    record_tape: bool = False
    shrink: bool = False
    source_commit: Optional[str] = None
    output_root: Optional[str] = None
    invocation_id: Optional[str] = None
    transport: Optional[str] = None
    cassette_path: Optional[str] = None

    def __post_init__(self):
        if self.universes <= 0:
            raise ValueError(f"universes must be positive, got {self.universes}")
        if self.output_root is not None and not os.path.isabs(self.output_root):
            raise ValueError(f"output_root must be absolute: {self.output_root!r}")
        if self.cassette_path is not None and not os.path.isabs(self.cassette_path):
            raise ValueError(f"cassette_path must be absolute: {self.cassette_path!r}")
        if self.transport is not None and self.transport not in {"cooperative"}:
            raise ValueError(f"unknown transport: {self.transport!r}")
