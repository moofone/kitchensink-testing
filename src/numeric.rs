//! Numeric property helpers.
//!
//! Common assertions for numeric properties like bounds, overflow checks, etc.
//!
//! # Example
//!
//! ```rust,ignore
//! use rust_pbt::numeric::assert_bounded;
//!
//! assert_bounded(5, 0, 10);  // passes
//! assert_bounded(15, 0, 10); // panics
//! ```

/// Assert that a value is within specified bounds (inclusive).
///
/// # Panics
///
/// Panics if the value is outside the specified bounds.
///
/// # Example
///
/// ```rust
/// use rust_pbt::numeric::assert_bounded;
///
/// assert_bounded(5, 0, 10);  // passes
/// assert_bounded(0.5, 0.0, 1.0);  // passes
/// ```
pub fn assert_bounded<T: PartialOrd + std::fmt::Debug>(value: T, min: T, max: T) {
    assert!(
        value >= min && value <= max,
        "Value {:?} is not within bounds [{:?}, {:?}]",
        value,
        min,
        max
    );
}

/// Assert that a floating point value is non-negative.
///
/// # Panics
///
/// Panics if the value is negative.
///
/// # Example
///
/// ```rust
/// use rust_pbt::numeric::assert_non_negative;
///
/// assert_non_negative(0.0);  // passes
/// assert_non_negative(1.0);  // passes
/// ```
pub fn assert_non_negative(value: f64) {
    assert!(value >= 0.0, "Value {} must be non-negative", value);
}

/// Assert that a value is positive (strictly greater than zero).
///
/// # Panics
///
/// Panics if the value is not positive.
///
/// # Example
///
/// ```rust
/// use rust_pbt::numeric::assert_positive;
///
/// assert_positive(0.1);  // passes
/// assert_positive(1.0);  // passes
/// ```
pub fn assert_positive(value: f64) {
    assert!(value > 0.0, "Value {} must be positive", value);
}

/// Assert that a floating point value is finite (not NaN or infinity).
///
/// # Panics
///
/// Panics if the value is NaN or infinite.
///
/// # Example
///
/// ```rust
/// use rust_pbt::numeric::assert_finite;
///
/// assert_finite(0.0);   // passes
/// assert_finite(1.0);   // passes
/// assert_finite(-1.0);  // passes
/// ```
pub fn assert_finite(value: f64) {
    assert!(
        value.is_finite(),
        "Value {} must be finite (not NaN or infinity)",
        value
    );
}

/// Assert that two floating point values are approximately equal within a tolerance.
///
/// # Panics
///
/// Panics if the values differ by more than the specified tolerance.
///
/// # Example
///
/// ```rust
/// use rust_pbt::numeric::assert_approx_eq;
///
/// assert_approx_eq(1.0, 1.0001, 0.001);  // passes
/// assert_approx_eq(100.0, 100.01, 0.1);  // passes
/// ```
pub fn assert_approx_eq(a: f64, b: f64, tolerance: f64) {
    assert!(
        (a - b).abs() <= tolerance,
        "Values {} and {} differ by more than tolerance {}",
        a,
        b,
        tolerance
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bounded_success() {
        assert_bounded(5, 0, 10);
        assert_bounded(0.0, 0.0, 1.0);
    }

    #[test]
    #[should_panic]
    fn test_bounded_failure_too_low() {
        assert_bounded(-1, 0, 10);
    }

    #[test]
    #[should_panic]
    fn test_bounded_failure_too_high() {
        assert_bounded(11, 0, 10);
    }

    #[test]
    fn test_non_negative_success() {
        assert_non_negative(0.0);
        assert_non_negative(1.0);
    }

    #[test]
    #[should_panic]
    fn test_non_negative_failure() {
        assert_non_negative(-0.1);
    }

    #[test]
    fn test_positive_success() {
        assert_positive(0.1);
        assert_positive(1.0);
    }

    #[test]
    #[should_panic]
    fn test_positive_failure_zero() {
        assert_positive(0.0);
    }

    #[test]
    fn test_finite_success() {
        assert_finite(0.0);
        assert_finite(1.0);
        assert_finite(-1.0);
    }

    #[test]
    #[should_panic]
    fn test_finite_failure_nan() {
        assert_finite(f64::NAN);
    }

    #[test]
    #[should_panic]
    fn test_finite_failure_infinity() {
        assert_finite(f64::INFINITY);
    }

    #[test]
    fn test_approx_eq_success() {
        assert_approx_eq(1.0, 1.0001, 0.001);
        assert_approx_eq(100.0, 100.01, 0.1);
    }

    #[test]
    #[should_panic]
    fn test_approx_eq_failure() {
        assert_approx_eq(1.0, 2.0, 0.5);
    }
}
