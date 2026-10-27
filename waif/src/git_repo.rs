use std::path::PathBuf;
use std::process::{Command, Output};

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) enum CurrentBranch {
    Branch(String),
    DetachedHead,
}

pub(crate) struct BranchState {
    pub(crate) name: String,
    pub(crate) commit: String,
    pub(crate) checked_out: bool,
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
        self.run_text_output(arguments)
    }

    pub(crate) fn initialise(&self) -> Result<(), String> {
        self.exec(&["init"])
    }

    pub(crate) fn validate_branch_name(&self, branch: &str) -> Result<(), String> {
        self.exec(&["check-ref-format", &format!("refs/heads/{branch}")])?;
        self.exec(&["check-ref-format", "--branch", branch])?;
        Ok(())
    }

    pub(crate) fn require_commit(&self, commit: &str) -> Result<(), String> {
        self.exec(&["cat-file", "-e", &format!("{commit}^{{commit}}")])
    }

    pub(crate) fn tracked_files_clean(&self) -> Result<bool, String> {
        Ok(self
            .run_text_output(&["status", "--porcelain", "--untracked-files=no"])?
            .is_empty())
    }

    pub(crate) fn branch_states(&self, branches: &[&str]) -> Result<Vec<BranchState>, String> {
        let refs: Vec<_> = branches
            .iter()
            .map(|name| format!("refs/heads/{name}"))
            .collect();
        let mut arguments = vec![
            "for-each-ref",
            "--format=%(refname)%00%(objectname)%00%(worktreepath)",
        ];
        arguments.extend(refs.iter().map(String::as_str));
        let output = self.run_bytes_output(&arguments)?;
        let mut states = Vec::new();
        for line in output.split(|byte| *byte == b'\n') {
            let mut fields = line.splitn(3, |byte| *byte == 0);
            let reference = fields.next().unwrap_or_default();
            let Some(index) = refs.iter().position(|name| name.as_bytes() == reference) else {
                continue;
            };
            let Some(commit) = fields.next() else {
                continue;
            };
            let Some(worktree) = fields.next() else {
                continue;
            };
            states.push(BranchState {
                name: branches[index].to_owned(),
                commit: String::from_utf8(commit.to_vec()).map_err(|error| error.to_string())?,
                checked_out: !worktree.is_empty(),
            });
        }
        Ok(states)
    }

    pub(crate) fn switch_branch(&self, branch: &str) -> Result<(), String> {
        self.exec(&["switch", "--", branch])
    }

    pub(crate) fn create_and_switch_branch(&self, branch: &str, base: &str) -> Result<(), String> {
        self.exec(&["switch", "-c", branch, base])
    }

    pub(crate) fn exec(&self, arguments: &[&str]) -> Result<(), String> {
        let output = self.run(arguments)?;
        self.check_output(arguments, &output)
    }

    fn run_text_output(&self, arguments: &[&str]) -> Result<String, String> {
        self.text(arguments, self.run(arguments)?)
    }

    fn run_bytes_output(&self, arguments: &[&str]) -> Result<Vec<u8>, String> {
        let output = self.run(arguments)?;
        self.check_output(arguments, &output)?;
        Ok(output.stdout)
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
        self.check_output(arguments, &output)?;
        String::from_utf8(output.stdout)
            .map(|value| value.trim_end_matches('\n').to_owned())
            .map_err(|error| format!("Git output is not UTF-8: {error}"))
    }

    fn check_output(&self, arguments: &[&str], output: &Output) -> Result<(), String> {
        if output.status.success() {
            return Ok(());
        }
        Err(format!(
            "{}: git {} failed ({}): {}",
            self.dir.display(),
            arguments.join(" "),
            output.status,
            String::from_utf8_lossy(&output.stderr).trim()
        ))
    }
}

#[cfg(test)]
#[allow(dead_code)]
impl GitRepo {
    pub(crate) fn initialise_on_branch(&self, branch: &str) -> Result<(), String> {
        self.exec(&["init", "-b", branch])
    }

    pub(crate) fn stage_file(&self, path: &str) -> Result<(), String> {
        self.exec(&["add", "--", path])
    }

    pub(crate) fn create_branch(&self, branch: &str) -> Result<(), String> {
        self.exec(&["branch", branch])
    }

    pub(crate) fn delete_branch(&self, branch: &str) -> Result<(), String> {
        self.exec(&["branch", "-d", branch])
    }

    pub(crate) fn detach_head(&self, commit: &str) -> Result<(), String> {
        self.exec(&["switch", "--detach", commit])
    }

    pub(crate) fn set_head_branch(&self, branch: &str) -> Result<(), String> {
        self.exec(&["symbolic-ref", "HEAD", &format!("refs/heads/{branch}")])
    }

    pub(crate) fn branch_name(&self) -> Result<String, String> {
        match self.current_branch()? {
            CurrentBranch::Branch(name) => Ok(name),
            CurrentBranch::DetachedHead => Err("HEAD is detached".to_owned()),
        }
    }

    pub(crate) fn references(&self) -> Result<Vec<u8>, String> {
        self.run_bytes_output(&["show-ref"])
    }

    pub(crate) fn index_entries(&self) -> Result<Vec<u8>, String> {
        self.run_bytes_output(&["ls-files", "--stage"])
    }

    pub(crate) fn working_tree_status(&self) -> Result<Vec<u8>, String> {
        self.run_bytes_output(&["status", "--porcelain"])
    }

    pub(crate) fn is_ignored(&self, path: &str) -> Result<bool, String> {
        let arguments = &["check-ignore", "--", path];
        let output = self.run(arguments)?;
        if output.status.code() == Some(1) {
            return Ok(false);
        }
        self.check_output(arguments, &output)?;
        Ok(true)
    }

    pub(crate) fn file_object_at_head(&self, path: &str) -> Result<String, String> {
        let argument = format!("HEAD:{path}");
        let arguments = &["rev-parse", argument.as_str()];
        self.run_text_output(arguments)
    }

    pub(crate) fn add_worktree(&self, branch: &str, path: &str) -> Result<(), String> {
        self.exec(&["worktree", "add", "-b", branch, path])
    }

    pub(crate) fn remove_worktree(&self, path: &str) -> Result<(), String> {
        self.exec(&["worktree", "remove", "--force", path])
    }
}
