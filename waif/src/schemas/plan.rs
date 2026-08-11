use crate::parser::{self, Diagnostic, ParserConfig, SectionConfig};
use crate::schema::{
    self, metadata_validators, ArtifactPrefixRule, ItemRule, MetadataRule, PlanItemRule, Schema,
    SectionRule,
};

const SCHEMA: Schema = Schema {
    prefix: ArtifactPrefixRule::Known("P-"),
    metadata: &METADATA,
    sections: &SECTIONS,
};

const METADATA: [MetadataRule; 7] = [
    MetadataRule {
        name: "Status",
        validator: metadata_validators::artifact_status,
    },
    MetadataRule {
        name: "Goal",
        validator: exact_goal,
    },
    MetadataRule {
        name: "Branch",
        validator: nonempty,
    },
    MetadataRule {
        name: "Base branch",
        validator: nonempty,
    },
    MetadataRule {
        name: "Base commit",
        validator: full_commit,
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

const SECTIONS: [SectionRule; 9] = [
    SectionRule::new("Technical Summary"),
    SectionRule::new("Decisions").with_items(ItemRule::new("D").with_expanded_family("DD").mixed()),
    SectionRule::optional("Open Questions").with_items(ItemRule::new("Q")),
    SectionRule::optional("Assumptions").with_items(ItemRule::new("A")),
    SectionRule::new("Current System"),
    SectionRule::new("Plan Items").with_plan_items(PlanItemRule {
        statuses: &["pending", "done"],
    }),
    SectionRule::new("Risks").with_items(ItemRule::new("R")),
    SectionRule::new("Cross-Cutting Validation").with_items(ItemRule::new("V")),
    SectionRule::optional("Revisions").with_items(ItemRule::new("REV").expanded_only()),
];

fn exact_goal(value: &str) -> Result<(), String> {
    (value == "./goal.md")
        .then_some(())
        .ok_or_else(|| format!("expected `./goal.md`, but got `{value}`"))
}

fn nonempty(value: &str) -> Result<(), String> {
    (!value.trim().is_empty())
        .then_some(())
        .ok_or_else(|| "must not be empty".to_owned())
}

fn full_commit(value: &str) -> Result<(), String> {
    (value.len() == 40 && value.bytes().all(|byte| byte.is_ascii_hexdigit()))
        .then_some(())
        .ok_or_else(|| format!("expected a full 40-character Git commit hash, but got `{value}`"))
}

pub(crate) fn check(source: &str) -> Vec<Diagnostic> {
    let (artifact, mut diagnostics) = parser::parse_with_diagnostics(source, &parser_config());
    diagnostics.extend(schema::validate(&artifact, &SCHEMA));
    diagnostics
}

fn parser_config() -> ParserConfig<'static> {
    ParserConfig::new(vec![
        SectionConfig::prose("Technical Summary"),
        SectionConfig::itemised("Decisions"),
        SectionConfig::itemised("Open Questions"),
        SectionConfig::itemised("Assumptions"),
        SectionConfig::prose("Current System"),
        SectionConfig::plan_items("Plan Items"),
        SectionConfig::itemised("Risks"),
        SectionConfig::itemised("Cross-Cutting Validation"),
        SectionConfig::itemised("Revisions"),
    ])
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::Severity;

    const VALID: &str = concat!(
        "# Plan\n",
        "- Status: accepted\n- Goal: ./goal.md\n- Branch: waif/task\n",
        "- Base branch: main\n",
        "- Base commit: 0123456789abcdef0123456789abcdef01234567\n",
        "- Created: 2026-08-01T10:00:00+09:00\n",
        "- Updated: 2026-08-01T11:00:00+09:00\n",
        "## Technical Summary\ntext\n",
        "## Decisions\n- P-D1: compact\n### P-DD1: Expanded\nbody\n",
        "## Current System\ntext\n",
        "## Plan Items\n### P1: First\n- Status: pending\n",
        "- Goal criteria: opaque\n#### Detail\ntext\n",
        "## Risks\n- P-R1: risk\n",
        "## Cross-Cutting Validation\n- P-V1: test\n",
    );

    fn messages(source: &str) -> Vec<(Severity, usize, String)> {
        check(source)
            .into_iter()
            .map(|d| (d.severity(), d.line(), d.message().to_owned()))
            .collect()
    }

    #[test]
    fn accepts_complete_plan_and_opaque_item_prose() {
        assert_eq!(messages(VALID), []);
    }

    #[test]
    fn validates_metadata_sections_and_decision_families_exhaustively() {
        let source = VALID
            .replace("- Goal: ./goal.md", "- Goal: goal.md")
            .replace("- Branch: waif/task", "- Branch: ")
            .replace("## Risks", "## Revisions\n- P-REV1: compact\n## Risks")
            .replace("- P-D1: compact", "- P-DD1: wrong")
            .replace("### P-DD1: Expanded", "### P-D1: Wrong");
        let found = messages(&source);
        assert!(
            found
                .iter()
                .filter(|entry| entry.0 == Severity::Error)
                .count()
                >= 5
        );
        assert!(found
            .iter()
            .any(|entry| entry.2.contains("metadata `Goal`")));
        assert!(found
            .iter()
            .any(|entry| entry.2.contains("metadata `Branch`")));
        assert!(found.iter().any(|entry| entry.2.contains("P-D<number>")));
        assert!(found.iter().any(|entry| entry.2.contains("P-DD<number>")));
        assert!(found
            .iter()
            .any(|entry| entry.2.contains("requires expanded items")));
    }

    #[test]
    fn validates_plan_item_ids_titles_and_statuses() {
        let source = VALID.replace(
            "### P1: First\n- Status: pending\n",
            concat!(
                "### P9223372036854775808: Too large\n- Status: blocked\n",
                "- Status: done\n",
                "### P2:\n- Goal criteria: opaque\n",
            ),
        );
        let found = messages(&source);
        assert!(found.iter().any(|entry| entry.2.contains("2^63 - 1")));
        assert!(found.iter().any(|entry| entry.2.contains("got `blocked`")));
        assert!(found
            .iter()
            .any(|entry| entry.2.contains("duplicate plan-item")));
        assert!(found
            .iter()
            .any(|entry| entry.2.contains("missing a title")));
        assert!(found
            .iter()
            .any(|entry| entry.2.contains("missing required metadata")));
    }

    #[test]
    fn accepts_numeric_upper_bound_and_known_metadata_in_any_order() {
        let source = VALID
            .replace("### P1: First", "### P9223372036854775807: First")
            .replace(
                "- Status: accepted\n- Goal: ./goal.md\n- Branch: waif/task\n",
                "- Branch: waif/task\n- Goal: ./goal.md\n- Status: drafting\n",
            );
        assert_eq!(messages(&source), []);
    }

    #[test]
    fn combines_parser_and_independent_schema_diagnostics() {
        let source = VALID
            .replace("- Status: accepted", "- Status: invalid")
            .replace(
                "## Plan Items\n### P1: First",
                "## Plan Items\nbare content\n### P1: First",
            )
            .replace("## Risks\n- P-R1: risk\n", "");
        let found = messages(&source);

        assert!(found
            .iter()
            .any(|entry| entry.2.contains("expected an expanded item")));
        assert!(found
            .iter()
            .any(|entry| entry.2.contains("metadata `Status`")));
        assert!(found
            .iter()
            .any(|entry| entry.2.contains("missing required section `Risks`")));
    }
}
