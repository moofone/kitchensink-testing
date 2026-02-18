//! Mutation run orchestration (new run, resume, status, report).

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use std::sync::OnceLock;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::time::Instant;

use thiserror::Error;

use super::config::MutationConfig;
use super::engine::{MutantExecutionResult, MutationEngine, MutationEngineError};
use super::events::{MutantSpec, MutationEvent, MutationOutcome, now_timestamp_ms};
use super::report::{ReportFormat, render_report};
use super::state::{MutationStateError, RunSnapshot, append_event, replay_events};

static INTERRUPTED: AtomicBool = AtomicBool::new(false);
static RUN_SEQUENCE: AtomicU64 = AtomicU64::new(0);

/// Run orchestration errors.
#[derive(Debug, Error)]
pub enum MutationRunError {
    /// State layer error.
    #[error("state error: {0}")]
    State(#[from] MutationStateError),
    /// Engine error.
    #[error("engine error: {0}")]
    Engine(#[from] MutationEngineError),
    /// IO error.
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    /// Signal handler error.
    #[error("signal handler installation failed: {0}")]
    Signal(String),
}

/// Result returned by run/resume operations.
#[derive(Debug, Clone)]
pub struct RunResult {
    /// Run id.
    pub run_id: String,
    /// Path to run directory.
    pub run_dir: PathBuf,
    /// Materialized snapshot after operation.
    pub snapshot: RunSnapshot,
}

fn install_signal_handler_once() -> Result<(), MutationRunError> {
    static INIT: OnceLock<Result<(), String>> = OnceLock::new();

    let result = INIT.get_or_init(|| {
        ctrlc::set_handler(|| {
            INTERRUPTED.store(true, Ordering::SeqCst);
        })
        .map_err(|e| e.to_string())
    });

    match result {
        Ok(()) => Ok(()),
        Err(msg) => Err(MutationRunError::Signal(msg.clone())),
    }
}

fn generate_run_id() -> String {
    let seq = RUN_SEQUENCE.fetch_add(1, Ordering::SeqCst);
    format!("run-{}-{}-{}", now_timestamp_ms(), std::process::id(), seq)
}

fn events_path(run_dir: &Path) -> PathBuf {
    run_dir.join("events.jsonl")
}

fn sanitize_mutant_id(mutant_id: &str) -> String {
    mutant_id
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '-' || c == '_' {
                c
            } else {
                '_'
            }
        })
        .collect()
}

fn write_mutant_artifacts(
    run_dir: &Path,
    mutant_id: &str,
    result: &MutantExecutionResult,
) -> Result<(Option<String>, Option<String>), MutationRunError> {
    let base = run_dir.join("artifacts");
    std::fs::create_dir_all(&base)?;

    let safe_id = sanitize_mutant_id(mutant_id);
    let mut stdout_artifact_path = None;
    let mut stderr_artifact_path = None;

    if !result.stdout.is_empty() || matches!(result.outcome, MutationOutcome::Error { .. }) {
        let path = base.join(format!("{safe_id}.stdout.log"));
        std::fs::write(&path, &result.stdout)?;
        stdout_artifact_path = Some(format!("artifacts/{safe_id}.stdout.log"));
    }

    if !result.stderr.is_empty() || matches!(result.outcome, MutationOutcome::Error { .. }) {
        let path = base.join(format!("{safe_id}.stderr.log"));
        std::fs::write(&path, &result.stderr)?;
        stderr_artifact_path = Some(format!("artifacts/{safe_id}.stderr.log"));
    }

    Ok((stdout_artifact_path, stderr_artifact_path))
}

#[derive(Debug, Clone, Copy)]
struct RunIdKey {
    timestamp_ms: i64,
    pid: u32,
    sequence: u64,
}

fn parse_run_id_key(run_id: &str) -> Option<RunIdKey> {
    let mut parts = run_id.split('-');
    if parts.next()? != "run" {
        return None;
    }

    Some(RunIdKey {
        timestamp_ms: parts.next()?.parse().ok()?,
        pid: parts.next()?.parse().ok()?,
        sequence: parts.next()?.parse().ok()?,
    })
}

fn is_snapshot_compatible(snapshot: &RunSnapshot, config: &MutationConfig) -> bool {
    let snapshot_filter = snapshot
        .info
        .config
        .as_ref()
        .and_then(|cfg| cfg.filter.clone());
    if snapshot_filter != config.filter {
        return false;
    }

    let snapshot_timeout = snapshot
        .info
        .config
        .as_ref()
        .and_then(|cfg| cfg.timeout_secs);
    snapshot_timeout == config.timeout_secs
}

fn is_newer_run_id(candidate: &RunIdKey, current: &RunIdKey) -> bool {
    candidate.timestamp_ms > current.timestamp_ms
        || (candidate.timestamp_ms == current.timestamp_ms && candidate.pid > current.pid)
        || (candidate.timestamp_ms == current.timestamp_ms
            && candidate.pid == current.pid
            && candidate.sequence > current.sequence)
}

fn latest_incomplete_run_id(config: &MutationConfig) -> Result<Option<String>, MutationRunError> {
    if !config.run_root.exists() {
        return Ok(None);
    }

    let mut newest: Option<(RunIdKey, String)> = None;

    for entry in std::fs::read_dir(&config.run_root)? {
        let entry = entry?;
        if !entry.file_type()?.is_dir() {
            continue;
        }

        let run_id = entry.file_name().to_string_lossy().to_string();
        let run_id_key = match parse_run_id_key(&run_id) {
            Some(key) => key,
            None => continue,
        };

        let snapshot = match load_run_status(config, &run_id) {
            Ok(snapshot) => snapshot,
            Err(MutationRunError::State(_)) => continue,
            Err(err) => return Err(err),
        };

        if snapshot.completed || snapshot.pending_mutants().is_empty() {
            continue;
        }

        if !is_snapshot_compatible(&snapshot, config) {
            continue;
        }

        let is_newer = match &newest {
            Some((current, _)) => is_newer_run_id(&run_id_key, current),
            None => true,
        };

        if is_newer {
            newest = Some((run_id_key, run_id));
        }
    }

    Ok(newest.map(|(_, run_id)| run_id))
}

fn latest_completed_run_with_survivors_id(
    config: &MutationConfig,
) -> Result<Option<String>, MutationRunError> {
    if !config.run_root.exists() {
        return Ok(None);
    }

    let mut newest: Option<(RunIdKey, String)> = None;

    for entry in std::fs::read_dir(&config.run_root)? {
        let entry = entry?;
        if !entry.file_type()?.is_dir() {
            continue;
        }

        let run_id = entry.file_name().to_string_lossy().to_string();
        let run_id_key = match parse_run_id_key(&run_id) {
            Some(key) => key,
            None => continue,
        };

        let snapshot = match load_run_status(config, &run_id) {
            Ok(snapshot) => snapshot,
            Err(MutationRunError::State(_)) => continue,
            Err(err) => return Err(err),
        };

        if !snapshot.completed || snapshot.survivor_mutants().is_empty() {
            continue;
        }

        if !is_snapshot_compatible(&snapshot, config) {
            continue;
        }

        let is_newer = match &newest {
            Some((current, _)) => is_newer_run_id(&run_id_key, current),
            None => true,
        };

        if is_newer {
            newest = Some((run_id_key, run_id));
        }
    }

    Ok(newest.map(|(_, run_id)| run_id))
}

fn run_mutant(
    run_id: &str,
    run_dir: &Path,
    events: &Path,
    config: &MutationConfig,
    engine: &dyn MutationEngine,
    mutant: &MutantSpec,
) -> Result<(), MutationRunError> {
    let started_at_ms = now_timestamp_ms();
    append_event(
        events,
        &MutationEvent::MutantStarted {
            run_id: run_id.to_string(),
            timestamp_ms: started_at_ms,
            mutant_id: mutant.id.clone(),
        },
    )?;

    let started = Instant::now();
    let execution = match engine.execute_mutant(config, mutant) {
        Ok(execution) => execution,
        Err(err) => MutantExecutionResult {
            outcome: MutationOutcome::Error {
                message: err.to_string(),
            },
            exit_code: None,
            stdout: String::new(),
            stderr: String::new(),
        },
    };

    let finished_at_ms = now_timestamp_ms();
    let duration_ms = started.elapsed().as_millis() as u64;
    let (stdout_artifact_path, stderr_artifact_path) =
        write_mutant_artifacts(run_dir, &mutant.id, &execution)?;

    append_event(
        events,
        &MutationEvent::MutantFinished {
            run_id: run_id.to_string(),
            timestamp_ms: finished_at_ms,
            mutant_id: mutant.id.clone(),
            outcome: execution.outcome,
            exit_code: execution.exit_code,
            stdout_artifact_path,
            stderr_artifact_path,
            started_at_ms: Some(started_at_ms),
            finished_at_ms: Some(finished_at_ms),
            duration_ms: Some(duration_ms),
            tests_run: Vec::new(),
            tests_failed: Vec::new(),
            stdout_preview: Some(super::events::truncate_preview(&execution.stdout)),
            stderr_preview: Some(super::events::truncate_preview(&execution.stderr)),
        },
    )?;

    Ok(())
}

/// Start a new mutation run.
pub fn run_new(
    config: &MutationConfig,
    engine: &dyn MutationEngine,
) -> Result<RunResult, MutationRunError> {
    install_signal_handler_once()?;
    INTERRUPTED.store(false, Ordering::SeqCst);

    if let Some(run_id) = latest_incomplete_run_id(config)? {
        println!("kitchensink-testing: resuming interrupted run {run_id}");
        return resume_run(config, &run_id, engine);
    }

    if let Some(run_id) = latest_completed_run_with_survivors_id(config)? {
        println!("kitchensink-testing: retesting survivors from completed run {run_id}");
        return rerun_survivors(config, &run_id, engine);
    }

    let run_id = generate_run_id();
    let run_dir = config.run_root.join(&run_id);
    std::fs::create_dir_all(&run_dir)?;
    let events = events_path(&run_dir);

    let mut mutants = engine.discover_mutants(config)?;
    if let Some(filter) = &config.filter {
        mutants.retain(|m| {
            m.id.contains(filter) || m.label.contains(filter) || m.selector.contains(filter)
        });
    }
    println!(
        "kitchensink-testing: discovered {} mutant(s) in {}",
        mutants.len(),
        config.project_dir.display()
    );

    append_event(
        &events,
        &MutationEvent::RunStarted {
            run_id: run_id.clone(),
            timestamp_ms: now_timestamp_ms(),
            discovered: mutants.len(),
            config: Some(super::events::RunConfigSnapshot {
                timeout_secs: config.timeout_secs,
                filter: config.filter.clone(),
                quality_gate_minimum_score: None,
                quality_gate_maximum_survived: None,
            }),
            metadata: Some(super::events::collect_metadata()),
        },
    )?;

    let total_mutants = mutants.len();
    for mutant in &mutants {
        append_event(
            &events,
            &MutationEvent::MutantDiscovered {
                run_id: run_id.clone(),
                timestamp_ms: now_timestamp_ms(),
                mutant: mutant.clone(),
            },
        )?;
    }

    for (index, mutant) in mutants.iter().enumerate() {
        let position = index + 1;
        println!(
            "kitchensink-testing: running mutant {position}/{total_mutants}: {}",
            mutant.label
        );
        if INTERRUPTED.load(Ordering::SeqCst) {
            append_event(
                &events,
                &MutationEvent::RunInterrupted {
                    run_id: run_id.clone(),
                    timestamp_ms: now_timestamp_ms(),
                    reason: "received interrupt signal".to_string(),
                },
            )?;
            break;
        }
        run_mutant(&run_id, &run_dir, &events, config, engine, mutant)?;
    }

    if !INTERRUPTED.load(Ordering::SeqCst) {
        append_event(
            &events,
            &MutationEvent::RunCompleted {
                run_id: run_id.clone(),
                timestamp_ms: now_timestamp_ms(),
            },
        )?;
    }

    let snapshot = replay_events(&events)?;
    Ok(RunResult {
        run_id,
        run_dir,
        snapshot,
    })
}

/// Resume an existing run id.
pub fn resume_run(
    config: &MutationConfig,
    run_id: &str,
    engine: &dyn MutationEngine,
) -> Result<RunResult, MutationRunError> {
    install_signal_handler_once()?;
    INTERRUPTED.store(false, Ordering::SeqCst);

    let run_dir = config.run_root.join(run_id);
    let events = events_path(&run_dir);
    let snapshot = replay_events(&events)?;
    let survivors = snapshot.survivor_mutants();
    let pending = snapshot.pending_mutants();

    if snapshot.completed && survivors.is_empty() {
        println!("kitchensink-testing: run {run_id} already completed");
        return Ok(RunResult {
            run_id: run_id.to_string(),
            run_dir,
            snapshot,
        });
    }

    if snapshot.completed {
        println!(
            "kitchensink-testing: rerunning {} survivor mutant(s) from completed run {run_id}",
            survivors.len()
        );
    } else {
        println!(
            "kitchensink-testing: resuming run {run_id}, {} mutant(s) remaining",
            pending.len()
        );
        if !survivors.is_empty() {
            println!(
                "kitchensink-testing: retesting {} survivor mutant(s) before pending queue",
                survivors.len()
            );
        }
    }

    let mut scheduled = survivors;
    if !snapshot.completed {
        let mut seen: BTreeSet<String> = scheduled.iter().map(|m| m.id.clone()).collect();
        for mutant in pending {
            if seen.insert(mutant.id.clone()) {
                scheduled.push(mutant);
            }
        }
    }

    append_event(
        &events,
        &MutationEvent::RunResumed {
            run_id: run_id.to_string(),
            timestamp_ms: now_timestamp_ms(),
            remaining: scheduled.len(),
        },
    )?;

    let total_mutants = scheduled.len();
    for (index, mutant) in scheduled.iter().enumerate() {
        let position = index + 1;
        println!(
            "kitchensink-testing: running mutant {position}/{total_mutants}: {}",
            mutant.label
        );
        if INTERRUPTED.load(Ordering::SeqCst) {
            append_event(
                &events,
                &MutationEvent::RunInterrupted {
                    run_id: run_id.to_string(),
                    timestamp_ms: now_timestamp_ms(),
                    reason: "received interrupt signal during resume".to_string(),
                },
            )?;
            break;
        }
        run_mutant(run_id, &run_dir, &events, config, engine, mutant)?;
    }

    if !INTERRUPTED.load(Ordering::SeqCst) {
        append_event(
            &events,
            &MutationEvent::RunCompleted {
                run_id: run_id.to_string(),
                timestamp_ms: now_timestamp_ms(),
            },
        )?;
    }

    let snapshot = replay_events(&events)?;
    Ok(RunResult {
        run_id: run_id.to_string(),
        run_dir,
        snapshot,
    })
}

/// Re-run only survivors for an existing run id.
pub fn rerun_survivors(
    config: &MutationConfig,
    run_id: &str,
    engine: &dyn MutationEngine,
) -> Result<RunResult, MutationRunError> {
    install_signal_handler_once()?;
    INTERRUPTED.store(false, Ordering::SeqCst);

    let run_dir = config.run_root.join(run_id);
    let events = events_path(&run_dir);
    let snapshot = replay_events(&events)?;
    let survivors = snapshot.survivor_mutants();

    if survivors.is_empty() {
        println!("kitchensink-testing: run {run_id} has no survivor mutants to rerun");
        return Ok(RunResult {
            run_id: run_id.to_string(),
            run_dir,
            snapshot,
        });
    }

    println!(
        "kitchensink-testing: rerunning {} survivor mutant(s) from run {run_id}",
        survivors.len()
    );

    append_event(
        &events,
        &MutationEvent::RunResumed {
            run_id: run_id.to_string(),
            timestamp_ms: now_timestamp_ms(),
            remaining: survivors.len(),
        },
    )?;

    let total_mutants = survivors.len();
    for (index, mutant) in survivors.iter().enumerate() {
        let position = index + 1;
        println!(
            "kitchensink-testing: running survivor {position}/{total_mutants}: {}",
            mutant.label
        );
        if INTERRUPTED.load(Ordering::SeqCst) {
            append_event(
                &events,
                &MutationEvent::RunInterrupted {
                    run_id: run_id.to_string(),
                    timestamp_ms: now_timestamp_ms(),
                    reason: "received interrupt signal during survivor rerun".to_string(),
                },
            )?;
            break;
        }
        run_mutant(run_id, &run_dir, &events, config, engine, mutant)?;
    }

    let snapshot = replay_events(&events)?;
    Ok(RunResult {
        run_id: run_id.to_string(),
        run_dir,
        snapshot,
    })
}

/// Load run status snapshot.
pub fn load_run_status(
    config: &MutationConfig,
    run_id: &str,
) -> Result<RunSnapshot, MutationRunError> {
    let events = events_path(&config.run_root.join(run_id));
    Ok(replay_events(&events)?)
}

/// Render run report.
pub fn render_run_report(
    config: &MutationConfig,
    run_id: &str,
    format: ReportFormat,
) -> Result<String, MutationRunError> {
    let snapshot = load_run_status(config, run_id)?;
    Ok(render_report(&snapshot, format))
}

#[cfg(test)]
mod tests {
    use std::sync::{Arc, Mutex, MutexGuard, OnceLock};
    use tempfile::tempdir;

    use super::*;

    fn test_guard() -> MutexGuard<'static, ()> {
        static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
        LOCK.get_or_init(|| Mutex::new(()))
            .lock()
            .expect("runner tests mutex should lock")
    }
    use crate::mutation::engine::MutationEngine;
    use crate::mutation::events::{MutantSpec, MutationType};

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

    #[derive(Clone)]
    struct FakeEngine;

    impl MutationEngine for FakeEngine {
        fn discover_mutants(
            &self,
            _config: &MutationConfig,
        ) -> Result<Vec<MutantSpec>, MutationEngineError> {
            Ok(vec![
                test_mutant("m1", "mutant-1", "sel1"),
                test_mutant("m2", "mutant-2", "sel2"),
            ])
        }

        fn execute_mutant(
            &self,
            _config: &MutationConfig,
            mutant: &MutantSpec,
        ) -> Result<MutantExecutionResult, MutationEngineError> {
            if mutant.id == "m1" {
                Ok(MutantExecutionResult {
                    outcome: MutationOutcome::Killed,
                    exit_code: None,
                    stdout: String::new(),
                    stderr: String::new(),
                })
            } else {
                Ok(MutantExecutionResult {
                    outcome: MutationOutcome::Survived,
                    exit_code: None,
                    stdout: String::new(),
                    stderr: String::new(),
                })
            }
        }
    }

    #[derive(Clone)]
    struct ErrorEngine;

    impl MutationEngine for ErrorEngine {
        fn discover_mutants(
            &self,
            _config: &MutationConfig,
        ) -> Result<Vec<MutantSpec>, MutationEngineError> {
            Ok(vec![test_mutant("m_err", "mutant-err", "sel-err")])
        }

        fn execute_mutant(
            &self,
            _config: &MutationConfig,
            _mutant: &MutantSpec,
        ) -> Result<MutantExecutionResult, MutationEngineError> {
            Err(MutationEngineError::Unsupported("forced error".to_string()))
        }
    }

    #[derive(Clone)]
    struct InterruptingEngine;

    impl MutationEngine for InterruptingEngine {
        fn discover_mutants(
            &self,
            _config: &MutationConfig,
        ) -> Result<Vec<MutantSpec>, MutationEngineError> {
            Ok(vec![
                test_mutant("m1", "mutant-1", "sel1"),
                test_mutant("m2", "mutant-2", "sel2"),
            ])
        }

        fn execute_mutant(
            &self,
            _config: &MutationConfig,
            mutant: &MutantSpec,
        ) -> Result<MutantExecutionResult, MutationEngineError> {
            if mutant.id == "m1" {
                INTERRUPTED.store(true, Ordering::SeqCst);
            }
            Ok(MutantExecutionResult {
                outcome: MutationOutcome::Killed,
                exit_code: None,
                stdout: String::new(),
                stderr: String::new(),
            })
        }
    }

    #[derive(Clone)]
    struct AlwaysKilledEngine;

    impl MutationEngine for AlwaysKilledEngine {
        fn discover_mutants(
            &self,
            _config: &MutationConfig,
        ) -> Result<Vec<MutantSpec>, MutationEngineError> {
            Ok(vec![
                test_mutant("m1", "mutant-1", "sel1"),
                test_mutant("m2", "mutant-2", "sel2"),
            ])
        }

        fn execute_mutant(
            &self,
            _config: &MutationConfig,
            _mutant: &MutantSpec,
        ) -> Result<MutantExecutionResult, MutationEngineError> {
            Ok(MutantExecutionResult {
                outcome: MutationOutcome::Killed,
                exit_code: None,
                stdout: String::new(),
                stderr: String::new(),
            })
        }
    }

    #[derive(Clone)]
    struct RecordingEngine {
        order: Arc<Mutex<Vec<String>>>,
    }

    impl MutationEngine for RecordingEngine {
        fn discover_mutants(
            &self,
            _config: &MutationConfig,
        ) -> Result<Vec<MutantSpec>, MutationEngineError> {
            Ok(Vec::new())
        }

        fn execute_mutant(
            &self,
            _config: &MutationConfig,
            mutant: &MutantSpec,
        ) -> Result<MutantExecutionResult, MutationEngineError> {
            self.order
                .lock()
                .expect("recording order mutex should lock")
                .push(mutant.id.clone());
            Ok(MutantExecutionResult {
                outcome: MutationOutcome::Killed,
                exit_code: None,
                stdout: String::new(),
                stderr: String::new(),
            })
        }
    }

    #[cfg(any(target_os = "linux", target_os = "macos"))]
    #[derive(Clone)]
    struct SlowEngine;

    #[cfg(any(target_os = "linux", target_os = "macos"))]
    impl MutationEngine for SlowEngine {
        fn discover_mutants(
            &self,
            _config: &MutationConfig,
        ) -> Result<Vec<MutantSpec>, MutationEngineError> {
            Ok(vec![
                test_mutant("m1", "mutant-1", "sel1"),
                test_mutant("m2", "mutant-2", "sel2"),
                test_mutant("m3", "mutant-3", "sel3"),
            ])
        }

        fn execute_mutant(
            &self,
            _config: &MutationConfig,
            _mutant: &MutantSpec,
        ) -> Result<MutantExecutionResult, MutationEngineError> {
            std::thread::sleep(std::time::Duration::from_millis(500));
            Ok(MutantExecutionResult {
                outcome: MutationOutcome::Killed,
                exit_code: None,
                stdout: String::new(),
                stderr: String::new(),
            })
        }
    }

    #[test]
    fn run_and_resume_roundtrip() {
        let _guard = test_guard();
        let tmp = tempdir().expect("tempdir should be created");
        let config = MutationConfig::default().with_run_root(tmp.path());
        let engine = FakeEngine;

        let first = run_new(&config, &engine).expect("run should succeed");
        assert!(first.snapshot.completed);

        let resumed = resume_run(&config, &first.run_id, &engine).expect("resume should succeed");
        assert!(resumed.snapshot.completed);
        assert_eq!(resumed.snapshot.pending_mutants().len(), 0);
    }

    #[test]
    fn resume_recovers_running_mutant() {
        let _guard = test_guard();
        let tmp = tempdir().expect("tempdir should be created");
        let run_root = tmp.path().join("runs");
        let run_id = "run-recover";
        let run_dir = run_root.join(run_id);
        std::fs::create_dir_all(&run_dir).expect("run dir should be created");
        let events = run_dir.join("events.jsonl");

        let m1 = test_mutant("m1", "mutant-1", "sel1");
        let m2 = test_mutant("m2", "mutant-2", "sel2");

        append_event(
            &events,
            &MutationEvent::RunStarted {
                run_id: run_id.to_string(),
                timestamp_ms: now_timestamp_ms(),
                discovered: 2,
                config: None,
                metadata: None,
            },
        )
        .expect("run started should append");
        append_event(
            &events,
            &MutationEvent::MutantDiscovered {
                run_id: run_id.to_string(),
                timestamp_ms: now_timestamp_ms(),
                mutant: m1.clone(),
            },
        )
        .expect("m1 discovered should append");
        append_event(
            &events,
            &MutationEvent::MutantDiscovered {
                run_id: run_id.to_string(),
                timestamp_ms: now_timestamp_ms(),
                mutant: m2.clone(),
            },
        )
        .expect("m2 discovered should append");
        append_event(
            &events,
            &MutationEvent::MutantStarted {
                run_id: run_id.to_string(),
                timestamp_ms: now_timestamp_ms(),
                mutant_id: m1.id.clone(),
            },
        )
        .expect("m1 started should append");
        append_event(
            &events,
            &MutationEvent::MutantFinished {
                run_id: run_id.to_string(),
                timestamp_ms: now_timestamp_ms(),
                mutant_id: m1.id.clone(),
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
        .expect("m1 finished should append");
        append_event(
            &events,
            &MutationEvent::MutantStarted {
                run_id: run_id.to_string(),
                timestamp_ms: now_timestamp_ms(),
                mutant_id: m2.id.clone(),
            },
        )
        .expect("m2 started should append");

        let config = MutationConfig::default().with_run_root(run_root);
        let engine = FakeEngine;
        let resumed = resume_run(&config, run_id, &engine).expect("resume should succeed");

        let m2_state = resumed
            .snapshot
            .mutants
            .get("m2")
            .expect("m2 state should exist");
        assert_eq!(
            m2_state.status,
            crate::mutation::state::MutationStatus::Survived
        );
        assert!(resumed.snapshot.completed);
    }

    #[test]
    fn resume_after_completion_keeps_terminal_outcomes_stable() {
        let _guard = test_guard();
        let tmp = tempdir().expect("tempdir should be created");
        let config = MutationConfig::default().with_run_root(tmp.path());
        let engine = FakeEngine;

        let first = run_new(&config, &engine).expect("run should succeed");
        assert!(first.snapshot.completed);

        let second = resume_run(&config, &first.run_id, &engine).expect("first resume should work");
        let third = resume_run(&config, &first.run_id, &engine).expect("second resume should work");

        let second_m1 = second
            .snapshot
            .mutants
            .get("m1")
            .expect("m1 state should exist after first resume");
        let second_m2 = second
            .snapshot
            .mutants
            .get("m2")
            .expect("m2 state should exist after first resume");
        let third_m1 = third
            .snapshot
            .mutants
            .get("m1")
            .expect("m1 state should exist after second resume");
        let third_m2 = third
            .snapshot
            .mutants
            .get("m2")
            .expect("m2 state should exist after second resume");

        assert_eq!(
            second_m1.status,
            crate::mutation::state::MutationStatus::Killed
        );
        assert_eq!(
            second_m2.status,
            crate::mutation::state::MutationStatus::Survived
        );
        assert_eq!(third_m1.status, second_m1.status);
        assert_eq!(third_m2.status, second_m2.status);
        assert!(third.snapshot.completed);
        assert_eq!(third.snapshot.pending_mutants().len(), 0);
    }

    #[test]
    fn run_new_retests_survivors_from_latest_completed_run() {
        let _guard = test_guard();
        let tmp = tempdir().expect("tempdir should be created");
        let config = MutationConfig::default().with_run_root(tmp.path());

        let first = run_new(&config, &FakeEngine).expect("initial run should succeed");
        let first_m2 = first
            .snapshot
            .mutants
            .get("m2")
            .expect("m2 state should exist on first run");
        assert_eq!(
            first_m2.status,
            crate::mutation::state::MutationStatus::Survived
        );

        let second = run_new(&config, &AlwaysKilledEngine)
            .expect("second run should retest survivors from the completed run");
        assert_eq!(
            second.run_id, first.run_id,
            "expected run_new to retest survivors in latest completed run"
        );
        let second_m2 = second
            .snapshot
            .mutants
            .get("m2")
            .expect("m2 state should exist on second run");
        assert_eq!(
            second_m2.status,
            crate::mutation::state::MutationStatus::Killed
        );
        assert!(second.snapshot.completed);
    }

    #[test]
    fn resume_executes_survivors_before_pending_mutants() {
        let _guard = test_guard();
        let tmp = tempdir().expect("tempdir should be created");
        let run_root = tmp.path().join("runs");
        let run_id = "run-priority";
        let run_dir = run_root.join(run_id);
        std::fs::create_dir_all(&run_dir).expect("run dir should be created");
        let events = run_dir.join("events.jsonl");

        let survivor = test_mutant("m1", "mutant-1", "sel1");
        let killed = test_mutant("m2", "mutant-2", "sel2");
        let pending = test_mutant("m3", "mutant-3", "sel3");

        append_event(
            &events,
            &MutationEvent::RunStarted {
                run_id: run_id.to_string(),
                timestamp_ms: now_timestamp_ms(),
                discovered: 3,
                config: None,
                metadata: None,
            },
        )
        .expect("run started should append");
        append_event(
            &events,
            &MutationEvent::MutantDiscovered {
                run_id: run_id.to_string(),
                timestamp_ms: now_timestamp_ms(),
                mutant: survivor.clone(),
            },
        )
        .expect("survivor discovered should append");
        append_event(
            &events,
            &MutationEvent::MutantDiscovered {
                run_id: run_id.to_string(),
                timestamp_ms: now_timestamp_ms(),
                mutant: killed.clone(),
            },
        )
        .expect("killed discovered should append");
        append_event(
            &events,
            &MutationEvent::MutantDiscovered {
                run_id: run_id.to_string(),
                timestamp_ms: now_timestamp_ms(),
                mutant: pending.clone(),
            },
        )
        .expect("pending discovered should append");
        append_event(
            &events,
            &MutationEvent::MutantFinished {
                run_id: run_id.to_string(),
                timestamp_ms: now_timestamp_ms(),
                mutant_id: survivor.id.clone(),
                outcome: MutationOutcome::Survived,
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
        .expect("survivor finished should append");
        append_event(
            &events,
            &MutationEvent::MutantFinished {
                run_id: run_id.to_string(),
                timestamp_ms: now_timestamp_ms(),
                mutant_id: killed.id.clone(),
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
        .expect("killed finished should append");

        let order = Arc::new(Mutex::new(Vec::new()));
        let config = MutationConfig::default().with_run_root(run_root);
        let engine = RecordingEngine {
            order: order.clone(),
        };

        let resumed = resume_run(&config, run_id, &engine).expect("resume should succeed");
        let execution_order = order
            .lock()
            .expect("recording order mutex should lock")
            .clone();
        assert_eq!(execution_order, vec!["m1".to_string(), "m3".to_string()]);
        assert!(resumed.snapshot.completed);
    }

    #[test]
    fn rerun_survivors_executes_only_survivor_queue() {
        let _guard = test_guard();
        let tmp = tempdir().expect("tempdir should be created");
        let run_root = tmp.path().join("runs");
        let run_id = "run-survivors-only";
        let run_dir = run_root.join(run_id);
        std::fs::create_dir_all(&run_dir).expect("run dir should be created");
        let events = run_dir.join("events.jsonl");

        let survivor = test_mutant("m1", "mutant-1", "sel1");
        let pending = test_mutant("m2", "mutant-2", "sel2");

        append_event(
            &events,
            &MutationEvent::RunStarted {
                run_id: run_id.to_string(),
                timestamp_ms: now_timestamp_ms(),
                discovered: 2,
                config: None,
                metadata: None,
            },
        )
        .expect("run started should append");
        append_event(
            &events,
            &MutationEvent::MutantDiscovered {
                run_id: run_id.to_string(),
                timestamp_ms: now_timestamp_ms(),
                mutant: survivor.clone(),
            },
        )
        .expect("survivor discovered should append");
        append_event(
            &events,
            &MutationEvent::MutantDiscovered {
                run_id: run_id.to_string(),
                timestamp_ms: now_timestamp_ms(),
                mutant: pending.clone(),
            },
        )
        .expect("pending discovered should append");
        append_event(
            &events,
            &MutationEvent::MutantFinished {
                run_id: run_id.to_string(),
                timestamp_ms: now_timestamp_ms(),
                mutant_id: survivor.id.clone(),
                outcome: MutationOutcome::Survived,
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
        .expect("survivor finished should append");

        let order = Arc::new(Mutex::new(Vec::new()));
        let config = MutationConfig::default().with_run_root(run_root);
        let engine = RecordingEngine {
            order: order.clone(),
        };

        let rerun =
            rerun_survivors(&config, run_id, &engine).expect("survivor rerun should succeed");
        let execution_order = order
            .lock()
            .expect("recording order mutex should lock")
            .clone();

        assert_eq!(execution_order, vec!["m1".to_string()]);
        assert!(!rerun.snapshot.completed);
        assert_eq!(rerun.snapshot.pending_mutants().len(), 1);
    }

    #[test]
    fn run_records_error_outcome_on_engine_failure() {
        let _guard = test_guard();
        let tmp = tempdir().expect("tempdir should be created");
        let config = MutationConfig::default().with_run_root(tmp.path());
        let engine = ErrorEngine;

        let run = run_new(&config, &engine).expect("run should still complete");
        let state = run
            .snapshot
            .mutants
            .get("m_err")
            .expect("error mutant should be tracked");
        assert_eq!(state.status, crate::mutation::state::MutationStatus::Error);
        assert!(
            state
                .last_error
                .as_ref()
                .is_some_and(|m| m.contains("forced error"))
        );
        assert!(run.snapshot.completed);
    }

    #[test]
    fn run_records_interruption_without_signal() {
        let _guard = test_guard();
        let tmp = tempdir().expect("tempdir should be created");
        let config = MutationConfig::default().with_run_root(tmp.path());
        let engine = SlowEngine;

        let interrupter = std::thread::spawn(|| {
            std::thread::sleep(std::time::Duration::from_millis(200));
            INTERRUPTED.store(true, Ordering::SeqCst);
        });

        let run = run_new(&config, &engine).expect("run should stop when interrupted");
        interrupter
            .join()
            .expect("interrupter thread should join cleanly");

        assert!(run.snapshot.interrupted);
        assert!(!run.snapshot.completed);
        assert!(
            !run.snapshot.pending_mutants().is_empty(),
            "interruption should leave pending mutants"
        );
        assert!(
            run.snapshot.pending_mutants().len() < 3,
            "at least one mutant should be skipped after interruption"
        );
    }

    #[test]
    fn run_records_interruption_and_leaves_pending_work() {
        let _guard = test_guard();
        let tmp = tempdir().expect("tempdir should be created");
        let config = MutationConfig::default().with_run_root(tmp.path());
        let engine = InterruptingEngine;

        let run = run_new(&config, &engine).expect("run should succeed");
        assert!(run.snapshot.interrupted);
        assert!(!run.snapshot.completed);
        assert_eq!(run.snapshot.pending_mutants().len(), 1);
    }

    #[test]
    fn run_new_resumes_latest_incomplete_run() {
        let _guard = test_guard();
        let tmp = tempdir().expect("tempdir should be created");
        let config = MutationConfig::default().with_run_root(tmp.path());

        let interrupted = run_new(&config, &InterruptingEngine)
            .expect("run should capture interruption and leave pending mutants");
        assert!(interrupted.snapshot.interrupted);
        assert!(!interrupted.snapshot.completed);
        assert!(!interrupted.snapshot.pending_mutants().is_empty());

        let resumed = run_new(&config, &FakeEngine).expect("rerun should resume interrupted run");
        assert_eq!(resumed.run_id, interrupted.run_id);
        assert!(resumed.snapshot.completed);
        assert_eq!(resumed.snapshot.pending_mutants().len(), 0);
    }

    #[test]
    fn load_status_for_missing_run_returns_io_error() {
        let _guard = test_guard();
        let tmp = tempdir().expect("tempdir should be created");
        let config = MutationConfig::default().with_run_root(tmp.path());

        let err = load_run_status(&config, "does-not-exist").expect_err("status should fail");
        match err {
            MutationRunError::State(MutationStateError::Io(_)) => {}
            other => panic!("expected IO state error, got {other:?}"),
        }
    }

    #[test]
    fn render_run_report_reads_persisted_events() {
        let _guard = test_guard();
        let tmp = tempdir().expect("tempdir should be created");
        let config = MutationConfig::default().with_run_root(tmp.path());
        let engine = FakeEngine;

        let run = run_new(&config, &engine).expect("run should succeed");
        let md = render_run_report(&config, &run.run_id, ReportFormat::Markdown)
            .expect("report render should succeed");
        assert!(md.contains(&run.run_id));
        assert!(md.contains("| killed |"));
    }

    #[cfg(any(target_os = "linux", target_os = "macos"))]
    #[test]
    fn run_handles_real_sigint_signal() {
        let _guard = test_guard();
        let tmp = tempdir().expect("tempdir should be created");
        let config = MutationConfig::default().with_run_root(tmp.path());
        let engine = SlowEngine;

        let pid = std::process::id().to_string();
        let signal_thread = std::thread::spawn(move || {
            std::thread::sleep(std::time::Duration::from_millis(200));
            let _ = std::process::Command::new("kill")
                .arg("-INT")
                .arg(pid)
                .status();
        });

        let run = run_new(&config, &engine).expect("run should complete with interruption");
        signal_thread
            .join()
            .expect("signal thread should join cleanly");

        assert!(run.snapshot.interrupted);
        assert!(!run.snapshot.completed);
        assert!(
            !run.snapshot.pending_mutants().is_empty(),
            "at least one mutant should remain pending after SIGINT"
        );
    }
}
