//! Synchronization and channel laws for Tokio-based systems.

use std::future::Future;

/// Observable outcomes for channel integrity checks.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ChannelIntegrityObservation {
    /// Number of messages sent.
    pub sent_messages: usize,
    /// Number of messages received.
    pub received_messages: usize,
    /// Number of duplicate message deliveries observed.
    pub duplicate_messages: usize,
    /// Number of missing/dropped messages observed.
    pub dropped_messages: usize,
}

/// Application-defined channel integrity probe.
pub trait ChannelIntegrityProbe {
    /// Execute channel send/receive behavior and report message accounting.
    fn observe_channel_integrity(&self) -> impl Future<Output = ChannelIntegrityObservation>;
}

/// Assert that a channel scenario does not drop or duplicate messages.
pub async fn assert_channel_no_drop_or_duplicate<P>(probe: &P)
where
    P: ChannelIntegrityProbe,
{
    let observation = probe.observe_channel_integrity().await;
    assert_eq!(
        observation.duplicate_messages, 0,
        "channel duplicated {} message(s)",
        observation.duplicate_messages
    );
    assert_eq!(
        observation.dropped_messages, 0,
        "channel dropped {} message(s)",
        observation.dropped_messages
    );
    assert_eq!(
        observation.sent_messages, observation.received_messages,
        "channel accounting mismatch (sent={}, received={})",
        observation.sent_messages, observation.received_messages
    );
}

/// Observable outcomes for channel backpressure checks.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ChannelBackpressureObservation {
    /// Configured channel capacity.
    pub channel_capacity: usize,
    /// Maximum number of in-flight items observed.
    pub max_in_flight: usize,
    /// Number of producer wait/block events observed.
    pub producer_wait_events: usize,
}

/// Application-defined channel backpressure probe.
pub trait ChannelBackpressureProbe {
    /// Execute a bounded channel scenario and report backpressure observations.
    fn observe_channel_backpressure(&self) -> impl Future<Output = ChannelBackpressureObservation>;
}

/// Assert channel backpressure bounds with optional strict wait-event requirement.
pub async fn assert_channel_backpressure<P>(probe: &P, must_observe_wait: bool)
where
    P: ChannelBackpressureProbe,
{
    let observation = probe.observe_channel_backpressure().await;
    assert!(
        observation.max_in_flight <= observation.channel_capacity,
        "channel exceeded configured capacity (in_flight={}, capacity={})",
        observation.max_in_flight,
        observation.channel_capacity
    );
    if must_observe_wait {
        assert!(
            observation.producer_wait_events > 0,
            "channel scenario did not observe producer wait/backpressure events"
        );
    }
}

/// Observable outcomes for permit accounting checks.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PermitAccountingObservation {
    /// Number of permits acquired.
    pub acquired_permits: usize,
    /// Number of permits released.
    pub released_permits: usize,
    /// Number of permits still held when scenario completes.
    pub outstanding_permits: usize,
}

/// Application-defined permit leak probe.
pub trait PermitLeakProbe {
    /// Execute semaphore behavior and report permit accounting.
    fn observe_permit_accounting(&self) -> impl Future<Output = PermitAccountingObservation>;
}

/// Assert that permit accounting is balanced and no permit is leaked.
pub async fn assert_no_permit_leak<P>(probe: &P)
where
    P: PermitLeakProbe,
{
    let observation = probe.observe_permit_accounting().await;
    assert_eq!(
        observation.outstanding_permits, 0,
        "permit leak detected: {} outstanding permit(s)",
        observation.outstanding_permits
    );
    assert_eq!(
        observation.acquired_permits, observation.released_permits,
        "permit accounting mismatch (acquired={}, released={})",
        observation.acquired_permits, observation.released_permits
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    struct HealthySyncProbe;

    impl ChannelIntegrityProbe for HealthySyncProbe {
        fn observe_channel_integrity(&self) -> impl Future<Output = ChannelIntegrityObservation> {
            std::future::ready(ChannelIntegrityObservation {
                sent_messages: 8,
                received_messages: 8,
                duplicate_messages: 0,
                dropped_messages: 0,
            })
        }
    }

    impl ChannelBackpressureProbe for HealthySyncProbe {
        fn observe_channel_backpressure(
            &self,
        ) -> impl Future<Output = ChannelBackpressureObservation> {
            std::future::ready(ChannelBackpressureObservation {
                channel_capacity: 4,
                max_in_flight: 4,
                producer_wait_events: 2,
            })
        }
    }

    impl PermitLeakProbe for HealthySyncProbe {
        fn observe_permit_accounting(&self) -> impl Future<Output = PermitAccountingObservation> {
            std::future::ready(PermitAccountingObservation {
                acquired_permits: 16,
                released_permits: 16,
                outstanding_permits: 0,
            })
        }
    }

    #[tokio::test(flavor = "current_thread")]
    async fn sync_laws_accept_healthy_observations() {
        let probe = HealthySyncProbe;
        assert_channel_no_drop_or_duplicate(&probe).await;
        assert_channel_backpressure(&probe, true).await;
        assert_no_permit_leak(&probe).await;
    }
}
