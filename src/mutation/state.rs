//! Event replay and run-state projection.

use std::collections::BTreeMap;
use std::fs::OpenOptions;
use std::io::{BufRead, BufReader, Write};
use std::path::Path;

use thiserror::Error;

use super::events::{MutantSpec, MutationEvent, MutationOutcome};

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
}

/// Materialized run state derived from `events.jsonl`.
#[derive(Debug, Clone, PartialEq, Eq)]
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
            MutationEvent::RunStarted { run_id: id, .. }
            | MutationEvent::RunResumed { run_id: id, .. } => {
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
    })
}

/// Append one event as JSONL line.
pub fn append_event(events_path: &Path, event: &MutationEvent) -> Result<(), MutationStateError> {
    let mut file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(events_path)?;
    let json = serde_json::to_string(event).expect("mutation events should serialize");
    file.write_all(json.as_bytes())?;
    file.write_all(b"\n")?;
    file.flush()?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use tempfile::tempdir;

    use super::*;
    use crate::mutation::events::now_timestamp_ms;

    #[test]
    fn replay_is_deterministic() {
        let tmp = tempdir().expect("tempdir should be created");
        let events_path = tmp.path().join("events.jsonl");

        let mutant = MutantSpec {
            id: "m1".to_string(),
            label: "mutant 1".to_string(),
            selector: "sel1".to_string(),
        };

        append_event(
            &events_path,
            &MutationEvent::RunStarted {
                run_id: "run-1".to_string(),
                timestamp_ms: now_timestamp_ms(),
                discovered: 1,
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
        assert_eq!(a, b);
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

        let m_pending = MutantSpec {
            id: "m_pending".to_string(),
            label: "pending".to_string(),
            selector: "sel_pending".to_string(),
        };
        let m_running = MutantSpec {
            id: "m_running".to_string(),
            label: "running".to_string(),
            selector: "sel_running".to_string(),
        };
        let m_done = MutantSpec {
            id: "m_done".to_string(),
            label: "done".to_string(),
            selector: "sel_done".to_string(),
        };

        append_event(
            &events_path,
            &MutationEvent::RunStarted {
                run_id: "run-2".to_string(),
                timestamp_ms: now_timestamp_ms(),
                discovered: 3,
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
        let mutant = MutantSpec {
            id: "m_err".to_string(),
            label: "error mutant".to_string(),
            selector: "sel_err".to_string(),
        };

        append_event(
            &events_path,
            &MutationEvent::RunStarted {
                run_id: "run-3".to_string(),
                timestamp_ms: now_timestamp_ms(),
                discovered: 1,
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
