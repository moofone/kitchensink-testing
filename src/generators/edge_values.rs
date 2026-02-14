//! Strategy helpers for boundary and edge-case values.

use proptest::prelude::*;

/// Include explicit `None` cases with normal values.
pub fn with_none<T>(strategy: T) -> impl Strategy<Value = Option<T::Value>>
where
    T: Strategy,
    T::Value: Clone + std::fmt::Debug,
{
    prop_oneof![
        1 => Just(Option::<T::Value>::None),
        3 => strategy.prop_map(Some),
    ]
}

/// Edge-case `i64` values including bounds and common special values.
pub fn i64_edge_values() -> impl Strategy<Value = i64> {
    prop_oneof![
        1 => Just(i64::MIN),
        1 => Just(-1_i64),
        1 => Just(0_i64),
        1 => Just(1_i64),
        1 => Just(i64::MAX),
        3 => (i64::MIN + 2)..=(i64::MAX - 1),
    ]
}

/// Edge-case `u64` values including bounds and low values.
pub fn u64_edge_values() -> impl Strategy<Value = u64> {
    prop_oneof![
        1 => Just(0_u64),
        1 => Just(1_u64),
        1 => Just(u64::MAX),
        3 => 2_u64..=(u64::MAX - 1),
    ]
}

/// Edge-case `f64` values including zero/negative zero, infinities, and NaN.
pub fn f64_edge_values() -> impl Strategy<Value = f64> {
    prop_oneof![
        1 => Just(-0.0_f64),
        1 => Just(0.0_f64),
        1 => Just(1.0_f64),
        1 => Just(-1.0_f64),
        1 => Just(f64::MAX),
        1 => Just(f64::MIN),
        1 => Just(f64::INFINITY),
        1 => Just(f64::NEG_INFINITY),
        1 => Just(f64::NAN),
        4 => (-1_000_000.0_f64)..=1_000_000.0_f64,
    ]
}

/// Finite `f64` edge values without NaN or infinities.
pub fn finite_f64_edge_values() -> impl Strategy<Value = f64> {
    prop_oneof![
        1 => Just(0.0_f64),
        1 => Just(-0.0_f64),
        1 => Just(1.0_f64),
        1 => Just(-1.0_f64),
        1 => Just(f64::MAX),
        1 => Just(f64::MIN),
        2 => (-1_000_000.0_f64)..=1_000_000.0_f64,
    ]
}
