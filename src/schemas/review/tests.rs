use super::*;
use crate::parser::Severity;

const CONCERNS: &str = concat!(
    "# Review 2\n",
    "\n",
    "- Step: ./step.md\n",
    "- Date: 2026-11-07T21:00:00+09:00\n",
    "- Reviewer: independent subagent\n",
    "- Verdict: concerns\n",
    "- Reviewed: 0123456789abcdef0123456789abcdef01234567-WORKING\n",
    "- Reviewed against: 0123456789abcdef0123456789abcdef01234567 (HEAD, waif/task)\n",
    "\n",
    "## Summary\n\nConcerns remain.\n\nThe changes add a checker.\n\n",
    "## Findings\n\n",
    "### S1-R2-F1: Still open from S1-R1-F1\n",
    "- Severity: medium\n",
    "- Location: `src/lib.rs:42`\n",
    "- Problem: It is wrong.\n",
    "- Impact: Checks fail.\n",
    "- Recommendation: Fix it.\n\n",
    "### S1-R2-F3: New finding\n",
    "- Severity: low\n",
    "- Location: `src/lib.rs:57`\n",
    "- Problem: It is unclear.\n",
    "- Impact: Readers stumble.\n",
    "- Recommendation: Clarify it.\n\n",
    "## Assessment\n\n",
    "- S1-DC1: met - tested\n",
    "- S1-DC2: unmet - see S1-R2-F1\n\n",
    "## Review History\n\n",
    "- `review1.md`: concerns - S1-R1-F2 fixed; S1-R1-F1 still open as S1-R2-F1\n\n",
    "## Validation\n\n- S1-R2-V1: `cargo test`: passed\n\n",
    "## Residual Risks\n\n- S1-R2-RR1: None.\n",
);

const PASS: &str = concat!(
    "# Review 1\n",
    "- Step: ./step.md\n",
    "- Date: 2026-11-07T21:00:00+09:00\n",
    "- Reviewer: current session\n",
    "- Verdict: pass\n",
    "- Reviewed: 0123456789abcdef0123456789abcdef01234567-STAGING\n",
    "- Reviewed against: 0123456789abcdef0123456789abcdef01234567\n",
    "## Summary\nPasses.\n",
    "## Findings\nNo findings.\n",
    "## Assessment\n",
    "- S1-DC1: met - tested\n",
    "- The error names the file: met\n",
    "## Validation\n- S1-R1-V1: `cargo test`: passed\n",
    "## Residual Risks\n- S1-R1-RR1: none known\n",
);

fn messages(diagnostics: &[Diagnostic]) -> String {
    diagnostics
        .iter()
        .map(Diagnostic::message)
        .collect::<Vec<_>>()
        .join("\n")
}

#[test]
fn accepts_a_concerns_review_shaped_like_the_template() {
    let diagnostics = check(Path::new("steps/01-name/review2.md"), CONCERNS);
    assert!(diagnostics.is_empty(), "{}", messages(&diagnostics));
}

#[test]
fn accepts_a_first_pass_review() {
    let diagnostics = check(Path::new("steps/01-name/review1.md"), PASS);
    assert!(diagnostics.is_empty(), "{}", messages(&diagnostics));
}

#[test]
fn accepts_a_sentinel_surrounded_by_spaces_and_a_tab() {
    let padded = PASS.replace("No findings.", " \tNo findings.  ");
    assert!(check(Path::new("steps/01-name/review1.md"), &padded).is_empty());
}

#[test]
fn accepts_a_finding_with_a_waiver_and_counter_opinion() {
    let source = CONCERNS.replace(
        "- Recommendation: Clarify it.\n",
        concat!(
            "- Recommendation: Clarify it.\n",
            "- Waived: Accepted as is.\n",
            "- Counter-opinion: Still unclear.\n",
        ),
    );
    let diagnostics = check(Path::new("steps/01-name/review2.md"), &source);
    assert!(diagnostics.is_empty(), "{}", messages(&diagnostics));
}

#[test]
fn rejects_a_malformed_reviewed_reference() {
    let source = CONCERNS.replace(
        "- Reviewed: 0123456789abcdef0123456789abcdef01234567-WORKING\n",
        "- Reviewed: 0123456789abcdef0123456789abcdef01234567-DIRTY (HEAD)\n",
    );
    let diagnostics = check(Path::new("steps/01-name/review2.md"), &source);
    let reported = messages(&diagnostics);
    assert!(reported.contains("metadata `Reviewed` expected a full 40-character"), "{reported}");
}

#[test]
fn rejects_an_unknown_verdict_and_a_missing_date() {
    let source = PASS
        .replace("- Verdict: pass", "- Verdict: approve")
        .replace("- Date: 2026-11-07T21:00:00+09:00\n", "");
    let reported = messages(&check(Path::new("steps/01-name/review1.md"), &source));
    assert!(reported.contains("expected `pass` or `concerns`, but got `approve`"), "{reported}");
    assert!(reported.contains("missing required metadata `Date`"), "{reported}");
}

#[test]
fn validates_filename_path_and_prefix_identities() {
    let source = PASS.replace("S1-R1-", "S5-R2-");
    let reported = messages(&check(Path::new("steps/01-name/review1.md"), &source));

    assert!(reported.contains("step identity `5` does not match containing directory"), "{reported}");
    assert!(reported.contains("review identity `2` does not match filename"), "{reported}");
}

#[test]
fn accepts_a_free_text_title() {
    let source = PASS.replace("# Review 1", "# Second look at the parser");
    let diagnostics = check(Path::new("steps/01-name/review1.md"), &source);
    assert!(diagnostics.is_empty(), "{}", messages(&diagnostics));
}

#[test]
fn warns_when_the_containing_directory_has_no_step_number() {
    let diagnostics = check(Path::new("elsewhere/review1.md"), PASS);
    let reported = messages(&diagnostics);
    assert!(reported.contains("cannot resolve a step number from the containing directory"), "{reported}");
    assert!(diagnostics.iter().all(|diagnostic| diagnostic.severity() == Severity::Warning), "{reported}");
}

#[test]
fn accepts_conventions_that_checking_does_not_enforce() {
    let source = PASS
        .replace("- Step: ./step.md\n", "- Step: elsewhere\n")
        .replace("- Reviewer: current session\n", "")
        .replace(
            "No findings.\n",
            "### S1-R1-F1: Loose finding\n- Severity: catastrophic\n",
        )
        .replace(
            "## Validation\n",
            "## Review History\nNone.\n## Validation\n",
        );
    let diagnostics = check(Path::new("steps/01-name/review1.md"), &source);
    assert!(diagnostics.is_empty(), "{}", messages(&diagnostics));
}

#[test]
fn rejects_missing_summary_and_assessment() {
    let source = PASS.replace("## Summary\nPasses.\n", "").replace(
        "## Assessment\n- S1-DC1: met - tested\n- The error names the file: met\n",
        "",
    );
    let reported = messages(&check(Path::new("steps/01-name/review1.md"), &source));
    assert!(reported.contains("missing required section `Summary`"), "{reported}");
    assert!(reported.contains("missing required section `Assessment`"), "{reported}");
}

#[test]
fn rejects_empty_findings_and_mixed_review_prefixes() {
    let empty = PASS.replace("No findings.\n", "");
    let diagnostics = check(Path::new("steps/01-name/review1.md"), &empty);
    assert!(diagnostics.iter().any(|diagnostic| diagnostic
            .message()
            .contains("section `Findings` must not be empty")));

    let mixed = PASS.replace("S1-R1-RR1", "S1-R2-RR1");
    let diagnostics = check(Path::new("steps/01-name/review1.md"), &mixed);
    assert!(diagnostics
            .iter()
            .any(|diagnostic| diagnostic.message().contains("item identifier `S1-R2-RR1`")));
}
