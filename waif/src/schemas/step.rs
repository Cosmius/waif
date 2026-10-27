use std::path::Path;

use crate::artifact::Artifact;
use crate::parser::{self, Diagnostic, ParserConfig, Position, SectionConfig};
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
        validator: step_status,
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

const SECTIONS: [SectionRule; 12] = [
    SectionRule::new("Objective"),
    SectionRule::new("Plan Item Coverage").with_items(ItemRule::new("PC")),
    SectionRule::new("Context"),
    SectionRule::optional("Open Questions").with_items(ItemRule::new("Q")),
    SectionRule::optional("Assumptions").with_items(ItemRule::new("A")),
    SectionRule::new("Done When").with_items(ItemRule::new("D")),
    SectionRule::optional("Changes").with_items(ItemRule::new("C")),
    SectionRule::optional("Size and Coherence"),
    SectionRule::optional("Tests").with_items(ItemRule::new("T")),
    SectionRule::optional("Validation").with_items(ItemRule::new("V")),
    SectionRule::optional("Risks and Edge Cases").with_items(ItemRule::new("R")),
    SectionRule::optional("Revisions").with_items(ItemRule::new("REV").expanded_only()),
];

pub(crate) fn parse(source: &str) -> Result<Artifact<'_>, Vec<Diagnostic>> {
    parser::parse_with_config(source, &parser_config())
}

pub(crate) fn check(path: &Path, source: &str) -> Vec<Diagnostic> {
    let (artifact, mut diagnostics) = parser::parse_with_diagnostics(source, &parser_config());
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

fn parser_config() -> ParserConfig<'static> {
    ParserConfig::new(vec![
        SectionConfig::prose("Objective"),
        SectionConfig::itemised("Plan Item Coverage"),
        SectionConfig::prose("Context"),
        SectionConfig::itemised("Open Questions"),
        SectionConfig::itemised("Assumptions"),
        SectionConfig::itemised("Done When"),
        SectionConfig::itemised("Changes"),
        SectionConfig::prose("Size and Coherence"),
        SectionConfig::itemised("Tests"),
        SectionConfig::itemised("Validation"),
        SectionConfig::itemised("Risks and Edge Cases"),
        SectionConfig::itemised("Revisions"),
    ])
}

fn step_status(value: &str) -> Result<(), String> {
    if matches!(value, "drafting" | "accepted" | "amending" | "done") {
        Ok(())
    } else {
        Err(format!(
            "expected `drafting`, `accepted`, `amending`, or `done`, but got `{value}`"
        ))
    }
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

fn validate_coverage(diagnostics: &mut Vec<Diagnostic>, artifact: &Artifact) {
    let Some(section) = artifact
        .sections()
        .iter()
        .find(|section| section.title() == "Plan Item Coverage")
        .and_then(|section| section.as_itemised())
    else {
        return;
    };
    for item in section.items() {
        let id_number = item
            .identifier()
            .and_then(|id| id.text().rsplit_once("-PC"))
            .and_then(|(_, number)| unpadded_decimal(number));
        let short_description = item.short_description().text();
        let parsed = short_description
            .strip_prefix('P')
            .and_then(|description| description.split_once(" - "))
            .and_then(|(number, rest)| {
                let number = unpadded_decimal(number)?;
                let (coverage, description) = rest.split_once(" - ")?;
                matches!(coverage, "partial" | "complete").then_some((number, description))
            });
        match (id_number, parsed) {
            (Some(id), Some((plan, description))) if !description.trim().is_empty() => {
                if id != plan {
                    diagnostics.push(Diagnostic::error_p(
                        *item.span().start(),
                        format!("coverage ID suffix `{id}` must match covered plan ID `P{plan}`"),
                    ));
                }
            }
            (Some(_), _) => diagnostics.push(Diagnostic::error_p(
                *item.span().start(),
                "coverage entry must use `P<number> - partial | complete - <description>`",
            )),
            _ => {}
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
