//! # [`Parser`]
//!
//! ![](https://github.com/mnmun/images/blob/main/cart.png?raw=true)
//!
//! Defines a [`parser`] that represents a JSON string data as a [`tree`]
//! of lazily-created [`nodes`].
//!
//! ---
//!
//! See the [`crate`] documentation for more information.
//!
//! [`parser`]: crate::Parser
//! [`tree`]: crate::lazy_tree::Tree
//! [`nodes`]: crate::lazy_tree::Node

use std::{borrow::Cow, fmt, ops::Range, sync::Arc};

use a_bc::error::{self, Error};
use lazy_tree::Cursor;

use crate::{JSON, lexer};

/// # How often should control be yielded back to the [`tokio`] runtime
const YIELD_INTERVAL: usize = 1;

const OBJECT: &str = "{...}";
const ARRAY: &str = "[...]";

enum Content {
    Object,
    Array,
    Slice(Range<usize>),
}

struct Cache {
    checkpoints: Option<Box<[usize]>>,
    interval: usize,
}

/// # [`Node`] `content` and its `kind`
///
/// ![](https://github.com/mnmun/images/blob/main/cherry.png?raw=true)
///
/// The [`parser`] is implemented on top of a [`tree`] of lazily-created
/// [`nodes`] from the [lazy_tree] crate, and the `Bundle` acts as a data
/// type stored in each [`node`] of the [`tree`].
///
/// The `content` field is represented as a string slice `&str` and can be
/// accessed using the [`content()`] method. When constructing [`nodes`], a
/// [`lexer`] is used to categorize tokens into [`JSON`] `kinds`; consequently,
/// the [`node`] `kind` is also represented with a [`JSON`] enumeration and
/// can be accessed using the [`kind()`] method.
///
/// Access to the getter methods of this structure is provided via a [`cursor`]
/// that implements the [`Deref`] trait, targeting the `Bundle`. A [`cursor`]
/// can be created using the [`Parser::cursor()`] method:
///
/// ```rust
/// # use pretty_assertions::assert_eq;
/// #
/// use lazy_json::{
///     JSON, Parser,
///     parser::{Builder, Bundle},
///     a_bc::Error,
///     lazy_tree::{Cursor, cursor::{Direction, Target}}
/// };
///
/// # #[tokio::main]
/// # async fn main() {
/// let source: &'static str = r#" "foo" "#;
///
/// let parser: Parser = Builder::new(source).build().unwrap();
///
/// let mut cursor: Cursor<Bundle, Error<JSON>> = parser.cursor().await.unwrap();
///
/// cursor.walk(Direction::Down(Target::First)).await.unwrap();
/// assert_eq!(cursor.kind(), &JSON::String);
/// assert_eq!(cursor.content(), "foo");
/// # }
/// ```
///
/// ---
///
/// See the [`module`] documentation for more information.
///
/// [`module`]: crate::parser
/// [`lexer`]: crate::Lexer
/// [`Node`]: crate::lazy_tree::Node
/// [`node`]: crate::lazy_tree::Node
/// [`nodes`]: crate::lazy_tree::Node
/// [`content()`]: crate::Bundle::content()
/// [`kind()`]: crate::Bundle::kind()
/// [`parser`]: crate::Parser
/// [`tree`]: crate::lazy_tree::Tree
/// [`cursor`]: crate::lazy_tree::Cursor
/// [`Deref`]: std::ops::Deref
/// [`Parser::cursor()`]: crate::Parser::cursor()
/// [lazy_tree]: crate::lazy_tree
pub struct Bundle {
    content: Content,
    kind: JSON,

    flag: a_bc::cancellation::Flag,
    source: Arc<Cow<'static, str>>,
    range: Option<Range<usize>>,

    cache: Option<Cache>,
}

impl fmt::Debug for Bundle {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Bundle")
            .field("kind", &self.kind)
            .field("content", &self.content())
            .finish_non_exhaustive()
    }
}

impl Bundle {
    fn new(
        content: Content,
        kind: JSON,

        flag: a_bc::cancellation::Flag,
        source: Arc<Cow<'static, str>>,
        range: impl Into<Option<Range<usize>>>,

        cache_interval: Option<usize>,
    ) -> Self {
        let range = range.into();
        let cache = cache_interval.map(|interval| Cache {
            checkpoints: None,
            interval,
        });

        Self {
            kind,
            content,

            flag,
            source,
            range,

            cache,
        }
    }

    /// # [`Node`] [`content`] kind
    ///
    /// Determined by the [`lexer`] during the execution of a [`query`]
    /// callback.
    ///
    /// ---
    ///
    /// See the [`bundle`] documentation for more information.
    ///
    /// [`bundle`]: crate::Bundle
    /// [`Node`]: crate::lazy_tree::Node
    /// [`content`]: crate::Bundle::content()
    /// [`lexer`]: crate::Lexer
    /// [`query`]: crate::lazy_tree::node::Callbacks::query()
    pub fn kind(&self) -> &JSON {
        &self.kind
    }

    /// # [`Node`] `content`
    ///
    /// Determined by the [`lexer`] during the execution of a [`query`]
    /// callback.
    ///
    /// ---
    ///
    /// See the [`bundle`] documentation for more information.
    ///
    /// [`bundle`]: crate::Bundle
    /// [`Node`]: crate::lazy_tree::Node
    /// [`lexer`]: crate::Lexer
    /// [`query`]: crate::lazy_tree::node::Callbacks::query()
    pub fn content(&self) -> &str {
        match &self.content {
            Content::Object => OBJECT,
            Content::Array => ARRAY,
            Content::Slice(range) => &self.source[range.clone()],
        }
    }
}

struct Callbacks;

#[lazy_tree::async_trait]
impl lazy_tree::node::Callbacks<Bundle, Error<JSON>> for Callbacks {
    async fn total(&self, parent: &mut Bundle) -> Result<usize, Error<JSON>> {
        let range = if let Some(range) = &parent.range {
            range
        } else {
            return Ok(0);
        };

        let mut lexer =
            lexer::Builder::new(Cow::Borrowed(parent.source.as_bytes()))
                .with_range(range.clone())
                .with_flag(parent.flag.clone())
                .build()?;

        let token = lexer.expect(&[
            JSON::Object,
            JSON::Array,
            JSON::String,
            JSON::Sequence,
        ])?;

        let count = match *token.kind() {
            JSON::Object => count::object(parent, token).await?,
            JSON::Array => count::array(parent, token).await?,
            JSON::String | JSON::Sequence => 1,
            _ => unreachable!(),
        };

        Ok(count)
    }

    async fn query(
        &self,
        parent: &mut Bundle,
        index: usize,
    ) -> Result<lazy_tree::node::Builder<Bundle, Error<JSON>>, Error<JSON>>
    {
        let range = parent.range.as_ref().unwrap();

        let mut lexer =
            lexer::Builder::new(Cow::Borrowed(parent.source.as_bytes()))
                .with_range(range.clone())
                .with_flag(parent.flag.clone())
                .build()?;

        let token = lexer.expect(&[
            JSON::Object,
            JSON::Array,
            JSON::String,
            JSON::Sequence,
        ])?;

        match *token.kind() {
            JSON::Object => query::object(parent, token, index).await,
            JSON::Array => query::array(parent, token, index).await,
            JSON::String => query::string(parent, token),
            JSON::Sequence => query::sequence(parent, token),
            _ => unreachable!(),
        }
    }
}

/// # `Parser`
///
/// ![](https://github.com/mnmun/images/blob/main/cart.png?raw=true)
///
/// Represents a JSON string data as a [`tree`] of lazily-created [`nodes`]:
///
/// ```text
///    *Source data*                *Parsed tree*
///
/// {                               *Source data*
///   "object": {                ┌────────┴────────┐
///     "string": "foo",       object            array
///     "number": 42         ┌───┴───┐       ┌─────┼─────┐
///   },                   string  number   true false  null
///   "array": [             │       │
///     true,               foo      42
///     false,
///     null
///   ]
/// }
/// ```
///
/// [`Nodes`] are created automatically when the [`cursor`] needs to access them
/// and are deleted when they are no longer used.
///
/// Implemented using the [lazy_tree] crate for the [tokio] runtime.
///
/// ## Creation
///
/// Can be created using the [`builder`]:
///
/// ```rust
/// use lazy_json::{Parser, parser::Builder};
///
/// let source: &'static str = r#" { "foo": "bar" } "#;
///
/// let parser: Parser = Builder::new(source).build().unwrap();
/// ```
///
/// ## Traversing
///
/// The [`tree`] of [`nodes`] is traversed using a [`cursor`], which can move
/// between [`nodes`] and provides access to the [`bundle`] stored in the
/// [`node`] it currently points to.
///
/// ## Caching
///
/// When creating the `i-th` child [`node`] using a [`cursor`], it is necessary
/// to sequentially iterate through the [`tokens`] of the source data fragment
/// that defines the child [`nodes`], continuing until the [`tokens`]
/// corresponding to the `i-th` child [`node`] are reached. In other words, the
/// time required to create the `i-th` child node scales as `O(n)`, where `n`
/// is the `total` number of child [`nodes`] for a given parent [`node`] (see
/// the [`total`] callback).
///
/// To address this issue, each [`tree`] [`node`] can hold an optional cache -
/// a sequence of saved [`lexer`] positions within the source data, which can
/// be accessed in constant time, `O(1)`. The caching process is defined by an
/// `interval` - specifying how many `name:value` [`token`] pairs should elapse
/// before a position is cached, and is performed once during the execution of
/// the [`total`] callback.
///
/// Enabling caching increases the amount of memory used per [`node`] in the
/// [`tree`], but reduces the time required to create a new [`node`]. The
/// smaller the caching interval, the higher the memory overhead and the
/// shorter the time needed to create a new [`node`]. The minimum caching
/// interval is 1. If a caching interval of 0 was specified, it is automatically
/// changed to 1.
///
/// ## Cancellation
///
/// New [`nodes`] are created via a [`query`] callback, which iterates through
/// the source data; for large sources, this process can be quite
/// time-consuming. Since the [`cursors`] that trigger this callback may operate
/// in separate threads, the `parser` includes a mechanism to stop the execution
/// of the [`query`] callbacks - [`Parser::stop()`]. This method is called
/// automatically on the `parser` drop, but it can also be invoked manually -
/// for instance, when an application needs to shut down and all [`cursors`]
/// across all threads must be stopped.
///
/// ## Examples
///
/// ![](https://github.com/mnmun/images/blob/main/bulb.png?raw=true)
///
/// [`Cursor`] pointing to the `root` [`node`] of a [`tree`] can be created
/// using the [`cursor()`] method. The `root` [`node`] [`bundle`] holds the
/// input JSON string data as [`content`] with corresponding [`kind`]:
///
/// ```rust
/// # use pretty_assertions::assert_eq;
/// #
/// use lazy_json::{
///     JSON, Parser,
///     parser::{Builder, Bundle},
///     a_bc::Error,
///     lazy_tree::Cursor,
/// };
///
/// # #[tokio::main]
/// # async fn main() {
/// let source: &'static str = r#"{ "foo": "bar" }"#;
///
/// let parser: Parser = Builder::new(source).build().unwrap();
///
/// let cursor: Cursor<Bundle, Error<JSON>> = parser.cursor().await.unwrap();
/// assert_eq!(cursor.kind(), &JSON::Object);
/// assert_eq!(cursor.content(), source);
/// # }
/// ```
///
/// `Parser` may have multiple [`cursors`] at the same time, pointing to
/// different [`nodes`] and moving independently of each other. For moving
/// cursors, there are the [`walk()`], [`jump()`] and [`go_to()`] methods:
///
/// ```rust
/// # use pretty_assertions::assert_eq;
/// #
/// use lazy_json::{
///     JSON, Parser,
///     parser::{Builder, Bundle},
///     a_bc::Error,
///     lazy_tree::{Cursor, cursor::{Direction, Target}},
/// };
///
/// # #[tokio::main]
/// # async fn main() {
/// //
/// //              source
/// //       ┌────────┴────────┐
/// //     object            array
/// //   ┌───┴───┐       ┌─────┼─────┐
/// // string  number   true false  null
/// //   │       │
/// //  foo      42
/// //
/// let source: &'static str = r#"
/// {
///   "object": {
///     "string": "foo",
///     "number": 42
///   },
///   "array": [
///     true,
///     false,
///     null
///   ]
/// }
/// "#;
///
/// let parser: Parser = Builder::new(source).build().unwrap();
///
/// let mut cursor_0: Cursor<Bundle, Error<JSON>> =
///     parser.cursor().await.unwrap();
/// cursor_0.walk(Direction::Down(Target::First)).await.unwrap();
/// assert_eq!(cursor_0.kind(), &JSON::Object);
/// assert_eq!(cursor_0.content(), "object");
///
/// let mut cursor_1: Cursor<Bundle, Error<JSON>> =
///     parser.cursor().await.unwrap();
/// cursor_1.jump(Direction::Up).await.unwrap();
/// assert_eq!(cursor_1.kind(), &JSON::String);
/// assert_eq!(cursor_1.content(), "foo");
///
/// let mut cursor_2: Cursor<Bundle, Error<JSON>> =
///     parser.cursor().await.unwrap();
/// cursor_2.go_to(&[0, 0, 1, 0]).await.unwrap();
/// assert_eq!(cursor_2.kind(), &JSON::Sequence);
/// assert_eq!(cursor_2.content(), "42");
/// # }
/// ```
///
/// [`Cursors`] implement the [`Send`] trait, so they can be sent between
/// threads safely. Each [`cursor`] remains borrowed from its `parser` and
/// cannot outlive it:
///
/// ```rust
/// # use std::sync::Arc;
/// #
/// # use pretty_assertions::assert_eq;
/// #
/// use lazy_json::{
///     JSON, Parser,
///     parser::{Builder, Bundle},
///     a_bc::Error,
///     lazy_tree::{Cursor, cursor::{Direction, Target}},
/// };
///
/// # #[tokio::main]
/// # async fn main() {
/// //
/// //              source
/// //       ┌────────┴────────┐
/// //     object            array
/// //   ┌───┴───┐       ┌─────┼─────┐
/// // string  number   true false  null
/// //   │       │
/// //  foo      42
/// //
/// let source: &'static str = r#"
/// {
///   "object": {
///     "string": "foo",
///     "number": 42
///   },
///   "array": [
///     true,
///     false,
///     null
///   ]
/// }
/// "#;
///
/// let parser: Arc<Parser> = Arc::new(Builder::new(source).build().unwrap());
///
/// let task_a = tokio::spawn({
///     let parser = parser.clone();
///
///     async move {
///         let mut cursor: Cursor<Bundle, Error<JSON>> =
///             parser.cursor().await.unwrap();
///         cursor.walk(Direction::Down(Target::First)).await.unwrap();
///
///         for _ in 0..10 {
///             cursor.jump(Direction::Right).await.unwrap();
///         }
///     }
/// });
///
/// let task_b = tokio::spawn({
///     let parser = parser.clone();
///
///     async move {
///         let mut cursor: Cursor<Bundle, Error<JSON>> =
///             parser.cursor().await.unwrap();
///         cursor.walk(Direction::Down(Target::Last)).await.unwrap();
///
///         for _ in 0..10 {
///             cursor.jump(Direction::Left).await.unwrap();
///         }
///     }
/// });
///
/// let (result_a, result_b) = tokio::join!(task_a, task_b);
/// result_a.unwrap();
/// result_b.unwrap();
/// # }
/// ```
///
/// ---
///
/// See the [`module`] documentation for more information.
///
/// [`module`]: crate::parser
/// [`tree`]: crate::lazy_tree::Tree
/// [`nodes`]: crate::lazy_tree::Node
/// [`node`]: crate::lazy_tree::Node
/// [`cursor`]: crate::lazy_tree::Cursor
/// [`Cursor`]: crate::lazy_tree::Cursor
/// [`Cursors`]: crate::lazy_tree::Cursor
/// [`walk()`]: crate::lazy_tree::Cursor::walk()
/// [`jump()`]: crate::lazy_tree::Cursor::jump()
/// [`go_to()`]: crate::lazy_tree::Cursor::go_to()
/// [`cursor()`]: crate::Parser::cursor()
/// [`content`]: crate::Bundle::content()
/// [`kind`]: crate::Bundle::kind()
/// [`bundle`]: crate::Bundle
/// [`builder`]: crate::parser::Builder
/// [`Send`]: std::marker::Send
/// [`tokens`]: crate::a_bc::Token
/// [`token`]: crate::a_bc::Token
/// [`lexer`]: crate::Lexer
/// [`total`]: crate::lazy_tree::node::Callbacks::total()
/// [`query`]: crate::lazy_tree::node::Callbacks::query()
/// [`Parser::stop()`]: crate::Parser::stop()
pub struct Parser {
    tree: lazy_tree::Tree<Bundle, Error<JSON>>,
    flag: a_bc::cancellation::Flag,
}

impl Parser {
    /// # Creates a [`cursor`] at the `root` [`node`]
    ///
    /// See the [`parser`] documentation for more information.
    ///
    /// [`parser`]: crate::Parser
    /// [`cursor`]: crate::lazy_tree::Cursor
    /// [`node`]: crate::lazy_tree::Node
    pub async fn cursor<'parser>(
        &'parser self,
    ) -> Result<Cursor<'parser, Bundle, Error<JSON>>, Error<JSON>> {
        self.tree.cursor().await
    }

    /// # Stops the creation of new [`nodes`] for all its [`cursors`]
    ///
    /// See the [`parser`] documentation for more information.
    ///
    /// [`parser`]: crate::Parser
    /// [`nodes`]: crate::lazy_tree::Node
    /// [`cursors`]: crate::lazy_tree::Cursor
    pub fn stop(&self) {
        self.flag.cancel();
    }
}

impl Drop for Parser {
    fn drop(&mut self) {
        self.stop();
    }
}

/// # [`Parser`] `builder`
///
/// ![](https://github.com/mnmun/images/blob/main/road_work.png?raw=true)
///
/// Holds the data used for the [`parser`] creation.
///
/// The `builder` makes it possible to specify an optional caching interval
/// using the [`with_cache_interval()`] and [`set_cache_interval()`] methods.
/// By default, the cache interval value is set to `None`. See the [`parser`]
/// documentation for more information about caching.
///
/// ## Creation
///
/// Can be created using the [`Builder::new()`] method.
///
/// ## Build
///
/// After setting all the necessary fields, call the [`build()`] method to get
/// a [`parser`] instance.
///
/// ---
///
/// See the [`parser`] documentation for more information.
///
/// [`parser`]: crate::Parser
/// [`with_cache_interval()`]: crate::parser::Builder::with_cache_interval()
/// [`set_cache_interval()`]: crate::parser::Builder::set_cache_interval()
/// [`Builder::new()`]: crate::parser::Builder::new()
/// [`build()`]: Builder::build()
#[derive(Debug, Clone)]
pub struct Builder {
    source: Arc<Cow<'static, str>>,
    cache_interval: Option<usize>,
}

impl Builder {
    /// See the [`parser`] documentation for more information about caching.
    ///
    /// [`parser`]: crate::Parser
    #[must_use = "method returns the modified value"]
    pub fn with_cache_interval(
        mut self,
        value: impl Into<Option<usize>>,
    ) -> Self {
        self.cache_interval = value.into();
        self
    }
}

impl Builder {
    /// See the [`parser`] documentation for more information about caching.
    ///
    /// [`parser`]: crate::Parser
    pub fn set_cache_interval(
        &mut self,
        value: impl Into<Option<usize>>,
    ) -> &mut Self {
        self.cache_interval = value.into();
        self
    }
}

impl Builder {
    /// # Creates a new [`builder`] over the `source`
    ///
    /// By default, the cache interval value is set to `None`.
    ///
    /// ---
    ///
    /// See the [`builder`] documentation for more information.
    ///
    /// [`builder`]: crate::parser::Builder
    pub fn new(source: impl Into<Cow<'static, str>>) -> Self {
        Self {
            source: Arc::new(source.into()),
            cache_interval: None,
        }
    }

    /// # Builds a configured [`parser`]
    ///
    /// Consumes `self` and moves all configuration fields into the resulting
    /// structure.
    ///
    /// Returns the [`a_bc::error::Lexer::SourceIsEmpty`] error if the provided
    /// `source` is empty.
    ///
    /// ---
    ///
    /// See the [`builder`] documentation for more information.
    ///
    /// [`builder`]: crate::parser::Builder
    /// [`parser`]: crate::Parser
    pub fn build(self) -> Result<Parser, Error<JSON>> {
        let source = self.source;

        if source.is_empty() {
            return Err(error::Lexer::SourceIsEmpty.into());
        }

        let cache_interval = self
            .cache_interval
            .map(|interval| if interval == 0 { 1 } else { interval });

        let range = 0..source.len();

        let kind =
            match source.as_bytes().iter().find(|c| !c.is_ascii_whitespace()) {
                Some(b'{') => JSON::Object,
                Some(b'[') => JSON::Array,
                Some(b'"') => JSON::String,
                Some(_) => JSON::Sequence,
                None => {
                    return Err(error::Token::ExpectedButGot {
                        expected: [
                            JSON::Object,
                            JSON::Array,
                            JSON::String,
                            JSON::Sequence,
                        ]
                        .into(),
                        got: None,
                        location: (
                            error::Relation::After,
                            error::row_col_pos(source.as_bytes()),
                        ),
                    }
                    .into());
                }
            };

        let flag = a_bc::cancellation::Flag::new();

        let bundle = Bundle::new(
            Content::Slice(range.clone()),
            kind,
            flag.clone(),
            source,
            range,
            cache_interval,
        );

        let tree = lazy_tree::Tree::new(lazy_tree::node::Builder::new(
            bundle, Callbacks,
        ));

        Ok(Parser { tree, flag })
    }
}

mod count {
    use std::borrow::Cow;

    use a_bc::{Error, Token, error};

    use crate::{Bundle, JSON, lexer, parser::YIELD_INTERVAL};

    pub async fn object(
        parent: &mut Bundle,
        token: Token<JSON>,
    ) -> Result<usize, Error<JSON>> {
        let mut lexer =
            lexer::Builder::new(Cow::Borrowed(parent.source.as_bytes()))
                .with_flag(parent.flag.clone())
                .with_range(token.inner_range().unwrap())
                .build()?;

        let mut checkpoints = Vec::default();

        let mut counter: usize = 0;

        loop {
            if parent
                .cache
                .as_ref()
                .is_some_and(|cache| counter.is_multiple_of(cache.interval))
            {
                checkpoints.push(*lexer.cursor().position());
            }

            match lexer.expect(&[JSON::String]) {
                Ok(key_token) => key_token,
                Err(e) => {
                    if let Error::Token(error::Token::ExpectedButGot {
                        got,
                        ..
                    }) = e
                        && got.is_none()
                        && counter == 0
                    {
                        break;
                    } else {
                        return Err(e);
                    }
                }
            };

            lexer.expect(&[JSON::Colon])?;

            lexer.expect(&[
                JSON::String,
                JSON::Sequence,
                JSON::Object,
                JSON::Array,
            ])?;

            counter += 1;

            if let Err(e) = lexer.expect(&[JSON::Comma]) {
                if let Error::Token(error::Token::ExpectedButGot {
                    got, ..
                }) = e
                    && got.is_none()
                {
                    break;
                } else {
                    return Err(e);
                }
            }

            if counter.is_multiple_of(YIELD_INTERVAL) {
                tokio::task::yield_now().await;
            }
        }

        if let Some(cache) = &mut parent.cache {
            cache.checkpoints = Some(checkpoints.into_boxed_slice());
        }

        Ok(counter)
    }

    pub async fn array(
        parent: &mut Bundle,
        token: Token<JSON>,
    ) -> Result<usize, Error<JSON>> {
        let mut lexer =
            lexer::Builder::new(Cow::Borrowed(parent.source.as_bytes()))
                .with_flag(parent.flag.clone())
                .with_range(token.inner_range().unwrap())
                .build()?;

        let mut checkpoints = Vec::default();

        let mut counter: usize = 0;

        loop {
            if parent
                .cache
                .as_ref()
                .is_some_and(|cache| counter.is_multiple_of(cache.interval))
            {
                checkpoints.push(*lexer.cursor().position());
            }

            match lexer.not_expect(&[JSON::Comma, JSON::Colon]) {
                Ok(token) => token,
                Err(e) => {
                    if let Error::Token(error::Token::NotExpectedButGot {
                        got,
                        ..
                    }) = e
                        && got.is_none()
                        && counter == 0
                    {
                        break;
                    } else {
                        return Err(e);
                    }
                }
            };

            counter += 1;

            if let Err(e) = lexer.expect(&[JSON::Comma]) {
                if let Error::Token(error::Token::ExpectedButGot {
                    got, ..
                }) = e
                    && got.is_none()
                {
                    break;
                } else {
                    return Err(e);
                }
            }

            if counter.is_multiple_of(YIELD_INTERVAL) {
                tokio::task::yield_now().await;
            }
        }

        if let Some(cache) = &mut parent.cache {
            cache.checkpoints = Some(checkpoints.into_boxed_slice());
        }

        Ok(counter)
    }
}

mod query {
    use std::borrow::Cow;

    use a_bc::{Error, Token};
    use lazy_tree::node;

    use crate::{
        Bundle, JSON, lexer,
        parser::{Cache, Content, YIELD_INTERVAL},
    };

    pub async fn object(
        parent: &Bundle,
        token: Token<JSON>,
        index: usize,
    ) -> Result<node::Builder<Bundle, Error<JSON>>, Error<JSON>> {
        let (position, mut counter) = match &parent.cache {
            Some(Cache {
                checkpoints,
                interval,
            }) if let Some(checkpoints) = checkpoints
                && !checkpoints.is_empty() =>
            {
                let position = checkpoints.get(index / interval).unwrap();
                let counter = (index / interval) * interval;

                (Some(*position), counter)
            }
            _ => (None, 0),
        };

        let mut lexer =
            lexer::Builder::new(Cow::Borrowed(parent.source.as_bytes()))
                .with_position(position)
                .with_flag(parent.flag.clone())
                .with_range(token.inner_range().unwrap())
                .build()?;

        loop {
            let name = lexer.expect(&[JSON::String])?;

            lexer.expect(&[JSON::Colon])?;

            let value = lexer.expect(&[
                JSON::String,
                JSON::Sequence,
                JSON::Object,
                JSON::Array,
            ])?;

            if index == counter {
                let bundle = Bundle::new(
                    Content::Slice(name.inner_range().unwrap()),
                    *value.kind(),
                    parent.flag.clone(),
                    parent.source.clone(),
                    value.range().clone(),
                    parent.cache.as_ref().map(|cache| cache.interval),
                );

                return Ok(node::Builder::new(bundle, super::Callbacks));
            }

            counter += 1;

            lexer.expect(&[JSON::Comma])?;

            if counter.is_multiple_of(YIELD_INTERVAL) {
                tokio::task::yield_now().await;
            }
        }
    }

    pub async fn array(
        parent: &Bundle,
        token: Token<JSON>,
        index: usize,
    ) -> Result<node::Builder<Bundle, Error<JSON>>, Error<JSON>> {
        let (position, mut counter) = match &parent.cache {
            Some(Cache {
                checkpoints,
                interval,
            }) if let Some(checkpoints) = checkpoints
                && !checkpoints.is_empty() =>
            {
                let position = checkpoints.get(index / interval).unwrap();
                let counter = (index / interval) * interval;

                (Some(*position), counter)
            }
            _ => (None, 0),
        };

        let mut lexer =
            lexer::Builder::new(Cow::Borrowed(parent.source.as_bytes()))
                .with_position(position)
                .with_flag(parent.flag.clone())
                .with_range(token.inner_range().unwrap())
                .build()?;

        loop {
            let value = lexer.not_expect(&[JSON::Comma, JSON::Colon])?;

            if index == counter {
                let (content, range) = match *value.kind() {
                    JSON::Object => {
                        (Content::Object, Some(value.range().clone()))
                    }
                    JSON::Array => {
                        (Content::Array, Some(value.range().clone()))
                    }
                    JSON::String => {
                        (Content::Slice(value.inner_range().unwrap()), None)
                    }
                    JSON::Sequence => {
                        (Content::Slice(value.range().clone()), None)
                    }
                    _ => unreachable!(),
                };

                let bundle = Bundle::new(
                    content,
                    *value.kind(),
                    parent.flag.clone(),
                    parent.source.clone(),
                    range,
                    parent.cache.as_ref().map(|cache| cache.interval),
                );

                return Ok(node::Builder::new(bundle, super::Callbacks));
            }

            counter += 1;

            lexer.expect(&[JSON::Comma])?;

            if counter.is_multiple_of(YIELD_INTERVAL) {
                tokio::task::yield_now().await;
            }
        }
    }

    pub fn string(
        parent: &Bundle,
        token: Token<JSON>,
    ) -> Result<node::Builder<Bundle, Error<JSON>>, Error<JSON>> {
        let bundle = Bundle::new(
            Content::Slice(token.inner_range().unwrap()),
            JSON::String,
            parent.flag.clone(),
            parent.source.clone(),
            None,
            parent.cache.as_ref().map(|cache| cache.interval),
        );

        Ok(node::Builder::new(bundle, super::Callbacks))
    }

    pub fn sequence(
        parent: &Bundle,
        token: Token<JSON>,
    ) -> Result<node::Builder<Bundle, Error<JSON>>, Error<JSON>> {
        let bundle = Bundle::new(
            Content::Slice(token.range().clone()),
            JSON::Sequence,
            parent.flag.clone(),
            parent.source.clone(),
            None,
            parent.cache.as_ref().map(|cache| cache.interval),
        );

        Ok(node::Builder::new(bundle, super::Callbacks))
    }
}
