use std::fs;
use std::io::ErrorKind;
#[cfg(unix)]
use std::os::unix::fs::symlink;
use std::path::{Component, Path, PathBuf};
use std::process::{Command, Stdio};

use chrono::Local;

use super::{ProtocolError, Result, Task};

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

    pub fn library_dir(&self) -> PathBuf {
        self.path.join("library")
    }

    pub fn tasks_dir(&self) -> PathBuf {
        self.library_dir().join("tasks")
    }

    pub fn current_link(&self) -> PathBuf {
        self.path.join("current_task")
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
        let current_link = self.current_link();
        let target = match fs::read_link(current_link) {
            Ok(target) => target,
            Err(error) if error.kind() == ErrorKind::NotFound => return Ok(None),
            Err(error) => return Err(error.into()),
        };
        if target
            .components()
            .any(|component| !matches!(component, Component::Normal(_)))
        {
            return Err(Box::new(ProtocolError::InvalidCurrentTask));
        }
        Ok(Some(Task::new(self.path.join(target))))
    }

    pub fn create_task(&self, slug: &str) -> Result<Task> {
        validate_slug(slug)?;

        let date = Local::now().format("%Y%m%d").to_string();
        let task_name = format!(
            "{date}-{}-{slug}",
            next_task_sequence(&self.tasks_dir(), &date)?
        );
        let task_dir = self.tasks_dir().join(&task_name);
        fs::create_dir(&task_dir)?;
        Ok(Task::new(task_dir))
    }

    pub fn task(&self, task_name: &str) -> Result<Task> {
        validate_task_name(task_name)?;

        let task_dir = self.tasks_dir().join(task_name);
        if !fs::metadata(&task_dir)?.is_dir() {
            return Err(Box::new(ProtocolError::TaskDoesNotExist));
        }
        Ok(Task::new(task_dir))
    }

    pub fn switch_task(&self, task: &Task) -> Result<()> {
        let task_name = task.name()?;
        let selected = self.task(task_name)?;
        if task.path != selected.path {
            return Err(Box::new(ProtocolError::TaskOutsideLibrary));
        }
        let current_link = self.current_link();
        match fs::remove_file(&current_link) {
            Ok(()) => (),
            Err(error) if error.kind() == ErrorKind::NotFound => (),
            Err(error) => return Err(error.into()),
        }
        symlink_task(task_name, &current_link)
    }
}

fn validate_slug(slug: &str) -> Result<()> {
    if !is_valid_slug(slug) {
        return Err(Box::new(ProtocolError::InvalidTaskSlug));
    }
    Ok(())
}

fn is_valid_slug(slug: &str) -> bool {
    !slug.is_empty()
        && !slug.starts_with('-')
        && !slug.ends_with('-')
        && slug
            .bytes()
            .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'-')
        && !slug.contains("--")
}

fn parse_task_name(task_name: &str) -> Result<(&str, u32, &str)> {
    if task_name.contains('/') {
        return Err(Box::new(ProtocolError::MalformedTaskName));
    }
    let Some((date, remainder)) = task_name.split_once('-') else {
        return Err(Box::new(ProtocolError::MalformedTaskName));
    };
    let Some((sequence, slug)) = remainder.split_once('-') else {
        return Err(Box::new(ProtocolError::MalformedTaskName));
    };
    let valid_date = date.len() >= 8;
    let Ok(sequence_number) = sequence.parse::<u32>() else {
        return Err(Box::new(ProtocolError::MalformedTaskName));
    };
    if !valid_date || sequence_number == 0 || !is_valid_slug(slug) {
        return Err(Box::new(ProtocolError::MalformedTaskName));
    }
    Ok((date, sequence_number, slug))
}

fn validate_task_name(task_name: &str) -> Result<()> {
    parse_task_name(task_name).map(|_| ())
}

fn next_task_sequence(tasks_dir: &Path, date: &str) -> Result<u32> {
    let mut highest_sequence = 0;
    for entry in fs::read_dir(tasks_dir)? {
        let entry = entry?;
        let Some(name) = entry.file_name().to_str().map(str::to_owned) else {
            continue;
        };
        let Ok((task_date, sequence, _)) = parse_task_name(&name) else {
            continue;
        };
        if task_date != date {
            continue;
        }
        highest_sequence = highest_sequence.max(sequence);
    }

    Ok(highest_sequence + 1)
}

#[cfg(unix)]
fn symlink_task(task_name: &str, current_link: &Path) -> Result<()> {
    symlink(format!("library/tasks/{task_name}"), current_link)?;
    Ok(())
}

#[cfg(not(unix))]
fn symlink_task(_task_name: &str, _current_link: &Path) -> Result<()> {
    Err(Box::new(ProtocolError::InvalidCurrentTask))
}
