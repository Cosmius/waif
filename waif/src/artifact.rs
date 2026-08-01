use std::fmt;
use std::ops::Range;

#[derive(Debug, PartialEq, Eq)]
pub struct Artifact {
    source: String,
    title: String,
    metadata: Vec<Located<Metadata>>,
    pre_section_prose: Located<String>,
    sections: Vec<Section>,
}

// This is the structured artifact API. The check command currently only needs
// parsing to succeed, while later commands will consume these accessors.
#[allow(dead_code)]
impl Artifact {
    pub(crate) fn new(
        source: String,
        title: String,
        metadata: Vec<Located<Metadata>>,
        pre_section_prose: Located<String>,
        sections: Vec<Section>,
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

    pub fn metadata(&self) -> &[Located<Metadata>] {
        &self.metadata
    }

    /// Existing metadata entries, mutably borrowed in source order.
    ///
    /// The slice cannot add, remove, or reorder entries. Use
    /// [`Metadata::set_value`] to change an existing value. Keys and ordering
    /// are controlled by the applicable artifact contract.
    pub fn metadata_mut(&mut self) -> &mut [Located<Metadata>] {
        &mut self.metadata
    }

    /// Opaque source between structured metadata and the first section.
    pub fn pre_section_prose(&self) -> &str {
        self.pre_section_prose.text()
    }

    pub fn located_pre_section_prose(&self) -> &Located<String> {
        &self.pre_section_prose
    }

    pub fn sections(&self) -> &[Section] {
        &self.sections
    }

    pub fn plan_item(&self, identifier: &str) -> Option<&PlanItem> {
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

    pub fn plan_item_mut(&mut self, identifier: &str) -> Option<&mut PlanItem> {
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

    /// Serialize this artifact, preserving untouched source text exactly.
    pub fn serialize(&self) -> String {
        serialize(self)
    }
}

#[derive(Debug, PartialEq, Eq)]
pub struct Metadata {
    key: String,
    value: Located<String>,
}

#[allow(dead_code)]
impl Metadata {
    pub(crate) fn new(key: String, value: Located<String>) -> Self {
        Self { key, value }
    }

    pub fn key(&self) -> &str {
        &self.key
    }

    pub fn value(&self) -> &str {
        self.value.text()
    }

    pub fn located_value(&self) -> &Located<String> {
        &self.value
    }

    /// Replace this entry's value for the next serialization.
    ///
    /// Metadata values are single-line values. The surrounding source text,
    /// including the separator and line ending, remains unchanged.
    pub fn set_value(&mut self, value: impl Into<String>) -> Result<(), InvalidValue> {
        let value = value.into();
        if value.contains(['\r', '\n']) {
            return Err(InvalidValue);
        }
        self.value.value = value;
        Ok(())
    }
}

#[derive(Debug, PartialEq, Eq)]
pub struct InvalidValue;

impl fmt::Display for InvalidValue {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "metadata values must be single-line")
    }
}

impl std::error::Error for InvalidValue {}

/// A half-open UTF-8 byte range and its one-based starting line.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SourceSpan {
    start_line: usize,
    range: Range<usize>,
}

#[allow(dead_code)]
impl SourceSpan {
    pub(crate) fn new(start_line: usize, range: Range<usize>) -> Self {
        Self { start_line, range }
    }

    pub fn start_line(&self) -> usize {
        self.start_line
    }

    pub fn range(&self) -> Range<usize> {
        self.range.clone()
    }
}

#[derive(Debug, PartialEq, Eq)]
pub enum Section {
    Prose(Located<ProseSection>),
    Itemised(Located<ItemisedSection>),
    PlanItems(Located<PlanItemSection>),
}

#[allow(dead_code)]
impl Section {
    pub fn name(&self) -> &str {
        match self {
            Self::Prose(section) => section.value.title.text(),
            Self::Itemised(section) => section.value.title.text(),
            Self::PlanItems(section) => section.value.title.text(),
        }
    }

    pub fn title(&self) -> &Located<String> {
        match self {
            Self::Prose(section) => &section.value.title,
            Self::Itemised(section) => &section.value.title,
            Self::PlanItems(section) => &section.value.title,
        }
    }

    pub fn body_span(&self) -> &SourceSpan {
        match self {
            Self::Prose(section) => section.value.body.span(),
            Self::Itemised(section) => section.value.items.span(),
            Self::PlanItems(section) => section.value.items.span(),
        }
    }

    pub fn span(&self) -> &SourceSpan {
        match self {
            Self::Prose(section) => section.span(),
            Self::Itemised(section) => section.span(),
            Self::PlanItems(section) => section.span(),
        }
    }

    pub fn as_prose(&self) -> Option<&ProseSection> {
        match self {
            Self::Prose(section) => Some(section.value()),
            Self::Itemised(_) | Self::PlanItems(_) => None,
        }
    }

    pub fn as_itemised(&self) -> Option<&ItemisedSection> {
        match self {
            Self::Prose(_) | Self::PlanItems(_) => None,
            Self::Itemised(section) => Some(section.value()),
        }
    }

    pub fn as_plan_items(&self) -> Option<&PlanItemSection> {
        match self {
            Self::PlanItems(section) => Some(section.value()),
            Self::Prose(_) | Self::Itemised(_) => None,
        }
    }

    pub fn as_plan_items_mut(&mut self) -> Option<&mut PlanItemSection> {
        match self {
            Self::PlanItems(section) => Some(section.value_mut()),
            Self::Prose(_) | Self::Itemised(_) => None,
        }
    }
}

#[derive(Debug, PartialEq, Eq)]
pub struct ProseSection {
    title: Located<String>,
    body: Located<String>,
}

#[allow(dead_code)]
impl ProseSection {
    pub(crate) fn new(title: Located<String>, body: Located<String>) -> Self {
        Self { title, body }
    }

    pub fn name(&self) -> &str {
        self.title.text()
    }

    pub fn body(&self) -> &str {
        self.body.text()
    }

    pub fn title(&self) -> &Located<String> {
        &self.title
    }

    pub fn located_body(&self) -> &Located<String> {
        &self.body
    }
}

#[derive(Debug, PartialEq, Eq)]
pub struct ItemisedSection {
    pub(crate) title: Located<String>,
    pub(crate) items: Located<Vec<Item>>,
}

#[allow(dead_code)]
impl ItemisedSection {
    pub fn items(&self) -> &[Item] {
        self.items.value()
    }

    pub fn title(&self) -> &Located<String> {
        &self.title
    }

    pub fn located_items(&self) -> &Located<Vec<Item>> {
        &self.items
    }
}

#[derive(Debug, PartialEq, Eq)]
pub struct PlanItemSection {
    pub(crate) title: Located<String>,
    pub(crate) items: Located<Vec<PlanItem>>,
}

#[allow(dead_code)]
impl PlanItemSection {
    pub fn items(&self) -> &[PlanItem] {
        self.items.value()
    }

    pub fn items_mut(&mut self) -> &mut [PlanItem] {
        self.items.value_mut()
    }

    pub fn title(&self) -> &Located<String> {
        &self.title
    }

    pub fn located_items(&self) -> &Located<Vec<PlanItem>> {
        &self.items
    }
}

#[derive(Debug, PartialEq, Eq)]
pub struct PlanItem {
    // Keep the ordinary expanded-item representation as the source of truth
    // for heading components and the complete record span. Plan items add only
    // a structured view over that item's body: leading metadata followed by
    // opaque prose.
    pub(crate) expanded: Located<ExpandedItem>,
    pub(crate) metadata: Vec<Located<Metadata>>,
    pub(crate) prose: Located<String>,
}

#[allow(dead_code)]
impl PlanItem {
    pub fn identifier(&self) -> Option<&Located<String>> {
        self.expanded.value.identifier.as_ref()
    }

    pub fn title(&self) -> &Located<String> {
        &self.expanded.value.content
    }

    pub fn metadata(&self) -> &[Located<Metadata>] {
        &self.metadata
    }

    pub fn status_mut(&mut self) -> Option<&mut Metadata> {
        let mut matches = self
            .metadata
            .iter_mut()
            .filter(|entry| entry.value().key() == "Status");
        let status = matches.next()?;
        matches.next().is_none().then_some(status.value_mut())
    }

    pub fn prose(&self) -> &Located<String> {
        &self.prose
    }

    pub fn span(&self) -> &SourceSpan {
        self.expanded.span()
    }
}

#[derive(Debug, PartialEq, Eq)]
pub enum Item {
    Compact(Located<CompactItem>),
    Expanded(Located<ExpandedItem>),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ItemForm {
    Compact,
    Expanded,
}

#[allow(dead_code)]
impl Item {
    pub fn form(&self) -> ItemForm {
        match self {
            Self::Compact(_) => ItemForm::Compact,
            Self::Expanded(_) => ItemForm::Expanded,
        }
    }

    pub fn identifier(&self) -> Option<&Located<String>> {
        match self {
            Self::Compact(item) => item.value.identifier.as_ref(),
            Self::Expanded(item) => item.value.identifier.as_ref(),
        }
    }

    pub fn content(&self) -> &Located<String> {
        match self {
            Self::Compact(item) => &item.value.content,
            Self::Expanded(item) => &item.value.content,
        }
    }

    pub fn body(&self) -> &Located<String> {
        match self {
            Self::Compact(item) => &item.value.body,
            Self::Expanded(item) => &item.value.body,
        }
    }

    pub fn span(&self) -> &SourceSpan {
        match self {
            Self::Compact(item) => item.span(),
            Self::Expanded(item) => item.span(),
        }
    }

    pub fn as_compact(&self) -> Option<&CompactItem> {
        match self {
            Self::Compact(item) => Some(item.value()),
            Self::Expanded(_) => None,
        }
    }

    pub fn as_expanded(&self) -> Option<&ExpandedItem> {
        match self {
            Self::Compact(_) => None,
            Self::Expanded(item) => Some(item.value()),
        }
    }
}

#[derive(Debug, PartialEq, Eq)]
pub struct CompactItem {
    pub(crate) marker: Located<String>,
    pub(crate) identifier: Option<Located<String>>,
    pub(crate) delimiter: Option<Located<String>>,
    pub(crate) content: Located<String>,
    pub(crate) body: Located<String>,
}

#[allow(dead_code)]
impl CompactItem {
    pub fn marker(&self) -> &Located<String> {
        &self.marker
    }

    pub fn delimiter(&self) -> Option<&Located<String>> {
        self.delimiter.as_ref()
    }

    pub fn body(&self) -> &Located<String> {
        &self.body
    }
}

#[derive(Debug, PartialEq, Eq)]
pub struct ExpandedItem {
    pub(crate) marker: Located<String>,
    pub(crate) identifier: Option<Located<String>>,
    pub(crate) delimiter: Option<Located<String>>,
    pub(crate) content: Located<String>,
    pub(crate) body: Located<String>,
}

#[allow(dead_code)]
impl ExpandedItem {
    pub fn marker(&self) -> &Located<String> {
        &self.marker
    }

    pub fn delimiter(&self) -> Option<&Located<String>> {
        self.delimiter.as_ref()
    }

    pub fn body(&self) -> &Located<String> {
        &self.body
    }
}

#[derive(Debug, PartialEq, Eq)]
pub struct Located<T> {
    value: T,
    span: SourceSpan,
}

#[allow(dead_code)]
impl<T> Located<T> {
    pub(crate) fn new(value: T, span: SourceSpan) -> Self {
        Self { value, span }
    }

    pub fn value(&self) -> &T {
        &self.value
    }

    pub fn value_mut(&mut self) -> &mut T {
        &mut self.value
    }

    pub fn span(&self) -> &SourceSpan {
        &self.span
    }
}

#[allow(dead_code)]
impl Located<String> {
    pub fn text(&self) -> &str {
        &self.value
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

impl fmt::Display for Artifact {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&serialize(self))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_multiline_metadata_replacements() {
        let mut metadata = Metadata {
            key: "Status".into(),
            value: Located::new("proposed".into(), SourceSpan::new(2, 20..28)),
        };

        let error = metadata
            .set_value("accepted\n- Extra: value")
            .expect_err("multiline metadata should be rejected");

        assert_eq!(error.to_string(), "metadata values must be single-line");
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
