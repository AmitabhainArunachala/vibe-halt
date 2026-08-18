# Holdout / Calibration Evaluation Contract v1

## Scope

This document defines the public, synthetic, non-secret dossier schema used
for Wave B/C Reality Bridge R3: versioned holdout manifests and calibration
dossiers. It is separate from `corpus/SCHEMA.md`, which governs the
regression corpus. Holdout and calibration dossiers may not award criterion-3
or criterion-4 credit; they exist only to freeze candidate metadata, state
transitions, and commitment/reveal shape before any target execution. A
`VALID` dossier is therefore not a real-execution admission, even when its
strings describe a plausible command, revision, or forward result.

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
line as `key=value`. Every source string (and the salt) must be a non-empty,
control-free single line; only the derived `reveal` may contain line breaks.
This restriction makes the line framing injective instead of allowing a
newline inside one field to impersonate the next `key=` boundary. Changing any
canonical field invalidates the reveal and therefore the digest, which detects
post-commitment edits such as changing `cohort` from `CALIBRATION` to
`HOLDOUT`.

## Bridge execution rules

- `FORWARD_CONFIRMED` requires `candidate_state == DETECTED` and
  `fixed_control_miss == true`.
- `FORWARD_NULL` requires `candidate_state == MISS`.
- `FORWARD_INVALID` requires `candidate_state == INVALID`.
- A non-null `bridge_execution` requires non-empty `evaluator_image`,
  `toolchain`, `treatment_command`, and `control_command`.
- `fixed_control_miss == true` is rejected for `FORWARD_NULL`,
  `FORWARD_INVALID`, and a null bridge.
- A `CALIBRATION` dossier cannot set a bridge result. Identity fields bearing
  `SYNTHETIC-*` or `NOT-EXECUTED` markers cannot be presented as an executed
  bridge.

These are closed shape rules for the v1 dossier. They do not authenticate the
named executable, prove that either command ran, or construct real-execution
authority. Real admission is a separate Rust-owned state described below.

## Validator

```
vh eval-validate --dossier PATH
```

The validator:

1. Reads the file (which may be a single dossier or a manifest).
2. Requires the exact versioned key set and value types; missing, duplicate,
   unknown, empty, or type-confused fields fail closed.
3. Verifies append-only state transitions.
4. Verifies domain separation, salt length, and commitment/reveal re-computation.
5. Rejects `CALIBRATION` dossiers claiming acceptance credit.
6. Applies the exact candidate/bridge/control matrix and rejects every other
   combination, including a fixed-control miss on null or invalid results.
7. Requires exactly one first-position manifest for multi-dossier documents,
   exact manifest fields, and unique dossier ids.
8. Never executes target code and never constructs real-execution authority.

Exit codes: `0` VALID, `1` INVALID, `2` usage or unreadable file.

## Real-execution admission (`vh-real-execution-receipt-v1`)

Real admission is deliberately not parsed from `vibe-halt.eval-dossier.v1`.
The Tier-1 Rust verifier constructs a private, non-cloneable proof value only
after verifying a canonical run receipt, its complete finding tree, and a
fresh execution of the exact closed-registry workload. A paired plan is frozen
before either arm runs. Its current v1 profile is intentionally tiny:

- treatment workload: `demo-buggy`;
- repaired control workload: `demo`;
- shared condition: caller-selected seed and universe budget, palette `v0`,
  FIFO scheduling, and divergence checking enabled;
- oracle contract: the identical closed `durability` property contract;
- target identity: the observed executing-file SHA-256 plus the role's closed
  workload/configuration identity.

The classifier consumes the plan and both proof values, rechecks the exact
role-specific target, command, condition, oracle, exhausted budget, result
count, and ordered fault-plan vector, and then derives this closed matrix:

| state | treatment | fixed control | `fixed_control_miss` | authority |
|---|---|---|---:|---|
| `CONFIRMED` | `FINDINGS` | `CLEAN` | `true` | `RUST_FRESH_REPLAY` |
| `NULL` | `CLEAN` | `CLEAN` | `false` | `RUST_FRESH_REPLAY` |
| `INVALID` | every other pair or binding failure | any | `false` | `none` |

The canonical paired receipt is length-framed, strictly reparsed, and
content-addressed. All safety-relevant values are derived or checked in Rust;
dossier strings, project metadata, model consensus, and prose cannot construct
the private proof values. `RUST_FRESH_REPLAY` means only that this local engine
freshly reproduced both bounded Tier-1 model executions. It is not an
independent witness, an arbitrary-app or foreign-target result, proof that the
property contract matches human intent, or proof that the observed executable
path bytes are the bytes the operating-system loader mapped into this process.
The last observation-to-loaded-image channel therefore remains explicit even
though the engine-owned workload semantics themselves are Tier-1/D0.

## Manifest schema (`vibe-halt.holdout-manifest.v1`)

A manifest is an NDJSON file starting with one `manifest` record, followed
by one or more `dossier` records. The manifest is versioned separately from
the regression corpus. Calibration manifests are permanently non-credit.
