#![cfg(all(feature = "mutation", any(target_os = "linux", target_os = "macos")))]

use std::env;
use std::fs::{self, File};
use std::io::Write;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::thread;
use std::time::Duration;

use tempfile::tempdir;

fn fake_cargo_path(tmp: &Path) -> PathBuf {
    let bin_dir = tmp.join("fake-bin");
    fs::create_dir_all(&bin_dir).expect("fake bin dir should be created");

    let script = bin_dir.join("cargo");
    let mut file = File::create(&script).expect("fake cargo script should be created");
    file.write_all(
        br#"#!/usr/bin/env sh
set -e

if [ "$1" != "mutants" ]; then
  echo "unsupported command" >&2
  exit 1
fi

shift

if [ "$1" = "--help" ]; then
  echo "cargo mutants"
  echo "  --mutant <selector> execute one mutant"
  exit 0
fi

if [ "$1" = "--list" ]; then
  echo "add"
  echo "sub"
  exit 0
fi

if [ "$1" = "--mutant" ]; then
  selector="$2"
  sleep "${RUST_PBT_FAKE_CARGO_SLEEP:-0.2}"

  if [ "$selector" = "add" ]; then
    echo "mutant killed"
  else
    echo "mutant survived"
  fi
  exit 0
fi

echo "unsupported mutants flag: $1" >&2
exit 1
"#,
    )
    .expect("fake cargo script should be written");
    file.sync_all()
        .expect("fake cargo script should be flushed");
    fs::set_permissions(&script, PermissionsExt::from_mode(0o755))
        .expect("fake cargo script should be executable");

    bin_dir
}

fn run_cli_with_fake_cargo(args: &[&str], fake_bin: &Path) -> std::process::Output {
    let binary = PathBuf::from(env!("CARGO_BIN_EXE_cargo-pbt"));
    let original_path = env::var("PATH").unwrap_or_else(|_| String::new());

    Command::new(binary)
        .args(args)
        .env("PATH", format!("{}:{}", fake_bin.display(), original_path))
        .env(
            "RUST_PBT_FAKE_CARGO_SLEEP",
            Duration::from_millis(180).as_secs_f32().to_string(),
        )
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .expect("cli command should run")
}

fn run_id_from_output(output: &[u8]) -> String {
    let text = String::from_utf8_lossy(output);
    text.lines()
        .find_map(|line| line.strip_prefix("run id: "))
        .expect("output should include run id")
        .trim()
        .to_string()
}

#[test]
fn e2e_cli_mutation_run_interrupt_resume_report() {
    let tmp = tempdir().expect("tempdir should be created");
    let project_dir = tmp.path().join("project");
    fs::create_dir_all(&project_dir).expect("project dir should be created");
    let run_root = tmp.path().join("runs");
    fs::create_dir_all(&run_root).expect("run root should be created");

    let fake_bin = fake_cargo_path(tmp.path());
    let binary = PathBuf::from(env!("CARGO_BIN_EXE_cargo-pbt"));
    let original_path = env::var("PATH").unwrap_or_else(|_| String::new());

    let run_child = Command::new(&binary)
        .args([
            "mutate",
            "run",
            "--project",
            project_dir
                .to_str()
                .expect("project path should be valid utf-8"),
            "--run-root",
            run_root
                .to_str()
                .expect("run_root path should be valid utf-8"),
        ])
        .env("PATH", format!("{}:{}", fake_bin.display(), original_path))
        .env(
            "RUST_PBT_FAKE_CARGO_SLEEP",
            Duration::from_millis(200).as_secs_f32().to_string(),
        )
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("cli run should spawn");

    let child_pid = run_child.id();

    let interrupt = thread::spawn(move || {
        thread::sleep(Duration::from_millis(350));
        let _ = Command::new("kill")
            .arg("-INT")
            .arg(child_pid.to_string())
            .status();
    });

    let run_output = run_child
        .wait_with_output()
        .expect("run command should finish");
    interrupt
        .join()
        .expect("interrupt thread should join cleanly");

    if !run_output.status.success() {
        panic!(
            "run command should stay alive long enough to checkpoint. status={:?}, stdout={:?}, stderr={:?}",
            run_output.status.code(),
            String::from_utf8_lossy(&run_output.stdout),
            String::from_utf8_lossy(&run_output.stderr)
        );
    }

    let run_id = run_id_from_output(&run_output.stdout);
    let config = kitchensink_testing::mutation::MutationConfig::default().with_run_root(&run_root);
    let snapshot = kitchensink_testing::mutation::load_run_status(&config, &run_id)
        .expect("status should load after interrupt");
    assert!(snapshot.interrupted);
    assert!(!snapshot.completed);
    assert!(
        !snapshot.pending_mutants().is_empty(),
        "interrupt should leave at least one mutant pending or running"
    );

    let resume_output = run_cli_with_fake_cargo(
        &[
            "mutate",
            "resume",
            run_id.as_str(),
            "--project",
            project_dir
                .to_str()
                .expect("project path should be valid utf-8"),
            "--run-root",
            run_root
                .to_str()
                .expect("run_root path should be valid utf-8"),
        ],
        &fake_bin,
    );

    assert!(resume_output.status.success());
    let resumed = kitchensink_testing::mutation::load_run_status(&config, &run_id)
        .expect("status should load after resume");
    assert!(resumed.completed);
    assert_eq!(resumed.pending_mutants().len(), 0);

    let status_output = run_cli_with_fake_cargo(
        &[
            "mutate",
            "status",
            run_id.as_str(),
            "--run-root",
            run_root
                .to_str()
                .expect("run_root path should be valid utf-8"),
        ],
        &fake_bin,
    );
    assert!(status_output.status.success());
    let status_stdout = String::from_utf8_lossy(&status_output.stdout);
    assert!(status_stdout.contains("run id: "));
    assert!(status_stdout.contains("completed: true"));

    let report_output = run_cli_with_fake_cargo(
        &[
            "mutate",
            "report",
            run_id.as_str(),
            "--format",
            "json",
            "--run-root",
            run_root
                .to_str()
                .expect("run_root path should be valid utf-8"),
        ],
        &fake_bin,
    );
    assert!(report_output.status.success());
    let report: serde_json::Value =
        serde_json::from_slice(&report_output.stdout).expect("report should be valid json");
    assert_eq!(report["run_id"].as_str(), Some(run_id.as_str()));
    assert_eq!(report["completed"].as_bool(), Some(true));
}
