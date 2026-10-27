use std::path::PathBuf;
use std::process::{Command, Output};

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) enum CurrentBranch {
    Branch(String),
    DetachedHead,
}

#[derive(Clone)]
pub(crate) struct GitRepo {
    dir: PathBuf,
}

impl GitRepo {
    pub(crate) fn new(dir: impl Into<PathBuf>) -> Self {
        Self { dir: dir.into() }
    }

    pub(crate) fn current_branch(&self) -> Result<CurrentBranch, String> {
        let arguments = &["symbolic-ref", "--quiet", "HEAD"];
        let output = self.run(arguments)?;
        if output.status.code() == Some(1) {
            return Ok(CurrentBranch::DetachedHead);
        }
        let reference = self.text(arguments, output)?;
        let name = reference
            .strip_prefix("refs/heads/")
            .ok_or("HEAD does not reference a local branch")?;
        Ok(CurrentBranch::Branch(name.to_owned()))
    }

    pub(crate) fn last_commit(&self) -> Result<String, String> {
        let arguments = &["rev-parse", "--verify", "HEAD^{commit}"];
        self.text(arguments, self.run(arguments)?)
    }

    fn run(&self, arguments: &[&str]) -> Result<Output, String> {
        Command::new("git")
            .arg("--no-optional-locks")
            .args(arguments)
            .current_dir(&self.dir)
            .output()
            .map_err(|error| format!("{}: failed to run Git: {error}", self.dir.display()))
    }

    fn text(&self, arguments: &[&str], output: Output) -> Result<String, String> {
        if !output.status.success() {
            return Err(format!(
                "{}: Git {} failed ({}): {}",
                self.dir.display(),
                arguments.join(" "),
                output.status,
                String::from_utf8_lossy(&output.stderr).trim()
            ));
        }
        String::from_utf8(output.stdout)
            .map(|value| value.trim_end_matches('\n').to_owned())
            .map_err(|error| format!("Git output is not UTF-8: {error}"))
    }
}
