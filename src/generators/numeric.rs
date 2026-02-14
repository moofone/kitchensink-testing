//! Numeric strategy primitives.

use proptest::prelude::*;

/// Generate any finite `f64` value.
pub fn finite_f64() -> impl Strategy<Value = f64> {
    any::<f64>().prop_filter("finite f64", |v| v.is_finite())
}

/// Generate `f64` in `[min, max]`.
pub fn bounded_f64(min: f64, max: f64) -> impl Strategy<Value = f64> {
    assert!(min.is_finite() && max.is_finite(), "bounds must be finite");
    assert!(max >= min, "max must be >= min");
    min..=max
}

/// Generate non-negative `f64` values in `[0, max]`.
pub fn non_negative_f64(max: f64) -> impl Strategy<Value = f64> {
    assert!(max >= 0.0, "max must be non-negative");
    0.0..=max
}

/// Generate positive `f64` values in `[min, max]`.
pub fn positive_f64(min: f64, max: f64) -> impl Strategy<Value = f64> {
    assert!(min > 0.0, "min must be positive");
    assert!(max >= min, "max must be >= min");
    min..=max
}

#[cfg(test)]
mod tests {
    use super::*;

    proptest! {
        #[test]
        fn bounded_respects_range(v in bounded_f64(-2.0, 3.0)) {
            prop_assert!(v >= -2.0 && v <= 3.0);
        }

        #[test]
        fn finite_is_finite(v in finite_f64()) {
            prop_assert!(v.is_finite());
        }
    }
}
