use std::fs;
#[cfg(unix)]
use std::os::unix::fs::symlink;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

mod test_support;

use test_support::TestDir;

#[cfg(unix)]
#[test]
fn switch_selects_an_exact_existing_task_and_reports_context() {
    let workspace = switch_workspace("successful-switch");
    let tasks_dir = workspace.path.join(".waif/tasks");
    fs::create_dir(tasks_dir.join("20260817-2-selected-task"))
        .expect("selected task should be created");

    let output = run_switch(&workspace.path, &["20260817-2-selected-task"]);
    let stdout = String::from_utf8_lossy(&output.stdout);

    assert!(output.status.success());
    assert_eq!(
        fs::read_link(tasks_dir.join("current")).expect("current link should exist"),
        PathBuf::from("./20260817-2-selected-task")
    );
    assert!(stdout.contains("Current task:       20260817-2-selected-task"));
    assert!(stdout.contains(
        "Current task dir:   .waif/tasks/20260817-2-selected-task"
    ));
    assert!(stdout.contains(
        "Current reference:  .waif/tasks/current -> ./20260817-2-selected-task"
    ));
}

#[cfg(unix)]
#[test]
fn switch_rejects_targets_without_changing_current() {
    for (name, setup) in [
        ("malformed", TargetSetup::None),
        ("20260817-2-missing", TargetSetup::None),
        ("20260817-3-regular-file", TargetSetup::File),
    ] {
        let workspace = switch_workspace(name);
        let tasks_dir = workspace.path.join(".waif/tasks");
        setup.create(&tasks_dir, name);

        let before =
            fs::read_link(tasks_dir.join("current")).expect("current link should be readable");
        let output = run_switch(&workspace.path, &[name]);
        let after =
            fs::read_link(tasks_dir.join("current")).expect("current link should remain readable");

        assert!(!output.status.success(), "target should fail: {name}");
        assert_eq!(after, before, "current should be preserved: {name}");
    }
}

#[cfg(unix)]
#[test]
fn switch_follows_a_task_symlink_to_a_nominal_directory() {
    let workspace = switch_workspace("linked-task");
    let tasks_dir = workspace.path.join(".waif/tasks");
    fs::create_dir(workspace.path.join(".waif/outside"))
        .expect("outside directory should be created");
    symlink("../outside", tasks_dir.join("20260817-2-linked-task"))
        .expect("task symlink should be created");

    let output = run_switch(&workspace.path, &["20260817-2-linked-task"]);
    let inspect = run_command(&workspace.path, "inspect", &[]);
    let inspect_stdout = String::from_utf8_lossy(&inspect.stdout);

    assert!(output.status.success());
    assert!(inspect.status.success());
    assert_eq!(
        fs::read_link(tasks_dir.join("current")).expect("current link should exist"),
        PathBuf::from("./20260817-2-linked-task")
    );
    assert!(inspect_stdout.contains("Current task:       20260817-2-linked-task"));
}

#[cfg(unix)]
#[test]
fn switch_preserves_an_invalid_existing_current_reference() {
    let workspace = TestDir::new("invalid-current");
    let tasks_dir = workspace.path.join(".waif/tasks");
    fs::create_dir_all(tasks_dir.join("20260817-1-selected-task"))
        .expect("selected task should be created");
    fs::write(tasks_dir.join("current"), "not a symlink")
        .expect("invalid current file should be created");

    let output = run_switch(&workspace.path, &["20260817-1-selected-task"]);

    assert!(!output.status.success());
    assert_eq!(
        fs::read_to_string(tasks_dir.join("current")).expect("current file should remain"),
        "not a symlink"
    );
}

#[test]
fn switch_requires_exactly_one_argument() {
    let workspace = TestDir::new("switch-arguments");

    let missing = run_switch(&workspace.path, &[]);
    let extra = run_switch(&workspace.path, &["20260817-1-first", "20260817-2-second"]);

    assert_eq!(missing.status.code(), Some(2));
    assert_eq!(extra.status.code(), Some(2));
}

#[cfg(unix)]
fn switch_workspace(name: &str) -> TestDir {
    let workspace = TestDir::new(name);
    let tasks_dir = workspace.path.join(".waif/tasks");
    fs::create_dir_all(tasks_dir.join("20260817-1-current-task"))
        .expect("current task should be created");
    symlink("./20260817-1-current-task", tasks_dir.join("current"))
        .expect("current link should be created");
    workspace
}

enum TargetSetup {
    None,
    File,
}

impl TargetSetup {
    #[cfg(unix)]
    fn create(&self, tasks_dir: &Path, name: &str) {
        match self {
            Self::None => {}
            Self::File => fs::write(tasks_dir.join(name), "not a directory")
                .expect("file target should be created"),
        }
    }
}

fn run_switch(current_dir: &Path, arguments: &[&str]) -> Output {
    run_command(current_dir, "switch", arguments)
}

fn run_command(current_dir: &Path, command: &str, arguments: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_waif"))
        .arg(command)
        .args(arguments)
        .current_dir(current_dir)
        .output()
        .expect("waif command should run")
}
