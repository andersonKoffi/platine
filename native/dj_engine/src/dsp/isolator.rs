//! Three-band DJ isolator EQ built from Linkwitz-Riley (LR4) crossovers.
//!
//! The signal is split into low, mid and high bands whose sum is an all-pass
//! filter: with every band at unity the EQ is flat, and setting a band to 0
//! removes it completely ("kill").

use super::biquad::{BUTTERWORTH_Q, Biquad, Coefficients};

/// Crossover between low and mid bands, in Hz.
pub const LOW_MID_HZ: f64 = 250.0;
/// Crossover between mid and high bands, in Hz.
pub const MID_HIGH_HZ: f64 = 2_500.0;

/// Fourth-order Linkwitz-Riley section: two identical Butterworth biquads.
#[derive(Debug, Clone, Copy, PartialEq)]
struct Lr4 {
    first: Biquad,
    second: Biquad,
}

impl Lr4 {
    fn new(coefficients: Coefficients) -> Self {
        Self {
            first: Biquad::new(coefficients),
            second: Biquad::new(coefficients),
        }
    }

    #[inline]
    fn process(&mut self, x: f32) -> f32 {
        self.second.process(self.first.process(x))
    }

    fn reset(&mut self) {
        self.first.reset();
        self.second.reset();
    }
}

/// Band split of one channel.
#[derive(Debug, Clone, Copy, PartialEq)]
struct ChannelSplitter {
    low_1: Lr4,
    rest_1: Lr4,
    mid_2: Lr4,
    high_2: Lr4,
    /// Aligns the low band's phase with the second crossover.
    low_phase: Biquad,
}

impl ChannelSplitter {
    fn new(sample_rate: f64) -> Self {
        Self {
            low_1: Lr4::new(Coefficients::low_pass(
                LOW_MID_HZ,
                sample_rate,
                BUTTERWORTH_Q,
            )),
            rest_1: Lr4::new(Coefficients::high_pass(
                LOW_MID_HZ,
                sample_rate,
                BUTTERWORTH_Q,
            )),
            mid_2: Lr4::new(Coefficients::low_pass(
                MID_HIGH_HZ,
                sample_rate,
                BUTTERWORTH_Q,
            )),
            high_2: Lr4::new(Coefficients::high_pass(
                MID_HIGH_HZ,
                sample_rate,
                BUTTERWORTH_Q,
            )),
            low_phase: Biquad::new(Coefficients::all_pass(
                MID_HIGH_HZ,
                sample_rate,
                BUTTERWORTH_Q,
            )),
        }
    }

    #[inline]
    fn process(&mut self, x: f32, gains: [f32; 3]) -> f32 {
        let low = self.low_phase.process(self.low_1.process(x));
        let rest = self.rest_1.process(x);
        let mid = self.mid_2.process(rest);
        let high = self.high_2.process(rest);
        low * gains[0] + mid * gains[1] + high * gains[2]
    }

    fn reset(&mut self) {
        self.low_1.reset();
        self.rest_1.reset();
        self.mid_2.reset();
        self.high_2.reset();
        self.low_phase.reset();
    }
}

/// Stereo three-band isolator.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Isolator {
    channels: [ChannelSplitter; 2],
}

impl Isolator {
    pub fn new(sample_rate: u32) -> Self {
        let splitter = ChannelSplitter::new(f64::from(sample_rate));
        Self {
            channels: [splitter, splitter],
        }
    }

    /// Filters one stereo frame with linear band gains `[low, mid, high]`.
    #[inline]
    pub fn process(&mut self, frame: [f32; 2], gains: [f32; 3]) -> [f32; 2] {
        [
            self.channels[0].process(frame[0], gains),
            self.channels[1].process(frame[1], gains),
        ]
    }

    pub fn reset(&mut self) {
        for channel in &mut self.channels {
            channel.reset();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dsp::biquad::tests::gain_at;

    const RATE: u32 = 48_000;

    fn response(frequency: f64, gains: [f32; 3]) -> f32 {
        let mut eq = Isolator::new(RATE);
        gain_at(|x| eq.process([x, x], gains)[0], frequency, f64::from(RATE))
    }

    #[test]
    fn unity_gains_are_flat_across_the_spectrum() {
        for frequency in [40.0, 250.0, 800.0, 2_500.0, 6_000.0, 15_000.0] {
            let gain = response(frequency, [1.0, 1.0, 1.0]);
            assert!((gain - 1.0).abs() < 0.02, "{frequency} Hz: {gain}");
        }
    }

    #[test]
    fn killing_a_band_removes_it() {
        assert!(response(50.0, [0.0, 1.0, 1.0]) < 0.02);
        assert!(response(900.0, [1.0, 0.0, 1.0]) < 0.12);
        assert!(response(12_000.0, [1.0, 1.0, 0.0]) < 0.02);
        // Other bands are untouched.
        assert!((response(12_000.0, [0.0, 1.0, 1.0]) - 1.0).abs() < 0.02);
    }

    #[test]
    fn boosting_a_band_raises_its_level() {
        let boosted = response(60.0, [2.0, 1.0, 1.0]);
        assert!((boosted - 2.0).abs() < 0.05, "{boosted}");
    }

    #[test]
    fn reset_clears_filter_memory() {
        let mut eq = Isolator::new(RATE);
        eq.process([1.0, 1.0], [1.0, 1.0, 1.0]);
        eq.reset();
        assert_eq!(eq, Isolator::new(RATE));
    }
}
