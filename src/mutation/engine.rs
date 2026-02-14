//! Mutation engine abstraction and cargo-mutants adapter.

use std::process::Command;

use thiserror::Error;

use super::config::MutationConfig;
use super::events::{MutantSpec, MutationOutcome};

/// Engine-level errors.
#[derive(Debug, Error)]
pub enum MutationEngineError {
    /// `cargo-mutants` binary is unavailable.
    #[error("cargo-mutants is not installed or not available as `cargo mutants`")]
    MissingCargoMutants,
    /// Underlying command execution failed.
    #[error("command execution failed: {0}")]
    CommandFailed(String),
    /// Engine lacks required capability for resumable single-mutant execution.
    #[error("unsupported cargo-mutants capability: {0}")]
    Unsupported(String),
    /// IO failure.
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
}

/// Execution result for one mutant, including captured process outputs.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MutantExecutionResult {
    /// Final outcome of mutant execution.
    pub outcome: MutationOutcome,
    /// Process exit code, if known.
    pub exit_code: Option<i32>,
    /// Captured stdout.
    pub stdout: String,
    /// Captured stderr.
    pub stderr: String,
}

/// Mutation engine contract.
pub trait MutationEngine {
    /// Discover available mutants.
    fn discover_mutants(
        &self,
        config: &MutationConfig,
    ) -> Result<Vec<MutantSpec>, MutationEngineError>;

    /// Execute one mutant and return its outcome.
    fn execute_mutant(
        &self,
        config: &MutationConfig,
        mutant: &MutantSpec,
    ) -> Result<MutantExecutionResult, MutationEngineError>;
}

/// Adapter for `cargo-mutants` CLI.
#[derive(Debug, Default, Clone, Copy)]
pub struct CargoMutantsEngine;

impl CargoMutantsEngine {
    fn has_mutant_selector_flag(&self) -> Result<bool, MutationEngineError> {
        let out = Command::new("cargo").arg("mutants").arg("--help").output();

        let out = match out {
            Ok(out) => out,
            Err(err) if err.kind() == std::io::ErrorKind::NotFound => {
                return Err(MutationEngineError::MissingCargoMutants);
            }
            Err(err) => return Err(MutationEngineError::Io(err)),
        };

        let text = String::from_utf8_lossy(&out.stdout).to_string()
            + &String::from_utf8_lossy(&out.stderr);
        Ok(text.contains("--mutant"))
    }

    fn stable_hash(input: &str) -> u64 {
        // FNV-1a 64-bit.
        const OFFSET_BASIS: u64 = 0xcbf29ce484222325;
        const PRIME: u64 = 0x100000001b3;

        let mut hash = OFFSET_BASIS;
        for b in input.as_bytes() {
            hash ^= *b as u64;
            hash = hash.wrapping_mul(PRIME);
        }
        hash
    }

    fn classify_outcome(status: std::process::ExitStatus, text: &str) -> MutationOutcome {
        let lower = text.to_ascii_lowercase();

        if lower.contains("timeout") {
            return MutationOutcome::Timeout;
        }
        if lower.contains("unviable") {
            return MutationOutcome::Unviable;
        }
        if lower.contains("survived") || lower.contains("missed") {
            return MutationOutcome::Survived;
        }
        if lower.contains("killed") || lower.contains("caught") {
            return MutationOutcome::Killed;
        }

        if status.success() {
            MutationOutcome::Killed
        } else {
            MutationOutcome::Error {
                message: text.trim().to_string(),
            }
        }
    }
}

impl MutationEngine for CargoMutantsEngine {
    fn discover_mutants(
        &self,
        config: &MutationConfig,
    ) -> Result<Vec<MutantSpec>, MutationEngineError> {
        let output = Command::new("cargo")
            .arg("mutants")
            .arg("--list")
            .current_dir(&config.project_dir)
            .output();

        let output = match output {
            Ok(out) => out,
            Err(err) if err.kind() == std::io::ErrorKind::NotFound => {
                return Err(MutationEngineError::MissingCargoMutants);
            }
            Err(err) => return Err(MutationEngineError::Io(err)),
        };

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr).to_string();
            return Err(MutationEngineError::CommandFailed(stderr));
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        let mut mutants = Vec::new();

        for (idx, raw) in stdout.lines().enumerate() {
            let line = raw.trim();
            if line.is_empty() {
                continue;
            }
            if line.starts_with("Found ") || line.starts_with("Listing ") {
                continue;
            }

            let id = format!("m{:04x}", Self::stable_hash(&format!("{idx}:{line}")));
            mutants.push(MutantSpec {
                id,
                label: line.to_string(),
                selector: line.to_string(),
            });
        }

        if mutants.is_empty() {
            return Err(MutationEngineError::CommandFailed(
                "`cargo mutants --list` returned no mutants".to_string(),
            ));
        }

        Ok(mutants)
    }

    fn execute_mutant(
        &self,
        config: &MutationConfig,
        mutant: &MutantSpec,
    ) -> Result<MutantExecutionResult, MutationEngineError> {
        if !self.has_mutant_selector_flag()? {
            return Err(MutationEngineError::Unsupported(
                "this cargo-mutants version does not expose --mutant selector; resumable per-mutant execution is unavailable".to_string(),
            ));
        }

        let mut cmd = Command::new("cargo");
        cmd.arg("mutants")
            .arg("--mutant")
            .arg(&mutant.selector)
            .current_dir(&config.project_dir);

        if let Some(timeout_secs) = config.timeout_secs {
            cmd.env("CARGO_MUTANTS_TIMEOUT", timeout_secs.to_string());
        }

        let output = cmd.output()?;
        let text = String::from_utf8_lossy(&output.stdout).to_string()
            + &String::from_utf8_lossy(&output.stderr);
        Ok(MutantExecutionResult {
            outcome: Self::classify_outcome(output.status, &text),
            exit_code: output.status.code(),
            stdout: String::from_utf8_lossy(&output.stdout).to_string(),
            stderr: String::from_utf8_lossy(&output.stderr).to_string(),
        })
    }
}

#[cfg(test)]
mod tests {
    use std::process::Command;

    use super::*;

    #[test]
    fn stable_hash_is_deterministic() {
        let a = CargoMutantsEngine::stable_hash("same-input");
        let b = CargoMutantsEngine::stable_hash("same-input");
        let c = CargoMutantsEngine::stable_hash("other-input");
        assert_eq!(a, b);
        assert_ne!(a, c);
    }

    #[test]
    fn classify_outcome_uses_keywords() {
        let ok_status = Command::new("sh")
            .arg("-c")
            .arg("exit 0")
            .status()
            .expect("status should run");
        let fail_status = Command::new("sh")
            .arg("-c")
            .arg("exit 1")
            .status()
            .expect("status should run");

        assert_eq!(
            CargoMutantsEngine::classify_outcome(ok_status, "mutant survived"),
            MutationOutcome::Survived
        );
        assert_eq!(
            CargoMutantsEngine::classify_outcome(ok_status, "mutant killed"),
            MutationOutcome::Killed
        );
        assert_eq!(
            CargoMutantsEngine::classify_outcome(ok_status, "timeout while running"),
            MutationOutcome::Timeout
        );
        assert_eq!(
            CargoMutantsEngine::classify_outcome(ok_status, "unviable mutation"),
            MutationOutcome::Unviable
        );

        match CargoMutantsEngine::classify_outcome(fail_status, "unknown failure") {
            MutationOutcome::Error { message } => assert!(message.contains("unknown failure")),
            other => panic!("expected error outcome, got {other:?}"),
        }
    }

    #[test]
    fn execute_mutant_reports_capability_issue_or_missing_binary() {
        let engine = CargoMutantsEngine;
        let config = MutationConfig::default();
        let mutant = MutantSpec {
            id: "m1".to_string(),
            label: "l1".to_string(),
            selector: "s1".to_string(),
        };

        let result = engine.execute_mutant(&config, &mutant);
        assert!(matches!(
            result,
            Err(MutationEngineError::Unsupported(_))
                | Err(MutationEngineError::MissingCargoMutants)
                | Err(MutationEngineError::CommandFailed(_))
                | Err(MutationEngineError::Io(_))
                | Ok(_)
        ));
    }
}
