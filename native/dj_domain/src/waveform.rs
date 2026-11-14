//! Coloured waveform summary of a track: the peak of three frequency bands
//! (low, mid, high) for each short window, normalised to `0..=255`.

/// Waveform of a whole track, one point per analysis window.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Waveform {
    points_per_second: u16,
    low: Vec<u8>,
    mid: Vec<u8>,
    high: Vec<u8>,
}

/// Exponent applied after normalisation so that quiet passages stay visible.
const DISPLAY_GAMMA: f32 = 0.7;

impl Waveform {
    /// Builds a waveform from raw band peaks (`[low, mid, high]` per window).
    /// The loudest band of the track is scaled to 255.
    pub fn from_peaks(points_per_second: u16, peaks: &[[f32; 3]]) -> Self {
        let max = peaks
            .iter()
            .flat_map(|bands| bands.iter().copied())
            .filter(|value| value.is_finite())
            .fold(0.0f32, f32::max);
        let quantise = |value: f32| -> u8 {
            if max <= 0.0 || !value.is_finite() {
                return 0;
            }
            let level = (value.abs() / max).clamp(0.0, 1.0).powf(DISPLAY_GAMMA);
            // `level` is within 0..=1, so the product fits in a u8.
            #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
            let byte = (level * 255.0).round() as u8;
            byte
        };
        let band = |index: usize| peaks.iter().map(|bands| quantise(bands[index])).collect();
        Self {
            points_per_second: points_per_second.max(1),
            low: band(0),
            mid: band(1),
            high: band(2),
        }
    }

    pub fn points_per_second(&self) -> u16 {
        self.points_per_second
    }

    pub fn low(&self) -> &[u8] {
        &self.low
    }

    pub fn mid(&self) -> &[u8] {
        &self.mid
    }

    pub fn high(&self) -> &[u8] {
        &self.high
    }

    pub fn len(&self) -> usize {
        self.low.len()
    }

    pub fn is_empty(&self) -> bool {
        self.low.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::Waveform;

    #[test]
    fn loudest_band_reaches_full_scale() {
        let waveform = Waveform::from_peaks(100, &[[0.5, 0.25, 0.0], [0.1, 0.0, 0.05]]);
        assert_eq!(waveform.len(), 2);
        assert_eq!(waveform.low()[0], 255);
        assert!(waveform.mid()[0] > 0 && waveform.mid()[0] < 255);
        assert_eq!(waveform.high()[0], 0);
        assert!(waveform.high()[1] > 0);
        assert_eq!(waveform.points_per_second(), 100);
    }

    #[test]
    fn silence_and_empty_input_stay_flat() {
        let silent = Waveform::from_peaks(100, &[[0.0; 3], [f32::NAN, 0.0, 0.0]]);
        assert!(silent.low().iter().all(|value| *value == 0));
        let empty = Waveform::from_peaks(0, &[]);
        assert!(empty.is_empty());
        assert_eq!(empty.points_per_second(), 1);
    }
}
