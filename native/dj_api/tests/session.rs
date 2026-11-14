//! Auto-mix wired to the real engine, decoders and analysis, rendered offline.
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::cast_precision_loss,
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss
)]

use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use crossbeam_channel::{Receiver, bounded};
use dj_api::session::{
    Controller, LoadMode, ScanSummary, SessionCommand, SessionEvent, SessionSettings,
    build_with_config,
};
use dj_domain::ports::AudioOutput;
use dj_domain::{
    AutoMixSettings, AutoMixStatus, CrossfadeDuration, CrossfaderPosition, DeckId, PlayOrder,
    RepeatMode, Seconds, TrackId,
};
use dj_engine::{EngineConfig, OfflineOutput};
use test_support::SeededRandom;
use test_support::wav::{Tone, write_corrupt};

const RATE: u32 = 48_000;
const BLOCK: usize = 1024;

struct TempDir(PathBuf);

impl TempDir {
    fn new(name: &str) -> Self {
        let path =
            std::env::temp_dir().join(format!("platine-session-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&path);
        std::fs::create_dir_all(&path).unwrap();
        Self(path)
    }
}

impl Drop for TempDir {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

/// Four-second tone with 0.5 s of leading and 1 s of trailing silence.
fn padded_tone(dir: &Path, name: &str, frequency: f64) {
    Tone::sine(frequency, 4.0)
        .with_silences(0.5, 1.0)
        .write(&dir.join(name))
        .unwrap();
}

struct Harness {
    controller: Controller<SeededRandom>,
    output: OfflineOutput,
    events: Receiver<SessionEvent>,
    log: Vec<SessionEvent>,
    audio: Vec<f32>,
}

impl Harness {
    fn new(settings: SessionSettings) -> Self {
        let config = EngineConfig {
            feed_capacity_chunks: 2048,
            ..EngineConfig::new(RATE)
        };
        let parts = build_with_config(config, 2, settings, SeededRandom::new(7)).unwrap();
        let mut output = OfflineOutput::new(RATE, 2);
        output.start(parts.render).unwrap();
        Self {
            controller: parts.controller,
            output,
            events: parts.events,
            log: Vec::new(),
            audio: Vec::new(),
        }
    }

    fn settings(fade: f64, order: PlayOrder) -> SessionSettings {
        SessionSettings {
            automix: AutoMixSettings {
                crossfade: CrossfadeDuration::new(fade).unwrap(),
                ..AutoMixSettings::default()
            },
            order,
            repeat: RepeatMode::Off,
            ui_interval: Duration::ZERO,
            ..SessionSettings::default()
        }
    }

    fn load_folder(&mut self, path: &Path, mode: LoadMode) -> Result<ScanSummary, String> {
        let (reply, answer) = bounded(1);
        self.controller.handle(SessionCommand::LoadFolder {
            path: path.to_path_buf(),
            mode,
            reply,
        });
        self.drain();
        answer.recv().unwrap()
    }

    fn drain(&mut self) {
        self.log.extend(self.events.try_iter());
    }

    /// Renders audio in blocks, pausing virtual time until loaded decks are
    /// fully buffered (rendering is much faster than real time),
    /// until `done` holds or `max_seconds` of audio were rendered.
    fn run_until(&mut self, max_seconds: f64, done: impl Fn(&[SessionEvent]) -> bool) {
        let max_frames = (max_seconds * f64::from(RATE)) as usize;
        let deadline = Instant::now() + Duration::from_mins(1);
        let mut rendered = 0;
        while rendered < max_frames && Instant::now() < deadline {
            self.controller.poll();
            self.drain();
            if done(&self.log) {
                return;
            }
            if !self.controller.is_fully_buffered() {
                std::thread::sleep(Duration::from_millis(1));
                continue;
            }
            let block = self.output.render(BLOCK, 512);
            self.audio.extend(block.iter().step_by(2));
            rendered += BLOCK;
        }
        self.controller.poll();
        self.drain();
    }

    fn loads(&self) -> Vec<(DeckId, TrackId)> {
        self.log
            .iter()
            .filter_map(|e| match e {
                SessionEvent::DeckLoading { deck, track } => Some((*deck, *track)),
                _ => None,
            })
            .collect()
    }

    fn transitions(&self) -> Vec<(DeckId, DeckId)> {
        self.log
            .iter()
            .filter_map(|e| match e {
                SessionEvent::TransitionStarted { from, to } => Some((*from, *to)),
                _ => None,
            })
            .collect()
    }

    fn level(&self, frequency: f64, from_s: f64, to_s: f64) -> f32 {
        let start = (from_s * f64::from(RATE)) as usize;
        let end = ((to_s * f64::from(RATE)) as usize).min(self.audio.len());
        let window = &self.audio[start..end];
        let (mut i, mut q) = (0.0f64, 0.0f64);
        for (n, sample) in window.iter().enumerate() {
            let phase =
                2.0 * std::f64::consts::PI * frequency * (start + n) as f64 / f64::from(RATE);
            i += f64::from(*sample) * phase.sin();
            q += f64::from(*sample) * phase.cos();
        }
        (2.0 * (i * i + q * q).sqrt() / window.len() as f64) as f32
    }
}

fn exhausted(log: &[SessionEvent]) -> bool {
    log.contains(&SessionEvent::QueueExhausted)
}

#[test]
fn plays_a_folder_with_automatic_loading_and_role_swaps() {
    let dir = TempDir::new("folder");
    padded_tone(&dir.0, "01.wav", 440.0);
    padded_tone(&dir.0, "02.wav", 660.0);
    padded_tone(&dir.0, "03.wav", 880.0);

    let mut h = Harness::new(Harness::settings(1.0, PlayOrder::Sequential));
    let summary = h.load_folder(&dir.0, LoadMode::Replace).unwrap();
    assert_eq!(summary.tracks.len(), 3);
    h.run_until(20.0, exhausted);

    // A gets track 1, B track 2, then A (freed by the fade) gets track 3.
    assert_eq!(
        h.loads(),
        vec![
            (DeckId::A, TrackId(1)),
            (DeckId::B, TrackId(2)),
            (DeckId::A, TrackId(3))
        ]
    );
    // Roles swap at every transition.
    assert_eq!(
        h.transitions(),
        vec![(DeckId::A, DeckId::B), (DeckId::B, DeckId::A)]
    );
    assert!(exhausted(&h.log));
    assert!(
        !h.log
            .iter()
            .any(|e| matches!(e, SessionEvent::Underrun { .. }))
    );

    // Leading silence is skipped: track 1 is audible right away.
    assert!(h.level(440.0, 0.05, 0.5) > 0.4);
    // Track 1 is audible until effective end - fade = 4.5 - 1.0 => 3.0 s after start.
    assert!(h.level(660.0, 2.0, 2.95) < 0.01);
    assert!(h.level(660.0, 3.6, 3.95) > 0.4);
    // The trailing silence of track 1 is never faded into.
    assert!(h.level(440.0, 4.1, 5.0) < 0.01);
    // Track 2 hands over to track 3 another 3 s later.
    assert!(h.level(880.0, 6.6, 6.95) > 0.4);
}

#[test]
fn corrupt_files_are_skipped_without_stopping_the_music() {
    let dir = TempDir::new("corrupt");
    padded_tone(&dir.0, "01.wav", 440.0);
    write_corrupt(&dir.0.join("02.wav")).unwrap();
    padded_tone(&dir.0, "03.wav", 880.0);

    let mut h = Harness::new(Harness::settings(1.0, PlayOrder::Sequential));
    h.load_folder(&dir.0, LoadMode::Replace).unwrap();
    h.run_until(20.0, exhausted);

    assert!(h.log.iter().any(|e| matches!(
        e,
        SessionEvent::TrackSkipped {
            track: TrackId(2),
            ..
        }
    )));
    assert_eq!(h.transitions(), vec![(DeckId::A, DeckId::B)]);
    assert!(h.level(880.0, 3.6, 3.95) > 0.4, "track 3 follows track 1");
}

#[test]
fn shuffle_plays_every_track_once() {
    let dir = TempDir::new("shuffle");
    for (index, frequency) in [330.0, 440.0, 550.0, 660.0].iter().enumerate() {
        padded_tone(&dir.0, &format!("{index}.wav"), *frequency);
    }
    let mut h = Harness::new(Harness::settings(0.5, PlayOrder::Shuffle));
    h.load_folder(&dir.0, LoadMode::Replace).unwrap();
    h.run_until(30.0, exhausted);
    let mut played: Vec<TrackId> = h.loads().into_iter().map(|(_, track)| track).collect();
    played.sort();
    assert_eq!(played, vec![TrackId(1), TrackId(2), TrackId(3), TrackId(4)]);
    assert_eq!(h.transitions().len(), 3);
}

#[test]
fn manual_crossfader_suspends_and_resume_restarts_the_automation() {
    let dir = TempDir::new("manual");
    padded_tone(&dir.0, "01.wav", 440.0);
    padded_tone(&dir.0, "02.wav", 660.0);
    let mut h = Harness::new(Harness::settings(1.0, PlayOrder::Sequential));
    h.load_folder(&dir.0, LoadMode::Replace).unwrap();
    h.run_until(1.0, |_| false);

    h.controller
        .handle(SessionCommand::SetCrossfader(CrossfaderPosition::CENTER));
    h.drain();
    assert_eq!(h.controller.status(), AutoMixStatus::Suspended);
    assert!(h.log.iter().any(|e| matches!(
        e,
        SessionEvent::AutoMixChanged {
            status: AutoMixStatus::Suspended,
            ..
        }
    )));
    h.run_until(1.0, |_| false);
    assert_eq!(h.transitions(), Vec::new());

    h.controller.handle(SessionCommand::ResumeAutoMix);
    assert_eq!(h.controller.status(), AutoMixStatus::Running);
    h.run_until(10.0, |log| {
        log.iter()
            .any(|e| matches!(e, SessionEvent::TransitionStarted { .. }))
    });
    assert_eq!(h.transitions(), vec![(DeckId::A, DeckId::B)]);
}

#[test]
fn pausing_and_seeking_the_deck_on_air_keep_the_automation_running() {
    let dir = TempDir::new("onair");
    for (name, frequency) in [("01.wav", 440.0), ("02.wav", 660.0), ("03.wav", 880.0)] {
        padded_tone(&dir.0, name, frequency);
    }
    let mut h = Harness::new(Harness::settings(1.0, PlayOrder::Sequential));
    h.load_folder(&dir.0, LoadMode::Replace).unwrap();
    h.run_until(0.5, |_| false);

    h.controller.handle(SessionCommand::TogglePlay(DeckId::A));
    h.controller.handle(SessionCommand::TogglePlay(DeckId::A));
    h.controller.handle(SessionCommand::Seek {
        deck: DeckId::A,
        position: Seconds::new(1.0).unwrap(),
    });
    assert_eq!(h.controller.status(), AutoMixStatus::Running);

    h.run_until(20.0, exhausted);
    assert_eq!(
        h.transitions(),
        vec![(DeckId::A, DeckId::B), (DeckId::B, DeckId::A)]
    );
}

#[test]
fn a_suspended_automix_takes_over_when_the_music_would_stop() {
    let dir = TempDir::new("takeover");
    for (name, frequency) in [("01.wav", 440.0), ("02.wav", 660.0), ("03.wav", 880.0)] {
        padded_tone(&dir.0, name, frequency);
    }
    let mut h = Harness::new(Harness::settings(1.0, PlayOrder::Sequential));
    h.load_folder(&dir.0, LoadMode::Replace).unwrap();
    h.run_until(0.5, |_| false);
    h.controller
        .handle(SessionCommand::SetCrossfader(CrossfaderPosition::fully(
            DeckId::A,
        )));
    assert_eq!(h.controller.status(), AutoMixStatus::Suspended);

    // Track 1 ends without a fade, then tracks 2 and 3 follow on their own.
    h.run_until(20.0, exhausted);
    assert_eq!(h.controller.status(), AutoMixStatus::Running);
    assert_eq!(h.transitions(), vec![(DeckId::B, DeckId::A)]);
    let loads: Vec<TrackId> = h.loads().into_iter().map(|(_, track)| track).collect();
    assert_eq!(loads, vec![TrackId(1), TrackId(2), TrackId(3)]);
}

#[test]
fn manual_controls_reach_the_decks() {
    let dir = TempDir::new("controls");
    padded_tone(&dir.0, "01.wav", 440.0);
    padded_tone(&dir.0, "02.wav", 660.0);
    let mut h = Harness::new(Harness::settings(1.0, PlayOrder::Sequential));
    h.load_folder(&dir.0, LoadMode::Replace).unwrap();
    h.run_until(0.5, |_| false);

    h.controller.handle(SessionCommand::TogglePlay(DeckId::A));
    h.run_until(0.5, |_| false);
    let paused_level = h.level(
        440.0,
        h.audio.len() as f64 / f64::from(RATE) - 0.3,
        h.audio.len() as f64 / f64::from(RATE),
    );
    assert!(paused_level < 0.01, "{paused_level}");

    h.controller.handle(SessionCommand::TogglePlay(DeckId::A));
    h.controller.handle(SessionCommand::Seek {
        deck: DeckId::A,
        position: dj_domain::Seconds::saturating(2.0),
    });
    h.controller.handle(SessionCommand::SetDeckGain {
        deck: DeckId::A,
        gain: dj_domain::Gain::UNITY,
    });
    h.controller.handle(SessionCommand::SetEq {
        deck: DeckId::A,
        band: dj_engine::EqBand::Low,
        gain: dj_domain::Gain::UNITY,
    });
    h.controller
        .handle(SessionCommand::SetMasterGain(dj_domain::Gain::UNITY));
    h.controller
        .handle(SessionCommand::SetSilenceThreshold(-60.0));
    h.controller.handle(SessionCommand::SetCrossfadeCurve(
        dj_domain::CrossfadeCurve::Linear,
    ));
    h.run_until(0.5, |_| false);
    let end = h.audio.len() as f64 / f64::from(RATE);
    assert!(
        h.level(440.0, end - 0.3, end) > 0.4,
        "playing again after the seek"
    );

    h.controller.handle(SessionCommand::LoadTrack {
        deck: DeckId::B,
        track: TrackId(1),
    });
    h.run_until(0.2, |_| false);
    assert_eq!(h.controller.deck_track(DeckId::B), Some(TrackId(1)));
    assert!(h.controller.track(TrackId(1)).is_some());
    assert!(!h.controller.handle(SessionCommand::Shutdown));
}

#[test]
fn folder_errors_empty_folders_and_append_mode() {
    let dir = TempDir::new("folders");
    let mut h = Harness::new(Harness::settings(1.0, PlayOrder::Sequential));
    assert!(
        h.load_folder(&dir.0.join("missing"), LoadMode::Replace)
            .is_err()
    );

    let summary = h.load_folder(&dir.0, LoadMode::Replace).unwrap();
    assert_eq!(summary.tracks, Vec::new());
    assert!(exhausted(&h.log));

    padded_tone(&dir.0, "01.wav", 440.0);
    h.load_folder(&dir.0, LoadMode::Append).unwrap();
    h.run_until(0.5, |_| false);
    assert_eq!(h.loads(), vec![(DeckId::A, TrackId(1))]);
    assert!(
        h.log
            .iter()
            .any(|e| matches!(e, SessionEvent::LibraryChanged { tracks } if tracks.len() == 1))
    );
    assert!(h.log.iter().any(|e| matches!(
        e,
        SessionEvent::DeckPosition {
            deck: DeckId::A,
            ..
        }
    )));
    assert!(h.log.iter().any(|e| matches!(e, SessionEvent::Levels(_))));

    padded_tone(&dir.0, "02.wav", 550.0);
    padded_tone(&dir.0, "03.wav", 660.0);
    padded_tone(&dir.0, "04.wav", 770.0);
    h.load_folder(&dir.0, LoadMode::Replace).unwrap();
    h.controller
        .handle(SessionCommand::MoveInQueue { from: 1, to: 0 });
    h.drain();
    let last_queue = h.log.iter().rev().find_map(|e| match e {
        SessionEvent::QueueChanged { upcoming } => Some(upcoming.clone()),
        _ => None,
    });
    // Deck B took track 2 of the new scan; tracks 3, 4, 5 wait, then 4 moves first.
    assert_eq!(last_queue, Some(vec![TrackId(4), TrackId(3), TrackId(5)]));
    h.controller
        .handle(SessionCommand::MoveInQueue { from: 7, to: 0 });

    h.controller.handle(SessionCommand::SetShuffle(true));
    h.controller.handle(SessionCommand::SetRepeat(true));
    h.controller.handle(SessionCommand::SetAutoMix(false));
    assert_eq!(h.controller.status(), AutoMixStatus::Off);
    assert!(format!("{:?}", h.controller).contains("Controller"));
}
