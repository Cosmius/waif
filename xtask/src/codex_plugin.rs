use std::path::Path;

use serde_json::{json, Value};

use crate::{copy_whitelist, write_json, Metadata, PackageTargetImpl, Result, StagingDir};

const PACKAGE_FILES: &[&str] = &[
    "protocol/workflow.md",
    "skills/dev-draft-charter/SKILL.md",
    "skills/dev-draft-charter/agents/openai.yaml",
    "skills/dev-draft-goal/SKILL.md",
    "skills/dev-draft-goal/agents/openai.yaml",
    "skills/dev-draft-plan/SKILL.md",
    "skills/dev-draft-plan/agents/openai.yaml",
    "skills/dev-plan-step/SKILL.md",
    "skills/dev-plan-step/agents/openai.yaml",
    "skills/dev-implement-step/SKILL.md",
    "skills/dev-implement-step/agents/openai.yaml",
    "skills/dev-final-review/SKILL.md",
    "skills/dev-final-review/agents/openai.yaml",
    "skills/dev-conclude-task/SKILL.md",
    "skills/dev-conclude-task/agents/openai.yaml",
];

pub(crate) struct CodexPlugin;

impl PackageTargetImpl for CodexPlugin {
    fn build(&self, root: &Path, dist: &Path, metadata: &Metadata) -> Result<()> {
        let output = dist.join("codex-plugin");
        let staging = StagingDir::new(dist, ".codex-plugin.")?;
        let plugin_dir = staging.path().join("plugins").join(metadata.name);
        copy_whitelist(root, &plugin_dir, PACKAGE_FILES)?;

        let mut manifest = manifest(metadata);
        manifest["skills"] = json!("./skills/");
        write_json(plugin_dir.join(".codex-plugin/plugin.json"), &manifest)?;

        write_json(
            staging.path().join(".agents/plugins/marketplace.json"),
            &marketplace(metadata)?,
        )?;
        staging.replace(output.as_path())?;
        println!("The Codex plugin package is built at {}", output.display());
        println!("Install it with");
        println!("  codex plugin marketplace add {}", output.display());
        Ok(())
    }
}

fn manifest(metadata: &Metadata) -> Value {
    json!({
        "name": metadata.name,
        "version": metadata.version,
        "description": "File-backed software development workflow skills for Codex.",
        "author": {
            "name": metadata.author,
        },
        "keywords": metadata.keywords,
        "interface": {
            "displayName": "Waif",
            "shortDescription": metadata.description,
            "longDescription": concat!(
                "A file-backed development workflow with explicit ",
                "human approval gates for goals, plans, implementation steps, ",
                "source changes, final review, and repository handoff report."
            ),
            "developerName": metadata.author,
            "category": "Developer Tools",
            "capabilities": ["Interactive", "Read", "Write"],
            "defaultPrompt": [
                "Draft a durable goal for this development task.",
                "Plan the next implementation step.",
                "Review the completed development task.",
                "Document a completed task for other contributors."
            ]
        }
    })
}

fn marketplace(metadata: &Metadata) -> Result<Value> {
    Ok(json!({
        "name": metadata.name,
        "interface": {
            "displayName": "Waif"
        },
        "plugins": [
            {
                "name": metadata.name,
                "source": {
                    "source": "local",
                    "path": format!("./plugins/{}", metadata.name)
                },
                "policy": {
                    "installation": "AVAILABLE",
                    "authentication": "ON_INSTALL"
                },
                "category": "Developer Tools"
            }
        ]
    }))
}
