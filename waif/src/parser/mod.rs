use self::cursor::Cursor;
use crate::artifact::{
    Artifact, CompactItem, ExpandedItem, Finding, FindingsBody, FindingsSection, Item,
    ItemisedSection, Located, Metadata, PlanItem, PlanItemsSection, ProseSection, Section,
    SourceSpan,
};
use std::fmt;
use std::iter::repeat_n;

pub use self::cursor::Position;

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
    column: usize,
    message: String,
}

impl Diagnostic {
    /// error from position
    pub(crate) fn error_p(position: Position, message: impl Into<String>) -> Self {
        Self::with_severity(Severity::Error, position.line(), position.column(), message)
    }

    /// warning from position
    pub(crate) fn warning_p(position: Position, message: impl Into<String>) -> Self {
        Self::with_severity(
            Severity::Warning,
            position.line(),
            position.column(),
            message,
        )
    }

    // TODO: remove it
    pub(crate) fn error1(line: usize, message: impl Into<String>) -> Self {
        Self::with_severity(Severity::Error, line, 1, message)
    }

    // TODO: remove it
    pub(crate) fn warning1(line: usize, message: impl Into<String>) -> Self {
        Self::with_severity(Severity::Warning, line, 1, message)
    }

    fn with_severity(
        severity: Severity,
        line: usize,
        column: usize,
        message: impl Into<String>,
    ) -> Self {
        Self {
            severity,
            line,
            column,
            message: message.into(),
        }
    }

    pub fn severity(&self) -> Severity {
        self.severity
    }

    pub fn line(&self) -> usize {
        self.line
    }

    #[allow(dead_code)]
    pub fn column(&self) -> usize {
        self.column
    }

    pub fn message(&self) -> &str {
        &self.message
    }
}

impl fmt::Display for Diagnostic {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "line {}, column {}: {}",
            self.line, self.column, self.message
        )
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
    sections: Vec<SectionConfig<'a>>,
}

impl<'a> ParserConfig<'a> {
    pub fn new(sections: Vec<SectionConfig<'a>>) -> Self {
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

pub fn parse_with_config<'a>(
    source: &'a str,
    config: &ParserConfig,
) -> Result<Artifact<'a>, Vec<Diagnostic>> {
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
pub fn parse_with_diagnostics<'a, 'b: 'a>(
    source: &'b str,
    config: &ParserConfig<'a>,
) -> (Artifact<'b>, Vec<Diagnostic>) {
    let mut ctx = ParsingContext::new(source, config.clone());
    let artifact = p_artifact(&mut ctx);
    (artifact, ctx.diagnostics)
}

// ============================================================================
// Parser state
// ============================================================================

#[derive(Clone)]
struct ParsingContext<'p, 'a> {
    source: &'a str,
    cursor: Cursor<'a>,
    diagnostics: Vec<Diagnostic>,
    config: ParserConfig<'p>,
}

impl<'p, 'a> ParsingContext<'p, 'a> {
    pub const fn new(source: &'a str, config: ParserConfig<'p>) -> Self {
        Self {
            source,
            cursor: Cursor::new(source),
            diagnostics: Vec::new(),
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

    pub(crate) fn located_with_pos(&self, position: Position, end: Position) -> Located<&'a str> {
        let text = &self.source[position.offset()..end.offset()];
        Located::new(text, SourceSpan::new(position, end))
    }
}

// ============================================================================
// Artifact envelope
// ============================================================================

fn p_artifact<'a>(ctx: &mut ParsingContext<'_, 'a>) -> Artifact<'a> {
    ctx.cursor.skip_whitespace_lines();
    let title = p_title_line(ctx);
    let metadata = p_metadata(ctx);
    let pre_section_prose = p_markdown_section_body(ctx, 2);
    let sections = p_sections(ctx);
    Artifact::new(ctx.source, title, metadata, pre_section_prose, sections)
}

fn p_title_line<'a>(ctx: &mut ParsingContext<'_, 'a>) -> Located<&'a str> {
    let pos = ctx.cursor.position();
    if ctx.cursor.is_eof() {
        ctx.diagnostics
            .push(Diagnostic::error_p(pos, "missing level-one title"));
        return ctx.located_with_pos(pos, pos);
    }

    let Ok(MarkdownHeading {
        level: 1, title, ..
    }) = ctx.try_(p_markdown_heading)
    else {
        ctx.diagnostics.push(Diagnostic::error_p(
            pos,
            "first non-whitespace line must be a level-one Markdown heading",
        ));
        return ctx.located_with_pos(pos, pos);
    };
    if title.value().is_empty() {
        ctx.diagnostics.push(Diagnostic::error_p(
            pos,
            "level-one title must not be empty",
        ));
    }
    title
}

fn p_metadata<'a>(ctx: &mut ParsingContext<'_, 'a>) -> Vec<Located<Metadata<'a>>> {
    let mut metadata = Vec::new();
    loop {
        let before_whitespace = ctx.cursor.position();
        ctx.cursor.skip_whitespace_lines();
        if ctx.cursor.is_eof() {
            ctx.cursor.rewind(before_whitespace);
            break;
        }
        if let Ok(entry) = ctx.try_(p_metadata_line) {
            metadata.push(entry);
            continue;
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

fn p_metadata_line<'a>(
    ctx: &mut ParsingContext<'_, 'a>,
) -> Result<Located<Metadata<'a>>, Diagnostic> {
    let mut subctx = ctx.clone();
    let item = p_markdown_item(ctx)?;
    let item_content = &item.value().content;
    let content = item_content
        .text()
        .trim_end_matches(['\t', '\r', '\n', ' ']);
    let value_end = item_content.span().start().advance(content);
    subctx
        .cursor
        .skip_to_offset(item_content.span().range().start);
    subctx.cursor.limit(value_end.offset());
    let key_start = subctx.cursor.position();
    let key = subctx
        .cursor
        .take_while(|ch| ch != ':' && ch != '\n')
        .trim_end_matches(['\t', ' ']);
    if key.is_empty() {
        return Err(Diagnostic::error_p(key_start, "expected key"));
    }
    let key_l = subctx.located_with_pos(key_start, key_start.advance(key));
    let colon_pos = subctx.cursor.position();
    if !subctx.cursor.take().is_some_and(|ch| ch == ':') {
        return Err(Diagnostic::error_p(colon_pos, "expected colon"));
    }
    subctx.cursor.skip_whitespaces_inline();
    let value_start = subctx.cursor.position();
    let value_l = subctx
        .located_with_pos(value_start, value_end)
        .map(|s| s.to_owned());
    Ok(Located::new(
        Metadata::new(key_l, value_l),
        item.span().clone(),
    ))
}

fn p_sections<'a>(ctx: &mut ParsingContext<'_, 'a>) -> Vec<Section<'a>> {
    let mut sections = Vec::new();
    loop {
        if let Some(section) = p_section(ctx) {
            sections.push(section);
            continue;
        }

        let pos = ctx.cursor.position();
        match ctx.try_(p_markdown_heading) {
            Ok(heading) if heading.level == 1 => {
                ctx.diagnostics.push(Diagnostic::error_p(
                    pos,
                    "artifact must contain exactly one level-one heading",
                ));
                p_markdown_section_body(ctx, 2);
            }
            _ => break,
        }
    }
    sections
}

fn p_section<'a>(ctx: &mut ParsingContext<'_, 'a>) -> Option<Section<'a>> {
    let pos = ctx.cursor.position();
    let (section, span) = ctx.try_(p_markdown_section).ok()?.unpack();
    if section.level != 2 {
        ctx.cursor.rewind(pos);
        return None;
    }
    if section.title.text().is_empty() {
        ctx.diagnostics.push(Diagnostic::error_p(
            pos,
            "level-two section name must not be empty",
        ));
    }
    let body_start = *section.content.span().start();
    let body_end = section.content.span().end().offset();
    let body_span = section.content.span().clone();
    let section_type = ctx.config.section_type(section.title.text());
    match section_type {
        SectionType::Prose => Some(Section::Prose(Located::new(
            ProseSection::new(section.title, section.content),
            span,
        ))),
        SectionType::Itemised => {
            ctx.cursor.rewind(body_start);
            let items = p_itemised_items(ctx, body_end);
            Some(Section::Itemised(Located::new(
                ItemisedSection {
                    title: section.title,
                    items: Located::new(items, body_span),
                },
                span,
            )))
        }
        SectionType::PlanItems => {
            ctx.cursor.rewind(body_start);
            let items = p_plan_items(ctx, body_end);
            Some(Section::PlanItems(Located::new(
                PlanItemsSection {
                    title: section.title,
                    items: Located::new(items, body_span),
                },
                span,
            )))
        }
        SectionType::Findings => {
            ctx.cursor.rewind(body_start);
            let body = p_findings_body(ctx, body_end);
            Some(Section::Findings(Located::new(
                FindingsSection {
                    title: section.title,
                    body,
                },
                span,
            )))
        }
    }
}

fn p_itemised_items<'a>(ctx: &mut ParsingContext<'_, 'a>, body_end: usize) -> Vec<Item<'a>> {
    let mut items = Vec::new();
    for ci in p_compact_items(ctx, body_end) {
        items.push(Item::Compact(ci));
    }
    if ctx.cursor.position().offset() < body_end {
        for ei in p_expanded_items(ctx, body_end) {
            items.push(Item::Expanded(ei));
        }
    }
    items
}

fn p_expanded_items<'a>(
    ctx: &mut ParsingContext<'_, 'a>,
    body_end: usize,
) -> Vec<Located<ExpandedItem<'a>>> {
    let mut items = Vec::new();
    let old_limit = ctx.cursor.limit(body_end);

    while !ctx.cursor.is_eof() {
        let (_, kind) = peek_line_kind(ctx);
        match kind {
            LineKind::Heading { level: 3 } => {
                let item = p_expanded_item(ctx).expect("level-three section should parse");
                items.push(item);
            }
            LineKind::FenceStart { .. } => {
                let pos = ctx.cursor.position();
                ctx.diagnostics
                    .push(Diagnostic::error_p(pos, "unexpected code block"));
                p_skip_code_block(ctx).expect("must be code block");
            }
            LineKind::Blank => {
                ctx.cursor.take_line().expect("not eof");
            }
            _ => {
                let pos = ctx.cursor.position();
                ctx.diagnostics.push(Diagnostic::error_p(
                    pos,
                    "expected an expanded item in `### ID: title` form",
                ));
                ctx.cursor.take_line().expect("not eof");
            }
        }
    }
    ctx.cursor.limit(old_limit);
    items
}

fn p_expanded_item<'a>(
    ctx: &mut ParsingContext<'_, 'a>,
) -> Result<Located<ExpandedItem<'a>>, Diagnostic> {
    let mut subctx = ctx.clone();
    let (section, span) = p_markdown_section(ctx)?.unpack();
    if section.level != 3 {
        return Err(Diagnostic::error_p(
            *section.marker.span().start(),
            "expected a level-three section",
        ));
    }
    let title_start = *section.title.span().start();
    subctx.cursor.rewind(title_start);
    subctx.cursor.limit(section.title.span().end().offset());
    let (identifier, delimiter, title) = match p_content_with_id(&mut subctx) {
        Ok(ContentWithId {
            id,
            content,
            delimiter,
        }) => (Some(id), Some(delimiter), content),
        Err(_) => (None, None, section.title),
    };

    Ok(Located::new(
        ExpandedItem {
            marker: section.marker,
            identifier,
            title,
            delimiter,
            content: section.content,
        },
        span,
    ))
}

// Parse compact peer items up to the section boundary or expanded phase.
fn p_compact_items<'a>(
    ctx: &mut ParsingContext<'_, 'a>,
    body_end: usize,
) -> Vec<Located<CompactItem<'a>>> {
    let mut items = Vec::new();

    let mut old_cursor = ctx.cursor.clone();
    ctx.cursor.limit(body_end);

    while !ctx.cursor.is_eof() {
        let (_, kind) = peek_line_kind(ctx);
        match kind {
            LineKind::Heading { .. } => {
                break;
            }
            LineKind::FenceStart { .. } => {
                let pos = ctx.cursor.position();
                ctx.diagnostics
                    .push(Diagnostic::error_p(pos, "unexpected code block"));
                p_skip_code_block(ctx).expect("must be code block");
                continue;
            }
            LineKind::Blank => {
                ctx.cursor.take_line().expect("not eof");
                continue;
            }
            _ => {}
        }
        match ctx.try_(p_compact_item) {
            Ok(item) => {
                items.push(item);
            }
            Err(diagnostic) => {
                ctx.cursor.take_line().expect("not eof");
                ctx.diagnostics.push(diagnostic);
            }
        };
    }
    old_cursor.rewind(ctx.cursor.position());
    ctx.cursor = old_cursor;
    items
}

fn p_compact_item<'a>(
    ctx: &mut ParsingContext<'_, 'a>,
) -> Result<Located<CompactItem<'a>>, Diagnostic> {
    let mut subctx = ctx.clone();
    let (m_item, span) = p_markdown_item(ctx)?.unpack();
    subctx.cursor.rewind(*m_item.content.span().start());
    subctx.cursor.limit(m_item.content.span().end().offset());
    Ok(match p_content_with_id(&mut subctx) {
        Ok(ContentWithId {
            id,
            delimiter,
            content,
        }) => Located::new(
            CompactItem {
                marker: m_item.marker,
                identifier: Some(id),
                delimiter: Some(delimiter),
                content,
            },
            span,
        ),
        Err(_) => Located::new(
            CompactItem {
                marker: m_item.marker,
                identifier: None,
                delimiter: None,
                content: m_item.content,
            },
            span,
        ),
    })
}

fn p_plan_items<'a>(
    ctx: &mut ParsingContext<'_, 'a>,
    body_end: usize,
) -> Vec<Located<PlanItem<'a>>> {
    p_expanded_items(ctx, body_end)
        .into_iter()
        .map(|item| {
            let mut subctx = ctx.clone();
            // let (item, span) = li.unpack();
            subctx.cursor.rewind(*item.value().content().span().start());
            subctx.cursor.skip_whitespace_lines();
            let metadata = p_metadata(&mut subctx);
            let prose_start = subctx.cursor.position();
            let content_end = *item.span().end();
            let span = item.span().clone();
            Located::new(
                PlanItem {
                    item,
                    metadata,
                    prose: ctx.located_with_pos(prose_start, content_end),
                },
                span,
            )
        })
        .collect()
}

fn p_findings_body<'a>(ctx: &mut ParsingContext<'_, 'a>, body_end: usize) -> FindingsBody<'a> {
    ctx.cursor.skip_whitespace_lines();
    if ctx
        .cursor
        .peek_line()
        .is_some_and(|(_, line)| line.trim() == "No findings.")
    {
        ctx.cursor.skip_whitespaces_inline();
        let start = ctx.cursor.position();
        ctx.cursor.take_while(|ch| ch != '.');
        ctx.cursor.take().expect("expect .");
        let end = ctx.cursor.position();
        ctx.cursor.take_line();
        ctx.cursor.skip_whitespace_lines();
        if !ctx.cursor.is_eof() && peek_heading_level(ctx).is_err() {
            let pos = ctx.cursor.position();
            ctx.diagnostics
                .push(Diagnostic::error_p(pos, "expect a heading"));
            ctx.cursor.skip_to_offset(body_end);
        }
        FindingsBody::Sentinel(ctx.located_with_pos(start, end))
    } else {
        let start = ctx.cursor.position();
        let items = p_expanded_items(ctx, body_end)
            .into_iter()
            .map(|item| Finding { item })
            .collect();
        let end = ctx.cursor.position();
        FindingsBody::Items(Located::new(items, SourceSpan::new(start, end)))
    }
}

// ============================================================================
// common parts
// ============================================================================

struct ContentWithId<'a> {
    id: Located<&'a str>,
    delimiter: Located<&'a str>,
    content: Located<&'a str>,
}

// expected to work in a limited context
fn p_content_with_id<'a>(
    ctx: &mut ParsingContext<'_, 'a>,
) -> Result<ContentWithId<'a>, Diagnostic> {
    let start = ctx.cursor.position();
    let id = ctx.cursor.take_while(|c| c.is_alphanumeric() || c == '-');
    if id.is_empty() {
        return Err(Diagnostic::error_p(start, "Expected identifier"));
    }
    let id_end = ctx.cursor.position();
    ctx.cursor.skip_whitespaces_inline();
    let delim_start = ctx.cursor.position();
    if ctx.cursor.take_if(|ch| ch == ':').is_none() {
        return Err(Diagnostic::error_p(start, "Expected delimiter"));
    }
    let delim_end = ctx.cursor.position();
    ctx.cursor.skip_whitespaces_inline();
    let content_start = ctx.cursor.position();
    ctx.cursor.skip_remaining();
    let content_end = ctx.cursor.position();
    Ok(ContentWithId {
        id: ctx.located_with_pos(start, id_end),
        delimiter: ctx.located_with_pos(delim_start, delim_end),
        content: ctx.located_with_pos(content_start, content_end),
    })
}

// ============================================================================
// Markdown components
// ============================================================================

#[allow(dead_code)]
enum LineKind<'a> {
    EOF,
    Blank,
    Heading { level: usize },
    UnorderedItem { marker: Located<&'a str> },
    FenceStart { marker: Located<&'a str> },
    Text,
}

/// Returns (indentation, line_kind)
fn peek_line_kind<'a>(ctx: &ParsingContext<'_, 'a>) -> (usize, LineKind<'a>) {
    let start = ctx.cursor.position();
    let Some((_, line)) = ctx.cursor.peek_line() else {
        return (0, LineKind::EOF);
    };
    if line.trim().is_empty() {
        return (0, LineKind::Blank);
    }
    if let Ok((level, _)) = peek_heading_level(ctx) {
        return (0, LineKind::Heading { level });
    }
    if let Ok((marker, _)) = peek_is_item(ctx) {
        return (
            0,
            LineKind::UnorderedItem {
                marker: ctx.located_with_pos(start, start.advance(&marker.to_string())),
            },
        );
    }
    let mut ctx = ctx.clone();
    if let Ok((marker, n, indentation)) = ctx.try_(p_maybe_code_block_fence) {
        let marker_s: String = repeat_n(marker, n).collect();
        return (
            indentation,
            LineKind::FenceStart {
                marker: ctx.located_with_pos(start, start.advance(&marker_s)),
            },
        );
    }
    let indentation = ctx.cursor.take_while(|ch| ch == ' ').len();
    (indentation, LineKind::Text)
}

struct MarkdownSection<'a> {
    level: usize,
    marker: Located<&'a str>,
    title: Located<&'a str>,
    content: Located<&'a str>,
}

/// Parse a Markdown heading and its body through the next peer or parent
/// heading. Deeper headings and fenced headings remain part of the body.
fn p_markdown_section<'a>(
    ctx: &mut ParsingContext<'_, 'a>,
) -> Result<Located<MarkdownSection<'a>>, Diagnostic> {
    let start = ctx.cursor.position();
    let heading = p_markdown_heading(ctx)?;
    let level = heading.level;
    let body = p_markdown_section_body(ctx, heading.level);
    let end = ctx.cursor.position();
    Ok(Located::new(
        MarkdownSection {
            level,
            marker: heading.marker,
            title: heading.title,
            content: body,
        },
        SourceSpan::new(start, end),
    ))
}

fn p_markdown_section_body<'a>(
    ctx: &mut ParsingContext<'_, 'a>,
    until_level: usize,
) -> Located<&'a str> {
    let start = ctx.cursor.position();
    while !ctx.cursor.is_eof() {
        if ctx.try_(p_skip_code_block).is_ok() {
            continue;
        }

        match peek_heading_level(ctx) {
            Ok((new_level, _)) if new_level <= until_level => {
                break;
            }
            _ => {
                ctx.cursor.take_line();
            }
        }
    }
    let end = ctx.cursor.position();
    ctx.located_with_pos(start, end)
}

struct MarkdownHeading<'a> {
    level: usize,
    marker: Located<&'a str>,
    title: Located<&'a str>,
}

/// Parse one ATX-style Markdown heading.
///
/// Branches:
///
/// - Success, titled:       `## Design Notes` -> level 2, title
///                           `Design Notes`.
/// - Success, empty:        `###` at EOF -> level 3, empty title span
///                           immediately after the markers.
/// - Failure, indentation:  `  # Title` and `    # Title` are rejected.
/// - Failure, marker count: `plain text` and `####### Title` are rejected.
/// - Failure, no whitespace: `##Title` is rejected.
///
/// On failure, the cursor position is not guaranteed.
fn p_markdown_heading<'a>(
    ctx: &mut ParsingContext<'_, 'a>,
) -> Result<MarkdownHeading<'a>, Diagnostic> {
    let start = ctx.cursor.position();
    let (level, pos) = peek_heading_level(ctx)?;
    let marker = ctx.located_with_pos(start, pos);
    ctx.cursor.rewind(pos);
    match ctx.cursor.take_if(|ch| ch != '\n') {
        None => {
            ctx.cursor.take_line();
            let title_pos = ctx.cursor.position();
            return Ok(MarkdownHeading {
                level,
                marker,
                title: ctx.located_with_pos(title_pos, title_pos),
            });
        }
        Some(ch) if ch != '\t' && ch != ' ' => {
            let title_pos = ctx.cursor.position();
            return Err(Diagnostic::error_p(title_pos, "whitespace expected"));
        }
        _ => (),
    }
    ctx.cursor.skip_whitespaces_inline();
    let text_pos = ctx.cursor.position();
    let Some((_, rest)) = ctx.cursor.take_line() else {
        return Err(Diagnostic::error_p(text_pos, "unexpected EOF"));
    };
    let trimmed = rest.trim_end();
    let without_hashes = trimmed.trim_end_matches('#');
    let title = if without_hashes.is_empty() {
        ""
    } else if without_hashes.ends_with([' ', '\t']) {
        without_hashes.trim_end()
    } else {
        trimmed
    };
    let title = ctx.located_with_pos(text_pos, text_pos.advance(title));
    Ok(MarkdownHeading {
        level,
        marker,
        title,
    })
}

/// Returns (level, position immediate after `#`)
fn peek_heading_level(ctx: &ParsingContext) -> Result<(usize, Position), Diagnostic> {
    let mut ctx = ctx.clone();
    // CommonMark says indentation is allowed, but we don't support it.
    let sharp_pos = ctx.cursor.position();
    let level = ctx.cursor.take_while(|ch| ch == '#').len();
    if !(1..=6).contains(&level) {
        return Err(Diagnostic::error_p(
            sharp_pos,
            format!("invalid heading level: {}", level),
        ));
    }
    if ctx
        .cursor
        .peek()
        .is_some_and(|ch| ![' ', '\t', '\n'].contains(&ch))
    {
        return Err(Diagnostic::error_p(
            ctx.cursor.position(),
            "expected space after heading marker",
        ));
    }
    Ok((level, ctx.cursor.position()))
}

struct MarkdownItem<'a> {
    marker: Located<&'a str>,
    content: Located<&'a str>,
}

fn p_markdown_item<'a>(
    ctx: &mut ParsingContext<'_, 'a>,
) -> Result<Located<MarkdownItem<'a>>, Diagnostic> {
    let start = ctx.cursor.position();
    let (_, marker_end) = peek_is_item(ctx)?;
    ctx.cursor.rewind(marker_end);
    ctx.cursor.skip_whitespaces_inline();
    let content_start = ctx.cursor.position();
    ctx.cursor.take_line();
    loop {
        let (indentation, kind) = peek_line_kind(ctx);
        let is_indented = indentation > start.column() - 1;

        match kind {
            LineKind::FenceStart { .. } if is_indented => {
                p_skip_code_block(ctx)?;
            }
            LineKind::EOF
            | LineKind::Heading { .. }
            | LineKind::UnorderedItem { .. }
            | LineKind::FenceStart { .. } => break,
            LineKind::Blank => {
                ctx.cursor.take_line();
            }
            LineKind::Text if is_indented => {
                ctx.cursor.take_line();
            }
            LineKind::Text => break,
        };
    }
    let content_end = ctx.cursor.position();

    Ok(Located::new(
        MarkdownItem {
            marker: ctx.located_with_pos(start, marker_end),
            content: ctx.located_with_pos(content_start, content_end),
        },
        SourceSpan::new(start, content_end),
    ))
}

/// Returns (marker, position immediate after `-`)
fn peek_is_item(ctx: &ParsingContext) -> Result<(char, Position), Diagnostic> {
    let mut ctx = ctx.clone();
    // CommonMark says indentation is allowed, but we don't support it.
    let Some(ch) = ctx.cursor.take_if(|ch| ch == '-') else {
        return Err(Diagnostic::error_p(ctx.cursor.position(), "expected `-`"));
    };
    if ctx
        .cursor
        .peek()
        .is_some_and(|ch| ![' ', '\t', '\n'].contains(&ch))
    {
        return Err(Diagnostic::error_p(
            ctx.cursor.position(),
            "expected space after item marker",
        ));
    }
    Ok((ch, ctx.cursor.position()))
}

/// Consume fenced code blocks and report whether consumed.
fn p_skip_code_block(ctx: &mut ParsingContext) -> Result<(), Diagnostic> {
    let (open_ch, open_count, _) = p_maybe_code_block_fence(ctx)?;
    ctx.cursor.take_line();
    loop {
        match p_maybe_code_block_fence(ctx) {
            Ok((ch, count, _)) if ch == open_ch && count >= open_count => {
                ctx.cursor.skip_whitespaces_inline();
                if !ctx.cursor.take().is_some_and(|ch| ch != '\n') {
                    return Ok(());
                }
            }
            _ => {
                if ctx.cursor.take_line().is_none() {
                    return Ok(());
                };
            }
        }
    }
}

/// Returns (marker_char, marker_count, indent_level) if the next line is a code block fence
fn p_maybe_code_block_fence(ctx: &mut ParsingContext) -> Result<(char, usize, usize), Diagnostic> {
    let start = ctx.cursor.position();
    while ctx.cursor.position().column() <= 3 {
        if ctx.cursor.take_if(|ch| ch == ' ').is_none() {
            break;
        }
    }
    let indentation_level = ctx.cursor.position().column() - 1;
    let marker_ch = match ctx.cursor.peek() {
        Some(ch) if ch == '`' || ch == '~' => ch,
        _ => return Err(Diagnostic::error_p(start, "expected code block marker")),
    };
    let marker = ctx.cursor.take_while(|ch| ch == marker_ch);
    if marker.len() < 3 {
        return Err(Diagnostic::error_p(start, "expected code block marker"));
    }
    if ctx
        .cursor
        .peek_line()
        .is_some_and(|(_, line)| marker_ch == '`' && line.contains('`'))
    {
        return Err(Diagnostic::error_p(start, "expected code block marker"));
    }
    Ok((marker_ch, marker.len(), indentation_level))
}

#[cfg(test)]
mod tests;
