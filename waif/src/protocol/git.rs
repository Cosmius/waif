use std::path::Path;
use std::process::Command;

use super::Result;

pub(super) fn query(workspace: &Path, args: &[&str]) -> Result<Vec<u8>> {
    let output = Command::new("git")
        .arg("--no-optional-locks")
        .args(args)
        .current_dir(workspace)
        .output()
        .map_err(|error| format!("{}: git {}: {error}", workspace.display(), args.join(" ")))?;
    if output.status.success() {
        Ok(output.stdout)
    } else {
        Err(format!(
            "{}: git {} failed ({}): {}",
            workspace.display(),
            args.join(" "),
            output.status,
            String::from_utf8_lossy(&output.stderr).trim_end(),
        )
        .into())
    }
}

pub(super) fn text(workspace: &Path, args: &[&str]) -> Result<String> {
    Ok(String::from_utf8(query(workspace, args)?)?
        .trim_end_matches('\n')
        .to_owned())
}
