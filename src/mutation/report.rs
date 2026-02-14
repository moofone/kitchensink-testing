use serde::Serialize;

use super::state::{MutationStatus, RunSnapshot};

/// Supported output formats for run reports.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReportFormat {
    /// Markdown summary.
    Markdown,
    /// JSON summary.
    Json,
}

/// Aggregated run counts.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct RunSummary {
    /// Total discovered mutants.
    pub total: usize,
    /// Mutation score (killed / testable mutants), percentage.
    pub mutation_score: f64,
    /// Killed mutants.
    pub killed: usize,
    /// Survived mutants.
    pub survived: usize,
    /// Timeout mutants.
    pub timeout: usize,
    /// Unviable mutants.
    pub unviable: usize,
    /// Skipped mutants.
    pub skipped: usize,
    /// Error mutants.
    pub error: usize,
    /// Still pending/running mutants.
    pub incomplete: usize,
}

impl RunSummary {
    /// Build summary from snapshot.
    pub fn from_snapshot(snapshot: &RunSnapshot) -> Self {
        let mut out = Self {
            total: snapshot.mutants.len(),
            mutation_score: 0.0,
            killed: 0,
            survived: 0,
            timeout: 0,
            unviable: 0,
            skipped: 0,
            error: 0,
            incomplete: 0,
        };

        for mutant in snapshot.mutants.values() {
            match mutant.status {
                MutationStatus::Killed => out.killed += 1,
                MutationStatus::Survived => out.survived += 1,
                MutationStatus::Timeout => out.timeout += 1,
                MutationStatus::Unviable => out.unviable += 1,
                MutationStatus::Skipped => out.skipped += 1,
                MutationStatus::Error => out.error += 1,
                MutationStatus::Pending | MutationStatus::Running => out.incomplete += 1,
            }
        }

        let testable = out.total.saturating_sub(out.skipped + out.unviable);
        if testable > 0 {
            out.mutation_score = (out.killed as f64) * 100.0 / (testable as f64);
        } else {
            out.mutation_score = 100.0;
        }

        out
    }
}

/// Render run report in requested format.
pub fn render_report(snapshot: &RunSnapshot, format: ReportFormat) -> String {
    let summary = RunSummary::from_snapshot(snapshot);

    match format {
        ReportFormat::Json => serde_json::to_string_pretty(&serde_json::json!({
            "run_id": snapshot.run_id,
            "completed": snapshot.completed,
            "interrupted": snapshot.interrupted,
            "malformed_lines": snapshot.malformed_lines,
            "summary": summary,
        }))
        .expect("report JSON should serialize"),
        ReportFormat::Markdown => {
            format!(
                "# Mutation Run {}\n\n- completed: {}\n- interrupted: {}\n- malformed lines: {}\n\n| metric | count |\n|---|---:|\n| total | {} |\n| killed | {} |\n| survived | {} |\n| timeout | {} |\n| unviable | {} |\n| skipped | {} |\n| error | {} |\n| incomplete | {} |\n| mutation score | {:.2}% |\n",
                snapshot.run_id,
                snapshot.completed,
                snapshot.interrupted,
                snapshot.malformed_lines,
                summary.total,
                summary.killed,
                summary.survived,
                summary.timeout,
                summary.unviable,
                summary.skipped,
                summary.error,
                summary.incomplete,
                summary.mutation_score,
            )
        }
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use super::*;
    use crate::mutation::events::MutantSpec;
    use crate::mutation::state::{MutantState, MutationStatus};

    fn snapshot_with_statuses(statuses: &[MutationStatus]) -> RunSnapshot {
        let mut mutants = BTreeMap::new();
        for (idx, status) in statuses.iter().enumerate() {
            let id = format!("m{idx}");
            mutants.insert(
                id.clone(),
                MutantState {
                    spec: MutantSpec {
                        id,
                        label: "label".to_string(),
                        selector: "selector".to_string(),
                    },
                    status: status.clone(),
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

        RunSnapshot {
            run_id: "run-report".to_string(),
            mutants,
            malformed_lines: 0,
            interrupted: false,
            completed: true,
        }
    }

    #[test]
    fn summary_counts_all_statuses() {
        let snapshot = snapshot_with_statuses(&[
            MutationStatus::Killed,
            MutationStatus::Survived,
            MutationStatus::Timeout,
            MutationStatus::Unviable,
            MutationStatus::Skipped,
            MutationStatus::Error,
            MutationStatus::Pending,
            MutationStatus::Running,
        ]);
        let summary = RunSummary::from_snapshot(&snapshot);
        assert_eq!(summary.total, 8);
        assert_eq!(summary.killed, 1);
        assert_eq!(summary.survived, 1);
        assert_eq!(summary.timeout, 1);
        assert_eq!(summary.unviable, 1);
        assert_eq!(summary.skipped, 1);
        assert_eq!(summary.error, 1);
        assert_eq!(summary.incomplete, 2);
        assert!((summary.mutation_score - (1.0 / 6.0 * 100.0)).abs() < 1e-12);
    }

    #[test]
    fn summary_handles_no_testable_mutants() {
        let snapshot = snapshot_with_statuses(&[MutationStatus::Skipped, MutationStatus::Unviable]);
        let summary = RunSummary::from_snapshot(&snapshot);
        assert_eq!(summary.mutation_score, 100.0);
    }

    #[test]
    fn report_renders_json_and_markdown() {
        let snapshot = snapshot_with_statuses(&[MutationStatus::Killed, MutationStatus::Pending]);
        let md = render_report(&snapshot, ReportFormat::Markdown);
        assert!(md.contains("# Mutation Run run-report"));
        assert!(md.contains("| killed | 1 |"));
        assert!(md.contains("mutation score"));

        let json = render_report(&snapshot, ReportFormat::Json);
        assert!(json.contains("\"run_id\": \"run-report\""));
        assert!(json.contains("\"incomplete\": 1"));
    }
}
