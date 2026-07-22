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

## Track-2 swarm-mask bakeoff result (2026-07-22)

Track 2 added an opt-in `--palette swarm` and ran the R2 seeded A/B
harness over 16 seeds with the pinned 100-universe budget:

```bash
python3 scripts/track2_swarm_bakeoff.py --seeds 16 --max-budget 100
```

Result: **negative**. Swarm passed **0/5** seeded classes against the
R2 threshold (needs `--palette swarm` to reach the pinned recall in
≤25% of v0's universe executions on at least 4/5 classes). The measured
class summaries were:

| workload | class_pass | median swarm/v0 | wins |
|---|---:|---:|---:|
| `corpus-lost-update` | false | 1.032967032967033 | 0/6 |
| `corpus-retry-double-apply` | false | NA | 0/1 |
| `corpus-dirty-read` | false | NA | 0/1 |
| `corpus-crash-toctou` | false | 1.0168539325842696 | 0/12 |
| `corpus-fsync-lie` | false | NA | 0/14 |

Action: keep `--palette v0` as the default and treat all
"guided exploration" claims based on swarm masks as **unproven** until a
new algorithm passes the same harness. Evidence:
`docs/audits/antithesis-dst-2026-07-21/commands/track2-w1-swarm-bakeoff.txt`.

## Track-2 PCT bakeoff result (2026-07-22, convergence C2)

C2 built VB-006 (`corpus-same-timestamp-race`): a bug INVISIBLE to FIFO
v0 by construction (0/10000 universes at seed 0xD1CE) that any
same-timestamp schedule strategy exposes. PCT d=3 finds it at universe
0 (76/100 universes red). The kill-criterion bakeoff then compared PCT
against uniform-with-random-tiebreak over 32 seeds at budget 1000:

```bash
python3 scripts/track2_pct_bakeoff.py --seeds 32 --budget 1000
```

Result: **null**. `median_pct=0 median_uniform=0 pct_wins=0 losses=8
ties=24` — PCT is not faster than uniform tiebreak. The kill criterion
FIRED: PCT is dropped as a guided-exploration bet (kept in-tree opt-in
as the reproducible falsification harness), the decision tape stays as
the replay/causality substrate, and — combined with W1's swarm-palette
0/5 — **the audit's guided-exploration thesis is now falsification-
complete**: on this rig's bug population, schedule/palette DIVERSITY
finds what FIFO cannot, but GUIDANCE has not beaten uniform randomness
anywhere it was measured. Evidence:
`docs/audits/antithesis-dst-2026-07-21/commands/convergence-c2-pct.txt`.
