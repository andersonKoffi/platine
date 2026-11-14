//! Second-order IIR filters (RBJ "Audio EQ Cookbook" designs).

use std::f64::consts::PI;

/// Normalised biquad coefficients (`a0 == 1`).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Coefficients {
    b0: f32,
    b1: f32,
    b2: f32,
    a1: f32,
    a2: f32,
}

/// Butterworth quality factor; two cascaded sections form a Linkwitz-Riley filter.
pub const BUTTERWORTH_Q: f64 = std::f64::consts::FRAC_1_SQRT_2;

impl Coefficients {
    pub const IDENTITY: Self = Self {
        b0: 1.0,
        b1: 0.0,
        b2: 0.0,
        a1: 0.0,
        a2: 0.0,
    };

    fn from_raw(b: [f64; 3], a: [f64; 3]) -> Self {
        let a0 = a[0];
        Self {
            b0: (b[0] / a0) as f32,
            b1: (b[1] / a0) as f32,
            b2: (b[2] / a0) as f32,
            a1: (a[1] / a0) as f32,
            a2: (a[2] / a0) as f32,
        }
    }

    fn omega(frequency: f64, sample_rate: f64, q: f64) -> (f64, f64) {
        let w0 = 2.0 * PI * frequency / sample_rate;
        (w0.cos(), w0.sin() / (2.0 * q))
    }

    pub fn low_pass(frequency: f64, sample_rate: f64, q: f64) -> Self {
        let (cos, alpha) = Self::omega(frequency, sample_rate, q);
        let b1 = 1.0 - cos;
        Self::from_raw(
            [b1 / 2.0, b1, b1 / 2.0],
            [1.0 + alpha, -2.0 * cos, 1.0 - alpha],
        )
    }

    pub fn high_pass(frequency: f64, sample_rate: f64, q: f64) -> Self {
        let (cos, alpha) = Self::omega(frequency, sample_rate, q);
        let b1 = 1.0 + cos;
        Self::from_raw(
            [b1 / 2.0, -b1, b1 / 2.0],
            [1.0 + alpha, -2.0 * cos, 1.0 - alpha],
        )
    }

    pub fn all_pass(frequency: f64, sample_rate: f64, q: f64) -> Self {
        let (cos, alpha) = Self::omega(frequency, sample_rate, q);
        Self::from_raw(
            [1.0 - alpha, -2.0 * cos, 1.0 + alpha],
            [1.0 + alpha, -2.0 * cos, 1.0 - alpha],
        )
    }
}

/// One biquad section with its state (transposed direct form II).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Biquad {
    coefficients: Coefficients,
    z1: f32,
    z2: f32,
}

impl Biquad {
    pub fn new(coefficients: Coefficients) -> Self {
        Self {
            coefficients,
            z1: 0.0,
            z2: 0.0,
        }
    }

    #[inline]
    pub fn process(&mut self, x: f32) -> f32 {
        let c = &self.coefficients;
        let y = c.b0 * x + self.z1;
        self.z1 = c.b1 * x - c.a1 * y + self.z2;
        self.z2 = c.b2 * x - c.a2 * y;
        y
    }

    pub fn reset(&mut self) {
        self.z1 = 0.0;
        self.z2 = 0.0;
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;

    /// Steady-state amplitude of `filter` for a unit sine at `frequency`,
    /// measured by I/Q demodulation over the second half of one second.
    pub(crate) fn gain_at(mut filter: impl FnMut(f32) -> f32, frequency: f64, rate: f64) -> f32 {
        let total = rate as usize;
        let (mut i, mut q, mut count) = (0.0f64, 0.0f64, 0.0f64);
        for n in 0..total {
            let phase = 2.0 * PI * frequency * n as f64 / rate;
            let y = f64::from(filter(phase.sin() as f32));
            if n >= total / 2 {
                i += y * phase.sin();
                q += y * phase.cos();
                count += 1.0;
            }
        }
        (2.0 * (i * i + q * q).sqrt() / count) as f32
    }

    const RATE: f64 = 48_000.0;

    #[test]
    fn identity_passes_samples_through() {
        let mut f = Biquad::new(Coefficients::IDENTITY);
        assert_eq!(f.process(0.3), 0.3);
        assert_eq!(f.process(-0.7), -0.7);
    }

    #[test]
    fn low_pass_keeps_lows_and_cuts_highs() {
        let c = Coefficients::low_pass(1000.0, RATE, BUTTERWORTH_Q);
        let mut f = Biquad::new(c);
        assert!((gain_at(|x| f.process(x), 100.0, RATE) - 1.0).abs() < 0.01);
        let mut f = Biquad::new(c);
        assert!(gain_at(|x| f.process(x), 10_000.0, RATE) < 0.02);
        let mut f = Biquad::new(c);
        assert!((gain_at(|x| f.process(x), 1000.0, RATE) - 0.707).abs() < 0.01);
    }

    #[test]
    fn high_pass_keeps_highs_and_cuts_lows() {
        let c = Coefficients::high_pass(1000.0, RATE, BUTTERWORTH_Q);
        let mut f = Biquad::new(c);
        assert!(gain_at(|x| f.process(x), 50.0, RATE) < 0.01);
        let mut f = Biquad::new(c);
        assert!((gain_at(|x| f.process(x), 10_000.0, RATE) - 1.0).abs() < 0.01);
    }

    #[test]
    fn all_pass_keeps_every_frequency() {
        let c = Coefficients::all_pass(2000.0, RATE, BUTTERWORTH_Q);
        for frequency in [60.0, 2000.0, 12_000.0] {
            let mut f = Biquad::new(c);
            assert!((gain_at(|x| f.process(x), frequency, RATE) - 1.0).abs() < 0.01);
        }
    }

    #[test]
    fn reset_clears_the_state() {
        let mut f = Biquad::new(Coefficients::low_pass(100.0, RATE, BUTTERWORTH_Q));
        f.process(1.0);
        f.reset();
        let mut fresh = Biquad::new(Coefficients::low_pass(100.0, RATE, BUTTERWORTH_Q));
        assert_eq!(f.process(0.5), fresh.process(0.5));
    }
}
