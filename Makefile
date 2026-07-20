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

# The gate battery: deny-list, lints, tests, frozen-identity doctor, the
# live 200-universe divergence gate, then EXACT negative gates — the
# expected-failure checks require the precise finding exit code (1) AND the
# machine-readable verdict text, so a panic (101) or usage error (2) can
# never be blessed as "correctly caught". Seeds are pinned.
gate:
	python3 scripts/check_determinism_denylist.py --self-test
	python3 scripts/check_determinism_denylist.py
	cargo fmt --all --check
	cargo clippy --workspace --all-targets -- -D warnings
	cargo test --workspace
	cargo run -q -p vh-cli -- doctor
	cargo run -q -p vh-cli -- run --workload demo --seed 0xD1CE --universes 200
	@out=$$(cargo run -q -p vh-cli -- run --workload demo-buggy --seed 0xD1CE --universes 50); code=$$?; \
	if [ "$$code" -ne 1 ] || ! printf '%s' "$$out" | grep -q "verdict: FINDINGS"; then \
		echo "GATE FAIL: demo-buggy expected exit 1 + 'verdict: FINDINGS', got exit $$code"; exit 1; \
	fi; echo "gate: demo-buggy correctly caught (exit 1, FINDINGS)"
	@out=$$(cargo run -q -p vh-cli -- run --workload demo-nondet --seed 0xD1CE --universes 5); code=$$?; \
	if [ "$$code" -ne 1 ] || ! printf '%s' "$$out" | grep -q "DIVERGENT universe"; then \
		echo "GATE FAIL: demo-nondet expected exit 1 + DIVERGENT, got exit $$code"; exit 1; \
	fi; echo "gate: demo-nondet correctly flagged divergent (exit 1, DIVERGENT)"
	@cargo run -q -p vh-cli -- run --workload demo --universes 0 >/dev/null 2>&1; code=$$?; \
	if [ "$$code" -ne 2 ] ; then \
		echo "GATE FAIL: --universes 0 must be rejected with exit 2, got exit $$code"; exit 1; \
	fi; echo "gate: zero universes correctly rejected (exit 2)"

demo:
	cargo run -q -p vh-cli -- run --workload demo-buggy --universes 100

ci: fmt-check gate
