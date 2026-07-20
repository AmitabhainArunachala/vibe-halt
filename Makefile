# vibe-halt — one door in: `make onboard` first, every session.

.PHONY: onboard build test gate fmt fmt-check demo ci

onboard:
	python3 scripts/onboard.py

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

demo:
	cargo run -q -p vh-cli -- run --workload demo-buggy --universes 100

ci: gate
