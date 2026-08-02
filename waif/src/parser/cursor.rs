use std::ops::Range;

/// A location in source text.
///
/// `offset` is a zero-based UTF-8 byte offset. `line` and `column` are
/// one-based character positions.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Position {
    offset: usize,
    line: usize,
    column: usize,
}

impl Position {
    pub fn offset(self) -> usize {
        self.offset
    }

    pub fn line(self) -> usize {
        self.line
    }

    #[allow(dead_code)]
    pub fn column(self) -> usize {
        self.column
    }
}

/// A forward-only cursor over UTF-8 source text.
#[derive(Clone, Debug)]
pub struct Cursor<'a> {
    source: &'a str,
    position: Position,
}

impl<'a> Cursor<'a> {
    pub const fn new(source: &'a str) -> Self {
        Self {
            source,
            position: Position {
                offset: 0,
                line: 1,
                column: 1,
            },
        }
    }

    pub fn within(source: &'a str, range: Range<usize>) -> Self {
        assert!(range.start <= range.end);
        assert!(range.end <= source.len());
        assert!(source.is_char_boundary(range.start));
        assert!(source.is_char_boundary(range.end));
        let mut cursor = Self::new(&source[..range.end]);
        while cursor.position.offset < range.start {
            cursor.take().expect("range start is within source");
        }
        cursor
    }

    /// Return the current source position.
    pub fn position(&self) -> Position {
        self.position
    }

    /// Rewind to a position previously returned by this cursor.
    ///
    /// The position must come from this cursor and must not be ahead of its
    /// current position. This precondition is not checked.
    pub fn rewind(&mut self, position: Position) {
        self.position = position;
    }

    /// Run a parsing operation, rewinding the cursor when it fails.
    pub fn try_<A, E>(&mut self, f: impl FnOnce(&mut Self) -> Result<A, E>) -> Result<A, E> {
        let pos = self.position();
        match f(self) {
            Ok(v) => Ok(v),
            Err(e) => {
                self.rewind(pos);
                Err(e)
            }
        }
    }

    /// Return the next character without advancing the cursor.
    pub fn peek(&self) -> Option<char> {
        self.remaining()
            .chars()
            .next()
            .map(|ch| if ch == '\r' { '\n' } else { ch })
    }

    /// Consume and return the next character.
    pub fn take(&mut self) -> Option<char> {
        let ch = self.peek()?;
        let consumed_bytes = if self.remaining().starts_with("\r\n") {
            2
        } else {
            ch.len_utf8()
        };
        self.position.offset += consumed_bytes;
        match ch {
            '\n' => {
                self.position.line += 1;
                self.position.column = 1;
            }
            _ => self.position.column += 1,
        }
        Some(ch)
    }

    /// Return the current line without advancing the cursor.
    ///
    /// Return whether the line has a line ending and its content. The line
    /// ending is not included in the content. Return `None` at end of input.
    pub fn peek_line(&self) -> Option<(bool, &'a str)> {
        let mut c = self.clone();
        c.take_line()
    }

    /// Consume and return the current line.
    ///
    /// Return whether the line has a line ending and its content. The line
    /// ending is consumed but not included in the content. Return `None` at
    /// end of input.
    pub fn take_line(&mut self) -> Option<(bool, &'a str)> {
        if self.is_eof() {
            return None;
        }

        let start = self.position.offset;
        while self.peek().is_some_and(|ch| ch != '\n') {
            self.take();
        }
        let end = self.position.offset;
        let has_nl = self.peek() == Some('\n');
        if has_nl {
            self.take();
        }

        Some((has_nl, &self.source[start..end]))
    }

    pub fn take_while(&mut self, p: impl Fn(char) -> bool) -> &'a str {
        let start = self.position.offset;
        while self.peek().is_some_and(&p) {
            self.take();
        }
        &self.source[start..self.position.offset]
    }

    pub fn take_if(&mut self, p: impl Fn(char) -> bool) -> Option<char> {
        let ch = self.peek()?;
        if p(ch) {
            self.take()
        } else {
            None
        }
    }

    /// Consume ASCII whitespace (space and tab) without crossing a line.
    pub fn skip_whitespaces_inline(&mut self) {
        while self.peek().is_some_and(|ch| ch == ' ' || ch == '\t') {
            self.take();
        }
    }

    /// Consume one line containing only ASCII whitespace.
    ///
    /// Return whether the cursor advanced. Whitespace on a line containing
    /// other content is left unconsumed. A final whitespace-only line does
    /// not need to have a line ending.
    pub fn skip_whitespace_line(&mut self) -> bool {
        let start = self.position();
        self.skip_whitespaces_inline();

        if self.peek() == Some('\n') {
            self.take();
            true
        } else if self.is_eof() && self.position() != start {
            true
        } else {
            self.rewind(start);
            false
        }
    }

    /// Consume consecutive lines containing only ASCII whitespace.
    pub fn skip_whitespace_lines(&mut self) {
        while self.skip_whitespace_line() {}
    }

    pub fn is_eof(&self) -> bool {
        self.position.offset == self.source.len()
    }

    fn remaining(&self) -> &'a str {
        &self.source[self.position.offset..]
    }
}

#[cfg(test)]
mod tests {
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
        fn transactions_keep_successes_and_rewind_failures() {
            let mut success = Cursor::new("ab");
            let result: Result<_, ()> = success.try_(|cursor| {
                cursor.take();
                Ok('a')
            });
            assert_eq!(result, Ok('a'));
            assert_eq!(success.peek(), Some('b'));

            let mut failure = Cursor::new("ab");
            let start = failure.position();
            let result: Result<(), _> = failure.try_(|cursor| {
                cursor.take();
                Err("failed")
            });
            assert_eq!(result, Err("failed"));
            assert_eq!(failure.position(), start);
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
            let mut cursor = Cursor::within(source, start..end);

            assert_eq!(cursor.position().offset(), start);
            assert_eq!(cursor.position().line(), 2);
            assert_eq!(cursor.take_line(), Some((false, "値: body")));
            assert_eq!(cursor.position().offset(), end);
            assert!(cursor.is_eof());
            assert_eq!(cursor.take(), None);
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
}
