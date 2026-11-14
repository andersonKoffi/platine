//! Commands sent to the engine and events it emits (Command and Observer patterns).
//!
//! Positions and durations are expressed in **engine frames** (at the engine
//! sample rate); converting from seconds is the controller's job.

use dj_domain::{CrossfadeCurve, CrossfaderPosition, DeckId, Gain};

/// One of the three isolator bands.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EqBand {
    Low,
    Mid,
    High,
}

impl EqBand {
    pub const ALL: [Self; 3] = [Self::Low, Self::Mid, Self::High];

    pub fn index(self) -> usize {
        match self {
            Self::Low => 0,
            Self::Mid => 1,
            Self::High => 2,
        }
    }
}

/// A transition to run sample-accurately (see `dj_domain::TransitionPlan`).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct EngineTransition {
    pub from: DeckId,
    pub to: DeckId,
    /// Position of the outgoing deck at which the fade starts.
    pub start_at_frame: u64,
    pub fade_frames: u64,
    pub curve: CrossfadeCurve,
}

/// Commands accepted by the audio thread.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum EngineCommand {
    /// Switches the deck to chunks of `generation` (a new load or a seek)
    /// and reports `position_frame` until they arrive.
    Cue {
        deck: DeckId,
        generation: u32,
        position_frame: u64,
    },
    Play {
        deck: DeckId,
    },
    Pause {
        deck: DeckId,
    },
    /// Pauses and forgets the loaded audio.
    Eject {
        deck: DeckId,
    },
    SetDeckGain {
        deck: DeckId,
        gain: Gain,
    },
    SetEq {
        deck: DeckId,
        band: EqBand,
        gain: Gain,
    },
    SetCrossfader(CrossfaderPosition),
    SetCrossfaderCurve(CrossfadeCurve),
    SetMasterGain(Gain),
    ScheduleTransition(EngineTransition),
    /// Cancels a scheduled transition or freezes a running one.
    CancelTransition,
}

/// Peak levels, linear (1.0 = 0 dBFS).
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Levels {
    pub deck_a: [f32; 2],
    pub deck_b: [f32; 2],
    pub master: [f32; 2],
}

/// Events emitted by the audio thread.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum EngineEvent {
    /// Periodic position report.
    Position {
        deck: DeckId,
        generation: u32,
        frame: u64,
        playing: bool,
    },
    /// Peak levels since the previous report.
    Levels(Levels),
    TrackEnded {
        deck: DeckId,
        generation: u32,
    },
    /// `at_frame` is the engine clock (frames rendered since start).
    TransitionStarted {
        from: DeckId,
        to: DeckId,
        at_frame: u64,
    },
    TransitionCompleted {
        from: DeckId,
        to: DeckId,
        at_frame: u64,
    },
    /// A playing deck ran out of decoded audio.
    Underrun {
        deck: DeckId,
    },
}

#[cfg(test)]
mod tests {
    use super::EqBand;

    #[test]
    fn bands_have_stable_indices() {
        let indices: Vec<usize> = EqBand::ALL.iter().map(|band| band.index()).collect();
        assert_eq!(indices, vec![0, 1, 2]);
    }
}
