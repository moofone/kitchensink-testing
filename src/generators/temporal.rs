//! Time-oriented generators.

use proptest::prelude::*;

const DEFAULT_START_MS: i64 = 1_577_836_800_000; // 2020-01-01T00:00:00Z
const DEFAULT_END_MS: i64 = 2_524_607_999_000; // 2049-12-31T23:59:59Z

/// Generate unix timestamp milliseconds in a practical default range.
pub fn valid_timestamp_millis() -> impl Strategy<Value = i64> {
    DEFAULT_START_MS..=DEFAULT_END_MS
}

/// Generate `(start, end)` where `end > start` and span <= `max_span_ms`.
pub fn timestamp_pair(max_span_ms: i64) -> impl Strategy<Value = (i64, i64)> {
    assert!(max_span_ms > 0, "max_span_ms must be positive");
    valid_timestamp_millis().prop_flat_map(move |start| {
        let end_max = (start + max_span_ms).min(DEFAULT_END_MS);
        (Just(start), (start + 1)..=end_max)
    })
}

/// Generate strictly increasing timestamps of length `count`.
pub fn monotonic_timestamps(
    count: usize,
    min_gap_ms: i64,
    max_gap_ms: i64,
) -> impl Strategy<Value = Vec<i64>> {
    assert!(count > 0, "count must be > 0");
    assert!(min_gap_ms > 0, "min_gap_ms must be > 0");
    assert!(max_gap_ms >= min_gap_ms, "max_gap_ms must be >= min_gap_ms");

    valid_timestamp_millis().prop_flat_map(move |start| {
        if count == 1 {
            return Just(vec![start]).boxed();
        }

        prop::collection::vec(min_gap_ms..=max_gap_ms, count - 1)
            .prop_map(move |gaps| {
                let mut out = Vec::with_capacity(count);
                out.push(start);
                for gap in gaps {
                    let next = out.last().copied().unwrap_or(start).saturating_add(gap);
                    out.push(next);
                }
                out
            })
            .boxed()
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    proptest! {
        #[test]
        fn monotonic_generation(ts in monotonic_timestamps(8, 10, 1000)) {
            for w in ts.windows(2) {
                prop_assert!(w[1] > w[0]);
            }
        }
    }
}
