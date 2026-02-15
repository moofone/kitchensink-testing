//! Chaos/fault-injection law helpers used by integration tests.

use std::fmt::Debug;
use std::marker::PhantomData;

/// Repeatedly execute an operation until success, allowing retryable failures up to a maximum budget.
///
/// Returns the number of attempts used. Panics when the operation fails without a successful result.
pub fn assert_retries_to_expected_success<T, E, Op, IsRetryable>(
    max_attempts: usize,
    operation: Op,
    is_retryable: IsRetryable,
    expected: T,
) -> usize
where
    Op: Fn() -> Result<T, E>,
    IsRetryable: Fn(&E) -> bool,
    T: PartialEq + Debug,
{
    let mut attempts = 0usize;
    for _ in 0..max_attempts {
        attempts += 1;
        match operation() {
            Ok(result) => {
                assert_eq!(result, expected);
                return attempts;
            }
            Err(error) => {
                assert!(
                    is_retryable(&error),
                    "non-retryable error encountered at attempt {attempts} before success"
                );
            }
        }
    }
    panic!(
        "operation did not succeed after {max_attempts} attempts and all failures were retryable"
    );
}

/// Repeatedly execute an operation until it succeeds or fails with a non-retryable error.
///
/// Returns the attempt at which execution stopped.
pub fn assert_retry_stops_after_permanent_error<T, E, Op, IsRetryable>(
    max_attempts: usize,
    operation: Op,
    is_retryable: IsRetryable,
) -> usize
where
    Op: Fn() -> Result<T, E>,
    IsRetryable: Fn(&E) -> bool,
{
    let mut attempts = 0usize;
    for _ in 0..max_attempts {
        attempts += 1;
        match operation() {
            Ok(_) => return attempts,
            Err(error) => {
                if !is_retryable(&error) {
                    return attempts;
                }
            }
        }
    }
    attempts
}

/// Repeatedly execute an operation up to a budget. If no success occurs, execute `fallback`.
///
/// Returns attempts performed; calls `verify_fallback` once for fallback values.
pub fn assert_retry_fallback<T, E, F, Op, IsRetryable, VerifyFallback>(
    max_attempts: usize,
    operation: Op,
    is_retryable: IsRetryable,
    fallback: F,
    verify_fallback: VerifyFallback,
) -> usize
where
    Op: Fn() -> Result<T, E>,
    IsRetryable: Fn(&E) -> bool,
    F: Fn() -> T,
    VerifyFallback: Fn(&T),
{
    let mut attempts = 0usize;
    for _ in 0..max_attempts {
        attempts += 1;
        match operation() {
            Ok(_) => return attempts,
            Err(error) => {
                if !is_retryable(&error) {
                    let fallback_value = fallback();
                    verify_fallback(&fallback_value);
                    return attempts;
                }
            }
        }
    }
    let fallback_value = fallback();
    verify_fallback(&fallback_value);
    attempts
}

/// Law-like wrapper for retry-until-success checks.
pub struct RetryEventuallySucceedsLaw<Op, IsRetryable, T, E> {
    /// Maximum number of attempts allowed.
    pub max_attempts: usize,
    /// Operation being retried.
    pub operation: Op,
    /// Predicate to determine if an error is retryable.
    pub is_retryable: IsRetryable,
    /// Expected success value.
    pub expected: T,
    _error_type: PhantomData<E>,
}

impl<Op, IsRetryable, T, E> RetryEventuallySucceedsLaw<Op, IsRetryable, T, E>
where
    Op: Fn() -> Result<T, E>,
    IsRetryable: Fn(&E) -> bool,
    T: Clone + PartialEq + Debug,
{
    /// Human-readable law name.
    #[must_use]
    pub fn name(&self) -> &'static str {
        "assert_retries_to_expected_success"
    }

    /// Execute the law assertion.
    pub fn check(&self) {
        assert_retries_to_expected_success(
            self.max_attempts,
            &self.operation,
            &self.is_retryable,
            self.expected.clone(),
        );
    }

    /// Construct a retry-until-success law wrapper.
    pub fn new(
        max_attempts: usize,
        operation: Op,
        is_retryable: IsRetryable,
        expected: T,
    ) -> Self {
        Self {
            max_attempts,
            operation,
            is_retryable,
            expected,
            _error_type: PhantomData,
        }
    }
}

/// Law-like wrapper for retry-stop-on-permanent-error checks.
pub struct RetryStopsAfterPermanentErrorLaw<Op, IsRetryable> {
    /// Maximum number of attempts allowed.
    pub max_attempts: usize,
    /// Operation being retried.
    pub operation: Op,
    /// Predicate to determine if an error is retryable.
    pub is_retryable: IsRetryable,
}

impl<Op, IsRetryable, E> RetryStopsAfterPermanentErrorLaw<Op, IsRetryable>
where
    Op: Fn() -> Result<(), E>,
    IsRetryable: Fn(&E) -> bool,
{
    /// Human-readable law name.
    #[must_use]
    pub fn name(&self) -> &'static str {
        "assert_retry_stops_after_permanent_error"
    }

    /// Execute the law assertion.
    pub fn check(&self) {
        let _ = assert_retry_stops_after_permanent_error(
            self.max_attempts,
            &self.operation,
            &self.is_retryable,
        );
    }
}

/// Law-like wrapper for retry-with-fallback checks.
pub struct RetryFallbackLaw<Op, IsRetryable, Fallback, VerifyFallback, T, E> {
    /// Maximum number of attempts allowed.
    pub max_attempts: usize,
    /// Operation being retried.
    pub operation: Op,
    /// Predicate to determine if an error is retryable.
    pub is_retryable: IsRetryable,
    /// Fallback provider used when retries are exhausted.
    pub fallback: Fallback,
    /// Fallback verification callback.
    pub verify_fallback: VerifyFallback,
    _result_type: PhantomData<T>,
    _error_type: PhantomData<E>,
}

impl<Op, IsRetryable, Fallback, VerifyFallback, T, E> RetryFallbackLaw<Op, IsRetryable, Fallback, VerifyFallback, T, E>
where
    Op: Fn() -> Result<T, E>,
    IsRetryable: Fn(&E) -> bool,
    Fallback: Fn() -> T,
    VerifyFallback: Fn(&T),
{
    /// Human-readable law name.
    #[must_use]
    pub fn name(&self) -> &'static str {
        "assert_retry_fallback"
    }

    /// Execute the law assertion.
    pub fn check(&self) {
        let _ = assert_retry_fallback(
            self.max_attempts,
            &self.operation,
            &self.is_retryable,
            &self.fallback,
            &self.verify_fallback,
        );
    }

    /// Construct a retry-with-fallback law wrapper.
    pub fn new(
        max_attempts: usize,
        operation: Op,
        is_retryable: IsRetryable,
        fallback: Fallback,
        verify_fallback: VerifyFallback,
    ) -> Self {
        Self {
            max_attempts,
            operation,
            is_retryable,
            fallback,
            verify_fallback,
            _result_type: PhantomData,
            _error_type: PhantomData,
        }
    }
}
