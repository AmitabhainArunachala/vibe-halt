# Vibe-Bug Harvesting Playbook v1

How a bug becomes a corpus entry. Owner: track
`vibe-bug-corpus-2026-07`. The corpus's telos: measured recall on REAL
vibe-coded defects, not self-graded demos (build-plan risk 4).

## Sources, in priority order

1. **Real AI-generated PRs** — public repos with disclosed AI
   authorship; bugs found in review, post-merge incident fixes, or
   reverts. Cite the exact commit/PR in `source`.
2. **Published bug taxonomies** — distributed-systems bug studies
   (Jepsen analyses, TigerBeetle/FoundationDB postmortems, OSDI/SOSP
   bug-study corpora); reduce a published bug to its minimal mechanism.
3. **Seeded classes** — written for the corpus when a class has no
   harvested instance yet. Marked `source: seeded`; lower-bound
   evidence only.

## The pipeline (every entry walks all six steps)

1. **Isolate the mechanism.** Reduce the bug to its smallest
   state-machine shape: what invariant breaks, under which fault class,
   in which window.
2. **Express as a workload** on the sim runtime (`crates/vh-cli/src/
   workloads/`): the RUNTIME owns fault injection; the workload declares
   interaction points and end state. The bug must live in the WORKLOAD's
   logic (a missing dedupe key, a stale check, a trusted flush) — never
   in a weakened runtime.
3. **State the law as an oracle** (`EndStateOracle` — one named law; the
   failure detail must name the violated records/rounds).
4. **Measure recall**: `vh run --workload <w> --seed <S> --universes <N>`
   — record found-universe count. If the rig cannot find it, the entry
   does NOT enter the corpus; file the gap as a finding against the
   runtime/scheduler instead (that gap is signal, not noise).
5. **Pin the gate**: add a recall gate to `scripts/gate.sh` with the
   exact exit code and an anchored `oracle:<name>` finding line.
6. **Write the entry** per `corpus/SCHEMA.md`, including the
   one-command repro of one failing universe.

## Anti-gaming rules

- Never tune a workload until the bug is unfindable-in-practice but
  technically present ("recall theater" in reverse).
- Never widen an oracle to catch unrelated noise so a gate looks strong.
- A palette exists to EXPOSE the bug class, not to guarantee failure in
  every universe: crash-free / fault-free universes must PASS (the
  vacuous-failure doctrine from the demo-buggy review GAP).
- Recall numbers in entries are frozen measurements; re-measurement
  after a runtime change that shifts them is a new pin with a changelog
  line in the entry, never a silent edit.
