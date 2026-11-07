use std::path::{Path, PathBuf};

use super::step::step_status;
use super::task::read_optional;
use super::{ArtifactClassification, ArtifactFile, Result, Task};
use crate::artifact::Artifact;
use crate::git_repo::{CurrentBranch, GitRepo};

pub(crate) struct GoalInspection {
    classification: ArtifactClassification,
    status: Option<String>,
    has_open_questions: bool,
}

impl GoalInspection {
    pub(crate) fn from_artifact(
        classification: ArtifactClassification,
        goal: &Artifact<'_>,
    ) -> Self {
        Self {
            classification,
            status: goal
                .get_metadata("Status")
                .map(|metadata| metadata.value().to_owned()),
            has_open_questions: goal.sections().iter().any(|section| {
                section.title() == "Open Questions"
                    && section
                        .as_itemised()
                        .is_some_and(|questions| !questions.items().is_empty())
            }),
        }
    }

    pub(crate) fn classification(&self) -> ArtifactClassification {
        self.classification
    }

    pub(crate) fn status(&self) -> Option<&str> {
        self.status.as_deref()
    }

    pub(crate) fn ready_to_accept(&self) -> Option<bool> {
        matches!(self.status(), Some("drafting" | "amending")).then(|| {
            self.classification == ArtifactClassification::Good && !self.has_open_questions
        })
    }

    pub(crate) fn has_open_questions(&self) -> bool {
        self.has_open_questions
    }
}

pub(crate) struct PlanInspection {
    classification: ArtifactClassification,
    status: Option<String>,
    has_open_questions: bool,
    goal: Option<GoalInspection>,
    branch: Option<String>,
    base_branch: Option<String>,
    base_commit: Option<String>,
    base_branch_commit: Option<String>,
    current_branch: Option<String>,
    head: Option<String>,
    clean: bool,
    branch_exists: bool,
    branch_commit: Option<String>,
    branch_in_other_worktree: bool,
    pending_items: Vec<String>,
}

impl PlanInspection {
    fn new(classification: ArtifactClassification) -> Self {
        Self {
            classification,
            status: None,
            has_open_questions: false,
            goal: None,
            branch: None,
            base_branch: None,
            base_commit: None,
            base_branch_commit: None,
            current_branch: None,
            head: None,
            clean: false,
            branch_exists: false,
            branch_commit: None,
            branch_in_other_worktree: false,
            pending_items: Vec::new(),
        }
    }

    pub(crate) fn from_artifact(
        classification: ArtifactClassification,
        plan: &Artifact<'_>,
        goal: Option<GoalInspection>,
    ) -> Self {
        Self {
            classification,
            status: plan
                .get_metadata("Status")
                .map(|metadata| metadata.value().to_owned()),
            has_open_questions: plan.sections().iter().any(|section| {
                section.title() == "Open Questions"
                    && section
                        .as_itemised()
                        .is_some_and(|questions| !questions.items().is_empty())
            }),
            goal,
            branch: plan
                .get_metadata("Branch")
                .map(|value| value.value().to_owned()),
            base_branch: plan
                .get_metadata("Base branch")
                .map(|value| value.value().to_owned()),
            base_commit: plan
                .get_metadata("Base commit")
                .map(|value| value.value().to_owned()),
            pending_items: pending_items(plan),
            ..Self::new(classification)
        }
    }

    pub(crate) fn classification(&self) -> ArtifactClassification {
        self.classification
    }

    pub(crate) fn status(&self) -> Option<&str> {
        self.status.as_deref()
    }

    pub(crate) fn has_open_questions(&self) -> bool {
        self.has_open_questions
    }

    pub(crate) fn goal(&self) -> Option<&GoalInspection> {
        self.goal.as_ref()
    }

    pub(super) fn artifacts_ready(&self) -> bool {
        self.classification == ArtifactClassification::Good
            && !self.has_open_questions
            && self.goal().is_some_and(|goal| {
                goal.classification() == ArtifactClassification::Good
                    && goal.status() == Some("accepted")
            })
    }

    pub(crate) fn branch(&self) -> Option<&str> {
        self.branch.as_deref()
    }

    pub(crate) fn base_branch(&self) -> Option<&str> {
        self.base_branch.as_deref()
    }

    pub(crate) fn base_commit(&self) -> Option<&str> {
        self.base_commit.as_deref()
    }

    pub(crate) fn base_branch_commit(&self) -> Option<&str> {
        self.base_branch_commit.as_deref()
    }

    pub(crate) fn head(&self) -> Option<&str> {
        self.head.as_deref()
    }

    pub(crate) fn current_branch(&self) -> Option<&str> {
        self.current_branch.as_deref()
    }

    pub(crate) fn clean(&self) -> bool {
        self.clean
    }

    pub(crate) fn branch_exists(&self) -> bool {
        self.branch_exists
    }

    pub(crate) fn branch_commit(&self) -> Option<&str> {
        self.branch_commit.as_deref()
    }

    pub(crate) fn branch_in_other_worktree(&self) -> bool {
        self.branch_in_other_worktree
    }

    /// IDs of plan items whose `Status` is not `done`, in plan order.
    pub(crate) fn pending_items(&self) -> &[String] {
        &self.pending_items
    }

    pub(crate) fn ready_to_accept(&self) -> bool {
        // Plan and goal prerequisites must pass.
        if !self.artifacts_ready() {
            return false;
        }
        let (Some(head), Some(base_commit)) = (self.head(), self.base_commit()) else {
            return false;
        };
        match self.status() {
            Some("drafting") => {
                let Some(base_tip) = self.base_branch_commit() else {
                    return false;
                };
                let head_matches = head.eq_ignore_ascii_case(base_commit); // HEAD matches.
                let base_matches = base_tip.eq_ignore_ascii_case(base_commit); // Base tip matches.
                let branch_absent = self.branch_commit().is_none();
                let branch_at_base = self
                    .branch_commit()
                    .is_some_and(|tip| tip.eq_ignore_ascii_case(base_commit));
                let workspace_clean = self.clean; // No staged or tracked-file changes.
                head_matches && base_matches && (branch_absent || branch_at_base) && workspace_clean
            }
            Some("amending") => {
                let branch_exists = self.branch_exists; // Recorded task branch exists.
                let on_task_branch = self.current_branch() == self.branch(); // On the task branch.
                branch_exists && on_task_branch
            }
            _ => false,
        }
    }

    fn inspect_git(&mut self, workspace: &Path) -> Result<()> {
        let branch = self.branch().expect("GOOD plan has Branch");
        let base_commit = self.base_commit().expect("GOOD plan has Base commit");
        let repo = GitRepo::new(workspace);
        repo.validate_branch_name(branch)?;
        repo.require_commit(base_commit)?;
        let head = repo.last_commit()?;
        let current_branch = match repo.current_branch()? {
            CurrentBranch::Branch(name) => Some(name),
            CurrentBranch::DetachedHead => None,
        };
        let clean = repo.tracked_files_clean()?;
        let base_branch = self.base_branch().expect("GOOD plan has Base branch");
        let states = repo.branch_states(&[branch, base_branch])?;
        let task_branch = states.iter().find(|state| state.name == branch);
        let base_branch_commit = states
            .iter()
            .find(|state| state.name == base_branch)
            .map(|state| state.commit.clone());
        let branch_in_other_worktree = current_branch.as_deref() != Some(branch)
            && task_branch.is_some_and(|state| state.checked_out);
        self.branch_exists = task_branch.is_some();
        self.branch_commit = task_branch.map(|state| state.commit.clone());
        self.branch_in_other_worktree = branch_in_other_worktree;
        self.base_branch_commit = base_branch_commit;
        self.current_branch = current_branch;
        self.head = Some(head);
        self.clean = clean;
        Ok(())
    }
}

fn pending_items(plan: &Artifact<'_>) -> Vec<String> {
    plan.sections()
        .iter()
        .filter(|section| section.title() == "Plan Items")
        .filter_map(|section| section.as_itemised())
        .flat_map(|section| section.items())
        .filter(|item| {
            item.as_expanded().is_some_and(|expanded| {
                !expanded.metadata().iter().any(|metadata| {
                    metadata.value().key() == "Status" && metadata.value().value() == "done"
                })
            })
        })
        .filter_map(|item| item.identifier().map(|id| id.text().to_owned()))
        .collect()
}

/// The current step's lifecycle facts for task inspection.
pub(crate) struct StepInspection {
    path: PathBuf,
    classification: ArtifactClassification,
    status: Option<String>,
    source_commit: Option<String>,
    blockers: Vec<String>,
    staged_changes: bool,
}

impl StepInspection {
    pub(crate) fn path(&self) -> &Path {
        &self.path
    }

    pub(crate) fn classification(&self) -> ArtifactClassification {
        self.classification
    }

    pub(crate) fn status(&self) -> Option<&str> {
        self.status.as_deref()
    }

    pub(crate) fn source_commit(&self) -> Option<&str> {
        self.source_commit.as_deref()
    }

    /// Blockers `waif step accept` would report, other than the status, for a
    /// `drafting` or `amending` step; empty otherwise.
    pub(crate) fn blockers(&self) -> &[String] {
        &self.blockers
    }

    /// Whether the index has staged changes; queried only for an `accepted`
    /// step.
    pub(crate) fn staged_changes(&self) -> bool {
        self.staged_changes
    }
}

impl Task {
    pub(crate) fn inspect_step(&self, workspace: &Path) -> Result<Option<StepInspection>> {
        let Some(path) = self.find_current_step_path()? else {
            return Ok(None);
        };
        let file = ArtifactFile::read(path.clone())?;
        let classification = file.check().classification();
        let mut inspection = StepInspection {
            path,
            classification,
            status: None,
            source_commit: None,
            blockers: Vec::new(),
            staged_changes: false,
        };
        if classification == ArtifactClassification::Bad {
            return Ok(Some(inspection));
        }
        let step = file.parse_step()?;
        let status = step_status(&step);
        inspection.status = Some(status.to_owned());
        inspection.source_commit = step
            .get_metadata("Source commit")
            .map(|metadata| metadata.value().to_owned());
        match status {
            "drafting" | "amending" => {
                if classification == ArtifactClassification::Incomplete {
                    inspection.blockers.push("step is INCOMPLETE".to_owned());
                }
                self.step_blockers(workspace, &step, &mut inspection.blockers)?;
            }
            "accepted" => {
                inspection.staged_changes = GitRepo::new(workspace).has_staged_changes()?;
            }
            _ => {}
        }
        Ok(Some(inspection))
    }

    pub(crate) fn inspect_plan(&self, workspace_dir: &Path) -> Result<Option<PlanInspection>> {
        let Some(file) = read_optional(self.path.join("plan.md"))? else {
            return Ok(None);
        };
        let classification = file.check().classification();
        if classification == ArtifactClassification::Bad {
            return Ok(Some(PlanInspection::new(classification)));
        }

        let plan = file.parse_plan()?;
        let mut inspection =
            PlanInspection::from_artifact(classification, &plan, self.inspect_goal()?);
        if inspection.artifacts_ready() {
            inspection.inspect_git(workspace_dir)?;
        }
        Ok(Some(inspection))
    }

    pub(crate) fn inspect_goal(&self) -> Result<Option<GoalInspection>> {
        let path = self.path.join("goal.md");
        let Some(file) = read_optional(path)? else {
            return Ok(None);
        };
        let check = file.check();
        if check.classification() == ArtifactClassification::Bad {
            return Ok(Some(GoalInspection {
                classification: ArtifactClassification::Bad,
                status: None,
                has_open_questions: false,
            }));
        }

        let goal = file.parse_goal()?;
        Ok(Some(GoalInspection::from_artifact(
            check.classification(),
            &goal,
        )))
    }
}

#[cfg(test)]
mod tests;
