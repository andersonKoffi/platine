//! Full chain: files → loader threads → engine → offline output.
#![allow(
    clippy::unwrap_used,
    clippy::cast_possible_truncation,
    clippy::cast_precision_loss
)]

mod common;

use common::{TempDir, left, max_step, tone_level, wait_for};
use crossbeam_channel::{Receiver, unbounded};
use dj_domain::{CrossfadeCurve, CrossfaderPosition, DeckId, Seconds};
use dj_engine::{
    Engine, EngineCommand as Cmd, EngineConfig, EngineEvent, EngineHandle, EngineTransition,
};
use dj_infra::{DeckLoader, LoaderEvent, SymphoniaDecoder};
use test_support::wav::{Tone, write_corrupt};

const RATE: u32 = 48_000;

struct Rig {
    engine: Engine,
    handle: EngineHandle,
    loaders: [DeckLoader; 2],
    events: Receiver<LoaderEvent>,
}

impl Rig {
    fn new() -> Self {
        let config = EngineConfig {
            // Large rings so that whole fixtures fit and the test can render
            // faster than real time.
            feed_capacity_chunks: 2048,
            ..EngineConfig::new(RATE)
        };
        let (engine, handle, [feed_a, feed_b]) = Engine::new(config);
        let (tx, events) = unbounded();
        let loaders = [
            DeckLoader::spawn(DeckId::A, SymphoniaDecoder, feed_a, RATE, tx.clone()).unwrap(),
            DeckLoader::spawn(DeckId::B, SymphoniaDecoder, feed_b, RATE, tx).unwrap(),
        ];
        Self {
            engine,
            handle,
            loaders,
            events,
        }
    }

    fn loader(&self, deck: DeckId) -> &DeckLoader {
        &self.loaders[deck.index()]
    }

    fn load_fully(&self, deck: DeckId, path: std::path::PathBuf, generation: u32) {
        self.loader(deck).load(path, generation, Seconds::ZERO);
        wait_for(
            &self.events,
            |e| matches!(e, LoaderEvent::Finished { deck: d, generation: g } if *d == deck && *g == generation),
        );
    }

    fn render(&mut self, frames: usize) -> Vec<f32> {
        let mut out = vec![0.0; frames * 2];
        for block in out.chunks_mut(512 * 2) {
            self.engine.render(block, 2);
        }
        out
    }

    fn engine_events(&mut self) -> Vec<EngineEvent> {
        std::iter::from_fn(|| self.handle.try_recv()).collect()
    }
}

#[test]
fn crossfades_two_decoded_files_at_the_planned_sample() {
    let dir = TempDir::new("mix");
    let (a, b) = (dir.file("a.wav"), dir.file("b.wav"));
    Tone::sine(440.0, 3.0).write(&a).unwrap();
    Tone::sine(880.0, 3.0).write(&b).unwrap();

    let mut rig = Rig::new();
    rig.load_fully(DeckId::A, a, 1);
    rig.load_fully(DeckId::B, b, 1);
    for deck in DeckId::ALL {
        rig.handle
            .send(Cmd::Cue {
                deck,
                generation: 1,
                position_frame: 0,
            })
            .unwrap();
    }
    rig.handle
        .send(Cmd::SetCrossfader(CrossfaderPosition::fully(DeckId::A)))
        .unwrap();
    rig.handle.send(Cmd::Play { deck: DeckId::A }).unwrap();
    rig.handle
        .send(Cmd::ScheduleTransition(EngineTransition {
            from: DeckId::A,
            to: DeckId::B,
            start_at_frame: 72_000,
            fade_frames: 24_000,
            curve: CrossfadeCurve::EqualPower,
        }))
        .unwrap();

    let out = left(&rig.render(5 * RATE as usize));
    let events = rig.engine_events();

    let started = events.iter().find_map(|e| match e {
        EngineEvent::TransitionStarted { at_frame, .. } => Some(*at_frame),
        _ => None,
    });
    assert_eq!(started, Some(72_000), "fade starts on the planned sample");
    assert!(events.iter().any(|e| matches!(
        e,
        EngineEvent::TransitionCompleted {
            at_frame: 96_000,
            ..
        }
    )));
    // Deck A (3 s at 48 kHz) and then deck B (started at 1.5 s) both end.
    assert!(events.contains(&EngineEvent::TrackEnded {
        deck: DeckId::A,
        generation: 1
    }));
    assert!(events.contains(&EngineEvent::TrackEnded {
        deck: DeckId::B,
        generation: 1
    }));
    assert!(
        !events
            .iter()
            .any(|e| matches!(e, EngineEvent::Underrun { .. }))
    );

    assert!((tone_level(&out[24_000..70_000], 440.0, RATE, 24_000) - 0.5).abs() < 0.02);
    assert!(tone_level(&out[24_000..70_000], 880.0, RATE, 24_000) < 0.01);
    assert!(tone_level(&out[100_000..140_000], 440.0, RATE, 100_000) < 0.01);
    assert!((tone_level(&out[100_000..140_000], 880.0, RATE, 100_000) - 0.5).abs() < 0.02);
    // During the equal-power fade, 0.5 * (cos·440 Hz + sin·880 Hz) moves by at
    // most ~0.065 per sample (plus resampler ripple); a click jumps further.
    assert!(max_step(&out) < 0.09, "click detected: {}", max_step(&out));
}

#[test]
fn corrupt_file_fails_without_disturbing_the_other_deck() {
    let dir = TempDir::new("corrupt");
    let (good, bad) = (dir.file("good.wav"), dir.file("bad.wav"));
    Tone::sine(440.0, 1.0).write(&good).unwrap();
    write_corrupt(&bad).unwrap();

    let mut rig = Rig::new();
    rig.load_fully(DeckId::A, good, 1);
    rig.loader(DeckId::B).load(bad, 1, Seconds::ZERO);
    let failed = wait_for(&rig.events, |e| matches!(e, LoaderEvent::Failed { .. }));
    assert!(matches!(
        failed,
        LoaderEvent::Failed {
            deck: DeckId::B,
            generation: 1,
            ..
        }
    ));

    rig.handle
        .send(Cmd::Cue {
            deck: DeckId::A,
            generation: 1,
            position_frame: 0,
        })
        .unwrap();
    rig.handle
        .send(Cmd::SetCrossfader(CrossfaderPosition::fully(DeckId::A)))
        .unwrap();
    rig.handle.send(Cmd::Play { deck: DeckId::A }).unwrap();
    let out = left(&rig.render(RATE as usize / 2));
    assert!((tone_level(&out[4_800..], 440.0, RATE, 4_800) - 0.5).abs() < 0.02);
}

#[test]
fn seeking_switches_to_the_new_generation() {
    let dir = TempDir::new("seek");
    let path = dir.file("long.wav");
    Tone::sine(440.0, 4.0).at_rate(48_000).write(&path).unwrap();

    let mut rig = Rig::new();
    rig.load_fully(DeckId::A, path, 1);
    rig.loader(DeckId::A).seek(2, Seconds::saturating(3.0));
    let ready = wait_for(&rig.events, |e| {
        matches!(e, LoaderEvent::Ready { generation: 2, .. })
    });
    let LoaderEvent::Ready { info, .. } = ready else {
        unreachable!()
    };
    assert_eq!(info.source_rate, 48_000);
    assert!((info.duration.unwrap().as_f64() - 4.0).abs() < 1e-3);

    rig.handle
        .send(Cmd::Cue {
            deck: DeckId::A,
            generation: 2,
            position_frame: 144_000,
        })
        .unwrap();
    rig.handle.send(Cmd::Play { deck: DeckId::A }).unwrap();
    rig.render(RATE as usize / 2);
    let last_position = rig.engine_events().into_iter().rev().find_map(|e| match e {
        EngineEvent::Position {
            deck: DeckId::A,
            generation: 2,
            frame,
            ..
        } => Some(frame),
        _ => None,
    });
    assert!(last_position.unwrap() >= 144_000 + 20_000);
}

#[test]
fn mono_tracks_play_on_both_channels() {
    let dir = TempDir::new("mono");
    let path = dir.file("mono.wav");
    Tone::sine(440.0, 1.0).mono().write(&path).unwrap();
    let mut rig = Rig::new();
    rig.load_fully(DeckId::B, path, 1);
    rig.handle
        .send(Cmd::Cue {
            deck: DeckId::B,
            generation: 1,
            position_frame: 0,
        })
        .unwrap();
    rig.handle
        .send(Cmd::SetCrossfader(CrossfaderPosition::fully(DeckId::B)))
        .unwrap();
    rig.handle.send(Cmd::Play { deck: DeckId::B }).unwrap();
    let out = rig.render(RATE as usize / 2);
    let right: Vec<f32> = out.iter().skip(1).step_by(2).copied().collect();
    assert!((tone_level(&right[4_800..], 440.0, RATE, 4_800) - 0.5).abs() < 0.02);
}

#[test]
fn unloading_stops_feeding_the_deck() {
    let dir = TempDir::new("unload");
    let path = dir.file("t.wav");
    Tone::sine(440.0, 0.5).write(&path).unwrap();
    let rig = Rig::new();
    rig.loader(DeckId::A).load(path, 1, Seconds::ZERO);
    rig.loader(DeckId::A).unload();
    // The thread keeps serving requests after an unload.
    let other = dir.file("u.wav");
    Tone::sine(440.0, 0.2).write(&other).unwrap();
    rig.load_fully(DeckId::A, other, 2);
}
