//! The audio callback must never allocate (ADR 0003).
//!
//! `assert_no_alloc` replaces the global allocator of this test binary and
//! aborts if anything allocates inside `assert_no_alloc(|| ...)`.
#![allow(clippy::unwrap_used, clippy::cast_possible_truncation)]

use assert_no_alloc::{AllocDisabler, assert_no_alloc};
use dj_domain::{CrossfadeCurve, CrossfaderPosition, DeckId, Gain, PitchPercent};
use dj_engine::{
    AudioChunk, CHUNK_FRAMES, Engine, EngineCommand as Cmd, EngineConfig, EngineTransition, EqBand,
};

#[global_allocator]
static ALLOCATOR: AllocDisabler = AllocDisabler;

#[test]
fn render_does_not_allocate() {
    let (mut engine, mut handle, [mut a, mut b]) = Engine::new(EngineConfig::new(48_000));
    let mut chunk = AudioChunk {
        generation: 1,
        frames: CHUNK_FRAMES as u16,
        ..AudioChunk::EMPTY
    };
    chunk.samples.fill(0.25);
    for index in 0..40u64 {
        chunk.start_frame = index * CHUNK_FRAMES as u64;
        assert!(a.try_push(&chunk));
        assert!(b.try_push(&chunk));
    }
    for deck in DeckId::ALL {
        handle
            .send(Cmd::Cue {
                deck,
                generation: 1,
                position_frame: 0,
            })
            .unwrap();
        handle
            .send(Cmd::SetEq {
                deck,
                band: EqBand::Low,
                gain: Gain::SILENCE,
            })
            .unwrap();
    }
    // Deck B is time-stretched (keylock) and pitched.
    handle
        .send(Cmd::SetKeylock {
            deck: DeckId::B,
            enabled: true,
        })
        .unwrap();
    handle
        .send(Cmd::SetPitch {
            deck: DeckId::B,
            pitch: PitchPercent::new(6.0).unwrap(),
        })
        .unwrap();
    handle
        .send(Cmd::SetPitch {
            deck: DeckId::A,
            pitch: PitchPercent::new(-3.0).unwrap(),
        })
        .unwrap();
    handle.send(Cmd::Play { deck: DeckId::A }).unwrap();
    handle
        .send(Cmd::SetCrossfader(CrossfaderPosition::CENTER))
        .unwrap();
    handle
        .send(Cmd::ScheduleTransition(EngineTransition {
            from: DeckId::A,
            to: DeckId::B,
            start_at_frame: 4_096,
            fade_frames: 4_096,
            curve: CrossfadeCurve::EqualPower,
        }))
        .unwrap();

    let mut out = vec![0.0f32; 512 * 2];
    assert_no_alloc(|| {
        for _ in 0..60 {
            engine.render(&mut out, 2);
        }
    });
    // Events were produced without allocating either.
    assert!(handle.try_recv().is_some());
}
