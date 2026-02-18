//! Resumable mutation testing orchestration.

pub mod config;
pub mod engine;
pub mod events;
/// Human-readable and machine-friendly report generation.
pub mod report;
pub mod runner;
pub mod state;

pub use config::MutationConfig;
pub use engine::{CargoMutantsEngine, MutationEngine};
pub use events::{
    MutantSpec, MutationOutcome, MutationType, RunConfigSnapshot, RunMetadata, TestFailure,
    collect_metadata, parse_mutation_type, truncate_preview,
};
pub use report::{MutantReport, ReportFormat, RunSummary, render_report};
pub use runner::{
    RunResult, load_run_status, render_run_report, rerun_survivors, resume_run, run_new,
};
pub use state::{MutationStatus, RunInfo, RunSnapshot};
