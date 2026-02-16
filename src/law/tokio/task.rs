//! Task lifecycle laws for Tokio-based systems.

use std::future::Future;

/// Observable outcomes for a cancellation safety scenario.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CancellationSafetyObservation {
    /// Number of panics observed while cancellation was exercised.
    pub panicked_tasks: usize,
    /// Number of tasks that completed after cancellation was requested.
    pub completed_after_cancel: usize,
    /// Number of side effects observed after cancellation was requested.
    pub side_effects_after_cancel: usize,
}

/// Application-defined cancellation safety probe.
pub trait CancellationSafetyProbe {
    /// Execute a cancellation scenario and report observations.
    fn observe_cancellation_safety(&self) -> impl Future<Output = CancellationSafetyObservation>;
}

/// Assert that cancellation does not panic, finish cancelled work, or commit post-cancel side effects.
pub async fn assert_cancellation_safe<P>(probe: &P)
where
    P: CancellationSafetyProbe,
{
    let observation = probe.observe_cancellation_safety().await;
    assert_eq!(
        observation.panicked_tasks, 0,
        "cancellation scenario panicked {} task(s)",
        observation.panicked_tasks
    );
    assert_eq!(
        observation.completed_after_cancel, 0,
        "cancellation scenario completed {} task(s) after cancel",
        observation.completed_after_cancel
    );
    assert_eq!(
        observation.side_effects_after_cancel, 0,
        "cancellation scenario produced {} side effect(s) after cancel",
        observation.side_effects_after_cancel
    );
}

/// Observable outcomes for task leak checks.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TaskLeakObservation {
    /// Number of tasks spawned by the scenario.
    pub spawned_tasks: usize,
    /// Number of tasks that completed normally.
    pub finished_tasks: usize,
    /// Number of tasks that were cancelled or aborted explicitly.
    pub aborted_tasks: usize,
    /// Number of tasks detached without a terminal outcome.
    pub detached_tasks: usize,
}

/// Application-defined task leak probe.
pub trait TaskLeakProbe {
    /// Execute a task lifecycle scenario and report accounting observations.
    fn observe_task_leaks(&self) -> impl Future<Output = TaskLeakObservation>;
}

/// Assert that task accounting is balanced and no task is detached.
pub async fn assert_no_task_leak<P>(probe: &P)
where
    P: TaskLeakProbe,
{
    let observation = probe.observe_task_leaks().await;
    assert_eq!(
        observation.detached_tasks, 0,
        "task leak detected: {} detached task(s)",
        observation.detached_tasks
    );
    assert_eq!(
        observation.spawned_tasks,
        observation.finished_tasks + observation.aborted_tasks,
        "spawned task count ({}) did not match completed accounting (finished={}, aborted={})",
        observation.spawned_tasks,
        observation.finished_tasks,
        observation.aborted_tasks
    );
}

/// Observable outcomes for graceful shutdown scenarios.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GracefulShutdownObservation {
    /// Number of in-flight tasks when shutdown started.
    pub in_flight_tasks: usize,
    /// Number of tasks drained successfully before shutdown completion.
    pub drained_tasks: usize,
    /// Number of tasks force-stopped to complete shutdown.
    pub force_stopped_tasks: usize,
}

/// Application-defined graceful shutdown probe.
pub trait GracefulShutdownProbe {
    /// Execute shutdown behavior and report task-drain observations.
    fn observe_graceful_shutdown(&self) -> impl Future<Output = GracefulShutdownObservation>;
}

/// Assert that shutdown drains all in-flight tasks without force-stopping them.
pub async fn assert_graceful_shutdown<P>(probe: &P)
where
    P: GracefulShutdownProbe,
{
    let observation = probe.observe_graceful_shutdown().await;
    assert_eq!(
        observation.force_stopped_tasks, 0,
        "shutdown force-stopped {} task(s)",
        observation.force_stopped_tasks
    );
    assert_eq!(
        observation.in_flight_tasks, observation.drained_tasks,
        "shutdown did not drain all tasks (in-flight={}, drained={})",
        observation.in_flight_tasks, observation.drained_tasks
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    struct HealthyTaskProbe;

    impl CancellationSafetyProbe for HealthyTaskProbe {
        fn observe_cancellation_safety(
            &self,
        ) -> impl Future<Output = CancellationSafetyObservation> {
            std::future::ready(CancellationSafetyObservation {
                panicked_tasks: 0,
                completed_after_cancel: 0,
                side_effects_after_cancel: 0,
            })
        }
    }

    impl TaskLeakProbe for HealthyTaskProbe {
        fn observe_task_leaks(&self) -> impl Future<Output = TaskLeakObservation> {
            std::future::ready(TaskLeakObservation {
                spawned_tasks: 6,
                finished_tasks: 5,
                aborted_tasks: 1,
                detached_tasks: 0,
            })
        }
    }

    impl GracefulShutdownProbe for HealthyTaskProbe {
        fn observe_graceful_shutdown(&self) -> impl Future<Output = GracefulShutdownObservation> {
            std::future::ready(GracefulShutdownObservation {
                in_flight_tasks: 4,
                drained_tasks: 4,
                force_stopped_tasks: 0,
            })
        }
    }

    #[tokio::test(flavor = "current_thread")]
    async fn task_laws_accept_healthy_observations() {
        let probe = HealthyTaskProbe;
        assert_cancellation_safe(&probe).await;
        assert_no_task_leak(&probe).await;
        assert_graceful_shutdown(&probe).await;
    }
}
