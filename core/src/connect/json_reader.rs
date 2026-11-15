//! Reads JSON the way 1.x read Claude Code's `settings.json`, and says where in the text each
//! value starts and ends.
//!
//! `serde_json`, which reads Core's own files, cannot be used here, for three reasons.
//! Claude Code's settings may hold comments and a comma after the last item, which are not
//! JSON. Connect cuts one value out of the file and leaves every other byte alone, so it needs
//! to know where that value sits in the text. And a file that 1.x refused must be refused here
//! too: what counts as JSON is decided by the same rules, down to how deep it may nest.
//!
//! Nothing is converted on the way. A number and a text are kept as the piece of the file
//! they were written in; [`text_of`] turns a text into what it says when someone asks.

/// The text is not JSON, or not the JSON this reader was asked for.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct NotJson;

/// What the reader lets through beyond plain JSON.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Dialect {
    /// JSON and nothing else.
    Strict,
    /// JSON that may also hold `// ...` and `/* ... */` comments, and a comma after the last
    /// item of an object or a list.
    Relaxed,
}

/// One value and where it sits in the text: `start` is its first byte, `end` the byte after
/// its last.
#[derive(Debug)]
pub(crate) struct Value<'a> {
    pub start: usize,
    pub end: usize,
    pub kind: Kind<'a>,
}

/// What a value is. A number and a text hold the piece of the file they were written in, the
/// text with its quotes.
#[derive(Debug)]
pub(crate) enum Kind<'a> {
    Null,
    True,
    False,
    Number(&'a str),
    Text(&'a str),
    List(Vec<Value<'a>>),
    Object(Vec<Member<'a>>),
}

/// One `"name": value` of an object. `name` is the name as written, with its quotes, and
/// `name_start` is where its first quote sits.
#[derive(Debug)]
pub(crate) struct Member<'a> {
    pub name_start: usize,
    pub name: &'a str,
    pub value: Value<'a>,
}

/// How many objects and lists may sit inside one another. 1.x's reader stopped at 64.
const DEEPEST: usize = 64;

/// Reads the whole of `text` as one JSON value, with nothing after it but spaces (and, when
/// relaxed, comments).
pub(crate) fn read(text: &str, dialect: Dialect) -> Result<Value<'_>, NotJson> {
    let mut reader = Reader::new(text, dialect);
    reader.skip_trivia()?;
    let value = reader.value()?;
    reader.end()?;
    Ok(value)
}

/// What a text says: `raw` is a text as written in the file, with its quotes, and the result
/// has every `\n`, `\"` and `é` turned into the character it stands for.
///
/// `None` when the text spells out half a character: `\uD800` with no second half after it,
/// which no text can hold.
pub(crate) fn text_of(raw: &str) -> Option<String> {
    // The reader has checked the text already, all but that one thing, so `serde_json` can do
    // the converting.
    serde_json::from_str(raw).ok()
}

/// The value under `name` in an object's members. A name written twice counts the last time
/// it is written, as in 1.x, and a name may be spelled with codes (`\u0061` for `a`).
pub(crate) fn member<'a>(members: &'a [Member<'a>], name: &str) -> Option<&'a Kind<'a>> {
    members
        .iter()
        .rev()
        .find(|member| text_of(member.name).as_deref() == Some(name))
        .map(|member| &member.value.kind)
}

/// Walks through a text one byte at a time. `at` is the byte it will look at next.
pub(crate) struct Reader<'a> {
    text: &'a str,
    at: usize,
    depth: usize,
    dialect: Dialect,
}

impl<'a> Reader<'a> {
    pub(crate) fn new(text: &'a str, dialect: Dialect) -> Reader<'a> {
        Reader {
            text,
            at: 0,
            depth: 0,
            dialect,
        }
    }

    /// The byte the reader is at, or `None` at the end of the text.
    pub(crate) fn peek(&self) -> Option<u8> {
        self.byte(self.at)
    }

    fn byte(&self, index: usize) -> Option<u8> {
        self.text.as_bytes().get(index).copied()
    }

    /// The piece of the text from `start` up to where the reader is now.
    fn since(&self, start: usize) -> &'a str {
        // Both ends sit on a quote, a digit or a letter the reader has just walked over, so
        // the piece is always there.
        self.text.get(start..self.at).unwrap_or_default()
    }

    /// Moves past spaces, tabs and line ends.
    fn skip_spaces(&mut self) {
        while matches!(self.peek(), Some(b' ' | b'\t' | b'\r' | b'\n')) {
            self.at += 1;
        }
    }

    /// Moves past spaces, tabs and line ends and, when relaxed, past comments.
    pub(crate) fn skip_trivia(&mut self) -> Result<(), NotJson> {
        loop {
            self.skip_spaces();
            match self.peek() {
                Some(b'/') if self.dialect == Dialect::Relaxed => self.comment()?,
                _ => return Ok(()),
            }
        }
    }

    /// Moves past one comment. The reader is at its first `/`.
    fn comment(&mut self) -> Result<(), NotJson> {
        match self.byte(self.at + 1) {
            // `// ...` runs to the end of the line, or of the text.
            Some(b'/') => {
                self.at += 2;
                loop {
                    match self.peek() {
                        None => return Ok(()),
                        Some(b'\n' | b'\r') => {
                            self.at += 1;
                            return Ok(());
                        }
                        // Unicode has two more characters that end a line. 1.x's reader
                        // refused them inside this kind of comment. In UTF-8 they are the
                        // three bytes E2 80 A8 and E2 80 A9.
                        Some(0xE2)
                            if self.byte(self.at + 1) == Some(0x80)
                                && matches!(self.byte(self.at + 2), Some(0xA8 | 0xA9)) =>
                        {
                            return Err(NotJson);
                        }
                        Some(_) => self.at += 1,
                    }
                }
            }
            // `/* ... */` must be closed.
            Some(b'*') => {
                self.at += 2;
                loop {
                    match (self.peek(), self.byte(self.at + 1)) {
                        (Some(b'*'), Some(b'/')) => {
                            self.at += 2;
                            return Ok(());
                        }
                        (None, _) => return Err(NotJson),
                        _ => self.at += 1,
                    }
                }
            }
            _ => Err(NotJson),
        }
    }

    /// Reads one value, with everything inside it. The reader must be at its first byte.
    pub(crate) fn value(&mut self) -> Result<Value<'a>, NotJson> {
        let start = self.at;
        let kind = match self.peek() {
            Some(b'{') => self.object()?,
            Some(b'[') => self.list()?,
            Some(b'"') => {
                self.text()?;
                Kind::Text(self.since(start))
            }
            Some(b'-' | b'0'..=b'9') => {
                self.number()?;
                Kind::Number(self.since(start))
            }
            Some(b't') => {
                self.word("true")?;
                Kind::True
            }
            Some(b'f') => {
                self.word("false")?;
                Kind::False
            }
            Some(b'n') => {
                self.word("null")?;
                Kind::Null
            }
            _ => return Err(NotJson),
        };
        Ok(Value {
            start,
            end: self.at,
            kind,
        })
    }

    /// Reads no more than the first piece of a value: the `{` or `[` that opens it, or the
    /// whole of a text, a number or a word. This is as far as 1.x read before it said that
    /// `settings.json` is something else than an object.
    pub(crate) fn first_piece(&mut self) -> Result<(), NotJson> {
        match self.peek() {
            Some(b'{' | b'[') => {
                self.at += 1;
                Ok(())
            }
            Some(b'"') => self.text(),
            Some(b'-' | b'0'..=b'9') => self.number(),
            Some(b't') => self.word("true"),
            Some(b'f') => self.word("false"),
            Some(b'n') => self.word("null"),
            _ => Err(NotJson),
        }
    }

    /// Moves past what may follow the last value. Anything but spaces and comments is an error.
    pub(crate) fn end(&mut self) -> Result<(), NotJson> {
        self.skip_trivia()?;
        match self.peek() {
            None => Ok(()),
            Some(_) => Err(NotJson),
        }
    }

    /// Steps into an object or a list, unless that would be one level too deep.
    fn open(&mut self) -> Result<(), NotJson> {
        if self.depth >= DEEPEST {
            return Err(NotJson);
        }
        self.depth += 1;
        self.at += 1;
        Ok(())
    }

    /// Steps out of an object or a list, past its closing brace or bracket.
    fn close(&mut self) {
        self.depth -= 1;
        self.at += 1;
    }

    /// Reads an object. The reader is at its `{`.
    fn object(&mut self) -> Result<Kind<'a>, NotJson> {
        self.open()?;
        let mut members = Vec::new();
        self.skip_trivia()?;
        while self.peek() != Some(b'}') {
            let name_start = self.at;
            if self.peek() != Some(b'"') {
                return Err(NotJson);
            }
            self.text()?;
            let name = self.since(name_start);

            // Between a name and its colon 1.x's reader took spaces only: a comment there is
            // an error, even when relaxed.
            self.skip_spaces();
            if self.peek() != Some(b':') {
                return Err(NotJson);
            }
            self.at += 1;

            self.skip_trivia()?;
            let value = self.value()?;
            members.push(Member {
                name_start,
                name,
                value,
            });

            self.comma_or_end(b'}')?;
        }
        self.close();
        Ok(Kind::Object(members))
    }

    /// Reads a list. The reader is at its `[`.
    fn list(&mut self) -> Result<Kind<'a>, NotJson> {
        self.open()?;
        let mut items = Vec::new();
        self.skip_trivia()?;
        while self.peek() != Some(b']') {
            items.push(self.value()?);
            self.comma_or_end(b']')?;
        }
        self.close();
        Ok(Kind::List(items))
    }

    /// After an item: moves past a comma to the next item, or stops at `closing`. A comma
    /// right before `closing` is let through only when relaxed.
    fn comma_or_end(&mut self, closing: u8) -> Result<(), NotJson> {
        self.skip_trivia()?;
        match self.peek() {
            Some(byte) if byte == closing => Ok(()),
            Some(b',') => {
                self.at += 1;
                self.skip_trivia()?;
                if self.peek() == Some(closing) && self.dialect == Dialect::Strict {
                    return Err(NotJson);
                }
                Ok(())
            }
            _ => Err(NotJson),
        }
    }

    /// Moves past a text. The reader is at its opening quote.
    fn text(&mut self) -> Result<(), NotJson> {
        self.at += 1;
        loop {
            match self.peek() {
                None => return Err(NotJson),
                Some(b'"') => {
                    self.at += 1;
                    return Ok(());
                }
                // A backslash starts a spelled-out character: one of eight short ones, or
                // `\u` and four hexadecimal digits.
                Some(b'\\') => match self.byte(self.at + 1) {
                    Some(b'"' | b'\\' | b'/' | b'b' | b'f' | b'n' | b'r' | b't') => self.at += 2,
                    Some(b'u') => {
                        let digits = self.text.as_bytes().get(self.at + 2..self.at + 6);
                        let all_hex =
                            digits.is_some_and(|four| four.iter().all(u8::is_ascii_hexdigit));
                        if !all_hex {
                            return Err(NotJson);
                        }
                        self.at += 6;
                    }
                    _ => return Err(NotJson),
                },
                // A tab or a line end inside a text must be spelled out.
                Some(byte) if byte < 0x20 => return Err(NotJson),
                Some(_) => self.at += 1,
            }
        }
    }

    /// Moves past a number: a minus sign or not, digits with no needless zero in front, then
    /// maybe a point and digits, then maybe an `e`, a sign and digits.
    fn number(&mut self) -> Result<(), NotJson> {
        if self.peek() == Some(b'-') {
            self.at += 1;
        }
        match self.peek() {
            Some(b'0') => self.at += 1,
            Some(b'1'..=b'9') => self.digits(),
            _ => return Err(NotJson),
        }
        if self.peek() == Some(b'.') {
            self.at += 1;
            self.at_least_one_digit()?;
        }
        if matches!(self.peek(), Some(b'e' | b'E')) {
            self.at += 1;
            if matches!(self.peek(), Some(b'+' | b'-')) {
                self.at += 1;
            }
            self.at_least_one_digit()?;
        }
        // What may come right after a number: the end of the text, a comma, a closing brace
        // or bracket, a space, or the start of a comment.
        match self.peek() {
            None | Some(b',' | b'}' | b']' | b' ' | b'\n' | b'\r' | b'\t' | b'/') => Ok(()),
            Some(_) => Err(NotJson),
        }
    }

    fn digits(&mut self) {
        while matches!(self.peek(), Some(b'0'..=b'9')) {
            self.at += 1;
        }
    }

    fn at_least_one_digit(&mut self) -> Result<(), NotJson> {
        if !matches!(self.peek(), Some(b'0'..=b'9')) {
            return Err(NotJson);
        }
        self.digits();
        Ok(())
    }

    /// Moves past `true`, `false` or `null`.
    fn word(&mut self, word: &str) -> Result<(), NotJson> {
        let rest = self.text.as_bytes().get(self.at..).unwrap_or_default();
        if !rest.starts_with(word.as_bytes()) {
            return Err(NotJson);
        }
        self.at += word.len();
        Ok(())
    }
}
