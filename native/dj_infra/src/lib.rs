//! Concrete adapters of Platine.
//!
//! - [`decode`]: symphonia decoder implementing the `TrackDecoder` port;
//! - [`resample`]: conversion to the engine sample rate (rubato);
//! - [`loader`]: one decoder thread per deck, feeding the engine's chunk rings;
//! - [`output`]: cpal output implementing the `AudioOutput` port;
//! - [`scan`]: recursive folder scanning (`LibraryScanner` port);
//! - [`analysis`]: duration and silence detection (`TrackAnalyzer` port);
//! - [`random`]: shuffle random source (`RandomSource` port).
//!
//! BPM, waveforms and the SQLite cache land in P5.

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

pub mod analysis;
pub mod cache;
pub mod decode;
pub mod loader;
pub mod output;
pub mod random;
pub mod resample;
pub mod scan;

pub use analysis::SilenceAnalyzer;
pub use cache::{CachedAnalyzer, SqliteAnalysisCache};
pub use decode::{SymphoniaDecoder, SymphoniaStream};
pub use loader::{DeckLoader, LoadedTrackInfo, LoaderEvent, LoaderRequest};
pub use output::CpalOutput;
pub use random::XorShiftRandom;
pub use resample::StereoResampler;
pub use scan::{AUDIO_EXTENSIONS, FolderScanner, is_audio_file};

/// Semantic version of the infrastructure crate, exposed for diagnostics.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");
