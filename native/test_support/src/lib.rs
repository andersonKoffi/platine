//! Shared test helpers for Platine: audio fixture generation and fakes of the
//! domain ports (`Clock`, `RandomSource`, ...).
//!
//! The helpers land together with the code they test (P1, P2).

/// Semantic version of the test-support crate, exposed for diagnostics.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");
