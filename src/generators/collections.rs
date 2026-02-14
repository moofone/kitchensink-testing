//! Collection-oriented generators.

use std::collections::HashSet;
use std::fmt::Debug;
use std::hash::Hash;

use proptest::collection::SizeRange;
use proptest::prelude::*;

/// Generate vectors with configurable length range.
pub fn vec_of<S>(element: S, len: impl Into<SizeRange>) -> impl Strategy<Value = Vec<S::Value>>
where
    S: Strategy,
{
    prop::collection::vec(element, len)
}

/// Generate vectors where all elements are unique.
pub fn unique_vec<S>(
    element: S,
    len: std::ops::RangeInclusive<usize>,
) -> impl Strategy<Value = Vec<S::Value>>
where
    S: Strategy,
    S::Value: Eq + Hash + Clone + Debug,
{
    prop::collection::vec(element, len).prop_filter("elements must be unique", |values| {
        let mut seen = HashSet::with_capacity(values.len());
        for value in values {
            if !seen.insert(value.clone()) {
                return false;
            }
        }
        true
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    proptest! {
        #[test]
        fn unique_vec_has_no_duplicates(values in unique_vec(0u8..20, 0..=10)) {
            let mut sorted = values.clone();
            sorted.sort_unstable();
            sorted.dedup();
            prop_assert_eq!(sorted.len(), values.len());
        }
    }
}
