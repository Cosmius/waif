use std::error::Error;
use std::fmt;
use std::path::{Path, PathBuf};

use clap::Args;

use super::{relative_path, severity_label};
use crate::parser::Severity;
use crate::protocol::{
    discover_artifact_paths, library_manifest, read_manifest, task_manifest,
    ArtifactClassification, ArtifactFile, WorkflowDir,
};

type Result<T> = std::result::Result<T, Box<dyn Error>>;

#[derive(Args)]
#[command(after_help = indoc::indoc! {"
    Examples:
      waif check
      waif check .waif/current_task/goal.md"
})]
pub struct CheckCommand {
    /// Artifact file or task directory; omit to check the current task.
    path: Option<PathBuf>,
}

impl CheckCommand {
    pub fn run(self) -> Result<()> {
        let workspace_dir = std::env::current_dir()?;
        let (paths, manifest_dirs) = match self.path {
            Some(path) if path.is_dir() => {
                let task_dir = std::path::absolute(&path)?;
                let library_dir = task_dir
                    .ancestors()
                    .nth(2)
                    .unwrap_or(Path::new("/"))
                    .to_owned();
                (
                    discover_artifact_paths(&path)?,
                    Some((library_dir, task_dir)),
                )
            }
            Some(path) => (discover_artifact_paths(&path)?, None),
            None => {
                let workflow_dir = WorkflowDir::for_workspace(workspace_dir.clone());
                let task = workflow_dir
                    .current_task()?
                    .ok_or("no current workflow task")?;
                (
                    task.artifact_paths()?,
                    Some((workflow_dir.library_dir(), task.path)),
                )
            }
        };

        let mut bad_artifacts = 0;
        let mut classification = ArtifactClassification::Good;
        if let Some((library_dir, task_dir)) = manifest_dirs {
            for manifest in [library_manifest(&library_dir), task_manifest(&task_dir)] {
                for (severity, message) in read_manifest(&manifest).findings() {
                    if severity == Severity::Error {
                        classification = ArtifactClassification::Bad;
                    }
                    eprintln!(
                        "{}: {}: {message}",
                        relative_path(&workspace_dir, &manifest),
                        severity_label(severity)
                    );
                }
            }
        }
        let paths = if classification == ArtifactClassification::Bad {
            Vec::new()
        } else {
            paths
        };
        for path in &paths {
            let display_path = relative_path(&workspace_dir, path);
            match ArtifactFile::read(path.clone()) {
                Ok(file) => {
                    let check = file.check();
                    classification =
                        ArtifactClassification::worse(classification, check.classification());

                    for diagnostic in check.diagnostics() {
                        eprintln!(
                            "{display_path}:{}: {}: {}",
                            diagnostic.line(),
                            severity_label(diagnostic.severity()),
                            diagnostic.message()
                        );
                    }

                    match check.classification() {
                        ArtifactClassification::Incomplete => {
                            eprintln!("{display_path}: INCOMPLETE");
                        }
                        ArtifactClassification::Bad => bad_artifacts += 1,
                        ArtifactClassification::Good => {}
                    }
                }
                Err(error) => {
                    bad_artifacts += 1;
                    classification = ArtifactClassification::Bad;
                    eprintln!("{display_path}: {error}");
                }
            }
        }

        println!(
            "checked {} artifact(s); {bad_artifacts} invalid; {classification}",
            paths.len(),
        );
        if classification != ArtifactClassification::Bad {
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
