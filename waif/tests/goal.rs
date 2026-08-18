use std::fs;
use std::path::Path;
use std::process::{Command, Output};

mod test_support;

use test_support::TestDir;

#[cfg(unix)]
use std::os::unix::fs::symlink;

#[cfg(unix)]
#[test]
fn accept_updates_only_status_and_updated_for_a_drafting_goal() {
    let workspace = current_task_workspace("accept-drafting");
    let source = valid_goal("drafting", "2026-08-17T12:00:00+09:00", "body");
    let path = current_task_path(&workspace).join("goal.md");
    fs::write(&path, &source).expect("goal should be written");

    let output = run_goal(&workspace.path, &["accept"]);
    let contents = fs::read_to_string(&path).expect("goal should load");

    assert!(output.status.success(), "{}", stderr(&output));
    assert!(String::from_utf8_lossy(&output.stdout).contains("Status:             accepted"));
    assert!(contents.contains("- Status: accepted\n"));
    assert!(contents.contains("- Created: 2026-08-17T12:00:00+09:00\n"));
    assert!(contents.contains("## Outcome\n\nbody\n"));
    let updated = metadata_value(&contents, "Updated");
    assert!(chrono::DateTime::parse_from_rfc3339(updated).is_ok());
    assert_ne!(contents, source);
}

#[cfg(unix)]
#[test]
fn accept_updates_an_amending_goal() {
    let workspace = current_task_workspace("accept-amending");
    let path = current_task_path(&workspace).join("goal.md");
    fs::write(
        &path,
        valid_goal("amending", "2026-08-17T12:00:00+09:00", "revised"),
    )
    .expect("goal should be written");

    let output = run_goal(&workspace.path, &["accept"]);

    assert!(output.status.success(), "{}", stderr(&output));
    assert_eq!(metadata_value(&fs::read_to_string(path).unwrap(), "Status"), "accepted");
}

#[cfg(unix)]
#[test]
fn accept_allows_a_warning_only_goal() {
    let workspace = current_task_workspace("accept-warning-only");
    let path = current_task_path(&workspace).join("goal.md");
    fs::write(
        &path,
        valid_goal("drafting", "2026-08-17T12:00:00+09:00", ""),
    )
    .expect("goal should be written");

    let output = run_goal(&workspace.path, &["accept"]);

    assert!(output.status.success(), "{}", stderr(&output));
    assert_eq!(metadata_value(&fs::read_to_string(path).unwrap(), "Status"), "accepted");
}

#[cfg(unix)]
#[test]
fn accept_follows_a_symlinked_goal_file() {
    let workspace = current_task_workspace("accept-symlinked-goal");
    let external = workspace.path.join("external-goal.md");
    fs::write(
        &external,
        valid_goal("drafting", "2026-08-17T12:00:00+09:00", "body"),
    )
    .expect("external goal should be written");
    symlink(
        "../../../external-goal.md",
        current_task_path(&workspace).join("goal.md"),
    )
    .expect("goal link should be created");

    let output = run_goal(&workspace.path, &["accept"]);

    assert!(output.status.success(), "{}", stderr(&output));
    assert_eq!(metadata_value(&fs::read_to_string(external).unwrap(), "Status"), "accepted");
}

#[cfg(unix)]
#[test]
fn accept_rejects_open_questions_without_mutating_the_file() {
    let workspace = current_task_workspace("open-questions");
    let source = format!(
        "{}\n## Open Questions\n\n- G-Q1: unresolved\n",
        valid_goal("drafting", "2026-08-17T12:00:00+09:00", "body")
    );
    let (output, contents) = run_rejected_goal(&workspace, &source);
    assert_eq!(output.status.code(), Some(1));
    assert!(stderr(&output).contains("blocking open questions"));
    assert!(stderr(&output).contains("Workflow dir:"));
    assert!(stderr(&output).contains("Current task:"));
    assert!(stderr(&output).contains("Goal:"));
    assert_eq!(contents, source);
}

#[cfg(unix)]
#[test]
fn accept_rejects_incomplete_goal_without_mutating_the_file() {
    let workspace = current_task_workspace("incomplete");
    let source = valid_goal("drafting", "__NOW__", "body");
    let (output, contents) = run_rejected_goal(&workspace, &source);
    assert_eq!(output.status.code(), Some(1));
    assert!(stderr(&output).contains("goal is incomplete"));
    assert!(stderr(&output).contains("Workflow dir:"));
    assert!(stderr(&output).contains("Current task:"));
    assert!(stderr(&output).contains("Goal:"));
    assert_eq!(contents, source);
}

#[cfg(unix)]
#[test]
fn accept_rejects_an_already_accepted_goal_without_mutating_the_file() {
    let workspace = current_task_workspace("accepted");
    let source = valid_goal("accepted", "2026-08-17T12:00:00+09:00", "body");
    let (output, contents) = run_rejected_goal(&workspace, &source);
    assert_eq!(output.status.code(), Some(1));
    assert!(stderr(&output).contains("cannot be reaccepted"));
    assert!(stderr(&output).contains("Workflow dir:"));
    assert!(stderr(&output).contains("Current task:"));
    assert!(stderr(&output).contains("Goal:"));
    assert_eq!(contents, source);
}

#[cfg(unix)]
#[test]
fn accept_rejects_a_bad_goal_without_mutating_the_file() {
    let workspace = current_task_workspace("bad");
    let source = "not a goal\n";
    let (output, contents) = run_rejected_goal(&workspace, source);
    assert_eq!(output.status.code(), Some(1));
    assert!(stderr(&output).contains("ERROR:"));
    assert!(stderr(&output).contains("Workflow dir:"));
    assert!(stderr(&output).contains("Current task:"));
    assert!(stderr(&output).contains("Goal:"));
    assert_eq!(contents, source);
}

#[cfg(unix)]
#[test]
fn accept_rejects_missing_context_and_arguments() {
    let workspace = TestDir::new("accept-no-context");
    let output = run_goal(&workspace.path, &["accept"]);
    assert_eq!(output.status.code(), Some(1));
    assert!(stderr(&output).contains("no current workflow task"));
    assert!(stderr(&output).contains("Workflow dir:"));

    let workspace = current_task_workspace("accept-arguments");
    let output = run_goal(&workspace.path, &["accept", "extra"]);
    assert_eq!(output.status.code(), Some(2));
}

#[cfg(unix)]
#[test]
fn accept_rejects_a_missing_goal() {
    let workspace = current_task_workspace("accept-missing-goal");

    let output = run_goal(&workspace.path, &["accept"]);

    assert_eq!(output.status.code(), Some(1));
    assert!(stderr(&output).contains("current task has no goal.md"));
    assert!(stderr(&output).contains("Workflow dir:"));
    assert!(stderr(&output).contains("Current task:"));
    assert!(stderr(&output).contains("Goal:"));
}

#[cfg(unix)]
#[test]
fn accept_reports_all_practical_goal_blockers() {
    let workspace = current_task_workspace("accept-all-blockers");
    let path = current_task_path(&workspace).join("goal.md");
    let source = format!(
        "{}\n## Open Questions\n\n- G-Q1: unresolved\n",
        valid_goal("accepted", "2026-08-17T12:00:00+09:00", "body")
    );
    fs::write(&path, &source).expect("goal should be written");

    let output = run_goal(&workspace.path, &["accept"]);
    let contents = fs::read_to_string(&path).expect("goal should load");
    let error = stderr(&output);

    assert_eq!(output.status.code(), Some(1));
    assert!(error.contains("goal cannot be reaccepted"));
    assert!(error.contains("goal has blocking open questions"));
    assert_eq!(contents, source);
}

#[cfg(unix)]
fn current_task_workspace(name: &str) -> TestDir {
    let workspace = TestDir::new(name);
    let tasks = workspace.path.join(".waif/tasks");
    fs::create_dir_all(tasks.join("20260728-1-example")).expect("task should be created");
    symlink("./20260728-1-example", tasks.join("current"))
        .expect("current task link should be created");
    workspace
}

#[cfg(unix)]
fn current_task_path(workspace: &TestDir) -> std::path::PathBuf {
    workspace.path.join(".waif/tasks/20260728-1-example")
}

#[cfg(unix)]
fn run_rejected_goal(workspace: &TestDir, source: &str) -> (Output, String) {
    let path = current_task_path(workspace).join("goal.md");
    fs::write(&path, source).expect("goal should be written");

    let output = run_goal(&workspace.path, &["accept"]);
    let contents = fs::read_to_string(path).expect("goal should load");
    (output, contents)
}

fn valid_goal(status: &str, updated: &str, outcome: &str) -> String {
    format!(
        "# Goal: Example\n\n\
         - Status: {status}\n\
         - Created: 2026-08-17T12:00:00+09:00\n\
         - Updated: {updated}\n\n\
         ## Outcome\n\n{outcome}\n\n\
         ## Acceptance Criteria\n\n- G-AC1: A criterion\n"
    )
}

fn metadata_value<'a>(source: &'a str, key: &str) -> &'a str {
    source
        .lines()
        .find_map(|line| line.strip_prefix(&format!("- {key}: ")))
        .expect("metadata should exist")
}

fn run_goal(current_dir: &Path, arguments: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_waif"))
        .arg("goal")
        .args(arguments)
        .current_dir(current_dir)
        .output()
        .expect("waif goal should run")
}

fn stderr(output: &Output) -> String {
    String::from_utf8_lossy(&output.stderr).into_owned()
}
