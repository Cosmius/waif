use std::error::Error;
use std::ffi::OsStr;
use std::fmt;
use std::fs;
use std::path::{Path, PathBuf};

use clap::Args;

use super::relative_path;
use crate::goal;
use crate::parser::{self, Severity};
use crate::plan;
use crate::protocol::WorkflowDir;

type Result<T> = std::result::Result<T, Box<dyn Error>>;

#[derive(Args)]
pub struct CheckCommand {
    /// An artifact file or a directory containing task artifacts.
    path: Option<PathBuf>,
}

impl CheckCommand {
    pub fn run(self) -> Result<()> {
        let workspace_dir = std::env::current_dir()?;
        let paths = match self.path {
            Some(path) => artifact_paths(&path)?,
            None => current_task_artifact_paths(&workspace_dir)?,
        };

        let mut bad_artifacts = 0;
        for path in &paths {
            let display_path = relative_path(&workspace_dir, path);
            match fs::read_to_string(path) {
                Ok(content) => {
                    let diagnostics = check_diagnostics(path, &content);
                    let has_errors = diagnostics
                        .iter()
                        .any(|diagnostic| diagnostic.severity() == Severity::Error);

                    for diagnostic in diagnostics {
                        let severity = match diagnostic.severity() {
                            Severity::Error => "ERROR",
                            Severity::Warning => "WARNING",
                        };
                        eprintln!(
                            "{display_path}:{}: {severity}: {}",
                            diagnostic.line(),
                            diagnostic.message()
                        );
                    }

                    if has_errors {
                        bad_artifacts += 1;
                    }
                }
                Err(error) => {
                    bad_artifacts += 1;
                    eprintln!("{display_path}: {error}");
                }
            }
        }

        println!(
            "checked {} artifact(s); {bad_artifacts} invalid",
            paths.len()
        );
        if bad_artifacts == 0 {
            Ok(())
        } else {
            Err(Box::new(CheckFailed))
        }
    }
}

#[derive(Debug)]
struct CheckFailed;

impl fmt::Display for CheckFailed {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "task artifact check failed")
    }
}

impl Error for CheckFailed {}

fn check_diagnostics(path: &Path, content: &str) -> Vec<parser::Diagnostic> {
    if path.file_name() == Some(OsStr::new("goal.md")) {
        goal::check(content)
    } else if path.file_name() == Some(OsStr::new("plan.md")) {
        plan::check(content)
    } else {
        parser::parse(content).err().unwrap_or_default()
    }
}

fn current_task_artifact_paths(workspace_dir: &Path) -> Result<Vec<PathBuf>> {
    let workflow_dir = WorkflowDir::for_workspace(workspace_dir.to_owned());
    let task = workflow_dir
        .current_task()?
        .ok_or("no current workflow task")?;
    artifact_paths(&task.path)
}

fn artifact_paths(path: &Path) -> Result<Vec<PathBuf>> {
    let Ok(entries) = sorted_directory_entries(path) else {
        return Ok(vec![path.to_owned()]);
    };

    let mut paths = Vec::new();
    for entry in entries {
        if is_task_artifact_name(&entry.file_name().to_string_lossy()) {
            paths.push(entry.path());
        }
    }
    collect_step_artifact_paths(&path.join("steps"), &mut paths);
    paths.sort();
    Ok(paths)
}

fn collect_step_artifact_paths(steps_directory: &Path, paths: &mut Vec<PathBuf>) {
    let Ok(step_entries) = sorted_directory_entries(steps_directory) else {
        return;
    };
    for step_entry in step_entries {
        let Ok(entries) = sorted_directory_entries(&step_entry.path()) else {
            continue;
        };
        for entry in entries {
            if is_step_artifact_name(&entry.file_name().to_string_lossy()) {
                paths.push(entry.path());
            }
        }
    }
}

fn sorted_directory_entries(directory: &Path) -> std::io::Result<Vec<fs::DirEntry>> {
    let mut entries = fs::read_dir(directory)?.collect::<std::io::Result<Vec<_>>>()?;
    entries.sort_by_key(|entry| entry.file_name());
    Ok(entries)
}

fn is_task_artifact_name(name: &str) -> bool {
    matches!(name, "goal.md" | "plan.md" | "review.md")
}

fn is_step_artifact_name(name: &str) -> bool {
    name == "step.md" || is_numbered_review_name(name)
}

fn is_numbered_review_name(name: &str) -> bool {
    let Some(number) = name
        .strip_prefix("review")
        .and_then(|name| name.strip_suffix(".md"))
    else {
        return false;
    };
    !number.is_empty() && number.bytes().all(|byte| byte.is_ascii_digit())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::TestDir;

    #[test]
    fn directory_discovery_finds_only_task_artifacts() {
        let directory = TestDir::new("check-directory-discovery");
        let step_dir = directory.path.join("steps/01-example");
        let unrelated_dir = directory.path.join("other/deep");
        fs::create_dir_all(&step_dir).expect("test directory should be created");
        fs::create_dir_all(&unrelated_dir).expect("test directory should be created");
        for path in [
            directory.path.join("goal.md"),
            directory.path.join("step.md"),
            directory.path.join("notes.md"),
            step_dir.join("step.md"),
            step_dir.join("goal.md"),
            step_dir.join("review1.md"),
            step_dir.join("review-latest.md"),
            unrelated_dir.join("goal.md"),
        ] {
            fs::write(path, "test").expect("test file should be written");
        }

        let paths = artifact_paths(&directory.path).expect("discovery should succeed");

        assert_eq!(paths.len(), 3);
        assert!(paths.iter().any(|path| path.ends_with("goal.md")));
        assert!(paths.iter().any(|path| path.ends_with("step.md")));
        assert!(paths.iter().any(|path| path.ends_with("review1.md")));
    }

    #[test]
    fn recognizes_artifact_names_in_their_expected_locations() {
        for name in ["goal.md", "plan.md", "review.md"] {
            assert!(
                is_task_artifact_name(name),
                "{name} should be recognized at the task root"
            );
        }
        for name in ["step.md", "review12.md"] {
            assert!(
                is_step_artifact_name(name),
                "{name} should be recognized in a step directory"
            );
        }
        for name in ["notes.md", "reviewN.md", "review.md.bak", "review-1.md"] {
            assert!(
                !is_task_artifact_name(name) && !is_step_artifact_name(name),
                "{name} should be ignored"
            );
        }
    }
}
