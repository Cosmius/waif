use crate::parser::{self, Diagnostic, ParserConfig, SectionConfig};
use crate::schema::{self, ArtifactPrefixRule, ItemRule, MetadataRule, Schema, SectionRule};

const METADATA: [MetadataRule; 3] = [
    MetadataRule {
        name: "Status",
        validator: schema::artifact_status,
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
const SECTIONS: [SectionRule; 7] = [
    SectionRule::new("Outcome"),
    SectionRule::new("Acceptance Criteria").with_items(ItemRule::new("AC")),
    SectionRule::new("Open Questions")
        .optional()
        .with_items(ItemRule::new("Q")),
    SectionRule::new("Assumptions")
        .optional()
        .with_items(ItemRule::new("A")),
    SectionRule::new("In Scope")
        .optional()
        .with_items(ItemRule::new("IN")),
    SectionRule::new("Out of Scope")
        .optional()
        .with_items(ItemRule::new("OUT")),
    SectionRule::new("Revisions")
        .optional()
        .with_items(ItemRule::new("REV").expanded_only()),
];
const SCHEMA: Schema = Schema {
    prefix: ArtifactPrefixRule::Known("G-"),
    metadata: &METADATA,
    sections: &SECTIONS,
};

/// Parse and validate the structural schema for a goal artifact.
///
/// Parser errors prevent schema validation because no typed artifact is
/// available. A successful parse returns every schema error and warning.
#[allow(dead_code)]
pub(crate) fn check(source: &str) -> Vec<Diagnostic> {
    match parser::parse_with_config(source, &parser_config()) {
        Ok(artifact) => schema::validate(&artifact, &SCHEMA),
        Err(diagnostics) => diagnostics,
    }
}

fn parser_config() -> ParserConfig<'static> {
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
                 ## Outcome\ntext\n\
                 ## Acceptance Criteria\n### G-AC1: expanded\n"
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

        assert!(diagnostics.iter().any(|entry| {
            entry.1 == 2
                && entry.2
                    == "metadata `Status` expected `drafting`, `accepted`, or \
                                   `amending`, but got `unknown`"
        }));
        assert!(diagnostics
            .iter()
            .any(|entry| { entry.1 == 3 && entry.2 == "duplicate metadata key `Status`" }));
        assert!(diagnostics.iter().any(|entry| {
            entry.1 == 4
                && entry.2
                    == "metadata `Created` expected an RFC 3339 timestamp with a \
                                   timezone, but got `2026-07-31T12:00:00`"
        }));
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
            "## Outcome\ntext\n",
            "## Acceptance Criteria\n### G-AC1: expanded\nbody\n",
            "## Custom\nopaque\n",
            "## In Scope\n- G-IN1: direct\n",
        );

        assert_eq!(messages(source), []);
    }

    #[test]
    fn schema_enforces_known_section_order_and_ignores_unknown_sections() {
        let source = concat!(
            "# Goal\n",
            "- Status: accepted\n",
            "- Created: 2026-07-31T12:00:00Z\n",
            "- Updated: 2026-07-31T12:00:00Z\n",
            "## Revisions\n### G-REV1: revision\n",
            "## Custom One\nopaque\n",
            "## Out of Scope\n- G-OUT1: excluded\n",
            "## In Scope\n- G-IN1: included\n",
            "## Assumptions\n- G-A1: assumed\n",
            "## Custom Two\nopaque\n",
            "## Open Questions\n- G-Q1: question\n",
            "## Acceptance Criteria\n- G-AC1: criterion\n",
            "## Outcome\noutcome\n",
        );
        let diagnostics = messages(source);
        let order_errors = diagnostics
            .iter()
            .filter(|entry| entry.2.contains("must appear before section `Revisions`"))
            .collect::<Vec<_>>();

        assert_eq!(order_errors.len(), 6);
        assert!(diagnostics.iter().all(|entry| entry.0 == Severity::Error));

        let valid = format!(
            "{VALID}## Custom\nopaque\n## In Scope\n- G-IN1: included\n\
             ## Revisions\n### G-REV1: revision\n"
        );
        assert_eq!(messages(&valid), []);
    }

    #[test]
    fn schema_enforces_increasing_ids_with_signed_64_bit_boundaries() {
        let source = concat!(
            "# Goal\n",
            "- Status: accepted\n",
            "- Created: 2026-07-31T12:00:00Z\n",
            "- Updated: 2026-07-31T12:00:00Z\n",
            "## Outcome\noutcome\n",
            "## Acceptance Criteria\n",
            "- G-AC2: two\n",
            "- G-AC9223372036854775807: maximum\n",
            "- G-AC9223372036854775808: overflow\n",
            "- G-AC999999999999999999999999999999999999: larger overflow\n",
            "- G-AC3: out of order\n",
            "- G-AC3: duplicate\n",
            "## Open Questions\n",
            "### G-Q9223372036854775807: maximum\n",
            "### G-Q1: out of order\n",
            "## In Scope\n",
            "- G-IN2: two\n",
            "- G-IN1: out of order\n",
        );
        let diagnostics = messages(source);

        assert_eq!(
            diagnostics
                .iter()
                .filter(|entry| entry.2.contains("must be greater than"))
                .count(),
            3
        );
        assert_eq!(
            diagnostics
                .iter()
                .filter(|entry| entry.2.contains("duplicate item identifier `G-AC3`"))
                .count(),
            1
        );
        assert_eq!(
            diagnostics
                .iter()
                .filter(|entry| entry.2.contains("from 1 through `2^63 - 1`"))
                .count(),
            2
        );
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
            "## Open Questions\n### G-Q3: question\nopaque body\n",
            "## Assumptions\n- G-A4:\n",
            "## In Scope\n- G-IN2: [ ] content is opaque\n",
            "## Out of Scope\n",
            "- G-OUT9223372036854775807: maximum\n",
            "## Revisions\n### G-REV5: revision\n",
            "- [x] nested body content\n",
            "#### G-REV0 is an opaque deeper heading\n",
        );

        assert_eq!(messages(source), []);
    }

    #[test]
    fn reports_malformed_item_syntax_errors() {
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
        assert_eq!(diagnostics.len(), 3);
        assert_eq!(
            diagnostics.iter().map(|entry| entry.1).collect::<Vec<_>>(),
            [8, 9, 14]
        );
        assert!(diagnostics.iter().all(|entry| entry.0 == Severity::Error));
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
            "## Open Questions\n- G-A1: wrong\n",
            "## Assumptions\n- G-REV1: wrong\n",
            "## In Scope\n- G-OUT1: wrong\n",
            "## Out of Scope\n- G-Q1: wrong\n",
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
                .filter(|entry| entry.2 == "Expected identifier")
                .count(),
            3
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
            "metadata `Status` expected",
            "metadata `Created` expected",
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
        );
        let diagnostics = messages(source);

        for expected in [
            "metadata `Status` expected",
            "missing required metadata `Updated`",
            "section `Outcome` is empty",
            "duplicate section `Outcome`",
            "item identifier `G-IN0`",
        ] {
            assert!(diagnostics.iter().any(|entry| entry.2.contains(expected)));
        }
        assert!(diagnostics.iter().any(|entry| {
            entry.0 == Severity::Warning && entry.2 == "section `Outcome` is empty"
        }));
    }
}
