use std::collections::HashSet;

use chrono::DateTime;

use crate::artifact::{Artifact, Section};
use crate::parser::{self, Diagnostic, ParserConfig, SectionConfig};

#[derive(Clone, Copy)]
struct MetadataValidator {
    name: &'static str,
    validator: Validator,
}

type Validator = fn(&str) -> Result<(), &'static str>;

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
        SectionConfig::expanded_itemised("Revisions"),
    ])
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

fn section_is_empty(section: &Section) -> bool {
    match section {
        Section::Prose(section) => section.value().body().trim().is_empty(),
        Section::Itemised(section) => section.value().items().is_empty(),
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
    fn accepts_valid_metadata_variants_and_unique_additions() {
        for (created, updated) in [
            ("2026-07-31T12:00:00Z", "2026-07-31T21:00:00+09:00"),
            (
                "2026-07-31T12:00:00.123-04:30",
                "2026-07-31T21:00:00.1+00:00",
            ),
        ] {
            let source = format!(
                "# Goal\n- Extra: yes\n- Updated: {updated}\n\
                 - Status: drafting\n- Created: {created}\n\
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
            "- Extra: one\n",
            "- Extra: two\n",
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
            .any(|entry| { entry.1 == 6 && entry.2 == "duplicate metadata key `Extra`" }));
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
}
