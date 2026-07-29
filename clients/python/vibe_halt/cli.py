"""Console entry point for the strict Python client.

The Python package is an adapter library, not a simulator. The Rust `vh`
binary remains the primary command-line tool.
"""

import sys


def main() -> None:
    print(
        "error: the vibe-halt Python CLI is intentionally minimal; "
        "use the Rust `vh` engine or the `vibe_halt.MultiverseRunner` Python API",
        file=sys.stderr,
    )
    raise SystemExit(2)


if __name__ == "__main__":
    main()
