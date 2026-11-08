use std::path::Path;

use crate::artifact::Artifact;
use crate::parser::{self, Diagnostic, Position};
use crate::schema::{
    self, metadata_validators, unpadded_decimal, ArtifactPrefixRule, ArtifactPrefixShape, ItemRule,
    MetadataRule, Schema, SectionRule,
};
use crate::schemas::step::step_directory_number;

const SCHEMA: Schema = Schema {
    prefix: ArtifactPrefixRule::Unknown(ArtifactPrefixShape::Review),
    metadata: &METADATA,
    sections: &SECTIONS,
};

const METADATA: [MetadataRule; 4] = [
    MetadataRule {
        name: "Date",
        validator: metadata_validators::rfc3339_timestamp,
    },
    MetadataRule {
        name: "Verdict",
        validator: verdict,
    },
    MetadataRule {
        name: "Reviewed",
        validator: reference,
    },
    MetadataRule {
        name: "Reviewed against",
        validator: reference,
    },
];

const SECTIONS: [SectionRule; 6] = [
    SectionRule::new("Summary"),
    SectionRule::new("Findings").with_items(ItemRule::new("F").with_sentinel("No findings.")),
    SectionRule::new("Assessment"),
    SectionRule::optional("Review History"),
    SectionRule::optional("Validation").with_items(ItemRule::new("V")),
    SectionRule::optional("Residual Risks").with_items(ItemRule::new("RR")),
];

pub(crate) fn parse(source: &str) -> Result<Artifact<'_>, Vec<Diagnostic>> {
    parser::parse_with_config(source, &SCHEMA.parser_config())
}

pub(crate) fn check(path: &Path, source: &str) -> Vec<Diagnostic> {
    let (artifact, mut diagnostics) =
        parser::parse_with_diagnostics(source, &SCHEMA.parser_config());
    let (schema_diagnostics, prefix) = schema::validate_with_prefix(&artifact, &SCHEMA);
    diagnostics.extend(schema_diagnostics);

    let filename_number = review_filename_number(path);
    if filename_number.is_none() {
        diagnostics.push(Diagnostic::error1(
            1,
            "review filename must use `reviewN.md` with a positive decimal number",
        ));
    }

    let path_step_number = step_directory_number(&mut diagnostics, path);
    if path_step_number.is_none() {
        diagnostics.push(Diagnostic::warning_p(
            Position::ZERO,
            "cannot resolve a step number from the containing directory; \
             skipping path-dependent identity checks",
        ));
    }
    if let Some(observed) = prefix {
        let step = &observed.components()[0];
        let review = &observed.components()[1];
        compare_identity(
            "containing directory",
            path_step_number,
            *step.value(),
            *step.span().start(),
            "step",
            &mut diagnostics,
        );
        compare_identity(
            "filename",
            filename_number,
            *review.value(),
            *review.span().start(),
            "review",
            &mut diagnostics,
        );
    }
    diagnostics
}

fn verdict(value: &str) -> Result<(), String> {
    match value {
        "pass" | "concerns" => Ok(()),
        _ => Err(format!("expected `pass` or `concerns`, but got `{value}`")),
    }
}

/// Accept a REF: a full commit hash, optionally followed by `-STAGING` or
/// `-WORKING`. Text after the first space, such as `(HEAD, main)`, is not
/// checked.
fn reference(value: &str) -> Result<(), String> {
    let revision = value
        .split_once(' ')
        .map_or(value, |(revision, _)| revision);
    let hash = revision
        .strip_suffix("-STAGING")
        .or_else(|| revision.strip_suffix("-WORKING"))
        .unwrap_or(revision);
    if hash.len() == 40 && hash.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        Ok(())
    } else {
        Err(format!(
            "expected a full 40-character Git commit hash, optionally followed by \
             `-STAGING` or `-WORKING`, but got `{value}`"
        ))
    }
}

fn review_filename_number(path: &Path) -> Option<i64> {
    path.file_name()?
        .to_str()?
        .strip_prefix("review")?
        .strip_suffix(".md")
        .and_then(unpadded_decimal)
}

fn compare_identity(
    source: &str,
    expected: Option<i64>,
    observed: i64,
    position: Position,
    component: &str,
    diagnostics: &mut Vec<Diagnostic>,
) {
    if expected.is_some_and(|expected| expected != observed) {
        diagnostics.push(Diagnostic::error_p(
            position,
            format!("review item {component} identity `{observed}` does not match {source}"),
        ));
    }
}

#[cfg(test)]
mod tests;
