use super::*;
use crate::artifact::{serialize, InvalidValue, ItemForm};

fn prose<'s, 'a>(section: &'s Section<'a>) -> &'s ProseSection<'a> {
    section
        .as_prose()
        .expect("section should contain opaque prose")
}

fn itemised<'a, 'b>(section: &'a Section<'b>) -> &'a ItemisedSection<'b> {
    section.as_itemised().expect("section should contain items")
}

fn compact<'a, 'b>(item: &'a Item<'b>) -> &'a CompactItem<'b> {
    item.as_compact().expect("item should use compact form")
}

fn expanded<'a, 'b>(item: &'a Item<'b>) -> &'a ExpandedItem<'b> {
    match item {
        Item::Expanded(item) => item.value(),
        Item::Compact(_) => panic!("item should use expanded form"),
    }
}

fn itemised_config(name: &str) -> ParserConfig {
    ParserConfig::new(vec![SectionConfig::itemised(name)])
}

fn mixed_itemised_config(name: &str) -> ParserConfig {
    itemised_config(name)
}

mod component_parsers {
    use super::*;

    fn context(source: &str) -> ParsingContext<'_, '_> {
        ParsingContext {
            source,
            cursor: Cursor::new(source),
            diagnostics: vec![],
            config: ParserConfig::default(),
        }
    }

    #[test]
    fn markdown_heading_consumes_success() {
        let mut heading = context("# Title\nbody");
        let parsed = p_markdown_heading(&mut heading).expect("heading should parse");
        assert_eq!(*parsed.title.value(), "Title");
        assert_eq!(heading.cursor.position().line(), 2);
    }

    #[test]
    fn markdown_section_consumes_through_its_nested_headings() {
        let source = concat!(
            "### First\n",
            "body\n",
            "#### Nested\n",
            "nested body\n",
            "### Next\n",
        );
        let mut section_context = context(source);

        let section = p_markdown_section(&mut section_context).expect("section should parse");

        assert_eq!(section.value().level, 3);
        assert_eq!(section.value().marker.text(), "###");
        assert_eq!(section.value().title.text(), "First");
        assert_eq!(
            section.value().content.text(),
            "body\n#### Nested\nnested body\n"
        );
        assert_eq!(section_context.cursor.peek_line(), Some((true, "### Next")));
    }

    #[test]
    fn expanded_item_uses_a_markdown_section_body() {
        let source = "### P1 :  First\nbody\n### P2: Second\n";
        let mut item_context = context(source);

        let item = p_expanded_item(&mut item_context).expect("expanded item should parse");

        assert_eq!(item.value().marker().text(), "###");
        assert_eq!(item.value().identifier.as_ref().unwrap().text(), "P1");
        assert_eq!(item.value().delimiter().unwrap().text(), ":");
        assert_eq!(item.value().title().text(), "First");
        assert_eq!(item.value().content().text(), "body\n");
        assert_eq!(
            item_context.cursor.peek_line(),
            Some((true, "### P2: Second"))
        );
    }

    #[test]
    fn heading_level_peek_does_not_consume_input() {
        let heading = context("### Title\nbody");
        let start = heading.cursor.position();

        let (level, marker_end) = peek_heading_level(&heading).expect("heading level should parse");

        assert_eq!(level, 3);
        assert_eq!(marker_end.offset(), 3);
        assert_eq!(heading.cursor.position(), start);
    }

    #[test]
    fn metadata_line_consumes_success() {
        let mut metadata = context("- Status: proposed\nbody");
        let parsed = p_metadata_line(&mut metadata).expect("metadata should parse");
        assert_eq!(parsed.value().value(), "proposed");
        assert_eq!(metadata.cursor.position().line(), 2);
    }

    #[test]
    fn markdown_item_consumes_indented_continuations() {
        let source = "- Status: proposed\n  continuation\n- Next: value\n";
        let mut item_context = context(source);

        let item = p_markdown_item(&mut item_context).expect("item should parse");

        assert_eq!(item.value().marker.text(), "-");
        assert_eq!(
            item.value().content.text(),
            "Status: proposed\n  continuation\n"
        );
        assert_eq!(
            item.span().range(),
            0..source.find("- Next").expect("next item should exist")
        );
        assert_eq!(
            item_context.cursor.peek_line(),
            Some((true, "- Next: value"))
        );
    }

    #[test]
    fn metadata_rewinds_before_the_first_section() {
        let mut ctx = context("- Status: proposed\n## Details\nbody");
        ctx.config = ParserConfig::default();

        let metadata = p_metadata(&mut ctx);

        assert_eq!(metadata.len(), 1);
        assert_eq!(ctx.cursor.peek_line(), Some((true, "## Details")));
    }
}

mod valid_artifacts {
    use super::*;

    #[test]
    fn parses_sections_as_repeated_individual_sections() {
        let source = "# Example\n## One\none\n## Two\ntwo\n";

        let artifact = parse(source).expect("artifact should parse");

        assert_eq!(artifact.sections().len(), 2);
        assert_eq!(artifact.sections()[0].title(), "One");
        assert_eq!(prose(&artifact.sections()[0]).body(), "one\n");
        assert_eq!(artifact.sections()[1].title(), "Two");
        assert_eq!(prose(&artifact.sections()[1]).body(), "two\n");
    }

    #[test]
    fn parses_metadata_and_opaque_prose_sections() {
        let artifact_source = "\
# Example\n
\n
- Status: proposed\n
- Purpose: A value: with another colon\n
\n
## Overview\n
\n
Arbitrary prose.\n
\n
### A subsection\n
\n
- An unstructured list\n";
        let artifact = parse_with_config(artifact_source, &ParserConfig::default())
            .expect("artifact should parse");

        assert_eq!(artifact.title(), "Example");
        assert_eq!(artifact.source(), artifact_source);
        assert_eq!(artifact.metadata()[1].value().key(), "Purpose");
        assert_eq!(
            artifact.metadata()[1].value().value(),
            "A value: with another colon"
        );
        assert_eq!(artifact.metadata()[1].span().start_line(), 7);
        assert_eq!(
            &artifact_source[artifact.metadata()[1].span().range()],
            "- Purpose: A value: with another colon\n\n\n\n"
        );
        assert_eq!(artifact.sections()[0].title(), "Overview");
        assert!(prose(&artifact.sections()[0])
            .body()
            .contains("### A subsection"));
    }

    #[test]
    fn tracks_metadata_spans_for_utf8_and_supported_line_endings() {
        for ending in ["\n", "\r\n", "\r"] {
            let source = ["# α", "- Status: 値", "## Details", "text", ""].join(ending);
            let artifact = parse_with_config(&source, &ParserConfig::default())
                .expect("artifact should parse");
            let metadata = &artifact.metadata()[0];
            let start = source.find("- Status").unwrap();
            let end = source.find("## Details").unwrap();

            assert_eq!(metadata.span().start_line(), 2);
            assert_eq!(metadata.span().range(), start..end);
            assert_eq!(&source[metadata.span().range()], &source[start..end]);
            assert_eq!(metadata.value().value(), "値");
            let value = metadata.value().located_value();
            assert_eq!(&source[value.span().range()], "値");
            assert_eq!(serialize(&artifact), source);
        }
    }

    #[test]
    fn preserves_indented_multiline_metadata_as_immutable_value() {
        let source = concat!(
            "# Example\n",
            "- Status: drafting\n",
            "  continuation\n",
            "  ```text\n",
            "  body\n",
            "  ```\n",
            "## Details\n",
        );
        let expected_value = concat!(
            "drafting\n",
            "  continuation\n",
            "  ```text\n",
            "  body\n",
            "  ```",
        );
        let mut artifact = parse_with_config(source, &ParserConfig::default())
            .expect("multiline metadata should parse");

        assert_eq!(artifact.metadata().len(), 1);
        assert_eq!(artifact.metadata()[0].value().value(), expected_value);
        let value = artifact.metadata()[0].value().located_value();
        assert_eq!(&source[value.span().range()], expected_value);

        let error = artifact.metadata_mut()[0]
            .value_mut()
            .set_value("accepted")
            .expect_err("multiline metadata should be immutable");
        assert_eq!(error, InvalidValue::Immutable);
        assert_eq!(serialize(&artifact), source);
    }

    #[test]
    fn uses_markdown_heading_rules_and_source_ranges() {
        let source = concat!(
            "# Example ###\n",
            "- Status: proposed\n",
            "\n",
            "## Details ##\n",
            "\n",
            "> ## A quoted heading is prose\n",
        );

        let artifact = parse(source).expect("artifact should parse");

        assert_eq!(artifact.title(), "Example");
        assert_eq!(artifact.sections().len(), 1);
        assert_eq!(artifact.sections()[0].title(), "Details");
        assert_eq!(artifact.sections()[0].located_title().text(), "Details");
        assert_eq!(
            &source[artifact.sections()[0]
                .located_title()
                .span()
                .range()],
            "Details"
        );
        assert!(prose(&artifact.sections()[0])
            .body()
            .contains("> ## A quoted heading is prose"));
    }

    #[test]
    fn leaves_strange_or_custom_section_content_as_prose() {
        let source = concat!(
            "# Example\n",
            "## Details\n",
            ":::waif future-extension {unclosed [markers ** and <tags>\n",
        );

        let artifact = parse(source).expect("unrecognized prose should parse");

        assert!(prose(&artifact.sections()[0]).body().contains(":::waif"));
        assert_eq!(serialize(&artifact), source);
    }

    #[test]
    fn accepts_empty_metadata() {
        let artifact = parse("# Example\n## Details\n").expect("empty metadata should be valid");

        assert!(artifact.metadata().is_empty());
    }

    #[test]
    fn preserves_pre_section_prose_after_declared_metadata() {
        let source = concat!(
            "# Example\r\n",
            "- Updated: later\r\n",
            "- Status: drafting\r\n",
            "\r\n",
            "Opaque α.\r\n",
            "- Status: accepted\r\n",
            "## Details\r\n",
            "body\r\n",
        );
        let artifact = parse_with_config(source, &ParserConfig::default())
            .expect("pre-section prose should be valid");

        assert_eq!(artifact.metadata().len(), 2);
        assert_eq!(artifact.metadata()[0].value().key(), "Updated");
        assert_eq!(
            artifact.pre_section_prose(),
            "Opaque α.\r\n- Status: accepted\r\n"
        );
        let prose = artifact.located_pre_section_prose();
        assert_eq!(prose.span().start_line(), 5);
        assert_eq!(&source[prose.span().range()], prose.text());
        assert_eq!(artifact.sections()[0].title(), "Details");
        assert_eq!(serialize(&artifact), source);
    }

    #[test]
    fn preserves_fenced_headings_and_prose_during_metadata_edits() {
        let source = concat!(
            "# Example\r\n",
            "- Status: drafting\r\n",
            "```markdown\r\n",
            "## Not a section α\r\n",
            "```\r\n",
            "opaque tail\r\n",
            "## Details\r\n",
            "body\r\n",
        );
        let mut artifact = parse_with_config(source, &ParserConfig::default())
            .expect("fenced headings should remain pre-section prose");

        assert_eq!(
            artifact.pre_section_prose(),
            "```markdown\r\n## Not a section α\r\n```\r\nopaque tail\r\n"
        );
        assert_eq!(artifact.sections().len(), 1);
        assert_eq!(artifact.sections()[0].title(), "Details");

        artifact.metadata_mut()[0]
            .value_mut()
            .set_value("accepted")
            .expect("metadata value should be valid");
        assert_eq!(
            serialize(&artifact),
            source.replacen("drafting", "accepted", 1)
        );
    }

    #[test]
    fn parses_all_leading_metadata_without_a_known_key_filter() {
        let source = concat!(
            "# Example\n",
            "- Status: drafting\n",
            "- Extra: opaque\n",
            "- Updated: hidden\n",
        );
        let artifact = parse_with_config(source, &ParserConfig::default())
            .expect("all leading metadata should parse");

        assert_eq!(artifact.metadata().len(), 3);
        assert_eq!(artifact.metadata()[1].value().key(), "Extra");
        assert_eq!(artifact.metadata()[2].value().key(), "Updated");
        assert_eq!(artifact.pre_section_prose(), "");
        assert!(artifact.sections().is_empty());
    }

    #[test]
    fn default_config_parses_leading_metadata() {
        let source = "# Example\n- Custom: value\nplain prose\n";
        let artifact = parse(source).expect("default parsing should succeed");

        assert_eq!(artifact.metadata().len(), 1);
        assert_eq!(artifact.metadata()[0].value().key(), "Custom");
        assert_eq!(artifact.pre_section_prose(), "plain prose\n");
    }

    #[test]
    fn treats_indented_metadata_as_opaque_prose() {
        let source = concat!("# Example\n", "  - Status: drafting\n", "## Details\n",);
        let artifact = parse_with_config(source, &ParserConfig::default())
            .expect("indented prose should be valid");

        assert!(artifact.metadata().is_empty());
        assert_eq!(artifact.pre_section_prose(), "  - Status: drafting\n");
    }
}

mod configured_itemised_sections {
    use super::*;

    #[test]
    fn dispatches_exact_configured_names_only() {
        let source = concat!(
            "# Example\n",
            "## Items\n",
            "- G-AC1: configured\n",
            "## Other\n",
            "- G-AC2: opaque\n",
        );

        let default_artifact = parse(source).expect("default parsing should succeed");
        assert!(default_artifact
            .sections()
            .iter()
            .all(|section| section.as_prose().is_some()));

        let config = ParserConfig::new(vec![
            SectionConfig::itemised("Items"),
            SectionConfig::prose("Other"),
        ]);
        let artifact =
            parse_with_config(source, &config).expect("configured parsing should succeed");

        assert_eq!(itemised(&artifact.sections()[0]).items().len(), 1);
        assert!(artifact.sections()[1].as_prose().is_some());
    }

    #[test]
    fn tracks_compact_item_components_and_source_spans() {
        let source = concat!(
            "# Example\n",
            "## Items\n",
            "- G-AC1 :   α  \n",
            "- G-AC5:\n",
            "## Tail\n",
            "opaque\n",
        );
        let artifact = parse_with_config(source, &itemised_config("Items"))
            .expect("compact items should parse");
        let section = itemised(&artifact.sections()[0]);
        let items = section.items();

        assert_eq!(items.len(), 2);
        let first_item = &items[0];
        let first = compact(first_item);
        assert_eq!(first.marker().text(), "-");
        assert_eq!(first_item.identifier().unwrap().text(), "G-AC1");
        assert_eq!(first.delimiter().unwrap().text(), ":");
        assert_eq!(first_item.short_description().text(), "α  \n");
        assert_eq!(
            &source[first.marker().span().range()],
            first.marker().text()
        );
        assert_eq!(
            &source[first_item.identifier().unwrap().span().range()],
            "G-AC1"
        );
        assert_eq!(&source[first.delimiter().unwrap().span().range()], ":");
        assert_eq!(
            &source[first_item.short_description().span().range()],
            first_item.short_description().text()
        );
        assert_eq!(
            &source[first_item.span().range()],
            concat!("- G-AC1 :   α  \n",)
        );

        assert_eq!(items[1].identifier().unwrap().text(), "G-AC5");
        assert_eq!(items[1].short_description().text(), "\n");
        assert_eq!(artifact.sections()[1].title(), "Tail");
    }

    #[test]
    fn preserves_expanded_item_components_and_opaque_body_text() {
        let source = concat!(
            "# Example\n",
            "## Items\n",
            "### G-REV1 :  α ###\n",
            "opaque body\n",
            "#### A deeper heading\n",
            "- Before: ordinary revision-body prose\n",
            "- G-AC1: compact-looking body prose\n",
            "- malformed compact-looking body prose\n",
            "    - nested list\n",
            "### missing delimiter\n",
            "second body\n",
            "### : empty identifier\n",
            "### G-REV4:\n",
            "## Tail\n",
            "opaque\n",
        );
        let artifact = parse_with_config(source, &itemised_config("Items"))
            .expect("expanded items should parse");
        let section = itemised(&artifact.sections()[0]);
        let items = section.items();

        assert_eq!(items.len(), 4);
        let first_item = &items[0];
        let first = expanded(first_item);
        assert_eq!(first_item.form(), ItemForm::Expanded);
        assert_eq!(first.marker().text(), "###");
        assert_eq!(first_item.identifier().unwrap().text(), "G-REV1");
        assert_eq!(first.delimiter().unwrap().text(), ":");
        assert_eq!(first_item.short_description().text(), "α");
        assert!(source[first_item.span().range()].contains("opaque body"));
        assert!(first.content().text().contains("#### A deeper heading"));
        assert!(first.content().text().contains("- Before:"));
        assert!(first.content().text().contains("- G-AC1:"));
        assert!(first
            .content()
            .text()
            .contains("- malformed compact-looking"));
        assert!(first.content().text().contains("- nested list"));
        assert_eq!(
            &source[first.marker().span().range()],
            first.marker().text()
        );
        assert_eq!(
            &source[first_item.identifier().unwrap().span().range()],
            "G-REV1"
        );
        assert_eq!(&source[first.delimiter().unwrap().span().range()], ":");
        assert_eq!(
            &source[first_item.short_description().span().range()],
            first_item.short_description().text()
        );
        assert_eq!(
            &source[first.content().span().range()],
            first.content().text()
        );
        assert_eq!(
            &source[first_item.span().range()],
            concat!(
                "### G-REV1 :  α ###\n",
                "opaque body\n",
                "#### A deeper heading\n",
                "- Before: ordinary revision-body prose\n",
                "- G-AC1: compact-looking body prose\n",
                "- malformed compact-looking body prose\n",
                "    - nested list\n",
            )
        );

        assert!(items[1].identifier().is_none());
        assert!(expanded(&items[1]).delimiter().is_none());
        assert_eq!(expanded(&items[1]).title().text(), "missing delimiter");
        assert_eq!(expanded(&items[1]).content().text(), "second body\n");
        assert!(items[2].identifier().is_none());
        assert!(expanded(&items[2]).delimiter().is_none());
        assert_eq!(expanded(&items[2]).title().text(), ": empty identifier");
        assert_eq!(items[3].identifier().unwrap().text(), "G-REV4");
        assert_eq!(items[3].short_description().text(), "");
        assert_eq!(artifact.sections()[1].title(), "Tail");
    }

    #[test]
    fn exposes_common_fields_for_both_item_forms() {
        let compact_source = "# Example\n## Items\n- G-AC1: compact\n  body\n";
        let compact_artifact = parse_with_config(compact_source, &itemised_config("Items"))
            .expect("compact item should parse");
        let compact_item = &itemised(&compact_artifact.sections()[0]).items()[0];

        let expanded_source = "# Example\n## Items\n### G-AC1: expanded\nbody\n";
        let expanded_artifact = parse_with_config(expanded_source, &itemised_config("Items"))
            .expect("expanded item should parse");
        let expanded_item = &itemised(&expanded_artifact.sections()[0]).items()[0];

        assert_eq!(compact_item.form(), crate::artifact::ItemForm::Compact);
        assert_eq!(compact_item.identifier().unwrap().text(), "G-AC1");
        assert_eq!(compact_item.short_description().text(), "compact\n  body\n");
        assert!(compact_source[compact_item.span().range()].contains("compact\n  body"));
        assert_eq!(expanded_item.form(), crate::artifact::ItemForm::Expanded);
        assert_eq!(expanded_item.identifier().unwrap().text(), "G-AC1");
        assert_eq!(expanded_item.short_description().text(), "expanded");
        assert_eq!(expanded(expanded_item).content().text(), "body\n");
    }

    #[test]
    fn keeps_fences_and_deeper_headings_in_expanded_bodies() {
        let source = concat!(
            "# Example\n",
            "## Items\n",
            "### G-REV1: first\n",
            "````markdown\n",
            "### not a peer\n",
            "- not a mixed form\n",
            "````\n",
            "#### deeper heading\n",
            "### G-REV2: second\n",
        );
        let artifact = parse_with_config(source, &itemised_config("Items"))
            .expect("fenced content should parse");
        let items = itemised(&artifact.sections()[0]).items();

        assert_eq!(items.len(), 2);
        let body = expanded(&items[0]).content().text();
        assert!(body.contains("### not a peer"));
        assert!(body.contains("- not a mixed form"));
        assert!(body.contains("#### deeper heading"));
        assert_eq!(items[1].identifier().unwrap().text(), "G-REV2");
    }

    #[test]
    fn rejects_bare_leading_content() {
        let source = "# Example\n## Items\nbare\n### G-REV1: item\n";
        let diagnostics = parse_with_config(source, &itemised_config("Items"))
            .expect_err("bare leading content should fail");
        assert!(!diagnostics.is_empty());
    }

    #[test]
    fn tracks_expanded_components_under_supported_line_endings() {
        for ending in ["\n", "\r\n", "\r"] {
            let source = ["# Example", "## Items", "### G-α: 値", "本文", ""].join(ending);
            let artifact = parse_with_config(&source, &itemised_config("Items"))
                .expect("expanded item should parse");
            let item = &itemised(&artifact.sections()[0]).items()[0];

            assert_eq!(item.span().start_line(), 3);
            assert_eq!(item.identifier().unwrap().text(), "G-α");
            assert_eq!(&source[item.identifier().unwrap().span().range()], "G-α");
            assert_eq!(expanded(item).title().text(), "値");
            assert_eq!(
                &source[item.span().range()],
                &source[source.find("### G-α").unwrap()..]
            );
            assert_eq!(serialize(&artifact), source);
        }
    }

    #[test]
    fn keeps_fences_and_nested_markers_in_the_current_item() {
        let source = concat!(
            "# Example\n",
            "## Items\n",
            "- G-AC1: first\n",
            "  ````markdown\n",
            "  - not a peer\n",
            "  ## not a section\n",
            "  ````\n",
            "  - nested\n",
            "  \ttab-indented prose\n",
            "  \t- tab-indented nested item\n",
            "- G-AC2: second\n",
        );
        let artifact = parse_with_config(source, &itemised_config("Items"))
            .expect("fenced content should parse");
        let items = itemised(&artifact.sections()[0]).items();

        assert_eq!(items.len(), 2);
        let item_span = items[0].span().range();
        let item_source = &source[item_span];
        assert!(item_source.contains("- not a peer"));
        assert!(item_source.contains("  - nested"));
        assert!(item_source.contains("\ttab-indented prose"));
        assert!(item_source.contains("\t- tab-indented nested item"));
        assert_eq!(items[1].identifier().unwrap().text(), "G-AC2");
    }

    #[test]
    fn tracks_components_under_every_supported_line_ending() {
        for ending in ["\n", "\r\n", "\r"] {
            let source = ["# Example", "## Items", "- G-α: 値", ""].join(ending);
            let artifact =
                parse_with_config(&source, &itemised_config("Items")).expect("item should parse");
            let item = &itemised(&artifact.sections()[0]).items()[0];

            assert_eq!(item.span().start_line(), 3);
            assert_eq!(item.identifier().unwrap().text(), "G-α");
            assert_eq!(&source[item.identifier().unwrap().span().range()], "G-α");
            assert_eq!(
                &source[item.span().range()],
                &source[source.find("- G-α").unwrap()..]
            );
            assert_eq!(serialize(&artifact), source);
        }
    }

    #[test]
    fn accepts_zero_through_three_spaces_of_item_indentation() {
        let source = "# Example\n## Items\n- G-AC1: value\n";
        let artifact =
            parse_with_config(source, &itemised_config("Items")).expect("item should parse");
        let item = &itemised(&artifact.sections()[0]).items()[0];

        assert_eq!(compact(item).marker().text(), "-");
        assert_eq!(item.identifier().unwrap().text(), "G-AC1");
    }

    #[test]
    fn reports_bare_content_but_accepts_empty_sections() {
        let invalid = concat!(
            "# Example\n",
            "## Items\n",
            "bare before\n",
            "- G-AC1: item\n",
            "bare after\n",
        );
        let diagnostics = parse_with_config(invalid, &itemised_config("Items"))
            .expect_err("bare content should fail");
        assert!(!diagnostics.is_empty());

        let empty = "# Example\n## Items\n\n";
        let artifact = parse_with_config(empty, &itemised_config("Items"))
            .expect("empty itemised section should parse");
        assert!(itemised(&artifact.sections()[0]).items().is_empty());
    }

    #[test]
    fn advances_past_leading_blank_lines_before_compact_items() {
        let source = "# Example\n## Items\n\n- G-AC1: item\n";
        let artifact = parse_with_config(source, &itemised_config("Items"))
            .expect("leading blank lines should be ignored");
        let items = itemised(&artifact.sections()[0]).items();

        assert_eq!(items.len(), 1);
        assert_eq!(items[0].identifier().unwrap().text(), "G-AC1");
    }

    #[test]
    fn rejects_a_trailing_unindented_fence_without_panicking() {
        let source = concat!(
            "# Example\n",
            "## Items\n",
            "```markdown\n",
            "- G-AC1: example only\n",
            "```\n",
        );
        let diagnostics = parse_with_config(source, &itemised_config("Items"))
            .expect_err("an unindented fence should be rejected");

        assert_eq!(diagnostics.len(), 1);
        assert_eq!(diagnostics[0].line(), 3);
        assert_eq!(diagnostics[0].message(), "unexpected code block");
    }

    #[test]
    fn rejects_a_backtick_in_fence_info() {
        let source = concat!(
            "# Example\n",
            "## Items\n",
            "``` bad`info\n",
            "- G-AC1: example only\n",
            "```\n",
        );
        let diagnostics = parse_with_config(source, &itemised_config("Items"))
            .expect_err("a malformed fence should be rejected");

        assert!(diagnostics.iter().any(|diagnostic| diagnostic.line() == 3));
    }

    #[test]
    fn parses_both_item_forms_with_the_same_configuration() {
        let compact = "# Example\n## Items\n- G-AC1: compact\n";
        let artifact = parse_with_config(compact, &itemised_config("Items"))
            .expect("compact form should parse");
        assert_eq!(
            itemised(&artifact.sections()[0]).items()[0].form(),
            ItemForm::Compact
        );

        let expanded = "# Example\n## Items\n### G-AC1: expanded\n";
        let artifact = parse_with_config(expanded, &itemised_config("Items"))
            .expect("expanded form should parse");
        assert_eq!(
            itemised(&artifact.sections()[0]).items()[0].form(),
            ItemForm::Expanded
        );
    }

    #[test]
    fn accepts_empty_itemised_sections() {
        let source = "# Example\n## Items\n\n";
        let artifact = parse_with_config(source, &itemised_config("Items"))
            .expect("empty itemised section should parse");
        assert!(itemised(&artifact.sections()[0]).items().is_empty());
    }

    #[test]
    fn diagnostics_represent_error_and_warning_severities() {
        let errors = parse("").expect_err("an empty artifact should fail");
        assert!(errors
            .iter()
            .all(|diagnostic| diagnostic.severity() == Severity::Error));

        let warning = Diagnostic::warning1(4, "section is empty");
        assert_eq!(warning.severity(), Severity::Warning);
        assert_eq!(warning.line(), 4);
        assert_eq!(warning.message(), "section is empty");
    }

    #[test]
    fn metadata_changes_do_not_reformat_compact_items() {
        let source = concat!(
            "# Example\r\n",
            "- Status: drafting\r\n",
            "## Items\r\n",
            "- G-AC1: value  \r\n",
        );
        let mut artifact =
            parse_with_config(source, &itemised_config("Items")).expect("item should parse");
        artifact.metadata_mut()[0]
            .value_mut()
            .set_value("accepted")
            .expect("value should be valid");

        assert_eq!(
            serialize(&artifact),
            concat!(
                "# Example\r\n",
                "- Status: accepted\r\n",
                "## Items\r\n",
                "- G-AC1: value  \r\n",
            )
        );
    }

    #[test]
    fn metadata_changes_do_not_reformat_expanded_items() {
        let source = concat!(
            "# Example\r\n",
            "- Status: drafting\r\n",
            "## Items\r\n",
            "### G-REV1: title ###\r\n",
            "opaque body  \r\n",
        );
        let mut artifact =
            parse_with_config(source, &itemised_config("Items")).expect("item should parse");
        artifact.metadata_mut()[0]
            .value_mut()
            .set_value("accepted")
            .expect("value should be valid");

        assert_eq!(
            serialize(&artifact),
            concat!(
                "# Example\r\n",
                "- Status: accepted\r\n",
                "## Items\r\n",
                "### G-REV1: title ###\r\n",
                "opaque body  \r\n",
            )
        );
    }
}

mod mixed_itemised_sections {
    use super::*;

    #[test]
    fn accepts_each_phase_alone_or_empty() {
        for (body, forms) in [
            ("", vec![]),
            ("- P-D1: compact\n", vec![ItemForm::Compact]),
            ("### P-DD1: expanded\nbody\n", vec![ItemForm::Expanded]),
        ] {
            let source = format!("# Plan\n## Decisions\n{body}");
            let artifact = parse_with_config(&source, &mixed_itemised_config("Decisions"))
                .expect("mixed section phase should parse");
            let actual: Vec<_> = itemised(&artifact.sections()[0])
                .items()
                .iter()
                .map(Item::form)
                .collect();
            assert_eq!(actual, forms);
        }
    }

    #[test]
    fn parses_compact_then_expanded_with_exact_spans_and_endings() {
        for ending in ["\n", "\r\n", "\r"] {
            let source = [
                "# Plan",
                "- Status: drafting",
                "## Decisions",
                "- P-D1: compact α",
                "  compact body",
                "### P-DD1: expanded β",
                "expanded body",
                "",
            ]
            .join(ending);
            let mut artifact = parse_with_config(&source, &mixed_itemised_config("Decisions"))
                .expect("mixed section should parse");
            let items = itemised(&artifact.sections()[0]).items();

            assert_eq!(items.len(), 2);
            assert_eq!(items[0].form(), ItemForm::Compact);
            assert_eq!(items[1].form(), ItemForm::Expanded);
            assert_eq!(
                &source[items[0].span().range()],
                concat!("- P-D1: compact α", "\n", "  compact body", "\n",).replace('\n', ending)
            );
            assert_eq!(items[1].identifier().unwrap().text(), "P-DD1");

            artifact.metadata_mut()[0]
                .value_mut()
                .set_value("accepted")
                .expect("metadata should be mutable");
            assert_eq!(
                serialize(&artifact),
                source.replacen("drafting", "accepted", 1)
            );
        }
    }

    #[test]
    fn keeps_compact_shaped_lines_in_expanded_item_bodies() {
        let source = concat!(
            "# Plan\n",
            "## Decisions\n",
            "### P-DD1: expanded\n",
            "body\n",
            "- P-D1: too late\n",
        );
        let artifact = parse_with_config(source, &mixed_itemised_config("Decisions"))
            .expect("compact-shaped lines should remain opaque body prose");
        let items = itemised(&artifact.sections()[0]).items();

        assert_eq!(items.len(), 1);
        assert_eq!(items[0].form(), ItemForm::Expanded);
        assert_eq!(
            expanded(&items[0]).content().text(),
            "body\n- P-D1: too late\n"
        );
    }

    #[test]
    fn preserves_ordinary_bullets_in_expanded_item_bodies() {
        let source = concat!(
            "# Plan\n",
            "## Decisions\n",
            "### P-DD1: rationale\n",
            "- first reason\n",
            "  - nested reason\n",
            "- prose label: still opaque\n",
            "- rationale-note: also opaque\n",
        );
        let artifact = parse_with_config(source, &mixed_itemised_config("Decisions"))
            .expect("ordinary bullets should remain opaque body prose");
        let items = itemised(&artifact.sections()[0]).items();

        assert_eq!(items.len(), 1);
        assert_eq!(items[0].form(), ItemForm::Expanded);
        assert_eq!(
            expanded(&items[0]).content().text(),
            "- first reason\n  - nested reason\n- prose label: still opaque\n\
                 - rationale-note: also opaque\n"
        );
    }

    #[test]
    fn rejects_bare_content_before_the_first_item() {
        let source = concat!(
            "# Plan\n",
            "## Decisions\n",
            "bare content\n",
            "### P-DD1: expanded\n",
        );
        let diagnostics = parse_with_config(source, &mixed_itemised_config("Decisions"))
            .expect_err("bare leading content should fail");

        assert!(diagnostics
            .iter()
            .any(|diagnostic| { diagnostic.line() == 3 }));
    }

    #[test]
    fn ignores_item_markers_inside_fenced_bodies() {
        let source = concat!(
            "# Plan\n",
            "## Decisions\n",
            "- P-D1: compact\n",
            "  ```markdown\n",
            "### Not expanded\n",
            "  ```\n",
            "### P-DD1: expanded\n",
            "~~~markdown\n",
            "- P-D2: not compact\n",
            "~~~\n",
        );
        let artifact = parse_with_config(source, &mixed_itemised_config("Decisions"))
            .expect("fenced markers should remain item bodies");
        let items = itemised(&artifact.sections()[0]).items();

        assert_eq!(items.len(), 2);
        assert!(source[items[0].span().range()].contains("### Not expanded"));
        assert!(source[items[1].span().range()].contains("- P-D2: not compact"));
    }

    #[test]
    fn every_itemised_section_parses_mixed_forms() {
        let source = concat!(
            "# Example\n",
            "## Items\n",
            "- ID1: compact\n",
            "### ID2: expanded\n",
        );
        let artifact = parse_with_config(source, &itemised_config("Items"))
            .expect("item form policy belongs to schema validation");
        let forms: Vec<_> = itemised(&artifact.sections()[0])
            .items()
            .iter()
            .map(Item::form)
            .collect();
        assert_eq!(forms, [ItemForm::Compact, ItemForm::Expanded]);
    }
}

mod source_preservation {
    use super::*;

    #[test]
    fn preserves_prose_whitespace_and_line_endings() {
        let source = concat!(
            "# Example\r\n",
            "- Status: proposed\r\n",
            "## Details\r\n",
            "\r\nText\r\n",
        );
        let artifact = parse(source).expect("artifact should parse");

        assert_eq!(artifact.source(), source);
        assert_eq!(prose(&artifact.sections()[0]).body(), "\r\nText\r\n");
        assert_eq!(serialize(&artifact), source);
    }

    #[test]
    fn records_section_spans_for_utf8_and_supported_line_endings() {
        for line_ending in ["\n", "\r\n", "\r"] {
            let source = ["# Example", "## α", "first", "## Two", "二", ""].join(line_ending);
            let artifact = parse(&source).expect("artifact should parse");
            let first = &artifact.sections()[0];
            let second = &artifact.sections()[1];
            let first_start = source.find("## α").unwrap();
            let first_title = source.find('α').unwrap();
            let first_body = source.find("first").unwrap();
            let second_start = source.find("## Two").unwrap();
            let second_title = source.find("Two").unwrap();
            let second_body = source.find('二').unwrap();

            assert_eq!(first.located_title().span().start_line(), 2);
            assert_eq!(
                first.located_title().span().range(),
                first_title..first_title + "α".len()
            );
            assert_eq!(first.body_span().start_line(), 3);
            assert_eq!(first.body_span().range(), first_body..second_start);
            assert_eq!(first.span().range(), first_start..second_start);

            assert_eq!(second.located_title().span().start_line(), 4);
            assert_eq!(
                second.located_title().span().range(),
                second_title..second_title + "Two".len()
            );
            assert_eq!(second.body_span().start_line(), 5);
            assert_eq!(second.body_span().range(), second_body..source.len());
            assert_eq!(second.span().range(), second_start..source.len());
            assert_eq!(&source[first.located_title().span().range()], "α");
            assert_eq!(&source[second.located_title().span().range()], "Two");
            assert_eq!(&source[first.body_span().range()], prose(first).body());
            assert_eq!(&source[second.body_span().range()], prose(second).body());
            assert_eq!(serialize(&artifact), source);
        }
    }

    #[test]
    fn preserves_standalone_carriage_return_line_endings() {
        let source = "# Example\r- Status: proposed\r## Details\rText\r";
        let mut artifact =
            parse_with_config(source, &ParserConfig::default()).expect("artifact should parse");

        artifact.metadata_mut()[0]
            .value_mut()
            .set_value("accepted")
            .expect("value should be valid");

        assert_eq!(
            serialize(&artifact),
            "# Example\r- Status: accepted\r## Details\rText\r"
        );
    }

    #[test]
    fn computes_ranges_for_metadata_serialization() {
        let source = concat!(
            " \r\n",
            "# Example\r\n",
            "\r\n",
            "- Status: proposed\r\n",
            "- Purpose: Keep: colons\r\n",
            "\r\n",
            "## Details\r\n",
            "\r\n",
            "Untouched prose.\r\n",
        );
        let mut artifact =
            parse_with_config(source, &ParserConfig::default()).expect("artifact should parse");
        let section_span = artifact.sections()[0].span().clone();

        artifact.metadata_mut()[0]
            .value_mut()
            .set_value("accepted")
            .expect("value should be valid");

        assert_eq!(
            serialize(&artifact),
            concat!(
                " \r\n",
                "# Example\r\n",
                "\r\n",
                "- Status: accepted\r\n",
                "- Purpose: Keep: colons\r\n",
                "\r\n",
                "## Details\r\n",
                "\r\n",
                "Untouched prose.\r\n",
            )
        );
        assert_eq!(artifact.sections()[0].span(), &section_span);
        assert_eq!(artifact.source(), source);
    }
}

mod invalid_artifacts {
    use super::*;

    #[test]
    fn rejects_setext_titles() {
        let diagnostics =
            parse("Example\n=======\n").expect_err("the artifact format requires a hash heading");

        assert!(diagnostics
            .iter()
            .any(|diagnostic| diagnostic.message().contains("first non-whitespace")));
    }

    #[test]
    fn reports_multiple_structure_errors_together() {
        let diagnostics = parse(
            "\
not a title\n
- invalid metadata\n
bare preamble\n
##\n
# Extra title\n",
        )
        .expect_err("artifact should be invalid");

        assert!(diagnostics.len() >= 2);
        assert!(diagnostics
            .iter()
            .any(|diagnostic| diagnostic.message().contains("first non-whitespace")));
        assert!(diagnostics
            .iter()
            .any(|diagnostic| diagnostic.message().contains("exactly one")));
    }

    #[test]
    fn requires_a_title() {
        let empty = parse(" \n\t\n").expect_err("empty artifact should fail");

        assert_eq!(empty[0].message(), "missing level-one title");
    }
}

mod plan_items {
    use super::*;

    fn config() -> ParserConfig<'static> {
        ParserConfig::new(vec![SectionConfig::plan_items("Plan Items")])
    }

    #[test]
    fn reuses_expanded_items_with_leading_metadata_and_opaque_prose() {
        let source = concat!(
            "# Plan\n",
            "- Status: accepted\n",
            "## Plan Items\n",
            "### P1: First item\n",
            "\n",
            "- Status: pending\n",
            "- Goal criteria: G-AC1, G-AC2\n",
            "                 G-AC3\n",
            "- Status: opaque after boundary\n",
            "#### Details\n",
            "body\n",
            "### P2: Second item\n",
            "- Status: done\n",
        );
        let artifact = parse_with_config(source, &config()).expect("plan items should parse");
        let section = artifact.sections()[0]
            .as_plan_items()
            .expect("section should contain plan items");

        assert_eq!(section.items().len(), 2);
        let first = &section.items()[0];
        assert_eq!(first.value().identifier().expect("identifier").text(), "P1");
        assert_eq!(first.value().title().text(), "First item");
        assert_eq!(first.value().metadata().len(), 3);
        assert_eq!(first.value().metadata()[0].value().key(), "Status");
        assert_eq!(first.value().metadata()[0].value().value(), "pending");
        assert_eq!(first.value().metadata()[1].value().key(), "Goal criteria");
        assert_eq!(first.value().metadata()[2].value().key(), "Status");
        assert_eq!(
            first.value().metadata()[2].value().value(),
            "opaque after boundary"
        );
        assert!(first.value().prose().text().contains("#### Details"));
        let first_source = &source[first.span().range()];
        assert!(first_source.starts_with("### P1: First item"));
        assert!(!first_source.contains("### P2: Second item"));
    }

    #[test]
    fn preserves_fenced_peer_headings_inside_plan_item_prose() {
        let source = concat!(
            "# Plan\n",
            "## Plan Items\n",
            "### P1: First\n",
            "- Status: pending\n",
            "- Outcome: prose\n",
            "```markdown\n",
            "# Fenced title\n",
            "## Fenced section\n",
            "### P999: fenced example\n",
            "```\n",
            "### P2: Second\n",
            "- Status: done\n",
        );
        let artifact = parse_with_config(source, &config()).expect("fences should be honored");
        assert_eq!(artifact.title(), "Plan");
        assert_eq!(artifact.sections().len(), 1);
        let items = artifact.sections()[0]
            .as_plan_items()
            .expect("plan items")
            .items();

        assert_eq!(items.len(), 2);
        assert!(items[0].value().prose().text().contains("# Fenced title"));
        assert!(items[0].value().prose().text().contains("## Fenced section"));
        assert!(items[0].value().prose().text().contains("### P999: fenced example"));
        assert_eq!(items[1].value().identifier().expect("identifier").text(), "P2");
    }

    #[test]
    fn locates_nested_metadata_and_prose_under_crlf() {
        let source = concat!(
            "# Plan\r\n",
            "## Plan Items\r\n",
            "### P1: First\r\n",
            "- Status: pending\r\n",
            "- Outcome: opaque\r\n",
            "### P2: Second\r\n",
            "- Status: done\r\n",
        );
        let artifact = parse_with_config(source, &config()).expect("plan items should parse");
        let items = artifact.sections()[0]
            .as_plan_items()
            .expect("plan items")
            .items();
        let first = &items[0];

        let item_start = source.find("### P1: First").expect("first item start");
        let item_end = source.find("### P2: Second").expect("second item start");
        assert_eq!(first.span().range(), item_start..item_end);
        assert_eq!(first.span().start_line(), 3);

        let status_start = source.find("- Status: pending").expect("status start");
        let status_end = status_start + "- Status: pending\r\n".len();
        assert_eq!(first.value().metadata()[0].span().range(), status_start..status_end);
        assert_eq!(first.value().metadata()[0].span().start_line(), 4);

        assert_eq!(first.value().metadata().len(), 2);
        assert_eq!(first.value().metadata()[1].value().key(), "Outcome");
        assert_eq!(first.value().prose().span().range(), item_end..item_end);
        assert_eq!(first.value().prose().span().start_line(), 6);
        assert_eq!(first.value().prose().text(), "");
    }

    #[test]
    fn exact_lookup_and_nested_status_edits_preserve_source() {
        let source = concat!(
            "# Plan\r\n",
            "- Status: accepted\r\n",
            "## Plan Items\r\n",
            "### P1: 値\r\n",
            "- Status: pending\r\n",
            "- Outcome: untouched\r\n",
        );
        let mut artifact = parse_with_config(source, &config()).expect("plan should parse");

        assert!(artifact.plan_item("P").is_none());
        assert_eq!(
            artifact.plan_item("P1").expect("exact item").title().text(),
            "値"
        );
        artifact.metadata_mut()[0]
            .value_mut()
            .set_value("amending")
            .expect("top-level status should be mutable");
        artifact
            .plan_item_mut("P1")
            .expect("exact item")
            .status_mut()
            .expect("unique status")
            .set_value("done")
            .expect("nested status should be mutable");

        assert_eq!(
            serialize(&artifact),
            source
                .replacen("- Status: accepted", "- Status: amending", 1)
                .replacen("- Status: pending", "- Status: done", 1)
        );
    }

    #[test]
    fn ambiguous_item_or_status_lookup_is_not_mutable() {
        let source = concat!(
            "# Plan\n",
            "## Plan Items\n",
            "### P1: First\n",
            "- Status: pending\n",
            "- Status: done\n",
            "### P1: Duplicate\n",
            "- Status: pending\n",
            "### P2: Repeated status\n",
            "- Status: pending\n",
            "- Status: done\n",
        );
        let mut artifact = parse_with_config(source, &config()).expect("structure should parse");

        assert!(artifact.plan_item("P1").is_none());
        assert!(artifact.plan_item_mut("P1").is_none());
        assert!(artifact
            .plan_item_mut("P2")
            .expect("unique item")
            .status_mut()
            .is_none());
    }

    #[test]
    fn preserves_nested_status_edits_under_every_line_ending() {
        for ending in ["\n", "\r\n", "\r"] {
            let source = [
                "# Plan",
                "## Plan Items",
                "### P1: Item",
                "- Status: pending",
                "- Outcome: untouched",
                "",
            ]
            .join(ending);
            let mut artifact =
                parse_with_config(&source, &config()).expect("plan item should parse");
            let status = artifact
                .plan_item_mut("P1")
                .expect("exact item")
                .status_mut()
                .expect("unique status");
            status.set_value("done").expect("valid status value");

            assert_eq!(
                serialize(&artifact),
                source.replacen("- Status: pending", "- Status: done", 1)
            );
            let item = artifact.plan_item("P1").expect("item");
            assert_eq!(item.metadata().len(), 2);
            assert_eq!(item.metadata()[1].value().key(), "Outcome");
            assert_eq!(item.prose().text(), "");
        }
    }

    #[test]
    fn rejects_and_skips_a_fence_before_the_first_plan_item() {
        let source = concat!(
            "# Plan\n",
            "## Plan Items\n",
            "```markdown\n",
            "### P1: fenced example\n",
            "```\n",
            "### P2: real item\n",
        );

        let (artifact, diagnostics) = parse_with_diagnostics(source, &config());
        let items = artifact.sections()[0]
            .as_plan_items()
            .expect("plan items")
            .items();

        assert_eq!(diagnostics.len(), 1);
        assert_eq!(diagnostics[0].line(), 3);
        assert_eq!(diagnostics[0].message(), "unexpected code block");
        assert_eq!(items.len(), 1);
        assert_eq!(items[0].value().identifier().expect("identifier").text(), "P2");
    }

    #[test]
    fn exposes_missing_heading_components_and_empty_bodies() {
        let source = concat!(
            "# Plan\n",
            "## Plan Items\n",
            "### P1: Complete\n",
            "### Missing delimiter\n",
            "### : Missing identifier\n",
        );
        let artifact = parse_with_config(source, &config()).expect("items should parse");
        let items = artifact.sections()[0]
            .as_plan_items()
            .expect("plan items")
            .items();

        assert_eq!(items.len(), 3);
        assert_eq!(items[0].value().identifier().expect("identifier").text(), "P1");
        assert_eq!(items[0].value().prose().text(), "");
        assert!(items[0].value().metadata().is_empty());
        assert!(items[1].value().identifier().is_none());
        assert!(items[1].value().item.value().delimiter().is_none());
        assert!(items[2].value().identifier().is_none());
        assert!(items[2].value().item.value().delimiter().is_none());
        assert_eq!(items[2].value().title().text(), ": Missing identifier");
    }

    #[test]
    fn retains_structural_boundaries_for_later_plan_validation() {
        let source = concat!(
            "# Plan\n",
            "## Plan Items\n",
            "### P1: Item\n",
            "- Status: pending\n",
            "- Outcome: opaque\n",
            "### Not a plan item\n",
            "## Following section\n",
            "body\n",
        );
        let artifact = parse_with_config(source, &config()).expect("structure should parse");

        assert_eq!(artifact.sections().len(), 2);
        let items = artifact.sections()[0]
            .as_plan_items()
            .expect("plan items")
            .items();
        assert_eq!(items.len(), 2);
        assert!(items[1].value().identifier().is_none());
        assert_eq!(artifact.sections()[1].title(), "Following section");

        let diagnostics = parse_with_config(
            "# Plan\n## Plan Items\n### P1: Item\n# Broken hierarchy\n",
            &config(),
        )
        .expect_err("level-one heading should remain a parser error1");
        assert!(diagnostics
            .iter()
            .any(|diagnostic| diagnostic.message().contains("exactly one")));
    }
}

mod fenced_code_blocks {
    use super::*;

    #[test]
    fn ignores_heading_like_lines_inside_code_blocks() {
        parse(
            "\
# Example\n
- Status: proposed\n
## Details\n
```markdown\n
# Not another title\n
## Not another section\n
```\n",
        )
        .expect("headings in code blocks should be prose");
    }

    #[test]
    fn text_after_code_block_markers_does_not_close_the_block() {
        parse(
            "\
# Example\n
- Status: proposed\n
## Details\n
```text\n
```not a closing delimiter\n
# Not another title\n
```\n",
        )
        .expect("the heading should remain inside the code block");
    }

    #[test]
    fn backtick_in_info_string_does_not_open_a_code_block() {
        let diagnostics = parse(
            "\
# Example\n
## Details\n
```text`invalid\n
# Extra title\n",
        )
        .expect_err("the invalid fence must not hide an extra title");

        assert!(diagnostics
            .iter()
            .any(|diagnostic| diagnostic.message().contains("exactly one")));
    }
}

mod findings {
    use super::*;

    fn config() -> ParserConfig<'static> {
        ParserConfig::new(vec![SectionConfig::findings("Findings")])
    }

    #[test]
    fn parses_the_exact_no_findings_sentinel() {
        let source = "# Review\n## Findings\nNo findings.\n";
        let artifact = parse_with_config(source, &config()).expect("should parse");
        let section = artifact.sections()[0]
            .as_findings()
            .expect("findings section should be structured");
        assert_eq!(section.body().as_sentinel(), Some("No findings."));
        assert_eq!(section.body().span().start_line(), 3);
    }

    #[test]
    fn reports_an_extra_title_after_a_findings_sentinel_and_blank_line() {
        let source = concat!(
            "# Review\n",
            "## Findings\n",
            "No findings.\n",
            "\n",
            "# Extra title\n",
        );

        let diagnostics = parse_with_config(source, &config())
            .expect_err("the artifact should reject an extra title");

        assert!(diagnostics
            .iter()
            .any(|diagnostic| diagnostic.message().contains("exactly one")));
    }

    #[test]
    fn parses_expanded_findings_and_preserves_opaque_bodies() {
        let source = concat!(
            "# Review\n",
            "## Findings\n",
            "### F1: High - Example\n",
            "- Severity: opaque\n",
            "## Next\n",
            "text\n",
        );
        let artifact = parse_with_config(source, &config()).expect("should parse");
        let section = artifact.sections()[0]
            .as_findings()
            .expect("findings section should be structured");
        let items = section.body().items().expect("expanded findings");
        assert_eq!(items.len(), 1);
        assert_eq!(items[0].identifier().expect("ID").text(), "F1");
        assert_eq!(items[0].content().text(), "- Severity: opaque\n");
        assert_eq!(artifact.sections()[1].title(), "Next");
    }

    #[test]
    fn ignores_heading_like_finding_body_text_inside_fences() {
        let source = concat!(
            "# Review\n",
            "## Findings\n",
            "### F1: Example\n",
            "```markdown\n",
            "### Not another finding\n",
            "```\n",
            "## Next\n",
        );
        let artifact = parse_with_config(source, &config()).expect("should parse");
        let section = artifact.sections()[0]
            .as_findings()
            .expect("findings section should be structured");
        assert_eq!(section.body().items().expect("items").len(), 1);
        assert_eq!(artifact.sections()[1].title(), "Next");
    }
}
