#![cfg(unix)]

use test_support::git_repo::GitRepo;

use std::fs;
use std::os::unix::fs::symlink;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

mod test_support;
use test_support::TestDir;

const TIMESTAMP: &str = "2026-08-17T12:00:00+09:00";

#[test]
fn acceptance_reports_step_and_changes_only_lifecycle_metadata() {
    let workspace = fixture("accept");
    let path = step_path(&workspace);
    let before = fs::read_to_string(&path).unwrap();

    let output = run(&workspace.path, &["step", "accept"]);
    let after = fs::read_to_string(&path).unwrap();
    let stdout = String::from_utf8_lossy(&output.stdout);

    assert!(output.status.success(), "{}", String::from_utf8_lossy(&output.stderr));
    assert!(stdout.contains("Workflow dir:       .waif\n"));
    assert!(stdout.contains("Current task:       20261031-1-test\n"));
    assert!(stdout.contains("Step:               .waif/library/tasks/20261031-1-test/steps/01-test/step.md\n"));
    assert!(stdout.contains("Status:             accepted\n"));
    assert_eq!(after, expected_accepted(&before, &after));
}

#[test]
fn rejection_reports_context_and_leaves_step_unchanged() {
    let workspace = fixture("reject");
    let path = step_path(&workspace);
    let source = fs::read_to_string(&path)
        .unwrap()
        .replace("- Status: drafting", "- Status: accepted");
    fs::write(&path, &source).unwrap();

    let output = run(&workspace.path, &["step", "accept"]);
    let stderr = String::from_utf8_lossy(&output.stderr);

    assert!(!output.status.success());
    assert!(stderr.contains("Workflow dir: .waif\nCurrent task: 20261031-1-test\nStep: .waif/library/tasks/20261031-1-test/steps/01-test/step.md\n"), "{stderr}");
    assert!(stderr.contains("step is already accepted"), "{stderr}");
    assert_eq!(fs::read_to_string(&path).unwrap(), source);
}

#[test]
fn missing_current_task_reports_context() {
    let workspace = fixture("missing-task");
    fs::remove_file(workspace.path.join(".waif/current_task")).unwrap();
    let before = fs::read(step_path(&workspace)).unwrap();

    let output = run(&workspace.path, &["step", "accept"]);
    let stderr = String::from_utf8_lossy(&output.stderr);

    assert_eq!(output.status.code(), Some(1));
    assert!(stderr.contains("Workflow dir: .waif\nCurrent task: none\nno current workflow task"), "{stderr}");
    assert_eq!(fs::read(step_path(&workspace)).unwrap(), before);
}

#[test]
fn step_resolution_failure_reports_context() {
    let workspace = fixture("no-step");
    let path = step_path(&workspace);
    fs::remove_dir_all(path.parent().unwrap()).unwrap();
    let plan_path = path
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .join("plan.md");
    let plan = fs::read(&plan_path).unwrap();

    let output = run(&workspace.path, &["step", "accept"]);
    let stderr = String::from_utf8_lossy(&output.stderr);

    assert_eq!(output.status.code(), Some(1));
    assert!(stderr.contains("Workflow dir: .waif\nCurrent task: 20261031-1-test\n"), "{stderr}");
    assert!(stderr.contains("no step directory found"), "{stderr}");
    assert_eq!(fs::read(&plan_path).unwrap(), plan);
}

#[test]
fn malformed_current_task_reports_context() {
    let workspace = fixture("malformed-task");
    let pointer = workspace.path.join(".waif/current_task");
    fs::remove_file(&pointer).unwrap();
    fs::write(&pointer, "invalid pointer").unwrap();
    let before = fs::read(step_path(&workspace)).unwrap();

    let output = run(&workspace.path, &["step", "accept"]);
    let stderr = String::from_utf8_lossy(&output.stderr);

    assert_eq!(output.status.code(), Some(1));
    assert!(stderr.contains("Workflow dir: .waif\n"), "{stderr}");
    assert!(!stderr.trim_end().ends_with("Workflow dir: .waif"), "{stderr}");
    assert_eq!(fs::read(step_path(&workspace)).unwrap(), before);
    assert_eq!(fs::read_to_string(&pointer).unwrap(), "invalid pointer");
}

fn fixture(name: &str) -> TestDir {
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

    let path = step_path(&workspace);
    let task = path.parent().unwrap().parent().unwrap().parent().unwrap();
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    symlink(
        "library/tasks/20261031-1-test",
        workspace.path.join(".waif/current_task"),
    )
    .unwrap();
    fs::write(
        task.join("goal.md"),
        format!(
            "# Goal: Test\n\n- Status: accepted\n- Created: {TIMESTAMP}\n\
             - Updated: {TIMESTAMP}\n\n## Outcome\n\nTest\n\n\
             ## Acceptance Criteria\n\n- G-AC1: Test\n"
        ),
    )
    .unwrap();
    fs::write(
        task.join("plan.md"),
        format!(
            "# Plan: Test\n\n- Status: accepted\n- Goal: ./goal.md\n\
             - Branch: waif/task\n- Base branch: main\n- Base commit: {base}\n\
             - Created: {TIMESTAMP}\n- Updated: {TIMESTAMP}\n\n\
             ## Technical Summary\n\nApproach\n\n\
             ## Decisions\n\n- P-D1: Decision\n\n\
             ## Current System\n\nContext\n\n\
             ## Plan Items\n\n### P1: First\n\n- Status: pending\n\
             - Goal criteria: G-AC1\n\n\
             ## Risks\n\n- P-R1: Risk\n\n\
             ## Cross-Cutting Validation\n\n- P-V1: Check\n"
        ),
    )
    .unwrap();
    fs::write(
        path,
        format!(
            "# Step 1: Test\n\n- Status: drafting\n\
             - Source commit: not-created\n\
             - Created: {TIMESTAMP}\n- Updated: {TIMESTAMP}\n\
             - Plan items: P1\n\
             - Goal criteria: G-AC1\n\n## Objective\n\nDo it.\n\n\
             ## Plan Item Coverage\n\n- S1-PC1: P1 - complete - All.\n\n\
             ## Context\n\nContext\n\n## Done When\n\n- S1-D1: Done\n"
        ),
    )
    .unwrap();
    workspace
}

fn step_path(workspace: &TestDir) -> PathBuf {
    workspace
        .path
        .join(".waif/library/tasks/20261031-1-test/steps/01-test/step.md")
}

fn run(workspace: &Path, args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_waif"))
        .args(args)
        .current_dir(workspace)
        .output()
        .unwrap()
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
        .replace(
            &format!("- Updated: {TIMESTAMP}"),
            &format!("- Updated: {}", updated(after)),
        )
}
