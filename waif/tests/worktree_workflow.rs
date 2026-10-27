#![cfg(unix)]

use test_support::git_repo::GitRepo;

use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

mod test_support;

use test_support::TestDir;

const DRAFT_GOAL: &str = "# Goal: Worktree task\n\
    - Status: drafting\n\
    - Created: 2026-10-25T12:00:00+09:00\n\
    - Updated: 2026-10-25T12:00:00+09:00\n\n\
    ## Outcome\n\nKeep worktree tasks isolated.\n\n\
    ## Acceptance Criteria\n\n- G-AC1: Only the selected task changes.\n";

#[test]
fn linked_commands_isolate_goal_acceptance_and_resume_shared_tasks() {
    let (_sandbox, main, linked) = worktrees("linked-lifecycle").unwrap();
    let main_setup = waif(&main, &["init"]);
    let linked_setup = waif(
        &linked,
        &[
            "init",
            "--link",
            main.join(".waif/library").to_str().unwrap(),
        ],
    );
    let main_created = waif(&main, &["new", "root-goal"]);
    let linked_created = waif(&linked, &["new", "linked-goal"]);
    let root_pointer = fs::read_link(main.join(".waif/current_task")).unwrap();
    let linked_pointer = fs::read_link(linked.join(".waif/current_task")).unwrap();
    let root_goal = main.join(".waif").join(&root_pointer).join("goal.md");
    let linked_goal = linked.join(".waif").join(&linked_pointer).join("goal.md");
    let root_source = DRAFT_GOAL.replace("Worktree task", "Root task");
    fs::write(&root_goal, &root_source).unwrap();
    fs::write(&linked_goal, DRAFT_GOAL).unwrap();
    let root_branch = GitRepo::new(&main).branch_name().unwrap();
    let linked_branch = GitRepo::new(&linked).branch_name().unwrap();
    let root_index = GitRepo::new(&main).index_entries().unwrap();
    let linked_index = GitRepo::new(&linked).index_entries().unwrap();

    let root_inspect = waif(&main, &["inspect"]);
    let linked_inspect = waif(&linked, &["inspect"]);
    let root_check = waif(&main, &["check"]);
    let linked_check = waif(&linked, &["check"]);
    let accepted = waif(&linked, &["goal", "accept"]);

    assert!(main_setup.status.success());
    assert!(linked_setup.status.success());
    assert!(main_created.status.success());
    assert!(linked_created.status.success());
    assert!(root_inspect.status.success(), "{}", String::from_utf8_lossy(&root_inspect.stderr));
    assert!(linked_inspect.status.success(), "{}", String::from_utf8_lossy(&linked_inspect.stderr));
    assert!(String::from_utf8_lossy(&root_inspect.stdout).contains(root_pointer.to_str().unwrap()));
    assert!(String::from_utf8_lossy(&linked_inspect.stdout).contains(linked_pointer.to_str().unwrap()));
    assert!(root_check.status.success());
    assert!(linked_check.status.success());
    assert!(accepted.status.success(), "{}", String::from_utf8_lossy(&accepted.stderr));
    assert_eq!(fs::read_to_string(&root_goal).unwrap(), root_source);
    assert!(fs::read_to_string(&linked_goal).unwrap().contains("- Status: accepted"));
    assert_eq!(fs::read_link(main.join(".waif/current_task")).unwrap(), root_pointer);
    assert_eq!(fs::read_link(linked.join(".waif/current_task")).unwrap(), linked_pointer);

    let switched = waif(
        &main,
        &[
            "switch",
            linked_pointer.file_name().unwrap().to_str().unwrap(),
        ],
    );
    let resumed = waif(&main, &["inspect"]);
    let linked_switched = waif(
        &linked,
        &[
            "switch",
            root_pointer.file_name().unwrap().to_str().unwrap(),
        ],
    );
    let linked_resumed = waif(&linked, &["inspect"]);

    assert!(switched.status.success());
    assert!(resumed.status.success());
    assert!(String::from_utf8_lossy(&resumed.stdout).contains("  Status:           accepted"));
    assert!(linked_switched.status.success());
    assert!(linked_resumed.status.success());
    assert!(String::from_utf8_lossy(&linked_resumed.stdout).contains("  Status:           drafting"));
    assert_eq!(fs::read_link(main.join(".waif/current_task")).unwrap(), linked_pointer);
    assert_eq!(fs::read_link(linked.join(".waif/current_task")).unwrap(), root_pointer);
    assert_eq!(fs::read_to_string(&root_goal).unwrap(), root_source);
    assert!(!linked.join(".waif/.git").exists());
    assert!(GitRepo::new(&main.join(".waif")).is_ignored("current_task").unwrap());
    assert!(String::from_utf8_lossy(&GitRepo::new(&main.join(".waif")).working_tree_status().unwrap()).contains("library/"));
    assert_eq!(GitRepo::new(&main).branch_name().unwrap(), root_branch);
    assert_eq!(GitRepo::new(&linked).branch_name().unwrap(), linked_branch);
    assert_eq!(GitRepo::new(&main).index_entries().unwrap(), root_index);
    assert_eq!(GitRepo::new(&linked).index_entries().unwrap(), linked_index);
    assert!(GitRepo::new(&main).working_tree_status().unwrap().is_empty());
    assert!(GitRepo::new(&linked).working_tree_status().unwrap().is_empty());
}

#[test]
fn independent_worktree_isolates_records_and_loses_its_library_on_removal() {
    let (_sandbox, main, independent) = worktrees("independent-removal").unwrap();
    let root_setup = waif(&main, &["init"]);
    let private_setup = waif(&independent, &["init"]);
    let root_created = waif(&main, &["new", "root-record"]);
    let private_created = waif(&independent, &["new", "private-record"]);
    let root_pointer = fs::read_link(main.join(".waif/current_task")).unwrap();
    let private_pointer = fs::read_link(independent.join(".waif/current_task")).unwrap();
    let root_goal = main.join(".waif").join(&root_pointer).join("goal.md");
    let private_goal = independent
        .join(".waif")
        .join(&private_pointer)
        .join("goal.md");
    fs::write(&root_goal, DRAFT_GOAL).unwrap();
    fs::write(&private_goal, DRAFT_GOAL).unwrap();

    let accepted = waif(&independent, &["goal", "accept"]);
    let inspected = waif(&independent, &["inspect"]);

    assert!(root_setup.status.success());
    assert!(private_setup.status.success());
    assert!(root_created.status.success());
    assert!(private_created.status.success());
    assert!(accepted.status.success());
    assert!(inspected.status.success());
    assert!(independent.join(".waif/.git").is_dir());
    assert_eq!(fs::read_to_string(&root_goal).unwrap(), DRAFT_GOAL);
    assert_eq!(fs::read_link(main.join(".waif/current_task")).unwrap(), root_pointer);
    assert!(!main.join(".waif").join(&private_pointer).exists());
    assert!(!independent.join(".waif").join(&root_pointer).exists());
    assert!(GitRepo::new(&main).working_tree_status().unwrap().is_empty());
    assert!(GitRepo::new(&independent).working_tree_status().unwrap().is_empty());

    let removed = GitRepo::new(&main).remove_worktree(independent.to_str().unwrap());

    assert!(removed.is_ok(), "{removed:?}");
    assert!(!independent.exists());
    assert_eq!(fs::read_to_string(&root_goal).unwrap(), DRAFT_GOAL);
    assert_eq!(fs::read_link(main.join(".waif/current_task")).unwrap(), root_pointer);
}

#[test]
fn linked_worktree_removal_preserves_shared_records_for_resumption() {
    let (_sandbox, main, linked) = worktrees("linked-removal").unwrap();
    let root_setup = waif(&main, &["init"]);
    let linked_setup = waif(
        &linked,
        &[
            "init",
            "--link",
            main.join(".waif/library").to_str().unwrap(),
        ],
    );
    let created = waif(&linked, &["new", "preserved-record"]);
    let pointer = fs::read_link(linked.join(".waif/current_task")).unwrap();
    let goal = main.join(".waif").join(&pointer).join("goal.md");
    fs::write(&goal, DRAFT_GOAL).unwrap();

    let removed = GitRepo::new(&main).remove_worktree(linked.to_str().unwrap());
    let switched = waif(
        &main,
        &["switch", pointer.file_name().unwrap().to_str().unwrap()],
    );
    let inspected = waif(&main, &["inspect"]);
    let checked = waif(&main, &["check"]);

    assert!(root_setup.status.success());
    assert!(linked_setup.status.success());
    assert!(created.status.success());
    assert!(removed.is_ok(), "{removed:?}");
    assert!(!linked.exists());
    assert_eq!(fs::read_to_string(&goal).unwrap(), DRAFT_GOAL);
    assert!(switched.status.success());
    assert!(inspected.status.success());
    assert!(checked.status.success());
    assert_eq!(fs::read_link(main.join(".waif/current_task")).unwrap(), pointer);
}

#[test]
fn broken_library_dependency_fails_commands_without_replacing_state() {
    let (_sandbox, main, linked) = worktrees("broken-library").unwrap();
    let root_setup = waif(&main, &["init"]);
    let linked_setup = waif(
        &linked,
        &[
            "init",
            "--link",
            main.join(".waif/library").to_str().unwrap(),
        ],
    );
    let created = waif(&linked, &["new", "broken-dependency"]);
    let pointer = fs::read_link(linked.join(".waif/current_task")).unwrap();
    let library = fs::read_link(linked.join(".waif/library")).unwrap();
    fs::rename(main.join(".waif/library"), main.join(".waif/saved-library")).unwrap();

    let inspected = waif(&linked, &["inspect"]);
    let created_again = waif(&linked, &["new", "must-fail"]);
    let switched = waif(
        &linked,
        &["switch", pointer.file_name().unwrap().to_str().unwrap()],
    );
    let checked = waif(&linked, &["check"]);

    assert!(root_setup.status.success());
    assert!(linked_setup.status.success());
    assert!(created.status.success());
    assert!(!inspected.status.success());
    assert!(!created_again.status.success());
    assert!(!switched.status.success());
    assert!(!checked.status.success());
    assert!(String::from_utf8_lossy(&inspected.stderr).contains(".waif/library/tasks:"));
    assert_eq!(fs::read_link(linked.join(".waif/library")).unwrap(), library);
    assert_eq!(fs::read_link(linked.join(".waif/current_task")).unwrap(), pointer);
    assert!(!linked.join(".waif/.git").exists());
    assert_eq!(fs::read_dir(main.join(".waif/saved-library/tasks")).unwrap().count(), 1);
    assert!(!main.join(".waif/library").exists());
}

#[test]
fn explicit_paths_remain_caller_relative_without_workflow_setup() {
    let workspace = TestDir::new("explicit-paths");
    fs::create_dir(workspace.path.join("artifacts")).unwrap();
    let goal = workspace.path.join("artifacts/goal.md");
    fs::write(
        &goal,
        DRAFT_GOAL.replace("2026-10-25T12:00:00+09:00", "__NOW__"),
    )
    .unwrap();

    let incomplete = waif(&workspace.path, &["check", "artifacts/goal.md"]);
    let filled = waif(&workspace.path, &["fill", "artifacts/goal.md"]);
    let checked = waif(&workspace.path, &["check", "artifacts/goal.md"]);

    assert!(incomplete.status.success());
    assert!(String::from_utf8_lossy(&incomplete.stdout).contains("INCOMPLETE"));
    assert!(filled.status.success());
    assert!(checked.status.success());
    assert!(String::from_utf8_lossy(&checked.stdout).contains("GOOD"));
    assert!(!fs::read_to_string(&goal).unwrap().contains("__NOW__"));
    assert!(!workspace.path.join(".waif").exists());
}

fn worktrees(name: &str) -> io::Result<(TestDir, PathBuf, PathBuf)> {
    let sandbox = TestDir::new(name);
    let main = sandbox.path.join("main");
    let linked = sandbox.path.join("linked");
    fs::create_dir(&main)?;
    GitRepo::new(&main)
        .initialise_on_branch("main")
        .map_err(io::Error::other)?;
    GitRepo::new(&main)
        .exec(&[
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
        .map_err(io::Error::other)?;
    fs::write(main.join(".git/info/exclude"), "/.waif/\n")?;
    GitRepo::new(&main)
        .add_worktree("linked", linked.to_str().unwrap())
        .map_err(io::Error::other)?;
    Ok((sandbox, main, linked))
}

fn waif(workspace: &Path, arguments: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_waif"))
        .args(arguments)
        .current_dir(workspace)
        .output()
        .expect("waif should run")
}
