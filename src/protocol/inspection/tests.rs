use std::fs;

use super::*;
use crate::test_support::TestDir;

#[test]
fn drafting_plan_is_ready_on_clean_base_and_inspection_is_read_only() {
    let (workspace, task) = git_plan_task("clean-base", "drafting");
    let head = GitRepo::new(&workspace.path).last_commit().unwrap();
    let head_ref = fs::read(workspace.path.join(".git/HEAD")).unwrap();
    let refs = GitRepo::new(&workspace.path).references().unwrap();
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
    assert_eq!(GitRepo::new(&workspace.path).references().unwrap(), refs);
    assert_eq!(GitRepo::new(&workspace.path).last_commit().unwrap(), head);
    assert_eq!(fs::read(workspace.path.join(".git/HEAD")).unwrap(), head_ref);
    assert_eq!(fs::read(workspace.path.join(".git/index")).unwrap(), index);
    assert_eq!(fs::read(task.path.join("plan.md")).unwrap(), plan);
    assert_eq!(fs::read(task.path.join("goal.md")).unwrap(), goal);
}

#[test]
fn drafting_plan_accepts_same_base_commit_on_another_branch() {
    let (workspace, task) = git_plan_task("same-base-other-branch", "drafting");
    GitRepo::new(&workspace.path)
        .create_and_switch_branch("other", "HEAD")
        .unwrap();
    let inspection = task.inspect_plan(&workspace.path).unwrap().unwrap();

    assert_eq!(inspection.head(), inspection.base_commit());
    assert!(inspection.ready_to_accept());
}

#[test]
fn drafting_plan_accepts_detached_head_at_base_commit() {
    let (workspace, task) = git_plan_task("same-base-detached", "drafting");
    GitRepo::new(&workspace.path).detach_head("HEAD").unwrap();
    let inspection = task.inspect_plan(&workspace.path).unwrap().unwrap();

    assert_eq!(inspection.head(), inspection.base_commit());
    assert!(inspection.ready_to_accept());
}

#[test]
fn drafting_plan_blocks_moved_base_ref_even_when_head_matches_base_commit() {
    let (workspace, task) = git_plan_task("moved-base-ref", "drafting");
    let base = GitRepo::new(&workspace.path).last_commit().unwrap();
    commit_fixture(&workspace.path);
    let tip = GitRepo::new(&workspace.path).last_commit().unwrap();
    GitRepo::new(&workspace.path).detach_head(&base).unwrap();
    let inspection = task.inspect_plan(&workspace.path).unwrap().unwrap();

    assert_eq!(inspection.head(), inspection.base_commit());
    assert_eq!(inspection.base_branch_commit(), Some(tip.as_str()));
    assert!(!inspection.ready_to_accept());
}

#[test]
fn drafting_plan_blocks_missing_named_base_ref() {
    let (workspace, task) = git_plan_task("missing-base-ref", "drafting");
    GitRepo::new(&workspace.path).detach_head("HEAD").unwrap();
    GitRepo::new(&workspace.path).delete_branch("main").unwrap();
    let inspection = task.inspect_plan(&workspace.path).unwrap().unwrap();

    assert_eq!(inspection.head(), inspection.base_commit());
    assert_eq!(inspection.base_branch_commit(), None);
    assert!(!inspection.ready_to_accept());
}

#[test]
fn drafting_plan_accepts_uppercase_base_commit_without_rewriting_it() {
    let (workspace, task) = git_plan_task("uppercase-base", "drafting");
    let head = GitRepo::new(&workspace.path).last_commit().unwrap();
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
    GitRepo::new(&workspace.path)
        .stage_file("source.txt")
        .unwrap();
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
    GitRepo::new(&workspace.path)
        .create_and_switch_branch("waif/task", "HEAD")
        .unwrap();
    let inspection = task.inspect_plan(&workspace.path).unwrap().unwrap();

    assert_eq!(inspection.current_branch(), Some("waif/task"));
    assert!(inspection.head().is_some());
    assert!(!inspection.ready_to_accept());
}

#[test]
fn amending_plan_allows_new_commits_with_clean_tracked_state() {
    let (workspace, task) = git_plan_task("amending-clean", "amending");
    GitRepo::new(&workspace.path)
        .create_and_switch_branch("waif/task", "HEAD")
        .unwrap();
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
    GitRepo::new(&workspace.path)
        .create_and_switch_branch("waif/task", "HEAD")
        .unwrap();
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
    let head = GitRepo::new(&workspace.path).last_commit().unwrap();
    let blob = GitRepo::new(&workspace.path)
        .file_object_at_head("source.txt")
        .unwrap();
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
    GitRepo::new(&workspace.path)
        .create_branch("waif/task")
        .unwrap();
    let inspection = task.inspect_plan(&workspace.path).unwrap().unwrap();

    assert_eq!(inspection.head(), inspection.base_commit());
    assert_eq!(inspection.current_branch(), Some("main"));
    assert!(!inspection.ready_to_accept());
}

#[test]
fn accepted_plan_rejects_detached_head() {
    let (workspace, task) = git_plan_task("accepted-detached", "accepted");
    GitRepo::new(&workspace.path)
        .create_branch("waif/task")
        .unwrap();
    GitRepo::new(&workspace.path).detach_head("HEAD").unwrap();
    let inspection = task.inspect_plan(&workspace.path).unwrap().unwrap();

    assert_eq!(inspection.current_branch(), None);
    assert!(!inspection.ready_to_accept());
}

#[test]
fn amending_plan_requires_task_branch() {
    let (workspace, task) = git_plan_task("amending-other-branch", "amending");
    GitRepo::new(&workspace.path)
        .create_branch("waif/task")
        .unwrap();
    let inspection = task.inspect_plan(&workspace.path).unwrap().unwrap();

    assert_eq!(inspection.current_branch(), Some("main"));
    assert!(!inspection.ready_to_accept());
}

#[test]
fn accepted_plan_blocks_branch_occupied_by_other_worktree() {
    let (workspace, task) = git_plan_task("occupied-branch", "accepted");
    let linked = workspace.path.join(".waif/linked");
    GitRepo::new(&workspace.path)
        .add_worktree("waif/task", linked.to_str().unwrap())
        .unwrap();
    let inspection = task.inspect_plan(&workspace.path).unwrap().unwrap();

    assert!(inspection.clean());
    assert!(inspection.branch_in_other_worktree());
    assert!(!inspection.ready_to_accept());
    assert_eq!(GitRepo::new(&workspace.path).branch_name().unwrap(), "main");
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
    GitRepo::new(&workspace.path)
        .set_head_branch("unborn")
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
    GitRepo::new(&workspace.path)
        .initialise_on_branch("main")
        .unwrap();
    fs::write(workspace.path.join(".git/info/exclude"), "/.waif/\n").unwrap();
    fs::write(workspace.path.join("source.txt"), "source\n").unwrap();
    GitRepo::new(&workspace.path)
        .stage_file("source.txt")
        .unwrap();
    commit_fixture(&workspace.path);
    let head = GitRepo::new(&workspace.path).last_commit().unwrap();
    replace_plan(&task, "0123456789abcdef0123456789abcdef01234567", &head);
    (workspace, task)
}

fn commit_fixture(workspace: &Path) {
    GitRepo::new(workspace)
        .exec(&[
            "-c",
            "user.name=Test",
            "-c",
            "user.email=test@example.invalid",
            "-c",
            "commit.gpgsign=false",
            "commit",
            "--allow-empty",
            "-m",
            "fixture",
        ])
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
