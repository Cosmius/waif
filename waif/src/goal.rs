use std::collections::HashSet;

use chrono::DateTime;

use crate::artifact::{Artifact, ItemForm, Section};
use crate::parser::{self, Diagnostic, ParserConfig, SectionConfig};

#[derive(Clone, Copy)]
struct MetadataValidator {
    name: &'static str,
    validator: Validator,
}

type Validator = fn(&str) -> Result<(), &'static str>;

#[derive(Clone, Copy)]
struct ItemValidator {
    section_name: &'static str,
    prefix: &'static str,
    required_form: Option<ItemForm>,
}

const REQUIRED_METADATA: [MetadataValidator; 3] = [
    MetadataValidator {
        name: "Status",
        validator: valid_status,
    },
    MetadataValidator {
        name: "Created",
        validator: valid_timestamp,
    },
    MetadataValidator {
        name: "Updated",
        validator: valid_timestamp,
    },
];
const REQUIRED_SECTIONS: [&str; 2] = ["Outcome", "Acceptance Criteria"];
const ITEM_VALIDATORS: [ItemValidator; 6] = [
    ItemValidator {
        section_name: "Acceptance Criteria",
        prefix: "G-AC",
        required_form: None,
    },
    ItemValidator {
        section_name: "In Scope",
        prefix: "G-IN",
        required_form: None,
    },
    ItemValidator {
        section_name: "Out of Scope",
        prefix: "G-OUT",
        required_form: None,
    },
    ItemValidator {
        section_name: "Open Questions",
        prefix: "G-Q",
        required_form: None,
    },
    ItemValidator {
        section_name: "Assumptions",
        prefix: "G-A",
        required_form: None,
    },
    ItemValidator {
        section_name: "Revisions",
        prefix: "G-REV",
        required_form: Some(ItemForm::Expanded),
    },
];

fn valid_status(value: &str) -> Result<(), &'static str> {
    if matches!(value, "drafting" | "accepted" | "amending") {
        Ok(())
    } else {
        Err("`drafting`, `accepted`, or `amending`")
    }
}

fn valid_timestamp(value: &str) -> Result<(), &'static str> {
    DateTime::parse_from_rfc3339(value)
        .map(|_| ())
        .map_err(|_| "an RFC 3339 timestamp with a timezone")
}

/// Parse and validate the structural schema for a goal artifact.
///
/// Parser errors prevent schema validation because no typed artifact is
/// available. A successful parse returns every schema error and warning.
#[allow(dead_code)]
pub(crate) fn check(source: &str) -> Vec<Diagnostic> {
    match parser::parse_with_config(source, &parser_config()) {
        Ok(artifact) => validate(&artifact),
        Err(diagnostics) => diagnostics,
    }
}

fn parser_config() -> ParserConfig {
    ParserConfig::new(vec![
        SectionConfig::prose("Outcome"),
        SectionConfig::itemised("Acceptance Criteria"),
        SectionConfig::itemised("In Scope"),
        SectionConfig::itemised("Out of Scope"),
        SectionConfig::itemised("Open Questions"),
        SectionConfig::itemised("Assumptions"),
        SectionConfig::itemised("Revisions"),
    ])
    .with_known_metadata(["Status", "Created", "Updated"])
}

fn validate(artifact: &Artifact) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();
    validate_metadata(artifact, &mut diagnostics);
    validate_sections(artifact, &mut diagnostics);
    diagnostics
}

fn validate_metadata(artifact: &Artifact, diagnostics: &mut Vec<Diagnostic>) {
    let mut seen = HashSet::new();

    for metadata in artifact.metadata() {
        let line = metadata.span().start_line();
        let metadata = metadata.value();
        let key = metadata.key();
        if !seen.insert(key) {
            diagnostics.push(Diagnostic::error(
                line,
                format!("duplicate metadata key `{key}`"),
            ));
        }

        if let Some(metadata_validator) = REQUIRED_METADATA
            .iter()
            .find(|metadata_validator| metadata_validator.name == key)
        {
            if let Err(message) = (metadata_validator.validator)(metadata.value()) {
                diagnostics.push(Diagnostic::error(
                    line,
                    format!("metadata `{}` must be {message}", metadata_validator.name),
                ));
            }
        }
    }

    for metadata_validator in REQUIRED_METADATA {
        if !seen.contains(metadata_validator.name) {
            diagnostics.push(Diagnostic::error(
                1,
                format!("missing required metadata `{}`", metadata_validator.name),
            ));
        }
    }
}

fn validate_sections(artifact: &Artifact, diagnostics: &mut Vec<Diagnostic>) {
    let mut seen = HashSet::new();
    let mut seen_item_ids = HashSet::new();

    for section in artifact.sections() {
        let name = section.name();
        let line = section.span().start_line();
        if !seen.insert(name) {
            diagnostics.push(Diagnostic::error(
                line,
                format!("duplicate section `{name}`"),
            ));
        }
        if section_is_empty(section) {
            diagnostics.push(Diagnostic::warning(
                line,
                format!("section `{name}` is empty"),
            ));
        }

        if let Some(validator) = ITEM_VALIDATORS
            .iter()
            .find(|validator| validator.section_name == name)
        {
            validate_items(section, *validator, &mut seen_item_ids, diagnostics);
        }
    }

    for name in REQUIRED_SECTIONS {
        if !seen.contains(name) {
            diagnostics.push(Diagnostic::error(
                1,
                format!("missing required section `{name}`"),
            ));
        }
    }
}

fn validate_items<'a>(
    section: &'a Section,
    validator: ItemValidator,
    seen: &mut HashSet<(&'static str, &'a str)>,
    diagnostics: &mut Vec<Diagnostic>,
) {
    let Some(section) = section.as_itemised() else {
        return;
    };

    validate_item_forms(section.items(), validator, diagnostics);

    for item in section.items() {
        let Some(identifier) = item.identifier() else {
            diagnostics.push(Diagnostic::error(
                item.span().start_line(),
                format!(
                    "item in section `{}` is missing an identifier; expected \
                     `{}<number>`",
                    validator.section_name, validator.prefix
                ),
            ));
            continue;
        };

        let identifier_text = identifier.text();
        let Some(number) = valid_item_number(identifier_text, validator.prefix) else {
            diagnostics.push(Diagnostic::error(
                identifier.span().start_line(),
                format!(
                    "item identifier `{identifier_text}` in section `{}` must use \
                     `{}<number>`, where number is a positive decimal integer",
                    validator.section_name, validator.prefix
                ),
            ));
            continue;
        };

        if !seen.insert((validator.prefix, number)) {
            diagnostics.push(Diagnostic::error(
                identifier.span().start_line(),
                format!(
                    "duplicate item identifier `{identifier_text}` in section `{}`",
                    validator.section_name
                ),
            ));
        }
    }
}

fn validate_item_forms(
    items: &[crate::artifact::Item],
    validator: ItemValidator,
    diagnostics: &mut Vec<Diagnostic>,
) {
    let Some(first) = items.first() else {
        return;
    };
    let required = validator.required_form.unwrap_or_else(|| first.form());
    let Some(conflicting) = items.iter().find(|item| item.form() != required) else {
        return;
    };

    if validator.required_form.is_some() {
        let (form, syntax) = match required {
            ItemForm::Compact => ("compact", "- ID: content"),
            ItemForm::Expanded => ("expanded", "### ID: title"),
        };
        diagnostics.push(Diagnostic::error(
            conflicting.span().start_line(),
            format!(
                "section `{}` requires {form} items in `{syntax}` form",
                validator.section_name
            ),
        ));
    } else {
        diagnostics.push(Diagnostic::error(
            conflicting.span().start_line(),
            format!(
                "itemised section `{}` cannot mix compact and expanded items",
                validator.section_name
            ),
        ));
    }
}

fn valid_item_number<'a>(identifier: &'a str, prefix: &str) -> Option<&'a str> {
    let number = identifier.strip_prefix(prefix)?;
    let mut bytes = number.bytes();
    if !matches!(bytes.next(), Some(b'1'..=b'9')) {
        return None;
    }
    bytes.all(|byte| byte.is_ascii_digit()).then_some(number)
}

fn section_is_empty(section: &Section) -> bool {
    match section {
        Section::Prose(section) => section.value().body().trim().is_empty(),
        Section::Itemised(section) => section.value().items().is_empty(),
        Section::PlanItems(section) => section.value().items().is_empty(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::Severity;

    const VALID: &str = concat!(
        "# Any title\n",
        "- Status: accepted\n",
        "- Created: 2026-07-31T12:00:00Z\n",
        "- Updated: 2026-07-31T21:00:00+09:00\n",
        "## Outcome\n",
        "An outcome.\n",
        "## Acceptance Criteria\n",
        "- G-AC1: accepted for now\n",
    );

    fn messages(source: &str) -> Vec<(Severity, usize, String)> {
        check(source)
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
    fn accepts_valid_metadata_variants_and_trailing_opaque_prose() {
        for (created, updated) in [
            ("2026-07-31T12:00:00Z", "2026-07-31T21:00:00+09:00"),
            (
                "2026-07-31T12:00:00.123-04:30",
                "2026-07-31T21:00:00.1+00:00",
            ),
        ] {
            let source = format!(
                "# Goal\n- Updated: {updated}\n\
                 - Status: drafting\n- Created: {created}\n- Extra: opaque\n\
                 ## Acceptance Criteria\n### G-AC1: expanded\n\
                 ## Outcome\ntext\n"
            );
            assert_eq!(messages(&source), []);
        }
    }

    #[test]
    fn reports_invalid_missing_and_duplicate_metadata_at_source_lines() {
        let source = concat!(
            "# Goal\n",
            "- Status: unknown\n",
            "- Status: accepted\n",
            "- Created: 2026-07-31T12:00:00\n",
            "- Extra: opaque\n",
            "- Updated: hidden in prose\n",
            "## Outcome\ntext\n",
            "## Acceptance Criteria\n- G-AC1: one\n",
        );
        let diagnostics = messages(source);

        assert!(diagnostics
            .iter()
            .any(|entry| entry.1 == 2 && entry.2.contains("`Status` must be")));
        assert!(diagnostics
            .iter()
            .any(|entry| { entry.1 == 3 && entry.2 == "duplicate metadata key `Status`" }));
        assert!(diagnostics
            .iter()
            .any(|entry| { entry.1 == 4 && entry.2.contains("`Created` must be an RFC 3339") }));
        assert!(diagnostics
            .iter()
            .any(|entry| { entry.1 == 1 && entry.2 == "missing required metadata `Updated`" }));
        assert!(diagnostics.iter().all(|entry| entry.0 == Severity::Error));
    }

    #[test]
    fn accepts_optional_direct_scope_and_unknown_prose_sections() {
        let source = concat!(
            "# Different title\n",
            "- Updated: 2026-07-31T12:00:00Z\n",
            "- Created: 2026-07-31T12:00:00Z\n",
            "- Status: amending\n",
            "## In Scope\n- G-IN1: direct\n",
            "## Custom\nopaque\n",
            "## Acceptance Criteria\n### G-AC1: expanded\nbody\n",
            "## Outcome\ntext\n",
        );

        assert_eq!(messages(source), []);
    }

    #[test]
    fn reports_missing_and_duplicate_known_or_unknown_sections() {
        let source = concat!(
            "# Goal\n",
            "- Status: accepted\n",
            "- Created: 2026-07-31T12:00:00Z\n",
            "- Updated: 2026-07-31T12:00:00Z\n",
            "## Outcome\none\n",
            "## Custom\none\n",
            "## Outcome\ntwo\n",
            "## Custom\ntwo\n",
        );
        let diagnostics = messages(source);

        assert!(diagnostics
            .iter()
            .any(|entry| { entry.1 == 9 && entry.2 == "duplicate section `Outcome`" }));
        assert!(diagnostics
            .iter()
            .any(|entry| { entry.1 == 11 && entry.2 == "duplicate section `Custom`" }));
        assert!(diagnostics.iter().any(|entry| {
            entry.1 == 1 && entry.2 == "missing required section `Acceptance Criteria`"
        }));
    }

    #[test]
    fn enforces_expanded_revisions_but_accepts_both_ordinary_forms() {
        let compact_revisions = format!("{VALID}## Revisions\n- G-REV1: compact\n");
        let diagnostics = messages(&compact_revisions);
        assert_eq!(diagnostics.len(), 1);
        assert_eq!(diagnostics[0].0, Severity::Error);
        assert!(diagnostics[0].2.contains("requires expanded items"));

        for revisions in [
            "## Revisions\n### G-REV1: expanded\nopaque body\n",
            "## In Scope\n- G-IN1: compact\n",
            "## In Scope\n### G-IN1: expanded\nopaque body\n",
        ] {
            assert_eq!(messages(&format!("{VALID}{revisions}")), []);
        }
    }

    #[test]
    fn rejects_mixed_forms_in_an_ordinary_itemised_section() {
        let source = format!("{VALID}## In Scope\n- G-IN1: compact\n### G-IN2: expanded\n");
        let diagnostics = messages(&source);

        assert!(diagnostics.iter().any(|entry| {
            entry.1 == 11
                && entry.2 == "itemised section `In Scope` cannot mix compact and expanded items"
        }));
    }

    #[test]
    fn accepts_every_item_namespace_and_opaque_content() {
        let source = concat!(
            "# Goal\n",
            "- Status: accepted\n",
            "- Created: 2026-07-31T12:00:00Z\n",
            "- Updated: 2026-07-31T12:00:00Z\n",
            "## Outcome\ntext\n",
            "## Acceptance Criteria\n### G-AC1:\n",
            "## In Scope\n- G-IN2: [ ] content is opaque\n",
            "## Out of Scope\n",
            "- G-OUT999999999999999999999999999999999999: large\n",
            "## Open Questions\n### G-Q3: question\nopaque body\n",
            "## Assumptions\n- G-A4:\n",
            "## Revisions\n### G-REV5: revision\n",
            "- [x] nested body content\n",
            "#### G-REV0 is an opaque deeper heading\n",
        );

        assert_eq!(messages(source), []);
    }

    #[test]
    fn reports_malformed_and_duplicate_item_identifiers() {
        let source = concat!(
            "# Goal\n",
            "- Status: accepted\n",
            "- Created: 2026-07-31T12:00:00Z\n",
            "- Updated: 2026-07-31T12:00:00Z\n",
            "## Outcome\ntext\n",
            "## Acceptance Criteria\n",
            "- missing delimiter\n",
            "- : empty identifier\n",
            "- G-IN1: wrong family\n",
            "- G-AC: empty suffix\n",
            "- G-AC0: zero\n",
            "- G-AC01: leading zero\n",
            "- G-AC+1: sign\n",
            "- G-AC-1: sign\n",
            "- G-AC1a: nondigit\n",
            "- G-AC2: first valid ID\n",
            "- G-AC2: duplicate valid ID\n",
        );
        let diagnostics = messages(source);

        assert_eq!(diagnostics.len(), 10);
        assert_eq!(
            diagnostics.iter().map(|entry| entry.1).collect::<Vec<_>>(),
            [8, 9, 10, 11, 12, 13, 14, 15, 16, 18]
        );
        assert!(diagnostics.iter().all(|entry| {
            entry.0 == Severity::Error
                && entry.2.contains("Acceptance Criteria")
                && (entry.2.contains("G-AC<number>")
                    || entry.2.contains("duplicate item identifier"))
        }));
    }

    #[test]
    fn enforces_the_exact_family_for_every_itemised_section() {
        let source = concat!(
            "# Goal\n",
            "- Status: accepted\n",
            "- Created: 2026-07-31T12:00:00Z\n",
            "- Updated: 2026-07-31T12:00:00Z\n",
            "## Outcome\ntext\n",
            "## Acceptance Criteria\n- G-IN1: wrong\n",
            "## In Scope\n- G-OUT1: wrong\n",
            "## Out of Scope\n- G-Q1: wrong\n",
            "## Open Questions\n- G-A1: wrong\n",
            "## Assumptions\n- G-REV1: wrong\n",
            "## Revisions\n### G-AC1: wrong\n",
        );
        let diagnostics = messages(source);

        assert_eq!(diagnostics.len(), 6);
        for (section, prefix) in [
            ("Acceptance Criteria", "G-AC<number>"),
            ("In Scope", "G-IN<number>"),
            ("Out of Scope", "G-OUT<number>"),
            ("Open Questions", "G-Q<number>"),
            ("Assumptions", "G-A<number>"),
            ("Revisions", "G-REV<number>"),
        ] {
            assert!(diagnostics
                .iter()
                .any(|entry| entry.2.contains(section) && entry.2.contains(prefix)));
        }
    }

    #[test]
    fn reports_checkbox_items_as_invalid_identifiers() {
        let source = concat!(
            "# Goal\n",
            "- Status: accepted\n",
            "- Created: 2026-07-31T12:00:00Z\n",
            "- Updated: 2026-07-31T12:00:00Z\n",
            "## Outcome\ntext\n",
            "## Acceptance Criteria\n",
            "- [ ] G-AC1: unchecked\n",
            "- [x] G-AC2: checked\n",
            "- [X] G-AC3\n",
            "- G-AC4: [ ] content is opaque\n",
            "  - [x] nested body is opaque\n",
            "  ```text\n",
            "  - [X] fenced body is opaque\n",
            "  ```\n",
            "## In Scope\n",
            "### G-IN1: expanded\n",
            "[ ] expanded body is opaque\n",
        );
        let diagnostics = messages(source);

        assert_eq!(diagnostics.len(), 3);
        assert_eq!(
            diagnostics.iter().map(|entry| entry.1).collect::<Vec<_>>(),
            [8, 9, 10]
        );
        assert_eq!(
            diagnostics
                .iter()
                .filter(|entry| entry.2.contains("must use `G-AC<number>`"))
                .count(),
            2
        );
        assert_eq!(
            diagnostics
                .iter()
                .filter(|entry| entry.2.contains("is missing an identifier"))
                .count(),
            1
        );
        assert!(diagnostics
            .iter()
            .all(|entry| { entry.0 == Severity::Error && !entry.2.contains("checkbox") }));
    }

    #[test]
    fn accumulates_section_and_namespace_duplicates() {
        let source = format!("{VALID}## Acceptance Criteria\n- G-AC1: duplicate across sections\n");
        let diagnostics = messages(&source);

        assert_eq!(diagnostics.len(), 2);
        assert_eq!(
            diagnostics[0],
            (
                Severity::Error,
                9,
                "duplicate section `Acceptance Criteria`".into(),
            )
        );
        assert_eq!(
            diagnostics[1],
            (
                Severity::Error,
                10,
                "duplicate item identifier `G-AC1` in section \
                 `Acceptance Criteria`"
                    .into(),
            )
        );
    }

    #[test]
    fn reports_empty_sections_as_non_failing_warnings() {
        let source = concat!(
            "# Goal\n",
            "- Status: accepted\n",
            "- Created: 2026-07-31T12:00:00Z\n",
            "- Updated: 2026-07-31T12:00:00Z\n",
            "## Outcome\n\n",
            "## Acceptance Criteria\n\n",
            "## Custom\n \t\n",
        );
        let diagnostics = messages(source);

        assert_eq!(diagnostics.len(), 3);
        assert_eq!(
            diagnostics[0],
            (Severity::Warning, 5, "section `Outcome` is empty".into(),)
        );
        assert_eq!(
            diagnostics[1],
            (
                Severity::Warning,
                7,
                "section `Acceptance Criteria` is empty".into(),
            )
        );
        assert_eq!(
            diagnostics[2],
            (Severity::Warning, 9, "section `Custom` is empty".into(),)
        );
    }

    #[test]
    fn accumulates_independent_metadata_and_topology_errors() {
        let source = concat!(
            "# Goal\n",
            "- Status: invalid\n",
            "- Created: no-timezone\n",
            "## Outcome\ntext\n",
            "## Outcome\nagain\n",
        );
        let diagnostics = messages(source);

        for expected in [
            "metadata `Status` must be",
            "metadata `Created` must be",
            "missing required metadata `Updated`",
            "duplicate section `Outcome`",
            "missing required section `Acceptance Criteria`",
        ] {
            assert!(diagnostics.iter().any(|entry| entry.2.contains(expected)));
        }
    }

    #[test]
    fn accumulates_metadata_topology_item_errors_and_warnings() {
        let source = concat!(
            "# Goal\n",
            "- Status: invalid\n",
            "- Created: 2026-07-31T12:00:00Z\n",
            "## Outcome\n\n",
            "## Outcome\ntext\n",
            "## Acceptance Criteria\n",
            "- G-IN0: wrong family and zero\n",
            "- [ ] G-AC1: checkbox\n",
        );
        let diagnostics = messages(source);

        for expected in [
            "metadata `Status` must be",
            "missing required metadata `Updated`",
            "section `Outcome` is empty",
            "duplicate section `Outcome`",
            "item identifier `G-IN0`",
            "item identifier `[ ] G-AC1`",
        ] {
            assert!(diagnostics.iter().any(|entry| entry.2.contains(expected)));
        }
        assert!(diagnostics.iter().any(|entry| {
            entry.0 == Severity::Warning && entry.2 == "section `Outcome` is empty"
        }));
    }
}
