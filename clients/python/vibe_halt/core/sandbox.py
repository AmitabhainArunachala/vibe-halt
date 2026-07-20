"""QUARANTINED sandbox stub (PR #1 hardening-loop-4 BLOCKER 3).

The previous stub returned fake exit-0 results from `run` and `apply_diff`
(another manufactured-success path) and `cleanup` recursively deleted a
caller-supplied `base_dir`. Execution methods now fail explicitly as
unimplemented, and cleanup only ever removes a directory this object
created itself — ownership is explicit, never assumed.
"""

import shutil
import tempfile
from pathlib import Path

from .runner import QUARANTINE_MESSAGE


class Sandbox:
    """Quarantined: directory lifecycle only; execution is unimplemented."""

    def __init__(self, base_dir: Path | None = None):
        # Ownership is recorded at construction: cleanup() may only delete
        # what this object itself created.
        self._owns_base_dir = base_dir is None
        self.base_dir = base_dir or Path(tempfile.mkdtemp(prefix="vibe_sandbox_"))

    def apply_diff(self, diff_text: str) -> bool:
        raise NotImplementedError(QUARANTINE_MESSAGE)

    def run(self, command: str) -> dict:
        raise NotImplementedError(QUARANTINE_MESSAGE)

    def cleanup(self) -> None:
        if not self._owns_base_dir:
            raise RuntimeError(
                "refusing to delete caller-supplied base_dir "
                f"{self.base_dir} — the sandbox only cleans up directories "
                "it created itself"
            )
        if self.base_dir.exists():
            shutil.rmtree(self.base_dir, ignore_errors=True)
