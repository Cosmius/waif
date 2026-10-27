use std::fs;
use std::process::{Command, Output};
use test_support::git_repo::GitRepo;

mod test_support;

use test_support::TestDir;

const VALID_GOAL: &str = concat!(
    "# Goal\n",
    "- Status: accepted\n",
    "- Created: 2026-08-14T12:00:00+09:00\n",
    "- Updated: __NOW__\n",
    "## Outcome\nFilled outcome.\n",
    "## Acceptance Criteria\n- G-AC1: Filled.\n",
);

#[test]
fn fill_replaces_arbitrary_text_with_one_timestamp() {
    let directory = TestDir::new("fill-text");
    let path = directory.path.join("notes.txt");
    fs::write(&path, "é __NOW____NOW__ before`__NOW__`after").expect("source should be written");

    let output = run_fill(&directory, &[path.to_str().expect("path should be UTF-8")]);
    let contents = fs::read_to_string(&path).expect("result should load");
    let expanded = contents
        .strip_prefix("é ")
        .and_then(|value| value.strip_suffix(" before`__NOW__`after"))
        .expect("surrounding source should remain");
    let midpoint = expanded.len() / 2;

    assert!(output.status.success());
    assert_eq!(&expanded[..midpoint], &expanded[midpoint..]);
    assert_eq!(String::from_utf8_lossy(&output.stdout), format!("Filled: {}\n", path.display()));
}

#[test]
fn fill_reports_no_work_without_changing_the_file() {
    let directory = TestDir::new("fill-noop");
    let path = directory.path.join("notes.txt");
    let source = "plain `__NOW__` text";
    fs::write(&path, source).expect("source should be written");

    let output = run_fill(&directory, &[path.to_str().expect("path should be UTF-8")]);

    assert!(output.status.success());
    assert_eq!(fs::read_to_string(&path).expect("source should load"), source);
    assert!(String::from_utf8_lossy(&output.stdout).contains("No fill needed:"));
}

#[test]
fn fill_checks_recognized_artifacts() {
    let directory = TestDir::new("fill-valid-goal");
    let path = directory.path.join("goal.md");
    fs::write(&path, VALID_GOAL).expect("goal should be written");

    let output = run_fill(&directory, &[path.to_str().expect("path should be UTF-8")]);

    assert!(output.status.success(), "{}", String::from_utf8_lossy(&output.stderr));
    assert!(!fs::read_to_string(&path).expect("goal should load").contains("__NOW__"));
    assert!(!path.with_file_name("goal.md.waif-recovery").exists());
}

#[test]
fn fill_recovers_bad_candidates_without_changing_the_target() {
    let directory = TestDir::new("fill-invalid-goal");
    let path = directory.path.join("goal.md");
    let source = VALID_GOAL.replace("accepted", "prefix__NOW__suffix");
    fs::write(&path, &source).expect("goal should be written");

    let output = run_fill(&directory, &[path.to_str().expect("path should be UTF-8")]);
    let recovery = path.with_file_name("goal.md.waif-recovery");

    assert_eq!(output.status.code(), Some(1));
    assert_eq!(fs::read_to_string(&path).expect("target should load"), source);
    assert!(recovery.exists());
    assert!(!fs::read_to_string(&recovery).expect("recovery should load").contains("__NOW__"));
    assert!(String::from_utf8_lossy(&output.stderr).contains("Recovery:"));
    assert!(String::from_utf8_lossy(&output.stderr).contains("ERROR:"));
}

#[test]
fn fill_requires_exactly_one_path() {
    let directory = TestDir::new("fill-arguments");

    assert_eq!(run_fill(&directory, &[]).status.code(), Some(2));
    assert_eq!(run_fill(&directory, &["one", "two"]).status.code(), Some(2));
}

fn run_fill(directory: &TestDir, arguments: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_waif"))
        .arg("fill")
        .args(arguments)
        .current_dir(&directory.path)
        .output()
        .expect("waif fill should run")
}

#[test]
fn fill_resolves_git_macros_from_invoking_workspace() {
    let directory = TestDir::new("fill-git");
    GitRepo::new(&directory.path)
        .initialise_on_branch("feature/macros")
        .unwrap();
    GitRepo::new(&directory.path)
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
    let commit = GitRepo::new(&directory.path).last_commit().unwrap();
    let other = TestDir::new("fill-git-target");
    let path = other.path.join("notes.txt");
    let source = "__CURRENT_BRANCH__ __LAST_COMMIT__ __LAST_COMMIT__ `__CURRENT_BRANCH__`";
    fs::write(&path, source).unwrap();

    let output = run_fill(&directory, &[path.to_str().unwrap()]);
    assert!(output.status.success(), "{}", String::from_utf8_lossy(&output.stderr));
    assert_eq!(fs::read_to_string(&path).unwrap(),
        format!("feature/macros {commit} {commit} `__CURRENT_BRANCH__`"));

    GitRepo::new(&directory.path).detach_head("HEAD").unwrap();
    fs::write(&path, "__LAST_COMMIT__").unwrap();
    assert!(run_fill(&directory, &[path.to_str().unwrap()]).status.success());
    assert_eq!(fs::read_to_string(&path).unwrap(), commit);

    fs::write(&path, source).unwrap();
    assert!(run_fill(&directory, &[path.to_str().unwrap()]).status.success());
    assert_eq!(fs::read_to_string(&path).unwrap(),
        format!("detached HEAD {commit} {commit} `__CURRENT_BRANCH__`"));
}

#[test]
fn fill_git_resolution_failures_preserve_source() {
    let directory = TestDir::new("fill-git-errors");
    let path = directory.path.join("notes.txt");
    for source in ["__CURRENT_BRANCH__", "__LAST_COMMIT__"] {
        fs::write(&path, source).unwrap();
        let output = run_fill(&directory, &[path.to_str().unwrap()]);
        assert!(!output.status.success());
        assert_eq!(fs::read_to_string(&path).unwrap(), source);
    }
    GitRepo::new(&directory.path)
        .initialise_on_branch("unborn")
        .unwrap();
    fs::write(&path, "__CURRENT_BRANCH__").unwrap();
    assert!(run_fill(&directory, &[path.to_str().unwrap()]).status.success());
    assert_eq!(fs::read_to_string(&path).unwrap(), "unborn");
    fs::write(&path, "__LAST_COMMIT__").unwrap();
    assert!(!run_fill(&directory, &[path.to_str().unwrap()]).status.success());
    assert_eq!(fs::read_to_string(&path).unwrap(), "__LAST_COMMIT__");
}
