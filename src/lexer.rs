//! # [`Lexer`]
//!
//! ![](https://github.com/mnmun/images/blob/main/box.png?raw=true)
//!
//! Defines a [`lexer`] that extracts [`tokens`] tagged with the [`JSON`] kinds
//! from the source raw bytes.
//!
//! ---
//!
//! See the [`crate`] documentation for more information.
//!
//! [`lexer`]: crate::Lexer
//! [`tokens`]: crate::a_bc::Token
//! [`JSON`]: crate::JSON

use std::{
    borrow::Cow,
    fmt,
    ops::{Deref, Range},
    str::from_utf8,
};

use a_bc::{
    Token,
    cancellation::Flag,
    error::{self, Relation, row_col_pos},
    memchr::memchr,
    traits::InnerRange,
    utils::{Needle, count_needles_considering_escaped_delimiters},
};

/// # [`Token`] kinds recognized by the [`lexer`]
///
/// ![](https://github.com/mnmun/images/blob/main/trail.png?raw=true)
///
/// See the [`module`] documentation for more information.
///
/// [`Token`]: crate::a_bc::Token
/// [`lexer`]: crate::Lexer
/// [`module`]: crate::lexer
#[derive(PartialEq, Eq, Clone, Copy, Debug)]
pub enum JSON {
    /// The comma separator `,`
    Comma,

    /// The name/value separator `:`
    Colon,

    /// An object delimited by `{` and `}`
    Object,

    /// An array delimited by `[` and `]`
    Array,

    /// A double-quoted string
    String,

    /// A bareword sequence: numbers, `true`, `false`, `null`, etc.
    Sequence,
}

impl fmt::Display for JSON {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match *self {
            JSON::Comma => write!(f, "Comma"),
            JSON::Colon => write!(f, "Colon"),
            JSON::Object => write!(f, "Object"),
            JSON::Array => write!(f, "Array"),
            JSON::String => write!(f, "String"),
            JSON::Sequence => write!(f, "Sequence"),
        }
    }
}

impl InnerRange for JSON {
    /// # Returns the range inside the delimiters for the [`object`], [`array`], and [`string`] [`kinds`]
    ///
    /// For all other [`kinds`], `None` is returned.
    ///
    /// ---
    ///
    /// See the [`JSON`] documentation for more information.
    ///
    /// [`object`]: crate::JSON::Object
    /// [`array`]: crate::JSON::Array
    /// [`string`]: crate::JSON::String
    /// [`kinds`]: crate::JSON
    /// [`JSON`]: crate::JSON
    fn inner_range(&self, range: &Range<usize>) -> Option<Range<usize>> {
        match self {
            JSON::Comma | JSON::Colon | JSON::Sequence => None,
            JSON::Object | JSON::Array | JSON::String => {
                Some(range.start + 1..range.end - 1)
            }
        }
    }
}

/// # `Lexer`
///
/// ![](https://github.com/mnmun/images/blob/main/box.png?raw=true)
///
/// Newtype wrapper around the [`a_bc::Lexer`] type that extracts [`tokens`]
/// of the following [`kinds`] from the source raw bytes:
///
/// - [`Comma`] - comma `,` character;
/// - [`Colon`] - colon `:` character;
/// - [`Object`] - curly braces `{` `}` and all characters between them;
/// - [`Array`] - square brackets `[` `]` and all characters between them;
/// - [`String`] - double quotes `"` `"` and all characters between them;
/// - [`Sequence`] - a contiguous run of characters: numbers, `true`, `false`,
///   `null`, etc.; [whitespace characters] as well as the `:` and `,`
///   characters act as separators.
///
/// It is important to note the [`sequence`] [`kind`]: the `lexer` does not
/// distinguish between numbers, `null` and the boolean values `true` and
/// `false`. Instead, it groups them all under the [`sequence`] category,
/// leaving it up to the developer to determine whether a [`token's`] value is
/// valid.
///
/// [`Tokens`] may be separated by any number of [whitespace characters].
///
/// The `lexer` supports escaping double-quote characters `"` with a backslash
/// character `\`. Note that a backslash can itself be escaped; in that case it
/// no longer escapes the following quote.
///
/// ## Creation
///
/// Can be created using the [`builder`]:
///
/// ```rust
/// use lazy_json::{Lexer, lexer::Builder};
///
/// let source: &'static [u8] = br#" { "name": "value" } "#;
///
/// let lexer: Lexer = Builder::new(source).build().unwrap();
/// ```
///
/// ## Behavior
///
/// Unlike `lexers` that examine all characters of the source data one by one
/// this `lexer` skips a portion of the characters and leaves them unexamined
/// if they do not belong to the [`token`] being extracted.
///
/// Consider the following source data:
///
/// ```rust
/// # use pretty_assertions::assert_eq;
/// #
/// use lazy_json::{JSON, Lexer, lexer::Builder, a_bc::Token};
///
/// let source: &'static [u8] = br#"
/// {
///     "name": "value"
/// }
/// "#;
///
/// let mut lexer: Lexer = Builder::new(source).build().unwrap();
///
/// let token: Token<JSON> = lexer.next().unwrap().unwrap();
/// assert_eq!(token.kind(), &JSON::Object)
/// ```
///
/// Conventional `lexer` would produce a sequence similar to the following:
/// `CurlyOpen`, `String`, `Colon`, `String` and `CurlyClose`. This `lexer`, by
/// contrast, returns a single [`object`] [`token`]; while extracting it, all
/// the characters between the opening `{` and closing `}` curly braces are
/// skipped. Simply put, the `lexer` indicates that the source data contains
/// an object, while its content remain unchecked and unknown.
///
/// To extract [`tokens`] from the [`object's`] content, you must descend to
/// the content level - that is, specify a range of source data for the
/// `lexer` that lies between the opening `{` and closing `}` curly braces
/// using the [`inner_range()`] method:
///
/// ```rust
/// # use pretty_assertions::assert_eq;
/// #
/// use lazy_json::{JSON, Lexer, lexer::Builder, a_bc::Token};
///
/// let source: &'static [u8] = br#"
/// {
///     "name": "value"
/// }
/// "#;
///
/// let mut lexer: Lexer = Builder::new(source).build().unwrap();
///
/// let token: Token<JSON> = lexer.next().unwrap().unwrap();
/// assert_eq!(token.kind(), &JSON::Object);
///
/// let mut lexer: Lexer =
///     Builder::new(source)
///         .with_range(token.inner_range().unwrap())
///         .build()
///         .unwrap();
///
/// let token: Token<JSON> = lexer.next().unwrap().unwrap();
/// assert_eq!(token.kind(), &JSON::String);
///
/// let token: Token<JSON> = lexer.next().unwrap().unwrap();
/// assert_eq!(token.kind(), &JSON::Colon);
///
/// let token: Token<JSON> = lexer.next().unwrap().unwrap();
/// assert_eq!(token.kind(), &JSON::String);
/// ```
///
/// The same approach is used for [`arrays`] delimited by `[` and `]`.
///
/// ## Examples
///
/// ![](https://github.com/mnmun/images/blob/main/bulb.png?raw=true)
///
/// `Lexer` implements the [`Iterator`] trait - use the [`next()`] method to
/// extract the next [`token`] from the source data:
///
/// ```rust
/// # use pretty_assertions::assert_eq;
/// #
/// use lazy_json::{JSON, Lexer, lexer::Builder};
///
/// let source: &'static [u8] = br#" "foo" "bar" "#;
///
/// let mut lexer: Lexer = Builder::new(source).build().unwrap();
///
/// let token = lexer.next().unwrap().unwrap();
/// assert_eq!(token.kind(), &JSON::String);
/// assert_eq!(lexer.format(&token).unwrap(), r#""foo""#);
///
/// let token = lexer.next().unwrap().unwrap();
/// assert_eq!(token.kind(), &JSON::String);
/// assert_eq!(lexer.format(&token).unwrap(), r#""bar""#);
/// ```
///
/// `Lexer` implements the [`Deref`] trait, which makes the methods of the
/// inner type accessible:
///
/// ```rust
/// # use pretty_assertions::assert_eq;
/// #
/// use lazy_json::{JSON, Lexer, lexer::Builder};
///
/// let source: &'static [u8] = br#" "foo" "bar" "#;
///
/// let lexer: Lexer = Builder::new(source).build().unwrap();
/// assert_eq!(*lexer.data().source(), source);
/// assert_eq!(*lexer.data().range(), 0..source.len());
/// ```
///
/// `Lexer` provides convenient [`expect()`] and [`not_expect()`] methods for
/// cases where a specific [`token`] [`kind`] is required:
///
/// ```rust
/// # use pretty_assertions::assert_eq;
/// #
/// use lazy_json::{JSON, Lexer, lexer::Builder};
///
/// let source: &'static [u8] = br#" "foo" "bar" "#;
///
/// let mut lexer: Lexer = Builder::new(source).build().unwrap();
///
/// let token = lexer.expect(&[JSON::String]).unwrap();
/// assert_eq!(token.kind(), &JSON::String);
/// assert_eq!(lexer.format(&token).unwrap(), r#""foo""#);
///
/// let token = lexer.not_expect(&[JSON::Colon]).unwrap();
/// assert_eq!(token.kind(), &JSON::String);
/// assert_eq!(lexer.format(&token).unwrap(), r#""bar""#);
/// ```
///
/// ---
///
/// See the [`module`] documentation for more information.
///
/// [`module`]: crate::Lexer
/// [`Comma`]: crate::JSON::Comma
/// [`Colon`]: crate::JSON::Colon
/// [`Object`]: crate::JSON::Object
/// [`object`]: crate::JSON::Object
/// [`object's`]: crate::JSON::Object
/// [`Array`]: crate::JSON::Array
/// [`array`]: crate::JSON::Array
/// [`arrays`]: crate::JSON::Array
/// [`String`]: crate::JSON::String
/// [`Sequence`]: crate::JSON::Sequence
/// [`sequence`]: crate::JSON::Sequence
/// [whitespace characters]: u8::is_ascii_whitespace()
/// [`kind`]: crate::JSON
/// [`kinds`]: crate::JSON
/// [`token`]: crate::a_bc::Token
/// [`tokens`]: crate::a_bc::Token
/// [`token's`]: crate::a_bc::Token
/// [`builder`]: crate::lexer::Builder
/// [`next()`]: crate::Lexer::next()
/// [`expect()`]: crate::Lexer::expect()
/// [`not_expect()`]: crate::Lexer::not_expect()
/// [`inner_range()`]: crate::a_bc::traits::InnerRange
/// [`a_bc::Lexer`]: crate::a_bc::Lexer
pub struct Lexer<'source>(a_bc::Lexer<'source>);

impl<'source> Deref for Lexer<'source> {
    type Target = a_bc::Lexer<'source>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<'source> Lexer<'source> {
    /// # Formats a [`token`] as `&str`
    ///
    /// Extracts the slice of the [`source`] occupied by the [`token`] and
    /// converts it into string slice `&str`.
    ///
    /// Returns `None` if the [`token`] [`range`] extends beyond the [`source`].
    ///
    /// ## Panics
    ///
    /// Panics if the extracted slice is not valid UTF-8.
    ///
    /// ## Example
    ///
    /// ```rust
    /// # use pretty_assertions::assert_eq;
    /// #
    /// use lazy_json::{JSON, Lexer, lexer::Builder, a_bc::Token};
    ///
    /// let source: &'static [u8] = br#""hello""#;
    ///
    /// let mut lexer: Lexer = Builder::new(source).build().unwrap();
    ///
    /// let token: Token<JSON> = lexer.next().unwrap().unwrap();
    /// assert_eq!(lexer.format(&token), Some(r#""hello""#));
    /// ```
    ///
    /// ---
    ///
    /// See the [`lexer`] documentation for more information.
    ///
    /// [`token`]: crate::a_bc::Token
    /// [`source`]: crate::a_bc::lexer::Data::source()
    /// [`range`]: crate::a_bc::Token::range()
    /// [`lexer`]: crate::Lexer
    pub fn format<'b>(
        &'source self,
        token: &'b Token<JSON>,
    ) -> Option<&'source str> {
        self.data()
            .source()
            .as_ref()
            .get(token.range().clone())
            .map(|slice| {
                from_utf8(slice).expect("Expected data to be a valid UTF-8")
            })
    }

    /// # Finds a delimited block
    ///
    /// Finds a matching block delimited by `opening` and `closing` characters
    /// starting at the [`cursor`], tracking nesting depth and considering
    /// quoted strings.
    ///
    /// The returned range includes the surrounding `opening` and `closing`
    /// characters, and the [`cursor`] is positioned after the `closing`
    /// character.
    ///
    /// ## Errors
    ///
    /// - [`a_bc::error::Token::PairNotFound`] - the `closing` delimiter
    ///   character is never found;
    /// - [`a_bc::error::Lexer::Cancelled`] - cancellation is [`requested`]
    ///   mid-scan.
    ///
    /// ## Example
    ///
    /// ```ignore
    /// # use pretty_assertions::assert_eq;
    /// #
    /// use lazy_json::{Lexer, lexer::Builder};
    ///
    /// let source: &'static [u8] = br#"{ " } { " }"#;
    /// //                   Chars idx: 01234567890
    ///
    /// let mut lexer: Lexer = Builder::new(source).build().unwrap();
    ///
    /// assert_eq!(lexer.find_block(b'{', b'}').unwrap(), 0..11);
    /// ```
    ///
    /// ---
    ///
    /// See the [`module`] documentation for more information.
    ///
    /// [`cursor`]: crate::a_bc::Lexer::cursor()
    /// [`requested`]: crate::a_bc::cancellation::Flag::cancel()
    /// [`module`]: crate::lexer
    pub(crate) fn find_block(
        &mut self,
        opening: u8,
        closing: u8,
    ) -> Result<Range<usize>, error::Error<JSON>> {
        let initial_position = *self.cursor().position();

        let mut quote_counter = 0;
        let mut opening_counter = 1;
        let mut search_start = initial_position + 1;

        if search_start >= self.data().range().end {
            return Err(error::Token::PairNotFound {
                opening: from_utf8(&[opening])
                    .expect("Expected data to be utf8 compatible")
                    .to_string()
                    .into_boxed_str(),
                closing: from_utf8(&[closing])
                    .expect("Expected data to be utf8 compatible")
                    .to_string()
                    .into_boxed_str(),
                location: (
                    Relation::After,
                    row_col_pos(&self.data().source()[..=initial_position]),
                ),
            }
            .into());
        }

        loop {
            if self.flag().is_cancelled() {
                return Err(error::Lexer::Cancelled.into());
            }

            if let Some(local_position) = memchr(
                closing,
                &self.data().source()[search_start..self.data().range().end],
            ) {
                let end = search_start + local_position;

                opening_counter += count_needles_considering_escaped_delimiters(
                    &Needle::One(opening),
                    br"\",
                    &Needle::One(b'"'),
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
                return Err(error::Token::PairNotFound {
                    opening: from_utf8(&[opening])
                        .expect("Expected data to be utf8 compatible")
                        .to_string()
                        .into_boxed_str(),
                    closing: from_utf8(&[closing])
                        .expect("Expected data to be utf8 compatible")
                        .to_string()
                        .into_boxed_str(),
                    location: (
                        Relation::After,
                        row_col_pos(&self.data().source()[..=initial_position]),
                    ),
                }
                .into());
            }
        }
    }

    /// # Finds a double-quoted string
    ///
    /// Finds a double-quoted string starting at the [`cursor`], skipping over
    /// escaped quotes.
    ///
    /// The returned range includes the surrounding quotes, and the [`cursor`]
    /// is positioned after the closing quote.
    ///
    /// ## Errors
    ///
    /// - [`a_bc::error::Token::PairNotFound`] - the `closing` delimiter
    ///   character is never found;
    /// - [`a_bc::error::Lexer::Cancelled`] - [`cancellation`] is requested
    ///   mid-scan.
    ///
    /// ## Example
    ///
    /// ```ignore
    /// # use pretty_assertions::assert_eq;
    /// #
    /// use lazy_json::{Lexer, lexer::Builder};
    ///
    /// let source: &'static [u8] = br#""John""#;
    /// //                   Chars idx: 012345
    ///
    /// let mut lexer: Lexer = Builder::new(source).build().unwrap();
    ///
    /// assert_eq!(lexer.find_string().unwrap(), 0..6);
    /// ```
    ///
    /// ---
    ///
    /// See the [`module`] documentation for more information.
    ///
    /// [`cursor`]: crate::a_bc::Lexer::cursor()
    /// [`cancellation`]: crate::a_bc::Cancel
    /// [`module`]: crate::lexer
    pub(crate) fn find_string(
        &mut self,
    ) -> Result<Range<usize>, error::Error<JSON>> {
        let initial_position = *self.cursor().position();

        let mut search_start = initial_position + 1;

        if search_start >= self.data().range().end {
            return Err(error::Token::PairNotFound {
                opening: "\"".into(),
                closing: "\"".into(),
                location: (
                    Relation::After,
                    row_col_pos(&self.data().source()[..=initial_position]),
                ),
            }
            .into());
        }

        loop {
            if self.flag().is_cancelled() {
                return Err(error::Lexer::Cancelled.into());
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
                return Err(error::Token::PairNotFound {
                    opening: "\"".into(),
                    closing: "\"".into(),
                    location: (
                        Relation::After,
                        row_col_pos(&self.data().source()[..=initial_position]),
                    ),
                }
                .into());
            }
        }
    }

    /// # Finds a bareword sequence
    ///
    /// Scans a bareword sequence starting at the [`cursor`] until a whitespace,
    /// `:` or `,` characters are reached.
    ///
    /// ## Errors
    ///
    /// Returns [`a_bc::error::Lexer::Cancelled`] if [`cancellation`] is
    /// requested mid-scan.
    ///
    /// ## Example
    ///
    /// ```ignore
    /// # use pretty_assertions::assert_eq;
    /// #
    /// use lazy_json::{Lexer, lexer::Builder};
    ///
    /// let source: &'static [u8] = b"somesequence";
    /// //                 Chars idx: 012345678901
    ///
    /// let mut lexer: Lexer = Builder::new(source).build().unwrap();
    ///
    /// assert_eq!(lexer.find_sequence().unwrap(), 0..12);
    /// ```
    ///
    /// ---
    ///
    /// See the [`module`] documentation for more information.
    ///
    /// [`cursor`]: crate::a_bc::Lexer::cursor()
    /// [`cancellation`]: crate::a_bc::Cancel
    /// [`module`]: crate::lexer
    pub(crate) fn find_sequence(
        &mut self,
    ) -> Result<Range<usize>, error::Error<JSON>> {
        let start = *self.cursor().position();

        loop {
            if self.flag().is_cancelled() {
                return Err(error::Lexer::Cancelled.into());
            }

            if let Some(next_byt) = self.0.peek_next_byte() {
                if next_byt.is_ascii_whitespace() || b":,".contains(&next_byt) {
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

impl<'source> Lexer<'source> {
    /// # Expecting the next [`token`] to be one of the given [`kinds`]
    ///
    /// Accepts a slice of expected [`token`] [`kinds`] and if the next
    /// [`token`] does not exist or its [`kind`] is not in the provided slice,
    /// the function returns an [`a_bc::error::Token::ExpectedButGot`] error.
    /// Otherwise the extracted [`token`] is returned unchanged.
    ///
    /// ## Examples
    ///
    /// The expected [`token`] occurs:
    ///
    /// ```rust
    /// # use pretty_assertions::assert_eq;
    /// #
    /// use lazy_json::{JSON, Lexer, lexer::Builder, a_bc::Token};
    ///
    /// let source: &'static [u8] = br#" " string " "#;
    ///
    /// let mut lexer: Lexer = Builder::new(source).build().unwrap();
    ///
    /// let token: Token<JSON> = lexer
    ///     .expect(&[JSON::String, JSON::Sequence])
    ///     .unwrap();
    /// assert_eq!(token.kind(), &JSON::String);
    /// assert_eq!(lexer.format(&token).unwrap(), r#"" string ""#);
    /// ```
    ///
    /// A [`token`] outside the expected [`kinds`] occurs:
    ///
    /// ```rust
    /// # use pretty_assertions::assert_eq;
    /// #
    /// use lazy_json::{
    ///     JSON, Lexer,
    ///     lexer::Builder,
    ///     a_bc::error::{Token, Relation, Position},
    /// };
    ///
    /// let source: &'static [u8] = br#" { object } "#;
    ///
    /// let mut lexer: Lexer = Builder::new(source).build().unwrap();
    ///
    /// assert!(lexer.expect(&[JSON::String, JSON::Sequence]).is_err_and(|e|
    ///     e == Token::ExpectedButGot {
    ///         expected: [JSON::String, JSON::Sequence].into(),
    ///         got: Some(JSON::Object),
    ///         location: (Relation::At, Position::new(1, 2)),
    ///     }.into()
    /// ));
    /// ```
    ///
    /// The source is exhausted (there are no more [`tokens`]):
    ///
    /// ```rust
    /// # use pretty_assertions::assert_eq;
    /// #
    /// use lazy_json::{
    ///     JSON, Lexer,
    ///     lexer::Builder,
    ///     a_bc::error::{Token, Relation, Position},
    /// };
    ///
    /// // Source containing only three whitespace characters:
    /// let source: &'static [u8] = br#"   "#;
    ///
    /// let mut lexer: Lexer = Builder::new(source).build().unwrap();
    ///
    /// assert!(lexer.expect(&[JSON::String, JSON::Sequence]).is_err_and(|e|
    ///     e == Token::ExpectedButGot {
    ///         expected: [JSON::String, JSON::Sequence].into(),
    ///         got: None,
    ///         location: (Relation::After, Position::new(1, 3)),
    ///     }.into()
    /// ));
    /// ```
    ///
    /// ---
    ///
    /// See the [`lexer`] documentation for more information.
    ///
    /// [`token`]: crate::a_bc::Token
    /// [`tokens`]: crate::a_bc::Token
    /// [`kinds`]: crate::JSON
    /// [`kind`]: crate::JSON
    /// [`lexer`]: crate::Lexer
    pub fn expect(
        &mut self,
        kinds: &[JSON],
    ) -> Result<Token<JSON>, error::Error<JSON>> {
        self.next()
            .ok_or_else(|| {
                error::Token::ExpectedButGot {
                    expected: kinds.to_vec().into_boxed_slice(),
                    got: None,
                    location: (
                        Relation::After,
                        row_col_pos(
                            &self.data().source()
                                [..*self.0.cursor().position()],
                        ),
                    ),
                }
                .into()
            })
            .and_then(|result| {
                result.and_then(|token| {
                    if !kinds.contains(token.kind()) {
                        Err(error::Token::ExpectedButGot {
                            expected: kinds.to_vec().into_boxed_slice(),
                            got: Some(*token.kind()),
                            location: (
                                Relation::At,
                                row_col_pos(
                                    &self.data().source()
                                        [..=token.range().start],
                                ),
                            ),
                        }
                        .into())
                    } else {
                        Ok(token)
                    }
                })
            })
    }

    /// # Not expecting the next [`token`] to be one of the given [`kinds`]
    ///
    /// Accepts a slice of not expected [`token`] [`kinds`] and if the next
    /// [`token`] does not exist or its [`kind`] is in the provided slice, the
    /// function returns an [`a_bc::error::Token::NotExpectedButGot`] error.
    /// Otherwise the extracted [`token`] is returned unchanged.
    ///
    /// ## Examples
    ///
    /// The [`token`] does not match the disallowed [`kinds`]:
    ///
    /// ```rust
    /// # use pretty_assertions::assert_eq;
    /// #
    /// use lazy_json::{JSON, Lexer, lexer::Builder, a_bc::Token};
    ///
    /// let source: &'static [u8] = br#" " string " "#;
    ///
    /// let mut lexer: Lexer = Builder::new(source).build().unwrap();
    ///
    /// let token: Token<JSON> = lexer
    ///     .not_expect(&[JSON::Object, JSON::Sequence])
    ///     .unwrap();
    /// assert_eq!(token.kind(), &JSON::String);
    /// assert_eq!(lexer.format(&token).unwrap(), r#"" string ""#);
    /// ```
    ///
    /// A disallowed [`token`] occurs:
    ///
    /// ```rust
    /// # use pretty_assertions::assert_eq;
    /// #
    /// use lazy_json::{
    ///     JSON, Lexer,
    ///     lexer::Builder,
    ///     a_bc::error::{Token, Relation, Position},
    /// };
    ///
    /// let source: &'static [u8] = br#" { object } "#;
    ///
    /// let mut lexer: Lexer = Builder::new(source).build().unwrap();
    ///
    /// assert!(lexer.not_expect(&[JSON::Object, JSON::Sequence]).is_err_and(|e|
    ///     e == Token::NotExpectedButGot {
    ///         not_expected: [JSON::Object, JSON::Sequence].into(),
    ///         got: Some(JSON::Object),
    ///         location: (Relation::At, Position::new(1, 2)),
    ///     }.into()
    /// ));
    /// ```
    ///
    /// The source is exhausted (there are no more [`tokens`]):
    ///
    /// ```rust
    /// # use pretty_assertions::assert_eq;
    /// #
    /// use lazy_json::{
    ///     JSON, Lexer,
    ///     lexer::Builder,
    ///     a_bc::error::{Token, Relation, Position},
    /// };
    ///
    /// // Source containing only three whitespace characters:
    /// let source: &'static [u8] = br#"   "#;
    ///
    /// let mut lexer: Lexer = Builder::new(source).build().unwrap();
    ///
    /// assert!(lexer.not_expect(&[JSON::Object, JSON::Sequence]).is_err_and(|e|
    ///     e == Token::NotExpectedButGot {
    ///         not_expected: [JSON::Object, JSON::Sequence].into(),
    ///         got: None,
    ///         location: (Relation::After, Position::new(1, 3)),
    ///     }.into()
    /// ));
    /// ```
    ///
    /// ---
    ///
    /// See the [`lexer`] documentation for more information.
    ///
    /// [`token`]: crate::a_bc::Token
    /// [`tokens`]: crate::a_bc::Token
    /// [`kinds`]: crate::JSON
    /// [`kind`]: crate::JSON
    /// [`lexer`]: crate::Lexer
    pub fn not_expect(
        &mut self,
        kinds: &[JSON],
    ) -> Result<Token<JSON>, error::Error<JSON>> {
        self.next()
            .ok_or_else(|| {
                error::Token::NotExpectedButGot {
                    not_expected: kinds.to_vec().into_boxed_slice(),
                    got: None,
                    location: (
                        Relation::After,
                        row_col_pos(
                            &self.data().source()
                                [..*self.0.cursor().position()],
                        ),
                    ),
                }
                .into()
            })
            .and_then(|result| {
                result.and_then(|token| {
                    if kinds.contains(token.kind()) {
                        Err(error::Token::NotExpectedButGot {
                            not_expected: kinds.to_vec().into_boxed_slice(),
                            got: Some(*token.kind()),
                            location: (
                                Relation::At,
                                row_col_pos(
                                    &self.data().source()
                                        [..=token.range().start],
                                ),
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

impl<'source> Iterator for Lexer<'source> {
    type Item = Result<Token<JSON>, error::Error<JSON>>;

    /// # Extracts the next [`token`] from the source data
    ///
    /// [`Tokens`] are separated by [whitespace characters] and its kind is
    /// determined by the first character:
    ///
    /// - `:` -> [`Colon`]
    /// - `,` -> [`Comma`]
    /// - `{` -> [`Object`]
    /// - `[` -> [`Array`]
    /// - `"` -> [`String`]
    /// - anything else -> [`Sequence`]
    ///
    /// On cancellation [`request`], immediately returns the
    /// [`a_bc::error::Lexer::Cancelled`] error and moves the [`cursor`] to the
    /// end of [`range`], so the next requested [`token`] will be `None.`
    ///
    /// ---
    ///
    /// See the [`lexer`] documentation for more information.
    ///
    /// [`Tokens`]: crate::a_bc::Token
    /// [`token`]: crate::a_bc::Token
    /// [`kind`]: crate::JSON
    /// [`Colon`]: crate::JSON::Colon
    /// [`Comma`]: crate::JSON::Comma
    /// [`Object`]: crate::JSON::Object
    /// [`Array`]: crate::JSON::Array
    /// [`String`]: crate::JSON::String
    /// [`Sequence`]: crate::JSON::Sequence
    /// [`request`]: crate::a_bc::cancellation::Flag::cancel()
    /// [`cursor`]: crate::a_bc::lexer::Cursor
    /// [`range`]: crate::a_bc::lexer::Data::range()
    /// [whitespace characters]: u8::is_ascii_whitespace()
    /// [`lexer`]: crate::Lexer
    fn next(&mut self) -> Option<Self::Item> {
        self.0.skip_whitespace();

        let token = self.cursor().byte().and_then(|byte| {
            let (kind, range) = match byte {
                b':' => (
                    JSON::Colon,
                    Ok(*self.cursor().position()..self.cursor().position() + 1),
                ),
                b',' => (
                    JSON::Comma,
                    Ok(*self.cursor().position()..self.cursor().position() + 1),
                ),
                b'{' => (JSON::Object, self.find_block(b'{', b'}')),
                b'[' => (JSON::Array, self.find_block(b'[', b']')),
                b'"' => (JSON::String, self.find_string()),
                _ => (JSON::Sequence, self.find_sequence()),
            };

            match range {
                Ok(range) => Some(Ok(Token::new(kind, range))),
                Err(e) => {
                    self.0.set_position(self.0.data().range().end);
                    Some(Err(e))
                }
            }
        });

        self.0.read_next_byte();

        token
    }
}

/// # [`Lexer`] `builder`
///
/// ![](https://github.com/mnmun/images/blob/main/road_work.png?raw=true)
///
/// Holds the data used for the [`lexer`] creation.
///
/// `Builder` is a simple newtype wrapper around the [`a_bc::lexer::Builder`]
/// type, provided for convenience when creating a [`lexer`] and implements
/// all the same methods that the original type has.
///
/// ## Creation
///
/// Can be created using the [`Builder::new()`] method.
///
/// ## Build
///
/// After setting all the necessary fields, call the [`build()`] method to get
/// a [`lexer`] instance.
///
/// ---
///
/// See the [`lexer`] documentation for more information.
///
/// [`lexer`]: crate::Lexer
/// [`Builder::new()`]: crate::lexer::Builder::new()
/// [`build()`]: crate::lexer::Builder::build()
pub struct Builder<'source>(a_bc::lexer::Builder<'source>);

impl<'source> Builder<'source> {
    pub fn new(source: impl Into<Cow<'source, [u8]>>) -> Self {
        Self(a_bc::lexer::Builder::new(source))
    }

    pub fn build(self) -> Result<Lexer<'source>, error::Lexer> {
        Ok(Lexer(self.0.build()?))
    }
}

impl Builder<'_> {
    /// See the [`builder`] documentation for more information.
    ///
    /// [`builder`]: crate::lexer::Builder
    pub fn set_range(
        &mut self,
        value: impl Into<Option<Range<usize>>>,
    ) -> &mut Self {
        self.0.set_range(value);
        self
    }

    /// See the [`builder`] documentation for more information.
    ///
    /// [`builder`]: crate::lexer::Builder
    pub fn set_position(
        &mut self,
        value: impl Into<Option<usize>>,
    ) -> &mut Self {
        self.0.set_position(value);
        self
    }

    /// See the [`builder`] documentation for more information.
    ///
    /// [`builder`]: crate::lexer::Builder
    pub fn set_flag(&mut self, value: impl Into<Option<Flag>>) -> &mut Self {
        self.0.set_flag(value);
        self
    }
}

impl Builder<'_> {
    /// See the [`builder`] documentation for more information.
    ///
    /// [`builder`]: crate::lexer::Builder
    #[must_use = "method returns the modified value"]
    pub fn with_range(
        mut self,
        value: impl Into<Option<Range<usize>>>,
    ) -> Self {
        self.0.set_range(value);
        self
    }

    /// See the [`builder`] documentation for more information.
    ///
    /// [`builder`]: crate::lexer::Builder
    #[must_use = "method returns the modified value"]
    pub fn with_position(mut self, value: impl Into<Option<usize>>) -> Self {
        self.0.set_position(value);
        self
    }

    /// See the [`builder`] documentation for more information.
    ///
    /// [`builder`]: crate::lexer::Builder
    #[must_use = "method returns the modified value"]
    pub fn with_flag(mut self, value: impl Into<Option<Flag>>) -> Self {
        self.0.set_flag(value);
        self
    }
}
