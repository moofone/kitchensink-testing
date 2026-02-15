use std::fs;
use std::path::{Path, PathBuf};
use std::env;

use anyhow::Result;
use clap::{Parser, Subcommand, ValueEnum};

use rust_pbt::mutation::state::MutantState;
use rust_pbt::mutation::{
    CargoMutantsEngine, MutationConfig, MutationStatus, ReportFormat, RunSummary, load_run_status,
    render_report, resume_run, run_new,
};

#[derive(Debug, Parser)]
#[command(name = "cargo-kitchensink")]
#[command(about = "Mutation orchestration for kitchensink-testing")]
struct Cli {
    #[command(subcommand)]
    command: TopCommand,
}

#[derive(Debug, Subcommand)]
enum TopCommand {
    /// Mutation workflows.
    Mutate {
        #[command(subcommand)]
        command: MutateCommand,
    },
}

#[derive(Debug, Subcommand)]
enum MutateCommand {
    /// Start a mutation run. If an incomplete interrupted run exists, it resumes automatically.
    Run {
        /// Project directory.
        #[arg(long)]
        project: Option<PathBuf>,
        /// Run root directory.
        #[arg(long)]
        run_root: Option<PathBuf>,
        /// Optional substring filter for mutant selection.
        #[arg(long)]
        filter: Option<String>,
        /// Optional timeout hint in seconds.
        #[arg(long)]
        timeout_secs: Option<u64>,
    },
    /// Resume an existing run id.
    Resume {
        /// Existing run id.
        run_id: String,
        /// Project directory.
        #[arg(long)]
        project: Option<PathBuf>,
        /// Run root directory.
        #[arg(long)]
        run_root: Option<PathBuf>,
        /// Optional timeout hint in seconds.
        #[arg(long)]
        timeout_secs: Option<u64>,
    },
    /// Show status for run id.
    Status {
        /// Existing run id.
        run_id: String,
        /// Run root directory.
        #[arg(long)]
        run_root: Option<PathBuf>,
    },
    /// Render report for run id.
    Report {
        /// Existing run id.
        run_id: String,
        /// Output format.
        #[arg(long, value_enum, default_value = "md")]
        format: OutputFormat,
        /// Run root directory.
        #[arg(long)]
        run_root: Option<PathBuf>,
    },
    /// List mutants for run id.
    List {
        /// Existing run id.
        run_id: String,
        /// Emit JSON output.
        #[arg(long)]
        json: bool,
        /// Run root directory.
        #[arg(long)]
        run_root: Option<PathBuf>,
    },
    /// Show one mutant's recorded state.
    Inspect {
        /// Existing run id.
        run_id: String,
        /// Mutant id.
        mutant_id: String,
        /// Emit JSON output.
        #[arg(long)]
        json: bool,
        /// Emit captured stdout/stderr contents.
        #[arg(long, conflicts_with = "json")]
        log: bool,
        /// Run root directory.
        #[arg(long)]
        run_root: Option<PathBuf>,
    },
}

#[derive(Debug, Clone, Copy, ValueEnum)]
enum OutputFormat {
    /// Markdown
    Md,
    /// JSON
    Json,
}

fn make_config(
    project: Option<PathBuf>,
    run_root: Option<PathBuf>,
    filter: Option<String>,
    timeout_secs: Option<u64>,
) -> MutationConfig {
    let mut config = MutationConfig::default();
    if let Some(project) = project {
        config = config.with_project_dir(project);
    }
    if let Some(run_root) = run_root {
        config = config.with_run_root(run_root);
    }
    if let Some(filter) = filter {
        config = config.with_filter(filter);
    }
    if let Some(timeout_secs) = timeout_secs {
        config = config.with_timeout_secs(timeout_secs);
    }
    config
}

fn status_to_string(status: &MutationStatus) -> &'static str {
    match status {
        MutationStatus::Pending => "pending",
        MutationStatus::Running => "running",
        MutationStatus::Killed => "killed",
        MutationStatus::Survived => "survived",
        MutationStatus::Timeout => "timeout",
        MutationStatus::Unviable => "unviable",
        MutationStatus::Skipped => "skipped",
        MutationStatus::Error => "error",
    }
}

fn absolute_artifact_path(run_dir: &Path, maybe_relative: &Option<String>) -> Option<String> {
    maybe_relative
        .as_ref()
        .map(|path| run_dir.join(path).display().to_string())
}

fn print_artifact_contents(mutant_id: &str, label: &str, run_dir: &Path, path: &Option<String>) {
    if let Some(path) = path {
        let abs_path = run_dir.join(path);
        println!("--- {mutant_id} {label} ---");
        match fs::read(&abs_path) {
            Ok(raw) => {
                let text = String::from_utf8_lossy(&raw);
                println!("{text}");
            }
            Err(err) => {
                println!("<unreadable: {err}>");
            }
        }
    }
}

fn inspect_payload(run_id: &str, state: &MutantState, run_dir: &Path) -> serde_json::Value {
    serde_json::json!({
        "run_id": run_id,
        "mutant_id": state.spec.id,
        "label": state.spec.label,
        "selector": state.spec.selector,
        "status": status_to_string(&state.status),
        "started_at_ms": state.started_at_ms,
        "finished_at_ms": state.finished_at_ms,
        "duration_ms": state.duration_ms,
        "exit_code": state.exit_code,
        "artifacts": {
            "stdout": absolute_artifact_path(run_dir, &state.stdout_artifact_path),
            "stderr": absolute_artifact_path(run_dir, &state.stderr_artifact_path),
        },
        "last_error": state.last_error,
    })
}

fn main() -> Result<()> {
    let mut args: Vec<String> = env::args().collect();
    while args.len() >= 2 {
        match args[1].as_str() {
            "kitchensink" | "kitchensink-testing" | "pbt" => {
                args.remove(1);
            }
            _ => break,
        }
    }
    let cli = Cli::parse_from(args);
    let engine = CargoMutantsEngine;

    match cli.command {
        TopCommand::Mutate { command } => match command {
            MutateCommand::Run {
                project,
                run_root,
                filter,
                timeout_secs,
            } => {
                let config = make_config(project, run_root, filter, timeout_secs);
                let run = run_new(&config, &engine)?;
                let summary = RunSummary::from_snapshot(&run.snapshot);
                println!("run id: {}", run.run_id);
                println!("run dir: {}", run.run_dir.display());
                println!(
                    "summary: killed={}, survived={}, incomplete={}, mutation_score={:.2}%",
                    summary.killed, summary.survived, summary.incomplete, summary.mutation_score
                );
            }
            MutateCommand::Resume {
                run_id,
                project,
                run_root,
                timeout_secs,
            } => {
                let config = make_config(project, run_root, None, timeout_secs);
                let run = resume_run(&config, &run_id, &engine)?;
                let summary = RunSummary::from_snapshot(&run.snapshot);
                println!("run id: {}", run.run_id);
                println!(
                    "summary: killed={}, survived={}, incomplete={}, mutation_score={:.2}%",
                    summary.killed, summary.survived, summary.incomplete, summary.mutation_score
                );
            }
            MutateCommand::Status { run_id, run_root } => {
                let config = make_config(None, run_root, None, None);
                let snapshot = rust_pbt::mutation::load_run_status(&config, &run_id)?;
                let summary = RunSummary::from_snapshot(&snapshot);
                println!("run id: {}", snapshot.run_id);
                println!("completed: {}", snapshot.completed);
                println!("interrupted: {}", snapshot.interrupted);
                println!(
                    "summary: killed={}, survived={}, incomplete={}, mutation_score={:.2}%",
                    summary.killed, summary.survived, summary.incomplete, summary.mutation_score
                );
            }
            MutateCommand::Report {
                run_id,
                format,
                run_root,
            } => {
                let config = make_config(None, run_root, None, None);
                let format = match format {
                    OutputFormat::Md => ReportFormat::Markdown,
                    OutputFormat::Json => ReportFormat::Json,
                };
                let snapshot = rust_pbt::mutation::load_run_status(&config, &run_id)?;
                println!("{}", render_report(&snapshot, format));
            }
            MutateCommand::List {
                run_id,
                json,
                run_root,
            } => {
                let config = make_config(None, run_root, None, None);
                let run_dir = config.run_root.join(&run_id);
                let snapshot = load_run_status(&config, &run_id)?;
                if json {
                    let mutants: Vec<_> = snapshot
                        .mutants
                        .values()
                        .map(|state| inspect_payload(&snapshot.run_id, state, &run_dir))
                        .collect();
                    let output = serde_json::json!({
                        "run_id": snapshot.run_id,
                        "completed": snapshot.completed,
                        "interrupted": snapshot.interrupted,
                        "mutants": mutants,
                    });
                    println!("{}", serde_json::to_string_pretty(&output)?);
                } else {
                    for state in snapshot.mutants.values() {
                        println!(
                            "{}\t{:?}\t{}",
                            state.spec.id, state.status, state.spec.label
                        );
                    }
                }
            }
            MutateCommand::Inspect {
                run_id,
                mutant_id,
                json,
                log,
                run_root,
            } => {
                let config = make_config(None, run_root, None, None);
                let run_dir = config.run_root.join(&run_id);
                let snapshot = load_run_status(&config, &run_id)?;
                match snapshot.mutants.get(&mutant_id) {
                    Some(state) => {
                        if json || log {
                            if log {
                                print_artifact_contents(
                                    &state.spec.id,
                                    "stdout",
                                    &run_dir,
                                    &state.stdout_artifact_path,
                                );
                                print_artifact_contents(
                                    &state.spec.id,
                                    "stderr",
                                    &run_dir,
                                    &state.stderr_artifact_path,
                                );
                            }

                            if json {
                                println!(
                                    "{}",
                                    serde_json::to_string_pretty(&inspect_payload(
                                        &snapshot.run_id,
                                        state,
                                        &run_dir,
                                    ))?
                                );
                            }
                        } else {
                            println!("run_id: {}", snapshot.run_id);
                            println!("mutant_id: {}", state.spec.id);
                            println!("label: {}", state.spec.label);
                            println!("selector: {}", state.spec.selector);
                            println!("status: {:?}", state.status);
                            println!("duration_ms: {:?}", state.duration_ms);
                            if let Some(started_at_ms) = state.started_at_ms {
                                println!("started_at_ms: {}", started_at_ms);
                            }
                            if let Some(finished_at_ms) = state.finished_at_ms {
                                println!("finished_at_ms: {}", finished_at_ms);
                            }
                            if let Some(exit_code) = state.exit_code {
                                println!("exit_code: {}", exit_code);
                            }
                            if let Some(path) =
                                absolute_artifact_path(&run_dir, &state.stdout_artifact_path)
                            {
                                println!("stdout_artifact: {path}");
                            }
                            if let Some(path) =
                                absolute_artifact_path(&run_dir, &state.stderr_artifact_path)
                            {
                                println!("stderr_artifact: {path}");
                            }
                            if let Some(last_error) = state.last_error.as_deref() {
                                println!("last_error: {last_error}");
                            }
                        }
                    }
                    None => {
                        if json {
                            let output = serde_json::json!({
                                "run_id": snapshot.run_id,
                                "error": "mutant_not_found",
                                "mutant_id": mutant_id,
                            });
                            println!("{}", serde_json::to_string_pretty(&output)?);
                        } else {
                            println!("mutant not found: {}", mutant_id);
                        }
                        std::process::exit(1);
                    }
                }
            }
        },
    }

    Ok(())
}
