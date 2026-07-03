use std::error::Error;
use std::fmt;
use std::fs;
use std::path::{Path, PathBuf};

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

    pub fn current_task(&self) -> Result<Option<Task>> {
        let tasks_dir = self.tasks_dir();
        let current_link = self.current_link();
        if !current_link.exists() {
            return Ok(None);
        }

        let task_dir = resolve_current_task(&tasks_dir, &current_link)?;
        Ok(Some(Task::new(task_dir)))
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
    metadata: Vec<(String, String)>,
}

impl TaskArtifact {
    pub fn read(path: PathBuf) -> Result<Self> {
        let content = fs::read_to_string(&path)?;
        Ok(Self {
            path,
            metadata: parse_metadata(&content),
        })
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    pub fn metadata_value(&self, key: &str) -> Option<&str> {
        self.metadata
            .iter()
            .find(|(metadata_key, _)| metadata_key == key)
            .map(|(_, value)| value.as_str())
    }
}

#[derive(Debug)]
enum ProtocolError {
    CurrentTaskEscapesTasksDir,
    InvalidCurrentTask,
    InvalidTaskName,
}

impl fmt::Display for ProtocolError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::CurrentTaskEscapesTasksDir => {
                write!(formatter, "current task must resolve inside .waif/tasks")
            }
            Self::InvalidCurrentTask => write!(formatter, "current task is invalid"),
            Self::InvalidTaskName => write!(formatter, "current task name is invalid"),
        }
    }
}

impl Error for ProtocolError {}

fn resolve_current_task(tasks_dir: &Path, current_link: &Path) -> Result<PathBuf> {
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
    if !task_dir.starts_with(tasks_dir) {
        return Err(Box::new(ProtocolError::CurrentTaskEscapesTasksDir));
    }
    Ok(task_dir)
}

fn parse_metadata(content: &str) -> Vec<(String, String)> {
    content
        .lines()
        .take_while(|line| !line.trim_start().starts_with("##"))
        .filter_map(parse_metadata_line)
        .collect()
}

fn parse_metadata_line(line: &str) -> Option<(String, String)> {
    let item = line.trim_start().strip_prefix("- ")?;
    let (key, value) = item.split_once(':')?;
    let key = key.trim();
    if key.is_empty() {
        return None;
    }

    Some((key.to_owned(), value.trim().to_owned()))
}

#[cfg(test)]
mod tests;
