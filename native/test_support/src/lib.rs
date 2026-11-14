//! Shared test helpers for Platine: deterministic fakes of the domain ports.
//!
//! Audio fixture generation joins in P2.

use std::cell::Cell;

use dj_domain::Seconds;
use dj_domain::ports::{Clock, RandomSource};

/// Semantic version of the test-support crate, exposed for diagnostics.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// Deterministic xorshift64 random source: the same seed gives the same shuffles.
#[derive(Debug, Clone)]
pub struct SeededRandom(u64);

impl SeededRandom {
    /// A zero seed is replaced by 1 (xorshift would stay at zero forever).
    pub fn new(seed: u64) -> Self {
        Self(seed.max(1))
    }
}

impl RandomSource for SeededRandom {
    fn next_below(&mut self, upper: usize) -> usize {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        let upper = u64::try_from(upper.max(1)).unwrap_or(u64::MAX);
        usize::try_from(self.0 % upper).unwrap_or(0)
    }
}

/// A clock that only moves when told to.
#[derive(Debug, Default)]
pub struct ManualClock(Cell<f64>);

impl ManualClock {
    pub fn advance(&self, seconds: f64) {
        self.0.set(self.0.get() + seconds.max(0.0));
    }
}

impl Clock for ManualClock {
    fn now(&self) -> Seconds {
        Seconds::saturating(self.0.get())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn seeded_random_is_reproducible_and_in_range() {
        let mut a = SeededRandom::new(5);
        let mut b = SeededRandom::new(5);
        for upper in 1..50 {
            let value = a.next_below(upper);
            assert!(value < upper);
            assert_eq!(value, b.next_below(upper));
        }
        assert_eq!(SeededRandom::new(0).next_below(1), 0);
    }

    #[test]
    fn manual_clock_only_moves_forward() {
        let clock = ManualClock::default();
        assert_eq!(clock.now(), Seconds::ZERO);
        clock.advance(1.5);
        clock.advance(-3.0);
        assert_eq!(clock.now(), Seconds::saturating(1.5));
    }
}
