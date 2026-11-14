//! Ports: traits implemented by adapters (`dj_infra`, `dj_engine`) and by
//! test fakes. They only use domain and standard-library types.
//!
//! The audio-facing ports are deliberately small; they are refined in P2 when
//! their adapters are written.

use std::path::{Path, PathBuf};

use crate::track::{Track, TrackTiming};
use crate::values::{Bpm, Seconds};

/// Monotonic time source.
pub trait Clock {
    /// Time elapsed since an arbitrary, fixed origin.
    fn now(&self) -> Seconds;
}

/// Source of randomness, injected so that shuffles are reproducible in tests.
pub trait RandomSource {
    /// Uniform integer in `0..upper`. `upper` is always at least 1.
    fn next_below(&mut self, upper: usize) -> usize;
}

/// Format of decoded audio.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StreamSpec {
    pub sample_rate: u32,
    pub channels: u16,
}

/// A decoded, seekable stream of interleaved `f32` samples.
pub trait AudioStream: Send {
    fn spec(&self) -> StreamSpec;

    /// Duration announced by the container, if known.
    fn duration(&self) -> Option<Seconds> {
        None
    }

    /// Fills `out` with interleaved samples; returns how many were written.
    /// `Ok(0)` means end of stream.
    ///
    /// # Errors
    /// Returns an error when the underlying data cannot be decoded.
    fn read(&mut self, out: &mut [f32]) -> Result<usize, DecodeError>;

    /// # Errors
    /// Returns an error when the position cannot be reached.
    fn seek(&mut self, position: Seconds) -> Result<(), DecodeError>;
}

/// Decoding failure. Messages come from the decoder and are shown to the user.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum DecodeError {
    #[error("unsupported format: {0}")]
    Unsupported(String),
    #[error("corrupt data: {0}")]
    Corrupt(String),
    #[error("cannot read file: {0}")]
    Io(String),
}

/// Opens audio files.
pub trait TrackDecoder {
    type Stream: AudioStream;

    /// # Errors
    /// Returns an error if the file cannot be opened or is not supported.
    fn open(&self, path: &Path) -> Result<Self::Stream, DecodeError>;
}

/// Result of analysing a track.
#[derive(Debug, Clone, PartialEq)]
pub struct TrackAnalysis {
    pub timing: TrackTiming,
    pub bpm: Option<Bpm>,
}

/// Analysis options.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct AnalysisRequest {
    /// Level under which audio counts as silence, in dBFS (default -50).
    pub silence_threshold_db: f32,
    pub detect_bpm: bool,
}

impl Default for AnalysisRequest {
    fn default() -> Self {
        Self {
            silence_threshold_db: -50.0,
            detect_bpm: false,
        }
    }
}

/// Computes silence boundaries, tempo, etc.
pub trait TrackAnalyzer {
    /// # Errors
    /// Returns an error if the file cannot be decoded.
    fn analyze(&self, path: &Path, request: &AnalysisRequest)
    -> Result<TrackAnalysis, DecodeError>;
}

/// A file that the scanner could not use.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SkippedFile {
    pub path: PathBuf,
    pub reason: String,
}

/// Outcome of scanning a folder. Unreadable files never abort the scan.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct ScanReport {
    pub tracks: Vec<Track>,
    pub skipped: Vec<SkippedFile>,
}

/// Finds audio files in a folder, recursively.
pub trait LibraryScanner {
    /// # Errors
    /// Returns an error only if the root folder itself cannot be read.
    fn scan(&self, root: &Path) -> Result<ScanReport, DecodeError>;
}

/// Something that renders audio into an interleaved buffer.
pub type RenderCallback = Box<dyn FnMut(&mut [f32]) + Send>;

/// Audio output device (or an offline renderer in tests).
pub trait AudioOutput {
    fn spec(&self) -> StreamSpec;

    /// Starts pulling audio from `render`.
    ///
    /// # Errors
    /// Returns an error if the device cannot be opened.
    fn start(&mut self, render: RenderCallback) -> Result<(), OutputError>;

    fn stop(&mut self);
}

/// Audio output failure.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum OutputError {
    #[error("no audio output device available")]
    NoDevice,
    #[error("audio device error: {0}")]
    Device(String),
}

#[cfg(test)]
mod tests {
    use super::*;

    struct Silence;

    impl AudioStream for Silence {
        fn spec(&self) -> StreamSpec {
            StreamSpec {
                sample_rate: 48_000,
                channels: 2,
            }
        }

        fn read(&mut self, _: &mut [f32]) -> Result<usize, DecodeError> {
            Ok(0)
        }

        fn seek(&mut self, _: Seconds) -> Result<(), DecodeError> {
            Ok(())
        }
    }

    #[test]
    fn streams_have_no_duration_by_default() {
        let mut stream = Silence;
        assert_eq!(stream.duration(), None);
        assert_eq!(stream.read(&mut []), Ok(0));
        assert_eq!(stream.seek(Seconds::ZERO), Ok(()));
        assert_eq!(stream.spec().channels, 2);
    }

    #[test]
    fn default_analysis_uses_minus_fifty_dbfs() {
        let request = AnalysisRequest::default();
        assert_eq!(request.silence_threshold_db, -50.0);
        assert!(!request.detect_bpm);
    }

    #[test]
    fn errors_have_readable_messages() {
        assert_eq!(
            DecodeError::Corrupt("bad frame".into()).to_string(),
            "corrupt data: bad frame"
        );
        assert_eq!(
            DecodeError::Unsupported("x".into()).to_string(),
            "unsupported format: x"
        );
        assert_eq!(
            DecodeError::Io("denied".into()).to_string(),
            "cannot read file: denied"
        );
        assert_eq!(
            OutputError::NoDevice.to_string(),
            "no audio output device available"
        );
        assert_eq!(
            OutputError::Device("gone".into()).to_string(),
            "audio device error: gone"
        );
    }
}
