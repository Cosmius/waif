use std::cmp::Ordering;

use super::SchemaVersion;

fn version(text: &str) -> SchemaVersion {
    SchemaVersion::parse(text).expect("version should parse")
}

#[test]
fn parses_major_minor_with_an_optional_lowercase_suffix() {
    for text in ["0.4", "0.4-dev", "10.0-rc"] {
        assert!(SchemaVersion::parse(text).is_some(), "`{text}` should parse");
    }
    for text in [
        "", "0", "0.4.0", "0.", ".4", "0..4", "00.4", "0.04", "v0.4", "-dev", "+0.4", "0.+4",
        "0.4-", "0.4-Dev", "0.4-dev1",
    ] {
        assert!(SchemaVersion::parse(text).is_none(), "`{text}` should not parse");
    }
}

#[test]
fn orders_numerically_with_suffixed_versions_first() {
    for (lower, higher) in [
        ("0.4-dev", "0.4"),
        ("0.3", "0.4-dev"),
        ("0.9", "0.10"),
        ("0.4-alpha", "0.4-dev"),
    ] {
        assert_eq!(version(lower).cmp(&version(higher)), Ordering::Less, "`{lower}` < `{higher}`");
    }
}
