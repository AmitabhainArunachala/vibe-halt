//! Virtual time. The only clock any simulated component may consult.

/// Nanoseconds of simulated time since universe start.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Default, Hash)]
pub struct VirtualTime(pub u64);

impl VirtualTime {
    pub const ZERO: VirtualTime = VirtualTime(0);

    pub fn nanos(&self) -> u64 {
        self.0
    }
}

#[derive(Debug, Clone, Default)]
pub struct VirtualClock {
    now: VirtualTime,
}

impl VirtualClock {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn now(&self) -> VirtualTime {
        self.now
    }

    /// Time only moves forward. A backwards advance is a kernel bug, so it
    /// panics rather than corrupting the timeline silently.
    pub fn advance_to(&mut self, t: VirtualTime) {
        assert!(
            t >= self.now,
            "virtual time moved backwards: {} -> {}",
            self.now.0,
            t.0
        );
        self.now = t;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn advances_monotonically() {
        let mut c = VirtualClock::new();
        c.advance_to(VirtualTime(5));
        c.advance_to(VirtualTime(5));
        c.advance_to(VirtualTime(9));
        assert_eq!(c.now(), VirtualTime(9));
    }

    #[test]
    #[should_panic(expected = "backwards")]
    fn refuses_backwards_time() {
        let mut c = VirtualClock::new();
        c.advance_to(VirtualTime(9));
        c.advance_to(VirtualTime(5));
    }
}
