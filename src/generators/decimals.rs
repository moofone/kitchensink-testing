//! Decimal/tick-aligned strategy helpers.

use proptest::prelude::*;

/// Generate prices aligned to `tick_size` in `[min, max]`.
pub fn tick_aligned(min: f64, max: f64, tick_size: f64) -> impl Strategy<Value = f64> {
    assert!(min.is_finite() && max.is_finite(), "bounds must be finite");
    assert!(
        tick_size.is_finite() && tick_size > 0.0,
        "tick_size must be positive and finite"
    );
    assert!(max >= min, "max must be >= min");

    let min_ticks = (min / tick_size).ceil() as i64;
    let max_ticks = (max / tick_size).floor() as i64;
    assert!(max_ticks >= min_ticks, "no valid ticks in range");

    (min_ticks..=max_ticks).prop_map(move |t| t as f64 * tick_size)
}

#[cfg(test)]
mod tests {
    use super::*;

    proptest! {
        #[test]
        fn tick_alignment_holds(v in tick_aligned(1.0, 100.0, 0.25)) {
            let ticks = (v / 0.25).round();
            prop_assert!((v - ticks * 0.25).abs() < 1e-10);
        }
    }
}
