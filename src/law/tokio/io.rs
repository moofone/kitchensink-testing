//! I/O laws for Tokio-based systems.

use std::future::Future;

/// Observable outcomes for partial read/write handling checks.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PartialIoObservation {
    /// Number of bytes the scenario expected to transfer.
    pub expected_bytes: usize,
    /// Number of bytes the scenario actually transferred.
    pub observed_bytes: usize,
    /// Number of partial read events observed.
    pub partial_read_events: usize,
    /// Number of partial write events observed.
    pub partial_write_events: usize,
}

/// Application-defined partial I/O probe.
pub trait PartialIoProbe {
    /// Execute a partial I/O scenario and report transfer observations.
    fn observe_partial_io(&self) -> impl Future<Output = PartialIoObservation>;
}

/// Assert that partial reads/writes are handled without losing bytes.
pub async fn assert_handles_partial_io<P>(probe: &P, require_partial_events: bool)
where
    P: PartialIoProbe,
{
    let observation = probe.observe_partial_io().await;
    assert_eq!(
        observation.expected_bytes, observation.observed_bytes,
        "partial I/O scenario transferred wrong byte count (expected={}, observed={})",
        observation.expected_bytes, observation.observed_bytes
    );
    if require_partial_events {
        assert!(
            observation.partial_read_events > 0 || observation.partial_write_events > 0,
            "partial I/O scenario did not exercise partial reads or writes"
        );
    }
}

/// Observable outcomes for transient error retry checks.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TransientIoRetryObservation {
    /// Number of transient errors encountered.
    pub transient_errors: usize,
    /// Number of permanent errors encountered.
    pub permanent_errors: usize,
    /// Number of retry attempts performed.
    pub retry_attempts: usize,
    /// Whether the operation succeeded eventually.
    pub succeeded: bool,
}

/// Application-defined transient retry probe.
pub trait TransientIoRetryProbe {
    /// Execute transient failure behavior and report retry observations.
    fn observe_transient_retry(&self) -> impl Future<Output = TransientIoRetryObservation>;
}

/// Assert transient I/O errors are retried within a budget and eventually succeed.
pub async fn assert_retries_transient_io_errors<P>(probe: &P, max_retries: usize)
where
    P: TransientIoRetryProbe,
{
    let observation = probe.observe_transient_retry().await;
    assert_eq!(
        observation.permanent_errors, 0,
        "transient retry scenario observed {} permanent error(s)",
        observation.permanent_errors
    );
    if observation.transient_errors > 0 {
        assert!(
            observation.retry_attempts >= observation.transient_errors,
            "retry attempts ({}) were fewer than transient errors ({})",
            observation.retry_attempts,
            observation.transient_errors
        );
    }
    assert!(
        observation.retry_attempts <= max_retries,
        "retry attempts ({}) exceeded max_retries ({})",
        observation.retry_attempts,
        max_retries
    );
    assert!(
        observation.succeeded,
        "operation did not succeed after transient retries"
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    struct HealthyIoProbe;

    impl PartialIoProbe for HealthyIoProbe {
        fn observe_partial_io(&self) -> impl Future<Output = PartialIoObservation> {
            std::future::ready(PartialIoObservation {
                expected_bytes: 512,
                observed_bytes: 512,
                partial_read_events: 2,
                partial_write_events: 1,
            })
        }
    }

    impl TransientIoRetryProbe for HealthyIoProbe {
        fn observe_transient_retry(&self) -> impl Future<Output = TransientIoRetryObservation> {
            std::future::ready(TransientIoRetryObservation {
                transient_errors: 3,
                permanent_errors: 0,
                retry_attempts: 3,
                succeeded: true,
            })
        }
    }

    #[tokio::test(flavor = "current_thread")]
    async fn io_laws_accept_healthy_observations() {
        let probe = HealthyIoProbe;
        assert_handles_partial_io(&probe, true).await;
        assert_retries_transient_io_errors(&probe, 5).await;
    }
}
