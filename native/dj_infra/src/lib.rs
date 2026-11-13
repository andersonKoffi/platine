//! Concrete adapters of Platine.
//!
//! Implements the domain ports: audio output (cpal), track decoding
//! (symphonia + rubato), folder scanning, analysis (silence, BPM, waveforms)
//! and the SQLite analysis cache.
//!
//! The content lands in phases P2, P3 and P5; P0 only provides the crate boundary.

/// Semantic version of the infrastructure crate, exposed for diagnostics.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

#[cfg(test)]
mod tests {
    use super::VERSION;

    #[test]
    fn exposes_crate_version() {
        assert!(!VERSION.is_empty());
    }
}
