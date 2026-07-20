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

# The gate battery: deny-list, tests, then the live divergence gate —
# 200 universes of the reference workload, every one run twice and
# hash-compared. `demo-buggy` must FAIL (the rig proves it still catches
# the seeded bug) and `demo-nondet` must FAIL (the detector still detects).
gate:
	python3 scripts/check_determinism_denylist.py --self-test
	python3 scripts/check_determinism_denylist.py
	cargo fmt --all --check
	cargo clippy --workspace --all-targets -- -D warnings
	cargo test --workspace
	cargo run -q -p vh-cli -- run --workload demo --seed 0xD1CE --universes 200
	@if cargo run -q -p vh-cli -- run --workload demo-buggy --universes 50 > /dev/null; then \
		echo "GATE FAIL: demo-buggy passed — the rig no longer finds the seeded bug"; exit 1; \
	else echo "gate: demo-buggy correctly caught"; fi
	@if cargo run -q -p vh-cli -- run --workload demo-nondet --universes 5 > /dev/null; then \
		echo "GATE FAIL: demo-nondet passed — divergence detector is blind"; exit 1; \
	else echo "gate: demo-nondet correctly flagged divergent"; fi

demo:
	cargo run -q -p vh-cli -- run --workload demo-buggy --universes 100

ci: fmt-check gate
