//! Tempo (BPM) and beat phase estimation.
//!
//! Two pure stages, fed by the analysis pass:
//! 1. [`OnsetDetector`] turns mono samples into an onset-strength envelope
//!    (how much the energy rises, sampled every few milliseconds), weighting
//!    the low band where kicks live.
//! 2. [`estimate_beat_grid`] finds the beat period with an autocorrelation of
//!    that envelope (with a mild preference for tempos near 120 BPM), folds it
//!    into the DJ range and refines it with a phase-aligned comb, which also
//!    gives the position of the first beat.

use crate::values::{Bpm, Seconds};

/// Hop between two envelope values, in seconds (5 ms).
const HOP_SECONDS: f64 = 0.005;

/// Cutoff of the low band emphasised by the onset detector, in Hz.
const LOW_BAND_HZ: f32 = 150.0;

/// Tempos considered by the coarse search.
const SEARCH_MIN_BPM: f64 = 60.0;
const SEARCH_MAX_BPM: f64 = 200.0;

/// Results are folded into `[FOLD_MIN_BPM, 2 × FOLD_MIN_BPM)`: a track at
/// 174 BPM is reported as 87, one at 70 as 140, as most DJ software does.
pub const FOLD_MIN_BPM: f64 = 80.0;

/// Shorter material has too few beats for a reliable estimate.
const MIN_SECONDS: f64 = 6.0;

/// Centre of the tempo preference, and its width in octaves.
const PRIOR_CENTRE_BPM: f64 = 120.0;
const PRIOR_WIDTH_OCTAVES: f64 = 0.9;

/// Refinement window around the coarse tempo (± fraction) and its step.
const REFINE_SPAN: f64 = 0.03;
const REFINE_STEP_BPM: f64 = 0.01;

/// Onset strength sampled at a fixed rate.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct OnsetEnvelope {
    rate: f64,
    values: Vec<f32>,
}

impl OnsetEnvelope {
    /// `rate` is the number of values per second.
    pub fn new(rate: f64, values: Vec<f32>) -> Self {
        Self { rate, values }
    }

    pub fn rate(&self) -> f64 {
        self.rate
    }

    pub fn values(&self) -> &[f32] {
        &self.values
    }

    /// Seconds covered by the envelope.
    pub fn duration(&self) -> f64 {
        if self.rate > 0.0 {
            // Envelope lengths fit in f64 without loss.
            #[allow(clippy::cast_precision_loss)]
            let len = self.values.len() as f64;
            len / self.rate
        } else {
            0.0
        }
    }
}

/// Tempo of a track and position of its first beat.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BeatGrid {
    pub bpm: Bpm,
    /// Time of the first beat, within the first beat period.
    pub first_beat: Seconds,
}

/// Builds an [`OnsetEnvelope`] from mono samples, one sample at a time.
#[derive(Debug, Clone)]
pub struct OnsetDetector {
    hop_frames: usize,
    rate: f64,
    low_coeff: f32,
    low_state: f32,
    in_hop: usize,
    low_energy: f32,
    full_energy: f32,
    previous: Option<(f32, f32)>,
    values: Vec<f32>,
}

impl OnsetDetector {
    pub fn new(sample_rate: u32) -> Self {
        let sample_rate = sample_rate.max(1);
        // Hops are a few hundred frames.
        #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
        let hop_frames = ((f64::from(sample_rate) * HOP_SECONDS).round() as usize).max(1);
        // Precision loss is irrelevant for a filter coefficient.
        #[allow(clippy::cast_precision_loss)]
        let low_coeff = 1.0 - (-std::f32::consts::TAU * LOW_BAND_HZ / sample_rate as f32).exp();
        #[allow(clippy::cast_precision_loss)]
        let rate = f64::from(sample_rate) / hop_frames as f64;
        Self {
            hop_frames,
            rate,
            low_coeff,
            low_state: 0.0,
            in_hop: 0,
            low_energy: 0.0,
            full_energy: 0.0,
            previous: None,
            values: Vec::new(),
        }
    }

    /// Feeds one mono sample.
    #[inline]
    pub fn push(&mut self, sample: f32) {
        self.low_state += self.low_coeff * (sample - self.low_state);
        self.low_energy += self.low_state * self.low_state;
        self.full_energy += sample * sample;
        self.in_hop += 1;
        if self.in_hop == self.hop_frames {
            self.close_hop();
        }
    }

    fn close_hop(&mut self) {
        // Compressed energies: rises matter relative to the level around them.
        let compress = |energy: f32| (1.0 + 1_000.0 * energy).ln();
        let current = (compress(self.low_energy), compress(self.full_energy));
        let onset = match self.previous {
            Some((low, full)) => (current.0 - low).max(0.0) + 0.5 * (current.1 - full).max(0.0),
            None => 0.0,
        };
        self.values.push(onset);
        self.previous = Some(current);
        self.in_hop = 0;
        self.low_energy = 0.0;
        self.full_energy = 0.0;
    }

    pub fn finish(self) -> OnsetEnvelope {
        OnsetEnvelope::new(self.rate, self.values)
    }
}

/// Estimates the tempo and the first beat, or `None` when the material is too
/// short or has no rhythm to lock onto.
pub fn estimate_beat_grid(envelope: &OnsetEnvelope) -> Option<BeatGrid> {
    let rate = envelope.rate();
    if rate <= 0.0 || envelope.duration() < MIN_SECONDS {
        return None;
    }
    let onsets = smooth(&normalise(envelope.values(), rate), rate);
    if onsets.iter().all(|value| *value <= 0.0) {
        return None;
    }
    let coarse = coarse_tempo(&onsets, rate)?;
    let folded = fold(coarse);
    let (bpm, phase) = refine(&onsets, rate, folded);
    // Tempos are within 80..160: no truncation.
    #[allow(clippy::cast_possible_truncation)]
    let bpm = Bpm::new(round_to(bpm, 0.01) as f32).ok()?;
    Some(BeatGrid {
        bpm,
        first_beat: Seconds::saturating(phase / rate),
    })
}

/// Removes the slowly varying part of the envelope (moving average over
/// ~0.4 s) and keeps the peaks, scaled to a unit maximum.
fn normalise(values: &[f32], rate: f64) -> Vec<f32> {
    // Window lengths are small positive integers.
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    let half = ((0.2 * rate) as usize).max(1);
    let mut prefix = Vec::with_capacity(values.len() + 1);
    prefix.push(0.0f64);
    for value in values {
        let last = prefix.last().copied().unwrap_or(0.0);
        prefix.push(last + f64::from(*value));
    }
    let mut out: Vec<f32> = values
        .iter()
        .enumerate()
        .map(|(index, value)| {
            let start = index.saturating_sub(half);
            let end = (index + half + 1).min(values.len());
            // Window lengths are small.
            #[allow(clippy::cast_precision_loss)]
            let mean = (prefix[end] - prefix[start]) / (end - start) as f64;
            #[allow(clippy::cast_possible_truncation)]
            let detrended = (f64::from(*value) - mean) as f32;
            detrended.max(0.0)
        })
        .collect();
    let max = out.iter().copied().fold(0.0f32, f32::max);
    if max > 0.0 {
        for value in &mut out {
            *value /= max;
        }
    }
    out
}

/// Best tempo of the autocorrelation, weighted by the tempo preference.
fn coarse_tempo(onsets: &[f32], rate: f64) -> Option<f64> {
    // Lags are positive and far below the envelope length.
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    let min_lag = ((60.0 * rate / SEARCH_MAX_BPM).floor() as usize).max(1);
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    let max_lag = (60.0 * rate / SEARCH_MIN_BPM).ceil() as usize;
    if max_lag + 2 >= onsets.len() {
        return None;
    }
    let correlation: Vec<f64> = (0..=max_lag + 1)
        .map(|lag| {
            let sum: f64 = onsets
                .iter()
                .zip(&onsets[lag..])
                .map(|(a, b)| f64::from(*a) * f64::from(*b))
                .sum();
            // Envelope lengths fit in f64.
            #[allow(clippy::cast_precision_loss)]
            let count = (onsets.len() - lag) as f64;
            sum / count
        })
        .collect();
    let score = |lag: usize| {
        // Lags are small.
        #[allow(clippy::cast_precision_loss)]
        let bpm = 60.0 * rate / lag as f64;
        correlation[lag] * prior(bpm)
    };
    let best = (min_lag..=max_lag).max_by(|a, b| score(*a).total_cmp(&score(*b)))?;
    if correlation[best] <= 0.0 {
        return None;
    }
    // Parabolic interpolation around the peak for a sub-hop lag.
    let (left, centre, right) = (
        correlation[best - 1],
        correlation[best],
        correlation[best + 1],
    );
    let denominator = left - 2.0 * centre + right;
    let offset = if denominator.abs() > f64::EPSILON {
        (0.5 * (left - right) / denominator).clamp(-0.5, 0.5)
    } else {
        0.0
    };
    // Lags are small.
    #[allow(clippy::cast_precision_loss)]
    let lag = best as f64 + offset;
    Some(60.0 * rate / lag)
}

/// Mild preference for tempos around 120 BPM (log-normal in octaves).
fn prior(bpm: f64) -> f64 {
    let octaves = (bpm / PRIOR_CENTRE_BPM).log2() / PRIOR_WIDTH_OCTAVES;
    (-0.5 * octaves * octaves).exp()
}

/// Brings a tempo into `[FOLD_MIN_BPM, 2 × FOLD_MIN_BPM)` by octaves.
fn fold(mut bpm: f64) -> f64 {
    while bpm < FOLD_MIN_BPM {
        bpm *= 2.0;
    }
    while bpm >= 2.0 * FOLD_MIN_BPM {
        bpm /= 2.0;
    }
    bpm
}

/// Scans tempos around `around` with a comb aligned on every phase; returns
/// the best tempo and its phase (in envelope samples).
fn refine(onsets: &[f32], rate: f64, around: f64) -> (f64, f64) {
    let low = (around * (1.0 - REFINE_SPAN)).max(FOLD_MIN_BPM * 0.97);
    let high = around * (1.0 + REFINE_SPAN);
    // Step counts are small.
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    let steps = ((high - low) / REFINE_STEP_BPM).round() as usize;
    let mut best = (around, 0.0, f64::MIN);
    for step in 0..=steps {
        // Step counts are small.
        #[allow(clippy::cast_precision_loss)]
        let bpm = low + step as f64 * REFINE_STEP_BPM;
        let period = 60.0 * rate / bpm;
        let (phase, score) = best_phase(onsets, period);
        if score > best.2 {
            best = (bpm, phase, score);
        }
    }
    (best.0, best.1)
}

/// Phase (in samples, within one period) whose comb collects the most onset
/// strength, and that strength per beat.
fn best_phase(onsets: &[f32], period: f64) -> (f64, f64) {
    // Envelope lengths fit in f64.
    #[allow(clippy::cast_precision_loss)]
    let len = onsets.len() as f64;
    let mut best = (0.0, f64::MIN);
    // Phases are scanned every envelope sample.
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    let phases = period.ceil() as usize;
    for phase_index in 0..phases {
        // Small integers.
        #[allow(clippy::cast_precision_loss)]
        let phase = phase_index as f64;
        let mut sum = 0.0;
        let mut beats = 0u32;
        let mut position = phase;
        while position < len - 1.0 {
            sum += interpolate(onsets, position);
            beats += 1;
            position += period;
        }
        if beats > 0 {
            let score = sum / f64::from(beats);
            if score > best.1 {
                best = (phase, score);
            }
        }
    }
    best
}

/// Linear interpolation of the envelope at a fractional index.
fn interpolate(onsets: &[f32], position: f64) -> f64 {
    // `position` is within bounds and non-negative.
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    let index = position as usize;
    let next = (index + 1).min(onsets.len() - 1);
    // Small integers.
    #[allow(clippy::cast_precision_loss)]
    let fraction = position - index as f64;
    f64::from(onsets[index]) * (1.0 - fraction) + f64::from(onsets[next]) * fraction
}

/// Gaussian smoothing (σ ≈ 8 ms) so that onset peaks have a slope the comb
/// can climb, which makes the tempo refinement precise.
fn smooth(values: &[f32], rate: f64) -> Vec<f32> {
    let sigma = (0.008 * rate).max(0.5);
    // Kernel radius: a few samples.
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    let radius = (3.0 * sigma).ceil() as usize;
    let kernel: Vec<f32> = (0..=2 * radius)
        .map(|i| {
            // Small integers.
            #[allow(clippy::cast_precision_loss)]
            let x = i as f64 - radius as f64;
            #[allow(clippy::cast_possible_truncation)]
            let weight = (-0.5 * (x / sigma).powi(2)).exp() as f32;
            weight
        })
        .collect();
    let total: f32 = kernel.iter().sum();
    (0..values.len())
        .map(|centre| {
            let mut sum = 0.0;
            for (k, weight) in kernel.iter().enumerate() {
                let index = (centre + k).checked_sub(radius);
                if let Some(value) = index.and_then(|i| values.get(i)) {
                    sum += value * weight;
                }
            }
            sum / total
        })
        .collect()
}

fn round_to(value: f64, step: f64) -> f64 {
    (value / step).round() * step
}

#[cfg(test)]
#[allow(
    clippy::cast_possible_truncation,
    clippy::cast_precision_loss,
    clippy::cast_sign_loss
)]
mod tests {
    use super::*;

    const RATE: u32 = 22_050;

    /// Deterministic noise in `[-1, 1]`.
    struct Noise(u32);

    impl Noise {
        fn next(&mut self) -> f32 {
            self.0 = self.0.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
            #[allow(clippy::cast_precision_loss)]
            let unit = (self.0 >> 8) as f32 / (1u32 << 24) as f32;
            unit * 2.0 - 1.0
        }
    }

    /// A drum loop: kick on every beat, hi-hat on the off-beats, quiet noise.
    fn drum_loop(bpm: f64, seconds: f64, offset: f64) -> Vec<f32> {
        let rate = f64::from(RATE);
        #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
        let frames = (seconds * rate) as usize;
        let beat = 60.0 / bpm;
        let mut noise = Noise(7);
        (0..frames)
            .map(|n| {
                #[allow(clippy::cast_precision_loss)]
                let t = n as f64 / rate;
                let mut sample = 0.02 * noise.next();
                if t >= offset {
                    let in_beat = (t - offset) % beat;
                    // Kick: decaying 60 Hz sine.
                    let kick =
                        (-in_beat * 30.0).exp() * (std::f64::consts::TAU * 60.0 * in_beat).sin();
                    sample += 0.8 * kick as f32;
                    // Hi-hat: short noise burst half a beat later.
                    let off = (in_beat - beat / 2.0).abs();
                    if off < 0.02 {
                        sample += 0.3 * noise.next();
                    }
                }
                sample
            })
            .collect()
    }

    fn analyse(samples: &[f32]) -> Option<BeatGrid> {
        let mut detector = OnsetDetector::new(RATE);
        for sample in samples {
            detector.push(*sample);
        }
        estimate_beat_grid(&detector.finish())
    }

    #[test]
    fn finds_common_dance_tempos() {
        for bpm in [85.0, 95.0, 100.0, 112.0, 118.5, 124.0, 128.0, 140.0, 150.0] {
            let grid = analyse(&drum_loop(bpm, 30.0, 0.0)).expect("a tempo");
            let found = f64::from(grid.bpm.value());
            assert!((found - bpm).abs() < 0.05, "{bpm} BPM detected as {found}");
        }
    }

    #[test]
    fn fast_and_slow_tempos_fold_into_the_dj_range() {
        let grid = analyse(&drum_loop(174.0, 30.0, 0.0)).unwrap();
        assert!((f64::from(grid.bpm.value()) - 87.0).abs() < 0.15);
        let grid = analyse(&drum_loop(70.0, 30.0, 0.0)).unwrap();
        assert!((f64::from(grid.bpm.value()) - 140.0).abs() < 0.15);
    }

    #[test]
    fn locates_the_first_beat() {
        let bpm = 120.0;
        let offset = 0.23;
        let grid = analyse(&drum_loop(bpm, 30.0, offset)).unwrap();
        let beat = 60.0 / bpm;
        let error = (grid.first_beat.as_f64() - offset).rem_euclid(beat);
        let error = error.min(beat - error);
        assert!(error < 0.02, "first beat at {:?}", grid.first_beat);
    }

    #[test]
    fn silence_and_short_material_have_no_tempo() {
        assert_eq!(analyse(&vec![0.0; RATE as usize * 20]), None);
        assert_eq!(analyse(&drum_loop(120.0, 3.0, 0.0)), None);
        assert_eq!(estimate_beat_grid(&OnsetEnvelope::default()), None);
    }

    #[test]
    fn folding_and_prior_behave() {
        assert_eq!(fold(174.0), 87.0);
        assert_eq!(fold(40.0), 80.0);
        assert_eq!(fold(159.0), 159.0);
        assert!(prior(120.0) > prior(60.0));
        assert!(prior(120.0) > prior(240.0));
    }
}
