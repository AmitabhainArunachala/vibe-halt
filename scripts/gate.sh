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

echo "== Tier-2 sandbox gate: clean replay must be CLEAN (exit 0) =="
set +e
out=$(cargo run -q --locked --offline -p vh-cli -- sandbox-demo --mode clean)
code=$?
set -e
verdicts=$(printf '%s\n' "$out" | grep -c '^  verdict: CLEAN')
evidence=$(printf '%s\n' "$out" | grep -c '^  tier=Tier-2 d-grade=D2 divergence-rate=0.000 evidence=run-twice agreement')
if [ "$code" -ne 0 ] || [ "$verdicts" -ne 1 ] || [ "$evidence" -ne 1 ]; then
  echo "GATE FAIL: sandbox clean expected exit 0 + CLEAN + Tier-2/D2 run-twice evidence, got exit $code / $verdicts / $evidence"
  echo "$out"
  exit 1
fi
echo "gate: sandbox clean replay is D2-honest and CLEAN (exit 0)"

echo "== Tier-2 sandbox negative gate: cassette miss must fail closed (exit 1) =="
set +e
out=$(cargo run -q --locked --offline -p vh-cli -- sandbox-demo --mode cassette-miss)
code=$?
set -e
verdicts=$(printf '%s\n' "$out" | grep -c '^  verdict: FINDINGS (fail-closed cassette miss)')
misses=$(printf '%s\n' "$out" | grep -c '^  FAIL cassette: miss digest=')
if [ "$code" -ne 1 ] || [ "$verdicts" -ne 1 ] || [ "$misses" -ne 1 ]; then
  echo "GATE FAIL: sandbox cassette miss expected exit 1 + anchored miss finding, got exit $code / $verdicts / $misses"
  echo "$out"
  exit 1
fi
echo "gate: sandbox cassette miss fails closed (exit 1)"

echo "== Tier-2 sandbox negative gate: subprocess nondeterminism must be divergent (exit 1) =="
set +e
out=$(cargo run -q --locked --offline -p vh-cli -- sandbox-demo --mode nondet)
code=$?
set -e
verdicts=$(printf '%s\n' "$out" | grep -c '^  verdict: FINDINGS')
divergent=$(printf '%s\n' "$out" | grep -c '^  DIVERGENT sandbox subprocess observable records differ')
rate=$(printf '%s\n' "$out" | grep -c '^  tier=Tier-2 d-grade=D2 divergence-rate=1.000 evidence=run-twice agreement')
if [ "$code" -ne 1 ] || [ "$verdicts" -ne 1 ] || [ "$divergent" -ne 1 ] || [ "$rate" -ne 1 ]; then
  echo "GATE FAIL: sandbox nondet expected exit 1 + DIVERGENT + divergence-rate=1.000, got exit $code / $verdicts / $divergent / $rate"
  echo "$out"
  exit 1
fi
echo "gate: sandbox subprocess nondeterminism is caught by run-twice (exit 1)"

echo "== live gate: demo-net — sim-runtime echo pair must be CLEAN (exit 0) =="
set +e
out=$(cargo run -q --locked --offline -p vh-cli -- run --workload demo-net --seed 0xD1CE --universes 200)
code=$?
set -e
verdicts=$(printf '%s\n' "$out" | grep -c '^  verdict: CLEAN')
if [ "$code" -ne 0 ] || [ "$verdicts" -ne 1 ]; then
  echo "GATE FAIL: demo-net expected exit 0 + one anchored CLEAN verdict, got exit $code / $verdicts"
  exit 1
fi
echo "gate: demo-net CLEAN on the sim runtime (exit 0)"

echo "== negative gate: demo-net-buggy — fire-and-forget must be FOUND (exact exit 1) =="
set +e
out=$(cargo run -q --locked --offline -p vh-cli -- run --workload demo-net-buggy --seed 0xD1CE --universes 100)
code=$?
set -e
verdicts=$(printf '%s\n' "$out" | grep -c '^  verdict: FINDINGS')
fails=$(printf '%s\n' "$out" | grep -c '^  FAIL universe .*: oracle:echo_acked')
if [ "$code" -ne 1 ] || [ "$verdicts" -ne 1 ] || [ "$fails" -lt 1 ]; then
  echo "GATE FAIL: demo-net-buggy expected exit 1 + FINDINGS + anchored oracle:echo_acked failure, got exit $code / $verdicts / $fails"
  exit 1
fi
echo "gate: demo-net-buggy correctly caught (exit 1, oracle:echo_acked)"

echo "== live gate: demo-disk — paranoid WAL on SimDisk must be CLEAN (exit 0) =="
set +e
out=$(cargo run -q --locked --offline -p vh-cli -- run --workload demo-disk --seed 0xD1CE --universes 200)
code=$?
set -e
verdicts=$(printf '%s\n' "$out" | grep -c '^  verdict: CLEAN')
if [ "$code" -ne 0 ] || [ "$verdicts" -ne 1 ]; then
  echo "GATE FAIL: demo-disk expected exit 0 + one anchored CLEAN verdict, got exit $code / $verdicts"
  exit 1
fi
echo "gate: demo-disk CLEAN on the sim runtime (exit 0)"

echo "== negative gate: demo-disk-buggy — flush-ack must be FOUND (exact exit 1) =="
set +e
out=$(cargo run -q --locked --offline -p vh-cli -- run --workload demo-disk-buggy --seed 0xD1CE --universes 100)
code=$?
set -e
verdicts=$(printf '%s\n' "$out" | grep -c '^  verdict: FINDINGS')
fails=$(printf '%s\n' "$out" | grep -c '^  FAIL universe .*: oracle:wal_durability')
if [ "$code" -ne 1 ] || [ "$verdicts" -ne 1 ] || [ "$fails" -lt 1 ]; then
  echo "GATE FAIL: demo-disk-buggy expected exit 1 + FINDINGS + anchored oracle:wal_durability failure, got exit $code / $verdicts / $fails"
  exit 1
fi
echo "gate: demo-disk-buggy correctly caught (exit 1, oracle:wal_durability)"

echo "== corpus recall gate: corpus-lost-update must be FOUND (exact exit 1) =="
set +e
out=$(cargo run -q --locked --offline -p vh-cli -- run --workload corpus-lost-update --seed 0xD1CE --universes 100)
code=$?
set -e
verdicts=$(printf '%s\n' "$out" | grep -c '^  verdict: FINDINGS')
fails=$(printf '%s\n' "$out" | grep -c '^  FAIL universe .*: oracle:no_lost_updates')
if [ "$code" -ne 1 ] || [ "$verdicts" -ne 1 ] || [ "$fails" -lt 1 ]; then
  echo "GATE FAIL: corpus-lost-update expected exit 1 + FINDINGS + anchored oracle:no_lost_updates, got exit $code / $verdicts / $fails"
  exit 1
fi
echo "gate: corpus-lost-update recalled (exit 1, oracle:no_lost_updates)"

echo "== corpus recall gate: corpus-retry-double-apply must be FOUND (exact exit 1) =="
set +e
out=$(cargo run -q --locked --offline -p vh-cli -- run --workload corpus-retry-double-apply --seed 0xD1CE --universes 100)
code=$?
set -e
verdicts=$(printf '%s\n' "$out" | grep -c '^  verdict: FINDINGS')
fails=$(printf '%s\n' "$out" | grep -c '^  FAIL universe .*: oracle:exactly_once')
if [ "$code" -ne 1 ] || [ "$verdicts" -ne 1 ] || [ "$fails" -lt 1 ]; then
  echo "GATE FAIL: corpus-retry-double-apply expected exit 1 + FINDINGS + anchored oracle:exactly_once, got exit $code / $verdicts / $fails"
  exit 1
fi
echo "gate: corpus-retry-double-apply recalled (exit 1, oracle:exactly_once)"

echo "== corpus recall gate: corpus-dirty-read must be FOUND (exact exit 1) =="
set +e
out=$(cargo run -q --locked --offline -p vh-cli -- run --workload corpus-dirty-read --seed 0xD1CE --universes 100)
code=$?
set -e
verdicts=$(printf '%s\n' "$out" | grep -c '^  verdict: FINDINGS')
fails=$(printf '%s\n' "$out" | grep -c '^  FAIL universe .*: oracle:published_implies_durable')
if [ "$code" -ne 1 ] || [ "$verdicts" -ne 1 ] || [ "$fails" -lt 1 ]; then
  echo "GATE FAIL: corpus-dirty-read expected exit 1 + FINDINGS + anchored oracle:published_implies_durable, got exit $code / $verdicts / $fails"
  exit 1
fi
echo "gate: corpus-dirty-read recalled (exit 1, oracle:published_implies_durable)"

echo "== corpus recall gate: corpus-crash-toctou must be FOUND (exact exit 1) =="
set +e
out=$(cargo run -q --locked --offline -p vh-cli -- run --workload corpus-crash-toctou --seed 0xD1CE --universes 100)
code=$?
set -e
verdicts=$(printf '%s\n' "$out" | grep -c '^  verdict: FINDINGS')
fails=$(printf '%s\n' "$out" | grep -c '^  FAIL universe .*: oracle:act_epoch_matches_check')
if [ "$code" -ne 1 ] || [ "$verdicts" -ne 1 ] || [ "$fails" -lt 1 ]; then
  echo "GATE FAIL: corpus-crash-toctou expected exit 1 + FINDINGS + anchored oracle:act_epoch_matches_check, got exit $code / $verdicts / $fails"
  exit 1
fi
echo "gate: corpus-crash-toctou recalled (exit 1, oracle:act_epoch_matches_check)"

echo "== corpus recall gate: corpus-fsync-lie must be FOUND (exact exit 1) =="
set +e
out=$(cargo run -q --locked --offline -p vh-cli -- run --workload corpus-fsync-lie --seed 0xD1CE --universes 100)
code=$?
set -e
verdicts=$(printf '%s\n' "$out" | grep -c '^  verdict: FINDINGS')
fails=$(printf '%s\n' "$out" | grep -c '^  FAIL universe .*: oracle:wal_durability')
if [ "$code" -ne 1 ] || [ "$verdicts" -ne 1 ] || [ "$fails" -lt 1 ]; then
  echo "GATE FAIL: corpus-fsync-lie expected exit 1 + FINDINGS + anchored oracle:wal_durability, got exit $code / $verdicts / $fails"
  exit 1
fi
echo "gate: corpus-fsync-lie recalled (exit 1, oracle:wal_durability)"

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

echo "== evidence-store gate: receipts deterministic + bundle replays standalone (C4/R4) =="
bundle_tmp="$(mktemp -d)"
trap 'rm -rf "$bundle_tmp"' EXIT
set +e
cargo run -q --locked --offline --all-features -p vh-cli -- run --workload demo-buggy --seed 0xD1CE --universes 100 --out "$bundle_tmp/A" >/dev/null
a_code=$?
cargo run -q --locked --offline --all-features -p vh-cli -- run --workload demo-buggy --seed 0xD1CE --universes 100 --out "$bundle_tmp/B" >/dev/null
b_code=$?
set -e
if [ "$a_code" -ne 1 ] || [ "$b_code" -ne 1 ]; then
  echo "GATE FAIL: --out must not change the exit contract (got $a_code / $b_code, expected 1)"
  exit 1
fi
if ! diff -r "$bundle_tmp/A" "$bundle_tmp/B" >/dev/null; then
  echo "GATE FAIL: two identical runs wrote different receipt bytes (bundle digests unstable)"
  exit 1
fi
first_bundle=$(find "$bundle_tmp/A/findings" -name finding.ndjson | sort | head -1)
if [ -z "$first_bundle" ]; then
  echo "GATE FAIL: demo-buggy run wrote no finding bundles"
  exit 1
fi
cp "$first_bundle" "$bundle_tmp/standalone.ndjson"
rm -rf "$bundle_tmp/A" "$bundle_tmp/B"
set +e
out=$(cargo run -q --locked --offline --all-features -p vh-cli -- replay-bundle "$bundle_tmp/standalone.ndjson")
code=$?
set -e
reproduced=$(printf '%s\n' "$out" | grep -c '^replay-bundle: REPRODUCED')
if [ "$code" -ne 0 ] || [ "$reproduced" -ne 1 ]; then
  echo "GATE FAIL: standalone bundle replay expected exit 0 + anchored REPRODUCED, got exit $code / $reproduced"
  exit 1
fi
echo "gate: receipts deterministic; standalone bundle replay REPRODUCED (exit 0)"

echo "== negative gate: tampered bundle must fail closed (exact exit 1 + anchored MISMATCH) =="
sed 's/"trace_hash":"[0-9a-f]*"/"trace_hash":"00000000000000000000000000000000"/' \
  "$bundle_tmp/standalone.ndjson" > "$bundle_tmp/tampered.ndjson"
set +e
out=$(cargo run -q --locked --offline --all-features -p vh-cli -- replay-bundle "$bundle_tmp/tampered.ndjson")
code=$?
set -e
mismatch=$(printf '%s\n' "$out" | grep -c '^replay-bundle: MISMATCH')
if [ "$code" -ne 1 ] || [ "$mismatch" -ne 1 ]; then
  echo "GATE FAIL: tampered bundle expected exit 1 + anchored MISMATCH, got exit $code / $mismatch"
  exit 1
fi
echo "gate: tampered bundle correctly fails closed (exit 1, MISMATCH)"
echo "== shrink gate: --shrink minimizes the first failing universe (C5/R1) =="
shrink_start=$SECONDS
set +e
out=$(cargo run -q --locked --offline --all-features -p vh-cli -- run --workload demo-buggy --seed 0xD1CE --universes 100 --shrink)
code=$?
set -e
shrink_secs=$((SECONDS - shrink_start))
minimized=$(printf '%s\n' "$out" | grep -c '^  shrink: MINIMIZED')
binding=$(printf '%s\n' "$out" | grep -c '^  shrink-binding: workload=demo-buggy')
if [ "$code" -ne 1 ] || [ "$minimized" -ne 1 ] || [ "$binding" -ne 1 ]; then
  echo "GATE FAIL: --shrink expected exit 1 + anchored MINIMIZED + binding, got exit $code / $minimized / $binding"
  exit 1
fi
# Kill-criterion telemetry (charter C5: median shrink >60s at 100
# universes fires the kill). Boundary wall clock only — never in kernels.
echo "gate: --shrink MINIMIZED with provenance binding (exit 1, ${shrink_secs}s wall)"
echo "== decision-tape gate: opt-in tape digest agrees across two processes (C1/W2) =="
set +e
tape_a=$(cargo run -q --locked --offline --all-features -p vh-cli -- run --workload demo-net --seed 0xD1CE --universe 3 --record-tape | grep '^  decision tape: ')
code_a=$?
tape_b=$(cargo run -q --locked --offline --all-features -p vh-cli -- run --workload demo-net --seed 0xD1CE --universe 3 --record-tape | grep '^  decision tape: ')
code_b=$?
set -e
if [ -z "$tape_a" ] || [ "$tape_a" != "$tape_b" ]; then
  echo "GATE FAIL: decision-tape digests must exist and agree across processes, got '$tape_a' vs '$tape_b' (exits $code_a/$code_b)"
  exit 1
fi
if ! printf '%s' "$tape_a" | grep -q 'vh-decision-tape-v1'; then
  echo "GATE FAIL: tape line missing its schema: $tape_a"
  exit 1
fi
echo "gate: decision tape agrees across two processes ($tape_a)"

echo "== negative gate: tape leak test — default path and legacy demo carry no tape =="
set +e
default_out=$(cargo run -q --locked --offline --all-features -p vh-cli -- run --workload demo-net --seed 0xD1CE --universe 3)
legacy_out=$(cargo run -q --locked --offline --all-features -p vh-cli -- run --workload demo --seed 0xD1CE --universe 0 --record-tape)
set -e
if printf '%s\n' "$default_out" | grep -q 'decision tape:'; then
  echo "GATE FAIL: default (un-flagged) run leaked a decision tape line"
  exit 1
fi
if printf '%s\n' "$legacy_out" | grep -q 'decision tape:'; then
  echo "GATE FAIL: legacy demo universe grew a decision tape"
  exit 1
fi
if ! printf '%s\n' "$legacy_out" | grep -q 'hash 9ce6199f133f4d3c9dd0da0075e352d2 events 45'; then
  echo "GATE FAIL: frozen demo identity moved under --record-tape"
  exit 1
fi
echo "gate: tape is opt-in and additive (no default/legacy leak; frozen identity intact)"

echo "== gate battery: ALL PASS =="
