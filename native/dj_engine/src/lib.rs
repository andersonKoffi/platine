//! Real-time audio graph of Platine.
//!
//! Two decks, 3-band biquad EQ, gain, crossfader, master and level metering.
//! The engine receives `EngineCommand`s and emits `EngineEvent`s through
//! lock-free ring buffers. Rules for the audio callback: no allocation, no
//! blocking lock, no I/O, no logging, no panic.
//!
//! The content lands in phase P2; P0 only provides the crate boundary.

/// Semantic version of the engine crate, exposed for diagnostics.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

#[cfg(test)]
mod tests {
    use super::VERSION;

    #[test]
    fn exposes_crate_version() {
        assert_ne!(VERSION, "");
    }
}
