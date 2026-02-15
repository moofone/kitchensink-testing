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
    collect_metadata, parse_mutation_type, truncate_preview, MutantSpec, MutationOutcome,
    MutationType, RunConfigSnapshot, RunMetadata, TestFailure,
};
pub use report::{render_report, MutantReport, ReportFormat, RunSummary};
pub use runner::{load_run_status, render_run_report, resume_run, run_new, RunResult};
pub use state::{MutationStatus, RunInfo, RunSnapshot};
