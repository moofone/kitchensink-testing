//! # rust-pbt
//!
//! A comprehensive property-based testing framework for Rust with custom strategies,
//! property templates, and reference implementation oracles.
//!
//! ## Features
//!
//! - **Custom Strategies**: Domain-specific `proptest` strategies for prices, timestamps, and more
//! - **Property Templates**: Reusable assertion patterns (commutative, associative, idempotent, etc.)
//! - **Reference Implementations**: Slow-but-correct oracles for testing optimized code
//! - **Feature-gated**: Only include what you need
//!
//! ## Quick Start
//!
//! ```rust,ignore
//! use rust_pbt::prelude::*;
//!
//! proptest! {
//!     #[test]
//!     fn addition_is_commutative(a in 0..100i32, b in 0..100i32) {
//!         assert_commutative(a, b, |x, y| x + y);
//!     }
//! }
//! ```
//!
//! ## Feature Flags
//!
//! - `numeric` (default): Numeric property helpers
//! - `collections` (default): Collection property templates
//! - `arithmetic` (default): Arithmetic property templates
//! - `stateful` (default): Stateful property templates
//! - `financial`: Financial domain strategies and properties
//! - `serialization`: Serialization roundtrip testing
//! - `temporal`: Timestamp strategies
//! - `trading`: Trading domain strategies (candles, orders)
//! - `arbitrary`: Integration with the `arbitrary` crate
//! - `full`: All features enabled

#![cfg_attr(docsrs, feature(doc_cfg))]
#![warn(missing_docs)]

// Re-export proptest for convenience
pub use proptest;

// Core modules (always available)
pub mod prelude;

// Feature-gated modules
#[cfg(feature = "numeric")]
#[cfg_attr(docsrs, doc(cfg(feature = "numeric")))]
pub mod numeric;

#[cfg(feature = "collections")]
#[cfg_attr(docsrs, doc(cfg(feature = "collections")))]
pub mod collections;

#[cfg(feature = "arithmetic")]
#[cfg_attr(docsrs, doc(cfg(feature = "arithmetic")))]
pub mod arithmetic;

#[cfg(feature = "stateful")]
#[cfg_attr(docsrs, doc(cfg(feature = "stateful")))]
pub mod stateful;

#[cfg(feature = "financial")]
#[cfg_attr(docsrs, doc(cfg(feature = "financial")))]
pub mod financial;

#[cfg(feature = "serialization")]
#[cfg_attr(docsrs, doc(cfg(feature = "serialization")))]
pub mod serialization;

#[cfg(feature = "temporal")]
#[cfg_attr(docsrs, doc(cfg(feature = "temporal")))]
pub mod temporal;

#[cfg(feature = "trading")]
#[cfg_attr(docsrs, doc(cfg(feature = "trading")))]
pub mod trading;

#[cfg(feature = "arbitrary")]
#[cfg_attr(docsrs, doc(cfg(feature = "arbitrary")))]
pub use arbitrary;

/// Macro to generate property tests for serialization roundtrips.
///
/// # Example
/// ```rust,ignore
/// use rust_pbt::proptest_roundtrip;
/// use proptest::prelude::*;
///
/// proptest_roundtrip!(json_roundtrip, String, ".*", assert_json_roundtrip);
/// ```
#[cfg(feature = "serialization")]
#[macro_export]
macro_rules! proptest_roundtrip {
    ($test_name:ident, $type:ty, $strategy:expr, $format:ident) => {
        #[cfg(test)]
        mod $test_name {
            use super::*;
            use $crate::proptest::prelude::*;

            proptest! {
                #[test]
                fn roundtrip(value in $strategy) {
                    $crate::serialization::$format(&value);
                }
            }
        }
    };
}
