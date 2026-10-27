use std::path::Path;

use crate::artifact::{Artifact, FindingsBody, Metadata};
use crate::parser::{self, Diagnostic, ParserConfig, Position, SectionConfig};
use crate::schema::{
    self, metadata_validators, unpadded_decimal, ArtifactPrefixRule, ArtifactPrefixShape,
    FindingsRule, ItemRule, MetadataRule, Schema, SectionRule,
};
use crate::schemas::step::step_directory_number;

const SCHEMA: Schema = Schema {
    prefix: ArtifactPrefixRule::Unknown(ArtifactPrefixShape::Review),
    metadata: &METADATA,
    sections: &SECTIONS,
};

const METADATA: [MetadataRule; 5] = [
    MetadataRule {
        name: "Step",
        validator: step_path,
    },
    MetadataRule {
        name: "Decision",
        validator: decision,
    },
    MetadataRule {
        name: "Date",
        validator: metadata_validators::rfc3339_timestamp,
    },
    MetadataRule {
        name: "Reviewer",
        validator: reviewer,
    },
    MetadataRule {
        name: "Workspace state",
        validator: workspace_state,
    },
];

const SECTIONS: [SectionRule; 4] = [
    SectionRule::new("Findings").with_findings(FindingsRule { family: "F" }),
    SectionRule::optional("Scope").with_items(ItemRule::new("SC")),
    SectionRule::optional("Validation").with_items(ItemRule::new("V")),
    SectionRule::optional("Residual Risks").with_items(ItemRule::new("RR")),
];

pub(crate) fn parse(source: &str) -> Result<Artifact<'_>, Vec<Diagnostic>> {
    parser::parse_with_config(source, &parser_config())
}

pub(crate) fn check(path: &Path, source: &str) -> Vec<Diagnostic> {
    let (artifact, mut diagnostics) = parser::parse_with_diagnostics(source, &parser_config());
    let (schema_diagnostics, prefix) = schema::validate_with_prefix(&artifact, &SCHEMA);
    diagnostics.extend(schema_diagnostics);

    let title_number = validate_title(&mut diagnostics, &artifact);
    let filename_number = review_filename_number(path);
    if filename_number.is_none() {
        diagnostics.push(Diagnostic::error1(
            1,
            "review filename must use `reviewN.md` with a positive decimal number",
        ));
    }
    validate_decision_findings(&mut diagnostics, &artifact);

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
            "title",
            title_number,
            *review.value(),
            *review.span().start(),
            "review",
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
    if let (Some(left), Some(right)) = (title_number, filename_number) {
        if left != right {
            diagnostics.push(Diagnostic::error_p(
                Position::ZERO,
                format!("review title number `{left}` does not match filename `{right}`"),
            ));
        }
    }
    diagnostics
}

fn parser_config() -> ParserConfig<'static> {
    ParserConfig::new(vec![
        SectionConfig::findings("Findings"),
        SectionConfig::itemised("Scope"),
        SectionConfig::itemised("Validation"),
        SectionConfig::itemised("Residual Risks"),
    ])
}

fn step_path(value: &str) -> Result<(), String> {
    exact(value, "./step.md")
}

fn decision(value: &str) -> Result<(), String> {
    choices(value, &["pass", "changes-requested"])
}

fn reviewer(value: &str) -> Result<(), String> {
    choices(
        value,
        &[
            "independent subagent",
            "current session - subagent unavailable",
        ],
    )
}

fn workspace_state(value: &str) -> Result<(), String> {
    exact(value, "uncommitted")
}

fn exact(value: &str, expected: &str) -> Result<(), String> {
    if value == expected {
        Ok(())
    } else {
        Err(format!("expected `{expected}`, but got `{value}`"))
    }
}

fn choices(value: &str, expected: &[&str]) -> Result<(), String> {
    if expected.contains(&value) {
        Ok(())
    } else {
        let expected = expected
            .iter()
            .map(|choice| format!("`{choice}`"))
            .collect::<Vec<_>>()
            .join(" or ");
        Err(format!("expected {expected}, but got `{value}`"))
    }
}

fn validate_title(diagnostics: &mut Vec<Diagnostic>, artifact: &Artifact) -> Option<i64> {
    let number = artifact
        .title()
        .strip_prefix("Implementation Review ")
        .and_then(unpadded_decimal);
    if number.is_none() {
        diagnostics.push(Diagnostic::error_p(
            *artifact.located_title().span().start(),
            "review title must use `Implementation Review N` with a positive decimal number",
        ));
    }
    number
}

fn validate_decision_findings(diagnostics: &mut Vec<Diagnostic>, artifact: &Artifact) {
    let decision = artifact.get_metadata("Decision").map(Metadata::value);
    let Some(section) = artifact
        .sections()
        .iter()
        .find(|section| section.title() == "Findings")
        .and_then(|section| section.as_findings())
    else {
        return;
    };
    let (empty, sentinel) = match section.body() {
        FindingsBody::Sentinel(_) => (false, true),
        FindingsBody::Items(items) => (items.value().is_empty(), false),
    };
    if empty {
        diagnostics.push(Diagnostic::error_p(
            *section.body().span().start(),
            "section `Findings` must contain `No findings.` or at least one finding",
        ));
    }
    match decision {
        Some("pass") if !sentinel => diagnostics.push(Diagnostic::error_p(
            *section.body().span().start(),
            "decision `pass` requires the exact `No findings.` sentinel",
        )),
        Some("changes-requested") if sentinel || empty => {
            diagnostics.push(Diagnostic::error_p(
                *section.body().span().start(),
                "decision `changes-requested` requires at least one finding",
            ));
        }
        _ => {}
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
