//! Manual check of the audio chain on a real sound card.
//!
//! ```text
//! cargo run --release -p dj_infra --example mix -- first.mp3 second.flac [start_fade_at_s] [fade_s]
//! ```
//! Plays the first file, crossfades into the second (equal-power), and stops
//! when the second file ends.
#![allow(
    clippy::print_stdout,
    clippy::print_stderr,
    clippy::cast_precision_loss
)]

use std::path::PathBuf;
use std::time::Duration;

use crossbeam_channel::unbounded;
use dj_domain::ports::AudioOutput;
use dj_domain::{CrossfadeCurve, CrossfaderPosition, DeckId, Seconds};
use dj_engine::{Engine, EngineCommand as Cmd, EngineConfig, EngineEvent, EngineTransition};
use dj_infra::{CpalOutput, DeckLoader, LoaderEvent, SymphoniaDecoder};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let [first, second, rest @ ..] = args.as_slice() else {
        eprintln!("usage: mix <first-file> <second-file> [start_fade_at_s=20] [fade_s=8]");
        std::process::exit(2);
    };
    let start_at: f64 = rest.first().map_or(Ok(20.0), |v| v.parse())?;
    let fade: f64 = rest.get(1).map_or(Ok(8.0), |v| v.parse())?;

    let mut output = CpalOutput::open_default()?;
    let spec = output.spec();
    println!(
        "Output: {} Hz, {} channels",
        spec.sample_rate, spec.channels
    );

    let (engine, mut handle, [feed_a, feed_b]) = Engine::new(EngineConfig::new(spec.sample_rate));
    let (tx, loader_events) = unbounded();
    let deck_a = DeckLoader::spawn(
        DeckId::A,
        SymphoniaDecoder,
        feed_a,
        spec.sample_rate,
        tx.clone(),
    )?;
    let deck_b = DeckLoader::spawn(DeckId::B, SymphoniaDecoder, feed_b, spec.sample_rate, tx)?;
    deck_a.load(PathBuf::from(first), 1, Seconds::ZERO);
    deck_b.load(PathBuf::from(second), 1, Seconds::ZERO);

    let mut ready = 0;
    while ready < 2 {
        match loader_events.recv_timeout(Duration::from_secs(10))? {
            LoaderEvent::Ready { deck, info, .. } => {
                println!("Deck {deck} ready: {info:?}");
                ready += 1;
            }
            LoaderEvent::Failed { deck, error, .. } => {
                return Err(format!("deck {deck}: {error}").into());
            }
            LoaderEvent::Finished { .. } => {}
        }
    }

    for deck in DeckId::ALL {
        handle.send(Cmd::Cue {
            deck,
            generation: 1,
            position_frame: 0,
        })?;
    }
    handle.send(Cmd::SetCrossfader(CrossfaderPosition::fully(DeckId::A)))?;
    handle.send(Cmd::Play { deck: DeckId::A })?;
    handle.send(Cmd::ScheduleTransition(EngineTransition {
        from: DeckId::A,
        to: DeckId::B,
        start_at_frame: handle.frames(Seconds::new(start_at)?),
        fade_frames: handle.frames(Seconds::new(fade)?),
        curve: CrossfadeCurve::EqualPower,
    }))?;
    output.start(engine.into_render_callback(usize::from(spec.channels)))?;
    println!("Playing. Fade at {start_at} s over {fade} s. Ctrl-C to quit.");

    let mut last_second = [-1i64; 2];
    loop {
        while let Some(event) = handle.try_recv() {
            match event {
                EngineEvent::Position {
                    deck,
                    frame,
                    playing: true,
                    ..
                } => {
                    let seconds = frame as f64 / f64::from(spec.sample_rate);
                    #[allow(clippy::cast_possible_truncation)]
                    let whole = seconds as i64;
                    if whole != last_second[deck.index()] {
                        last_second[deck.index()] = whole;
                        println!("deck {deck} at {seconds:6.1} s");
                    }
                }
                EngineEvent::TransitionStarted { .. } => println!(">>> crossfade started"),
                EngineEvent::TransitionCompleted { .. } => println!(">>> crossfade done"),
                EngineEvent::Underrun { deck } => println!("!!! underrun on deck {deck}"),
                EngineEvent::TrackEnded {
                    deck: DeckId::B, ..
                } => {
                    println!("Done.");
                    return Ok(());
                }
                _ => {}
            }
        }
        std::thread::sleep(Duration::from_millis(30));
    }
}
