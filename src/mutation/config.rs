//! Mutation run configuration.

use std::path::PathBuf;

use serde::{Deserialize, Serialize};

/// Configuration for a mutation run.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct MutationConfig {
    /// Project directory where cargo commands are executed.
    pub project_dir: PathBuf,
    /// Root directory where run state is persisted.
    pub run_root: PathBuf,
    /// Optional substring filter for mutant selection.
    pub filter: Option<String>,
    /// Optional per-mutant timeout hint in seconds.
    pub timeout_secs: Option<u64>,
}

impl Default for MutationConfig {
    fn default() -> Self {
        let project_dir = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
        let run_root = project_dir
            .join(".kitchensink-testing")
            .join("mutation")
            .join("runs");
        Self {
            project_dir,
            run_root,
            filter: None,
            timeout_secs: None,
        }
    }
}

impl MutationConfig {
    /// Set project directory.
    pub fn with_project_dir(mut self, project_dir: impl Into<PathBuf>) -> Self {
        self.project_dir = project_dir.into();
        self
    }

    /// Set run-state root.
    pub fn with_run_root(mut self, run_root: impl Into<PathBuf>) -> Self {
        self.run_root = run_root.into();
        self
    }

    /// Set selector filter.
    pub fn with_filter(mut self, filter: impl Into<String>) -> Self {
        self.filter = Some(filter.into());
        self
    }

    /// Set timeout in seconds.
    pub fn with_timeout_secs(mut self, timeout_secs: u64) -> Self {
        self.timeout_secs = Some(timeout_secs);
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_paths_and_builder_overrides_work() {
        let default = MutationConfig::default();
        assert!(default
            .run_root
            .ends_with(".kitchensink-testing/mutation/runs"));

        let cfg = MutationConfig::default()
            .with_project_dir("/tmp/project-a")
            .with_run_root("/tmp/runs-a")
            .with_filter("abc")
            .with_timeout_secs(42);

        assert_eq!(cfg.project_dir, PathBuf::from("/tmp/project-a"));
        assert_eq!(cfg.run_root, PathBuf::from("/tmp/runs-a"));
        assert_eq!(cfg.filter.as_deref(), Some("abc"));
        assert_eq!(cfg.timeout_secs, Some(42));
    }
}
