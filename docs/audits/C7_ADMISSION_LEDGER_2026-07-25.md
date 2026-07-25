# C7 — Supervisor Admission Ledger (operator decisions in progress)

**Artifact type:** docs-only admission ledger (controller §12). Grants
nothing. No unsafe code, no helper implementation, no repository-creation
action, no spending is authorized by this file.
**Tracks:** the fourteen operator-named items required by
`docs/audits/C7_SUPERVISOR_ADMISSION_PACKET_2026-07-25.md` §4, and the
terminal ADMIT/DECLINE/DEFER decision in that packet's §7.
**Status:** `DEFER` — the packet remains a PROPOSAL. **2 of 14** required
items are recorded below; **12 remain unnamed**, and the separate
location/repository authorization has **not** been issued. Per §7, helper
code cannot begin until *all fourteen* are recorded *and* that separate
authorization is issued. Until then this stays DEFER and the shipped
product is the cassette-backed **D2** profile.

> This ledger exists so operator decisions survive across ephemeral
> sessions. It changes no authority. Recording a decision here is not the
> same as issuing it — the location/repository authorization in §7 is a
> distinct act the operator alone performs.

---

## 1. Fourteen-item ledger (packet §4)

| # | Item (packet §4) | Status | Recorded value |
|---|---|---|---|
| 1 | topology: sibling repo vs excluded in-repo workspace | **RECORDED** | **Sibling repository** `vibe-halt-supervisor-linux` (packet §3 "Preferred"). `crates/vh-sandbox` stays 100% safe Rust and speaks to the helper over the minimal length-prefixed protocol. |
| 2 | repository owner/organization and who creates it | PENDING | — (operator must name owner/org and the human who creates it; the executing agent does not create repositories) |
| 3 | visibility (public vs private) — never inferred | PENDING | — (packet forbids inferring; recommend **private** for unsafe-code repo, operator confirms) |
| 4 | license | PENDING | — |
| 5 | branch protection | PENDING | — (recommend: require the named reviewer's approval before merge) |
| 6 | exact repo/path + allowed unsafe files/functions/ABI wrappers | PENDING | — (design proposal in §3 below, operator ratifies) |
| 7 | build hosts | PENDING | — |
| 8 | credentials policy (default: none) | PENDING | — (default **none** unless operator states otherwise) |
| 9 | named independent unsafe/security reviewer (not the author) | **RECORDED** | **Codex** — see the residual-risk note in §2 below. |
| 10 | named protocol reviewer | PENDING | — |
| 11 | two clean Linux x86-64 evidence hosts for the S7 two-host gate | PENDING | — |
| 12 | budget + no-unapproved-spend rule | PENDING | — |
| 13 | cleanup/archival owner | PENDING | — |
| 14 | ten-working-day start/end timestamps + stop rule | PENDING | stop rule fixed by packet §6; start/end timestamps operator-set |

**Tally: 2 recorded, 12 pending. Location/repository authorization: NOT issued.**

## 2. Residual-risk note on item 9 (reviewer = Codex)

Codex is an AI reviewer. For code that runs **as root** and drives
`ptrace` / `seccomp` / namespace syscalls, AI-only review of the final
privileged syscall surface is a genuine residual risk: a defect there can
compromise the build host, not merely produce a wrong test verdict. This
ledger records Codex as the design/logic reviewer as the operator
directed, and additionally recommends — not as a blocker, but on the
record per the repo's citation-or-silence rule — that a **human** sign off
on the actual privileged syscall surface (the seccomp allowlist and every
`unsafe` block) before it is ever executed as root. The operator may
accept this residual risk explicitly or name a human co-reviewer under
item 9.

## 3. Design proposal for item 6 (safe-side seam — NOT unsafe code)

Non-binding proposal for the operator to ratify when naming item 6. This
describes only the **safe side** the packet §3 already fixes in-repo
(`crates/vh-sandbox` speaking a minimal length-prefixed protocol); it
proposes no unsafe implementation and writes none.

- Unsafe confined to named files in the sibling repo, e.g.
  `src/supervisor/seccomp.rs`, `src/supervisor/ptrace.rs`,
  `src/supervisor/namespace.rs` — every `unsafe` block inside those, each
  wrapping exactly one syscall/ABI boundary, audited under the sibling
  repo's own commands (packet §3: root `make gate` never audits helper
  unsafe).
- Wire protocol: the existing safe length-prefixed framing family
  (`vh-cassette-transport-v1` shape, `crates/vh-sandbox/src/cassette_v2.rs`)
  extended with a **channel-disposition attestation** record — for each
  §2 channel class, the helper declares `CLOSED` (with the interposition
  evidence) or `REJECTED` (with the stopped syscall), and the safe side
  refuses any record that leaves a reachable channel merely `Open`. This
  attestation record is the D1 grade's machine-checkable definition.
- The attestation codec is pure and belongs in the safe kernel; it is an
  **S1 deliverable to design after admission** (packet §5/§6 defer spike
  work to admission time), not written under this ledger.

## 4. Informal feasibility probe (NOT an S7 evidence host)

Per packet §5 the host-capability probes run **at admission time on the
two named evidence hosts**, not now. The following was gathered informally
on the current session container only, to inform the operator's decision;
it is **not** S7 evidence and must be re-probed on the item-11 hosts:

| Capability | This session container | Note |
|---|---|---|
| kernel | Linux 6.18.5 x86-64 | matches supported profile (packet §1) |
| user namespaces | present (`max_user_namespaces` = 64265) | needed for network/mount/PID isolation |
| ASLR control | controllable (`randomize_va_space` = 2) | `personality(ADDR_NO_RANDOMIZE)` path available |
| cgroup v2 | **absent** | narrows resource-bounding strategy — a real gap to resolve on the evidence hosts |
| seccomp user-notify | not confirmed | packet §5 requires confirming `SECCOMP_RET_USER_NOTIF` on each evidence host |
| ptrace policy | not readable here | must read `yama/ptrace_scope` on evidence hosts |
| privilege | running as root (`id -u` = 0) | feasible but a hazard: root magnifies any unsafe defect (see §2) |
| CPython | 3.11.15 | matches the C5/C6 reference fixture target |

Feasibility is plausible on this container, but **not proven**, and this
container is not one of the two evidence hosts the S7 gate requires.

## 5. What is still required before ADMIT (packet §7)

1. Record the remaining **12** items above.
2. Issue the **separate location/repository authorization** (§7) — a
   distinct operator act, not implied by naming the items.

Only when both hold does helper code begin, in the named location, under
the named reviewers. Even then, **D1 remains unproven** until the S7
two-host gate produces 100/100 byte-identical world identity + effect tape
+ complete observation (packet §6). Partial closure is never relabeled D1.
