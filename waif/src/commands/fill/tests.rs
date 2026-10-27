use super::*;
use crate::test_support::TestDir;

const VALID_GOAL: &str = concat!(
    "# Goal\n",
    "- Status: accepted\n",
    "- Created: 2026-08-14T12:00:00+09:00\n",
    "- Updated: __NOW__\n",
    "## Outcome\nFilled outcome.\n",
    "## Acceptance Criteria\n- G-AC1: Filled.\n",
);

#[test]
fn fill_writes_generic_text_and_reports_noop() {
    let directory = TestDir::new("fill-generic");
    let path = directory.path.join("notes.txt");
    fs::write(&path, "before __NOW__ after").expect("source should be written");

    assert!(matches!(fill_path(&path).expect("fill should work"), FillOutcome::Filled));
    assert!(!fs::read_to_string(&path).expect("result should load").contains("__NOW__"));
    assert!(matches!(fill_path(&path).expect("noop should work"), FillOutcome::NoFillNeeded));
}

#[test]
fn fill_checks_recognized_candidates_before_writing() {
    let directory = TestDir::new("fill-goal");
    let path = directory.path.join("goal.md");
    fs::write(&path, VALID_GOAL).expect("goal should be written");

    assert!(matches!(fill_path(&path).expect("fill should work"), FillOutcome::Filled));
    let artifact = ArtifactFile::read(path).expect("goal should load");
    assert_eq!(artifact.check().classification(), ArtifactClassification::Good);
}

#[test]
fn bad_candidate_uses_the_next_recovery_name_and_preserves_target() {
    let directory = TestDir::new("fill-recovery");
    let path = directory.path.join("goal.md");
    let source = VALID_GOAL.replace("accepted", "prefix__NOW__suffix");
    fs::write(&path, &source).expect("goal should be written");
    fs::write(path.with_file_name("goal.md.waif-recovery"), "existing")
        .expect("collision should be written");

    let outcome = fill_path(&path).expect("rejection should be reported");
    let FillOutcome::Rejected {
        recovery_path,
        check,
    } = outcome
    else {
        panic!("candidate should be rejected");
    };

    assert_eq!(recovery_path, path.with_file_name("goal.md.waif-recovery-2"));
    assert_eq!(check.classification(), ArtifactClassification::Bad);
    assert!(!check.diagnostics().is_empty());
    assert_eq!(fs::read_to_string(&path).expect("target should load"), source);
    assert!(!fs::read_to_string(recovery_path).expect("recovery should load").contains("__NOW__"));
}

#[test]
fn failures_before_target_write_do_not_call_the_writer() {
    let directory = TestDir::new("fill-recovery-failure");
    let path = directory.path.join("goal.md");
    fs::write(&path, VALID_GOAL.replace("accepted", "prefix__NOW__suffix"))
        .expect("goal should be written");

    let error = fill_path_with(
        &path,
        |_, _| panic!("target writer must not run"),
        |_, _| Err(io::Error::new(io::ErrorKind::PermissionDenied, "denied")),
    )
    .expect_err("recovery failure should fail");

    assert!(error.to_string().contains("failed to write recovery for"));
}

#[test]
fn target_write_failures_are_identified() {
    let directory = TestDir::new("fill-target-failure");
    let path = directory.path.join("notes.txt");
    fs::write(&path, "__NOW__").expect("source should be written");

    let error = fill_path_with(
        &path,
        |_, _| Err(io::Error::new(io::ErrorKind::WriteZero, "short write")),
        |_, _| panic!("recovery writer must not run"),
    )
    .expect_err("target failure should fail");

    assert!(error.to_string().contains("failed to overwrite"));
}
