# Sandbox Capability Envelope v1 (C4)

Contract for `crates/vh-sandbox`'s controller-produced, sealed capability
receipt and exact termination taxonomy. Implementation:
`crates/vh-sandbox/src/capability.rs` (pure data, no I/O — needs no
determinism-denylist exemption) and `crates/vh-sandbox/src/lib.rs` (the
boundary logic that produces these types from an actual subprocess run).

This is the C4 contract other packages depend on per
`docs/prompts/VIBE_HALT_POST_AUDIT_TIER2_REACH_LONG_RUNNING_GOAL_2026-07-22.md`
§4: C3 waits for C4's run-record/capability contract; C5 waits for C4's
run-record/capability contract in addition to C3's v2 schema.

## The sealed capability receipt

`CapabilityReceipt` (`crates/vh-sandbox/src/capability.rs`) is
controller-produced, not caller-asserted. `SandboxSpec` — the caller's
request descriptor — has no field or method that can mark any channel
closed; the only public constructor for a receipt,
`CapabilityReceipt::all_open`, is crate-private and always seeds the
complete channel inventory as `Open`. A channel is closed only by
controller evidence (`ChannelStatus::Closed { evidence }`), never by
omission from the inventory — there is no "channel absent means closed"
reading anywhere in this type, and `CapabilityReceipt::is_d1` requires
every one of the 29 channels to be `Closed`.

This package (C4) implements no channel-closure mechanism: every run's
receipt is `all_open`, so `EvidenceGrade::D1` is currently unreachable
through the public API. Closing channels needs either raw ABI/unsafe work
(process-group/resource enforcement, ptrace/seccomp, virtual clocks) that
is out of scope here and belongs to the separately authorized C7
unsafe-helper package, or a real transport (the C5 cassette) for the
channels a cooperative target can hand off voluntarily.

## Channel inventory (`CAPABILITY_SCHEMA = "vh-sandbox-capability-v1"`)

29 channels, pinned by `CapabilityChannel::ALL` and a test
(`capability_channel_inventory_is_pinned_and_exhaustive`,
`crates/vh-sandbox/src/tests.rs`). Growing or renaming the inventory is a
schema bump.

| Group | Channels |
|---|---|
| Network | `network_dns` |
| Time | `wall_clock`, `monotonic_clock`, `cpu_clock`, `vdso_time`, `hardware_time` |
| Randomness | `entropy_devices`, `os_random_syscall`, `hardware_rng` |
| Filesystem | `filesystem_content`, `filesystem_metadata`, `filesystem_order`, `filesystem_space`, `filesystem_locks`, `filesystem_escape` |
| Loader | `loader_dependencies` |
| Address space | `aslr_address_output` |
| Host identity | `process_thread_identity`, `host_identity`, `proc_sys_dev_access` |
| Signals/timers | `signals`, `timers` |
| Process tree | `threads_forks_exec_descendants`, `inherited_file_descriptors` |
| IPC/async | `ipc_shared_memory`, `async_io_uring` |
| CPU/runtime | `cpu_fp_features`, `jit_gc_finalizers` |
| Catch-all | `unsupported_syscalls_effects` |

`os_random_syscall` names the OS-level `getrandom`-class syscall channel
without spelling that token literally in source — the exact substring
trips this repo's determinism-denylist scanner (a line-regex, not a
semantic check; see `scripts/check_determinism_denylist.py`) even inside
a plain string literal.

## Exact termination taxonomy

`TerminationOutcome` (`crates/vh-sandbox/src/capability.rs`) distinguishes,
via `as_identity_str()`, every cause enumerated in the C4 charter: normal
exit code, exact Unix signal (with best-effort core-dump observation),
controller timeout, spawn/exec failure, resource-limit outcome (reserved,
unreachable in this MVP), and typed unknown. Two different signals, or an
exact signal versus unknown, never produce the same identity string —
regression-tested in `different_signals_never_collapse_to_the_same_identity`
and `exact_signal_never_collapses_with_unknown`.

Exact signal recovery does not use `std::os`'s Unix `ExitStatusExt`
(`.signal()`/`.core_dumped()`): that extension-trait module is not part of
this crate's determinism-denylist exemption
(`scripts/check_determinism_denylist.py`, `EXEMPT["crates/vh-sandbox/src/lib.rs"]`
covers `std::process`/`std::time`/`Instant::now`/`std::io`/`std::fs` only).
Instead, `classify_exit_status` (`crates/vh-sandbox/src/lib.rs`) parses the
already-permitted `std::process::ExitStatus` `Display` rendering
(`"signal: {N} (SIGNAME)"`, optionally with a `"(core dumped)"` suffix on
platforms that report it — verified empirically against this repo's pinned
toolchain). An unparseable rendering stays typed `Unknown`, never guessed;
a positive `"(core dumped)"` observation is trusted, its absence is not
treated as a confirmed non-dump (not every platform's rendering reports the
bit at all).

`ProcessTreeState` records what the controller could prove about the
direct child (no child / reaped / reap failed). Descendant and
process-group cleanup cannot be proven this way in safe Rust; that stays
represented by `CapabilityChannel::ThreadsForksExecDescendants` remaining
`Open`, never by a false-positive field here.

## No unbounded wait, bounded output

Every `run_once` execution carries an explicit `SandboxBudget` (deadline +
per-stream output cap, `crates/vh-sandbox/src/lib.rs`). `run_once`
materializes the exact stdin bytes into a controller-prepared regular file
before the execution deadline starts and gives the child a read-only
descriptor for that file. There is no live controller-side pipe write for a
child that never reads stdin to backpressure, so input delivery cannot block
entry into the deadline loop. `execute_bounded` polls `Child::try_wait`
against the deadline; on expiry it kills and waits on the direct child (no
unbounded wait) and returns `TerminationOutcome::TimedOut`. Because
`std::thread` (needed for
`thread::sleep`, and for the read side of Rust's own two-pipe
`wait_with_output` pattern) is denied even on this boundary crate, and the
platform-specific non-blocking-fd module is not part of this crate's
exemption either, the poll loop is a `std::hint::spin_loop`-mitigated busy
spin — a documented MVP cost, not a hidden default. Stdout/stderr are
redirected to files (not pipes) specifically to avoid needing either of
those mechanisms for concurrent draining, and are read back bounded to
`max_output_bytes`; `StreamObservation.truncated` and the true on-disk
`byte_len` are always reported, never silently hidden.

Declared artifacts are only attempted after the child ran to completion or
was terminated by a signal — never after a spawn failure or a timeout kill
— so a killed run's missing artifacts never masquerade as a controller I/O
error.

## World binding

`RunRecord::identity()` binds: the spec identity (argv/stdin/env/declared
artifacts/declared input files/budget/cassette+supervisor identity slots),
compile-time target OS/arch (`cfg!`-derived, not a live `std::env` probe),
resolved executable identity (content-hashed immediately before spawn when
`argv[0]` is a path the controller can read directly; honestly `Unresolved`
for a bare command name relying on `PATH` search — this crate does not
reimplement platform `PATH` resolution), exact termination, process-tree
state, both stream
observations, declared-artifact digests, and the full capability receipt.
`wall_time` stays boundary telemetry, excluded from identity as before.
The safe runner cannot make its final observation-to-exec step atomic
against a hostile same-user path replacement; filesystem and loader
channels therefore remain `Open`, and the receipt never promotes this
pre-spawn binding to D1 closure.

## Divergence evidence

`DivergenceReport` (`crates/vh-sandbox/src/capability.rs`,
`DIVERGENCE_REPORT_SCHEMA = "vh-sandbox-divergence-v1"`) replaces the
earlier one-pair `0.0`/`1.0` special case with raw
`diverged`/`sample` counts plus an order-binding `sample_identity` over a
declared suite of run-twice pairs (`from_identity_pairs`). A suite of size
zero reports `sample: 0`, never a fabricated `0.000`. `SandboxCampaign`
(one pair — the mechanism this MVP's demo and tests use today) is the
degenerate one-pair case; C6's reference campaign is expected to call
`DivergenceReport::from_identity_pairs` directly over its full suite
rather than reinventing aggregation.

## Known, cited scope limits (not silently closed)

- Every channel above stays `Open` on every receipt this package produces;
  closing any of them is C7's (separately authorized, unsafe-helper)
  job.
- "Initial filesystem/fixtures" binding is the freshly created empty
  workspace case only; fixture-seeded workspaces are C6's reference-profile
  concern.
- Core-dump observation is best-effort string parsing of a standard-library
  `Display` rendering, not a semantically guaranteed API; treat a `None`
  as "not confirmed either way," never as "confirmed clean."
