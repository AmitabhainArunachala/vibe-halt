# vibe-halt — one door in: `make onboard` first, every session.

PYTHON := $(shell \
	for candidate in python3 python3.14 python3.13 python3.12 python3.11; do \
		command -v $$candidate >/dev/null 2>&1 || continue; \
		$$candidate -c 'import sys; raise SystemExit(0 if sys.version_info >= (3, 11) else 1)' >/dev/null 2>&1 || continue; \
		command -v $$candidate; \
		break; \
	done)

ifeq ($(strip $(PYTHON)),)
$(error Python >= 3.11 is required; install it or put python3.11+ on PATH)
endif

export VH_PYTHON := $(PYTHON)

.PHONY: onboard build test gate fmt fmt-check review release-dry-run project-plan project-sync demo ci

onboard:
	$(PYTHON) scripts/onboard.py

build:
	cargo build --workspace

test:
	cargo test --workspace

fmt:
	cargo fmt --all

fmt-check:
	cargo fmt --all --check

# The gate battery lives in scripts/gate.sh — THE single implementation,
# executed identically here and by CI (hardening-loop-4 GAP: the old
# Makefile mirror claimed step parity with ci.yml while omitting
# --offline; centralizing kills that drift class).
gate:
	bash scripts/gate.sh

review:
	bash scripts/self_review.sh

release-dry-run:
	$(PYTHON) scripts/package_release.py --version "$${VERSION:-$$(sed -n 's/^version = "\([^"]*\)"/\1/p' Cargo.toml | head -1)}" --binary "$${BINARY:-target/release/vh}" --target "$${TARGET:-local}" --out "$${OUT:-dist}"

project-plan:
	$(PYTHON) scripts/sync_github_project.py --plan

project-sync:
	$(PYTHON) scripts/sync_github_project.py --apply

demo:
	cargo run -q -p vh-cli -- run --workload demo-buggy --universes 100

ci: gate
