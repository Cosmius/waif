use std::fs;
use std::path::{Path, PathBuf};

use chrono::{Local, SecondsFormat};

use super::inspection::PlanInspection;
use super::{replace_artifact, ArtifactClassification, ArtifactFile, Result, Task};
use crate::artifact::{serialize, Artifact};
use crate::git_repo::GitRepo;
use crate::schemas::step::step_directory_number;

/// The outcome of a successful step commit.
#[derive(Debug)]
pub(crate) struct StepCommit {
    pub(crate) hash: String,
    pub(crate) done_items: Vec<String>,
}

impl Task {
    pub(crate) fn accept_step(&self, workspace: &Path) -> Result<()> {
        let path = self.current_step_path()?;
        let file = read_good_step(&path)?;
        let mut step = file.parse_step()?;
        let mut blockers = match step_status(&step) {
            "drafting" | "amending" => Vec::new(),
            "accepted" => vec!["step is already accepted".to_owned()],
            status => vec![format!("step is already {status}")],
        };
        self.step_blockers(workspace, &step, &mut blockers)?;
        reject(&path, blockers)?;

        set_metadata(&mut step, "Status", "accepted")?;
        set_metadata(&mut step, "Updated", &now())?;
        replace_artifact(&path, &serialize(&step))
    }

    /// Commit the staged index for the accepted current step, then record the
    /// commit in the step and mark completely covered plan items done.
    pub(crate) fn commit_step(&self, workspace: &Path, message: &str) -> Result<StepCommit> {
        let path = self.current_step_path()?;
        let file = read_good_step(&path)?;
        let mut step = file.parse_step()?;
        let mut blockers = match step_status(&step) {
            "accepted" => Vec::new(),
            "done" => vec!["step is already done".to_owned()],
            _ => vec!["step is not accepted".to_owned()],
        };
        if message.trim().is_empty() {
            blockers.push("commit message is empty".to_owned());
        }
        self.step_blockers(workspace, &step, &mut blockers)?;
        let repo = GitRepo::new(workspace);
        if !repo.has_staged_changes()? {
            blockers.push("no staged changes to commit".to_owned());
        }
        if repo.has_unmerged_entries()? {
            blockers.push("the index has unmerged entries".to_owned());
        }
        reject(&path, blockers)?;

        let plan_path = self.path.join("plan.md");
        let plan_file = ArtifactFile::read(plan_path.clone())?;
        let mut plan = plan_file.parse_plan()?;
        let done_items = complete_items(&step);
        let updated = now();
        set_metadata(&mut step, "Status", "done")?;
        set_metadata(&mut step, "Updated", &updated)?;
        for id in &done_items {
            plan.plan_item_mut(id)
                .and_then(|item| item.metadatum_mut("Status"))
                .ok_or_else(|| format!("plan item {id} has no single Status"))?
                .set_value("done")?;
        }
        if !done_items.is_empty() {
            set_metadata(&mut plan, "Updated", &updated)?;
        }

        repo.commit(message)?;
        let hash = repo
            .last_commit()
            .map_err(|error| format!("commit created but its hash is unreadable: {error}"))?;
        let unwritten = |artifact: &Path, error: Box<dyn std::error::Error>| {
            let artifact = artifact.display();
            format!("commit {hash} created but {artifact} was not updated: {error}")
        };
        set_metadata(&mut step, "Source commit", &hash)
            .and_then(|()| replace_artifact(&path, &serialize(&step)))
            .map_err(|error| unwritten(&path, error))?;
        if !done_items.is_empty() {
            replace_artifact(&plan_path, &serialize(&plan))
                .map_err(|error| unwritten(&plan_path, error))?;
        }
        Ok(StepCommit { hash, done_items })
    }

    /// Resolve the `step.md` of the step directory with the greatest number.
    pub(crate) fn current_step_path(&self) -> Result<PathBuf> {
        let steps = self.path.join("steps");
        let display = steps.display();
        let entries = match fs::read_dir(&steps) {
            Ok(entries) => entries,
            Err(error) => return Err(format!("{display}: {error}").into()),
        };
        let mut greatest: Option<(i64, Vec<PathBuf>)> = None;
        for entry in entries {
            let entry = entry?;
            if !entry.path().is_dir() {
                continue;
            }
            let path = entry.path().join("step.md");
            let number = step_directory_number(&mut Vec::new(), &path);
            let Some(number) = number else {
                continue;
            };
            match &mut greatest {
                Some((max, paths)) if number == *max => paths.push(path),
                Some((max, _)) if number < *max => {}
                _ => greatest = Some((number, vec![path])),
            }
        }
        let Some((_, mut paths)) = greatest else {
            return Err(format!("{display}: no step directory found").into());
        };
        if paths.len() > 1 {
            paths.sort();
            let names: Vec<_> = paths
                .iter()
                .filter_map(|path| path.parent())
                .map(|dir| dir.display().to_string())
                .collect();
            return Err(format!(
                "several step directories share the greatest number: {}",
                names.join(", ")
            )
            .into());
        }
        let path = paths.remove(0);
        if !path.is_file() {
            let display = path.display();
            return Err(format!("{display}: step file is missing").into());
        }
        Ok(path)
    }

    /// Collect goal, plan, branch, and coverage blockers for the current
    /// step.
    fn step_blockers(
        &self,
        workspace: &Path,
        step: &Artifact<'_>,
        blockers: &mut Vec<String>,
    ) -> Result<()> {
        if has_open_questions(step) {
            blockers.push("step has blocking open questions".to_owned());
        }
        let Some(plan) = self.inspect_plan(workspace)? else {
            blockers.push("current task has no plan.md".to_owned());
            return Ok(());
        };
        if !plan.artifacts_ready() || plan.status() != Some("accepted") {
            blockers.push(plan_blocker(&plan).to_owned());
            return Ok(());
        }
        let branch = plan.branch().expect("GOOD plan has Branch");
        let current = plan.current_branch();
        if current != Some(branch) {
            let current = current.unwrap_or("(detached HEAD)");
            blockers.push(format!(
                "current branch {current} is not the recorded task \
                 branch {branch}"
            ));
        }

        let plan_file = ArtifactFile::read(self.path.join("plan.md"))?;
        blockers.extend(coverage_blockers(&plan_file.parse_plan()?, step));
        Ok(())
    }
}

/// Read the step at `path`, rejecting BAD and INCOMPLETE steps.
fn read_good_step(path: &Path) -> Result<ArtifactFile> {
    let file = ArtifactFile::read(path.to_path_buf())?;
    let classification = match file.check().classification() {
        ArtifactClassification::Bad => "BAD",
        ArtifactClassification::Incomplete => "INCOMPLETE",
        ArtifactClassification::Good => return Ok(file),
    };
    Err(format!("{}: step is {classification}", path.display()).into())
}

fn step_status<'a>(step: &'a Artifact<'_>) -> &'a str {
    step.get_metadata("Status")
        .map(|metadata| metadata.value())
        .expect("a GOOD step has Status metadata")
}

/// Fail with every blocker, prefixed by the step path, when any exist.
fn reject(path: &Path, blockers: Vec<String>) -> Result<()> {
    if blockers.is_empty() {
        return Ok(());
    }
    let message = blockers.join("\n");
    Err(format!("{}: {message}", path.display()).into())
}

fn set_metadata(artifact: &mut Artifact<'_>, key: &str, value: &str) -> Result<()> {
    for metadata in artifact.metadata_mut() {
        if metadata.value().key() == key {
            metadata.value_mut().set_value(value)?;
        }
    }
    Ok(())
}

fn now() -> String {
    Local::now().to_rfc3339_opts(SecondsFormat::Secs, false)
}

/// List the plan items whose coverage entry is `complete`.
fn complete_items(step: &Artifact<'_>) -> Vec<String> {
    section_items(step, "Plan Item Coverage")
        .iter()
        .filter_map(|item| {
            let mut fields = item.short_description().text().split(" - ");
            let id = fields.next()?.trim();
            (fields.next()?.trim() == "complete").then(|| id.to_owned())
        })
        .collect()
}

/// Explain why a goal or plan blocks step acceptance.
fn plan_blocker(plan: &PlanInspection) -> &'static str {
    match plan.classification() {
        ArtifactClassification::Bad => return "plan is BAD",
        ArtifactClassification::Incomplete => return "plan is INCOMPLETE",
        ArtifactClassification::Good => {}
    }
    if plan.has_open_questions() {
        return "plan has blocking open questions";
    }
    let Some(goal) = plan.goal() else {
        return "referenced goal is missing";
    };
    if goal.classification() != ArtifactClassification::Good {
        return "referenced goal is not GOOD";
    }
    if goal.status() != Some("accepted") {
        return "referenced goal is not accepted";
    }
    "plan is not accepted"
}

/// Check that each plan item in the step's coverage is pending in the plan.
/// The step schema already rejects duplicate coverage entries.
fn coverage_blockers(plan: &Artifact<'_>, step: &Artifact<'_>) -> Vec<String> {
    let mut blockers = Vec::new();
    for item in section_items(step, "Plan Item Coverage") {
        let text = item.short_description().text();
        let id = text.split_once(" - ").map_or(text, |(id, _)| id).trim();
        let status = plan.plan_item(id).map(|item| {
            item.metadata()
                .iter()
                .find(|metadata| metadata.value().key() == "Status")
                .map(|metadata| metadata.value().value().to_owned())
        });
        match status {
            None => blockers.push(format!("plan item {id} does not exist")),
            Some(Some(status)) if status == "pending" => {}
            Some(_) => blockers.push(format!("plan item {id} is not pending")),
        }
    }
    blockers
}

fn section_items<'s, 'a>(
    artifact: &'s Artifact<'a>,
    title: &str,
) -> &'s [crate::artifact::Item<'a>] {
    artifact
        .sections()
        .iter()
        .find(|section| section.title() == title)
        .and_then(|section| section.as_itemised())
        .map_or(&[], |section| section.items())
}

fn has_open_questions(artifact: &Artifact<'_>) -> bool {
    !section_items(artifact, "Open Questions").is_empty()
}

#[cfg(test)]
mod tests;
