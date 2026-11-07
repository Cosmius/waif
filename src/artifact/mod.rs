use std::fmt;
use std::ops::Range;

use crate::parser::Position;

//region artifact structure

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

    pub fn get_metadata(&self, key: &str) -> Option<&Metadata<'a, L>> {
        let mut entries = self
            .metadata
            .iter()
            .filter(|entry| entry.value().key() == key);
        let entry = entries.next()?;
        entries.next().is_none().then_some(entry.value())
    }

    pub fn pre_section_prose(&self) -> &'a str {
        self.pre_section_prose.text()
    }

    pub fn located_title(&self) -> &Located<&'a str, L> {
        &self.title
    }

    pub fn located_pre_section_prose(&self) -> &Located<&'a str, L> {
        &self.pre_section_prose
    }

    pub fn sections(&self) -> &[Section<'a, L>] {
        &self.sections
    }

    /// Return the expanded item with `identifier` in the itemised section
    /// titled `section`, or `None` when it is missing or not unique.
    pub fn expanded_item(&self, section: &str, identifier: &str) -> Option<&ExpandedItem<'a, L>> {
        let mut matches = self
            .sections
            .iter()
            .filter(|candidate| candidate.title() == section)
            .filter_map(Section::as_itemised)
            .flat_map(ItemisedSection::items)
            .filter_map(Item::as_expanded)
            .filter(|item| {
                item.identifier
                    .as_ref()
                    .is_some_and(|id| id.text() == identifier)
            });
        let item = matches.next()?;
        matches.next().is_none().then_some(item)
    }

    /// Mutable form of [`Artifact::expanded_item`].
    pub fn expanded_item_mut(
        &mut self,
        section: &str,
        identifier: &str,
    ) -> Option<&mut ExpandedItem<'a, L>> {
        let mut matches = self
            .sections
            .iter_mut()
            .filter(|candidate| candidate.title() == section)
            .filter_map(Section::as_itemised_mut)
            .flat_map(ItemisedSection::items_mut)
            .filter_map(Item::as_expanded_mut)
            .filter(|item| {
                item.identifier
                    .as_ref()
                    .is_some_and(|id| id.text() == identifier)
            });
        let item = matches.next()?;
        matches.next().is_none().then_some(item)
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
}

impl<'a, L> Section<'a, L> {
    pub fn title(&self) -> &'a str {
        self.located_title().text()
    }

    pub fn located_title(&self) -> &Located<&'a str, L> {
        match self {
            Self::Prose(section) => &section.value.title,
            Self::Itemised(section) => &section.value.title,
        }
    }

    pub fn span(&self) -> &L {
        match self {
            Self::Prose(section) => section.span(),
            Self::Itemised(section) => section.span(),
        }
    }

    pub fn body_span(&self) -> &L {
        match self {
            Self::Prose(section) => section.value.body.span(),
            Self::Itemised(section) => section.value.body.span(),
        }
    }

    pub fn as_prose(&self) -> Option<&ProseSection<'a, L>> {
        match self {
            Self::Prose(section) => Some(section.value()),
            Self::Itemised(_) => None,
        }
    }

    pub fn as_itemised(&self) -> Option<&ItemisedSection<'a, L>> {
        match self {
            Self::Prose(_) => None,
            Self::Itemised(section) => Some(section.value()),
        }
    }

    pub fn as_itemised_mut(&mut self) -> Option<&mut ItemisedSection<'a, L>> {
        match self {
            Self::Prose(_) => None,
            Self::Itemised(section) => Some(section.value_mut()),
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
    pub(crate) body: Located<ItemisedBody<'a, L>, L>,
}

impl<'a, L> ItemisedSection<'a, L> {
    /// The section's items; empty when the body is the sentinel.
    pub fn items(&self) -> &[Item<'a, L>] {
        match self.body.value() {
            ItemisedBody::Sentinel(_) => &[],
            ItemisedBody::Items(items) => items,
        }
    }

    pub fn items_mut(&mut self) -> &mut [Item<'a, L>] {
        match self.body.value_mut() {
            ItemisedBody::Sentinel(_) => &mut [],
            ItemisedBody::Items(items) => items,
        }
    }

    pub fn sentinel(&self) -> Option<&Located<&'a str, L>> {
        match self.body.value() {
            ItemisedBody::Sentinel(sentinel) => Some(sentinel),
            ItemisedBody::Items(_) => None,
        }
    }
}

/// An itemised section body: either the section's declared sentinel alone or
/// its items.
#[derive(Debug, PartialEq, Eq)]
pub enum ItemisedBody<'a, L = SourceSpan> {
    Sentinel(Located<&'a str, L>),
    Items(Vec<Item<'a, L>>),
}

#[derive(Debug, PartialEq, Eq)]
pub enum Item<'a, L = SourceSpan> {
    Compact(Located<CompactItem<'a, L>, L>),
    Expanded(Located<ExpandedItem<'a, L>, L>),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
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

    pub fn as_expanded_mut(&mut self) -> Option<&mut ExpandedItem<'a, L>> {
        match self {
            Self::Compact(_) => None,
            Self::Expanded(item) => Some(item.value_mut()),
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

    pub fn identifier(&self) -> Option<&Located<&'a str, L>> {
        self.identifier.as_ref()
    }
}

#[derive(Debug, PartialEq, Eq)]
pub struct ExpandedItem<'a, L = SourceSpan> {
    pub(crate) marker: Located<&'a str, L>,
    pub(crate) identifier: Option<Located<&'a str, L>>,
    pub(crate) title: Located<&'a str, L>,
    pub(crate) delimiter: Option<Located<&'a str, L>>,
    pub(crate) metadata: Vec<Located<Metadata<'a, L>, L>>,
    pub(crate) body: Located<&'a str, L>,
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

    /// The leading `- Key: value` entries after the heading.
    pub fn metadata(&self) -> &[Located<Metadata<'a, L>, L>] {
        &self.metadata
    }

    /// Return the entry for `key`, or `None` when it is missing or repeated.
    pub fn metadatum_mut(&mut self, key: &str) -> Option<&mut Metadata<'a, L>> {
        let mut matches = self
            .metadata
            .iter_mut()
            .filter(|entry| entry.value().key() == key);
        let entry = matches.next()?;
        matches.next().is_none().then_some(entry.value_mut())
    }

    /// The opaque text after the leading metadata.
    pub fn body(&self) -> &Located<&'a str, L> {
        &self.body
    }
}

//endregion artifact structure

//region source location

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

//endregion source location

/// Serialize an artifact while retaining all source text not explicitly
/// changed through the structured API.
pub fn serialize(artifact: &Artifact<'_, SourceSpan>) -> String {
    let mut entries: Vec<_> = artifact.metadata.iter().collect();
    entries.extend(
        artifact
            .sections
            .iter()
            .filter_map(Section::as_itemised)
            .flat_map(ItemisedSection::items)
            .filter_map(Item::as_expanded)
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

impl fmt::Display for Artifact<'_, SourceSpan> {
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
