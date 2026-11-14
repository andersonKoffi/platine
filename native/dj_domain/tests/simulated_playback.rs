//! End-to-end auto-mix runs against a simulated engine driven by a manual clock.

#![allow(clippy::unwrap_used, clippy::float_cmp)]

mod common;

use std::collections::HashMap;

use common::{SimEngine, SimEvent, ids, t, trimmed};
use dj_domain::{
    AutoMixInput, AutoMixSettings, AutoMixState, CrossfadeDuration, DeckId, PlayOrder, RepeatMode,
    TrackId, TrackTiming,
};

const DT: f64 = 0.01;

/// Five 30 s tracks with 2 s of leading and 3 s of trailing silence.
fn library(count: u64) -> HashMap<TrackId, TrackTiming> {
    (1..=count)
        .map(|id| (TrackId(id), trimmed(30.0, 2.0, 3.0)))
        .collect()
}

fn engine(order: PlayOrder, repeat: RepeatMode) -> SimEngine {
    SimEngine::new(AutoMixSettings::default(), order, repeat, library(5))
}

#[test]
fn plays_the_whole_folder_with_alternating_decks() {
    let mut sim = engine(PlayOrder::Sequential, RepeatMode::Off);
    sim.load_library(ids(&[1, 2, 3, 4, 5]));
    sim.run(600.0, DT);

    assert_eq!(sim.scheduler.state(), AutoMixState::Finished);
    assert_eq!(
        sim.started_tracks(),
        vec![
            (DeckId::A, TrackId(1)),
            (DeckId::B, TrackId(2)),
            (DeckId::A, TrackId(3)),
            (DeckId::B, TrackId(4)),
            (DeckId::A, TrackId(5)),
        ]
    );
    assert!(matches!(
        sim.events.last(),
        Some(SimEvent::Exhausted { .. })
    ));
}

#[test]
fn fades_start_eight_seconds_before_the_trailing_silence() {
    let mut sim = engine(PlayOrder::Sequential, RepeatMode::Off);
    sim.load_library(ids(&[1, 2, 3, 4, 5]));
    sim.run(600.0, DT);

    // Track 1 starts at 0.5 s (load latency) from position 2 s; its audible
    // part ends at 27 s, so the fade starts at position 19 s, i.e. 17.5 s.
    // Each following track adds 25 s (playable) - 8 s (overlap) = 17 s.
    let fades = sim.fades();
    assert_eq!(fades.len(), 4);
    for (index, (at, fade)) in (0u32..).zip(fades.iter()) {
        let expected = 17.5 + 17.0 * f64::from(index);
        // The simulation detects events once per step, so each transition may
        // lag by up to one step; the real engine is sample-accurate.
        let tolerance = (f64::from(index) + 2.0) * DT;
        assert!(
            (at - expected).abs() <= tolerance,
            "fade {index} at {at}, expected {expected}"
        );
        assert!((fade - 8.0).abs() < 1e-9);
    }
}

#[test]
fn shuffle_plays_every_track_exactly_once() {
    let mut sim = engine(PlayOrder::Shuffle, RepeatMode::Off);
    sim.load_library(ids(&[1, 2, 3, 4, 5]));
    sim.run(600.0, DT);

    let mut played: Vec<TrackId> = sim
        .started_tracks()
        .into_iter()
        .map(|(_, track)| track)
        .collect();
    assert_eq!(played.len(), 5);
    played.sort();
    assert_eq!(played, ids(&[1, 2, 3, 4, 5]));
}

#[test]
fn corrupt_file_is_skipped_without_interrupting_playback() {
    let mut sim = engine(PlayOrder::Sequential, RepeatMode::Off).with_corrupt(&[t(3)]);
    sim.load_library(ids(&[1, 2, 3, 4, 5]));
    sim.run(600.0, DT);

    let played: Vec<TrackId> = sim
        .started_tracks()
        .into_iter()
        .map(|(_, track)| track)
        .collect();
    assert_eq!(played, ids(&[1, 2, 4, 5]));
    assert_eq!(sim.fades().len(), 3);
}

#[test]
fn instant_transitions_without_fade() {
    let settings = AutoMixSettings {
        crossfade: CrossfadeDuration::new(0.0).unwrap(),
        ..AutoMixSettings::default()
    };
    let mut sim = SimEngine::new(settings, PlayOrder::Sequential, RepeatMode::Off, library(3));
    sim.load_library(ids(&[1, 2, 3]));
    sim.run(600.0, DT);

    let fades = sim.fades();
    assert_eq!(fades.len(), 2);
    // Track 1 audible part ends at 0.5 + 25 = 25.5 s.
    assert!((fades[0].0 - 25.5).abs() <= 2.0 * DT);
    assert!(fades.iter().all(|(_, fade)| *fade == 0.0));
}

#[test]
fn repeat_keeps_playing_beyond_one_cycle() {
    let mut sim = engine(PlayOrder::Shuffle, RepeatMode::All);
    sim.load_library(ids(&[1, 2, 3, 4, 5]));
    sim.run(400.0, DT);

    let played: Vec<TrackId> = sim
        .started_tracks()
        .into_iter()
        .map(|(_, track)| track)
        .collect();
    assert!(played.len() >= 10, "only {} tracks in 400 s", played.len());
    for cycle in played.chunks(5).take(2) {
        let mut sorted = cycle.to_vec();
        sorted.sort();
        assert_eq!(sorted, ids(&[1, 2, 3, 4, 5]), "cycle {cycle:?}");
    }
}

#[test]
fn suspension_stops_automatic_transitions_until_resumed() {
    let mut sim = engine(PlayOrder::Sequential, RepeatMode::Off);
    sim.load_library(ids(&[1, 2, 3]));
    sim.run(5.0, DT);
    sim.feed(AutoMixInput::ManualIntervention);
    sim.run(40.0, DT);
    assert!(sim.fades().is_empty());

    sim.feed(AutoMixInput::Resume);
    sim.run(600.0, DT);
    let played: Vec<TrackId> = sim
        .started_tracks()
        .into_iter()
        .map(|(_, track)| track)
        .collect();
    assert_eq!(played.last(), Some(&t(3)));
}
