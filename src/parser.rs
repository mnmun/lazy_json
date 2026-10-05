//! # Lazy JSON parser
//!
//! ![](https://github.com/mnmun/images/blob/main/scissors.png?raw=true)
//!
//! Provides a lazy JSON [`parser`] implemented using the lazy JSON [`lexer`]
//! and the lazily [`populated`] [`tree`] from the [`lazy_tree`] crate.
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
//! ## Use cases
//!
//! The [`parser`] may be beneficial for large JSON data with deep nesting, as
//! it does not build the whole [`tree`] up front but [`creates`] [`nodes`]
//! lazily, only when they are actually needed.
//!
//! ## Behavior
//!
//! The [`parser`] is implemented on top of the [`tree`] from the [`lazy_tree`]
//! crate - a data structure that maintains a hierarchy of lazily [`populated`]
//! [`nodes`] holding arbitrary [`value`].
//!
//! Each [`tree`] [`node`] stores a [`value`] of type [`Bundle`], which contains
//! [`kind`] and [`content`].
//!
//! The tool for examining the [`source`] data is a [`cursor`], which traverses
//! the [`tree`] and provides access to the [`value`] stored in the [`nodes`].
//! To move between [`nodes`], the [`cursor`] provides the [`walk()`],
//! [`jump()`] and [`go_to()`] methods.
//!
//! The [`parser's`] [`tree`] can have an unlimited number of [`cursors`]
//! existing at the same time. [`Cursors`] can freely move between threads,
//! enabling multithreaded analysis of the [`source`] data, but keep in mind
//! that a [`cursor`] cannot outlive the [`tree`] from which it was created.
//!
//! ## Examples
//!
//! ![](https://github.com/mnmun/images/blob/main/bulb.png?raw=true)
//!
//! The following examples demonstrate JSON parsing and the resulting [`tree`]
//! traversal using [`cursors`].
//!
//! ### Valid source
//!
//! Correctly formed [`source`] data can be traversed as follows:
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
//! ### Malformed value
//!
//! Traversing [`source`] data with a malformed object value:
//!
//! ```rust
//! use pretty_assertions::assert_eq;
//!
//! use lazy_json::{
//!     a_bc::error::{self, Relation, Position},
//!     lazy_tree::cursor::{Direction, Target},
//!     Json,
//!     Parser,
//! };
//!
//! //
//! //             source
//! //       ┌───────┴───────┐
//! //     object          array
//! //       │         ┌─────┼─────┐
//! //      -X-       true false  null
//! //
//! let source = r#"
//! {
//!   "object": {
//!       malformed object with unexpected token
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
//! // An attempt to descend into the malformed object node ends with an error:
//! assert!(cursor.walk(Direction::Down(Target::First)).is_err_and(|e|
//!     e == error::Token::ExpectedButGot {
//!         expected: [Json::String].into(),
//!         got: Some(Json::Sequence),
//!         location: (Relation::At, Position::new(4, 7))
//!     }.into()
//! ));
//!
//! // The cursor descends into the last child node of the root - `array`:
//! assert!(cursor.walk(Direction::Down(Target::Last)).unwrap());
//! assert_eq!(*cursor.value().kind(), Json::Array);
//! assert_eq!(*cursor.value().content(), "array");
//! assert_eq!(
//!     cursor.children().iter().map(|child|
//!         *child.value().content()
//!     ).collect::<Vec<&str>>(),
//!     ["true", "false", "null"]
//! );
//!
//! // The cursor descends into the leaf child node of the `array` by
//! // index - `false`:
//! assert!(cursor.walk(Direction::Down(Target::Index(1))).unwrap());
//! assert_eq!(*cursor.value().kind(), Json::Sequence);
//! assert_eq!(*cursor.value().content(), "false");
//! assert!(cursor.children().is_empty());
//! ```
//!
//! ### Empty [`source`] data
//!
//! A [`parser`] cannot be created over empty [`source`] data:
//!
//! ```rust
//! use pretty_assertions::assert_eq;
//!
//! use lazy_json::{
//!     a_bc::error::{self, Relation, Position},
//!     lazy_tree::cursor::{Direction, Target},
//!     Json,
//!     Parser,
//! };
//!
//! let empty_source = "";
//!
//! // An attempt to create a parser over empty source data ends with an error:
//! assert!(Parser::new(empty_source).is_err_and(|e|
//!     e == error::Lexer::SourceIsEmpty.into()
//! ));
//! ```
//!
//! ---
//!
//! See the [`crate`] documentation for more information.
//!
//! [`lexer`]: crate::Lexer
//! [`lazy_tree`]: crate::lazy_tree
//! [`tree`]: crate::lazy_tree::Tree
//! [`parser`]: crate::Parser
//! [`parser's`]: crate::Parser
//! [`source`]: crate::a_bc::lexer::Data::source()
//! [`value`]: crate::lazy_tree::Node::value()
//! [`node`]: crate::lazy_tree::Node
//! [`nodes`]: crate::lazy_tree::Node
//! [`cursor`]: crate::lazy_tree::Cursor
//! [`cursors`]: crate::lazy_tree::Cursor
//! [`Cursors`]: crate::lazy_tree::Cursor
//! [`populated`]: crate::lazy_tree::node::Callback
//! [`creates`]: crate::lazy_tree::node::Callback
//! [`kind`]: crate::parser::Bundle::kind()
//! [`content`]: crate::parser::Bundle::content()
//! [`walk()`]: crate::lazy_tree::Cursor::walk()
//! [`jump()`]: crate::lazy_tree::Cursor::jump()
//! [`go_to()`]: crate::lazy_tree::Cursor::go_to()

use std::{borrow::Cow, ops::Range};

use a_bc::error::row_col_pos;
use getset::Getters;

use crate::{
    a_bc::{
        self, Cancel,
        error::{self, Relation},
    },
    lazy_tree::Node,
    lexer::{Json, Lexer},
};

type Error = error::Error<Json>;
type Link<'source> = lazy_tree::node::Link<Bundle<'source>, Error>;
type Children<'source> = Box<[Link<'source>]>;

/// # [`Node`] [`value`]
///
/// ![](https://github.com/mnmun/images/blob/main/book.png?raw=true)
///
/// The data stored in each [`node`] of the [`tree`].
///
/// Use the following methods to access the data:
///
/// - [`kind()`];
/// - [`content()`].
///
/// ---
///
/// See the [`module`] documentation for more information.
///
/// [`value`]: crate::lazy_tree::Node::value()
/// [`node`]: crate::lazy_tree::Node
/// [`tree`]: crate::lazy_tree::Tree
/// [`kind()`]: crate::parser::Bundle::kind()
/// [`content()`]: crate::parser::Bundle::content()
/// [`module`]: crate::parser
#[derive(Debug, Clone, Getters, PartialEq, Eq)]
pub struct Bundle<'source> {
    #[getset(get = "pub")]
    kind: Json,
    #[getset(get = "pub")]
    content: &'source str,

    flag: Cancel,
    source: &'source str,
    range: Option<Range<usize>>,
}

impl<'source> Bundle<'source> {
    fn new(
        kind: Json,
        content: &'source str,
        flag: Cancel,
        source: &'source str,
        range: impl Into<Option<Range<usize>>>,
    ) -> Self {
        let range = range.into();

        Self {
            kind,
            content,
            flag,
            source,
            range,
        }
    }
}

fn populate<'source>(
    parent: &Bundle<'source>,
) -> Result<Children<'source>, Error> {
    let range = if let Some(range) = &parent.range {
        range
    } else {
        return Ok(Box::default());
    };

    let source = parent.source;
    let flag = parent.flag.clone();

    let mut lexer = Lexer::new(
        a_bc::lexer::Builder::new(Cow::Borrowed(source.as_bytes()))
            .with_range(range.clone())
            .build()?,
    );

    let token = lexer.expect(&[
        Json::Object,
        Json::Array,
        Json::String,
        Json::Sequence,
    ])?;

    let mut children = vec![];

    match *token.kind() {
        Json::Object => {
            let mut lexer = Lexer::new(
                a_bc::lexer::Builder::new(Cow::Borrowed(source.as_bytes()))
                    .with_flag(flag.clone())
                    .with_range(token.inner_range().unwrap())
                    .build()?,
            );

            let mut object_is_empty = true;

            loop {
                let key_token = match lexer.expect(&[Json::String]) {
                    Ok(key_token) => key_token,
                    Err(e) => {
                        if let Error::Token(error::Token::ExpectedButGot {
                            got,
                            ..
                        }) = e
                            && got.is_none()
                            && object_is_empty
                        {
                            break;
                        } else {
                            return Err(e);
                        }
                    }
                };

                object_is_empty = false;

                lexer.expect(&[Json::Colon])?;

                let value_token = lexer.expect(&[
                    Json::String,
                    Json::Sequence,
                    Json::Object,
                    Json::Array,
                ])?;

                children.push({
                    let bundle = Bundle::new(
                        *value_token.kind(),
                        &source[key_token.inner_range().unwrap()],
                        flag.clone(),
                        source,
                        value_token.range().clone(),
                    );

                    Node::new(bundle, populate)
                });

                match lexer.expect(&[Json::Comma]) {
                    Ok(_) => {}
                    Err(e) => {
                        if let Error::Token(error::Token::ExpectedButGot {
                            got,
                            ..
                        }) = e
                            && got.is_none()
                        {
                            break;
                        } else {
                            return Err(e);
                        }
                    }
                }
            }
        }
        Json::Array => {
            let mut lexer = Lexer::new(
                a_bc::lexer::Builder::new(source.as_bytes())
                    .with_flag(flag.clone())
                    .with_range(token.inner_range().unwrap())
                    .build()?,
            );

            let mut array_is_empty = true;

            loop {
                let value_token =
                    match lexer.not_expect(&[Json::Comma, Json::Colon]) {
                        Ok(token) => token,
                        Err(e) => {
                            if let Error::Token(
                                error::Token::NotExpectedButGot { got, .. },
                            ) = e
                                && got.is_none()
                                && array_is_empty
                            {
                                break;
                            } else {
                                return Err(e);
                            }
                        }
                    };

                array_is_empty = false;

                children.push({
                    let (value, range) = match *value_token.kind() {
                        Json::Object => {
                            ("{...}", Some(value_token.range().clone()))
                        }
                        Json::Array => {
                            ("[...]", Some(value_token.range().clone()))
                        }
                        Json::String => {
                            (&source[value_token.inner_range().unwrap()], None)
                        }
                        Json::Sequence => {
                            (&source[value_token.range().clone()], None)
                        }
                        _ => unreachable!(),
                    };

                    let bundle = Bundle::new(
                        *value_token.kind(),
                        value,
                        flag.clone(),
                        source,
                        range,
                    );

                    Node::new(bundle, populate)
                });

                match lexer.expect(&[Json::Comma]) {
                    Ok(_) => {}
                    Err(e) => {
                        if let Error::Token(error::Token::ExpectedButGot {
                            got,
                            ..
                        }) = e
                            && got.is_none()
                        {
                            break;
                        } else {
                            return Err(e);
                        }
                    }
                }
            }
        }
        Json::String => children.push({
            let bundle = Bundle::new(
                Json::String,
                &source[token.inner_range().unwrap()],
                flag.clone(),
                source,
                None,
            );

            Node::new(bundle, populate)
        }),
        Json::Sequence => children.push({
            let bundle = Bundle::new(
                Json::Sequence,
                &source[token.range().clone()],
                flag.clone(),
                source,
                None,
            );

            Node::new(bundle, populate)
        }),
        _ => unreachable!(),
    }

    Ok(children.into_boxed_slice())
}

type Tree<'source> = lazy_tree::Tree<Bundle<'source>, Error>;
pub type Cursor<'source, 'tree> =
    lazy_tree::Cursor<'tree, Bundle<'source>, Error>;

/// # Lazy JSON `parser`
///
/// ![](https://github.com/mnmun/images/blob/main/scissors.png?raw=true)
///
/// Converts the [`source`] data into a lazily populated [`tree`] of [`nodes`].
///
/// ## Traversal
///
/// The `parser's` [`tree`] is traversed using [`cursors`], which are created
/// with the [`cursor()`] method. After creation, a [`cursor`] points to the
/// `root` [`node`] of the [`tree`].
///
/// The [`tree`] can have an unlimited number of simultaneously existing
/// [`cursors`], which can freely move between threads.
///
/// Keep in mind that a [`cursor`] cannot outlive the `parser's` [`tree`].
///
/// ## Creation
///
/// Use the [`Parser::new()`] method to create a `parser`.
///
/// ---
///
/// See the [`module`] documentation for more information.
///
/// [`source`]: crate::a_bc::lexer::Data::source()
/// [`tree`]: crate::lazy_tree::Tree
/// [`node`]: crate::lazy_tree::Node
/// [`nodes`]: crate::lazy_tree::Node
/// [`cursor`]: crate::lazy_tree::Cursor
/// [`cursors`]: crate::lazy_tree::Cursor
/// [`cursor()`]: crate::Parser::cursor()
/// [`module`]: crate::parser
pub struct Parser<'source> {
    tree: Tree<'source>,
    flag: Cancel,
}

impl<'source, 'tree> Parser<'source> {
    /// # Creates a [`parser`] over the [`source`] data
    ///
    /// Constructs the [`tree`] `root` [`node`] from the provided [`source`].
    ///
    /// The [`parser's`] [`tree`] is not [`populated`] at this point: the
    /// children of the `root` [`node`] are created lazily, on the first visit
    /// by a [`cursor`].
    ///
    /// ## Errors
    ///
    /// - [`SourceIsEmpty`] - the provided [`source`] is empty;
    /// - [`ExpectedButGot`] - the provided [`source`] contains no JSON values.
    ///
    /// ---
    ///
    /// See the [`parser`] documentation for more information.
    ///
    /// [`parser`]: crate::Parser
    /// [`parser's`]: crate::Parser
    /// [`source`]: crate::a_bc::lexer::Data::source()
    /// [`tree`]: crate::lazy_tree::Tree
    /// [`node`]: crate::lazy_tree::Node
    /// [`populated`]: crate::lazy_tree::node::Callback
    /// [`cursor`]: crate::lazy_tree::Cursor
    /// [`SourceIsEmpty`]: crate::a_bc::error::Lexer::SourceIsEmpty
    /// [`ExpectedButGot`]: crate::a_bc::error::Token::ExpectedButGot
    /// [`parser`]: crate::Parser
    pub fn new(source: &'source str) -> Result<Self, Error> {
        if source.is_empty() {
            return Err(error::Lexer::SourceIsEmpty.into());
        }

        let kind = match source
            .as_bytes()
            .iter()
            .find(|c| !c.is_ascii_whitespace())
        {
            Some(b'{') => Json::Object,
            Some(b'[') => Json::Array,
            Some(b'"') => Json::String,
            Some(_) => Json::Sequence,
            None => {
                return Err(error::Token::ExpectedButGot {
                    expected: [
                        Json::Object,
                        Json::Array,
                        Json::String,
                        Json::Sequence,
                    ]
                    .into(),
                    got: None,
                    location: (Relation::After, row_col_pos(source.as_bytes())),
                }
                .into());
            }
        };

        let flag = Cancel::new();

        let bundle =
            Bundle::new(kind, source, flag.clone(), source, 0..source.len());

        let tree = lazy_tree::Tree::new(Node::new(bundle, populate));

        Ok(Self { tree, flag })
    }

    /// # Creates a [`cursor`] at the `root` [`node`] of the [`parser's`] [`tree`]
    ///
    /// Returns the population [`callback`] error unchanged if the children of
    /// the `root` [`node`] cannot be populated from the [`source`] data.
    ///
    /// ---
    ///
    /// See the [`parser`] documentation for more information.
    ///
    /// [`cursor`]: crate::lazy_tree::Cursor
    /// [`node`]: crate::lazy_tree::Node
    /// [`parser's`]: crate::Parser
    /// [`tree`]: crate::lazy_tree::Tree
    /// [`callback`]: crate::lazy_tree::node::Callback
    /// [`source`]: crate::a_bc::lexer::Data::source()
    /// [`parser`]: crate::Parser
    pub fn cursor(&'tree self) -> Result<Cursor<'source, 'tree>, Error> {
        self.tree.cursor()
    }

    /// # Terminates [`node`] [`population`] by all [`cursors`] across all threads
    ///
    /// See the [`parser`] documentation for more information.
    ///
    /// [`node`]: crate::lazy_tree::Node
    /// [`population`]: crate::lazy_tree::node::Callback
    /// [`cursors`]: crate::lazy_tree::Cursor
    /// [`parser`]: crate::Parser
    pub fn cancel(&self) {
        self.flag.cancel();
    }
}

impl Drop for Parser<'_> {
    fn drop(&mut self) {
        self.cancel();
    }
}

#[cfg(test)]
mod tests {
    use std::thread;

    use pretty_assertions::assert_eq;

    use crate::{
        a_bc::error::{self, Error, Position, Relation},
        lazy_tree::cursor::{Direction, Target},
        lexer::Json,
        parser::Parser,
    };

    #[test]
    fn cancellation() {
        let n = 10000;
        let source = {
            let mut source = String::with_capacity(2 + n * 2);
            source.push('[');

            for i in 0..n - 1 {
                source.push_str(format!("{}", i % 10).as_ref());
                source.push(',');
            }

            source.push_str("0]");

            source
        };

        let parser = Parser::new(&source).unwrap();

        thread::scope(|s| {
            s.spawn(|| {
                assert!(
                    parser
                        .cursor()
                        .is_err_and(|e| e == error::Lexer::Cancelled.into())
                )
            });

            s.spawn(|| {
                parser.cancel();
            });
        });
    }

    #[test]
    fn empty_source() {
        let source = "";
        assert!(
            Parser::new(source)
                .is_err_and(|e| { e == error::Lexer::SourceIsEmpty.into() })
        );
    }

    #[test]
    fn whitespace_source() {
        let source = " \n \t ";
        assert!(Parser::new(source).is_err_and(|e| {
            e == error::Token::ExpectedButGot {
                expected: [
                    Json::Object,
                    Json::Array,
                    Json::String,
                    Json::Sequence,
                ]
                .into(),
                got: None,
                location: (Relation::After, Position::new(2, 3)),
            }
            .into()
        }));
    }

    #[test]
    fn empty_string() {
        let source = " \n \t \"\" \n \t ";
        let parser = Parser::new(source).unwrap();
        let cursor = parser.cursor().unwrap();
        assert_eq!(cursor.value().kind, Json::String);
        assert_eq!(cursor.value().content, source);
        cursor
            .children()
            .iter()
            .zip([(Json::String, "")].iter())
            .for_each(|(left, right)| {
                assert_eq!(left.value().kind, right.0);
                assert_eq!(left.value().content, right.1);
            });

        let source = r#""""#;
        let parser = Parser::new(source).unwrap();
        let cursor = parser.cursor().unwrap();
        assert_eq!(cursor.value().kind, Json::String);
        assert_eq!(cursor.value().content, source);
        cursor
            .children()
            .iter()
            .zip([(Json::String, "")].iter())
            .for_each(|(left, right)| {
                assert_eq!(left.value().kind, right.0);
                assert_eq!(left.value().content, right.1);
            });
    }

    #[test]
    fn valid_string() {
        let source = " \n \t \"foo\" \n \t ";
        let parser = Parser::new(source).unwrap();
        let cursor = parser.cursor().unwrap();
        assert_eq!(cursor.value().kind, Json::String);
        assert_eq!(cursor.value().content, source);
        cursor
            .children()
            .iter()
            .zip([(Json::String, "foo")].iter())
            .for_each(|(left, right)| {
                assert_eq!(left.value().kind, right.0);
                assert_eq!(left.value().content, right.1);
            });

        let source = r#""foo""#;
        let parser = Parser::new(source).unwrap();
        let cursor = parser.cursor().unwrap();
        assert_eq!(cursor.value().kind, Json::String);
        assert_eq!(cursor.value().content, source);
        cursor
            .children()
            .iter()
            .zip([(Json::String, "foo")].iter())
            .for_each(|(left, right)| {
                assert_eq!(left.value().kind, right.0);
                assert_eq!(left.value().content, right.1);
            });

        let source = r#""   ""#;
        let parser = Parser::new(source).unwrap();
        let cursor = parser.cursor().unwrap();
        assert_eq!(cursor.value().kind, Json::String);
        assert_eq!(cursor.value().content, source);
        cursor
            .children()
            .iter()
            .zip([(Json::String, "   ")].iter())
            .for_each(|(left, right)| {
                assert_eq!(left.value().kind, right.0);
                assert_eq!(left.value().content, right.1);
            });

        let source = r#"" foo bar ""#;
        let parser = Parser::new(source).unwrap();
        let cursor = parser.cursor().unwrap();
        assert_eq!(cursor.value().kind, Json::String);
        assert_eq!(cursor.value().content, source);
        cursor
            .children()
            .iter()
            .zip([(Json::String, " foo bar ")].iter())
            .for_each(|(left, right)| {
                assert_eq!(left.value().kind, right.0);
                assert_eq!(left.value().content, right.1);
            });

        let source = r#""foo \" bar""#;
        let parser = Parser::new(source).unwrap();
        let cursor = parser.cursor().unwrap();
        assert_eq!(cursor.value().kind, Json::String);
        assert_eq!(cursor.value().content, source);
        cursor
            .children()
            .iter()
            .zip([(Json::String, r#"foo \" bar"#)].iter())
            .for_each(|(left, right)| {
                assert_eq!(left.value().kind, right.0);
                assert_eq!(left.value().content, right.1);
            });

        let source = r#""foo \"  \" bar""#;
        let parser = Parser::new(source).unwrap();
        let cursor = parser.cursor().unwrap();
        assert_eq!(cursor.value().kind, Json::String);
        assert_eq!(cursor.value().content, source);
        cursor
            .children()
            .iter()
            .zip([(Json::String, r#"foo \"  \" bar"#)].iter())
            .for_each(|(left, right)| {
                assert_eq!(left.value().kind, right.0);
                assert_eq!(left.value().content, right.1);
            });

        let source = r#""foo \"  \" bar""#;
        let parser = Parser::new(source).unwrap();
        let cursor = parser.cursor().unwrap();
        assert_eq!(cursor.value().kind, Json::String);
        assert_eq!(cursor.value().content, source);
        cursor
            .children()
            .iter()
            .zip([(Json::String, r#"foo \"  \" bar"#)].iter())
            .for_each(|(left, right)| {
                assert_eq!(left.value().kind, right.0);
                assert_eq!(left.value().content, right.1);
            });

        let source = r#""foo { bar""#;
        let parser = Parser::new(source).unwrap();
        let cursor = parser.cursor().unwrap();
        assert_eq!(cursor.value().kind, Json::String);
        assert_eq!(cursor.value().content, source);
        cursor
            .children()
            .iter()
            .zip([(Json::String, r#"foo { bar"#)].iter())
            .for_each(|(left, right)| {
                assert_eq!(left.value().kind, right.0);
                assert_eq!(left.value().content, right.1);
            });

        let source = r#""foo } bar""#;
        let parser = Parser::new(source).unwrap();
        let cursor = parser.cursor().unwrap();
        assert_eq!(cursor.value().kind, Json::String);
        assert_eq!(cursor.value().content, source);
        cursor
            .children()
            .iter()
            .zip([(Json::String, r#"foo } bar"#)].iter())
            .for_each(|(left, right)| {
                assert_eq!(left.value().kind, right.0);
                assert_eq!(left.value().content, right.1);
            });

        let source = r#""foo {} bar""#;
        let parser = Parser::new(source).unwrap();
        let cursor = parser.cursor().unwrap();
        assert_eq!(cursor.value().kind, Json::String);
        assert_eq!(cursor.value().content, source);
        cursor
            .children()
            .iter()
            .zip([(Json::String, r#"foo {} bar"#)].iter())
            .for_each(|(left, right)| {
                assert_eq!(left.value().kind, right.0);
                assert_eq!(left.value().content, right.1);
            });

        let source = r#""foo [ bar""#;
        let parser = Parser::new(source).unwrap();
        let cursor = parser.cursor().unwrap();
        assert_eq!(cursor.value().kind, Json::String);
        assert_eq!(cursor.value().content, source);
        cursor
            .children()
            .iter()
            .zip([(Json::String, r#"foo [ bar"#)].iter())
            .for_each(|(left, right)| {
                assert_eq!(left.value().kind, right.0);
                assert_eq!(left.value().content, right.1);
            });

        let source = r#""foo ] bar""#;
        let parser = Parser::new(source).unwrap();
        let cursor = parser.cursor().unwrap();
        assert_eq!(cursor.value().kind, Json::String);
        assert_eq!(cursor.value().content, source);
        cursor
            .children()
            .iter()
            .zip([(Json::String, r#"foo ] bar"#)].iter())
            .for_each(|(left, right)| {
                assert_eq!(left.value().kind, right.0);
                assert_eq!(left.value().content, right.1);
            });

        let source = r#""foo [] bar""#;
        let parser = Parser::new(source).unwrap();
        let cursor = parser.cursor().unwrap();
        assert_eq!(cursor.value().kind, Json::String);
        assert_eq!(cursor.value().content, source);
        cursor
            .children()
            .iter()
            .zip([(Json::String, r#"foo [] bar"#)].iter())
            .for_each(|(left, right)| {
                assert_eq!(left.value().kind, right.0);
                assert_eq!(left.value().content, right.1);
            });

        let source = r#""foo : bar""#;
        let parser = Parser::new(source).unwrap();
        let cursor = parser.cursor().unwrap();
        assert_eq!(cursor.value().kind, Json::String);
        assert_eq!(cursor.value().content, source);
        cursor
            .children()
            .iter()
            .zip([(Json::String, r#"foo : bar"#)].iter())
            .for_each(|(left, right)| {
                assert_eq!(left.value().kind, right.0);
                assert_eq!(left.value().content, right.1);
            });
    }

    #[test]
    fn invalid_string() {
        let source = r#" " "#;
        let parser = Parser::new(source).unwrap();
        assert!(parser.cursor().is_err_and(|e| {
            e == Error::Token(error::Token::PairNotFound {
                opening: "\"".into(),
                closing: "\"".into(),
                location: (Relation::After, (1, 2).into()),
            })
        }));
    }

    #[test]
    fn sequence() {
        let source = "asdfgh";
        let parser = Parser::new(source).unwrap();
        let cursor = parser.cursor().unwrap();
        assert_eq!(cursor.value().kind, Json::Sequence);
        assert_eq!(cursor.value().content, source);
        cursor
            .children()
            .iter()
            .zip([(Json::Sequence, "asdfgh")].iter())
            .for_each(|(left, right)| {
                assert_eq!(left.value().kind, right.0);
                assert_eq!(left.value().content, right.1);
            });

        let source = "123456";
        let parser = Parser::new(source).unwrap();
        let cursor = parser.cursor().unwrap();
        assert_eq!(cursor.value().kind, Json::Sequence);
        assert_eq!(cursor.value().content, source);
        cursor
            .children()
            .iter()
            .zip([(Json::Sequence, "123456")].iter())
            .for_each(|(left, right)| {
                assert_eq!(left.value().kind, right.0);
                assert_eq!(left.value().content, right.1);
            });

        let source = "a1s2d3f4g5h6";
        let parser = Parser::new(source).unwrap();
        let cursor = parser.cursor().unwrap();
        assert_eq!(cursor.value().kind, Json::Sequence);
        assert_eq!(cursor.value().content, source);
        cursor
            .children()
            .iter()
            .zip([(Json::Sequence, "a1s2d3f4g5h6")].iter())
            .for_each(|(left, right)| {
                assert_eq!(left.value().kind, right.0);
                assert_eq!(left.value().content, right.1);
            });

        let source = r#"a{1s"2d[3f}4g"5h]6"#;
        let parser = Parser::new(source).unwrap();
        let cursor = parser.cursor().unwrap();
        assert_eq!(cursor.value().kind, Json::Sequence);
        assert_eq!(cursor.value().content, source);
        cursor
            .children()
            .iter()
            .zip([(Json::Sequence, r#"a{1s"2d[3f}4g"5h]6"#)].iter())
            .for_each(|(left, right)| {
                assert_eq!(left.value().kind, right.0);
                assert_eq!(left.value().content, right.1);
            });
    }

    #[test]
    fn empty_object() {
        let source = "{}";
        let parser = Parser::new(source).unwrap();
        let cursor = parser.cursor().unwrap();
        assert_eq!(cursor.value().kind, Json::Object);
        assert_eq!(cursor.value().content, source);
        assert!(cursor.children().is_empty());

        let source = " \n \t {} \n \t";
        let parser = Parser::new(source).unwrap();
        let cursor = parser.cursor().unwrap();
        assert_eq!(cursor.value().kind, Json::Object);
        assert_eq!(cursor.value().content, source);
        assert!(cursor.children().is_empty());

        let source = "{ \n \t }";
        let parser = Parser::new(source).unwrap();
        let cursor = parser.cursor().unwrap();
        assert_eq!(cursor.value().kind, Json::Object);
        assert_eq!(cursor.value().content, source);
        assert!(cursor.children().is_empty());

        let source = " \n \t { \n \t } \n \t";
        let parser = Parser::new(source).unwrap();
        let cursor = parser.cursor().unwrap();
        assert_eq!(cursor.value().kind, Json::Object);
        assert_eq!(cursor.value().content, source);
        assert!(cursor.children().is_empty());
    }

    #[test]
    fn valid_object() {
        let source =
            " \n \t { \n \t \"foo\" \n \t : \n \t \"bar\" \n \t } \n \t ";
        let parser = Parser::new(source).unwrap();
        let cursor = parser.cursor().unwrap();
        assert_eq!(cursor.value().kind, Json::Object);
        assert_eq!(cursor.value().content, source);
        cursor
            .children()
            .iter()
            .zip([(Json::String, "foo")].iter())
            .for_each(|(left, right)| {
                assert_eq!(left.value().kind, right.0);
                assert_eq!(left.value().content, right.1);
            });

        let source = r#"{"foo":"bar"}"#;
        let parser = Parser::new(source).unwrap();
        let cursor = parser.cursor().unwrap();
        assert_eq!(cursor.value().kind, Json::Object);
        assert_eq!(cursor.value().content, source);
        cursor
            .children()
            .iter()
            .zip([(Json::String, "foo")].iter())
            .for_each(|(left, right)| {
                assert_eq!(left.value().kind, right.0);
                assert_eq!(left.value().content, right.1);
            });

        let source = r#" { "foo": 42, "bar": "baz" } "#;
        let parser = Parser::new(source).unwrap();
        let cursor = parser.cursor().unwrap();
        assert_eq!(cursor.value().kind, Json::Object);
        assert_eq!(cursor.value().content, source);
        cursor
            .children()
            .iter()
            .zip([(Json::Sequence, "foo"), (Json::String, "bar")].iter())
            .for_each(|(left, right)| {
                assert_eq!(left.value().kind, right.0);
                assert_eq!(left.value().content, right.1);
            });

        let source = r#" { "foo": bar } "#;
        let parser = Parser::new(source).unwrap();
        let cursor = parser.cursor().unwrap();
        assert_eq!(cursor.value().kind, Json::Object);
        assert_eq!(cursor.value().content, source);
        cursor
            .children()
            .iter()
            .zip([(Json::Sequence, "foo")].iter())
            .for_each(|(left, right)| {
                assert_eq!(left.value().kind, right.0);
                assert_eq!(left.value().content, right.1);
            });

        let source = r#" { "foo": { not parsed yet } } "#;
        let parser = Parser::new(source).unwrap();
        let cursor = parser.cursor().unwrap();
        assert_eq!(cursor.value().kind, Json::Object);
        assert_eq!(cursor.value().content, source);
        cursor
            .children()
            .iter()
            .zip([(Json::Object, "foo")].iter())
            .for_each(|(left, right)| {
                assert_eq!(left.value().kind, right.0);
                assert_eq!(left.value().content, right.1);
            });

        let source = r#" { "foo": [ not parsed yet ] } "#;
        let parser = Parser::new(source).unwrap();
        let cursor = parser.cursor().unwrap();
        assert_eq!(cursor.value().kind, Json::Object);
        assert_eq!(cursor.value().content, source);
        cursor
            .children()
            .iter()
            .zip([(Json::Array, "foo")].iter())
            .for_each(|(left, right)| {
                assert_eq!(left.value().kind, right.0);
                assert_eq!(left.value().content, right.1);
            });

        let source = r#" { "": "" } "#;
        let parser = Parser::new(source).unwrap();
        let cursor = parser.cursor().unwrap();
        assert_eq!(cursor.value().kind, Json::Object);
        assert_eq!(cursor.value().content, source);
        cursor
            .children()
            .iter()
            .zip([(Json::String, "")].iter())
            .for_each(|(left, right)| {
                assert_eq!(left.value().kind, right.0);
                assert_eq!(left.value().content, right.1);
            });
    }

    #[test]
    fn invalid_object() {
        let source = r#" { "foo": "bar"  "#;
        let parser = Parser::new(source).unwrap();
        assert!(parser.cursor().is_err_and(|e| {
            e == Error::Token(error::Token::PairNotFound {
                opening: "{".into(),
                closing: "}".into(),
                location: (Relation::After, (1, 2).into()),
            })
        }));

        let source = r#" { "foo": "bar", } "#;
        let parser = Parser::new(source).unwrap();
        assert!(parser.cursor().is_err_and(|e| {
            e == Error::Token(error::Token::ExpectedButGot {
                expected: [Json::String].into(),
                got: None,
                location: (Relation::After, (1, 17).into()),
            })
        }));

        let source = r#" { foo: "bar" } "#;
        let parser = Parser::new(source).unwrap();
        assert!(parser.cursor().is_err_and(|e| {
            e == Error::Token(error::Token::ExpectedButGot {
                expected: [Json::String].into(),
                got: Some(Json::Sequence),
                location: (Relation::At, (1, 4).into()),
            })
        }));

        let source = r#" { "foo" } "#;
        let parser = Parser::new(source).unwrap();
        assert!(parser.cursor().is_err_and(|e| {
            e == Error::Token(error::Token::ExpectedButGot {
                expected: [Json::Colon].into(),
                got: None,
                location: (Relation::After, (1, 9).into()),
            })
        }));

        let source = r#" { foo } "#;
        let parser = Parser::new(source).unwrap();
        assert!(parser.cursor().is_err_and(|e| {
            e == Error::Token(error::Token::ExpectedButGot {
                expected: [Json::String].into(),
                got: Some(Json::Sequence),
                location: (Relation::At, (1, 4).into()),
            })
        }));

        let source = r#" { "foo" "bar" } "#;
        let parser = Parser::new(source).unwrap();
        assert!(parser.cursor().is_err_and(|e| {
            e == Error::Token(error::Token::ExpectedButGot {
                expected: [Json::Colon].into(),
                got: Some(Json::String),
                location: (Relation::At, (1, 10).into()),
            })
        }));
    }

    #[test]
    fn empty_array() {
        let source = "[]";
        let parser = Parser::new(source).unwrap();
        let cursor = parser.cursor().unwrap();
        assert_eq!(cursor.value().kind, Json::Array);
        assert_eq!(cursor.value().content, source);
        assert!(cursor.children().is_empty());

        let source = " \n \t [] \n \t";
        let parser = Parser::new(source).unwrap();
        let cursor = parser.cursor().unwrap();
        assert_eq!(cursor.value().kind, Json::Array);
        assert_eq!(cursor.value().content, source);
        assert!(cursor.children().is_empty());

        let source = "[ \n \t ]";
        let parser = Parser::new(source).unwrap();
        let cursor = parser.cursor().unwrap();
        assert_eq!(cursor.value().kind, Json::Array);
        assert_eq!(cursor.value().content, source);
        assert!(cursor.children().is_empty());

        let source = " \n \t [ \n \t ] \n \t ";
        let parser = Parser::new(source).unwrap();
        let cursor = parser.cursor().unwrap();
        assert_eq!(cursor.value().kind, Json::Array);
        assert_eq!(cursor.value().content, source);
        assert!(cursor.children().is_empty());
    }

    #[test]
    fn valid_array() {
        let source = " \n \t [ \n \t \"foo\" \n \t , \n \t bar \n \t , \n \t [ \n \t ] \n \t , \n \t { \n \t } \n \t ]";
        let parser = Parser::new(source).unwrap();
        let cursor = parser.cursor().unwrap();
        assert_eq!(cursor.value().kind, Json::Array);
        assert_eq!(cursor.value().content, source);
        cursor
            .children()
            .iter()
            .zip(
                [
                    (Json::String, "foo"),
                    (Json::Sequence, "bar"),
                    (Json::Array, "[...]"),
                    (Json::Object, "{...}"),
                ]
                .iter(),
            )
            .for_each(|(left, right)| {
                assert_eq!(left.value().kind, right.0);
                assert_eq!(left.value().content, right.1);
            });

        let source = r#"["foo",bar,[],{}]"#;
        let parser = Parser::new(source).unwrap();
        let cursor = parser.cursor().unwrap();
        assert_eq!(cursor.value().kind, Json::Array);
        assert_eq!(cursor.value().content, source);
        cursor
            .children()
            .iter()
            .zip(
                [
                    (Json::String, "foo"),
                    (Json::Sequence, "bar"),
                    (Json::Array, "[...]"),
                    (Json::Object, "{...}"),
                ]
                .iter(),
            )
            .for_each(|(left, right)| {
                assert_eq!(left.value().kind, right.0);
                assert_eq!(left.value().content, right.1);
            });

        let source = r#" [ "foo", 42 ] "#;
        let parser = Parser::new(source).unwrap();
        let cursor = parser.cursor().unwrap();
        assert_eq!(cursor.value().kind, Json::Array);
        assert_eq!(cursor.value().content, source);
        cursor
            .children()
            .iter()
            .zip([(Json::String, "foo"), (Json::Sequence, "42")].iter())
            .for_each(|(left, right)| {
                assert_eq!(left.value().kind, right.0);
                assert_eq!(left.value().content, right.1);
            });

        let source = r#" [ "foo", bar ] "#;
        let parser = Parser::new(source).unwrap();
        let cursor = parser.cursor().unwrap();
        assert_eq!(cursor.value().kind, Json::Array);
        assert_eq!(cursor.value().content, source);
        cursor
            .children()
            .iter()
            .zip([(Json::String, "foo"), (Json::Sequence, "bar")].iter())
            .for_each(|(left, right)| {
                assert_eq!(left.value().kind, right.0);
                assert_eq!(left.value().content, right.1);
            });

        let source = r#" [ "foo", { not parsed yet } ] "#;
        let parser = Parser::new(source).unwrap();
        let cursor = parser.cursor().unwrap();
        assert_eq!(cursor.value().kind, Json::Array);
        assert_eq!(cursor.value().content, source);
        cursor
            .children()
            .iter()
            .zip([(Json::String, "foo"), (Json::Object, "{...}")].iter())
            .for_each(|(left, right)| {
                assert_eq!(left.value().kind, right.0);
                assert_eq!(left.value().content, right.1);
            });

        let source = r#" [ "foo", [ not parsed yet ] ] "#;
        let parser = Parser::new(source).unwrap();
        let cursor = parser.cursor().unwrap();
        assert_eq!(cursor.value().kind, Json::Array);
        assert_eq!(cursor.value().content, source);
        cursor
            .children()
            .iter()
            .zip([(Json::String, "foo"), (Json::Array, "[...]")].iter())
            .for_each(|(left, right)| {
                assert_eq!(left.value().kind, right.0);
                assert_eq!(left.value().content, right.1);
            });

        let source = r#" [ [] ] "#;
        let parser = Parser::new(source).unwrap();
        let cursor = parser.cursor().unwrap();
        assert_eq!(cursor.value().kind, Json::Array);
        assert_eq!(cursor.value().content, source);
        cursor
            .children()
            .iter()
            .zip([(Json::Array, "[...]")].iter())
            .for_each(|(left, right)| {
                assert_eq!(left.value().kind, right.0);
                assert_eq!(left.value().content, right.1);
            });
    }

    #[test]
    fn invalid_array() {
        let source = r#" [ "foo", bar "#;
        let parser = Parser::new(source).unwrap();
        assert!(parser.cursor().is_err_and(|e| {
            e == Error::Token(error::Token::PairNotFound {
                opening: "[".into(),
                closing: "]".into(),
                location: (Relation::After, (1, 2).into()),
            })
        }));

        let source = r#" [ "foo", bar, ] "#;
        let parser = Parser::new(source).unwrap();
        assert!(parser.cursor().is_err_and(|e| {
            e == Error::Token(error::Token::NotExpectedButGot {
                not_expected: [Json::Comma, Json::Colon].into(),
                got: None,
                location: (Relation::After, (1, 15).into()),
            })
        }));

        let source = r#" [ "foo" bar ] "#;
        let parser = Parser::new(source).unwrap();
        assert!(parser.cursor().is_err_and(|e| {
            e == Error::Token(error::Token::ExpectedButGot {
                expected: [Json::Comma].into(),
                got: Some(Json::Sequence),
                location: (Relation::At, (1, 10).into()),
            })
        }));
    }

    #[test]
    fn full() {
        let source = r#"
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

        let parser = Parser::new(source).unwrap();

        let mut cursor = parser.cursor().unwrap();
        assert_eq!(cursor.value().kind, Json::Object);
        assert_eq!(cursor.value().content, source);
        cursor
            .children()
            .iter()
            .zip(
                [
                    (Json::String, "first_name"),
                    (Json::String, "last_name"),
                    (Json::Sequence, "is_alive"),
                    (Json::Sequence, "age"),
                    (Json::Object, "address"),
                    (Json::Array, "phone_numbers"),
                    (Json::Array, "children"),
                    (Json::Sequence, "spouse"),
                ]
                .iter(),
            )
            .for_each(|(left, right)| {
                assert_eq!(left.value().kind, right.0);
                assert_eq!(left.value().content, right.1);
            });
        assert_eq!(cursor.path(), &[0]);

        assert!(cursor.go_to(&[0, 0]).unwrap());
        assert_eq!(cursor.value().kind, Json::String);
        assert_eq!(cursor.value().content, "first_name");
        cursor
            .children()
            .iter()
            .zip([(Json::String, "John")].iter())
            .for_each(|(left, right)| {
                assert_eq!(left.value().kind, right.0);
                assert_eq!(left.value().content, right.1);
            });
        assert_eq!(cursor.path(), &[0, 0]);

        assert!(cursor.go_to(&[0, 0, 0]).unwrap());
        assert_eq!(cursor.value().kind, Json::String);
        assert_eq!(cursor.value().content, "John");
        assert!(cursor.children().is_empty());
        assert_eq!(cursor.path(), &[0, 0, 0]);

        assert!(cursor.go_to(&[0, 1]).unwrap());
        assert_eq!(cursor.value().kind, Json::String);
        assert_eq!(cursor.value().content, "last_name");
        cursor
            .children()
            .iter()
            .zip([(Json::String, "Smith")].iter())
            .for_each(|(left, right)| {
                assert_eq!(left.value().kind, right.0);
                assert_eq!(left.value().content, right.1);
            });
        assert_eq!(cursor.path(), &[0, 1]);

        assert!(cursor.go_to(&[0, 1, 0]).unwrap());
        assert_eq!(cursor.value().kind, Json::String);
        assert_eq!(cursor.value().content, "Smith");
        assert!(cursor.children().is_empty());
        assert_eq!(cursor.path(), &[0, 1, 0]);

        assert!(cursor.go_to(&[0, 2]).unwrap());
        assert_eq!(cursor.value().kind, Json::Sequence);
        assert_eq!(cursor.value().content, "is_alive");
        cursor
            .children()
            .iter()
            .zip([(Json::Sequence, "true")].iter())
            .for_each(|(left, right)| {
                assert_eq!(left.value().kind, right.0);
                assert_eq!(left.value().content, right.1);
            });
        assert_eq!(cursor.path(), &[0, 2]);

        assert!(cursor.go_to(&[0, 2, 0]).unwrap());
        assert_eq!(cursor.value().kind, Json::Sequence);
        assert_eq!(cursor.value().content, "true");
        assert!(cursor.children().is_empty());
        assert_eq!(cursor.path(), &[0, 2, 0]);

        assert!(cursor.go_to(&[0, 3]).unwrap());
        assert_eq!(cursor.value().kind, Json::Sequence);
        assert_eq!(cursor.value().content, "age");
        cursor
            .children()
            .iter()
            .zip([(Json::Sequence, "27")].iter())
            .for_each(|(left, right)| {
                assert_eq!(left.value().kind, right.0);
                assert_eq!(left.value().content, right.1);
            });
        assert_eq!(cursor.path(), &[0, 3]);

        assert!(cursor.go_to(&[0, 3, 0]).unwrap());
        assert_eq!(cursor.value().kind, Json::Sequence);
        assert_eq!(cursor.value().content, "27");
        assert!(cursor.children().is_empty());
        assert_eq!(cursor.path(), &[0, 3, 0]);

        assert!(cursor.go_to(&[0, 4]).unwrap());
        assert_eq!(cursor.value().kind, Json::Object);
        assert_eq!(cursor.value().content, "address");
        cursor
            .children()
            .iter()
            .zip(
                [
                    (Json::String, "street_address"),
                    (Json::String, "city"),
                    (Json::String, "state"),
                    (Json::String, "postal_code"),
                ]
                .iter(),
            )
            .for_each(|(left, right)| {
                assert_eq!(left.value().kind, right.0);
                assert_eq!(left.value().content, right.1);
            });
        assert_eq!(cursor.path(), &[0, 4]);

        assert!(cursor.go_to(&[0, 4, 0]).unwrap());
        assert_eq!(cursor.value().kind, Json::String);
        assert_eq!(cursor.value().content, "street_address");
        cursor
            .children()
            .iter()
            .zip([(Json::String, "21 2nd Street")].iter())
            .for_each(|(left, right)| {
                assert_eq!(left.value().kind, right.0);
                assert_eq!(left.value().content, right.1);
            });
        assert_eq!(cursor.path(), &[0, 4, 0]);

        assert!(cursor.go_to(&[0, 4, 0, 0]).unwrap());
        assert_eq!(cursor.value().kind, Json::String);
        assert_eq!(cursor.value().content, "21 2nd Street");
        assert!(cursor.children().is_empty());
        assert_eq!(cursor.path(), &[0, 4, 0, 0]);

        assert!(cursor.go_to(&[0, 4, 1]).unwrap());
        assert_eq!(cursor.value().kind, Json::String);
        assert_eq!(cursor.value().content, "city");
        cursor
            .children()
            .iter()
            .zip([(Json::String, "New York")].iter())
            .for_each(|(left, right)| {
                assert_eq!(left.value().kind, right.0);
                assert_eq!(left.value().content, right.1);
            });
        assert_eq!(cursor.path(), &[0, 4, 1]);

        assert!(cursor.go_to(&[0, 4, 1, 0]).unwrap());
        assert_eq!(cursor.value().kind, Json::String);
        assert_eq!(cursor.value().content, "New York");
        assert!(cursor.children().is_empty());
        assert_eq!(cursor.path(), &[0, 4, 1, 0]);

        assert!(cursor.go_to(&[0, 4, 2]).unwrap());
        assert_eq!(cursor.value().kind, Json::String);
        assert_eq!(cursor.value().content, "state");
        cursor
            .children()
            .iter()
            .zip([(Json::String, "NY")].iter())
            .for_each(|(left, right)| {
                assert_eq!(left.value().kind, right.0);
                assert_eq!(left.value().content, right.1);
            });
        assert_eq!(cursor.path(), &[0, 4, 2]);

        assert!(cursor.go_to(&[0, 4, 2, 0]).unwrap());
        assert_eq!(cursor.value().kind, Json::String);
        assert_eq!(cursor.value().content, "NY");
        assert!(cursor.children().is_empty());
        assert_eq!(cursor.path(), &[0, 4, 2, 0]);

        assert!(cursor.go_to(&[0, 4, 3]).unwrap());
        assert_eq!(cursor.value().kind, Json::String);
        assert_eq!(cursor.value().content, "postal_code");
        cursor
            .children()
            .iter()
            .zip([(Json::String, "10021-3100")].iter())
            .for_each(|(left, right)| {
                assert_eq!(left.value().kind, right.0);
                assert_eq!(left.value().content, right.1);
            });
        assert_eq!(cursor.path(), &[0, 4, 3]);

        assert!(cursor.go_to(&[0, 4, 3, 0]).unwrap());
        assert_eq!(cursor.value().kind, Json::String);
        assert_eq!(cursor.value().content, "10021-3100");
        assert!(cursor.children().is_empty());
        assert_eq!(cursor.path(), &[0, 4, 3, 0]);

        assert!(cursor.go_to(&[0, 5]).unwrap());
        assert_eq!(cursor.value().kind, Json::Array);
        assert_eq!(cursor.value().content, "phone_numbers");
        cursor
            .children()
            .iter()
            .zip([(Json::Object, "{...}"), (Json::Object, "{...}")].iter())
            .for_each(|(left, right)| {
                assert_eq!(left.value().kind, right.0);
                assert_eq!(left.value().content, right.1);
            });
        assert_eq!(cursor.path(), &[0, 5]);

        assert!(cursor.go_to(&[0, 5, 0]).unwrap());
        assert_eq!(cursor.value().kind, Json::Object);
        assert_eq!(cursor.value().content, "{...}");
        cursor
            .children()
            .iter()
            .zip([(Json::String, "type"), (Json::String, "number")].iter())
            .for_each(|(left, right)| {
                assert_eq!(left.value().kind, right.0);
                assert_eq!(left.value().content, right.1);
            });
        assert_eq!(cursor.path(), &[0, 5, 0]);

        assert!(cursor.go_to(&[0, 5, 0, 0]).unwrap());
        assert_eq!(cursor.value().kind, Json::String);
        assert_eq!(cursor.value().content, "type");
        cursor
            .children()
            .iter()
            .zip([(Json::String, "home")].iter())
            .for_each(|(left, right)| {
                assert_eq!(left.value().kind, right.0);
                assert_eq!(left.value().content, right.1);
            });
        assert_eq!(cursor.path(), &[0, 5, 0, 0]);

        assert!(cursor.go_to(&[0, 5, 0, 0, 0]).unwrap());
        assert_eq!(cursor.value().kind, Json::String);
        assert_eq!(cursor.value().content, "home");
        assert!(cursor.children().is_empty());
        assert_eq!(cursor.path(), &[0, 5, 0, 0, 0]);

        assert!(cursor.go_to(&[0, 5, 0, 1]).unwrap());
        assert_eq!(cursor.value().kind, Json::String);
        assert_eq!(cursor.value().content, "number");
        cursor
            .children()
            .iter()
            .zip([(Json::String, "212 555-1234")].iter())
            .for_each(|(left, right)| {
                assert_eq!(left.value().kind, right.0);
                assert_eq!(left.value().content, right.1);
            });
        assert_eq!(cursor.path(), &[0, 5, 0, 1]);

        assert!(cursor.go_to(&[0, 5, 0, 1, 0]).unwrap());
        assert_eq!(cursor.value().kind, Json::String);
        assert_eq!(cursor.value().content, "212 555-1234");
        assert!(cursor.children().is_empty());
        assert_eq!(cursor.path(), &[0, 5, 0, 1, 0]);

        assert!(cursor.go_to(&[0, 5, 1]).unwrap());
        assert_eq!(cursor.value().kind, Json::Object);
        assert_eq!(cursor.value().content, "{...}");
        cursor
            .children()
            .iter()
            .zip([(Json::String, "type"), (Json::String, "number")].iter())
            .for_each(|(left, right)| {
                assert_eq!(left.value().kind, right.0);
                assert_eq!(left.value().content, right.1);
            });
        assert_eq!(cursor.path(), &[0, 5, 1]);

        assert!(cursor.go_to(&[0, 5, 1, 0]).unwrap());
        assert_eq!(cursor.value().kind, Json::String);
        assert_eq!(cursor.value().content, "type");
        cursor
            .children()
            .iter()
            .zip([(Json::String, "office")].iter())
            .for_each(|(left, right)| {
                assert_eq!(left.value().kind, right.0);
                assert_eq!(left.value().content, right.1);
            });
        assert_eq!(cursor.path(), &[0, 5, 1, 0]);

        assert!(cursor.go_to(&[0, 5, 1, 0, 0]).unwrap());
        assert_eq!(cursor.value().kind, Json::String);
        assert_eq!(cursor.value().content, "office");
        assert!(cursor.children().is_empty());
        assert_eq!(cursor.path(), &[0, 5, 1, 0, 0]);

        assert!(cursor.go_to(&[0, 5, 1, 1]).unwrap());
        assert_eq!(cursor.value().kind, Json::String);
        assert_eq!(cursor.value().content, "number");
        cursor
            .children()
            .iter()
            .zip([(Json::String, "646 555-4567")].iter())
            .for_each(|(left, right)| {
                assert_eq!(left.value().kind, right.0);
                assert_eq!(left.value().content, right.1);
            });
        assert_eq!(cursor.path(), &[0, 5, 1, 1]);

        assert!(cursor.go_to(&[0, 5, 1, 1, 0]).unwrap());
        assert_eq!(cursor.value().kind, Json::String);
        assert_eq!(cursor.value().content, "646 555-4567");
        assert!(cursor.children().is_empty());
        assert_eq!(cursor.path(), &[0, 5, 1, 1, 0]);

        assert!(cursor.go_to(&[0, 6]).unwrap());
        assert_eq!(cursor.value().kind, Json::Array);
        assert_eq!(cursor.value().content, "children");
        cursor
            .children()
            .iter()
            .zip(
                [
                    (Json::String, "Catherine"),
                    (Json::String, "Thomas"),
                    (Json::String, "Trevor"),
                ]
                .iter(),
            )
            .for_each(|(left, right)| {
                assert_eq!(left.value().kind, right.0);
                assert_eq!(left.value().content, right.1);
            });
        assert_eq!(cursor.path(), &[0, 6]);

        assert!(cursor.go_to(&[0, 6, 0]).unwrap());
        assert_eq!(cursor.value().kind, Json::String);
        assert_eq!(cursor.value().content, "Catherine");
        assert!(cursor.children().is_empty());
        assert_eq!(cursor.path(), &[0, 6, 0]);

        assert!(cursor.go_to(&[0, 6, 1]).unwrap());
        assert_eq!(cursor.value().kind, Json::String);
        assert_eq!(cursor.value().content, "Thomas");
        assert!(cursor.children().is_empty());
        assert_eq!(cursor.path(), &[0, 6, 1]);

        assert!(cursor.go_to(&[0, 6, 2]).unwrap());
        assert_eq!(cursor.value().kind, Json::String);
        assert_eq!(cursor.value().content, "Trevor");
        assert!(cursor.children().is_empty());
        assert_eq!(cursor.path(), &[0, 6, 2]);

        assert!(cursor.go_to(&[0, 7]).unwrap());
        assert_eq!(cursor.value().kind, Json::Sequence);
        assert_eq!(cursor.value().content, "spouse");
        cursor
            .children()
            .iter()
            .zip([(Json::Sequence, "null")].iter())
            .for_each(|(left, right)| {
                assert_eq!(left.value().kind, right.0);
                assert_eq!(left.value().content, right.1);
            });
        assert_eq!(cursor.path(), &[0, 7]);

        assert!(cursor.go_to(&[0, 7, 0]).unwrap());
        assert_eq!(cursor.value().kind, Json::Sequence);
        assert_eq!(cursor.value().content, "null");
        assert!(cursor.children().is_empty());
        assert_eq!(cursor.path(), &[0, 7, 0]);

        assert!(cursor.jump(Direction::Right).unwrap());
        assert_eq!(cursor.value().kind, Json::String);
        assert_eq!(cursor.value().content, "John");
        assert!(cursor.children().is_empty());
        assert_eq!(cursor.path(), &[0, 0, 0]);

        assert!(cursor.jump(Direction::Down(Target::First)).unwrap());
        assert_eq!(cursor.value().kind, Json::Object);
        assert_eq!(cursor.value().content, source);
        cursor
            .children()
            .iter()
            .zip(
                [
                    (Json::String, "first_name"),
                    (Json::String, "last_name"),
                    (Json::Sequence, "is_alive"),
                    (Json::Sequence, "age"),
                    (Json::Object, "address"),
                    (Json::Array, "phone_numbers"),
                    (Json::Array, "children"),
                    (Json::Sequence, "spouse"),
                ]
                .iter(),
            )
            .for_each(|(left, right)| {
                assert_eq!(left.value().kind, right.0);
                assert_eq!(left.value().content, right.1);
            });
        assert_eq!(cursor.path(), &[0]);
    }
}
