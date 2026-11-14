//! Cost of one audio callback (512 frames, both decks playing, EQ active).
#![allow(
    missing_docs,
    clippy::unwrap_used,
    clippy::cast_possible_truncation,
    clippy::cast_precision_loss
)]

use std::hint::black_box;

use criterion::{Criterion, criterion_group, criterion_main};
use dj_domain::{CrossfaderPosition, DeckId, Gain};
use dj_engine::{AudioChunk, CHUNK_FRAMES, Engine, EngineCommand as Cmd, EngineConfig, EqBand};

fn bench_render(c: &mut Criterion) {
    let mut config = EngineConfig::new(48_000);
    config.feed_capacity_chunks = 8;
    let (mut engine, mut handle, [mut a, mut b]) = Engine::new(config);
    let mut chunk = AudioChunk {
        generation: 1,
        frames: CHUNK_FRAMES as u16,
        ..AudioChunk::EMPTY
    };
    for (i, sample) in chunk.samples.iter_mut().enumerate() {
        *sample = ((i as f32) * 0.01).sin() * 0.5;
    }
    for deck in DeckId::ALL {
        handle
            .send(Cmd::Cue {
                deck,
                generation: 1,
                position_frame: 0,
            })
            .unwrap();
        handle.send(Cmd::Play { deck }).unwrap();
        handle
            .send(Cmd::SetEq {
                deck,
                band: EqBand::Mid,
                gain: Gain::from_db(-6.0).unwrap(),
            })
            .unwrap();
    }
    handle
        .send(Cmd::SetCrossfader(CrossfaderPosition::CENTER))
        .unwrap();

    let mut out = vec![0.0f32; CHUNK_FRAMES * 2];
    let mut next_start = 0u64;
    c.bench_function("render 512 frames, 2 decks, EQ", |bencher| {
        bencher.iter(|| {
            // Keep both decks fed so that the measured path is the playing one.
            chunk.start_frame = next_start;
            next_start += CHUNK_FRAMES as u64;
            a.try_push(&chunk);
            b.try_push(&chunk);
            engine.render(black_box(&mut out), 2);
            while handle.try_recv().is_some() {}
        });
    });
}

criterion_group!(benches, bench_render);
criterion_main!(benches);
