use super::*;
use crate::parser::Severity;

const VALID: &str = concat!(
    "# Step 3: Check steps\n",
    "- Status: accepted\n",
    "- Source commit: not-created\n",
    "- Created: 2026-08-02T12:00:00+09:00\n",
    "- Updated: 2026-08-02T12:01:00+09:00\n",
    "- Plan items: P2\n- Goal criteria: opaque\n",
    "## Objective\ntext\n",
    "## Plan Item Coverage\n- S3-PC2: P2 - complete - checker\n",
    "## Context\ntext\n",
    "## Done When\n- S3-D1: done\n",
);

fn messages(path: &str, source: &str) -> Vec<(Severity, usize, String)> {
    check(Path::new(path), source)
        .into_iter()
        .map(|diagnostic| {
            (
                diagnostic.severity(),
                diagnostic.line(),
                diagnostic.message().to_owned(),
            )
        })
        .collect()
}

#[test]
fn accepts_conforming_step_and_opaque_semantic_metadata() {
    assert_eq!(messages("steps/03-check/step.md", VALID), []);
    let expanded = VALID.replace(
        "- S3-D1: done",
        "### S3-D1: done\n\nOpaque body with `## heading`.",
    );
    assert_eq!(messages("steps/03-check/step.md", &expanded), []);
}

#[test]
fn aggregates_lifecycle_coverage_order_and_identity_errors() {
    let source = VALID
        .replace("- Status: accepted", "- Updated: invalid\n- Status: done")
        .replace("- Updated: 2026-08-02T12:01:00+09:00\n", "")
        .replace("not-created", "abc")
        .replace("S3-PC2: P2", "S4-PC2: P3")
        .replace("S3-D1", "S4-D1");
    let found = messages("steps/03-check/step.md", &source);
    assert!(found.len() >= 5, "{found:?}");
    assert!(found.iter().any(|entry| entry.2.contains("incompatible")));
    assert!(found
            .iter()
            .any(|entry| entry.2.contains("must match covered")));
    assert!(found
            .iter()
            .any(|entry| entry.2.contains("containing directory")));
    assert!(found
            .iter()
            .any(|entry| { entry.1 == 11 && entry.2.contains("containing directory") }));
}

#[test]
fn malformed_directory_warns_but_keeps_internal_prefix_checks() {
    let found = messages("steps/not-a-step/step.md", VALID);
    assert_eq!(found.len(), 1);
    assert_eq!(found[0].0, Severity::Warning);
    assert!(found[0].2.contains("cannot resolve a step number"));

    let mixed = VALID.replace("S3-D1", "S4-D1");
    let found = messages("steps/not-a-step/step.md", &mixed);
    assert!(found.iter().any(|entry| entry.0 == Severity::Error));
}

#[test]
fn requires_a_nonempty_but_otherwise_opaque_directory_suffix() {
    let found = messages("steps/03-/step.md", VALID);
    assert_eq!(found.len(), 1);
    assert_eq!(found[0].0, Severity::Warning);
    assert!(found[0].2.contains("short name is empty"));
    let mut diagnostics = Vec::new();
    assert_eq!(
            step_directory_number(&mut diagnostics, Path::new("steps/03-/step.md")),
            Some(3)
        );
    assert_eq!(diagnostics.len(), 1);

    for path in ["steps/03-Bad/step.md", "steps/03-a--b/step.md"] {
        assert_eq!(messages(path, VALID), [], "{path}");
    }
}

#[test]
fn validates_title_directory_and_plain_done_when() {
    for source in [
        VALID.replace("Step 3: Check steps", "Step 03: Check steps"),
        VALID.replace("Step 3: Check steps", "Step 3:"),
        VALID.replace("- S3-D1: done", "- [ ] S3-D1: done"),
    ] {
        assert!(messages("steps/03-check/step.md", &source)
                .iter()
                .any(|entry| entry.0 == Severity::Error));
    }
    assert!(messages("steps/04-check/step.md", VALID)
            .iter()
            .any(|entry| entry.2.contains("title number")));
}

#[test]
fn validates_optional_section_order_forms_and_numeric_bounds() {
    let out_of_order = VALID.replace(
        "## Done When\n- S3-D1: done",
        "## Changes\n- S3-C1: change\n## Done When\n- S3-D1: done",
    );
    assert!(messages("steps/03-check/step.md", &out_of_order)
            .iter()
            .any(|entry| entry.2.contains("must appear before")));

    let compact_revision = format!("{VALID}## Revisions\n- S3-REV1: compact\n");
    assert!(messages("steps/03-check/step.md", &compact_revision)
            .iter()
            .any(|entry| entry.2.contains("requires expanded items")));

    let overflow = VALID.replace("S3-D1", "S9223372036854775808-D1");
    assert!(messages("steps/03-check/step.md", &overflow)
            .iter()
            .any(|entry| entry.2.contains("2^63 - 1")));
}

#[test]
fn keeps_fenced_headings_opaque_and_uses_nominal_paths() {
    let fenced = VALID.replace(
        "## Context\ntext",
        "## Context\n```markdown\n## Done When\n- S99-D1: fenced\n```",
    );
    assert_eq!(messages("steps/03-check/step.md", &fenced), []);

    let mut diagnostics = Vec::new();
    assert_eq!(
            step_directory_number(&mut diagnostics, Path::new("linked/03-check/step.md")),
            Some(3)
        );
    assert_eq!(
            step_directory_number(&mut diagnostics, Path::new("real/99-target/step.md")),
            Some(99)
        );
    assert!(diagnostics.is_empty());
}
