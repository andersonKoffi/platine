//! Track analysis: exact duration and leading/trailing silence.
//!
//! BPM and waveforms join in P5.

use std::path::Path;

use dj_domain::ports::{
    AnalysisRequest, AudioStream, DecodeError, TrackAnalysis, TrackAnalyzer, TrackDecoder,
};
use dj_domain::{Seconds, TrackTiming};

/// Length of the analysis windows, in seconds.
const WINDOW_SECONDS: f64 = 0.01;

/// Finds silences by decoding the whole file once.
#[derive(Debug, Clone, Copy, Default)]
pub struct SilenceAnalyzer<D> {
    decoder: D,
}

impl<D: TrackDecoder> SilenceAnalyzer<D> {
    pub fn new(decoder: D) -> Self {
        Self { decoder }
    }
}

impl<D: TrackDecoder> TrackAnalyzer for SilenceAnalyzer<D> {
    fn analyze(
        &self,
        path: &Path,
        request: &AnalysisRequest,
    ) -> Result<TrackAnalysis, DecodeError> {
        let mut stream = self.decoder.open(path)?;
        let spec = stream.spec();
        let channels = usize::from(spec.channels.max(1));
        // Rates are small positive integers.
        #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
        let window_frames = ((f64::from(spec.sample_rate) * WINDOW_SECONDS) as usize).max(1);
        let threshold = 10f32.powf(request.silence_threshold_db / 20.0);

        let mut detector = SilenceDetector::new(threshold);
        let mut buffer = vec![0.0f32; window_frames * channels * 64];
        let mut window_peak = 0.0f32;
        let mut in_window = 0usize;
        loop {
            let read = stream.read(&mut buffer)?;
            if read == 0 {
                break;
            }
            for frame in buffer[..read].chunks_exact(channels) {
                for sample in frame {
                    window_peak = window_peak.max(sample.abs());
                }
                in_window += 1;
                if in_window == window_frames {
                    detector.push_window(window_peak, window_frames);
                    window_peak = 0.0;
                    in_window = 0;
                }
            }
        }
        if in_window > 0 {
            detector.push_window(window_peak, in_window);
        }

        let rate = f64::from(spec.sample_rate.max(1));
        // Frame counts of a track fit in f64 without loss.
        #[allow(clippy::cast_precision_loss)]
        let seconds = |frames: usize| Seconds::saturating(frames as f64 / rate);
        let (leading, trailing) = detector.silences();
        Ok(TrackAnalysis {
            timing: TrackTiming::from_silences(
                seconds(detector.total_frames),
                seconds(leading),
                seconds(trailing),
            ),
            bpm: None,
        })
    }
}

/// Tracks the first and last audible windows of a stream.
#[derive(Debug, Clone, Copy)]
struct SilenceDetector {
    threshold: f32,
    total_frames: usize,
    first_audible: Option<usize>,
    last_audible_end: usize,
}

impl SilenceDetector {
    fn new(threshold: f32) -> Self {
        Self {
            threshold,
            total_frames: 0,
            first_audible: None,
            last_audible_end: 0,
        }
    }

    fn push_window(&mut self, peak: f32, frames: usize) {
        if peak >= self.threshold {
            self.first_audible.get_or_insert(self.total_frames);
            self.last_audible_end = self.total_frames + frames;
        }
        self.total_frames += frames;
    }

    /// Leading and trailing silent frames. A fully silent stream has none.
    fn silences(&self) -> (usize, usize) {
        match self.first_audible {
            Some(first) => (first, self.total_frames - self.last_audible_end),
            None => (0, 0),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::SilenceDetector;

    #[test]
    fn detects_silence_at_both_ends() {
        let mut detector = SilenceDetector::new(0.01);
        for peak in [0.0, 0.001, 0.5, 0.0, 0.3, 0.002, 0.0] {
            detector.push_window(peak, 10);
        }
        assert_eq!(detector.silences(), (20, 20));
        assert_eq!(detector.total_frames, 70);
    }

    #[test]
    fn silent_stream_is_not_trimmed() {
        let mut detector = SilenceDetector::new(0.01);
        detector.push_window(0.0, 100);
        assert_eq!(detector.silences(), (0, 0));
    }
}
