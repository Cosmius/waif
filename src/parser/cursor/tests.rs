use super::*;

mod navigation {
    use super::*;

    #[test]
    fn peeks_without_consuming() {
        let mut cursor = Cursor::new("ab");

        assert_eq!(cursor.peek(), Some('a'));
        assert_eq!(cursor.peek(), Some('a'));
        assert_eq!(cursor.position().offset(), 0);
        assert_eq!(cursor.take(), Some('a'));
        assert_eq!(cursor.peek(), Some('b'));
    }

    #[test]
    fn rewinds_to_a_previous_position() {
        let mut cursor = Cursor::new("a阿\r\nb");
        cursor.take();
        let checkpoint = cursor.position();
        cursor.take();
        cursor.take();

        cursor.rewind(checkpoint);

        assert_eq!(cursor.position(), checkpoint);
        assert_eq!(cursor.peek(), Some('阿'));
        assert_eq!(cursor.take(), Some('阿'));
        assert_eq!(cursor.peek(), Some('\n'));
    }

    #[test]
    fn takes_characters_while_the_predicate_matches() {
        let mut cursor = Cursor::new("key:value");

        assert_eq!(cursor.take_while(|ch| ch != ':'), "key");
        assert_eq!(cursor.peek(), Some(':'));
    }

    #[test]
    fn takes_one_character_only_when_the_predicate_matches() {
        let mut cursor = Cursor::new(":value");

        assert_eq!(cursor.take_if(|ch| ch == '-'), None);
        assert_eq!(cursor.position().offset(), 0);
        assert_eq!(cursor.take_if(|ch| ch == ':'), Some(':'));
        assert_eq!(cursor.peek(), Some('v'));
    }

    #[test]
    fn bounded_cursor_keeps_absolute_positions_and_stops_at_range_end() {
        let source = "first\r\n値: body\r\nafter";
        let start = source.find('値').expect("range start");
        let end = source.find("\r\nafter").expect("range end");
        let mut cursor = Cursor::new(source);
        cursor.limit(end);
        cursor.skip_to_offset(start);

        assert_eq!(cursor.position().offset(), start);
        assert_eq!(cursor.position().line(), 2);
        assert_eq!(cursor.take_line(), Some((false, "値: body")));
        assert_eq!(cursor.position().offset(), end);
        assert!(cursor.is_eof());
        assert_eq!(cursor.take(), None);
    }

    #[test]
    fn resetting_the_limit_restores_the_hidden_suffix() {
        let source = "first\r\nsecond";
        let mut cursor = Cursor::new(source);
        cursor.limit(5);
        cursor.skip_to_offset(5);

        assert!(cursor.is_eof());
        cursor.limit(source.len());

        assert_eq!(cursor.take(), Some('\n'));
        assert_eq!(cursor.take_line(), Some((false, "second")));
    }
}

mod position_tracking {
    use super::*;

    fn pos(offset: usize, line: usize, column: usize) -> Position {
        Position {
            offset,
            line,
            column,
        }
    }

    #[test]
    fn tracks_utf8_byte_offsets_and_character_columns() {
        let mut cursor = Cursor::new("a阿");

        cursor.take();
        assert_eq!(cursor.position(), pos(1, 1, 2));

        cursor.take();
        assert_eq!(cursor.position(), pos(4, 1, 3));
    }

    #[test]
    fn treats_common_line_endings_as_one_line_break() {
        let mut cursor = Cursor::new("a\r\nb\rc\nd");

        assert_eq!(cursor.take(), Some('a'));
        assert_eq!(cursor.position(), pos(1, 1, 2));

        assert_eq!(cursor.take(), Some('\n'));
        assert_eq!(cursor.position(), pos(3, 2, 1));

        assert_eq!(cursor.take(), Some('b'));
        assert_eq!(cursor.position(), pos(4, 2, 2));

        assert_eq!(cursor.take(), Some('\n'));
        assert_eq!(cursor.position(), pos(5, 3, 1));

        assert_eq!(cursor.take(), Some('c'));
        assert_eq!(cursor.position(), pos(6, 3, 2));

        assert_eq!(cursor.take(), Some('\n'));
        assert_eq!(cursor.position(), pos(7, 4, 1));

        assert_eq!(cursor.take(), Some('d'));
        assert_eq!(cursor.position(), pos(8, 4, 2));

        assert!(cursor.is_eof());
    }

    #[test]
    fn normalises_line_endings() {
        fn assert_normalised_line_ending(source: &str) {
            let mut cursor = Cursor::new(source);

            assert_eq!(cursor.peek(), Some('\n'));
            assert_eq!(cursor.take(), Some('\n'));
            assert!(cursor.is_eof());
        }

        assert_normalised_line_ending("\r");
        assert_normalised_line_ending("\r\n");
        assert_normalised_line_ending("\n");
    }
}

mod whitespace {
    use super::*;

    #[test]
    fn skips_inline_whitespaces_without_crossing_a_line() {
        fn assert_skips_inline_whitespaces(source: &str) {
            let mut cursor = Cursor::new(source);

            cursor.skip_whitespaces_inline();

            assert_eq!(cursor.peek(), Some('\n'));
            assert_eq!(cursor.position().line(), 1);
            assert_eq!(cursor.position().column(), 3);
        }

        assert_skips_inline_whitespaces(" \t\rvalue");
        assert_skips_inline_whitespaces(" \t\r\nvalue");
        assert_skips_inline_whitespaces(" \t\nvalue");
    }

    #[test]
    fn skips_one_whitespace_line_and_reports_advancement() {
        fn assert_skips_whitespace_line(source: &str) {
            let mut cursor = Cursor::new(source);

            assert!(cursor.skip_whitespace_line());
            assert_eq!(cursor.peek(), Some('v'));
        }

        assert_skips_whitespace_line(" \t\rvalue");
        assert_skips_whitespace_line(" \t\r\nvalue");
        assert_skips_whitespace_line(" \t\nvalue");

        let mut final_line = Cursor::new(" \t");
        assert!(final_line.skip_whitespace_line());
        assert!(final_line.is_eof());

        let mut content = Cursor::new("  value\n");
        assert!(!content.skip_whitespace_line());
        assert_eq!(content.position().offset(), 0);

        let mut empty = Cursor::new("");
        assert!(!empty.skip_whitespace_line());
    }

    #[test]
    fn skips_consecutive_whitespace_lines() {
        let mut cursor = Cursor::new(" \t\r\n\n  \rvalue");

        cursor.skip_whitespace_lines();

        assert_eq!(cursor.peek(), Some('v'));
        assert_eq!(cursor.position().line(), 4);
        assert_eq!(cursor.position().column(), 1);
    }
}

mod lines {
    use super::*;

    #[test]
    fn peeks_without_advancing() {
        fn assert_peeks_without_advancing(source: &str) {
            let cursor = Cursor::new(source);

            assert_eq!(cursor.peek_line(), Some((true, "value")));
            assert_eq!(cursor.position().offset(), 0);
        }

        assert_peeks_without_advancing("value\rrest");
        assert_peeks_without_advancing("value\r\nrest");
        assert_peeks_without_advancing("value\nrest");
    }

    #[test]
    fn consumes_lines_and_reports_their_line_endings() {
        let mut cursor = Cursor::new("first\r\nsecond\nthird");

        assert_eq!(cursor.take_line(), Some((true, "first")));
        assert_eq!(cursor.position().line(), 2);
        assert_eq!(cursor.take_line(), Some((true, "second")));
        assert_eq!(cursor.take_line(), Some((false, "third")));
        assert_eq!(cursor.take_line(), None);
    }

    #[test]
    fn consumes_an_empty_line() {
        let mut cursor = Cursor::new("\nvalue");

        assert_eq!(cursor.take_line(), Some((true, "")));
        assert_eq!(cursor.peek(), Some('v'));
    }
}
