use std::collections::{HashMap, HashSet};
use std::ops::Range;

use chrono::DateTime;

use crate::artifact::{
    Artifact, Finding, FindingsBody, Item, ItemForm, Located, PlanItem, Section, SourceSpan,
};
use crate::parser::Diagnostic;

pub(crate) type ValueValidator = fn(&str) -> Result<(), String>;

#[derive(Clone, Copy)]
pub(crate) struct MetadataRule {
    pub(crate) name: &'static str,
    pub(crate) validator: ValueValidator,
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

    pub(crate) const fn with_expanded_family(mut self, family: &'static str) -> Self {
        self.expanded_family = Some(family);
        self
    }

    pub(crate) const fn expanded_only(mut self) -> Self {
        self.forms = ItemForms::ExpandedOnly;
        self
    }

    pub(crate) const fn mixed(mut self) -> Self {
        self.forms = ItemForms::Mixed;
        self
    }
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

#[derive(Clone, Copy)]
#[allow(dead_code)]
pub(crate) enum ArtifactPrefixRule {
    Known(&'static str),
    Unknown(ArtifactPrefixShape),
}

#[derive(Clone, Copy)]
#[allow(dead_code)]
pub(crate) enum ArtifactPrefixShape {
    Step,
    Review,
}

#[derive(Debug, PartialEq, Eq)]
pub(crate) struct ObservedArtifactPrefix {
    text: String,
    components: Vec<Located<i64>>,
}

#[derive(Default)]
struct ItemSequence {
    seen: HashSet<i64>,
    last_number: Option<i64>,
}

type PrefixComponent = (i64, Range<usize>);
// The byte offset after an unknown prefix's trailing dash, followed by its
// numeric values and their identifier-relative byte ranges.
type ParsedPrefix = (usize, Vec<PrefixComponent>);

#[allow(dead_code)]
impl ObservedArtifactPrefix {
    pub(crate) fn text(&self) -> &str {
        &self.text
    }

    pub(crate) fn components(&self) -> &[Located<i64>] {
        &self.components
    }
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

    pub(crate) const fn optional(mut self) -> Self {
        self.required = false;
        self
    }

    pub(crate) const fn with_items(mut self, items: ItemRule) -> Self {
        self.items = Some(items);
        self
    }

    pub(crate) const fn with_plan_items(mut self, plan_items: PlanItemRule) -> Self {
        self.plan_items = Some(plan_items);
        self
    }

    #[allow(dead_code)]
    pub(crate) const fn with_findings(mut self, findings: FindingsRule) -> Self {
        self.findings = Some(findings);
        self
    }
}

pub(crate) struct Schema {
    pub(crate) prefix: ArtifactPrefixRule,
    pub(crate) metadata: &'static [MetadataRule],
    pub(crate) sections: &'static [SectionRule],
}

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
        .map_err(|_| format!("expected an RFC 3339 timestamp with a timezone, but got `{value}`"))
}

pub(crate) fn validate(artifact: &Artifact, schema: &Schema) -> Vec<Diagnostic> {
    validate_with_prefix(artifact, schema).0
}

pub(crate) fn validate_with_prefix(
    artifact: &Artifact,
    schema: &Schema,
) -> (Vec<Diagnostic>, Option<ObservedArtifactPrefix>) {
    let mut diagnostics = Vec::new();
    validate_metadata(artifact, schema, &mut diagnostics);
    let prefix = validate_sections(artifact, schema, &mut diagnostics);
    (diagnostics, prefix)
}

fn validate_metadata(artifact: &Artifact, schema: &Schema, diagnostics: &mut Vec<Diagnostic>) {
    let mut seen = HashSet::new();

    for metadata in artifact.metadata() {
        let line = metadata.span().start_line();
        let metadata = metadata.value();
        let key = metadata.key();
        if !seen.insert(key) {
            diagnostics.push(Diagnostic::error1(
                line,
                format!("duplicate metadata key `{key}`"),
            ));
        }

        if let Some(rule) = schema.metadata.iter().find(|rule| rule.name == key) {
            if let Err(message) = (rule.validator)(metadata.value()) {
                diagnostics.push(Diagnostic::error1(
                    line,
                    format!("metadata `{}` {message}", rule.name),
                ));
            }
        }
    }

    for rule in schema.metadata {
        if !seen.contains(rule.name) {
            diagnostics.push(Diagnostic::error1(
                1,
                format!("missing required metadata `{}`", rule.name),
            ));
        }
    }
}

fn validate_sections(
    artifact: &Artifact,
    schema: &Schema,
    diagnostics: &mut Vec<Diagnostic>,
) -> Option<ObservedArtifactPrefix> {
    let mut seen_sections = HashSet::new();
    let mut seen_ids = HashSet::new();
    let mut last_numbers = HashMap::new();
    let mut greatest_known: Option<(usize, &str)> = None;
    let mut observed_prefix = None;

    for section in artifact.sections() {
        let name = section.title();
        let line = section.span().start_line();
        if !seen_sections.insert(name) {
            diagnostics.push(Diagnostic::error1(
                line,
                format!("duplicate section `{name}`"),
            ));
        }
        if section_is_empty(section) {
            diagnostics.push(Diagnostic::warning1(
                line,
                format!("section `{name}` is empty"),
            ));
        }

        let Some((rank, rule)) = schema
            .sections
            .iter()
            .enumerate()
            .find(|(_, rule)| rule.name == name)
        else {
            continue;
        };
        if let Some((greatest_rank, greatest_name)) = greatest_known {
            if rank < greatest_rank {
                diagnostics.push(Diagnostic::error1(
                    line,
                    format!("section `{name}` must appear before section `{greatest_name}`"),
                ));
            } else {
                greatest_known = Some((rank, name));
            }
        } else {
            greatest_known = Some((rank, name));
        }

        if let Some(item_rule) = rule.items {
            validate_items(
                section,
                item_rule,
                schema.prefix,
                &mut observed_prefix,
                &mut seen_ids,
                &mut last_numbers,
                diagnostics,
            );
        }
        if let Some(plan_item_rule) = rule.plan_items {
            validate_plan_items(section, plan_item_rule, diagnostics);
        }
        if let Some(findings_rule) = rule.findings {
            validate_findings(
                section,
                findings_rule,
                schema.prefix,
                &mut observed_prefix,
                diagnostics,
            );
        }
    }

    for rule in schema.sections.iter().filter(|rule| rule.required) {
        if !seen_sections.contains(rule.name) {
            diagnostics.push(Diagnostic::error1(
                1,
                format!("missing required section `{}`", rule.name),
            ));
        }
    }
    observed_prefix
}

fn validate_items(
    section: &Section,
    rule: ItemRule,
    prefix_rule: ArtifactPrefixRule,
    observed_prefix: &mut Option<ObservedArtifactPrefix>,
    seen: &mut HashSet<(&'static str, i64)>,
    last_numbers: &mut HashMap<&'static str, i64>,
    diagnostics: &mut Vec<Diagnostic>,
) {
    let section_name = section.title();
    let section = section
        .as_itemised()
        .expect("an item rule requires an itemised parser section");
    validate_item_forms(section_name, section.items(), rule.forms, diagnostics);

    for item in section.items() {
        let Some(identifier) = item.identifier() else {
            diagnostics.push(Diagnostic::error1(
                item.span().start_line(),
                format!(
                    "item in section `{}` is missing an identifier; expected `{}<number>`",
                    section_name,
                    expected_identifier(prefix_rule, rule.family, observed_prefix.as_ref())
                ),
            ));
            continue;
        };
        let identifier_text = identifier.text();
        let family = match item.form() {
            ItemForm::Compact => rule.family,
            ItemForm::Expanded => rule.expanded_family.unwrap_or(rule.family),
        };
        let Ok(number) = match_item_identifier(identifier, prefix_rule, family, observed_prefix)
        else {
            diagnostics.push(Diagnostic::error1(
                identifier.span().start_line(),
                format!(
                    "item identifier `{identifier_text}` in section `{}` must use \
                     `{}<number>`, where number is from 1 through `2^63 - 1`",
                    section_name,
                    expected_identifier(prefix_rule, family, observed_prefix.as_ref())
                ),
            ));
            continue;
        };

        let duplicate = !seen.insert((family, number));
        if duplicate {
            diagnostics.push(Diagnostic::error1(
                identifier.span().start_line(),
                format!(
                    "duplicate item identifier `{identifier_text}` in section `{}`",
                    section_name
                ),
            ));
        }
        if !duplicate {
            if let Some(previous) = last_numbers.get(family) {
                if number < *previous {
                    diagnostics.push(Diagnostic::error1(
                        identifier.span().start_line(),
                        format!(
                            "item identifier `{identifier_text}` in section `{}` must be \
                             greater than `{}{previous}`",
                            section_name,
                            expected_identifier(prefix_rule, family, observed_prefix.as_ref())
                        ),
                    ));
                }
            }
        }
        if last_numbers
            .get(family)
            .is_none_or(|previous| number > *previous)
        {
            last_numbers.insert(family, number);
        }
    }
}

fn validate_item_forms(
    section_name: &str,
    items: &[Item],
    forms: ItemForms,
    diagnostics: &mut Vec<Diagnostic>,
) {
    let Some(first) = items.first() else {
        return;
    };
    let required = match forms {
        ItemForms::Consistent => first.form(),
        ItemForms::ExpandedOnly => ItemForm::Expanded,
        ItemForms::Mixed => return,
    };
    let Some(conflicting) = items.iter().find(|item| item.form() != required) else {
        return;
    };

    match forms {
        ItemForms::Consistent => diagnostics.push(Diagnostic::error1(
            conflicting.span().start_line(),
            format!("itemised section `{section_name}` cannot mix compact and expanded items"),
        )),
        ItemForms::ExpandedOnly => diagnostics.push(Diagnostic::error1(
            conflicting.span().start_line(),
            format!("section `{section_name}` requires expanded items in `### ID: title` form"),
        )),
        ItemForms::Mixed => unreachable!("mixed forms return before comparison"),
    }
}

fn validate_plan_items(section: &Section, rule: PlanItemRule, diagnostics: &mut Vec<Diagnostic>) {
    let section_name = section.title();
    let section = section
        .as_plan_items()
        .expect("a plan-item rule requires a plan-items parser section");
    let mut sequence = ItemSequence::default();

    for item in section.items() {
        validate_plan_item_heading(section_name, item.value(), &mut sequence, diagnostics);
        let statuses: Vec<_> = item
            .value()
            .metadata()
            .iter()
            .filter(|entry| entry.value().key() == "Status")
            .collect();
        if statuses.is_empty() {
            diagnostics.push(Diagnostic::error1(
                item.span().start_line(),
                format!(
                    "plan item in section `{section_name}` is missing required metadata `Status`"
                ),
            ));
        }
        for status in &statuses {
            if !rule.statuses.contains(&status.value().value()) {
                diagnostics.push(Diagnostic::error1(
                    status.span().start_line(),
                    format!(
                        "plan-item metadata `Status` expected {}, but got `{}`",
                        joined_choices(rule.statuses),
                        status.value().value()
                    ),
                ));
            }
        }
        for duplicate in statuses.iter().skip(1) {
            diagnostics.push(Diagnostic::error1(
                duplicate.span().start_line(),
                "duplicate plan-item metadata key `Status`",
            ));
        }
    }
}

fn validate_findings(
    section: &Section,
    rule: FindingsRule,
    prefix_rule: ArtifactPrefixRule,
    observed_prefix: &mut Option<ObservedArtifactPrefix>,
    diagnostics: &mut Vec<Diagnostic>,
) {
    let section_name = section.title();
    let section = section
        .as_findings()
        .expect("a findings rule requires a findings parser section");
    let FindingsBody::Items(items) = section.body() else {
        return;
    };
    let mut sequence = ItemSequence::default();
    for finding in items.value() {
        validate_finding(
            section_name,
            finding,
            prefix_rule,
            rule.family,
            observed_prefix,
            &mut sequence,
            diagnostics,
        );
    }
}

fn validate_finding(
    section_name: &str,
    finding: &Finding,
    prefix_rule: ArtifactPrefixRule,
    family: &'static str,
    observed_prefix: &mut Option<ObservedArtifactPrefix>,
    sequence: &mut ItemSequence,
    diagnostics: &mut Vec<Diagnostic>,
) {
    let Some(identifier) = finding.identifier() else {
        diagnostics.push(Diagnostic::error1(
            finding.span().start_line(),
            format!("finding in section `{section_name}` is missing an identifier"),
        ));
        return;
    };
    let Ok(number) = match_item_identifier(identifier, prefix_rule, family, observed_prefix) else {
        diagnostics.push(Diagnostic::error1(
            identifier.span().start_line(),
            format!(
                "finding identifier `{}` in section `{section_name}` must use \
                 `{}<number>`, where number is from 1 through `2^63 - 1`",
                identifier.text(),
                expected_identifier(prefix_rule, family, observed_prefix.as_ref())
            ),
        ));
        return;
    };
    if !sequence.seen.insert(number) {
        diagnostics.push(Diagnostic::error1(
            identifier.span().start_line(),
            format!(
                "duplicate finding identifier `{}` in section `{section_name}`",
                identifier.text()
            ),
        ));
    } else if sequence
        .last_number
        .is_some_and(|previous| number < previous)
    {
        diagnostics.push(Diagnostic::error1(
            identifier.span().start_line(),
            format!(
                "finding identifier `{}` must be greater than `{}{}`",
                identifier.text(),
                expected_identifier(prefix_rule, family, observed_prefix.as_ref()),
                sequence.last_number.expect("previous number exists")
            ),
        ));
    }
    if sequence
        .last_number
        .is_none_or(|previous| number > previous)
    {
        sequence.last_number = Some(number);
    }
}

fn validate_plan_item_heading(
    section_name: &str,
    item: &PlanItem,
    sequence: &mut ItemSequence,
    diagnostics: &mut Vec<Diagnostic>,
) {
    if item.title().text().is_empty() {
        diagnostics.push(Diagnostic::error1(
            item.span().start_line(),
            format!("plan item in section `{section_name}` is missing a title"),
        ));
    }
    let Some(identifier) = item.identifier() else {
        diagnostics.push(Diagnostic::error1(
            item.span().start_line(),
            format!("plan item in section `{section_name}` is missing an identifier"),
        ));
        return;
    };
    let Some(number) = identifier.text().strip_prefix('P').and_then(valid_number) else {
        diagnostics.push(Diagnostic::error1(
            identifier.span().start_line(),
            format!(
                "plan item identifier `{}` in section `{section_name}` must use \
                 `P<number>`, where number is from 1 through `2^63 - 1`",
                identifier.text()
            ),
        ));
        return;
    };
    let duplicate = !sequence.seen.insert(number);
    if duplicate {
        diagnostics.push(Diagnostic::error1(
            identifier.span().start_line(),
            format!("duplicate plan item identifier `{}`", identifier.text()),
        ));
    } else if sequence
        .last_number
        .is_some_and(|previous| number < previous)
    {
        diagnostics.push(Diagnostic::error1(
            identifier.span().start_line(),
            format!(
                "plan item identifier `{}` must be greater than `P{}`",
                identifier.text(),
                sequence.last_number.expect("previous number exists")
            ),
        ));
    }
    if sequence
        .last_number
        .is_none_or(|previous| number > previous)
    {
        sequence.last_number = Some(number);
    }
}

fn joined_choices(choices: &[&str]) -> String {
    choices
        .iter()
        .map(|choice| format!("`{choice}`"))
        .collect::<Vec<_>>()
        .join(" or ")
}

fn expected_identifier(
    prefix: ArtifactPrefixRule,
    family: &str,
    observed: Option<&ObservedArtifactPrefix>,
) -> String {
    if let Some(observed) = observed {
        return format!("{}{family}", observed.text);
    }
    match prefix {
        ArtifactPrefixRule::Known(prefix) => format!("{prefix}{family}"),
        ArtifactPrefixRule::Unknown(ArtifactPrefixShape::Step) => {
            format!("S<number>-{family}")
        }
        ArtifactPrefixRule::Unknown(ArtifactPrefixShape::Review) => {
            format!("S<number>-R<number>-{family}")
        }
    }
}

fn match_item_identifier(
    identifier: &Located<&str>,
    prefix_rule: ArtifactPrefixRule,
    family: &str,
    observed: &mut Option<ObservedArtifactPrefix>,
) -> Result<i64, ()> {
    let text = identifier.text();
    let (prefix_end, components) = match prefix_rule {
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
        .and_then(valid_number)
        .ok_or(())?;

    if matches!(prefix_rule, ArtifactPrefixRule::Unknown(_)) {
        let prefix_text = &text[..prefix_end];
        if let Some(previous) = observed.as_ref() {
            if previous.text != prefix_text {
                return Err(());
            }
        } else {
            let components = components
                .into_iter()
                .map(|(value, range)| {
                    Located::new(
                        value,
                        SourceSpan::new(
                            identifier
                                .span()
                                .start()
                                .advance(&prefix_text[..range.start]),
                            identifier.span().start().advance(&prefix_text[..range.end]),
                        ),
                    )
                })
                .collect();
            *observed = Some(ObservedArtifactPrefix {
                text: prefix_text.to_owned(),
                components,
            });
        }
    }
    Ok(number)
}

fn parse_unknown_prefix(identifier: &str, shape: ArtifactPrefixShape) -> Result<ParsedPrefix, ()> {
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

fn number_at(identifier: &str, start: usize) -> Option<(i64, Range<usize>)> {
    let end = start
        + identifier[start..]
            .bytes()
            .take_while(u8::is_ascii_digit)
            .count();
    let range = start..end;
    valid_number(&identifier[range.clone()]).map(|number| (number, range))
}

fn valid_number(number: &str) -> Option<i64> {
    let mut bytes = number.bytes();
    if !matches!(bytes.next(), Some(b'1'..=b'9')) {
        return None;
    }
    if !bytes.all(|byte| byte.is_ascii_digit()) {
        return None;
    }
    number.parse().ok()
}

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

#[cfg(test)]
mod tests;
