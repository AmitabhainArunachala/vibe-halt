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
# "correctly caught". Seeds are pinned. Corpus recall gates hold each
# entry's K1-frozen contract as an exact-equality assertion on the full
# summary line plus a fault-free control replay (C2b). The quarantined
# Python client is held closed by a negative gate of its own.
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

# C2b (post-audit controller §6): every corpus recall claim is an EXACT
# executable assertion against the K1-frozen contract in
# corpus/entries/VB-*.md. The pin is the full six-counter summary line,
# matched byte-exact and sourced from the UNTRUNCATED
# "always-failures: N universe(s)" summary — never the FAIL list, which
# truncates at ten and cannot carry a count. Drift in EITHER direction
# (up included: a guaranteed-failure palette is not an improvement) is
# red. Each entry's pinned fault-free control universe must then replay
# with no finding — exit 3 under the single-replay UNCHECKED policy —
# so "the oracle fails everything" can never masquerade as recall.
corpus_recall_gate() { # entry workload oracle fails_pin clean_pin control_universe
  local entry="$1" workload="$2" oracle="$3" fails_pin="$4" clean_pin="$5" control_u="$6"
  echo "== corpus recall gate: $entry ($workload) must recall EXACTLY ${fails_pin}/100 (exit 1) =="
  local summary="  always-failures: ${fails_pin} universe(s); divergent: 0; sometimes unreached: 0; invalid completions: 0; contract violations: 0; clean: ${clean_pin}"
  set +e
  out=$(cargo run -q --locked --offline -p vh-cli -- run --workload "$workload" --seed 0xD1CE --universes 100)
  code=$?
  set -e
  verdicts=$(printf '%s\n' "$out" | grep -c '^  verdict: FINDINGS' || true)
  anchored=$(printf '%s\n' "$out" | grep -c "^  FAIL universe .*: oracle:$oracle" || true)
  exact=$(printf '%s\n' "$out" | grep -cFx -- "$summary" || true)
  if [ "$code" -ne 1 ] || [ "$verdicts" -ne 1 ] || [ "$anchored" -eq 0 ] || [ "$exact" -ne 1 ]; then
    echo "GATE FAIL: $entry ($workload) expected exit 1 + FINDINGS + anchored oracle:$oracle + EXACT pinned summary, got exit $code / verdicts $verdicts / anchored $anchored / exact-summary $exact"
    echo "  pinned:  $summary"
    printf '%s\n' "$out" | grep '^  always-failures: ' | sed 's/^  /  actual:  /' || true
    exit 1
  fi
  set +e
  ctl=$(cargo run -q --locked --offline -p vh-cli -- run --workload "$workload" --seed 0xD1CE --universe "$control_u")
  ctl_code=$?
  set -e
  ctl_unchecked=$(printf '%s\n' "$ctl" | grep -c '^  replay verdict: UNCHECKED' || true)
  ctl_findings=$(printf '%s\n' "$ctl" | grep -c 'ALWAYS-FAIL' || true)
  if [ "$ctl_code" -ne 3 ] || [ "$ctl_unchecked" -ne 1 ] || [ "$ctl_findings" -ne 0 ]; then
    echo "GATE FAIL: $entry fault-free control universe $control_u must replay with no finding (exit 3 UNCHECKED), got exit $ctl_code / unchecked $ctl_unchecked / findings $ctl_findings"
    exit 1
  fi
  echo "gate: $entry recalled at exactly ${fails_pin}/100 (exit 1, oracle:$oracle); control universe $control_u clean (exit 3)"
}

# Pins transcribed from each entry's K1 contract freeze
# (corpus/entries/VB-*.md `counts` and `control` fields, 2026-07-25):
#                  entry   workload                      oracle                    fail clean ctl-universe
corpus_recall_gate VB-001 corpus-lost-update            no_lost_updates            29  71  0
corpus_recall_gate VB-002 corpus-retry-double-apply     exactly_once               76  24  0
corpus_recall_gate VB-003 corpus-dirty-read             published_implies_durable  96   4 23
corpus_recall_gate VB-004 corpus-crash-toctou           act_epoch_matches_check    38  62  1
corpus_recall_gate VB-005 corpus-fsync-lie              wal_durability             21  79  0
corpus_recall_gate VB-007 corpus-stale-redispatch       exactly_once_dispatch      91   9 11
corpus_recall_gate VB-008 corpus-unvalidated-checkpoint checkpoint_recoverable     96   4 17
corpus_recall_gate VB-009 corpus-transient-fatal-abort  session_complete           79  21  8
corpus_recall_gate VB-010 corpus-resume-replay          resume_at_most_once        70  30  0
corpus_recall_gate VB-011 corpus-blind-stream-append    stream_integrity           58  42  0

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
# Same SIGPIPE-under-pipefail class as the `printf | grep -q` fix above
# (reproduced 2026-07-23 in a fresh worktree with 30 finding bundles:
# `sort`'s write can outlive `head -1`'s read end on macOS). Drain the
# rest of the pipe instead of letting the reader close it early.
first_bundle=$(find "$bundle_tmp/A/findings" -name finding.ndjson | sort | { head -n 1; cat >/dev/null; })
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

echo "== C2 gate: VB-006 invisible to FIFO v0 (exact exit 0, CLEAN, 2000 universes) =="
set +e
out=$(cargo run -q --locked --offline -p vh-cli -- run --workload corpus-same-timestamp-race --seed 0xD1CE --universes 2000)
code=$?
set -e
clean=$(printf '%s\n' "$out" | grep -c '^  verdict: CLEAN')
if [ "$code" -ne 0 ] || [ "$clean" -ne 1 ]; then
  echo "GATE FAIL: VB-006 must be INVISIBLE to FIFO v0 (expected exit 0 + CLEAN, got exit $code / $clean) — a FIFO finding here means the frozen tiebreak moved"
  exit 1
fi
echo "gate: VB-006 invisible to FIFO v0 (exit 0, CLEAN over 2000 universes; 10k pinned in the C2 receipt)"

echo "== C2 gate: PCT d=3 finds VB-006 within 100 universes (exact exit 1) =="
set +e
out=$(cargo run -q --locked --offline -p vh-cli -- run --workload corpus-same-timestamp-race --seed 0xD1CE --universes 100 --schedule pct:3 --record-tape)
code=$?
set -e
verdicts=$(printf '%s\n' "$out" | grep -c '^  verdict: FINDINGS')
fails=$(printf '%s\n' "$out" | grep -c '^  FAIL universe .*: oracle:init_before_commit')
# Exact-count pins (Codex audit B.1): the published recall is 76/100
# with the first finding at universe 0 — drift in either direction
# (including UP: the anti-gaming rule) is a gate failure, not a bonus.
pinned=$(printf '%s\n' "$out" | grep -c '^  always-failures: 76 universe(s)')
first=$(printf '%s\n' "$out" | grep -c '^  FAIL universe 0: oracle:init_before_commit')
if [ "$code" -ne 1 ] || [ "$verdicts" -ne 1 ] || [ "$fails" -lt 1 ] || [ "$pinned" -ne 1 ] || [ "$first" -ne 1 ]; then
  echo "GATE FAIL: PCT d=3 expected exit 1 + FINDINGS + exactly 76/100 failing with first at universe 0, got exit $code / verdicts $verdicts / fails $fails / pin76 $pinned / first0 $first"
  exit 1
fi
echo "gate: PCT d=3 finds VB-006 within 100 universes (exit 1, oracle:init_before_commit, pinned 76/100 first=0)"

echo "== C2 gate: PCT replay is byte-identical from (seed, tape digest) =="
set +e
rep_a=$(cargo run -q --locked --offline -p vh-cli -- run --workload corpus-same-timestamp-race --seed 0xD1CE --universe 0 --schedule pct:3 --record-tape | grep -E '^universe 0 |^  decision tape: ')
rep_b=$(cargo run -q --locked --offline -p vh-cli -- run --workload corpus-same-timestamp-race --seed 0xD1CE --universe 0 --schedule pct:3 --record-tape | grep -E '^universe 0 |^  decision tape: ')
set -e
if [ -z "$rep_a" ] || [ "$rep_a" != "$rep_b" ]; then
  echo "GATE FAIL: PCT replay identity broke across two processes"
  exit 1
fi
echo "gate: PCT replay byte-identical across processes (trace hash + tape digest agree)"

echo "== gate battery: ALL PASS =="
