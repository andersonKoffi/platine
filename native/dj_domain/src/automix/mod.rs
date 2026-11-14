//! Auto-mix: automatic two-deck playback with crossfades.
//!
//! [`AutoMixScheduler`] is a pure state machine (State pattern). It receives
//! [`AutoMixInput`]s (user settings and engine reports) and answers with
//! [`AutoMixAction`]s (Command pattern) that the controller forwards to the
//! engine and the decoders. It never touches audio or files.
//!
//! ```text
//! Idle ──first deck ready──▶ Playing(A) ──transition starts──▶ Crossfading(A→B)
//!   ▲                          ▲   │                                │
//!   │                          │   └─track ended, next not ready─▶ WaitingForNext(B)
//!   │                          └──────── transition completes ◀─────┘
//!   └── (auto-mix off when a track ends)          … ──queue exhausted──▶ Finished
//! ```

mod scheduler;

pub use scheduler::AutoMixScheduler;

use crate::crossfade::CrossfadeCurve;
use crate::track::{TrackId, TrackTiming};
use crate::values::{CrossfadeDuration, CrossfaderPosition, DeckId, Seconds};

/// User settings of the auto-mix.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct AutoMixSettings {
    pub enabled: bool,
    pub crossfade: CrossfadeDuration,
    pub curve: CrossfadeCurve,
}

impl Default for AutoMixSettings {
    fn default() -> Self {
        Self {
            enabled: true,
            crossfade: CrossfadeDuration::default(),
            curve: CrossfadeCurve::EqualPower,
        }
    }
}

/// Where playback stands.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AutoMixState {
    /// Nothing is playing yet (decks may be loading or ready).
    Idle,
    /// One deck is playing; a transition to the other may be scheduled.
    Playing { active: DeckId },
    /// Both decks are playing while the crossfader moves.
    Crossfading { from: DeckId, to: DeckId },
    /// The active track ended before the next one finished loading.
    WaitingForNext { deck: DeckId },
    /// Every track has been played (repeat off) or the library is empty.
    Finished,
}

/// Auto-mix status shown in the UI.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AutoMixStatus {
    Off,
    Running,
    /// A manual action paused the automation; the user can resume it.
    Suspended,
}

/// A transition the engine must execute sample-accurately.
///
/// When the outgoing deck reaches `start_at` (on its own timeline), the
/// engine starts the incoming deck at `incoming_start` and moves the
/// crossfader from `from` to `to` over `fade` with `curve`. If the outgoing
/// deck is already past `start_at`, the transition starts immediately.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TransitionPlan {
    pub from: DeckId,
    pub to: DeckId,
    pub start_at: Seconds,
    pub fade: Seconds,
    pub incoming_start: Seconds,
    pub curve: CrossfadeCurve,
}

/// Events fed to the scheduler.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum AutoMixInput {
    SetEnabled(bool),
    /// Applies to the next transitions; a running crossfade keeps its length.
    SetCrossfadeDuration(CrossfadeDuration),
    SetCurve(CrossfadeCurve),
    TrackLoaded {
        deck: DeckId,
        track: TrackId,
        timing: TrackTiming,
    },
    TrackLoadFailed {
        deck: DeckId,
        track: TrackId,
    },
    TransitionStarted {
        from: DeckId,
        to: DeckId,
    },
    TransitionCompleted {
        from: DeckId,
        to: DeckId,
    },
    /// The deck reached the end of its track.
    TrackEnded {
        deck: DeckId,
    },
    /// "Mix now": start the transition to the standby deck immediately.
    /// `position` is where the deck on air currently is. Also hands control
    /// back to a suspended automation.
    MixNow {
        position: Seconds,
    },
    /// The user took the mix over (crossfader, playing the standby deck,
    /// replacing the track on air).
    ManualIntervention,
    /// The user loaded a track by hand. On the standby deck it becomes the
    /// next track; on the deck on air it suspends the automation.
    ManualLoad {
        deck: DeckId,
        track: TrackId,
    },
    Resume,
}

/// Commands emitted by the scheduler.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum AutoMixAction {
    LoadTrack {
        deck: DeckId,
        track: TrackId,
    },
    /// Starts (or keeps) playing; `from` seeks first when set.
    Play {
        deck: DeckId,
        from: Option<Seconds>,
    },
    Stop {
        deck: DeckId,
    },
    SetCrossfader(CrossfaderPosition),
    ScheduleTransition(TransitionPlan),
    /// Cancels the scheduled transition, or freezes a running one.
    CancelTransition,
    /// A track could not be decoded and was removed from the queue.
    SkippedUnplayable {
        track: TrackId,
    },
    QueueExhausted,
}
