#!/usr/bin/env bash
# Fresh-context review gate before publishing a branch.

set -euo pipefail
cd "$(dirname "$0")/.."

base_ref="${BASE_REF:-origin/main}"
if ! git rev-parse --verify "$base_ref^{commit}" >/dev/null 2>&1; then
  echo "review: base ref is unavailable: $base_ref" >&2
  exit 2
fi

merge_base="$(git merge-base HEAD "$base_ref")"
echo "== review context =="
printf 'base-ref: %s\nmerge-base: %s\nhead: %s\n' \
  "$base_ref" "$merge_base" "$(git rev-parse HEAD)"
git status --short --branch

echo "== changed files =="
git diff --name-status "$merge_base"
git diff --check "$merge_base"
untracked="$(git ls-files --others --exclude-standard)"
if [[ -n "$untracked" ]]; then
  printf '%s\n' "$untracked" | sed 's/^/?	/'
  while IFS= read -r path; do
    awk '
      /[[:blank:]]+$/ {
        printf "%s:%d: trailing whitespace\n", FILENAME, FNR
        bad = 1
      }
      END { exit bad }
    ' "$path"
  done <<< "$untracked"
fi

echo "== authority and evidence scan =="
changed="$(
  {
    git diff --name-only "$merge_base"
    printf '%s\n' "$untracked"
  } | sed '/^$/d' | LC_ALL=C sort -u
)"
if printf '%s\n' "$changed" | grep -Eq '^(crates/|clients/|scripts/|Cargo|Makefile|\.github/workflows/)'; then
  # Read the message into a variable first: with pipefail, `grep -q` exiting
  # at the first match SIGPIPEs `git log` on long messages (observed exit 141
  # on 1d74b7e) and the negation misreads the match as a missing-evidence
  # failure.
  head_message="$(git log -1 --format=%B)"
  if [[ "$merge_base" != "$(git rev-parse HEAD)" ]] \
    && ! grep -Eiq '(test|gate|verif|evidence|receipt)' <<<"$head_message"; then
    echo "review: latest commit message does not name test/gate/verification evidence" >&2
    exit 1
  fi
fi

marker_pattern='(^|[^[:alnum:]_])(TO''DO|FIX''ME|X''XX)([^[:alnum:]_]|$)'
if {
  git diff --unified=0 "$merge_base" | grep '^+' | grep -Ev '^\+\+\+' || true
  while IFS= read -r path; do
    [[ -n "$path" ]] && sed 's/^/+/' "$path"
  done <<< "$untracked"
} | grep -En "$marker_pattern"; then
  echo "review: new deferred-work marker found; move admitted work to a GitHub issue" >&2
  exit 1
fi

echo "== full repository gate =="
make gate
echo "review: PASS"
