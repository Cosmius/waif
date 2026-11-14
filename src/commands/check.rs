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
        let (paths, library_manifest, task_manifest) = match self.path {
            Some(path) if path.is_dir() => (
                discover_artifact_paths(&path)?,
                None,
                Some(task_manifest(&path)),
            ),
            Some(path) => (vec![path], None, None),
            None => {
                let workflow_dir = WorkflowDir::for_workspace(workspace_dir.clone());
                let task = workflow_dir
                    .current_task()?
                    .ok_or("no current workflow task")?;
                (
                    task.artifact_paths()?,
                    Some(library_manifest(&workflow_dir.library_dir())),
                    Some(task_manifest(&task.path)),
                )
            }
        };

        let mut bad_artifacts = 0;
        let mut classification = ArtifactClassification::Good;
        if let Some(manifest) = &library_manifest {
            if manifest_has_error(&workspace_dir, manifest) {
                classification = ArtifactClassification::Bad;
            }
        }
        if let Some(manifest) = &task_manifest {
            if manifest_has_error(&workspace_dir, manifest) {
                classification = ArtifactClassification::Bad;
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

/// Reports a manifest's findings and returns whether any is an error.
fn manifest_has_error(workspace_dir: &Path, manifest: &Path) -> bool {
    let mut has_error = false;
    for (severity, message) in read_manifest(manifest).findings() {
        if severity == Severity::Error {
            has_error = true;
        }
        eprintln!(
            "{}: {}: {message}",
            relative_path(workspace_dir, manifest),
            severity_label(severity)
        );
    }
    has_error
}

#[derive(Debug)]
struct CheckFailed;

impl fmt::Display for CheckFailed {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "task artifact check failed")
    }
}

impl Error for CheckFailed {}
