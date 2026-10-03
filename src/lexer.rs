//! # Lazy JSON lexer
//!
//! ![](https://github.com/mnmun/images/blob/main/paper.png?raw=true)
//!
//! Provides a lazy JSON [`lexer`] based on the [`a_bc`] crate.
//!
//! The [`lexer`] operates on data represented as a collection of `u8` values.
//! Typically, the [`source`] data is provided as an `&str` and converted to
//! `&[u8]` using the [`.as_bytes()`] method. Although the [`lexer`] actually
//! operates on raw bytes internally, the following documentation will use the
//! terminology associated with string data - specifically, the term "character".
//!
//! The [`lexer`] [`extracts`] [`tokens`] of the following [`kinds`] from a
//! [`source`] data:
//!
//! - [`Comma`] - the `,` character;
//! - [`Colon`] - the `:` character;
//! - [`Object`] - the `{` and `}` characters together with every character
//!   between them;
//! - [`Array`] - the `[` and `]` characters together with every character
//!   between them;
//! - [`String`] - the `"` and `"` characters together with every character
//!   between them;
//! - [`Sequence`] - a contiguous run of characters; [`whitespace`] as well as
//!   the `:` and `,` characters act as separators.
//!
//! [`Tokens`] may be separated by any number of [`whitespace characters`].
//!
//! The [`lexer`] supports escaping double-quote characters `"` with a backslash
//! character `\`. Note that a backslash can itself be escaped; in that case it
//! no longer escapes the following quote.
//!
//! ## Use cases
//!
//! The [`lexer`] was originally designed to back the lazy JSON [`parser`], but
//! it can equally be used on its own. It [`extracts`] only top-level
//! [`tokens`], skipping the characters of deeper [`tokens`] that are not
//! currently being [`extracted`]. This approach may be beneficial for large
//! JSON data with deep nesting.
//!
//! ## Behavior
//!
//! Unlike lexers that examine the characters of the source data one by one
//! this [`lexer`] skips a portion of the characters and leaves them unexamined.
//! Put differently, within the given hierarchical structure of JSON, the
//! [`lexer`] [`extracts`] only the top-level [`tokens`].
//!
//! Consider the following [`source`] data:
//!
//! ```json
//! {
//!     "name": "value"
//! }
//! ```
//!
//! A conventional lexer would produce a sequence similar to the following:
//! `CurlyOpen`, `String`, `Colon`, `String` and `CurlyClose`. This [`lexer`],
//! by contrast, returns a single [`Object`] [`token`]; while assembling it, all
//! the characters between the opening `{` and closing `}` braces are skipped.
//!
//! To [`extract`] the [`tokens`] of the [`object`] body, one has to "descend
//! one level down" in the JSON hierarchy - that is, point the [`lexer`] at the
//! [`source`] data inside the [`object's`] braces:
//!
//! ```json
//! "name": "value"
//! ```
//!
//! In this case the lexer returns the token sequence [`String`], [`Colon`],
//! [`String`].
//!
//! The same approach is used for [`arrays`] delimited by `[` and `]`.
//!
//! ## Examples
//!
//! ![](https://github.com/mnmun/images/blob/main/bulb.png?raw=true)
//!
//! The following examples demonstrate how [`tokens`] are [`extracted`] from
//! different inputs.
//!
//! ### Valid [`object`]
//!
//! A correctly formed [`object`] can be [`extracted`] as follows:
//!
//! ```rust
//! use pretty_assertions::assert_eq;
//!
//! use lazy_json::{
//!     Json,
//!     Lexer,
//!     Token,
//!     a_bc::lexer::Builder
//! };
//!
//! // Valid source data - an object with two fields
//! let valid_source: &[u8] = br#"
//! {
//!     "first_name": "John",
//!     "second_name": "Smith"
//! }
//! "#;
//!
//! // A lexer operating over the whole source data:
//! // {
//! //     "first_name": "John",
//! //     "second_name": "Smith"
//! // }
//! let mut lexer = Lexer::new(Builder::new(valid_source).build().unwrap());
//!
//! // For the current source segment, the lexer yields a single top-level
//! // `Json::Object` token: its fields are not lexed yet
//! let token = lexer.next().unwrap().unwrap();
//! assert_eq!(*token.kind(), Json::Object);
//!
//! // To lex the object's fields, a new lexer is created within the object
//! // boundaries
//! let mut lexer = Lexer::new(
//!     // The whole source data is used for diagnostic messages
//!     Builder::new(valid_source)
//!         // The lexer operates only on the object's inner range
//!         .with_range(token.inner_range().unwrap())
//!         .build()
//!         .unwrap()
//! );
//! // Now lexer is operating over the following segment of the source data:
//! // "first_name": "John",
//! // "second_name": "Smith"
//!
//! let token = lexer.next().unwrap().unwrap();
//! assert_eq!(*token.kind(), Json::String);
//! assert_eq!(lexer.format(&token).unwrap(), r#""first_name""#);
//!
//! let token = lexer.next().unwrap().unwrap();
//! assert_eq!(*token.kind(), Json::Colon);
//! assert_eq!(lexer.format(&token).unwrap(), ":");
//!
//! let token = lexer.next().unwrap().unwrap();
//! assert_eq!(*token.kind(), Json::String);
//! assert_eq!(lexer.format(&token).unwrap(), r#""John""#);
//!
//! let token = lexer.next().unwrap().unwrap();
//! assert_eq!(*token.kind(), Json::Comma);
//! assert_eq!(lexer.format(&token).unwrap(), ",");
//!
//! let token = lexer.next().unwrap().unwrap();
//! assert_eq!(*token.kind(), Json::String);
//! assert_eq!(lexer.format(&token).unwrap(), r#""second_name""#);
//!
//! let token = lexer.next().unwrap().unwrap();
//! assert_eq!(*token.kind(), Json::Colon);
//! assert_eq!(lexer.format(&token).unwrap(), ":");
//!
//! let token = lexer.next().unwrap().unwrap();
//! assert_eq!(*token.kind(), Json::String);
//! assert_eq!(lexer.format(&token).unwrap(), r#""Smith""#);
//!
//! let token = lexer.next();
//! assert_eq!(token, None);
//! ```
//!
//! ### Malformed value
//!
//! [`Token`] characters corresponding to deep levels in the hierarchy are
//! skipped during [`extracting`]:
//!
//! ```rust
//! use pretty_assertions::assert_eq;
//!
//! use lazy_json::{
//!     Json,
//!     Lexer,
//!     Token,
//!     error::{self, Relation, Position},
//!     a_bc::lexer::Builder,
//! };
//!
//! // Invalid source data - valid top-level object with invalid field
//! let invalid_source: &[u8] = br#"
//! {
//!     "invalid_array": [ "missing" "brace"
//! }
//! "#;
//!
//! // A lexer operating over the whole source data:
//! // {
//! //     "invalid_array": [ "missing" "brace"
//! // }
//! let mut lexer = Lexer::new(Builder::new(invalid_source).build().unwrap());
//!
//! // Lexer successfully extracted the top-level `Json::Object` token despite
//! // the malformed `Json::Array` field inside it
//! let token = lexer.next().unwrap().unwrap();
//! assert_eq!(*token.kind(), Json::Object);
//!
//! // To lex the object's fields, a new lexer is created within the object
//! // boundaries
//! let mut lexer = Lexer::new(
//!     // The whole source data is used for diagnostic messages
//!     Builder::new(invalid_source)
//!         // The lexer operates only on the object's inner range
//!         .with_range(token.inner_range().unwrap())
//!         .build()
//!         .unwrap()
//! );
//! // Now lexer is operating over the following segment of the source data:
//! // "invalid_array": [ "missing" "brace"
//!
//! let token = lexer.next().unwrap().unwrap();
//! assert_eq!(*token.kind(), Json::String);
//! assert_eq!(lexer.format(&token).unwrap(), r#""invalid_array""#);
//!
//! let token = lexer.next().unwrap().unwrap();
//! assert_eq!(*token.kind(), Json::Colon);
//! assert_eq!(lexer.format(&token).unwrap(), ":");
//!
//! // The error is returned only when an attempt is made to parse the incorrect
//! // token: `lexer.next()` returned `Some(Err(...))` - a token exists, but an
//! // error occurred while extracting it
//! assert!(lexer.next().unwrap().is_err_and(|e|
//!     e == error::Token::PairNotFound {
//!         opening: "[".into(),
//!         closing: "]".into(),
//!         // Diagnostic data: `Position::new(3, 22)` points to the openning
//!         // brace in the source data (row 3, column 22)
//!         location: (Relation::After, Position::new(3, 22))
//!     }.into()
//! ));
//! ```
//!
//! ### Empty [`source`] data
//!
//! A [`lexer`] cannot be created over empty [`source`] data:
//!
//! ```rust
//! use lazy_json::{error, a_bc::lexer::Builder};
//!
//! let empty_source: &[u8] = b"";
//!
//! assert!(Builder::new(empty_source).build().is_err_and(|e|
//!     e == error::Lexer::SourceIsEmpty
//! ));
//! ```
//!
//! ### Escaping
//!
//! Double-quotes are escaped with backslashes:
//!
//! ```rust
//! use pretty_assertions::assert_eq;
//!
//! use lazy_json::{
//!     Json,
//!     Lexer,
//!     Token,
//!     a_bc::lexer::Builder,
//! };
//!
//! // Valid source data - a string with escaped double-quotes inside:
//! let source: &[u8] = br#" " escaped \" double \" quotes \\" "#;
//! // Note that the last double-quote                       ^
//! // is treated as the closing quote,                      │
//! // not as an escaped one ────────────────────────────────┘
//!
//! // A lexer operating over the whole source data:
//! // " escaped \" double \" quotes \\"
//! let mut lexer = Lexer::new(Builder::new(source).build().unwrap());
//!
//! let token = lexer.next().unwrap().unwrap();
//! assert_eq!(*token.kind(), Json::String);
//! assert_eq!(lexer.format(&token).unwrap(), r#"" escaped \" double \" quotes \\""#);
//!
//! let token = lexer.next();
//! assert_eq!(token, None);
//! ```
//!
//! ---
//!
//! See the [`crate`] documentation for more information.
//!
//! [`lexer`]: Lexer
//! [`source`]: a_bc::lexer::Data::source()
//! [`.as_bytes()`]: str::as_bytes()
//! [`extract`]: Lexer::next()
//! [`extracts`]: Lexer::next()
//! [`extracted`]: Lexer::next()
//! [`extracting`]: Lexer::next()
//! [`Extracting`]: Lexer::next()
//! [`token`]: a_bc::Token
//! [`Token`]: a_bc::Token
//! [`tokens`]: a_bc::Token
//! [`Tokens`]: a_bc::Token
//! [`kinds`]: Json
//! [`Comma`]: Json::Comma
//! [`Colon`]: Json::Colon
//! [`object`]: Json::Object
//! [`object's`]: Json::Object
//! [`Object`]: Json::Object
//! [`arrays`]: Json::Array
//! [`Array`]: Json::Array
//! [`String`]: Json::String
//! [`Sequence`]: Json::Sequence
//! [`whitespace`]: u8::is_ascii_whitespace()
//! [`whitespace characters`]: u8::is_ascii_whitespace()
//! [`builder`]: a_bc::lexer::Builder
//! [`parser`]: crate::parser
//! [`error`]: a_bc::error

use std::{fmt, ops::Range, str::from_utf8};

use crate::a_bc::{
    Cancel,
    error::{self, Relation, row_col_pos},
    lexer::{Cursor, Data},
    memchr::memchr,
    token::Token,
    traits::InnerRange,
    utils::{Needle, count_needles_considering_escaped_delimiters},
};

/// # [`Token`] kinds recognized by the [`lexer`]
///
/// ![](https://github.com/mnmun/images/blob/main/trail.png?raw=true)
///
/// See the [`module`] documentation for more information.
///
/// [`Token`]: a_bc::Token
/// [`lexer`]: Lexer
/// [`module`]: crate::lexer
#[derive(PartialEq, Eq, Clone, Copy, Debug)]
pub enum Json {
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

impl fmt::Display for Json {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match *self {
            Json::Comma => write!(f, "Comma"),
            Json::Colon => write!(f, "Colon"),
            Json::Object => write!(f, "Object"),
            Json::Array => write!(f, "Array"),
            Json::String => write!(f, "String"),
            Json::Sequence => write!(f, "Sequence"),
        }
    }
}

impl InnerRange for Json {
    /// # Returns the range inside the delimiters for the [`object`], [`array`], and [`string`] [`kinds`]
    ///
    /// For all other [`kinds`], `None` is returned.
    ///
    /// ---
    ///
    /// See the [`module`] documentation for more information.
    ///
    /// [`object`]: Json::Object
    /// [`array`]: Json::Array
    /// [`string`]: Json::String
    /// [`kinds`]: Json
    /// [`module`]: crate::lexer
    fn inner_range(&self, range: &Range<usize>) -> Option<Range<usize>> {
        match self {
            Json::Comma | Json::Colon | Json::Sequence => None,
            Json::Object | Json::Array | Json::String => {
                Some(range.start + 1..range.end - 1)
            }
        }
    }
}

/// # Lazy JSON `lexer`
///
/// ![](https://github.com/mnmun/images/blob/main/paper.png?raw=true)
///
/// A newtype wrapper around an [`a_bc::Lexer`] that produces [`tokens`] tagged
/// with [`kinds`].
///
/// ## Methods
///
/// Use the following methods to inspect the [`inner lexer`](`a_bc::Lexer`)
/// state:
///
/// - [`data()`] - the [`source bytes`] and the [`active scan range`];
/// - [`cursor()`] - the [`currently observed byte`] and its [`position`] within
///   the [`source`];
/// - [`flag()`] - a cloneable thread-safe [`cancellation flag`].
///
/// The `lexer` implements the [`Iterator`] trait - use the [`next()`] method to
/// extract the next [`token`] from the [`source`] data.
///
/// In addition, the `lexer` has the following useful methods:
///
/// - [`expect()`] - attempts to [`extract`] the next [`token`] given a list of
///   expected [`kinds`];
/// - [`not_expect()`] - attempts to [`extract`] the next [`token`] given a list
///   of disallowed [`kinds`].
///
/// ## Creation
///
/// Use the [`Lexer::new()`] method to create a `lexer`.
///
/// ---
///
/// See the [`module`] documentation for more information.
///
/// [`tokens`]: a_bc::Token
/// [`kinds`]: Json
/// [`data()`]: Lexer::data()
/// [`source`]: a_bc::lexer::Data::source()
/// [`source bytes`]: a_bc::lexer::Data::source()
/// [`active scan range`]: a_bc::lexer::Data::range()
/// [`cursor()`]: Lexer::cursor()
/// [`currently observed byte`]: a_bc::lexer::Cursor::byte()
/// [`position`]: a_bc::lexer::Cursor::position()
/// [`flag()`]: Lexer::flag()
/// [`cancellation flag`]: a_bc::Cancel
/// [`next()`]: Lexer::next()
/// [`token`]: a_bc::Token
/// [`expect()`]: Lexer::expect()
/// [`extract`]: Lexer::next()
/// [`not_expect()`]: Lexer::not_expect()
/// [`module`]: crate::lexer
pub struct Lexer<'a>(a_bc::Lexer<'a>);

impl<'a> From<a_bc::Lexer<'a>> for Lexer<'a> {
    /// # Converts an [`a_bc::Lexer`] into a [`Lexer`]
    fn from(value: a_bc::Lexer<'a>) -> Self {
        Lexer(value)
    }
}

impl<'a> Lexer<'a> {
    /// # Creates a new [`lexer`] from an [`a_bc::Lexer`]
    ///
    /// See the [`lexer`] documentation for more information.
    ///
    /// [`lexer`]: Lexer
    pub fn new(lexer: a_bc::Lexer<'a>) -> Self {
        Self(lexer)
    }
}

impl Lexer<'_> {
    /// # Returns a reference to the [`data`] of the [`inner lexer`]
    ///
    /// See the [`lexer`] documentation for more information.
    ///
    /// [`data`]: a_bc::Lexer::data()
    /// [`inner lexer`]: a_bc::Lexer
    /// [`lexer`]: Lexer
    pub fn data(&self) -> &Data<'_> {
        self.0.data()
    }

    /// # Returns a reference to the [`cursor`] of the [`inner lexer`]
    ///
    /// See the [`lexer`] documentation for more information.
    ///
    /// [`cursor`]: a_bc::Lexer::cursor()
    /// [`inner lexer`]: a_bc::Lexer
    /// [`lexer`]: Lexer
    pub fn cursor(&self) -> &Cursor {
        self.0.cursor()
    }

    /// # Returns a reference to the [`flag`] of the [`inner lexer`]
    ///
    /// See the [`lexer`] documentation for more information.
    ///
    /// [`flag`]: a_bc::Lexer::flag()
    /// [`inner lexer`]: a_bc::Lexer
    /// [`lexer`]: Lexer
    pub fn flag(&self) -> &Cancel {
        self.0.flag()
    }
}

impl<'a> Lexer<'a> {
    /// # Formats a [`token`] as `&str`
    ///
    /// Extracts the slice of the [`source`] occupied by the [`token`] and
    /// converts it into `&str`.
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
    /// use pretty_assertions::assert_eq;
    ///
    /// use lazy_json::{Lexer, a_bc::lexer::Builder};
    ///
    /// let source = br#""hello""#;
    ///
    /// let mut lexer = Lexer::new(Builder::new(source).build().unwrap());
    /// let token = lexer.next().unwrap().unwrap();
    ///
    /// assert_eq!(lexer.format(&token), Some(r#""hello""#));
    /// ```
    ///
    /// ---
    ///
    /// See the [`lexer`] documentation for more information.
    ///
    /// [`token`]: a_bc::Token
    /// [`source`]: a_bc::lexer::Data::source()
    /// [`range`]: a_bc::Token::range()
    /// [`lexer`]: Lexer
    pub fn format<'b>(&'a self, token: &'b Token<Json>) -> Option<&'a str> {
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
    /// Returns [`PairNotFound`] if the `closing` delimiter character is never
    /// found, or [`Cancelled`] if [`cancellation`] is requested mid-scan.
    ///
    /// ## Example
    ///
    /// ```ignore
    /// use pretty_assertions::assert_eq;
    ///
    /// use lazy_json::{Lexer, a_bc::lexer::Builder};
    ///
    /// let source = br#"{ " } { " }"#;
    /// // Chars idx:    01234567890
    ///
    /// let mut lexer = Lexer::new(Builder::new(source).build().unwrap());
    ///
    /// assert_eq!(lexer.find_block(b'{', b'}').unwrap(), 0..11);
    /// ```
    ///
    /// ---
    ///
    /// See the [`module`] documentation for more information.
    ///
    /// [`cursor`]: Lexer::cursor()
    /// [`PairNotFound`]: a_bc::error::Token::PairNotFound
    /// [`Cancelled`]: a_bc::error::Lexer::Cancelled
    /// [`cancellation`]: a_bc::Cancel
    /// [`module`]: crate::lexer
    fn find_block(
        &mut self,
        opening: u8,
        closing: u8,
    ) -> Result<Range<usize>, error::Error<Json>> {
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
    /// Returns [`PairNotFound`] if the closing quote character is never found,
    /// or [`Cancelled`] if [`cancellation`] is requested mid-scan.
    ///
    /// ## Example
    ///
    /// ```ignore
    /// use pretty_assertions::assert_eq;
    ///
    /// use lazy_json::{Lexer, a_bc::lexer::Builder};
    ///
    /// let source = br#""John""#;
    /// // Chars idx:    012345
    ///
    /// let mut lexer = Lexer::new(Builder::new(source).build().unwrap());
    ///
    /// assert_eq!(lexer.find_string().unwrap(), 0..6);
    /// ```
    ///
    /// ---
    ///
    /// See the [`module`] documentation for more information.
    ///
    /// [`cursor`]: Lexer::cursor()
    /// [`PairNotFound`]: a_bc::error::Token::PairNotFound
    /// [`Cancelled`]: a_bc::error::Lexer::Cancelled
    /// [`cancellation`]: a_bc::Cancel
    /// [`module`]: crate::lexer
    fn find_string(&mut self) -> Result<Range<usize>, error::Error<Json>> {
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
    /// Returns [`Cancelled`] if [`cancellation`] is requested mid-scan.
    ///
    /// ## Example
    ///
    /// ```ignore
    /// use pretty_assertions::assert_eq;
    ///
    /// use lazy_json::{Lexer, a_bc::lexer::Builder};
    ///
    /// let source = b"somesequence";
    /// // Chars idx:  012345678901
    ///
    /// let mut lexer = Lexer::new(Builder::new(source).build().unwrap());
    ///
    /// assert_eq!(lexer.find_sequence().unwrap(), 0..12);
    /// ```
    ///
    /// ---
    ///
    /// See the [`module`] documentation for more information.
    ///
    /// [`cursor`]: Lexer::cursor()
    /// [`Cancelled`]: a_bc::error::Lexer::Cancelled
    /// [`cancellation`]: a_bc::Cancel
    /// [`module`]: crate::lexer
    fn find_sequence(&mut self) -> Result<Range<usize>, error::Error<Json>> {
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

impl<'a> Lexer<'a> {
    /// # Expecting the next [`token`] to be one of the given [`kinds`]
    ///
    /// Accepts a slice of expected [`token`] [`kinds`] and if the next
    /// [`token`] does not exist or its [`kind`] is not in the provided slice,
    /// the function returns an [`ExpectedButGot`] error. Otherwise the
    /// [`extracted`] [`token`] is returned unchanged.
    ///
    /// ## Examples
    ///
    /// The expected [`token`] occurs:
    ///
    /// ```rust
    /// use pretty_assertions::assert_eq;
    ///
    /// use lazy_json::{
    ///     Json,
    ///     Lexer,
    ///     Token,
    ///     a_bc::lexer::Builder,
    /// };
    ///
    /// let source: &[u8] = br#" " string " "#;
    ///
    /// let mut lexer = Lexer::new(Builder::new(source).build().unwrap());
    ///
    /// let token = lexer.expect(&[Json::String, Json::Sequence]).unwrap();
    /// assert_eq!(*token.kind(), Json::String);
    /// assert_eq!(lexer.format(&token).unwrap(), r#"" string ""#);
    /// ```
    ///
    /// A [`token`] outside the expected [`kinds`] occurs:
    ///
    /// ```rust
    /// use pretty_assertions::assert_eq;
    ///
    /// use lazy_json::{
    ///     Json,
    ///     Lexer,
    ///     Token,
    ///     error::{self, Relation, Position},
    ///     a_bc::lexer::Builder,
    /// };
    ///
    /// let source: &[u8] = br#" { object } "#;
    ///
    /// let mut lexer = Lexer::new(Builder::new(source).build().unwrap());
    ///
    /// assert!(lexer.expect(&[Json::String, Json::Sequence]).is_err_and(|e|
    ///     e == error::Token::ExpectedButGot {
    ///         expected: [Json::String, Json::Sequence].into(),
    ///         got: Some(Json::Object),
    ///         location: (Relation::At, Position::new(1, 2)),
    ///     }.into()
    /// ));
    /// ```
    ///
    /// The [`input`] is exhausted (there are no more [`tokens`]):
    ///
    /// ```rust
    /// use pretty_assertions::assert_eq;
    ///
    /// use lazy_json::{
    ///     Json,
    ///     Lexer,
    ///     Token,
    ///     error::{self, Relation, Position},
    ///     a_bc::lexer::Builder,
    /// };
    ///
    /// // Source containing only three whitespace characters:
    /// let source: &[u8] = br#"   "#;
    ///
    /// let mut lexer = Lexer::new(Builder::new(source).build().unwrap());
    ///
    /// assert!(lexer.expect(&[Json::String, Json::Sequence]).is_err_and(|e|
    ///     e == error::Token::ExpectedButGot {
    ///         expected: [Json::String, Json::Sequence].into(),
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
    /// [`token`]: a_bc::Token
    /// [`tokens`]: a_bc::Token
    /// [`kinds`]: Json
    /// [`kind`]: Json
    /// [`ExpectedButGot`]: a_bc::error::Token::ExpectedButGot
    /// [`extracted`]: Lexer::next()
    /// [`input`]: a_bc::lexer::Data::source()
    /// [`lexer`]: Lexer
    pub fn expect(
        &mut self,
        kinds: &[Json],
    ) -> Result<Token<Json>, error::Error<Json>> {
        self.next()
            .ok_or(
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
                .into(),
            )
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
    /// function returns an [`NotExpectedButGot`] error. Otherwise the
    /// [`extracted`] [`token`] is returned unchanged.
    ///
    /// ## Examples
    ///
    /// The [`token`] does not match the disallowed [`kinds`]:
    ///
    /// ```rust
    /// use pretty_assertions::assert_eq;
    ///
    /// use lazy_json::{
    ///     Json,
    ///     Lexer,
    ///     Token,
    ///     a_bc::lexer::Builder,
    /// };
    ///
    /// let source: &[u8] = br#" " string " "#;
    ///
    /// let mut lexer = Lexer::new(Builder::new(source).build().unwrap());
    ///
    /// let token = lexer.not_expect(&[Json::Object, Json::Sequence]).unwrap();
    /// assert_eq!(*token.kind(), Json::String);
    /// assert_eq!(lexer.format(&token).unwrap(), r#"" string ""#);
    /// ```
    ///
    /// A disallowed [`token`] occurs:
    ///
    /// ```rust
    /// use pretty_assertions::assert_eq;
    ///
    /// use lazy_json::{
    ///     Json,
    ///     Lexer,
    ///     Token,
    ///     error::{self, Relation, Position},
    ///     a_bc::lexer::Builder,
    /// };
    ///
    /// let source: &[u8] = br#" { object } "#;
    ///
    /// let mut lexer = Lexer::new(Builder::new(source).build().unwrap());
    ///
    /// assert!(lexer.not_expect(&[Json::Object, Json::Sequence]).is_err_and(|e|
    ///     e == error::Token::NotExpectedButGot {
    ///         not_expected: [Json::Object, Json::Sequence].into(),
    ///         got: Some(Json::Object),
    ///         location: (Relation::At, Position::new(1, 2)),
    ///     }.into()
    /// ));
    /// ```
    ///
    /// The [`input`] is exhausted (there are no more [`tokens`]):
    ///
    /// ```rust
    /// use pretty_assertions::assert_eq;
    ///
    /// use lazy_json::{
    ///     Json,
    ///     Lexer,
    ///     Token,
    ///     error::{self, Relation, Position},
    ///     a_bc::lexer::Builder,
    /// };
    ///
    /// // Source containing only three whitespace characters:
    /// let source: &[u8] = br#"   "#;
    ///
    /// let mut lexer = Lexer::new(Builder::new(source).build().unwrap());
    ///
    /// assert!(lexer.not_expect(&[Json::Object, Json::Sequence]).is_err_and(|e|
    ///     e == error::Token::NotExpectedButGot {
    ///         not_expected: [Json::Object, Json::Sequence].into(),
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
    /// [`token`]: a_bc::Token
    /// [`tokens`]: a_bc::Token
    /// [`kinds`]: Json
    /// [`kind`]: Json
    /// [`NotExpectedButGot`]: a_bc::error::Token::NotExpectedButGot
    /// [`extracted`]: Lexer::next()
    /// [`input`]: a_bc::lexer::Data::source()
    /// [`lexer`]: Lexer
    pub fn not_expect(
        &mut self,
        kinds: &[Json],
    ) -> Result<Token<Json>, error::Error<Json>> {
        self.next()
            .ok_or(
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
                .into(),
            )
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

impl<'a> Iterator for Lexer<'a> {
    type Item = Result<Token<Json>, error::Error<Json>>;

    /// # Extracts the next [`token`] from the [`source`] data
    ///
    /// The [`token`] [`kind`] is dispatched on the first character:
    ///
    /// - `:` -> [`Colon`]
    /// - `,` -> [`Comma`]
    /// - `{` -> [`Object`]
    /// - `[` -> [`Array`]
    /// - `"` -> [`String`]
    /// - anything else -> [`Sequence`]
    ///
    /// On [`cancellation request`], immediately returns the [`Cancelled`] error
    /// and moves the [`cursor`] to the end of [`range`], so the next requested
    /// [`token`] will be `None.`
    ///
    /// ---
    ///
    /// See the [`lexer`] documentation for more information.
    ///
    /// [`token`]: a_bc::Token
    /// [`source`]: a_bc::lexer::Data::source()
    /// [`kind`]: Json
    /// [`Colon`]: Json::Colon
    /// [`Comma`]: Json::Comma
    /// [`Object`]: Json::Object
    /// [`Array`]: Json::Array
    /// [`String`]: Json::String
    /// [`Sequence`]: Json::Sequence
    /// [`cancellation request`]: a_bc::Cancel::cancel()
    /// [`Cancelled`]: error::Lexer::Cancelled
    /// [`cursor`]: a_bc::lexer::Cursor
    /// [`range`]: a_bc::lexer::Data::range()
    /// [`lexer`]: Lexer
    fn next(&mut self) -> Option<Self::Item> {
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

#[cfg(test)]
mod tests {
    use pretty_assertions::assert_eq;
    use std::{
        thread::{self, sleep},
        time::Duration,
    };

    use crate::{
        Token,
        a_bc::lexer::Builder,
        error::{self, Relation},
        lexer::{Json, Lexer},
    };

    #[test]
    fn cancellation() {
        let n = 10000;
        let source = {
            let mut source = Vec::with_capacity(n * 2);

            for i in 0..n {
                source.extend_from_slice(format!("{}", i % 10).as_bytes());
                source.extend_from_slice(b" ");
            }

            source
        };

        let lexer = Lexer::new(Builder::new(source).build().unwrap());
        let flag = lexer.flag().clone();

        thread::scope(|s| {
            s.spawn(|| {
                sleep(Duration::from_micros(1));
                flag.cancel();
            });

            s.spawn(|| {
                assert!(lexer.count() < n);
            });
        });
    }

    #[test]
    fn parse() {
        let source = br#"
            {
                "first_name": "John",
                "last_name": "Smith",
                "is_alive": true,
                "age": 27,
                "address": {
                    "street_address": "21 2nd Street",
                    "city": "New York",
                    "state": "NY",
                    "postal_code": "10021-3100"
                },
                "phone_numbers": [
                    {
                        "type": "home",
                        "number": "212 555-1234"
                    },
                    {
                        "type": "office",
                        "number": "646 555-4567"
                    }
                ],
                "children": [
                    "Catherine",
                    "Thomas",
                    "Trevor"
                ],
                "spouse": null
            }
        "#;

        let mut lexer = Lexer::new(Builder::new(source).build().unwrap());

        let token = lexer.expect(&[Json::Object]).unwrap();
        assert_eq!(token.kind(), &Json::Object);

        {
            let mut lexer = Lexer::new(
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
                .unwrap();
            assert_eq!(lexer.format(&token).unwrap(), r#""first_name""#);
            assert_eq!(token.kind(), &Json::String);

            let token = lexer.expect(&[Json::Colon]).unwrap();
            assert_eq!(lexer.format(&token).unwrap(), ":");
            assert_eq!(token.kind(), &Json::Colon);

            let token = lexer.not_expect(&[Json::Comma, Json::Colon]).unwrap();
            assert_eq!(lexer.format(&token).unwrap(), r#""John""#);
            assert_eq!(token.kind(), &Json::String);

            let token = lexer.expect(&[Json::Comma]).unwrap();
            assert_eq!(lexer.format(&token).unwrap(), ",");
            assert_eq!(token.kind(), &Json::Comma);

            // ---

            let token = lexer
                .not_expect(&[
                    Json::Object,
                    Json::Array,
                    Json::Comma,
                    Json::Colon,
                ])
                .unwrap();
            assert_eq!(lexer.format(&token).unwrap(), r#""last_name""#);
            assert_eq!(token.kind(), &Json::String);

            let token = lexer.expect(&[Json::Colon]).unwrap();
            assert_eq!(lexer.format(&token).unwrap(), ":");
            assert_eq!(token.kind(), &Json::Colon);

            let token = lexer.not_expect(&[Json::Comma, Json::Colon]).unwrap();
            assert_eq!(lexer.format(&token).unwrap(), r#""Smith""#);
            assert_eq!(token.kind(), &Json::String);

            let token = lexer.expect(&[Json::Comma]).unwrap();
            assert_eq!(lexer.format(&token).unwrap(), ",");
            assert_eq!(token.kind(), &Json::Comma);

            // ---

            let token = lexer
                .not_expect(&[
                    Json::Object,
                    Json::Array,
                    Json::Comma,
                    Json::Colon,
                ])
                .unwrap();
            assert_eq!(lexer.format(&token).unwrap(), r#""is_alive""#);
            assert_eq!(token.kind(), &Json::String);

            let token = lexer.expect(&[Json::Colon]).unwrap();
            assert_eq!(lexer.format(&token).unwrap(), ":");
            assert_eq!(token.kind(), &Json::Colon);

            let token = lexer.not_expect(&[Json::Comma, Json::Colon]).unwrap();
            assert_eq!(lexer.format(&token).unwrap(), "true");
            assert_eq!(token.kind(), &Json::Sequence);

            let token = lexer.expect(&[Json::Comma]).unwrap();
            assert_eq!(lexer.format(&token).unwrap(), ",");
            assert_eq!(token.kind(), &Json::Comma);

            // ---

            let token = lexer
                .not_expect(&[
                    Json::Object,
                    Json::Array,
                    Json::Comma,
                    Json::Colon,
                ])
                .unwrap();
            assert_eq!(lexer.format(&token).unwrap(), r#""age""#);
            assert_eq!(token.kind(), &Json::String);

            let token = lexer.expect(&[Json::Colon]).unwrap();
            assert_eq!(lexer.format(&token).unwrap(), ":");
            assert_eq!(token.kind(), &Json::Colon);

            let token = lexer.not_expect(&[Json::Comma, Json::Colon]).unwrap();
            assert_eq!(lexer.format(&token).unwrap(), "27");
            assert_eq!(token.kind(), &Json::Sequence);

            let token = lexer.expect(&[Json::Comma]).unwrap();
            assert_eq!(lexer.format(&token).unwrap(), ",");
            assert_eq!(token.kind(), &Json::Comma);

            // ---

            let token = lexer
                .not_expect(&[
                    Json::Object,
                    Json::Array,
                    Json::Comma,
                    Json::Colon,
                ])
                .unwrap();
            assert_eq!(lexer.format(&token).unwrap(), r#""address""#);
            assert_eq!(token.kind(), &Json::String);

            let token = lexer.expect(&[Json::Colon]).unwrap();
            assert_eq!(lexer.format(&token).unwrap(), ":");
            assert_eq!(token.kind(), &Json::Colon);

            let token = lexer.expect(&[Json::Object]).unwrap();
            assert_eq!(token.kind(), &Json::Object);
            {
                let mut lexer = Lexer::new(
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
                    .unwrap();
                assert_eq!(
                    lexer.format(&token).unwrap(),
                    r#""street_address""#
                );
                assert_eq!(token.kind(), &Json::String);

                let token = lexer.expect(&[Json::Colon]).unwrap();
                assert_eq!(lexer.format(&token).unwrap(), ":");
                assert_eq!(token.kind(), &Json::Colon);

                let token =
                    lexer.not_expect(&[Json::Comma, Json::Colon]).unwrap();
                assert_eq!(lexer.format(&token).unwrap(), r#""21 2nd Street""#);
                assert_eq!(token.kind(), &Json::String);

                let token = lexer.expect(&[Json::Comma]).unwrap();
                assert_eq!(lexer.format(&token).unwrap(), ",");
                assert_eq!(token.kind(), &Json::Comma);

                // ---

                let token = lexer
                    .not_expect(&[
                        Json::Object,
                        Json::Array,
                        Json::Comma,
                        Json::Colon,
                    ])
                    .unwrap();
                assert_eq!(lexer.format(&token).unwrap(), r#""city""#);
                assert_eq!(token.kind(), &Json::String);

                let token = lexer.expect(&[Json::Colon]).unwrap();
                assert_eq!(lexer.format(&token).unwrap(), ":");
                assert_eq!(token.kind(), &Json::Colon);

                let token =
                    lexer.not_expect(&[Json::Comma, Json::Colon]).unwrap();
                assert_eq!(lexer.format(&token).unwrap(), r#""New York""#);
                assert_eq!(token.kind(), &Json::String);

                let token = lexer.expect(&[Json::Comma]).unwrap();
                assert_eq!(lexer.format(&token).unwrap(), ",");
                assert_eq!(token.kind(), &Json::Comma);

                // ---

                let token = lexer
                    .not_expect(&[
                        Json::Object,
                        Json::Array,
                        Json::Comma,
                        Json::Colon,
                    ])
                    .unwrap();
                assert_eq!(lexer.format(&token).unwrap(), r#""state""#);
                assert_eq!(token.kind(), &Json::String);

                let token = lexer.expect(&[Json::Colon]).unwrap();
                assert_eq!(lexer.format(&token).unwrap(), ":");
                assert_eq!(token.kind(), &Json::Colon);

                let token =
                    lexer.not_expect(&[Json::Comma, Json::Colon]).unwrap();
                assert_eq!(lexer.format(&token).unwrap(), r#""NY""#);
                assert_eq!(token.kind(), &Json::String);

                let token = lexer.expect(&[Json::Comma]).unwrap();
                assert_eq!(lexer.format(&token).unwrap(), ",");
                assert_eq!(token.kind(), &Json::Comma);

                // ---

                let token = lexer
                    .not_expect(&[
                        Json::Object,
                        Json::Array,
                        Json::Comma,
                        Json::Colon,
                    ])
                    .unwrap();
                assert_eq!(lexer.format(&token).unwrap(), r#""postal_code""#);
                assert_eq!(token.kind(), &Json::String);

                let token = lexer.expect(&[Json::Colon]).unwrap();
                assert_eq!(lexer.format(&token).unwrap(), ":");
                assert_eq!(token.kind(), &Json::Colon);

                let token =
                    lexer.not_expect(&[Json::Comma, Json::Colon]).unwrap();
                assert_eq!(lexer.format(&token).unwrap(), r#""10021-3100""#);
                assert_eq!(token.kind(), &Json::String);
            }

            let token = lexer.expect(&[Json::Comma]).unwrap();
            assert_eq!(lexer.format(&token).unwrap(), ",");
            assert_eq!(token.kind(), &Json::Comma);

            // ---

            let token = lexer
                .not_expect(&[
                    Json::Object,
                    Json::Array,
                    Json::Comma,
                    Json::Colon,
                ])
                .unwrap();
            assert_eq!(lexer.format(&token).unwrap(), r#""phone_numbers""#);
            assert_eq!(token.kind(), &Json::String);

            let token = lexer.expect(&[Json::Colon]).unwrap();
            assert_eq!(lexer.format(&token).unwrap(), ":");
            assert_eq!(token.kind(), &Json::Colon);

            let token = lexer.expect(&[Json::Array]).unwrap();
            assert_eq!(token.kind(), &Json::Array);

            {
                let mut lexer = Lexer::new(
                    Builder::new(source)
                        .with_range(token.inner_range().unwrap())
                        .build()
                        .unwrap(),
                );

                let token = lexer.expect(&[Json::Object]).unwrap();
                assert_eq!(token.kind(), &Json::Object);

                {
                    let mut lexer = Lexer::new(
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
                        .unwrap();
                    assert_eq!(lexer.format(&token).unwrap(), r#""type""#);
                    assert_eq!(token.kind(), &Json::String);

                    let token = lexer.expect(&[Json::Colon]).unwrap();
                    assert_eq!(lexer.format(&token).unwrap(), ":");
                    assert_eq!(token.kind(), &Json::Colon);

                    let token =
                        lexer.not_expect(&[Json::Comma, Json::Colon]).unwrap();
                    assert_eq!(lexer.format(&token).unwrap(), r#""home""#);
                    assert_eq!(token.kind(), &Json::String);

                    let token = lexer.expect(&[Json::Comma]).unwrap();
                    assert_eq!(lexer.format(&token).unwrap(), ",");
                    assert_eq!(token.kind(), &Json::Comma);

                    // ---

                    let token = lexer
                        .not_expect(&[
                            Json::Object,
                            Json::Array,
                            Json::Comma,
                            Json::Colon,
                        ])
                        .unwrap();
                    assert_eq!(lexer.format(&token).unwrap(), r#""number""#);
                    assert_eq!(token.kind(), &Json::String);

                    let token = lexer.expect(&[Json::Colon]).unwrap();
                    assert_eq!(lexer.format(&token).unwrap(), ":");
                    assert_eq!(token.kind(), &Json::Colon);

                    let token =
                        lexer.not_expect(&[Json::Comma, Json::Colon]).unwrap();
                    assert_eq!(
                        lexer.format(&token).unwrap(),
                        r#""212 555-1234""#
                    );
                    assert_eq!(token.kind(), &Json::String);
                }

                let token = lexer.expect(&[Json::Comma]).unwrap();
                assert_eq!(lexer.format(&token).unwrap(), ",");
                assert_eq!(token.kind(), &Json::Comma);

                // ---

                let token = lexer.expect(&[Json::Object]).unwrap();
                assert_eq!(token.kind(), &Json::Object);

                {
                    let mut lexer = Lexer::new(
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
                        .unwrap();
                    assert_eq!(lexer.format(&token).unwrap(), r#""type""#);
                    assert_eq!(token.kind(), &Json::String);

                    let token = lexer.expect(&[Json::Colon]).unwrap();
                    assert_eq!(lexer.format(&token).unwrap(), ":");
                    assert_eq!(token.kind(), &Json::Colon);

                    let token =
                        lexer.not_expect(&[Json::Comma, Json::Colon]).unwrap();
                    assert_eq!(lexer.format(&token).unwrap(), r#""office""#);
                    assert_eq!(token.kind(), &Json::String);

                    let token = lexer.expect(&[Json::Comma]).unwrap();
                    assert_eq!(lexer.format(&token).unwrap(), ",");
                    assert_eq!(token.kind(), &Json::Comma);

                    // ---

                    let token = lexer
                        .not_expect(&[
                            Json::Object,
                            Json::Array,
                            Json::Comma,
                            Json::Colon,
                        ])
                        .unwrap();
                    assert_eq!(lexer.format(&token).unwrap(), r#""number""#);
                    assert_eq!(token.kind(), &Json::String);

                    let token = lexer.expect(&[Json::Colon]).unwrap();
                    assert_eq!(lexer.format(&token).unwrap(), ":");
                    assert_eq!(token.kind(), &Json::Colon);

                    let token =
                        lexer.not_expect(&[Json::Comma, Json::Colon]).unwrap();
                    assert_eq!(
                        lexer.format(&token).unwrap(),
                        r#""646 555-4567""#
                    );
                    assert_eq!(token.kind(), &Json::String);
                }
            }

            let token = lexer.expect(&[Json::Comma]).unwrap();
            assert_eq!(lexer.format(&token).unwrap(), ",");
            assert_eq!(token.kind(), &Json::Comma);

            // ---

            let token = lexer
                .not_expect(&[
                    Json::Object,
                    Json::Array,
                    Json::Comma,
                    Json::Colon,
                ])
                .unwrap();
            assert_eq!(lexer.format(&token).unwrap(), r#""children""#);
            assert_eq!(token.kind(), &Json::String);

            let token = lexer.expect(&[Json::Colon]).unwrap();
            assert_eq!(lexer.format(&token).unwrap(), ":");
            assert_eq!(token.kind(), &Json::Colon);

            let token = lexer.expect(&[Json::Array]).unwrap();
            assert_eq!(token.kind(), &Json::Array);

            {
                let mut lexer = Lexer::new(
                    Builder::new(source)
                        .with_range(token.inner_range())
                        .build()
                        .unwrap(),
                );

                let token =
                    lexer.not_expect(&[Json::Colon, Json::Comma]).unwrap();
                assert_eq!(lexer.format(&token).unwrap(), r#""Catherine""#);
                assert_eq!(token.kind(), &Json::String);

                let token = lexer.expect(&[Json::Comma]).unwrap();
                assert_eq!(lexer.format(&token).unwrap(), ",");
                assert_eq!(token.kind(), &Json::Comma);

                // ---

                let token =
                    lexer.not_expect(&[Json::Colon, Json::Comma]).unwrap();
                assert_eq!(lexer.format(&token).unwrap(), r#""Thomas""#);
                assert_eq!(token.kind(), &Json::String);

                let token = lexer.expect(&[Json::Comma]).unwrap();
                assert_eq!(lexer.format(&token).unwrap(), ",");
                assert_eq!(token.kind(), &Json::Comma);

                // ---

                let token =
                    lexer.not_expect(&[Json::Colon, Json::Comma]).unwrap();
                assert_eq!(lexer.format(&token).unwrap(), r#""Trevor""#);
                assert_eq!(token.kind(), &Json::String);
            }

            let token = lexer.expect(&[Json::Comma]).unwrap();
            assert_eq!(lexer.format(&token).unwrap(), ",");
            assert_eq!(token.kind(), &Json::Comma);

            // ---

            let token = lexer
                .not_expect(&[
                    Json::Object,
                    Json::Array,
                    Json::Comma,
                    Json::Colon,
                ])
                .unwrap();
            assert_eq!(lexer.format(&token).unwrap(), r#""spouse""#);
            assert_eq!(token.kind(), &Json::String);

            let token = lexer.expect(&[Json::Colon]).unwrap();
            assert_eq!(lexer.format(&token).unwrap(), ":");
            assert_eq!(token.kind(), &Json::Colon);

            let token = lexer.not_expect(&[Json::Comma, Json::Colon]).unwrap();
            assert_eq!(lexer.format(&token).unwrap(), "null");
            assert_eq!(token.kind(), &Json::Sequence);
        }
    }

    #[test]
    fn empty_source() {
        let source = b"";

        assert_eq!(
            Err(error::Lexer::SourceIsEmpty),
            Builder::new(source).build()
        );
    }

    #[test]
    fn find_block() {
        let source = b"{123}";

        let mut lexer = Lexer::new(Builder::new(source).build().unwrap());
        let block_range = lexer.find_block(b'{', b'}').unwrap();
        assert_eq!(block_range, 0..5);
    }

    #[test]
    fn find_complicated_block() {
        let source = br#"{ "} {" }"#;

        let mut lexer = Lexer::new(Builder::new(source).build().unwrap());
        let block_range = lexer.find_block(b'{', b'}').unwrap();
        assert_eq!(block_range, 0..9);
    }

    #[test]
    fn colon() {
        let source = b":";

        let mut lexer = Lexer::new(Builder::new(source).build().unwrap());
        let token = lexer.next();
        assert_eq!(token, Some(Ok(Token::new(Json::Colon, 0..1))));

        let token = lexer.next();
        assert_eq!(token, None);
        assert_eq!(*lexer.cursor().position(), 1);
    }

    #[test]
    fn comma() {
        let source = b",";

        let mut lexer = Lexer::new(Builder::new(source).build().unwrap());
        let token = lexer.next();
        assert_eq!(token, Some(Ok(Token::new(Json::Comma, 0..1))));

        let token = lexer.next();
        assert_eq!(token, None);
        assert_eq!(*lexer.cursor().position(), 1);
    }

    #[test]
    fn empty_object() {
        let source = b"{}";

        let mut lexer = Lexer::new(Builder::new(source).build().unwrap());
        let token = lexer.next();
        assert_eq!(token, Some(Ok(Token::new(Json::Object, 0..2))));

        let token = lexer.next();
        assert_eq!(token, None);
        assert_eq!(*lexer.cursor().position(), 2);
    }

    #[test]
    fn empty_array() {
        let source = b"[]";

        let mut lexer = Lexer::new(Builder::new(source).build().unwrap());
        let token = lexer.next();
        assert_eq!(token, Some(Ok(Token::new(Json::Array, 0..2))));

        let token = lexer.next();
        assert_eq!(token, None);
        assert_eq!(*lexer.cursor().position(), 2);
    }

    #[test]
    fn empty_string() {
        let source = br#""""#;

        let mut lexer = Lexer::new(Builder::new(source).build().unwrap());
        let token = lexer.next();
        assert_eq!(token, Some(Ok(Token::new(Json::String, 0..2))));

        let token = lexer.next();
        assert_eq!(token, None);
        assert_eq!(*lexer.cursor().position(), 2);
    }

    #[test]
    fn filled_object() {
        let source = br#"{"name":"value"}"#;

        let mut lexer = Lexer::new(Builder::new(source).build().unwrap());
        let token = lexer.next();
        assert_eq!(token, Some(Ok(Token::new(Json::Object, 0..16))));

        let token = lexer.next();
        assert_eq!(token, None);
        assert_eq!(*lexer.cursor().position(), 16);
    }

    #[test]
    fn incomplete_object() {
        let source = br#"{"name":"value""#;

        let mut lexer = Lexer::new(Builder::new(source).build().unwrap());
        let token = lexer.next();
        assert_eq!(
            token,
            Some(Err(error::Token::PairNotFound {
                opening: "{".into(),
                closing: "}".into(),
                location: (Relation::After, (1, 1).into())
            }
            .into()))
        );
    }

    #[test]
    fn filled_array() {
        let source = b"[1,2,3,4,5,6,7,8,9,0]";

        let mut lexer = Lexer::new(Builder::new(source).build().unwrap());
        let token = lexer.next();
        assert_eq!(token, Some(Ok(Token::new(Json::Array, 0..21))));

        let token = lexer.next();
        assert_eq!(token, None);
        assert_eq!(*lexer.cursor().position(), 21);
    }

    #[test]
    fn incomplete_array() {
        let source = b"[1,2,3,4,5,6,7,8,9,0";

        let mut lexer = Lexer::new(Builder::new(source).build().unwrap());
        let token = lexer.next();
        assert_eq!(
            token,
            Some(Err(error::Token::PairNotFound {
                opening: "[".into(),
                closing: "]".into(),
                location: (Relation::After, (1, 1).into())
            }
            .into()))
        );
    }

    #[test]
    fn filled_string() {
        let source = br#""John""#;

        let mut lexer = Lexer::new(Builder::new(source).build().unwrap());
        let token = lexer.next();
        assert_eq!(token, Some(Ok(Token::new(Json::String, 0..6))));

        let token = lexer.next();
        assert_eq!(token, None);
        assert_eq!(*lexer.cursor().position(), 6);
    }

    #[test]
    fn escaped_string() {
        let source = br#""a\"b""#;

        let mut lexer = Lexer::new(Builder::new(source).build().unwrap());
        let token = lexer.next();
        assert_eq!(token, Some(Ok(Token::new(Json::String, 0..6))));

        let token = lexer.next();
        assert_eq!(token, None);
        assert_eq!(*lexer.cursor().position(), 6);
    }

    #[test]
    fn incomplete_string() {
        let source = br#""John"#;

        let mut lexer = Lexer::new(Builder::new(source).build().unwrap());
        let token = lexer.next();
        assert_eq!(
            token,
            Some(Err(error::Token::PairNotFound {
                opening: "\"".into(),
                closing: "\"".into(),
                location: (Relation::After, (1, 1).into())
            }
            .into()))
        );
    }

    #[test]
    fn sequence() {
        let source = b"asd123qwe456";

        let mut lexer = Lexer::new(Builder::new(source).build().unwrap());
        let token = lexer.next();
        assert_eq!(token, Some(Ok(Token::new(Json::Sequence, 0..12))));

        let token = lexer.next();
        assert_eq!(token, None);
    }
}
