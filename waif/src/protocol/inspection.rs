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
}

impl PlanInspection {
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

    pub(crate) fn ready_to_accept(&self) -> bool {
        self.classification == ArtifactClassification::Good
            && !self.has_open_questions
            && self.goal().is_some_and(|goal| {
                goal.classification() == ArtifactClassification::Good
                    && goal.status() == Some("accepted")
            })
    }
}

impl Task {
    pub(crate) fn inspect_plan(&self) -> Result<Option<PlanInspection>> {
        let Some(file) = read_optional(self.path.join("plan.md"))? else {
            return Ok(None);
        };
        let classification = file.check().classification();
        if classification == ArtifactClassification::Bad {
            return Ok(Some(PlanInspection {
                classification,
                status: None,
                has_open_questions: false,
                goal: None,
            }));
        }

        let plan = file.parse_plan()?;
        Ok(Some(PlanInspection::from_artifact(
            classification,
            &plan,
            self.inspect_goal()?,
        )))
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
    fn plan_inspection_is_read_only_and_keeps_item_values_opaque() {
        let (_workspace, task) = plan_task("plan-read-only", "drafting");
        let plan_before = fs::read(task.path.join("plan.md")).unwrap();
        let goal_before = fs::read(task.path.join("goal.md")).unwrap();

        let first = task.inspect_plan().unwrap().unwrap();
        let second = task.inspect_plan().unwrap().unwrap();

        assert_eq!(first.classification(), ArtifactClassification::Good);
        assert_eq!(first.status(), Some("drafting"));
        assert!(!first.has_open_questions());
        assert!(first.ready_to_accept());
        assert!(second.ready_to_accept());
        assert_eq!(fs::read(task.path.join("plan.md")).unwrap(), plan_before);
        assert_eq!(fs::read(task.path.join("goal.md")).unwrap(), goal_before);
    }

    #[test]
    fn amending_plan_can_meet_artifact_prerequisites() {
        let (_workspace, task) = plan_task("amending-plan", "amending");
        let inspection = task.inspect_plan().unwrap().unwrap();

        assert_eq!(inspection.status(), Some("amending"));
        assert!(inspection.ready_to_accept());
    }

    #[test]
    fn accepted_plan_can_meet_artifact_prerequisites() {
        let (_workspace, task) = plan_task("accepted-plan", "accepted");
        let inspection = task.inspect_plan().unwrap().unwrap();

        assert_eq!(inspection.status(), Some("accepted"));
        assert!(inspection.ready_to_accept());
    }

    #[test]
    fn missing_plan_has_no_inspection() {
        let (_workspace, task) = goal_task("missing-plan");

        assert!(task.inspect_plan().unwrap().is_none());
    }

    #[test]
    fn bad_plan_has_no_trusted_status() {
        let (_workspace, task) = plan_task("bad-plan", "drafting");
        fs::write(task.path.join("plan.md"), "not a plan\n").unwrap();
        let inspection = task.inspect_plan().unwrap().unwrap();

        assert_eq!(inspection.classification(), ArtifactClassification::Bad);
        assert_eq!(inspection.status(), None);
        assert!(!inspection.ready_to_accept());
    }

    #[test]
    fn incomplete_plan_blocks_artifact_prerequisites() {
        let (_workspace, task) = plan_task("incomplete-plan", "__NOW__");
        let inspection = task.inspect_plan().unwrap().unwrap();

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
        let inspection = task.inspect_plan().unwrap().unwrap();

        assert_eq!(inspection.classification(), ArtifactClassification::Good);
        assert!(inspection.has_open_questions());
        assert!(!inspection.ready_to_accept());
    }

    #[test]
    fn empty_plan_open_questions_do_not_block() {
        let (_workspace, task) = plan_task("plan-empty-questions", "drafting");
        let source = valid_plan("drafting")
            .replace("## Current System", "## Open Questions\n## Current System");
        fs::write(task.path.join("plan.md"), source).unwrap();
        let inspection = task.inspect_plan().unwrap().unwrap();

        assert!(!inspection.has_open_questions());
        assert!(inspection.ready_to_accept());
    }

    #[test]
    fn missing_goal_blocks_plan_without_changing_its_classification() {
        let (_workspace, task) = plan_task("plan-missing-goal", "drafting");
        fs::remove_file(task.path.join("goal.md")).unwrap();
        let inspection = task.inspect_plan().unwrap().unwrap();

        assert_eq!(inspection.classification(), ArtifactClassification::Good);
        assert!(inspection.goal().is_none());
        assert!(!inspection.ready_to_accept());
    }

    #[test]
    fn bad_goal_blocks_plan_without_changing_its_classification() {
        let (_workspace, task) = plan_task("plan-bad-goal", "drafting");
        fs::write(task.path.join("goal.md"), "not a goal\n").unwrap();
        let inspection = task.inspect_plan().unwrap().unwrap();

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
        let inspection = task.inspect_plan().unwrap().unwrap();

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
        let inspection = task.inspect_plan().unwrap().unwrap();

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
        let inspection = task.inspect_plan().unwrap().unwrap();

        assert_eq!(inspection.classification(), ArtifactClassification::Good);
        assert_eq!(inspection.goal().unwrap().status(), Some("amending"));
        assert!(!inspection.ready_to_accept());
    }

    #[test]
    fn plan_inspection_propagates_read_errors() {
        let (_workspace, task) = goal_task("plan-read-error");
        fs::create_dir(task.path.join("plan.md")).unwrap();
        let native_error = fs::read_to_string(task.path.join("plan.md")).unwrap_err();
        let error = task.inspect_plan().err().unwrap();

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
        let path = workspace.path.join("task");
        fs::create_dir(&path).expect("task should be created");
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
