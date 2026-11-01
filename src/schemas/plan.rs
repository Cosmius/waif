use crate::artifact::Artifact;
use crate::parser::{self, Diagnostic, ParserConfig, SectionConfig};
use crate::schema::{
    self, metadata_validators, ArtifactPrefixRule, ItemRule, MetadataRule, PlanItemRule, Schema,
    SectionRule,
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
    SectionRule::new("Decisions").with_items(ItemRule::new("D").with_expanded_family("DD").mixed()),
    SectionRule::optional("Open Questions").with_items(ItemRule::new("Q")),
    SectionRule::optional("Assumptions").with_items(ItemRule::new("A")),
    SectionRule::new("Current System"),
    SectionRule::new("Plan Items").with_plan_items(PlanItemRule {
        statuses: &["pending", "done"],
    }),
    SectionRule::new("Risks").with_items(ItemRule::new("R")),
    SectionRule::new("Cross-Cutting Validation").with_items(ItemRule::new("V")),
    SectionRule::optional("Revisions").with_items(ItemRule::new("REV").expanded_only()),
];

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
    parser::parse_with_config(source, &parser_config())
}

pub(crate) fn check(source: &str) -> Vec<Diagnostic> {
    let (artifact, mut diagnostics) = parser::parse_with_diagnostics(source, &parser_config());
    diagnostics.extend(schema::validate(&artifact, &SCHEMA));
    diagnostics
}

fn parser_config() -> ParserConfig<'static> {
    ParserConfig::new(vec![
        SectionConfig::prose("Technical Summary"),
        SectionConfig::itemised("Decisions"),
        SectionConfig::itemised("Open Questions"),
        SectionConfig::itemised("Assumptions"),
        SectionConfig::prose("Current System"),
        SectionConfig::plan_items("Plan Items"),
        SectionConfig::itemised("Risks"),
        SectionConfig::itemised("Cross-Cutting Validation"),
        SectionConfig::itemised("Revisions"),
    ])
}

#[cfg(test)]
mod tests;
