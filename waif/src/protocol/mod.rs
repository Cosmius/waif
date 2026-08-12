use std::error::Error;
use std::fmt;

mod artifact;
mod task;
mod workflow;

pub(crate) use artifact::{ArtifactFile, ArtifactKind};
pub(crate) use task::{discover_artifact_paths, Task};
pub(crate) use workflow::WorkflowDir;

type Result<T> = std::result::Result<T, Box<dyn Error>>;

#[derive(Debug)]
enum ProtocolError {
    ArtifactKindMismatch,
    ArtifactOutsideTask,
    CurrentTaskEscapesTasksDir,
    InvalidCurrentTask,
    InvalidTaskSlug,
    InvalidTaskName,
    TaskDoesNotExist,
    TasksDirMissing,
    WorkflowDirMissing,
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
            Self::CurrentTaskEscapesTasksDir => {
                write!(formatter, "current task must resolve inside .waif/tasks")
            }
            Self::InvalidCurrentTask => write!(formatter, "current task is invalid"),
            Self::InvalidTaskSlug => write!(
                formatter,
                "task slug must be lowercase letters, numbers, and hyphens"
            ),
            Self::InvalidTaskName => write!(formatter, "current task name is invalid"),
            Self::TaskDoesNotExist => write!(formatter, "task dir does not exist"),
            Self::TasksDirMissing => write!(formatter, "workflow tasks dir is missing"),
            Self::WorkflowDirMissing => write!(formatter, "workflow dir is missing"),
        }
    }
}

impl Error for ProtocolError {}

#[cfg(test)]
mod tests;
