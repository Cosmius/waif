use std::collections::{HashMap, HashSet};
use std::ops::Range;

use crate::artifact::{Artifact, Item, ItemForm, Located, Metadata, Section, SourceSpan};
use crate::macro_processor;
use crate::parser::{Diagnostic, ParserConfig, Position, SectionConfig};

//region schema definition

#[derive(Clone)]
pub(crate) struct Schema {
    pub(crate) prefix: ArtifactPrefixRule,
    pub(crate) metadata: &'static [MetadataRule],
    pub(crate) sections: &'static [SectionRule],
}

impl Schema {
    /// Derive the parser configuration from the section rules: sections with
    /// an item rule are itemised, and others are prose.
    pub(crate) fn parser_config(&self) -> ParserConfig<'static> {
        ParserConfig::new(
            self.sections
                .iter()
                .map(|rule| match rule.items {
                    Some(ItemRule {
                        sentinel: Some(sentinel),
                        ..
                    }) => SectionConfig::itemised(rule.name).with_sentinel(sentinel),
                    Some(_) => SectionConfig::itemised(rule.name),
                    None => SectionConfig::prose(rule.name),
                })
                .collect(),
        )
    }
}

/// The `Questions` section shared by the goal, plan, and step contracts.
pub(crate) const QUESTIONS: SectionRule = SectionRule::optional("Questions")
    .with_items(ItemRule::new("Q").with_metadata(&QUESTION_METADATA));

const QUESTION_METADATA: [MetadataRule; 1] = [MetadataRule {
    name: "Status",
    validator: metadata_validators::question_status,
}];

/// Whether an artifact has a question that is not `settled`.
///
/// Placeholders carry no metadata and stand for an expanded question, so only
/// expanded questions are considered.
pub(crate) fn has_open_questions(artifact: &Artifact<'_>) -> bool {
    artifact
        .sections()
        .iter()
        .filter(|section| section.title() == QUESTIONS.name)
        .filter_map(|section| section.as_itemised())
        .flat_map(|section| section.items())
        .filter_map(Item::as_expanded)
        .any(|question| {
            question
                .metadatum("Status")
                .is_none_or(|status| status.value() != "settled")
        })
}

#[derive(Clone, Copy)]
pub(crate) enum ArtifactPrefixRule {
    Known(&'static str),
    Unknown(ArtifactPrefixShape),
}

#[derive(Clone, Copy)]
pub(crate) enum ArtifactPrefixShape {
    Step,
    Review,
}

#[derive(Clone, Copy)]
pub(crate) struct MetadataRule {
    pub(crate) name: &'static str,
    pub(crate) validator: MetadataValidator,
}

#[derive(Clone, Copy)]
pub(crate) struct SectionRule {
    pub(crate) name: &'static str,
    pub(crate) required: bool,
    pub(crate) items: Option<ItemRule>,
}

impl SectionRule {
    pub(crate) const fn new(name: &'static str) -> Self {
        Self {
            name,
            required: true,
            items: None,
        }
    }

    pub(crate) const fn optional(name: &'static str) -> Self {
        Self {
            name,
            required: false,
            items: None,
        }
    }

    pub(crate) const fn with_items(mut self, items: ItemRule) -> Self {
        self.items = Some(items);
        self
    }
}

/// The configuration of an itemised section.
#[derive(Clone, Copy)]
pub(crate) struct ItemRule {
    /// The ID family, such as `AC` in `G-AC1`.
    pub(crate) family: &'static str,
    /// Keys that every item except a placeholder must carry as metadata.
    pub(crate) metadata: &'static [MetadataRule],
    /// The text that may form the whole section body instead of items.
    pub(crate) sentinel: Option<&'static str>,
    /// Whether IDs omit the artifact prefix, as plan items do.
    pub(crate) unprefixed: bool,
}

impl ItemRule {
    pub(crate) const fn new(family: &'static str) -> Self {
        Self {
            family,
            metadata: &[],
            sentinel: None,
            unprefixed: false,
        }
    }

    pub(crate) const fn with_metadata(mut self, metadata: &'static [MetadataRule]) -> Self {
        self.metadata = metadata;
        self
    }

    pub(crate) const fn with_sentinel(mut self, sentinel: &'static str) -> Self {
        self.sentinel = Some(sentinel);
        self
    }

    pub(crate) const fn unprefixed(mut self) -> Self {
        self.unprefixed = true;
        self
    }
}

//endregion schema definition

//region validations

struct ValidationContext {
    schema: Schema,
    diagnostics: Vec<Diagnostic>,
    observed_prefix: Option<ObservedArtifactPrefix>,
    item_seq_map: HashMap<(&'static str, ItemForm), ItemSeq>,
}

impl ValidationContext {
    pub(crate) fn new(schema: Schema) -> Self {
        Self {
            schema,
            diagnostics: Vec::new(),
            observed_prefix: None,
            item_seq_map: HashMap::new(),
        }
    }

    fn on_seeing_id(
        &mut self,
        section_name: &str,
        sequence: (&'static str, ItemForm),
        number: i64,
        position: Position,
        id_text: &str,
        prefix_for_error_info: &str,
    ) {
        let item_seq = self
            .item_seq_map
            .entry(sequence)
            .or_insert_with(|| ItemSeq {
                seen: HashSet::new(),
                last: 0,
            });
        if !item_seq.seen.insert(number) {
            self.diagnostics.push(Diagnostic::error_p(
                position,
                format!("duplicate item identifier `{id_text}` in section `{section_name}`"),
            ));
        } else if number < item_seq.last {
            self.diagnostics.push(Diagnostic::error_p(
                position,
                format!(
                    "item identifier `{id_text}` in section `{}` must be \
                     greater than `{}{}`",
                    section_name, prefix_for_error_info, item_seq.last,
                ),
            ));
        }
        if number > item_seq.last {
            item_seq.last = number;
        }
    }
}

#[derive(Debug, PartialEq, Eq)]
pub(crate) struct ObservedArtifactPrefix {
    text: String,
    components: Vec<Located<i64>>,
}

impl ObservedArtifactPrefix {
    pub(crate) fn text(&self) -> &str {
        &self.text
    }

    pub(crate) fn components(&self) -> &[Located<i64>] {
        &self.components
    }
}

struct ItemSeq {
    seen: HashSet<i64>,
    last: i64,
}

pub(crate) fn validate(artifact: &Artifact, schema: &Schema) -> Vec<Diagnostic> {
    validate_with_prefix(artifact, schema).0
}

pub(crate) fn validate_with_prefix(
    artifact: &Artifact,
    schema: &Schema,
) -> (Vec<Diagnostic>, Option<ObservedArtifactPrefix>) {
    let mut ctx = ValidationContext::new(schema.clone());
    validate_metadata(&mut ctx, artifact);
    validate_sections(&mut ctx, artifact);
    (ctx.diagnostics, ctx.observed_prefix)
}

fn validate_metadata(ctx: &mut ValidationContext, artifact: &Artifact) {
    let rules = ctx.schema.metadata;
    let seen = validate_entries(ctx, artifact.metadata(), rules, "metadata");
    for rule in rules {
        if !seen.contains(rule.name) {
            ctx.diagnostics.push(Diagnostic::error_p(
                *artifact.located_title().span().end(),
                format!("missing required metadata `{}`", rule.name),
            ));
        }
    }
}

/// Report repeated keys and values rejected by their rule's validator, which
/// is skipped for a value containing a macro. Return the keys seen.
fn validate_entries<'a>(
    ctx: &mut ValidationContext,
    entries: &[Located<Metadata<'a>>],
    rules: &[MetadataRule],
    kind: &str,
) -> HashSet<&'a str> {
    let mut seen = HashSet::new();

    for entry in entries {
        let pos = *entry.span().start();
        let metadata = entry.value();
        let key = metadata.key();
        if !seen.insert(key) {
            ctx.diagnostics.push(Diagnostic::error_p(
                pos,
                format!("duplicate {kind} key `{key}`"),
            ));
        }

        if let Some(rule) = rules.iter().find(|rule| rule.name == key) {
            let value = metadata.value();
            if macro_processor::scan(value).is_empty() {
                if let Err(message) = (rule.validator)(value) {
                    ctx.diagnostics.push(Diagnostic::error_p(
                        pos,
                        format!("{kind} `{}` {message}", rule.name),
                    ));
                }
            }
        }
    }
    seen
}

fn validate_sections(ctx: &mut ValidationContext, artifact: &Artifact) {
    let mut seen_sections = HashSet::new();
    let mut greatest_known: Option<(usize, &str)> = None;

    for section in artifact.sections() {
        let name = section.title();
        let pos = *section.span().start();
        if !seen_sections.insert(name) {
            ctx.diagnostics.push(Diagnostic::error_p(
                pos,
                format!("duplicate section `{name}`"),
            ));
        }
        let Some((rank, rule)) = ctx
            .schema
            .sections
            .iter()
            .enumerate()
            .find(|(_, rule)| rule.name == name)
        else {
            if section_is_empty(section) {
                ctx.diagnostics.push(empty_section_warning(pos, name));
            }
            continue;
        };
        if section_is_empty(section) {
            if rule.required && rule.items.is_some() {
                ctx.diagnostics.push(Diagnostic::error_p(
                    pos,
                    format!("required section `{name}` must not be empty"),
                ));
            } else {
                ctx.diagnostics.push(empty_section_warning(pos, name));
            }
        }
        if let Some((greatest_rank, greatest_name)) = greatest_known {
            if rank < greatest_rank {
                ctx.diagnostics.push(Diagnostic::error_p(
                    pos,
                    format!("section `{name}` must appear before section `{greatest_name}`"),
                ));
            } else {
                greatest_known = Some((rank, name));
            }
        } else {
            greatest_known = Some((rank, name));
        }

        if let Some(item_rule) = rule.items {
            validate_items(ctx, section, item_rule);
        }
    }

    for rule in ctx.schema.sections.iter().filter(|rule| rule.required) {
        if !seen_sections.contains(rule.name) {
            ctx.diagnostics.push(Diagnostic::error_p(
                *artifact.located_title().span().end(),
                format!("missing required section `{}`", rule.name),
            ));
        }
    }
}

/// Validate every item of an itemised section against the section's rule.
fn validate_items(ctx: &mut ValidationContext, section: &Section, rule: ItemRule) {
    let section_name = section.title();
    let Some(section) = section.as_itemised() else {
        ctx.diagnostics.push(Diagnostic::error_p(
            *section.located_title().span().start(),
            format!("section {section_name} must be an itemised section"),
        ));
        return;
    };
    let prefix = if rule.unprefixed {
        ArtifactPrefixRule::Known("")
    } else {
        ctx.schema.prefix
    };
    let expanded_ids: HashSet<_> = section
        .items()
        .iter()
        .filter(|item| item.form() == ItemForm::Expanded)
        .filter_map(Item::identifier)
        .map(Located::text)
        .collect();

    for item in section.items() {
        validate_item_identifier(ctx, section_name, rule.family, prefix, item);

        let pos = *item.span().start();
        let seen = match item {
            Item::Expanded(ei) => {
                if ei.value().title().text().is_empty() {
                    ctx.diagnostics.push(Diagnostic::error_p(
                        pos,
                        format!("item in section `{section_name}` is missing a title"),
                    ));
                }
                validate_entries(ctx, ei.value().metadata(), rule.metadata, "item metadata")
            }
            Item::Compact(ci) => {
                match ci.value().identifier() {
                    Some(identifier) if expanded_ids.contains(identifier.text()) => {
                        // A placeholder stands in for its expanded item and
                        // carries no metadata of its own.
                        if is_placeholder(item.short_description().text()) {
                            continue;
                        }
                        ctx.diagnostics.push(Diagnostic::error_p(
                            pos,
                            format!(
                                "compact item `{}` in section `{section_name}` shares its \
                                 identifier with an expanded item but does not start with \
                                 `Vide infra`",
                                identifier.text()
                            ),
                        ));
                    }
                    _ => {}
                }
                HashSet::new()
            }
        };
        for metadata_rule in rule.metadata {
            if !seen.contains(metadata_rule.name) {
                ctx.diagnostics.push(Diagnostic::error_p(
                    pos,
                    format!(
                        "item in section `{section_name}` is missing required metadata `{}`",
                        metadata_rule.name
                    ),
                ));
            }
        }
    }
}

/// Check an item's ID against the section's family and track its order and
/// uniqueness within the item's group of compact or expanded items.
fn validate_item_identifier(
    ctx: &mut ValidationContext,
    section_name: &str,
    family: &'static str,
    prefix: ArtifactPrefixRule,
    item: &Item,
) {
    let Some(identifier) = item.identifier() else {
        ctx.diagnostics.push(Diagnostic::error_p(
            *item.span().start(),
            format!(
                "item in section `{}` is missing an identifier; expected `{}<number>`",
                section_name,
                expected_identifier(prefix, family, ctx.observed_prefix.as_ref())
            ),
        ));
        return;
    };
    let identifier_text = identifier.text();
    let Ok(number) = match_item_identifier(ctx, prefix, family, identifier) else {
        ctx.diagnostics.push(Diagnostic::error_p(
            *identifier.span().start(),
            format!(
                "item identifier `{identifier_text}` in section `{}` must use \
                 `{}<number>`, where number is from 1 through `2^63 - 1`",
                section_name,
                expected_identifier(prefix, family, ctx.observed_prefix.as_ref())
            ),
        ));
        return;
    };
    let prefix_for_error_info = expected_identifier(prefix, family, ctx.observed_prefix.as_ref());
    ctx.on_seeing_id(
        section_name,
        (family, item.form()),
        number,
        *identifier.span().start(),
        identifier_text,
        &prefix_for_error_info,
    );
}

/// Whether compact item content starts with `Vide infra` or `vide infra` as a
/// whole word.
fn is_placeholder(content: &str) -> bool {
    let rest = match content.strip_prefix("Vide infra") {
        Some(rest) => rest,
        None => match content.strip_prefix("vide infra") {
            Some(rest) => rest,
            None => return false,
        },
    };
    !rest.starts_with(|ch: char| ch.is_alphanumeric() || ch == '_')
}

//endregion validations

//region metadata validators

pub(crate) type MetadataValidator = fn(&str) -> Result<(), String>;

pub(crate) mod metadata_validators {
    use chrono::DateTime;

    pub(crate) fn artifact_status(value: &str) -> Result<(), String> {
        if matches!(
            value,
            "drafting" | "accepted" | "amending" | "done" | "aborted"
        ) {
            Ok(())
        } else {
            Err(format!(
                "expected `drafting`, `accepted`, `amending`, `done`, or `aborted`, \
                 but got `{value}`"
            ))
        }
    }

    pub(crate) fn question_status(value: &str) -> Result<(), String> {
        match value {
            "open" | "settled" => Ok(()),
            _ => Err(format!("expected `open` or `settled`, but got `{value}`")),
        }
    }

    pub(crate) fn rfc3339_timestamp(value: &str) -> Result<(), String> {
        DateTime::parse_from_rfc3339(value)
            .map(|_| ())
            .map_err(|_| {
                format!("expected an RFC 3339 timestamp with a timezone, but got `{value}`")
            })
    }
}

//endregion metadata validators

//region helpers

fn empty_section_warning(pos: Position, name: &str) -> Diagnostic {
    Diagnostic::warning_p(pos, format!("section `{name}` is empty"))
}

fn section_is_empty(section: &Section) -> bool {
    match section {
        Section::Prose(section) => section.value().body().trim().is_empty(),
        Section::Itemised(section) => {
            section.value().sentinel().is_none() && section.value().items().is_empty()
        }
    }
}

fn expected_identifier(
    prefix: ArtifactPrefixRule,
    family: &str,
    observed: Option<&ObservedArtifactPrefix>,
) -> String {
    match prefix {
        ArtifactPrefixRule::Known(prefix) => format!("{prefix}{family}"),
        ArtifactPrefixRule::Unknown(shape) => {
            if let Some(observed) = observed {
                format!("{}{family}", observed.text)
            } else {
                match shape {
                    ArtifactPrefixShape::Step => format!("S<number>-{family}"),
                    ArtifactPrefixShape::Review => format!("S<number>-R<number>-{family}"),
                }
            }
        }
    }
}

fn match_item_identifier(
    ctx: &mut ValidationContext,
    prefix: ArtifactPrefixRule,
    family: &str,
    identifier: &Located<&str>,
) -> Result<i64, ()> {
    let text = identifier.text();
    let (prefix_end, components) = match prefix {
        ArtifactPrefixRule::Known(prefix) => {
            if !text.starts_with(prefix) {
                return Err(());
            }
            (prefix.len(), Vec::new())
        }
        ArtifactPrefixRule::Unknown(shape) => parse_unknown_prefix(text, shape)?,
    };
    let number = text[prefix_end..]
        .strip_prefix(family)
        .and_then(unpadded_decimal)
        .ok_or(())?;

    if matches!(prefix, ArtifactPrefixRule::Unknown(_)) {
        let prefix_text = &text[..prefix_end];
        if let Some(previous) = ctx.observed_prefix.as_ref() {
            if previous.text != prefix_text {
                return Err(());
            }
        } else {
            let components = components
                .into_iter()
                .map(|(value, range)| {
                    let start = identifier
                        .span()
                        .start()
                        .advance(&prefix_text[..range.start]);
                    let end = identifier.span().start().advance(&prefix_text[..range.end]);
                    Located::new(value, SourceSpan::new(start, end))
                })
                .collect();
            ctx.observed_prefix = Some(ObservedArtifactPrefix {
                text: prefix_text.to_owned(),
                components,
            });
        }
    }
    Ok(number)
}

// Returns the byte offset after an unknown prefix's trailing dash, followed by
// its numeric values and their identifier-relative byte ranges.
fn parse_unknown_prefix(
    identifier: &str,
    shape: ArtifactPrefixShape,
) -> Result<(usize, Vec<(i64, Range<usize>)>), ()> {
    if !identifier.starts_with('S') {
        return Err(());
    }
    let (step, step_range) = number_at(identifier, 1).ok_or(())?;
    let mut end = step_range.end;
    let mut components = vec![(step, step_range)];
    if matches!(shape, ArtifactPrefixShape::Review) {
        if !identifier[end..].starts_with("-R") {
            return Err(());
        }
        end += 2;
        let (review, review_range) = number_at(identifier, end).ok_or(())?;
        end = review_range.end;
        components.push((review, review_range));
    }
    if !identifier[end..].starts_with('-') {
        return Err(());
    }
    end += 1;
    Ok((end, components))
}

// Parses decimal numbers at offset, returns value and range
fn number_at(identifier: &str, start: usize) -> Option<(i64, Range<usize>)> {
    let end = start
        + identifier[start..]
            .bytes()
            .take_while(u8::is_ascii_digit)
            .count();
    let range = start..end;
    unpadded_decimal(&identifier[range.clone()]).map(|number| (number, range))
}

// Parses decimal number into i64, no leading zeros allowed
pub(crate) fn unpadded_decimal(number: &str) -> Option<i64> {
    let mut bytes = number.bytes();
    if !matches!(bytes.next(), Some(b'1'..=b'9')) {
        return None;
    }
    if !bytes.all(|byte| byte.is_ascii_digit()) {
        return None;
    }
    number.parse().ok()
}

//endregion helpers

#[cfg(test)]
mod tests;
