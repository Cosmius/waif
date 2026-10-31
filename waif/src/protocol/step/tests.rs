use std::fs;
use std::path::PathBuf;

use super::*;
use crate::git_repo::GitRepo;
use crate::test_support::TestDir;

const TIMESTAMP: &str = "2026-08-17T12:00:00+09:00";

#[test]
fn drafting_step_is_accepted_changing_only_lifecycle_metadata() {
    let (workspace, task) = fixture("drafting", "drafting");
    let path = step_path(&task, "01-test");
    let before = fs::read_to_string(&path).unwrap();
    let plan = fs::read(task.path.join("plan.md")).unwrap();

    task.accept_step(&workspace.path).unwrap();
    let after = fs::read_to_string(&path).unwrap();

    assert_eq!(after, expected_accepted(&before, &after));
    assert_ne!(after, before);
    assert!(chrono::DateTime::parse_from_rfc3339(updated(&after)).is_ok());
    assert_eq!(fs::read(task.path.join("plan.md")).unwrap(), plan);
    assert_eq!(fs::read_dir(path.parent().unwrap()).unwrap().count(), 1);
}

#[test]
fn amending_step_is_accepted() {
    let (workspace, task) = fixture("amending", "amending");
    let path = step_path(&task, "01-test");
    let before = fs::read_to_string(&path).unwrap();

    task.accept_step(&workspace.path).unwrap();
    let after = fs::read_to_string(&path).unwrap();

    assert_eq!(after, expected_accepted(&before, &after));
}

#[test]
fn accepted_step_is_rejected_as_already_accepted() {
    let (workspace, task) = fixture("accepted", "accepted");
    let before = read_step(&task);
    let plan = fs::read(task.path.join("plan.md")).unwrap();

    let error = task.accept_step(&workspace.path).unwrap_err().to_string();

    assert!(error.contains("step is already accepted"), "{error}");
    assert_eq!(read_step(&task), before);
    assert_eq!(fs::read(task.path.join("plan.md")).unwrap(), plan);
}

#[test]
fn greatest_numbered_step_is_accepted_over_earlier_drafting_step() {
    let (workspace, task) = fixture("greatest", "drafting");
    write_step(
        &task,
        "02-later",
        &step_source("drafting")
            .replace("S1-", "S2-")
            .replace("Step 1", "Step 2"),
    );
    let earlier = read_step(&task);

    task.accept_step(&workspace.path).unwrap();

    assert_eq!(read_step(&task), earlier);
    assert!(fs::read_to_string(step_path(&task, "02-later")).unwrap().contains("- Status: accepted"));
}

#[test]
fn missing_step_directory_is_rejected() {
    let (workspace, task) = fixture("no-steps", "drafting");
    fs::remove_dir_all(task.path.join("steps/01-test")).unwrap();
    let plan = fs::read(task.path.join("plan.md")).unwrap();

    let error = task.accept_step(&workspace.path).unwrap_err().to_string();

    assert!(error.contains("no step directory found"), "{error}");
    assert_eq!(fs::read(task.path.join("plan.md")).unwrap(), plan);
}

#[test]
fn wrong_current_branch_is_rejected() {
    let (workspace, task) = fixture("wrong-branch", "drafting");
    GitRepo::new(&workspace.path).switch_branch("main").unwrap();
    let before = read_step(&task);
    let plan = fs::read(task.path.join("plan.md")).unwrap();

    let error = task.accept_step(&workspace.path).unwrap_err().to_string();

    assert!(error.contains("current branch main is not the recorded task branch waif/task"), "{error}");
    assert_eq!(read_step(&task), before);
    assert_eq!(fs::read(task.path.join("plan.md")).unwrap(), plan);
}

#[test]
fn step_with_open_questions_is_rejected() {
    let (workspace, task) = fixture("open-question", "drafting");
    let source = read_step(&task).replace(
        "## Done When",
        "## Open Questions\n\n- S1-Q1: Unsure\n\n## Done When",
    );
    write_step(&task, "01-test", &source);
    let plan = fs::read(task.path.join("plan.md")).unwrap();

    let error = task.accept_step(&workspace.path).unwrap_err().to_string();

    assert!(error.contains("step has blocking open questions"), "{error}");
    assert_eq!(read_step(&task), source);
    assert_eq!(fs::read(task.path.join("plan.md")).unwrap(), plan);
}

#[test]
fn unknown_plan_item_is_rejected() {
    let (workspace, task) = fixture("unknown-item", "drafting");
    let source = read_step(&task).replace("- S1-PC1: P1", "- S1-PC9: P9");
    write_step(&task, "01-test", &source);
    let plan = fs::read(task.path.join("plan.md")).unwrap();

    let error = task.accept_step(&workspace.path).unwrap_err().to_string();

    assert!(error.contains("plan item P9 does not exist"), "{error}");
    assert_eq!(read_step(&task), source);
    assert_eq!(fs::read(task.path.join("plan.md")).unwrap(), plan);
}

#[test]
fn done_plan_item_is_rejected() {
    let (workspace, task) = fixture("done-item", "drafting");
    let source = read_step(&task).replace("- S1-PC1: P1", "- S1-PC2: P2");
    write_step(&task, "01-test", &source);
    let plan = fs::read(task.path.join("plan.md")).unwrap();

    let error = task.accept_step(&workspace.path).unwrap_err().to_string();

    assert!(error.contains("plan item P2 is not pending"), "{error}");
    assert_eq!(read_step(&task), source);
    assert_eq!(fs::read(task.path.join("plan.md")).unwrap(), plan);
}

#[test]
fn duplicate_coverage_entry_is_rejected() {
    let (workspace, task) = fixture("duplicate-coverage", "drafting");
    let source = read_step(&task).replace(
        "- S1-PC1: P1 - partial - Part.\n",
        "- S1-PC1: P1 - partial - Part.\n- S1-PC1: P1 - partial - Again.\n",
    );
    write_step(&task, "01-test", &source);
    let plan = fs::read(task.path.join("plan.md")).unwrap();

    let error = task.accept_step(&workspace.path).unwrap_err().to_string();

    assert!(error.contains("step is BAD"), "{error}");
    assert_eq!(read_step(&task), source);
    assert_eq!(fs::read(task.path.join("plan.md")).unwrap(), plan);
}

#[test]
fn bad_step_is_rejected() {
    let (workspace, task) = fixture("bad-step", "drafting");
    let source = read_step(&task).replace("# Step 1: Test", "# Step 2: Test");
    write_step(&task, "01-test", &source);
    let plan = fs::read(task.path.join("plan.md")).unwrap();

    let error = task.accept_step(&workspace.path).unwrap_err().to_string();

    assert!(error.contains("step is BAD"), "{error}");
    assert_eq!(read_step(&task), source);
    assert_eq!(fs::read(task.path.join("plan.md")).unwrap(), plan);
}

#[test]
fn incomplete_step_is_rejected() {
    let (workspace, task) = fixture("incomplete-step", "drafting");
    let source = read_step(&task).replace(&format!("- Updated: {TIMESTAMP}"), "- Updated: __NOW__");
    write_step(&task, "01-test", &source);
    let plan = fs::read(task.path.join("plan.md")).unwrap();

    let error = task.accept_step(&workspace.path).unwrap_err().to_string();

    assert!(error.contains("step is INCOMPLETE"), "{error}");
    assert_eq!(read_step(&task), source);
    assert_eq!(fs::read(task.path.join("plan.md")).unwrap(), plan);
}

#[test]
fn drafting_goal_is_rejected() {
    let (workspace, task) = fixture("drafting-goal", "drafting");
    let goal_path = task.path.join("goal.md");
    let goal = fs::read_to_string(&goal_path)
        .unwrap()
        .replace("- Status: accepted", "- Status: drafting");
    fs::write(&goal_path, &goal).unwrap();
    let before = read_step(&task);
    let plan = fs::read(task.path.join("plan.md")).unwrap();

    let error = task.accept_step(&workspace.path).unwrap_err().to_string();

    assert!(error.contains("referenced goal is not accepted"), "{error}");
    assert_eq!(read_step(&task), before);
    assert_eq!(fs::read(task.path.join("plan.md")).unwrap(), plan);
    assert_eq!(fs::read_to_string(&goal_path).unwrap(), goal);
}

#[test]
fn drafting_plan_is_rejected() {
    let (workspace, task) = fixture("drafting-plan", "drafting");
    let plan_path = task.path.join("plan.md");
    let plan = fs::read_to_string(&plan_path)
        .unwrap()
        .replace("- Status: accepted", "- Status: drafting");
    fs::write(&plan_path, &plan).unwrap();
    let before = read_step(&task);

    let error = task.accept_step(&workspace.path).unwrap_err().to_string();

    assert!(error.contains("plan is not accepted"), "{error}");
    assert_eq!(read_step(&task), before);
    assert_eq!(fs::read_to_string(&plan_path).unwrap(), plan);
}

#[test]
fn bad_plan_is_rejected() {
    let (workspace, task) = fixture("bad-plan", "drafting");
    let plan_path = task.path.join("plan.md");
    let plan = fs::read_to_string(&plan_path)
        .unwrap()
        .replace("## Risks\n\n- P-R1: Risk\n\n", "");
    fs::write(&plan_path, &plan).unwrap();
    let before = read_step(&task);

    let error = task.accept_step(&workspace.path).unwrap_err().to_string();

    assert!(error.contains("plan is BAD"), "{error}");
    assert_eq!(read_step(&task), before);
    assert_eq!(fs::read_to_string(&plan_path).unwrap(), plan);
}

#[test]
fn tied_greatest_step_numbers_are_rejected() {
    let (workspace, task) = fixture("tied-steps", "drafting");
    write_step(&task, "01-other", &step_source("drafting"));
    let before = read_step(&task);
    let plan = fs::read(task.path.join("plan.md")).unwrap();

    let error = task.accept_step(&workspace.path).unwrap_err().to_string();

    assert!(error.contains("several step directories share the greatest number"), "{error}");
    assert_eq!(read_step(&task), before);
    assert_eq!(fs::read_to_string(step_path(&task, "01-other")).unwrap(), step_source("drafting"));
    assert_eq!(fs::read(task.path.join("plan.md")).unwrap(), plan);
}

#[test]
fn greatest_step_directory_without_step_file_is_rejected() {
    let (workspace, task) = fixture("missing-step-file", "drafting");
    fs::create_dir_all(task.path.join("steps/02-empty")).unwrap();
    let before = read_step(&task);
    let plan = fs::read(task.path.join("plan.md")).unwrap();

    let error = task.accept_step(&workspace.path).unwrap_err().to_string();

    assert!(error.contains("step file is missing"), "{error}");
    assert_eq!(read_step(&task), before);
    assert_eq!(fs::read(task.path.join("plan.md")).unwrap(), plan);
}

#[test]
fn done_step_is_rejected() {
    let (workspace, task) = fixture("done-step", "done");
    let source = read_step(&task).replace(
        "- Source commit: not-created",
        "- Source commit: 0123456789abcdef0123456789abcdef01234567",
    );
    write_step(&task, "01-test", &source);
    let plan = fs::read(task.path.join("plan.md")).unwrap();

    let error = task.accept_step(&workspace.path).unwrap_err().to_string();

    assert!(error.contains("step is already done"), "{error}");
    assert_eq!(read_step(&task), source);
    assert_eq!(fs::read(task.path.join("plan.md")).unwrap(), plan);
}

#[test]
fn missing_plan_is_rejected() {
    let (workspace, task) = fixture("missing-plan", "drafting");
    fs::remove_file(task.path.join("plan.md")).unwrap();
    let before = read_step(&task);

    let error = task.accept_step(&workspace.path).unwrap_err().to_string();

    assert!(error.contains("current task has no plan.md"), "{error}");
    assert_eq!(read_step(&task), before);
}

#[test]
fn incomplete_plan_is_rejected() {
    let (workspace, task) = fixture("incomplete-plan", "drafting");
    let plan_path = task.path.join("plan.md");
    let plan = fs::read_to_string(&plan_path).unwrap().replacen(
        &format!("- Updated: {TIMESTAMP}"),
        "- Updated: __NOW__",
        1,
    );
    fs::write(&plan_path, &plan).unwrap();
    let before = read_step(&task);

    let error = task.accept_step(&workspace.path).unwrap_err().to_string();

    assert!(error.contains("plan is INCOMPLETE"), "{error}");
    assert_eq!(read_step(&task), before);
    assert_eq!(fs::read_to_string(&plan_path).unwrap(), plan);
}

#[test]
fn plan_with_open_questions_is_rejected() {
    let (workspace, task) = fixture("plan-open-question", "drafting");
    let plan_path = task.path.join("plan.md");
    let plan = fs::read_to_string(&plan_path).unwrap().replace(
        "## Current System",
        "## Open Questions\n\n- P-Q1: Unsure\n\n## Current System",
    );
    fs::write(&plan_path, &plan).unwrap();
    let before = read_step(&task);

    let error = task.accept_step(&workspace.path).unwrap_err().to_string();

    assert!(error.contains("plan has blocking open questions"), "{error}");
    assert_eq!(read_step(&task), before);
    assert_eq!(fs::read_to_string(&plan_path).unwrap(), plan);
}

#[test]
fn missing_goal_is_rejected() {
    let (workspace, task) = fixture("missing-goal", "drafting");
    fs::remove_file(task.path.join("goal.md")).unwrap();
    let before = read_step(&task);
    let plan = fs::read(task.path.join("plan.md")).unwrap();

    let error = task.accept_step(&workspace.path).unwrap_err().to_string();

    assert!(error.contains("referenced goal is missing"), "{error}");
    assert_eq!(read_step(&task), before);
    assert_eq!(fs::read(task.path.join("plan.md")).unwrap(), plan);
}

#[test]
fn incomplete_goal_is_rejected() {
    let (workspace, task) = fixture("incomplete-goal", "drafting");
    let goal_path = task.path.join("goal.md");
    let goal = fs::read_to_string(&goal_path).unwrap().replacen(
        &format!("- Updated: {TIMESTAMP}"),
        "- Updated: __NOW__",
        1,
    );
    fs::write(&goal_path, &goal).unwrap();
    let before = read_step(&task);
    let plan = fs::read(task.path.join("plan.md")).unwrap();

    let error = task.accept_step(&workspace.path).unwrap_err().to_string();

    assert!(error.contains("referenced goal is not GOOD"), "{error}");
    assert_eq!(read_step(&task), before);
    assert_eq!(fs::read(task.path.join("plan.md")).unwrap(), plan);
    assert_eq!(fs::read_to_string(&goal_path).unwrap(), goal);
}

#[test]
fn detached_head_is_rejected() {
    let (workspace, task) = fixture("detached-head", "drafting");
    GitRepo::new(&workspace.path).detach_head("HEAD").unwrap();
    let before = read_step(&task);
    let plan = fs::read(task.path.join("plan.md")).unwrap();

    let error = task.accept_step(&workspace.path).unwrap_err().to_string();

    assert!(error.contains("current branch (detached HEAD) is not the recorded task branch waif/task"), "{error}");
    assert_eq!(read_step(&task), before);
    assert_eq!(fs::read(task.path.join("plan.md")).unwrap(), plan);
}

#[test]
fn missing_steps_directory_is_rejected() {
    let (workspace, task) = fixture("no-steps-dir", "drafting");
    fs::remove_dir_all(task.path.join("steps")).unwrap();
    let plan = fs::read(task.path.join("plan.md")).unwrap();

    let error = task.accept_step(&workspace.path).unwrap_err().to_string();

    assert!(error.contains(&task.path.join("steps").display().to_string()), "{error}");
    assert_eq!(fs::read(task.path.join("plan.md")).unwrap(), plan);
}

fn fixture(name: &str, status: &str) -> (TestDir, Task) {
    let workspace = TestDir::new(name);
    let repo = GitRepo::new(&workspace.path);
    repo.initialise_on_branch("main").unwrap();
    fs::write(workspace.path.join(".git/info/exclude"), "/.waif/\n").unwrap();
    repo.exec(&[
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
    let base = repo.last_commit().unwrap();
    repo.create_and_switch_branch("waif/task", "HEAD").unwrap();

    let path = workspace.path.join(".waif/library/tasks/20261031-1-test");
    fs::create_dir_all(&path).unwrap();
    let task = Task::new(path);
    fs::write(
        task.path.join("goal.md"),
        format!(
            "# Goal: Test\n\n- Status: accepted\n- Created: {TIMESTAMP}\n\
             - Updated: {TIMESTAMP}\n\n## Outcome\n\nTest\n\n\
             ## Acceptance Criteria\n\n- G-AC1: First\n- G-AC2: Second\n"
        ),
    )
    .unwrap();
    fs::write(
        task.path.join("plan.md"),
        format!(
            "# Plan: Test\n\n- Status: accepted\n- Goal: ./goal.md\n\
             - Branch: waif/task\n- Base branch: main\n- Base commit: {base}\n\
             - Created: {TIMESTAMP}\n- Updated: {TIMESTAMP}\n\n\
             ## Technical Summary\n\nApproach\n\n\
             ## Decisions\n\n- P-D1: Decision\n\n\
             ## Current System\n\nContext\n\n\
             ## Plan Items\n\n### P1: First\n\n- Status: pending\n\
             - Goal criteria: G-AC1\n\n\
             ### P2: Earlier work\n\n- Status: done\n\
             - Goal criteria: G-AC2\n\n\
             ### P3: Later work\n\n- Status: pending\n\
             - Goal criteria: G-AC2\n\n\
             ## Risks\n\n- P-R1: Risk\n\n\
             ## Cross-Cutting Validation\n\n- P-V1: Check\n"
        ),
    )
    .unwrap();
    write_step(&task, "01-test", &step_source(status));
    (workspace, task)
}

fn step_source(status: &str) -> String {
    format!(
        "# Step 1: Test\n\n- Status: {status}\n- Source commit: not-created\n\
         - Created: {TIMESTAMP}\n- Updated: {TIMESTAMP}\n- Plan items: P1\n\
         - Goal criteria: G-AC1\n\n## Objective\n\nDo  it.\n\n\
         ## Plan Item Coverage\n\n- S1-PC1: P1 - partial - Part.\n\n\
         ## Context\n\nContext\n\n## Done When\n\n- S1-D1: Done\n"
    )
}

fn write_step(task: &Task, directory: &str, source: &str) {
    let path = step_path(task, directory);
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, source).unwrap();
}

fn step_path(task: &Task, directory: &str) -> PathBuf {
    task.path.join("steps").join(directory).join("step.md")
}

fn read_step(task: &Task) -> String {
    fs::read_to_string(step_path(task, "01-test")).unwrap()
}

fn updated(source: &str) -> &str {
    source
        .lines()
        .find_map(|line| line.strip_prefix("- Updated: "))
        .unwrap()
}

fn expected_accepted(before: &str, after: &str) -> String {
    before
        .replace("- Status: drafting", "- Status: accepted")
        .replace("- Status: amending", "- Status: accepted")
        .replace(
            &format!("- Updated: {TIMESTAMP}"),
            &format!("- Updated: {}", updated(after)),
        )
}
