#![cfg(unix)]

use test_support::git_repo::GitRepo;

use std::fs;
use std::os::unix::fs::symlink;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

mod test_support;

use test_support::TestDir;

#[test]
fn init_creates_an_independent_library_and_workflow_repository() {
    let workspace = TestDir::new("init-independent");
    let workflow = workspace.path.join(".waif");

    let output = waif(&workspace.path, &["init"]);

    assert!(output.status.success(), "{}", String::from_utf8_lossy(&output.stderr));
    assert!(workflow.join("library/tasks").is_dir());
    assert!(!fs::symlink_metadata(workflow.join("library")).unwrap().is_symlink());
    assert!(workflow.join(".git").is_dir());
    assert_eq!(fs::read_to_string(workflow.join(".gitignore")).unwrap(), "/current_task\n");
    assert!(!workflow.join("current_task").exists());
    assert!(!workflow.join("library/charter.md").exists());
    assert_eq!(fs::read_dir(workflow.join("library/tasks")).unwrap().count(), 0);
    assert!(GitRepo::new(&workflow).is_ignored("current_task").unwrap());
}

#[test]
fn init_in_an_existing_instance_preserves_ignore_and_git_metadata() {
    let workspace = existing_instance("init-existing-instance");
    let workflow = workspace.path.join(".waif");

    let output = waif(&workspace.path, &["init"]);

    assert!(output.status.success(), "{}", String::from_utf8_lossy(&output.stderr));
    assert!(workflow.join("library/tasks").is_dir());
    assert_eq!(fs::read_to_string(workflow.join(".gitignore")).unwrap(), "keep-ignore\n");
    assert_eq!(fs::read_to_string(workflow.join(".git/sentinel")).unwrap(), "keep-git");
    assert_eq!(fs::read_dir(workflow.join(".git")).unwrap().count(), 1);
    assert_eq!(fs::read_link(workflow.join("current_task")).unwrap(), PathBuf::from("keep-selection"));
}

#[test]
fn init_reports_an_existing_directory_without_writes() {
    let workspace = existing_instance("init-directory");
    let workflow = workspace.path.join(".waif");
    fs::create_dir(workflow.join("library")).unwrap();
    fs::write(workflow.join("library/charter.md"), "keep-charter").unwrap();

    let output = waif(&workspace.path, &["init"]);

    assert!(output.status.success());
    assert!(String::from_utf8_lossy(&output.stdout).contains("Library already exists; setup skipped"));
    assert!(output.stderr.is_empty());
    assert!(!workflow.join("library/tasks").exists());
    assert_eq!(fs::read_to_string(workflow.join("library/charter.md")).unwrap(), "keep-charter");
    assert_eq!(fs::read_to_string(workflow.join(".gitignore")).unwrap(), "keep-ignore\n");
    assert_eq!(fs::read_dir(workflow.join(".git")).unwrap().count(), 1);
    assert_eq!(fs::read_link(workflow.join("current_task")).unwrap(), PathBuf::from("keep-selection"));
}

#[test]
fn linked_init_reports_an_existing_directory_without_checking_target() {
    let workspace = TestDir::new("linked-init-directory");
    fs::create_dir_all(workspace.path.join(".waif/library")).unwrap();

    let output = waif(&workspace.path, &["init", "--link", "missing-target"]);

    assert!(output.status.success());
    assert!(String::from_utf8_lossy(&output.stdout).contains("Library already exists; setup skipped"));
    assert_eq!(fs::read_dir(workspace.path.join(".waif/library")).unwrap().count(), 0);
    assert_eq!(fs::read_dir(workspace.path.join(".waif")).unwrap().count(), 1);
}

#[test]
fn init_reports_an_existing_file_without_writes() {
    let workspace = TestDir::new("init-file");
    fs::create_dir(workspace.path.join(".waif")).unwrap();
    fs::write(workspace.path.join(".waif/library"), "keep-file").unwrap();

    let output = waif(&workspace.path, &["init"]);

    assert!(output.status.success());
    assert!(String::from_utf8_lossy(&output.stdout).contains("Library already exists; setup skipped"));
    assert_eq!(fs::read_to_string(workspace.path.join(".waif/library")).unwrap(), "keep-file");
    assert_eq!(fs::read_dir(workspace.path.join(".waif")).unwrap().count(), 1);
}

#[test]
fn linked_init_reports_an_existing_file_without_checking_target() {
    let workspace = TestDir::new("linked-init-file");
    fs::create_dir(workspace.path.join(".waif")).unwrap();
    fs::write(workspace.path.join(".waif/library"), "keep-file").unwrap();

    let output = waif(&workspace.path, &["init", "--link", "missing-target"]);

    assert!(output.status.success());
    assert!(String::from_utf8_lossy(&output.stdout).contains("Library already exists; setup skipped"));
    assert_eq!(fs::read_to_string(workspace.path.join(".waif/library")).unwrap(), "keep-file");
    assert_eq!(fs::read_dir(workspace.path.join(".waif")).unwrap().count(), 1);
}

#[test]
fn init_reports_an_existing_library_link_without_writes() {
    let workspace = TestDir::new("init-link");
    fs::create_dir_all(workspace.path.join("root/tasks")).unwrap();
    fs::create_dir(workspace.path.join(".waif")).unwrap();
    symlink("../root", workspace.path.join(".waif/library")).unwrap();

    let output = waif(&workspace.path, &["init"]);

    assert!(output.status.success());
    assert!(String::from_utf8_lossy(&output.stdout).contains("Library already exists; setup skipped"));
    assert_eq!(fs::read_link(workspace.path.join(".waif/library")).unwrap(), PathBuf::from("../root"));
    assert_eq!(fs::read_dir(workspace.path.join("root")).unwrap().count(), 1);
    assert_eq!(fs::read_dir(workspace.path.join(".waif")).unwrap().count(), 1);
}

#[test]
fn linked_init_reports_an_existing_link_without_changing_libraries() {
    let workspace = TestDir::new("linked-init-link");
    fs::create_dir_all(workspace.path.join("root/tasks")).unwrap();
    fs::create_dir_all(workspace.path.join("other/tasks")).unwrap();
    fs::create_dir(workspace.path.join(".waif")).unwrap();
    symlink("../root", workspace.path.join(".waif/library")).unwrap();

    let output = waif(&workspace.path, &["init", "--link", "other"]);

    assert!(output.status.success());
    assert!(String::from_utf8_lossy(&output.stdout).contains("Library already exists; setup skipped"));
    assert_eq!(fs::read_link(workspace.path.join(".waif/library")).unwrap(), PathBuf::from("../root"));
    assert_eq!(fs::read_dir(workspace.path.join(".waif")).unwrap().count(), 1);
    assert_eq!(fs::read_dir(workspace.path.join("other")).unwrap().count(), 1);
}

#[test]
fn init_reports_a_dangling_library_link_without_writes() {
    let workspace = TestDir::new("init-dangling-link");
    fs::create_dir(workspace.path.join(".waif")).unwrap();
    symlink("../missing", workspace.path.join(".waif/library")).unwrap();

    let output = waif(&workspace.path, &["init"]);

    assert!(output.status.success());
    assert!(String::from_utf8_lossy(&output.stdout).contains("Library already exists; setup skipped"));
    assert_eq!(fs::read_link(workspace.path.join(".waif/library")).unwrap(), PathBuf::from("../missing"));
    assert_eq!(fs::read_dir(workspace.path.join(".waif")).unwrap().count(), 1);
    assert!(!workspace.path.join("missing").exists());
}

#[test]
fn linked_init_reports_a_dangling_link_without_checking_target() {
    let workspace = TestDir::new("linked-init-dangling-link");
    fs::create_dir(workspace.path.join(".waif")).unwrap();
    symlink("../missing", workspace.path.join(".waif/library")).unwrap();

    let output = waif(&workspace.path, &["init", "--link", "missing-target"]);

    assert!(output.status.success());
    assert!(String::from_utf8_lossy(&output.stdout).contains("Library already exists; setup skipped"));
    assert_eq!(fs::read_link(workspace.path.join(".waif/library")).unwrap(), PathBuf::from("../missing"));
    assert_eq!(fs::read_dir(workspace.path.join(".waif")).unwrap().count(), 1);
    assert!(!workspace.path.join("missing").exists());
}

#[test]
fn linked_init_rejects_a_missing_target() {
    let workspace = TestDir::new("linked-init-missing-target");

    let output = waif(&workspace.path, &["init", "--link", "missing-target"]);

    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("initialize or repair the root library"));
}

#[test]
fn linked_init_rejects_a_library_without_tasks() {
    let workspace = TestDir::new("linked-init-missing-tasks");
    fs::create_dir(workspace.path.join("target")).unwrap();

    let output = waif(&workspace.path, &["init", "--link", "target"]);

    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("initialize or repair the root library"));
}

#[test]
fn linked_init_canonicalizes_a_relative_target_through_a_symlink_chain() {
    let workspace = TestDir::new("linked-init-chain");
    fs::create_dir_all(workspace.path.join("root/tasks")).unwrap();
    symlink("root", workspace.path.join("first")).unwrap();
    symlink("first", workspace.path.join("second")).unwrap();

    let output = waif(&workspace.path, &["init", "--link", "second"]);

    assert!(output.status.success(), "{}", String::from_utf8_lossy(&output.stderr));
    assert_eq!(fs::read_link(workspace.path.join(".waif/library")).unwrap(), fs::canonicalize(workspace.path.join("root")).unwrap());
    assert_eq!(fs::read_dir(workspace.path.join(".waif")).unwrap().count(), 1);
    assert_eq!(fs::read_dir(workspace.path.join("root")).unwrap().count(), 1);
}

#[test]
fn ordinary_commands_allow_nested_library_links_after_setup() {
    let workspace = TestDir::new("ordinary-library-chain");
    fs::create_dir_all(workspace.path.join("root/tasks")).unwrap();
    let initialized = waif(&workspace.path, &["init", "--link", "root"]);
    assert!(initialized.status.success());
    symlink("root", workspace.path.join("first")).unwrap();
    fs::remove_file(workspace.path.join(".waif/library")).unwrap();
    symlink("../first", workspace.path.join(".waif/library")).unwrap();

    let created = waif(&workspace.path, &["new", "through-chain"]);
    let inspected = waif(&workspace.path, &["inspect"]);

    assert!(created.status.success(), "{}", String::from_utf8_lossy(&created.stderr));
    assert!(inspected.status.success(), "{}", String::from_utf8_lossy(&inspected.stderr));
    assert_eq!(fs::read_link(workspace.path.join(".waif/library")).unwrap(), PathBuf::from("../first"));
    assert_eq!(fs::read_dir(workspace.path.join("root/tasks")).unwrap().count(), 1);
    assert!(String::from_utf8_lossy(&inspected.stdout).contains(".waif/library/tasks/"));
}

#[test]
fn linked_worktree_setup_shares_records_and_keeps_source_contexts_unchanged() {
    let sandbox = TestDir::new("init-worktrees");
    let main = sandbox.path.join("main");
    let linked = sandbox.path.join("linked");
    fs::create_dir(&main).unwrap();
    assert!(GitRepo::new(&main).initialise_on_branch("main").is_ok());
    let committed = GitRepo::new(&main).exec(&[
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
    ]);
    assert!(committed.is_ok(), "{committed:?}");
    fs::write(main.join(".git/info/exclude"), "/.waif/\n").unwrap();
    let worktree = GitRepo::new(&main).add_worktree("linked", linked.to_str().unwrap());
    assert!(worktree.is_ok(), "{worktree:?}");
    let main_branch = GitRepo::new(&main).branch_name().unwrap();
    let linked_branch = GitRepo::new(&linked).branch_name().unwrap();
    let main_index = GitRepo::new(&main).index_entries().unwrap();
    let linked_index = GitRepo::new(&linked).index_entries().unwrap();

    let root_setup = waif(&main, &["init"]);
    let linked_setup = waif(
        &linked,
        &[
            "init",
            "--link",
            main.join(".waif/library").to_str().unwrap(),
        ],
    );
    let first = waif(&main, &["new", "root-task"]);
    let second = waif(&linked, &["new", "linked-task"]);
    let root_pointer = fs::read_link(main.join(".waif/current_task")).unwrap();
    let linked_pointer = fs::read_link(linked.join(".waif/current_task")).unwrap();
    fs::write(
        linked.join(".waif").join(&linked_pointer).join("goal.md"),
        "shared-goal",
    )
    .unwrap();

    assert!(root_setup.status.success(), "{}", String::from_utf8_lossy(&root_setup.stderr));
    assert!(linked_setup.status.success(), "{}", String::from_utf8_lossy(&linked_setup.stderr));
    assert!(first.status.success(), "{}", String::from_utf8_lossy(&first.stderr));
    assert!(second.status.success(), "{}", String::from_utf8_lossy(&second.stderr));
    assert_ne!(root_pointer, linked_pointer);
    assert_eq!(fs::read_link(linked.join(".waif/library")).unwrap(), fs::canonicalize(main.join(".waif/library")).unwrap());
    assert_eq!(fs::read_link(main.join(".waif/current_task")).unwrap(), root_pointer);
    assert!(linked.join(".waif").join(&root_pointer).is_dir());
    assert_eq!(fs::read_to_string(main.join(".waif").join(&linked_pointer).join("goal.md")).unwrap(), "shared-goal");
    assert!(!linked.join(".waif/.git").exists());
    assert!(!main.join(".waif/library/current_task").exists());
    assert!(GitRepo::new(&main.join(".waif")).is_ignored("current_task").unwrap());
    assert!(String::from_utf8_lossy(&GitRepo::new(&main.join(".waif")).working_tree_status().unwrap()).contains("library/"));
    assert_eq!(GitRepo::new(&main).branch_name().unwrap(), main_branch);
    assert_eq!(GitRepo::new(&linked).branch_name().unwrap(), linked_branch);
    assert_eq!(GitRepo::new(&main).index_entries().unwrap(), main_index);
    assert_eq!(GitRepo::new(&linked).index_entries().unwrap(), linked_index);
    assert!(GitRepo::new(&main).working_tree_status().unwrap().is_empty());
    assert!(GitRepo::new(&linked).working_tree_status().unwrap().is_empty());
}

fn existing_instance(name: &str) -> TestDir {
    let workspace = TestDir::new(name);
    let workflow = workspace.path.join(".waif");
    fs::create_dir_all(workflow.join(".git")).unwrap();
    fs::write(workflow.join(".git/sentinel"), "keep-git").unwrap();
    fs::write(workflow.join(".gitignore"), "keep-ignore\n").unwrap();
    symlink("keep-selection", workflow.join("current_task")).unwrap();
    workspace
}

fn waif(workspace: &Path, arguments: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_waif"))
        .args(arguments)
        .current_dir(workspace)
        .output()
        .expect("waif should run")
}
