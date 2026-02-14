//! Stateful property templates.
//!
//! Reusable templates for testing stateful operations like state transitions,
//! idempotency, and state machine properties.
//!
//! # Example
//!
//! ```rust,ignore
//! use rust_pbt::stateful::assert_idempotent;
//!
//! // Absolute value is idempotent
//! assert_idempotent(-5, |x| x.abs());  // passes
//! ```

use std::fmt::Debug;

/// Assert that a state transition is valid.
///
/// # Panics
///
/// Panics if the state transition is invalid according to the provided predicate.
///
/// # Example
///
/// ```rust
/// use rust_pbt::stateful::assert_valid_state_transition;
///
/// #[derive(Debug, PartialEq)]
/// enum State { Pending, Active, Closed }
///
/// #[derive(Debug)]
/// enum Event { Activate, Close }
///
/// fn is_valid(from: &State, event: &Event, to: &State) -> bool {
///     matches!(
///         (from, event, to),
///         (State::Pending, Event::Activate, State::Active) |
///         (State::Active, Event::Close, State::Closed)
///     )
/// }
///
/// assert_valid_state_transition(
///     &State::Pending,
///     &Event::Activate,
///     &State::Active,
///     is_valid
/// );
/// ```
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
        "Invalid state transition from {:?} via {:?} to {:?}",
        initial,
        event,
        final_state
    );
}

/// Assert that a function is idempotent: `f(f(x)) = f(x)`
///
/// An idempotent function produces the same result when applied multiple times.
///
/// # Panics
///
/// Panics if the function is not idempotent for the given input.
///
/// # Example
///
/// ```rust
/// use rust_pbt::stateful::assert_idempotent;
///
/// // Absolute value is idempotent
/// assert_idempotent(-5, |x: i32| x.abs());
///
/// // Normalization should be idempotent
/// assert_idempotent(10.5, |x: f64| (x * 100.0).round() / 100.0);
/// ```
pub fn assert_idempotent<T, F>(value: T, f: F)
where
    T: Clone + PartialEq + Debug,
    F: Fn(T) -> T,
{
    let once = f(value.clone());
    let twice = f(once.clone());
    assert_eq!(
        once, twice,
        "Function should be idempotent: f(f({:?})) = {:?} != f({:?}) = {:?}",
        value, twice, value, once
    );
}

/// Assert that a function is involutive: `f(f(x)) = x`
///
/// An involution is its own inverse.
///
/// # Panics
///
/// Panics if the function is not involutive for the given input.
///
/// # Example
///
/// ```rust
/// use rust_pbt::stateful::assert_involutive;
///
/// // Negation is involutive
/// assert_involutive(5, |x: i32| -x);
///
/// // NOT operation is involutive
/// assert_involutive(true, |x: bool| !x);
/// ```
pub fn assert_involutive<T, F>(value: T, f: F)
where
    T: Clone + PartialEq + Debug,
    F: Fn(T) -> T,
{
    let twice = f(f(value.clone()));
    assert_eq!(
        value, twice,
        "Function should be involutive: f(f({:?})) = {:?} != {:?}",
        value, twice, value
    );
}

/// Assert that a state machine never enters an invalid state.
///
/// # Panics
///
/// Panics if the state invariant is violated.
///
/// # Example
///
/// ```rust
/// use rust_pbt::stateful::assert_state_invariant;
///
/// #[derive(Debug)]
/// struct OrderState {
///     filled: f64,
///     total: f64,
/// }
///
/// fn is_valid_order_state(state: &OrderState) -> bool {
///     state.filled >= 0.0 && state.filled <= state.total
/// }
///
/// let state = OrderState { filled: 50.0, total: 100.0 };
/// assert_state_invariant(&state, is_valid_order_state);
/// ```
pub fn assert_state_invariant<S>(state: &S, is_valid: fn(&S) -> bool)
where
    S: Debug,
{
    assert!(is_valid(state), "State invariant violated: {:?}", state);
}

/// Assert that a sequence of state transitions is valid.
///
/// # Panics
///
/// Panics if any transition in the sequence is invalid.
///
/// # Example
///
/// ```rust
/// use rust_pbt::stateful::assert_valid_state_sequence;
///
/// #[derive(Debug, PartialEq, Clone)]
/// enum State { Pending, Active, Closed }
///
/// fn is_valid_transition(from: &State, to: &State) -> bool {
///     matches!(
///         (from, to),
///         (State::Pending, State::Active) |
///         (State::Active, State::Closed)
///     )
/// }
///
/// let states = vec![State::Pending, State::Active, State::Closed];
/// assert_valid_state_sequence(&states, is_valid_transition);
/// ```
pub fn assert_valid_state_sequence<S>(states: &[S], is_valid_transition: fn(&S, &S) -> bool)
where
    S: Debug,
{
    for window in states.windows(2) {
        assert!(
            is_valid_transition(&window[0], &window[1]),
            "Invalid transition from {:?} to {:?}",
            window[0],
            window[1]
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Debug, PartialEq, Clone)]
    enum TestState {
        Start,
        Middle,
        End,
    }

    #[derive(Debug)]
    enum TestEvent {
        Next,
        Finish,
    }

    fn is_valid_transition(from: &TestState, event: &TestEvent, to: &TestState) -> bool {
        matches!(
            (from, event, to),
            (TestState::Start, TestEvent::Next, TestState::Middle)
                | (TestState::Middle, TestEvent::Finish, TestState::End)
        )
    }

    #[test]
    fn test_valid_state_transition() {
        assert_valid_state_transition(
            &TestState::Start,
            &TestEvent::Next,
            &TestState::Middle,
            is_valid_transition,
        );
    }

    #[test]
    #[should_panic]
    fn test_invalid_state_transition() {
        assert_valid_state_transition(
            &TestState::Start,
            &TestEvent::Finish,
            &TestState::End,
            is_valid_transition,
        );
    }

    #[test]
    fn test_idempotent_abs() {
        assert_idempotent(-5, |x: i32| x.abs());
    }

    #[test]
    fn test_involutive_negation() {
        assert_involutive(5, |x: i32| -x);
    }

    #[test]
    fn test_involutive_not() {
        assert_involutive(true, |x: bool| !x);
    }

    #[derive(Debug)]
    struct TestOrderState {
        filled: f64,
        total: f64,
    }

    fn is_valid_order_state(state: &TestOrderState) -> bool {
        state.filled >= 0.0 && state.filled <= state.total
    }

    #[test]
    fn test_state_invariant() {
        let state = TestOrderState {
            filled: 50.0,
            total: 100.0,
        };
        assert_state_invariant(&state, is_valid_order_state);
    }

    #[test]
    #[should_panic]
    fn test_state_invariant_violation() {
        let state = TestOrderState {
            filled: 150.0,
            total: 100.0,
        };
        assert_state_invariant(&state, is_valid_order_state);
    }

    fn is_valid_seq_transition(from: &TestState, to: &TestState) -> bool {
        matches!(
            (from, to),
            (TestState::Start, TestState::Middle) | (TestState::Middle, TestState::End)
        )
    }

    #[test]
    fn test_valid_state_sequence() {
        let states = vec![TestState::Start, TestState::Middle, TestState::End];
        assert_valid_state_sequence(&states, is_valid_seq_transition);
    }

    #[test]
    #[should_panic]
    fn test_invalid_state_sequence() {
        let states = vec![TestState::Start, TestState::End];
        assert_valid_state_sequence(&states, is_valid_seq_transition);
    }
}
