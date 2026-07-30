use std::fmt;
use std::ops::Range;

#[derive(Debug, PartialEq, Eq)]
pub struct Artifact {
    source: String,
    title: String,
    metadata: Vec<Metadata>,
    sections: Vec<Section>,
}

// This is the structured artifact API. The check command currently only needs
// parsing to succeed, while later commands will consume these accessors.
#[allow(dead_code)]
impl Artifact {
    pub(crate) fn new(
        source: String,
        title: String,
        metadata: Vec<Metadata>,
        sections: Vec<Section>,
    ) -> Self {
        Self {
            source,
            title,
            metadata,
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

    pub fn metadata(&self) -> &[Metadata] {
        &self.metadata
    }

    /// Existing metadata entries, mutably borrowed in source order.
    ///
    /// The slice cannot add, remove, or reorder entries. Use
    /// [`Metadata::set_value`] to change an existing value. Keys and ordering
    /// are controlled by the applicable artifact contract.
    pub fn metadata_mut(&mut self) -> &mut [Metadata] {
        &mut self.metadata
    }

    pub fn sections(&self) -> &[Section] {
        &self.sections
    }

    /// Serialize this artifact, preserving untouched source text exactly.
    pub fn serialize(&self) -> String {
        serialize(self)
    }
}

#[derive(Debug, PartialEq, Eq)]
pub struct Metadata {
    key: String,
    value: String,
    value_range: Range<usize>,
}

#[allow(dead_code)]
impl Metadata {
    pub(crate) fn new(key: String, value: String, value_range: Range<usize>) -> Self {
        Self {
            key,
            value,
            value_range,
        }
    }

    pub fn key(&self) -> &str {
        &self.key
    }

    pub fn value(&self) -> &str {
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
        self.value = value;
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
}

#[allow(dead_code)]
impl Section {
    pub fn name(&self) -> &str {
        match self {
            Self::Prose(section) => section.value.title.text(),
            Self::Itemised(section) => section.value.title.text(),
        }
    }

    pub fn title(&self) -> &Located<String> {
        match self {
            Self::Prose(section) => &section.value.title,
            Self::Itemised(section) => &section.value.title,
        }
    }

    pub fn body_span(&self) -> &SourceSpan {
        match self {
            Self::Prose(section) => section.value.body.span(),
            Self::Itemised(section) => section.value.items.span(),
        }
    }

    pub fn span(&self) -> &SourceSpan {
        match self {
            Self::Prose(section) => section.span(),
            Self::Itemised(section) => section.span(),
        }
    }

    pub fn as_prose(&self) -> Option<&ProseSection> {
        match self {
            Self::Prose(section) => Some(section.value()),
            Self::Itemised(_) => None,
        }
    }

    pub fn as_itemised(&self) -> Option<&ItemisedSection> {
        match self {
            Self::Prose(_) => None,
            Self::Itemised(section) => Some(section.value()),
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
pub enum Item {
    Compact(Located<CompactItem>),
}

#[allow(dead_code)]
impl Item {
    pub fn identifier(&self) -> Option<&Located<String>> {
        match self {
            Self::Compact(item) => item.value.identifier.as_ref(),
        }
    }

    pub fn content(&self) -> &Located<String> {
        match self {
            Self::Compact(item) => &item.value.content,
        }
    }

    pub fn span(&self) -> &SourceSpan {
        match self {
            Self::Compact(item) => item.span(),
        }
    }

    pub fn as_compact(&self) -> Option<&CompactItem> {
        match self {
            Self::Compact(item) => Some(item.value()),
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
    let capacity = artifact
        .metadata
        .iter()
        .fold(artifact.source.len(), |capacity, entry| {
            capacity - entry.value_range.len() + entry.value.len()
        });
    let mut output = String::with_capacity(capacity);
    let mut copied_until = 0;

    for entry in &artifact.metadata {
        let unchanged = &artifact.source[copied_until..entry.value_range.start];
        output.push_str(unchanged);
        output.push_str(&entry.value);
        copied_until = entry.value_range.end;
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
            value: "proposed".into(),
            value_range: 20..28,
        };

        let error = metadata
            .set_value("accepted\n- Extra: value")
            .expect_err("multiline metadata should be rejected");

        assert_eq!(error.to_string(), "metadata values must be single-line");
    }

    #[test]
    fn serializes_metadata_changes_without_reformatting_source() {
        let source = "# Example\r\n- Status: proposed\r\n";
        let mut artifact = crate::parser::parse(source).expect("artifact should parse");
        artifact.metadata_mut()[0]
            .set_value("accepted")
            .expect("value should be valid");

        assert_eq!(serialize(&artifact), "# Example\r\n- Status: accepted\r\n");
        assert_eq!(artifact.to_string(), "# Example\r\n- Status: accepted\r\n");
        assert_eq!(artifact.source(), "# Example\r\n- Status: proposed\r\n");
    }
}
