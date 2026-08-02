use self::cursor::{Cursor, Position};
use crate::artifact::{
    Artifact, CompactItem, ExpandedItem, Finding, FindingsBody, FindingsSection, Item,
    ItemisedSection, Located, Metadata, PlanItem, PlanItemSection, ProseSection, Section,
    SourceSpan,
};
use std::fmt;
use std::ops::Range;

mod cursor;

// ============================================================================
// Diagnostics
// ============================================================================

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Severity {
    Error,
    Warning,
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct Diagnostic {
    severity: Severity,
    line: usize,
    message: String,
}

impl Diagnostic {
    pub(crate) fn error(line: usize, message: impl Into<String>) -> Self {
        Self::with_severity(Severity::Error, line, message)
    }

    pub(crate) fn warning(line: usize, message: impl Into<String>) -> Self {
        Self::with_severity(Severity::Warning, line, message)
    }

    fn with_severity(severity: Severity, line: usize, message: impl Into<String>) -> Self {
        Self {
            severity,
            line,
            message: message.into(),
        }
    }

    pub fn severity(&self) -> Severity {
        self.severity
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
// Parser entry point
// ============================================================================

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SectionType {
    Prose,
    Itemised,
    PlanItems,
    Findings,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SectionConfig<'a> {
    name: &'a str,
    section_type: SectionType,
}

impl<'a> SectionConfig<'a> {
    pub const fn prose(name: &'a str) -> Self {
        Self::with_type(name, SectionType::Prose)
    }

    pub const fn itemised(name: &'a str) -> Self {
        Self::with_type(name, SectionType::Itemised)
    }

    pub const fn plan_items(name: &'a str) -> Self {
        Self::with_type(name, SectionType::PlanItems)
    }

    pub const fn findings(name: &'a str) -> Self {
        Self::with_type(name, SectionType::Findings)
    }

    const fn with_type(name: &'a str, section_type: SectionType) -> Self {
        Self { name, section_type }
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ParserConfig<'a> {
    known_metadata: Vec<&'a str>,
    sections: Vec<SectionConfig<'a>>,
}

impl<'a> ParserConfig<'a> {
    pub fn new(sections: Vec<SectionConfig<'a>>) -> Self {
        Self {
            known_metadata: Vec::new(),
            sections,
        }
    }

    /// Declare the metadata keys recognized by this artifact contract.
    pub fn with_known_metadata<I>(mut self, names: I) -> Self
    where
        I: IntoIterator<Item = &'a str>,
    {
        self.known_metadata = names.into_iter().collect();
        self
    }

    fn recognizes_metadata(&self, name: &str) -> bool {
        self.known_metadata
            .iter()
            .any(|candidate| *candidate == name)
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
    let (artifact, diagnostics) = parse_with_diagnostics(source, config);
    if diagnostics.is_empty() {
        Ok(artifact)
    } else {
        Err(diagnostics)
    }
}

/// Parse all recoverable structure and return it with every parser diagnostic.
///
/// Schema checkers use this entry point when independent contract violations
/// remain meaningful despite a malformed structural fragment.
pub fn parse_with_diagnostics(source: &str, config: &ParserConfig) -> (Artifact, Vec<Diagnostic>) {
    let mut ctx = ParsingContext::new(source, config.clone());
    let artifact = p_artifact(&mut ctx);
    (artifact, ctx.diagnostics)
}

// ============================================================================
// Parser state
// ============================================================================

#[derive(Clone)]
struct ParsingContext<'a> {
    source: &'a str,
    cursor: Cursor<'a>,
    diagnostics: Vec<Diagnostic>,
    /// The open code fence as `(marker, opening length)`, or `None` outside
    /// a fenced code block.
    code_block: Option<(char, usize)>,
    config: ParserConfig<'a>,
}

impl<'a> ParsingContext<'a> {
    pub const fn new(source: &'a str, config: ParserConfig<'a>) -> Self {
        Self {
            source,
            cursor: Cursor::new(source),
            diagnostics: Vec::new(),
            code_block: None,
            config,
        }
    }

    pub fn try_<A, E>(&mut self, k: impl FnOnce(&mut Self) -> Result<A, E>) -> Result<A, E> {
        let ctx = self.clone();
        match k(self) {
            Ok(a) => Ok(a),
            Err(e) => {
                *self = ctx;
                Err(e)
            }
        }
    }

    pub(crate) fn located_with_pos(&self, position: Position, end: usize) -> Located<&'a str> {
        Located::new(
            &self.source[position.offset()..end],
            SourceSpan::new(position.line(), position.offset()..end),
        )
    }
}

// ============================================================================
// Artifact envelope
// ============================================================================

fn p_artifact(ctx: &mut ParsingContext) -> Artifact {
    ctx.cursor.skip_whitespace_lines();
    let title = p_title_line(ctx);
    let metadata = p_metadata(ctx);
    let pre_section_prose = p_pre_section_prose(ctx);
    let sections = p_sections(ctx);
    Artifact::new(
        ctx.source.to_owned(),
        title.text().to_owned(),
        metadata,
        pre_section_prose,
        sections,
    )
}

fn p_title_line<'a>(ctx: &mut ParsingContext<'a>) -> Located<&'a str> {
    let pos = ctx.cursor.position();
    if ctx.cursor.is_eof() {
        ctx.diagnostics
            .push(Diagnostic::error(pos.line(), "missing level-one title"));
        return ctx.located_with_pos(pos, pos.offset());
    }

    let Ok(MarkdownHeading {
        level: 1, title, ..
    }) = ctx.try_(p_markdown_heading)
    else {
        ctx.diagnostics.push(Diagnostic::error(
            pos.line(),
            "first non-whitespace line must be a level-one Markdown heading",
        ));
        return ctx.located_with_pos(pos, pos.offset());
    };
    if title.value().is_empty() {
        ctx.diagnostics.push(Diagnostic::error(
            pos.line(),
            "level-one title must not be empty",
        ));
    }
    title
}

fn p_metadata(ctx: &mut ParsingContext) -> Vec<Located<Metadata>> {
    let mut metadata = Vec::new();
    loop {
        let before_whitespace = ctx.cursor.position();
        ctx.cursor.skip_whitespace_lines();
        if ctx.cursor.is_eof() {
            ctx.cursor.rewind(before_whitespace);
            break;
        }
        if let Some(entry) = p_metadata_line(ctx) {
            if ctx.config.recognizes_metadata(entry.value().key()) {
                metadata.push(entry);
                continue;
            }
            ctx.cursor.rewind(before_whitespace);
            break;
        }

        if let Ok(heading) = ctx.try_(p_markdown_heading) {
            if heading.level == 2 {
                ctx.cursor.rewind(before_whitespace);
                break;
            }
        }
        ctx.cursor.rewind(before_whitespace);
        break;
    }
    metadata
}

fn p_pre_section_prose(ctx: &mut ParsingContext) -> Located<String> {
    let start = ctx.cursor.position();
    ctx.code_block = None;
    loop {
        let Some((_, line)) = ctx.cursor.peek_line() else {
            break;
        };
        let line_start = ctx.cursor.position();
        if !is_code_block_line(line, &mut ctx.code_block) {
            match ctx.try_(p_markdown_heading) {
                Ok(heading) if heading.level == 1 => {
                    report_additional_title(ctx, line_start);
                    continue;
                }
                Ok(heading) if heading.level == 2 => {
                    ctx.cursor.rewind(line_start);
                    break;
                }
                Ok(_) => continue,
                Err(_) => {}
            }
        }
        ctx.cursor.take_line();
    }
    let end = ctx.cursor.position();
    Located::new(
        ctx.source[start.offset()..end.offset()].to_owned(),
        SourceSpan::new(start.line(), start.offset()..end.offset()),
    )
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
    let heading = ctx.try_(p_markdown_heading).ok()?;
    if heading.level != 2 {
        ctx.cursor.rewind(pos);
        return None;
    }
    if heading.title.text().is_empty() {
        ctx.diagnostics.push(Diagnostic::error(
            heading.pos.line(),
            "level-two section name must not be empty",
        ));
    }
    let body_start = ctx.cursor.position();
    let body_end = p_section_body_end(ctx);

    let heading_line = heading.pos.line();
    let section_span = SourceSpan::new(heading_line, heading.pos.offset()..body_end);
    let body_span = SourceSpan::new(body_start.line(), body_start.offset()..body_end);
    let section_type = ctx.config.section_type(heading.title.text());
    match section_type {
        SectionType::Prose => Some(Section::Prose(Located::new(
            ProseSection::new(
                heading.title.map(|s| s.to_owned()),
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
                    title: heading.title.map(|s| s.to_owned()),
                    items: Located::new(items, body_span),
                },
                section_span,
            )))
        }
        SectionType::PlanItems => {
            ctx.cursor.rewind(body_start);
            ctx.code_block = None;
            let items = p_plan_items(ctx, body_end);
            Some(Section::PlanItems(Located::new(
                PlanItemSection {
                    title: heading.title.map(|s| s.to_owned()),
                    items: Located::new(items, body_span),
                },
                section_span,
            )))
        }
        SectionType::Findings => {
            ctx.cursor.rewind(body_start);
            ctx.code_block = None;
            let body = p_findings_body(ctx, body_end, body_span.clone());
            Some(Section::Findings(Located::new(
                FindingsSection {
                    title: heading.title.map(|s| s.to_owned()),
                    body,
                },
                section_span,
            )))
        }
    }
}

fn p_findings_body(
    ctx: &mut ParsingContext,
    body_end: usize,
    body_span: SourceSpan,
) -> FindingsBody {
    let body = &ctx.source[body_span.range()];
    if body.trim() == "No findings." {
        while ctx.cursor.position().offset() < body_end {
            ctx.cursor.take_line();
        }
        return FindingsBody::Sentinel(Located::new(body.to_owned(), body_span));
    }

    let items = p_expanded_items(ctx, body_end)
        .into_iter()
        .filter_map(|item| match item {
            Item::Expanded(expanded) => Some(Finding { expanded }),
            Item::Compact(_) => None,
        })
        .collect();
    FindingsBody::Items(Located::new(items, body_span))
}

fn p_section_body_end(ctx: &mut ParsingContext) -> usize {
    loop {
        let Some((_, line)) = ctx.cursor.peek_line() else {
            return ctx.source.len();
        };
        let start = ctx.cursor.position();
        if !is_code_block_line(line, &mut ctx.code_block) {
            match ctx.try_(p_markdown_heading) {
                Ok(next) if next.level == 1 => {
                    report_additional_title(ctx, start);
                    continue;
                }
                Ok(next) if next.level == 2 => {
                    ctx.cursor.rewind(start);
                    return start.offset();
                }
                Ok(_) => continue,
                Err(_) => {}
            }
        }
        ctx.cursor.take_line();
    }
}

fn p_itemised_items(ctx: &mut ParsingContext, body_end: usize) -> Vec<Item> {
    let mut items = p_compact_items(ctx, body_end);
    if ctx.cursor.position().offset() < body_end {
        items.extend(p_expanded_items(ctx, body_end));
    }
    items
}

fn p_plan_items(ctx: &mut ParsingContext, body_end: usize) -> Vec<PlanItem> {
    p_expanded_items(ctx, body_end)
        .into_iter()
        .map(|item| match item {
            Item::Expanded(expanded) => p_plan_item(ctx.source, expanded),
            Item::Compact(_) => unreachable!("expanded parser returned a compact item"),
        })
        .collect()
}

fn p_plan_item(source: &str, expanded: Located<ExpandedItem>) -> PlanItem {
    let body_span = expanded.value().body().span();
    let body_range = body_span.range();
    let body_end = body_range.end;
    let cursor = Cursor::within(source, body_range);
    let mut ctx = ParsingContext {
        source,
        cursor,
        diagnostics: Vec::new(),
        code_block: None,
        config: ParserConfig::default().with_known_metadata(["Status"]),
    };
    let metadata = p_metadata(&mut ctx);
    let prose_start = ctx.cursor.position();
    PlanItem {
        expanded,
        metadata,
        prose: Located::new(
            source[prose_start.offset()..body_end].to_owned(),
            SourceSpan::new(prose_start.line(), prose_start.offset()..body_end),
        ),
    }
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

// Parse compact peer items up to the section boundary or expanded phase.
fn p_compact_items(ctx: &mut ParsingContext, body_end: usize) -> Vec<Item> {
    let mut items = Vec::new();
    let mut peer_indentation = None;
    let mut open = None;

    while ctx.cursor.position().offset() < body_end {
        let start = ctx.cursor.position();
        let (_, line) = ctx.cursor.peek_line().expect("body has a line");
        let in_code_block = is_code_block_line(line, &mut ctx.code_block);
        let marker = (!in_code_block).then(|| compact_marker(line)).flatten();

        if !in_code_block && peek_heading_level(ctx) == Some(3) {
            if let Some(item) = open.take() {
                items.push(finish_compact_item(ctx.source, item, start));
            }
            break;
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
            ctx.diagnostics.push(Diagnostic::error(
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

        if !in_code_block && peek_heading_level(ctx) == Some(3) {
            if let Some(item) = open.take() {
                items.push(finish_expanded_item(ctx.source, item, start));
            }
            open = Some(p_expanded_item_opening(ctx));
            continue;
        }

        if open.is_none() && !line.trim().is_empty() {
            ctx.diagnostics.push(Diagnostic::error(
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
    ctx.diagnostics.push(Diagnostic::error(
        position.line(),
        "artifact must contain exactly one level-one heading",
    ));
}

// ============================================================================
// Markdown headings
// ============================================================================

struct MarkdownHeading<'a> {
    level: usize,
    title: Located<&'a str>,
    pos: Position,
}

/// Parse one ATX-style Markdown heading.
///
/// Branches:
///
/// - Success, titled:       `## Design Notes` -> level 2, title
///                           `Design Notes`.
/// - Success, empty:        `###` at EOF -> level 3, empty title span
///                           immediately after the markers.
/// - Success, indented:     `  # Title` is accepted.
/// - Failure, indentation:  `    # Title` is rejected.
/// - Failure, marker count: `plain text` and `####### Title` are rejected.
/// - Failure, no whitespace: `##Title` is rejected.
///
/// On failure, the cursor is left advanced; callers that need transactional
/// behavior should use `ParsingContext::try_`.
fn p_markdown_heading<'a>(ctx: &mut ParsingContext<'a>) -> Result<MarkdownHeading<'a>, ()> {
    let pos = ctx.cursor.position();
    for _ in 1..=3 {
        if ctx.cursor.take_if(|ch| ch == ' ').is_none() {
            break;
        }
    }
    let level = ctx.cursor.take_while(|ch| ch == '#').len();
    if !(1..=6).contains(&level) {
        return Err(());
    }
    match ctx.cursor.take_if(|ch| ch != '\n') {
        None => {
            ctx.cursor.take_line();
            let title_pos = ctx.cursor.position();
            return Ok(MarkdownHeading {
                level,
                title: ctx.located_with_pos(title_pos, title_pos.offset()),
                pos,
            });
        }
        Some(ch) if ch != '\t' && ch != ' ' => return Err(()),
        _ => (),
    }
    ctx.cursor.skip_whitespaces_inline();
    let text_pos = ctx.cursor.position();
    let Some((_, rest)) = ctx.cursor.take_line() else {
        return Err(());
    };
    let trimmed = rest.trim_end();
    let without_hashes = trimmed.trim_end_matches('#');
    let title_len = if without_hashes.is_empty() {
        0
    } else if without_hashes.ends_with([' ', '\t']) {
        without_hashes.trim_end().len()
    } else {
        trimmed.len()
    };
    let title = ctx.located_with_pos(text_pos, text_pos.offset() + title_len);
    Ok(MarkdownHeading { level, title, pos })
}

fn peek_heading_level(ctx: &mut ParsingContext) -> Option<usize> {
    let pos = ctx.cursor.position();
    let level = p_markdown_heading(ctx).map(|heading| heading.level).ok();
    ctx.cursor.rewind(pos);
    level
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

fn p_metadata_line(ctx: &mut ParsingContext) -> Option<Located<Metadata>> {
    ctx.cursor
        .try_(|cursor| {
            let start = cursor.position();
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
            let end = cursor.position().offset();
            Ok(Located::new(
                Metadata::new(
                    key.to_owned(),
                    Located::new(
                        value.to_owned(),
                        SourceSpan::new(start.line(), value_start..value_start + value.len()),
                    ),
                ),
                SourceSpan::new(start.line(), start.offset()..end),
            ))
        })
        .ok()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::artifact::ItemForm;

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
        ParserConfig::new(vec![SectionConfig::itemised(name)]).with_known_metadata(["Status"])
    }

    fn metadata_config<'a>(names: &[&'a str]) -> ParserConfig<'a> {
        ParserConfig::new(vec![]).with_known_metadata(names.iter().copied())
    }

    fn mixed_itemised_config(name: &str) -> ParserConfig {
        itemised_config(name)
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
            assert_eq!(*parsed.title.value(), "Title");
            assert_eq!(heading.cursor.position().line(), 2);

            let mut not_heading = context("###plain text\n");
            let start = not_heading.cursor.position();
            assert!(p_markdown_heading(&mut not_heading).is_err());
            assert_ne!(not_heading.cursor.position(), start);
        }

        #[test]
        fn metadata_line_consumes_success_and_rewinds_failures() {
            let mut metadata = context("- Status: proposed\nbody");
            let parsed = p_metadata_line(&mut metadata).expect("metadata should parse");
            assert_eq!(parsed.value().value(), "proposed");
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
            ctx.config = metadata_config(&["Status"]);

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
            let artifact =
                parse_with_config(artifact_source, &metadata_config(&["Status", "Purpose"]))
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
                "- Purpose: A value: with another colon\n"
            );
            assert_eq!(artifact.sections()[0].name(), "Overview");
            assert!(prose(&artifact.sections()[0])
                .body()
                .contains("### A subsection"));
        }

        #[test]
        fn tracks_metadata_spans_for_utf8_and_supported_line_endings() {
            for ending in ["\n", "\r\n", "\r"] {
                let source = ["# α", "  - Status: 値", "## Details", "text", ""].join(ending);
                let artifact = parse_with_config(&source, &metadata_config(&["Status"]))
                    .expect("artifact should parse");
                let metadata = &artifact.metadata()[0];
                let start = source.find("  - Status").unwrap();
                let end = source.find("## Details").unwrap();

                assert_eq!(metadata.span().start_line(), 2);
                assert_eq!(metadata.span().range(), start..end);
                assert_eq!(&source[metadata.span().range()], &source[start..end]);
                assert_eq!(metadata.value().value(), "値");
                let value = metadata.value().located_value();
                assert_eq!(&source[value.span().range()], "値");
                assert_eq!(artifact.serialize(), source);
            }
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
            let artifact = parse_with_config(source, &metadata_config(&["Status", "Updated"]))
                .expect("pre-section prose should be valid");

            assert_eq!(artifact.metadata().len(), 2);
            assert_eq!(artifact.metadata()[0].value().key(), "Updated");
            assert_eq!(
                artifact.pre_section_prose(),
                "\r\nOpaque α.\r\n- Status: accepted\r\n"
            );
            let prose = artifact.located_pre_section_prose();
            assert_eq!(prose.span().start_line(), 4);
            assert_eq!(&source[prose.span().range()], prose.text());
            assert_eq!(artifact.sections()[0].name(), "Details");
            assert_eq!(artifact.serialize(), source);
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
            let mut artifact = parse_with_config(source, &metadata_config(&["Status"]))
                .expect("fenced headings should remain pre-section prose");

            assert_eq!(
                artifact.pre_section_prose(),
                "```markdown\r\n## Not a section α\r\n```\r\nopaque tail\r\n"
            );
            assert_eq!(artifact.sections().len(), 1);
            assert_eq!(artifact.sections()[0].name(), "Details");

            artifact.metadata_mut()[0]
                .value_mut()
                .set_value("accepted")
                .expect("metadata value should be valid");
            assert_eq!(
                artifact.serialize(),
                source.replacen("drafting", "accepted", 1)
            );
        }

        #[test]
        fn unknown_metadata_starts_opaque_prose_for_a_contract() {
            let source = concat!(
                "# Example\n",
                "- Status: drafting\n",
                "- Extra: opaque\n",
                "- Updated: hidden\n",
            );
            let artifact = parse_with_config(source, &metadata_config(&["Status", "Updated"]))
                .expect("unknown metadata should start prose");

            assert_eq!(artifact.metadata().len(), 1);
            assert_eq!(
                artifact.pre_section_prose(),
                "- Extra: opaque\n- Updated: hidden\n"
            );
            assert!(artifact.sections().is_empty());
        }

        #[test]
        fn default_config_produces_no_metadata() {
            let source = "# Example\n- Custom: value\nplain prose\n";
            let artifact = parse(source).expect("default parsing should succeed");

            assert!(artifact.metadata().is_empty());
            assert_eq!(
                artifact.pre_section_prose(),
                "- Custom: value\nplain prose\n"
            );
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

            let warning = Diagnostic::warning(4, "section is empty");
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
                .value_mut()
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
                    concat!("- P-D1: compact α", "\n", "  compact body", "\n",)
                        .replace('\n', ending)
                );
                assert_eq!(items[1].identifier().unwrap().text(), "P-DD1");

                artifact.metadata_mut()[0]
                    .value_mut()
                    .set_value("accepted")
                    .expect("metadata should be mutable");
                assert_eq!(
                    artifact.serialize(),
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
            assert_eq!(items[0].body().text(), "body\n- P-D1: too late\n");
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
                items[0].body().text(),
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

            assert!(diagnostics.iter().any(|diagnostic| {
                diagnostic.line() == 3 && diagnostic.message().contains("expected a compact item")
            }));
        }

        #[test]
        fn ignores_item_markers_inside_fenced_bodies() {
            let source = concat!(
                "# Plan\n",
                "## Decisions\n",
                "- P-D1: compact\n",
                "```markdown\n",
                "### Not expanded\n",
                "```\n",
                "### P-DD1: expanded\n",
                "~~~markdown\n",
                "- P-D2: not compact\n",
                "~~~\n",
            );
            let artifact = parse_with_config(source, &mixed_itemised_config("Decisions"))
                .expect("fenced markers should remain item bodies");
            let items = itemised(&artifact.sections()[0]).items();

            assert_eq!(items.len(), 2);
            assert!(items[0].body().text().contains("### Not expanded"));
            assert!(items[1].body().text().contains("- P-D2: not compact"));
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
            let mut artifact = parse_with_config(source, &metadata_config(&["Status"]))
                .expect("artifact should parse");

            artifact.metadata_mut()[0]
                .value_mut()
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
            let mut artifact = parse_with_config(source, &metadata_config(&["Status", "Purpose"]))
                .expect("artifact should parse");
            let section_span = artifact.sections()[0].span().clone();

            artifact.metadata_mut()[0]
                .value_mut()
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
            let mut artifact =
                parse_with_config(source, &metadata_config(&["First", "Second", "Third"]))
                    .expect("artifact should parse");

            let entries: Vec<_> = artifact
                .metadata()
                .iter()
                .map(|entry| {
                    let metadata = entry.value();
                    (metadata.key(), metadata.value())
                })
                .collect();
            assert_eq!(
                entries,
                vec![("First", "one"), ("Second", "two"), ("Third", "three")]
            );

            artifact.metadata_mut()[0]
                .value_mut()
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
                .with_known_metadata(["Status"])
        }

        #[test]
        fn reuses_expanded_items_with_structured_status_and_opaque_prose() {
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
            assert_eq!(first.identifier().expect("identifier").text(), "P1");
            assert_eq!(first.title().text(), "First item");
            assert_eq!(first.metadata().len(), 1);
            assert_eq!(first.metadata()[0].value().key(), "Status");
            assert_eq!(first.metadata()[0].value().value(), "pending");
            assert!(first.prose().text().contains("- Goal criteria: G-AC1"));
            assert!(first
                .prose()
                .text()
                .contains("- Status: opaque after boundary"));
            assert!(first.prose().text().contains("#### Details"));
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
            assert!(items[0].prose().text().contains("# Fenced title"));
            assert!(items[0].prose().text().contains("## Fenced section"));
            assert!(items[0].prose().text().contains("### P999: fenced example"));
            assert_eq!(items[1].identifier().expect("identifier").text(), "P2");
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
            assert_eq!(first.metadata()[0].span().range(), status_start..status_end);
            assert_eq!(first.metadata()[0].span().start_line(), 4);

            let prose_start = source.find("- Outcome: opaque").expect("prose start");
            assert_eq!(first.prose().span().range(), prose_start..item_end);
            assert_eq!(first.prose().span().start_line(), 5);
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
                artifact.serialize(),
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
            let mut artifact =
                parse_with_config(source, &config()).expect("structure should parse");

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
                    artifact.serialize(),
                    source.replacen("- Status: pending", "- Status: done", 1)
                );
                assert_eq!(
                    artifact.plan_item("P1").expect("item").prose().text(),
                    format!("- Outcome: untouched{ending}")
                );
            }
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
            assert_eq!(items[0].identifier().expect("identifier").text(), "P1");
            assert_eq!(items[0].prose().text(), "");
            assert!(items[0].metadata().is_empty());
            assert!(items[1].identifier().is_none());
            assert!(items[1].expanded.value().delimiter().is_none());
            assert_eq!(items[2].identifier().expect("empty identifier").text(), "");
            assert!(items[2].expanded.value().delimiter().is_some());
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
            assert!(items[1].identifier().is_none());
            assert_eq!(artifact.sections()[1].name(), "Following section");

            let diagnostics = parse_with_config(
                "# Plan\n## Plan Items\n### P1: Item\n# Broken hierarchy\n",
                &config(),
            )
            .expect_err("level-one heading should remain a parser error");
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
            assert_eq!(section.body().as_sentinel(), Some("No findings.\n"));
            assert_eq!(section.body().span().start_line(), 3);
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
            assert_eq!(items[0].body().text(), "- Severity: opaque\n");
            assert_eq!(artifact.sections()[1].name(), "Next");
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
            assert_eq!(artifact.sections()[1].name(), "Next");
        }
    }
}
