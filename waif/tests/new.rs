use std::fs;
use std::process::{Command, Output};

use chrono::Local;

mod test_support;

use test_support::TestDir;

#[cfg(unix)]
#[test]
fn new_initializes_a_missing_workflow_and_selects_the_task() {
    let workspace = TestDir::new("new-initialize");
    let date_before = Local::now().format("%Y%m%d").to_string();

    let output = run_new(&workspace, &["first-task"]);
    let date_after = Local::now().format("%Y%m%d").to_string();
    let current = fs::read_link(workspace.path.join(".waif/current_task"))
        .expect("current task should be a symlink");
    let task_name = current
        .file_name()
        .expect("current task should have a name")
        .to_string_lossy();

    assert!(output.status.success(), "{}", String::from_utf8_lossy(&output.stderr));
    assert!(workspace.path.join(".waif/.git").is_dir());
    assert!(task_name == format!("{date_before}-1-first-task")
        || task_name == format!("{date_after}-1-first-task"));
    assert!(workspace
        .path
        .join(".waif/library/tasks")
        .join(task_name.as_ref())
        .is_dir());
    assert_eq!(current, std::path::PathBuf::from(format!("library/tasks/{task_name}")));
    assert!(String::from_utf8_lossy(&output.stdout).contains(task_name.as_ref()));
}

#[cfg(unix)]
#[test]
fn new_sequences_tasks_without_touching_existing_repository_metadata() {
    let workspace = TestDir::new("new-existing-workflow");
    let workflow = workspace.path.join(".waif");
    let tasks = workflow.join("library/tasks");
    let date_before = Local::now().format("%Y%m%d").to_string();
    fs::create_dir_all(workflow.join(".git")).expect("git metadata dir should be created");
    fs::create_dir_all(tasks.join(format!("{date_before}-1-existing-task")))
        .expect("existing task should be created");
    fs::write(workflow.join(".git/sentinel"), "preserve")
        .expect("metadata sentinel should be written");
    let mut metadata_before = fs::read_dir(workflow.join(".git"))
        .expect("git metadata should be readable")
        .map(|entry| entry.expect("metadata entry should load").file_name())
        .collect::<Vec<_>>();
    metadata_before.sort();

    let output = run_new(&workspace, &["next-task"]);
    let date_after = Local::now().format("%Y%m%d").to_string();
    let current = fs::read_link(workspace.path.join(".waif/current_task"))
        .expect("current task should be a symlink");
    let task_name = current
        .file_name()
        .expect("current task should have a name")
        .to_string_lossy();
    let mut metadata_after = fs::read_dir(workflow.join(".git"))
        .expect("git metadata should remain readable")
        .map(|entry| entry.expect("metadata entry should load").file_name())
        .collect::<Vec<_>>();
    metadata_after.sort();

    assert!(output.status.success(), "{}", String::from_utf8_lossy(&output.stderr));
    assert!(task_name == format!("{date_before}-2-next-task")
        || task_name == format!("{date_after}-1-next-task"));
    assert!(tasks.join(task_name.as_ref()).is_dir());
    assert_eq!(metadata_after, metadata_before);
    assert_eq!(
        fs::read_to_string(workflow.join(".git/sentinel"))
            .expect("metadata sentinel should remain"),
        "preserve"
    );
    assert_eq!(current, std::path::PathBuf::from(format!("library/tasks/{task_name}")));
}

fn run_new(workspace: &TestDir, arguments: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_waif"))
        .arg("new")
        .args(arguments)
        .current_dir(&workspace.path)
        .output()
        .expect("waif new should run")
}
