use std::error::Error;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use clap::ValueEnum;
use serde_json::Value;

type Result<T> = std::result::Result<T, Box<dyn Error>>;

/// Embeds repository files once, keyed by their repository-relative paths.
macro_rules! embed {
    ($($path:literal),* $(,)?) => {
        &[$(
            ($path, include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"), "/", $path))),
        )*]
    };
}

/// Every file any package ships; targets select from it by path.
static CONTENT: &[(&str, &[u8])] = embed![
    "protocol/waif-helper.md",
    "protocol/workflow.md",
    "protocol/workflow_migration.md",
    "skills/dev-conclude-task/SKILL.md",
    "skills/dev-conclude-task/agents/openai.yaml",
    "skills/dev-draft-goal/SKILL.md",
    "skills/dev-draft-goal/agents/openai.yaml",
    "skills/dev-draft-plan/SKILL.md",
    "skills/dev-draft-plan/agents/openai.yaml",
    "skills/dev-final-review/SKILL.md",
    "skills/dev-final-review/agents/openai.yaml",
    "skills/dev-implement-step/SKILL.md",
    "skills/dev-implement-step/agents/openai.yaml",
    "skills/dev-plan-step/SKILL.md",
    "skills/dev-plan-step/agents/openai.yaml",
    "skills/dev-workflow-setup/SKILL.md",
    "skills/dev-workflow-setup/agents/openai.yaml",
];

mod claude_plugin;
mod codex_plugin;
mod pi_package;
mod skill_zip;

static METADATA: Metadata = Metadata {
    name: "waif",
    version: env!("CARGO_PKG_VERSION"),
    author: "Cosmia Fu",
    keywords: &["development", "planning", "review", "workflow"],
    description: "Plan, implement, and review development tasks.",
};

#[derive(Copy, Clone, ValueEnum)]
pub enum PackageTarget {
    PiPackage,
    CodexPlugin,
    ClaudePlugin,
    SkillZip,
}

pub fn build(target: PackageTarget) -> Result<()> {
    let dist = std::env::current_dir()?.join("dist");
    match target {
        PackageTarget::PiPackage => pi_package::PiPackage.build(&dist, &METADATA),
        PackageTarget::CodexPlugin => codex_plugin::CodexPlugin.build(&dist, &METADATA),
        PackageTarget::ClaudePlugin => claude_plugin::ClaudePlugin.build(&dist, &METADATA),
        PackageTarget::SkillZip => skill_zip::SkillZip.build(&dist, &METADATA),
    }
}

/// The packaged helper guide, which records the SHA-1 of the packaging binary.
const HELPER_GUIDE: &str = "protocol/waif-helper.md";

fn package_file(name: &str) -> Result<Vec<u8>> {
    let mut contents = embedded(name)?.to_vec();
    if name == HELPER_GUIDE {
        let hash = sha1_smol::Sha1::from(fs::read(std::env::current_exe()?)?).digest();
        contents.extend(
            format!(
                "\n## Debug Information\n\nPackaged by the `waif` binary with SHA-1 `{hash}`.\n"
            )
            .bytes(),
        );
    }
    Ok(contents)
}

fn embedded(name: &str) -> Result<&'static [u8]> {
    CONTENT
        .iter()
        .find(|(path, _)| *path == name)
        .map(|(_, contents)| *contents)
        .ok_or_else(|| format!("{name} is not embedded in this binary").into())
}

fn write_files(destination: &Path, files: &[&str]) -> Result<()> {
    for file in files {
        let destination_file = destination.join(file);
        if let Some(parent) = destination_file.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(&destination_file, package_file(file)?)?;
    }
    Ok(())
}

fn write_json(path: PathBuf, value: &Value) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(path, format!("{}\n", serde_json::to_string_pretty(value)?))?;
    Ok(())
}

trait PackageTargetImpl {
    fn build(&self, dist: &Path, metadata: &Metadata) -> Result<()>;
}

struct Metadata {
    name: &'static str,
    version: &'static str,
    author: &'static str,
    keywords: &'static [&'static str],
    description: &'static str,
}

struct StagingDir {
    path: PathBuf,
}

impl StagingDir {
    fn new(parent: &Path, prefix: &str) -> Result<Self> {
        fs::create_dir_all(parent)?;
        let nonce = SystemTime::now().duration_since(UNIX_EPOCH)?.as_nanos();
        let path = parent.join(format!("{prefix}{}.{nonce}", std::process::id()));
        fs::create_dir(&path)?;
        Ok(Self { path })
    }

    fn path(&self) -> &Path {
        &self.path
    }

    fn replace(mut self, output: &Path) -> Result<()> {
        if output.exists() {
            fs::remove_dir_all(output)?;
        }
        fs::rename(&self.path, output)?;
        self.path = PathBuf::new();
        Ok(())
    }
}

impl Drop for StagingDir {
    fn drop(&mut self) {
        if !self.path.as_os_str().is_empty() {
            let _ = fs::remove_dir_all(&self.path);
        }
    }
}
