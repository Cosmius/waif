use std::error::Error;
use std::fmt;
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

use crate::parser::Severity;

mod artifact;
mod inspection;
mod manifest;
mod plan;
mod step;
mod task;
mod workflow;

pub(crate) use artifact::{ArtifactCheck, ArtifactClassification, ArtifactFile, ArtifactKind};
pub use manifest::SCHEMA_VERSION;
pub(crate) use manifest::{library_manifest, read_manifest, task_manifest};
pub(crate) use task::{discover_artifact_paths, Task};
pub(crate) use workflow::{Initialisation, WorkflowDir};

type Result<T> = std::result::Result<T, Box<dyn Error>>;

#[derive(Debug)]
enum ProtocolError {
    ArtifactKindMismatch,
    ArtifactOutsideTask,
    GoalInvalid {
        path: std::path::PathBuf,
        diagnostics: Vec<crate::parser::Diagnostic>,
    },
    GoalMissing,
    GoalPreconditions(Vec<String>),
    InvalidCurrentTask,
    InvalidTaskSlug,
    MalformedTaskName,
    TaskDoesNotExist,
    TaskOutsideLibrary,
}

impl fmt::Display for ProtocolError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ArtifactKindMismatch => {
                write!(formatter, "artifact kind does not match task accessor")
            }
            Self::ArtifactOutsideTask => {
                write!(formatter, "artifact is outside the task directory")
            }
            Self::GoalInvalid { path, diagnostics } => {
                for (index, diagnostic) in diagnostics.iter().enumerate() {
                    if index > 0 {
                        formatter.write_str("\n")?;
                    }
                    write!(
                        formatter,
                        "{}:{}: {}: {}",
                        path.display(),
                        diagnostic.line(),
                        match diagnostic.severity() {
                            Severity::Error => "ERROR",
                            Severity::Warning => "WARNING",
                        },
                        diagnostic.message()
                    )?;
                }
                Ok(())
            }
            Self::GoalMissing => write!(formatter, "current task has no goal.md"),
            Self::GoalPreconditions(blockers) => {
                for (index, blocker) in blockers.iter().enumerate() {
                    if index > 0 {
                        formatter.write_str("\n")?;
                    }
                    formatter.write_str(blocker)?;
                }
                Ok(())
            }
            Self::InvalidCurrentTask => write!(formatter, "current task is invalid"),
            Self::InvalidTaskSlug => write!(
                formatter,
                "task slug must be lowercase letters, numbers, and hyphens"
            ),
            Self::MalformedTaskName => write!(
                formatter,
                "task name must use YYYYMMDD-N-lowercase-hyphenated-slug"
            ),
            Self::TaskDoesNotExist => write!(formatter, "task dir does not exist"),
            Self::TaskOutsideLibrary => {
                write!(formatter, "task directory is outside the workflow library")
            }
        }
    }
}

impl Error for ProtocolError {}

/// Replace an artifact through a temporary file renamed over the target.
fn replace_artifact(path: &Path, contents: &str) -> Result<()> {
    let nonce = SystemTime::now().duration_since(UNIX_EPOCH)?.as_nanos();
    let name = path
        .file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_default();
    let pid = std::process::id();
    let temporary = path.with_file_name(format!(".{name}.{pid}.{nonce}.tmp"));
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&temporary)?;
    let result = (|| -> Result<()> {
        file.write_all(contents.as_bytes())?;
        drop(file);
        fs::rename(&temporary, path)?;
        Ok(())
    })();
    if result.is_err() {
        let _ = fs::remove_file(&temporary);
    }
    result
}

#[cfg(test)]
mod tests;
