use crate::artifact::Artifact;
use crate::parser::{self, Diagnostic, ParserConfig, SectionConfig};
use crate::schema::{
    self, metadata_validators, ArtifactPrefixRule, ItemRule, MetadataRule, Schema, SectionRule,
};

const SCHEMA: Schema = Schema {
    prefix: ArtifactPrefixRule::Known("G-"),
    metadata: &METADATA,
    sections: &SECTIONS,
};

const METADATA: [MetadataRule; 3] = [
    MetadataRule {
        name: "Status",
        validator: metadata_validators::artifact_status,
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

const SECTIONS: [SectionRule; 7] = [
    SectionRule::new("Outcome"),
    SectionRule::new("Acceptance Criteria").with_items(ItemRule::new("AC")),
    SectionRule::optional("Open Questions").with_items(ItemRule::new("Q")),
    SectionRule::optional("Assumptions").with_items(ItemRule::new("A")),
    SectionRule::optional("In Scope").with_items(ItemRule::new("IN")),
    SectionRule::optional("Out of Scope").with_items(ItemRule::new("OUT")),
    SectionRule::optional("Revisions").with_items(ItemRule::new("REV").expanded_only()),
];

pub(crate) fn parse(source: &str) -> Result<Artifact<'_>, Vec<Diagnostic>> {
    parser::parse_with_config(source, &parser_config())
}

/// Parse and validate the structural schema for a goal artifact.
///
/// Parser diagnostics and schema diagnostics are accumulated against the
/// partially parsed artifact so independent problems are reported together.
pub(crate) fn check(source: &str) -> Vec<Diagnostic> {
    let (artifact, mut diagnostics) = parser::parse_with_diagnostics(source, &parser_config());
    diagnostics.extend(schema::validate(&artifact, &SCHEMA));
    diagnostics
}

pub(crate) fn parser_config() -> ParserConfig<'static> {
    ParserConfig::new(vec![
        SectionConfig::prose("Outcome"),
        SectionConfig::itemised("Acceptance Criteria"),
        SectionConfig::itemised("In Scope"),
        SectionConfig::itemised("Out of Scope"),
        SectionConfig::itemised("Open Questions"),
        SectionConfig::itemised("Assumptions"),
        SectionConfig::itemised("Revisions"),
    ])
}

#[cfg(test)]
mod tests;
