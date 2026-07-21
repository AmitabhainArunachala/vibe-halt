"""QUARANTINED (PR #1 hardening-loop-4 BLOCKER 3).

The previous MultiverseRunner was a second simulator that manufactured
success without executing its target: `target` was never used, universes
were fabricated, dict results passed property checks through getattr
defaults, and the report hardcoded reproducibility_score=1.0 — the
installed console script printed "All properties held" for a nonexistent
repository.

Python must never be a second simulator. This surface stays quarantined
until it is a strict schema/process client for the Rust engine
(`crates/vh-cli`), planned for Phase 4. Until then, construction fails
explicitly instead of fabricating evidence.
"""

from .evidence import EvidenceReport  # noqa: F401  (schema type, data-only)

QUARANTINE_MESSAGE = (
    "vibe-halt Python client is quarantined: MultiverseRunner is NOT "
    "implemented and must never simulate. Use the Rust engine: "
    "`cargo run -p vh-cli -- run ...` (see README.md). The Python package "
    "will return as a strict client of the Rust engine in Phase 4."
)


class MultiverseRunner:
    """Quarantined stub: constructing it raises NotImplementedError."""

    def __init__(self, target: str, universes: int = 1000, base_seed: int = 42):
        raise NotImplementedError(QUARANTINE_MESSAGE)
