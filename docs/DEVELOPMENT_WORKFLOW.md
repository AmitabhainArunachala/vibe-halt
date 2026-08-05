# Vibe Halt Development Workflow

This is the repository-specific implementation of the GitHub/agent loop. The
canonical queue is the linked
[Vibe Halt project](https://github.com/users/AmitabhainArunachala/projects/1);
Markdown plans explain contracts but do not replace issues.

## 1. Admit work

Every change begins with one issue containing:

- objective and non-goals;
- acceptance criteria and exact proof command;
- affected determinism tier and claim boundary;
- authority required (`Agent executable`, `Human merge`, `Operator approval`,
  or `External confirmation`);
- rollback or kill condition.

Use the bug form for a demonstrated defect and the work-packet form for
features, audits, measurements, or governance. Add the item to the project and
set Priority, Phase, Type, Outcome, Claim boundary, Evidence, Authority, Gate
state, Risk, Horizon, Blocked by, Target date, Owner lane, and (once evidence
exists) Acceptance record. `Blocked by` is plain text under the grammar in
"Project metadata semantics" below: use stable issue or PR references rather
than an implied dependency.

`make project-plan` reports project drift without writing and previews the
exact command set, including any view filter/layout value an update would
replace. `make project-sync` creates missing managed structure, converges
registered managed views while preserving extra visible columns, and adds
current open issues/PRs; it never deletes, closes, merges, or resolves
anything. Both FAIL CLOSED: a managed field with the wrong type, a managed
single-select missing required options, a possibly-truncated listing, an
unregistered or duplicated managed-view name, or an unconverged apply is a
non-zero exit, never a warning-and-continue. Managed views are owned by ID in
`scripts/project_sync_views.lock.json`; a name match alone never authorizes an
update — adopt a view explicitly with `--adopt-view` and commit the lock file.
Apply re-reads each view immediately before writing and treats a concurrent
human edit as a typed conflict; it succeeds only after a fresh read-back plan
is empty. The managed views are All Work, Delivery Board, Critical Path,
Evidence & Authority, 12-Week Roadmap, Human Gates, and Parked / Research.
Table and board views expose the managed evidence/authority columns; the
roadmap carries only its layout and declared filter because GitHub rejects
visible-field configuration for roadmap views. Existing single-select options
are never replaced: missing managed options fail the run for a human to
resolve.

### Project metadata semantics (normative)

- **Project fields and views are untrusted coordination metadata.** No field
  value, view, checkbox, model consensus, signature text, or green sync result
  can promote a claim's evidence grade, modality, or authority.
  `scripts/check_project_acceptance.py` enforces the conformance rules below
  (`--self-test` offline in the gate; `make project-accept` against the live
  board).
- **Authority** names the highest authority the item's claim ultimately
  requires; it is stable across the item's life. **Gate state** tracks the
  next currently unsatisfied gate: `Dependency blocked` while an open `#NN`
  dependency remains (items mixing open dependencies with external-actor
  conditions may honestly carry the pending flavor of their authority class
  instead); a pending flavor must match the Authority class
  (Human pending ↔ Human merge, Operator pending ↔ Operator approval,
  External pending ↔ External confirmation); `Satisfied` means every gate for
  the declared claim boundary is met.
- **Status is lifecycle only.** `Done` means the row froze, never that a claim
  was accepted. Acceptance is DERIVED, never stored: an item counts as
  **Accepted** only when its content is merged/closed-complete, Gate state is
  `Satisfied`, its Evidence grade is admissible for the declared Claim
  boundary, and a non-empty **Acceptance record** resolves the justifying
  artifact. A predeclared null closes as **NullResult** (record prefixed
  `null:`); a parked or rejected closure is **ClosedUnaccepted** and keeps its
  unsatisfied gate visible.
- **Acceptance record** holds the URL/SHA that justifies the row's Evidence
  grade or `Satisfied` gate (merge SHA, run URL, published null). `CI green`
  and `Satisfied` require one. Typed prefixes: `null:` a predeclared null,
  `excluded:` a disposition note on an unaccepted closure, `operator:` /
  `external:` authority-class records — `External proof` and the
  Operator/External authority classes require a record the single-account
  estate did not author (those prefixes or a non-repo artifact). Records are
  checked for presence, shape, and location only — a single-account estate
  cannot mechanically authenticate WHO wrote a record; authorship attestation
  stays out-of-band.
- **Blocked by grammar**: `#NN` refs are live dependencies; annotate satisfied
  ones `#NN (satisfied YYYY-MM-DD)`; `decides:` marks a disposition-about
  reference; `constraint:` marks a scope constraint. Only unannotated `#NN`
  refs outside those segments count as blocking edges.
- **New items are born untriaged**: they carry no custom field values, so they
  are invisible to positively-filtered views and appear untriaged in Human
  Gates (`-authority:"Agent executable"` includes empty values). Triage every
  admitted item promptly; the Human Gates view doubles as the catcher.
- **Standalone issue/PR auto-add** is a browser-only GitHub workflow with no
  proven API path; the explicit fallback is `make project-sync` after admitting
  work.

## 2. Start one bounded branch

```bash
make onboard
git fetch origin
git switch -c agent/<issue>-<slug> origin/main
```

Record the issue in the branch or first commit. One issue may have several
sequential PRs when ownership or rollback differs; one PR should not mix
unrelated issues.

Before editing, compare paths with
`docs/governance/ACTIVE_TRACK.yaml`. Shared surfaces have one named integration
writer.

## 3. Implement as a verification loop

Define the loop before coding:

1. **Run** the actual CLI, test, replay, or benchmark.
2. **Use** the changed behavior through its public entry point.
3. **Prove** it with an exit code, machine record, replay, artifact digest, or
   explicit unavailable reason.
4. **Falsify** the happy path with malformed, stale, empty, boundary, and
   concurrency cases.
5. **Record** only durable claims; runtime receipts remain outside git.

Do not infer `CLEAN` from stdout, manufacture evidence in Python, or promote
agreement beyond the recorded tier.

## 4. Self-review before publication

```bash
make review
```

This checks committed and worktree changes against `origin/main`, rejects new
deferred-work markers in favor of GitHub issues, requires verification
language in the latest code-changing commit, and runs the full gate.

Then inspect:

```bash
git diff --check origin/main...HEAD
git diff --stat origin/main...HEAD
git status --short
```

## 5. Open a draft PR

Push the branch and open a **draft** PR using the repository template. The PR
must link its issue, name exact base/head SHAs, list changed surfaces, include
local proof, state the claim boundary, and describe rollback.

Agents may implement, test, commit, push, open/update draft PRs, fetch review
comments, reply, and rerun CI. Agents do not approve or merge their own work.

## 6. CI and review loop

Required evidence:

- `ci / gate`;
- the verifier matrix and aggregate cross-OS comparison;
- all actionable review threads answered;
- no unresolved P1/P2 thread;
- branch rebased or updated on current `main`;
- fresh `make review` at the proposed head.

For every review finding:

1. reproduce or inspect the exact behavior;
2. add a red-first regression when the finding is valid;
3. fix narrowly;
4. run targeted checks, then `make gate`;
5. push;
6. reply with the fixing commit and command;
7. resolve only after the remote diff contains the fix.

Technical disagreement is allowed, but it must cite code or a runnable
falsifier. Thread state is queried with GitHub GraphQL `reviewThreads`; flat
PR comments are not a complete review state.

## 7. Human merge and cleanup

Only a human marks ready, approves, and merges. Green automation is evidence,
not authority.

After merge:

- confirm the `main` push run is green;
- close the issue only when its acceptance criteria are met;
- set the project item to Done, Evidence to the strongest demonstrated level,
  and Acceptance record to the justifying merge SHA / run URL (Status alone
  never implies acceptance — see "Project metadata semantics");
- delete the merged branch only after verifying no open PR or worktree
  depends on it;
- update a canonical plan only when a claim materially changed.

## 8. Releases

The release workflow is tag-triggered. A tag `vX.Y.Z` must exactly equal the
workspace package version in `Cargo.toml`.

```bash
make review
git tag -s vX.Y.Z
git push origin vX.Y.Z
```

GitHub Actions builds `vh` for Linux, macOS, and Windows, creates deterministic
archives containing the executable plus tagged source needed for independent
offline replay, emits Rust-computed executable metadata plus SHA-256 checksums,
and creates a GitHub release. A release proves only those builds and checks at
the tagged SHA; it is not a safety certificate.

## Current critical path (2026-08-05)

1. Issue #63 / this workflow implementation.
2. Issue #60 / separately operator-authorized foreign-target confirmation or
   predeclared null.
3. Issues #59 and #64 / representative shrink and named-box throughput.
4. Issue #62 / reconsider Truth Kernel PR #58 only after the Reality Bridge
   terminal state.
