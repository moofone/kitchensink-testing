//! Temporal (timestamp) strategies and property helpers.
//!
//! Provides strategies for generating valid timestamps with various constraints
//! and helpers for testing temporal properties.
//!
//! # Example
//!
//! ```rust,ignore
//! use rust_pbt::temporal::{valid_timestamp, assert_monotonic};
//!
//! proptest! {
//!     #[test]
//!     fn timestamps_are_ordered(ts in monotonic_timestamps(10, 1000)) {
//!         assert_monotonic(&ts);
//!     }
//! }
//! ```

use chrono::{TimeZone, Utc};
use proptest::prelude::*;

/// Generate valid timestamps in milliseconds (2020-2025 range).
///
/// Returns timestamps as Unix milliseconds (i64).
///
/// # Example
///
/// ```rust,ignore
/// use rust_pbt::temporal::valid_timestamp;
///
/// proptest! {
///     #[test]
///     fn timestamp_in_valid_range(ts in valid_timestamp()) {
///         // ts is between 2020 and 2025
///     }
/// }
/// ```
pub fn valid_timestamp() -> impl Strategy<Value = i64> {
    let start = Utc
        .with_ymd_and_hms(2020, 1, 1, 0, 0, 0)
        .unwrap()
        .timestamp_millis();
    let end = Utc
        .with_ymd_and_hms(2025, 12, 31, 23, 59, 59)
        .unwrap()
        .timestamp_millis();

    start..=end
}

/// Generate valid timestamps in seconds (for systems using second precision).
///
/// # Example
///
/// ```rust,ignore
/// use rust_pbt::temporal::valid_timestamp_seconds;
///
/// proptest! {
///     #[test]
///     fn timestamp_seconds(ts in valid_timestamp_seconds()) {
///         // ts is in seconds
///     }
/// }
/// ```
pub fn valid_timestamp_seconds() -> impl Strategy<Value = i64> {
    let start = Utc.with_ymd_and_hms(2020, 1, 1, 0, 0, 0).unwrap().timestamp();
    let end = Utc
        .with_ymd_and_hms(2025, 12, 31, 23, 59, 59)
        .unwrap()
        .timestamp();

    start..=end
}

/// Generate monotonic sequence of timestamps.
///
/// Produces a vector of timestamps where each timestamp is strictly greater than the previous.
///
/// # Arguments
///
/// * `n` - Number of timestamps to generate
/// * `min_gap_ms` - Minimum gap between consecutive timestamps (milliseconds)
///
/// # Panics
///
/// Panics if `n == 0` or `min_gap_ms == 0`.
///
/// # Example
///
/// ```rust,ignore
/// use rust_pbt::temporal::monotonic_timestamps;
///
/// proptest! {
///     #[test]
///     fn timestamps_are_ordered(ts in monotonic_timestamps(10, 1000)) {
///         for window in ts.windows(2) {
///             prop_assert!(window[1] > window[0]);
///             prop_assert!(window[1] - window[0] >= 1000);
///         }
///     }
/// }
/// ```
pub fn monotonic_timestamps(n: usize, min_gap_ms: i64) -> impl Strategy<Value = Vec<i64>> {
    assert!(n > 0, "Must generate at least one timestamp");
    assert!(min_gap_ms > 0, "Minimum gap must be positive");

    let max_gap = min_gap_ms.max(1000) + 1000;

    valid_timestamp().prop_flat_map(move |start| {
        prop::collection::vec(min_gap_ms..max_gap, n).prop_map(move |gaps| {
            gaps.iter()
                .scan(start, |acc, &gap| {
                    *acc += gap;
                    Some(*acc)
                })
                .collect()
        })
    })
}

/// Generate monotonic sequence of timestamps in seconds.
///
/// # Arguments
///
/// * `n` - Number of timestamps to generate
/// * `min_gap_sec` - Minimum gap between consecutive timestamps (seconds)
///
/// # Panics
///
/// Panics if `n == 0` or `min_gap_sec == 0`.
///
/// # Example
///
/// ```rust,ignore
/// use rust_pbt::temporal::monotonic_timestamps_seconds;
///
/// // Generate 5 timestamps with at least 60 seconds between each
/// let ts = monotonic_timestamps_seconds(5, 60);
/// ```
pub fn monotonic_timestamps_seconds(n: usize, min_gap_sec: i64) -> impl Strategy<Value = Vec<i64>> {
    assert!(n > 0, "Must generate at least one timestamp");
    assert!(min_gap_sec > 0, "Minimum gap must be positive");

    valid_timestamp_seconds().prop_flat_map(move |start| {
        prop::collection::vec(min_gap_sec..3600, n).prop_map(move |gaps| {
            gaps.iter()
                .scan(start, |acc, &gap| {
                    *acc += gap;
                    Some(*acc)
                })
                .collect()
        })
    })
}

/// Generate a pair of timestamps (start, end) where end > start.
///
/// Useful for testing time range operations.
///
/// # Example
///
/// ```rust,ignore
/// use rust_pbt::temporal::timestamp_pair;
///
/// proptest! {
///     #[test]
///     fn time_range_valid((start, end) in timestamp_pair()) {
///         prop_assert!(end > start);
///     }
/// }
/// ```
pub fn timestamp_pair() -> impl Strategy<Value = (i64, i64)> {
    valid_timestamp().prop_flat_map(|start| {
        (
            Just(start),
            (start + 1)..=(start + 86400000), // Up to 24 hours later
        )
    })
}

/// Assert that a sequence of timestamps is monotonically increasing.
///
/// # Panics
///
/// Panics if any timestamp is not strictly greater than the previous one.
///
/// # Example
///
/// ```rust
/// use rust_pbt::temporal::assert_monotonic;
///
/// assert_monotonic(&[1, 2, 3, 4, 5]);  // passes
/// ```
pub fn assert_monotonic(timestamps: &[i64]) {
    for window in timestamps.windows(2) {
        assert!(
            window[1] > window[0],
            "Timestamps are not monotonic: {} is not greater than {}",
            window[1],
            window[0]
        );
    }
}

/// Assert that a sequence of timestamps has no gaps larger than the specified maximum.
///
/// # Panics
///
/// Panics if any gap between consecutive timestamps exceeds the maximum.
///
/// # Example
///
/// ```rust
/// use rust_pbt::temporal::assert_no_large_gaps;
///
/// assert_no_large_gaps(&[0, 10, 20, 30], 15);  // passes
/// ```
pub fn assert_no_large_gaps(timestamps: &[i64], max_gap: i64) {
    for window in timestamps.windows(2) {
        let gap = window[1] - window[0];
        assert!(
            gap <= max_gap,
            "Gap {} between timestamps {} and {} exceeds maximum {}",
            gap, window[0], window[1], max_gap
        );
    }
}

/// Assert that a sequence of timestamps has minimum gaps between consecutive values.
///
/// # Panics
///
/// Panics if any gap between consecutive timestamps is less than the minimum.
///
/// # Example
///
/// ```rust
/// use rust_pbt::temporal::assert_min_gaps;
///
/// assert_min_gaps(&[0, 10, 20, 30], 10);  // passes
/// ```
pub fn assert_min_gaps(timestamps: &[i64], min_gap: i64) {
    for window in timestamps.windows(2) {
        let gap = window[1] - window[0];
        assert!(
            gap >= min_gap,
            "Gap {} between timestamps {} and {} is less than minimum {}",
            gap, window[0], window[1], min_gap
        );
    }
}

/// Assert that all timestamps are within a valid range.
///
/// # Panics
///
/// Panics if any timestamp is outside the specified range.
///
/// # Example
///
/// ```rust
/// use rust_pbt::temporal::assert_valid_range;
///
/// assert_valid_range(&[100, 200, 300], 0, 1000);  // passes
/// ```
pub fn assert_valid_range(timestamps: &[i64], min: i64, max: i64) {
    for &ts in timestamps {
        assert!(
            ts >= min && ts <= max,
            "Timestamp {} is outside valid range [{}, {}]",
            ts, min, max
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    proptest! {
        #[test]
        fn test_valid_timestamp_in_range(ts in valid_timestamp()) {
            let start = Utc.with_ymd_and_hms(2020, 1, 1, 0, 0, 0).unwrap().timestamp_millis();
            let end = Utc.with_ymd_and_hms(2025, 12, 31, 23, 59, 59).unwrap().timestamp_millis();
            prop_assert!(ts >= start);
            prop_assert!(ts <= end);
        }

        #[test]
        fn test_monotonic_timestamps_ordered(ts in monotonic_timestamps(10, 100)) {
            for window in ts.windows(2) {
                prop_assert!(window[1] > window[0]);
                prop_assert!(window[1] - window[0] >= 100);
            }
        }

        #[test]
        fn test_timestamp_pair_ordering((start, end) in timestamp_pair()) {
            prop_assert!(end > start);
        }
    }

    #[test]
    fn test_assert_monotonic_success() {
        assert_monotonic(&[1, 2, 3, 4, 5]);
        assert_monotonic(&[100, 200, 300]);
    }

    #[test]
    #[should_panic]
    fn test_assert_monotonic_failure_equal() {
        assert_monotonic(&[1, 2, 2, 3]);
    }

    #[test]
    #[should_panic]
    fn test_assert_monotonic_failure_decreasing() {
        assert_monotonic(&[1, 2, 1]);
    }

    #[test]
    fn test_assert_no_large_gaps_success() {
        assert_no_large_gaps(&[0, 10, 20, 30], 15);
    }

    #[test]
    #[should_panic]
    fn test_assert_no_large_gaps_failure() {
        assert_no_large_gaps(&[0, 10, 30], 15);
    }

    #[test]
    fn test_assert_min_gaps_success() {
        assert_min_gaps(&[0, 10, 20, 30], 10);
    }

    #[test]
    #[should_panic]
    fn test_assert_min_gaps_failure() {
        assert_min_gaps(&[0, 10, 15], 10);
    }

    #[test]
    fn test_assert_valid_range_success() {
        assert_valid_range(&[100, 200, 300], 0, 1000);
    }

    #[test]
    #[should_panic]
    fn test_assert_valid_range_failure_too_low() {
        assert_valid_range(&[50, 100], 100, 1000);
    }

    #[test]
    #[should_panic]
    fn test_assert_valid_range_failure_too_high() {
        assert_valid_range(&[100, 2000], 0, 1000);
    }
}
