use std::fs;

use super::*;
use crate::protocol::ArtifactClassification;
use crate::test_support::TestDir;

#[test]
fn accepts_a_drafting_goal_losslessly() {
    let (source, accepted) = accept_goal_for_test("drafting");
    assert!(accepted.contains("- Status: accepted\n"));
    assert!(accepted.contains("- Created: 2026-08-17T12:00:00+09:00\n"));
    assert!(accepted.contains("## Outcome\n\nbody\n"));
    assert_ne!(accepted, source);
}

#[test]
fn accepts_an_amending_goal_losslessly() {
    let (source, accepted) = accept_goal_for_test("amending");
    assert!(accepted.contains("- Status: accepted\n"));
    assert!(accepted.contains("- Created: 2026-08-17T12:00:00+09:00\n"));
    assert!(accepted.contains("## Outcome\n\nbody\n"));
    assert_ne!(accepted, source);
}

#[test]
fn rejects_an_incomplete_goal_before_writing() {
    let (result, before, after, classification) =
        rejected_goal("incomplete", valid_goal("drafting", "__NOW__"));
    assert!(result.is_err());
    assert_eq!(classification, ArtifactClassification::Incomplete);
    assert_eq!(after, before);
}

#[test]
fn rejects_a_goal_with_open_questions_before_writing() {
    let (result, before, after, classification) = rejected_goal(
        "questions",
        format!(
            "{}\n## Open Questions\n\n- G-Q1: unresolved\n",
            valid_goal("drafting", "body")
        ),
    );
    assert!(result.is_err());
    assert_eq!(classification, ArtifactClassification::Good);
    assert_eq!(after, before);
}

fn accept_goal_for_test(status: &str) -> (String, String) {
    let workspace = TestDir::new(status);
    let tasks_dir = workspace.path.join(".waif/library/tasks");
    let task_path = tasks_dir.join("20260818-1-task");
    fs::create_dir_all(&task_path).expect("task should be created");
    let task = Task::new(task_path.clone());
    let source = valid_goal(status, "body");
    fs::write(task_path.join("goal.md"), &source).expect("goal should be written");

    task.accept_goal().expect("goal should be accepted");
    let accepted =
        fs::read_to_string(task_path.join("goal.md")).expect("accepted goal should load");
    (source, accepted)
}

fn rejected_goal(
    name: &str,
    source: String,
) -> (Result<()>, Vec<u8>, Vec<u8>, ArtifactClassification) {
    let workspace = TestDir::new(name);
    let tasks_dir = workspace.path.join(".waif/library/tasks");
    let task_path = tasks_dir.join("20260818-1-task");
    fs::create_dir_all(&task_path).expect("task should be created");
    let task = Task::new(task_path.clone());
    let goal_path = task_path.join("goal.md");
    fs::write(&goal_path, &source).expect("goal should be written");
    let before = fs::read(&goal_path).expect("goal should load");

    let result = task.accept_goal();
    let after = fs::read(task_path.join("goal.md")).unwrap();
    let classification = ArtifactFile::read(goal_path)
        .unwrap()
        .check()
        .classification();
    (result, before, after, classification)
}

fn valid_goal(status: &str, outcome: &str) -> String {
    format!(
        "# Goal: Example\n\n\
             - Status: {status}\n\
             - Created: 2026-08-17T12:00:00+09:00\n\
             - Updated: 2026-08-17T12:00:00+09:00\n\n\
             ## Outcome\n\n{outcome}\n\n\
             ## Acceptance Criteria\n\n- G-AC1: A criterion\n"
    )
}
