use super::*;
use crate::parser::{self, ParserConfig, SectionConfig};

const FINDINGS_SCHEMA: Schema = Schema {
    prefix: ArtifactPrefixRule::Known(""),
    metadata: &[],
    sections: &[SectionRule::new("Findings").with_findings(FindingsRule { family: "F" })],
};

fn check(source: &str) -> Vec<Diagnostic> {
    let config = ParserConfig::new(vec![SectionConfig::findings("Findings")]);
    let (artifact, mut diagnostics) = parser::parse_with_diagnostics(source, &config);
    diagnostics.extend(validate(&artifact, &FINDINGS_SCHEMA));
    diagnostics
}

#[test]
fn accepts_sentinel_and_ordered_findings() {
    assert!(check("# Review\n## Findings\nNo findings.\n").is_empty());
    assert!(check("# Review\n## Findings\n### F1: First\n\n### F2: Second\n").is_empty());
}

#[test]
fn reports_multiple_finding_id_errors() {
    let diagnostics = check(
        "# Review\n## Findings\n### F2: First\n\n### F2: Duplicate\n\n### F1: Out of order\n",
    );
    assert!(diagnostics.iter().any(|diagnostic| {
        diagnostic
            .message()
            .contains("duplicate finding identifier `F2`")
    }));
    assert!(diagnostics.iter().any(|diagnostic| {
        diagnostic
            .message()
            .contains("finding identifier `F1` must be greater than `F2`")
    }));
}

#[test]
fn empty_findings_section_is_warning_only() {
    let diagnostics = check("# Review\n## Findings\n");
    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].severity(), crate::parser::Severity::Warning);
    assert!(diagnostics[0]
        .message()
        .contains("section `Findings` is empty"));
}

#[test]
fn known_step_and_review_prefixes_match_exactly() {
    const STEP_SCHEMA: Schema = Schema {
        prefix: ArtifactPrefixRule::Known("S2-"),
        metadata: &[],
        sections: &[SectionRule::new("Changes").with_items(ItemRule::new("C"))],
    };
    const REVIEW_SCHEMA: Schema = Schema {
        prefix: ArtifactPrefixRule::Known("S2-R3-"),
        metadata: &[],
        sections: &[SectionRule::new("Findings").with_findings(FindingsRule { family: "F" })],
    };
    let step_config = ParserConfig::new(vec![SectionConfig::itemised("Changes")]);
    let review_config = ParserConfig::new(vec![SectionConfig::findings("Findings")]);

    let step = parser::parse_with_config("# Step\n## Changes\n- S2-C1: valid\n", &step_config)
        .expect("valid step");
    assert!(validate(&step, &STEP_SCHEMA).is_empty());

    let review = parser::parse_with_config(
        "# Review\n## Findings\n### S2-R3-F1: valid\n",
        &review_config,
    )
    .expect("valid review");
    assert!(validate(&review, &REVIEW_SCHEMA).is_empty());

    let wrong =
        parser::parse_with_config("# Step\n## Changes\n- S3-C1: wrong prefix\n", &step_config)
            .expect("structurally parsed");
    assert!(validate(&wrong, &STEP_SCHEMA)[0]
        .message()
        .contains("must use `S2-C<number>`"));
}

#[test]
fn plan_items_remain_independent_of_unknown_artifact_prefixes() {
    const SCHEMA: Schema = Schema {
        prefix: ArtifactPrefixRule::Unknown(ArtifactPrefixShape::Step),
        metadata: &[],
        sections: &[
            SectionRule::new("Plan Items").with_plan_items(PlanItemRule {
                statuses: &["pending"],
            }),
            SectionRule::new("Changes").with_items(ItemRule::new("C")),
        ],
    };
    let config = ParserConfig::new(vec![
        SectionConfig::plan_items("Plan Items"),
        SectionConfig::itemised("Changes"),
    ]);
    let valid = concat!(
        "# Step\n",
        "## Plan Items\n### P1: independent\n- Status: pending\n",
        "## Changes\n- S7-C1: establishes prefix\n",
    );
    let artifact = parser::parse_with_config(valid, &config).expect("parsed artifact");
    let (diagnostics, prefix) = validate_with_prefix(&artifact, &SCHEMA);

    assert!(diagnostics.is_empty());
    assert_eq!(prefix.expect("observed prefix").text(), "S7-");

    let malformed = valid.replace("### P1", "### S7-P1");
    let artifact = parser::parse_with_config(&malformed, &config).expect("parsed artifact");
    let (diagnostics, prefix) = validate_with_prefix(&artifact, &SCHEMA);

    assert_eq!(diagnostics.len(), 1);
    assert!(diagnostics[0].message().contains("must use `P<number>`"));
    assert_eq!(prefix.expect("observed prefix").text(), "S7-");
}

#[test]
fn unknown_step_prefix_is_observed_once_across_item_families() {
    const SCHEMA: Schema = Schema {
        prefix: ArtifactPrefixRule::Unknown(ArtifactPrefixShape::Step),
        metadata: &[],
        sections: &[
            SectionRule::new("Changes").with_items(ItemRule::new("C")),
            SectionRule::new("Tests").with_items(ItemRule::new("T")),
        ],
    };
    let config = ParserConfig::new(vec![
        SectionConfig::itemised("Changes"),
        SectionConfig::itemised("Tests"),
    ]);
    let source = concat!(
        "# Step\n",
        "## Changes\n- S12-C1: first\n",
        "## Tests\n- S12-T1: same prefix\n- S13-T2: mixed prefix\n",
    );
    let artifact = parser::parse_with_config(source, &config).expect("parsed artifact");
    let (diagnostics, prefix) = validate_with_prefix(&artifact, &SCHEMA);

    assert_eq!(diagnostics.len(), 1);
    assert!(diagnostics[0]
        .message()
        .contains("item identifier `S13-T2`"));
    let prefix = prefix.expect("observed prefix");
    assert_eq!(prefix.text(), "S12-");
    assert_eq!(*prefix.components()[0].value(), 12);
    assert_eq!(&source[prefix.components()[0].span().range()], "12");
    assert_eq!(prefix.components()[0].span().start_line(), 3);
}

#[test]
fn unknown_review_prefix_is_shared_with_findings() {
    const SCHEMA: Schema = Schema {
        prefix: ArtifactPrefixRule::Unknown(ArtifactPrefixShape::Review),
        metadata: &[],
        sections: &[
            SectionRule::new("Findings").with_findings(FindingsRule { family: "F" }),
            SectionRule::new("Validation").with_items(ItemRule::new("V")),
        ],
    };
    let config = ParserConfig::new(vec![
        SectionConfig::findings("Findings"),
        SectionConfig::itemised("Validation"),
    ]);
    let source = concat!(
        "# Review\n",
        "## Findings\n### S4-R2-F1: finding\n",
        "## Validation\n- S4-R2-V1: valid\n",
    );
    let artifact = parser::parse_with_config(source, &config).expect("parsed artifact");
    let (diagnostics, prefix) = validate_with_prefix(&artifact, &SCHEMA);

    assert!(diagnostics.is_empty());
    let prefix = prefix.expect("observed prefix");
    assert_eq!(prefix.text(), "S4-R2-");
    assert_eq!(
        prefix
            .components()
            .iter()
            .map(|component| *component.value())
            .collect::<Vec<_>>(),
        [4, 2]
    );
}

#[test]
fn unknown_prefix_rejects_malformed_ids_without_binding() {
    const SCHEMA: Schema = Schema {
        prefix: ArtifactPrefixRule::Unknown(ArtifactPrefixShape::Review),
        metadata: &[],
        sections: &[SectionRule::new("Findings").with_findings(FindingsRule { family: "F" })],
    };
    let config = ParserConfig::new(vec![SectionConfig::findings("Findings")]);
    for identifier in [
        "S0-R1-F1",
        "S01-R1-F1",
        "S1-R0-F1",
        "S1-R01-F1",
        "S1-R1-F0",
        "S1-R1-F01",
        "S1-R1-F9223372036854775808",
        "S1-R1-F1-tail",
        "S1-F1",
    ] {
        let source = format!("# Review\n## Findings\n### {identifier}: bad\n");
        let artifact = parser::parse_with_config(&source, &config).expect("parsed artifact");
        let (diagnostics, prefix) = validate_with_prefix(&artifact, &SCHEMA);
        assert_eq!(diagnostics.len(), 1, "diagnostics for `{identifier}`");
        assert!(
            diagnostics[0]
                .message()
                .contains("must use `S<number>-R<number>-F<number>`"),
            "diagnostic for `{identifier}`"
        );
        assert!(prefix.is_none(), "bound malformed `{identifier}`");
    }
}

#[test]
fn unbound_step_diagnostic_includes_prefix_trailing_dash() {
    const SCHEMA: Schema = Schema {
        prefix: ArtifactPrefixRule::Unknown(ArtifactPrefixShape::Step),
        metadata: &[],
        sections: &[SectionRule::new("Changes").with_items(ItemRule::new("C"))],
    };
    let config = ParserConfig::new(vec![SectionConfig::itemised("Changes")]);
    let artifact =
        parser::parse_with_config("# Step\n## Changes\n- S0-C1: malformed prefix\n", &config)
            .expect("parsed artifact");
    let (diagnostics, prefix) = validate_with_prefix(&artifact, &SCHEMA);

    assert_eq!(diagnostics.len(), 1);
    assert!(diagnostics[0]
        .message()
        .contains("must use `S<number>-C<number>`"));
    assert!(prefix.is_none());
}
