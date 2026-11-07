use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::PathBuf;

use super::*;
use crate::git_repo::GitRepo;
use crate::test_support::TestDir;

const TIMESTAMP: &str = "2026-08-17T12:00:00+09:00";

#[test]
fn drafting_step_is_accepted_changing_only_lifecycle_metadata() {
    let (workspace, task) = fixture("drafting", "drafting");
    let path = step_path(&task, "01-test");
    let before = fs::read_to_string(&path).unwrap();
    let plan = fs::read(task.path.join("plan.md")).unwrap();

    task.accept_step(&workspace.path).unwrap();
    let after = fs::read_to_string(&path).unwrap();

    assert_eq!(after, expected_accepted(&before, &after));
    assert_ne!(after, before);
    assert!(chrono::DateTime::parse_from_rfc3339(updated(&after)).is_ok());
    assert_eq!(fs::read(task.path.join("plan.md")).unwrap(), plan);
    assert_eq!(fs::read_dir(path.parent().unwrap()).unwrap().count(), 1);
}

#[test]
fn amending_step_is_accepted() {
    let (workspace, task) = fixture("amending", "amending");
    let path = step_path(&task, "01-test");
    let before = fs::read_to_string(&path).unwrap();

    task.accept_step(&workspace.path).unwrap();
    let after = fs::read_to_string(&path).unwrap();

    assert_eq!(after, expected_accepted(&before, &after));
}

#[test]
fn accepted_step_is_rejected_as_already_accepted() {
    let (workspace, task) = fixture("accepted", "accepted");
    let before = read_step(&task);
    let plan = fs::read(task.path.join("plan.md")).unwrap();

    let error = task.accept_step(&workspace.path).unwrap_err().to_string();

    assert!(error.contains("step is already accepted"), "{error}");
    assert_eq!(read_step(&task), before);
    assert_eq!(fs::read(task.path.join("plan.md")).unwrap(), plan);
}

#[test]
fn greatest_numbered_step_is_accepted_over_earlier_drafting_step() {
    let (workspace, task) = fixture("greatest", "drafting");
    write_step(
        &task,
        "02-later",
        &step_source("drafting")
            .replace("S1-", "S2-")
            .replace("Step 1", "Step 2"),
    );
    let earlier = read_step(&task);

    task.accept_step(&workspace.path).unwrap();

    assert_eq!(read_step(&task), earlier);
    assert!(fs::read_to_string(step_path(&task, "02-later")).unwrap().contains("- Status: accepted"));
}

#[test]
fn missing_step_directory_is_rejected() {
    let (workspace, task) = fixture("no-steps", "drafting");
    fs::remove_dir_all(task.path.join("steps/01-test")).unwrap();
    let plan = fs::read(task.path.join("plan.md")).unwrap();

    let error = task.accept_step(&workspace.path).unwrap_err().to_string();

    assert!(error.contains("no step directory found"), "{error}");
    assert_eq!(fs::read(task.path.join("plan.md")).unwrap(), plan);
}

#[test]
fn wrong_current_branch_is_rejected() {
    let (workspace, task) = fixture("wrong-branch", "drafting");
    GitRepo::new(&workspace.path).switch_branch("main").unwrap();
    let before = read_step(&task);
    let plan = fs::read(task.path.join("plan.md")).unwrap();

    let error = task.accept_step(&workspace.path).unwrap_err().to_string();

    assert!(error.contains("current branch main is not the recorded task branch waif/task"), "{error}");
    assert_eq!(read_step(&task), before);
    assert_eq!(fs::read(task.path.join("plan.md")).unwrap(), plan);
}

#[test]
fn step_with_open_questions_is_rejected() {
    let (workspace, task) = fixture("open-question", "drafting");
    let source = format!(
        "{}\n## Questions\n\n### S1-Q1: Unsure\n- Status: open\n",
        read_step(&task)
    );
    write_step(&task, "01-test", &source);
    let plan = fs::read(task.path.join("plan.md")).unwrap();

    let error = task.accept_step(&workspace.path).unwrap_err().to_string();

    assert!(error.contains("step has open questions"), "{error}");
    assert_eq!(read_step(&task), source);
    assert_eq!(fs::read(task.path.join("plan.md")).unwrap(), plan);
}

#[test]
fn step_with_settled_questions_is_accepted() {
    let (workspace, task) = fixture("settled-question", "drafting");
    let source = format!(
        "{}\n## Questions\n\n### S1-Q1: Resolved\n- Status: settled\n",
        read_step(&task)
    );
    write_step(&task, "01-test", &source);

    task.accept_step(&workspace.path).unwrap();

    assert!(read_step(&task).contains("- Status: accepted\n"));
}

#[test]
fn step_with_compact_question_is_rejected() {
    let (workspace, task) = fixture("compact-question", "drafting");
    let source = format!("{}\n## Questions\n\n- S1-Q1: Unsure\n", read_step(&task));
    write_step(&task, "01-test", &source);
    let plan = fs::read(task.path.join("plan.md")).unwrap();

    let error = task.accept_step(&workspace.path).unwrap_err().to_string();

    assert!(error.contains("step is BAD"), "{error}");
    assert_eq!(read_step(&task), source);
    assert_eq!(fs::read(task.path.join("plan.md")).unwrap(), plan);
}

#[test]
fn unknown_plan_item_is_rejected() {
    let (workspace, task) = fixture("unknown-item", "drafting");
    let source = read_step(&task).replace(
        "### S1-PC1: Part\n- Plan item: P1",
        "### S1-PC9: Part\n- Plan item: P9",
    );
    write_step(&task, "01-test", &source);
    let plan = fs::read(task.path.join("plan.md")).unwrap();

    let error = task.accept_step(&workspace.path).unwrap_err().to_string();

    assert!(error.contains("plan item P9 does not exist"), "{error}");
    assert_eq!(read_step(&task), source);
    assert_eq!(fs::read(task.path.join("plan.md")).unwrap(), plan);
}

#[test]
fn done_plan_item_is_rejected() {
    let (workspace, task) = fixture("done-item", "drafting");
    let source = read_step(&task).replace(
        "### S1-PC1: Part\n- Plan item: P1",
        "### S1-PC2: Part\n- Plan item: P2",
    );
    write_step(&task, "01-test", &source);
    let plan = fs::read(task.path.join("plan.md")).unwrap();

    let error = task.accept_step(&workspace.path).unwrap_err().to_string();

    assert!(error.contains("plan item P2 is not pending"), "{error}");
    assert_eq!(read_step(&task), source);
    assert_eq!(fs::read(task.path.join("plan.md")).unwrap(), plan);
}

#[test]
fn duplicate_coverage_entry_is_rejected() {
    let (workspace, task) = fixture("duplicate-coverage", "drafting");
    let source = read_step(&task).replace(
        "### S1-PC1: Part\n- Plan item: P1\n- Coverage: partial\n",
        "### S1-PC1: Part\n- Plan item: P1\n- Coverage: partial\n\n\
         ### S1-PC1: Again\n- Plan item: P1\n- Coverage: partial\n",
    );
    write_step(&task, "01-test", &source);
    let plan = fs::read(task.path.join("plan.md")).unwrap();

    let error = task.accept_step(&workspace.path).unwrap_err().to_string();

    assert!(error.contains("step is BAD"), "{error}");
    assert_eq!(read_step(&task), source);
    assert_eq!(fs::read(task.path.join("plan.md")).unwrap(), plan);
}

#[test]
fn bad_step_is_rejected() {
    let (workspace, task) = fixture("bad-step", "drafting");
    let source = read_step(&task).replace("# Step 1: Test", "# Step 2: Test");
    write_step(&task, "01-test", &source);
    let plan = fs::read(task.path.join("plan.md")).unwrap();

    let error = task.accept_step(&workspace.path).unwrap_err().to_string();

    assert!(error.contains("step is BAD"), "{error}");
    assert_eq!(read_step(&task), source);
    assert_eq!(fs::read(task.path.join("plan.md")).unwrap(), plan);
}

#[test]
fn incomplete_step_is_rejected() {
    let (workspace, task) = fixture("incomplete-step", "drafting");
    let source = read_step(&task).replace(&format!("- Updated: {TIMESTAMP}"), "- Updated: __NOW__");
    write_step(&task, "01-test", &source);
    let plan = fs::read(task.path.join("plan.md")).unwrap();

    let error = task.accept_step(&workspace.path).unwrap_err().to_string();

    assert!(error.contains("step is INCOMPLETE"), "{error}");
    assert_eq!(read_step(&task), source);
    assert_eq!(fs::read(task.path.join("plan.md")).unwrap(), plan);
}

#[test]
fn drafting_goal_is_rejected() {
    let (workspace, task) = fixture("drafting-goal", "drafting");
    let goal_path = task.path.join("goal.md");
    let goal = fs::read_to_string(&goal_path)
        .unwrap()
        .replace("- Status: accepted", "- Status: drafting");
    fs::write(&goal_path, &goal).unwrap();
    let before = read_step(&task);
    let plan = fs::read(task.path.join("plan.md")).unwrap();

    let error = task.accept_step(&workspace.path).unwrap_err().to_string();

    assert!(error.contains("referenced goal is not accepted"), "{error}");
    assert_eq!(read_step(&task), before);
    assert_eq!(fs::read(task.path.join("plan.md")).unwrap(), plan);
    assert_eq!(fs::read_to_string(&goal_path).unwrap(), goal);
}

#[test]
fn drafting_plan_is_rejected() {
    let (workspace, task) = fixture("drafting-plan", "drafting");
    let plan_path = task.path.join("plan.md");
    let plan = fs::read_to_string(&plan_path)
        .unwrap()
        .replace("- Status: accepted", "- Status: drafting");
    fs::write(&plan_path, &plan).unwrap();
    let before = read_step(&task);

    let error = task.accept_step(&workspace.path).unwrap_err().to_string();

    assert!(error.contains("plan is not accepted"), "{error}");
    assert_eq!(read_step(&task), before);
    assert_eq!(fs::read_to_string(&plan_path).unwrap(), plan);
}

#[test]
fn bad_plan_is_rejected() {
    let (workspace, task) = fixture("bad-plan", "drafting");
    let plan_path = task.path.join("plan.md");
    let plan = fs::read_to_string(&plan_path)
        .unwrap()
        .replace("## Risks\n\n- P-R1: Risk\n\n", "");
    fs::write(&plan_path, &plan).unwrap();
    let before = read_step(&task);

    let error = task.accept_step(&workspace.path).unwrap_err().to_string();

    assert!(error.contains("plan is BAD"), "{error}");
    assert_eq!(read_step(&task), before);
    assert_eq!(fs::read_to_string(&plan_path).unwrap(), plan);
}

#[test]
fn tied_greatest_step_numbers_are_rejected() {
    let (workspace, task) = fixture("tied-steps", "drafting");
    write_step(&task, "01-other", &step_source("drafting"));
    let before = read_step(&task);
    let plan = fs::read(task.path.join("plan.md")).unwrap();

    let error = task.accept_step(&workspace.path).unwrap_err().to_string();

    assert!(error.contains("several step directories share the greatest number"), "{error}");
    assert_eq!(read_step(&task), before);
    assert_eq!(fs::read_to_string(step_path(&task, "01-other")).unwrap(), step_source("drafting"));
    assert_eq!(fs::read(task.path.join("plan.md")).unwrap(), plan);
}

#[test]
fn greatest_step_directory_without_step_file_is_rejected() {
    let (workspace, task) = fixture("missing-step-file", "drafting");
    fs::create_dir_all(task.path.join("steps/02-empty")).unwrap();
    let before = read_step(&task);
    let plan = fs::read(task.path.join("plan.md")).unwrap();

    let error = task.accept_step(&workspace.path).unwrap_err().to_string();

    assert!(error.contains("step file is missing"), "{error}");
    assert_eq!(read_step(&task), before);
    assert_eq!(fs::read(task.path.join("plan.md")).unwrap(), plan);
}

#[test]
fn done_step_is_rejected() {
    let (workspace, task) = fixture("done-step", "done");
    let source = read_step(&task).replace(
        "- Source commit: not-created",
        "- Source commit: 0123456789abcdef0123456789abcdef01234567",
    );
    write_step(&task, "01-test", &source);
    let plan = fs::read(task.path.join("plan.md")).unwrap();

    let error = task.accept_step(&workspace.path).unwrap_err().to_string();

    assert!(error.contains("step is already done"), "{error}");
    assert_eq!(read_step(&task), source);
    assert_eq!(fs::read(task.path.join("plan.md")).unwrap(), plan);
}

#[test]
fn missing_plan_is_rejected() {
    let (workspace, task) = fixture("missing-plan", "drafting");
    fs::remove_file(task.path.join("plan.md")).unwrap();
    let before = read_step(&task);

    let error = task.accept_step(&workspace.path).unwrap_err().to_string();

    assert!(error.contains("current task has no plan.md"), "{error}");
    assert_eq!(read_step(&task), before);
}

#[test]
fn incomplete_plan_is_rejected() {
    let (workspace, task) = fixture("incomplete-plan", "drafting");
    let plan_path = task.path.join("plan.md");
    let plan = fs::read_to_string(&plan_path).unwrap().replacen(
        &format!("- Updated: {TIMESTAMP}"),
        "- Updated: __NOW__",
        1,
    );
    fs::write(&plan_path, &plan).unwrap();
    let before = read_step(&task);

    let error = task.accept_step(&workspace.path).unwrap_err().to_string();

    assert!(error.contains("plan is INCOMPLETE"), "{error}");
    assert_eq!(read_step(&task), before);
    assert_eq!(fs::read_to_string(&plan_path).unwrap(), plan);
}

#[test]
fn plan_with_open_questions_is_rejected() {
    let (workspace, task) = fixture("plan-open-question", "drafting");
    let plan_path = task.path.join("plan.md");
    let plan = fs::read_to_string(&plan_path).unwrap().replace(
        "## Current System",
        "## Questions\n\n### P-Q1: Unsure\n- Status: open\n\n## Current System",
    );
    fs::write(&plan_path, &plan).unwrap();
    let before = read_step(&task);

    let error = task.accept_step(&workspace.path).unwrap_err().to_string();

    assert!(error.contains("plan has open questions"), "{error}");
    assert_eq!(read_step(&task), before);
    assert_eq!(fs::read_to_string(&plan_path).unwrap(), plan);
}

#[test]
fn plan_with_settled_questions_allows_step_acceptance() {
    let (workspace, task) = fixture("plan-settled-question", "drafting");
    let plan_path = task.path.join("plan.md");
    let plan = fs::read_to_string(&plan_path).unwrap().replace(
        "## Current System",
        "## Questions\n\n### P-Q1: Resolved\n- Status: settled\n\n## Current System",
    );
    fs::write(&plan_path, &plan).unwrap();

    task.accept_step(&workspace.path).unwrap();

    assert!(read_step(&task).contains("- Status: accepted\n"));
    assert_eq!(fs::read_to_string(&plan_path).unwrap(), plan);
}

#[test]
fn plan_with_compact_question_is_rejected() {
    let (workspace, task) = fixture("plan-compact-question", "drafting");
    let plan_path = task.path.join("plan.md");
    let plan = fs::read_to_string(&plan_path).unwrap().replace(
        "## Current System",
        "## Questions\n\n- P-Q1: Unsure\n\n## Current System",
    );
    fs::write(&plan_path, &plan).unwrap();
    let before = read_step(&task);

    let error = task.accept_step(&workspace.path).unwrap_err().to_string();

    assert!(error.contains("plan is BAD"), "{error}");
    assert_eq!(read_step(&task), before);
    assert_eq!(fs::read_to_string(&plan_path).unwrap(), plan);
}

#[test]
fn missing_goal_is_rejected() {
    let (workspace, task) = fixture("missing-goal", "drafting");
    fs::remove_file(task.path.join("goal.md")).unwrap();
    let before = read_step(&task);
    let plan = fs::read(task.path.join("plan.md")).unwrap();

    let error = task.accept_step(&workspace.path).unwrap_err().to_string();

    assert!(error.contains("referenced goal is missing"), "{error}");
    assert_eq!(read_step(&task), before);
    assert_eq!(fs::read(task.path.join("plan.md")).unwrap(), plan);
}

#[test]
fn incomplete_goal_is_rejected() {
    let (workspace, task) = fixture("incomplete-goal", "drafting");
    let goal_path = task.path.join("goal.md");
    let goal = fs::read_to_string(&goal_path).unwrap().replacen(
        &format!("- Updated: {TIMESTAMP}"),
        "- Updated: __NOW__",
        1,
    );
    fs::write(&goal_path, &goal).unwrap();
    let before = read_step(&task);
    let plan = fs::read(task.path.join("plan.md")).unwrap();

    let error = task.accept_step(&workspace.path).unwrap_err().to_string();

    assert!(error.contains("referenced goal is not GOOD"), "{error}");
    assert_eq!(read_step(&task), before);
    assert_eq!(fs::read(task.path.join("plan.md")).unwrap(), plan);
    assert_eq!(fs::read_to_string(&goal_path).unwrap(), goal);
}

#[test]
fn detached_head_is_rejected() {
    let (workspace, task) = fixture("detached-head", "drafting");
    GitRepo::new(&workspace.path).detach_head("HEAD").unwrap();
    let before = read_step(&task);
    let plan = fs::read(task.path.join("plan.md")).unwrap();

    let error = task.accept_step(&workspace.path).unwrap_err().to_string();

    assert!(error.contains("current branch (detached HEAD) is not the recorded task branch waif/task"), "{error}");
    assert_eq!(read_step(&task), before);
    assert_eq!(fs::read(task.path.join("plan.md")).unwrap(), plan);
}

#[test]
fn missing_steps_directory_is_rejected() {
    let (workspace, task) = fixture("no-steps-dir", "drafting");
    fs::remove_dir_all(task.path.join("steps")).unwrap();
    let plan = fs::read(task.path.join("plan.md")).unwrap();

    let error = task.accept_step(&workspace.path).unwrap_err().to_string();

    assert!(error.contains(&task.path.join("steps").display().to_string()), "{error}");
    assert_eq!(fs::read(task.path.join("plan.md")).unwrap(), plan);
}

#[test]
fn committing_complete_step_records_commit_and_marks_plan_item_done() {
    let (workspace, task) = fixture("commit-complete", "accepted");
    let source = read_step(&task).replace("- Coverage: partial", "- Coverage: complete");
    write_step(&task, "01-test", &source);
    let repo = GitRepo::new(&workspace.path);
    let base = repo.last_commit().unwrap();
    stage(&workspace, "a.txt", "staged\n");
    fs::write(workspace.path.join("a.txt"), "unstaged\n").unwrap();
    let plan = fs::read_to_string(task.path.join("plan.md")).unwrap();

    let commit = task.commit_step(&workspace.path, "Add a").unwrap();
    let step = read_step(&task);
    let plan_after = fs::read_to_string(task.path.join("plan.md")).unwrap();
    let expected_step = source
        .replace("- Status: accepted", "- Status: done")
        .replace("not-created", &commit.hash)
        .replace(
            &format!("- Updated: {TIMESTAMP}"),
            &format!("- Updated: {}", updated(&step)),
        );
    let expected_plan = plan
        .replace(
            "### P1: First\n\n- Status: pending",
            "### P1: First\n\n- Status: done",
        )
        .replace(
            &format!("- Updated: {TIMESTAMP}"),
            &format!("- Updated: {}", updated(&plan_after)),
        );

    assert_eq!(commit.hash, repo.last_commit().unwrap());
    assert_eq!(commit.done_items, ["P1"]);
    assert_eq!(git(&workspace, &["rev-parse", "HEAD~1"]), base);
    assert_eq!(git(&workspace, &["log", "-1", "--format=%s"]), "Add a");
    assert_eq!(git(&workspace, &["show", "HEAD:a.txt"]), "staged");
    assert_eq!(fs::read_to_string(workspace.path.join("a.txt")).unwrap(), "unstaged\n");
    assert_eq!(step, expected_step);
    assert_eq!(plan_after, expected_plan);
}

#[test]
fn committing_partial_step_leaves_plan_unchanged() {
    let (workspace, task) = fixture("commit-partial", "accepted");
    stage(&workspace, "a.txt", "staged\n");
    let plan = fs::read(task.path.join("plan.md")).unwrap();

    let commit = task.commit_step(&workspace.path, "Add a").unwrap();

    assert!(commit.done_items.is_empty());
    assert!(read_step(&task).contains(&format!("- Source commit: {}\n", commit.hash)));
    assert!(read_step(&task).contains("- Status: done\n"));
    assert_eq!(fs::read(task.path.join("plan.md")).unwrap(), plan);
}

#[test]
fn committing_drafting_step_is_rejected() {
    let (workspace, task) = fixture("commit-drafting", "drafting");
    stage(&workspace, "a.txt", "staged\n");
    let head = GitRepo::new(&workspace.path).last_commit().unwrap();
    let before = read_step(&task);
    let plan = fs::read(task.path.join("plan.md")).unwrap();

    let error = task
        .commit_step(&workspace.path, "Add a")
        .unwrap_err()
        .to_string();

    assert!(error.contains("step is not accepted"), "{error}");
    assert_eq!(GitRepo::new(&workspace.path).last_commit().unwrap(), head);
    assert_eq!(read_step(&task), before);
    assert_eq!(fs::read(task.path.join("plan.md")).unwrap(), plan);
}

#[test]
fn committing_done_step_is_rejected() {
    let (workspace, task) = fixture("commit-done", "done");
    let source = read_step(&task).replace(
        "- Source commit: not-created",
        "- Source commit: 0123456789abcdef0123456789abcdef01234567",
    );
    write_step(&task, "01-test", &source);
    stage(&workspace, "a.txt", "staged\n");
    let head = GitRepo::new(&workspace.path).last_commit().unwrap();
    let plan = fs::read(task.path.join("plan.md")).unwrap();

    let error = task
        .commit_step(&workspace.path, "Add a")
        .unwrap_err()
        .to_string();

    assert!(error.contains("step is already done"), "{error}");
    assert_eq!(GitRepo::new(&workspace.path).last_commit().unwrap(), head);
    assert_eq!(read_step(&task), source);
    assert_eq!(fs::read(task.path.join("plan.md")).unwrap(), plan);
}

#[test]
fn committing_on_wrong_branch_is_rejected() {
    let (workspace, task) = fixture("commit-wrong-branch", "accepted");
    let repo = GitRepo::new(&workspace.path);
    repo.switch_branch("main").unwrap();
    stage(&workspace, "a.txt", "staged\n");
    let head = repo.last_commit().unwrap();
    let before = read_step(&task);
    let plan = fs::read(task.path.join("plan.md")).unwrap();

    let error = task
        .commit_step(&workspace.path, "Add a")
        .unwrap_err()
        .to_string();

    assert!(error.contains("current branch main is not the recorded task branch waif/task"), "{error}");
    assert_eq!(repo.last_commit().unwrap(), head);
    assert_eq!(read_step(&task), before);
    assert_eq!(fs::read(task.path.join("plan.md")).unwrap(), plan);
}

#[test]
fn blank_commit_message_is_rejected() {
    let (workspace, task) = fixture("commit-blank", "accepted");
    stage(&workspace, "a.txt", "staged\n");
    let head = GitRepo::new(&workspace.path).last_commit().unwrap();
    let before = read_step(&task);
    let plan = fs::read(task.path.join("plan.md")).unwrap();

    let error = task
        .commit_step(&workspace.path, " \n\t")
        .unwrap_err()
        .to_string();

    assert!(error.contains("commit message is empty"), "{error}");
    assert_eq!(GitRepo::new(&workspace.path).last_commit().unwrap(), head);
    assert_eq!(read_step(&task), before);
    assert_eq!(fs::read(task.path.join("plan.md")).unwrap(), plan);
}

#[test]
fn commit_without_staged_changes_is_rejected() {
    let (workspace, task) = fixture("commit-nothing", "accepted");
    fs::write(workspace.path.join("a.txt"), "unstaged\n").unwrap();
    let head = GitRepo::new(&workspace.path).last_commit().unwrap();
    let before = read_step(&task);
    let plan = fs::read(task.path.join("plan.md")).unwrap();

    let error = task
        .commit_step(&workspace.path, "Add a")
        .unwrap_err()
        .to_string();

    assert!(error.contains("no staged changes to commit"), "{error}");
    assert_eq!(GitRepo::new(&workspace.path).last_commit().unwrap(), head);
    assert_eq!(read_step(&task), before);
    assert_eq!(fs::read(task.path.join("plan.md")).unwrap(), plan);
}

#[test]
fn commit_with_unmerged_entries_is_rejected() {
    let (workspace, task) = fixture("commit-unmerged", "accepted");
    let repo = GitRepo::new(&workspace.path);
    repo.create_and_switch_branch("other", "HEAD").unwrap();
    stage(&workspace, "a.txt", "theirs\n");
    repo.exec(&["commit", "-m", "theirs"]).unwrap();
    repo.switch_branch("waif/task").unwrap();
    stage(&workspace, "a.txt", "ours\n");
    repo.exec(&["commit", "-m", "ours"]).unwrap();
    assert!(repo.exec(&["merge", "other"]).is_err());
    let head = repo.last_commit().unwrap();
    let before = read_step(&task);
    let plan = fs::read(task.path.join("plan.md")).unwrap();

    let error = task
        .commit_step(&workspace.path, "Merge")
        .unwrap_err()
        .to_string();

    assert!(error.contains("the index has unmerged entries"), "{error}");
    assert_eq!(repo.last_commit().unwrap(), head);
    assert_eq!(read_step(&task), before);
    assert_eq!(fs::read(task.path.join("plan.md")).unwrap(), plan);
}

#[test]
fn commit_refused_by_hook_is_rejected() {
    let (workspace, task) = fixture("commit-hook", "accepted");
    install_hook(&workspace, "exit 1");
    stage(&workspace, "a.txt", "staged\n");
    let head = GitRepo::new(&workspace.path).last_commit().unwrap();
    let before = read_step(&task);
    let plan = fs::read(task.path.join("plan.md")).unwrap();

    let error = task
        .commit_step(&workspace.path, "Add a")
        .unwrap_err()
        .to_string();

    assert!(error.contains("git commit -m Add a failed"), "{error}");
    assert_eq!(GitRepo::new(&workspace.path).last_commit().unwrap(), head);
    assert_eq!(read_step(&task), before);
    assert_eq!(fs::read(task.path.join("plan.md")).unwrap(), plan);
}

#[test]
fn step_write_failure_after_commit_reports_hash_and_step() {
    let (workspace, task) = fixture("commit-step-unwritable", "accepted");
    let step_dir = task.path.join("steps/01-test");
    install_hook(&workspace, &format!("chmod a-w '{}'", step_dir.display()));
    stage(&workspace, "a.txt", "staged\n");
    let repo = GitRepo::new(&workspace.path);
    let base = repo.last_commit().unwrap();
    let before = read_step(&task);
    let plan = fs::read(task.path.join("plan.md")).unwrap();

    let error = task
        .commit_step(&workspace.path, "Add a")
        .unwrap_err()
        .to_string();
    fs::set_permissions(&step_dir, fs::Permissions::from_mode(0o755)).unwrap();
    let hash = repo.last_commit().unwrap();

    assert_ne!(hash, base);
    assert!(error.contains(&format!("commit {hash} created but {} was not updated", step_path(&task, "01-test").display())), "{error}");
    assert_eq!(read_step(&task), before);
    assert_eq!(fs::read(task.path.join("plan.md")).unwrap(), plan);
}

#[test]
fn plan_write_failure_after_commit_reports_hash_and_plan() {
    let (workspace, task) = fixture("commit-plan-unwritable", "accepted");
    let source = read_step(&task).replace("- Coverage: partial", "- Coverage: complete");
    write_step(&task, "01-test", &source);
    install_hook(&workspace, &format!("chmod a-w '{}'", task.path.display()));
    stage(&workspace, "a.txt", "staged\n");
    let repo = GitRepo::new(&workspace.path);
    let plan = fs::read(task.path.join("plan.md")).unwrap();

    let error = task
        .commit_step(&workspace.path, "Add a")
        .unwrap_err()
        .to_string();
    fs::set_permissions(&task.path, fs::Permissions::from_mode(0o755)).unwrap();
    let hash = repo.last_commit().unwrap();

    assert!(error.contains(&format!("commit {hash} created but {} was not updated", task.path.join("plan.md").display())), "{error}");
    assert!(read_step(&task).contains(&format!("- Source commit: {hash}\n")));
    assert!(read_step(&task).contains("- Status: done\n"));
    assert_eq!(fs::read(task.path.join("plan.md")).unwrap(), plan);
}

fn fixture(name: &str, status: &str) -> (TestDir, Task) {
    let workspace = TestDir::new(name);
    let repo = GitRepo::new(&workspace.path);
    repo.initialise_on_branch("main").unwrap();
    fs::write(workspace.path.join(".git/info/exclude"), "/.waif/\n").unwrap();
    repo.exec(&["config", "user.name", "Test"]).unwrap();
    repo.exec(&["config", "user.email", "test@example.invalid"])
        .unwrap();
    repo.exec(&["config", "commit.gpgsign", "false"]).unwrap();
    repo.exec(&["commit", "--allow-empty", "-m", "fixture"])
        .unwrap();
    let base = repo.last_commit().unwrap();
    repo.create_and_switch_branch("waif/task", "HEAD").unwrap();

    let path = workspace.path.join(".waif/library/tasks/20261031-1-test");
    fs::create_dir_all(&path).unwrap();
    let task = Task::new(path);
    fs::write(
        task.path.join("goal.md"),
        format!(
            "# Goal: Test\n\n- Status: accepted\n- Created: {TIMESTAMP}\n\
             - Updated: {TIMESTAMP}\n\n## Outcome\n\nTest\n\n\
             ## Acceptance Criteria\n\n- G-AC1: First\n- G-AC2: Second\n"
        ),
    )
    .unwrap();
    fs::write(
        task.path.join("plan.md"),
        format!(
            "# Plan: Test\n\n- Status: accepted\n- Goal: ./goal.md\n\
             - Branch: waif/task\n- Base branch: main\n- Base commit: {base}\n\
             - Created: {TIMESTAMP}\n- Updated: {TIMESTAMP}\n\n\
             ## Technical Summary\n\nApproach\n\n\
             ## Decisions\n\n- P-D1: Decision\n\n\
             ## Current System\n\nContext\n\n\
             ## Plan Items\n\n### P1: First\n\n- Status: pending\n\
             - Goal criteria: G-AC1\n\n\
             ### P2: Earlier work\n\n- Status: done\n\
             - Goal criteria: G-AC2\n\n\
             ### P3: Later work\n\n- Status: pending\n\
             - Goal criteria: G-AC2\n\n\
             ## Risks\n\n- P-R1: Risk\n\n\
             ## Cross-Cutting Validation\n\n- P-V1: Check\n"
        ),
    )
    .unwrap();
    write_step(&task, "01-test", &step_source(status));
    (workspace, task)
}

fn step_source(status: &str) -> String {
    format!(
        "# Step 1: Test\n\n- Status: {status}\n- Source commit: not-created\n\
         - Created: {TIMESTAMP}\n- Updated: {TIMESTAMP}\n- Plan items: P1\n\
         - Goal criteria: G-AC1\n\n## Objective\n\nDo  it.\n\n\
         ## Plan Item Coverage\n\n### S1-PC1: Part\n- Plan item: P1\n- Coverage: partial\n\n\
         ## Done Conditions\n\n- S1-DC1: Done\n\n## Context\n\nContext\n"
    )
}

fn write_step(task: &Task, directory: &str, source: &str) {
    let path = step_path(task, directory);
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, source).unwrap();
}

fn step_path(task: &Task, directory: &str) -> PathBuf {
    task.path.join("steps").join(directory).join("step.md")
}

fn read_step(task: &Task) -> String {
    fs::read_to_string(step_path(task, "01-test")).unwrap()
}

fn updated(source: &str) -> &str {
    source
        .lines()
        .find_map(|line| line.strip_prefix("- Updated: "))
        .unwrap()
}

fn expected_accepted(before: &str, after: &str) -> String {
    before
        .replace("- Status: drafting", "- Status: accepted")
        .replace("- Status: amending", "- Status: accepted")
        .replace(
            &format!("- Updated: {TIMESTAMP}"),
            &format!("- Updated: {}", updated(after)),
        )
}

fn stage(workspace: &TestDir, path: &str, contents: &str) {
    fs::write(workspace.path.join(path), contents).unwrap();
    GitRepo::new(&workspace.path).stage_file(path).unwrap();
}

fn git(workspace: &TestDir, arguments: &[&str]) -> String {
    let output = std::process::Command::new("git")
        .args(arguments)
        .current_dir(&workspace.path)
        .output()
        .unwrap();
    String::from_utf8(output.stdout)
        .unwrap()
        .trim_end()
        .to_owned()
}

fn install_hook(workspace: &TestDir, script: &str) {
    let hook = workspace.path.join(".git/hooks/pre-commit");
    fs::create_dir_all(hook.parent().unwrap()).unwrap();
    fs::write(&hook, format!("#!/bin/sh\n{script}\n")).unwrap();
    fs::set_permissions(&hook, fs::Permissions::from_mode(0o755)).unwrap();
}
