//! Use cases exposed to Flutter: session lifecycle, library, auto-mix and
//! deck/mixer controls, plus the stream of engine updates.
//!
//! Types here are plain data transfer objects; the Dart side maps them to its
//! own models in `data/` repositories.

// FRB requires owned parameters and passes them by value.
#![allow(clippy::needless_pass_by_value)]

use std::path::PathBuf;
use std::sync::{Mutex, MutexGuard, OnceLock};

use anyhow::{Context, anyhow};
use crossbeam_channel::bounded;
use dj_domain::{
    AutoMixSettings, AutoMixState, AutoMixStatus, CrossfadeCurve, CrossfadeDuration,
    CrossfaderPosition, DeckId, Gain, PlayOrder, RepeatMode, Seconds, Track, TrackId,
};
use dj_engine::EqBand;
use dj_infra::{CpalOutput, XorShiftRandom};

use crate::frb_generated::StreamSink;
use crate::session::{LoadMode, SessionCommand, SessionEvent, SessionHandle, SessionSettings};

// ----- data transfer objects ---------------------------------------------------

/// One of the two decks.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Deck {
    A,
    B,
}

/// Crossfader curve.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CurveKind {
    EqualPower,
    Linear,
    Cut,
}

/// Isolator band.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Band {
    Low,
    Mid,
    High,
}

/// What a newly chosen folder does to the queue.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FolderMode {
    Replace,
    Append,
}

/// Settings applied when the session starts.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SessionConfig {
    pub automix: bool,
    pub crossfade_seconds: f64,
    pub curve: CurveKind,
    pub shuffle: bool,
    pub repeat: bool,
    pub silence_threshold_db: f32,
}

/// A library track.
#[derive(Debug, Clone, PartialEq)]
pub struct TrackDto {
    pub id: u64,
    pub path: String,
    pub title: String,
    pub artist: Option<String>,
}

/// A file that could not be read during the scan.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SkippedDto {
    pub path: String,
    pub reason: String,
}

/// Result of scanning a folder.
#[derive(Debug, Clone, PartialEq)]
pub struct ScanResultDto {
    pub tracks: Vec<TrackDto>,
    pub skipped: Vec<SkippedDto>,
}

/// Auto-mix state machine position.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AutoMixStateDto {
    Idle,
    Playing { active: Deck },
    Crossfading { from: Deck, to: Deck },
    WaitingForNext { deck: Deck },
    Finished,
}

/// Auto-mix status shown by the UI.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AutoMixStatusDto {
    Off,
    Running,
    Suspended,
}

/// Peak levels (linear, 1.0 = 0 dBFS), left and right.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LevelsDto {
    pub deck_a: [f32; 2],
    pub deck_b: [f32; 2],
    pub master: [f32; 2],
}

/// Everything the UI needs to follow the session.
#[derive(Debug, Clone, PartialEq)]
pub enum EngineUpdate {
    LibraryChanged {
        tracks: Vec<TrackDto>,
    },
    DeckLoading {
        deck: Deck,
        track_id: u64,
    },
    DeckLoaded {
        deck: Deck,
        track_id: u64,
        duration_seconds: f64,
        start_seconds: f64,
        end_seconds: f64,
    },
    DeckPosition {
        deck: Deck,
        position_seconds: f64,
        playing: bool,
    },
    Levels(LevelsDto),
    AutoMixChanged {
        state: AutoMixStateDto,
        status: AutoMixStatusDto,
    },
    QueueChanged {
        upcoming: Vec<u64>,
    },
    TransitionStarted {
        from: Deck,
        to: Deck,
    },
    TransitionCompleted {
        from: Deck,
        to: Deck,
    },
    TrackSkipped {
        track_id: u64,
        reason: String,
    },
    QueueExhausted,
    Underrun {
        deck: Deck,
    },
}

// ----- session lifecycle -----------------------------------------------------------

fn session_slot() -> MutexGuard<'static, Option<SessionHandle>> {
    static SESSION: OnceLock<Mutex<Option<SessionHandle>>> = OnceLock::new();
    SESSION
        .get_or_init(|| Mutex::new(None))
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
}

fn with_session<T>(action: impl FnOnce(&SessionHandle) -> T) -> anyhow::Result<T> {
    let slot = session_slot();
    let session = slot
        .as_ref()
        .ok_or_else(|| anyhow!("audio session not started"))?;
    Ok(action(session))
}

fn send(command: SessionCommand) -> anyhow::Result<()> {
    with_session(|session| session.send(command))
}

/// Opens the default audio output and starts the engine. Replaces a running session.
///
/// # Errors
/// Fails if no audio output is available or a setting is out of range.
pub fn start_session(config: SessionConfig) -> anyhow::Result<()> {
    let settings = SessionSettings {
        automix: AutoMixSettings {
            enabled: config.automix,
            crossfade: CrossfadeDuration::new(config.crossfade_seconds)?,
            curve: config.curve.into(),
        },
        order: if config.shuffle {
            PlayOrder::Shuffle
        } else {
            PlayOrder::Sequential
        },
        repeat: if config.repeat {
            RepeatMode::All
        } else {
            RepeatMode::Off
        },
        silence_threshold_db: config.silence_threshold_db,
        ..SessionSettings::default()
    };
    let mut slot = session_slot();
    *slot = None; // stop the previous session first
    let output = CpalOutput::open_default().context("no audio output")?;
    let session = SessionHandle::start(Box::new(output), settings, XorShiftRandom::from_time())?;
    *slot = Some(session);
    Ok(())
}

/// Stops the audio and every worker thread.
pub fn stop_session() {
    *session_slot() = None;
}

/// Streams engine updates to Dart until the session stops.
///
/// # Errors
/// Fails if the session is not started.
pub fn engine_updates(sink: StreamSink<EngineUpdate>) -> anyhow::Result<()> {
    let events = with_session(|session| session.events().clone())?;
    std::thread::Builder::new()
        .name("platine-ui-events".into())
        .spawn(move || {
            for event in events {
                if sink.add(EngineUpdate::from(event)).is_err() {
                    return; // the Dart stream was cancelled
                }
            }
        })?;
    Ok(())
}

// ----- library and auto-mix ------------------------------------------------------------

/// Scans `path` recursively and replaces or extends the queue.
///
/// # Errors
/// Fails if the folder cannot be read or the session is not started.
pub fn load_folder(path: String, mode: FolderMode) -> anyhow::Result<ScanResultDto> {
    let (reply, answer) = bounded(1);
    send(SessionCommand::LoadFolder {
        path: PathBuf::from(path),
        mode: match mode {
            FolderMode::Replace => LoadMode::Replace,
            FolderMode::Append => LoadMode::Append,
        },
        reply,
    })?;
    let summary = answer
        .recv()
        .context("session stopped")?
        .map_err(|message| anyhow!(message))?;
    Ok(ScanResultDto {
        tracks: summary.tracks.iter().map(TrackDto::from).collect(),
        skipped: summary
            .skipped
            .into_iter()
            .map(|file| SkippedDto {
                path: file.path.to_string_lossy().into_owned(),
                reason: file.reason,
            })
            .collect(),
    })
}

/// # Errors
/// Fails if the session is not started.
pub fn set_automix_enabled(enabled: bool) -> anyhow::Result<()> {
    send(SessionCommand::SetAutoMix(enabled))
}

/// # Errors
/// Fails if `seconds` is outside `[0, 30]` or the session is not started.
pub fn set_crossfade_duration(seconds: f64) -> anyhow::Result<()> {
    send(SessionCommand::SetCrossfadeDuration(
        CrossfadeDuration::new(seconds)?,
    ))
}

/// # Errors
/// Fails if the session is not started.
pub fn set_crossfade_curve(curve: CurveKind) -> anyhow::Result<()> {
    send(SessionCommand::SetCrossfadeCurve(curve.into()))
}

/// Threshold (dBFS) below which the start and end of tracks count as silence.
///
/// # Errors
/// Fails if the session is not started.
pub fn set_silence_threshold(db: f32) -> anyhow::Result<()> {
    send(SessionCommand::SetSilenceThreshold(db))
}

/// # Errors
/// Fails if the session is not started.
pub fn set_shuffle(enabled: bool) -> anyhow::Result<()> {
    send(SessionCommand::SetShuffle(enabled))
}

/// # Errors
/// Fails if the session is not started.
pub fn set_repeat(enabled: bool) -> anyhow::Result<()> {
    send(SessionCommand::SetRepeat(enabled))
}

/// Resumes a suspended auto-mix.
///
/// # Errors
/// Fails if the session is not started.
pub fn resume_automix() -> anyhow::Result<()> {
    send(SessionCommand::ResumeAutoMix)
}

// ----- decks and mixer -------------------------------------------------------------------

/// # Errors
/// Fails if the session is not started.
pub fn toggle_play(deck: Deck) -> anyhow::Result<()> {
    send(SessionCommand::TogglePlay(deck.into()))
}

/// # Errors
/// Fails if `seconds` is negative or the session is not started.
pub fn seek(deck: Deck, seconds: f64) -> anyhow::Result<()> {
    send(SessionCommand::Seek {
        deck: deck.into(),
        position: Seconds::new(seconds)?,
    })
}

/// `position`: 0 = only deck A, 1 = only deck B.
///
/// # Errors
/// Fails if `position` is outside `[0, 1]` or the session is not started.
pub fn set_crossfader(position: f32) -> anyhow::Result<()> {
    send(SessionCommand::SetCrossfader(CrossfaderPosition::new(
        position,
    )?))
}

/// # Errors
/// Fails if `linear` is outside `[0, +12 dB]` or the session is not started.
pub fn set_deck_gain(deck: Deck, linear: f32) -> anyhow::Result<()> {
    send(SessionCommand::SetDeckGain {
        deck: deck.into(),
        gain: Gain::new(linear)?,
    })
}

/// `linear`: 0 kills the band, 1 is neutral.
///
/// # Errors
/// Fails if `linear` is outside `[0, +12 dB]` or the session is not started.
pub fn set_eq(deck: Deck, band: Band, linear: f32) -> anyhow::Result<()> {
    send(SessionCommand::SetEq {
        deck: deck.into(),
        band: band.into(),
        gain: Gain::new(linear)?,
    })
}

/// # Errors
/// Fails if `linear` is outside `[0, +12 dB]` or the session is not started.
pub fn set_master_gain(linear: f32) -> anyhow::Result<()> {
    send(SessionCommand::SetMasterGain(Gain::new(linear)?))
}

/// Moves a waiting track of the queue from index `from` to index `to`.
///
/// # Errors
/// Fails if the session is not started.
pub fn move_in_queue(from: u32, to: u32) -> anyhow::Result<()> {
    send(SessionCommand::MoveInQueue {
        from: from as usize,
        to: to as usize,
    })
}

/// Loads a library track on a deck by hand (suspends the auto-mix).
///
/// # Errors
/// Fails if the session is not started.
pub fn load_track(deck: Deck, track_id: u64) -> anyhow::Result<()> {
    send(SessionCommand::LoadTrack {
        deck: deck.into(),
        track: TrackId(track_id),
    })
}

// ----- conversions -------------------------------------------------------------------------

impl From<Deck> for DeckId {
    fn from(deck: Deck) -> Self {
        match deck {
            Deck::A => Self::A,
            Deck::B => Self::B,
        }
    }
}

impl From<DeckId> for Deck {
    fn from(deck: DeckId) -> Self {
        match deck {
            DeckId::A => Self::A,
            DeckId::B => Self::B,
        }
    }
}

impl From<CurveKind> for CrossfadeCurve {
    fn from(curve: CurveKind) -> Self {
        match curve {
            CurveKind::EqualPower => Self::EqualPower,
            CurveKind::Linear => Self::Linear,
            CurveKind::Cut => Self::Cut,
        }
    }
}

impl From<Band> for EqBand {
    fn from(band: Band) -> Self {
        match band {
            Band::Low => Self::Low,
            Band::Mid => Self::Mid,
            Band::High => Self::High,
        }
    }
}

impl From<&Track> for TrackDto {
    fn from(track: &Track) -> Self {
        Self {
            id: track.id.0,
            path: track.path.to_string_lossy().into_owned(),
            title: track.title.clone(),
            artist: track.artist.clone(),
        }
    }
}

impl From<AutoMixState> for AutoMixStateDto {
    fn from(state: AutoMixState) -> Self {
        match state {
            AutoMixState::Idle => Self::Idle,
            AutoMixState::Playing { active } => Self::Playing {
                active: active.into(),
            },
            AutoMixState::Crossfading { from, to } => Self::Crossfading {
                from: from.into(),
                to: to.into(),
            },
            AutoMixState::WaitingForNext { deck } => Self::WaitingForNext { deck: deck.into() },
            AutoMixState::Finished => Self::Finished,
        }
    }
}

impl From<AutoMixStatus> for AutoMixStatusDto {
    fn from(status: AutoMixStatus) -> Self {
        match status {
            AutoMixStatus::Off => Self::Off,
            AutoMixStatus::Running => Self::Running,
            AutoMixStatus::Suspended => Self::Suspended,
        }
    }
}

impl From<SessionEvent> for EngineUpdate {
    fn from(event: SessionEvent) -> Self {
        match event {
            SessionEvent::LibraryChanged { tracks } => Self::LibraryChanged {
                tracks: tracks.iter().map(TrackDto::from).collect(),
            },
            SessionEvent::DeckLoading { deck, track } => Self::DeckLoading {
                deck: deck.into(),
                track_id: track.0,
            },
            SessionEvent::DeckLoaded {
                deck,
                track,
                timing,
            } => Self::DeckLoaded {
                deck: deck.into(),
                track_id: track.0,
                duration_seconds: timing.duration().as_f64(),
                start_seconds: timing.effective_start().as_f64(),
                end_seconds: timing.effective_end().as_f64(),
            },
            SessionEvent::DeckPosition {
                deck,
                position,
                playing,
            } => Self::DeckPosition {
                deck: deck.into(),
                position_seconds: position.as_f64(),
                playing,
            },
            SessionEvent::Levels(levels) => Self::Levels(LevelsDto {
                deck_a: levels.deck_a,
                deck_b: levels.deck_b,
                master: levels.master,
            }),
            SessionEvent::AutoMixChanged { state, status } => Self::AutoMixChanged {
                state: state.into(),
                status: status.into(),
            },
            SessionEvent::QueueChanged { upcoming } => Self::QueueChanged {
                upcoming: upcoming.iter().map(|track| track.0).collect(),
            },
            SessionEvent::TransitionStarted { from, to } => Self::TransitionStarted {
                from: from.into(),
                to: to.into(),
            },
            SessionEvent::TransitionCompleted { from, to } => Self::TransitionCompleted {
                from: from.into(),
                to: to.into(),
            },
            SessionEvent::TrackSkipped { track, reason } => Self::TrackSkipped {
                track_id: track.0,
                reason,
            },
            SessionEvent::QueueExhausted => Self::QueueExhausted,
            SessionEvent::Underrun { deck } => Self::Underrun { deck: deck.into() },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use dj_domain::{TrackTiming, values::Seconds as S};
    use dj_engine::Levels;

    #[test]
    fn controls_fail_cleanly_without_a_session() {
        stop_session();
        assert!(set_shuffle(true).is_err());
        assert!(toggle_play(Deck::A).is_err());
        assert!(load_folder("/tmp".into(), FolderMode::Replace).is_err());
        assert!(move_in_queue(0, 1).is_err());
    }

    #[test]
    fn invalid_values_are_rejected_before_reaching_the_session() {
        assert!(set_crossfade_duration(31.0).is_err());
        assert!(set_crossfader(1.5).is_err());
        assert!(seek(Deck::B, -1.0).is_err());
        assert!(set_deck_gain(Deck::A, -0.5).is_err());
        assert!(set_eq(Deck::A, Band::Mid, 9.0).is_err());
        assert!(set_master_gain(f32::NAN).is_err());
    }

    #[test]
    fn domain_types_convert_both_ways() {
        assert_eq!(DeckId::from(Deck::B), DeckId::B);
        assert_eq!(Deck::from(DeckId::A), Deck::A);
        assert_eq!(CrossfadeCurve::from(CurveKind::Cut), CrossfadeCurve::Cut);
        assert_eq!(
            CrossfadeCurve::from(CurveKind::Linear),
            CrossfadeCurve::Linear
        );
        assert_eq!(EqBand::from(Band::High), EqBand::High);
        assert_eq!(EqBand::from(Band::Low), EqBand::Low);
        assert_eq!(
            AutoMixStateDto::from(AutoMixState::Crossfading {
                from: DeckId::A,
                to: DeckId::B
            }),
            AutoMixStateDto::Crossfading {
                from: Deck::A,
                to: Deck::B
            }
        );
        assert_eq!(
            AutoMixStatusDto::from(AutoMixStatus::Suspended),
            AutoMixStatusDto::Suspended
        );
    }

    #[test]
    fn session_events_map_to_updates() {
        let timing =
            TrackTiming::from_silences(S::saturating(10.0), S::saturating(1.0), S::saturating(2.0));
        let loaded = EngineUpdate::from(SessionEvent::DeckLoaded {
            deck: DeckId::A,
            track: TrackId(4),
            timing,
        });
        assert_eq!(
            loaded,
            EngineUpdate::DeckLoaded {
                deck: Deck::A,
                track_id: 4,
                duration_seconds: 10.0,
                start_seconds: 1.0,
                end_seconds: 8.0,
            }
        );
        let track = Track::from_path(TrackId(2), "/m/a.mp3");
        assert!(matches!(
            EngineUpdate::from(SessionEvent::LibraryChanged { tracks: vec![track] }),
            EngineUpdate::LibraryChanged { tracks } if tracks[0].title == "a"
        ));
        let events = [
            SessionEvent::DeckLoading {
                deck: DeckId::B,
                track: TrackId(1),
            },
            SessionEvent::DeckPosition {
                deck: DeckId::B,
                position: S::ZERO,
                playing: true,
            },
            SessionEvent::Levels(Levels::default()),
            SessionEvent::AutoMixChanged {
                state: AutoMixState::Idle,
                status: AutoMixStatus::Off,
            },
            SessionEvent::QueueChanged {
                upcoming: vec![TrackId(3)],
            },
            SessionEvent::TransitionStarted {
                from: DeckId::A,
                to: DeckId::B,
            },
            SessionEvent::TransitionCompleted {
                from: DeckId::A,
                to: DeckId::B,
            },
            SessionEvent::TrackSkipped {
                track: TrackId(9),
                reason: "bad".into(),
            },
            SessionEvent::QueueExhausted,
            SessionEvent::Underrun { deck: DeckId::A },
        ];
        for event in events {
            let _ = EngineUpdate::from(event);
        }
        for state in [
            AutoMixState::Idle,
            AutoMixState::Playing { active: DeckId::A },
            AutoMixState::WaitingForNext { deck: DeckId::B },
            AutoMixState::Finished,
        ] {
            let _ = AutoMixStateDto::from(state);
        }
        let _ = AutoMixStatusDto::from(AutoMixStatus::Running);
        let _ = CrossfadeCurve::from(CurveKind::EqualPower);
        let _ = EqBand::from(Band::Mid);
    }
}
