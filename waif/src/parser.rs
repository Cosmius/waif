use self::cursor::{Cursor, Position};
use crate::artifact::{Artifact, Metadata, ProseSection, Section, SourceSpan};
use std::fmt;

mod cursor;

// ============================================================================
// Diagnostics
// ============================================================================

#[derive(Debug, PartialEq, Eq)]
pub struct Diagnostic {
    line: usize,
    message: String,
}

impl Diagnostic {
    fn new(line: usize, message: impl Into<String>) -> Self {
        Self {
            line,
            message: message.into(),
        }
    }

    pub fn line(&self) -> usize {
        self.line
    }

    pub fn message(&self) -> &str {
        &self.message
    }
}

impl fmt::Display for Diagnostic {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "line {}: {}", self.line, self.message)
    }
}

// ============================================================================
// Parser entry point and state
// ============================================================================

/// Parse the common task-artifact envelope.
///
/// All level-two sections are currently parsed as opaque prose. The parser
/// deliberately does not apply artifact-specific contracts yet.
pub fn parse(source: &str) -> Result<Artifact, Vec<Diagnostic>> {
    let diagnostics = Vec::new();
    let cursor = Cursor::new(source);
    let mut ctx = ParsingContext {
        source,
        cursor,
        diagnostics,
        code_block: None,
    };
    let artifact = p_artifact(&mut ctx);
    if ctx.diagnostics.is_empty() {
        Ok(artifact)
    } else {
        Err(ctx.diagnostics)
    }
}

struct ParsingContext<'a> {
    source: &'a str,
    cursor: Cursor<'a>,
    diagnostics: Vec<Diagnostic>,
    /// The open code fence as `(marker, opening length)`, or `None` outside
    /// a fenced code block.
    code_block: Option<(char, usize)>,
}

// ============================================================================
// Artifact envelope
// ============================================================================

fn p_artifact(ctx: &mut ParsingContext) -> Artifact {
    ctx.cursor.skip_whitespace_lines();
    let title = p_title_line(ctx);
    let metadata = p_metadata(ctx);
    let sections = p_sections(ctx);
    Artifact::new(ctx.source.to_owned(), title, metadata, sections)
}

fn p_title_line(ctx: &mut ParsingContext) -> String {
    let pos = ctx.cursor.position();
    if ctx.cursor.is_eof() {
        ctx.diagnostics
            .push(Diagnostic::new(pos.line(), "missing level-one title"));
        return String::new();
    }

    let Some(MarkdownHeading {
        level: 1,
        name: title,
        ..
    }) = p_markdown_heading(ctx)
    else {
        ctx.diagnostics.push(Diagnostic::new(
            pos.line(),
            "first non-whitespace line must be a level-one Markdown heading",
        ));
        return String::new();
    };
    if title.is_empty() {
        ctx.diagnostics.push(Diagnostic::new(
            pos.line(),
            "level-one title must not be empty",
        ));
    }
    title
}

fn p_metadata(ctx: &mut ParsingContext) -> Vec<Metadata> {
    let mut metadata = Vec::new();
    while {
        ctx.cursor.skip_whitespace_lines();
        !ctx.cursor.is_eof()
    } {
        let start = ctx.cursor.position();
        if let Some(entry) = p_metadata_line(ctx) {
            metadata.push(entry);
            continue;
        }

        if let Some(heading) = p_markdown_heading(ctx) {
            if heading.level == 2 {
                ctx.cursor.rewind(start);
                return metadata;
            }
            if heading.level == 1 {
                report_additional_title(ctx, start);
                continue;
            }
            ctx.diagnostics.push(Diagnostic::new(
                start.line(),
                "expected metadata in `- Key: value` form or a \
                 level-two section",
            ));
            continue;
        }

        ctx.diagnostics.push(Diagnostic::new(
            start.line(),
            "expected metadata in `- Key: value` form or a \
             level-two section",
        ));
        ctx.cursor.take_line();
    }
    metadata
}

fn p_sections(ctx: &mut ParsingContext) -> Vec<Section> {
    let mut sections = Vec::new();
    while let Some(section) = p_section(ctx) {
        sections.push(section);
    }
    sections
}

fn p_section(ctx: &mut ParsingContext) -> Option<Section> {
    let pos = ctx.cursor.position();
    let heading = p_markdown_heading(ctx)?;
    if heading.level != 2 {
        ctx.cursor.rewind(pos);
        return None;
    }
    if heading.name.is_empty() {
        ctx.diagnostics.push(Diagnostic::new(
            heading.pos.line(),
            "level-two section name must not be empty",
        ));
    }
    let body_start = ctx.cursor.position();
    let body_end = loop {
        let Some((_, line)) = ctx.cursor.peek_line() else {
            break ctx.source.len();
        };
        let start = ctx.cursor.position();
        if !is_code_block_line(line, &mut ctx.code_block) {
            match p_markdown_heading(ctx) {
                Some(next) if next.level == 1 => {
                    report_additional_title(ctx, start);
                    continue;
                }
                Some(next) if next.level == 2 => {
                    ctx.cursor.rewind(start);
                    break start.offset();
                }
                Some(_) => continue,
                None => {}
            }
        }
        ctx.cursor.take_line();
    };

    Some(Section::Prose(ProseSection::new(
        heading.name,
        ctx.source[body_start.offset()..body_end].to_owned(),
        heading.span,
        SourceSpan::new(body_start.line(), body_start.offset()..body_end),
        SourceSpan::new(heading.pos.line(), heading.pos.offset()..body_end),
    )))
}

fn report_additional_title(ctx: &mut ParsingContext, position: Position) {
    ctx.diagnostics.push(Diagnostic::new(
        position.line(),
        "artifact must contain exactly one level-one heading",
    ));
}

// ============================================================================
// Markdown headings
// ============================================================================

struct MarkdownHeading {
    level: usize,
    name: String,
    pos: Position,
    span: SourceSpan,
}

fn p_markdown_heading(ctx: &mut ParsingContext) -> Option<MarkdownHeading> {
    ctx.cursor
        .try_(|cursor| {
            let pos = cursor.position();
            let (_, line) = cursor.take_line().ok_or(())?;
            let end = cursor.position().offset();
            let candidate = line.trim_start_matches(' ');
            let indentation = line.len() - candidate.len();
            if indentation > 3 {
                return Err(());
            }

            let level = candidate.bytes().take_while(|byte| *byte == b'#').count();
            if !(1..=6).contains(&level) {
                return Err(());
            }
            let rest = &candidate[level..];
            if rest.is_empty() {
                return Ok(MarkdownHeading {
                    level,
                    name: String::new(),
                    pos,
                    span: SourceSpan::new(pos.line(), pos.offset()..end),
                });
            }
            if !rest.starts_with([' ', '\t']) {
                return Err(());
            }

            let name = rest.trim();
            let without_hashes = name.trim_end_matches('#');
            let name = if without_hashes.is_empty() {
                without_hashes
            } else if without_hashes.ends_with(char::is_whitespace) {
                without_hashes.trim_end()
            } else {
                name
            };
            Ok(MarkdownHeading {
                level,
                name: name.to_owned(),
                pos,
                span: SourceSpan::new(pos.line(), pos.offset()..end),
            })
        })
        .ok()
}

// ============================================================================
// Fenced code blocks
// ============================================================================

/// Track fenced code blocks and report whether `line` belongs to one.
///
/// The opening fence, every line inside it, and the closing fence return
/// `true`. Lines outside the fence return `false`:
///
/// For these four input lines, it returns `true`, `true`, `true`, and `false`,
/// respectively:
///
/// ```text
/// ~~~markdown
/// ## Example
/// ~~~
/// ## Real
/// ```
fn is_code_block_line(line: &str, code_block: &mut Option<(char, usize)>) -> bool {
    let candidate = line.trim_start_matches(' ');
    if line.len() - candidate.len() > 3 {
        return code_block.is_some();
    }
    let Some(marker) = candidate.chars().next() else {
        return code_block.is_some();
    };
    if marker != '`' && marker != '~' {
        return code_block.is_some();
    }
    let count = candidate
        .chars()
        .take_while(|character| *character == marker)
        .count();
    if count < 3 {
        return code_block.is_some();
    }

    match code_block {
        None => {
            let info = &candidate[count..];
            if marker == '`' && info.contains('`') {
                return false;
            }
            *code_block = Some((marker, count));
        }
        Some((open_marker, open_count))
            if *open_marker == marker
                && count >= *open_count
                && candidate[count..].trim().is_empty() =>
        {
            *code_block = None;
        }
        Some(_) => {}
    }
    true
}

// ============================================================================
// Metadata lines
// ============================================================================

fn p_metadata_line(ctx: &mut ParsingContext) -> Option<Metadata> {
    ctx.cursor
        .try_(|cursor| {
            cursor.skip_whitespaces_inline();
            cursor.take_if(|ch| ch == '-').ok_or(())?;
            if cursor.take_while(|ch| ch == ' ' || ch == '\t').is_empty() {
                return Err(());
            };
            let key = cursor.take_while(|ch| ch != '\n' && ch != ':').trim();
            if key.is_empty() {
                return Err(());
            }
            cursor.take_if(|ch| ch == ':').ok_or(())?;
            cursor.take_while(|ch| ch == ' ' || ch == '\t');
            let value_start = cursor.position().offset();
            let value = cursor.take_line().map_or("", |(_, value)| value.trim_end());
            Ok(Metadata::new(
                key.to_owned(),
                value.to_owned(),
                value_start..value_start + value.len(),
            ))
        })
        .ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn prose(section: &Section) -> &ProseSection {
        section
            .as_prose()
            .expect("section should contain opaque prose")
    }

    mod component_parsers {
        use super::*;

        fn context(source: &str) -> ParsingContext<'_> {
            ParsingContext {
                source,
                cursor: Cursor::new(source),
                diagnostics: vec![],
                code_block: None,
            }
        }

        #[test]
        fn markdown_heading_consumes_success_and_rewinds_failure() {
            let mut heading = context("# Title\nbody");
            let parsed = p_markdown_heading(&mut heading).expect("heading should parse");
            assert_eq!(parsed.name, "Title");
            assert_eq!(heading.cursor.position().line(), 2);

            let mut not_heading = context("plain text\n");
            let start = not_heading.cursor.position();
            assert!(p_markdown_heading(&mut not_heading).is_none());
            assert_eq!(not_heading.cursor.position(), start);
        }

        #[test]
        fn metadata_line_consumes_success_and_rewinds_failures() {
            let mut metadata = context("- Status: proposed\nbody");
            let parsed = p_metadata_line(&mut metadata).expect("metadata should parse");
            assert_eq!(parsed.value(), "proposed");
            assert_eq!(metadata.cursor.position().line(), 2);

            let mut not_metadata = context("plain text\n");
            let start = not_metadata.cursor.position();
            assert!(p_metadata_line(&mut not_metadata).is_none());
            assert_eq!(not_metadata.cursor.position(), start);

            let mut invalid_metadata = context("- invalid metadata\nnext");
            let start = invalid_metadata.cursor.position();
            assert!(p_metadata_line(&mut invalid_metadata).is_none());
            assert_eq!(invalid_metadata.cursor.position(), start);
            assert!(invalid_metadata.diagnostics.is_empty());
        }

        #[test]
        fn metadata_rewinds_before_the_first_section() {
            let mut ctx = context("- Status: proposed\n## Details\nbody");

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
            assert_eq!(artifact.sections()[0].name(), "One");
            assert_eq!(prose(&artifact.sections()[0]).body(), "one\n");
            assert_eq!(artifact.sections()[1].name(), "Two");
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
            let artifact = parse(artifact_source).expect("artifact should parse");

            assert_eq!(artifact.title(), "Example");
            assert_eq!(artifact.source(), artifact_source);
            assert_eq!(artifact.metadata()[1].key(), "Purpose");
            assert_eq!(
                artifact.metadata()[1].value(),
                "A value: with another colon"
            );
            assert_eq!(artifact.sections()[0].name(), "Overview");
            assert!(prose(&artifact.sections()[0])
                .body()
                .contains("### A subsection"));
        }

        #[test]
        fn uses_markdown_heading_rules_and_source_ranges() {
            let source = concat!(
                "  # Example ###\n",
                "- Status: proposed\n",
                "\n",
                "## Details ##\n",
                "\n",
                "> ## A quoted heading is prose\n",
            );

            let artifact = parse(source).expect("artifact should parse");

            assert_eq!(artifact.title(), "Example");
            assert_eq!(artifact.sections().len(), 1);
            assert_eq!(artifact.sections()[0].name(), "Details");
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
            assert_eq!(artifact.serialize(), source);
        }

        #[test]
        fn accepts_empty_metadata() {
            let artifact =
                parse("# Example\n## Details\n").expect("empty metadata should be valid");

            assert!(artifact.metadata().is_empty());
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
            assert_eq!(artifact.serialize(), source);
        }

        #[test]
        fn records_section_spans_for_utf8_and_supported_line_endings() {
            for line_ending in ["\n", "\r\n", "\r"] {
                let source = ["# Example", "## α", "first", "## Two", "二", ""].join(line_ending);
                let artifact = parse(&source).expect("artifact should parse");
                let first = &artifact.sections()[0];
                let second = &artifact.sections()[1];
                let first_start = source.find("## α").unwrap();
                let first_body = source.find("first").unwrap();
                let second_start = source.find("## Two").unwrap();
                let second_body = source.find('二').unwrap();

                assert_eq!(first.heading_span().start_line(), 2);
                assert_eq!(first.heading_span().range(), first_start..first_body);
                assert_eq!(first.body_span().start_line(), 3);
                assert_eq!(first.body_span().range(), first_body..second_start);
                assert_eq!(first.span().range(), first_start..second_start);

                assert_eq!(second.heading_span().start_line(), 4);
                assert_eq!(second.heading_span().range(), second_start..second_body);
                assert_eq!(second.body_span().start_line(), 5);
                assert_eq!(second.body_span().range(), second_body..source.len());
                assert_eq!(second.span().range(), second_start..source.len());
                assert_eq!(&source[first.body_span().range()], prose(first).body());
                assert_eq!(&source[second.body_span().range()], prose(second).body());
                assert_eq!(artifact.serialize(), source);
            }
        }

        #[test]
        fn preserves_standalone_carriage_return_line_endings() {
            let source = "# Example\r- Status: proposed\r## Details\rText\r";
            let mut artifact = parse(source).expect("artifact should parse");

            artifact.metadata_mut()[0]
                .set_value("accepted")
                .expect("value should be valid");

            assert_eq!(
                artifact.serialize(),
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
            let mut artifact = parse(source).expect("artifact should parse");
            let section_span = artifact.sections()[0].span().clone();

            artifact.metadata_mut()[0]
                .set_value("accepted")
                .expect("value should be valid");

            assert_eq!(
                artifact.serialize(),
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

        #[test]
        fn accepts_spacing_variations_in_metadata() {
            let source = concat!(
                "# Example\n",
                "-    First  :   one  \n",
                "     - Second : two\n",
                "- Third:three\n",
            );
            let mut artifact = parse(source).expect("artifact should parse");

            let entries: Vec<_> = artifact
                .metadata()
                .iter()
                .map(|entry| (entry.key(), entry.value()))
                .collect();
            assert_eq!(
                entries,
                vec![("First", "one"), ("Second", "two"), ("Third", "three")]
            );

            artifact.metadata_mut()[0]
                .set_value("updated")
                .expect("value should be valid");
            assert_eq!(
                artifact.serialize(),
                concat!(
                    "# Example\n",
                    "-    First  :   updated  \n",
                    "     - Second : two\n",
                    "- Third:three\n",
                )
            );
        }
    }

    mod invalid_artifacts {
        use super::*;

        #[test]
        fn rejects_setext_titles() {
            let diagnostics = parse("Example\n=======\n")
                .expect_err("the artifact format requires a hash heading");

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

            assert!(diagnostics.len() >= 5);
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
}
