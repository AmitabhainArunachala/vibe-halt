//! demo-net: a retry-over-partition echo pair on the Phase-1 sim
//! runtime. The RUNTIME owns fault injection (partitions, delays,
//! duplicates, reorders); the workload only declares interaction points.
//!
//! `demo-net` (correct): the client retries each ping on a timeout until
//! ponged or the attempt budget is spent. The plan's blackout budget is
//! bounded below the retry budget BY CONSTRUCTION (see `plan()`), so
//! every round is eventually acknowledged and the campaign is CLEAN.
//!
//! `demo-net-buggy` (seeded bug): fire-and-forget — the client sends
//! each ping exactly once and assumes delivery. Universes where a
//! partition (or an expiring reorder hold) eats the ping or the pong
//! violate the `echo_acked` oracle: the classic vibe-coded
//! the-network-is-reliable fallacy.

use vh_gremlin::{FaultInjection, FaultKind, FaultPalette, FaultPlan, PaletteChooser};
use vh_multiverse::{
    EndState, EndStateOracle, PropertyContract, RunOutcome, StepEvent, UniverseCtx, Workload,
};

const ROUNDS: u64 = 6;
const ROUND_SPACING_NANOS: u64 = 100_000;
const HORIZON_NANOS: u64 = ROUNDS * ROUND_SPACING_NANOS;
const RETRY_TIMEOUT_NANOS: u64 = 60_000;
const MAX_ATTEMPTS: u64 = 8;

const CLIENT: u32 = 0;
const SERVER: u32 = 1;

/// Timer token encoding: round * 16 + attempt (attempt 0 = round start).
fn token(round: u64, attempt: u64) -> u64 {
    round * 16 + attempt
}

pub struct EchoDemo {
    /// true = fire-and-forget (the seeded bug); false = retry until acked.
    pub no_retry: bool,
}

impl EchoDemo {
    /// Deterministic per-universe plan from the workload's own stream.
    /// Blackout budget: at most 4 injections; a partition lasts at most
    /// 100_000 — worst case 400_000 of blackout against a retry budget
    /// of MAX_ATTEMPTS * RETRY_TIMEOUT = 480_000 per round, so the
    /// correct variant always lands inside its budget.
    fn plan(
        rng: &mut vh_core::Xoshiro256pp,
        palette: FaultPalette,
        universe_seed: u64,
    ) -> FaultPlan {
        let count = 2 + rng.next_below(3); // 2..=4
        let chooser = PaletteChooser::new(palette, universe_seed, 4);
        let injections = (0..count)
            .map(|_| {
                let at_nanos = rng.next_below(HORIZON_NANOS);
                let fault = match chooser.choose(rng) {
                    0 => FaultKind::NetworkPartition {
                        duration_nanos: 20_000 + rng.next_below(80_000),
                    },
                    1 => FaultKind::NetworkDelay {
                        delay_nanos: 5_000 + rng.next_below(45_000),
                    },
                    2 => FaultKind::NetworkDuplicate,
                    _ => FaultKind::NetworkReorder,
                };
                FaultInjection { at_nanos, fault }
            })
            .collect();
        FaultPlan::new(injections)
    }
}

fn echo_acked_oracle(end: &EndState) -> Result<(), String> {
    let mut missing = Vec::new();
    for round in 0..ROUNDS {
        if end.get(&format!("acked:{round}")).map(String::as_str) != Some("true") {
            missing.push(round.to_string());
        }
    }
    if missing.is_empty() {
        Ok(())
    } else {
        Err(format!(
            "echo round(s) {} were never acknowledged",
            missing.join(",")
        ))
    }
}

impl Workload for EchoDemo {
    fn name(&self) -> &str {
        if self.no_retry {
            "demo-net-buggy"
        } else {
            "demo-net"
        }
    }

    fn property_contract(&self) -> PropertyContract {
        if self.no_retry {
            PropertyContract::new(&[], &[]).with_oracles(&["echo_acked"])
        } else {
            PropertyContract::new(
                &[],
                &[
                    "message_dropped",
                    "delivery_delayed",
                    "retry_resent",
                    "duplicate_delivered",
                    "reordered_delivery",
                ],
            )
            .with_oracles(&["echo_acked"])
        }
    }

    fn end_state_oracles(&self) -> Vec<EndStateOracle> {
        vec![EndStateOracle {
            name: "echo_acked",
            check: echo_acked_oracle,
        }]
    }

    fn run(&self, ctx: &mut UniverseCtx) -> RunOutcome {
        if !self.no_retry {
            ctx.declare_sometimes("message_dropped");
            ctx.declare_sometimes("delivery_delayed");
            ctx.declare_sometimes("retry_resent");
            ctx.declare_sometimes("duplicate_delivered");
            ctx.declare_sometimes("reordered_delivery");
        }
        let mut gremlin = ctx.stream("gremlin");
        let fault_palette = ctx.fault_palette();
        let universe_seed = ctx.universe_seed();
        let mut rt = ctx.runtime(|| Self::plan(&mut gremlin, fault_palette, universe_seed));

        let mut acked = [false; ROUNDS as usize];
        for round in 0..ROUNDS {
            rt.set_timer(round * ROUND_SPACING_NANOS, token(round, 0));
        }

        while let Some(ev) = rt.step() {
            match ev {
                StepEvent::Timer { token: t } => {
                    let (round, attempt) = (t / 16, t % 16);
                    if acked[round as usize] || attempt >= MAX_ATTEMPTS {
                        continue;
                    }
                    rt.send(CLIENT, SERVER, &format!("ping:{round}"));
                    if attempt > 0 && !self.no_retry {
                        rt.sometimes("retry_resent");
                    }
                    if !self.no_retry {
                        let now = rt.now_nanos();
                        rt.set_timer(now + RETRY_TIMEOUT_NANOS, token(round, attempt + 1));
                    }
                }
                StepEvent::Delivered {
                    to, payload, note, ..
                } => {
                    if !self.no_retry {
                        if note.delayed {
                            rt.sometimes("delivery_delayed");
                        }
                        if note.duplicate {
                            rt.sometimes("duplicate_delivered");
                        }
                        if note.reordered {
                            rt.sometimes("reordered_delivery");
                        }
                    }
                    if to == SERVER {
                        if let Some(round) = payload.strip_prefix("ping:") {
                            let reply = format!("pong:{round}");
                            rt.send(SERVER, CLIENT, &reply);
                        }
                    } else if let Some(round) = payload.strip_prefix("pong:") {
                        let round: u64 = round.parse().expect("deterministic payload");
                        acked[round as usize] = true;
                    }
                }
                StepEvent::Crashed => {
                    unreachable!("demo-net's palette contains no CrashRestart")
                }
            }
        }

        if !self.no_retry && rt.drops() > 0 {
            rt.sometimes("message_dropped");
        }
        for round in 0..ROUNDS {
            rt.declare_end(
                &format!("acked:{round}"),
                &acked[round as usize].to_string(),
            );
        }
        rt.finish();
        RunOutcome::Completed
    }
}
