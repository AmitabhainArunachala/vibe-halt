# VB-005 — fsync-lie-hole

| field | value |
|---|---|
| `id` | VB-005 |
| `class` | fsync-lie-hole |
| `source` | seeded |
| `workload` | `corpus-fsync-lie` |
| `expected_finding` | `oracle:wal_durability` |
| `recall` | found 21/100 at seed 0xD1CE, universe budget 100 |
| `repro` | `vh run --workload corpus-fsync-lie --seed 0xD1CE --universe 5` |
| `gate` | `corpus recall gate: corpus-fsync-lie` in `scripts/gate.sh` |
| `tier` | Tier 1 (engine-owned workload on the sim runtime) |
| `root_seed` | `0xD1CE` |
| `universe_budget` | 100 |
| `oracle_contract` | `required_oracles=[wal_durability] required_always=[] required_sometimes=[]` (CLI-printed; a missing required oracle counts as a contract violation, pinned 0) |
| `generator` | palette `v0`, fault-plan schema `vh-fault-plan-v1` (CLI banner); failing-repro fault-plan digest `198d20e8ffc6717b5a4bb402a752cd1c` |
| `schedule` | `fifo`, no decision tape (`tape=false`) |
| `counts` | always-failures **21**; clean **79**; divergent 0; sometimes unreached 0; invalid completions 0; contract violations 0 |
| `expected_exit` | exit 1, `verdict: FINDINGS` |
| `control` | fault-free/harmless universes must PASS: clean = 79 exactly (>=1) at the pinned budget; pinned clean universe 0: `vh run --workload corpus-fsync-lie --seed 0xD1CE --universe 0` -> no finding, exit 3 (single-replay UNCHECKED policy) |
| `required_facts` | every acknowledged record must be intact in the final durable state; required-progress added by PR #32 (measured count unchanged at 21, receipt in the PR #32 commit message). |

## Mechanism

The CORRECT paranoid WAL client (write -> flush -> fsync -> read-back verify -> ack) under lying hardware: an armed FsyncLie returns Ok while persisting nothing, and the verify read sees the OS cache, so the lie is invisible to any application-level defense. A later crash erases data an Ok fsync claimed durable. This is the class no app logic can close — the rig exists to expose it.

## The law

Acked implies durable-and-intact; when the hardware lies, only crash-testing the durability boundary reveals the hole.

Seeded entry: lower-bound evidence that the rig finds this class
(corpus/SCHEMA.md law 3). Recall pinned 2026-07-21.

## Contract freeze (K1, 2026-07-25)

All counts measured twice consecutively at engine head `ca6b37f`,
byte-identical summaries both passes (corpus/** edits never touch
the engine, so this entry's PR does not move them):

```
$ vh run --workload corpus-fsync-lie --seed 0xD1CE --universes 100
always-failures: 21 universe(s); divergent: 0; sometimes unreached: 0; invalid completions: 0; contract violations: 0; clean: 79
verdict: FINDINGS   (exit 1)
```

Failing-repro receipt (universe 5): trace hash
`548fddeeb89390518ee61648a2b89571` (132 events), fault-plan digest
`198d20e8ffc6717b5a4bb402a752cd1c` (`vh-fault-plan-v1`), exit 1.
Clean-control receipt (universe 0): trace hash
`e04cb392f8c472a64438bc14779b147a` (86 events), fault-plan digest
`31efdf2ab16620744d53a995e9d034f9`, exit 3 (`UNCHECKED`, no finding).

Drift law: a future measurement differing from these pins — in
EITHER direction — is a finding to explain and re-pin with its
semantic cause, never a tolerance band. A count that is not
identical across two consecutive runs at one head makes this
entry's claim UNCHECKED and files an identity defect
(controller kill rule).
