//! Value objects: validated at construction, immutable afterwards.

use std::cmp::Ordering;
use std::fmt;
use std::ops::{Add, Sub};

use crate::error::{DomainError, check_range};

/// A non-negative, finite duration or position, in seconds.
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd, Default)]
pub struct Seconds(f64);

impl Seconds {
    pub const ZERO: Self = Self(0.0);

    /// Validates and wraps a number of seconds.
    ///
    /// # Errors
    /// Returns an error if `value` is negative, NaN or infinite.
    pub fn new(value: f64) -> Result<Self, DomainError> {
        check_range("seconds", value, 0.0, f64::MAX).map(Self)
    }

    /// Builds a value from a constant known to be valid; clamps anything
    /// invalid to zero. Intended for literals and test fixtures.
    pub fn saturating(value: f64) -> Self {
        Self::new(value).unwrap_or(Self::ZERO)
    }

    pub fn as_f64(self) -> f64 {
        self.0
    }

    /// `self - other`, floored at zero.
    #[must_use]
    pub fn saturating_sub(self, other: Self) -> Self {
        Self((self.0 - other.0).max(0.0))
    }

    #[must_use]
    pub fn half(self) -> Self {
        Self(self.0 / 2.0)
    }

    #[must_use]
    pub fn min(self, other: Self) -> Self {
        if other.0 < self.0 { other } else { self }
    }

    #[must_use]
    pub fn max(self, other: Self) -> Self {
        if other.0 > self.0 { other } else { self }
    }

    /// Total order (values are always finite).
    pub fn total_cmp(&self, other: &Self) -> Ordering {
        self.0.total_cmp(&other.0)
    }
}

impl Add for Seconds {
    type Output = Self;
    fn add(self, rhs: Self) -> Self {
        Self(self.0 + rhs.0)
    }
}

impl Sub for Seconds {
    type Output = Self;
    /// Saturating subtraction: durations never become negative.
    fn sub(self, rhs: Self) -> Self {
        self.saturating_sub(rhs)
    }
}

impl fmt::Display for Seconds {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:.3}s", self.0)
    }
}

/// Duration of the automatic crossfade, between 0 and 30 seconds.
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd)]
pub struct CrossfadeDuration(Seconds);

impl CrossfadeDuration {
    pub const MAX_SECONDS: f64 = 30.0;
    pub const DEFAULT_SECONDS: f64 = 8.0;
    /// Presets offered by the UI.
    pub const PRESETS_SECONDS: [f64; 5] = [0.0, 4.0, 8.0, 12.0, 16.0];

    /// # Errors
    /// Returns an error if `seconds` is not within `[0, 30]`.
    pub fn new(seconds: f64) -> Result<Self, DomainError> {
        check_range("crossfade duration", seconds, 0.0, Self::MAX_SECONDS)
            .map(|value| Self(Seconds(value)))
    }

    pub fn seconds(self) -> Seconds {
        self.0
    }

    pub fn is_instant(self) -> bool {
        self.0 == Seconds::ZERO
    }
}

impl Default for CrossfadeDuration {
    fn default() -> Self {
        Self(Seconds(Self::DEFAULT_SECONDS))
    }
}

/// Linear amplitude gain, from silence (0) to +12 dB (~3.98).
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd)]
pub struct Gain(f32);

impl Gain {
    pub const SILENCE: Self = Self(0.0);
    pub const UNITY: Self = Self(1.0);
    pub const MAX_DB: f32 = 12.0;

    /// # Errors
    /// Returns an error if `linear` is not within `[0, +12 dB]`.
    pub fn new(linear: f32) -> Result<Self, DomainError> {
        // Small tolerance so that `from_db(MAX_DB)` survives f32 rounding.
        let max = 10f64.powf(f64::from(Self::MAX_DB) / 20.0) + 1e-4;
        #[allow(clippy::cast_possible_truncation)]
        check_range("gain", f64::from(linear), 0.0, max).map(|v| Self(v as f32))
    }

    /// Builds a gain from decibels; `-inf` dB (or anything below -120 dB) is silence.
    ///
    /// # Errors
    /// Returns an error above +12 dB or for NaN.
    pub fn from_db(db: f32) -> Result<Self, DomainError> {
        if db.is_nan() {
            return Err(DomainError::NotFinite {
                what: "gain (dB)",
                value: f64::from(db),
            });
        }
        if db <= -120.0 {
            return Ok(Self::SILENCE);
        }
        Self::new(10f32.powf(db / 20.0))
    }

    pub fn linear(self) -> f32 {
        self.0
    }

    /// Gain in dB; silence maps to negative infinity.
    pub fn db(self) -> f32 {
        20.0 * self.0.log10()
    }
}

impl Default for Gain {
    fn default() -> Self {
        Self::UNITY
    }
}

/// Tempo in beats per minute, within a musically meaningful range.
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd)]
pub struct Bpm(f32);

impl Bpm {
    pub const MIN: f32 = 30.0;
    pub const MAX: f32 = 300.0;

    /// # Errors
    /// Returns an error if `value` is not within `[30, 300]`.
    pub fn new(value: f32) -> Result<Self, DomainError> {
        #[allow(clippy::cast_possible_truncation)]
        check_range(
            "bpm",
            f64::from(value),
            f64::from(Self::MIN),
            f64::from(Self::MAX),
        )
        .map(|v| Self(v as f32))
    }

    pub fn value(self) -> f32 {
        self.0
    }

    /// Duration of one beat.
    pub fn beat_period(self) -> Seconds {
        Seconds(60.0 / f64::from(self.0))
    }

    /// Tempo heard when the track plays at `pitch`.
    #[must_use]
    pub fn at_pitch(self, pitch: PitchPercent) -> f32 {
        self.0 * pitch.ratio()
    }
}

/// Range of the pitch fader.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum PitchRange {
    #[default]
    Eight,
    Sixteen,
    Fifty,
}

impl PitchRange {
    pub fn max_percent(self) -> f32 {
        match self {
            Self::Eight => 8.0,
            Self::Sixteen => 16.0,
            Self::Fifty => 50.0,
        }
    }
}

/// Playback speed offset, in percent (`+8` plays 8 % faster).
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd, Default)]
pub struct PitchPercent(f32);

impl PitchPercent {
    pub const ZERO: Self = Self(0.0);

    /// # Errors
    /// Returns an error if `percent` exceeds the ±50 % hardware range.
    pub fn new(percent: f32) -> Result<Self, DomainError> {
        #[allow(clippy::cast_possible_truncation)]
        check_range("pitch", f64::from(percent), -50.0, 50.0).map(|v| Self(v as f32))
    }

    /// Clamps `percent` into the given fader range.
    pub fn clamped(percent: f32, range: PitchRange) -> Self {
        let max = range.max_percent();
        if percent.is_nan() {
            return Self::ZERO;
        }
        Self(percent.clamp(-max, max))
    }

    pub fn percent(self) -> f32 {
        self.0
    }

    /// Playback-rate multiplier (`1.08` for +8 %).
    pub fn ratio(self) -> f32 {
        1.0 + self.0 / 100.0
    }
}

/// One of the two decks.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum DeckId {
    A,
    B,
}

impl DeckId {
    pub const ALL: [Self; 2] = [Self::A, Self::B];

    #[must_use]
    pub fn other(self) -> Self {
        match self {
            Self::A => Self::B,
            Self::B => Self::A,
        }
    }

    /// Index usable for `[T; 2]` storage.
    pub fn index(self) -> usize {
        match self {
            Self::A => 0,
            Self::B => 1,
        }
    }
}

impl fmt::Display for DeckId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::A => "A",
            Self::B => "B",
        })
    }
}

/// Crossfader position: `0.0` is fully deck A, `1.0` fully deck B.
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd)]
pub struct CrossfaderPosition(f32);

impl CrossfaderPosition {
    pub const CENTER: Self = Self(0.5);

    /// # Errors
    /// Returns an error if `value` is not within `[0, 1]`.
    pub fn new(value: f32) -> Result<Self, DomainError> {
        #[allow(clippy::cast_possible_truncation)]
        check_range("crossfader position", f64::from(value), 0.0, 1.0).map(|v| Self(v as f32))
    }

    /// The position where only `deck` is heard.
    pub fn fully(deck: DeckId) -> Self {
        match deck {
            DeckId::A => Self(0.0),
            DeckId::B => Self(1.0),
        }
    }

    pub fn value(self) -> f32 {
        self.0
    }

    /// Progress from `from`'s side towards the other side, in `[0, 1]`.
    pub fn progress_away_from(self, from: DeckId) -> f32 {
        match from {
            DeckId::A => self.0,
            DeckId::B => 1.0 - self.0,
        }
    }
}

impl Default for CrossfaderPosition {
    fn default() -> Self {
        Self::CENTER
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    mod seconds {
        use super::*;

        #[test]
        fn rejects_negative_and_non_finite_values() {
            assert!(Seconds::new(-0.1).is_err());
            assert!(Seconds::new(f64::NAN).is_err());
            assert!(Seconds::new(f64::INFINITY).is_err());
            assert_eq!(Seconds::new(1.5).map(Seconds::as_f64), Ok(1.5));
        }

        #[test]
        fn saturating_constructor_clamps_invalid_values_to_zero() {
            assert_eq!(Seconds::saturating(-3.0), Seconds::ZERO);
            assert_eq!(Seconds::saturating(2.0).as_f64(), 2.0);
        }

        #[test]
        fn arithmetic_never_goes_negative() {
            let a = Seconds::saturating(3.0);
            let b = Seconds::saturating(5.0);
            assert_eq!((a + b).as_f64(), 8.0);
            assert_eq!(a - b, Seconds::ZERO);
            assert_eq!((b - a).as_f64(), 2.0);
            assert_eq!(b.half().as_f64(), 2.5);
        }

        #[test]
        fn min_max_and_ordering() {
            let a = Seconds::saturating(1.0);
            let b = Seconds::saturating(2.0);
            assert_eq!(a.min(b), a);
            assert_eq!(b.min(a), a);
            assert_eq!(a.max(b), b);
            assert_eq!(b.max(a), b);
            assert_eq!(a.total_cmp(&b), Ordering::Less);
            assert_eq!(a.to_string(), "1.000s");
        }
    }

    mod crossfade_duration {
        use super::*;

        #[test]
        fn defaults_to_eight_seconds() {
            assert_eq!(CrossfadeDuration::default().seconds().as_f64(), 8.0);
        }

        #[test]
        fn accepts_zero_to_thirty_seconds() {
            assert!(CrossfadeDuration::new(0.0).is_ok_and(CrossfadeDuration::is_instant));
            assert!(CrossfadeDuration::new(30.0).is_ok());
            assert!(CrossfadeDuration::new(30.1).is_err());
            assert!(CrossfadeDuration::new(-1.0).is_err());
        }

        #[test]
        fn every_preset_is_valid() {
            for preset in CrossfadeDuration::PRESETS_SECONDS {
                assert!(CrossfadeDuration::new(preset).is_ok());
            }
        }
    }

    mod gain {
        use super::*;

        #[test]
        fn converts_between_linear_and_decibels() {
            let gain = Gain::from_db(-6.0).unwrap();
            assert!((gain.linear() - 0.501).abs() < 1e-3);
            assert!((gain.db() + 6.0).abs() < 1e-4);
            assert_eq!(Gain::default(), Gain::UNITY);
        }

        #[test]
        fn very_low_decibels_mean_silence() {
            assert_eq!(Gain::from_db(-150.0), Ok(Gain::SILENCE));
            assert_eq!(Gain::SILENCE.db(), f32::NEG_INFINITY);
        }

        #[test]
        fn rejects_more_than_plus_twelve_db_and_nan() {
            assert!(Gain::from_db(12.0).is_ok());
            assert!(Gain::from_db(12.5).is_err());
            assert!(Gain::from_db(f32::NAN).is_err());
            assert!(Gain::new(-0.1).is_err());
        }
    }

    mod bpm {
        use super::*;

        #[test]
        fn validates_range_and_computes_beat_period() {
            assert!(Bpm::new(29.0).is_err());
            assert!(Bpm::new(301.0).is_err());
            let bpm = Bpm::new(120.0).unwrap();
            assert_eq!(bpm.value(), 120.0);
            assert_eq!(bpm.beat_period().as_f64(), 0.5);
        }

        #[test]
        fn follows_the_pitch() {
            let bpm = Bpm::new(100.0).unwrap();
            let pitch = PitchPercent::new(8.0).unwrap();
            assert!((bpm.at_pitch(pitch) - 108.0).abs() < 1e-4);
        }
    }

    mod pitch {
        use super::*;

        #[test]
        fn ratio_matches_percent() {
            assert_eq!(PitchPercent::ZERO.ratio(), 1.0);
            assert!((PitchPercent::new(-16.0).unwrap().ratio() - 0.84).abs() < 1e-6);
            assert!(PitchPercent::new(50.5).is_err());
        }

        #[test]
        fn clamps_to_the_selected_range() {
            assert_eq!(
                PitchPercent::clamped(12.0, PitchRange::Eight).percent(),
                8.0
            );
            assert_eq!(
                PitchPercent::clamped(-20.0, PitchRange::Sixteen).percent(),
                -16.0
            );
            assert_eq!(
                PitchPercent::clamped(40.0, PitchRange::Fifty).percent(),
                40.0
            );
            assert_eq!(
                PitchPercent::clamped(f32::NAN, PitchRange::Eight),
                PitchPercent::ZERO
            );
            assert_eq!(PitchRange::default(), PitchRange::Eight);
        }
    }

    mod deck_and_crossfader {
        use super::*;

        #[test]
        fn decks_know_their_counterpart_and_index() {
            assert_eq!(DeckId::A.other(), DeckId::B);
            assert_eq!(DeckId::B.other(), DeckId::A);
            assert_eq!(DeckId::A.index(), 0);
            assert_eq!(DeckId::B.index(), 1);
            assert_eq!(format!("{}{}", DeckId::A, DeckId::B), "AB");
        }

        #[test]
        fn crossfader_positions() {
            assert_eq!(CrossfaderPosition::fully(DeckId::A).value(), 0.0);
            assert_eq!(CrossfaderPosition::fully(DeckId::B).value(), 1.0);
            assert_eq!(CrossfaderPosition::default(), CrossfaderPosition::CENTER);
            assert!(CrossfaderPosition::new(1.2).is_err());
            let pos = CrossfaderPosition::new(0.25).unwrap();
            assert_eq!(pos.progress_away_from(DeckId::A), 0.25);
            assert_eq!(pos.progress_away_from(DeckId::B), 0.75);
        }
    }
}
