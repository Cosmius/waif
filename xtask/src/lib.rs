use std::error::Error;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use clap::{Parser, Subcommand, ValueEnum};
use serde_json::Value;

static METADATA: Metadata = Metadata {
    name: "waif",
    version: "0.1.0",
    author: "Cosmia Fu",
    keywords: &["development", "planning", "review", "workflow"],
    description: "Plan, implement, and review development tasks.",
};

mod codex_plugin;
mod pi_package;
mod skill_zip;

type Result<T> = std::result::Result<T, Box<dyn Error>>;

pub fn run() -> Result<()> {
    let root = workspace_root()?;
    match Cli::parse().command {
        Command::Package(package) => package_build(&root, package.target),
    }
}

fn workspace_root() -> Result<PathBuf> {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    manifest_dir
        .parent()
        .map(Path::to_path_buf)
        .ok_or_else(|| "xtask manifest has no parent directory".into())
}

fn package_build(root: &Path, target: PackageTarget) -> Result<()> {
    let dist = root.join("dist");
    match target {
        PackageTarget::PiPackage => pi_package::PiPackage.build(root, &dist, &METADATA),
        PackageTarget::CodexPlugin => codex_plugin::CodexPlugin.build(root, &dist, &METADATA),
        PackageTarget::SkillZip => skill_zip::SkillZip.build(root, &dist, &METADATA),
    }
}

fn copy_whitelist(source: &Path, destination: &Path, files: &[&str]) -> Result<()> {
    for file in files {
        let source_file = source.join(file);
        let destination_file = destination.join(file);
        if let Some(parent) = destination_file.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::copy(&source_file, &destination_file)?;
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

#[derive(Parser)]
#[command(about = "Repository automation tasks")]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    Package(PackageCommand),
}

#[derive(Parser)]
struct PackageCommand {
    #[arg(long, value_enum)]
    target: PackageTarget,
}

#[derive(Copy, Clone, ValueEnum)]
enum PackageTarget {
    PiPackage,
    CodexPlugin,
    SkillZip,
}

trait PackageTargetImpl {
    fn build(&self, root: &Path, dist: &Path, metadata: &Metadata) -> Result<()>;
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
