//! Production random source for shuffles.

use std::time::{SystemTime, UNIX_EPOCH};

use dj_domain::ports::RandomSource;

/// xorshift64* generator. Not cryptographic: it only orders playlists.
#[derive(Debug, Clone)]
pub struct XorShiftRandom(u64);

impl XorShiftRandom {
    /// A generator with a fixed seed (reproducible shuffles).
    pub fn seeded(seed: u64) -> Self {
        Self(seed.max(1))
    }

    /// A generator seeded from the clock.
    pub fn from_time() -> Self {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_or(0x9E37_79B9_7F4A_7C15, |d| {
                d.as_secs() ^ u64::from(d.subsec_nanos()).rotate_left(32)
            });
        Self::seeded(nanos)
    }

    fn next_u64(&mut self) -> u64 {
        self.0 ^= self.0 >> 12;
        self.0 ^= self.0 << 25;
        self.0 ^= self.0 >> 27;
        self.0.wrapping_mul(0x2545_F491_4F6C_DD1D)
    }
}

impl RandomSource for XorShiftRandom {
    fn next_below(&mut self, upper: usize) -> usize {
        let upper = u64::try_from(upper.max(1)).unwrap_or(u64::MAX);
        usize::try_from(self.next_u64() % upper).unwrap_or(0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn values_stay_in_range_and_vary() {
        let mut rng = XorShiftRandom::from_time();
        let values: Vec<usize> = (0..100).map(|_| rng.next_below(10)).collect();
        assert!(values.iter().all(|v| *v < 10));
        assert!(values.iter().any(|v| *v != values[0]));
    }

    #[test]
    fn seeded_generators_repeat() {
        let mut a = XorShiftRandom::seeded(42);
        let mut b = XorShiftRandom::seeded(42);
        assert_eq!(a.next_below(1000), b.next_below(1000));
        assert_eq!(XorShiftRandom::seeded(0).next_below(1), 0);
    }
}
