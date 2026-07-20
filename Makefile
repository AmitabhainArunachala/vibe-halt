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
# expected-failure checks require the precise finding exit code (1) AND
# exactly one ANCHORED machine-readable verdict line, so a panic (101), a
# usage error (2), or a matching substring smuggled into other output can
# never be blessed as "correctly caught". Seeds are pinned. Mirrors
# .github/workflows/ci.yml step for step.
gate:
	python3 scripts/check_determinism_denylist.py --self-test
	python3 scripts/check_determinism_denylist.py
	cargo fmt --all --check
	cargo clippy --workspace --all-targets --locked -- -D warnings
	cargo test --workspace --locked
	cargo run -q --locked -p vh-cli -- doctor
	cargo run -q --locked -p vh-cli -- run --workload demo --seed 0xD1CE --universes 200
	@out=$$(cargo run -q --locked -p vh-cli -- run --workload demo-buggy --seed 0xD1CE --universes 50); code=$$?; \
	verdicts=$$(printf '%s\n' "$$out" | grep -c '^  verdict: FINDINGS'); \
	if [ "$$code" -ne 1 ] || [ "$$verdicts" -ne 1 ]; then \
		echo "GATE FAIL: demo-buggy expected exit 1 + one anchored FINDINGS verdict, got exit $$code / $$verdicts"; exit 1; \
	fi; echo "gate: demo-buggy correctly caught (exit 1, FINDINGS)"
	@out=$$(cargo run -q --locked -p vh-cli -- run --workload demo-nondet --seed 0xD1CE --universes 5); code=$$?; \
	verdicts=$$(printf '%s\n' "$$out" | grep -c '^  verdict: FINDINGS'); \
	divergent=$$(printf '%s\n' "$$out" | grep -c '^  DIVERGENT universe'); \
	if [ "$$code" -ne 1 ] || [ "$$verdicts" -ne 1 ] || [ "$$divergent" -lt 1 ]; then \
		echo "GATE FAIL: demo-nondet expected exit 1 + one FINDINGS verdict + anchored DIVERGENT, got exit $$code / $$verdicts / $$divergent"; exit 1; \
	fi; echo "gate: demo-nondet correctly flagged divergent (exit 1, DIVERGENT)"
	@err=$$(cargo run -q --locked -p vh-cli -- run --workload demo --universes 0 2>&1 >/dev/null); code=$$?; \
	if [ "$$code" -ne 2 ] || ! printf '%s' "$$err" | grep -q -- '--universes must be nonzero — zero work is never certified'; then \
		echo "GATE FAIL: --universes 0 must be rejected with exit 2 + the typed diagnostic, got exit $$code"; exit 1; \
	fi; echo "gate: zero universes correctly rejected (exit 2, typed diagnostic)"
	@out=$$(cargo run -q --locked -p vh-cli -- run --workload demo-nondet --universe 0); code=$$?; \
	if [ "$$code" -ne 3 ] || ! printf '%s\n' "$$out" | grep -q '^  replay verdict: UNCHECKED'; then \
		echo "GATE FAIL: single-universe replay must be UNCHECKED exit 3, got exit $$code"; exit 1; \
	fi; echo "gate: single-universe replay correctly UNCHECKED (exit 3)"

demo:
	cargo run -q -p vh-cli -- run --workload demo-buggy --universes 100

ci: fmt-check gate
