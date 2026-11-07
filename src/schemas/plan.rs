use crate::artifact::Artifact;
use crate::parser::{self, Diagnostic};
use crate::schema::{
    self, metadata_validators, ArtifactPrefixRule, ItemRule, MetadataRule, Schema, SectionRule,
};

const SCHEMA: Schema = Schema {
    prefix: ArtifactPrefixRule::Known("P-"),
    metadata: &METADATA,
    sections: &SECTIONS,
};

const METADATA: [MetadataRule; 7] = [
    MetadataRule {
        name: "Status",
        validator: metadata_validators::artifact_status,
    },
    MetadataRule {
        name: "Goal",
        validator: exact_goal,
    },
    MetadataRule {
        name: "Branch",
        validator: nonempty,
    },
    MetadataRule {
        name: "Base branch",
        validator: nonempty,
    },
    MetadataRule {
        name: "Base commit",
        validator: full_commit,
    },
    MetadataRule {
        name: "Created",
        validator: metadata_validators::rfc3339_timestamp,
    },
    MetadataRule {
        name: "Updated",
        validator: metadata_validators::rfc3339_timestamp,
    },
];

const SECTIONS: [SectionRule; 9] = [
    SectionRule::new("Technical Summary"),
    SectionRule::new("Decisions").with_items(ItemRule::new("D")),
    SectionRule::optional("Open Questions").with_items(ItemRule::new("Q")),
    SectionRule::optional("Assumptions").with_items(ItemRule::new("A")),
    SectionRule::new("Current System"),
    SectionRule::new("Plan Items").with_items(
        ItemRule::new("P")
            .unprefixed()
            .with_metadata(&PLAN_ITEM_METADATA),
    ),
    SectionRule::new("Risks").with_items(ItemRule::new("R")),
    SectionRule::new("Cross-Cutting Validation").with_items(ItemRule::new("V")),
    SectionRule::optional("Revisions").with_items(ItemRule::new("REV")),
];

const PLAN_ITEM_METADATA: [MetadataRule; 1] = [MetadataRule {
    name: "Status",
    validator: plan_item_status,
}];

fn plan_item_status(value: &str) -> Result<(), String> {
    if matches!(value, "pending" | "done") {
        Ok(())
    } else {
        Err(format!("expected `pending` or `done`, but got `{value}`"))
    }
}

fn exact_goal(value: &str) -> Result<(), String> {
    (value == "./goal.md")
        .then_some(())
        .ok_or_else(|| format!("expected `./goal.md`, but got `{value}`"))
}

fn nonempty(value: &str) -> Result<(), String> {
    (!value.trim().is_empty())
        .then_some(())
        .ok_or_else(|| "must not be empty".to_owned())
}

fn full_commit(value: &str) -> Result<(), String> {
    (value.len() == 40 && value.bytes().all(|byte| byte.is_ascii_hexdigit()))
        .then_some(())
        .ok_or_else(|| format!("expected a full 40-character Git commit hash, but got `{value}`"))
}

pub(crate) fn parse(source: &str) -> Result<Artifact<'_>, Vec<Diagnostic>> {
    parser::parse_with_config(source, &SCHEMA.parser_config())
}

pub(crate) fn check(source: &str) -> Vec<Diagnostic> {
    let (artifact, mut diagnostics) =
        parser::parse_with_diagnostics(source, &SCHEMA.parser_config());
    diagnostics.extend(schema::validate(&artifact, &SCHEMA));
    diagnostics
}

#[cfg(test)]
mod tests;
