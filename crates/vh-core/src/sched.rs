//! Deterministic event scheduler.
//!
//! Ordering is total and stable: events fire by (virtual time, insertion
//! sequence). The sequence tie-break is what makes same-time events
//! deterministic — never remove it.

use std::cmp::Ordering;
use std::cmp::Reverse;
use std::collections::BinaryHeap;

use crate::clock::VirtualTime;

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
    #[should_panic(expected = "scheduled into the past")]
    fn refuses_scheduling_before_the_watermark() {
        let mut s = Scheduler::new();
        s.schedule(VirtualTime(10), "a");
        let _ = s.pop(); // watermark now 10
        s.schedule(VirtualTime(5), "past");
    }
}
