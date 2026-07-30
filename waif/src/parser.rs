use self::cursor::{Cursor, Position};
use crate::artifact::{
    Artifact, CompactItem, ExpandedItem, Item, ItemisedSection, Located, Metadata, ProseSection,
    Section, SourceSpan,
};
use std::fmt;
use std::ops::Range;

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

#[allow(dead_code)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SectionType {
    Prose,
    Itemised,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SectionConfig {
    name: String,
    section_type: SectionType,
}

#[allow(dead_code)]
impl SectionConfig {
    pub fn new(name: impl Into<String>, section_type: SectionType) -> Self {
        Self {
            name: name.into(),
            section_type,
        }
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ParserConfig {
    sections: Vec<SectionConfig>,
}

#[allow(dead_code)]
impl ParserConfig {
    pub fn new(sections: Vec<SectionConfig>) -> Self {
        Self { sections }
    }

    fn section_type(&self, name: &str) -> SectionType {
        self.sections
            .iter()
            .find(|section| section.name == name)
            .map_or(SectionType::Prose, |section| section.section_type)
    }
}

/// Parse the common task-artifact envelope.
///
/// With no configuration, every level-two section is opaque prose.
pub fn parse(source: &str) -> Result<Artifact, Vec<Diagnostic>> {
    parse_with_config(source, &ParserConfig::default())
}

pub fn parse_with_config(source: &str, config: &ParserConfig) -> Result<Artifact, Vec<Diagnostic>> {
    let diagnostics = Vec::new();
    let cursor = Cursor::new(source);
    let mut ctx = ParsingContext {
        source,
        cursor,
        diagnostics,
        code_block: None,
        config: config.clone(),
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
    config: ParserConfig,
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
        level: 1, title, ..
    }) = p_markdown_heading(ctx)
    else {
        ctx.diagnostics.push(Diagnostic::new(
            pos.line(),
            "first non-whitespace line must be a level-one Markdown heading",
        ));
        return String::new();
    };
    if title.text().is_empty() {
        ctx.diagnostics.push(Diagnostic::new(
            pos.line(),
            "level-one title must not be empty",
        ));
    }
    title.value().to_owned()
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
    if heading.title.text().is_empty() {
        ctx.diagnostics.push(Diagnostic::new(
            heading.pos.line(),
            "level-two section name must not be empty",
        ));
    }
    let body_start = ctx.cursor.position();
    let body_end = p_section_body_end(ctx);

    let heading_line = heading.pos.line();
    let section_span = SourceSpan::new(heading_line, heading.pos.offset()..body_end);
    let body_span = SourceSpan::new(body_start.line(), body_start.offset()..body_end);
    match ctx.config.section_type(heading.title.text()) {
        SectionType::Prose => Some(Section::Prose(Located::new(
            ProseSection::new(
                heading.title,
                Located::new(
                    ctx.source[body_start.offset()..body_end].to_owned(),
                    body_span,
                ),
            ),
            section_span,
        ))),
        SectionType::Itemised => {
            ctx.cursor.rewind(body_start);
            ctx.code_block = None;
            let items = p_itemised_items(ctx, body_end);
            Some(Section::Itemised(Located::new(
                ItemisedSection {
                    title: heading.title,
                    items: Located::new(items, body_span),
                },
                section_span,
            )))
        }
    }
}

fn p_section_body_end(ctx: &mut ParsingContext) -> usize {
    loop {
        let Some((_, line)) = ctx.cursor.peek_line() else {
            return ctx.source.len();
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
                    return start.offset();
                }
                Some(_) => continue,
                None => {}
            }
        }
        ctx.cursor.take_line();
    }
}

fn p_itemised_items(ctx: &mut ParsingContext, body_end: usize) -> Vec<Item> {
    let mut leading_content = Vec::new();

    while ctx.cursor.position().offset() < body_end {
        let start = ctx.cursor.position();
        let (_, line) = ctx.cursor.peek_line().expect("body has a line");
        let in_code_block = is_code_block_line(line, &mut ctx.code_block);

        if !in_code_block {
            if compact_marker(line).is_some() {
                report_leading_item_content(ctx, &leading_content, "compact", "- ID: content");
                return p_compact_items(ctx, body_end);
            }
            if heading_level(ctx) == Some(3) {
                report_leading_item_content(ctx, &leading_content, "expanded", "### ID: title");
                return p_expanded_items(ctx, body_end);
            }
        }

        if !line.trim().is_empty() {
            leading_content.push(start.line());
        }
        ctx.cursor.take_line();
    }
    for line in leading_content {
        ctx.diagnostics.push(Diagnostic::new(
            line,
            "expected an item in `- ID: content` or `### ID: title` form",
        ));
    }
    Vec::new()
}

fn report_leading_item_content(
    ctx: &mut ParsingContext,
    lines: &[usize],
    form: &str,
    syntax: &str,
) {
    for line in lines {
        ctx.diagnostics.push(Diagnostic::new(
            *line,
            format!("expected a {form} item in `{syntax}` form"),
        ));
    }
}

fn heading_level(ctx: &mut ParsingContext) -> Option<usize> {
    let start = ctx.cursor.position();
    let level = p_markdown_heading(ctx).map(|heading| heading.level);
    ctx.cursor.rewind(start);
    level
}

// A compact item whose body end is not known until the next peer or section.
struct OpenCompactItem {
    start: Position,
    body_start: Position,
    marker: Located<String>,
    identifier: Option<Located<String>>,
    delimiter: Option<Located<String>>,
    content: Located<String>,
}

// Parse compact peer items up to the known section-body boundary.
fn p_compact_items(ctx: &mut ParsingContext, body_end: usize) -> Vec<Item> {
    let mut items = Vec::new();
    let mut peer_indentation = None;
    let mut open = None;

    while ctx.cursor.position().offset() < body_end {
        let start = ctx.cursor.position();
        let (_, line) = ctx.cursor.peek_line().expect("body has a line");
        let in_code_block = is_code_block_line(line, &mut ctx.code_block);
        let marker = (!in_code_block).then(|| compact_marker(line)).flatten();

        if !in_code_block && heading_level(ctx) == Some(3) {
            ctx.diagnostics.push(Diagnostic::new(
                start.line(),
                "itemised section cannot mix compact and expanded items",
            ));
            ctx.cursor.take_line();
            continue;
        }

        if let Some((indentation, _)) = marker {
            let peer = *peer_indentation.get_or_insert(indentation);
            if indentation == peer {
                if let Some(item) = open.take() {
                    items.push(finish_compact_item(ctx.source, item, start));
                }
                open = Some(p_compact_item_opening(ctx));
                continue;
            }
        }

        // Before the first item, every nonblank line is outside an item. Once
        // an item is open, only unfenced content at or before peer indentation
        // falls outside its body. An open item always has a peer indentation.
        let outside_item = open.is_none()
            || (!in_code_block && leading_indentation_columns(line) <= peer_indentation.unwrap());
        if !line.trim().is_empty() && outside_item {
            ctx.diagnostics.push(Diagnostic::new(
                start.line(),
                "expected a compact item in `- ID: content` form",
            ));
        }
        ctx.cursor.take_line();
    }

    if let Some(item) = open {
        items.push(finish_compact_item(ctx.source, item, ctx.cursor.position()));
    }
    items
}

// An expanded item whose body end is not known until the next peer or section.
struct OpenExpandedItem {
    start: Position,
    body_start: Position,
    marker: Located<String>,
    identifier: Option<Located<String>>,
    delimiter: Option<Located<String>>,
    content: Located<String>,
}

fn p_expanded_items(ctx: &mut ParsingContext, body_end: usize) -> Vec<Item> {
    let mut items = Vec::new();
    let mut open = None;

    while ctx.cursor.position().offset() < body_end {
        let start = ctx.cursor.position();
        let (_, line) = ctx.cursor.peek_line().expect("body has a line");
        let in_code_block = is_code_block_line(line, &mut ctx.code_block);

        if !in_code_block && heading_level(ctx) == Some(3) {
            if let Some(item) = open.take() {
                items.push(finish_expanded_item(ctx.source, item, start));
            }
            open = Some(p_expanded_item_opening(ctx));
            continue;
        }

        if open.is_none() && !line.trim().is_empty() {
            ctx.diagnostics.push(Diagnostic::new(
                start.line(),
                "expected an expanded item in `### ID: title` form",
            ));
        }
        ctx.cursor.take_line();
    }

    if let Some(item) = open {
        items.push(finish_expanded_item(
            ctx.source,
            item,
            ctx.cursor.position(),
        ));
    }
    items
}

fn p_expanded_item_opening(ctx: &mut ParsingContext) -> OpenExpandedItem {
    let start = ctx.cursor.position();
    let heading = p_markdown_heading(ctx).expect("item has a heading");
    debug_assert_eq!(heading.level, 3);
    let title_range = heading.title.span().range();
    let title = heading.title.text();
    let marker = located_source_range(ctx.source, start.line(), start.offset()..title_range.start);
    let ranges = item_opening_ranges(title).offset(title_range.start);
    let identifier = ranges
        .identifier
        .map(|range| located_source_range(ctx.source, start.line(), range));
    let delimiter = ranges
        .delimiter
        .map(|range| located_source_range(ctx.source, start.line(), range));
    let content = located_source_range(ctx.source, start.line(), ranges.content);

    OpenExpandedItem {
        start,
        body_start: ctx.cursor.position(),
        marker,
        identifier,
        delimiter,
        content,
    }
}

fn finish_expanded_item(source: &str, open: OpenExpandedItem, end: Position) -> Item {
    Item::Expanded(Located::new(
        ExpandedItem {
            marker: open.marker,
            identifier: open.identifier,
            delimiter: open.delimiter,
            content: open.content,
            body: located_source_range(
                source,
                open.body_start.line(),
                open.body_start.offset()..end.offset(),
            ),
        },
        SourceSpan::new(open.start.line(), open.start.offset()..end.offset()),
    ))
}

// Return a compact marker's indentation and end offset.
// For `"  - item"`, return `(2, 4)`.
fn compact_marker(line: &str) -> Option<(usize, usize)> {
    let indentation = leading_spaces(line);
    if indentation > 3 {
        return None;
    }
    let rest = &line[indentation..];
    let after_marker = rest.strip_prefix('-')?;
    let whitespace = after_marker
        .bytes()
        .take_while(|byte| *byte == b' ' || *byte == b'\t')
        .count();
    (whitespace > 0).then_some((indentation, indentation + 1 + whitespace))
}

fn leading_spaces(line: &str) -> usize {
    line.bytes().take_while(|byte| *byte == b' ').count()
}

// Measure leading spaces and tabs using four-column tab stops.
fn leading_indentation_columns(line: &str) -> usize {
    line.bytes()
        .take_while(|byte| *byte == b' ' || *byte == b'\t')
        .fold(0, |column, byte| {
            if byte == b'\t' {
                column + 4 - column % 4
            } else {
                column + 1
            }
        })
}

// Parse one compact opening line while leaving its body open.
fn p_compact_item_opening(ctx: &mut ParsingContext) -> OpenCompactItem {
    let start = ctx.cursor.position();
    // The caller has just peeked this line and matched `compact_marker`
    // without advancing the cursor, so both operations must succeed.
    let (_, line) = ctx.cursor.take_line().expect("item has an opening line");
    let (_, marker_end) = compact_marker(line).expect("item has a marker");
    let remainder = &line[marker_end..];
    let marker = located_line_range(line, start, 0..marker_end);

    // For `G-AC1 :  value`, retain `G-AC1` as the identifier, ` :  ` as the
    // delimiter, and begin content at `value`.
    let ranges = item_opening_ranges(remainder).offset(marker_end);
    let identifier = ranges
        .identifier
        .map(|range| located_line_range(line, start, range));
    let delimiter = ranges
        .delimiter
        .map(|range| located_line_range(line, start, range));

    OpenCompactItem {
        start,
        body_start: ctx.cursor.position(),
        marker,
        identifier,
        delimiter,
        content: located_line_range(line, start, ranges.content),
    }
}

struct ItemOpeningRanges {
    identifier: Option<Range<usize>>,
    delimiter: Option<Range<usize>>,
    content: Range<usize>,
}

impl ItemOpeningRanges {
    fn offset(self, offset: usize) -> Self {
        Self {
            identifier: self.identifier.map(|range| offset_range(range, offset)),
            delimiter: self.delimiter.map(|range| offset_range(range, offset)),
            content: offset_range(self.content, offset),
        }
    }
}

fn item_opening_ranges(text: &str) -> ItemOpeningRanges {
    let Some(colon) = text.find(':') else {
        return ItemOpeningRanges {
            identifier: None,
            delimiter: None,
            content: 0..text.len(),
        };
    };

    let candidate = &text[..colon];
    let identifier = candidate.trim();
    let identifier_start = candidate.len() - candidate.trim_start().len();
    let identifier_end = identifier_start + identifier.len();
    let mut content_start = colon + 1;
    while text[content_start..].starts_with([' ', '\t']) {
        content_start += 1;
    }
    ItemOpeningRanges {
        identifier: Some(identifier_start..identifier_end),
        delimiter: Some(identifier_end..content_start),
        content: content_start..text.len(),
    }
}

fn offset_range(range: Range<usize>, offset: usize) -> Range<usize> {
    range.start + offset..range.end + offset
}

// Copy a line range while translating it to an artifact source span.
fn located_line_range(line: &str, line_start: Position, range: Range<usize>) -> Located<String> {
    Located::new(
        line[range.clone()].to_owned(),
        SourceSpan::new(
            line_start.line(),
            line_start.offset() + range.start..line_start.offset() + range.end,
        ),
    )
}

fn located_source_range(source: &str, start_line: usize, range: Range<usize>) -> Located<String> {
    Located::new(
        source[range.clone()].to_owned(),
        SourceSpan::new(start_line, range),
    )
}

// Close an open compact item at its next peer or section boundary.
fn finish_compact_item(source: &str, open: OpenCompactItem, end: Position) -> Item {
    Item::Compact(Located::new(
        CompactItem {
            marker: open.marker,
            identifier: open.identifier,
            delimiter: open.delimiter,
            content: open.content,
            body: Located::new(
                source[open.body_start.offset()..end.offset()].to_owned(),
                SourceSpan::new(
                    open.body_start.line(),
                    open.body_start.offset()..end.offset(),
                ),
            ),
        },
        SourceSpan::new(open.start.line(), open.start.offset()..end.offset()),
    ))
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
    title: Located<String>,
    pos: Position,
}

fn p_markdown_heading(ctx: &mut ParsingContext) -> Option<MarkdownHeading> {
    ctx.cursor
        .try_(|cursor| {
            let pos = cursor.position();
            let (_, line) = cursor.take_line().ok_or(())?;
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
                    title: Located::new(
                        String::new(),
                        SourceSpan::new(
                            pos.line(),
                            pos.offset() + indentation + level..pos.offset() + indentation + level,
                        ),
                    ),
                    pos,
                });
            }
            if !rest.starts_with([' ', '\t']) {
                return Err(());
            }

            let trimmed = rest.trim();
            let without_hashes = trimmed.trim_end_matches('#');
            let title = if without_hashes.is_empty() {
                without_hashes
            } else if without_hashes.ends_with(char::is_whitespace) {
                without_hashes.trim_end()
            } else {
                trimmed
            };
            let title_start =
                pos.offset() + indentation + level + rest.len() - rest.trim_start().len();
            Ok(MarkdownHeading {
                level,
                title: Located::new(
                    title.to_owned(),
                    SourceSpan::new(pos.line(), title_start..title_start + title.len()),
                ),
                pos,
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

    fn itemised(section: &Section) -> &ItemisedSection {
        section.as_itemised().expect("section should contain items")
    }

    fn compact(item: &Item) -> &CompactItem {
        item.as_compact().expect("item should use compact form")
    }

    fn expanded(item: &Item) -> &ExpandedItem {
        item.as_expanded().expect("item should use expanded form")
    }

    fn itemised_config(name: &str) -> ParserConfig {
        ParserConfig::new(vec![SectionConfig::new(name, SectionType::Itemised)])
    }

    mod component_parsers {
        use super::*;

        fn context(source: &str) -> ParsingContext<'_> {
            ParsingContext {
                source,
                cursor: Cursor::new(source),
                diagnostics: vec![],
                code_block: None,
                config: ParserConfig::default(),
            }
        }

        #[test]
        fn markdown_heading_consumes_success_and_rewinds_failure() {
            let mut heading = context("# Title\nbody");
            let parsed = p_markdown_heading(&mut heading).expect("heading should parse");
            assert_eq!(parsed.title.text(), "Title");
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
            assert_eq!(artifact.sections()[0].title().text(), "Details");
            assert_eq!(
                &source[artifact.sections()[0].title().span().range()],
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
            assert_eq!(artifact.serialize(), source);
        }

        #[test]
        fn accepts_empty_metadata() {
            let artifact =
                parse("# Example\n## Details\n").expect("empty metadata should be valid");

            assert!(artifact.metadata().is_empty());
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
                SectionConfig::new("Items", SectionType::Itemised),
                SectionConfig::new("Other", SectionType::Prose),
            ]);
            let artifact =
                parse_with_config(source, &config).expect("configured parsing should succeed");

            assert_eq!(itemised(&artifact.sections()[0]).items().len(), 1);
            assert!(artifact.sections()[1].as_prose().is_some());
        }

        #[test]
        fn preserves_compact_components_and_malformed_candidates() {
            let source = concat!(
                "# Example\n",
                "## Items\n",
                "  - G-AC1 :   α  \n",
                "             continuation\n",
                "             - nested\n",
                "  - [ ] Bad checkbox: retained\n",
                "  - missing identifier\n",
                "  - : empty identifier\n",
                "  - G-AC5:\n",
                "## Tail\n",
                "opaque\n",
            );
            let artifact = parse_with_config(source, &itemised_config("Items"))
                .expect("compact items should parse");
            let section = itemised(&artifact.sections()[0]);
            let items = section.items();

            assert_eq!(items.len(), 5);
            assert_eq!(
                section.located_items().span(),
                artifact.sections()[0].body_span()
            );
            let first_item = &items[0];
            let first = compact(first_item);
            assert_eq!(first.marker().text(), "  - ");
            assert_eq!(first_item.identifier().unwrap().text(), "G-AC1");
            assert_eq!(first.delimiter().unwrap().text(), " :   ");
            assert_eq!(first_item.content().text(), "α  ");
            assert!(first.body().text().contains("- nested"));
            assert_eq!(
                &source[first.marker().span().range()],
                first.marker().text()
            );
            assert_eq!(
                &source[first_item.identifier().unwrap().span().range()],
                "G-AC1"
            );
            assert_eq!(&source[first.delimiter().unwrap().span().range()], " :   ");
            assert_eq!(
                &source[first_item.content().span().range()],
                first_item.content().text()
            );
            assert_eq!(&source[first.body().span().range()], first.body().text());
            assert_eq!(
                &source[first_item.span().range()],
                concat!(
                    "  - G-AC1 :   α  \n",
                    "             continuation\n",
                    "             - nested\n",
                )
            );

            assert_eq!(items[1].identifier().unwrap().text(), "[ ] Bad checkbox");
            assert!(items[2].identifier().is_none());
            assert!(compact(&items[2]).delimiter().is_none());
            assert_eq!(items[2].content().text(), "missing identifier");
            assert_eq!(items[3].identifier().unwrap().text(), "");
            assert_eq!(items[4].identifier().unwrap().text(), "G-AC5");
            assert_eq!(items[4].content().text(), "");
            assert_eq!(artifact.sections()[1].name(), "Tail");
        }

        #[test]
        fn preserves_expanded_components_and_malformed_candidates() {
            let source = concat!(
                "# Example\n",
                "## Items\n",
                " ### G-REV1 :  α ###\n",
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
            assert_eq!(first_item.form(), crate::artifact::ItemForm::Expanded);
            assert_eq!(first.marker().text(), " ### ");
            assert_eq!(first_item.identifier().unwrap().text(), "G-REV1");
            assert_eq!(first.delimiter().unwrap().text(), " :  ");
            assert_eq!(first_item.content().text(), "α");
            assert!(first_item.body().text().contains("opaque body"));
            assert!(first.body().text().contains("#### A deeper heading"));
            assert!(first.body().text().contains("- Before:"));
            assert!(first.body().text().contains("- G-AC1:"));
            assert!(first.body().text().contains("- malformed compact-looking"));
            assert!(first.body().text().contains("- nested list"));
            assert_eq!(
                &source[first.marker().span().range()],
                first.marker().text()
            );
            assert_eq!(
                &source[first_item.identifier().unwrap().span().range()],
                "G-REV1"
            );
            assert_eq!(&source[first.delimiter().unwrap().span().range()], " :  ");
            assert_eq!(
                &source[first_item.content().span().range()],
                first_item.content().text()
            );
            assert_eq!(&source[first.body().span().range()], first.body().text());
            assert_eq!(
                &source[first_item.span().range()],
                concat!(
                    " ### G-REV1 :  α ###\n",
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
            assert_eq!(items[1].content().text(), "missing delimiter");
            assert_eq!(items[2].identifier().unwrap().text(), "");
            assert_eq!(items[3].identifier().unwrap().text(), "G-REV4");
            assert_eq!(items[3].content().text(), "");
            assert_eq!(artifact.sections()[1].name(), "Tail");
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
            assert_eq!(compact_item.content().text(), "compact");
            assert_eq!(compact_item.body().text(), "  body\n");
            assert_eq!(expanded_item.form(), crate::artifact::ItemForm::Expanded);
            assert_eq!(expanded_item.identifier().unwrap().text(), "G-AC1");
            assert_eq!(expanded_item.content().text(), "expanded");
            assert_eq!(expanded_item.body().text(), "body\n");
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
            assert!(items[0].body().text().contains("### not a peer"));
            assert!(items[0].body().text().contains("- not a mixed form"));
            assert!(items[0].body().text().contains("#### deeper heading"));
            assert_eq!(items[1].identifier().unwrap().text(), "G-REV2");
        }

        #[test]
        fn rejects_bare_leading_content_and_compact_then_expanded_forms() {
            for source in [
                "# Example\n## Items\nbare\n### G-REV1: item\n",
                "# Example\n## Items\n- G-AC1: compact\n### G-REV1: expanded\n",
            ] {
                let diagnostics = parse_with_config(source, &itemised_config("Items"))
                    .expect_err("invalid itemised section should fail");
                assert!(!diagnostics.is_empty());
            }
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
                assert_eq!(item.content().text(), "値");
                assert_eq!(
                    &source[item.span().range()],
                    &source[source.find("### G-α").unwrap()..]
                );
                assert_eq!(artifact.serialize(), source);
            }
        }

        #[test]
        fn keeps_fences_and_nested_markers_in_the_current_item() {
            let source = concat!(
                "# Example\n",
                "## Items\n",
                "- G-AC1: first\n",
                "````markdown\n",
                "- not a peer\n",
                "## not a section\n",
                "````\n",
                "  - nested\n",
                "\ttab-indented prose\n",
                "\t- tab-indented nested item\n",
                "- G-AC2: second\n",
            );
            let artifact = parse_with_config(source, &itemised_config("Items"))
                .expect("fenced content should parse");
            let items = itemised(&artifact.sections()[0]).items();

            assert_eq!(items.len(), 2);
            assert!(compact(&items[0]).body().text().contains("- not a peer"));
            assert!(compact(&items[0]).body().text().contains("  - nested"));
            assert!(compact(&items[0])
                .body()
                .text()
                .contains("\ttab-indented prose"));
            assert!(compact(&items[0])
                .body()
                .text()
                .contains("\t- tab-indented nested item"));
            assert_eq!(items[1].identifier().unwrap().text(), "G-AC2");
        }

        #[test]
        fn tracks_components_under_every_supported_line_ending() {
            for ending in ["\n", "\r\n", "\r"] {
                let source = ["# Example", "## Items", "- G-α: 値", ""].join(ending);
                let artifact = parse_with_config(&source, &itemised_config("Items"))
                    .expect("item should parse");
                let item = &itemised(&artifact.sections()[0]).items()[0];

                assert_eq!(item.span().start_line(), 3);
                assert_eq!(item.identifier().unwrap().text(), "G-α");
                assert_eq!(&source[item.identifier().unwrap().span().range()], "G-α");
                assert_eq!(
                    &source[item.span().range()],
                    &source[source.find("- G-α").unwrap()..]
                );
                assert_eq!(artifact.serialize(), source);
            }
        }

        #[test]
        fn accepts_zero_through_three_spaces_of_item_indentation() {
            for marker in ["- ", " - ", "  - ", "   - "] {
                let source = format!("# Example\n## Items\n{marker}G-AC1: value\n");
                let artifact = parse_with_config(&source, &itemised_config("Items"))
                    .expect("item should parse");
                let item = &itemised(&artifact.sections()[0]).items()[0];

                assert_eq!(compact(item).marker().text(), marker);
                assert_eq!(item.identifier().unwrap().text(), "G-AC1");
            }
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
            assert_eq!(
                diagnostics
                    .iter()
                    .filter(|diagnostic| diagnostic.message().contains("expected a compact item"))
                    .count(),
                2
            );

            let empty = "# Example\n## Items\n\n";
            let artifact = parse_with_config(empty, &itemised_config("Items"))
                .expect("empty itemised section should parse");
            assert!(itemised(&artifact.sections()[0]).items().is_empty());
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
                .set_value("accepted")
                .expect("value should be valid");

            assert_eq!(
                artifact.serialize(),
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
                .set_value("accepted")
                .expect("value should be valid");

            assert_eq!(
                artifact.serialize(),
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
                let first_title = source.find('α').unwrap();
                let first_body = source.find("first").unwrap();
                let second_start = source.find("## Two").unwrap();
                let second_title = source.find("Two").unwrap();
                let second_body = source.find('二').unwrap();

                assert_eq!(first.title().span().start_line(), 2);
                assert_eq!(
                    first.title().span().range(),
                    first_title..first_title + "α".len()
                );
                assert_eq!(first.body_span().start_line(), 3);
                assert_eq!(first.body_span().range(), first_body..second_start);
                assert_eq!(first.span().range(), first_start..second_start);

                assert_eq!(second.title().span().start_line(), 4);
                assert_eq!(
                    second.title().span().range(),
                    second_title..second_title + "Two".len()
                );
                assert_eq!(second.body_span().start_line(), 5);
                assert_eq!(second.body_span().range(), second_body..source.len());
                assert_eq!(second.span().range(), second_start..source.len());
                assert_eq!(&source[first.title().span().range()], "α");
                assert_eq!(&source[second.title().span().range()], "Two");
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
