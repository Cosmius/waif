use std::fs;
#[cfg(unix)]
use std::os::unix::fs::symlink;
use std::path::PathBuf;
use std::process::{Command, Output};

mod test_support;

use test_support::TestDir;

#[cfg(unix)]
#[test]
fn inspect_reads_goal_metadata_with_the_artifact_parser() {
    let workspace = current_task_workspace("valid-goal");
    fs::write(
        current_task_path(&workspace).join("goal.md"),
        "\
# Goal: Example

- Status: drafting

## Goal

Build the example.
",
    )
    .expect("goal should be written");

    let output = run_inspect(&workspace.path);
    let stdout = String::from_utf8_lossy(&output.stdout);

    assert!(output.status.success());
    assert!(stdout.contains("Status:           drafting"));
}

#[cfg(unix)]
#[test]
fn inspect_rejects_a_goal_without_a_title() {
    let workspace = current_task_workspace("invalid-goal");
    fs::write(
        current_task_path(&workspace).join("goal.md"),
        "\
- Status: drafting

## Goal

Build the example.
",
    )
    .expect("goal should be written");

    let output = run_inspect(&workspace.path);
    let stderr = String::from_utf8_lossy(&output.stderr);

    assert_eq!(output.status.code(), Some(1));
    assert!(stderr
        .contains("goal.md:1: first non-whitespace line must be a level-one Markdown heading"));
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

fn current_task_path(workspace: &TestDir) -> PathBuf {
    workspace.path.join(".waif/tasks/20260728-1-example")
}

fn run_inspect(current_dir: &std::path::Path) -> Output {
    Command::new(env!("CARGO_BIN_EXE_waif"))
        .arg("inspect")
        .current_dir(current_dir)
        .output()
        .expect("waif inspect should run")
}
