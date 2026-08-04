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
set Priority, Phase, Evidence, Authority, and Owner lane.

`make project-plan` reports additive project drift without writing.
`make project-sync` creates only missing managed structure and adds current
open issues/PRs; it never deletes, closes, merges, or resolves anything.

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
- set the project item to Done and Evidence to the strongest demonstrated
  level;
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

## Current critical path (2026-08-04)

1. Issue #63 / this workflow implementation.
2. Issue #61 / repair PR #57's unresolved cooperative-transport review debt.
3. Human merge of corrected PR #57.
4. Issue #60 / separately operator-authorized foreign-target confirmation or
   predeclared null.
5. Issues #59 and #64 / representative shrink and named-box throughput.
6. Issue #62 / reconsider Truth Kernel PR #58 only after the Reality Bridge
   terminal state.
