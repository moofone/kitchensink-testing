//! Cross-cutting invariant assertions.

use std::fmt::Debug;

/// Assert monotonically increasing sequence.
pub fn assert_monotonic_increasing<T>(values: &[T])
where
    T: PartialOrd + Debug,
{
    for window in values.windows(2) {
        assert!(
            window[1] > window[0],
            "sequence is not strictly increasing: {:?} !> {:?}",
            window[1],
            window[0]
        );
    }
}

/// Assert all values are inside `[min, max]`.
pub fn assert_all_in_range<T>(values: &[T], min: T, max: T)
where
    T: PartialOrd + Debug + Copy,
{
    for value in values {
        assert!(
            *value >= min && *value <= max,
            "value {:?} outside range [{:?}, {:?}]",
            value,
            min,
            max
        );
    }
}

/// Assert approximate equality with absolute tolerance.
pub fn assert_approx_eq(left: f64, right: f64, tolerance: f64) {
    assert!(tolerance >= 0.0, "tolerance must be non-negative");
    assert!(
        (left - right).abs() <= tolerance,
        "{} and {} differ more than {}",
        left,
        right,
        tolerance
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn range_and_monotonic_examples() {
        assert_monotonic_increasing(&[1_i32, 2, 3, 10]);
        assert_all_in_range(&[0.1_f64, 0.2, 0.9], 0.0, 1.0);
        assert_approx_eq(1.0, 1.000_1, 0.001);
    }
}
