use std::fmt;
use std::ops::Range;

use crate::parser::Position;

#[derive(Debug, PartialEq, Eq)]
pub struct Artifact<'a, L = SourceSpan> {
    source: &'a str,
    title: Located<&'a str, L>,
    metadata: Vec<Located<Metadata<'a, L>, L>>,
    pre_section_prose: Located<&'a str, L>,
    sections: Vec<Section<'a, L>>,
}

impl<'a, L> Artifact<'a, L> {
    pub(crate) fn new(
        source: &'a str,
        title: Located<&'a str, L>,
        metadata: Vec<Located<Metadata<'a, L>, L>>,
        pre_section_prose: Located<&'a str, L>,
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
    pub fn source(&self) -> &'a str {
        self.source
    }

    pub fn title(&self) -> &'a str {
        self.title.text()
    }

    pub fn metadata(&self) -> &[Located<Metadata<'a, L>, L>] {
        &self.metadata
    }

    pub fn metadata_mut(&mut self) -> &mut [Located<Metadata<'a, L>, L>] {
        &mut self.metadata
    }

    pub fn pre_section_prose(&self) -> &'a str {
        self.pre_section_prose.text()
    }

    pub fn located_pre_section_prose(&self) -> &Located<&'a str, L> {
        &self.pre_section_prose
    }

    pub fn sections(&self) -> &[Section<'a, L>] {
        &self.sections
    }

    pub fn plan_item(&self, identifier: &str) -> Option<&PlanItem<'a, L>> {
        let mut matches = self
            .sections
            .iter()
            .filter_map(Section::as_plan_items)
            .flat_map(|section| {
                section.items().iter().filter(|item| {
                    item.value()
                        .identifier()
                        .is_some_and(|id| id.text() == identifier)
                })
            });
        let item = matches.next()?;
        matches.next().is_none().then_some(item.value())
    }

    pub fn plan_item_mut(&mut self, identifier: &str) -> Option<&mut PlanItem<'a, L>> {
        let mut matches = self
            .sections
            .iter_mut()
            .filter_map(Section::as_plan_items_mut)
            .flat_map(|section| {
                section.items_mut().iter_mut().filter(|item| {
                    item.value()
                        .identifier()
                        .is_some_and(|id| id.text() == identifier)
                })
            });
        let item = matches.next()?;
        matches.next().is_none().then_some(item.value_mut())
    }
}

#[derive(Debug, PartialEq, Eq)]
pub struct Metadata<'a, L = SourceSpan> {
    key: Located<&'a str, L>,
    value: Located<String, L>,
}

impl<'a, L> Metadata<'a, L> {
    pub(crate) fn new(key: Located<&'a str, L>, value: Located<String, L>) -> Self {
        Self { key, value }
    }

    pub fn key(&self) -> &'a str {
        self.key.text()
    }

    pub fn value(&self) -> &str {
        self.value.value()
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
pub enum Section<'a, L = SourceSpan> {
    Prose(Located<ProseSection<'a, L>, L>),
    Itemised(Located<ItemisedSection<'a, L>, L>),
    PlanItems(Located<PlanItemsSection<'a, L>, L>),
    Findings(Located<FindingsSection<'a, L>, L>),
}

impl<'a, L> Section<'a, L> {
    pub fn title(&self) -> &'a str {
        self.located_title().text()
    }

    pub fn located_title(&self) -> &Located<&'a str, L> {
        match self {
            Self::Prose(section) => &section.value.title,
            Self::Itemised(section) => &section.value.title,
            Self::PlanItems(section) => &section.value.title,
            Self::Findings(section) => &section.value.title,
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

    pub fn body_span(&self) -> &L {
        match self {
            Self::Prose(section) => section.value.body.span(),
            Self::Itemised(section) => section.value.items.span(),
            Self::PlanItems(section) => section.value.items.span(),
            Self::Findings(section) => section.value.body.span(),
        }
    }

    pub fn as_prose(&self) -> Option<&ProseSection<'a, L>> {
        match self {
            Self::Prose(section) => Some(section.value()),
            Self::Itemised(_) | Self::PlanItems(_) | Self::Findings(_) => None,
        }
    }

    pub fn as_itemised(&self) -> Option<&ItemisedSection<'a, L>> {
        match self {
            Self::Prose(_) | Self::PlanItems(_) | Self::Findings(_) => None,
            Self::Itemised(section) => Some(section.value()),
        }
    }

    pub fn as_plan_items(&self) -> Option<&PlanItemsSection<'a, L>> {
        match self {
            Self::PlanItems(section) => Some(section.value()),
            Self::Prose(_) | Self::Itemised(_) | Self::Findings(_) => None,
        }
    }

    pub fn as_plan_items_mut(&mut self) -> Option<&mut PlanItemsSection<'a, L>> {
        match self {
            Self::PlanItems(section) => Some(section.value_mut()),
            Self::Prose(_) | Self::Itemised(_) | Self::Findings(_) => None,
        }
    }

    pub fn as_findings(&self) -> Option<&FindingsSection<'a, L>> {
        match self {
            Self::Findings(section) => Some(section.value()),
            Self::Prose(_) | Self::Itemised(_) | Self::PlanItems(_) => None,
        }
    }
}

#[derive(Debug, PartialEq, Eq)]
pub struct ProseSection<'a, L = SourceSpan> {
    title: Located<&'a str, L>,
    body: Located<&'a str, L>,
}

impl<'a, L> ProseSection<'a, L> {
    pub(crate) fn new(title: Located<&'a str, L>, body: Located<&'a str, L>) -> Self {
        Self { title, body }
    }

    pub fn body(&self) -> &'a str {
        self.body.text()
    }
}

#[derive(Debug, PartialEq, Eq)]
pub struct ItemisedSection<'a, L = SourceSpan> {
    pub(crate) title: Located<&'a str, L>,
    pub(crate) items: Located<Vec<Item<'a, L>>, L>,
}

impl<'a, L> ItemisedSection<'a, L> {
    pub fn items(&self) -> &[Item<'a, L>] {
        self.items.value()
    }
}

#[derive(Debug, PartialEq, Eq)]
pub enum Item<'a, L = SourceSpan> {
    Compact(Located<CompactItem<'a, L>, L>),
    Expanded(Located<ExpandedItem<'a, L>, L>),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ItemForm {
    Compact,
    Expanded,
}

impl<'a, L> Item<'a, L> {
    pub fn form(&self) -> ItemForm {
        match self {
            Self::Compact(_) => ItemForm::Compact,
            Self::Expanded(_) => ItemForm::Expanded,
        }
    }

    pub fn identifier(&self) -> Option<&Located<&'a str, L>> {
        match self {
            Self::Compact(item) => item.value.identifier.as_ref(),
            Self::Expanded(item) => item.value.identifier.as_ref(),
        }
    }

    pub fn short_description(&self) -> &Located<&'a str, L> {
        match self {
            Self::Compact(item) => &item.value.content,
            Self::Expanded(item) => &item.value.title,
        }
    }

    pub fn span(&self) -> &L {
        match self {
            Self::Compact(item) => item.span(),
            Self::Expanded(item) => item.span(),
        }
    }

    pub fn as_compact(&self) -> Option<&CompactItem<'a, L>> {
        match self {
            Self::Compact(item) => Some(item.value()),
            Self::Expanded(_) => None,
        }
    }

    pub fn as_expanded(&self) -> Option<&ExpandedItem<'a, L>> {
        match self {
            Self::Compact(_) => None,
            Self::Expanded(item) => Some(item.value()),
        }
    }
}

#[derive(Debug, PartialEq, Eq)]
pub struct CompactItem<'a, L = SourceSpan> {
    pub(crate) marker: Located<&'a str, L>,
    pub(crate) identifier: Option<Located<&'a str, L>>,
    pub(crate) delimiter: Option<Located<&'a str, L>>,
    pub(crate) content: Located<&'a str, L>,
}

impl<'a, L> CompactItem<'a, L> {
    pub fn marker(&self) -> &Located<&'a str, L> {
        &self.marker
    }

    pub fn delimiter(&self) -> Option<&Located<&'a str, L>> {
        self.delimiter.as_ref()
    }
}

#[derive(Debug, PartialEq, Eq)]
pub struct ExpandedItem<'a, L = SourceSpan> {
    pub(crate) marker: Located<&'a str, L>,
    pub(crate) identifier: Option<Located<&'a str, L>>,
    pub(crate) title: Located<&'a str, L>,
    pub(crate) delimiter: Option<Located<&'a str, L>>,
    pub(crate) content: Located<&'a str, L>,
}

impl<'a, L> ExpandedItem<'a, L> {
    pub fn marker(&self) -> &Located<&'a str, L> {
        &self.marker
    }

    pub fn delimiter(&self) -> Option<&Located<&'a str, L>> {
        self.delimiter.as_ref()
    }

    pub fn title(&self) -> &Located<&'a str, L> {
        &self.title
    }

    pub fn content(&self) -> &Located<&'a str, L> {
        &self.content
    }
}

#[derive(Debug, PartialEq, Eq)]
pub struct PlanItemsSection<'a, L = SourceSpan> {
    pub(crate) title: Located<&'a str, L>,
    pub(crate) items: Located<Vec<Located<PlanItem<'a, L>, L>>, L>,
}

impl<'a, L> PlanItemsSection<'a, L> {
    pub fn items(&self) -> &[Located<PlanItem<'a, L>, L>] {
        self.items.value()
    }

    pub fn items_mut(&mut self) -> &mut [Located<PlanItem<'a, L>, L>] {
        self.items.value_mut()
    }
}

#[derive(Debug, PartialEq, Eq)]
pub struct PlanItem<'a, L = SourceSpan> {
    // Keep the ordinary expanded-item representation as the source of truth
    // for heading components and the complete record span. Plan items add only
    // a structured view over that item's body: leading metadata followed by
    // opaque prose.
    pub(crate) item: Located<ExpandedItem<'a, L>, L>,
    pub(crate) metadata: Vec<Located<Metadata<'a, L>, L>>,
    pub(crate) prose: Located<&'a str, L>,
}

impl<'a, L> PlanItem<'a, L> {
    pub fn identifier(&self) -> Option<&Located<&'a str, L>> {
        self.item.value.identifier.as_ref()
    }

    pub fn title(&self) -> &Located<&'a str, L> {
        &self.item.value.title
    }

    pub fn metadata(&self) -> &[Located<Metadata<'a, L>, L>] {
        &self.metadata
    }

    pub fn metadatum_mut(&mut self, key: &str) -> Option<&mut Metadata<'a, L>> {
        let mut matches = self
            .metadata
            .iter_mut()
            .filter(|entry| entry.value().key() == key);
        let status = matches.next()?;
        matches.next().is_none().then_some(status.value_mut())
    }

    pub fn prose(&self) -> &Located<&'a str, L> {
        &self.prose
    }

    pub fn span(&self) -> &L {
        self.item.span()
    }
}

#[derive(Debug, PartialEq, Eq)]
pub struct FindingsSection<'a, L = SourceSpan> {
    pub(crate) title: Located<&'a str, L>,
    pub(crate) body: FindingsBody<'a, L>,
}

impl<'a, L> FindingsSection<'a, L> {
    pub fn body(&self) -> &FindingsBody<'a, L> {
        &self.body
    }
}

#[derive(Debug, PartialEq, Eq)]
pub enum FindingsBody<'a, L = SourceSpan> {
    Sentinel(Located<&'a str, L>),
    Items(Located<Vec<Finding<'a, L>>, L>),
}

impl<'a, L> FindingsBody<'a, L> {
    pub fn span(&self) -> &L {
        match self {
            Self::Sentinel(body) => body.span(),
            Self::Items(items) => items.span(),
        }
    }

    pub fn as_sentinel(&self) -> Option<&'a str> {
        match self {
            Self::Sentinel(body) => Some(body.text()),
            Self::Items(_) => None,
        }
    }

    pub fn items(&self) -> Option<&[Finding<'a, L>]> {
        match self {
            Self::Items(items) => Some(items.value()),
            Self::Sentinel(_) => None,
        }
    }
}

#[derive(Debug, PartialEq, Eq)]
pub struct Finding<'a, L = SourceSpan> {
    pub(crate) item: Located<ExpandedItem<'a, L>, L>,
}

impl<'a, L> Finding<'a, L> {
    pub fn identifier(&self) -> Option<&Located<&'a str, L>> {
        self.item.value.identifier.as_ref()
    }

    pub fn content(&self) -> &Located<&'a str, L> {
        &self.item.value.content
    }

    pub fn span(&self) -> &L {
        self.item.span()
    }
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct Located<T, L = SourceSpan> {
    value: T,
    span: L,
}

impl<T, L> Located<T, L> {
    pub(crate) fn new(value: T, span: L) -> Self {
        Self { value, span }
    }

    pub fn unpack(self) -> (T, L) {
        (self.value, self.span)
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

impl<'a, L> Located<&'a str, L> {
    pub fn text(&self) -> &'a str {
        self.value
    }
}

/// A half-open UTF-8 byte range and its one-based starting line.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SourceSpan {
    start: Position,
    end: Position,
}

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

/// Serialize an artifact while retaining all source text not explicitly
/// changed through the structured API.
pub fn serialize(artifact: &Artifact) -> String {
    let mut entries: Vec<_> = artifact.metadata.iter().collect();
    entries.extend(
        artifact
            .sections
            .iter()
            .filter_map(Section::as_plan_items)
            .flat_map(PlanItemsSection::items)
            .flat_map(|item| item.value.metadata.iter()),
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

#[cfg(test)]
mod tests;
