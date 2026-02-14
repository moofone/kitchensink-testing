//! Financial domain strategies and property templates.
//!
//! This module provides strategies for generating financial data (prices, etc.)
//! and property templates for testing financial calculations.
//!
//! # Example
//!
//! ```rust,ignore
//! use rust_pbt::financial::{valid_price, assert_pnl_sign_correct};
//!
//! proptest! {
//!     #[test]
//!     fn pnl_correct_for_long(entry in valid_price(), exit in valid_price()) {
//!         let pnl = (exit - entry) * 10.0;
//!         assert_pnl_sign_correct(entry, exit, 10.0, true, pnl);
//!     }
//! }
//! ```

use proptest::prelude::*;

/// Generate valid trading prices (positive, reasonable range).
///
/// Generates prices in the range `[0.01, 1,000,000.0]` with reasonable precision
/// to avoid extreme floating point edge cases.
///
/// # Example
///
/// ```rust,ignore
/// use rust_pbt::financial::valid_price;
/// use proptest::prelude::*;
///
/// proptest! {
///     #[test]
///     fn price_is_positive(price in valid_price()) {
///         prop_assert!(price > 0.0);
///     }
/// }
/// ```
pub fn valid_price() -> impl Strategy<Value = f64> {
    0.01f64..1_000_000.0
}

/// Generate prices within a specific range.
///
/// # Arguments
///
/// * `min` - Minimum price (inclusive)
/// * `max` - Maximum price (exclusive)
///
/// # Panics
///
/// Panics if `min <= 0`, `max <= min`, or arguments are not finite.
///
/// # Example
///
/// ```rust,ignore
/// use rust_pbt::financial::price_range;
///
/// // Generate BTC prices between $20k and $100k
/// let btc_price = price_range(20_000.0, 100_000.0);
/// ```
pub fn price_range(min: f64, max: f64) -> impl Strategy<Value = f64> {
    assert!(min > 0.0, "Minimum price must be positive");
    assert!(max > min, "Maximum price must be greater than minimum");
    min..max
}

/// Generate price with specific tick size.
///
/// Ensures generated prices are exact multiples of the tick size,
/// matching real exchange constraints.
///
/// # Arguments
///
/// * `min` - Minimum price
/// * `max` - Maximum price
/// * `tick` - Tick size (e.g., 0.01 for cent precision, 0.5 for half-dollar)
///
/// # Panics
///
/// Panics if arguments are invalid (non-positive, etc.)
///
/// # Example
///
/// ```rust,ignore
/// use rust_pbt::financial::price_with_tick;
///
/// // Generate prices in $0.50 increments between $1 and $100
/// let price = price_with_tick(1.0, 100.0, 0.5);
/// // Generates: 1.0, 1.5, 2.0, 2.5, ..., 99.5, 100.0
/// ```
pub fn price_with_tick(min: f64, max: f64, tick: f64) -> impl Strategy<Value = f64> {
    assert!(min > 0.0, "Minimum price must be positive");
    assert!(max > min, "Maximum price must be greater than minimum");
    assert!(tick > 0.0, "Tick size must be positive");

    let min_ticks = (min / tick).ceil() as i64;
    let max_ticks = (max / tick).floor() as i64;

    (min_ticks..=max_ticks).prop_map(move |ticks| ticks as f64 * tick)
}

/// Generate a pair of prices (entry, exit) for testing PNL calculations.
pub fn price_pair() -> impl Strategy<Value = (f64, f64)> {
    (valid_price(), valid_price())
}

/// Generate a price pair where exit > entry (profitable long scenario).
pub fn profitable_long_pair() -> impl Strategy<Value = (f64, f64)> {
    valid_price().prop_flat_map(|entry| (Just(entry), (entry * 1.001)..=(entry * 10.0)))
}

/// Generate a price pair where entry > exit (profitable short scenario).
pub fn profitable_short_pair() -> impl Strategy<Value = (f64, f64)> {
    valid_price().prop_flat_map(|entry| (Just(entry), (entry * 0.1)..=(entry * 0.999)))
}

/// Trait for types that have financial properties.
pub trait FinancialProperties {
    /// Check if the value represents a valid financial amount.
    fn is_valid_amount(&self) -> bool;
}

impl FinancialProperties for f64 {
    fn is_valid_amount(&self) -> bool {
        self.is_finite() && *self >= 0.0
    }
}

/// Assert that PNL sign is correct for the given entry/exit prices and position direction.
///
/// # Arguments
///
/// * `entry_price` - Entry price
/// * `exit_price` - Exit price
/// * `quantity` - Position quantity
/// * `is_long` - True for long position, false for short
/// * `pnl` - Calculated PNL
///
/// # Panics
///
/// Panics if the PNL sign doesn't match expectations for the position direction.
///
/// # Example
///
/// ```rust
/// use rust_pbt::financial::assert_pnl_sign_correct;
///
/// // Long position: bought at 100, sold at 110, quantity 10
/// // PNL = (110 - 100) * 10 = 100 (before fees)
/// assert_pnl_sign_correct(100.0, 110.0, 10.0, true, 95.0); // 95 after fees
/// ```
pub fn assert_pnl_sign_correct(
    entry_price: f64,
    exit_price: f64,
    quantity: f64,
    is_long: bool,
    pnl: f64,
) {
    let price_diff = exit_price - entry_price;

    // Determine expected sign based on direction and price movement
    let expected_sign = if is_long {
        // Long: profit when exit > entry
        price_diff.signum()
    } else {
        // Short: profit when entry > exit
        (-price_diff).signum()
    };

    // Allow for small losses due to fees when price movement is minimal
    if price_diff.abs() > (entry_price * 0.01) {
        // Significant price movement - PNL sign should match expected
        assert_eq!(
            pnl.signum(),
            expected_sign,
            "PNL sign incorrect. Entry: {}, Exit: {}, Quantity: {}, Long: {}, PNL: {}, Expected sign: {}",
            entry_price, exit_price, quantity, is_long, pnl, expected_sign
        );
    }
}

/// Assert that fees reduce profit or increase loss.
///
/// Net PNL = Gross PNL - Fees
///
/// # Panics
///
/// Panics if fees don't correctly reduce net PNL.
///
/// # Example
///
/// ```rust
/// use rust_pbt::financial::assert_fee_reduces_profit;
///
/// let gross_pnl = 100.0;
/// let fees = 5.0;
/// let net_pnl = 95.0;
/// assert_fee_reduces_profit(gross_pnl, fees, net_pnl);
/// ```
pub fn assert_fee_reduces_profit(gross_pnl: f64, fees: f64, net_pnl: f64) {
    // Net should equal gross minus fees
    let expected_net = gross_pnl - fees;
    assert!(
        (net_pnl - expected_net).abs() < 1e-6,
        "Net PNL should equal Gross PNL minus Fees. Gross: {}, Fees: {}, Net: {}, Expected: {}",
        gross_pnl, fees, net_pnl, expected_net
    );

    // Fees should always reduce net compared to gross
    if fees > 0.0 {
        assert!(
            net_pnl < gross_pnl,
            "Fees should reduce PNL. Gross: {}, Net: {}, Fees: {}",
            gross_pnl, net_pnl, fees
        );
    }
}

/// Assert that position size is conserved through fills.
///
/// `entry_size - sum(fills) = remaining_size`
///
/// # Panics
///
/// Panics if position size is not conserved.
///
/// # Example
///
/// ```rust
/// use rust_pbt::financial::assert_position_size_conserved;
///
/// let entry_size = 100.0;
/// let fills = vec![30.0, 20.0, 50.0];
/// let remaining = 0.0;
/// assert_position_size_conserved(entry_size, &fills, remaining);
/// ```
pub fn assert_position_size_conserved(entry_size: f64, fills: &[f64], current_size: f64) {
    let total_filled: f64 = fills.iter().sum();
    let expected_current = entry_size - total_filled;

    assert!(
        (current_size - expected_current).abs() < 1e-6,
        "Position size should be conserved: entry {} - fills {} = current {}, expected {}",
        entry_size, total_filled, current_size, expected_current
    );
}

/// Assert that total PNL equals realized plus unrealized.
///
/// # Panics
///
/// Panics if total PNL doesn't equal the sum of realized and unrealized.
///
/// # Example
///
/// ```rust
/// use rust_pbt::financial::assert_total_pnl_conservation;
///
/// let realized = 50.0;
/// let unrealized = 25.0;
/// let total = 75.0;
/// assert_total_pnl_conservation(realized, unrealized, total);
/// ```
pub fn assert_total_pnl_conservation(realized_pnl: f64, unrealized_pnl: f64, total_pnl: f64) {
    let expected_total = realized_pnl + unrealized_pnl;
    assert!(
        (total_pnl - expected_total).abs() < 1e-6,
        "Total PNL should equal Realized + Unrealized. Realized: {}, Unrealized: {}, Total: {}, Expected: {}",
        realized_pnl, unrealized_pnl, total_pnl, expected_total
    );
}

/// Assert that all fills are less than or equal to the original order size.
///
/// # Panics
///
/// Panics if total fills exceed order size.
///
/// # Example
///
/// ```rust
/// use rust_pbt::financial::assert_no_overfill;
///
/// let order_size = 100.0;
/// let fills = vec![30.0, 40.0, 30.0];
/// assert_no_overfill(order_size, &fills);
/// ```
pub fn assert_no_overfill(order_size: f64, fills: &[f64]) {
    let total_filled: f64 = fills.iter().sum();
    assert!(
        total_filled <= order_size + 1e-6,
        "Total fills {} exceed order size {}",
        total_filled, order_size
    );

    // Also check each individual fill
    for (i, &fill) in fills.iter().enumerate() {
        assert!(
            fill <= order_size + 1e-6,
            "Individual fill[{}] = {} exceeds order size {}",
            i, fill, order_size
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    proptest! {
        #[test]
        fn test_valid_price_is_positive(price in valid_price()) {
            prop_assert!(price > 0.0);
            prop_assert!(price < 1_000_001.0);
        }

        #[test]
        fn test_price_with_tick_alignment(price in price_with_tick(1.0, 100.0, 0.5)) {
            let ticks = (price / 0.5).round();
            prop_assert!((price - (ticks * 0.5)).abs() < 1e-10);
        }

        #[test]
        fn test_profitable_long_pair_ordering((entry, exit) in profitable_long_pair()) {
            prop_assert!(exit > entry);
        }

        #[test]
        fn test_profitable_short_pair_ordering((entry, exit) in profitable_short_pair()) {
            prop_assert!(entry > exit);
        }
    }

    #[test]
    fn test_pnl_sign_correct_long_profit() {
        assert_pnl_sign_correct(100.0, 110.0, 10.0, true, 95.0);
    }

    #[test]
    fn test_pnl_sign_correct_long_loss() {
        assert_pnl_sign_correct(100.0, 90.0, 10.0, true, -105.0);
    }

    #[test]
    fn test_pnl_sign_correct_short_profit() {
        assert_pnl_sign_correct(100.0, 90.0, 10.0, false, 95.0);
    }

    #[test]
    fn test_pnl_sign_correct_short_loss() {
        assert_pnl_sign_correct(100.0, 110.0, 10.0, false, -105.0);
    }

    #[test]
    fn test_fee_reduces_profit() {
        assert_fee_reduces_profit(100.0, 5.0, 95.0);
    }

    #[test]
    fn test_fee_increases_loss() {
        assert_fee_reduces_profit(-50.0, 5.0, -55.0);
    }

    #[test]
    fn test_position_size_conserved() {
        assert_position_size_conserved(100.0, &[30.0, 40.0, 30.0], 0.0);
    }

    #[test]
    fn test_total_pnl_conservation() {
        assert_total_pnl_conservation(50.0, 25.0, 75.0);
    }

    #[test]
    fn test_no_overfill() {
        assert_no_overfill(100.0, &[30.0, 40.0, 30.0]);
    }

    #[test]
    #[should_panic]
    fn test_no_overfill_fails() {
        assert_no_overfill(100.0, &[30.0, 40.0, 40.0]); // 110 > 100
    }
}
