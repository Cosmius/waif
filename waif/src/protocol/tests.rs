use super::*;

use chrono::Local;

use std::fs;
use std::time::{SystemTime, UNIX_EPOCH};

#[cfg(unix)]
use std::os::unix::fs::symlink;

struct TestDir {
    path: PathBuf,
}

impl TestDir {
    fn new(name: &str) -> Self {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system clock should be after Unix epoch")
            .as_nanos();
        let path = std::env::temp_dir().join(format!("waif-test-{}-{unique}", sanitize_name(name)));
        fs::create_dir_all(&path).expect("test dir should be created");
        Self { path }
    }
}

impl Drop for TestDir {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.path);
    }
}

#[test]
fn workflow_dir_uses_workspace_waif_directory() {
    let workspace = PathBuf::from("/tmp/example-workspace");
    let workflow_dir = WorkflowDir::for_workspace(workspace);

    assert_eq!(
        workflow_dir.path,
        PathBuf::from("/tmp/example-workspace/.waif")
    );
    assert_eq!(
        workflow_dir.tasks_dir(),
        PathBuf::from("/tmp/example-workspace/.waif/tasks")
    );
    assert_eq!(
        workflow_dir.current_link(),
        PathBuf::from("/tmp/example-workspace/.waif/tasks/current")
    );
}

#[test]
fn current_task_is_none_when_current_link_is_missing() {
    let workspace = TestDir::new("missing-current-link");
    let workflow_dir = WorkflowDir::for_workspace(workspace.path.clone());

    assert!(workflow_dir
        .current_task()
        .expect("missing link should not fail")
        .is_none());
}

#[test]
fn initialise_creates_workflow_dir_and_tasks_dir() {
    let workspace = TestDir::new("initialise-workflow");
    let workflow_dir = WorkflowDir::for_workspace(workspace.path.clone());

    let git_error = workflow_dir
        .initialise()
        .expect("workflow should initialise");

    assert_eq!(git_error, None);
    assert!(workflow_dir.path.join(".git").is_dir());
    assert!(workflow_dir.tasks_dir().is_dir());
    assert!(workflow_dir
        .current_task()
        .expect("missing current should not fail")
        .is_none());
}

#[cfg(unix)]
#[test]
fn create_task_creates_task_without_switching_current() {
    let workspace = TestDir::new("create-first-task");
    let workflow_dir = WorkflowDir::for_workspace(workspace.path.clone());
    let date = Local::now().format("%Y%m%d").to_string();
    workflow_dir
        .initialise()
        .expect("workflow should initialise");

    let task = workflow_dir
        .create_task("add-session-timeout")
        .expect("task should be created");

    assert_eq!(
        task.path,
        workflow_dir
            .tasks_dir()
            .join(format!("{date}-1-add-session-timeout"))
    );
    assert!(!workflow_dir.current_link().exists());
}

#[test]
fn create_task_requires_workflow_dir() {
    let workspace = TestDir::new("missing-workflow");
    let workflow_dir = WorkflowDir::for_workspace(workspace.path.clone());

    let error = match workflow_dir.create_task("new-task") {
        Ok(_) => panic!("missing workflow dir should fail"),
        Err(error) => error,
    };

    assert_eq!(error.to_string(), "workflow dir is missing");
}

#[test]
fn create_task_requires_tasks_dir() {
    let workspace = TestDir::new("missing-tasks");
    let workflow_dir = WorkflowDir::for_workspace(workspace.path.clone());
    fs::create_dir_all(&workflow_dir.path).expect("workflow dir should be created");

    let error = match workflow_dir.create_task("new-task") {
        Ok(_) => panic!("missing tasks dir should fail"),
        Err(error) => error,
    };

    assert_eq!(error.to_string(), "workflow tasks dir is missing");
}

#[cfg(unix)]
#[test]
fn switch_task_updates_current_link() {
    let workspace = TestDir::new("switch-task");
    let workflow_dir = WorkflowDir::for_workspace(workspace.path.clone());
    let date = Local::now().format("%Y%m%d").to_string();
    workflow_dir
        .initialise()
        .expect("workflow should initialise");
    let task = workflow_dir
        .create_task("add-session-timeout")
        .expect("task should be created");

    workflow_dir
        .switch_task(&task)
        .expect("task should become current");

    assert_eq!(
        fs::read_link(workflow_dir.current_link()).expect("current link should exist"),
        PathBuf::from(format!("./{date}-1-add-session-timeout"))
    );

    let current_task = workflow_dir
        .current_task()
        .expect("current task should resolve")
        .expect("current task should exist");
    assert_eq!(
        current_task.path,
        task.path
            .canonicalize()
            .expect("task dir should canonicalize")
    );
}

#[cfg(unix)]
#[test]
fn create_task_increments_sequence_for_same_date() {
    let workspace = TestDir::new("create-next-task");
    let workflow_dir = WorkflowDir::for_workspace(workspace.path.clone());
    let date = Local::now().format("%Y%m%d").to_string();
    workflow_dir
        .initialise()
        .expect("workflow should initialise");
    fs::create_dir_all(
        workflow_dir
            .tasks_dir()
            .join(format!("{date}-1-existing-task")),
    )
    .expect("existing task should be created");

    let task = workflow_dir
        .create_task("follow-up-task")
        .expect("task should be created");

    assert_eq!(
        task.path,
        workflow_dir
            .tasks_dir()
            .join(format!("{date}-2-follow-up-task"))
    );
}

#[test]
fn create_task_rejects_invalid_slugs() {
    let workspace = TestDir::new("invalid-slug");
    let workflow_dir = WorkflowDir::for_workspace(workspace.path.clone());

    for slug in ["", "Bad-Slug", "bad_slug", "-bad", "bad-", "bad--slug"] {
        let error = match workflow_dir.create_task(slug) {
            Ok(_) => panic!("invalid slug should fail: {slug}"),
            Err(error) => error,
        };

        assert_eq!(
            error.to_string(),
            "task slug must be lowercase letters, numbers, and hyphens"
        );
    }
}

#[cfg(unix)]
#[test]
fn switch_task_rejects_current_file() {
    let workspace = TestDir::new("current-file");
    let workflow_dir = WorkflowDir::for_workspace(workspace.path.clone());
    workflow_dir
        .initialise()
        .expect("workflow should initialise");
    let task = workflow_dir
        .create_task("new-task")
        .expect("task should be created");
    fs::write(workflow_dir.current_link(), "not a symlink")
        .expect("current file should be written");

    let error = match workflow_dir.switch_task(&task) {
        Ok(_) => panic!("current file should fail"),
        Err(error) => error,
    };

    assert_eq!(error.to_string(), "current task is invalid");
}

#[cfg(unix)]
#[test]
fn switch_task_rejects_current_link_to_nested_task_dir() {
    let workspace = TestDir::new("current-nested-link");
    let workflow_dir = WorkflowDir::for_workspace(workspace.path.clone());
    workflow_dir
        .initialise()
        .expect("workflow should initialise");
    let task = workflow_dir
        .create_task("new-task")
        .expect("task should be created");
    let nested_task_dir = workflow_dir.tasks_dir().join("parent/nested");
    fs::create_dir_all(&nested_task_dir).expect("nested task dir should be created");
    symlink("parent/nested", workflow_dir.current_link()).expect("current link should be created");

    let error = match workflow_dir.switch_task(&task) {
        Ok(_) => panic!("nested current link should fail"),
        Err(error) => error,
    };

    assert_eq!(
        error.to_string(),
        "current task must resolve inside .waif/tasks"
    );
}

#[cfg(unix)]
#[test]
fn current_task_resolves_relative_link_inside_tasks_dir() {
    let workspace = TestDir::new("relative-current-link");
    let workflow_dir = WorkflowDir::for_workspace(workspace.path.clone());
    let tasks_dir = workflow_dir.tasks_dir();
    let task_dir = tasks_dir.join("implement-feature");
    fs::create_dir_all(&task_dir).expect("task dir should be created");
    symlink("implement-feature", workflow_dir.current_link())
        .expect("current link should be created");

    let task = workflow_dir
        .current_task()
        .expect("current task should resolve")
        .expect("current task should exist");

    assert_eq!(
        task.path,
        task_dir
            .canonicalize()
            .expect("task dir should canonicalize")
    );
    assert_eq!(
        task.name().expect("task name should be valid"),
        "implement-feature"
    );
}

#[cfg(unix)]
#[test]
fn current_task_rejects_links_that_escape_tasks_dir() {
    let workspace = TestDir::new("escaped-current-link");
    let workflow_dir = WorkflowDir::for_workspace(workspace.path.clone());
    fs::create_dir_all(workflow_dir.tasks_dir()).expect("tasks dir should be created");
    symlink("..", workflow_dir.current_link()).expect("current link should be created");

    let error = match workflow_dir.current_task() {
        Ok(_) => panic!("escaping link should fail"),
        Err(error) => error,
    };

    assert_eq!(
        error.to_string(),
        "current task must resolve inside .waif/tasks"
    );
}

#[cfg(unix)]
#[test]
fn current_task_rejects_links_to_nested_task_dirs() {
    let workspace = TestDir::new("nested-current-link");
    let workflow_dir = WorkflowDir::for_workspace(workspace.path.clone());
    let nested_task_dir = workflow_dir.tasks_dir().join("parent/nested");
    fs::create_dir_all(&nested_task_dir).expect("nested task dir should be created");
    symlink("parent/nested", workflow_dir.current_link()).expect("current link should be created");

    let error = match workflow_dir.current_task() {
        Ok(_) => panic!("nested link should fail"),
        Err(error) => error,
    };

    assert_eq!(
        error.to_string(),
        "current task must resolve inside .waif/tasks"
    );
}

#[test]
fn task_goal_is_none_when_goal_file_is_missing() {
    let workspace = TestDir::new("missing-goal");
    let task = Task::new(workspace.path.join(".waif/tasks/example-task"));

    assert!(task.goal().expect("missing goal should not fail").is_none());
}

#[test]
fn task_goal_reads_metadata_from_goal_file() {
    let workspace = TestDir::new("goal-metadata");
    let task = Task::new(workspace.path.join(".waif/tasks/example-task"));
    fs::create_dir_all(&task.path).expect("task dir should be created");
    fs::write(
        task.goal_path(),
        "\
- Status: drafting
- Created: 2026-07-04T09:00:00+09:00

## Goal

Create tests.
",
    )
    .expect("goal should be written");

    let goal = task
        .goal()
        .expect("goal should load")
        .expect("goal should exist");

    assert_eq!(goal.metadata_value("Status"), Some("drafting"));
    assert_eq!(
        goal.metadata_value("Created"),
        Some("2026-07-04T09:00:00+09:00")
    );
    assert_eq!(goal.metadata_value("Missing"), None);
}

#[test]
fn parse_metadata_stops_at_first_second_level_heading() {
    let metadata = parse_metadata(
        "\
- Status: done
- Updated: 2026-07-04T10:00:00+09:00
## Details
- Updated: 2026-07-04T11:00:00+09:00
",
    );

    assert_eq!(
        metadata,
        vec![
            ("Status".to_owned(), "done".to_owned()),
            ("Updated".to_owned(), "2026-07-04T10:00:00+09:00".to_owned()),
        ]
    );
}

#[test]
fn parse_metadata_line_trims_keys_and_values() {
    assert_eq!(
        parse_metadata_line("  - Status :  accepted  "),
        Some(("Status".to_owned(), "accepted".to_owned()))
    );
}

#[test]
fn parse_metadata_line_ignores_invalid_lines() {
    assert_eq!(parse_metadata_line("- : missing key"), None);
    assert_eq!(parse_metadata_line("Status: missing bullet"), None);
    assert_eq!(parse_metadata_line("- Missing separator"), None);
}

fn sanitize_name(name: &str) -> String {
    name.chars()
        .map(|character| {
            if character.is_ascii_alphanumeric() {
                character
            } else {
                '-'
            }
        })
        .collect()
}
