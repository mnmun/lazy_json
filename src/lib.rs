//! Provides a [`lexer`] that extracts [`tokens`] at the current level of the
//! hierarchy and a [`parser`] that represents source data as a [`tree`] of
//! lazily-created [`nodes`].
//!
//! ## [`Lexer`]
//!
//! ![](https://github.com/mnmun/images/blob/main/box.png?raw=true)
//!
//! Implemented using the [`a_bc`] crate.
//!
//! Splits the JSON source data into tokens of the following [`kinds`]:
//!
//! - [`Comma`] - comma `,` character;
//! - [`Colon`] - colon `:` character;
//! - [`Object`] - curly braces `{` `}` and all characters between them;
//! - [`Array`] - square brackets `[` `]` and all characters between them;
//! - [`String`] - double quotes `"` `"` and all characters between them;
//! - [`Sequence`] - a contiguous run of characters: numbers, `true`, `false`,
//!   `null`, etc.; [whitespace characters] as well as the `:` and `,`
//!   characters act as separators.
//!
//! [`Lexer`] can be created using a [`builder`](crate::lexer::Builder), and
//! [`tokens`] can be extracted using the [`next()`] method:
//!
//! ```rust
//! # use pretty_assertions::assert_eq;
//! #
//! use lazy_json::{JSON, Lexer, lexer::Builder, a_bc::Token};
//!
//! let source: &'static [u8] = br#" "name": "value" "#;
//!
//! let mut lexer: Lexer = Builder::new(source).build().unwrap();
//!
//! let token: Token<JSON> = lexer.next().unwrap().unwrap();
//! assert_eq!(token.kind(), &JSON::String);
//! assert_eq!(lexer.format(&token).unwrap(), r#""name""#);
//!
//! let token: Token<JSON> = lexer.next().unwrap().unwrap();
//! assert_eq!(token.kind(), &JSON::Colon);
//! assert_eq!(lexer.format(&token).unwrap(), ":");
//!
//! let token: Token<JSON> = lexer.next().unwrap().unwrap();
//! assert_eq!(token.kind(), &JSON::String);
//! assert_eq!(lexer.format(&token).unwrap(), r#""value""#);
//! ```
//!
//! ## [`Parser`]
//!
//! ![](https://github.com/mnmun/images/blob/main/cart.png?raw=true)
//!
//! Implemented using the [`lexer`] and the [`lazy_tree`] crate.
//!
//! Represents a JSON string data as a [`tree`] of lazily-created [`nodes`]:
//!
//! ```text
//!    *Source data*                *Parsed tree*
//!
//! {                               *Source data*
//!   "object": {                ┌────────┴────────┐
//!     "string": "foo",       object            array
//!     "number": 42         ┌───┴───┐       ┌─────┼─────┐
//!   },                   string  number   true false  null
//!   "array": [             │       │
//!     true,               foo      42
//!     false,
//!     null
//!   ]
//! }
//! ```
//!
//! [`Nodes`] are created automatically when the [`cursor`] needs to access them
//! and are deleted when they are no longer used.
//!
//! [`Parser`] can be created using a [`builder`](crate::parser::Builder), and
//! the [`tree`] of lazily-created [`nodes`] can be traversed using a
//! [`cursor`]:
//!
//! ```rust
//! # use pretty_assertions::assert_eq;
//! #
//! use lazy_json::{
//!     JSON, Parser,
//!     parser::{Builder, Bundle},
//!     a_bc::Error,
//!     lazy_tree::{Cursor, cursor::{Direction, Target}},
//! };
//!
//! # #[tokio::main]
//! # async fn main() {
//! //
//! //              source
//! //       ┌────────┴────────┐
//! //     object            array
//! //   ┌───┴───┐       ┌─────┼─────┐
//! // string  number   true false  null
//! //   │       │
//! //  foo      42
//! //
//! let source: &'static str = r#"
//! {
//!   "object": {
//!     "string": "foo",
//!     "number": 42
//!   },
//!   "array": [
//!     true,
//!     false,
//!     null
//!   ]
//! }
//! "#;
//!
//! let parser: Parser = Builder::new(source).build().unwrap();
//!
//! let mut cursor: Cursor<Bundle, Error<JSON>> =
//!     parser.cursor().await.unwrap();
//! assert_eq!(cursor.kind(), &JSON::Object);
//! assert_eq!(cursor.content(), source);
//!
//! cursor.walk(Direction::Down(Target::First)).await.unwrap();
//! assert_eq!(cursor.kind(), &JSON::Object);
//! assert_eq!(cursor.content(), "object");
//!
//! cursor.jump(Direction::Left).await.unwrap();
//! assert_eq!(cursor.kind(), &JSON::Array);
//! assert_eq!(cursor.content(), "array");
//!
//! cursor.go_to(&[0, 0, 1, 0]).await.unwrap();
//! assert_eq!(cursor.kind(), &JSON::Sequence);
//! assert_eq!(cursor.content(), "42");
//! # }
//! ```
//!
//! ## License
//!
//! [MIT](https://github.com/mnmun/lazy_json/blob/main/LICENSE)
//!
//! [`lexer`]: crate::Lexer
//! [`parser`]: crate::Parser
//! [`kinds`]: crate::JSON
//! [`Comma`]: crate::JSON::Comma
//! [`Colon`]: crate::JSON::Colon
//! [`Object`]: crate::JSON::Object
//! [`Array`]: crate::JSON::Array
//! [`String`]: crate::JSON::String
//! [`Sequence`]: crate::JSON::Sequence
//! [whitespace characters]: u8::is_ascii_whitespace()
//! [`tokens`]: crate::a_bc::Token
//! [`next()`]: crate::Lexer::next()
//! [`tree`]: crate::lazy_tree::Tree
//! [`node`]: crate::lazy_tree::Node
//! [`Nodes`]: crate::lazy_tree::Node
//! [`cursor`]: crate::lazy_tree::Cursor

#![deny(rustdoc::broken_intra_doc_links)]
#![deny(rustdoc::private_intra_doc_links)]
#![deny(rustdoc::missing_crate_level_docs)]
#![deny(rustdoc::invalid_codeblock_attributes)]
#![deny(rustdoc::invalid_html_tags)]
#![deny(rustdoc::invalid_rust_codeblocks)]
#![deny(rustdoc::unescaped_backticks)]
#![deny(rustdoc::redundant_explicit_links)]

pub mod lexer;
pub mod parser;

pub use lexer::{JSON, Lexer};
pub use parser::{Bundle, Parser};

pub use a_bc;
pub use lazy_tree;

#[cfg(test)]
mod tests;
