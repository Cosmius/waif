use std::fs;
use std::path::Path;
use std::process::{Command, Output};

use serde_json::Value;

mod test_support;

use test_support::TestDir;

const VERSION: &str = env!("CARGO_PKG_VERSION");

const SKILL_FILES: &[&str] = &[
    "migration/README.md",
    "migration/v0.1.md",
    "migration/v0.2.md",
    "protocol/template/final-review.md",
    "protocol/template/goal.md",
    "protocol/template/implementation-review.md",
    "protocol/template/plan.md",
    "protocol/template/review.md",
    "protocol/template/step.md",
    "protocol/template/task-conclusion.md",
    "protocol/template/task-review.md",
    "protocol/waif-helper.md",
    "protocol/workflow.md",
    "skills/dev-conclude-task/SKILL.md",
    "skills/dev-draft-goal/SKILL.md",
    "skills/dev-draft-plan/SKILL.md",
    "skills/dev-final-review/SKILL.md",
    "skills/dev-implement-step/SKILL.md",
    "skills/dev-plan-step/SKILL.md",
    "skills/dev-review/SKILL.md",
    "skills/dev-workflow-setup/SKILL.md",
];

#[test]
fn package_builds_claude_plugin() {
    let workspace = TestDir::new("package-claude-plugin");

    let output = run_package(&workspace, "claude-plugin");

    assert!(output.status.success());
    let package = workspace.path.join("dist/claude-plugin");
    let mut expected = vec![
        ".claude-plugin/marketplace.json".to_string(),
        "plugins/waif/.claude-plugin/plugin.json".to_string(),
    ];
    expected.extend(
        SKILL_FILES
            .iter()
            .map(|file| format!("plugins/waif/{file}")),
    );
    expected.sort();
    assert_eq!(list_files(&package), expected);
    let manifest = read_json(&package.join("plugins/waif/.claude-plugin/plugin.json"));
    assert_eq!(manifest["version"], VERSION);
}

#[test]
fn package_builds_codex_plugin() {
    let workspace = TestDir::new("package-codex-plugin");

    let output = run_package(&workspace, "codex-plugin");

    assert!(output.status.success());
    let package = workspace.path.join("dist/codex-plugin");
    let mut expected = vec![
        ".agents/plugins/marketplace.json".to_string(),
        "plugins/waif/.codex-plugin/plugin.json".to_string(),
    ];
    expected.extend(
        SKILL_FILES
            .iter()
            .map(|file| format!("plugins/waif/{file}")),
    );
    expected.extend(
        SKILL_FILES
            .iter()
            .filter_map(|file| file.strip_suffix("/SKILL.md"))
            .map(|skill| format!("plugins/waif/{skill}/agents/openai.yaml")),
    );
    expected.sort();
    assert_eq!(list_files(&package), expected);
    let manifest = read_json(&package.join("plugins/waif/.codex-plugin/plugin.json"));
    assert_eq!(manifest["version"], VERSION);
}

#[test]
fn package_builds_pi_package() {
    let workspace = TestDir::new("package-pi-package");

    let output = run_package(&workspace, "pi-package");

    assert!(output.status.success());
    let package = workspace.path.join("dist/pi-package");
    let mut expected = vec!["package.json".to_string()];
    expected.extend(SKILL_FILES.iter().map(|file| file.to_string()));
    expected.sort();
    assert_eq!(list_files(&package), expected);
    assert_eq!(read_json(&package.join("package.json"))["version"], VERSION);
}

#[test]
fn package_builds_skill_zip() {
    let workspace = TestDir::new("package-skill-zip");

    let output = run_package(&workspace, "skill-zip");

    assert!(output.status.success());
    let file =
        fs::File::open(workspace.path.join("dist/waif-skills.zip")).expect("skill zip should open");
    let archive = zip::ZipArchive::new(file).expect("skill zip should be readable");
    let mut entries = archive
        .file_names()
        .collect::<Result<Vec<_>, _>>()
        .expect("skill zip entry names should be readable");
    entries.sort();
    assert_eq!(entries, SKILL_FILES);
}

#[test]
fn package_appends_binary_hash_to_helper_guide() {
    let workspace = TestDir::new("package-helper-hash");

    let output = run_package(&workspace, "pi-package");

    assert!(output.status.success());
    let binary = fs::read(env!("CARGO_BIN_EXE_waif")).expect("waif binary should be readable");
    let hash = sha1_smol::Sha1::from(binary).digest();
    let expected = indoc::formatdoc! {"
            {guide}
            ## Debug Information

            Packaged by this `waif` binary:

            - Version: `{version}`
            - Schema version: `0.3`
            - Git commit: `{commit}`
            - SHA-1: `{hash}`
        ",
        guide = include_str!("../protocol/waif-helper.md"),
        version = env!("CARGO_PKG_VERSION"),
        commit = env!("WAIF_GIT_COMMIT"),
    };
    let packaged = fs::read_to_string(
        workspace
            .path
            .join("dist/pi-package/protocol/waif-helper.md"),
    )
    .expect("packaged helper guide should be readable");
    assert_eq!(packaged, expected);
}

fn run_package(workspace: &TestDir, target: &str) -> Output {
    Command::new(env!("CARGO_BIN_EXE_waif"))
        .args(["package", "--target", target])
        .current_dir(&workspace.path)
        .output()
        .expect("waif should run")
}

fn list_files(root: &Path) -> Vec<String> {
    let mut files = Vec::new();
    let mut pending = vec![root.to_path_buf()];
    while let Some(dir) = pending.pop() {
        for entry in fs::read_dir(&dir).expect("package directory should be readable") {
            let path = entry.expect("package entry should load").path();
            if path.is_dir() {
                pending.push(path);
            } else {
                let relative = path.strip_prefix(root).expect("entry should be in package");
                files.push(relative.display().to_string());
            }
        }
    }
    files.sort();
    files
}

fn read_json(path: &Path) -> Value {
    let text = fs::read_to_string(path).expect("manifest should be readable");
    serde_json::from_str(&text).expect("manifest should be valid JSON")
}
