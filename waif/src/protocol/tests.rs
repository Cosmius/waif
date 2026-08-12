use super::*;

use chrono::Local;

use std::fs;
use std::path::PathBuf;

#[cfg(unix)]
use std::os::unix::fs::symlink;

use crate::test_support::TestDir;

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

    assert!(task
        .with_goal(|_| ())
        .expect("missing goal should not fail")
        .is_none());
}

#[test]
fn task_singleton_accessors_use_configured_schema_parsers() {
    let workspace = TestDir::new("goal-metadata");
    let task = Task::new(workspace.path.join(".waif/tasks/example-task"));
    fs::create_dir_all(&task.path).expect("task dir should be created");
    fs::write(
        task.path.join("goal.md"),
        "\
# Goal: Test the workflow
- Status: drafting
- Created: 2026-07-04T09:00:00+09:00
## Outcome
Create tests.
## Acceptance Criteria
- G-AC1: Tests exist.
",
    )
    .expect("goal should be written");
    fs::write(
        task.path.join("plan.md"),
        "\
# Plan
- Status: drafting
## Plan Items
### P1: First
- Status: pending
",
    )
    .expect("plan should be written");

    let status = task
        .with_goal(|goal| goal.metadata()[0].value().value().to_owned())
        .expect("goal should parse");
    let plan_items_are_structured = task
        .with_plan(|plan| plan.sections()[0].as_plan_items().is_some())
        .expect("plan should parse");

    assert_eq!(status.as_deref(), Some("drafting"));
    assert_eq!(plan_items_are_structured, Some(true));
}

#[test]
fn task_singleton_accessor_reports_path_aware_parser_errors() {
    let workspace = TestDir::new("invalid-goal");
    let task = Task::new(workspace.path.join(".waif/tasks/example-task"));
    fs::create_dir_all(&task.path).expect("task dir should be created");
    fs::write(
        task.path.join("goal.md"),
        "\
- Status: drafting
",
    )
    .expect("goal should be written");

    let error = task
        .with_goal(|_| ())
        .expect_err("invalid goal should fail when parsed");

    assert!(error
        .to_string()
        .contains("goal.md:1: first non-whitespace line must be a level-one"));
}

#[test]
fn artifact_file_classifies_nominal_paths_and_retains_contents() {
    let workspace = TestDir::new("artifact-kinds");
    for (name, expected) in [
        ("goal.md", ArtifactKind::Goal),
        ("plan.md", ArtifactKind::Plan),
        ("step.md", ArtifactKind::Step),
        ("review12.md", ArtifactKind::ImplementationReview),
        ("review.md", ArtifactKind::Generic),
        ("review-latest.md", ArtifactKind::Generic),
    ] {
        let path = workspace.path.join(name);
        fs::write(&path, "# Artifact\n").expect("artifact should be written");
        let file = ArtifactFile::read(path).expect("artifact should load");

        assert_eq!(file.kind(), expected, "wrong kind for {name}");
        assert_eq!(file.contents(), "# Artifact\n");
    }
}

#[test]
fn artifact_file_parsers_use_each_schema_structure() {
    let workspace = TestDir::new("artifact-parsers");
    let cases = [
        (
            "goal.md",
            "# Goal\n## Acceptance Criteria\n- G-AC1: One\n",
            "Acceptance Criteria",
        ),
        (
            "plan.md",
            "# Plan\n## Plan Items\n### P1: One\n- Status: pending\n",
            "Plan Items",
        ),
        (
            "step.md",
            "# Step 1\n## Done When\n- S1-D1: One\n",
            "Done When",
        ),
        (
            "review1.md",
            "# Implementation Review 1\n## Findings\nNo findings.\n",
            "Findings",
        ),
    ];

    for (name, source, section_name) in cases {
        let path = workspace.path.join(name);
        fs::write(&path, source).expect("artifact should be written");
        let file = ArtifactFile::read(path).expect("artifact should load");
        let artifact = match file.kind() {
            ArtifactKind::Goal => file.parse_goal(),
            ArtifactKind::Plan => file.parse_plan(),
            ArtifactKind::Step => file.parse_step(),
            ArtifactKind::ImplementationReview => file.parse_review(),
            ArtifactKind::Generic => unreachable!(),
        }
        .expect("artifact should parse");

        let section = artifact
            .sections()
            .iter()
            .find(|section| section.title() == section_name)
            .expect("configured section should exist");
        let configured = match file.kind() {
            ArtifactKind::Plan => section.as_plan_items().is_some(),
            ArtifactKind::ImplementationReview => section.as_findings().is_some(),
            _ => section.as_itemised().is_some(),
        };
        assert!(configured, "section should be configured for {name}");
    }
}

#[test]
fn generic_parse_and_diagnostics_keep_schema_validation_separate() {
    let workspace = TestDir::new("separate-parse-check");
    let goal_path = workspace.path.join("goal.md");
    fs::write(&goal_path, "# Goal\n- Status: invalid\n## Outcome\nText.\n")
        .expect("goal should be written");
    let goal = ArtifactFile::read(goal_path).expect("goal should load");

    assert!(goal.parse_goal().is_ok());
    assert!(goal.diagnostics().iter().any(|diagnostic| diagnostic
        .message()
        .contains("metadata `Status`")));

    let generic_path = workspace.path.join("review.md");
    fs::write(&generic_path, "# Review\n## Notes\nOpaque.\n").expect("review should be written");
    let generic = ArtifactFile::read(generic_path).expect("generic artifact should load");
    assert!(generic.parse_generic().is_ok());
    assert!(generic.diagnostics().is_empty());
}

#[test]
fn task_artifact_discovery_preserves_the_existing_layout() {
    let directory = TestDir::new("protocol-directory-discovery");
    let step_dir = directory.path.join("steps/01-example");
    let unrelated_dir = directory.path.join("other/deep");
    fs::create_dir_all(&step_dir).expect("step directory should be created");
    fs::create_dir_all(&unrelated_dir).expect("unrelated directory should be created");
    for path in [
        directory.path.join("goal.md"),
        directory.path.join("step.md"),
        directory.path.join("notes.md"),
        step_dir.join("step.md"),
        step_dir.join("goal.md"),
        step_dir.join("review1.md"),
        step_dir.join("review-latest.md"),
        unrelated_dir.join("goal.md"),
    ] {
        fs::write(path, "test").expect("test file should be written");
    }

    let paths = discover_artifact_paths(&directory.path).expect("discovery should succeed");
    let task = Task::new(directory.path.clone());

    assert_eq!(task.artifact_paths().expect("task discovery should work"), paths);
    assert_eq!(paths.len(), 3);
    assert!(paths.iter().any(|path| path == &directory.path.join("goal.md")));
    assert!(paths.iter().any(|path| path == &step_dir.join("step.md")));
    assert!(paths
        .iter()
        .any(|path| path == &step_dir.join("review1.md")));
}

#[test]
fn explicit_file_discovery_preserves_the_selected_nominal_path() {
    let path = PathBuf::from("missing/../selected/goal.md");

    assert_eq!(
        discover_artifact_paths(&path).expect("file discovery should work"),
        [path]
    );
}

#[test]
fn task_step_and_review_accessors_reject_wrong_or_outside_files() {
    let workspace = TestDir::new("task-artifact-access");
    let task = Task::new(workspace.path.join("task"));
    let step_dir = task.path.join("steps/01-example");
    fs::create_dir_all(&step_dir).expect("step directory should be created");
    let step_path = step_dir.join("step.md");
    let review_path = step_dir.join("review1.md");
    let outside_path = workspace.path.join("outside/step.md");
    fs::create_dir_all(outside_path.parent().expect("outside parent"))
        .expect("outside directory should be created");
    fs::write(&step_path, "# Step 1\n").expect("step should be written");
    fs::write(&review_path, "# Implementation Review 1\n").expect("review should be written");
    fs::write(&outside_path, "# Step 1\n").expect("outside step should be written");
    let step = ArtifactFile::read(step_path).expect("step should load");
    let review = ArtifactFile::read(review_path).expect("review should load");
    let outside = ArtifactFile::read(outside_path).expect("outside should load");

    assert_eq!(
        task.with_step(&step, |artifact| artifact.title().to_owned())
            .expect("step should parse"),
        "Step 1"
    );
    assert_eq!(
        task.with_review(&review, |artifact| artifact.title().to_owned())
            .expect("review should parse"),
        "Implementation Review 1"
    );
    assert_eq!(
        task.with_step(&review, |_| ())
            .expect_err("review should not pass as step")
            .to_string(),
        "artifact kind does not match task accessor"
    );
    assert_eq!(
        task.with_step(&outside, |_| ())
            .expect_err("outside step should fail")
            .to_string(),
        "artifact is outside the task directory"
    );
}
