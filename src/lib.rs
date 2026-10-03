//! # Lazy JSON
//!
//! ![](https://github.com/mnmun/images/blob/main/lounge.png?raw=true)
//!
//! Provides the following lazy JSON utilities:
//!
//! - [`Lexer`]: Based on the [`a_bc`] crate;
//! - [`Parser`]: Implemented using the [`lexer`] and the lazily [`populated`]
//!   [`tree`] from the [`lazy_tree`] crate.
//!
//! ## [`Lexer`]
//!
//! ![](https://github.com/mnmun/images/blob/main/paper.png?raw=true)
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
//! Unlike lexers that examine the characters of the source data one by one
//! this [`lexer`] skips a portion of the characters and leaves them unexamined.
//! Put differently, within the given hierarchical structure of JSON, the
//! [`lexer`] [`extracts`] only the top-level [`tokens`].
//!
//! ### Use cases
//!
//! The [`lexer`] was originally designed to back the lazy JSON [`parser`], but
//! it can equally be used on its own. It [`extracts`] only top-level
//! [`tokens`], skipping the characters of deeper [`tokens`] that are not
//! currently being [`extracted`]. This approach may be beneficial for large
//! JSON data with deep nesting.
//!
//! ### Example
//!
//! The following example shows how [`tokens`] are [`extracted`] from the
//! [`source`] data:
//!
//! ```rust
//! use pretty_assertions::assert_eq;
//!
//! use lazy_json::{
//!     Json,
//!     Lexer,
//!     a_bc::{
//!         lexer::Builder,
//!         token::Token,
//!     },
//! };
//!
//! let source: &[u8] = br#" "name": "John Smith" "#;
//!
//! let mut lexer = Lexer::new(Builder::new(source).build().unwrap());
//!
//! let token = lexer.next().unwrap().unwrap();
//! assert_eq!(*token.kind(), Json::String);
//! assert_eq!(lexer.format(&token).unwrap(), r#""name""#);
//!
//! let token = lexer.next().unwrap().unwrap();
//! assert_eq!(*token.kind(), Json::Colon);
//! assert_eq!(lexer.format(&token).unwrap(), ":");
//!
//! let token = lexer.next().unwrap().unwrap();
//! assert_eq!(*token.kind(), Json::String);
//! assert_eq!(lexer.format(&token).unwrap(), r#""John Smith""#);
//!
//! let token = lexer.next();
//! assert_eq!(token, None);
//! ```
//!
//! ## [`Parser`]
//!
//! ![](https://github.com/mnmun/images/blob/main/scissors.png?raw=true)
//!
//! The [`parser`] represents the [`source`] data as a hierarchical structure of
//! [`nodes`] and provides a [`cursors`] to traverse it. The [`parser`] does not
//! build the whole [`tree`] at once: the [`nodes`] are [`populated`] lazily
//! when visited by [`cursors`] and remain available only while they are needed.
//!
//! ```text
//!    *Source data*               *Parsed tree*
//!
//! {                              *Source data*
//!   "object": {                ┌───────┴───────┐
//!     "string": "foo",       object          array
//!     "number": 42         ┌───┴───┐     ┌─────┼─────┐
//!   },                   string  number true false  null
//!   "array": [             │       │
//!     true,               foo      42
//!     false,
//!     null
//!   ]
//! }
//! ```
//!
//! ### Use cases
//!
//! The [`parser`] may be beneficial for large JSON data with deep nesting, as
//! it does not build the whole [`tree`] up front but [`creates`] [`nodes`]
//! lazily, only when they are actually needed.
//!
//! ### Example
//!
//! The following example demonstrates JSON parsing and the resulting [`tree`]
//! traversal using [`cursors`]:
//!
//! ```rust
//! use pretty_assertions::assert_eq;
//!
//! use lazy_json::{
//!     lazy_tree::cursor::{Direction, Target},
//!     Json,
//!     Parser,
//! };
//!
//! //
//! //             source
//! //       ┌───────┴───────┐
//! //     object          array
//! //   ┌───┴───┐     ┌─────┼─────┐
//! // string  number true false  null
//! //   │       │
//! //  foo      42
//! //
//! let source = r#"
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
//! let parser = Parser::new(source).unwrap();
//!
//! // After creation, the cursor points to the tree root node:
//! let mut cursor = parser.cursor().unwrap();
//! assert_eq!(*cursor.value().kind(), Json::Object);
//! assert_eq!(*cursor.value().content(), source);
//! assert_eq!(
//!     cursor.children().iter().map(|child|
//!         *child.value().content()
//!     ).collect::<Vec<&str>>(),
//!     ["object", "array"]
//! );
//!
//! // The cursor descends into the first child node of the root - `object`:
//! assert!(cursor.walk(Direction::Down(Target::First)).unwrap());
//! assert_eq!(*cursor.value().kind(), Json::Object);
//! assert_eq!(*cursor.value().content(), "object");
//! assert_eq!(
//!     cursor.children().iter().map(|child|
//!         *child.value().content()
//!     ).collect::<Vec<&str>>(),
//!     ["string", "number"]
//! );
//!
//! // The cursor moves to the next sibling node - `array`:
//! assert!(cursor.walk(Direction::Right).unwrap());
//! assert_eq!(*cursor.value().kind(), Json::Array);
//! assert_eq!(*cursor.value().content(), "array");
//! assert_eq!(
//!     cursor.children().iter().map(|child|
//!         *child.value().content()
//!     ).collect::<Vec<&str>>(),
//!     ["true", "false", "null"]
//! );
//! ```
//!
//! ## License
//!
//! [MIT](https://github.com/mnmun/lazy_json/blob/main/LICENSE)
//!
//! [`lexer`]: Lexer
//! [`parser`]: Parser
//! [`extracts`]: Lexer::next()
//! [`extracted`]: Lexer::next()
//! [`tokens`]: Token
//! [`Tokens`]: Token
//! [`kinds`]: Json
//! [`source`]: a_bc::lexer::Data::source()
//! [`Comma`]: Json::Comma
//! [`Colon`]: Json::Colon
//! [`Object`]: Json::Object
//! [`Array`]: Json::Array
//! [`String`]: Json::String
//! [`Sequence`]: Json::Sequence
//! [`whitespace`]: u8::is_ascii_whitespace()
//! [`whitespace characters`]: u8::is_ascii_whitespace()
//! [`cursors`]: Cursor
//! [`tree`]: Tree
//! [`nodes`]: lazy_tree::Node
//! [`populated`]: lazy_tree::node::Callback
//! [`creates`]: lazy_tree::node::Callback

#![allow(dead_code)]
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

pub use a_bc::{self, Cancel, Token, error};
pub use lazy_tree::{self, Cursor, Tree};
pub use lexer::{Json, Lexer};
pub use parser::{Bundle, Parser};
