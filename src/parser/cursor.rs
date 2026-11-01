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
    pub const ZERO: Self = Self {
        offset: 0,
        line: 1,
        column: 1,
    };

    pub fn offset(self) -> usize {
        self.offset
    }

    pub fn line(self) -> usize {
        self.line
    }

    pub fn column(self) -> usize {
        self.column
    }

    pub(crate) fn advance(mut self, text: &str) -> Self {
        let mut last = '\0';
        for ch in text.chars() {
            match ch {
                '\n' if last == '\r' => {}
                '\r' | '\n' => {
                    self.column = 1;
                    self.line += 1;
                }
                _ => {
                    self.column += 1;
                }
            }
            self.offset += ch.len_utf8();
            last = ch;
        }
        self
    }
}

/// A forward-only cursor over UTF-8 source text.
#[derive(Clone, Debug)]
pub struct Cursor<'a> {
    source: &'a str,
    position: Position,
    limit: usize,
}

impl<'a> Cursor<'a> {
    pub const fn new(source: &'a str) -> Self {
        Self {
            source,
            position: Position::ZERO,
            limit: source.len(),
        }
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

    pub fn skip_to_offset(&mut self, abs_offset: usize) {
        assert!(abs_offset <= self.limit);
        while self.position.offset < abs_offset {
            self.take().expect("abs_offset is within source");
        }
        assert_eq!(self.position.offset, abs_offset);
    }

    /// Set a new limit and return the old limit
    pub fn limit(&mut self, offset: usize) -> usize {
        let old_limit = self.limit;
        assert!(self.source.is_char_boundary(offset));
        assert!(offset >= self.position.offset);
        assert!(offset <= self.source.len());
        self.limit = offset;
        old_limit
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

    pub fn skip_remaining(&mut self) {
        while !self.is_eof() {
            self.take();
        }
    }

    pub fn is_eof(&self) -> bool {
        self.position.offset == self.limit
    }

    fn remaining(&self) -> &'a str {
        &self.source[self.position.offset..self.limit]
    }
}

#[cfg(test)]
mod tests;
