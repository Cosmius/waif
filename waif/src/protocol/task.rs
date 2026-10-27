use std::fs;
use std::io;
use std::path::{Component, Path, PathBuf};

use chrono::{Local, SecondsFormat};

use crate::artifact::serialize;
use crate::artifact::Artifact;

use super::artifact::is_numbered_review_name;
use super::inspection::GoalInspection;
use super::{ArtifactFile, ArtifactKind, ProtocolError, Result};

pub struct Task {
    pub path: PathBuf,
    _private: (),
}

impl Task {
    pub(super) fn new(path: PathBuf) -> Self {
        Self { path, _private: () }
    }

    pub fn name(&self) -> Result<&str> {
        self.path
            .file_name()
            .and_then(|name| name.to_str())
            .ok_or_else(|| Box::new(ProtocolError::InvalidTaskName) as Box<dyn std::error::Error>)
    }

    pub fn artifact_paths(&self) -> Result<Vec<PathBuf>> {
        discover_artifact_paths(&self.path)
    }

    pub fn with_goal<A>(&self, k: impl for<'a> FnOnce(&Artifact<'a>) -> A) -> Result<Option<A>> {
        let Some(file) = read_optional(self.path.join("goal.md"))? else {
            return Ok(None);
        };
        let artifact = file.parse_goal()?;
        Ok(Some(k(&artifact)))
    }

    pub fn with_plan<A>(&self, k: impl for<'a> FnOnce(&Artifact<'a>) -> A) -> Result<Option<A>> {
        let Some(file) = read_optional(self.path.join("plan.md"))? else {
            return Ok(None);
        };
        let artifact = file.parse_plan()?;
        Ok(Some(k(&artifact)))
    }

    pub(crate) fn accept_goal(&self) -> Result<()> {
        let path = self.path.join("goal.md");
        if !path.exists() {
            return Err(Box::new(ProtocolError::GoalMissing));
        }
        let file = ArtifactFile::read(path.clone())?;

        let check = file.check();
        if check.classification() == super::ArtifactClassification::Bad {
            return Err(Box::new(ProtocolError::GoalInvalid {
                path: PathBuf::from("goal.md"),
                diagnostics: check.diagnostics().to_vec(),
            }));
        }

        let mut blockers = Vec::new();
        if check.classification() == super::ArtifactClassification::Incomplete {
            blockers
                .push("goal is incomplete; fill all recognized macros before accepting".to_owned());
        }

        let mut goal = file.parse_goal()?;
        let inspection = GoalInspection::from_artifact(check.classification(), &goal);
        let status = inspection
            .status()
            .expect("a non-BAD goal has Status metadata");
        if status == "accepted" {
            blockers.push("goal cannot be reaccepted".to_string());
        }
        if inspection.has_open_questions() {
            blockers.push("goal has blocking open questions".to_owned());
        }
        if !blockers.is_empty() {
            return Err(Box::new(ProtocolError::GoalPreconditions(blockers)));
        }

        let updated = Local::now().to_rfc3339_opts(SecondsFormat::Secs, false);
        for metadata in goal.metadata_mut() {
            match metadata.value().key() {
                "Status" => metadata.value_mut().set_value("accepted")?,
                "Updated" => metadata.value_mut().set_value(&updated)?,
                _ => {}
            }
        }
        fs::write(path, serialize(&goal))?;
        Ok(())
    }

    pub fn with_step<A>(
        &self,
        file: &ArtifactFile,
        k: impl for<'a> FnOnce(&Artifact<'a>) -> A,
    ) -> Result<A> {
        self.ensure_file(file, ArtifactKind::Step)?;
        let artifact = file.parse_step()?;
        Ok(k(&artifact))
    }

    pub fn with_review<A>(
        &self,
        file: &ArtifactFile,
        k: impl for<'a> FnOnce(&Artifact<'a>) -> A,
    ) -> Result<A> {
        self.ensure_file(file, ArtifactKind::ImplementationReview)?;
        let artifact = file.parse_review()?;
        Ok(k(&artifact))
    }

    fn ensure_file(&self, file: &ArtifactFile, expected: ArtifactKind) -> Result<()> {
        if file.kind() != expected {
            return Err(Box::new(ProtocolError::ArtifactKindMismatch));
        }
        let relative = file
            .path()
            .strip_prefix(&self.path)
            .map_err(|_| ProtocolError::ArtifactOutsideTask)?;
        if relative.as_os_str().is_empty()
            || relative.components().any(|component| {
                matches!(
                    component,
                    Component::ParentDir | Component::RootDir | Component::Prefix(_)
                )
            })
        {
            return Err(Box::new(ProtocolError::ArtifactOutsideTask));
        }
        Ok(())
    }
}

pub fn discover_artifact_paths(path: &Path) -> Result<Vec<PathBuf>> {
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

pub(super) fn read_optional(path: PathBuf) -> Result<Option<ArtifactFile>> {
    if path.exists() {
        ArtifactFile::read(path).map(Some)
    } else {
        Ok(None)
    }
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

fn sorted_directory_entries(directory: &Path) -> io::Result<Vec<fs::DirEntry>> {
    let mut entries = fs::read_dir(directory)?.collect::<io::Result<Vec<_>>>()?;
    entries.sort_by_key(|entry| entry.file_name());
    Ok(entries)
}

fn is_task_artifact_name(name: &str) -> bool {
    matches!(name, "goal.md" | "plan.md" | "review.md")
}

fn is_step_artifact_name(name: &str) -> bool {
    name == "step.md" || is_numbered_review_name(name)
}

#[cfg(test)]
mod acceptance_tests;
