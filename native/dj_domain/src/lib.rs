//! Pure domain model of Platine.
//!
//! This crate has **no audio and no I/O dependency**. It holds the entities
//! (tracks, decks, playback queue), the auto-mix state machine, the shuffle and
//! crossfade strategies, and the ports (traits) implemented by the adapters in
//! `dj_infra` and `dj_engine`.

// Tests compare exactly representable values on purpose.
#![cfg_attr(test, allow(clippy::float_cmp))]

pub mod automix;
pub mod crossfade;
pub mod deck;
pub mod error;
pub mod ports;
pub mod queue;
pub mod shuffle;
pub mod track;
pub mod values;
pub mod waveform;

pub use automix::{
    AutoMixAction, AutoMixInput, AutoMixScheduler, AutoMixSettings, AutoMixState, AutoMixStatus,
    TransitionPlan,
};
pub use crossfade::{CrossfadeCurve, CrossfadeGains, effective_fade};
pub use deck::{Deck, DeckLoad};
pub use error::DomainError;
pub use queue::{PlaybackQueue, RepeatMode};
pub use shuffle::{FisherYates, PlayOrder, Sequential, ShuffleStrategy};
pub use track::{Playlist, Track, TrackId, TrackTiming};
pub use values::{
    Bpm, CrossfadeDuration, CrossfaderPosition, DeckId, Gain, PitchPercent, PitchRange, Seconds,
};
pub use waveform::Waveform;

/// Semantic version of the domain crate, exposed for diagnostics.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

#[cfg(test)]
mod tests {
    use super::VERSION;

    #[test]
    fn exposes_crate_version() {
        assert_ne!(VERSION, "");
    }
}
