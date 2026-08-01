use std::collections::{HashMap, HashSet};

use chrono::DateTime;

use crate::artifact::{Artifact, Item, ItemForm, PlanItem, Section};
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
    pub(crate) prefix: &'static str,
    pub(crate) expanded_prefix: Option<&'static str>,
    pub(crate) forms: ItemForms,
}

impl ItemRule {
    pub(crate) const fn new(prefix: &'static str) -> Self {
        Self {
            prefix,
            expanded_prefix: None,
            forms: ItemForms::Consistent,
        }
    }

    pub(crate) const fn with_expanded_prefix(mut self, prefix: &'static str) -> Self {
        self.expanded_prefix = Some(prefix);
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
    pub(crate) prefix: &'static str,
    pub(crate) statuses: &'static [&'static str],
}

#[derive(Clone, Copy)]
pub(crate) struct SectionRule {
    pub(crate) name: &'static str,
    pub(crate) required: bool,
    pub(crate) items: Option<ItemRule>,
    pub(crate) plan_items: Option<PlanItemRule>,
}

impl SectionRule {
    pub(crate) const fn new(name: &'static str) -> Self {
        Self {
            name,
            required: true,
            items: None,
            plan_items: None,
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
}

pub(crate) struct Schema {
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
    let mut diagnostics = Vec::new();
    validate_metadata(artifact, schema, &mut diagnostics);
    validate_sections(artifact, schema, &mut diagnostics);
    diagnostics
}

fn validate_metadata(artifact: &Artifact, schema: &Schema, diagnostics: &mut Vec<Diagnostic>) {
    let mut seen = HashSet::new();

    for metadata in artifact.metadata() {
        let line = metadata.span().start_line();
        let metadata = metadata.value();
        let key = metadata.key();
        if !seen.insert(key) {
            diagnostics.push(Diagnostic::error(
                line,
                format!("duplicate metadata key `{key}`"),
            ));
        }

        if let Some(rule) = schema.metadata.iter().find(|rule| rule.name == key) {
            if let Err(message) = (rule.validator)(metadata.value()) {
                diagnostics.push(Diagnostic::error(
                    line,
                    format!("metadata `{}` {message}", rule.name),
                ));
            }
        }
    }

    for rule in schema.metadata {
        if !seen.contains(rule.name) {
            diagnostics.push(Diagnostic::error(
                1,
                format!("missing required metadata `{}`", rule.name),
            ));
        }
    }
}

fn validate_sections(artifact: &Artifact, schema: &Schema, diagnostics: &mut Vec<Diagnostic>) {
    let mut seen_sections = HashSet::new();
    let mut seen_ids = HashSet::new();
    let mut last_numbers = HashMap::new();
    let mut greatest_known: Option<(usize, &str)> = None;

    for section in artifact.sections() {
        let name = section.name();
        let line = section.span().start_line();
        if !seen_sections.insert(name) {
            diagnostics.push(Diagnostic::error(
                line,
                format!("duplicate section `{name}`"),
            ));
        }
        if section_is_empty(section) {
            diagnostics.push(Diagnostic::warning(
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
                diagnostics.push(Diagnostic::error(
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
                &mut seen_ids,
                &mut last_numbers,
                diagnostics,
            );
        }
        if let Some(plan_item_rule) = rule.plan_items {
            validate_plan_items(section, plan_item_rule, diagnostics);
        }
    }

    for rule in schema.sections.iter().filter(|rule| rule.required) {
        if !seen_sections.contains(rule.name) {
            diagnostics.push(Diagnostic::error(
                1,
                format!("missing required section `{}`", rule.name),
            ));
        }
    }
}

fn validate_items(
    section: &Section,
    rule: ItemRule,
    seen: &mut HashSet<(&'static str, i64)>,
    last_numbers: &mut HashMap<&'static str, i64>,
    diagnostics: &mut Vec<Diagnostic>,
) {
    let section_name = section.name();
    let section = section
        .as_itemised()
        .expect("an item rule requires an itemised parser section");
    validate_item_forms(section_name, section.items(), rule.forms, diagnostics);

    for item in section.items() {
        let Some(identifier) = item.identifier() else {
            diagnostics.push(Diagnostic::error(
                item.span().start_line(),
                format!(
                    "item in section `{}` is missing an identifier; expected `{}<number>`",
                    section_name, rule.prefix
                ),
            ));
            continue;
        };
        let identifier_text = identifier.text();
        let prefix = match item.form() {
            ItemForm::Compact => rule.prefix,
            ItemForm::Expanded => rule.expanded_prefix.unwrap_or(rule.prefix),
        };
        let Some(number) = valid_item_number(identifier_text, prefix) else {
            diagnostics.push(Diagnostic::error(
                identifier.span().start_line(),
                format!(
                    "item identifier `{identifier_text}` in section `{}` must use \
                     `{}<number>`, where number is from 1 through `2^63 - 1`",
                    section_name, prefix
                ),
            ));
            continue;
        };

        let duplicate = !seen.insert((prefix, number));
        if duplicate {
            diagnostics.push(Diagnostic::error(
                identifier.span().start_line(),
                format!(
                    "duplicate item identifier `{identifier_text}` in section `{}`",
                    section_name
                ),
            ));
        }
        if !duplicate {
            if let Some(previous) = last_numbers.get(prefix) {
                if number < *previous {
                    diagnostics.push(Diagnostic::error(
                        identifier.span().start_line(),
                        format!(
                            "item identifier `{identifier_text}` in section `{}` must be \
                             greater than `{}{previous}`",
                            section_name, prefix
                        ),
                    ));
                }
            }
        }
        if last_numbers
            .get(prefix)
            .is_none_or(|previous| number > *previous)
        {
            last_numbers.insert(prefix, number);
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
        ItemForms::Consistent => diagnostics.push(Diagnostic::error(
            conflicting.span().start_line(),
            format!("itemised section `{section_name}` cannot mix compact and expanded items"),
        )),
        ItemForms::ExpandedOnly => diagnostics.push(Diagnostic::error(
            conflicting.span().start_line(),
            format!("section `{section_name}` requires expanded items in `### ID: title` form"),
        )),
        ItemForms::Mixed => unreachable!("mixed forms return before comparison"),
    }
}

fn validate_plan_items(section: &Section, rule: PlanItemRule, diagnostics: &mut Vec<Diagnostic>) {
    let section_name = section.name();
    let section = section
        .as_plan_items()
        .expect("a plan-item rule requires a plan-items parser section");
    let mut seen = HashSet::new();
    let mut last_number = None;

    for item in section.items() {
        validate_plan_item_heading(
            section_name,
            item,
            rule.prefix,
            &mut seen,
            &mut last_number,
            diagnostics,
        );
        let statuses: Vec<_> = item
            .metadata()
            .iter()
            .filter(|entry| entry.value().key() == "Status")
            .collect();
        if statuses.is_empty() {
            diagnostics.push(Diagnostic::error(
                item.span().start_line(),
                format!(
                    "plan item in section `{section_name}` is missing required metadata `Status`"
                ),
            ));
        }
        for status in &statuses {
            if !rule.statuses.contains(&status.value().value()) {
                diagnostics.push(Diagnostic::error(
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
            diagnostics.push(Diagnostic::error(
                duplicate.span().start_line(),
                "duplicate plan-item metadata key `Status`",
            ));
        }
    }
}

fn validate_plan_item_heading(
    section_name: &str,
    item: &PlanItem,
    prefix: &'static str,
    seen: &mut HashSet<i64>,
    last_number: &mut Option<i64>,
    diagnostics: &mut Vec<Diagnostic>,
) {
    if item.title().text().is_empty() {
        diagnostics.push(Diagnostic::error(
            item.span().start_line(),
            format!("plan item in section `{section_name}` is missing a title"),
        ));
    }
    let Some(identifier) = item.identifier() else {
        diagnostics.push(Diagnostic::error(
            item.span().start_line(),
            format!("plan item in section `{section_name}` is missing an identifier"),
        ));
        return;
    };
    let Some(number) = valid_item_number(identifier.text(), prefix) else {
        diagnostics.push(Diagnostic::error(
            identifier.span().start_line(),
            format!(
                "plan item identifier `{}` in section `{section_name}` must use \
                 `{prefix}<number>`, where number is from 1 through `2^63 - 1`",
                identifier.text()
            ),
        ));
        return;
    };
    let duplicate = !seen.insert(number);
    if duplicate {
        diagnostics.push(Diagnostic::error(
            identifier.span().start_line(),
            format!("duplicate plan item identifier `{}`", identifier.text()),
        ));
    } else if last_number.is_some_and(|previous| number < previous) {
        diagnostics.push(Diagnostic::error(
            identifier.span().start_line(),
            format!(
                "plan item identifier `{}` must be greater than `{prefix}{}`",
                identifier.text(),
                last_number.expect("previous number exists")
            ),
        ));
    }
    if last_number.is_none_or(|previous| number > previous) {
        *last_number = Some(number);
    }
}

fn joined_choices(choices: &[&str]) -> String {
    choices
        .iter()
        .map(|choice| format!("`{choice}`"))
        .collect::<Vec<_>>()
        .join(" or ")
}

fn valid_item_number(identifier: &str, prefix: &str) -> Option<i64> {
    let number = identifier.strip_prefix(prefix)?;
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
    }
}
