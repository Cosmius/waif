use std::fmt;
use std::ops::Range;

#[derive(Debug, PartialEq, Eq)]
pub struct Artifact<'a, L = SourceSpan> {
    source: String,
    title: String,
    metadata: Vec<Located<Metadata<'a, L>, L>>,
    pre_section_prose: Located<String, L>,
    sections: Vec<Section<'a, L>>,
}

// This is the structured artifact API. The check command currently only needs
// parsing to succeed, while later commands will consume these accessors.
#[allow(dead_code)]
impl<'a, L> Artifact<'a, L> {
    pub(crate) fn new(
        source: String,
        title: String,
        metadata: Vec<Located<Metadata<'a, L>, L>>,
        pre_section_prose: Located<String, L>,
        sections: Vec<Section<'a, L>>,
    ) -> Self {
        Self {
            source,
            title,
            metadata,
            pre_section_prose,
            sections,
        }
    }

    /// The exact input, including its original whitespace and line endings.
    pub fn source(&self) -> &str {
        &self.source
    }

    pub fn title(&self) -> &str {
        &self.title
    }

    pub fn metadata(&self) -> &[Located<Metadata<'a, L>, L>] {
        &self.metadata
    }

    /// Existing metadata entries, mutably borrowed in source order.
    ///
    /// The slice cannot add, remove, or reorder entries. Use
    /// [`Metadata::set_value`] to change an existing value. Keys and ordering
    /// are controlled by the applicable artifact contract.
    pub fn metadata_mut(&mut self) -> &mut [Located<Metadata<'a, L>, L>] {
        &mut self.metadata
    }

    /// Opaque source between structured metadata and the first section.
    pub fn pre_section_prose(&self) -> &str {
        self.pre_section_prose.text()
    }

    pub fn located_pre_section_prose(&self) -> &Located<String, L> {
        &self.pre_section_prose
    }

    pub fn sections(&self) -> &[Section<'a, L>] {
        &self.sections
    }

    pub fn plan_item(&self, identifier: &str) -> Option<&PlanItem<'a, L>> {
        let mut matches =
            self.sections
                .iter()
                .filter_map(Section::as_plan_items)
                .flat_map(|section| {
                    section.items().iter().filter(move |item| {
                        item.identifier().is_some_and(|id| id.text() == identifier)
                    })
                });
        let item = matches.next()?;
        matches.next().is_none().then_some(item)
    }

    pub fn plan_item_mut(&mut self, identifier: &str) -> Option<&mut PlanItem<'a, L>> {
        let count = self
            .sections
            .iter()
            .filter_map(Section::as_plan_items)
            .flat_map(PlanItemSection::items)
            .filter(|item| item.identifier().is_some_and(|id| id.text() == identifier))
            .count();
        if count != 1 {
            return None;
        }
        self.sections
            .iter_mut()
            .filter_map(Section::as_plan_items_mut)
            .flat_map(PlanItemSection::items_mut)
            .find(|item| item.identifier().is_some_and(|id| id.text() == identifier))
    }

}

impl<'a> Artifact<'a, SourceSpan> {
    pub fn serialize(&self) -> String { serialize(self) }
}

#[derive(Debug, PartialEq, Eq)]
pub struct Metadata<'a, L = SourceSpan> {
    key: Located<&'a str, L>,
    value: Located<String, L>,
}

#[allow(dead_code)]
impl<'a, L> Metadata<'a, L> {
    pub(crate) fn new(key: Located<&'a str, L>, value: Located<String, L>) -> Self {
        Self { key, value }
    }

    pub fn key(&self) -> &'a str {
        self.key.text()
    }

    pub fn value(&self) -> &str {
        self.value.text()
    }

    pub fn located_value(&self) -> &Located<String, L> {
        &self.value
    }

    /// Replace this entry's value for the next serialization.
    ///
    /// Metadata values are single-line values. The surrounding source text,
    /// including the separator and line ending, remains unchanged.
    pub fn set_value(&mut self, value: impl Into<String>) -> Result<(), InvalidValue> {
        let value = value.into();
        if self.value.value.contains(['\r', '\n']) {
            return Err(InvalidValue::Immutable);
        }
        if value.contains(['\r', '\n']) {
            return Err(InvalidValue::Multiline);
        }
        self.value.value = value;
        Ok(())
    }
}

#[derive(Debug, PartialEq, Eq)]
pub enum InvalidValue {
    /// The parsed value spans multiple lines and cannot be edited safely.
    Immutable,
    /// The replacement value contains a line ending.
    Multiline,
}

impl fmt::Display for InvalidValue {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Immutable => write!(formatter, "metadata value is immutable"),
            Self::Multiline => write!(formatter, "metadata values must be single-line"),
        }
    }
}

impl std::error::Error for InvalidValue {}

use crate::parser::Position;

/// A half-open UTF-8 byte range and its one-based starting line.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SourceSpan {
    start: Position,
    end: Position,
}

#[allow(dead_code)]
impl SourceSpan {
    pub(crate) fn new(start: Position, end: Position) -> Self {
        Self { start, end }
    }

    pub fn start(&self) -> &Position {
        &self.start
    }

    pub fn end(&self) -> &Position {
        &self.end
    }

    pub fn start_line(&self) -> usize {
        self.start.line()
    }

    pub fn range(&self) -> Range<usize> {
        self.start.offset()..self.end.offset()
    }
}

#[derive(Debug, PartialEq, Eq)]
pub enum Section<'a, L = SourceSpan> {
    Prose(Located<ProseSection<L>, L>),
    Itemised(Located<ItemisedSection<L>, L>),
    PlanItems(Located<PlanItemSection<'a, L>, L>),
    Findings(Located<FindingsSection<L>, L>),
}

#[allow(dead_code)]
impl<'a, L> Section<'a, L> {
    pub fn name(&self) -> &str {
        match self {
            Self::Prose(section) => section.value.title.text(),
            Self::Itemised(section) => section.value.title.text(),
            Self::PlanItems(section) => section.value.title.text(),
            Self::Findings(section) => section.value.title.text(),
        }
    }

    pub fn title(&self) -> &Located<String, L> {
        match self {
            Self::Prose(section) => &section.value.title,
            Self::Itemised(section) => &section.value.title,
            Self::PlanItems(section) => &section.value.title,
            Self::Findings(section) => &section.value.title,
        }
    }

    pub fn body_span(&self) -> &L {
        match self {
            Self::Prose(section) => section.value.body.span(),
            Self::Itemised(section) => section.value.items.span(),
            Self::PlanItems(section) => section.value.items.span(),
            Self::Findings(section) => section.value.body.span(),
        }
    }

    pub fn span(&self) -> &L {
        match self {
            Self::Prose(section) => section.span(),
            Self::Itemised(section) => section.span(),
            Self::PlanItems(section) => section.span(),
            Self::Findings(section) => section.span(),
        }
    }

    pub fn as_prose(&self) -> Option<&ProseSection<L>> {
        match self {
            Self::Prose(section) => Some(section.value()),
            Self::Itemised(_) | Self::PlanItems(_) | Self::Findings(_) => None,
        }
    }

    pub fn as_itemised(&self) -> Option<&ItemisedSection<L>> {
        match self {
            Self::Prose(_) | Self::PlanItems(_) | Self::Findings(_) => None,
            Self::Itemised(section) => Some(section.value()),
        }
    }

    pub fn as_plan_items(&self) -> Option<&PlanItemSection<'a, L>> {
        match self {
            Self::PlanItems(section) => Some(section.value()),
            Self::Prose(_) | Self::Itemised(_) | Self::Findings(_) => None,
        }
    }

    pub fn as_plan_items_mut(&mut self) -> Option<&mut PlanItemSection<'a, L>> {
        match self {
            Self::PlanItems(section) => Some(section.value_mut()),
            Self::Prose(_) | Self::Itemised(_) | Self::Findings(_) => None,
        }
    }

    pub fn as_findings(&self) -> Option<&FindingsSection<L>> {
        match self {
            Self::Findings(section) => Some(section.value()),
            Self::Prose(_) | Self::Itemised(_) | Self::PlanItems(_) => None,
        }
    }
}

#[derive(Debug, PartialEq, Eq)]
pub struct ProseSection<L = SourceSpan> {
    title: Located<String, L>,
    body: Located<String, L>,
}

#[allow(dead_code)]
impl<L> ProseSection<L> {
    pub(crate) fn new(title: Located<String, L>, body: Located<String, L>) -> Self {
        Self { title, body }
    }

    pub fn name(&self) -> &str {
        self.title.text()
    }

    pub fn body(&self) -> &str {
        self.body.text()
    }

    pub fn title(&self) -> &Located<String, L> {
        &self.title
    }

    pub fn located_body(&self) -> &Located<String, L> {
        &self.body
    }
}

#[derive(Debug, PartialEq, Eq)]
pub struct ItemisedSection<L = SourceSpan> {
    pub(crate) title: Located<String, L>,
    pub(crate) items: Located<Vec<Item<L>>, L>,
}

#[allow(dead_code)]
impl<L> ItemisedSection<L> {
    pub fn items(&self) -> &[Item<L>] {
        self.items.value()
    }

    pub fn title(&self) -> &Located<String, L> {
        &self.title
    }

    pub fn located_items(&self) -> &Located<Vec<Item<L>>, L> {
        &self.items
    }
}

#[derive(Debug, PartialEq, Eq)]
pub struct PlanItemSection<'a, L = SourceSpan> {
    pub(crate) title: Located<String, L>,
    pub(crate) items: Located<Vec<PlanItem<'a, L>>, L>,
}

#[derive(Debug, PartialEq, Eq)]
pub struct FindingsSection<L = SourceSpan> {
    pub(crate) title: Located<String, L>,
    pub(crate) body: FindingsBody<L>,
}

#[allow(dead_code)]
impl FindingsSection {
    pub fn title(&self) -> &Located<String> {
        &self.title
    }

    pub fn body(&self) -> &FindingsBody {
        &self.body
    }
}

#[derive(Debug, PartialEq, Eq)]
pub enum FindingsBody<L = SourceSpan> {
    Sentinel(Located<String, L>),
    Items(Located<Vec<Finding<L>>, L>),
}

#[allow(dead_code)]
impl<L> FindingsBody<L> {
    pub fn span(&self) -> &L {
        match self {
            Self::Sentinel(body) => body.span(),
            Self::Items(items) => items.span(),
        }
    }

    pub fn as_sentinel(&self) -> Option<&str> {
        match self {
            Self::Sentinel(body) => Some(body.text()),
            Self::Items(_) => None,
        }
    }

    pub fn items(&self) -> Option<&[Finding<L>]> {
        match self {
            Self::Items(items) => Some(items.value()),
            Self::Sentinel(_) => None,
        }
    }
}

#[derive(Debug, PartialEq, Eq)]
pub struct Finding<L = SourceSpan> {
    pub(crate) expanded: Located<ExpandedItem<L>, L>,
}

#[allow(dead_code)]
impl Finding {
    pub fn identifier(&self) -> Option<&Located<String>> {
        self.expanded.value.identifier.as_ref()
    }

    pub fn content(&self) -> &Located<String> {
        &self.expanded.value.content
    }

    pub fn body(&self) -> &Located<String> {
        &self.expanded.value.body
    }

    pub fn span(&self) -> &SourceSpan {
        self.expanded.span()
    }
}

#[allow(dead_code)]
impl<'a, L> PlanItemSection<'a, L> {
    pub fn items(&self) -> &[PlanItem<'a, L>] {
        self.items.value()
    }

    pub fn items_mut(&mut self) -> &mut [PlanItem<'a, L>] {
        self.items.value_mut()
    }

    pub fn title(&self) -> &Located<String, L> {
        &self.title
    }

    pub fn located_items(&self) -> &Located<Vec<PlanItem<'a, L>>, L> {
        &self.items
    }
}

#[derive(Debug, PartialEq, Eq)]
pub struct PlanItem<'a, L = SourceSpan> {
    // Keep the ordinary expanded-item representation as the source of truth
    // for heading components and the complete record span. Plan items add only
    // a structured view over that item's body: leading metadata followed by
    // opaque prose.
    pub(crate) expanded: Located<ExpandedItem<L>, L>,
    pub(crate) metadata: Vec<Located<Metadata<'a, L>, L>>,
    pub(crate) prose: Located<String, L>,
}

#[allow(dead_code)]
impl<'a, L> PlanItem<'a, L> {
    pub fn identifier(&self) -> Option<&Located<String, L>> {
        self.expanded.value.identifier.as_ref()
    }

    pub fn title(&self) -> &Located<String, L> {
        &self.expanded.value.content
    }

    pub fn metadata(&self) -> &[Located<Metadata<'a, L>, L>] {
        &self.metadata
    }

    pub fn status_mut(&mut self) -> Option<&mut Metadata<'a, L>> {
        let mut matches = self
            .metadata
            .iter_mut()
            .filter(|entry| entry.value().key() == "Status");
        let status = matches.next()?;
        matches.next().is_none().then_some(status.value_mut())
    }

    pub fn prose(&self) -> &Located<String, L> {
        &self.prose
    }

    pub fn span(&self) -> &L {
        self.expanded.span()
    }
}

#[derive(Debug, PartialEq, Eq)]
pub enum Item<L = SourceSpan> {
    Compact(Located<CompactItem<L>, L>),
    Expanded(Located<ExpandedItem<L>, L>),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ItemForm {
    Compact,
    Expanded,
}

#[allow(dead_code)]
impl<L> Item<L> {
    pub fn form(&self) -> ItemForm {
        match self {
            Self::Compact(_) => ItemForm::Compact,
            Self::Expanded(_) => ItemForm::Expanded,
        }
    }

    pub fn identifier(&self) -> Option<&Located<String, L>> {
        match self {
            Self::Compact(item) => item.value.identifier.as_ref(),
            Self::Expanded(item) => item.value.identifier.as_ref(),
        }
    }

    pub fn content(&self) -> &Located<String, L> {
        match self {
            Self::Compact(item) => &item.value.content,
            Self::Expanded(item) => &item.value.content,
        }
    }

    pub fn body(&self) -> &Located<String, L> {
        match self {
            Self::Compact(item) => &item.value.body,
            Self::Expanded(item) => &item.value.body,
        }
    }

    pub fn span(&self) -> &L {
        match self {
            Self::Compact(item) => item.span(),
            Self::Expanded(item) => item.span(),
        }
    }

    pub fn as_compact(&self) -> Option<&CompactItem<L>> {
        match self {
            Self::Compact(item) => Some(item.value()),
            Self::Expanded(_) => None,
        }
    }

    pub fn as_expanded(&self) -> Option<&ExpandedItem<L>> {
        match self {
            Self::Compact(_) => None,
            Self::Expanded(item) => Some(item.value()),
        }
    }
}

#[derive(Debug, PartialEq, Eq)]
pub struct CompactItem<L = SourceSpan> {
    pub(crate) marker: Located<String, L>,
    pub(crate) identifier: Option<Located<String, L>>,
    pub(crate) delimiter: Option<Located<String, L>>,
    pub(crate) content: Located<String, L>,
    pub(crate) body: Located<String, L>,
}

#[allow(dead_code)]
impl<L> CompactItem<L> {
    pub fn marker(&self) -> &Located<String, L> {
        &self.marker
    }

    pub fn delimiter(&self) -> Option<&Located<String, L>> {
        self.delimiter.as_ref()
    }

    pub fn body(&self) -> &Located<String, L> {
        &self.body
    }
}

#[derive(Debug, PartialEq, Eq)]
pub struct ExpandedItem<L = SourceSpan> {
    pub(crate) marker: Located<String, L>,
    pub(crate) identifier: Option<Located<String, L>>,
    pub(crate) delimiter: Option<Located<String, L>>,
    pub(crate) content: Located<String, L>,
    pub(crate) body: Located<String, L>,
}

#[allow(dead_code)]
impl<L> ExpandedItem<L> {
    pub fn marker(&self) -> &Located<String, L> {
        &self.marker
    }

    pub fn delimiter(&self) -> Option<&Located<String, L>> {
        self.delimiter.as_ref()
    }

    pub fn body(&self) -> &Located<String, L> {
        &self.body
    }
}

#[derive(Debug, PartialEq, Eq)]
pub struct Located<T, L = SourceSpan> {
    value: T,
    span: L,
}

#[allow(dead_code)]
impl<T, L> Located<T, L> {
    pub(crate) fn new(value: T, span: L) -> Self {
        Self { value, span }
    }

    pub fn value(&self) -> &T {
        &self.value
    }

    pub fn value_mut(&mut self) -> &mut T {
        &mut self.value
    }

    pub fn span(&self) -> &L {
        &self.span
    }

    pub fn map<U, F: FnOnce(T) -> U>(self, f: F) -> Located<U, L> {
        Located::new(f(self.value), self.span)
    }
}

#[allow(dead_code)]
impl<L> Located<String, L> {
    pub fn text(&self) -> &str {
        &self.value
    }
}

impl<'a, L> Located<&'a str, L> {
    pub fn text(&self) -> &'a str {
        self.value
    }
}

/// Serialize an artifact while retaining all source text not explicitly
/// changed through the structured API.
pub fn serialize(artifact: &Artifact) -> String {
    let mut entries: Vec<_> = artifact.metadata.iter().collect();
    entries.extend(
        artifact
            .sections
            .iter()
            .filter_map(Section::as_plan_items)
            .flat_map(PlanItemSection::items)
            .flat_map(|item| item.metadata.iter()),
    );
    entries.sort_by_key(|entry| entry.value().located_value().span().range().start);

    let capacity = entries
        .iter()
        .fold(artifact.source.len(), |capacity, entry| {
            let metadata = entry.value();
            capacity - metadata.value.span().range().len() + metadata.value().len()
        });
    let mut output = String::with_capacity(capacity);
    let mut copied_until = 0;

    for entry in entries {
        let metadata = entry.value();
        let value_range = metadata.value.span().range();
        let unchanged = &artifact.source[copied_until..value_range.start];
        output.push_str(unchanged);
        output.push_str(metadata.value());
        copied_until = value_range.end;
    }
    output.push_str(&artifact.source[copied_until..]);
    output
}

impl fmt::Display for Artifact<'_> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&serialize(self))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_multiline_metadata_replacements() {
        let mut metadata: Metadata<'_, ()> = Metadata {
            key: Located::new(
                "Status",
                (),
            ),
            value: Located::new(
                "proposed".into(),
                (),
            ),
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
            key: Located::new(
                "Status",
                (),
            ),
            value: Located::new(
                "proposed\n  continued".into(),
                (),
            ),
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
        let config = crate::parser::ParserConfig::new(vec![]).with_known_metadata(["Status"]);
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
        let config = crate::parser::ParserConfig::new(vec![]).with_known_metadata(["Status"]);
        let artifact =
            crate::parser::parse_with_config(source, &config).expect("artifact should parse");
        let prose = artifact.located_pre_section_prose();

        assert_eq!(prose.text(), "");
        assert_eq!(prose.span().range(), 29..29);
        assert_eq!(prose.span().start_line(), 3);
    }
}
