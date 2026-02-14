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
pub use events::{MutantSpec, MutationOutcome};
pub use report::{ReportFormat, RunSummary, render_report};
pub use runner::{RunResult, load_run_status, render_run_report, resume_run, run_new};
pub use state::{MutationStatus, RunSnapshot};
