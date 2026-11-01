//! Records the Git commit that the `waif` binary is built from.

use std::process::Command;

fn main() {
    let commit = git(&[
        "describe",
        "--always",
        "--abbrev=40",
        "--dirty",
        "--exclude=*",
    ])
    .unwrap_or_else(|| "unknown".to_owned());
    println!("cargo:rustc-env=WAIF_GIT_COMMIT={commit}");

    println!("cargo:rerun-if-changed=build.rs");
    let Some(files) = git(&["ls-files"]) else {
        return;
    };
    // Tracked files drive the dirty marker; HEAD, the index, and refs drive
    // the commit hash.
    for file in files.lines() {
        println!("cargo:rerun-if-changed={file}");
    }
    let mut git_paths = vec![
        "HEAD".to_owned(),
        "index".to_owned(),
        "packed-refs".to_owned(),
    ];
    git_paths.extend(git(&["symbolic-ref", "-q", "HEAD"]));
    for path in git_paths {
        if let Some(path) = git(&["rev-parse", "--git-path", &path]) {
            println!("cargo:rerun-if-changed={path}");
        }
    }
}

fn git(args: &[&str]) -> Option<String> {
    let output = Command::new("git").args(args).output().ok()?;
    if !output.status.success() {
        return None;
    }
    Some(String::from_utf8(output.stdout).ok()?.trim().to_owned())
}
