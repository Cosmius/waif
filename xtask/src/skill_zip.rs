use std::fs;
use std::io::Write;
use std::path::Path;

use zip::write::SimpleFileOptions;
use zip::{CompressionMethod, ZipWriter};

use crate::{Metadata, PackageTargetImpl, Result, StagingDir};

const PACKAGE_FILES: &[&str] = &[
    "protocol/workflow.md",
    "skills/dev-draft-charter/SKILL.md",
    "skills/dev-draft-goal/SKILL.md",
    "skills/dev-draft-plan/SKILL.md",
    "skills/dev-plan-step/SKILL.md",
    "skills/dev-implement-step/SKILL.md",
    "skills/dev-final-review/SKILL.md",
    "skills/dev-conclude-task/SKILL.md",
];

pub(crate) struct SkillZip;

impl PackageTargetImpl for SkillZip {
    fn build(&self, root: &Path, dist: &Path, metadata: &Metadata) -> Result<()> {
        let output = dist.join(format!("{}-skills.zip", metadata.name));
        let staging = StagingDir::new(dist, ".skill-zip.")?;
        let staged_output = staging.path().join("skills.zip");
        write_zip(root, &staged_output, PACKAGE_FILES)?;
        replace_file(staged_output.as_path(), output.as_path())?;
        println!("The skill zip package is built at {}", output.display());
        Ok(())
    }
}

fn replace_file(source: &Path, output: &Path) -> Result<()> {
    if output.exists() {
        fs::remove_file(output)?;
    }
    fs::rename(source, output)?;
    Ok(())
}

fn write_zip(root: &Path, output: &Path, files: &[&str]) -> Result<()> {
    if let Some(parent) = output.parent() {
        fs::create_dir_all(parent)?;
    }

    let file = fs::File::create(output)?;
    let mut zip = ZipWriter::new(file);
    let options = SimpleFileOptions::default()
        .compression_method(CompressionMethod::Stored)
        .unix_permissions(0o644);

    for name in files {
        zip.start_file(name, options)?;
        zip.write_all(&fs::read(root.join(name))?)?;
    }

    zip.finish()?;
    Ok(())
}
