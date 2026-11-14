//! Crossfade curves (Strategy): how the crossfader position maps to deck gains.

use crate::track::TrackTiming;
use crate::values::{CrossfadeDuration, Seconds};

/// Shape of the crossfader.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum CrossfadeCurve {
    /// Constant perceived loudness: `cos² + sin² = 1`. Default for auto-mix.
    #[default]
    EqualPower,
    /// Gains vary linearly; there is a ~3 dB dip in the middle.
    Linear,
    /// Scratch curve: both decks at full volume except near the edges.
    Cut,
}

/// Linear gains of the outgoing and incoming sides.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CrossfadeGains {
    pub outgoing: f32,
    pub incoming: f32,
}

impl CrossfadeCurve {
    /// Width of the edge zone of the `Cut` curve, as a fraction of the travel.
    pub const CUT_EDGE: f32 = 0.05;

    /// Gains for `progress` in `[0, 1]` (0 = only the outgoing side is heard).
    /// Out-of-range progress is clamped.
    pub fn gains(self, progress: f32) -> CrossfadeGains {
        let x = if progress.is_nan() {
            0.0
        } else {
            progress.clamp(0.0, 1.0)
        };
        match self {
            Self::EqualPower => {
                let angle = x * std::f32::consts::FRAC_PI_2;
                // `cos(π/2)` is slightly negative in f32: clamp to stay in [0, 1].
                CrossfadeGains {
                    outgoing: angle.cos().clamp(0.0, 1.0),
                    incoming: angle.sin().clamp(0.0, 1.0),
                }
            }
            Self::Linear => CrossfadeGains {
                outgoing: 1.0 - x,
                incoming: x,
            },
            Self::Cut => CrossfadeGains {
                outgoing: ((1.0 - x) / Self::CUT_EDGE).min(1.0),
                incoming: (x / Self::CUT_EDGE).min(1.0),
            },
        }
    }
}

/// Fade length actually used between two tracks.
///
/// A track shorter than twice the configured fade gets a fade of half its
/// playable length, so that it is never faded in and out at the same time.
pub fn effective_fade(
    configured: CrossfadeDuration,
    outgoing: &TrackTiming,
    incoming: &TrackTiming,
) -> Seconds {
    configured
        .seconds()
        .min(outgoing.playable().half())
        .min(incoming.playable().half())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn close(a: f32, b: f32) -> bool {
        (a - b).abs() < 1e-5
    }

    #[test]
    fn equal_power_is_the_default_and_keeps_power_constant() {
        assert_eq!(CrossfadeCurve::default(), CrossfadeCurve::EqualPower);
        for step in 0..=10u8 {
            let g = CrossfadeCurve::EqualPower.gains(f32::from(step) / 10.0);
            assert!(close(g.outgoing.powi(2) + g.incoming.powi(2), 1.0));
        }
        let mid = CrossfadeCurve::EqualPower.gains(0.5);
        assert!(close(mid.outgoing, std::f32::consts::FRAC_1_SQRT_2));
    }

    #[test]
    fn endpoints_hear_only_one_side() {
        for curve in [
            CrossfadeCurve::EqualPower,
            CrossfadeCurve::Linear,
            CrossfadeCurve::Cut,
        ] {
            let start = curve.gains(0.0);
            let end = curve.gains(1.0);
            assert!(
                close(start.outgoing, 1.0) && close(start.incoming, 0.0),
                "{curve:?}"
            );
            assert!(
                close(end.outgoing, 0.0) && close(end.incoming, 1.0),
                "{curve:?}"
            );
        }
    }

    #[test]
    fn linear_curve_is_linear() {
        let g = CrossfadeCurve::Linear.gains(0.25);
        assert!(close(g.outgoing, 0.75) && close(g.incoming, 0.25));
    }

    #[test]
    fn cut_curve_keeps_both_decks_loud_in_the_middle() {
        let g = CrossfadeCurve::Cut.gains(0.5);
        assert!(close(g.outgoing, 1.0) && close(g.incoming, 1.0));
        let edge = CrossfadeCurve::Cut.gains(0.025);
        assert!(close(edge.incoming, 0.5));
    }

    #[test]
    fn progress_is_clamped() {
        assert_eq!(
            CrossfadeCurve::Linear.gains(-1.0),
            CrossfadeCurve::Linear.gains(0.0)
        );
        assert_eq!(
            CrossfadeCurve::Linear.gains(7.0),
            CrossfadeCurve::Linear.gains(1.0)
        );
        assert_eq!(
            CrossfadeCurve::Linear.gains(f32::NAN),
            CrossfadeCurve::Linear.gains(0.0)
        );
    }

    mod effective_fade {
        use super::*;

        fn timing(playable: f64) -> TrackTiming {
            TrackTiming::untrimmed(Seconds::saturating(playable))
        }

        #[test]
        fn uses_the_configured_fade_for_long_tracks() {
            let fade = effective_fade(CrossfadeDuration::default(), &timing(180.0), &timing(200.0));
            assert_eq!(fade.as_f64(), 8.0);
        }

        #[test]
        fn is_limited_to_half_of_a_short_outgoing_track() {
            let fade = effective_fade(CrossfadeDuration::default(), &timing(10.0), &timing(200.0));
            assert_eq!(fade.as_f64(), 5.0);
        }

        #[test]
        fn is_limited_to_half_of_a_short_incoming_track() {
            let fade = effective_fade(CrossfadeDuration::default(), &timing(200.0), &timing(6.0));
            assert_eq!(fade.as_f64(), 3.0);
        }

        #[test]
        fn zero_configured_fade_stays_zero() {
            let zero = CrossfadeDuration::new(0.0).unwrap();
            assert_eq!(
                effective_fade(zero, &timing(100.0), &timing(100.0)),
                Seconds::ZERO
            );
        }
    }
}
