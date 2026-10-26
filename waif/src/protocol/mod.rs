use std::error::Error;
use std::fmt;

use crate::parser::Severity;

mod artifact;
mod git;
mod inspection;
mod plan;
mod task;
mod workflow;

pub(crate) use artifact::{ArtifactCheck, ArtifactClassification, ArtifactFile, ArtifactKind};
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
    InvalidTaskName,
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
            Self::InvalidTaskName => write!(formatter, "current task name is invalid"),
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

#[cfg(test)]
mod tests;
