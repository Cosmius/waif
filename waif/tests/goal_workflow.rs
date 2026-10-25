use std::fs;
use std::process::{Command, Output};

mod test_support;

use test_support::TestDir;

const DRAFT_GOAL: &str = concat!(
    "# Goal: Test workflow\n",
    "\n",
    "- Status: drafting\n",
    "- Created: 2026-09-16T12:00:00+09:00\n",
    "- Updated: __NOW__\n",
    "\n",
    "## Outcome\n",
    "\n",
    "Exercise the goal workflow.\n",
    "\n",
    "## Acceptance Criteria\n",
    "\n",
    "- G-AC1: The workflow succeeds.\n",
);

#[cfg(unix)]
#[test]
fn goal_stage_commands_complete_one_workflow_without_crossing_mutation_boundaries() {
    let workspace = TestDir::new("goal-workflow");

    let created = run(&workspace, &["new", "first-goal"]);
    let initial = run(&workspace, &["inspect"]);
    let first_current = fs::read_link(workspace.path.join(".waif/current_task"))
        .expect("current task should be a symlink");
    let first_name = first_current
        .file_name()
        .expect("current task should have a name")
        .to_string_lossy()
        .into_owned();

    assert!(created.status.success(), "{}", String::from_utf8_lossy(&created.stderr));
    assert!(initial.status.success(), "{}", String::from_utf8_lossy(&initial.stderr));
    assert!(first_name.ends_with("-1-first-goal"));
    assert!(String::from_utf8_lossy(&initial.stdout).contains("Next:               Draft goal"));

    let goal_path = workspace
        .path
        .join(".waif/library/tasks")
        .join(&first_name)
        .join("goal.md");
    let rejected_source = DRAFT_GOAL.replace("drafting", "prefix__NOW__suffix");
    fs::write(&goal_path, &rejected_source).expect("rejected goal should be written");

    let rejected = run(&workspace, &["fill", ".waif/current_task/goal.md"]);

    assert_eq!(rejected.status.code(), Some(1));
    assert_eq!(
        fs::read_to_string(&goal_path).expect("rejected target should remain"),
        rejected_source
    );
    assert!(goal_path.with_file_name("goal.md.waif-recovery").is_file());
    assert!(String::from_utf8_lossy(&rejected.stderr).contains("Recovery:"));

    fs::write(&goal_path, DRAFT_GOAL).expect("draft goal should be written");
    let incomplete = run(&workspace, &["check"]);
    let filled = run(&workspace, &["fill", ".waif/current_task/goal.md"]);
    let complete = run(&workspace, &["check"]);

    assert!(incomplete.status.success());
    assert!(String::from_utf8_lossy(&incomplete.stdout).contains("INCOMPLETE"));
    assert!(filled.status.success(), "{}", String::from_utf8_lossy(&filled.stderr));
    assert!(complete.status.success());
    assert!(String::from_utf8_lossy(&complete.stdout).contains("GOOD"));
    assert!(!fs::read_to_string(&goal_path).expect("goal should load").contains("__NOW__"));

    let accepted = run(&workspace, &["goal", "accept"]);
    let accepted_inspection = run(&workspace, &["inspect"]);
    let no_fill = run(&workspace, &["fill", ".waif/current_task/goal.md"]);

    assert!(accepted.status.success(), "{}", String::from_utf8_lossy(&accepted.stderr));
    assert!(fs::read_to_string(&goal_path)
        .expect("accepted goal should load")
        .contains("- Status: accepted"));
    assert!(accepted_inspection.status.success());
    assert!(String::from_utf8_lossy(&accepted_inspection.stdout)
        .contains("Next:               Draft plan"));
    assert!(no_fill.status.success());
    assert!(String::from_utf8_lossy(&no_fill.stdout).contains("No fill needed:"));

    let second = run(&workspace, &["new", "second-goal"]);
    let second_current = fs::read_link(workspace.path.join(".waif/current_task"))
        .expect("current task should be a symlink");
    let second_name = second_current
        .file_name()
        .expect("current task should have a name")
        .to_string_lossy()
        .into_owned();
    let switched = run(&workspace, &["switch", &first_name]);

    assert!(second.status.success(), "{}", String::from_utf8_lossy(&second.stderr));
    assert!(second_name.ends_with("-second-goal"));
    assert!(workspace.path.join(".waif/library/tasks").join(&second_name).is_dir());
    assert!(switched.status.success(), "{}", String::from_utf8_lossy(&switched.stderr));
    assert_eq!(
        fs::read_link(workspace.path.join(".waif/current_task"))
            .expect("current task should be a symlink"),
        std::path::PathBuf::from(format!("library/tasks/{first_name}"))
    );
}

fn run(workspace: &TestDir, arguments: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_waif"))
        .args(arguments)
        .current_dir(&workspace.path)
        .output()
        .expect("waif command should run")
}
