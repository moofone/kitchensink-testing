#![cfg(feature = "tokio-laws")]

use std::future::Future;
use std::time::Duration;

use rust_pbt::prelude::*;

struct Probe;

impl CancellationSafetyProbe for Probe {
    fn observe_cancellation_safety(&self) -> impl Future<Output = CancellationSafetyObservation> {
        std::future::ready(CancellationSafetyObservation {
            panicked_tasks: 0,
            completed_after_cancel: 0,
            side_effects_after_cancel: 0,
        })
    }
}

impl TaskLeakProbe for Probe {
    fn observe_task_leaks(&self) -> impl Future<Output = TaskLeakObservation> {
        std::future::ready(TaskLeakObservation {
            spawned_tasks: 4,
            finished_tasks: 3,
            aborted_tasks: 1,
            detached_tasks: 0,
        })
    }
}

impl GracefulShutdownProbe for Probe {
    fn observe_graceful_shutdown(&self) -> impl Future<Output = GracefulShutdownObservation> {
        std::future::ready(GracefulShutdownObservation {
            in_flight_tasks: 3,
            drained_tasks: 3,
            force_stopped_tasks: 0,
        })
    }
}

impl TimeoutBehaviorProbe for Probe {
    fn observe_timeout_behavior(&self) -> impl Future<Output = TimeoutObservation> {
        std::future::ready(TimeoutObservation {
            timeout: Duration::from_millis(25),
            elapsed: Duration::from_millis(28),
            timed_out: true,
            completed_after_timeout: false,
        })
    }
}

impl BackoffProbe for Probe {
    fn observe_backoff(&self) -> impl Future<Output = BackoffObservation> {
        std::future::ready(BackoffObservation {
            delays: vec![
                Duration::from_millis(5),
                Duration::from_millis(10),
                Duration::from_millis(20),
            ],
        })
    }
}

impl IntervalDriftProbe for Probe {
    fn observe_interval_drift(&self) -> impl Future<Output = IntervalDriftObservation> {
        std::future::ready(IntervalDriftObservation {
            absolute_drifts: vec![Duration::from_millis(1), Duration::from_millis(2)],
        })
    }
}

impl ChannelIntegrityProbe for Probe {
    fn observe_channel_integrity(&self) -> impl Future<Output = ChannelIntegrityObservation> {
        std::future::ready(ChannelIntegrityObservation {
            sent_messages: 6,
            received_messages: 6,
            duplicate_messages: 0,
            dropped_messages: 0,
        })
    }
}

impl ChannelBackpressureProbe for Probe {
    fn observe_channel_backpressure(&self) -> impl Future<Output = ChannelBackpressureObservation> {
        std::future::ready(ChannelBackpressureObservation {
            channel_capacity: 2,
            max_in_flight: 2,
            producer_wait_events: 1,
        })
    }
}

impl PermitLeakProbe for Probe {
    fn observe_permit_accounting(&self) -> impl Future<Output = PermitAccountingObservation> {
        std::future::ready(PermitAccountingObservation {
            acquired_permits: 10,
            released_permits: 10,
            outstanding_permits: 0,
        })
    }
}

impl PartialIoProbe for Probe {
    fn observe_partial_io(&self) -> impl Future<Output = PartialIoObservation> {
        std::future::ready(PartialIoObservation {
            expected_bytes: 1024,
            observed_bytes: 1024,
            partial_read_events: 1,
            partial_write_events: 2,
        })
    }
}

impl TransientIoRetryProbe for Probe {
    fn observe_transient_retry(&self) -> impl Future<Output = TransientIoRetryObservation> {
        std::future::ready(TransientIoRetryObservation {
            transient_errors: 2,
            permanent_errors: 0,
            retry_attempts: 2,
            succeeded: true,
        })
    }
}

#[tokio::test(flavor = "current_thread")]
async fn tokio_prelude_surface_compiles() {
    let probe = Probe;

    assert_cancellation_safe(&probe).await;
    assert_no_task_leak(&probe).await;
    assert_graceful_shutdown(&probe).await;

    assert_timeout_behavior(&probe, Duration::from_millis(5)).await;
    assert_backoff_bounds(&probe, Duration::from_millis(1), Duration::from_millis(25)).await;
    assert_interval_no_drift(&probe, Duration::from_millis(5)).await;

    assert_channel_no_drop_or_duplicate(&probe).await;
    assert_channel_backpressure(&probe, true).await;
    assert_no_permit_leak(&probe).await;

    assert_handles_partial_io(&probe, true).await;
    assert_retries_transient_io_errors(&probe, 4).await;
}

#[cfg(feature = "tokio-loom")]
struct LoomSmoke;

#[cfg(feature = "tokio-loom")]
impl TokioLoomModel for LoomSmoke {
    fn run_model(&self) {}
}

#[cfg(feature = "tokio-loom")]
#[test]
fn tokio_loom_surface_compiles() {
    assert_loom_model(LoomSmoke);
}
