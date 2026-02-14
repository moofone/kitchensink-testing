//! Arithmetic property templates.
//!
//! Reusable templates for testing arithmetic properties like commutativity,
//! associativity, and identity elements.
//!
//! # Example
//!
//! ```rust,ignore
//! use rust_pbt::arithmetic::assert_commutative;
//!
//! assert_commutative(5, 3, |a, b| a + b);  // passes
//! assert_commutative(5, 3, |a, b| a - b);  // panics (subtraction is not commutative)
//! ```

use std::fmt::Debug;

/// Assert commutative property: `f(a, b) = f(b, a)`
///
/// # Panics
///
/// Panics if the operation is not commutative for the given inputs.
///
/// # Example
///
/// ```rust
/// use rust_pbt::arithmetic::assert_commutative;
///
/// assert_commutative(5, 3, |a, b| a + b);  // addition is commutative
/// assert_commutative(7, 4, |a, b| a * b);  // multiplication is commutative
/// ```
pub fn assert_commutative<T, F, R>(a: T, b: T, f: F)
where
    T: Clone + Debug,
    R: PartialEq + Debug,
    F: Fn(T, T) -> R,
{
    let a_for_msg = format!("{:?}", a);
    let b_for_msg = format!("{:?}", b);
    let forward = f(a.clone(), b.clone());
    let backward = f(b.clone(), a.clone());
    assert_eq!(
        forward,
        backward,
        "Operation should be commutative: f({}, {}) != f({}, {})",
        a_for_msg, b_for_msg, b_for_msg, a_for_msg
    );
}

/// Assert associative property: `f(f(a, b), c) = f(a, f(b, c))`
///
/// # Panics
///
/// Panics if the operation is not associative for the given inputs.
///
/// # Example
///
/// ```rust
/// use rust_pbt::arithmetic::assert_associative;
///
/// assert_associative(2, 3, 4, |a, b| a + b);  // addition is associative
/// assert_associative(2, 3, 4, |a, b| a * b);  // multiplication is associative
/// ```
pub fn assert_associative<T, F>(a: T, b: T, c: T, f: F)
where
    T: Clone + Debug + PartialEq,
    F: Fn(T, T) -> T,
{
    let a_for_msg = format!("{:?}", a);
    let b_for_msg = format!("{:?}", b);
    let c_for_msg = format!("{:?}", c);
    let left = f(f(a.clone(), b.clone()), c.clone());
    let right = f(a.clone(), f(b.clone(), c.clone()));
    assert_eq!(
        left, right,
        "Operation should be associative: f(f({}, {}), {}) != f({}, f({}, {}))",
        a_for_msg, b_for_msg, c_for_msg, a_for_msg, b_for_msg, c_for_msg
    );
}

/// Assert identity property: `f(a, identity) = a`
///
/// # Panics
///
/// Panics if the identity element does not preserve the value.
///
/// # Example
///
/// ```rust
/// use rust_pbt::arithmetic::assert_identity;
///
/// assert_identity(5, 0, |a, b| a + b);   // 0 is additive identity
/// assert_identity(5, 1, |a, b| a * b);   // 1 is multiplicative identity
/// ```
pub fn assert_identity<T, F>(a: T, identity: T, f: F)
where
    T: Clone + Debug + PartialEq,
    F: Fn(T, T) -> T,
{
    let a_for_msg = format!("{:?}", a);
    let result = f(a.clone(), identity);
    assert_eq!(
        result, a,
        "Identity element should not change value: f({}, identity) != {}",
        a_for_msg, a_for_msg
    );
}

/// Assert distributive property: `f(a, g(b, c)) = g(f(a, b), f(a, c))`
///
/// # Panics
///
/// Panics if the distributive property does not hold for the given inputs.
///
/// # Example
///
/// ```rust
/// use rust_pbt::arithmetic::assert_distributive;
///
/// // Multiplication distributes over addition: a * (b + c) = (a * b) + (a * c)
/// assert_distributive(2, 3, 4, |a, b| a * b, |a, b| a + b);
/// ```
pub fn assert_distributive<T, F, G>(a: T, b: T, c: T, f: F, g: G)
where
    T: Clone + Debug + PartialEq,
    F: Fn(T, T) -> T,
    G: Fn(T, T) -> T,
{
    let a_for_msg = format!("{:?}", a);
    let b_for_msg = format!("{:?}", b);
    let c_for_msg = format!("{:?}", c);
    let left = f(a.clone(), g(b.clone(), c.clone()));
    let right = g(f(a.clone(), b.clone()), f(a.clone(), c.clone()));
    assert_eq!(
        left, right,
        "Distributive property should hold: f({}, g({}, {})) != g(f({}, {}), f({}, {}))",
        a_for_msg, b_for_msg, c_for_msg, a_for_msg, b_for_msg, a_for_msg, c_for_msg
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_commutative_addition() {
        assert_commutative(5, 3, |a, b| a + b);
    }

    #[test]
    fn test_commutative_multiplication() {
        assert_commutative(7, 4, |a, b| a * b);
    }

    #[test]
    #[should_panic]
    fn test_commutative_subtraction_fails() {
        // Subtraction is not commutative
        assert_commutative(5, 3, |a, b| a - b);
    }

    #[test]
    fn test_associative_addition() {
        assert_associative(2, 3, 4, |a, b| a + b);
    }

    #[test]
    fn test_associative_multiplication() {
        assert_associative(2, 3, 4, |a, b| a * b);
    }

    #[test]
    fn test_identity_addition() {
        assert_identity(5, 0, |a, b| a + b);
    }

    #[test]
    fn test_identity_multiplication() {
        assert_identity(5, 1, |a, b| a * b);
    }

    #[test]
    fn test_distributive() {
        // 2 * (3 + 4) = (2 * 3) + (2 * 4)
        assert_distributive(2, 3, 4, |a, b| a * b, |a, b| a + b);
    }
}
