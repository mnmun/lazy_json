#![warn(missing_docs)]
//! ![logo](https://github.com/mnmun/json/blob/main/logo.png?raw=true)
//!
//! # JSON lexer built on top of [`a_bc`]
//!
//! A ready-made [lexer](`JsonLexer`) that splits a JSON byte input into
//! [`Token`]s tagged with [`Json`] kinds.
//!
//! ## Example
//!
//! ```rust
//! use pretty_assertions::assert_eq;
//!
//! use a_bc::{lexer::Builder, token::Token};
//! use json::{Json, JsonLexer};
//!
//! let source = b"
//!     {
//!         \"first_name\": \"John\",
//!         \"second_name\": \"Smith\"
//!     }
//! ";
//!
//! let mut lexer = JsonLexer::new(Builder::new(source).build().unwrap());
//!
//! let token = lexer.next().unwrap().unwrap();
//! assert_eq!(token.kind(), &Json::Object);
//!
//! let mut lexer = JsonLexer::new(
//!     Builder::new(source)
//!         .with_range(token.inner_range().unwrap())
//!         .build()
//!         .unwrap()
//! );
//!
//! let token = lexer.expect(&[Json::String]).unwrap().unwrap();
//! assert_eq!(token.kind(), &Json::String);
//! assert_eq!(lexer.format(&token), "\"first_name\"");
//!
//! let token = lexer.expect(&[Json::Colon]).unwrap().unwrap();
//! assert_eq!(token.kind(), &Json::Colon);
//! assert_eq!(lexer.format(&token), ":");
//!
//! let token = lexer.not_expect(&[Json::Comma]).unwrap().unwrap();
//! assert_eq!(token.kind(), &Json::String);
//! assert_eq!(lexer.format(&token), "\"John\"");
//!
//! let token = lexer.expect(&[Json::Comma]).unwrap().unwrap();
//! assert_eq!(token.kind(), &Json::Comma);
//! assert_eq!(lexer.format(&token), ",");
//!
//! let token = lexer
//!     .not_expect(&[
//!         Json::Object,
//!         Json::Array,
//!         Json::Comma,
//!         Json::Colon,
//!     ])
//!     .unwrap()
//!     .unwrap();
//! assert_eq!(token.kind(), &Json::String);
//! assert_eq!(lexer.format(&token), "\"second_name\"");
//!
//! let token = lexer.expect(&[Json::Colon]).unwrap().unwrap();
//! assert_eq!(token.kind(), &Json::Colon);
//! assert_eq!(lexer.format(&token), ":");
//!
//! let token = lexer.not_expect(&[Json::Comma]).unwrap().unwrap();
//! assert_eq!(token.kind(), &Json::String);
//! assert_eq!(lexer.format(&token), "\"Smith\"");
//!
//! let token = lexer.next();
//! assert_eq!(token, None);
//! ```
//!
//! ## License
//!
//! [MIT](https://github.com/mnmun/json/blob/main/LICENSE)

use core::fmt;
use memchr::memchr;
use std::{ops::Range, str::from_utf8};

use a_bc::{
    cancel::Cancel,
    error,
    lexer::{Cursor, Data, Lexer},
    token::Token,
    traits::InnerRange,
    utils::{
        memchr_count_needles_considering_escaped_delimiters, memchr_needle,
        row_col_pos,
    },
};

/// Token kinds for JSON
#[derive(PartialEq, Clone, Copy)]
pub enum Json {
    /// The comma separator ','
    Comma,

    /// The key/value separator ':'
    Colon,

    /// An object delimited by '{' and '}'
    Object,

    /// An array delimited by '\[' and '\]'
    Array,

    /// A double-quoted string
    String,

    /// A bareword sequence: numbers, `true`, `false`, `null`, etc.
    Sequence,
}

impl InnerRange for Json {
    /// Returns the range inside the delimiters for [`Json::Object`],
    /// [`Json::Array`], and [`Json::String`]. Returns `None` for other kinds.
    fn inner_range(&self, range: &Range<usize>) -> Option<Range<usize>> {
        match self {
            Json::Comma => None,
            Json::Colon => None,
            Json::Object => {
                if range.start + 1 < range.end - 1 {
                    Some(range.start + 1..range.end - 1)
                } else {
                    None
                }
            }
            Json::Array => {
                if range.start + 1 < range.end - 1 {
                    Some(range.start + 1..range.end - 1)
                } else {
                    None
                }
            }
            Json::String => {
                if range.start + 1 < range.end - 1 {
                    Some(range.start + 1..range.end - 1)
                } else {
                    None
                }
            }
            Json::Sequence => None,
        }
    }
}

impl fmt::Debug for Json {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Json::Comma => f.write_str("Comma"),
            Json::Colon => f.write_str("Colon"),
            Json::Object => f.write_str("Object"),
            Json::Array => f.write_str("Array"),
            Json::String => f.write_str("String"),
            Json::Sequence => f.write_str("Sequence"),
        }
    }
}

/// # Newtype wrapper around [`Lexer`] that yields JSON tokens
///
/// Implements [`Iterator`] for tokenizing JSON byte input.
///
/// See [`crate`] for usage example.
pub struct JsonLexer<'a>(Lexer<'a>);

impl<'a> From<Lexer<'a>> for JsonLexer<'a> {
    fn from(value: Lexer<'a>) -> Self {
        JsonLexer(value)
    }
}

impl<'a> JsonLexer<'a> {
    /// Creates a new [`JsonLexer`] on top of [`Lexer`]
    pub fn new(lexer: Lexer<'a>) -> Self {
        Self(lexer)
    }
}

impl JsonLexer<'_> {
    /// Returns a reference to the underlying [`lexer`]'s [`data`]
    ///
    /// [`lexer`]: Lexer
    /// [`data`]: Data
    pub fn data(&self) -> &Data<'_> {
        self.0.data()
    }

    /// Returns a reference to the underlying [`lexer`]'s [`cursor`]
    ///
    /// [`lexer`]: Lexer
    /// [`cursor`]: `Cursor`
    pub fn cursor(&self) -> &Cursor {
        self.0.cursor()
    }

    /// Returns a reference to the underlying [`lexer`]'s [`cancellation token`]
    ///
    /// [`lexer`]: Lexer
    /// [`cancellation token`]: `Cancel`
    pub fn cancel(&self) -> &Cancel {
        self.0.cancel()
    }
}

impl<'a> JsonLexer<'a> {
    /// Converts a token's byte range into a `&str`.
    ///
    /// # Panics
    ///
    /// Panics if the underlying bytes are not valid UTF-8.
    ///
    /// # Example
    ///
    /// ```rust
    /// use pretty_assertions::assert_eq;
    ///
    /// use a_bc::lexer::Builder;
    /// use json::JsonLexer;
    ///
    /// let source = b"\"hello\"";
    ///
    /// let mut lexer = JsonLexer::new(Builder::new(source).build().unwrap());
    /// let token = lexer.next().unwrap().unwrap();
    ///
    /// assert_eq!(lexer.format(&token), "\"hello\"");
    /// ```
    pub fn format<'b>(&'a self, token: &'b Token<Json>) -> &'a str {
        from_utf8(&self.data().source().as_ref()[token.range().clone()])
            .expect("Expected data to be a valid UTF-8")
    }

    /// Finds a matching block delimited by `opening` and `closing` starting at
    /// the [`cursor`], tracking nesting depth and considering quoted strings.
    /// The returned range includes the surrounding `opening` and `closing`, and
    /// the cursor is positioned after the `closing`.
    ///
    /// # Errors
    ///
    /// Returns [`PairNotFound`] if the closing delimiter is never found, or
    /// [`Cancelled`] if cancellation is requested mid-scan.
    ///
    /// # Example
    ///
    /// ```rust
    /// use pretty_assertions::assert_eq;
    ///
    /// use a_bc::lexer::Builder;
    /// use json::JsonLexer;
    ///
    /// let source = b"{ \" } { \" }";
    /// // Chars idx:  01 234567 890
    ///
    /// let mut lexer = JsonLexer::new(Builder::new(source).build().unwrap());
    ///
    /// assert_eq!(lexer.find_block(b'{', b'}').unwrap(), 0..11);
    /// ```
    ///
    /// [`cursor`]: JsonLexer::cursor()
    /// [`PairNotFound`]: error::token::Error::PairNotFound
    /// [`Cancelled`]: error::basic::Error::Cancelled
    pub fn find_block(
        &mut self,
        opening: u8,
        closing: u8,
    ) -> Result<Range<usize>, error::Error<Json>> {
        let initial_position = *self.cursor().position();

        let mut quote_counter = 0;
        let mut opening_counter = 1;
        let mut search_start = initial_position + 1;

        if search_start >= self.data().range().end {
            return Err(error::token::Error::PairNotFound {
                opening: from_utf8(&[opening])
                    .expect("Expected data to be utf8 compatible")
                    .to_string()
                    .into_boxed_str(),
                closing: from_utf8(&[closing])
                    .expect("Expected data to be utf8 compatible")
                    .to_string()
                    .into_boxed_str(),
                after: row_col_pos(&self.data().source()[..=initial_position]),
            }
            .into());
        }

        loop {
            if self.cancel().is_cancelled() {
                return Err(error::basic::Error::Cancelled.into());
            }

            if let Some(local_position) = memchr(
                closing,
                &self.data().source()[search_start..self.data().range().end],
            ) {
                let end = search_start + local_position;

                opening_counter +=
                    memchr_count_needles_considering_escaped_delimiters(
                        &memchr_needle::Kind::One(opening),
                        b"\\",
                        &memchr_needle::Kind::One(b'"'),
                        &mut quote_counter,
                        &self.data().source()[search_start..end],
                    );

                if quote_counter % 2 != 0 {
                    search_start = end + 1;
                    continue;
                }

                opening_counter -= 1;

                if opening_counter == 0 {
                    self.0.set_position(search_start + local_position);

                    return Ok(initial_position..end + 1);
                } else {
                    search_start = end + 1;
                }
            } else {
                return Err(error::token::Error::PairNotFound {
                    opening: from_utf8(&[opening])
                        .expect("Expected data to be utf8 compatible")
                        .to_string()
                        .into_boxed_str(),
                    closing: from_utf8(&[closing])
                        .expect("Expected data to be utf8 compatible")
                        .to_string()
                        .into_boxed_str(),
                    after: row_col_pos(
                        &self.data().source()[..=initial_position],
                    ),
                }
                .into());
            }
        }
    }

    /// Finds a double-quoted string starting at the [`cursor`], skipping over
    /// escaped quotes. The returned range includes the surrounding quotes, and
    /// the cursor is positioned after the closing quote.
    ///
    /// # Errors
    ///
    /// Returns [`PairNotFound`] if the closing quote is never found, or
    /// [`Cancelled`] if cancellation is requested mid-scan.
    ///
    /// # Example
    ///
    /// ```rust
    /// use pretty_assertions::assert_eq;
    ///
    /// use a_bc::lexer::Builder;
    /// use json::JsonLexer;
    ///
    /// let source = b"\"John\"";
    /// // Chars idx:   01234 5
    ///
    /// let mut lexer = JsonLexer::new(Builder::new(source).build().unwrap());
    ///
    /// assert_eq!(lexer.find_string().unwrap(), 0..6);
    /// ```
    ///
    /// [`cursor`]: JsonLexer::cursor()
    /// [`PairNotFound`]: error::token::Error::PairNotFound
    /// [`Cancelled`]: error::basic::Error::Cancelled
    pub fn find_string(&mut self) -> Result<Range<usize>, error::Error<Json>> {
        let initial_position = *self.cursor().position();

        let mut search_start = initial_position + 1;

        if search_start >= self.data().range().end {
            return Err(error::token::Error::PairNotFound {
                opening: "\"".into(),
                closing: "\"".into(),
                after: row_col_pos(&self.data().source()[..=initial_position]),
            }
            .into());
        }

        loop {
            if self.cancel().is_cancelled() {
                return Err(error::basic::Error::Cancelled.into());
            }

            if let Some(local_position) = memchr(
                b'"',
                &self.data().source()[search_start..self.data().range().end],
            ) {
                let end = search_start + local_position + 1;
                let mut backslash_counter = 0;
                let mut i = end - 1;

                loop {
                    if i > 0 {
                        i -= 1;

                        if self.data().source()[i] == b'\\' {
                            backslash_counter += 1;
                        } else {
                            break;
                        }
                    } else {
                        break;
                    }
                }

                if backslash_counter % 2 == 0 {
                    self.0.set_position(search_start + local_position);

                    return Ok(initial_position..end);
                } else {
                    search_start = end + 1;
                }
            } else {
                return Err(error::token::Error::PairNotFound {
                    opening: "\"".into(),
                    closing: "\"".into(),
                    after: row_col_pos(
                        &self.data().source()[..=initial_position],
                    ),
                }
                .into());
            }
        }
    }

    /// Scans a bareword sequence starting at the [`cursor`] until a whitespace
    /// byte, `:`, or `,` are reached.
    ///
    /// # Errors
    ///
    /// Returns [`Cancelled`] if cancellation is requested mid-scan.
    ///
    /// # Example
    ///
    /// ```rust
    /// use pretty_assertions::assert_eq;
    ///
    /// use a_bc::lexer::Builder;
    /// use json::JsonLexer;
    ///
    /// let source = b"somesequence";
    /// // Chars idx:  012345678901
    ///
    /// let mut lexer = JsonLexer::new(Builder::new(source).build().unwrap());
    ///
    /// assert_eq!(lexer.find_sequence().unwrap(), 0..12);
    /// ```
    ///
    /// [`cursor`]: JsonLexer::cursor()
    /// [`Cancelled`]: error::basic::Error::Cancelled
    pub fn find_sequence(
        &mut self,
    ) -> Result<Range<usize>, error::Error<Json>> {
        let start = *self.cursor().position();

        loop {
            if self.cancel().is_cancelled() {
                return Err(error::basic::Error::Cancelled.into());
            }

            if let Some(next_byt) = self.0.peek_next_byte() {
                if next_byt.is_ascii_whitespace()
                    || [b':', b','].contains(&next_byt)
                {
                    break;
                } else {
                    self.0.read_next_byte();
                }
            } else {
                break;
            }
        }

        let end = self.cursor().position() + 1;

        Ok(start..end)
    }
}

impl<'a> JsonLexer<'a> {
    /// Advances the lexer and returns the next token if it matches one of
    /// the expected `kinds`.
    ///
    /// Returns `Error::ExpectedButGot` if the token kind is not in the list.
    pub fn expect(
        &mut self,
        kinds: &[Json],
    ) -> Option<Result<Token<Json>, error::Error<Json>>> {
        self.next()
            .map(|result: Result<Token<Json>, error::Error<Json>>| {
                result.and_then(|token: Token<Json>| {
                    if !kinds.contains(token.kind()) {
                        Err(error::token::Error::ExpectedButGot {
                            expected: kinds.to_vec().into_boxed_slice(),
                            got: *token.kind(),
                            after: row_col_pos(
                                &self.data().source()[..=token.range().start],
                            ),
                        }
                        .into())
                    } else {
                        Ok(token)
                    }
                })
            })
    }

    /// Advances the lexer and returns the next token if it does not match
    /// any of the disallowed `kinds`.
    ///
    /// Returns `Error::NotExpectedButGot` if the token kind is in the list.
    pub fn not_expect(
        &mut self,
        kinds: &[Json],
    ) -> Option<Result<Token<Json>, error::Error<Json>>> {
        self.next()
            .map(|result: Result<Token<Json>, error::Error<Json>>| {
                result.and_then(|token: Token<Json>| {
                    if kinds.contains(token.kind()) {
                        Err(error::token::Error::NotExpectedButGot {
                            not_expected: kinds.to_vec().into_boxed_slice(),
                            got: *token.kind(),
                            after: row_col_pos(
                                &self.data().source()[..=token.range().start],
                            ),
                        }
                        .into())
                    } else {
                        Ok(token)
                    }
                })
            })
    }
}

impl<'a> Iterator for JsonLexer<'a> {
    type Item = Result<Token<Json>, error::Error<Json>>;

    /// Produces the next JSON token.
    ///
    /// Token dispatch by first byte:
    /// - `:` -> [`Json::Colon`]
    /// - `,` -> [`Json::Comma`]
    /// - `{` -> [`Json::Object`]
    /// - `[` -> [`Json::Array`]
    /// - `"` -> [`Json::String`]
    /// - anything else -> [`Json::Sequence`]
    fn next(&mut self) -> Option<Self::Item> {
        if self.cancel().is_cancelled() {
            return None;
        }

        self.0.skip_whitespace();

        let token = self.cursor().byte().and_then(|byte| {
            let (kind, range) = match byte {
                b':' => (
                    Json::Colon,
                    Ok(*self.cursor().position()..self.cursor().position() + 1),
                ),
                b',' => (
                    Json::Comma,
                    Ok(*self.cursor().position()..self.cursor().position() + 1),
                ),
                b'{' => (Json::Object, self.find_block(b'{', b'}')),
                b'[' => (Json::Array, self.find_block(b'[', b']')),
                b'"' => (Json::String, self.find_string()),
                _ => (Json::Sequence, self.find_sequence()),
            };

            match range {
                Ok(range) => Some(Ok(Token::new(kind, range))),
                Err(e) if e == error::basic::Error::Cancelled.into() => None,
                Err(e) => Some(Err(e)),
            }
        });

        self.0.read_next_byte();

        token
    }
}

#[cfg(test)]
mod tests {
    use pretty_assertions::assert_eq;
    use std::time::Duration;

    use a_bc::{error, lexer::Builder, token::Token};

    use crate::{Json, JsonLexer};

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    #[ignore = "depends on your system's specs"]
    async fn cancellation() {
        let n = 10000;
        let source = {
            let mut source = Vec::with_capacity(n);

            for i in 0..n {
                source.extend_from_slice(format!("{i}").as_bytes());
                source.extend_from_slice(b" ");
            }

            source
        };

        let lexer = JsonLexer(Builder::new(source).build().unwrap());
        let token = lexer.cancel().clone();

        let task = tokio::spawn(async move { lexer.count() });

        tokio::time::sleep(Duration::from_micros(1)).await;
        token.cancel();

        let result = task.await.unwrap();

        assert!(result < n);
    }

    #[test]
    fn parse() {
        let source = b"
            {
                \"first_name\": \"John\",
                \"last_name\": \"Smith\",
                \"is_alive\": true,
                \"age\": 27,
                \"address\": {
                    \"street_address\": \"21 2nd Street\",
                    \"city\": \"New York\",
                    \"state\": \"NY\",
                    \"postal_code\": \"10021-3100\"
                },
                \"phone_numbers\": [
                    {
                    \"type\": \"home\",
                    \"number\": \"212 555-1234\"
                    },
                    {
                    \"type\": \"office\",
                    \"number\": \"646 555-4567\"
                    }
                ],
                \"children\": [
                    \"Catherine\",
                    \"Thomas\",
                    \"Trevor\"
                ],
                \"spouse\": null
            }
        ";

        let mut lexer = JsonLexer(Builder::new(source).build().unwrap());

        let token = lexer.expect(&[Json::Object]).unwrap().unwrap();
        assert_eq!(token.kind(), &Json::Object);

        {
            let mut lexer = JsonLexer(
                Builder::new(source)
                    .with_range(token.inner_range().unwrap())
                    .build()
                    .unwrap(),
            );

            let token = lexer
                .not_expect(&[
                    Json::Object,
                    Json::Array,
                    Json::Comma,
                    Json::Colon,
                ])
                .unwrap()
                .unwrap();
            assert_eq!(lexer.format(&token), "\"first_name\"");
            assert_eq!(token.kind(), &Json::String);

            let token = lexer.expect(&[Json::Colon]).unwrap().unwrap();
            assert_eq!(lexer.format(&token), ":");
            assert_eq!(token.kind(), &Json::Colon);

            let token = lexer
                .not_expect(&[Json::Comma, Json::Colon])
                .unwrap()
                .unwrap();
            assert_eq!(lexer.format(&token), "\"John\"");
            assert_eq!(token.kind(), &Json::String);

            let token = lexer.expect(&[Json::Comma]).unwrap().unwrap();
            assert_eq!(lexer.format(&token), ",");
            assert_eq!(token.kind(), &Json::Comma);

            // ---

            let token = lexer
                .not_expect(&[
                    Json::Object,
                    Json::Array,
                    Json::Comma,
                    Json::Colon,
                ])
                .unwrap()
                .unwrap();
            assert_eq!(lexer.format(&token), "\"last_name\"");
            assert_eq!(token.kind(), &Json::String);

            let token = lexer.expect(&[Json::Colon]).unwrap().unwrap();
            assert_eq!(lexer.format(&token), ":");
            assert_eq!(token.kind(), &Json::Colon);

            let token = lexer
                .not_expect(&[Json::Comma, Json::Colon])
                .unwrap()
                .unwrap();
            assert_eq!(lexer.format(&token), "\"Smith\"");
            assert_eq!(token.kind(), &Json::String);

            let token = lexer.expect(&[Json::Comma]).unwrap().unwrap();
            assert_eq!(lexer.format(&token), ",");
            assert_eq!(token.kind(), &Json::Comma);

            // ---

            let token = lexer
                .not_expect(&[
                    Json::Object,
                    Json::Array,
                    Json::Comma,
                    Json::Colon,
                ])
                .unwrap()
                .unwrap();
            assert_eq!(lexer.format(&token), "\"is_alive\"");
            assert_eq!(token.kind(), &Json::String);

            let token = lexer.expect(&[Json::Colon]).unwrap().unwrap();
            assert_eq!(lexer.format(&token), ":");
            assert_eq!(token.kind(), &Json::Colon);

            let token = lexer
                .not_expect(&[Json::Comma, Json::Colon])
                .unwrap()
                .unwrap();
            assert_eq!(lexer.format(&token), "true");
            assert_eq!(token.kind(), &Json::Sequence);

            let token = lexer.expect(&[Json::Comma]).unwrap().unwrap();
            assert_eq!(lexer.format(&token), ",");
            assert_eq!(token.kind(), &Json::Comma);

            // ---

            let token = lexer
                .not_expect(&[
                    Json::Object,
                    Json::Array,
                    Json::Comma,
                    Json::Colon,
                ])
                .unwrap()
                .unwrap();
            assert_eq!(lexer.format(&token), "\"age\"");
            assert_eq!(token.kind(), &Json::String);

            let token = lexer.expect(&[Json::Colon]).unwrap().unwrap();
            assert_eq!(lexer.format(&token), ":");
            assert_eq!(token.kind(), &Json::Colon);

            let token = lexer
                .not_expect(&[Json::Comma, Json::Colon])
                .unwrap()
                .unwrap();
            assert_eq!(lexer.format(&token), "27");
            assert_eq!(token.kind(), &Json::Sequence);

            let token = lexer.expect(&[Json::Comma]).unwrap().unwrap();
            assert_eq!(lexer.format(&token), ",");
            assert_eq!(token.kind(), &Json::Comma);

            // ---

            let token = lexer
                .not_expect(&[
                    Json::Object,
                    Json::Array,
                    Json::Comma,
                    Json::Colon,
                ])
                .unwrap()
                .unwrap();
            assert_eq!(lexer.format(&token), "\"address\"");
            assert_eq!(token.kind(), &Json::String);

            let token = lexer.expect(&[Json::Colon]).unwrap().unwrap();
            assert_eq!(lexer.format(&token), ":");
            assert_eq!(token.kind(), &Json::Colon);

            let token = lexer.expect(&[Json::Object]).unwrap().unwrap();
            assert_eq!(token.kind(), &Json::Object);
            {
                let mut lexer = JsonLexer(
                    Builder::new(source)
                        .with_range(token.inner_range().unwrap())
                        .build()
                        .unwrap(),
                );

                let token = lexer
                    .not_expect(&[
                        Json::Object,
                        Json::Array,
                        Json::Comma,
                        Json::Colon,
                    ])
                    .unwrap()
                    .unwrap();
                assert_eq!(lexer.format(&token), "\"street_address\"");
                assert_eq!(token.kind(), &Json::String);

                let token = lexer.expect(&[Json::Colon]).unwrap().unwrap();
                assert_eq!(lexer.format(&token), ":");
                assert_eq!(token.kind(), &Json::Colon);

                let token = lexer
                    .not_expect(&[Json::Comma, Json::Colon])
                    .unwrap()
                    .unwrap();
                assert_eq!(lexer.format(&token), "\"21 2nd Street\"");
                assert_eq!(token.kind(), &Json::String);

                let token = lexer.expect(&[Json::Comma]).unwrap().unwrap();
                assert_eq!(lexer.format(&token), ",");
                assert_eq!(token.kind(), &Json::Comma);

                // ---

                let token = lexer
                    .not_expect(&[
                        Json::Object,
                        Json::Array,
                        Json::Comma,
                        Json::Colon,
                    ])
                    .unwrap()
                    .unwrap();
                assert_eq!(lexer.format(&token), "\"city\"");
                assert_eq!(token.kind(), &Json::String);

                let token = lexer.expect(&[Json::Colon]).unwrap().unwrap();
                assert_eq!(lexer.format(&token), ":");
                assert_eq!(token.kind(), &Json::Colon);

                let token = lexer
                    .not_expect(&[Json::Comma, Json::Colon])
                    .unwrap()
                    .unwrap();
                assert_eq!(lexer.format(&token), "\"New York\"");
                assert_eq!(token.kind(), &Json::String);

                let token = lexer.expect(&[Json::Comma]).unwrap().unwrap();
                assert_eq!(lexer.format(&token), ",");
                assert_eq!(token.kind(), &Json::Comma);

                // ---

                let token = lexer
                    .not_expect(&[
                        Json::Object,
                        Json::Array,
                        Json::Comma,
                        Json::Colon,
                    ])
                    .unwrap()
                    .unwrap();
                assert_eq!(lexer.format(&token), "\"state\"");
                assert_eq!(token.kind(), &Json::String);

                let token = lexer.expect(&[Json::Colon]).unwrap().unwrap();
                assert_eq!(lexer.format(&token), ":");
                assert_eq!(token.kind(), &Json::Colon);

                let token = lexer
                    .not_expect(&[Json::Comma, Json::Colon])
                    .unwrap()
                    .unwrap();
                assert_eq!(lexer.format(&token), "\"NY\"");
                assert_eq!(token.kind(), &Json::String);

                let token = lexer.expect(&[Json::Comma]).unwrap().unwrap();
                assert_eq!(lexer.format(&token), ",");
                assert_eq!(token.kind(), &Json::Comma);

                // ---

                let token = lexer
                    .not_expect(&[
                        Json::Object,
                        Json::Array,
                        Json::Comma,
                        Json::Colon,
                    ])
                    .unwrap()
                    .unwrap();
                assert_eq!(lexer.format(&token), "\"postal_code\"");
                assert_eq!(token.kind(), &Json::String);

                let token = lexer.expect(&[Json::Colon]).unwrap().unwrap();
                assert_eq!(lexer.format(&token), ":");
                assert_eq!(token.kind(), &Json::Colon);

                let token = lexer
                    .not_expect(&[Json::Comma, Json::Colon])
                    .unwrap()
                    .unwrap();
                assert_eq!(lexer.format(&token), "\"10021-3100\"");
                assert_eq!(token.kind(), &Json::String);
            }

            let token = lexer.expect(&[Json::Comma]).unwrap().unwrap();
            assert_eq!(lexer.format(&token), ",");
            assert_eq!(token.kind(), &Json::Comma);

            // ---

            let token = lexer
                .not_expect(&[
                    Json::Object,
                    Json::Array,
                    Json::Comma,
                    Json::Colon,
                ])
                .unwrap()
                .unwrap();
            assert_eq!(lexer.format(&token), "\"phone_numbers\"");
            assert_eq!(token.kind(), &Json::String);

            let token = lexer.expect(&[Json::Colon]).unwrap().unwrap();
            assert_eq!(lexer.format(&token), ":");
            assert_eq!(token.kind(), &Json::Colon);

            let token = lexer.expect(&[Json::Array]).unwrap().unwrap();
            assert_eq!(token.kind(), &Json::Array);

            {
                let mut lexer = JsonLexer(
                    Builder::new(source)
                        .with_range(token.inner_range().unwrap())
                        .build()
                        .unwrap(),
                );

                let token = lexer.expect(&[Json::Object]).unwrap().unwrap();
                assert_eq!(token.kind(), &Json::Object);

                {
                    let mut lexer = JsonLexer(
                        Builder::new(source)
                            .with_range(token.inner_range().unwrap())
                            .build()
                            .unwrap(),
                    );

                    let token = lexer
                        .not_expect(&[
                            Json::Object,
                            Json::Array,
                            Json::Comma,
                            Json::Colon,
                        ])
                        .unwrap()
                        .unwrap();
                    assert_eq!(lexer.format(&token), "\"type\"");
                    assert_eq!(token.kind(), &Json::String);

                    let token = lexer.expect(&[Json::Colon]).unwrap().unwrap();
                    assert_eq!(lexer.format(&token), ":");
                    assert_eq!(token.kind(), &Json::Colon);

                    let token = lexer
                        .not_expect(&[Json::Comma, Json::Colon])
                        .unwrap()
                        .unwrap();
                    assert_eq!(lexer.format(&token), "\"home\"");
                    assert_eq!(token.kind(), &Json::String);

                    let token = lexer.expect(&[Json::Comma]).unwrap().unwrap();
                    assert_eq!(lexer.format(&token), ",");
                    assert_eq!(token.kind(), &Json::Comma);

                    // ---

                    let token = lexer
                        .not_expect(&[
                            Json::Object,
                            Json::Array,
                            Json::Comma,
                            Json::Colon,
                        ])
                        .unwrap()
                        .unwrap();
                    assert_eq!(lexer.format(&token), "\"number\"");
                    assert_eq!(token.kind(), &Json::String);

                    let token = lexer.expect(&[Json::Colon]).unwrap().unwrap();
                    assert_eq!(lexer.format(&token), ":");
                    assert_eq!(token.kind(), &Json::Colon);

                    let token = lexer
                        .not_expect(&[Json::Comma, Json::Colon])
                        .unwrap()
                        .unwrap();
                    assert_eq!(lexer.format(&token), "\"212 555-1234\"");
                    assert_eq!(token.kind(), &Json::String);
                }

                let token = lexer.expect(&[Json::Comma]).unwrap().unwrap();
                assert_eq!(lexer.format(&token), ",");
                assert_eq!(token.kind(), &Json::Comma);

                // ---

                let token = lexer.expect(&[Json::Object]).unwrap().unwrap();
                assert_eq!(token.kind(), &Json::Object);

                {
                    let mut lexer = JsonLexer(
                        Builder::new(source)
                            .with_range(token.inner_range().unwrap())
                            .build()
                            .unwrap(),
                    );

                    let token = lexer
                        .not_expect(&[
                            Json::Object,
                            Json::Array,
                            Json::Comma,
                            Json::Colon,
                        ])
                        .unwrap()
                        .unwrap();
                    assert_eq!(lexer.format(&token), "\"type\"");
                    assert_eq!(token.kind(), &Json::String);

                    let token = lexer.expect(&[Json::Colon]).unwrap().unwrap();
                    assert_eq!(lexer.format(&token), ":");
                    assert_eq!(token.kind(), &Json::Colon);

                    let token = lexer
                        .not_expect(&[Json::Comma, Json::Colon])
                        .unwrap()
                        .unwrap();
                    assert_eq!(lexer.format(&token), "\"office\"");
                    assert_eq!(token.kind(), &Json::String);

                    let token = lexer.expect(&[Json::Comma]).unwrap().unwrap();
                    assert_eq!(lexer.format(&token), ",");
                    assert_eq!(token.kind(), &Json::Comma);

                    // ---

                    let token = lexer
                        .not_expect(&[
                            Json::Object,
                            Json::Array,
                            Json::Comma,
                            Json::Colon,
                        ])
                        .unwrap()
                        .unwrap();
                    assert_eq!(lexer.format(&token), "\"number\"");
                    assert_eq!(token.kind(), &Json::String);

                    let token = lexer.expect(&[Json::Colon]).unwrap().unwrap();
                    assert_eq!(lexer.format(&token), ":");
                    assert_eq!(token.kind(), &Json::Colon);

                    let token = lexer
                        .not_expect(&[Json::Comma, Json::Colon])
                        .unwrap()
                        .unwrap();
                    assert_eq!(lexer.format(&token), "\"646 555-4567\"");
                    assert_eq!(token.kind(), &Json::String);
                }
            }

            let token = lexer.expect(&[Json::Comma]).unwrap().unwrap();
            assert_eq!(lexer.format(&token), ",");
            assert_eq!(token.kind(), &Json::Comma);

            // ---

            let token = lexer
                .not_expect(&[
                    Json::Object,
                    Json::Array,
                    Json::Comma,
                    Json::Colon,
                ])
                .unwrap()
                .unwrap();
            assert_eq!(lexer.format(&token), "\"children\"");
            assert_eq!(token.kind(), &Json::String);

            let token = lexer.expect(&[Json::Colon]).unwrap().unwrap();
            assert_eq!(lexer.format(&token), ":");
            assert_eq!(token.kind(), &Json::Colon);

            let token = lexer.expect(&[Json::Array]).unwrap().unwrap();
            assert_eq!(token.kind(), &Json::Array);

            {
                let mut lexer = JsonLexer(
                    Builder::new(source)
                        .with_range(token.inner_range())
                        .build()
                        .unwrap(),
                );

                let token = lexer
                    .not_expect(&[Json::Colon, Json::Comma])
                    .unwrap()
                    .unwrap();
                assert_eq!(lexer.format(&token), "\"Catherine\"");
                assert_eq!(token.kind(), &Json::String);

                let token = lexer.expect(&[Json::Comma]).unwrap().unwrap();
                assert_eq!(lexer.format(&token), ",");
                assert_eq!(token.kind(), &Json::Comma);

                // ---

                let token = lexer
                    .not_expect(&[Json::Colon, Json::Comma])
                    .unwrap()
                    .unwrap();
                assert_eq!(lexer.format(&token), "\"Thomas\"");
                assert_eq!(token.kind(), &Json::String);

                let token = lexer.expect(&[Json::Comma]).unwrap().unwrap();
                assert_eq!(lexer.format(&token), ",");
                assert_eq!(token.kind(), &Json::Comma);

                // ---

                let token = lexer
                    .not_expect(&[Json::Colon, Json::Comma])
                    .unwrap()
                    .unwrap();
                assert_eq!(lexer.format(&token), "\"Trevor\"");
                assert_eq!(token.kind(), &Json::String);
            }

            let token = lexer.expect(&[Json::Comma]).unwrap().unwrap();
            assert_eq!(lexer.format(&token), ",");
            assert_eq!(token.kind(), &Json::Comma);

            // ---

            let token = lexer
                .not_expect(&[
                    Json::Object,
                    Json::Array,
                    Json::Comma,
                    Json::Colon,
                ])
                .unwrap()
                .unwrap();
            assert_eq!(lexer.format(&token), "\"spouse\"");
            assert_eq!(token.kind(), &Json::String);

            let token = lexer.expect(&[Json::Colon]).unwrap().unwrap();
            assert_eq!(lexer.format(&token), ":");
            assert_eq!(token.kind(), &Json::Colon);

            let token = lexer
                .not_expect(&[Json::Comma, Json::Colon])
                .unwrap()
                .unwrap();
            assert_eq!(lexer.format(&token), "null");
            assert_eq!(token.kind(), &Json::Sequence);
        }
    }

    #[test]
    fn empty_source() {
        let source = b"";

        assert_eq!(
            Err(error::basic::Error::SourceIsEmpty),
            Builder::new(source).build()
        );
    }

    #[test]
    fn find_block() {
        let source = b"{123}";

        let mut lexer = JsonLexer(Builder::new(source).build().unwrap());
        let block_range = lexer.find_block(b'{', b'}').unwrap();
        assert_eq!(block_range, 0..5);
    }

    #[test]
    fn find_complicated_block() {
        let source = b"{ \"} {\" }";

        let mut lexer = JsonLexer(Builder::new(source).build().unwrap());
        let block_range = lexer.find_block(b'{', b'}').unwrap();
        assert_eq!(block_range, 0..9);
    }

    #[test]
    fn colon() {
        let source = b":";

        let mut lexer = JsonLexer(Builder::new(source).build().unwrap());
        let token = lexer.next();
        assert_eq!(token, Some(Ok(Token::new(Json::Colon, 0..1))));

        let token = lexer.next();
        assert_eq!(token, None);
        assert_eq!(*lexer.cursor().position(), 1);
    }

    #[test]
    fn comma() {
        let source = b",";

        let mut lexer = JsonLexer(Builder::new(source).build().unwrap());
        let token = lexer.next();
        assert_eq!(token, Some(Ok(Token::new(Json::Comma, 0..1))));

        let token = lexer.next();
        assert_eq!(token, None);
        assert_eq!(*lexer.cursor().position(), 1);
    }

    #[test]
    fn empty_object() {
        let source = b"{}";

        let mut lexer = JsonLexer(Builder::new(source).build().unwrap());
        let token = lexer.next();
        assert_eq!(token, Some(Ok(Token::new(Json::Object, 0..2))));

        let token = lexer.next();
        assert_eq!(token, None);
        assert_eq!(*lexer.cursor().position(), 2);
    }

    #[test]
    fn empty_array() {
        let source = b"[]";

        let mut lexer = JsonLexer(Builder::new(source).build().unwrap());
        let token = lexer.next();
        assert_eq!(token, Some(Ok(Token::new(Json::Array, 0..2))));

        let token = lexer.next();
        assert_eq!(token, None);
        assert_eq!(*lexer.cursor().position(), 2);
    }

    #[test]
    fn empty_string() {
        let source = b"\"\"";

        let mut lexer = JsonLexer(Builder::new(source).build().unwrap());
        let token = lexer.next();
        assert_eq!(token, Some(Ok(Token::new(Json::String, 0..2))));

        let token = lexer.next();
        assert_eq!(token, None);
        assert_eq!(*lexer.cursor().position(), 2);
    }

    #[test]
    fn filled_object() {
        let source = b"{\"key\":\"value\"}";

        let mut lexer = JsonLexer(Builder::new(source).build().unwrap());
        let token = lexer.next();
        assert_eq!(token, Some(Ok(Token::new(Json::Object, 0..15))));

        let token = lexer.next();
        assert_eq!(token, None);
        assert_eq!(*lexer.cursor().position(), 15);
    }

    #[test]
    fn incomplete_object() {
        let source = b"{\"key\":\"value\"";

        let mut lexer = JsonLexer(Builder::new(source).build().unwrap());
        let token = lexer.next();
        assert_eq!(
            token,
            Some(Err(error::token::Error::PairNotFound {
                opening: "{".into(),
                closing: "}".into(),
                after: (1, 1)
            }
            .into()))
        );
    }

    #[test]
    fn filled_array() {
        let source = b"[1,2,3,4,5,6,7,8,9,0]";

        let mut lexer = JsonLexer(Builder::new(source).build().unwrap());
        let token = lexer.next();
        assert_eq!(token, Some(Ok(Token::new(Json::Array, 0..21))));

        let token = lexer.next();
        assert_eq!(token, None);
        assert_eq!(*lexer.cursor().position(), 21);
    }

    #[test]
    fn incomplete_array() {
        let source = b"[1,2,3,4,5,6,7,8,9,0";

        let mut lexer = JsonLexer(Builder::new(source).build().unwrap());
        let token = lexer.next();
        assert_eq!(
            token,
            Some(Err(error::token::Error::PairNotFound {
                opening: "[".into(),
                closing: "]".into(),
                after: (1, 1)
            }
            .into()))
        );
    }

    #[test]
    fn filled_string() {
        let source = b"\"John\"";

        let mut lexer = JsonLexer(Builder::new(source).build().unwrap());
        let token = lexer.next();
        assert_eq!(token, Some(Ok(Token::new(Json::String, 0..6))));

        let token = lexer.next();
        assert_eq!(token, None);
        assert_eq!(*lexer.cursor().position(), 6);
    }

    #[test]
    fn escaped_string() {
        let source = b"\"a\\\"b\"";

        let mut lexer = JsonLexer(Builder::new(source).build().unwrap());
        let token = lexer.next();
        assert_eq!(token, Some(Ok(Token::new(Json::String, 0..6))));

        let token = lexer.next();
        assert_eq!(token, None);
        assert_eq!(*lexer.cursor().position(), 6);
    }

    #[test]
    fn incomplete_string() {
        let source = b"\"John";

        let mut lexer = JsonLexer(Builder::new(source).build().unwrap());
        let token = lexer.next();
        assert_eq!(
            token,
            Some(Err(error::token::Error::PairNotFound {
                opening: "\"".into(),
                closing: "\"".into(),
                after: (1, 1)
            }
            .into()))
        );
    }

    #[test]
    fn sequence() {
        let source = b"asd123qwe456";

        let mut lexer = JsonLexer(Builder::new(source).build().unwrap());
        let token = lexer.next();
        assert_eq!(token, Some(Ok(Token::new(Json::Sequence, 0..12))));

        let token = lexer.next();
        assert_eq!(token, None);
    }
}
