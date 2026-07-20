#!/usr/bin/env bash
# The vibe-halt gate battery — THE single implementation.
#
# Both `make gate` and CI (.github/workflows/ci.yml) execute this script,
# so the two can never drift step-for-step again (hardening-loop-4 GAP 6:
# the Makefile mirror omitted --offline while claiming step parity).
#
# Battery: deny-list (gate 0), governance admission (gate G), fmt, strict
# clippy, workspace tests, frozen-identity doctor, the live 200-universe
# divergence gate, then EXACT negative gates — expected-failure checks
# require the precise finding exit code AND exactly one ANCHORED
# machine-readable verdict line, so a panic (101), a usage error (2), or a
# matching substring smuggled into other output can never be blessed as
# "correctly caught". Seeds are pinned. The quarantined Python client is
# held closed by a negative gate of its own.
#
# Cargo runs --locked --offline (the workspace has zero external
# dependencies by design) and --all-features (no features exist yet; the
# flag is here so feature-gated code can never dodge the gate later).

set -euo pipefail
cd "$(dirname "$0")/.."

echo "== gate 0: determinism deny-list =="
python3 scripts/check_determinism_denylist.py --self-test
python3 scripts/check_determinism_denylist.py

echo "== gate G: governance admission (strict schema, ownership, WIP) =="
python3 scripts/check_governance.py --self-test
python3 scripts/check_governance.py

echo "== format =="
cargo fmt --all --check

echo "== clippy (strict) =="
cargo clippy --workspace --all-targets --all-features --locked --offline -- -D warnings

echo "== tests =="
cargo test --workspace --all-features --locked --offline

echo "== doctor: frozen Tier-1 complete-observable identity =="
cargo run -q --locked --offline -p vh-cli -- doctor

echo "== divergence gate: 200 universes, each run twice, full-observable-compared =="
cargo run -q --locked --offline -p vh-cli -- run --workload demo --seed 0xD1CE --universes 200

echo "== negative gate: seeded bug (exact exit 1 + one anchored FINDINGS verdict) =="
set +e
out=$(cargo run -q --locked --offline -p vh-cli -- run --workload demo-buggy --seed 0xD1CE --universes 50)
code=$?
set -e
verdicts=$(printf '%s\n' "$out" | grep -c '^  verdict: FINDINGS')
if [ "$code" -ne 1 ] || [ "$verdicts" -ne 1 ]; then
  echo "GATE FAIL: demo-buggy expected exit 1 + one anchored FINDINGS verdict, got exit $code / $verdicts"
  exit 1
fi
echo "gate: demo-buggy correctly caught (exit 1, FINDINGS)"

echo "== negative gate: nondeterminism detector (exact exit 1 + anchored DIVERGENT) =="
set +e
out=$(cargo run -q --locked --offline -p vh-cli -- run --workload demo-nondet --seed 0xD1CE --universes 5)
code=$?
set -e
verdicts=$(printf '%s\n' "$out" | grep -c '^  verdict: FINDINGS')
divergent=$(printf '%s\n' "$out" | grep -c '^  DIVERGENT universe')
if [ "$code" -ne 1 ] || [ "$verdicts" -ne 1 ] || [ "$divergent" -lt 1 ]; then
  echo "GATE FAIL: demo-nondet expected exit 1 + one FINDINGS verdict + anchored DIVERGENT, got exit $code / $verdicts / $divergent"
  exit 1
fi
echo "gate: demo-nondet correctly flagged divergent (exit 1, DIVERGENT)"

echo "== negative gate: zero universes rejected (exit 2 + typed diagnostic) =="
set +e
err=$(cargo run -q --locked --offline -p vh-cli -- run --workload demo --universes 0 2>&1 >/dev/null)
code=$?
set -e
# Here-string, not a pipeline: under `set -o pipefail`, BSD grep -q
# exits at first match and the writer can take SIGPIPE (exit 141), false-
# failing the gate on a MATCHING diagnostic (reproduced on macOS
# 2026-07-21: `printf | grep -q` returned 141 with the pattern present).
if [ "$code" -ne 2 ] || ! grep -q -- '--universes must be nonzero — zero work is never certified' <<< "$err"; then
  echo "GATE FAIL: --universes 0 must be rejected with exit 2 + the typed diagnostic, got exit $code"
  exit 1
fi
echo "gate: zero universes correctly rejected (exit 2, typed diagnostic)"

echo "== negative gate: finding-free single replay is UNCHECKED exit 3, never 0 =="
set +e
out=$(cargo run -q --locked --offline -p vh-cli -- run --workload demo-nondet --universe 0)
code=$?
set -e
if [ "$code" -ne 3 ] || ! grep -q '^  replay verdict: UNCHECKED' <<< "$out"; then
  echo "GATE FAIL: single-universe replay must be UNCHECKED exit 3, got exit $code"
  exit 1
fi
echo "gate: single-universe replay correctly UNCHECKED (exit 3)"

echo "== negative gate: Python client quarantine holds (no manufactured success) =="
set +e
PYTHONPATH=clients/python python3 -c "
from vibe_halt import MultiverseRunner
MultiverseRunner('/definitely/not/a/repository', 3, 42)
" 2>/dev/null
runner_code=$?
PYTHONPATH=clients/python python3 -m vibe_halt.cli >/dev/null 2>&1
cli_code=$?
set -e
if [ "$runner_code" -eq 0 ] || [ "$cli_code" -ne 2 ]; then
  echo "GATE FAIL: quarantined Python client executed (runner exit $runner_code, cli exit $cli_code) — it must fail as unimplemented, never simulate"
  exit 1
fi
echo "gate: python client quarantine holds (runner refuses, cli exit 2)"

echo "== gate battery: ALL PASS =="
