"""Immutable request data and engine policy for the strict adapter."""

import os
import unicodedata
from dataclasses import dataclass
from typing import Optional


MAX_RECEIPT_UNIVERSES = 10_000
MAX_U64 = (1 << 64) - 1
MAX_REQUEST_STRING_BYTES = 4096


def _contains_control(value: str) -> bool:
    """Reject terminal and invisible-format controls in argv-bound strings."""

    return any(
        unicodedata.category(character) in {"Cc", "Cf", "Zl", "Zp"}
        for character in value
    )


@dataclass(frozen=True)
class EnginePolicy:
    """Approved engine path and optional content digest trust root.

    The path must be absolute. If `expected_digest` is supplied the
    adapter copies the executable into the private invocation directory,
    verifies the copy's SHA-256, and runs that exact copy. If it is
    omitted the adapter still copies the binary but cannot expose a checked
    verdict; `Grade.UNTRUSTED` is reported instead. A matching digest is only
    a local trust root and does not confer production status.
    """

    path: str
    expected_digest: Optional[str] = None

    def __post_init__(self):
        if type(self.path) is not str:
            raise TypeError("engine path must be a string")
        if "\0" in self.path:
            raise ValueError("engine path must not contain NUL bytes")
        if _contains_control(self.path):
            raise ValueError("engine path must not contain control characters")
        try:
            encoded_path = self.path.encode("utf-8", errors="strict")
        except UnicodeEncodeError:
            raise ValueError("engine path must be valid UTF-8") from None
        if len(encoded_path) > MAX_REQUEST_STRING_BYTES:
            raise ValueError("engine path exceeds the 4096-byte request bound")
        if not os.path.isabs(self.path):
            raise ValueError(f"engine path must be absolute: {self.path!r}")
        if self.expected_digest is not None:
            d = self.expected_digest
            if type(d) is not str:
                raise TypeError("expected_digest must be a string or None")
            if len(d) != 64 or any(c not in "0123456789abcdefABCDEF" for c in d):
                raise ValueError(
                    f"expected_digest must be a 64-character hex SHA-256: {d!r}"
                )
            # hashlib emits lowercase. Normalize the accepted spelling once so
            # policy equality can never depend on caller-selected hex case.
            object.__setattr__(self, "expected_digest", d.lower())


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
        string_fields = {
            "workload": self.workload,
            "palette": self.palette,
            "schedule": self.schedule,
        }
        optional_string_fields = {
            "source_commit": self.source_commit,
            "output_root": self.output_root,
            "invocation_id": self.invocation_id,
            "transport": self.transport,
            "cassette_path": self.cassette_path,
        }
        for field, value in string_fields.items():
            if type(value) is not str:
                raise TypeError(f"{field} must be a string")
            if "\0" in value:
                raise ValueError(f"{field} must not contain NUL bytes")
            if _contains_control(value):
                raise ValueError(f"{field} must not contain control characters")
            try:
                encoded = value.encode("utf-8", errors="strict")
            except UnicodeEncodeError:
                raise ValueError(f"{field} must be valid UTF-8") from None
            if len(encoded) > MAX_REQUEST_STRING_BYTES:
                raise ValueError(f"{field} exceeds the 4096-byte request bound")
        for field, value in optional_string_fields.items():
            if value is not None and type(value) is not str:
                raise TypeError(f"{field} must be a string or None")
            if value is not None and "\0" in value:
                raise ValueError(f"{field} must not contain NUL bytes")
            if value is not None and _contains_control(value):
                raise ValueError(f"{field} must not contain control characters")
            if value is not None:
                try:
                    encoded = value.encode("utf-8", errors="strict")
                except UnicodeEncodeError:
                    raise ValueError(f"{field} must be valid UTF-8") from None
                if len(encoded) > MAX_REQUEST_STRING_BYTES:
                    raise ValueError(f"{field} exceeds the 4096-byte request bound")
        if self.invocation_id == "":
            raise ValueError("invocation_id must be non-empty when supplied")
        for field, value in {"universes": self.universes, "seed": self.seed}.items():
            # `bool` is an `int` subclass; accepting it would make `True`
            # silently satisfy the cooperative `universes == 1` contract.
            if type(value) is not int:
                raise TypeError(f"{field} must be an integer (not bool)")
        for field, value in {
            "check_divergence": self.check_divergence,
            "record_tape": self.record_tape,
            "shrink": self.shrink,
        }.items():
            if type(value) is not bool:
                raise TypeError(f"{field} must be a bool")
        if self.universes <= 0:
            raise ValueError(f"universes must be positive, got {self.universes}")
        if self.universes > MAX_RECEIPT_UNIVERSES:
            raise ValueError(
                "universes exceeds the fresh receipt-verification work bound "
                f"({MAX_RECEIPT_UNIVERSES})"
            )
        if self.seed < 0 or self.seed > MAX_U64:
            raise ValueError("seed must be in the Rust u64 domain")
        if self.output_root is not None and not os.path.isabs(self.output_root):
            raise ValueError(f"output_root must be absolute: {self.output_root!r}")
        if self.cassette_path is not None and not os.path.isabs(self.cassette_path):
            raise ValueError(f"cassette_path must be absolute: {self.cassette_path!r}")
        if self.transport is not None and self.transport not in {"cooperative"}:
            raise ValueError(f"unknown transport: {self.transport!r}")
        if self.cassette_path is not None and self.transport != "cooperative":
            raise ValueError(
                "cassette_path requires transport='cooperative': a cassette is "
                "meaningless to any other transport and must be rejected "
                "before runner invocation"
            )
        if self.transport == "cooperative":
            self._enforce_cooperative_contract()
        else:
            if self.palette not in {"v0", "swarm"}:
                raise ValueError("generic receipt runs support palette='v0' or 'swarm'")
            if self.schedule != "fifo":
                raise ValueError("generic receipt runs require schedule='fifo'")
            if self.record_tape:
                raise ValueError("generic receipt runs do not support record_tape=True")
            if self.shrink:
                raise ValueError(
                    "generic receipt runs do not support shrink=True because v2 "
                    "cannot bind that request when no lineage is emitted"
                )

    def _enforce_cooperative_contract(self):
        """Reject any canonical request control outside the cooperative
        contract rather than silently ignoring it."""
        required = {
            "workload": ("cooperative-echo", self.workload),
            "universes": (1, self.universes),
            "seed": (0xD1CE, self.seed),
            "palette": ("v0", self.palette),
            "schedule": ("fifo", self.schedule),
            "check_divergence": (True, self.check_divergence),
            "record_tape": (False, self.record_tape),
            "shrink": (False, self.shrink),
            "source_commit": (None, self.source_commit),
        }
        for field, (expected, actual) in required.items():
            if type(actual) is not type(expected) or actual != expected:
                raise ValueError(
                    f"cooperative transport does not support {field}={actual!r} "
                    f"(requires {expected!r}); refusing to silently ignore a "
                    f"canonical request control"
                )
