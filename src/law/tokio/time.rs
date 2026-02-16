//! Time and scheduling laws for Tokio-based systems.

use std::future::Future;
use std::time::Duration;

/// Observable outcomes for timeout behavior checks.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TimeoutObservation {
    /// Timeout configured by the scenario.
    pub timeout: Duration,
    /// Wall-clock elapsed duration observed by the scenario.
    pub elapsed: Duration,
    /// Whether the operation timed out.
    pub timed_out: bool,
    /// Whether the operation still completed after timeout cancellation.
    pub completed_after_timeout: bool,
}

/// Application-defined timeout behavior probe.
pub trait TimeoutBehaviorProbe {
    /// Execute timeout behavior and report observations.
    fn observe_timeout_behavior(&self) -> impl Future<Output = TimeoutObservation>;
}

/// Assert timeout behavior with a maximum allowed scheduling overrun.
pub async fn assert_timeout_behavior<P>(probe: &P, max_overrun: Duration)
where
    P: TimeoutBehaviorProbe,
{
    let observation = probe.observe_timeout_behavior().await;
    assert!(
        observation.timed_out,
        "timeout scenario did not time out as expected"
    );
    assert!(
        !observation.completed_after_timeout,
        "timeout scenario completed operation after timeout"
    );
    assert!(
        observation.elapsed <= observation.timeout + max_overrun,
        "timeout exceeded allowed overrun (elapsed={:?}, timeout={:?}, max_overrun={:?})",
        observation.elapsed,
        observation.timeout,
        max_overrun
    );
}

/// Observable outcomes for retry backoff delay checks.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BackoffObservation {
    /// Attempt-to-attempt delays observed in the scenario.
    pub delays: Vec<Duration>,
}

/// Application-defined retry backoff probe.
pub trait BackoffProbe {
    /// Execute backoff behavior and report observed delays.
    fn observe_backoff(&self) -> impl Future<Output = BackoffObservation>;
}

/// Assert backoff delays stay within bounds and do not shrink between attempts.
pub async fn assert_backoff_bounds<P>(probe: &P, min_delay: Duration, max_delay: Duration)
where
    P: BackoffProbe,
{
    assert!(
        min_delay <= max_delay,
        "min_delay ({:?}) must be <= max_delay ({:?})",
        min_delay,
        max_delay
    );

    let observation = probe.observe_backoff().await;
    assert!(
        !observation.delays.is_empty(),
        "backoff scenario reported no delays"
    );

    for delay in &observation.delays {
        assert!(
            *delay >= min_delay && *delay <= max_delay,
            "backoff delay {:?} outside bounds [{:?}, {:?}]",
            delay,
            min_delay,
            max_delay
        );
    }

    for window in observation.delays.windows(2) {
        assert!(
            window[1] >= window[0],
            "backoff delay shrank from {:?} to {:?}",
            window[0],
            window[1]
        );
    }
}

/// Observable outcomes for interval drift checks.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IntervalDriftObservation {
    /// Absolute drift values observed for each tick.
    pub absolute_drifts: Vec<Duration>,
}

/// Application-defined interval drift probe.
pub trait IntervalDriftProbe {
    /// Execute an interval schedule and report absolute drift per tick.
    fn observe_interval_drift(&self) -> impl Future<Output = IntervalDriftObservation>;
}

/// Assert every observed interval drift stays below a maximum tolerance.
pub async fn assert_interval_no_drift<P>(probe: &P, max_drift: Duration)
where
    P: IntervalDriftProbe,
{
    let observation = probe.observe_interval_drift().await;
    assert!(
        !observation.absolute_drifts.is_empty(),
        "interval drift scenario reported no ticks"
    );

    for drift in &observation.absolute_drifts {
        assert!(
            *drift <= max_drift,
            "interval drift {:?} exceeded max drift {:?}",
            drift,
            max_drift
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct HealthyTimeProbe;

    impl TimeoutBehaviorProbe for HealthyTimeProbe {
        fn observe_timeout_behavior(&self) -> impl Future<Output = TimeoutObservation> {
            std::future::ready(TimeoutObservation {
                timeout: Duration::from_millis(50),
                elapsed: Duration::from_millis(53),
                timed_out: true,
                completed_after_timeout: false,
            })
        }
    }

    impl BackoffProbe for HealthyTimeProbe {
        fn observe_backoff(&self) -> impl Future<Output = BackoffObservation> {
            std::future::ready(BackoffObservation {
                delays: vec![
                    Duration::from_millis(5),
                    Duration::from_millis(10),
                    Duration::from_millis(15),
                ],
            })
        }
    }

    impl IntervalDriftProbe for HealthyTimeProbe {
        fn observe_interval_drift(&self) -> impl Future<Output = IntervalDriftObservation> {
            std::future::ready(IntervalDriftObservation {
                absolute_drifts: vec![
                    Duration::from_millis(1),
                    Duration::from_millis(2),
                    Duration::from_millis(3),
                ],
            })
        }
    }

    #[tokio::test(flavor = "current_thread")]
    async fn time_laws_accept_healthy_observations() {
        let probe = HealthyTimeProbe;
        assert_timeout_behavior(&probe, Duration::from_millis(5)).await;
        assert_backoff_bounds(&probe, Duration::from_millis(1), Duration::from_millis(20)).await;
        assert_interval_no_drift(&probe, Duration::from_millis(5)).await;
    }
}
