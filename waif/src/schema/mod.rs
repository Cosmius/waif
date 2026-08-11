use std::collections::{HashMap, HashSet};
use std::ops::Range;

use crate::artifact::{Artifact, FindingsBody, Item, ItemForm, Located, Section, SourceSpan};
use crate::parser::{Diagnostic, Position};

//region schema definition

#[derive(Clone)]
pub(crate) struct Schema {
    pub(crate) prefix: ArtifactPrefixRule,
    pub(crate) metadata: &'static [MetadataRule],
    pub(crate) sections: &'static [SectionRule],
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
    pub(crate) plan_items: Option<PlanItemRule>,
    pub(crate) findings: Option<FindingsRule>,
}

impl SectionRule {
    pub(crate) const fn new(name: &'static str) -> Self {
        Self {
            name,
            required: true,
            items: None,
            plan_items: None,
            findings: None,
        }
    }

    pub(crate) const fn optional(name: &'static str) -> Self {
        Self {
            name,
            required: false,
            items: None,
            plan_items: None,
            findings: None,
        }
    }

    pub(crate) const fn with_items(mut self, items: ItemRule) -> Self {
        self.items = Some(items);
        self
    }

    pub(crate) const fn with_plan_items(mut self, plan_items: PlanItemRule) -> Self {
        self.plan_items = Some(plan_items);
        self
    }

    pub(crate) const fn with_findings(mut self, findings: FindingsRule) -> Self {
        self.findings = Some(findings);
        self
    }
}

#[derive(Clone, Copy)]
pub(crate) struct ItemRule {
    pub(crate) family: &'static str,
    pub(crate) expanded_family: Option<&'static str>,
    pub(crate) forms: ItemForms,
}

impl ItemRule {
    pub(crate) const fn new(family: &'static str) -> Self {
        Self {
            family,
            expanded_family: None,
            forms: ItemForms::Consistent,
        }
    }

    pub(crate) const fn expanded_only(mut self) -> Self {
        self.forms = ItemForms::ExpandedOnly;
        self
    }

    pub(crate) const fn mixed(mut self) -> Self {
        self.forms = ItemForms::Mixed;
        self
    }

    pub(crate) const fn with_expanded_family(mut self, family: &'static str) -> Self {
        self.expanded_family = Some(family);
        self
    }
}

#[derive(Clone, Copy)]
pub(crate) enum ItemForms {
    Consistent,
    ExpandedOnly,
    // The parser represents itemised sections as compact items followed by
    // expanded items. Once an expanded item starts, compact-looking bullets
    // are its opaque body, so a compact item cannot structurally follow it.
    Mixed,
}

#[derive(Clone, Copy)]
pub(crate) struct PlanItemRule {
    pub(crate) statuses: &'static [&'static str],
}

#[derive(Clone, Copy)]
#[allow(dead_code)]
pub(crate) struct FindingsRule {
    pub(crate) family: &'static str,
}

//endregion schema definition

//region validations

struct ValidationContext {
    schema: Schema,
    diagnostics: Vec<Diagnostic>,
    observed_prefix: Option<ObservedArtifactPrefix>,
    item_seq_map: HashMap<&'static str, ItemSeq>,
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
        family: &'static str,
        number: i64,
        position: Position,
        id_text: &str,
        prefix_for_error_info: &str,
    ) {
        let item_seq = self.item_seq_map.entry(family).or_insert_with(|| ItemSeq {
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
    let mut seen = HashSet::new();

    for metadata in artifact.metadata() {
        let pos = *metadata.span().start();
        let metadata = metadata.value();
        let key = metadata.key();
        if !seen.insert(key) {
            ctx.diagnostics.push(Diagnostic::error_p(
                pos,
                format!("duplicate metadata key `{key}`"),
            ));
        }

        if let Some(rule) = ctx.schema.metadata.iter().find(|rule| rule.name == key) {
            if let Err(message) = (rule.validator)(metadata.value()) {
                ctx.diagnostics.push(Diagnostic::error_p(
                    pos,
                    format!("metadata `{}` {message}", rule.name),
                ));
            }
        }
    }

    for rule in ctx.schema.metadata {
        if !seen.contains(rule.name) {
            ctx.diagnostics.push(Diagnostic::error_p(
                *artifact.located_title().span().end(),
                format!("missing required metadata `{}`", rule.name),
            ));
        }
    }
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
        if section_is_empty(section) {
            ctx.diagnostics.push(Diagnostic::warning_p(
                pos,
                format!("section `{name}` is empty"),
            ));
        }

        let Some((rank, rule)) = ctx
            .schema
            .sections
            .iter()
            .enumerate()
            .find(|(_, rule)| rule.name == name)
        else {
            continue;
        };
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
        if let Some(plan_item_rule) = rule.plan_items {
            validate_plan_items(ctx, section, plan_item_rule);
        }
        if let Some(findings_rule) = rule.findings {
            validate_findings(ctx, section, findings_rule);
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

fn validate_items(ctx: &mut ValidationContext, section: &Section, rule: ItemRule) {
    let section_name = section.title();
    let Some(section) = section.as_itemised() else {
        ctx.diagnostics.push(Diagnostic::error_p(
            *section.located_title().span().start(),
            format!("section {section_name} must be an itemised section"),
        ));
        return;
    };
    validate_item_forms(ctx, section_name, section.items(), rule.forms);

    for item in section.items() {
        let Some(identifier) = item.identifier() else {
            ctx.diagnostics.push(Diagnostic::error_p(
                *item.span().start(),
                format!(
                    "item in section `{}` is missing an identifier; expected `{}<number>`",
                    section_name,
                    expected_identifier(
                        ctx.schema.prefix,
                        rule.family,
                        ctx.observed_prefix.as_ref()
                    )
                ),
            ));
            continue;
        };
        let identifier_text = identifier.text();
        let family = match item.form() {
            ItemForm::Compact => rule.family,
            ItemForm::Expanded => rule.expanded_family.unwrap_or(rule.family),
        };
        let Ok(number) = match_item_identifier(ctx, family, identifier) else {
            ctx.diagnostics.push(Diagnostic::error_p(
                *identifier.span().start(),
                format!(
                    "item identifier `{identifier_text}` in section `{}` must use \
                     `{}<number>`, where number is from 1 through `2^63 - 1`",
                    section_name,
                    expected_identifier(ctx.schema.prefix, family, ctx.observed_prefix.as_ref())
                ),
            ));
            continue;
        };
        let prefix_for_error_info =
            expected_identifier(ctx.schema.prefix, family, ctx.observed_prefix.as_ref());
        ctx.on_seeing_id(
            section_name,
            family,
            number,
            *identifier.span().start(),
            identifier_text,
            &prefix_for_error_info,
        );
    }
}

fn validate_item_forms(
    ctx: &mut ValidationContext,
    section_name: &str,
    items: &[Item],
    forms: ItemForms,
) {
    let required = match forms {
        ItemForms::Consistent => match items.first() {
            Some(first) => first.form(),
            None => return,
        },
        ItemForms::ExpandedOnly => ItemForm::Expanded,
        ItemForms::Mixed => return,
    };
    let Some(conflicting) = items.iter().find(|item| item.form() != required) else {
        return;
    };

    match forms {
        ItemForms::Consistent => ctx.diagnostics.push(Diagnostic::error_p(
            *conflicting.span().start(),
            format!("itemised section `{section_name}` cannot mix compact and expanded items"),
        )),
        ItemForms::ExpandedOnly => ctx.diagnostics.push(Diagnostic::error_p(
            *conflicting.span().start(),
            format!("section `{section_name}` requires expanded items in `### ID: title` form"),
        )),
        ItemForms::Mixed => unreachable!("mixed forms return before comparison"),
    }
}

fn validate_plan_items(ctx: &mut ValidationContext, section: &Section, rule: PlanItemRule) {
    let section_name = section.title();
    let Some(section) = section.as_plan_items() else {
        ctx.diagnostics.push(Diagnostic::error_p(
            *section.located_title().span().start(),
            format!("section {section_name} must be a plan items section"),
        ));
        return;
    };

    for item in section.items() {
        let statuses: Vec<_> = item
            .value()
            .metadata()
            .iter()
            .filter(|entry| entry.value().key() == "Status")
            .collect();
        if statuses.is_empty() {
            ctx.diagnostics.push(Diagnostic::error_p(
                *item.span().start(),
                format!(
                    "plan item in section `{section_name}` is missing required metadata `Status`"
                ),
            ));
        }
        for status in &statuses {
            if !rule.statuses.contains(&status.value().value()) {
                ctx.diagnostics.push(Diagnostic::error_p(
                    *status.span().start(),
                    format!(
                        "plan-item metadata `Status` expected {}, but got `{}`",
                        joined_choices(rule.statuses),
                        status.value().value()
                    ),
                ));
            }
        }
        for duplicate in statuses.iter().skip(1) {
            ctx.diagnostics.push(Diagnostic::error_p(
                *duplicate.span().start(),
                "duplicate plan-item metadata key `Status`",
            ));
        }
        if item.value().title().text().is_empty() {
            ctx.diagnostics.push(Diagnostic::error_p(
                *item.value().span().start(),
                format!("plan item in section `{section_name}` is missing a title"),
            ));
        }
        let Some(identifier) = item.value().identifier() else {
            ctx.diagnostics.push(Diagnostic::error_p(
                *item.value().span().start(),
                format!("plan item in section `{section_name}` is missing an identifier"),
            ));
            continue;
        };
        let Some(number) = identifier
            .text()
            .strip_prefix('P')
            .and_then(unpadded_decimal)
        else {
            ctx.diagnostics.push(Diagnostic::error_p(
                *identifier.span().start(),
                format!(
                    "plan item identifier `{}` in section `{section_name}` must use \
                 `P<number>`, where number is from 1 through `2^63 - 1`",
                    identifier.text()
                ),
            ));
            continue;
        };
        ctx.on_seeing_id(
            section_name,
            "P",
            number,
            *identifier.span().start(),
            identifier.text(),
            "P",
        );
    }
}

fn validate_findings(ctx: &mut ValidationContext, section: &Section, rule: FindingsRule) {
    let section_name = section.title();
    let Some(section) = section.as_findings() else {
        ctx.diagnostics.push(Diagnostic::error_p(
            *section.located_title().span().start(),
            format!("section {section_name} must be a findings section"),
        ));
        return;
    };

    let FindingsBody::Items(items) = section.body() else {
        return;
    };
    for finding in items.value() {
        let Some(identifier) = finding.identifier() else {
            ctx.diagnostics.push(Diagnostic::error_p(
                *finding.span().start(),
                format!("finding in section `{section_name}` is missing an identifier"),
            ));
            continue;
        };
        let Ok(number) = match_item_identifier(ctx, rule.family, identifier) else {
            ctx.diagnostics.push(Diagnostic::error_p(
                *identifier.span().start(),
                format!(
                    "finding identifier `{}` in section `{section_name}` must use \
                 `{}<number>`, where number is from 1 through `2^63 - 1`",
                    identifier.text(),
                    expected_identifier(
                        ctx.schema.prefix,
                        rule.family,
                        ctx.observed_prefix.as_ref()
                    )
                ),
            ));
            continue;
        };

        let prefix_for_error_info =
            expected_identifier(ctx.schema.prefix, rule.family, ctx.observed_prefix.as_ref());
        ctx.on_seeing_id(
            section_name,
            rule.family,
            number,
            *finding.span().start(),
            identifier.text(),
            &prefix_for_error_info,
        );
    }
}

//endregion validations

//region metadata validators

pub(crate) type MetadataValidator = fn(&str) -> Result<(), String>;

pub(crate) mod metadata_validators {
    use chrono::DateTime;

    pub(crate) fn artifact_status(value: &str) -> Result<(), String> {
        if matches!(value, "drafting" | "accepted" | "amending") {
            Ok(())
        } else {
            Err(format!(
                "expected `drafting`, `accepted`, or `amending`, but got `{value}`"
            ))
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

fn section_is_empty(section: &Section) -> bool {
    match section {
        Section::Prose(section) => section.value().body().trim().is_empty(),
        Section::Itemised(section) => section.value().items().is_empty(),
        Section::PlanItems(section) => section.value().items().is_empty(),
        Section::Findings(section) => match section.value().body() {
            FindingsBody::Sentinel(body) => body.text().trim().is_empty(),
            FindingsBody::Items(items) => items.value().is_empty(),
        },
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

fn joined_choices(choices: &[&str]) -> String {
    choices
        .iter()
        .map(|choice| format!("`{choice}`"))
        .collect::<Vec<_>>()
        .join(" or ")
}

fn match_item_identifier(
    ctx: &mut ValidationContext,
    family: &str,
    identifier: &Located<&str>,
) -> Result<i64, ()> {
    let text = identifier.text();
    let (prefix_end, components) = match ctx.schema.prefix {
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

    if matches!(ctx.schema.prefix, ArtifactPrefixRule::Unknown(_)) {
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
