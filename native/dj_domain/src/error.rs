//! Typed errors of the domain layer.

use thiserror::Error;

/// Errors raised when constructing or manipulating domain values.
#[derive(Debug, Clone, PartialEq, Error)]
pub enum DomainError {
    /// A floating-point value was NaN or infinite.
    #[error("{what} must be a finite number, got {value}")]
    NotFinite { what: &'static str, value: f64 },

    /// A value was outside of its allowed range.
    #[error("{what} must be within [{min}, {max}], got {value}")]
    OutOfRange {
        what: &'static str,
        value: f64,
        min: f64,
        max: f64,
    },

    /// Track timing boundaries are inconsistent.
    #[error("invalid track timing: {reason}")]
    InvalidTiming { reason: &'static str },

    /// An index into the playback queue does not exist.
    #[error("queue index {index} is out of bounds (len {len})")]
    QueueIndexOutOfBounds { index: usize, len: usize },
}

/// Ensures `value` is finite and within `[min, max]`.
pub(crate) fn check_range(
    what: &'static str,
    value: f64,
    min: f64,
    max: f64,
) -> Result<f64, DomainError> {
    if !value.is_finite() {
        return Err(DomainError::NotFinite { what, value });
    }
    if value < min || value > max {
        return Err(DomainError::OutOfRange {
            what,
            value,
            min,
            max,
        });
    }
    Ok(value)
}

#[cfg(test)]
mod tests {
    use super::{DomainError, check_range};

    #[test]
    fn accepts_bounds_inclusively() {
        assert_eq!(check_range("x", 0.0, 0.0, 1.0), Ok(0.0));
        assert_eq!(check_range("x", 1.0, 0.0, 1.0), Ok(1.0));
    }

    #[test]
    fn rejects_non_finite_values() {
        let err = check_range("x", f64::NAN, 0.0, 1.0).unwrap_err();
        assert!(matches!(err, DomainError::NotFinite { what: "x", .. }));
        assert!(check_range("x", f64::INFINITY, 0.0, 1.0).is_err());
    }

    #[test]
    fn rejects_out_of_range_values_with_a_readable_message() {
        let err = check_range("gain", 2.0, 0.0, 1.0).unwrap_err();
        assert_eq!(err.to_string(), "gain must be within [0, 1], got 2");
    }
}
