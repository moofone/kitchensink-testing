//! Collection property templates.
//!
//! Reusable templates for testing collection properties like size preservation,
//! uniqueness, ordering, etc.
//!
//! # Example
//!
//! ```rust,ignore
//! use rust_pbt::collections::assert_sorted;
//!
//! let values = vec![1, 2, 3, 4, 5];
//! assert_sorted(&values);  // passes
//! ```

use std::collections::HashSet;
use std::fmt::Debug;
use std::hash::Hash;

/// Assert that a transformation preserves collection size.
///
/// # Panics
///
/// Panics if the input and output collections have different sizes.
///
/// # Example
///
/// ```rust
/// use rust_pbt::collections::assert_size_preserved;
///
/// let input = vec![1, 2, 3, 4, 5];
/// let output = input.iter().map(|x| x * 2).collect::<Vec<_>>();
/// assert_size_preserved(&input, &output);
/// ```
pub fn assert_size_preserved<T, U>(input: &[T], output: &[U]) {
    assert_eq!(
        input.len(),
        output.len(),
        "Collection size should be preserved: input {} != output {}",
        input.len(),
        output.len()
    );
}

/// Assert that a collection contains no duplicates.
///
/// # Panics
///
/// Panics if the collection contains duplicate elements.
///
/// # Example
///
/// ```rust
/// use rust_pbt::collections::assert_no_duplicates;
///
/// let values = vec![1, 2, 3, 4, 5];
/// assert_no_duplicates(&values);  // passes
/// ```
pub fn assert_no_duplicates<T>(values: &[T])
where
    T: Eq + Hash + Debug,
{
    let unique: HashSet<_> = values.iter().collect();
    assert_eq!(
        values.len(),
        unique.len(),
        "Collection should contain no duplicates. Size: {}, Unique: {}",
        values.len(),
        unique.len()
    );
}

/// Assert that a collection is sorted in ascending order.
///
/// # Panics
///
/// Panics if the collection is not sorted.
///
/// # Example
///
/// ```rust
/// use rust_pbt::collections::assert_sorted;
///
/// let values = vec![1, 2, 3, 4, 5];
/// assert_sorted(&values);  // passes
/// ```
pub fn assert_sorted<T>(values: &[T])
where
    T: Ord + Debug,
{
    for window in values.windows(2) {
        assert!(
            window[0] <= window[1],
            "Collection should be sorted: {:?} > {:?}",
            window[0],
            window[1]
        );
    }
}

/// Assert that a collection is sorted in descending order.
///
/// # Panics
///
/// Panics if the collection is not sorted in descending order.
///
/// # Example
///
/// ```rust
/// use rust_pbt::collections::assert_sorted_descending;
///
/// let values = vec![5, 4, 3, 2, 1];
/// assert_sorted_descending(&values);  // passes
/// ```
pub fn assert_sorted_descending<T>(values: &[T])
where
    T: Ord + Debug,
{
    for window in values.windows(2) {
        assert!(
            window[0] >= window[1],
            "Collection should be sorted descending: {:?} < {:?}",
            window[0],
            window[1]
        );
    }
}

/// Assert that two collections contain the same elements (regardless of order).
///
/// # Panics
///
/// Panics if the collections contain different elements.
///
/// # Example
///
/// ```rust
/// use rust_pbt::collections::assert_same_elements;
///
/// let a = vec![1, 2, 3];
/// let b = vec![3, 1, 2];
/// assert_same_elements(&a, &b);  // passes
/// ```
pub fn assert_same_elements<T>(a: &[T], b: &[T])
where
    T: Eq + Hash + Debug,
{
    let set_a: HashSet<_> = a.iter().collect();
    let set_b: HashSet<_> = b.iter().collect();
    assert_eq!(set_a, set_b, "Collections should contain the same elements");
}

/// Assert that all elements in a collection satisfy a predicate.
///
/// # Panics
///
/// Panics if any element does not satisfy the predicate.
///
/// # Example
///
/// ```rust
/// use rust_pbt::collections::assert_all_satisfy;
///
/// let values = vec![2, 4, 6, 8];
/// assert_all_satisfy(&values, |x| x % 2 == 0);  // passes
/// ```
pub fn assert_all_satisfy<T, F>(values: &[T], predicate: F)
where
    T: Debug,
    F: Fn(&T) -> bool,
{
    for value in values {
        assert!(
            predicate(value),
            "Element {:?} does not satisfy predicate",
            value
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_size_preserved() {
        let input = vec![1, 2, 3];
        let output = vec![2, 4, 6];
        assert_size_preserved(&input, &output);
    }

    #[test]
    #[should_panic]
    fn test_size_preserved_fails() {
        let input = vec![1, 2, 3];
        let output = vec![2, 4];
        assert_size_preserved(&input, &output);
    }

    #[test]
    fn test_no_duplicates() {
        let values = vec![1, 2, 3, 4, 5];
        assert_no_duplicates(&values);
    }

    #[test]
    #[should_panic]
    fn test_no_duplicates_fails() {
        let values = vec![1, 2, 3, 2, 5];
        assert_no_duplicates(&values);
    }

    #[test]
    fn test_sorted() {
        let values = vec![1, 2, 3, 4, 5];
        assert_sorted(&values);
    }

    #[test]
    #[should_panic]
    fn test_sorted_fails() {
        let values = vec![1, 3, 2, 4];
        assert_sorted(&values);
    }

    #[test]
    fn test_sorted_descending() {
        let values = vec![5, 4, 3, 2, 1];
        assert_sorted_descending(&values);
    }

    #[test]
    fn test_same_elements() {
        let a = vec![1, 2, 3];
        let b = vec![3, 1, 2];
        assert_same_elements(&a, &b);
    }

    #[test]
    fn test_all_satisfy() {
        let values = vec![2, 4, 6, 8];
        assert_all_satisfy(&values, |x| x % 2 == 0);
    }
}
