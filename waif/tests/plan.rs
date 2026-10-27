#![cfg(unix)]

use std::fs;
use std::os::unix::fs::symlink;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

mod test_support;
use test_support::TestDir;

const TIMESTAMP: &str = "2026-08-17T12:00:00+09:00";

#[test]
fn initial_acceptance_creates_branch_and_changes_only_lifecycle_metadata() {
    let workspace = fixture("create", "drafting");
    let path = plan_path(&workspace);
    let before = fs::read_to_string(&path).unwrap();
    let goal = fs::read(path.with_file_name("goal.md")).unwrap();
    let base = git(&workspace.path, &["rev-parse", "HEAD"]);

    let output = run(&workspace.path, &["plan", "accept"]);
    let after = fs::read_to_string(&path).unwrap();
    let stdout = String::from_utf8_lossy(&output.stdout);

    assert!(output.status.success(), "{}", stderr(&output));
    assert!(stdout.contains("Workflow dir:       .waif\n"));
    assert!(stdout.contains("Current task:       20261026-1-test\n"));
    assert!(stdout.contains("Plan:               .waif/library/tasks/20261026-1-test/plan.md\n"));
    assert!(stdout.contains("Status:             accepted\n"));
    assert_eq!(after, expected_accepted(&before, &after));
    assert_ne!(after, before);
    assert!(chrono::DateTime::parse_from_rfc3339(metadata(&after, "Updated")).is_ok());
    assert_eq!(git(&workspace.path, &["symbolic-ref", "--short", "HEAD"]), "waif/task");
    assert_eq!(git(&workspace.path, &["rev-parse", "HEAD"]), base);
    assert_eq!(fs::read(path.with_file_name("goal.md")).unwrap(), goal);
}

#[test]
fn initial_acceptance_reuses_a_matching_existing_branch() {
    let workspace = fixture("reuse", "drafting");
    git(&workspace.path, &["branch", "waif/task"]);
    let base = git(&workspace.path, &["rev-parse", "HEAD"]);
    let before = fs::read_to_string(plan_path(&workspace)).unwrap();

    let output = run(&workspace.path, &["plan", "accept"]);
    let after = fs::read_to_string(plan_path(&workspace)).unwrap();

    assert!(output.status.success(), "{}", stderr(&output));
    assert_eq!(git(&workspace.path, &["symbolic-ref", "--short", "HEAD"]), "waif/task");
    assert_eq!(git(&workspace.path, &["rev-parse", "HEAD"]), base);
    assert_eq!(after, expected_accepted(&before, &after));
}

#[test]
fn amendment_acceptance_restores_branch_beyond_base() {
    let workspace = fixture("amending", "amending");
    git(&workspace.path, &["switch", "-c", "waif/task"]);
    fs::write(workspace.path.join("source.txt"), "new revision\n").unwrap();
    git(&workspace.path, &["add", "source.txt"]);
    commit(&workspace.path);
    let tip = git(&workspace.path, &["rev-parse", "HEAD"]);
    git(&workspace.path, &["switch", "main"]);
    let before = fs::read_to_string(plan_path(&workspace)).unwrap();

    let output = run(&workspace.path, &["plan", "accept"]);
    let after = fs::read_to_string(plan_path(&workspace)).unwrap();

    assert!(output.status.success(), "{}", stderr(&output));
    assert_ne!(tip, metadata(&before, "Base commit"));
    assert_eq!(git(&workspace.path, &["symbolic-ref", "--short", "HEAD"]), "waif/task");
    assert_eq!(git(&workspace.path, &["rev-parse", "HEAD"]), tip);
    assert_eq!(after, expected_accepted(&before, &after));
}

#[test]
fn accepted_plan_rejection_preserves_other_branch() {
    let workspace = fixture("accepted-other", "accepted");
    git(&workspace.path, &["switch", "-c", "waif/task"]);
    commit(&workspace.path);
    git(&workspace.path, &["switch", "main"]);
    let before = fs::read(plan_path(&workspace)).unwrap();
    let head = git(&workspace.path, &["rev-parse", "HEAD"]);
    let refs = git(&workspace.path, &["show-ref"]);

    let output = run(&workspace.path, &["plan", "accept"]);

    assert_eq!(output.status.code(), Some(1));
    assert!(stderr(&output).contains("plan is already accepted"));
    assert!(output.stdout.is_empty());
    assert_eq!(fs::read(plan_path(&workspace)).unwrap(), before);
    assert_eq!(git(&workspace.path, &["symbolic-ref", "--short", "HEAD"]), "main");
    assert_eq!(git(&workspace.path, &["rev-parse", "HEAD"]), head);
    assert_eq!(git(&workspace.path, &["show-ref"]), refs);
}

#[test]
fn accepted_plan_rejection_preserves_task_branch_and_staging() {
    let workspace = fixture("accepted-staged", "accepted");
    git(&workspace.path, &["switch", "-c", "waif/task"]);
    fs::write(workspace.path.join("source.txt"), "staged edit\n").unwrap();
    git(&workspace.path, &["add", "source.txt"]);
    let before = fs::read(plan_path(&workspace)).unwrap();
    let index = fs::read(workspace.path.join(".git/index")).unwrap();
    let head = git(&workspace.path, &["rev-parse", "HEAD"]);
    let refs = git(&workspace.path, &["show-ref"]);

    let output = run(&workspace.path, &["plan", "accept"]);

    assert_eq!(output.status.code(), Some(1));
    assert!(stderr(&output).contains("plan is already accepted"));
    assert!(output.stdout.is_empty());
    assert_eq!(fs::read(plan_path(&workspace)).unwrap(), before);
    assert_eq!(fs::read(workspace.path.join(".git/index")).unwrap(), index);
    assert_eq!(git(&workspace.path, &["symbolic-ref", "--short", "HEAD"]), "waif/task");
    assert_eq!(git(&workspace.path, &["rev-parse", "HEAD"]), head);
    assert_eq!(git(&workspace.path, &["show-ref"]), refs);
}

#[test]
fn dirty_drafting_rejection_preserves_plan_branch_and_refs() {
    let workspace = fixture("dirty", "drafting");
    fs::write(workspace.path.join("source.txt"), "dirty\n").unwrap();
    let before = fs::read(plan_path(&workspace)).unwrap();
    let refs = git(&workspace.path, &["show-ref"]);

    let output = run(&workspace.path, &["plan", "accept"]);
    let error = stderr(&output);

    assert_eq!(output.status.code(), Some(1));
    assert!(error.contains("Workflow dir: .waif\nCurrent task: 20261026-1-test\nPlan: .waif/library/tasks/20261026-1-test/plan.md\n"));
    assert!(error.contains("staged or modified tracked files"));
    assert!(output.stdout.is_empty());
    assert_eq!(fs::read(plan_path(&workspace)).unwrap(), before);
    assert_eq!(git(&workspace.path, &["show-ref"]), refs);
    assert_eq!(git(&workspace.path, &["symbolic-ref", "--short", "HEAD"]), "main");
}

#[test]
fn task_branch_drift_rejection_preserves_plan_branch_and_refs() {
    let workspace = fixture("drift", "drafting");
    git(&workspace.path, &["switch", "-c", "waif/task"]);
    commit(&workspace.path);
    git(&workspace.path, &["switch", "main"]);
    let before = fs::read(plan_path(&workspace)).unwrap();
    let refs = git(&workspace.path, &["show-ref"]);

    let output = run(&workspace.path, &["plan", "accept"]);
    let error = stderr(&output);

    assert_eq!(output.status.code(), Some(1));
    assert!(error.contains("Workflow dir: .waif\nCurrent task: 20261026-1-test\nPlan: .waif/library/tasks/20261026-1-test/plan.md\n"));
    assert!(error.contains("existing task branch differs from Base commit"));
    assert_eq!(fs::read(plan_path(&workspace)).unwrap(), before);
    assert_eq!(git(&workspace.path, &["show-ref"]), refs);
    assert_eq!(git(&workspace.path, &["symbolic-ref", "--short", "HEAD"]), "main");
}

#[test]
fn amendment_restoration_collision_preserves_git_error_and_original_state() {
    let workspace = fixture("collision", "amending");
    git(&workspace.path, &["switch", "-c", "waif/task"]);
    fs::write(workspace.path.join("collision.txt"), "tracked\n").unwrap();
    git(&workspace.path, &["add", "collision.txt"]);
    commit(&workspace.path);
    git(&workspace.path, &["switch", "main"]);
    fs::write(workspace.path.join("collision.txt"), "untracked\n").unwrap();
    let before = fs::read(plan_path(&workspace)).unwrap();

    let output = run(&workspace.path, &["plan", "accept"]);
    let error = stderr(&output);

    assert_eq!(output.status.code(), Some(1));
    assert!(error.contains("Workflow dir: .waif\nCurrent task: 20261026-1-test\nPlan:"));
    assert!(error.contains("git switch -- waif/task failed"));
    assert!(error.contains("collision.txt"));
    assert!(output.stdout.is_empty());
    assert_eq!(fs::read(plan_path(&workspace)).unwrap(), before);
    assert_eq!(git(&workspace.path, &["symbolic-ref", "--short", "HEAD"]), "main");
    assert_eq!(fs::read_to_string(workspace.path.join("collision.txt")).unwrap(), "untracked\n");
}

#[test]
fn missing_current_task_reports_context() {
    let workspace = fixture("missing-task", "drafting");
    fs::remove_file(workspace.path.join(".waif/current_task")).unwrap();
    let before = fs::read(plan_path(&workspace)).unwrap();

    let output = run(&workspace.path, &["plan", "accept"]);

    assert_eq!(output.status.code(), Some(1));
    assert!(stderr(&output).contains("Workflow dir: .waif\nCurrent task: none\nno current workflow task"));
    assert_eq!(fs::read(plan_path(&workspace)).unwrap(), before);
}

#[test]
fn malformed_current_task_reports_context() {
    let workspace = fixture("malformed-task", "drafting");
    fs::remove_file(workspace.path.join(".waif/current_task")).unwrap();
    fs::write(workspace.path.join(".waif/current_task"), "invalid pointer").unwrap();
    let before = fs::read(plan_path(&workspace)).unwrap();

    let output = run(&workspace.path, &["plan", "accept"]);

    assert_eq!(output.status.code(), Some(1));
    assert!(stderr(&output).contains("Workflow dir: .waif\n"));
    assert!(!stderr(&output).trim_end().ends_with("Workflow dir: .waif"));
    assert_eq!(fs::read(plan_path(&workspace)).unwrap(), before);
    assert_eq!(fs::read_to_string(workspace.path.join(".waif/current_task")).unwrap(), "invalid pointer");
}

#[test]
fn missing_plan_reports_context_and_preserves_goal() {
    let workspace = fixture("missing-plan", "drafting");
    fs::remove_file(plan_path(&workspace)).unwrap();
    let goal_path = plan_path(&workspace).with_file_name("goal.md");
    let goal = fs::read(&goal_path).unwrap();

    let output = run(&workspace.path, &["plan", "accept"]);

    assert_eq!(output.status.code(), Some(1));
    assert!(stderr(&output).contains("Workflow dir: .waif\nCurrent task: 20261026-1-test\nPlan:"));
    assert!(stderr(&output).contains("current task has no plan.md"));
    assert!(!plan_path(&workspace).exists());
    assert_eq!(fs::read(goal_path).unwrap(), goal);
}

#[test]
fn nonaccepted_goal_rejection_preserves_artifacts() {
    let workspace = fixture("goal-drafting", "drafting");
    let goal_path = plan_path(&workspace).with_file_name("goal.md");
    let goal = fs::read_to_string(&goal_path)
        .unwrap()
        .replace("accepted", "drafting");
    fs::write(&goal_path, &goal).unwrap();
    let before = fs::read(plan_path(&workspace)).unwrap();

    let output = run(&workspace.path, &["plan", "accept"]);

    assert_eq!(output.status.code(), Some(1));
    assert!(stderr(&output).contains("Workflow dir: .waif\nCurrent task: 20261026-1-test\nPlan:"));
    assert!(stderr(&output).contains("referenced goal is not accepted"));
    assert_eq!(fs::read(plan_path(&workspace)).unwrap(), before);
    assert_eq!(fs::read_to_string(goal_path).unwrap(), goal);
    assert_eq!(git(&workspace.path, &["symbolic-ref", "--short", "HEAD"]), "main");
}

#[test]
fn help_exposes_plan_acceptance_and_approval_contract() {
    let workspace = TestDir::new("plan-help");
    let top = run(&workspace.path, &["--help"]);
    let plan = run(&workspace.path, &["plan", "--help"]);
    let accept = run(&workspace.path, &["plan", "accept", "--help"]);
    let plan_help = String::from_utf8_lossy(&plan.stdout);
    let accept_help = String::from_utf8_lossy(&accept.stdout);

    assert!(top.status.success());
    assert!(plan.status.success());
    assert!(accept.status.success());
    assert!(String::from_utf8_lossy(&top.stdout).contains("waif plan accept"));
    assert!(plan_help.contains("explicit human approval"));
    assert!(plan_help.contains("branch is established before metadata updates"));
    assert!(plan_help.contains("Already accepted plans return an error without changing branch or artifact state"));
    assert!(accept_help.contains("explicit human approval"));
    assert!(accept_help.contains("before updating Status and Updated"));
    assert!(accept_help.contains("Already accepted plans return an error without branch or artifact mutation"));
    assert!(accept_help.contains("metadata failure reports completed branch setup"));
}

#[test]
fn extra_accept_argument_is_a_usage_error() {
    let workspace = TestDir::new("plan-extra-argument");
    let output = run(&workspace.path, &["plan", "accept", "extra"]);

    assert_eq!(output.status.code(), Some(2));
    assert!(stderr(&output).contains("unexpected argument"));
}

#[test]
fn unknown_plan_subcommand_is_a_usage_error() {
    let workspace = TestDir::new("plan-unknown-subcommand");
    let output = run(&workspace.path, &["plan", "unknown"]);

    assert_eq!(output.status.code(), Some(2));
    assert!(stderr(&output).contains("unrecognized subcommand"));
}

fn fixture(name: &str, status: &str) -> TestDir {
    let workspace = TestDir::new(name);
    git(&workspace.path, &["init", "-b", "main"]);
    fs::write(workspace.path.join(".git/info/exclude"), "/.waif/\n").unwrap();
    fs::write(workspace.path.join("source.txt"), "source\n").unwrap();
    git(&workspace.path, &["add", "source.txt"]);
    commit(&workspace.path);
    let base = git(&workspace.path, &["rev-parse", "HEAD"]);
    let path = plan_path(&workspace);
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    symlink(
        "library/tasks/20261026-1-test",
        workspace.path.join(".waif/current_task"),
    )
    .unwrap();
    fs::write(
        path.with_file_name("goal.md"),
        format!(
            "# Goal: Test\n\n- Status: accepted\n- Created: {TIMESTAMP}\n\
                 - Updated: {TIMESTAMP}\n\n## Outcome\n\nTest\n\n\
                 ## Acceptance Criteria\n\n- G-AC1: Test\n"
        ),
    )
    .unwrap();
    fs::write(
        path,
        format!(
            "# Plan: Test\n\n- Status: {status}\n- Goal: ./goal.md\n\
                 - Branch: waif/task\n- Base branch: main\n- Base commit: {base}\n\
                 - Created: {TIMESTAMP}\n- Updated: {TIMESTAMP}\n\n\
                 ## Technical Summary\n\nApproach  \n\n\
                 ## Decisions\n\n- P-D1: Decision\n\n\
                 ## Current System\n\nContext\n\n\
                 ## Plan Items\n\n### P1: First\n\n- Status: pending\n\
                 - Goal criteria: opaque coverage\n- Depends on: opaque dependency\n\n\
                 ### P2: Earlier work\n\n- Status: done\n\nProse  with spacing.\n\n\
                 ## Risks\n\n- P-R1: Risk\n\n\
                 ## Cross-Cutting Validation\n\n- P-V1: Check\n\n\
                 ## Revisions\n\n### P-REV1: {TIMESTAMP} - Fixture revision\n\n\
                 - Before: Earlier approach\n- Changed:\n  - Revised approach\n"
        ),
    )
    .unwrap();
    workspace
}

fn plan_path(workspace: &TestDir) -> PathBuf {
    workspace
        .path
        .join(".waif/library/tasks/20261026-1-test/plan.md")
}

fn run(workspace: &Path, args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_waif"))
        .args(args)
        .current_dir(workspace)
        .output()
        .unwrap()
}

fn git(workspace: &Path, args: &[&str]) -> String {
    let output = Command::new("git")
        .args(args)
        .current_dir(workspace)
        .output()
        .unwrap();
    if !output.status.success() {
        panic!("git {args:?}: {}", stderr(&output));
    }
    String::from_utf8(output.stdout)
        .unwrap()
        .trim_end()
        .to_owned()
}

fn commit(workspace: &Path) {
    git(
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
    );
}

fn metadata<'a>(source: &'a str, key: &str) -> &'a str {
    source
        .lines()
        .find_map(|line| line.strip_prefix(&format!("- {key}: ")))
        .unwrap()
}

fn expected_accepted(before: &str, after: &str) -> String {
    before
        .replace("- Status: drafting", "- Status: accepted")
        .replace("- Status: amending", "- Status: accepted")
        .replace(
            &format!("- Updated: {TIMESTAMP}"),
            &format!("- Updated: {}", metadata(after, "Updated")),
        )
}

fn stderr(output: &Output) -> String {
    String::from_utf8_lossy(&output.stderr).into_owned()
}
