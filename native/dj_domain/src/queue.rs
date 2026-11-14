//! Playback queue: what plays next, in which order, and what happens at the end.

use std::collections::VecDeque;

use crate::error::DomainError;
use crate::ports::RandomSource;
use crate::shuffle::PlayOrder;
use crate::track::TrackId;

/// What happens when every track has been played.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum RepeatMode {
    /// Stop at the end of the queue.
    #[default]
    Off,
    /// Start a new cycle (re-shuffled in shuffle mode).
    All,
}

/// How many recently played tracks a new shuffle cycle avoids starting with.
const RECENT_LEN: usize = 2;

/// Queue of tracks still to be played in the current cycle.
///
/// Tracks handed out by [`PlaybackQueue::next`] (those loaded on the decks)
/// leave the queue; re-ordering only affects the tracks still waiting.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct PlaybackQueue {
    /// Library order (alphabetical), the reference for sequential play.
    natural: Vec<TrackId>,
    upcoming: VecDeque<TrackId>,
    recent: VecDeque<TrackId>,
    order: PlayOrder,
    repeat: RepeatMode,
}

impl PlaybackQueue {
    /// Builds a queue from tracks in natural order.
    pub fn new(
        natural: Vec<TrackId>,
        order: PlayOrder,
        repeat: RepeatMode,
        rng: &mut dyn RandomSource,
    ) -> Self {
        let mut queue = Self {
            natural,
            upcoming: VecDeque::new(),
            recent: VecDeque::with_capacity(RECENT_LEN),
            order,
            repeat,
        };
        queue.start_cycle(&[], rng);
        queue
    }

    /// Hands out the next track, starting a new cycle if repeat is on.
    pub fn next(&mut self, rng: &mut dyn RandomSource) -> Option<TrackId> {
        if self.upcoming.is_empty() && self.repeat == RepeatMode::All {
            let recent: Vec<TrackId> = self.recent.iter().copied().collect();
            self.start_cycle(&recent, rng);
        }
        let track = self.upcoming.pop_front()?;
        if self.recent.len() == RECENT_LEN {
            self.recent.pop_front();
        }
        self.recent.push_back(track);
        Some(track)
    }

    /// Tracks waiting in the current cycle, in play order.
    pub fn upcoming(&self) -> impl ExactSizeIterator<Item = TrackId> + '_ {
        self.upcoming.iter().copied()
    }

    /// Next track without consuming it (does not start a new cycle).
    pub fn peek(&self) -> Option<TrackId> {
        self.upcoming.front().copied()
    }

    /// Number of tracks in the library (all cycles).
    pub fn library_len(&self) -> usize {
        self.natural.len()
    }

    pub fn is_exhausted(&self) -> bool {
        self.upcoming.is_empty() && (self.repeat == RepeatMode::Off || self.natural.is_empty())
    }

    pub fn order(&self) -> PlayOrder {
        self.order
    }

    pub fn repeat(&self) -> RepeatMode {
        self.repeat
    }

    pub fn set_repeat(&mut self, repeat: RepeatMode) {
        self.repeat = repeat;
    }

    /// Changes the play order of the tracks still waiting.
    pub fn set_order(&mut self, order: PlayOrder, rng: &mut dyn RandomSource) {
        self.order = order;
        let mut waiting: Vec<TrackId> = self.upcoming.drain(..).collect();
        match order {
            PlayOrder::Sequential => self.sort_naturally(&mut waiting),
            PlayOrder::Shuffle => order.strategy().arrange(&mut waiting, &[], rng),
        }
        self.upcoming = waiting.into();
    }

    /// Adds tracks to the library and to the current cycle.
    pub fn append(&mut self, tracks: &[TrackId], rng: &mut dyn RandomSource) {
        let new: Vec<TrackId> = tracks
            .iter()
            .copied()
            .filter(|track| !self.natural.contains(track))
            .collect();
        self.natural.extend(new.iter().copied());
        self.upcoming.extend(new);
        let order = self.order;
        self.set_order(order, rng);
    }

    /// Replaces the whole library and starts a fresh cycle.
    pub fn replace(&mut self, natural: Vec<TrackId>, rng: &mut dyn RandomSource) {
        self.natural = natural;
        self.recent.clear();
        self.start_cycle(&[], rng);
    }

    /// Removes a track that cannot be decoded, for this and future cycles.
    pub fn mark_unplayable(&mut self, track: TrackId) {
        self.natural.retain(|candidate| *candidate != track);
        self.upcoming.retain(|candidate| *candidate != track);
    }

    /// Moves a waiting track (drag and drop in the UI).
    ///
    /// # Errors
    /// Returns an error if an index is out of bounds.
    pub fn move_upcoming(&mut self, from: usize, to: usize) -> Result<(), DomainError> {
        let len = self.upcoming.len();
        for index in [from, to] {
            if index >= len {
                return Err(DomainError::QueueIndexOutOfBounds { index, len });
            }
        }
        if let Some(track) = self.upcoming.remove(from) {
            self.upcoming.insert(to, track);
        }
        Ok(())
    }

    fn start_cycle(&mut self, avoid_first: &[TrackId], rng: &mut dyn RandomSource) {
        let mut cycle = self.natural.clone();
        self.order.strategy().arrange(&mut cycle, avoid_first, rng);
        self.upcoming = cycle.into();
    }

    fn sort_naturally(&self, tracks: &mut [TrackId]) {
        tracks.sort_by_key(|track| {
            self.natural
                .iter()
                .position(|candidate| candidate == track)
                .unwrap_or(usize::MAX)
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::shuffle::test_rng::XorShift;

    fn ids(values: &[u64]) -> Vec<TrackId> {
        values.iter().copied().map(TrackId).collect()
    }

    fn drain(queue: &mut PlaybackQueue, rng: &mut XorShift, count: usize) -> Vec<TrackId> {
        (0..count).filter_map(|_| queue.next(rng)).collect()
    }

    fn sequential(values: &[u64], repeat: RepeatMode) -> PlaybackQueue {
        PlaybackQueue::new(ids(values), PlayOrder::Sequential, repeat, &mut XorShift(1))
    }

    #[test]
    fn sequential_queue_plays_in_natural_order_then_stops() {
        let mut rng = XorShift(1);
        let mut queue = sequential(&[1, 2, 3], RepeatMode::Off);
        assert_eq!(queue.peek(), Some(TrackId(1)));
        assert_eq!(drain(&mut queue, &mut rng, 5), ids(&[1, 2, 3]));
        assert!(queue.is_exhausted());
        assert_eq!(queue.next(&mut rng), None);
    }

    #[test]
    fn repeat_all_restarts_the_cycle() {
        let mut rng = XorShift(1);
        let mut queue = sequential(&[1, 2], RepeatMode::All);
        assert!(!queue.is_exhausted());
        assert_eq!(drain(&mut queue, &mut rng, 5), ids(&[1, 2, 1, 2, 1]));
    }

    #[test]
    fn single_track_with_repeat_loops_on_itself() {
        let mut rng = XorShift(1);
        let mut queue = sequential(&[7], RepeatMode::All);
        assert_eq!(drain(&mut queue, &mut rng, 3), ids(&[7, 7, 7]));
    }

    #[test]
    fn empty_queue_is_exhausted_even_with_repeat() {
        let mut rng = XorShift(1);
        let mut queue = sequential(&[], RepeatMode::All);
        assert!(queue.is_exhausted());
        assert_eq!(queue.next(&mut rng), None);
        assert_eq!(queue.library_len(), 0);
    }

    #[test]
    fn shuffle_plays_every_track_once_per_cycle() {
        let mut rng = XorShift(7);
        let mut queue = PlaybackQueue::new(
            ids(&[1, 2, 3, 4, 5]),
            PlayOrder::Shuffle,
            RepeatMode::All,
            &mut rng,
        );
        for _ in 0..4 {
            let mut cycle = drain(&mut queue, &mut rng, 5);
            cycle.sort();
            assert_eq!(cycle, ids(&[1, 2, 3, 4, 5]));
        }
    }

    #[test]
    fn new_shuffle_cycle_does_not_start_with_the_last_two_tracks() {
        let mut rng = XorShift(3);
        let mut queue = PlaybackQueue::new(
            ids(&[1, 2, 3, 4]),
            PlayOrder::Shuffle,
            RepeatMode::All,
            &mut rng,
        );
        for _ in 0..50 {
            let cycle = drain(&mut queue, &mut rng, 4);
            let next = queue.next(&mut rng).unwrap();
            assert!(!cycle[2..].contains(&next), "cycle {cycle:?} then {next:?}");
            // put the drawn track back in perspective: it starts the next cycle
            let rest = drain(&mut queue, &mut rng, 3);
            assert!(!rest.contains(&next));
            let mut all = rest.clone();
            all.push(next);
            all.sort();
            assert_eq!(all, ids(&[1, 2, 3, 4]));
        }
    }

    #[test]
    fn switching_order_only_touches_waiting_tracks() {
        let mut rng = XorShift(11);
        let mut queue = sequential(&[1, 2, 3, 4, 5, 6], RepeatMode::Off);
        let loaded = drain(&mut queue, &mut rng, 2);
        assert_eq!(loaded, ids(&[1, 2]));

        queue.set_order(PlayOrder::Shuffle, &mut rng);
        assert_eq!(queue.order(), PlayOrder::Shuffle);
        let mut waiting: Vec<TrackId> = queue.upcoming().collect();
        waiting.sort();
        assert_eq!(waiting, ids(&[3, 4, 5, 6]));

        queue.set_order(PlayOrder::Sequential, &mut rng);
        assert_eq!(queue.upcoming().collect::<Vec<_>>(), ids(&[3, 4, 5, 6]));
    }

    #[test]
    fn append_adds_new_tracks_once() {
        let mut rng = XorShift(1);
        let mut queue = sequential(&[1, 2], RepeatMode::Off);
        queue.append(&ids(&[2, 3]), &mut rng);
        assert_eq!(queue.library_len(), 3);
        assert_eq!(queue.upcoming().collect::<Vec<_>>(), ids(&[1, 2, 3]));
    }

    #[test]
    fn append_after_exhaustion_makes_tracks_available_again() {
        let mut rng = XorShift(1);
        let mut queue = sequential(&[1], RepeatMode::Off);
        assert_eq!(queue.next(&mut rng), Some(TrackId(1)));
        assert!(queue.is_exhausted());
        queue.append(&ids(&[5]), &mut rng);
        assert_eq!(queue.next(&mut rng), Some(TrackId(5)));
    }

    #[test]
    fn replace_starts_a_fresh_cycle() {
        let mut rng = XorShift(1);
        let mut queue = sequential(&[1, 2, 3], RepeatMode::Off);
        queue.next(&mut rng);
        queue.replace(ids(&[9, 8]), &mut rng);
        assert_eq!(queue.upcoming().collect::<Vec<_>>(), ids(&[9, 8]));
    }

    #[test]
    fn unplayable_tracks_never_come_back() {
        let mut rng = XorShift(1);
        let mut queue = sequential(&[1, 2, 3], RepeatMode::All);
        queue.mark_unplayable(TrackId(2));
        assert_eq!(drain(&mut queue, &mut rng, 4), ids(&[1, 3, 1, 3]));
    }

    #[test]
    fn waiting_tracks_can_be_reordered() {
        let mut queue = sequential(&[1, 2, 3, 4], RepeatMode::Off);
        queue.move_upcoming(3, 0).unwrap();
        assert_eq!(queue.upcoming().collect::<Vec<_>>(), ids(&[4, 1, 2, 3]));
        queue.move_upcoming(0, 2).unwrap();
        assert_eq!(queue.upcoming().collect::<Vec<_>>(), ids(&[1, 2, 4, 3]));
        assert_eq!(
            queue.move_upcoming(4, 0),
            Err(DomainError::QueueIndexOutOfBounds { index: 4, len: 4 })
        );
        assert!(queue.move_upcoming(0, 9).is_err());
    }

    #[test]
    fn repeat_can_be_toggled() {
        let mut queue = sequential(&[1], RepeatMode::Off);
        assert_eq!(queue.repeat(), RepeatMode::Off);
        queue.set_repeat(RepeatMode::All);
        assert_eq!(queue.repeat(), RepeatMode::All);
        assert_eq!(RepeatMode::default(), RepeatMode::Off);
        assert_eq!(PlaybackQueue::default().library_len(), 0);
    }
}
