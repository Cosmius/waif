use std::error::Error;
use std::fmt;
use std::fs::{self, OpenOptions};
use std::io::{self, Write};
use std::path::{Path, PathBuf};

use clap::Args;

use super::relative_path;
use crate::macro_processor;
use crate::parser::Severity;
use crate::protocol::{ArtifactCheck, ArtifactClassification, ArtifactFile, ArtifactKind};

type Result<T> = std::result::Result<T, Box<dyn Error>>;

#[derive(Args)]
#[command(after_help = macro_processor::fill_help())]
pub struct FillCommand {
    /// Text file containing known macros.
    path: PathBuf,
}

impl FillCommand {
    pub fn run(self) -> Result<()> {
        let workspace_dir = std::env::current_dir()?;
        let display_path = relative_path(&workspace_dir, &self.path);

        match fill_path(&self.path)? {
            FillOutcome::Filled => println!("Filled: {display_path}"),
            FillOutcome::NoFillNeeded => println!("No fill needed: {display_path}"),
            FillOutcome::Rejected {
                recovery_path,
                check,
            } => {
                for diagnostic in check.diagnostics() {
                    let severity = match diagnostic.severity() {
                        Severity::Error => "ERROR",
                        Severity::Warning => "WARNING",
                    };
                    eprintln!(
                        "{display_path}:{}: {severity}: {}",
                        diagnostic.line(),
                        diagnostic.message()
                    );
                }
                eprintln!(
                    "Recovery: {}",
                    relative_path(&workspace_dir, &recovery_path)
                );
                return Err(Box::new(RejectedCandidate));
            }
        }
        Ok(())
    }
}

#[derive(Debug)]
struct RejectedCandidate;

impl fmt::Display for RejectedCandidate {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("filled candidate is BAD; target was not changed")
    }
}

impl Error for RejectedCandidate {}

#[derive(Debug)]
enum FillOutcome {
    Filled,
    NoFillNeeded,
    Rejected {
        recovery_path: PathBuf,
        check: ArtifactCheck,
    },
}

fn fill_path(path: &Path) -> Result<FillOutcome> {
    fill_path_with(
        path,
        |path, candidate| fs::write(path, candidate),
        write_recovery,
    )
}

fn fill_path_with(
    path: &Path,
    write_target: impl FnOnce(&Path, &str) -> io::Result<()>,
    write_rejected: impl FnOnce(&Path, &str) -> io::Result<PathBuf>,
) -> Result<FillOutcome> {
    let source = fs::read_to_string(path).map_err(|error| contextual_error("read", path, error))?;
    let Some(candidate) = macro_processor::expand(&source)
        .map_err(|error| io::Error::other(format!("failed to resolve macros: {error}")))?
    else {
        return Ok(FillOutcome::NoFillNeeded);
    };

    if ArtifactKind::for_path(path).is_recognized() {
        let artifact = ArtifactFile::from_contents(path.to_path_buf(), candidate.clone());
        let check = artifact.check();
        if check.classification() == ArtifactClassification::Bad {
            let recovery_path = write_rejected(path, &candidate)
                .map_err(|error| contextual_error("write recovery for", path, error))?;
            return Ok(FillOutcome::Rejected {
                recovery_path,
                check,
            });
        }
    }

    write_target(path, &candidate).map_err(|error| contextual_error("overwrite", path, error))?;
    Ok(FillOutcome::Filled)
}

fn write_recovery(path: &Path, candidate: &str) -> io::Result<PathBuf> {
    let filename = path
        .file_name()
        .ok_or_else(|| io::Error::other("target path has no filename"))?
        .to_string_lossy();

    for sequence in 1_u64.. {
        let suffix = if sequence == 1 {
            ".waif-recovery".to_owned()
        } else {
            format!(".waif-recovery-{sequence}")
        };
        let recovery_path = path.with_file_name(format!("{filename}{suffix}"));

        match OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&recovery_path)
        {
            Ok(mut file) => {
                file.write_all(candidate.as_bytes())?;
                return Ok(recovery_path);
            }
            Err(error) if error.kind() == io::ErrorKind::AlreadyExists => {}
            Err(error) => return Err(error),
        }
    }
    unreachable!("recovery sequence is unbounded")
}

fn contextual_error(action: &str, path: &Path, error: io::Error) -> io::Error {
    io::Error::new(
        error.kind(),
        format!("failed to {action} {}: {error}", path.display()),
    )
}

#[cfg(test)]
mod tests {
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
}
