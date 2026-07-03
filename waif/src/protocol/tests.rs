use super::*;

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
