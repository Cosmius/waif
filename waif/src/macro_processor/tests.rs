use super::*;

#[test]
fn macro_scanner_finds_occurrences_in_source_order() {
    let source = "é__NOW__ and __NOW____NOW__";
    let found = scan(source);

    assert_eq!(found.len(), 3);
    assert_eq!(&source[found[0].range.clone()], "__NOW__");
    assert_eq!(found[0].range, 2..9);
    assert_eq!(found[1].range, 14..21);
    assert_eq!(found[2].range, 21..28);
    assert!(found.iter().all(|found| found.literal == "__NOW__"));
}

#[test]
fn macro_scanner_uses_only_immediately_adjacent_backticks() {
    let source = concat!(
        "`__NOW__` ",
        "before`__NOW__`after ",
        "`__NOW__ suffix ",
        "prefix __NOW__` ",
        "``__NOW__``",
    );
    let found = scan(source);

    assert_eq!(found.len(), 2);
    assert_eq!(&source[found[0].range.clone()], "__NOW__");
    assert_eq!(&source[found[1].range.clone()], "__NOW__");
}

#[test]
fn macro_scanner_returns_no_unknown_or_excluded_occurrences() {
    assert!(scan("plain __LATER__ text").is_empty());
    assert!(scan("`__NOW__`").is_empty());
}

#[test]
fn macro_resolver_uses_the_shared_context_value() {
    let found = scan("__NOW__ and __NOW__");
    let context = MacroContext::new();
    let first = (found[0].resolver)(&context).expect("macro should resolve");
    let second = (found[1].resolver)(&context).expect("macro should resolve");

    assert_eq!(first, second);
    assert!(DateTime::parse_from_rfc3339(&first).is_ok());
}

#[test]
fn macro_expansion_preserves_text_and_uses_one_timestamp() {
    let source = "é __NOW____NOW__ `__NOW__` tail";
    let candidate = expand(source)
        .expect("macros should resolve")
        .expect("macros should be found");
    let expanded = candidate
        .strip_prefix("é ")
        .and_then(|candidate| candidate.strip_suffix(" `__NOW__` tail"))
        .expect("surrounding source should be preserved");
    let midpoint = expanded.len() / 2;

    assert_eq!(&expanded[..midpoint], &expanded[midpoint..]);
    assert!(DateTime::parse_from_rfc3339(&expanded[..midpoint]).is_ok());
}

#[test]
fn macro_expansion_reports_no_work_without_changing_source() {
    assert_eq!(expand("plain `__NOW__` text").expect("scan should work"), None);
}
