//! Engine behaviour with synthetic audio (no files, no threads).
#![allow(
    clippy::unwrap_used,
    clippy::float_cmp,
    clippy::cast_precision_loss,
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss
)]

use dj_domain::{CrossfadeCurve, CrossfaderPosition, DeckId, Gain, Seconds};
use dj_engine::{
    AudioChunk, CHUNK_FRAMES, DeckFeed, Engine, EngineCommand as Cmd, EngineConfig, EngineEvent,
    EngineHandle, EngineTransition, EqBand,
};

const RATE: u32 = 48_000;

fn config() -> EngineConfig {
    EngineConfig {
        feed_capacity_chunks: 1024,
        ..EngineConfig::new(RATE)
    }
}

fn engine() -> (Engine, EngineHandle, [DeckFeed; 2]) {
    Engine::new(config())
}

/// Pushes `seconds` of a sine (or DC when `frequency` is 0) as `generation`.
fn feed_tone(feed: &mut DeckFeed, generation: u32, frequency: f64, seconds: f64, amplitude: f32) {
    let total = (seconds * f64::from(RATE)) as u64;
    let mut start = 0u64;
    while start < total {
        let frames = (total - start).min(CHUNK_FRAMES as u64);
        let mut chunk = AudioChunk {
            generation,
            start_frame: start,
            frames: u16::try_from(frames).unwrap(),
            end_of_stream: start + frames >= total,
            ..AudioChunk::EMPTY
        };
        for i in 0..frames as usize {
            let n = (start + i as u64) as f64;
            let value = if frequency == 0.0 {
                amplitude
            } else {
                amplitude
                    * (2.0 * std::f64::consts::PI * frequency * n / f64::from(RATE)).sin() as f32
            };
            chunk.samples[2 * i] = value;
            chunk.samples[2 * i + 1] = value;
        }
        assert!(feed.try_push(&chunk));
        start += frames;
    }
}

fn render(engine: &mut Engine, frames: usize) -> Vec<f32> {
    let mut out = vec![0.0; frames * 2];
    for block in out.chunks_mut(512 * 2) {
        engine.render(block, 2);
    }
    out
}

fn events(handle: &mut EngineHandle) -> Vec<EngineEvent> {
    std::iter::from_fn(|| handle.try_recv()).collect()
}

fn left(stereo: &[f32]) -> Vec<f32> {
    stereo.iter().step_by(2).copied().collect()
}

fn max_step(samples: &[f32]) -> f32 {
    samples
        .windows(2)
        .map(|w| (w[1] - w[0]).abs())
        .fold(0.0, f32::max)
}

/// Amplitude of `frequency` in `samples` (I/Q demodulation).
fn tone_level(samples: &[f32], frequency: f64, first_frame: usize) -> f32 {
    let (mut i, mut q) = (0.0f64, 0.0f64);
    for (n, sample) in samples.iter().enumerate() {
        let phase =
            2.0 * std::f64::consts::PI * frequency * (first_frame + n) as f64 / f64::from(RATE);
        i += f64::from(*sample) * phase.sin();
        q += f64::from(*sample) * phase.cos();
    }
    (2.0 * (i * i + q * q).sqrt() / samples.len() as f64) as f32
}

fn start_deck(handle: &mut EngineHandle, deck: DeckId) {
    handle
        .send(Cmd::Cue {
            deck,
            generation: 1,
            position_frame: 0,
        })
        .unwrap();
    handle.send(Cmd::Play { deck }).unwrap();
}

#[test]
fn a_crossfader_on_deck_a_only_plays_deck_a() {
    let (mut engine, mut handle, [mut a, mut b]) = engine();
    feed_tone(&mut a, 1, 440.0, 1.0, 0.5);
    feed_tone(&mut b, 1, 880.0, 1.0, 0.5);
    start_deck(&mut handle, DeckId::A);
    start_deck(&mut handle, DeckId::B);
    handle
        .send(Cmd::SetCrossfader(CrossfaderPosition::fully(DeckId::A)))
        .unwrap();
    let out = left(&render(&mut engine, RATE as usize / 2));
    let steady = &out[4800..];
    assert!((tone_level(steady, 440.0, 4800) - 0.5).abs() < 0.02);
    assert!(tone_level(steady, 880.0, 4800) < 0.01);
}

#[test]
fn transition_starts_at_the_planned_frame_and_swaps_decks() {
    let (mut engine, mut handle, [mut a, mut b]) = engine();
    feed_tone(&mut a, 1, 440.0, 3.0, 0.5);
    feed_tone(&mut b, 1, 880.0, 3.0, 0.5);
    handle
        .send(Cmd::SetCrossfader(CrossfaderPosition::fully(DeckId::A)))
        .unwrap();
    start_deck(&mut handle, DeckId::A);
    handle
        .send(Cmd::Cue {
            deck: DeckId::B,
            generation: 1,
            position_frame: 0,
        })
        .unwrap();
    handle
        .send(Cmd::ScheduleTransition(EngineTransition {
            from: DeckId::A,
            to: DeckId::B,
            start_at_frame: 48_000,
            fade_frames: 24_000,
            curve: CrossfadeCurve::EqualPower,
        }))
        .unwrap();

    let out = left(&render(&mut engine, 2 * RATE as usize));
    let events = events(&mut handle);
    let started = events.iter().find_map(|e| match e {
        EngineEvent::TransitionStarted { at_frame, .. } => Some(*at_frame),
        _ => None,
    });
    let completed = events.iter().find_map(|e| match e {
        EngineEvent::TransitionCompleted { at_frame, from, to } => Some((*at_frame, *from, *to)),
        _ => None,
    });
    // Deck A started at engine frame 0, so its position equals the engine clock.
    assert_eq!(started, Some(48_000));
    assert_eq!(completed, Some((72_000, DeckId::A, DeckId::B)));

    assert!(
        tone_level(&out[40_000..47_000], 880.0, 40_000) < 0.01,
        "B silent before the fade"
    );
    assert!(
        tone_level(&out[80_000..95_000], 440.0, 80_000) < 0.01,
        "A silent after the fade"
    );
    assert!((tone_level(&out[80_000..95_000], 880.0, 80_000) - 0.5).abs() < 0.02);
    assert!(max_step(&out) < 0.1, "no click: {}", max_step(&out));
}

#[test]
fn zero_length_transition_switches_in_one_frame() {
    let (mut engine, mut handle, [mut a, mut b]) = engine();
    feed_tone(&mut a, 1, 0.0, 1.0, 0.25);
    feed_tone(&mut b, 1, 0.0, 1.0, 0.25);
    handle
        .send(Cmd::SetCrossfader(CrossfaderPosition::fully(DeckId::A)))
        .unwrap();
    start_deck(&mut handle, DeckId::A);
    handle
        .send(Cmd::Cue {
            deck: DeckId::B,
            generation: 1,
            position_frame: 0,
        })
        .unwrap();
    handle
        .send(Cmd::ScheduleTransition(EngineTransition {
            from: DeckId::A,
            to: DeckId::B,
            start_at_frame: 1000,
            fade_frames: 0,
            curve: CrossfadeCurve::Linear,
        }))
        .unwrap();
    render(&mut engine, 4000);
    let kinds: Vec<_> = events(&mut handle)
        .into_iter()
        .filter_map(|e| match e {
            EngineEvent::TransitionStarted { at_frame, .. } => Some(("start", at_frame)),
            EngineEvent::TransitionCompleted { at_frame, .. } => Some(("end", at_frame)),
            _ => None,
        })
        .collect();
    assert_eq!(kinds, vec![("start", 1000), ("end", 1001)]);
}

#[test]
fn cancel_freezes_a_running_fade() {
    let (mut engine, mut handle, [mut a, mut b]) = engine();
    feed_tone(&mut a, 1, 440.0, 2.0, 0.5);
    feed_tone(&mut b, 1, 880.0, 2.0, 0.5);
    handle
        .send(Cmd::SetCrossfader(CrossfaderPosition::fully(DeckId::A)))
        .unwrap();
    start_deck(&mut handle, DeckId::A);
    handle
        .send(Cmd::Cue {
            deck: DeckId::B,
            generation: 1,
            position_frame: 0,
        })
        .unwrap();
    handle
        .send(Cmd::ScheduleTransition(EngineTransition {
            from: DeckId::A,
            to: DeckId::B,
            start_at_frame: 0,
            fade_frames: 48_000,
            curve: CrossfadeCurve::Linear,
        }))
        .unwrap();
    render(&mut engine, 24_064);
    handle.send(Cmd::CancelTransition).unwrap();
    let out = left(&render(&mut engine, 24_000));
    // Frozen half-way with a linear curve: both decks at about 0.5 * 0.5.
    let a_level = tone_level(&out[4_000..], 440.0, 24_064 + 4_000);
    let b_level = tone_level(&out[4_000..], 880.0, 24_064 + 4_000);
    assert!((a_level - 0.25).abs() < 0.02, "{a_level}");
    assert!((b_level - 0.25).abs() < 0.02, "{b_level}");
    assert!(
        !events(&mut handle)
            .iter()
            .any(|e| matches!(e, EngineEvent::TransitionCompleted { .. }))
    );
}

#[test]
fn a_transition_starts_when_the_outgoing_deck_ends_early() {
    let (mut engine, mut handle, [mut a, mut b]) = engine();
    feed_tone(&mut a, 1, 0.0, 0.1, 0.2);
    feed_tone(&mut b, 1, 0.0, 1.0, 0.2);
    start_deck(&mut handle, DeckId::A);
    handle
        .send(Cmd::Cue {
            deck: DeckId::B,
            generation: 1,
            position_frame: 0,
        })
        .unwrap();
    handle
        .send(Cmd::ScheduleTransition(EngineTransition {
            from: DeckId::A,
            to: DeckId::B,
            start_at_frame: 1_000_000,
            fade_frames: 100,
            curve: CrossfadeCurve::EqualPower,
        }))
        .unwrap();
    render(&mut engine, 12_000);
    let events = events(&mut handle);
    assert!(events.contains(&EngineEvent::TrackEnded {
        deck: DeckId::A,
        generation: 1
    }));
    assert!(
        events
            .iter()
            .any(|e| matches!(e, EngineEvent::TransitionStarted { .. }))
    );
}

#[test]
fn reports_positions_levels_and_track_end() {
    let (mut engine, mut handle, [mut a, _b]) = engine();
    feed_tone(&mut a, 7, 440.0, 0.25, 0.5);
    handle
        .send(Cmd::Cue {
            deck: DeckId::A,
            generation: 7,
            position_frame: 0,
        })
        .unwrap();
    handle.send(Cmd::Play { deck: DeckId::A }).unwrap();
    handle
        .send(Cmd::SetCrossfader(CrossfaderPosition::fully(DeckId::A)))
        .unwrap();
    render(&mut engine, RATE as usize / 2);
    assert_eq!(engine.clock(), u64::from(RATE / 2));
    let events = events(&mut handle);

    let positions: Vec<u64> = events
        .iter()
        .filter_map(|e| match e {
            EngineEvent::Position {
                deck: DeckId::A,
                generation: 7,
                frame,
                ..
            } => Some(*frame),
            _ => None,
        })
        .collect();
    assert!(positions.windows(2).all(|w| w[0] <= w[1]));
    assert_eq!(positions.last(), Some(&12_000));

    let loud = events
        .iter()
        .any(|e| matches!(e, EngineEvent::Levels(l) if l.deck_a[0] > 0.4 && l.master[0] > 0.4));
    assert!(loud);
    assert!(events.contains(&EngineEvent::TrackEnded {
        deck: DeckId::A,
        generation: 7
    }));
    assert!(
        !events
            .iter()
            .any(|e| matches!(e, EngineEvent::Underrun { .. }))
    );
}

#[test]
fn starving_deck_reports_an_underrun() {
    let (mut engine, mut handle, _feeds) = engine();
    start_deck(&mut handle, DeckId::B);
    render(&mut engine, 1024);
    assert!(events(&mut handle).contains(&EngineEvent::Underrun { deck: DeckId::B }));
}

#[test]
fn gain_eq_and_master_changes_are_click_free() {
    let (mut engine, mut handle, [mut a, _]) = engine();
    feed_tone(&mut a, 1, 1000.0, 1.0, 0.5);
    handle
        .send(Cmd::SetCrossfader(CrossfaderPosition::fully(DeckId::A)))
        .unwrap();
    start_deck(&mut handle, DeckId::A);
    render(&mut engine, 4_800);
    handle
        .send(Cmd::SetDeckGain {
            deck: DeckId::A,
            gain: Gain::SILENCE,
        })
        .unwrap();
    handle
        .send(Cmd::SetEq {
            deck: DeckId::A,
            band: EqBand::Mid,
            gain: Gain::SILENCE,
        })
        .unwrap();
    handle
        .send(Cmd::SetMasterGain(Gain::from_db(-6.0).unwrap()))
        .unwrap();
    handle
        .send(Cmd::SetCrossfaderCurve(CrossfadeCurve::Cut))
        .unwrap();
    let out = left(&render(&mut engine, 4_800));
    assert!(max_step(&out) < 0.1);
    assert!(out[2_000..].iter().all(|s| s.abs() < 1e-3));
}

#[test]
fn pause_and_eject_silence_the_deck_smoothly() {
    let (mut engine, mut handle, [mut a, _]) = engine();
    feed_tone(&mut a, 1, 440.0, 1.0, 0.8);
    handle
        .send(Cmd::SetCrossfader(CrossfaderPosition::fully(DeckId::A)))
        .unwrap();
    start_deck(&mut handle, DeckId::A);
    let mut out = left(&render(&mut engine, 4_800));
    handle.send(Cmd::Pause { deck: DeckId::A }).unwrap();
    out.extend(left(&render(&mut engine, 4_800)));
    assert!(max_step(&out) < 0.1);
    assert!(out[6_000..].iter().all(|s| *s == 0.0));
    handle.send(Cmd::Eject { deck: DeckId::A }).unwrap();
    handle.send(Cmd::Play { deck: DeckId::A }).unwrap();
    assert!(left(&render(&mut engine, 512)).iter().all(|s| *s == 0.0));
}

#[test]
fn mono_and_multichannel_outputs_are_supported() {
    let (mut engine, mut handle, [mut a, _]) = engine();
    feed_tone(&mut a, 1, 0.0, 1.0, 0.5);
    handle
        .send(Cmd::SetCrossfader(CrossfaderPosition::fully(DeckId::A)))
        .unwrap();
    start_deck(&mut handle, DeckId::A);
    let mut mono = vec![0.0; 2048];
    engine.render(&mut mono, 1);
    assert!(mono[2000] > 0.0);
    let mut quad = vec![1.0; 4096];
    engine.render(&mut quad, 4);
    assert_eq!(quad[4000 + 2], 0.0);
    assert_eq!(quad[4000 + 3], 0.0);
    assert!(quad[4000] > 0.0);
    engine.render(&mut quad, 0);
}

#[test]
fn handle_converts_units_and_reports_a_full_queue() {
    let mut config = config();
    config.command_capacity = 1;
    let (mut engine, mut handle, _) = Engine::new(config);
    assert_eq!(handle.sample_rate(), RATE);
    assert_eq!(handle.frames(Seconds::saturating(1.5)), 72_000);
    assert_eq!(handle.seconds(24_000), Seconds::saturating(0.5));
    handle.send(Cmd::Play { deck: DeckId::A }).unwrap();
    let err = handle.send(Cmd::Play { deck: DeckId::B }).unwrap_err();
    assert!(err.to_string().contains("queue is full"));
    engine.render(&mut [0.0; 2], 2);
    assert!(handle.send(Cmd::Play { deck: DeckId::B }).is_ok());
}

#[test]
fn render_callback_drives_the_engine() {
    use dj_domain::ports::AudioOutput;
    let (engine, mut handle, [mut a, _]) = engine();
    feed_tone(&mut a, 1, 0.0, 1.0, 0.5);
    handle
        .send(Cmd::SetCrossfader(CrossfaderPosition::fully(DeckId::A)))
        .unwrap();
    start_deck(&mut handle, DeckId::A);
    let mut output = dj_engine::OfflineOutput::new(RATE, 2);
    output.start(engine.into_render_callback(2)).unwrap();
    let out = output.render(4_800, 256);
    assert!(out[9_000] > 0.4);
}

#[test]
fn the_pitch_fader_changes_the_playback_speed() {
    use dj_domain::PitchPercent;

    let (mut engine, mut handle, [mut a, _]) = engine();
    feed_tone(&mut a, 1, 440.0, 1.0, 0.8);
    handle
        .send(Cmd::SetCrossfader(CrossfaderPosition::fully(DeckId::A)))
        .unwrap();
    handle
        .send(Cmd::SetPitch {
            deck: DeckId::A,
            pitch: PitchPercent::new(50.0).unwrap(),
        })
        .unwrap();
    start_deck(&mut handle, DeckId::A);
    let quarter = RATE as usize / 4;
    let out = left(&render(&mut engine, 2 * quarter));
    // Rising zero crossings over the last quarter second give the frequency.
    let crossings = out[quarter..]
        .windows(2)
        .filter(|w| w[0] < 0.0 && w[1] >= 0.0)
        .count();
    let frequency = crossings as f64 * 4.0;
    assert!((frequency - 660.0).abs() <= 8.0, "heard {frequency} Hz");
    // Half a second of output consumed three quarters of a second of track.
    let position = events(&mut handle)
        .iter()
        .rev()
        .find_map(|e| match e {
            EngineEvent::Position {
                deck: DeckId::A,
                frame,
                ..
            } => Some(*frame),
            _ => None,
        })
        .unwrap();
    let expected = (0.75 * f64::from(RATE)) as u64;
    assert!(
        position.abs_diff(expected) < u64::from(RATE / 50),
        "{position}"
    );
}

#[test]
fn keylock_keeps_the_key_when_the_pitch_changes() {
    use dj_domain::PitchPercent;

    let (mut engine, mut handle, [mut a, _]) = engine();
    feed_tone(&mut a, 1, 440.0, 1.5, 0.8);
    handle
        .send(Cmd::SetCrossfader(CrossfaderPosition::fully(DeckId::A)))
        .unwrap();
    handle
        .send(Cmd::SetKeylock {
            deck: DeckId::A,
            enabled: true,
        })
        .unwrap();
    handle
        .send(Cmd::SetPitch {
            deck: DeckId::A,
            pitch: PitchPercent::new(25.0).unwrap(),
        })
        .unwrap();
    start_deck(&mut handle, DeckId::A);
    let quarter = RATE as usize / 4;
    let out = left(&render(&mut engine, 4 * quarter));
    let crossings = out[2 * quarter..]
        .windows(2)
        .filter(|w| w[0] < 0.0 && w[1] >= 0.0)
        .count();
    let frequency = crossings as f64 * 2.0;
    assert!((frequency - 440.0).abs() <= 10.0, "heard {frequency} Hz");
    // One second of output went through 1.25 s of track; what is heard lags
    // by the latency of the stretcher (a few tenths of a second at most).
    let position = events(&mut handle)
        .iter()
        .rev()
        .find_map(|e| match e {
            EngineEvent::Position {
                deck: DeckId::A,
                frame,
                ..
            } => Some(*frame),
            _ => None,
        })
        .unwrap();
    let consumed = (1.25 * f64::from(RATE)) as u64;
    assert!(position < consumed, "{position}");
    assert!(consumed - position < u64::from(RATE * 3 / 10), "{position}");
}
