use std::error::Error;
use std::fmt;
use std::fs;
#[cfg(unix)]
use std::os::unix::fs::symlink;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use chrono::Local;

use crate::artifact::Artifact;
use crate::parser::{self, Diagnostic, ParserConfig};

type Result<T> = std::result::Result<T, Box<dyn Error>>;

pub struct WorkflowDir {
    pub path: PathBuf,
    _private: (),
}

impl WorkflowDir {
    pub fn for_workspace(workspace_dir: PathBuf) -> Self {
        Self {
            path: workspace_dir.join(".waif"),
            _private: (),
        }
    }

    pub fn tasks_dir(&self) -> PathBuf {
        self.path.join("tasks")
    }

    pub fn current_link(&self) -> PathBuf {
        self.tasks_dir().join("current")
    }

    pub fn exists(&self) -> bool {
        self.path.exists()
    }

    pub fn initialise(&self) -> Result<Option<String>> {
        fs::create_dir_all(self.tasks_dir())?;
        let exit_status = Command::new("git")
            .arg("init")
            .current_dir(&self.path)
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status();
        let git_error = match exit_status {
            Ok(status) if status.success() => None,
            Ok(status) => Some(format!("git init exited with {status}")),
            Err(error) => Some(error.to_string()),
        };
        Ok(git_error)
    }

    pub fn current_task(&self) -> Result<Option<Task>> {
        let tasks_dir = self.tasks_dir();
        let current_link = self.current_link();
        if !current_link.exists() {
            return Ok(None);
        }

        let task_dir = resolve_current_task(&tasks_dir, &current_link)?;
        Ok(Some(Task::new(task_dir)))
    }

    pub fn create_task(&self, slug: &str) -> Result<Task> {
        validate_slug(slug)?;
        if !self.path.is_dir() {
            return Err(Box::new(ProtocolError::WorkflowDirMissing));
        }
        if !self.tasks_dir().is_dir() {
            return Err(Box::new(ProtocolError::TasksDirMissing));
        }

        let date = Local::now().format("%Y%m%d").to_string();

        let task_name = format!(
            "{date}-{}-{slug}",
            next_task_sequence(&self.tasks_dir(), &date)?
        );
        let task_dir = self.tasks_dir().join(&task_name);
        fs::create_dir(&task_dir)?;
        Ok(Task::new(task_dir))
    }

    pub fn switch_task(&self, task: &Task) -> Result<()> {
        let task_name = task.name()?;
        if !task.path.is_dir() {
            return Err(Box::new(ProtocolError::TaskDoesNotExist));
        }

        let current_link = self.current_link();
        if let Ok(metadata) = fs::symlink_metadata(&current_link) {
            if !metadata.file_type().is_symlink() {
                return Err(Box::new(ProtocolError::InvalidCurrentTask));
            }
            resolve_current_task(&self.tasks_dir(), &current_link)?;
            fs::remove_file(&current_link)?;
        }

        symlink_task(task_name, &current_link)
    }
}

pub struct Task {
    pub path: PathBuf,
    _private: (),
}

impl Task {
    pub fn new(path: PathBuf) -> Self {
        Self { path, _private: () }
    }

    pub fn goal_path(&self) -> PathBuf {
        self.path.join("goal.md")
    }

    pub fn name(&self) -> Result<&str> {
        self.path
            .file_name()
            .and_then(|name| name.to_str())
            .ok_or_else(|| Box::new(ProtocolError::InvalidTaskName) as Box<dyn Error>)
    }

    pub fn goal(&self) -> Result<Option<TaskArtifact>> {
        let goal_path = self.goal_path();
        if !goal_path.exists() {
            return Ok(None);
        }

        Ok(Some(TaskArtifact::read(goal_path)?))
    }
}

pub struct TaskArtifact {
    path: PathBuf,
    source: String,
}

impl TaskArtifact {
    pub fn read(path: PathBuf) -> Result<Self> {
        let content = fs::read_to_string(&path)?;
        Ok(Self {
            path,
            source: content,
        })
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    pub fn artifact(&self) -> Result<Artifact<'_>> {
        let config = ParserConfig::default();
        parser::parse_with_config(&self.source, &config).map_err(|diagnostics| {
            Box::new(InvalidTaskArtifact {
                path: self.path.clone(),
                diagnostics,
            }) as Box<dyn Error>
        })
    }
}

#[derive(Debug)]
struct InvalidTaskArtifact {
    path: PathBuf,
    diagnostics: Vec<Diagnostic>,
}

impl fmt::Display for InvalidTaskArtifact {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        for (index, diagnostic) in self.diagnostics.iter().enumerate() {
            if index > 0 {
                formatter.write_str("\n")?;
            }
            write!(
                formatter,
                "{}:{}: {}",
                self.path.display(),
                diagnostic.line(),
                diagnostic.message()
            )?;
        }
        Ok(())
    }
}

impl Error for InvalidTaskArtifact {}

#[derive(Debug)]
enum ProtocolError {
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

fn resolve_current_task(tasks_dir: &Path, current_link: &Path) -> Result<PathBuf> {
    let metadata = fs::symlink_metadata(current_link)?;
    if !metadata.file_type().is_symlink() {
        return Err(Box::new(ProtocolError::InvalidCurrentTask));
    }

    let target = fs::read_link(current_link)?;
    let task_dir = if target.is_absolute() {
        target
    } else {
        current_link
            .parent()
            .ok_or(ProtocolError::InvalidCurrentTask)?
            .join(target)
    };

    let tasks_dir = tasks_dir.canonicalize()?;
    let task_dir = task_dir.canonicalize()?;
    if task_dir.parent() != Some(tasks_dir.as_path()) {
        return Err(Box::new(ProtocolError::CurrentTaskEscapesTasksDir));
    }
    Ok(task_dir)
}

fn validate_slug(slug: &str) -> Result<()> {
    let valid = !slug.is_empty()
        && !slug.starts_with('-')
        && !slug.ends_with('-')
        && slug
            .bytes()
            .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'-')
        && !slug.contains("--");
    if !valid {
        return Err(Box::new(ProtocolError::InvalidTaskSlug));
    }
    Ok(())
}

fn next_task_sequence(tasks_dir: &Path, date: &str) -> Result<u32> {
    let prefix = format!("{date}-");
    let mut highest_sequence = 0;
    for entry in fs::read_dir(tasks_dir)? {
        let entry = entry?;
        let Some(name) = entry.file_name().to_str().map(str::to_owned) else {
            continue;
        };
        let Some(remainder) = name.strip_prefix(&prefix) else {
            continue;
        };
        let Some((sequence, _)) = remainder.split_once('-') else {
            continue;
        };
        let Ok(sequence) = sequence.parse::<u32>() else {
            continue;
        };
        highest_sequence = highest_sequence.max(sequence);
    }

    Ok(highest_sequence + 1)
}

#[cfg(unix)]
fn symlink_task(task_name: &str, current_link: &Path) -> Result<()> {
    symlink(format!("./{task_name}"), current_link)?;
    Ok(())
}

#[cfg(not(unix))]
fn symlink_task(_task_name: &str, _current_link: &Path) -> Result<()> {
    Err(Box::new(ProtocolError::InvalidCurrentTask))
}

#[cfg(test)]
mod tests;
