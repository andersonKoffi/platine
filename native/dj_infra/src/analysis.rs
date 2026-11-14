//! Track analysis in one decoding pass: exact duration, leading/trailing
//! silence and a three-band coloured waveform. BPM joins in P5.

use std::path::Path;

use dj_domain::ports::{
    AnalysisRequest, AudioStream, DecodeError, TrackAnalysis, TrackAnalyzer, TrackDecoder,
};
use dj_domain::{Seconds, TrackTiming, Waveform};

/// Length of the analysis windows, in seconds.
const WINDOW_SECONDS: f64 = 0.01;

/// Crossover frequencies of the waveform bands, in Hz (same as the EQ).
const LOW_MID_HZ: f32 = 250.0;
const MID_HIGH_HZ: f32 = 2_500.0;

/// Finds silences and builds the waveform by decoding the whole file once.
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
        let mut bands = BandSplitter::new(spec.sample_rate);
        let mut peaks: Vec<[f32; 3]> = Vec::new();
        let mut band_peak = [0.0f32; 3];
        let mut buffer = vec![0.0f32; window_frames * channels * 64];
        let mut window_peak = 0.0f32;
        let mut in_window = 0usize;
        loop {
            let read = stream.read(&mut buffer)?;
            if read == 0 {
                break;
            }
            for frame in buffer[..read].chunks_exact(channels) {
                let mut sum = 0.0f32;
                for sample in frame {
                    window_peak = window_peak.max(sample.abs());
                    sum += sample;
                }
                // Channel counts are tiny.
                #[allow(clippy::cast_precision_loss)]
                let mono = sum / channels as f32;
                for (peak, value) in band_peak.iter_mut().zip(bands.split(mono)) {
                    *peak = peak.max(value.abs());
                }
                in_window += 1;
                if in_window == window_frames {
                    detector.push_window(window_peak, window_frames);
                    peaks.push(band_peak);
                    window_peak = 0.0;
                    band_peak = [0.0; 3];
                    in_window = 0;
                }
            }
        }
        if in_window > 0 {
            detector.push_window(window_peak, in_window);
            peaks.push(band_peak);
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
            waveform: Waveform::from_peaks(
                points_per_second(window_frames, spec.sample_rate),
                &peaks,
            ),
        })
    }
}

/// Waveform resolution for a window length.
fn points_per_second(window_frames: usize, sample_rate: u32) -> u16 {
    // Window lengths are a few hundred frames.
    #[allow(clippy::cast_precision_loss)]
    let points = f64::from(sample_rate) / window_frames.max(1) as f64;
    // Rounded and clamped into u16 range first.
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    let points = points.round().clamp(1.0, f64::from(u16::MAX)) as u16;
    points
}

/// Splits a signal into low / mid / high bands with two one-pole low-passes.
#[derive(Debug, Clone, Copy)]
struct BandSplitter {
    low_coeff: f32,
    mid_coeff: f32,
    low_state: f32,
    mid_state: f32,
}

impl BandSplitter {
    fn new(sample_rate: u32) -> Self {
        // Precision loss is irrelevant for a filter coefficient.
        #[allow(clippy::cast_precision_loss)]
        let rate = sample_rate.max(1) as f32;
        let coeff = |cutoff: f32| 1.0 - (-std::f32::consts::TAU * cutoff / rate).exp();
        Self {
            low_coeff: coeff(LOW_MID_HZ),
            mid_coeff: coeff(MID_HIGH_HZ),
            low_state: 0.0,
            mid_state: 0.0,
        }
    }

    fn split(&mut self, sample: f32) -> [f32; 3] {
        self.low_state += self.low_coeff * (sample - self.low_state);
        self.mid_state += self.mid_coeff * (sample - self.mid_state);
        [
            self.low_state,
            self.mid_state - self.low_state,
            sample - self.mid_state,
        ]
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
    use super::{BandSplitter, SilenceDetector, points_per_second};

    fn band_peaks(frequency: f32) -> [f32; 3] {
        let mut splitter = BandSplitter::new(48_000);
        let mut peaks = [0.0f32; 3];
        for n in 0..48_000 {
            let phase = std::f32::consts::TAU * frequency * n as f32 / 48_000.0;
            let bands = splitter.split(phase.sin());
            if n > 24_000 {
                for (peak, value) in peaks.iter_mut().zip(bands) {
                    *peak = peak.max(value.abs());
                }
            }
        }
        peaks
    }

    #[test]
    fn bands_follow_the_frequency() {
        let bass = band_peaks(60.0);
        assert!(bass[0] > bass[1] && bass[0] > bass[2], "{bass:?}");
        let treble = band_peaks(8_000.0);
        assert!(treble[2] > treble[0] && treble[2] > treble[1], "{treble:?}");
    }

    #[test]
    fn ten_millisecond_windows_give_a_hundred_points_per_second() {
        assert_eq!(points_per_second(441, 44_100), 100);
        assert_eq!(points_per_second(480, 48_000), 100);
        assert_eq!(points_per_second(1, 96_000), u16::MAX);
    }

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
