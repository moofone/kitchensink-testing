//! Trading domain strategies (candles, orders).
//!
//! This module provides strategies for generating trading-related data like
//! OHLCV candles and orders with proper constraints.
//!
//! # Example
//!
//! ```rust,ignore
//! use rust_pbt::trading::{valid_candle, Candle};
//!
//! proptest! {
//!     #[test]
//!     fn candle_ohlc_constraints(candle in valid_candle()) {
//!         prop_assert!(candle.high >= candle.open);
//!         prop_assert!(candle.high >= candle.close);
//!         prop_assert!(candle.low <= candle.open);
//!         prop_assert!(candle.low <= candle.close);
//!     }
//! }
//! ```

use proptest::prelude::*;
use serde::{Deserialize, Serialize};

use crate::financial::valid_price;
use crate::temporal::{monotonic_timestamps, valid_timestamp};

/// A trading candle with OHLCV data.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Candle {
    /// Timestamp in milliseconds
    pub timestamp: i64,
    /// Opening price
    pub open: f64,
    /// Highest price
    pub high: f64,
    /// Lowest price
    pub low: f64,
    /// Closing price
    pub close: f64,
    /// Volume
    pub volume: f64,
}

impl Default for Candle {
    fn default() -> Self {
        Self {
            timestamp: 0,
            open: 100.0,
            high: 100.0,
            low: 100.0,
            close: 100.0,
            volume: 0.0,
        }
    }
}

/// Order side (Buy or Sell).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Side {
    /// Buy order
    Buy,
    /// Sell order
    Sell,
}

/// Order type.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum OrderType {
    /// Market order
    Market,
    /// Limit order
    Limit,
    /// Stop loss order
    StopLoss,
    /// Take profit order
    TakeProfit,
}

/// Order status.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum OrderStatus {
    /// Order is pending
    Pending,
    /// Order is partially filled
    PartiallyFilled,
    /// Order is fully filled
    Filled,
    /// Order was cancelled
    Cancelled,
    /// Order was rejected
    Rejected,
}

/// A trading order.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Order {
    /// Order ID
    pub id: String,
    /// Timestamp in milliseconds
    pub timestamp: i64,
    /// Order side (buy/sell)
    pub side: Side,
    /// Order type
    pub order_type: OrderType,
    /// Order price
    pub price: f64,
    /// Order quantity
    pub quantity: f64,
    /// Filled quantity
    pub filled: f64,
    /// Order status
    pub status: OrderStatus,
}

/// Generate a valid order side (Buy or Sell).
pub fn valid_side() -> impl Strategy<Value = Side> {
    prop_oneof![Just(Side::Buy), Just(Side::Sell),]
}

/// Generate a valid order type.
pub fn valid_order_type() -> impl Strategy<Value = OrderType> {
    prop_oneof![
        Just(OrderType::Market),
        Just(OrderType::Limit),
        Just(OrderType::StopLoss),
        Just(OrderType::TakeProfit),
    ]
}

/// Generate a valid order status.
pub fn valid_order_status() -> impl Strategy<Value = OrderStatus> {
    prop_oneof![
        Just(OrderStatus::Pending),
        Just(OrderStatus::PartiallyFilled),
        Just(OrderStatus::Filled),
        Just(OrderStatus::Cancelled),
        Just(OrderStatus::Rejected),
    ]
}

/// Generate a valid quantity (positive, reasonable range).
pub fn valid_quantity() -> impl Strategy<Value = f64> {
    0.001f64..10_000.0
}

/// Generate a valid OHLCV candle with proper constraints.
///
/// Ensures:
/// - `high >= max(open, close)`
/// - `low <= min(open, close)`
/// - `volume >= 0`
///
/// # Example
///
/// ```rust,ignore
/// use rust_pbt::trading::valid_candle;
///
/// proptest! {
///     #[test]
///     fn candle_ohlc_constraints(candle in valid_candle()) {
///         prop_assert!(candle.high >= candle.open);
///         prop_assert!(candle.high >= candle.close);
///         prop_assert!(candle.low <= candle.open);
///         prop_assert!(candle.low <= candle.close);
///     }
/// }
/// ```
pub fn valid_candle() -> impl Strategy<Value = Candle> {
    (
        valid_timestamp(),
        valid_price(),
        0.0f64..1.0,         // high_offset (% above max)
        0.0f64..1.0,         // low_offset (% below min)
        valid_price(),       // close
        0.0f64..1_000_000.0, // volume
    )
        .prop_map(|(ts, open, high_off, low_off, close, volume)| {
            let max_oc = open.max(close);
            let min_oc = open.min(close);

            let high = max_oc * (1.0 + high_off);
            let low = min_oc * (1.0 - low_off);

            Candle {
                timestamp: ts,
                open,
                high,
                low,
                close,
                volume,
            }
        })
}

/// Generate a candle with specific price range.
///
/// # Arguments
///
/// * `min_price` - Minimum price for OHLC values
/// * `max_price` - Maximum price for OHLC values
pub fn candle_in_range(min_price: f64, max_price: f64) -> impl Strategy<Value = Candle> {
    (
        valid_timestamp(),
        min_price..max_price,
        0.0f64..0.5,          // high_offset (up to 50% above)
        0.0f64..0.5,          // low_offset (up to 50% below)
        min_price..max_price, // close
        0.0f64..1_000_000.0,  // volume
    )
        .prop_map(|(ts, open, high_off, low_off, close, volume)| {
            let max_oc = open.max(close);
            let min_oc = open.min(close);

            let high = max_oc * (1.0 + high_off);
            let low = min_oc * (1.0 - low_off);

            Candle {
                timestamp: ts,
                open,
                high,
                low,
                close,
                volume,
            }
        })
}

/// Generate a bullish candle (close > open).
pub fn bullish_candle() -> impl Strategy<Value = Candle> {
    valid_candle().prop_filter("Close must be greater than open", |c| c.close > c.open)
}

/// Generate a bearish candle (open > close).
pub fn bearish_candle() -> impl Strategy<Value = Candle> {
    valid_candle().prop_filter("Open must be greater than close", |c| c.open > c.close)
}

/// Generate a sequence of candles with monotonic timestamps.
///
/// # Arguments
///
/// * `n` - Number of candles to generate
/// * `min_gap_ms` - Minimum time gap between candles (milliseconds)
pub fn candle_sequence(n: usize, min_gap_ms: i64) -> impl Strategy<Value = Vec<Candle>> {
    (
        monotonic_timestamps(n, min_gap_ms),
        prop::collection::vec(valid_price(), n),
        prop::collection::vec(0.0f64..1.0, n),
        prop::collection::vec(0.0f64..1.0, n),
        prop::collection::vec(valid_price(), n),
        prop::collection::vec(0.0f64..1_000_000.0, n),
    )
        .prop_map(
            |(timestamps, opens, high_offs, low_offs, closes, volumes)| {
                timestamps
                    .into_iter()
                    .zip(opens)
                    .zip(high_offs)
                    .zip(low_offs)
                    .zip(closes)
                    .zip(volumes)
                    .map(
                        |(((((ts, open), high_off), low_off), close), volume)| {
                            let max_oc = open.max(close);
                            let min_oc = open.min(close);

                            let high = max_oc * (1.0 + high_off);
                            let low = min_oc * (1.0 - low_off);

                            Candle {
                                timestamp: ts,
                                open,
                                high,
                                low,
                                close,
                                volume,
                            }
                        },
                    )
                    .collect()
            },
        )
}

/// Generate a valid order.
///
/// # Example
///
/// ```rust,ignore
/// use rust_pbt::trading::valid_order;
///
/// proptest! {
///     #[test]
///     fn order_properties(order in valid_order()) {
///         prop_assert!(order.quantity > 0.0);
///         prop_assert!(order.filled >= 0.0);
///         prop_assert!(order.filled <= order.quantity);
///     }
/// }
/// ```
pub fn valid_order() -> impl Strategy<Value = Order> {
    use proptest::strategy::BoxedStrategy;

    (
        "[a-zA-Z0-9]{10}",
        valid_timestamp(),
        valid_side(),
        valid_order_type(),
        valid_price(),
        valid_quantity(),
        valid_order_status(),
    )
        .prop_flat_map(|(id, ts, side, order_type, price, quantity, status)| {
            let filled_strategy: BoxedStrategy<f64> = match status {
                OrderStatus::Pending => Just(0.0).boxed(),
                OrderStatus::PartiallyFilled => (0.01..=quantity * 0.99).boxed(),
                OrderStatus::Filled => Just(quantity).boxed(),
                OrderStatus::Cancelled | OrderStatus::Rejected => (0.0..=quantity * 0.5).boxed(),
            };

            (
                Just(id),
                Just(ts),
                Just(side),
                Just(order_type),
                Just(price),
                Just(quantity),
                filled_strategy,
                Just(status),
            )
        })
        .prop_map(
            move |(id, ts, side, order_type, price, quantity, filled, status)| Order {
                id,
                timestamp: ts,
                side,
                order_type,
                price,
                quantity,
                filled,
                status,
            },
        )
}

/// Generate a pending order (not yet filled).
pub fn pending_order() -> impl Strategy<Value = Order> {
    valid_order().prop_filter(
        "Status must be Pending",
        |o| o.status == OrderStatus::Pending && o.filled == 0.0,
    )
}

/// Generate a filled order.
pub fn filled_order() -> impl Strategy<Value = Order> {
    valid_order().prop_filter(
        "Status must be Filled",
        |o| o.status == OrderStatus::Filled && (o.filled - o.quantity).abs() < 1e-10,
    )
}

/// Generate a partially filled order.
pub fn partially_filled_order() -> impl Strategy<Value = Order> {
    valid_order().prop_filter(
        "Status must be PartiallyFilled",
        |o| {
            o.status == OrderStatus::PartiallyFilled
                && o.filled > 0.0
                && o.filled < o.quantity
        },
    )
}

/// Generate a buy order.
pub fn buy_order() -> impl Strategy<Value = Order> {
    valid_order().prop_filter("Side must be Buy", |o| o.side == Side::Buy)
}

/// Generate a sell order.
pub fn sell_order() -> impl Strategy<Value = Order> {
    valid_order().prop_filter("Side must be Sell", |o| o.side == Side::Sell)
}

#[cfg(test)]
mod tests {
    use super::*;

    proptest! {
        #[test]
        fn test_valid_candle_high_constraint(candle in valid_candle()) {
            prop_assert!(candle.high >= candle.open);
            prop_assert!(candle.high >= candle.close);
        }

        #[test]
        fn test_valid_candle_low_constraint(candle in valid_candle()) {
            prop_assert!(candle.low <= candle.open);
            prop_assert!(candle.low <= candle.close);
        }

        #[test]
        fn test_valid_candle_volume_nonnegative(candle in valid_candle()) {
            prop_assert!(candle.volume >= 0.0);
        }

        #[test]
        fn test_bullish_candle_ordering(candle in bullish_candle()) {
            prop_assert!(candle.close > candle.open);
        }

        #[test]
        fn test_bearish_candle_ordering(candle in bearish_candle()) {
            prop_assert!(candle.open > candle.close);
        }

        #[test]
        fn test_candle_sequence_timestamps(candles in candle_sequence(5, 1000)) {
            for window in candles.windows(2) {
                prop_assert!(window[1].timestamp > window[0].timestamp);
                prop_assert!(window[1].timestamp - window[0].timestamp >= 1000);
            }
        }

        #[test]
        fn test_valid_order_filled_constraint(order in valid_order()) {
            prop_assert!(order.filled >= 0.0);
            prop_assert!(order.filled <= order.quantity);
        }

        #[test]
        fn test_valid_order_quantity_positive(order in valid_order()) {
            prop_assert!(order.quantity > 0.0);
        }

        #[test]
        fn test_pending_order_not_filled(order in pending_order()) {
            prop_assert_eq!(order.status, OrderStatus::Pending);
            prop_assert_eq!(order.filled, 0.0);
        }

        #[test]
        fn test_filled_order_completely_filled(order in filled_order()) {
            prop_assert_eq!(order.status, OrderStatus::Filled);
            prop_assert!((order.filled - order.quantity).abs() < 1e-10);
        }

        #[test]
        fn test_partially_filled_order_partial(order in partially_filled_order()) {
            prop_assert_eq!(order.status, OrderStatus::PartiallyFilled);
            prop_assert!(order.filled > 0.0);
            prop_assert!(order.filled < order.quantity);
        }

        #[test]
        fn test_buy_order_side(order in buy_order()) {
            prop_assert_eq!(order.side, Side::Buy);
        }

        #[test]
        fn test_sell_order_side(order in sell_order()) {
            prop_assert_eq!(order.side, Side::Sell);
        }
    }
}
