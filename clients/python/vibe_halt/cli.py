"""QUARANTINED console script (PR #1 hardening-loop-4 BLOCKER 3).

The installed `vibe-halt` command used to manufacture success — "All
properties held across N universes" — without executing its target. It
now fails explicitly as unimplemented instead of emitting fabricated
evidence. Use the Rust engine: `cargo run -p vh-cli -- run ...`.
"""

import sys

from .core.runner import QUARANTINE_MESSAGE


def main() -> None:
    print(f"error: {QUARANTINE_MESSAGE}", file=sys.stderr)
    raise SystemExit(2)


if __name__ == "__main__":
    main()
