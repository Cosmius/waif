use std::collections::HashMap;
use std::ops::Range;

use chrono::{DateTime, Local, SecondsFormat};

#[derive(Clone)]
pub(crate) struct MacroContext {
    now: DateTime<Local>,
}

impl MacroContext {
    pub(crate) fn new() -> Self {
        Self { now: Local::now() }
    }
}

type MacroResolver = fn(&MacroContext) -> Result<String, String>;

struct MacroDefinition {
    literal: &'static str,
    help: &'static str,
    resolver: MacroResolver,
}

const CATALOGUE: &[MacroDefinition] = &[MacroDefinition {
    literal: "__NOW__",
    help: "one local RFC 3339 timestamp per invocation",
    resolver: resolve_now,
}];

fn resolve_now(context: &MacroContext) -> Result<String, String> {
    Ok(context.now.to_rfc3339_opts(SecondsFormat::Secs, false))
}

#[derive(Clone, Debug)]
pub(crate) struct MacroOccurrence {
    pub(crate) literal: &'static str,
    pub(crate) resolver: MacroResolver,
    pub(crate) range: Range<usize>,
}

pub(crate) fn scan(source: &str) -> Vec<MacroOccurrence> {
    let bytes = source.as_bytes();
    let mut occurrences = Vec::new();

    for definition in CATALOGUE {
        for (start, _) in source.match_indices(definition.literal) {
            let end = start + definition.literal.len();
            let enclosed =
                bytes.get(start.wrapping_sub(1)) == Some(&b'`') && bytes.get(end) == Some(&b'`');
            if !enclosed {
                occurrences.push(MacroOccurrence {
                    literal: definition.literal,
                    resolver: definition.resolver,
                    range: start..end,
                });
            }
        }
    }

    occurrences.sort_by_key(|occurrence| occurrence.range.start);
    occurrences
}

pub(crate) fn fill_help() -> String {
    let mut help = String::from(indoc::indoc! {"
        Example:
          waif fill .waif/current_task/goal.md

        Known macros:
    "});
    for definition in CATALOGUE {
        help.push_str(&format!("  {:<9}{}\n", definition.literal, definition.help));
    }
    help.push_str(indoc::indoc! {"

        An occurrence immediately enclosed by backticks is not replaced. A BAD candidate leaves the target unchanged and is saved to a recovery path."
    });
    help
}

pub(crate) fn expand(source: &str) -> Result<Option<String>, String> {
    let occurrences = scan(source);
    if occurrences.is_empty() {
        return Ok(None);
    }

    let context = MacroContext::new();
    let mut resolutions = HashMap::<&str, String>::new();
    let mut candidate = String::with_capacity(source.len());
    let mut copied_until = 0;

    for occurrence in occurrences {
        candidate.push_str(&source[copied_until..occurrence.range.start]);
        let replacement = match resolutions.get(occurrence.literal) {
            None => {
                let replacement = (occurrence.resolver)(&context)?;
                resolutions.insert(occurrence.literal, replacement);
                resolutions
                    .get(occurrence.literal)
                    .expect("resolution was just added")
            }
            Some(v) => v,
        };
        candidate.push_str(replacement);
        copied_until = occurrence.range.end;
    }
    candidate.push_str(&source[copied_until..]);

    Ok(Some(candidate))
}

#[cfg(test)]
mod tests;
