//! Concrete adapters of Platine.
//!
//! - [`decode`]: symphonia decoder implementing the `TrackDecoder` port;
//! - [`resample`]: conversion to the engine sample rate (rubato);
//! - [`loader`]: one decoder thread per deck, feeding the engine's chunk rings;
//! - [`output`]: cpal output implementing the `AudioOutput` port.
//!
//! Scanning, analysis and the SQLite cache land in P3 and P5.

// Tests build signals with numeric casts on purpose.
#![cfg_attr(
    test,
    allow(
        clippy::cast_possible_truncation,
        clippy::cast_precision_loss,
        clippy::cast_possible_wrap,
        clippy::float_cmp
    )
)]

pub mod decode;
pub mod loader;
pub mod output;
pub mod resample;

pub use decode::{SymphoniaDecoder, SymphoniaStream};
pub use loader::{DeckLoader, LoadedTrackInfo, LoaderEvent, LoaderRequest};
pub use output::CpalOutput;
pub use resample::StereoResampler;

/// Semantic version of the infrastructure crate, exposed for diagnostics.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");
