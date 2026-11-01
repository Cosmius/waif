use super::*;
use crate::parser::Severity;

const PASS: &str = concat!(
    "# Implementation Review 1\n",
    "- Step: ./step.md\n",
    "- Decision: pass\n",
    "- Date: 2026-08-02T13:00:00+09:00\n",
    "- Reviewer: independent subagent\n",
    "- Workspace state: uncommitted\n",
    "## Findings\nNo findings.\n",
    "## Scope\n- S4-R1-SC1: review checker\n",
    "## Validation\n- S4-R1-V1: tests passed\n",
    "## Residual Risks\n- S4-R1-RR1: None.\n",
);

#[test]
fn accepts_pass_and_changes_requested_reviews() {
    assert!(check(Path::new("steps/04-check/review1.md"), PASS).is_empty());

    let changes = PASS
        .replace("- Decision: pass", "- Decision: changes-requested")
        .replace(
            "No findings.",
            "### S4-R1-F1: Defect\n\nOpaque severity and body.",
        );
    assert!(check(Path::new("steps/04-check/review1.md"), &changes).is_empty());
}

#[test]
fn aggregates_metadata_section_and_decision_errors() {
    let source = PASS
        .replace("- Step: ./step.md\n", "- Step: wrong\n")
        .replace(
            "- Decision: pass\n",
            "- Reviewer: robot\n- Decision: pass\n",
        )
        .replace("- Reviewer: independent subagent\n", "")
        .replace("- Workspace state: uncommitted", "- Workspace state: dirty")
        .replace("No findings.", "### S5-R2-F1: defect")
        .replace("## Scope", "## Scope\n## Findings\nNo findings.\n## Scope");
    let diagnostics = check(Path::new("steps/bad/review2.md"), &source);
    let messages = diagnostics
        .iter()
        .map(Diagnostic::message)
        .collect::<Vec<_>>()
        .join("\n");

    for expected in [
        "metadata `Step` expected `./step.md`",
        "metadata `Reviewer` expected",
        "metadata `Workspace state` expected `uncommitted`",
        "duplicate section `Findings`",
        "duplicate section `Scope`",
        "decision `pass` requires",
        "cannot resolve a step number",
    ] {
        assert!(
                messages.contains(expected),
                "missing {expected}: {messages}"
            );
    }
    assert!(diagnostics
            .iter()
            .any(|diagnostic| diagnostic.severity() == Severity::Error));
}

#[test]
fn validates_title_filename_path_and_prefix_identities() {
    let source = PASS
        .replace("Review 1", "Review 3")
        .replace("S4-R1-", "S5-R2-");
    let diagnostics = check(Path::new("steps/04-check/review2.md"), &source);
    let messages = diagnostics
        .iter()
        .map(Diagnostic::message)
        .collect::<Vec<_>>()
        .join("\n");

    assert!(messages.contains("does not match containing directory"));
    assert!(messages.contains("does not match title"));
    assert!(messages.contains("review title number `3` does not match filename `2`"));
}

#[test]
fn rejects_empty_findings_and_mixed_review_prefixes() {
    let empty = PASS.replace("No findings.\n", "");
    let diagnostics = check(Path::new("steps/04-check/review1.md"), &empty);
    assert!(diagnostics.iter().any(|diagnostic| diagnostic
            .message()
            .contains("section `Findings` must contain")));

    let mixed = PASS.replace("S4-R1-V1", "S4-R2-V1");
    let diagnostics = check(Path::new("steps/04-check/review1.md"), &mixed);
    assert!(diagnostics
            .iter()
            .any(|diagnostic| diagnostic.message().contains("item identifier `S4-R2-V1`")));
}
