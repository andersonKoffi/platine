//! Real-time audio graph of Platine.
//!
//! Two decks, a 3-band isolator EQ per deck, gain, crossfader (manual or
//! automated), master gain and peak metering. The engine receives
//! [`EngineCommand`]s and emits [`EngineEvent`]s through lock-free ring
//! buffers; decoded audio arrives as [`AudioChunk`]s through one ring per deck.
//!
//! Rules for [`Engine::render`] (ADR 0003): no allocation, no blocking lock,
//! no I/O, no logging, no panic.

// DSP code converts between frame counts and floats on purpose.
#![allow(
    clippy::cast_precision_loss,
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss
)]
// Tests compare exactly representable values on purpose.
#![cfg_attr(test, allow(clippy::float_cmp))]

pub mod chunk;
pub mod dsp;
pub mod engine;
pub mod offline;
pub mod protocol;
mod voice;

pub use chunk::{AudioChunk, CHUNK_FRAMES, DeckFeed, ENGINE_CHANNELS};
pub use engine::{CommandQueueFull, Engine, EngineConfig, EngineHandle};
pub use offline::OfflineOutput;
pub use protocol::{EngineCommand, EngineEvent, EngineTransition, EqBand, Levels};

/// Semantic version of the engine crate, exposed for diagnostics.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");
