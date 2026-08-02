use std::path::Path;

use crate::artifact::{Artifact, FindingsBody, Metadata};
use crate::parser::{self, Diagnostic, ParserConfig, SectionConfig};
use crate::schema::{
    self, ArtifactPrefixRule, ArtifactPrefixShape, FindingsRule, ItemRule, MetadataRule, Schema,
    SectionRule,
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
        validator: schema::rfc3339_timestamp,
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
    SectionRule::new("Scope")
        .optional()
        .with_items(ItemRule::new("SC")),
    SectionRule::new("Validation")
        .optional()
        .with_items(ItemRule::new("V")),
    SectionRule::new("Residual Risks")
        .optional()
        .with_items(ItemRule::new("RR")),
];

const SCHEMA: Schema = Schema {
    prefix: ArtifactPrefixRule::Unknown(ArtifactPrefixShape::Review),
    metadata: &METADATA,
    sections: &SECTIONS,
};

pub(crate) fn check(path: &Path, source: &str) -> Vec<Diagnostic> {
    let (artifact, mut diagnostics) = parser::parse_with_diagnostics(source, &parser_config());
    let (schema_diagnostics, prefix) = schema::validate_with_prefix(&artifact, &SCHEMA);
    diagnostics.extend(schema_diagnostics);

    let title_number = validate_title(&artifact, &mut diagnostics);
    let filename_number = review_filename_number(path);
    if filename_number.is_none() {
        diagnostics.push(Diagnostic::error1(
            1,
            "review filename must use `reviewN.md` with a positive decimal number",
        ));
    }
    validate_metadata_order(&artifact, &mut diagnostics);
    validate_decision_findings(&artifact, &mut diagnostics);

    let path_step_number = step_directory_number(path);
    if path_step_number.is_none() {
        diagnostics.push(Diagnostic::warning1(
            1,
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
            step.span().start_line(),
            "step",
            &mut diagnostics,
        );
        compare_identity(
            "title",
            title_number,
            *review.value(),
            review.span().start_line(),
            "review",
            &mut diagnostics,
        );
        compare_identity(
            "filename",
            filename_number,
            *review.value(),
            review.span().start_line(),
            "review",
            &mut diagnostics,
        );
    }
    compare_pair(
        "review title number",
        title_number,
        "filename",
        filename_number,
        &mut diagnostics,
    );
    diagnostics
}

fn parser_config() -> ParserConfig<'static> {
    ParserConfig::new(vec![
        SectionConfig::findings("Findings"),
        SectionConfig::itemised("Scope"),
        SectionConfig::itemised("Validation"),
        SectionConfig::itemised("Residual Risks"),
    ])
    .with_known_metadata(["Step", "Decision", "Date", "Reviewer", "Workspace state"])
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

fn validate_title(artifact: &Artifact, diagnostics: &mut Vec<Diagnostic>) -> Option<i64> {
    let number = artifact
        .title()
        .strip_prefix("Implementation Review ")
        .and_then(positive_number);
    if number.is_none() {
        diagnostics.push(Diagnostic::error1(
            1,
            "review title must use `Implementation Review N` with a positive decimal number",
        ));
    }
    number
}

fn validate_metadata_order(artifact: &Artifact, diagnostics: &mut Vec<Diagnostic>) {
    let mut greatest = None;
    for entry in artifact.metadata() {
        let Some(rank) = METADATA
            .iter()
            .position(|rule| rule.name == entry.value().key())
        else {
            continue;
        };
        if greatest.is_some_and(|previous| rank < previous) {
            diagnostics.push(Diagnostic::error1(
                entry.span().start_line(),
                format!("metadata `{}` is out of order", entry.value().key()),
            ));
        }
        greatest = Some(greatest.map_or(rank, |previous: usize| previous.max(rank)));
    }
}

fn validate_decision_findings(artifact: &Artifact, diagnostics: &mut Vec<Diagnostic>) {
    let decision = metadata(artifact, "Decision").map(Metadata::value);
    let Some(section) = artifact
        .sections()
        .iter()
        .find(|section| section.name() == "Findings")
        .and_then(|section| section.as_findings())
    else {
        return;
    };
    let (empty, sentinel) = match section.body() {
        FindingsBody::Sentinel(_) => (false, true),
        FindingsBody::Items(items) => (items.value().is_empty(), false),
    };
    if empty {
        diagnostics.push(Diagnostic::error1(
            section.body().span().start_line(),
            "section `Findings` must contain `No findings.` or at least one finding",
        ));
    }
    match decision {
        Some("pass") if !sentinel => diagnostics.push(Diagnostic::error1(
            section.body().span().start_line(),
            "decision `pass` requires the exact `No findings.` sentinel",
        )),
        Some("changes-requested") if sentinel || empty => {
            diagnostics.push(Diagnostic::error1(
                section.body().span().start_line(),
                "decision `changes-requested` requires at least one finding",
            ));
        }
        _ => {}
    }
}

fn metadata<'a>(artifact: &'a Artifact, key: &str) -> Option<&'a Metadata> {
    let mut entries = artifact
        .metadata()
        .iter()
        .filter(|entry| entry.value().key() == key);
    let entry = entries.next()?;
    entries.next().is_none().then_some(entry.value())
}

fn review_filename_number(path: &Path) -> Option<i64> {
    path.file_name()?
        .to_str()?
        .strip_prefix("review")?
        .strip_suffix(".md")
        .and_then(positive_number)
}

fn step_directory_number(path: &Path) -> Option<i64> {
    let name = path.parent()?.file_name()?.to_str()?;
    let (number, short_name) = name.split_once('-')?;
    if short_name.is_empty()
        || short_name.starts_with('-')
        || short_name.ends_with('-')
        || short_name.contains("--")
        || !short_name
            .bytes()
            .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'-')
    {
        return None;
    }
    (number.len() >= 2)
        .then(|| positive_number(number))
        .flatten()
}

fn positive_number(number: &str) -> Option<i64> {
    (!number.is_empty() && number.bytes().all(|byte| byte.is_ascii_digit()))
        .then(|| number.parse().ok())
        .flatten()
        .filter(|number| *number > 0)
}

fn compare_identity(
    source: &str,
    expected: Option<i64>,
    observed: i64,
    line: usize,
    component: &str,
    diagnostics: &mut Vec<Diagnostic>,
) {
    if expected.is_some_and(|expected| expected != observed) {
        diagnostics.push(Diagnostic::error1(
            line,
            format!("review item {component} identity `{observed}` does not match {source}"),
        ));
    }
}

fn compare_pair(
    left_name: &str,
    left: Option<i64>,
    right_name: &str,
    right: Option<i64>,
    diagnostics: &mut Vec<Diagnostic>,
) {
    if let (Some(left), Some(right)) = (left, right) {
        if left != right {
            diagnostics.push(Diagnostic::error1(
                1,
                format!("{left_name} `{left}` does not match {right_name} `{right}`"),
            ));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::Severity;

    const PASS: &str = concat!(
        "# Implementation Review 1\n",
        "- Step: ./step.md\n",
        "- Decision: pass\n",
        "- Date: 2026-08-02T13:00:00+09:00\n",
        "- Reviewer: independent subagent\n",
        "- Workspace state: uncommitted\n",
        "## Findings\nNo findings.\n",
        "## Scope\n- S4-R1-SC1: review checker\n",
        "## Validation\n- S4-R1-V1: tests passed\n",
        "## Residual Risks\n- S4-R1-RR1: None.\n",
    );

    #[test]
    fn accepts_pass_and_changes_requested_reviews() {
        assert!(check(Path::new("steps/04-check/review1.md"), PASS).is_empty());

        let changes = PASS
            .replace("- Decision: pass", "- Decision: changes-requested")
            .replace(
                "No findings.",
                "### S4-R1-F1: Defect\n\nOpaque severity and body.",
            );
        assert!(check(Path::new("steps/04-check/review1.md"), &changes).is_empty());
    }

    #[test]
    fn aggregates_metadata_section_and_decision_errors() {
        let source = PASS
            .replace("- Step: ./step.md\n", "- Step: wrong\n")
            .replace(
                "- Decision: pass\n",
                "- Reviewer: robot\n- Decision: pass\n",
            )
            .replace("- Reviewer: independent subagent\n", "")
            .replace("- Workspace state: uncommitted", "- Workspace state: dirty")
            .replace("No findings.", "### S5-R2-F1: defect")
            .replace("## Scope", "## Scope\n## Findings\nNo findings.\n## Scope");
        let diagnostics = check(Path::new("steps/bad/review2.md"), &source);
        let messages = diagnostics
            .iter()
            .map(Diagnostic::message)
            .collect::<Vec<_>>()
            .join("\n");

        for expected in [
            "metadata `Step` expected `./step.md`",
            "metadata `Reviewer` expected",
            "metadata `Decision` is out of order",
            "metadata `Workspace state` expected `uncommitted`",
            "duplicate section `Findings`",
            "duplicate section `Scope`",
            "decision `pass` requires",
            "cannot resolve a step number",
        ] {
            assert!(
                messages.contains(expected),
                "missing {expected}: {messages}"
            );
        }
        assert!(diagnostics
            .iter()
            .any(|diagnostic| diagnostic.severity() == Severity::Error));
    }

    #[test]
    fn validates_title_filename_path_and_prefix_identities() {
        let source = PASS
            .replace("Review 1", "Review 3")
            .replace("S4-R1-", "S5-R2-");
        let diagnostics = check(Path::new("steps/04-check/review2.md"), &source);
        let messages = diagnostics
            .iter()
            .map(Diagnostic::message)
            .collect::<Vec<_>>()
            .join("\n");

        assert!(messages.contains("does not match containing directory"));
        assert!(messages.contains("does not match title"));
        assert!(messages.contains("review title number `3` does not match filename `2`"));
    }

    #[test]
    fn rejects_empty_findings_and_mixed_review_prefixes() {
        let empty = PASS.replace("No findings.\n", "");
        let diagnostics = check(Path::new("steps/04-check/review1.md"), &empty);
        assert!(diagnostics.iter().any(|diagnostic| diagnostic
            .message()
            .contains("section `Findings` must contain")));

        let mixed = PASS.replace("S4-R1-V1", "S4-R2-V1");
        let diagnostics = check(Path::new("steps/04-check/review1.md"), &mixed);
        assert!(diagnostics
            .iter()
            .any(|diagnostic| diagnostic.message().contains("item identifier `S4-R2-V1`")));
    }
}
