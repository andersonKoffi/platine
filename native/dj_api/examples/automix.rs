//! Auto-mix a folder on the sound card, without the UI.
//!
//! ```text
//! cargo run --release -p dj_api --example automix -- <folder> [fade_seconds=8] [--shuffle] [--repeat]
//! ```
#![allow(clippy::print_stdout, clippy::print_stderr)]

use std::collections::HashMap;
use std::path::PathBuf;

use crossbeam_channel::bounded;
use dj_api::session::{LoadMode, SessionCommand, SessionEvent, SessionHandle, SessionSettings};
use dj_domain::{AutoMixSettings, CrossfadeDuration, PlayOrder, RepeatMode, TrackId};
use dj_infra::{CpalOutput, XorShiftRandom};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let Some(folder) = args.first() else {
        eprintln!("usage: automix <folder> [fade_seconds=8] [--shuffle] [--repeat]");
        std::process::exit(2);
    };
    let fade: f64 = args
        .iter()
        .skip(1)
        .find(|arg| !arg.starts_with("--"))
        .map_or(Ok(8.0), |value| value.parse())?;
    let shuffle = args.iter().any(|arg| arg == "--shuffle");
    let repeat = args.iter().any(|arg| arg == "--repeat");

    let settings = SessionSettings {
        automix: AutoMixSettings {
            crossfade: CrossfadeDuration::new(fade)?,
            ..AutoMixSettings::default()
        },
        order: if shuffle {
            PlayOrder::Shuffle
        } else {
            PlayOrder::Sequential
        },
        repeat: if repeat {
            RepeatMode::All
        } else {
            RepeatMode::Off
        },
        ..SessionSettings::default()
    };
    let output = CpalOutput::open_default()?;
    let session = SessionHandle::start(Box::new(output), settings, XorShiftRandom::from_time())?;

    let (reply, answer) = bounded(1);
    session.send(SessionCommand::LoadFolder {
        path: PathBuf::from(folder),
        mode: LoadMode::Replace,
        reply,
    });
    let summary = answer.recv()??;
    println!(
        "{} tracks found, {} unreadable entries.",
        summary.tracks.len(),
        summary.skipped.len()
    );
    let titles: HashMap<TrackId, String> = summary
        .tracks
        .iter()
        .map(|track| (track.id, track.title.clone()))
        .collect();
    let title = |id: &TrackId| titles.get(id).cloned().unwrap_or_default();

    for event in session.events() {
        match event {
            SessionEvent::DeckLoading { deck, track } => {
                println!("[{deck}] loading  {}", title(&track));
            }
            SessionEvent::DeckLoaded {
                deck,
                track,
                timing,
            } => println!(
                "[{deck}] ready    {} (audible {} → {})",
                title(&track),
                timing.effective_start(),
                timing.effective_end()
            ),
            SessionEvent::TransitionStarted { from, to } => println!(">>> crossfade {from} → {to}"),
            SessionEvent::TransitionCompleted { from, to } => {
                println!("<<< now playing deck {to} (deck {from} free)");
            }
            SessionEvent::TrackSkipped { track, reason } => {
                println!("!!! skipped {}: {reason}", title(&track));
            }
            SessionEvent::AutoMixChanged { state, status } => {
                println!("    auto-mix {status:?}, {state:?}");
            }
            SessionEvent::Underrun { deck } => println!("!!! underrun on deck {deck}"),
            SessionEvent::QueueExhausted => {
                println!("End of the queue.");
                break;
            }
            _ => {}
        }
    }
    // Let the last track finish its tail before stopping the output.
    std::thread::sleep(std::time::Duration::from_millis(300));
    Ok(())
}
