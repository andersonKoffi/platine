//! Pure domain model of Platine.
//!
//! This crate has **no audio and no I/O dependency**. It holds the entities
//! (tracks, decks, playback queue), the auto-mix state machine, the shuffle and
//! crossfade strategies, and the ports (traits) implemented by the adapters in
//! `dj_infra` and `dj_engine`.
//!
//! The content lands in phase P1; P0 only provides the crate boundary.

/// Semantic version of the domain crate, exposed for diagnostics.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

#[cfg(test)]
mod tests {
    use super::VERSION;

    #[test]
    fn exposes_crate_version() {
        assert!(!VERSION.is_empty());
    }
}
