use std::path::Path;

use crate::artifact::{Artifact, Metadata};
use crate::parser::{self, Diagnostic, ParserConfig, SectionConfig};
use crate::schema::{
    self, ArtifactPrefixRule, ArtifactPrefixShape, ItemRule, MetadataRule, Schema, SectionRule,
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
        validator: schema::rfc3339_timestamp,
    },
    MetadataRule {
        name: "Updated",
        validator: schema::rfc3339_timestamp,
    },
];

const SECTIONS: [SectionRule; 12] = [
    SectionRule::new("Objective"),
    SectionRule::new("Plan Item Coverage").with_items(ItemRule::new("PC")),
    SectionRule::new("Context"),
    SectionRule::new("Open Questions")
        .optional()
        .with_items(ItemRule::new("Q")),
    SectionRule::new("Assumptions")
        .optional()
        .with_items(ItemRule::new("A")),
    SectionRule::new("Done When").with_items(ItemRule::new("D")),
    SectionRule::new("Changes")
        .optional()
        .with_items(ItemRule::new("C")),
    SectionRule::new("Size and Coherence").optional(),
    SectionRule::new("Tests")
        .optional()
        .with_items(ItemRule::new("T")),
    SectionRule::new("Validation")
        .optional()
        .with_items(ItemRule::new("V")),
    SectionRule::new("Risks and Edge Cases")
        .optional()
        .with_items(ItemRule::new("R")),
    SectionRule::new("Revisions")
        .optional()
        .with_items(ItemRule::new("REV").expanded_only()),
];

const SCHEMA: Schema = Schema {
    prefix: ArtifactPrefixRule::Unknown(ArtifactPrefixShape::Step),
    metadata: &METADATA,
    sections: &SECTIONS,
};

pub(crate) fn check(path: &Path, source: &str) -> Vec<Diagnostic> {
    let (artifact, mut diagnostics) = parser::parse_with_diagnostics(source, &parser_config());
    let (schema_diagnostics, prefix) = schema::validate_with_prefix(&artifact, &SCHEMA);
    diagnostics.extend(schema_diagnostics);

    let title_number = validate_title(&artifact, &mut diagnostics);
    validate_metadata_order(&artifact, &mut diagnostics);
    validate_lifecycle(&artifact, &mut diagnostics);
    validate_coverage(&artifact, &mut diagnostics);

    let path_number = step_directory_number(path);
    if path_number.is_none() {
        diagnostics.push(Diagnostic::warning1(
            1,
            "cannot resolve a step number from the containing directory; \
             skipping path-dependent identity checks",
        ));
    }
    if let Some(observed) = prefix {
        let component = &observed.components()[0];
        let observed_number = *component.value();
        let line = component.span().start_line();
        compare_identity(
            "title",
            title_number,
            observed_number,
            line,
            &mut diagnostics,
        );
        compare_identity(
            "containing directory",
            path_number,
            observed_number,
            line,
            &mut diagnostics,
        );
    }
    if let (Some(title), Some(path)) = (title_number, path_number) {
        if title != path {
            diagnostics.push(Diagnostic::error1(
                1,
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

fn validate_title(artifact: &Artifact, diagnostics: &mut Vec<Diagnostic>) -> Option<i64> {
    let number = artifact
        .title()
        .strip_prefix("Step ")
        .and_then(|title| title.split_once(": "))
        .filter(|(_, title)| !title.is_empty())
        .and_then(|(number, _)| padded_step_number(number));
    if number.is_none() {
        diagnostics.push(Diagnostic::error1(
            1,
            "step title must use `Step NN: <non-empty title>` with a positive, \
             at-least-two-digit step number",
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

fn validate_lifecycle(artifact: &Artifact, diagnostics: &mut Vec<Diagnostic>) {
    let status = metadata(artifact, "Status");
    let commit = metadata(artifact, "Source commit");
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
        diagnostics.push(Diagnostic::error1(
            commit.located_value().span().start_line(),
            format!(
                "metadata `Source commit` is incompatible with step status `{}`",
                status.value()
            ),
        ));
    }
}

fn validate_coverage(artifact: &Artifact, diagnostics: &mut Vec<Diagnostic>) {
    let Some(section) = artifact
        .sections()
        .iter()
        .find(|section| section.name() == "Plan Item Coverage")
        .and_then(|section| section.as_itemised())
    else {
        return;
    };
    for item in section.items() {
        let id_number = item
            .identifier()
            .and_then(|id| id.text().rsplit_once("-PC"))
            .and_then(|(_, number)| positive_number(number));
        let short_description = item.short_description().text();
        let parsed = short_description
            .strip_prefix('P')
            .and_then(|description| description.split_once(" - "))
            .and_then(|(number, rest)| {
                let number = positive_number(number)?;
                let (coverage, description) = rest.split_once(" - ")?;
                matches!(coverage, "partial" | "complete").then_some((number, description))
            });
        match (id_number, parsed) {
            (Some(id), Some((plan, description))) if !description.trim().is_empty() => {
                if id != plan {
                    diagnostics.push(Diagnostic::error1(
                        item.span().start_line(),
                        format!("coverage ID suffix `{id}` must match covered plan ID `P{plan}`"),
                    ));
                }
            }
            (Some(_), _) => diagnostics.push(Diagnostic::error1(
                item.span().start_line(),
                "coverage entry must use `P<number> - partial | complete - <description>`",
            )),
            _ => {}
        }
    }
}

fn metadata<'a, 'b>(artifact: &'a Artifact<'b>, key: &str) -> Option<&'a Metadata<'b>> {
    let mut entries = artifact
        .metadata()
        .iter()
        .filter(|entry| entry.value().key() == key);
    let entry = entries.next()?;
    entries.next().is_none().then_some(entry.value())
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
    padded_step_number(number)
}

fn padded_step_number(number: &str) -> Option<i64> {
    (number.len() >= 2 && number.bytes().all(|byte| byte.is_ascii_digit()))
        .then(|| number.parse().ok())
        .flatten()
        .filter(|number| *number > 0)
}

fn positive_number(number: &str) -> Option<i64> {
    let mut bytes = number.bytes();
    matches!(bytes.next(), Some(b'1'..=b'9'))
        .then(|| bytes.all(|byte| byte.is_ascii_digit()))
        .filter(|valid| *valid)
        .and_then(|_| number.parse().ok())
}

fn compare_identity(
    source: &str,
    expected: Option<i64>,
    observed: i64,
    line: usize,
    diagnostics: &mut Vec<Diagnostic>,
) {
    if expected.is_some_and(|expected| expected != observed) {
        diagnostics.push(Diagnostic::error1(
            line,
            format!("step item prefix `S{observed}-` does not match {source} identity"),
        ));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::Severity;

    const VALID: &str = concat!(
        "# Step 03: Check steps\n",
        "- Status: accepted\n",
        "- Source commit: not-created\n",
        "- Created: 2026-08-02T12:00:00+09:00\n",
        "- Updated: 2026-08-02T12:01:00+09:00\n",
        "- Plan items: P2\n- Goal criteria: opaque\n",
        "## Objective\ntext\n",
        "## Plan Item Coverage\n- S3-PC2: P2 - complete - checker\n",
        "## Context\ntext\n",
        "## Done When\n- S3-D1: done\n",
    );

    fn messages(path: &str, source: &str) -> Vec<(Severity, usize, String)> {
        check(Path::new(path), source)
            .into_iter()
            .map(|diagnostic| {
                (
                    diagnostic.severity(),
                    diagnostic.line(),
                    diagnostic.message().to_owned(),
                )
            })
            .collect()
    }

    #[test]
    fn accepts_conforming_step_and_opaque_semantic_metadata() {
        assert_eq!(messages("steps/03-check/step.md", VALID), []);
        let expanded = VALID.replace(
            "- S3-D1: done",
            "### S3-D1: done\n\nOpaque body with `## heading`.",
        );
        assert_eq!(messages("steps/03-check/step.md", &expanded), []);
    }

    #[test]
    fn aggregates_lifecycle_coverage_order_and_identity_errors() {
        let source = VALID
            .replace("- Status: accepted", "- Updated: invalid\n- Status: done")
            .replace("- Updated: 2026-08-02T12:01:00+09:00\n", "")
            .replace("not-created", "abc")
            .replace("S3-PC2: P2", "S4-PC2: P3")
            .replace("S3-D1", "S4-D1");
        let found = messages("steps/03-check/step.md", &source);
        assert!(found.len() >= 5, "{found:?}");
        assert!(found.iter().any(|entry| entry.2.contains("out of order")));
        assert!(found.iter().any(|entry| entry.2.contains("incompatible")));
        assert!(found
            .iter()
            .any(|entry| entry.2.contains("must match covered")));
        assert!(found
            .iter()
            .any(|entry| entry.2.contains("containing directory")));
        assert!(found
            .iter()
            .any(|entry| { entry.1 == 11 && entry.2.contains("containing directory") }));
    }

    #[test]
    fn malformed_directory_warns_but_keeps_internal_prefix_checks() {
        let found = messages("steps/not-a-step/step.md", VALID);
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].0, Severity::Warning);
        assert!(found[0].2.contains("cannot resolve a step number"));

        let mixed = VALID.replace("S3-D1", "S4-D1");
        let found = messages("steps/not-a-step/step.md", &mixed);
        assert!(found.iter().any(|entry| entry.0 == Severity::Error));
    }

    #[test]
    fn validates_title_directory_and_plain_done_when() {
        for source in [
            VALID.replace("Step 03: Check steps", "Step 3: Check steps"),
            VALID.replace("Step 03: Check steps", "Step 03:"),
            VALID.replace("- S3-D1: done", "- [ ] S3-D1: done"),
        ] {
            assert!(messages("steps/03-check/step.md", &source)
                .iter()
                .any(|entry| entry.0 == Severity::Error));
        }
        assert!(messages("steps/04-check/step.md", VALID)
            .iter()
            .any(|entry| entry.2.contains("title number")));
    }

    #[test]
    fn validates_optional_section_order_forms_and_numeric_bounds() {
        let out_of_order = VALID.replace(
            "## Done When\n- S3-D1: done",
            "## Changes\n- S3-C1: change\n## Done When\n- S3-D1: done",
        );
        assert!(messages("steps/03-check/step.md", &out_of_order)
            .iter()
            .any(|entry| entry.2.contains("must appear before")));

        let compact_revision = format!("{VALID}## Revisions\n- S3-REV1: compact\n");
        assert!(messages("steps/03-check/step.md", &compact_revision)
            .iter()
            .any(|entry| entry.2.contains("requires expanded items")));

        let overflow = VALID.replace("S3-D1", "S9223372036854775808-D1");
        assert!(messages("steps/03-check/step.md", &overflow)
            .iter()
            .any(|entry| entry.2.contains("2^63 - 1")));
    }

    #[test]
    fn keeps_fenced_headings_opaque_and_uses_nominal_paths() {
        let fenced = VALID.replace(
            "## Context\ntext",
            "## Context\n```markdown\n## Done When\n- S99-D1: fenced\n```",
        );
        assert_eq!(messages("steps/03-check/step.md", &fenced), []);

        assert_eq!(
            step_directory_number(Path::new("linked/03-check/step.md")),
            Some(3)
        );
        assert_eq!(
            step_directory_number(Path::new("real/99-target/step.md")),
            Some(99)
        );
    }
}
