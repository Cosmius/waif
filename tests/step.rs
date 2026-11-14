#![cfg(unix)]

use test_support::git_repo::GitRepo;

use std::fs;
use std::io::Write;
use std::os::unix::fs::symlink;
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};

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
    assert!(stdout.contains("Step:               .waif/current_task/steps/01-test/step.md\n"));
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
    assert!(stderr.contains("Workflow dir: .waif\nCurrent task: 20261031-1-test\nStep: .waif/current_task/steps/01-test/step.md\n"), "{stderr}");
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

#[test]
fn commit_with_message_reports_commit_and_marks_plan_item_done() {
    let workspace = commit_fixture("commit-message");

    let output = run(&workspace.path, &["step", "commit", "-m", "Add a"]);
    let stdout = String::from_utf8_lossy(&output.stdout);
    let hash = GitRepo::new(&workspace.path).last_commit().unwrap();

    assert!(output.status.success(), "{}", String::from_utf8_lossy(&output.stderr));
    assert!(stdout.contains("Step:               .waif/current_task/steps/01-test/step.md\n"));
    assert!(stdout.contains(&format!("Commit:             {hash}\n")));
    assert!(stdout.contains("Status:             done\n"));
    assert!(stdout.contains("Plan items done:    P1\n"));
    assert_eq!(git(&workspace, &["log", "-1", "--format=%s"]), "Add a");
    assert!(fs::read_to_string(step_path(&workspace)).unwrap().contains(&format!("- Source commit: {hash}\n")));
}

#[test]
fn commit_reads_message_from_file() {
    let workspace = commit_fixture("commit-file");
    fs::write(workspace.path.join("message.txt"), "Add a from file\n").unwrap();

    let output = run(&workspace.path, &["step", "commit", "-F", "message.txt"]);

    assert!(output.status.success(), "{}", String::from_utf8_lossy(&output.stderr));
    assert_eq!(git(&workspace, &["log", "-1", "--format=%s"]), "Add a from file");
}

#[test]
fn commit_reads_message_from_standard_input() {
    let workspace = commit_fixture("commit-stdin");

    let mut child = Command::new(env!("CARGO_BIN_EXE_waif"))
        .args(["step", "commit", "-F", "-"])
        .current_dir(&workspace.path)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    child
        .stdin
        .take()
        .unwrap()
        .write_all(b"Add a from stdin\n")
        .unwrap();
    let output = child.wait_with_output().unwrap();

    assert!(output.status.success(), "{}", String::from_utf8_lossy(&output.stderr));
    assert_eq!(git(&workspace, &["log", "-1", "--format=%s"]), "Add a from stdin");
}

#[test]
fn commit_with_both_message_sources_is_a_usage_error() {
    let workspace = commit_fixture("commit-both");
    let head = GitRepo::new(&workspace.path).last_commit().unwrap();

    let output = run(
        &workspace.path,
        &["step", "commit", "-m", "Add a", "-F", "-"],
    );

    assert_eq!(output.status.code(), Some(2));
    assert_eq!(GitRepo::new(&workspace.path).last_commit().unwrap(), head);
}

#[test]
fn commit_without_message_source_is_a_usage_error() {
    let workspace = commit_fixture("commit-neither");
    let head = GitRepo::new(&workspace.path).last_commit().unwrap();

    let output = run(&workspace.path, &["step", "commit"]);

    assert_eq!(output.status.code(), Some(2));
    assert_eq!(GitRepo::new(&workspace.path).last_commit().unwrap(), head);
}

#[test]
fn commit_with_unreadable_message_file_is_rejected() {
    let workspace = commit_fixture("commit-missing-file");
    let head = GitRepo::new(&workspace.path).last_commit().unwrap();
    let before = fs::read(step_path(&workspace)).unwrap();

    let output = run(&workspace.path, &["step", "commit", "-F", "missing.txt"]);
    let stderr = String::from_utf8_lossy(&output.stderr);

    assert_eq!(output.status.code(), Some(1));
    assert!(stderr.contains("missing.txt"), "{stderr}");
    assert_eq!(GitRepo::new(&workspace.path).last_commit().unwrap(), head);
    assert_eq!(fs::read(step_path(&workspace)).unwrap(), before);
}

#[test]
fn no_commit_reports_none_and_marks_plan_item_done() {
    let workspace = commit_fixture("no-commit");
    GitRepo::new(&workspace.path)
        .exec(&["rm", "--cached", "-q", "a.txt"])
        .unwrap();
    fs::remove_file(workspace.path.join("a.txt")).unwrap();
    let head = GitRepo::new(&workspace.path).last_commit().unwrap();

    let output = run(&workspace.path, &["step", "commit", "--no-commit"]);
    let stdout = String::from_utf8_lossy(&output.stdout);

    assert!(output.status.success(), "{}", String::from_utf8_lossy(&output.stderr));
    assert!(stdout.contains("Commit:             none\n"), "{stdout}");
    assert!(stdout.contains("Status:             done\n"));
    assert!(stdout.contains("Plan items done:    P1\n"));
    assert_eq!(GitRepo::new(&workspace.path).last_commit().unwrap(), head);
    assert!(fs::read_to_string(step_path(&workspace)).unwrap().contains("- Source commit: none\n"));
}

#[test]
fn no_commit_with_message_is_a_usage_error() {
    let workspace = commit_fixture("no-commit-message");
    let before = fs::read(step_path(&workspace)).unwrap();

    let output = run(
        &workspace.path,
        &["step", "commit", "--no-commit", "-m", "Add a"],
    );

    assert_eq!(output.status.code(), Some(2));
    assert_eq!(fs::read(step_path(&workspace)).unwrap(), before);
}

/// An accepted step with complete coverage and one staged file.
fn commit_fixture(name: &str) -> TestDir {
    let workspace = fixture(name);
    let path = step_path(&workspace);
    let source = fs::read_to_string(&path)
        .unwrap()
        .replace("- Status: drafting", "- Status: accepted");
    fs::write(&path, source).unwrap();
    fs::write(workspace.path.join("a.txt"), "staged\n").unwrap();
    GitRepo::new(&workspace.path).stage_file("a.txt").unwrap();
    workspace
}

fn git(workspace: &TestDir, arguments: &[&str]) -> String {
    let output = Command::new("git")
        .args(arguments)
        .current_dir(&workspace.path)
        .output()
        .unwrap();
    String::from_utf8(output.stdout)
        .unwrap()
        .trim_end()
        .to_owned()
}

fn fixture(name: &str) -> TestDir {
    let workspace = TestDir::new(name);
    let repo = GitRepo::new(&workspace.path);
    repo.initialise_on_branch("main").unwrap();
    fs::write(workspace.path.join(".git/info/exclude"), "/.waif/\n").unwrap();
    repo.exec(&["config", "user.name", "Test"]).unwrap();
    repo.exec(&["config", "user.email", "test@example.invalid"])
        .unwrap();
    repo.exec(&["config", "commit.gpgsign", "false"]).unwrap();
    repo.exec(&["commit", "--allow-empty", "-m", "fixture"])
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
            "# Goal: Test\n\n- Task name: 20261031-1-test\n- Status: accepted\n- Created: {TIMESTAMP}\n\
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
             ## Plan Item Coverage\n\n### S1-PC1: All\n- Plan item: P1\n- Coverage: complete\n\n\
             ## Done Conditions\n\n- S1-DC1: Done\n\n## Context\n\nContext\n"
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
