use super::*;

#[test]
fn rejects_multiline_metadata_replacements() {
    let mut metadata: Metadata<'_, ()> = Metadata {
        key: Located::new("Status", ()),
        value: Located::new("proposed".into(), ()),
    };

    let error = metadata
        .set_value("accepted\n- Extra: value")
        .expect_err("multiline metadata should be rejected");

    assert_eq!(error, InvalidValue::Multiline);
    assert_eq!(error.to_string(), "metadata values must be single-line");
}

#[test]
fn rejects_edits_to_parsed_multiline_metadata_values() {
    let mut metadata: Metadata<'_, ()> = Metadata {
        key: Located::new("Status", ()),
        value: Located::new("proposed\n  continued".into(), ()),
    };

    let error = metadata
        .set_value("accepted")
        .expect_err("parsed multiline metadata should be immutable");

    assert_eq!(error, InvalidValue::Immutable);
    assert_eq!(error.to_string(), "metadata value is immutable");
    assert_eq!(metadata.value(), "proposed\n  continued");
}

#[test]
fn serializes_metadata_changes_without_reformatting_source() {
    let source = "# Example\r\n- Status: proposed\r\n";
    let config = crate::parser::ParserConfig::new(vec![]);
    let mut artifact =
        crate::parser::parse_with_config(source, &config).expect("artifact should parse");
    artifact.metadata_mut()[0]
        .value_mut()
        .set_value("accepted")
        .expect("value should be valid");

    assert_eq!(serialize(&artifact), "# Example\r\n- Status: accepted\r\n");
    assert_eq!(artifact.to_string(), "# Example\r\n- Status: accepted\r\n");
    assert_eq!(artifact.source(), "# Example\r\n- Status: proposed\r\n");
}

#[test]
fn exposes_empty_pre_section_prose_with_a_source_span() {
    let source = "# Example\n- Status: proposed\n## Details\n";
    let config = crate::parser::ParserConfig::new(vec![]);
    let artifact =
        crate::parser::parse_with_config(source, &config).expect("artifact should parse");
    let prose = artifact.located_pre_section_prose();

    assert_eq!(prose.text(), "");
    assert_eq!(prose.span().range(), 29..29);
    assert_eq!(prose.span().start_line(), 3);
}
