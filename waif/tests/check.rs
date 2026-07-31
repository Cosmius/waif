use std::fs;
#[cfg(unix)]
use std::os::unix::fs::symlink;
use std::process::{Command, Output};

mod test_support;

use test_support::TestDir;

const VALID_GOAL: &str = concat!(
    "# Goal\n",
    "- Status: accepted\n",
    "- Created: 2026-07-31T12:00:00Z\n",
    "- Updated: 2026-07-31T21:00:00+09:00\n",
    "## Outcome\n",
    "A checked outcome.\n",
    "## Acceptance Criteria\n",
    "- G-AC1: The outcome is checked.\n",
);

#[test]
fn check_reports_every_bad_artifact_before_exiting_one() {
    let directory = TestDir::new("all-errors");
    fs::write(directory.path.join("goal.md"), "not a title\n").expect("goal should be written");
    fs::write(directory.path.join("plan.md"), "also not a title\n")
        .expect("plan should be written");

    let output = run_check(&directory.path, &[directory.path.as_os_str()]);
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    assert_eq!(output.status.code(), Some(1));
    assert!(stderr.contains("goal.md:1: ERROR:"));
    assert!(stderr.contains("plan.md:1: ERROR:"));
    assert!(stdout.contains("checked 2 artifact(s); 2 invalid"));
}

#[test]
fn check_accepts_an_explicit_valid_file() {
    let directory = TestDir::new("valid-file");
    let artifact = directory.path.join("anything.md");
    fs::write(
        &artifact,
        "# Example\n\n- Status: proposed\n\n## Details\n\nProse.\n",
    )
    .expect("artifact should be written");

    let output = run_check(&directory.path, &[artifact.as_os_str()]);
    let stdout = String::from_utf8_lossy(&output.stdout);

    assert!(output.status.success());
    assert!(output.stderr.is_empty());
    assert!(stdout.contains("anything.md: valid"));
    assert!(stdout.contains("checked 1 artifact(s); 0 invalid"));
}

#[test]
fn check_accepts_an_explicit_valid_goal_with_optional_sections() {
    let directory = TestDir::new("valid-goal");
    let artifact = directory.path.join("goal.md");
    let source = concat!(
        "# Different title\n",
        "- Updated: 2026-07-31T21:00:00+09:00\n",
        "- Status: drafting\n",
        "- Created: 2026-07-31T12:00:00Z\n",
        "## In Scope\n",
        "- G-IN1: Direct scope.\n",
        "## Custom\n",
        "Opaque prose.\n",
        "## Acceptance Criteria\n",
        "- G-AC1: Checked.\n",
        "## Outcome\n",
        "A checked outcome.\n",
    );
    fs::write(&artifact, source).expect("goal should be written");

    let output = run_check(&directory.path, &[artifact.as_os_str()]);
    let stdout = String::from_utf8_lossy(&output.stdout);

    assert!(output.status.success());
    assert!(output.stderr.is_empty());
    assert!(stdout.contains("goal.md: valid"));
    assert!(stdout.contains("checked 1 artifact(s); 0 invalid"));
}

#[test]
fn check_reports_all_goal_errors_with_explicit_severity() {
    let directory = TestDir::new("goal-errors");
    let artifact = directory.path.join("goal.md");
    let source = concat!(
        "# Goal\n",
        "- Status: invalid\n",
        "- Extra: one\n",
        "- Extra: two\n",
        "## Outcome\n",
        "Text.\n",
        "## Acceptance Criteria\n",
        "- WRONG1: item\n",
        "## Outcome\n",
        "Duplicate.\n",
    );
    fs::write(&artifact, source).expect("goal should be written");

    let output = run_check(&directory.path, &[artifact.as_os_str()]);
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    assert_eq!(output.status.code(), Some(1));
    for expected in [
        "goal.md:2: ERROR: metadata `Status` must be",
        "goal.md:4: ERROR: duplicate metadata key `Extra`",
        "goal.md:1: ERROR: missing required metadata `Created`",
        "goal.md:1: ERROR: missing required metadata `Updated`",
        "goal.md:8: ERROR: item identifier `WRONG1`",
        "goal.md:9: ERROR: duplicate section `Outcome`",
    ] {
        assert!(stderr.contains(expected), "missing diagnostic: {expected}");
    }
    assert!(stdout.contains("checked 1 artifact(s); 1 invalid"));
}

#[test]
fn check_reports_warning_only_goals_as_valid() {
    let directory = TestDir::new("goal-warnings");
    let artifact = directory.path.join("goal.md");
    let source = concat!(
        "# Goal\n",
        "- Status: accepted\n",
        "- Created: 2026-07-31T12:00:00Z\n",
        "- Updated: 2026-07-31T21:00:00+09:00\n",
        "## Outcome\n",
        "## Acceptance Criteria\n",
    );
    fs::write(&artifact, source).expect("goal should be written");

    let output = run_check(&directory.path, &[artifact.as_os_str()]);
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    assert!(output.status.success());
    assert!(stderr.contains("goal.md:5: WARNING: section `Outcome` is empty"));
    assert!(stderr.contains("goal.md:6: WARNING: section `Acceptance Criteria` is empty"));
    assert!(stdout.contains("goal.md: valid"));
    assert!(stdout.contains("checked 1 artifact(s); 0 invalid"));
}

#[test]
fn check_continues_through_mixed_errors_and_warnings() {
    let directory = TestDir::new("mixed-findings");
    let warning_goal = concat!(
        "# Goal\n",
        "- Status: accepted\n",
        "- Created: 2026-07-31T12:00:00Z\n",
        "- Updated: 2026-07-31T21:00:00+09:00\n",
        "## Outcome\n",
        "## Acceptance Criteria\n",
        "- G-AC1: Checked.\n",
    );
    fs::write(directory.path.join("goal.md"), warning_goal).expect("goal should be written");
    fs::write(directory.path.join("plan.md"), "not a title\n").expect("plan should be written");

    let output = run_check(&directory.path, &[directory.path.as_os_str()]);
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    assert_eq!(output.status.code(), Some(1));
    assert!(stderr.contains("goal.md:5: WARNING:"));
    assert!(stderr.contains("plan.md:1: ERROR:"));
    assert!(stdout.contains("goal.md: valid"));
    assert!(stdout.contains("checked 2 artifact(s); 1 invalid"));
}

#[cfg(unix)]
#[test]
fn check_accepts_symlinked_artifacts_in_a_directory() {
    let directory = TestDir::new("symlinked-artifact");
    let target = directory.path.join("target.md");
    let intermediate = directory.path.join("intermediate.md");
    fs::write(&target, VALID_GOAL).expect("target should be written");
    symlink("target.md", &intermediate).expect("intermediate link should be created");
    symlink("intermediate.md", directory.path.join("goal.md"))
        .expect("artifact link should be created");

    let output = run_check(&directory.path, &[directory.path.as_os_str()]);
    let stdout = String::from_utf8_lossy(&output.stdout);

    assert!(output.status.success());
    assert!(stdout.contains("goal.md: valid"));
    assert!(stdout.contains("checked 1 artifact(s); 0 invalid"));
}

#[cfg(unix)]
#[test]
fn check_uses_the_selected_symlink_name_for_goal_dispatch() {
    let directory = TestDir::new("symlinked-goal-dispatch");
    let target = directory.path.join("target.md");
    let envelope_only = "# Example\n- Status: proposed\n## Details\nProse.\n";
    fs::write(&target, envelope_only).expect("target should be written");
    symlink("target.md", directory.path.join("goal.md")).expect("artifact link should be created");

    let output = run_check(&directory.path, &[directory.path.as_os_str()]);
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    assert_eq!(output.status.code(), Some(1));
    assert!(stderr.contains("goal.md:2: ERROR: metadata `Status` must be"));
    assert!(stdout.contains("checked 1 artifact(s); 1 invalid"));
}

#[cfg(unix)]
#[test]
fn check_rejects_symlinked_artifacts_targeting_directories() {
    let directory = TestDir::new("symlinked-directory");
    let target = directory.path.join("target");
    fs::create_dir(&target).expect("target directory should be created");
    symlink("target", directory.path.join("goal.md")).expect("artifact link should be created");

    let output = run_check(&directory.path, &[directory.path.as_os_str()]);
    let stderr = String::from_utf8_lossy(&output.stderr);

    assert_eq!(output.status.code(), Some(1));
    assert!(stderr.contains("goal.md"));
}

#[cfg(unix)]
#[test]
fn check_rejects_symlinked_artifacts_with_missing_targets() {
    let directory = TestDir::new("broken-symlink");
    symlink("missing.md", directory.path.join("goal.md")).expect("artifact link should be created");

    let output = run_check(&directory.path, &[directory.path.as_os_str()]);
    let stderr = String::from_utf8_lossy(&output.stderr);

    assert_eq!(output.status.code(), Some(1));
    assert!(stderr.contains("goal.md"));
}

#[cfg(unix)]
#[test]
fn check_uses_a_symlinked_steps_directory_nominally() {
    let directory = TestDir::new("symlinked-directory");
    let task = directory.path.join("task");
    let target = directory.path.join("step-target");
    let step = target.join("01-example");
    fs::create_dir_all(&task).expect("task directory should be created");
    fs::create_dir_all(&step).expect("target directory should be created");
    fs::write(step.join("step.md"), "# Example\n## Details\nProse.\n")
        .expect("step should be written");
    symlink("../step-target", task.join("steps")).expect("directory link should be created");

    let output = run_check(&task, &[task.as_os_str()]);
    let stdout = String::from_utf8_lossy(&output.stdout);

    assert!(output.status.success());
    assert!(stdout.contains("steps/01-example/step.md: valid"));
    assert!(stdout.contains("checked 1 artifact(s); 0 invalid"));
}

#[cfg(unix)]
#[test]
fn check_without_path_uses_the_current_task() {
    let workspace = TestDir::new("current-task");
    let tasks = workspace.path.join(".waif/tasks");
    let task = tasks.join("20260710-1-example");
    let step = task.join("steps/01-example");
    fs::create_dir_all(&step).expect("task should be created");
    symlink("./20260710-1-example", tasks.join("current"))
        .expect("current task link should be created");
    let warning_goal = concat!(
        "# Goal\n",
        "- Status: accepted\n",
        "- Created: 2026-07-31T12:00:00Z\n",
        "- Updated: 2026-07-31T21:00:00+09:00\n",
        "## Outcome\n",
        "## Acceptance Criteria\n",
        "- G-AC1: Checked.\n",
    );
    fs::write(task.join("goal.md"), warning_goal).expect("goal should be written");
    fs::write(step.join("review1.md"), "not a title\n").expect("review should be written");

    let output = run_check(&workspace.path, &[]);
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    assert_eq!(output.status.code(), Some(1));
    assert!(stderr.contains("goal.md:5: WARNING: section `Outcome` is empty"));
    assert!(stderr.contains("steps/01-example/review1.md:1: ERROR:"));
    assert!(stdout.contains("goal.md: valid"));
    assert!(stdout.contains("checked 2 artifact(s); 1 invalid"));
}

fn run_check(current_dir: &std::path::Path, arguments: &[&std::ffi::OsStr]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_waif"))
        .arg("check")
        .args(arguments)
        .current_dir(current_dir)
        .output()
        .expect("waif check should run")
}
