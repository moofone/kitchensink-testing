use serde::Serialize;

use super::events::{RunConfigSnapshot, RunMetadata};
use super::state::{MutantState, MutationStatus, RunSnapshot};

/// Supported output formats for run reports.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReportFormat {
    /// Markdown summary.
    Markdown,
    /// JSON summary with all mutants inline.
    Json,
    /// SARIF format for GitHub Code Scanning.
    Sarif,
    /// JUnit XML format for CI systems.
    Junit,
}

/// Per-mutant report entry for LLM-friendly JSON output.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct MutantReport {
    /// Mutant id.
    pub id: String,
    /// Human-readable label.
    pub label: String,
    /// Source file path.
    pub source_file: String,
    /// Source line number.
    pub source_line: u32,
    /// Mutation type classification.
    pub mutation_type: String,
    /// Original code snippet.
    pub original_code: String,
    /// Mutated code snippet.
    pub mutated_code: String,
    /// Execution status.
    pub status: String,
    /// Duration in milliseconds.
    pub duration_ms: Option<u64>,
    /// Tests that ran.
    pub tests_run: Vec<String>,
    /// Tests that failed.
    pub tests_failed: Vec<TestFailureReport>,
    /// Stdout preview.
    pub stdout_preview: Option<String>,
    /// Stderr preview.
    pub stderr_preview: Option<String>,
}

/// Test failure report entry.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct TestFailureReport {
    /// Test name.
    pub test_name: String,
    /// Failure message.
    pub message: Option<String>,
}

impl From<&MutantState> for MutantReport {
    fn from(state: &MutantState) -> Self {
        Self {
            id: state.spec.id.clone(),
            label: state.spec.label.clone(),
            source_file: state.spec.source_file.clone(),
            source_line: state.spec.source_line,
            mutation_type: state.spec.mutation_type.to_string(),
            original_code: state.spec.original_code.clone(),
            mutated_code: state.spec.mutated_code.clone(),
            status: status_to_string(&state.status),
            duration_ms: state.duration_ms,
            tests_run: state.tests_run.clone(),
            tests_failed: state
                .tests_failed
                .iter()
                .map(|f| TestFailureReport {
                    test_name: f.test_name.clone(),
                    message: f.message.clone(),
                })
                .collect(),
            stdout_preview: state.stdout_preview.clone(),
            stderr_preview: state.stderr_preview.clone(),
        }
    }
}

fn status_to_string(status: &MutationStatus) -> String {
    match status {
        MutationStatus::Pending => "pending".to_string(),
        MutationStatus::Running => "running".to_string(),
        MutationStatus::Killed => "killed".to_string(),
        MutationStatus::Survived => "survived".to_string(),
        MutationStatus::Timeout => "timeout".to_string(),
        MutationStatus::Unviable => "unviable".to_string(),
        MutationStatus::Skipped => "skipped".to_string(),
        MutationStatus::Error => "error".to_string(),
    }
}

/// Run metadata for reports.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct RunMetadataReport {
    /// Rust compiler version.
    pub rustc_version: String,
    /// Cargo version.
    pub cargo_version: String,
    /// cargo-mutants version.
    pub cargo_mutants_version: String,
    /// Git commit hash.
    pub git_commit: String,
    /// Git branch name.
    pub git_branch: String,
    /// Operating system.
    pub os: String,
    /// Architecture.
    pub arch: String,
}

impl From<RunMetadata> for RunMetadataReport {
    fn from(m: RunMetadata) -> Self {
        Self {
            rustc_version: m.rustc_version,
            cargo_version: m.cargo_version,
            cargo_mutants_version: m.cargo_mutants_version,
            git_commit: m.git_commit,
            git_branch: m.git_branch,
            os: m.os,
            arch: m.arch,
        }
    }
}

/// Run config for reports.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct RunConfigReport {
    /// Timeout in seconds.
    pub timeout_secs: Option<u64>,
    /// Filter applied.
    pub filter: Option<String>,
}

impl From<RunConfigSnapshot> for RunConfigReport {
    fn from(c: RunConfigSnapshot) -> Self {
        Self {
            timeout_secs: c.timeout_secs,
            filter: c.filter,
        }
    }
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
    let mutants: Vec<MutantReport> = snapshot.mutants.values().map(MutantReport::from).collect();

    let config = snapshot.info.config.clone().map(RunConfigReport::from);
    let metadata = snapshot.info.metadata.clone().map(RunMetadataReport::from);

    match format {
        ReportFormat::Json => serde_json::to_string_pretty(&serde_json::json!({
            "run_id": snapshot.run_id,
            "completed": snapshot.completed,
            "interrupted": snapshot.interrupted,
            "malformed_lines": snapshot.malformed_lines,
            "config": config,
            "metadata": metadata,
            "summary": summary,
            "mutants": mutants,
        }))
        .expect("report JSON should serialize"),
        ReportFormat::Markdown => {
            let mut out = format!("# Mutation Run {}\n\n", snapshot.run_id);

            out.push_str(&format!(
                "- completed: {}\n- interrupted: {}\n- malformed lines: {}\n\n",
                snapshot.completed, snapshot.interrupted, snapshot.malformed_lines
            ));

            if let Some(ref meta) = metadata {
                out.push_str("## Environment\n\n");
                if !meta.git_commit.is_empty() {
                    out.push_str(&format!("- git commit: {}\n", meta.git_commit));
                }
                if !meta.git_branch.is_empty() {
                    out.push_str(&format!("- git branch: {}\n", meta.git_branch));
                }
                if !meta.rustc_version.is_empty() {
                    out.push_str(&format!("- rustc: {}\n", meta.rustc_version));
                }
                out.push_str("\n");
            }

            out.push_str("## Summary\n\n| metric | count |\n|---|---:|\n");
            out.push_str(&format!("| total | {} |\n", summary.total));
            out.push_str(&format!("| killed | {} |\n", summary.killed));
            out.push_str(&format!("| survived | {} |\n", summary.survived));
            out.push_str(&format!("| timeout | {} |\n", summary.timeout));
            out.push_str(&format!("| unviable | {} |\n", summary.unviable));
            out.push_str(&format!("| skipped | {} |\n", summary.skipped));
            out.push_str(&format!("| error | {} |\n", summary.error));
            out.push_str(&format!("| incomplete | {} |\n", summary.incomplete));
            out.push_str(&format!(
                "| mutation score | {:.2}% |\n",
                summary.mutation_score
            ));

            if !mutants.is_empty() {
                out.push_str("\n## Mutants\n\n");
                for m in &mutants {
                    out.push_str(&format!("### {}\n\n", m.id));
                    out.push_str(&format!("- **label**: {}\n", m.label));
                    if !m.source_file.is_empty() {
                        out.push_str(&format!(
                            "- **location**: {}:{}\n",
                            m.source_file, m.source_line
                        ));
                    }
                    out.push_str(&format!("- **type**: {}\n", m.mutation_type));
                    out.push_str(&format!("- **status**: {}\n", m.status));
                    if let Some(d) = m.duration_ms {
                        out.push_str(&format!("- **duration**: {}ms\n", d));
                    }
                    if !m.original_code.is_empty() || !m.mutated_code.is_empty() {
                        out.push_str("\n**diff**:\n");
                        if !m.original_code.is_empty() {
                            out.push_str(&format!("- original: `{}`\n", m.original_code));
                        }
                        if !m.mutated_code.is_empty() {
                            out.push_str(&format!("- mutated: `{}`\n", m.mutated_code));
                        }
                    }
                    if !m.tests_failed.is_empty() {
                        out.push_str("\n**failed tests**:\n");
                        for f in &m.tests_failed {
                            out.push_str(&format!(
                                "- {}{}\n",
                                f.test_name,
                                f.message
                                    .as_ref()
                                    .map(|m| format!(": {}", m))
                                    .unwrap_or_default()
                            ));
                        }
                    }
                    out.push_str("\n");
                }
            }

            out
        }
        ReportFormat::Sarif => render_sarif_report(snapshot, &summary, &mutants),
        ReportFormat::Junit => render_junit_report(snapshot, &summary, &mutants),
    }
}

fn render_sarif_report(
    snapshot: &RunSnapshot,
    summary: &RunSummary,
    mutants: &[MutantReport],
) -> String {
    let results: Vec<serde_json::Value> = mutants
        .iter()
        .filter(|m| m.status == "survived")
        .map(|m| {
            serde_json::json!({
                "ruleId": "survived-mutant",
                "level": "warning",
                "message": {
                    "text": format!("Mutant survived: {} in {} at line {}", m.label, m.source_file, m.source_line)
                },
                "locations": [{
                    "physicalLocation": {
                        "artifactLocation": {
                            "uri": m.source_file
                        },
                        "region": {
                            "startLine": m.source_line
                        }
                    }
                }]
            })
        })
        .collect();

    serde_json::to_string_pretty(&serde_json::json!({
        "$schema": "https://raw.githubusercontent.com/oasis-tcs/sarif-spec/master/Schemata/sarif-schema-2.1.0.json",
        "version": "2.1.0",
        "runs": [{
            "tool": {
                "driver": {
                    "name": "kitchensink-testing",
                    "version": env!("CARGO_PKG_VERSION"),
                    "informationUri": "https://github.com/moofone/rust-pbt",
                    "rules": [{
                        "id": "survived-mutant",
                        "shortDescription": {
                            "text": "Survived Mutant"
                        },
                        "fullDescription": {
                            "text": "A mutation that was not caught by any test, indicating a potential gap in test coverage."
                        },
                        "defaultConfiguration": {
                            "level": "warning"
                        }
                    }]
                }
            },
            "results": results,
            "properties": {
                "runId": snapshot.run_id,
                "mutationScore": summary.mutation_score,
                "totalMutants": summary.total,
                "killed": summary.killed,
                "survived": summary.survived
            }
        }]
    }))
    .expect("SARIF JSON should serialize")
}

fn render_junit_report(
    snapshot: &RunSnapshot,
    _summary: &RunSummary,
    mutants: &[MutantReport],
) -> String {
    let mut out = String::new();
    out.push_str("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n");
    out.push_str("<testsuites>\n");
    out.push_str(&format!(
        "  <testsuite name=\"mutation-{}\" tests=\"{}\">\n",
        snapshot.run_id,
        mutants.len()
    ));

    for m in mutants {
        let classname = if m.source_file.is_empty() {
            "mutation".to_string()
        } else {
            m.source_file.replace('/', ".")
        };

        match m.status.as_str() {
            "killed" => {
                out.push_str(&format!(
                    "    <testcase classname=\"{}\" name=\"{}\"/>\n",
                    classname, m.id
                ));
            }
            "survived" => {
                out.push_str(&format!(
                    "    <testcase classname=\"{}\" name=\"{}\">\n",
                    classname, m.id
                ));
                out.push_str(&format!(
                    "      <failure message=\"Mutant survived\">{}</failure>\n",
                    xml_escape(&m.label)
                ));
                out.push_str("    </testcase>\n");
            }
            "timeout" => {
                out.push_str(&format!(
                    "    <testcase classname=\"{}\" name=\"{}\">\n",
                    classname, m.id
                ));
                out.push_str(&format!("      <skipped message=\"Timeout\"/>\n"));
                out.push_str("    </testcase>\n");
            }
            _ => {
                out.push_str(&format!(
                    "    <testcase classname=\"{}\" name=\"{}\">\n",
                    classname, m.id
                ));
                out.push_str(&format!(
                    "      <skipped message=\"{}\"/>\n",
                    xml_escape(&m.status)
                ));
                out.push_str("    </testcase>\n");
            }
        }
    }

    out.push_str("  </testsuite>\n");
    out.push_str("</testsuites>\n");
    out
}

fn xml_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use super::*;
    use crate::mutation::events::{MutantSpec, MutationType};
    use crate::mutation::state::{MutantState, MutationStatus, RunInfo};

    fn test_mutant(id: &str) -> MutantState {
        MutantState {
            spec: MutantSpec {
                id: id.to_string(),
                label: "label".to_string(),
                selector: "selector".to_string(),
                source_file: String::new(),
                source_line: 0,
                mutation_type: MutationType::Unknown,
                original_code: String::new(),
                mutated_code: String::new(),
            },
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
        }
    }

    fn snapshot_with_statuses(statuses: &[MutationStatus]) -> RunSnapshot {
        let mut mutants = BTreeMap::new();
        for (idx, status) in statuses.iter().enumerate() {
            let id = format!("m{idx}");
            let mut state = test_mutant(&id);
            state.status = status.clone();
            mutants.insert(id, state);
        }

        RunSnapshot {
            run_id: "run-report".to_string(),
            mutants,
            malformed_lines: 0,
            interrupted: false,
            completed: true,
            info: RunInfo::default(),
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
        assert!(json.contains("\"mutants\":"));
    }

    #[test]
    fn report_renders_sarif_format() {
        let snapshot = snapshot_with_statuses(&[MutationStatus::Survived, MutationStatus::Killed]);
        let sarif = render_report(&snapshot, ReportFormat::Sarif);
        assert!(sarif.contains("\"$schema\""));
        assert!(sarif.contains("\"survived-mutant\""));
        assert!(sarif.contains("\"mutationScore\""));
    }

    #[test]
    fn report_renders_junit_format() {
        let snapshot = snapshot_with_statuses(&[MutationStatus::Survived, MutationStatus::Killed]);
        let junit = render_report(&snapshot, ReportFormat::Junit);
        assert!(junit.contains("<?xml version"));
        assert!(junit.contains("<testsuites>"));
        assert!(junit.contains("<failure"));
    }
}
