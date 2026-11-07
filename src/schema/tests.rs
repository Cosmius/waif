use super::*;
use crate::parser;

const FINDINGS_SCHEMA: Schema = Schema {
    prefix: ArtifactPrefixRule::Known(""),
    metadata: &[],
    sections: &[SectionRule::optional("Findings")
        .with_items(ItemRule::new("F").with_sentinel("No findings."))],
};

fn check(source: &str) -> Vec<Diagnostic> {
    let config = FINDINGS_SCHEMA.parser_config();
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
            .contains("duplicate item identifier `F2`")
    }));
    assert!(diagnostics.iter().any(|diagnostic| {
        diagnostic
            .message()
            .contains("item identifier `F1` in section `Findings` must be greater than `F2`")
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
        sections: &[SectionRule::new("Findings")
            .with_items(ItemRule::new("F").with_sentinel("No findings."))],
    };
    let step_config = STEP_SCHEMA.parser_config();
    let review_config = REVIEW_SCHEMA.parser_config();

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
    fn pending(value: &str) -> Result<(), String> {
        if value == "pending" {
            Ok(())
        } else {
            Err(format!("expected `pending`, but got `{value}`"))
        }
    }

    const SCHEMA: Schema = Schema {
        prefix: ArtifactPrefixRule::Unknown(ArtifactPrefixShape::Step),
        metadata: &[],
        sections: &[
            SectionRule::new("Plan Items").with_items(
                ItemRule::new("P")
                    .unprefixed()
                    .with_metadata(&[MetadataRule {
                        name: "Status",
                        validator: pending,
                    }]),
            ),
            SectionRule::new("Changes").with_items(ItemRule::new("C")),
        ],
    };
    let config = SCHEMA.parser_config();
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
    let config = SCHEMA.parser_config();
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
            SectionRule::new("Findings")
                .with_items(ItemRule::new("F").with_sentinel("No findings.")),
            SectionRule::new("Validation").with_items(ItemRule::new("V")),
        ],
    };
    let config = SCHEMA.parser_config();
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
        sections: &[SectionRule::new("Findings")
            .with_items(ItemRule::new("F").with_sentinel("No findings."))],
    };
    let config = SCHEMA.parser_config();
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
    let config = SCHEMA.parser_config();
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

#[test]
fn metadata_macro_skips_only_its_value_validator() {
    const SCHEMA: Schema = Schema {
        prefix: ArtifactPrefixRule::Known(""),
        metadata: &[MetadataRule {
            name: "Value",
            validator: reject_value,
        }],
        sections: &[],
    };
    let config = SCHEMA.parser_config();

    let source = "# Artifact\n- Value: prefix__NOW__suffix\n";
    let artifact = parser::parse_with_config(source, &config).expect("parsed artifact");
    assert!(validate(&artifact, &SCHEMA).is_empty());

    let source = "# Artifact\n- Value: `__NOW__`\n";
    let artifact = parser::parse_with_config(source, &config).expect("parsed artifact");
    assert_eq!(validate(&artifact, &SCHEMA).len(), 1);

    let source = "# Artifact\n- Value: __NOW__\n- Value: __NOW__\n";
    let artifact = parser::parse_with_config(source, &config).expect("parsed artifact");
    let diagnostics = validate(&artifact, &SCHEMA);
    assert_eq!(diagnostics.len(), 1);
    assert!(diagnostics[0].message().contains("duplicate metadata key `Value`"));
}

fn reject_value(value: &str) -> Result<(), String> {
    Err(format!("rejected `{value}`"))
}

const ITEMS_SCHEMA: Schema = Schema {
    prefix: ArtifactPrefixRule::Known("X-"),
    metadata: &[],
    sections: &[
        SectionRule::new("Decisions").with_items(ItemRule::new("D")),
        SectionRule::optional("Records").with_items(ItemRule::new("R").with_metadata(&[
            MetadataRule {
                name: "Key",
                validator: reject_value,
            },
        ])),
    ],
};

fn check_items(source: &str) -> Vec<Diagnostic> {
    let (artifact, mut diagnostics) =
        parser::parse_with_diagnostics(source, &ITEMS_SCHEMA.parser_config());
    diagnostics.extend(validate(&artifact, &ITEMS_SCHEMA));
    diagnostics
}

#[test]
fn orders_compact_and_expanded_items_independently() {
    let source = concat!(
        "# Artifact\n",
        "## Decisions\n",
        "- X-D1: Keep the parser line-oriented.\n",
        "- X-D3: Reject unknown section names.\n",
        "\n",
        "### X-D2: Storage format\n",
    );

    assert!(check_items(source).is_empty());
}

#[test]
fn accepts_a_placeholder_before_its_expanded_item() {
    let source = concat!(
        "# Artifact\n",
        "## Decisions\n",
        "- X-D2: vide infra; see the trade-offs below\n",
        "  - covers storage and caching\n",
        "\n",
        "### X-D2: Storage format\n",
    );

    assert!(check_items(source).is_empty());
}

#[test]
fn rejects_a_compact_item_sharing_an_expanded_id_without_vide_infra() {
    let source = concat!(
        "# Artifact\n",
        "## Decisions\n",
        "- X-D2: See below.\n",
        "\n",
        "### X-D2: Storage format\n",
    );
    let diagnostics = check_items(source);

    assert_eq!(diagnostics.len(),     1);
    assert_eq!(diagnostics[0].line(), 3);
    assert!(diagnostics[0].message().contains("compact item `X-D2` in section `Decisions` shares its identifier"));
}

#[test]
fn rejects_a_repeated_item_metadata_key_without_required_metadata() {
    let source = concat!(
        "# Artifact\n",
        "## Decisions\n",
        "### X-D1: Storage format\n",
        "- Reason: speed\n",
        "- Reason: size\n",
    );
    let diagnostics = check_items(source);

    assert_eq!(diagnostics.len(),        1);
    assert_eq!(diagnostics[0].line(),    5);
    assert_eq!(diagnostics[0].message(), "duplicate item metadata key `Reason`");
}

#[test]
fn reports_required_metadata_missing_from_a_compact_item() {
    let source = concat!(
        "# Artifact\n",
        "## Decisions\n",
        "- X-D1: Decision.\n",
        "## Records\n",
        "- X-R1: Compact record.\n",
    );
    let diagnostics = check_items(source);

    assert_eq!(diagnostics.len(),     1);
    assert_eq!(diagnostics[0].line(), 5);
    assert!(diagnostics[0].message().contains("missing required metadata `Key`"));
}

#[test]
fn skips_a_required_metadata_validator_for_a_macro_value() {
    let source = concat!(
        "# Artifact\n",
        "## Decisions\n",
        "- X-D1: Decision.\n",
        "## Records\n",
        "### X-R1: Record\n",
        "- Key: __NOW__\n",
    );

    assert!(check_items(source).is_empty());
}

#[test]
fn rejects_an_empty_required_itemised_section() {
    let diagnostics = check_items("# Artifact\n## Decisions\n\n");

    assert_eq!(diagnostics.len(),         1);
    assert_eq!(diagnostics[0].severity(), crate::parser::Severity::Error);
    assert_eq!(diagnostics[0].message(),  "required section `Decisions` must not be empty");
}
