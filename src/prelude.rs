//! Prelude module for convenient imports.
//!
//! This module re-exports the most commonly used items from this crate.
//!
//! # Example
//!
//! ```rust,ignore
//! use rust_pbt::prelude::*;
//! ```

// Re-export proptest
pub use proptest::prelude::*;

// Re-export core utilities based on enabled features
#[cfg(feature = "arithmetic")]
pub use crate::arithmetic::{
    assert_associative, assert_commutative, assert_distributive, assert_identity,
};

#[cfg(feature = "collections")]
pub use crate::collections::{
    assert_all_satisfy, assert_no_duplicates, assert_same_elements, assert_size_preserved,
    assert_sorted, assert_sorted_descending,
};

#[cfg(feature = "numeric")]
pub use crate::numeric::{
    assert_approx_eq, assert_bounded, assert_finite, assert_non_negative, assert_positive,
};

#[cfg(feature = "stateful")]
pub use crate::stateful::{
    assert_idempotent, assert_involutive, assert_state_invariant, assert_valid_state_sequence,
    assert_valid_state_transition,
};

#[cfg(feature = "financial")]
pub use crate::financial::{
    assert_fee_reduces_profit, assert_no_overfill, assert_pnl_sign_correct,
    assert_position_size_conserved, assert_total_pnl_conservation, profitable_long_pair,
    profitable_short_pair, valid_price, FinancialProperties,
};

#[cfg(feature = "serialization")]
pub use crate::serialization::{
    assert_bincode_deterministic, assert_bincode_roundtrip, assert_json_deterministic,
    assert_json_roundtrip,
};

#[cfg(feature = "temporal")]
pub use crate::temporal::{
    assert_min_gaps, assert_monotonic, assert_no_large_gaps, assert_valid_range, monotonic_timestamps,
    timestamp_pair, valid_timestamp,
};

#[cfg(feature = "trading")]
pub use crate::trading::{
    bearish_candle, bullish_candle, candle_sequence, valid_candle, valid_order, Candle, Order,
    OrderStatus, OrderType, Side,
};
