use std::path::Path;

use serde_json::{json, Value};

use crate::{copy_whitelist, write_json, Metadata, PackageTargetImpl, Result, StagingDir};

const PACKAGE_FILES: &[&str] = &[
    "protocol/workflow.md",
    "skills/dev-draft-goal/SKILL.md",
    "skills/dev-draft-plan/SKILL.md",
    "skills/dev-plan-step/SKILL.md",
    "skills/dev-implement-step/SKILL.md",
    "skills/dev-final-review/SKILL.md",
    "skills/dev-conclude-task/SKILL.md",
];

pub(crate) struct PiPackage;

impl PackageTargetImpl for PiPackage {
    fn build(&self, root: &Path, dist: &Path, metadata: &Metadata) -> Result<()> {
        let output = dist.join("pi-package");
        let staging = StagingDir::new(dist, ".pi-package.")?;
        copy_whitelist(root, staging.path(), PACKAGE_FILES)?;
        write_json(staging.path().join("package.json"), &package(metadata)?)?;
        staging.replace(output.as_path())?;
        println!("The pi package is built at {}", output.display());
        println!("You might install it with:");
        println!("CAUTION: It may overwrite your files!");
        println!("  mkdir -p $HOME/.pi/local-packages && cp -r {}/ $HOME/.pi/local-packages/{}/ && pi install $HOME/.pi/local-packages/{}/",
                 output.display(), metadata.name, metadata.name);
        Ok(())
    }
}

fn package(metadata: &Metadata) -> Result<Value> {
    let mut keywords = metadata
        .keywords
        .iter()
        .map(|keyword| json!(keyword))
        .collect::<Vec<_>>();
    keywords.push(json!("pi-package"));

    Ok(json!({
        "name": metadata.name,
        "version": metadata.version,
        "description": metadata.description,
        "keywords": keywords,
        "pi": {
            "skills": ["./skills"]
        },
        "author": {
            "name": metadata.author
        }
    }))
}
