//! Play-order strategies (Strategy pattern): sequential or Fisher-Yates shuffle.

use crate::ports::RandomSource;
use crate::track::TrackId;

/// Arranges a list of tracks in place.
pub trait ShuffleStrategy {
    /// Reorders `tracks`. When possible, the first track is not one of
    /// `avoid_first` (used so that a new shuffle cycle does not start with a
    /// track that was just played).
    fn arrange(&self, tracks: &mut [TrackId], avoid_first: &[TrackId], rng: &mut dyn RandomSource);
}

/// Keeps the given (natural) order.
#[derive(Debug, Clone, Copy, Default)]
pub struct Sequential;

impl ShuffleStrategy for Sequential {
    fn arrange(&self, _: &mut [TrackId], _: &[TrackId], _: &mut dyn RandomSource) {}
}

/// Uniform Fisher-Yates shuffle.
#[derive(Debug, Clone, Copy, Default)]
pub struct FisherYates;

impl ShuffleStrategy for FisherYates {
    fn arrange(&self, tracks: &mut [TrackId], avoid_first: &[TrackId], rng: &mut dyn RandomSource) {
        for i in (1..tracks.len()).rev() {
            let j = rng.next_below(i + 1).min(i);
            tracks.swap(i, j);
        }
        avoid_as_first(tracks, avoid_first);
    }
}

/// Moves the first track that is not in `avoid` to the front.
/// Does nothing if every track is to be avoided.
fn avoid_as_first(tracks: &mut [TrackId], avoid: &[TrackId]) {
    let Some(first) = tracks.first() else { return };
    if !avoid.contains(first) {
        return;
    }
    if let Some(index) = tracks.iter().position(|track| !avoid.contains(track)) {
        tracks.swap(0, index);
    }
}

/// Play order selected by the user.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum PlayOrder {
    #[default]
    Sequential,
    Shuffle,
}

impl PlayOrder {
    /// Strategy implementing this order.
    pub fn strategy(self) -> &'static dyn ShuffleStrategy {
        match self {
            Self::Sequential => &Sequential,
            Self::Shuffle => &FisherYates,
        }
    }
}

#[cfg(test)]
pub(crate) mod test_rng {
    use crate::ports::RandomSource;

    /// Deterministic xorshift generator for unit tests.
    #[derive(Debug, Clone)]
    pub(crate) struct XorShift(pub u64);

    impl RandomSource for XorShift {
        fn next_below(&mut self, upper: usize) -> usize {
            self.0 ^= self.0 << 13;
            self.0 ^= self.0 >> 7;
            self.0 ^= self.0 << 17;
            usize::try_from(self.0 % upper as u64).unwrap_or(0)
        }
    }

    /// Always answers `upper - 1` (the identity permutation for Fisher-Yates).
    #[derive(Debug, Clone, Copy)]
    pub(crate) struct Highest;

    impl RandomSource for Highest {
        fn next_below(&mut self, upper: usize) -> usize {
            upper - 1
        }
    }
}

#[cfg(test)]
mod tests {
    use super::test_rng::{Highest, XorShift};
    use super::*;

    fn ids(values: &[u64]) -> Vec<TrackId> {
        values.iter().copied().map(TrackId).collect()
    }

    #[test]
    fn sequential_keeps_the_order() {
        let mut tracks = ids(&[3, 1, 2]);
        Sequential.arrange(&mut tracks, &ids(&[3]), &mut XorShift(1));
        assert_eq!(tracks, ids(&[3, 1, 2]));
    }

    #[test]
    fn fisher_yates_is_deterministic_for_a_given_seed() {
        let mut first = ids(&[1, 2, 3, 4, 5, 6]);
        let mut second = first.clone();
        FisherYates.arrange(&mut first, &[], &mut XorShift(42));
        FisherYates.arrange(&mut second, &[], &mut XorShift(42));
        assert_eq!(first, second);
        assert_ne!(first, ids(&[1, 2, 3, 4, 5, 6]));
    }

    #[test]
    fn fisher_yates_avoids_recent_tracks_as_first() {
        let mut tracks = ids(&[1, 2, 3, 4]);
        // `Highest` yields the identity permutation, so 1 would come first.
        FisherYates.arrange(&mut tracks, &ids(&[1, 2]), &mut Highest);
        assert_eq!(tracks[0], TrackId(3));
        let mut sorted = tracks.clone();
        sorted.sort();
        assert_eq!(sorted, ids(&[1, 2, 3, 4]));
    }

    #[test]
    fn avoidance_is_best_effort_when_every_track_is_recent() {
        let mut tracks = ids(&[1, 2]);
        FisherYates.arrange(&mut tracks, &ids(&[1, 2]), &mut Highest);
        assert_eq!(tracks, ids(&[1, 2]));
    }

    #[test]
    fn empty_and_single_lists_are_fine() {
        let mut empty: Vec<TrackId> = Vec::new();
        FisherYates.arrange(&mut empty, &ids(&[1]), &mut XorShift(1));
        assert!(empty.is_empty());
        let mut single = ids(&[1]);
        FisherYates.arrange(&mut single, &ids(&[1]), &mut XorShift(1));
        assert_eq!(single, ids(&[1]));
    }

    #[test]
    fn play_order_selects_its_strategy() {
        assert_eq!(PlayOrder::default(), PlayOrder::Sequential);
        let mut tracks = ids(&[1, 2, 3, 4, 5, 6, 7, 8]);
        PlayOrder::Sequential
            .strategy()
            .arrange(&mut tracks, &[], &mut XorShift(9));
        assert_eq!(tracks, ids(&[1, 2, 3, 4, 5, 6, 7, 8]));
        PlayOrder::Shuffle
            .strategy()
            .arrange(&mut tracks, &[], &mut XorShift(9));
        assert_ne!(tracks, ids(&[1, 2, 3, 4, 5, 6, 7, 8]));
    }
}
