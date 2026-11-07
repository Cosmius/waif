use std::path::Path;

use crate::artifact::{Artifact, Located, Metadata};
use crate::macro_processor;
use crate::parser::{self, Diagnostic, Position};
use crate::schema::{
    self, metadata_validators, unpadded_decimal, ArtifactPrefixRule, ArtifactPrefixShape, ItemRule,
    MetadataRule, Schema, SectionRule,
};

const SCHEMA: Schema = Schema {
    prefix: ArtifactPrefixRule::Unknown(ArtifactPrefixShape::Step),
    metadata: &METADATA,
    sections: &SECTIONS,
};

const METADATA: [MetadataRule; 4] = [
    MetadataRule {
        name: "Status",
        validator: metadata_validators::artifact_status,
    },
    MetadataRule {
        name: "Source commit",
        validator: source_commit,
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

const SECTIONS: [SectionRule; 13] = [
    SectionRule::new("Objective"),
    SectionRule::new("Plan Item Coverage")
        .with_items(ItemRule::new("PC").with_metadata(&COVERAGE_METADATA)),
    SectionRule::new("Done Conditions").with_items(ItemRule::new("DC")),
    SectionRule::new("Context"),
    SectionRule::optional("Key Decisions").with_items(ItemRule::new("K")),
    SectionRule::optional("Open Questions").with_items(ItemRule::new("Q")),
    SectionRule::optional("Assumptions").with_items(ItemRule::new("A")),
    SectionRule::optional("Changes").with_items(ItemRule::new("C")),
    SectionRule::optional("Tests").with_items(ItemRule::new("T")),
    SectionRule::optional("Validation").with_items(ItemRule::new("V")),
    SectionRule::optional("Size and Coherence"),
    SectionRule::optional("Risks and Edge Cases").with_items(ItemRule::new("R")),
    SectionRule::optional("Revisions").with_items(ItemRule::new("REV")),
];

const COVERAGE_METADATA: [MetadataRule; 2] = [
    MetadataRule {
        name: "Plan item",
        validator: any_value,
    },
    MetadataRule {
        name: "Coverage",
        validator: coverage,
    },
];

/// Accept any value of a key that only needs to be present.
fn any_value(_: &str) -> Result<(), String> {
    Ok(())
}

fn coverage(value: &str) -> Result<(), String> {
    match value {
        "partial" | "complete" => Ok(()),
        _ => Err(format!(
            "expected `partial` or `complete`, but got `{value}`"
        )),
    }
}

/// A `Plan Item Coverage` entry. `plan_item` and `coverage` are `None` when
/// the key is missing or repeated.
pub(crate) struct CoverageEntry<'s, 'a> {
    pub(crate) identifier: Option<&'s Located<&'a str>>,
    pub(crate) plan_item: Option<&'s Metadata<'a>>,
    pub(crate) coverage: Option<&'s str>,
}

/// Read the coverage of every expanded `Plan Item Coverage` entry in source
/// order. Placeholders carry no coverage and are skipped.
pub(crate) fn coverage_entries<'s, 'a>(step: &'s Artifact<'a>) -> Vec<CoverageEntry<'s, 'a>> {
    step.sections()
        .iter()
        .find(|section| section.title() == "Plan Item Coverage")
        .and_then(|section| section.as_itemised())
        .map_or(&[][..], |section| section.items())
        .iter()
        .filter_map(|item| {
            let expanded = item.as_expanded()?;
            Some(CoverageEntry {
                identifier: expanded.identifier(),
                plan_item: expanded.metadatum("Plan item"),
                coverage: expanded.metadatum("Coverage").map(Metadata::value),
            })
        })
        .collect()
}

pub(crate) fn parse(source: &str) -> Result<Artifact<'_>, Vec<Diagnostic>> {
    parser::parse_with_config(source, &SCHEMA.parser_config())
}

pub(crate) fn check(path: &Path, source: &str) -> Vec<Diagnostic> {
    let (artifact, mut diagnostics) =
        parser::parse_with_diagnostics(source, &SCHEMA.parser_config());
    let (schema_diagnostics, prefix) = schema::validate_with_prefix(&artifact, &SCHEMA);
    diagnostics.extend(schema_diagnostics);

    let title_number = validate_title(&mut diagnostics, &artifact);
    validate_lifecycle(&mut diagnostics, &artifact);
    validate_coverage(&mut diagnostics, &artifact);

    let path_number = step_directory_number(&mut diagnostics, path);
    if path_number.is_none() {
        diagnostics.push(Diagnostic::warning_p(
            Position::ZERO,
            "cannot resolve a step number from the containing directory; \
             skipping path-dependent identity checks",
        ));
    }
    if let Some(observed) = prefix {
        let component = &observed.components()[0];
        let observed_number = *component.value();
        let pos = *component.span().start();
        compare_identity(
            "title",
            title_number,
            observed_number,
            pos,
            &mut diagnostics,
        );
        compare_identity(
            "containing directory",
            path_number,
            observed_number,
            pos,
            &mut diagnostics,
        );
    }
    if let (Some(title), Some(path)) = (title_number, path_number) {
        if title != path {
            diagnostics.push(Diagnostic::error_p(
                Position::ZERO,
                format!("step title number `{title}` does not match containing directory `{path}`"),
            ));
        }
    }
    diagnostics
}

fn source_commit(value: &str) -> Result<(), String> {
    if value == "not-created"
        || (value.len() == 40 && value.bytes().all(|byte| byte.is_ascii_hexdigit()))
    {
        Ok(())
    } else {
        Err(format!(
            "expected `not-created` or a full 40-character Git commit hash, but got `{value}`"
        ))
    }
}

fn validate_title(diagnostics: &mut Vec<Diagnostic>, artifact: &Artifact) -> Option<i64> {
    let number = artifact
        .title()
        .strip_prefix("Step ")
        .and_then(|title| title.split_once(": "))
        .filter(|(_, title)| !title.is_empty())
        .and_then(|(number, _)| unpadded_decimal(number));
    if number.is_none() {
        diagnostics.push(Diagnostic::error_p(
            Position::ZERO,
            "step title must use `Step N: <non-empty title>` with a positive step number",
        ));
    }
    number
}

fn validate_lifecycle(diagnostics: &mut Vec<Diagnostic>, artifact: &Artifact) {
    let status = artifact.get_metadata("Status");
    let commit = artifact.get_metadata("Source commit");
    let Some((status, commit)) = status.zip(commit) else {
        return;
    };
    let valid = if status.value() == "done" {
        commit.value().len() == 40 && commit.value().bytes().all(|byte| byte.is_ascii_hexdigit())
    } else if matches!(status.value(), "drafting" | "accepted" | "amending") {
        commit.value() == "not-created"
    } else {
        true
    };
    if !valid {
        diagnostics.push(Diagnostic::error_p(
            *commit.located_value().span().start(),
            format!(
                "metadata `Source commit` is incompatible with step status `{}`",
                status.value()
            ),
        ));
    }
}

/// Report a `Plan item` that is not `P<m>` for coverage entry `S<n>-PC<m>`.
fn validate_coverage(diagnostics: &mut Vec<Diagnostic>, artifact: &Artifact) {
    for entry in coverage_entries(artifact) {
        let (Some(identifier), Some(plan_item)) = (entry.identifier, entry.plan_item) else {
            continue;
        };
        let Some(number) = identifier
            .text()
            .rsplit_once("-PC")
            .and_then(|(_, number)| unpadded_decimal(number))
        else {
            continue;
        };
        let value = plan_item.value();
        let expected = format!("P{number}");
        if value != expected && macro_processor::scan(value).is_empty() {
            diagnostics.push(Diagnostic::error_p(
                *plan_item.located_value().span().start(),
                format!(
                    "item metadata `Plan item` expected `{expected}` for coverage entry `{}`, \
                     but got `{value}`",
                    identifier.text()
                ),
            ));
        }
    }
}

pub(crate) fn step_directory_number(diagnostics: &mut Vec<Diagnostic>, path: &Path) -> Option<i64> {
    let name = path.parent()?.file_name()?.to_str()?;
    let (number, short_name) = name.split_once('-')?;
    let number = (number.len() >= 2 && number.bytes().all(|byte| byte.is_ascii_digit()))
        .then(|| number.parse().ok())
        .flatten()
        .filter(|number| *number > 0)?;
    if short_name.is_empty() {
        diagnostics.push(Diagnostic::warning_p(
            Position::ZERO,
            "step directory short name is empty",
        ));
    }
    Some(number)
}

fn compare_identity(
    source: &str,
    expected: Option<i64>,
    observed: i64,
    position: Position,
    diagnostics: &mut Vec<Diagnostic>,
) {
    if expected.is_some_and(|expected| expected != observed) {
        diagnostics.push(Diagnostic::error_p(
            position,
            format!("step item prefix `S{observed}-` does not match {source} identity"),
        ));
    }
}

#[cfg(test)]
mod tests;
