# C7 — Linux Deterministic-Process-Supervisor Admission Packet

**Artifact type:** docs-only decision/admission packet (controller §12).
**Authored:** 2026-07-25.
**Controller:** `docs/prompts/VIBE_HALT_POST_AUDIT_TIER2_REACH_LONG_RUNNING_GOAL_2026-07-22.md`.
**Status:** PROPOSAL. Grants nothing. No helper code exists or is written under this file.

> **This packet is not authority.** It contains no unsafe code, no helper
> implementation, no repository-creation action, and no spending. Merging
> it grants no D1 admission, no unsafe-boundary authority, and no
> location decision. Per controller §12, helper code requires BOTH a
> human-merged C7 admission AND a separate explicit location/repository
> authorization — two keys the human operator alone holds. This file
> exists so the operator has a single, complete artifact to decide over.

---

## 0. Why this packet exists

The safe-phase campaign is complete on merged `origin/main` (C0–C6, CDa,
CDb, V1, K1). The cassette-backed subprocess profile is honestly **Tier-2
/ D2**: every channel the safe runner cannot interpose stays `Open` on
the sealed capability receipt, and the C6 leak battery confirms no path
to a false CLEAN (`crates/vh-cli/src/sandbox_campaign.rs`, three anchored
gates in `scripts/gate.sh`).

Audit finding **F.14** asked whether a deterministic single-process
supervisor is "our version of the hypervisor" — the mechanism that would
close time, entropy, PID, ASLR, filesystem, and network channels for one
cooperative target and lift it from D2 to **D1**. First-principles
rejection **REJ-R1** already scopes out whole-machine hypervisor
determinism; the supervisor spike is the deliberately narrow alternative,
and its own falsifier is written into REJ-R1: *"Tier-2 subprocess
determinism proves unachievable at acceptable fidelity in the current
spike."*

This packet is the decision gate between the finished safe phase and any
supervisor work. It is the last item the controller admits before the
campaign returns exactly one operator decision.

## 1. Exact supported target and non-goals

**Supported profile (the ONLY initial target):**
- Linux, x86-64.
- One process, one thread.
- One single-threaded CPython or CLI fixture — the C5/C6 cooperative
  child-visible cassette fixture is the reference target.
- Every unsupported effect is REJECTED synchronously before it produces
  a side effect, never observed-and-ignored.

**Non-goals (never in scope under this packet):**
- No VM, hypervisor, or whole-machine determinism (REJ-R1).
- No arbitrary-binary record/replay.
- No multithreaded, multi-process, or descendant-spawning targets.
- No other kernel, architecture, or libc.
- No promotion of the D2 cassette profile to D1 by relabeling — D1 is
  admitted ONLY after the two-host S7 gate (controller §13) passes on the
  exact supported profile, with every reachable channel closed or
  synchronously rejected.

## 2. Threat / channel model and syscall-effect support profile

The supervisor's job is to make each channel below either **CLOSED**
(interposed/virtualized/denied with controller evidence) or **REJECTED**
(the target is stopped before the effect escapes). A channel that can
only be *observed* is not closed and keeps the profile at D2.

| Channel class | Required disposition for D1 | Mechanism family (to be specified in the spike, not here) |
|---|---|---|
| wall / monotonic / CPU / vDSO time | virtualized or rejected | seccomp-unotify on clock syscalls; vDSO time defeated or rejected |
| entropy: getrandom, /dev/*random, RDRAND | virtualized (seeded) or rejected | syscall interception; RDRAND path rejected |
| real network / DNS | denied (cassette is the only effect surface) | network namespace with no route + syscall denial |
| filesystem content / metadata / order / space / locks / escape | content-addressed RO root + deterministic writable layer | mount/pivot namespace; declared fixtures only |
| PID / TID / hostname / uname / /proc / /sys / /dev | virtualized or rejected | PID namespace; proc masking; uname interception |
| ASLR / address-derived output | disabled for the profile or address observables rejected | personality(ADDR_NO_RANDOMIZE); still fail-closed on leaks |
| signals / timers | modeled or rejected | signalfd/timer interception |
| threads / forks / exec / descendants | rejected (single-process/single-thread only) | seccomp deny of clone/clone3/fork/vfork/further exec |
| inherited FDs, IPC, shared memory, io_uring | rejected | seccomp deny; FD hygiene at launch |
| CPU/FP feature queries, JIT/GC/finalizers | pinned or rejected | feature masking; interpreter pinned single-threaded |
| unsupported / unknown syscalls | **fail closed** — unknown is rejected, never guessed | seccomp default-deny with an explicit allowlist |

**Support profile law:** the allowlist is closed. A syscall or effect not
on it is rejected before its side effect. "Not observed in N traces" is
never closure (controller §13 S7).

## 3. Preferred topology and the excluded-workspace fallback

**Preferred:** a **sibling repository** `vibe-halt-supervisor-linux`,
separate from this repo, with its own ratified unsafe charter, its own
CI, and its own named unsafe/security reviewer. `crates/vh-sandbox` stays
100% safe Rust and speaks to the helper over a minimal length-prefixed
protocol.

**Only acceptable in-repo fallback:** a separately built, explicitly
`exclude`d `tools/vh-supervisor-linux` workspace. It is authorized ONLY
by an explicit operator decision — never merely because this packet
mentions it. It must:
- carry a root-workspace `exclude`/non-membership proof (it is never a
  `crates/**` member and never a deny-list exemption);
- keep every `unsafe` block inside its own named files;
- build, lint, sanitize, and unsafe-audit under its OWN commands — root
  `make gate` never audits helper unsafe and must never claim to.

**In both topologies** the root invariant holds: `unsafe_code = "forbid"`
at the workspace root, kernel crates deny-list pure, zero external
dependencies in the kernel.

## 4. What the operator must name to grant authority

Per controller §12, helper code cannot begin until the operator records
**all** of the following (this packet does not fill any of them in —
every one is a human decision):

1. topology: sibling repo vs excluded in-repo workspace;
2. repository owner/organization and who creates it;
3. visibility (public vs private) — never inferred;
4. license;
5. branch protection;
6. exact repo/path and the allowed unsafe files/functions/ABI wrappers;
7. build hosts;
8. credentials policy (default: none);
9. a named independent unsafe/security reviewer (not the author);
10. a named protocol reviewer;
11. two clean Linux x86-64 evidence hosts for the S7 two-host gate;
12. budget and a no-unapproved-spend rule;
13. cleanup/archival owner;
14. the ten-working-day start/end timestamps and the stop rule.

## 5. Host-capability probes (to run at admission time, not now)

Before S1, the spike must probe and record, on each of the two evidence
hosts: kernel version, `seccomp` + user-notification (`SECCOMP_RET_
USER_NOTIF`) availability, `ptrace` policy (`yama/ptrace_scope`), cgroup
v2, user/PID/mount/network namespace availability, and
`ADDR_NO_RANDOMIZE` support. Any missing capability narrows or kills the
supported profile — it is never worked around.

## 6. Ten-working-day spike shape and stop rule (controller §13)

If admitted, S1–S7 run within ten working days. Partial rungs stay
**Tier-2 / D2 / UNCHECKED** — never relabeled D1. The spike passes only
if every S1–S7 acceptance holds, every reachable channel in §2 is closed
or synchronously rejected, complete effect-tape consumption and the leak
battery pass, the independent unsafe audit is complete, and the two-host
S7 gate produces **100/100 byte-identical** world identity + effect tape
+ complete observation. Otherwise the kill fires:

1. stop supervisor investment for v0.1;
2. publish the exact failed rung/channel and evidence;
3. keep the helper isolated or archive the branch — no experimental
   unsafe merges into the main product;
4. ship the cassette-backed D2 profile honestly (already true today);
5. redirect remaining runway to the recorded fidelity bottleneck or, if
   its realism kill is still open, to criterion 4.

No deadline extension by relabeling partial closure as D1.

## 7. The operator decision (this is the campaign's terminal gate)

Exactly one of these is the human's to choose. The executing agent takes
none of them autonomously.

- **ADMIT** — merge this packet AND record §4's fourteen items AND issue
  the separate location/repository authorization. Only then may helper
  code begin, in the named location, under the named reviewers. D1
  remains unproven until S7 passes on two hosts.
- **DECLINE** — record that C7 location/unsafe authority is declined. No
  helper code is created. The cassette-backed **D2 profile ships as the
  product**, and audit F.14 is reported **"not run by operator
  decision"** — never "killed" or "failed" (controller §16 completion
  outcome 3). This is a complete, honest campaign end.
- **DEFER** — leave the packet unmerged. The safe phase remains complete
  on `main`; nothing changes.

## 8. Claim boundary

Claimed: this packet enumerates the supported profile, the channel model,
the two topologies, the fourteen operator-named prerequisites, the
host-capability probes, and the spike stop rule — completely enough for a
single operator decision. **Not** claimed: that the spike will succeed,
that D1 is achievable, that any topology is chosen, or that any authority
is granted. Every SHA, capability, and host fact herein is
decision-time-fresh only when re-probed at admission. Nothing here is
D1 evidence; D1 lives only in the separately identified two-host S7
artifact, if the operator ever admits the spike.
