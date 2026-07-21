import tempfile
import shutil
from pathlib import Path
from typing import Any


class Sandbox:
    """Simple sandbox for safe diff application and execution (Phase 1 foundation)."""

    def __init__(self, base_dir: Path | None = None):
        self.base_dir = base_dir or Path(tempfile.mkdtemp(prefix="vibe_sandbox_"))

    def apply_diff(self, diff_text: str) -> bool:
        # Placeholder - in full impl use difflib or patch
        print("[SANDBOX] Applying diff (stub)")
        return True

    def run(self, command: str) -> dict:
        # Placeholder for execution
        print(f"[SANDBOX] Running: {command}")
        return {"exit_code": 0, "stdout": "Phase 1 stub output", "stderr": ""}

    def cleanup(self):
        if self.base_dir.exists():
            shutil.rmtree(self.base_dir, ignore_errors=True)
