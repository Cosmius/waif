use std::path::Path;

use serde_json::{json, Value};

use super::{write_files, write_json, Metadata, PackageTargetImpl, Result, StagingDir};

const PACKAGE_FILES: &[&str] = &[
    "migration/README.md",
    "migration/v0.1.md",
    "migration/v0.2.md",
    "protocol/workflow.md",
    "protocol/waif-helper.md",
    "protocol/template/final-review.md",
    "protocol/template/goal.md",
    "protocol/template/implementation-review.md",
    "protocol/template/plan.md",
    "protocol/template/review.md",
    "protocol/template/step.md",
    "protocol/template/task-conclusion.md",
    "protocol/template/task-review.md",
    "skills/dev-workflow-setup/SKILL.md",
    "skills/dev-draft-goal/SKILL.md",
    "skills/dev-draft-plan/SKILL.md",
    "skills/dev-plan-step/SKILL.md",
    "skills/dev-implement-step/SKILL.md",
    "skills/dev-review/SKILL.md",
    "skills/dev-final-review/SKILL.md",
    "skills/dev-conclude-task/SKILL.md",
];

pub(crate) struct ClaudePlugin;

impl PackageTargetImpl for ClaudePlugin {
    fn build(&self, dist: &Path, metadata: &Metadata) -> Result<()> {
        let output = dist.join("claude-plugin");
        let staging = StagingDir::new(dist, ".claude-plugin.")?;
        let plugin_dir = staging.path().join("plugins").join(metadata.name);
        write_files(&plugin_dir, PACKAGE_FILES)?;
        write_json(
            plugin_dir.join(".claude-plugin/plugin.json"),
            &manifest(metadata),
        )?;
        write_json(
            staging.path().join(".claude-plugin/marketplace.json"),
            &marketplace(metadata),
        )?;
        staging.replace(output.as_path())?;
        println!(
            "The Claude Code plugin package is built at {}",
            output.display()
        );
        println!("Install it with");
        println!("  claude plugin marketplace add {}", output.display());
        println!("  claude plugin install {0}@{0}", metadata.name);
        Ok(())
    }
}

fn manifest(metadata: &Metadata) -> Value {
    json!({
        "name": metadata.name,
        "version": metadata.version,
        "description": metadata.description,
        "author": {
            "name": metadata.author,
        },
        "keywords": metadata.keywords,
    })
}

fn marketplace(metadata: &Metadata) -> Value {
    json!({
        "name": metadata.name,
        "owner": {
            "name": metadata.author,
        },
        "description": metadata.description,
        "plugins": [
            {
                "name": metadata.name,
                "source": format!("./plugins/{}", metadata.name),
                "description": metadata.description,
            }
        ]
    })
}
