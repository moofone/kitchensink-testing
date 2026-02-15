//! Event replay and run-state projection.

use std::collections::BTreeMap;
use std::fs::OpenOptions;
use std::io::{BufRead, BufReader, Write};
use std::path::Path;

use thiserror::Error;

use super::events::{
    MutantSpec, MutationEvent, MutationOutcome, RunConfigSnapshot, RunMetadata, TestFailure,
};

/// Status derived from event stream for each mutant.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MutationStatus {
    /// Discovered and not started.
    Pending,
    /// Started but no terminal outcome yet.
    Running,
    /// Terminal: killed.
    Killed,
    /// Terminal: survived.
    Survived,
    /// Terminal: timeout.
    Timeout,
    /// Terminal: unviable.
    Unviable,
    /// Terminal: skipped.
    Skipped,
    /// Terminal: error.
    Error,
}

impl MutationStatus {
    /// True if status is terminal.
    pub fn is_terminal(&self) -> bool {
        matches!(
            self,
            Self::Killed
                | Self::Survived
                | Self::Timeout
                | Self::Unviable
                | Self::Skipped
                | Self::Error
        )
    }
}

/// Per-mutant state in replay snapshot.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MutantState {
    /// Mutant descriptor.
    pub spec: MutantSpec,
    /// Derived status.
    pub status: MutationStatus,
    /// Mutant execution start timestamp.
    pub started_at_ms: Option<i64>,
    /// Mutant execution finish timestamp.
    pub finished_at_ms: Option<i64>,
    /// Duration in milliseconds.
    pub duration_ms: Option<u64>,
    /// Mutant process exit code.
    pub exit_code: Option<i32>,
    /// Relative path to stdout artifact.
    pub stdout_artifact_path: Option<String>,
    /// Relative path to stderr artifact.
    pub stderr_artifact_path: Option<String>,
    /// Optional last error string.
    pub last_error: Option<String>,
    /// Test names that ran against this mutant.
    pub tests_run: Vec<String>,
    /// Tests that failed (for killed mutants).
    pub tests_failed: Vec<TestFailure>,
    /// Preview of stdout output.
    pub stdout_preview: Option<String>,
    /// Preview of stderr output.
    pub stderr_preview: Option<String>,
}

/// Run-level metadata from RunStarted event.
#[derive(Debug, Clone, Default)]
pub struct RunInfo {
    /// Configuration snapshot.
    pub config: Option<RunConfigSnapshot>,
    /// Environment metadata.
    pub metadata: Option<RunMetadata>,
}

/// Materialized run state derived from `events.jsonl`.
#[derive(Debug, Clone)]
pub struct RunSnapshot {
    /// Run id.
    pub run_id: String,
    /// Mutants by id.
    pub mutants: BTreeMap<String, MutantState>,
    /// Number of malformed event lines ignored.
    pub malformed_lines: usize,
    /// Whether any interruption event has occurred.
    pub interrupted: bool,
    /// Whether a completion event has occurred.
    pub completed: bool,
    /// Run-level info (config, metadata).
    pub info: RunInfo,
}

impl RunSnapshot {
    /// Collect remaining mutants to execute/re-execute.
    pub fn pending_mutants(&self) -> Vec<MutantSpec> {
        self.mutants
            .values()
            .filter(|m| !m.status.is_terminal())
            .map(|m| m.spec.clone())
            .collect()
    }

    /// Collect survivors for targeted re-test.
    pub fn survivor_mutants(&self) -> Vec<MutantSpec> {
        self.mutants
            .values()
            .filter(|m| matches!(m.status, MutationStatus::Survived))
            .map(|m| m.spec.clone())
            .collect()
    }
}

/// State replay errors.
#[derive(Debug, Error)]
pub enum MutationStateError {
    /// IO failure.
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
}

/// Replay event log from `events.jsonl` into a snapshot.
pub fn replay_events(events_path: &Path) -> Result<RunSnapshot, MutationStateError> {
    let file = std::fs::File::open(events_path)?;
    let reader = BufReader::new(file);

    let mut run_id = String::new();
    let mut mutants: BTreeMap<String, MutantState> = BTreeMap::new();
    let mut malformed_lines = 0;
    let mut interrupted = false;
    let mut completed = false;
    let mut info = RunInfo::default();

    for line in reader.lines() {
        let line = line?;
        if line.trim().is_empty() {
            continue;
        }

        let event = match serde_json::from_str::<MutationEvent>(&line) {
            Ok(event) => event,
            Err(_) => {
                malformed_lines += 1;
                continue;
            }
        };

        match event {
            MutationEvent::RunStarted {
                run_id: id,
                config,
                metadata,
                ..
            } => {
                if run_id.is_empty() {
                    run_id = id;
                }
                info.config = config;
                info.metadata = metadata;
            }
            MutationEvent::RunResumed { run_id: id, .. } => {
                if run_id.is_empty() {
                    run_id = id;
                }
            }
            MutationEvent::MutantDiscovered { mutant, .. } => {
                mutants.insert(
                    mutant.id.clone(),
                    MutantState {
                        spec: mutant,
                        status: MutationStatus::Pending,
                        started_at_ms: None,
                        finished_at_ms: None,
                        duration_ms: None,
                        exit_code: None,
                        stdout_artifact_path: None,
                        stderr_artifact_path: None,
                        last_error: None,
                        tests_run: Vec::new(),
                        tests_failed: Vec::new(),
                        stdout_preview: None,
                        stderr_preview: None,
                    },
                );
            }
            MutationEvent::MutantStarted {
                mutant_id,
                timestamp_ms,
                ..
            } => {
                if let Some(state) = mutants.get_mut(&mutant_id) {
                    state.status = MutationStatus::Running;
                    state.started_at_ms = Some(timestamp_ms);
                }
            }
            MutationEvent::MutantFinished {
                mutant_id,
                outcome,
                exit_code,
                stdout_artifact_path,
                stderr_artifact_path,
                started_at_ms,
                finished_at_ms: _,
                duration_ms,
                timestamp_ms,
                tests_run,
                tests_failed,
                stdout_preview,
                stderr_preview,
                ..
            } => {
                if let Some(state) = mutants.get_mut(&mutant_id) {
                    state.finished_at_ms = Some(timestamp_ms);
                    state.started_at_ms = started_at_ms.or(state.started_at_ms);
                    state.duration_ms = duration_ms.or_else(|| {
                        state
                            .started_at_ms
                            .zip(state.finished_at_ms)
                            .and_then(|(start, finish)| {
                                finish
                                    .checked_sub(start)
                                    .and_then(|delta| u64::try_from(delta).ok())
                            })
                    });
                    state.exit_code = exit_code;
                    state.stdout_artifact_path = stdout_artifact_path;
                    state.stderr_artifact_path = stderr_artifact_path;
                    state.tests_run = tests_run;
                    state.tests_failed = tests_failed;
                    state.stdout_preview = stdout_preview;
                    state.stderr_preview = stderr_preview;
                    match outcome {
                        MutationOutcome::Killed => state.status = MutationStatus::Killed,
                        MutationOutcome::Survived => state.status = MutationStatus::Survived,
                        MutationOutcome::Timeout => state.status = MutationStatus::Timeout,
                        MutationOutcome::Unviable => state.status = MutationStatus::Unviable,
                        MutationOutcome::Skipped => state.status = MutationStatus::Skipped,
                        MutationOutcome::Error { message } => {
                            state.status = MutationStatus::Error;
                            state.last_error = Some(message);
                        }
                    }
                }
            }
            MutationEvent::RunInterrupted { .. } => {
                interrupted = true;
            }
            MutationEvent::RunCompleted { .. } => {
                completed = true;
            }
        }
    }

    Ok(RunSnapshot {
        run_id,
        mutants,
        malformed_lines,
        interrupted,
        completed,
        info,
    })
}

/// Append one event as JSONL line with fsync for durability.
pub fn append_event(events_path: &Path, event: &MutationEvent) -> Result<(), MutationStateError> {
    let mut file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(events_path)?;
    let json = serde_json::to_string(event).expect("mutation events should serialize");
    file.write_all(json.as_bytes())?;
    file.write_all(b"\n")?;
    file.flush()?;
    file.sync_all()?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use tempfile::tempdir;

    use super::*;
    use crate::mutation::events::{now_timestamp_ms, MutationType};

    fn test_mutant(id: &str, label: &str, selector: &str) -> MutantSpec {
        MutantSpec {
            id: id.to_string(),
            label: label.to_string(),
            selector: selector.to_string(),
            source_file: String::new(),
            source_line: 0,
            mutation_type: MutationType::Unknown,
            original_code: String::new(),
            mutated_code: String::new(),
        }
    }

    #[test]
    fn replay_is_deterministic() {
        let tmp = tempdir().expect("tempdir should be created");
        let events_path = tmp.path().join("events.jsonl");

        let mutant = test_mutant("m1", "mutant 1", "sel1");

        append_event(
            &events_path,
            &MutationEvent::RunStarted {
                run_id: "run-1".to_string(),
                timestamp_ms: now_timestamp_ms(),
                discovered: 1,
                config: None,
                metadata: None,
            },
        )
        .expect("run started should append");
        append_event(
            &events_path,
            &MutationEvent::MutantDiscovered {
                run_id: "run-1".to_string(),
                timestamp_ms: now_timestamp_ms(),
                mutant,
            },
        )
        .expect("mutant discovered should append");

        let a = replay_events(&events_path).expect("first replay should work");
        let b = replay_events(&events_path).expect("second replay should work");
        assert_eq!(a.run_id, b.run_id);
        assert_eq!(a.mutants.len(), b.mutants.len());
    }

    #[test]
    fn malformed_tail_is_ignored() {
        let tmp = tempdir().expect("tempdir should be created");
        let events_path = tmp.path().join("events.jsonl");

        append_event(
            &events_path,
            &MutationEvent::RunStarted {
                run_id: "run-1".to_string(),
                timestamp_ms: now_timestamp_ms(),
                discovered: 0,
                config: None,
                metadata: None,
            },
        )
        .expect("run started should append");

        let mut file = OpenOptions::new()
            .append(true)
            .open(&events_path)
            .expect("events file should open");
        file.write_all(b"{bad json\n")
            .expect("malformed tail should write");

        let snapshot = replay_events(&events_path).expect("replay should ignore malformed line");
        assert_eq!(snapshot.malformed_lines, 1);
        assert_eq!(snapshot.run_id, "run-1");
    }

    #[test]
    fn pending_mutants_includes_running_and_pending_only() {
        let tmp = tempdir().expect("tempdir should be created");
        let events_path = tmp.path().join("events.jsonl");

        let m_pending = test_mutant("m_pending", "pending", "sel_pending");
        let m_running = test_mutant("m_running", "running", "sel_running");
        let m_done = test_mutant("m_done", "done", "sel_done");

        append_event(
            &events_path,
            &MutationEvent::RunStarted {
                run_id: "run-2".to_string(),
                timestamp_ms: now_timestamp_ms(),
                discovered: 3,
                config: None,
                metadata: None,
            },
        )
        .expect("run started should append");
        for mutant in [m_pending.clone(), m_running.clone(), m_done.clone()] {
            append_event(
                &events_path,
                &MutationEvent::MutantDiscovered {
                    run_id: "run-2".to_string(),
                    timestamp_ms: now_timestamp_ms(),
                    mutant,
                },
            )
            .expect("mutant discovered should append");
        }
        append_event(
            &events_path,
            &MutationEvent::MutantStarted {
                run_id: "run-2".to_string(),
                timestamp_ms: now_timestamp_ms(),
                mutant_id: m_running.id.clone(),
            },
        )
        .expect("running mutant should append");
        append_event(
            &events_path,
            &MutationEvent::MutantFinished {
                run_id: "run-2".to_string(),
                timestamp_ms: now_timestamp_ms(),
                mutant_id: m_done.id.clone(),
                outcome: MutationOutcome::Killed,
                exit_code: None,
                stdout_artifact_path: None,
                stderr_artifact_path: None,
                started_at_ms: None,
                finished_at_ms: None,
                duration_ms: None,
                tests_run: Vec::new(),
                tests_failed: Vec::new(),
                stdout_preview: None,
                stderr_preview: None,
            },
        )
        .expect("done mutant should append");

        let snapshot = replay_events(&events_path).expect("replay should work");
        let pending_ids: std::collections::BTreeSet<String> = snapshot
            .pending_mutants()
            .into_iter()
            .map(|m| m.id)
            .collect();
        assert!(pending_ids.contains("m_pending"));
        assert!(pending_ids.contains("m_running"));
        assert!(!pending_ids.contains("m_done"));
    }

    #[test]
    fn error_outcome_persists_error_message() {
        let tmp = tempdir().expect("tempdir should be created");
        let events_path = tmp.path().join("events.jsonl");
        let mutant = test_mutant("m_err", "error mutant", "sel_err");

        append_event(
            &events_path,
            &MutationEvent::RunStarted {
                run_id: "run-3".to_string(),
                timestamp_ms: now_timestamp_ms(),
                discovered: 1,
                config: None,
                metadata: None,
            },
        )
        .expect("run started should append");
        append_event(
            &events_path,
            &MutationEvent::MutantDiscovered {
                run_id: "run-3".to_string(),
                timestamp_ms: now_timestamp_ms(),
                mutant: mutant.clone(),
            },
        )
        .expect("mutant discovered should append");
        append_event(
            &events_path,
            &MutationEvent::MutantFinished {
                run_id: "run-3".to_string(),
                timestamp_ms: now_timestamp_ms(),
                mutant_id: mutant.id.clone(),
                outcome: MutationOutcome::Error {
                    message: "boom".to_string(),
                },
                exit_code: None,
                stdout_artifact_path: None,
                stderr_artifact_path: None,
                started_at_ms: None,
                finished_at_ms: None,
                duration_ms: None,
                tests_run: Vec::new(),
                tests_failed: Vec::new(),
                stdout_preview: None,
                stderr_preview: None,
            },
        )
        .expect("mutant finished should append");

        let snapshot = replay_events(&events_path).expect("replay should work");
        let state = snapshot
            .mutants
            .get("m_err")
            .expect("error mutant should exist");
        assert_eq!(state.status, MutationStatus::Error);
        assert_eq!(state.last_error.as_deref(), Some("boom"));
    }
}
