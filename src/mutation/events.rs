//! Event model for append-only mutation run logs.

use serde::{Deserialize, Serialize};

/// Stable mutant descriptor discovered from a mutation engine.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MutantSpec {
    /// Stable identifier.
    pub id: String,
    /// Human-readable description.
    pub label: String,
    /// Engine-specific selector token for reruns.
    pub selector: String,
}

/// Outcome of executing tests against a mutant.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MutationOutcome {
    /// Tests failed (mutant killed).
    Killed,
    /// Tests passed (mutant survived).
    Survived,
    /// Timed out.
    Timeout,
    /// Mutant was invalid/unviable.
    Unviable,
    /// Mutant was intentionally skipped.
    Skipped,
    /// Execution ended with an error.
    Error {
        /// Human-readable engine or execution error detail.
        message: String,
    },
}

/// Log event emitted during mutation orchestration.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "event", rename_all = "snake_case")]
pub enum MutationEvent {
    /// New run created.
    RunStarted {
        /// Run id.
        run_id: String,
        /// Unix timestamp millis.
        timestamp_ms: i64,
        /// Number of mutants discovered for this run.
        discovered: usize,
    },
    /// Existing run resumed.
    RunResumed {
        /// Run id.
        run_id: String,
        /// Unix timestamp millis.
        timestamp_ms: i64,
        /// Number of remaining mutants before resume.
        remaining: usize,
    },
    /// Mutant known for this run.
    MutantDiscovered {
        /// Run id.
        run_id: String,
        /// Unix timestamp millis.
        timestamp_ms: i64,
        /// Discovered mutant.
        mutant: MutantSpec,
    },
    /// Mutant execution started.
    MutantStarted {
        /// Run id.
        run_id: String,
        /// Unix timestamp millis.
        timestamp_ms: i64,
        /// Mutant id.
        mutant_id: String,
    },
    /// Mutant execution finished.
    MutantFinished {
        /// Run id.
        run_id: String,
        /// Unix timestamp millis.
        timestamp_ms: i64,
        /// Mutant id.
        mutant_id: String,
        /// Final outcome.
        outcome: MutationOutcome,
        /// Optional process exit code.
        #[serde(default)]
        exit_code: Option<i32>,
        /// Optional relative stdout artifact path.
        #[serde(default)]
        stdout_artifact_path: Option<String>,
        /// Optional relative stderr artifact path.
        #[serde(default)]
        stderr_artifact_path: Option<String>,
        /// Optional start timestamp.
        #[serde(default)]
        started_at_ms: Option<i64>,
        /// Optional finish timestamp.
        #[serde(default)]
        finished_at_ms: Option<i64>,
        /// Optional runtime in milliseconds.
        #[serde(default)]
        duration_ms: Option<u64>,
    },
    /// Run interrupted by signal or operator.
    RunInterrupted {
        /// Run id.
        run_id: String,
        /// Unix timestamp millis.
        timestamp_ms: i64,
        /// Free-form reason.
        reason: String,
    },
    /// Run completed terminally.
    RunCompleted {
        /// Run id.
        run_id: String,
        /// Unix timestamp millis.
        timestamp_ms: i64,
    },
}

/// Current unix timestamp in milliseconds.
pub fn now_timestamp_ms() -> i64 {
    let now = std::time::SystemTime::now();
    let duration = now
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_else(|_| std::time::Duration::from_secs(0));
    (duration.as_secs() as i64)
        .saturating_mul(1000)
        .saturating_add(duration.subsec_millis() as i64)
}
