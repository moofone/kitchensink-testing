//! Curated prelude imports for day-to-day property testing.

pub use proptest::prelude::*;

pub use crate::generators::collections::{unique_vec, vec_of};
pub use crate::generators::decimals::tick_aligned;
pub use crate::generators::edge_values::{
    f64_edge_values, finite_f64_edge_values, i64_edge_values, u64_edge_values, with_none,
};
pub use crate::generators::identifiers::{alphanumeric_id, prefixed_id};
pub use crate::generators::numeric::{bounded_f64, finite_f64, non_negative_f64, positive_f64};
pub use crate::generators::temporal::{
    monotonic_timestamps, timestamp_pair, valid_timestamp_millis,
};

pub use crate::law::algebraic::{
    assert_associative, assert_commutative, assert_distributive, assert_identity,
};
pub use crate::law::invariants::{
    assert_all_in_range, assert_approx_eq, assert_monotonic_increasing,
};
pub use crate::law::stateful::{
    assert_idempotent, assert_involutive, assert_state_invariant, assert_valid_state_sequence,
    assert_valid_state_transition,
};

#[cfg(feature = "serialization")]
pub use crate::law::serialization::{assert_bincode_deterministic, assert_bincode_roundtrip};
pub use crate::law::serialization::{assert_json_deterministic, assert_json_roundtrip};

#[cfg(feature = "mutation")]
pub use crate::mutation::{MutationConfig, MutationOutcome, MutationStatus, RunSnapshot};
