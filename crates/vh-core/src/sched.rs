//! Deterministic event scheduler.
//!
//! Ordering is total and stable: events fire by (virtual time, insertion
//! sequence). The sequence tie-break is what makes same-time events
//! deterministic — never remove it.

use std::cmp::Ordering;
use std::cmp::Reverse;
use std::collections::BinaryHeap;

use crate::clock::VirtualTime;

const FNV128_OFFSET: u128 = 0x6c62_272e_07bb_0142_62b8_2175_6295_c58d;
const FNV128_PRIME: u128 = 0x0000_0000_0100_0000_0000_0000_0000_013B;

fn fnv128_absorb(state: &mut u128, bytes: &[u8]) {
    for &b in bytes {
        *state ^= b as u128;
        *state = state.wrapping_mul(FNV128_PRIME);
    }
}

fn scheduler_candidate_digest(candidates: &[(VirtualTime, u64)]) -> String {
    let mut state = FNV128_OFFSET;
    fnv128_absorb(&mut state, b"vh-scheduler-candidate-set-v1");
    fnv128_absorb(&mut state, &(candidates.len() as u64).to_le_bytes());
    for (at, seq) in candidates {
        fnv128_absorb(&mut state, &at.0.to_le_bytes());
        fnv128_absorb(&mut state, &seq.to_le_bytes());
    }
    format!("{state:032x}")
}

/// One scheduler choice point for the Track-2 decision tape. The current
/// FIFO scheduler's chosen index is always the first same-timestamp
/// candidate, but recording the candidate digest and policy makes the tape
/// replayable by a future PCT/random-tiebreak strategy without changing the
/// frozen v0 trace stream.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SchedulerDecision {
    pub site_id: String,
    pub candidate_set_digest: String,
    pub chosen_index: u64,
    pub policy_id: String,
}

struct Entry<E> {
    at: VirtualTime,
    seq: u64,
    event: E,
}

impl<E> PartialEq for Entry<E> {
    fn eq(&self, other: &Self) -> bool {
        self.at == other.at && self.seq == other.seq
    }
}
impl<E> Eq for Entry<E> {}
impl<E> PartialOrd for Entry<E> {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}
impl<E> Ord for Entry<E> {
    fn cmp(&self, other: &Self) -> Ordering {
        (self.at, self.seq).cmp(&(other.at, other.seq))
    }
}

#[derive(Default)]
pub struct Scheduler<E> {
    heap: BinaryHeap<Reverse<Entry<E>>>,
    seq: u64,
    /// Time of the most recently popped event. Scheduling before this
    /// would make the event loop drive `VirtualClock` backwards (which
    /// panics), so it is rejected here at the source (PR #1 review GAP).
    watermark: VirtualTime,
}

impl<E> Scheduler<E> {
    pub fn new() -> Self {
        Self {
            heap: BinaryHeap::new(),
            seq: 0,
            watermark: VirtualTime::ZERO,
        }
    }

    /// Schedule an event. `at` may equal the watermark (same-time events
    /// fire in insertion order) but must never precede an already-popped
    /// event's time — time only moves forward.
    pub fn schedule(&mut self, at: VirtualTime, event: E) {
        assert!(
            at >= self.watermark,
            "scheduled into the past: {} < watermark {}",
            at.0,
            self.watermark.0
        );
        let seq = self.seq;
        self.seq += 1;
        self.heap.push(Reverse(Entry { at, seq, event }));
    }

    /// Pop the next event in deterministic order, advancing the watermark.
    pub fn pop(&mut self) -> Option<(VirtualTime, E)> {
        self.heap.pop().map(|Reverse(e)| {
            self.watermark = e.at;
            (e.at, e.event)
        })
    }

    /// Pop the next event and record the scheduler choice point to a new,
    /// caller-owned tape stream. This preserves [`Scheduler::pop`] exactly:
    /// FIFO remains the v0 behavior, represented as a constant policy id.
    ///
    /// The candidate set is the same-timestamp frontier that a non-FIFO
    /// schedule strategy may eventually choose among. It is digested from
    /// `(VirtualTime, insertion-seq)` pairs only: event payloads are not
    /// required to implement any hashing trait and replay equivalence is
    /// still checked at the runtime/workload layer.
    pub fn pop_recorded(
        &mut self,
        site_id: &str,
        policy_id: &str,
        mut record: impl FnMut(SchedulerDecision),
    ) -> Option<(VirtualTime, E)> {
        let earliest = self.heap.peek().map(|Reverse(e)| e.at)?;
        let mut candidates: Vec<(VirtualTime, u64)> = self
            .heap
            .iter()
            .filter_map(|Reverse(e)| (e.at == earliest).then_some((e.at, e.seq)))
            .collect();
        candidates.sort_by_key(|(_, seq)| *seq);
        let chosen_seq = candidates[0].1;
        let candidate_set_digest = scheduler_candidate_digest(&candidates);
        let (at, event) = self.pop()?;
        debug_assert_eq!(at, earliest);
        let chosen_index = candidates
            .iter()
            .position(|(_, seq)| *seq == chosen_seq)
            .expect("chosen seq is in candidate set") as u64;
        record(SchedulerDecision {
            site_id: site_id.to_string(),
            candidate_set_digest,
            chosen_index,
            policy_id: policy_id.to_string(),
        });
        Some((at, event))
    }

    /// Pop the same-timestamp candidate CHOSEN by `choose` (an index
    /// into the seq-sorted frontier), recording the decision
    /// (convergence C2). `choose(_) -> 0` is exactly FIFO. The frontier
    /// is drained, the chosen entry removed, and the rest re-pushed
    /// with their ORIGINAL sequence numbers — total order among
    /// unchosen events is preserved, so the only degree of freedom is
    /// the one the policy exercises. An out-of-range choice is clamped
    /// to the last candidate (defensive; policies are deterministic).
    pub fn pop_chosen(
        &mut self,
        site_id: &str,
        policy_id: &str,
        choose: impl FnOnce(&[(VirtualTime, u64)]) -> usize,
        mut record: impl FnMut(SchedulerDecision),
    ) -> Option<(VirtualTime, E)> {
        let earliest = self.heap.peek().map(|Reverse(e)| e.at)?;
        let mut frontier: Vec<Entry<E>> = Vec::new();
        while self.heap.peek().is_some_and(|Reverse(e)| e.at == earliest) {
            frontier.push(self.heap.pop().expect("peeked").0);
        }
        frontier.sort_by_key(|e| e.seq);
        let candidates: Vec<(VirtualTime, u64)> = frontier.iter().map(|e| (e.at, e.seq)).collect();
        let idx = choose(&candidates).min(candidates.len() - 1);
        record(SchedulerDecision {
            site_id: site_id.to_string(),
            candidate_set_digest: scheduler_candidate_digest(&candidates),
            chosen_index: idx as u64,
            policy_id: policy_id.to_string(),
        });
        let chosen = frontier.remove(idx);
        for e in frontier {
            self.heap.push(Reverse(e));
        }
        self.watermark = chosen.at;
        Some((chosen.at, chosen.event))
    }

    pub fn len(&self) -> usize {
        self.heap.len()
    }

    pub fn is_empty(&self) -> bool {
        self.heap.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fires_in_time_order() {
        let mut s = Scheduler::new();
        s.schedule(VirtualTime(30), "c");
        s.schedule(VirtualTime(10), "a");
        s.schedule(VirtualTime(20), "b");
        assert_eq!(s.pop(), Some((VirtualTime(10), "a")));
        assert_eq!(s.pop(), Some((VirtualTime(20), "b")));
        assert_eq!(s.pop(), Some((VirtualTime(30), "c")));
        assert_eq!(s.pop(), None);
    }

    #[test]
    fn same_time_events_fire_in_insertion_order() {
        let mut s = Scheduler::new();
        s.schedule(VirtualTime(10), "first");
        s.schedule(VirtualTime(10), "second");
        s.schedule(VirtualTime(10), "third");
        assert_eq!(s.pop().unwrap().1, "first");
        assert_eq!(s.pop().unwrap().1, "second");
        assert_eq!(s.pop().unwrap().1, "third");
    }

    #[test]
    fn interleaved_same_time_scheduling_stays_in_insertion_order() {
        let mut s = Scheduler::new();
        s.schedule(VirtualTime(10), "a");
        assert_eq!(s.pop().unwrap().1, "a");
        // Equal to the watermark: allowed, fires next.
        s.schedule(VirtualTime(10), "b");
        s.schedule(VirtualTime(10), "c");
        assert_eq!(s.pop().unwrap().1, "b");
        assert_eq!(s.pop().unwrap().1, "c");
    }

    #[test]
    fn recorded_pop_preserves_fifo_order_and_records_choice_points() {
        let mut s = Scheduler::new();
        s.schedule(VirtualTime(10), "first");
        s.schedule(VirtualTime(10), "second");
        s.schedule(VirtualTime(20), "third");
        let mut decisions = Vec::new();

        assert_eq!(
            s.pop_recorded("runtime.step", "fifo-v0", |d| decisions.push(d))
                .unwrap()
                .1,
            "first"
        );
        assert_eq!(
            s.pop_recorded("runtime.step", "fifo-v0", |d| decisions.push(d))
                .unwrap()
                .1,
            "second"
        );
        assert_eq!(
            s.pop_recorded("runtime.step", "fifo-v0", |d| decisions.push(d))
                .unwrap()
                .1,
            "third"
        );
        assert_eq!(decisions.len(), 3);
        assert_eq!(decisions[0].site_id, "runtime.step");
        assert_eq!(decisions[0].policy_id, "fifo-v0");
        assert_eq!(decisions[0].chosen_index, 0);
        assert_ne!(
            decisions[0].candidate_set_digest, decisions[1].candidate_set_digest,
            "same-time frontier changed after the first pop"
        );
    }

    #[test]
    fn decision_candidate_digest_is_stable_for_same_schedule() {
        fn run() -> Vec<SchedulerDecision> {
            let mut s = Scheduler::new();
            s.schedule(VirtualTime(7), "a");
            s.schedule(VirtualTime(7), "b");
            s.schedule(VirtualTime(9), "c");
            let mut out = Vec::new();
            while s
                .pop_recorded("runtime.step", "fifo-v0", |d| out.push(d))
                .is_some()
            {}
            out
        }

        assert_eq!(run(), run());
    }

    #[test]
    #[should_panic(expected = "scheduled into the past")]
    fn refuses_scheduling_before_the_watermark() {
        let mut s = Scheduler::new();
        s.schedule(VirtualTime(10), "a");
        let _ = s.pop(); // watermark now 10
        s.schedule(VirtualTime(5), "past");
    }
}
