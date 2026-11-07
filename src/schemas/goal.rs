use crate::artifact::Artifact;
use crate::parser::{self, Diagnostic};
use crate::schema::{
    self, metadata_validators, ArtifactPrefixRule, ItemRule, MetadataRule, Schema, SectionRule,
};

pub(crate) const SCHEMA: Schema = Schema {
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
    schema::QUESTIONS,
    SectionRule::optional("Assumptions").with_items(ItemRule::new("A")),
    SectionRule::optional("In Scope").with_items(ItemRule::new("IN")),
    SectionRule::optional("Out of Scope").with_items(ItemRule::new("OUT")),
    SectionRule::optional("Revisions").with_items(ItemRule::new("REV")),
];

pub(crate) fn parse(source: &str) -> Result<Artifact<'_>, Vec<Diagnostic>> {
    parser::parse_with_config(source, &SCHEMA.parser_config())
}

/// Parse and validate the structural schema for a goal artifact.
///
/// Parser diagnostics and schema diagnostics are accumulated against the
/// partially parsed artifact so independent problems are reported together.
pub(crate) fn check(source: &str) -> Vec<Diagnostic> {
    let (artifact, mut diagnostics) =
        parser::parse_with_diagnostics(source, &SCHEMA.parser_config());
    diagnostics.extend(schema::validate(&artifact, &SCHEMA));
    diagnostics
}

#[cfg(test)]
mod tests;
