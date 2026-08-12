use std::error::Error;
use std::fmt;
use std::path::PathBuf;

use clap::Args;

use super::relative_path;
use crate::parser::Severity;
use crate::protocol::{discover_artifact_paths, ArtifactFile, WorkflowDir};

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
            Some(path) => discover_artifact_paths(&path)?,
            None => {
                let workflow_dir = WorkflowDir::for_workspace(workspace_dir.clone());
                workflow_dir
                    .current_task()?
                    .ok_or("no current workflow task")?
                    .artifact_paths()?
            }
        };

        let mut bad_artifacts = 0;
        for path in &paths {
            let display_path = relative_path(&workspace_dir, path);
            match ArtifactFile::read(path.clone()) {
                Ok(file) => {
                    let diagnostics = file.diagnostics();
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
