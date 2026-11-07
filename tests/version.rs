use std::process::Command;

#[test]
fn version_reports_crate_and_schema_versions_and_git_commit() {
    let output = Command::new(env!("CARGO_BIN_EXE_waif"))
        .arg("--version")
        .output()
        .expect("waif should run");

    let expected = format!(
        "waif {}\nSchema version: 0.3\nGit commit: {}\n",
        env!("CARGO_PKG_VERSION"),
        env!("WAIF_GIT_COMMIT"),
    );
    assert!(output.status.success());
    assert_eq!(String::from_utf8_lossy(&output.stdout), expected);
}
