# Vibe Halt Product Lock v1

**Status:** RATIFIED PRODUCT DIRECTION; effective only after human merge.

**Authority:** operator decision recorded in
[issue #81](https://github.com/AmitabhainArunachala/vibe-halt/issues/81#issuecomment-5230157890).
This document fixes product purpose and scope. It does not prove capability,
authorize foreign-code execution, permit spending, or weaken any evidence,
determinism, security, or human-merge rule.

## Product thesis

Vibe Halt is an AI-native adversarial verification environment for vibe coders
and serious builders who cannot safely trust the quality or efficacy of
AI-generated or AI-modified software.

A submitted target may be a whole repository, application, feature, agentic
system, trading system, or high-level fintech workflow. Every source artifact
must be bound to an exact observed revision.

The broad front door does not create a broad proof claim. Vibe Halt reports
exactly what it tested, what it found, what it could not test, and which
channels remained uncontrolled. Unsupported, incomplete, stale, divergent, or
unverifiable coverage is `UNKNOWN`, never evidence of safety.

The long-term north star is an Antithesis-adjacent software-verification
environment with increasing ownership of target execution and, if evidence,
resources, and economics justify it, a Vibe Halt deterministic hypervisor.
Hypervisor development is not part of v0.1 or the present execution grant.

## Golden path

1. The builder submits a target manifest containing exact source identity,
   target type, permitted commands and data boundaries, critical workflows,
   required invariants, and fixed time and monetary budgets.
2. Vibe Halt inventories the target and publishes a target map and coverage
   plan before interpreting absence of findings.
3. It may generate disposable harnesses, mocks, cassettes, or adversarial
   attacks outside the production source tree.
4. Preflight and runtime engines execute within their declared evidence grades.
5. Vibe Halt emits findings, a coverage ledger, independently replayable
   evidence where supported, and one target-level decision: `HALT`, `PROCEED`,
   or `UNKNOWN`.
6. The builder uses that receipt in a merge or release decision.

Vibe Halt v0.1 does not author or modify production code. It may materialize an
externally supplied revision or diff in a disposable workspace solely to test
that exact input. Any future patch-generation capability must remain separate
from verification authority and may not certify its own patch.

## Included v0.1 defect surfaces

A bounded `vh preflight` may compose:

- build, package, dependency, API, and cross-file contract facts;
- malformed or partial diff and configuration facts;
- static security and error-handling checks as modules within a target run;
- deterministic state-machine and semantic fault testing at retry,
  acknowledgement, persistence, checkpoint, cancellation, time, provider,
  tool, and integrity boundaries; and
- target-declared end-to-end workflow and domain invariants, including
  agentic, trading, and fintech workflows.

Admission of a surface means Vibe Halt may test it when a named engine and
oracle exist. It does not imply every submitted target is covered on every
surface.

## Decision contract

The existing engine outcomes `CLEAN`, `FINDINGS`, `UNCHECKED`, and `ERROR`
remain evidence-layer states. The target-level decision is a separate,
versioned, fail-closed policy projection:

- `HALT`: an admitted blocking finding reproduced, or a mandatory preflight
  fact failed.
- `PROCEED`: every mandatory declared check completed within its stated
  boundary, its evidence verified, and no blocking finding remains.
- `UNKNOWN`: any mandatory area is unsupported, incomplete, stale, untrusted,
  divergent, non-replayable, errored, or otherwise unresolved.

`UNKNOWN` outranks `PROCEED`. `PROCEED` is a bounded merge/release
recommendation for the recorded target, revision, checks, properties, fault
model, budget, and evidence grades—not a general safety certificate.

Findings are emitted independently of the target-level decision.

## Product benchmark

Vibe Halt succeeds by finding more important real faults within a fixed budget,
not by producing more prose.

Its primary efficiency metric is:

```text
confirmed_yield =
  sum(predeclared severity weight of unique independently confirmed faults)
  / (USD spend * wall-clock minutes)
```

A product tournament uses the same pinned target and equal spend and wall-clock
ceilings for Vibe Halt and at least three decorrelated frontier-AI reviewers.
Baseline models, prompts, context, tools, independence rules, severity weights,
confirmation procedure, and root-cause deduplication are frozen before
execution. Unconfirmed findings receive no yield credit.

Supporting measures are time to first confirmed fault, independent replay
success, invalid-claim rate, faults missed by every baseline reviewer, and
whether a finding changes a real merge or release decision.

## Dharma Swarm

Dharma Swarm is simultaneously:

- the first proving ground;
- the first major product consumer;
- a source of real agentic, orchestration, persistence, and financial workflow
  surfaces; and
- a future consumer of Vibe Halt evidence at multiple internal gates.

Vibe Halt remains independently usable; Dharma is the first tenant, not the
product boundary.

## Six-week victory

One exact Dharma target enters through the whole-target path and completes the
target-map, coverage-plan, attack, evidence, independent-replay, and
`HALT`/`PROCEED`/`UNKNOWN` flow.

Within the equal-budget tournament, Vibe Halt produces at least one important,
independently confirmed, reproducible fault missed by every baseline AI
reviewer.

This milestone supplements rather than silently replaces the seven ratified
technical success criteria in
`docs/plans/VIBE_HALT_BUILD_PLAN_2026-07-20.md:27-35`.

## Explicit non-goals for v0.1

- no claim that a whole-target submission receives exhaustive coverage;
- no generic prose code-review bot or standalone scanner product;
- no general safety, security, or financial-correctness certificate;
- no deterministic claim for arbitrary repositories or binaries;
- no destructive testing of live production systems;
- no production-code edits, automatic merge, deployment, or self-approval; and
- no full deterministic hypervisor in the current build phase.

## Kill and reorientation conditions

If the six-week path cannot execute one Dharma target end to end, stop adding
analysis breadth and repair the single intake-to-receipt path.

If three preregistered, mechanism-eligible, equal-budget target tournaments
produce no advantage in confirmed severity-weighted yield, pause broad product
expansion. Diagnose the fault classes won by baseline AIs, pivot the engine
portfolio while preserving the evidence kernel, and require a new human
product-scope decision before further hypervisor or category expansion.

## Immediate build order

1. Bind operation/feature negotiation and observed target revision.
2. Repair known evaluator and D2 trust-boundary defects.
3. Implement the target manifest, target map, coverage plan, coverage ledger,
   and target-level decision receipt.
4. Connect the existing deterministic engine as the first deep runtime engine.
5. Build the preregistered decorrelated-AI baseline harness.
6. Run the first Dharma tournament and publish the result, including an honest
   null if no qualifying fault is found.
