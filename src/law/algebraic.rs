//! Algebraic law assertions.

use std::fmt::Debug;

/// Assert commutativity: `f(a, b) == f(b, a)`.
pub fn assert_commutative<T, F, R>(a: T, b: T, f: F)
where
    T: Clone + Debug,
    F: Fn(T, T) -> R,
    R: PartialEq + Debug,
{
    let left = f(a.clone(), b.clone());
    let right = f(b, a);
    assert_eq!(left, right, "operation should be commutative");
}

/// Assert associativity: `f(f(a,b),c) == f(a,f(b,c))`.
pub fn assert_associative<T, F>(a: T, b: T, c: T, f: F)
where
    T: Clone + PartialEq + Debug,
    F: Fn(T, T) -> T,
{
    let left = f(f(a.clone(), b.clone()), c.clone());
    let right = f(a, f(b, c));
    assert_eq!(left, right, "operation should be associative");
}

/// Assert identity element: `f(a, id) == a` and `f(id, a) == a`.
pub fn assert_identity<T, F>(a: T, identity: T, f: F)
where
    T: Clone + PartialEq + Debug,
    F: Fn(T, T) -> T,
{
    let right = f(a.clone(), identity.clone());
    let left = f(identity, a.clone());
    assert_eq!(right, a, "right identity should hold");
    assert_eq!(left, a, "left identity should hold");
}

/// Assert distributivity: `f(a, g(b,c)) == g(f(a,b), f(a,c))`.
pub fn assert_distributive<T, F, G>(a: T, b: T, c: T, f: F, g: G)
where
    T: Clone + PartialEq + Debug,
    F: Fn(T, T) -> T,
    G: Fn(T, T) -> T,
{
    let left = f(a.clone(), g(b.clone(), c.clone()));
    let right = g(f(a.clone(), b), f(a, c));
    assert_eq!(left, right, "left distributivity should hold");
}
