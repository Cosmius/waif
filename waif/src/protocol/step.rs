use std::fs;
use std::path::{Path, PathBuf};

use chrono::{Local, SecondsFormat};

use super::inspection::PlanInspection;
use super::{replace_artifact, ArtifactClassification, ArtifactFile, Result, Task};
use crate::artifact::{serialize, Artifact};
use crate::schemas::step::step_directory_number;

impl Task {
    pub(crate) fn accept_step(&self, workspace: &Path) -> Result<()> {
        let path = self.current_step_path()?;
        let display = path.display();
        let file = ArtifactFile::read(path.clone())?;
        let classification = match file.check().classification() {
            ArtifactClassification::Bad => Some("BAD"),
            ArtifactClassification::Incomplete => Some("INCOMPLETE"),
            ArtifactClassification::Good => None,
        };
        if let Some(classification) = classification {
            return Err(format!("{display}: step is {classification}").into());
        }

        let mut step = file.parse_step()?;
        let status = step
            .get_metadata("Status")
            .map(|metadata| metadata.value().to_owned())
            .expect("a GOOD step has Status metadata");
        let mut blockers = match status.as_str() {
            "drafting" | "amending" => Vec::new(),
            "accepted" => vec!["step is already accepted".to_owned()],
            _ => vec![format!("step is already {status}")],
        };
        self.step_blockers(workspace, &step, &mut blockers)?;
        if !blockers.is_empty() {
            let message = blockers.join("\n");
            return Err(format!("{display}: {message}").into());
        }

        let updated = Local::now().to_rfc3339_opts(SecondsFormat::Secs, false);
        for metadata in step.metadata_mut() {
            match metadata.value().key() {
                "Status" => metadata.value_mut().set_value("accepted")?,
                "Updated" => metadata.value_mut().set_value(&updated)?,
                _ => {}
            }
        }
        replace_artifact(&path, &serialize(&step))
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
