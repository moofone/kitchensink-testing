//! Stateful/system law assertions.

use std::fmt::Debug;

/// Assert valid transition under predicate.
pub fn assert_valid_state_transition<S, E>(
    initial: &S,
    event: &E,
    final_state: &S,
    is_valid_transition: fn(&S, &E, &S) -> bool,
) where
    S: Debug,
    E: Debug,
{
    assert!(
        is_valid_transition(initial, event, final_state),
        "invalid state transition from {:?} via {:?} to {:?}",
        initial,
        event,
        final_state
    );
}

/// Assert idempotence: `f(f(x)) == f(x)`.
pub fn assert_idempotent<T, F>(value: T, f: F)
where
    T: Clone + PartialEq + Debug,
    F: Fn(T) -> T,
{
    let once = f(value.clone());
    let twice = f(once.clone());
    assert_eq!(once, twice, "function should be idempotent");
}

/// Assert involution: `f(f(x)) == x`.
pub fn assert_involutive<T, F>(value: T, f: F)
where
    T: Clone + PartialEq + Debug,
    F: Fn(T) -> T,
{
    let twice = f(f(value.clone()));
    assert_eq!(value, twice, "function should be involutive");
}

/// Assert state invariant.
pub fn assert_state_invariant<S>(state: &S, is_valid: fn(&S) -> bool)
where
    S: Debug,
{
    assert!(is_valid(state), "state invariant violated: {:?}", state);
}

/// Assert all transitions in a sequence are valid.
pub fn assert_valid_state_sequence<S>(states: &[S], is_valid_transition: fn(&S, &S) -> bool)
where
    S: Debug,
{
    for window in states.windows(2) {
        assert!(
            is_valid_transition(&window[0], &window[1]),
            "invalid transition from {:?} to {:?}",
            window[0],
            window[1]
        );
    }
}
