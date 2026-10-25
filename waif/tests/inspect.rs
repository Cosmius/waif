use std::fs;
#[cfg(unix)]
use std::os::unix::fs::symlink;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

mod test_support;

use test_support::TestDir;

#[cfg(unix)]
#[test]
fn inspect_reports_ready_drafting_goal_in_the_existing_layout() {
    let workspace = current_task_workspace("ready-goal");
    write_goal(&workspace, "drafting", "2026-08-17T12:00:00+09:00", "");

    let output = run_inspect(&workspace.path);
    let stdout = String::from_utf8_lossy(&output.stdout);

    assert!(output.status.success());
    assert!(stdout.contains("Workflow dir:       .waif"));
    assert!(stdout.contains("Library:            .waif/library"));
    assert!(stdout.contains("Current task:       20260728-1-example"));
    assert!(stdout.contains("Current task dir:   .waif/library/tasks/20260728-1-example"));
    assert!(stdout.contains(
        "Goal:\n  Path:             .waif/library/tasks/20260728-1-example/goal.md"
    ));
    assert!(stdout.contains("  Status:           drafting"));
    assert!(stdout.contains("  Ready to accept:  yes"));
    assert!(stdout.contains("Next:               Revise or accept goal\n"));
}

#[cfg(unix)]
#[test]
fn inspect_reports_open_questions_as_not_ready() {
    let workspace = current_task_workspace("open-questions");
    write_goal(
        &workspace,
        "drafting",
        "2026-08-17T12:00:00+09:00",
        QUESTIONS,
    );

    let output = run_inspect(&workspace.path);
    let stdout = String::from_utf8_lossy(&output.stdout);

    assert!(output.status.success());
    assert!(stdout.contains("  Status:           drafting"));
    assert!(stdout.contains("  Ready to accept:  no"));
    assert!(stdout.contains("Next:               Revise goal\n"));
}

#[cfg(unix)]
#[test]
fn inspect_reports_incomplete_amending_goal() {
    let workspace = current_task_workspace("incomplete");
    write_goal(&workspace, "amending", "__NOW__", "");

    let output = run_inspect(&workspace.path);
    let stdout = String::from_utf8_lossy(&output.stdout);

    assert!(output.status.success());
    assert!(stdout.contains("  Status:           amending"));
    assert!(stdout.contains("  Ready to accept:  no"));
    assert!(stdout.contains("Next:               Fill goal\n"));
}

#[cfg(unix)]
#[test]
fn inspect_reports_accepted_goal_without_readiness() {
    let workspace = current_task_workspace("accepted");
    write_goal(&workspace, "accepted", "2026-08-17T12:00:00+09:00", "");

    let output = run_inspect(&workspace.path);
    let stdout = String::from_utf8_lossy(&output.stdout);

    assert!(output.status.success());
    assert!(stdout.contains("  Status:           accepted"));
    assert!(!stdout.contains("Ready to accept:"));
    assert!(stdout.contains("Next:               Draft plan\n"));
}

#[cfg(unix)]
#[test]
fn inspect_treats_empty_open_questions_as_ready() {
    let workspace = current_task_workspace("empty-questions");
    write_goal(
        &workspace,
        "drafting",
        "2026-08-17T12:00:00+09:00",
        "\n## Open Questions\n",
    );

    let output = run_inspect(&workspace.path);
    let stdout = String::from_utf8_lossy(&output.stdout);

    assert!(output.status.success());
    assert!(stdout.contains("  Status:           drafting"));
    assert!(stdout.contains("  Ready to accept:  yes"));
    assert!(stdout.contains("Next:               Revise or accept goal\n"));
}

#[cfg(unix)]
#[test]
fn inspect_reports_missing_and_bad_goals_without_diagnostics() {
    let missing = current_task_workspace("missing-goal");
    let missing_output = run_inspect(&missing.path);
    let missing_stdout = String::from_utf8_lossy(&missing_output.stdout);

    assert!(missing_output.status.success());
    assert!(missing_stdout.contains(
        "Goal:\n  not created yet\nNext:               Draft goal\n"
    ));

    let bad = current_task_workspace("bad-goal");
    fs::write(current_task_path(&bad).join("goal.md"), "not a goal\n")
        .expect("goal should be written");
    let bad_output = run_inspect(&bad.path);
    let bad_stdout = String::from_utf8_lossy(&bad_output.stdout);
    let bad_stderr = String::from_utf8_lossy(&bad_output.stderr);

    assert_eq!(bad_output.status.code(), Some(1));
    assert!(bad_stdout.contains("  Status:           BAD\nNext:               Check goal\n"));
    assert_eq!(bad_stderr, "workflow context is unusable\n");
    assert!(!bad_stdout.contains("first non-whitespace line"));
    assert!(!bad_stderr.contains("first non-whitespace line"));
}

#[cfg(unix)]
#[test]
fn inspect_reports_unavailable_context_with_one_next_instruction() {
    let missing = TestDir::new("missing-workflow");
    let missing_output = run_inspect(&missing.path);
    let missing_stdout = String::from_utf8_lossy(&missing_output.stdout);

    assert_eq!(missing_output.status.code(), Some(1));
    assert!(missing_stdout.contains(
        "Current task:       unavailable\nNext:               Run waif init or repair library\n"
    ));
    assert!(String::from_utf8_lossy(&missing_output.stderr).contains(".waif/library/tasks: No such file or directory"));
    assert!(!missing.path.join(".waif").exists());

    let no_current = TestDir::new("missing-current");
    fs::create_dir_all(no_current.path.join(".waif/library/tasks"))
        .expect("tasks should be created");
    let no_current_output = run_inspect(&no_current.path);
    let no_current_stdout = String::from_utf8_lossy(&no_current_output.stdout);

    assert_eq!(no_current_output.status.code(), Some(1));
    assert!(no_current_stdout.contains(
        "Current task:       none\nNext:               Create or select task\n"
    ));

    let selectable = TestDir::new("selectable-task");
    fs::create_dir_all(
        selectable
            .path
            .join(".waif/library/tasks/20260818-1-existing-task"),
    )
    .expect("task should be created");
    let selectable_output = run_inspect(&selectable.path);
    let selectable_stdout = String::from_utf8_lossy(&selectable_output.stdout);

    assert_eq!(selectable_output.status.code(), Some(1));
    assert!(selectable_stdout.contains(
        "Current task:       none\nNext:               Create or select task\n"
    ));
}

#[cfg(unix)]
#[test]
fn inspect_preserves_pointer_read_errors_without_changing_the_entry() {
    let workspace = current_task_workspace("invalid-current");
    let pointer = workspace.path.join(".waif/current_task");
    fs::remove_file(&pointer).unwrap();
    fs::write(&pointer, "not a link").unwrap();
    let native_error = fs::read_link(&pointer).unwrap_err();

    let output = run_inspect(&workspace.path);
    let stderr = String::from_utf8_lossy(&output.stderr);

    assert!(!output.status.success());
    assert!(stderr.contains(&format!(".waif/current_task: {native_error}")));
    assert!(String::from_utf8_lossy(&output.stdout).contains("Next:               Repair or select task"));
    assert_eq!(fs::read_to_string(&pointer).unwrap(), "not a link");
}

#[cfg(unix)]
#[test]
fn inspect_preserves_a_dangling_selected_task_and_reports_its_path() {
    let workspace = current_task_workspace("dangling-selection");
    fs::remove_dir(current_task_path(&workspace)).unwrap();
    let pointer = workspace.path.join(".waif/current_task");
    let before = fs::read_link(&pointer).unwrap();
    let native_error = fs::read_dir(current_task_path(&workspace)).unwrap_err();

    let output = run_inspect(&workspace.path);

    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains(&format!(".waif/library/tasks/20260728-1-example: {native_error}")));
    assert!(String::from_utf8_lossy(&output.stdout).contains("Current task:       20260728-1-example"));
    assert_eq!(fs::read_link(&pointer).unwrap(), before);
    assert!(!current_task_path(&workspace).exists());
}

#[cfg(unix)]
#[test]
fn inspect_is_read_only_and_rejects_arguments() {
    let workspace = current_task_workspace("read-only");
    write_goal(&workspace, "drafting", "2026-08-17T12:00:00+09:00", "");
    let goal_path = current_task_path(&workspace).join("goal.md");
    let before_goal = fs::read(&goal_path).expect("goal should be readable");
    let before_current = fs::read_link(workspace.path.join(".waif/current_task"))
        .expect("current link should be readable");

    let first = run_inspect(&workspace.path);
    let second = run_inspect(&workspace.path);
    let extra = Command::new(env!("CARGO_BIN_EXE_waif"))
        .args(["inspect", "extra"])
        .current_dir(&workspace.path)
        .output()
        .expect("waif inspect should run");

    assert!(first.status.success());
    assert_eq!(first.stdout, second.stdout);
    assert_eq!(fs::read(&goal_path).expect("goal should remain readable"), before_goal);
    assert_eq!(
        fs::read_link(workspace.path.join(".waif/current_task"))
            .expect("current link should remain readable"),
        before_current
    );
    assert_eq!(extra.status.code(), Some(2));
}

#[cfg(unix)]
fn current_task_workspace(name: &str) -> TestDir {
    let workspace = TestDir::new(name);
    let tasks = workspace.path.join(".waif/library/tasks");
    fs::create_dir_all(tasks.join("20260728-1-example")).expect("task should be created");
    symlink(
        "library/tasks/20260728-1-example",
        workspace.path.join(".waif/current_task"),
    )
    .expect("current task link should be created");
    workspace
}

fn current_task_path(workspace: &TestDir) -> PathBuf {
    workspace
        .path
        .join(".waif/library/tasks/20260728-1-example")
}

fn write_goal(workspace: &TestDir, status: &str, updated: &str, questions: &str) {
    fs::write(
        current_task_path(workspace).join("goal.md"),
        format!(
            "# Goal: Example\n\n\
             - Status: {status}\n\
             - Created: 2026-08-17T12:00:00+09:00\n\
             - Updated: {updated}\n\n\
             ## Outcome\n\nAn outcome\n\n\
             ## Acceptance Criteria\n\n- G-AC1: A criterion\n\
             {questions}"
        ),
    )
    .expect("goal should be written");
}

fn run_inspect(current_dir: &Path) -> Output {
    Command::new(env!("CARGO_BIN_EXE_waif"))
        .arg("inspect")
        .current_dir(current_dir)
        .output()
        .expect("waif inspect should run")
}

const QUESTIONS: &str = "\n## Open Questions\n\n- G-Q1: A question\n";
