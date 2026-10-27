use super::*;
use crate::test_support::TestDir;

const TIMESTAMP: &str = "2026-08-17T12:00:00+09:00";

#[test]
fn initial_acceptance_creates_branch_and_preserves_other_bytes() {
    let (workspace, task) = fixture("create", "drafting");
    let before = read_plan(&task);
    let goal = fs::read(task.path.join("goal.md")).unwrap();
    let base = GitRepo::new(&workspace.path).last_commit().unwrap();

    task.accept_plan(&workspace.path).unwrap();
    let after = read_plan(&task);

    assert_eq!(after, expected_accepted(&before, &after));
    assert_ne!(after, before);
    assert_eq!(GitRepo::new(&workspace.path).branch_name().unwrap(), "waif/task");
    assert_eq!(GitRepo::new(&workspace.path).last_commit().unwrap(), base);
    assert_eq!(fs::read(task.path.join("goal.md")).unwrap(), goal);
    assert_eq!(fs::read_dir(&task.path).unwrap().count(), 2);
}

#[test]
fn initial_acceptance_reuses_matching_existing_branch() {
    let (workspace, task) = fixture("reuse", "drafting");
    GitRepo::new(&workspace.path)
        .create_branch("waif/task")
        .unwrap();
    let inspection = task.inspect_plan(&workspace.path).unwrap().unwrap();
    let before = read_plan(&task);

    task.accept_plan(&workspace.path).unwrap();
    let after = read_plan(&task);

    assert!(inspection.ready_to_accept());
    assert_eq!(inspection.branch_commit(), inspection.base_commit());
    assert_eq!(after, expected_accepted(&before, &after));
    assert_eq!(GitRepo::new(&workspace.path).branch_name().unwrap(), "waif/task");
}

#[test]
fn initial_acceptance_already_on_matching_task_branch() {
    let (workspace, task) = fixture("same-branch", "drafting");
    GitRepo::new(&workspace.path)
        .create_and_switch_branch("waif/task", "HEAD")
        .unwrap();
    let inspection = task.inspect_plan(&workspace.path).unwrap().unwrap();
    let before = read_plan(&task);
    let refs = GitRepo::new(&workspace.path).references().unwrap();

    task.accept_plan(&workspace.path).unwrap();
    let after = read_plan(&task);

    assert!(inspection.ready_to_accept());
    assert_eq!(inspection.current_branch(), inspection.branch());
    assert_eq!(after, expected_accepted(&before, &after));
    assert_eq!(GitRepo::new(&workspace.path).references().unwrap(), refs);
}

#[test]
fn initial_acceptance_rejects_existing_branch_drift() {
    let (workspace, task) = fixture("branch-drift", "drafting");
    GitRepo::new(&workspace.path)
        .create_and_switch_branch("waif/task", "HEAD")
        .unwrap();
    commit(&workspace.path);
    GitRepo::new(&workspace.path).switch_branch("main").unwrap();
    let inspection = task.inspect_plan(&workspace.path).unwrap().unwrap();
    let before = read_plan(&task);
    let refs = GitRepo::new(&workspace.path).references().unwrap();

    let error = task.accept_plan(&workspace.path).unwrap_err().to_string();

    assert!(!inspection.ready_to_accept());
    assert!(error.contains("existing task branch differs"));
    assert_eq!(read_plan(&task), before);
    assert_eq!(GitRepo::new(&workspace.path).references().unwrap(), refs);
    assert_eq!(GitRepo::new(&workspace.path).branch_name().unwrap(), "main");
}

#[test]
fn initial_acceptance_from_another_branch_at_base() {
    let (workspace, task) = fixture("other-branch", "drafting");
    GitRepo::new(&workspace.path)
        .create_and_switch_branch("other", "HEAD")
        .unwrap();

    task.accept_plan(&workspace.path).unwrap();

    assert_eq!(GitRepo::new(&workspace.path).branch_name().unwrap(), "waif/task");
    assert!(read_plan(&task).contains("- Status: accepted"));
}

#[test]
fn initial_acceptance_from_detached_head_at_base() {
    let (workspace, task) = fixture("detached", "drafting");
    GitRepo::new(&workspace.path).detach_head("HEAD").unwrap();

    task.accept_plan(&workspace.path).unwrap();

    assert_eq!(GitRepo::new(&workspace.path).branch_name().unwrap(), "waif/task");
    assert!(read_plan(&task).contains("- Status: accepted"));
}

#[test]
fn initial_acceptance_rejects_dirty_tracked_files() {
    let (workspace, task) = fixture("dirty", "drafting");
    fs::write(workspace.path.join("source.txt"), "edit\n").unwrap();
    let before = read_plan(&task);
    let refs = GitRepo::new(&workspace.path).references().unwrap();

    let error = task.accept_plan(&workspace.path).unwrap_err().to_string();

    assert!(error.contains("staged or modified tracked files"));
    assert_eq!(read_plan(&task), before);
    assert_eq!(GitRepo::new(&workspace.path).references().unwrap(), refs);
    assert_eq!(fs::read_to_string(workspace.path.join("source.txt")).unwrap(), "edit\n");
}

#[test]
fn initial_acceptance_rejects_head_drift() {
    let (workspace, task) = fixture("head-drift", "drafting");
    GitRepo::new(&workspace.path)
        .create_and_switch_branch("other", "HEAD")
        .unwrap();
    commit(&workspace.path);
    let before = read_plan(&task);
    let refs = GitRepo::new(&workspace.path).references().unwrap();

    let error = task.accept_plan(&workspace.path).unwrap_err().to_string();

    assert!(error.contains("HEAD differs"));
    assert_eq!(read_plan(&task), before);
    assert_eq!(GitRepo::new(&workspace.path).references().unwrap(), refs);
}

#[test]
fn initial_acceptance_rejects_base_ref_drift() {
    let (workspace, task) = fixture("base-drift", "drafting");
    let base = GitRepo::new(&workspace.path).last_commit().unwrap();
    commit(&workspace.path);
    GitRepo::new(&workspace.path).detach_head(&base).unwrap();
    let before = read_plan(&task);
    let refs = GitRepo::new(&workspace.path).references().unwrap();

    let error = task.accept_plan(&workspace.path).unwrap_err().to_string();

    assert!(error.contains("Base branch is missing or differs"));
    assert_eq!(read_plan(&task), before);
    assert_eq!(GitRepo::new(&workspace.path).references().unwrap(), refs);
    assert_eq!(GitRepo::new(&workspace.path).last_commit().unwrap(), base);
}

#[test]
fn amendment_restores_branch_with_tracked_edit_and_historical_base_drift() {
    let (workspace, task) = fixture("amend", "amending");
    GitRepo::new(&workspace.path)
        .create_and_switch_branch("waif/task", "HEAD")
        .unwrap();
    commit(&workspace.path);
    let tip = GitRepo::new(&workspace.path).last_commit().unwrap();
    GitRepo::new(&workspace.path).switch_branch("main").unwrap();
    commit(&workspace.path);
    fs::write(workspace.path.join("source.txt"), "carried edit\n").unwrap();
    let before = read_plan(&task);

    task.accept_plan(&workspace.path).unwrap();
    let after = read_plan(&task);

    assert_eq!(after, expected_accepted(&before, &after));
    assert_eq!(GitRepo::new(&workspace.path).last_commit().unwrap(), tip);
    assert_eq!(GitRepo::new(&workspace.path).branch_name().unwrap(), "waif/task");
    assert_eq!(fs::read_to_string(workspace.path.join("source.txt")).unwrap(), "carried edit\n");
}

#[test]
fn amendment_rejects_missing_recorded_branch() {
    let (workspace, task) = fixture("missing-branch", "amending");
    let before = read_plan(&task);
    let refs = GitRepo::new(&workspace.path).references().unwrap();

    let error = task.accept_plan(&workspace.path).unwrap_err().to_string();

    assert!(error.contains("recorded task branch waif/task is missing"));
    assert_eq!(read_plan(&task), before);
    assert_eq!(GitRepo::new(&workspace.path).references().unwrap(), refs);
}

#[test]
fn accepted_plan_rejection_preserves_other_branch() {
    let (workspace, task) = fixture("accepted-other", "accepted");
    GitRepo::new(&workspace.path)
        .create_and_switch_branch("waif/task", "HEAD")
        .unwrap();
    commit(&workspace.path);
    GitRepo::new(&workspace.path).switch_branch("main").unwrap();
    let before = read_plan(&task);
    let head = GitRepo::new(&workspace.path).last_commit().unwrap();
    let refs = GitRepo::new(&workspace.path).references().unwrap();

    let error = task.accept_plan(&workspace.path).unwrap_err().to_string();

    assert!(error.contains("plan is already accepted"));
    assert_eq!(read_plan(&task), before);
    assert_eq!(GitRepo::new(&workspace.path).branch_name().unwrap(), "main");
    assert_eq!(GitRepo::new(&workspace.path).last_commit().unwrap(), head);
    assert_eq!(GitRepo::new(&workspace.path).references().unwrap(), refs);
}

#[test]
fn accepted_plan_rejection_preserves_task_branch_and_staging() {
    let (workspace, task) = fixture("accepted-staged", "accepted");
    GitRepo::new(&workspace.path)
        .create_and_switch_branch("waif/task", "HEAD")
        .unwrap();
    fs::write(workspace.path.join("source.txt"), "staged edit\n").unwrap();
    GitRepo::new(&workspace.path)
        .stage_file("source.txt")
        .unwrap();
    let before = read_plan(&task);
    let index = fs::read(workspace.path.join(".git/index")).unwrap();
    let head = GitRepo::new(&workspace.path).last_commit().unwrap();
    let refs = GitRepo::new(&workspace.path).references().unwrap();

    let error = task.accept_plan(&workspace.path).unwrap_err().to_string();

    assert!(error.contains("plan is already accepted"));
    assert_eq!(read_plan(&task), before);
    assert_eq!(fs::read(workspace.path.join(".git/index")).unwrap(), index);
    assert_eq!(GitRepo::new(&workspace.path).branch_name().unwrap(), "waif/task");
    assert_eq!(GitRepo::new(&workspace.path).last_commit().unwrap(), head);
    assert_eq!(GitRepo::new(&workspace.path).references().unwrap(), refs);
}

#[test]
fn occupied_branch_rejection_preserves_plan_and_current_branch() {
    let (workspace, task) = fixture("occupied", "drafting");
    let other = TestDir::new("acceptance-other-worktree");
    GitRepo::new(&workspace.path)
        .add_worktree("waif/task", other.path.to_str().unwrap())
        .unwrap();
    let before = read_plan(&task);

    let error = task.accept_plan(&workspace.path).unwrap_err().to_string();

    assert!(error.contains("git switch -- waif/task failed"));
    assert_eq!(read_plan(&task), before);
    assert_eq!(GitRepo::new(&workspace.path).branch_name().unwrap(), "main");
}

#[test]
fn untracked_collision_rejection_preserves_plan_and_current_branch() {
    let (workspace, task) = fixture("collision", "amending");
    GitRepo::new(&workspace.path)
        .create_and_switch_branch("waif/task", "HEAD")
        .unwrap();
    fs::write(workspace.path.join("collision.txt"), "tracked\n").unwrap();
    GitRepo::new(&workspace.path)
        .stage_file("collision.txt")
        .unwrap();
    commit(&workspace.path);
    GitRepo::new(&workspace.path).switch_branch("main").unwrap();
    fs::write(workspace.path.join("collision.txt"), "untracked\n").unwrap();
    let before = read_plan(&task);

    let error = task.accept_plan(&workspace.path).unwrap_err().to_string();

    assert!(error.contains("git switch -- waif/task failed"));
    assert_eq!(read_plan(&task), before);
    assert_eq!(GitRepo::new(&workspace.path).branch_name().unwrap(), "main");
    assert_eq!(fs::read_to_string(workspace.path.join("collision.txt")).unwrap(), "untracked\n");
}

#[test]
fn incomplete_plan_rejects_before_git_mutation() {
    let (workspace, task) = fixture("incomplete", "drafting");
    let before = read_plan(&task).replace("Approach", "__NOW__");
    fs::write(task.path.join("plan.md"), &before).unwrap();
    let refs = GitRepo::new(&workspace.path).references().unwrap();

    let error = task.accept_plan(&workspace.path).unwrap_err().to_string();

    assert!(error.contains("plan is INCOMPLETE"));
    assert_eq!(read_plan(&task), before);
    assert_eq!(GitRepo::new(&workspace.path).references().unwrap(), refs);
}

#[test]
fn nonaccepted_goal_rejects_before_git_mutation() {
    let (workspace, task) = fixture("goal-drafting", "drafting");
    let path = task.path.join("goal.md");
    let goal = fs::read_to_string(&path)
        .unwrap()
        .replace("accepted", "drafting");
    fs::write(&path, &goal).unwrap();
    let before = read_plan(&task);
    let refs = GitRepo::new(&workspace.path).references().unwrap();

    let error = task.accept_plan(&workspace.path).unwrap_err().to_string();

    assert!(error.contains("referenced goal is not accepted"));
    assert_eq!(read_plan(&task), before);
    assert_eq!(fs::read_to_string(path).unwrap(), goal);
    assert_eq!(GitRepo::new(&workspace.path).references().unwrap(), refs);
}

#[test]
fn replacement_failure_reports_completed_branch_setup() {
    let (workspace, task) = fixture("write-failure", "drafting");
    let before = read_plan(&task);

    let error = task
        .accept_plan_with(&workspace.path, |_, _| {
            Err(std::io::Error::other("injected write failure").into())
        })
        .unwrap_err()
        .to_string();

    assert!(error.contains("branch setup succeeded for waif/task"));
    assert!(error.contains("metadata update failed"));
    assert!(error.contains("injected write failure"));
    assert_eq!(read_plan(&task), before);
    assert_eq!(GitRepo::new(&workspace.path).branch_name().unwrap(), "waif/task");
}

#[test]
fn rename_failure_preserves_target_and_cleans_up_temporary() {
    let directory = TestDir::new("rename-failure");
    let target = directory.path.join("plan.md");
    fs::create_dir(&target).unwrap();
    fs::write(target.join("original"), "original bytes").unwrap();

    let result = replace_plan(&target, "replacement");

    assert!(result.is_err());
    assert_eq!(fs::read_to_string(target.join("original")).unwrap(), "original bytes");
    assert_eq!(fs::read_dir(&directory.path).unwrap().count(), 1);
}

fn fixture(name: &str, status: &str) -> (TestDir, Task) {
    let workspace = TestDir::new(name);
    GitRepo::new(&workspace.path)
        .initialise_on_branch("main")
        .unwrap();
    fs::write(workspace.path.join(".git/info/exclude"), "/.waif/\n").unwrap();
    fs::write(workspace.path.join("source.txt"), "source\n").unwrap();
    GitRepo::new(&workspace.path)
        .stage_file("source.txt")
        .unwrap();
    commit(&workspace.path);
    let base = GitRepo::new(&workspace.path).last_commit().unwrap();
    let path = workspace.path.join(".waif/library/tasks/20261026-1-test");
    fs::create_dir_all(&path).unwrap();
    let task = Task::new(path);
    fs::write(
        task.path.join("goal.md"),
        format!(
            "# Goal: Test\n\n- Status: accepted\n- Created: {TIMESTAMP}\n\
             - Updated: {TIMESTAMP}\n\n## Outcome\n\nTest\n\n\
             ## Acceptance Criteria\n\n- G-AC1: Test\n"
        ),
    )
    .unwrap();
    fs::write(
        task.path.join("plan.md"),
        format!(
            "# Plan: Test\n\n- Status: {status}\n- Goal: ./goal.md\n\
             - Branch: waif/task\n- Base branch: main\n- Base commit: {base}\n\
             - Created: {TIMESTAMP}\n- Updated: {TIMESTAMP}\n\n\
             ## Technical Summary\n\nApproach  \n\n\
             ## Decisions\n\n- P-D1: Decision\n\n\
             ## Current System\n\nContext\n\n\
             ## Plan Items\n\n### P1: First\n\n- Status: pending\n\
             - Goal criteria: opaque coverage\n- Depends on: opaque dependency\n\n\
             ### P2: Earlier work\n\n- Status: done\n\nProse  with spacing.\n\n\
             ## Risks\n\n- P-R1: Risk\n\n\
             ## Cross-Cutting Validation\n\n- P-V1: Check\n"
        ),
    )
    .unwrap();
    (workspace, task)
}

fn commit(workspace: &Path) {
    GitRepo::new(workspace)
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
        .unwrap();
}

fn read_plan(task: &Task) -> String {
    fs::read_to_string(task.path.join("plan.md")).unwrap()
}

fn expected_accepted(before: &str, after: &str) -> String {
    let updated = after
        .lines()
        .find_map(|line| line.strip_prefix("- Updated: "))
        .unwrap();
    before
        .replace("- Status: drafting", "- Status: accepted")
        .replace("- Status: amending", "- Status: accepted")
        .replace(
            &format!("- Updated: {TIMESTAMP}"),
            &format!("- Updated: {updated}"),
        )
}
