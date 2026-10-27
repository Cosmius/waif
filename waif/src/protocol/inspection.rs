use std::path::Path;

use super::git;
use super::task::read_optional;
use super::{ArtifactClassification, Result, Task};
use crate::artifact::Artifact;

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
        let branch_ref = format!("refs/heads/{branch}");
        git::query(workspace, &["check-ref-format", &branch_ref])?;
        git::query(workspace, &["check-ref-format", "--branch", branch])?;
        git::query(
            workspace,
            &["cat-file", "-e", &format!("{base_commit}^{{commit}}")],
        )?;
        let head = git::text(workspace, &["rev-parse", "--verify", "HEAD"])?;
        let current_branch = git::optional_query(workspace, &["symbolic-ref", "--quiet", "HEAD"])?
            .map(|bytes| -> Result<String> {
                let reference = String::from_utf8(bytes)?;
                Ok(reference
                    .trim_end_matches('\n')
                    .strip_prefix("refs/heads/")
                    .ok_or("HEAD does not reference a local branch")?
                    .to_owned())
            })
            .transpose()?;
        let clean = git::query(
            workspace,
            &["status", "--porcelain", "--untracked-files=no"],
        )?
        .is_empty();
        let base_ref = format!(
            "refs/heads/{}",
            self.base_branch().expect("GOOD plan has Base branch")
        );
        let refs = git::query(
            workspace,
            &[
                "for-each-ref",
                "--format=%(refname)%00%(objectname)%00%(worktreepath)",
                &branch_ref,
                &base_ref,
            ],
        )?;
        let mut occupancy = None;
        let mut branch_commit = None;
        let mut base_branch_commit = None;
        for line in refs.split(|byte| *byte == b'\n') {
            let mut fields = line.splitn(3, |byte| *byte == 0);
            let name = fields.next().unwrap_or_default();
            let Some(commit) = fields.next() else {
                continue;
            };
            let Some(worktree) = fields.next() else {
                continue;
            };
            if name == branch_ref.as_bytes() {
                occupancy = Some(worktree);
                branch_commit = Some(String::from_utf8(commit.to_vec())?);
            }
            if name == base_ref.as_bytes() {
                base_branch_commit = Some(String::from_utf8(commit.to_vec())?);
            }
        }
        let branch_in_other_worktree = current_branch.as_deref() != Some(branch)
            && occupancy.is_some_and(|path| !path.is_empty());
        self.branch_exists = occupancy.is_some();
        self.branch_commit = branch_commit;
        self.branch_in_other_worktree = branch_in_other_worktree;
        self.base_branch_commit = base_branch_commit;
        self.current_branch = current_branch;
        self.head = Some(head);
        self.clean = clean;
        Ok(())
    }
}

impl Task {
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
mod tests {
    use std::fs;

    use super::*;
    use crate::test_support::TestDir;

    #[test]
    fn drafting_plan_is_ready_on_clean_base_and_inspection_is_read_only() {
        let (workspace, task) = git_plan_task("clean-base", "drafting");
        let head = git::text(&workspace.path, &["rev-parse", "HEAD"]).unwrap();
        let head_ref = fs::read(workspace.path.join(".git/HEAD")).unwrap();
        let refs = git::query(&workspace.path, &["show-ref"]).unwrap();
        let index = fs::read(workspace.path.join(".git/index")).unwrap();
        let plan = fs::read(task.path.join("plan.md")).unwrap();
        let goal = fs::read(task.path.join("goal.md")).unwrap();

        let first = task.inspect_plan(&workspace.path).unwrap().unwrap();
        let second = task.inspect_plan(&workspace.path).unwrap().unwrap();

        assert!(first.ready_to_accept());
        assert!(second.ready_to_accept());
        assert_eq!(first.head(), Some(head.as_str()));
        assert_eq!(first.base_branch(), Some("main"));
        assert_eq!(first.base_commit(), Some(head.as_str()));
        assert_eq!(first.base_branch_commit(), Some(head.as_str()));
        assert_eq!(first.branch(), Some("waif/task"));
        assert!(first.clean());
        assert!(!first.branch_exists());
        assert!(!first.branch_in_other_worktree());
        assert_eq!(git::query(&workspace.path, &["show-ref"]).unwrap(), refs);
        assert_eq!(git::text(&workspace.path, &["rev-parse", "HEAD"]).unwrap(), head);
        assert_eq!(fs::read(workspace.path.join(".git/HEAD")).unwrap(), head_ref);
        assert_eq!(fs::read(workspace.path.join(".git/index")).unwrap(), index);
        assert_eq!(fs::read(task.path.join("plan.md")).unwrap(), plan);
        assert_eq!(fs::read(task.path.join("goal.md")).unwrap(), goal);
    }

    #[test]
    fn drafting_plan_accepts_same_base_commit_on_another_branch() {
        let (workspace, task) = git_plan_task("same-base-other-branch", "drafting");
        git::query(&workspace.path, &["switch", "-c", "other"]).unwrap();
        let inspection = task.inspect_plan(&workspace.path).unwrap().unwrap();

        assert_eq!(inspection.head(), inspection.base_commit());
        assert!(inspection.ready_to_accept());
    }

    #[test]
    fn drafting_plan_accepts_detached_head_at_base_commit() {
        let (workspace, task) = git_plan_task("same-base-detached", "drafting");
        git::query(&workspace.path, &["switch", "--detach"]).unwrap();
        let inspection = task.inspect_plan(&workspace.path).unwrap().unwrap();

        assert_eq!(inspection.head(), inspection.base_commit());
        assert!(inspection.ready_to_accept());
    }

    #[test]
    fn drafting_plan_blocks_moved_base_ref_even_when_head_matches_base_commit() {
        let (workspace, task) = git_plan_task("moved-base-ref", "drafting");
        let base = git::text(&workspace.path, &["rev-parse", "HEAD"]).unwrap();
        commit_fixture(&workspace.path);
        let tip = git::text(&workspace.path, &["rev-parse", "HEAD"]).unwrap();
        git::query(&workspace.path, &["switch", "--detach", &base]).unwrap();
        let inspection = task.inspect_plan(&workspace.path).unwrap().unwrap();

        assert_eq!(inspection.head(), inspection.base_commit());
        assert_eq!(inspection.base_branch_commit(), Some(tip.as_str()));
        assert!(!inspection.ready_to_accept());
    }

    #[test]
    fn drafting_plan_blocks_missing_named_base_ref() {
        let (workspace, task) = git_plan_task("missing-base-ref", "drafting");
        git::query(&workspace.path, &["switch", "--detach"]).unwrap();
        git::query(&workspace.path, &["branch", "-d", "main"]).unwrap();
        let inspection = task.inspect_plan(&workspace.path).unwrap().unwrap();

        assert_eq!(inspection.head(), inspection.base_commit());
        assert_eq!(inspection.base_branch_commit(), None);
        assert!(!inspection.ready_to_accept());
    }

    #[test]
    fn drafting_plan_accepts_uppercase_base_commit_without_rewriting_it() {
        let (workspace, task) = git_plan_task("uppercase-base", "drafting");
        let head = git::text(&workspace.path, &["rev-parse", "HEAD"]).unwrap();
        let uppercase = head.to_ascii_uppercase();
        replace_plan(&task, &head, &uppercase);
        let plan = fs::read(task.path.join("plan.md")).unwrap();
        let inspection = task.inspect_plan(&workspace.path).unwrap().unwrap();

        assert_eq!(inspection.base_commit(), Some(uppercase.as_str()));
        assert!(inspection.ready_to_accept());
        assert_eq!(fs::read(task.path.join("plan.md")).unwrap(), plan);
    }

    #[test]
    fn drafting_plan_blocks_head_drift() {
        let (workspace, task) = git_plan_task("head-drift", "drafting");
        commit_fixture(&workspace.path);
        let inspection = task.inspect_plan(&workspace.path).unwrap().unwrap();

        assert_ne!(inspection.head(), inspection.base_commit());
        assert!(!inspection.ready_to_accept());
    }

    #[test]
    fn drafting_plan_blocks_staged_changes() {
        let (workspace, task) = git_plan_task("staged", "drafting");
        fs::write(workspace.path.join("source.txt"), "changed\n").unwrap();
        git::query(&workspace.path, &["add", "source.txt"]).unwrap();
        let inspection = task.inspect_plan(&workspace.path).unwrap().unwrap();

        assert!(!inspection.clean());
        assert!(!inspection.ready_to_accept());
    }

    #[test]
    fn drafting_plan_blocks_unstaged_changes() {
        let (workspace, task) = git_plan_task("unstaged", "drafting");
        fs::write(workspace.path.join("source.txt"), "changed\n").unwrap();
        let inspection = task.inspect_plan(&workspace.path).unwrap().unwrap();

        assert!(!inspection.clean());
        assert!(!inspection.ready_to_accept());
    }

    #[test]
    fn drafting_plan_allows_untracked_files() {
        let (workspace, task) = git_plan_task("untracked-allowed", "drafting");
        fs::write(workspace.path.join("extra.txt"), "untracked\n").unwrap();
        let inspection = task.inspect_plan(&workspace.path).unwrap().unwrap();

        assert!(inspection.clean());
        assert!(inspection.ready_to_accept());
    }

    #[test]
    fn accepted_plan_is_not_ready_on_task_branch() {
        let (workspace, task) = git_plan_task("accepted-not-ready", "accepted");
        git::query(&workspace.path, &["switch", "-c", "waif/task"]).unwrap();
        let inspection = task.inspect_plan(&workspace.path).unwrap().unwrap();

        assert_eq!(inspection.current_branch(), Some("waif/task"));
        assert!(inspection.head().is_some());
        assert!(!inspection.ready_to_accept());
    }

    #[test]
    fn amending_plan_allows_new_commits_with_clean_tracked_state() {
        let (workspace, task) = git_plan_task("amending-clean", "amending");
        git::query(&workspace.path, &["switch", "-c", "waif/task"]).unwrap();
        commit_fixture(&workspace.path);
        let inspection = task.inspect_plan(&workspace.path).unwrap().unwrap();

        assert_ne!(inspection.head(), inspection.base_commit());
        assert_eq!(inspection.base_branch_commit(), inspection.base_commit());
        assert!(inspection.clean());
        assert!(inspection.ready_to_accept());
    }

    #[test]
    fn plan_inspection_rejects_invalid_task_ref() {
        let (workspace, task) = git_plan_task("invalid-task-ref", "drafting");
        replace_plan(&task, "waif/task", "waif/bad..name");
        let error = task.inspect_plan(&workspace.path).err().unwrap();

        assert!(error.to_string().contains("check-ref-format refs/heads/waif/bad..name"));
    }

    #[test]
    fn amending_plan_allows_modified_tracked_files_on_task_branch() {
        let (workspace, task) = git_plan_task("amending-modified-allowed", "amending");
        git::query(&workspace.path, &["switch", "-c", "waif/task"]).unwrap();
        fs::write(workspace.path.join("source.txt"), "changed\n").unwrap();
        let inspection = task.inspect_plan(&workspace.path).unwrap().unwrap();

        assert!(!inspection.clean());
        assert!(inspection.ready_to_accept());
    }

    #[test]
    fn drafting_plan_accepts_a_valid_supplied_branch_name() {
        let (workspace, task) = git_plan_task("supplied-branch", "drafting");
        replace_plan(&task, "waif/task", "feature/task");
        let inspection = task.inspect_plan(&workspace.path).unwrap().unwrap();

        assert_eq!(inspection.branch(), Some("feature/task"));
        assert!(inspection.ready_to_accept());
    }

    #[test]
    fn plan_inspection_rejects_reserved_task_branch_name() {
        let (workspace, task) = git_plan_task("reserved-task-name", "drafting");
        replace_plan(&task, "waif/task", "HEAD");
        let error = task.inspect_plan(&workspace.path).err().unwrap();

        assert!(error.to_string().contains("check-ref-format --branch HEAD"));
    }

    #[test]
    fn plan_inspection_rejects_task_branch_starting_with_dash() {
        let (workspace, task) = git_plan_task("dash-task-name", "drafting");
        replace_plan(&task, "waif/task", "-task");
        let error = task.inspect_plan(&workspace.path).err().unwrap();

        assert!(error.to_string().contains("check-ref-format --branch -task"));
    }

    #[test]
    fn plan_inspection_rejects_non_commit_base_object() {
        let (workspace, task) = git_plan_task("blob-base", "drafting");
        let head = git::text(&workspace.path, &["rev-parse", "HEAD"]).unwrap();
        let blob = git::text(&workspace.path, &["rev-parse", "HEAD:source.txt"]).unwrap();
        replace_plan(&task, &head, &blob);
        let error = task.inspect_plan(&workspace.path).err().unwrap();

        assert!(error.to_string().contains("cat-file -e"));
    }

    #[test]
    fn accepted_plan_blocks_missing_recorded_branch() {
        let (workspace, task) = git_plan_task("missing-recorded-branch", "accepted");
        let inspection = task.inspect_plan(&workspace.path).unwrap().unwrap();

        assert!(!inspection.branch_exists());
        assert!(!inspection.ready_to_accept());
    }

    #[test]
    fn accepted_plan_requires_task_branch_even_at_same_commit() {
        let (workspace, task) = git_plan_task("accepted-other-branch", "accepted");
        git::query(&workspace.path, &["branch", "waif/task"]).unwrap();
        let inspection = task.inspect_plan(&workspace.path).unwrap().unwrap();

        assert_eq!(inspection.head(), inspection.base_commit());
        assert_eq!(inspection.current_branch(), Some("main"));
        assert!(!inspection.ready_to_accept());
    }

    #[test]
    fn accepted_plan_rejects_detached_head() {
        let (workspace, task) = git_plan_task("accepted-detached", "accepted");
        git::query(&workspace.path, &["branch", "waif/task"]).unwrap();
        git::query(&workspace.path, &["switch", "--detach"]).unwrap();
        let inspection = task.inspect_plan(&workspace.path).unwrap().unwrap();

        assert_eq!(inspection.current_branch(), None);
        assert!(!inspection.ready_to_accept());
    }

    #[test]
    fn amending_plan_requires_task_branch() {
        let (workspace, task) = git_plan_task("amending-other-branch", "amending");
        git::query(&workspace.path, &["branch", "waif/task"]).unwrap();
        let inspection = task.inspect_plan(&workspace.path).unwrap().unwrap();

        assert_eq!(inspection.current_branch(), Some("main"));
        assert!(!inspection.ready_to_accept());
    }

    #[test]
    fn accepted_plan_blocks_branch_occupied_by_other_worktree() {
        let (workspace, task) = git_plan_task("occupied-branch", "accepted");
        let linked = workspace.path.join(".waif/linked");
        git::query(
            &workspace.path,
            &[
                "worktree",
                "add",
                "-b",
                "waif/task",
                linked.to_str().unwrap(),
            ],
        )
        .unwrap();
        let inspection = task.inspect_plan(&workspace.path).unwrap().unwrap();

        assert!(inspection.clean());
        assert!(inspection.branch_in_other_worktree());
        assert!(!inspection.ready_to_accept());
        assert_eq!(git::text(&workspace.path, &["symbolic-ref", "--short", "HEAD"]).unwrap(), "main");
    }

    #[test]
    fn ready_artifacts_outside_git_return_contextual_error() {
        let (workspace, task) = git_plan_task("outside-git", "drafting");
        let outside = TestDir::new("outside-git-query");
        let error = task.inspect_plan(&outside.path).err().unwrap();

        assert!(error.to_string().contains(outside.path.to_str().unwrap()));
        assert!(error.to_string().contains("not a git repository"));
        assert!(workspace.path.join(".git").exists());
    }

    #[test]
    fn unborn_head_returns_contextual_error() {
        let (workspace, task) = git_plan_task("unborn-head", "drafting");
        git::query(
            &workspace.path,
            &["symbolic-ref", "HEAD", "refs/heads/unborn"],
        )
        .unwrap();
        let error = task.inspect_plan(&workspace.path).err().unwrap();

        assert!(error.to_string().contains("rev-parse --verify HEAD"));
    }

    #[test]
    fn empty_plan_questions_allow_combined_readiness() {
        let (workspace, task) = git_plan_task("empty-questions-ready", "drafting");
        replace_plan(
            &task,
            "## Current System",
            "## Open Questions\n## Current System",
        );
        let inspection = task.inspect_plan(&workspace.path).unwrap().unwrap();

        assert!(!inspection.has_open_questions());
        assert!(inspection.ready_to_accept());
    }

    fn git_plan_task(name: &str, status: &str) -> (TestDir, Task) {
        let (workspace, task) = plan_task(name, status);
        git::query(&workspace.path, &["init", "-b", "main"]).unwrap();
        fs::write(workspace.path.join(".git/info/exclude"), "/.waif/\n").unwrap();
        fs::write(workspace.path.join("source.txt"), "source\n").unwrap();
        git::query(&workspace.path, &["add", "source.txt"]).unwrap();
        commit_fixture(&workspace.path);
        let head = git::text(&workspace.path, &["rev-parse", "HEAD"]).unwrap();
        replace_plan(&task, "0123456789abcdef0123456789abcdef01234567", &head);
        (workspace, task)
    }

    fn commit_fixture(workspace: &Path) {
        git::query(
            workspace,
            &[
                "-c",
                "user.name=Test",
                "-c",
                "user.email=test@example.invalid",
                "commit",
                "--allow-empty",
                "-m",
                "fixture",
            ],
        )
        .unwrap();
    }

    fn replace_plan(task: &Task, from: &str, to: &str) {
        let path = task.path.join("plan.md");
        let source = fs::read_to_string(&path).unwrap();
        fs::write(path, source.replace(from, to)).unwrap();
    }

    #[test]
    fn missing_plan_has_no_inspection() {
        let (_workspace, task) = goal_task("missing-plan");

        assert!(task.inspect_plan(&_workspace.path).unwrap().is_none());
    }

    #[test]
    fn bad_plan_has_no_trusted_status() {
        let (_workspace, task) = plan_task("bad-plan", "drafting");
        fs::write(task.path.join("plan.md"), "not a plan\n").unwrap();
        let inspection = task.inspect_plan(&_workspace.path).unwrap().unwrap();

        assert_eq!(inspection.classification(), ArtifactClassification::Bad);
        assert_eq!(inspection.status(), None);
        assert!(!inspection.ready_to_accept());
    }

    #[test]
    fn incomplete_plan_blocks_artifact_prerequisites() {
        let (_workspace, task) = plan_task("incomplete-plan", "__NOW__");
        let inspection = task.inspect_plan(&_workspace.path).unwrap().unwrap();

        assert_eq!(inspection.classification(), ArtifactClassification::Incomplete);
        assert!(!inspection.ready_to_accept());
    }

    #[test]
    fn plan_open_questions_block_artifact_prerequisites() {
        let (_workspace, task) = plan_task("plan-questions", "drafting");
        let source = valid_plan("drafting").replace(
            "## Current System",
            "## Open Questions\n- P-Q1: Unresolved\n## Current System",
        );
        fs::write(task.path.join("plan.md"), source).unwrap();
        let inspection = task.inspect_plan(&_workspace.path).unwrap().unwrap();

        assert_eq!(inspection.classification(), ArtifactClassification::Good);
        assert!(inspection.has_open_questions());
        assert!(!inspection.ready_to_accept());
    }

    #[test]
    fn missing_goal_blocks_plan_without_changing_its_classification() {
        let (_workspace, task) = plan_task("plan-missing-goal", "drafting");
        fs::remove_file(task.path.join("goal.md")).unwrap();
        let inspection = task.inspect_plan(&_workspace.path).unwrap().unwrap();

        assert_eq!(inspection.classification(), ArtifactClassification::Good);
        assert!(inspection.goal().is_none());
        assert!(!inspection.ready_to_accept());
    }

    #[test]
    fn bad_goal_blocks_plan_without_changing_its_classification() {
        let (_workspace, task) = plan_task("plan-bad-goal", "drafting");
        fs::write(task.path.join("goal.md"), "not a goal\n").unwrap();
        let inspection = task.inspect_plan(&_workspace.path).unwrap().unwrap();

        assert_eq!(inspection.classification(), ArtifactClassification::Good);
        assert_eq!(inspection.goal().unwrap().classification(), ArtifactClassification::Bad);
        assert!(!inspection.ready_to_accept());
    }

    #[test]
    fn incomplete_goal_blocks_plan_without_changing_its_classification() {
        let (_workspace, task) = plan_task("plan-incomplete-goal", "drafting");
        fs::write(
            task.path.join("goal.md"),
            valid_goal("accepted", "__NOW__", ""),
        )
        .unwrap();
        let inspection = task.inspect_plan(&_workspace.path).unwrap().unwrap();

        assert_eq!(inspection.classification(), ArtifactClassification::Good);
        assert_eq!(inspection.goal().unwrap().classification(), ArtifactClassification::Incomplete);
        assert!(!inspection.ready_to_accept());
    }

    #[test]
    fn drafting_goal_blocks_plan_without_changing_its_classification() {
        let (_workspace, task) = plan_task("plan-drafting-goal", "drafting");
        fs::write(
            task.path.join("goal.md"),
            valid_goal("drafting", TIMESTAMP, ""),
        )
        .unwrap();
        let inspection = task.inspect_plan(&_workspace.path).unwrap().unwrap();

        assert_eq!(inspection.classification(), ArtifactClassification::Good);
        assert_eq!(inspection.goal().unwrap().status(), Some("drafting"));
        assert!(!inspection.ready_to_accept());
    }

    #[test]
    fn amending_goal_blocks_plan_without_changing_its_classification() {
        let (_workspace, task) = plan_task("plan-amending-goal", "drafting");
        fs::write(
            task.path.join("goal.md"),
            valid_goal("amending", TIMESTAMP, ""),
        )
        .unwrap();
        let inspection = task.inspect_plan(&_workspace.path).unwrap().unwrap();

        assert_eq!(inspection.classification(), ArtifactClassification::Good);
        assert_eq!(inspection.goal().unwrap().status(), Some("amending"));
        assert!(!inspection.ready_to_accept());
    }

    #[test]
    fn plan_inspection_propagates_read_errors() {
        let (_workspace, task) = goal_task("plan-read-error");
        fs::create_dir(task.path.join("plan.md")).unwrap();
        let native_error = fs::read_to_string(task.path.join("plan.md")).unwrap_err();
        let error = task.inspect_plan(&_workspace.path).err().unwrap();

        assert_eq!(error.to_string(), native_error.to_string());
    }

    fn plan_task(name: &str, status: &str) -> (TestDir, Task) {
        let (workspace, task) = goal_task(name);
        fs::write(task.path.join("plan.md"), valid_plan(status)).unwrap();
        fs::write(
            task.path.join("goal.md"),
            valid_goal("accepted", TIMESTAMP, ""),
        )
        .unwrap();
        (workspace, task)
    }

    fn valid_plan(status: &str) -> String {
        format!(
            "# Plan: Example\n\
             - Status: {status}\n\
             - Goal: ./goal.md\n\
             - Branch: waif/task\n\
             - Base branch: main\n\
             - Base commit: 0123456789abcdef0123456789abcdef01234567\n\
             - Created: {TIMESTAMP}\n\
             - Updated: {TIMESTAMP}\n\
             ## Technical Summary\nApproach\n\
             ## Decisions\n- P-D1: Decision\n\
             ## Current System\nContext\n\
             ## Plan Items\n### P1: First\n- Status: pending\n\
             - Goal criteria: opaque coverage\n\
             - Depends on: opaque dependency\n\
             ## Risks\n- P-R1: Risk\n\
             ## Cross-Cutting Validation\n- P-V1: Check\n"
        )
    }

    const TIMESTAMP: &str = "2026-08-17T12:00:00+09:00";

    #[test]
    fn protocol_inspection_derives_goal_readiness() {
        for (name, status, questions, expected) in [
            ("draft-ready", "drafting", "", Some(true)),
            ("draft-blocked", "drafting", QUESTIONS, Some(false)),
            ("amending-ready", "amending", "", Some(true)),
            ("accepted", "accepted", "", None),
        ] {
            let (workspace, task) = goal_task(name);
            fs::write(
                task.path.join("goal.md"),
                valid_goal(status, "2026-08-17T12:00:00+09:00", questions),
            )
            .expect("goal should be written");

            let inspection = task
                .inspect_goal()
                .expect("inspection should succeed")
                .expect("goal should exist");

            assert_eq!(inspection.ready_to_accept(), expected);
            drop(workspace);
        }
    }

    #[test]
    fn protocol_inspection_classifies_incomplete_and_bad_goals() {
        let (incomplete_workspace, incomplete_task) = goal_task("incomplete");
        fs::write(
            incomplete_task.path.join("goal.md"),
            valid_goal("drafting", "__NOW__", ""),
        )
        .expect("goal should be written");
        let incomplete = incomplete_task
            .inspect_goal()
            .expect("inspection should succeed")
            .expect("goal should exist");

        let (bad_workspace, bad_task) = goal_task("bad");
        fs::write(bad_task.path.join("goal.md"), "not a goal\n").expect("goal should be written");
        let bad = bad_task
            .inspect_goal()
            .expect("inspection should succeed")
            .expect("goal should exist");

        assert_eq!(incomplete.classification(), ArtifactClassification::Incomplete);
        assert_eq!(incomplete.ready_to_accept(), Some(false));
        assert_eq!(bad.classification(), ArtifactClassification::Bad);
        assert_eq!(bad.ready_to_accept(), None);
        drop((incomplete_workspace, bad_workspace));
    }

    fn goal_task(name: &str) -> (TestDir, Task) {
        let workspace = TestDir::new(name);
        let path = workspace.path.join(".waif/library/tasks/example");
        fs::create_dir_all(&path).expect("task should be created");
        (workspace, Task::new(path))
    }

    fn valid_goal(status: &str, updated: &str, questions: &str) -> String {
        format!(
            "# Goal: Example\n\n\
             - Status: {status}\n\
             - Created: 2026-08-17T12:00:00+09:00\n\
             - Updated: {updated}\n\n\
             ## Outcome\n\nAn outcome\n\n\
             ## Acceptance Criteria\n\n- G-AC1: A criterion\n\
             {questions}"
        )
    }

    const QUESTIONS: &str = "\n## Open Questions\n\n- G-Q1: A question\n";
}
