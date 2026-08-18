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

impl Task {
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
