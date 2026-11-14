//! A deck, as seen by the domain: which track it holds and whether it is ready.

use crate::track::{TrackId, TrackTiming};
use crate::values::DeckId;

/// Loading state of a deck.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub enum DeckLoad {
    #[default]
    Empty,
    /// A load was requested and has not completed yet.
    Loading(TrackId),
    /// The track is decoded enough to play and its timing is known.
    Ready { track: TrackId, timing: TrackTiming },
}

/// One of the two decks.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Deck {
    id: DeckId,
    load: DeckLoad,
}

impl Deck {
    pub fn new(id: DeckId) -> Self {
        Self {
            id,
            load: DeckLoad::Empty,
        }
    }

    pub fn id(&self) -> DeckId {
        self.id
    }

    pub fn load(&self) -> DeckLoad {
        self.load
    }

    /// Track currently held, whether loading or ready.
    pub fn track(&self) -> Option<TrackId> {
        match self.load {
            DeckLoad::Empty => None,
            DeckLoad::Loading(track) | DeckLoad::Ready { track, .. } => Some(track),
        }
    }

    /// Timing of the ready track.
    pub fn ready_timing(&self) -> Option<TrackTiming> {
        match self.load {
            DeckLoad::Ready { timing, .. } => Some(timing),
            DeckLoad::Empty | DeckLoad::Loading(_) => None,
        }
    }

    pub fn is_empty(&self) -> bool {
        self.load == DeckLoad::Empty
    }

    pub fn is_loading(&self) -> bool {
        matches!(self.load, DeckLoad::Loading(_))
    }

    pub(crate) fn start_loading(&mut self, track: TrackId) {
        self.load = DeckLoad::Loading(track);
    }

    pub(crate) fn mark_ready(&mut self, track: TrackId, timing: TrackTiming) {
        self.load = DeckLoad::Ready { track, timing };
    }

    pub(crate) fn clear(&mut self) {
        self.load = DeckLoad::Empty;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::values::Seconds;

    #[test]
    fn walks_through_its_loading_states() {
        let mut deck = Deck::new(DeckId::B);
        assert_eq!(deck.id(), DeckId::B);
        assert!(deck.is_empty());
        assert_eq!(deck.track(), None);

        deck.start_loading(TrackId(7));
        assert!(deck.is_loading());
        assert_eq!(deck.track(), Some(TrackId(7)));
        assert_eq!(deck.ready_timing(), None);

        let timing = TrackTiming::untrimmed(Seconds::saturating(60.0));
        deck.mark_ready(TrackId(7), timing);
        assert_eq!(
            deck.load(),
            DeckLoad::Ready {
                track: TrackId(7),
                timing
            }
        );
        assert_eq!(deck.ready_timing(), Some(timing));
        assert_eq!(deck.track(), Some(TrackId(7)));

        deck.clear();
        assert!(deck.is_empty());
    }
}
