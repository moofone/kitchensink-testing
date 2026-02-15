//! Event model for append-only mutation run logs.

use serde::{Deserialize, Serialize};

/// Classification of mutation type for LLM-friendly analysis.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MutationType {
    /// Arithmetic operator mutation (+, -, *, /, %).
    Arithmetic,
    /// Comparison operator mutation (==, !=, <, >, <=, >=).
    Comparison,
    /// Logical operator mutation (&&, ||, !).
    Logical,
    /// Boolean literal mutation (true <-> false).
    Boolean,
    /// Return value mutation.
    ReturnValue,
    /// Method call removal or replacement.
    MethodCall,
    /// Assignment mutation.
    Assignment,
    /// Boundary condition mutation.
    Boundary,
    /// Negation insertion or removal.
    Negation,
    /// Unknown or unclassified mutation type.
    Unknown,
}

impl Default for MutationType {
    fn default() -> Self {
        Self::Unknown
    }
}

impl std::fmt::Display for MutationType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Arithmetic => write!(f, "arithmetic"),
            Self::Comparison => write!(f, "comparison"),
            Self::Logical => write!(f, "logical"),
            Self::Boolean => write!(f, "boolean"),
            Self::ReturnValue => write!(f, "return_value"),
            Self::MethodCall => write!(f, "method_call"),
            Self::Assignment => write!(f, "assignment"),
            Self::Boundary => write!(f, "boundary"),
            Self::Negation => write!(f, "negation"),
            Self::Unknown => write!(f, "unknown"),
        }
    }
}

/// Stable mutant descriptor discovered from a mutation engine.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MutantSpec {
    /// Stable identifier.
    pub id: String,
    /// Human-readable description.
    pub label: String,
    /// Engine-specific selector token for reruns.
    pub selector: String,
    /// Source file path relative to project root.
    #[serde(default)]
    pub source_file: String,
    /// Source line number (1-indexed).
    #[serde(default)]
    pub source_line: u32,
    /// Classification of mutation type.
    #[serde(default)]
    pub mutation_type: MutationType,
    /// Original code snippet before mutation.
    #[serde(default)]
    pub original_code: String,
    /// Mutated code snippet.
    #[serde(default)]
    pub mutated_code: String,
}

/// Test failure details for killed mutants.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TestFailure {
    /// Test name or path.
    pub test_name: String,
    /// Assertion or failure message.
    #[serde(default)]
    pub message: Option<String>,
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

/// Snapshot of configuration for reproducibility.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RunConfigSnapshot {
    /// Per-mutant timeout in seconds.
    #[serde(default)]
    pub timeout_secs: Option<u64>,
    /// Filter applied to mutant selection.
    #[serde(default)]
    pub filter: Option<String>,
    /// Quality gate minimum score.
    #[serde(default)]
    pub quality_gate_minimum_score: Option<f64>,
    /// Quality gate maximum survived count.
    #[serde(default)]
    pub quality_gate_maximum_survived: Option<usize>,
}

/// Environment metadata for reproducibility and debugging.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RunMetadata {
    /// Rust compiler version.
    #[serde(default)]
    pub rustc_version: String,
    /// Cargo version.
    #[serde(default)]
    pub cargo_version: String,
    /// cargo-mutants version if available.
    #[serde(default)]
    pub cargo_mutants_version: String,
    /// Git commit hash.
    #[serde(default)]
    pub git_commit: String,
    /// Git branch name.
    #[serde(default)]
    pub git_branch: String,
    /// Host operating system.
    #[serde(default)]
    pub os: String,
    /// Host architecture.
    #[serde(default)]
    pub arch: String,
}

/// Log event emitted during mutation orchestration.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "event", rename_all = "snake_case")]
pub enum MutationEvent {
    /// New run created with metadata.
    RunStarted {
        /// Run id.
        run_id: String,
        /// Unix timestamp millis.
        timestamp_ms: i64,
        /// Number of mutants discovered for this run.
        discovered: usize,
        /// Configuration snapshot for reproducibility.
        #[serde(default)]
        config: Option<RunConfigSnapshot>,
        /// Environment metadata.
        #[serde(default)]
        metadata: Option<RunMetadata>,
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
        /// Test names that ran against this mutant.
        #[serde(default)]
        tests_run: Vec<String>,
        /// Tests that failed (for killed mutants).
        #[serde(default)]
        tests_failed: Vec<TestFailure>,
        /// Preview of stdout output (first N bytes).
        #[serde(default)]
        stdout_preview: Option<String>,
        /// Preview of stderr output (first N bytes).
        #[serde(default)]
        stderr_preview: Option<String>,
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

/// Maximum bytes to include in stdout/stderr previews.
pub const OUTPUT_PREVIEW_MAX_BYTES: usize = 4096;

/// Truncate output to preview size.
pub fn truncate_preview(output: &str) -> String {
    if output.len() <= OUTPUT_PREVIEW_MAX_BYTES {
        output.to_string()
    } else {
        let truncated = &output[..OUTPUT_PREVIEW_MAX_BYTES.saturating_sub(3)];
        format!("{truncated}...")
    }
}

/// Collect environment metadata for the current run.
pub fn collect_metadata() -> RunMetadata {
    let rustc_version = std::process::Command::new("rustc")
        .arg("--version")
        .output()
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
        .unwrap_or_default();

    let cargo_version = std::process::Command::new("cargo")
        .arg("--version")
        .output()
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
        .unwrap_or_default();

    let cargo_mutants_version = std::process::Command::new("cargo")
        .args(["mutants", "--version"])
        .output()
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
        .unwrap_or_default();

    let git_commit = std::process::Command::new("git")
        .args(["rev-parse", "HEAD"])
        .output()
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
        .unwrap_or_default();

    let git_branch = std::process::Command::new("git")
        .args(["rev-parse", "--abbrev-ref", "HEAD"])
        .output()
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
        .unwrap_or_default();

    RunMetadata {
        rustc_version,
        cargo_version,
        cargo_mutants_version,
        git_commit,
        git_branch,
        os: std::env::consts::OS.to_string(),
        arch: std::env::consts::ARCH.to_string(),
    }
}

/// Parse mutation type from a label string (best-effort classification).
pub fn parse_mutation_type(label: &str) -> MutationType {
    let lower = label.to_ascii_lowercase();

    if lower.contains("replace ") || lower.contains(" -> ") {
        if lower.contains('+')
            || lower.contains('-')
            || lower.contains('*')
            || lower.contains('/')
            || lower.contains('%')
        {
            if lower.contains("==")
                || lower.contains("!=")
                || lower.contains("<")
                || lower.contains(">")
            {
                return MutationType::Comparison;
            }
            return MutationType::Arithmetic;
        }
        if lower.contains("&&") || lower.contains("||") || lower.contains('!') {
            return MutationType::Logical;
        }
        if lower.contains("true") || lower.contains("false") {
            return MutationType::Boolean;
        }
    }

    if lower.contains("return") {
        return MutationType::ReturnValue;
    }

    if lower.contains("call") || lower.contains("method") {
        return MutationType::MethodCall;
    }

    if lower.contains("assign") || lower.contains('=') {
        return MutationType::Assignment;
    }

    if lower.contains("boundary") || lower.contains("off by one") {
        return MutationType::Boundary;
    }

    if lower.contains("negate") || lower.contains("negation") {
        return MutationType::Negation;
    }

    MutationType::Unknown
}
