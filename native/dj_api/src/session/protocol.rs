//! Messages between the UI façade and the session thread.

use std::path::PathBuf;

use crossbeam_channel::Sender;
use dj_domain::ports::SkippedFile;
use dj_domain::{
    AutoMixState, AutoMixStatus, CrossfadeCurve, CrossfaderPosition, DeckId, Gain, Seconds, Track,
    TrackId, TrackTiming, Waveform,
};
use dj_engine::{EqBand, Levels};

/// What to do with the current queue when a folder is chosen.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LoadMode {
    Replace,
    Append,
}

/// Result of scanning a folder.
#[derive(Debug, Clone, PartialEq)]
pub struct ScanSummary {
    pub tracks: Vec<Track>,
    pub skipped: Vec<SkippedFile>,
}

/// Use cases sent to the session.
#[derive(Debug, Clone)]
pub enum SessionCommand {
    LoadFolder {
        path: PathBuf,
        mode: LoadMode,
        reply: Sender<Result<ScanSummary, String>>,
    },
    SetAutoMix(bool),
    SetCrossfadeDuration(dj_domain::CrossfadeDuration),
    SetCrossfadeCurve(CrossfadeCurve),
    SetSilenceThreshold(f32),
    SetShuffle(bool),
    SetRepeat(bool),
    ResumeAutoMix,
    TogglePlay(DeckId),
    Seek {
        deck: DeckId,
        position: Seconds,
    },
    SetCrossfader(CrossfaderPosition),
    SetDeckGain {
        deck: DeckId,
        gain: Gain,
    },
    SetEq {
        deck: DeckId,
        band: EqBand,
        gain: Gain,
    },
    SetMasterGain(Gain),
    /// Moves a waiting track in the queue (drag and drop).
    MoveInQueue {
        from: usize,
        to: usize,
    },
    /// Loads a library track by hand (suspends the auto-mix).
    LoadTrack {
        deck: DeckId,
        track: TrackId,
    },
    Shutdown,
}

/// Events published to the UI.
#[derive(Debug, Clone, PartialEq)]
pub enum SessionEvent {
    LibraryChanged {
        tracks: Vec<Track>,
    },
    DeckLoading {
        deck: DeckId,
        track: TrackId,
    },
    DeckLoaded {
        deck: DeckId,
        track: TrackId,
        timing: TrackTiming,
    },
    /// Coloured overview of the track being loaded on `deck`.
    Waveform {
        deck: DeckId,
        track: TrackId,
        waveform: Waveform,
    },
    DeckPosition {
        deck: DeckId,
        position: Seconds,
        playing: bool,
    },
    Levels(Levels),
    AutoMixChanged {
        state: AutoMixState,
        status: AutoMixStatus,
    },
    QueueChanged {
        upcoming: Vec<TrackId>,
    },
    TransitionStarted {
        from: DeckId,
        to: DeckId,
    },
    TransitionCompleted {
        from: DeckId,
        to: DeckId,
    },
    /// A track could not be decoded and was skipped.
    TrackSkipped {
        track: TrackId,
        reason: String,
    },
    QueueExhausted,
    Underrun {
        deck: DeckId,
    },
}
