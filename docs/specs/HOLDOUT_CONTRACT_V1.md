# Holdout / Calibration Evaluation Contract v1

## Scope

This document defines the public, synthetic, non-secret dossier schema used
for Wave B/C Reality Bridge R3: versioned holdout manifests and calibration
dossiers. It is separate from `corpus/SCHEMA.md`, which governs the
regression corpus. Holdout and calibration dossiers may not award
criterion-3 or criterion-4 credit; they exist only to freeze candidate
metadata, state transitions, and commitment/reveal shape before any target
execution.

## Dossier schema (`vibe-halt.eval-dossier.v1`)

A dossier is a single flat NDJSON record with the following fields:

| field | type | meaning |
|---|---|---|
| `record` | string | always `"dossier"` |
| `schema` | string | always `"vibe-halt.eval-dossier.v1"` |
| `dossier_id` | string | stable unique id, e.g. `vb008_langgraph_6491` |
| `vb_id` | string | canonical vibe-bug id, e.g. `VB-008` |
| `title` | string | human-readable summary |
| `class` | string | bug class slug |
| `source_repo` | string | repository where the real bug was reported |
| `source_issue` | string | issue/PR number |
| `source_url` | string | public URL |
| `workload` | string | vibe-halt workload that embodies the reduced mechanism |
| `oracle` | string | named end-state oracle |
| `mechanism` | string | reduced mechanism description |
| `pre_fix_revision` | string | synthetic pre-fix revision placeholder |
| `post_fix_revision` | string | synthetic fixed-control revision placeholder |
| `injection_seam` | string | exact injection seam (synthetic for calibration) |
| `evaluator_image` | string | frozen evaluator image (synthetic when UNRUN) |
| `toolchain` | string | frozen toolchain (synthetic when UNRUN) |
| `treatment_command` | string | synthetic treatment command placeholder |
| `control_command` | string | synthetic fixed-control command placeholder |
| `required_facts` | string | facts the oracle must independently derive |
| `status` | string | `DRAFT`, `NOT_ADMISSIBLE`, or `ADMISSIBLE` |
| `cohort` | string | `HOLDOUT` or `CALIBRATION` (immutable after admittance) |
| `candidate_state` | string | `UNRUN`, `AUTHORITY_BLOCKED`, `DETECTED`, `MISS`, or `INVALID` |
| `candidate_state_log` | string | semicolon-separated state history |
| `bridge_execution` | string or null | `FORWARD_CONFIRMED`, `FORWARD_NULL`, `FORWARD_INVALID`, or `null` |
| `fixed_control_miss` | bool | true only when bridge is `FORWARD_CONFIRMED` |
| `acceptance_credit` | bool | must be false for `CALIBRATION` dossiers |
| `commitment_domain` | string | must be `"vh-eval-dossier-commitment-v1"` |
| `commitment_salt` | string | public synthetic salt, length >= 32 |
| `commitment_digest` | string | SHA-256 of `domain\0salt\0reveal` |
| `reveal` | string | canonical dossier string bound by the commitment |

## State layers

1. Pre-cohort: `DRAFT`, `NOT_ADMISSIBLE`, `ADMISSIBLE`.
2. Every `ADMISSIBLE` dossier has one immutable cohort: `HOLDOUT` or
   `CALIBRATION`.
3. Frozen candidate state: `UNRUN`, `AUTHORITY_BLOCKED`, `DETECTED`, `MISS`,
   `INVALID`.
4. Bridge execution: `FORWARD_CONFIRMED`, `FORWARD_NULL`, `FORWARD_INVALID`.

## Append-only candidate-state transitions

Allowed transitions (other than `X -> X`):

- `UNRUN` -> `AUTHORITY_BLOCKED`, `DETECTED`, `MISS`, `INVALID`
- `AUTHORITY_BLOCKED` -> `DETECTED`, `MISS`, `INVALID`
- `DETECTED` -> `INVALID`
- `MISS` -> `INVALID`

A `DETECTED -> MISS` transition is a red-matrix violation and must be rejected.

## Commitment / reveal

The commitment binds the canonical dossier bytes with a domain-separated,
public, synthetic salt. It is a shape check only; it does not use or store
any real selection secret.

```
commitment_digest = SHA256(domain || '\0' || salt || '\0' || reveal)
```

The `reveal` is the deterministic canonical rendering of all
non-commitment, non-envelope fields in the fixed order listed above, one per
line as `key=value`. Changing any canonical field invalidates the reveal and
therefore the digest, which detects post-commitment edits such as changing
`cohort` from `CALIBRATION` to `HOLDOUT`.

## Bridge execution rules

- `FORWARD_CONFIRMED` requires `candidate_state == DETECTED` and
  `fixed_control_miss == true`.
- `FORWARD_NULL` requires `candidate_state == MISS`.
- `FORWARD_INVALID` requires `candidate_state == INVALID`.
- A non-null `bridge_execution` requires non-empty `evaluator_image`,
  `toolchain`, `treatment_command`, and `control_command`.

## Validator

```
vh eval-validate --dossier PATH
```

The validator:

1. Reads the file (which may be a single dossier or a manifest).
2. Checks every `dossier` record for required fields and allowed enum values.
3. Verifies append-only state transitions.
4. Verifies domain separation, salt length, and commitment/reveal re-computation.
5. Rejects `CALIBRATION` dossiers claiming acceptance credit.
6. Rejects malformed `FORWARD_CONFIRMED` claims without a checked detection
   and fixed-control miss.
7. Never executes any target code.

Exit codes: `0` VALID, `1` INVALID, `2` usage or unreadable file.

## Manifest schema (`vibe-halt.holdout-manifest.v1`)

A manifest is an NDJSON file starting with one `manifest` record, followed
by one or more `dossier` records. The manifest is versioned separately from
the regression corpus. Calibration manifests are permanently non-credit.
