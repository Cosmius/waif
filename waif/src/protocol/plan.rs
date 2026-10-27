use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

use chrono::{Local, SecondsFormat};

use super::{git, ArtifactClassification, ArtifactFile, Result, Task};
use crate::artifact::serialize;

impl Task {
    pub(crate) fn accept_plan(&self, workspace_dir: &Path) -> Result<()> {
        self.accept_plan_with(workspace_dir, replace_plan)
    }

    fn accept_plan_with(
        &self,
        workspace: &Path,
        replace: impl FnOnce(&Path, &str) -> Result<()>,
    ) -> Result<()> {
        let path = self.path.join("plan.md");
        let inspection = self
            .inspect_plan(workspace)
            .map_err(|error| format!("{}: {error}", path.display()))?
            .ok_or_else(|| format!("{}: current task has no plan.md", path.display()))?;
        if !inspection.artifacts_ready() {
            let reason = match inspection.classification() {
                ArtifactClassification::Bad => "plan is BAD",
                ArtifactClassification::Incomplete => "plan is INCOMPLETE",
                ArtifactClassification::Good if inspection.has_open_questions() => {
                    "plan has blocking open questions"
                }
                ArtifactClassification::Good => match inspection.goal() {
                    None => "referenced goal is missing",
                    Some(goal) if goal.classification() != ArtifactClassification::Good => {
                        "referenced goal is not GOOD"
                    }
                    _ => "referenced goal is not accepted",
                },
            };
            return Err(format!("{}: {reason}", path.display()).into());
        }
        let status = inspection.status().expect("GOOD plan has Status");
        if status == "accepted" {
            return Err(format!("{}: plan is already accepted", path.display()).into());
        }
        let branch = inspection.branch().expect("GOOD plan has Branch");
        let base = inspection.base_commit().expect("GOOD plan has Base commit");
        if status == "drafting" {
            if !inspection.ready_to_accept() {
                let reason = if !inspection.clean() {
                    "staged or modified tracked files"
                } else if !inspection
                    .head()
                    .is_some_and(|tip| tip.eq_ignore_ascii_case(base))
                {
                    "HEAD differs from Base commit"
                } else if !inspection
                    .base_branch_commit()
                    .is_some_and(|tip| tip.eq_ignore_ascii_case(base))
                {
                    "Base branch is missing or differs from Base commit"
                } else {
                    "existing task branch differs from Base commit"
                };
                return Err(format!("{}: {reason}", path.display()).into());
            }
        } else if !inspection.branch_exists() {
            return Err(format!(
                "{}: recorded task branch {branch} is missing",
                path.display()
            )
            .into());
        }

        // Establish the branch before changing lifecycle metadata.
        if inspection.current_branch() != Some(branch) {
            if inspection.branch_exists() {
                git::query(workspace, &["switch", "--", branch])?;
            } else {
                git::query(workspace, &["switch", "-c", branch, base])?;
            }
        }
        let current = git::text(workspace, &["symbolic-ref", "--quiet", "HEAD"])?;
        if current != format!("refs/heads/{branch}") {
            return Err(
                format!("branch setup did not establish recorded task branch {branch}").into(),
            );
        }

        let update = || -> Result<()> {
            let file = ArtifactFile::read(path.clone())?;
            let mut plan = file.parse_plan()?;
            let updated = Local::now().to_rfc3339_opts(SecondsFormat::Secs, false);
            for metadata in plan.metadata_mut() {
                match metadata.value().key() {
                    "Status" => metadata.value_mut().set_value("accepted")?,
                    "Updated" => metadata.value_mut().set_value(&updated)?,
                    _ => {}
                }
            }
            replace(&path, &serialize(&plan))
        };
        update().map_err(|error| {
            format!(
                "branch setup succeeded for {branch}; metadata update failed for {}: {error}",
                path.display(),
            )
            .into()
        })
    }
}

fn replace_plan(path: &Path, contents: &str) -> Result<()> {
    let nonce = SystemTime::now().duration_since(UNIX_EPOCH)?.as_nanos();
    let temporary = path.with_file_name(format!(".plan.md.{}.{nonce}.tmp", std::process::id()));
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
