use std::{sync::Arc, thread};

use pretty_assertions::assert_eq;
use serial_test::serial;
use tokio::runtime::Runtime;

use a_bc::error::{self, Error, Position, Relation};
use lazy_tree::cursor::{Direction, Target};

use crate::{lexer::JSON, parser::Builder};

macro_rules! runtime_test {
    ($body:ident) => {
        mod $body {
            use super::*;

            #[tokio::test(flavor = "current_thread")]
            #[serial]
            async fn tokio_current() {
                $body().await
            }

            #[tokio::test(flavor = "multi_thread")]
            #[serial]
            async fn tokio_multi() {
                $body().await
            }

            #[test]
            #[serial]
            fn futures() {
                futures::executor::block_on($body())
            }

            #[test]
            #[serial]
            fn futures_lite() {
                futures_lite::future::block_on($body())
            }

            #[test]
            #[serial]
            fn smol() {
                smol::block_on($body())
            }
        }
    };
}

#[test]
#[serial]
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

    let parser = {
        let rt = Runtime::new().unwrap();
        rt.block_on(async { Arc::new(Builder::new(source).build().unwrap()) })
    };

    thread::scope(|scope| {
        scope.spawn({
            let parser = parser.clone();
            move || {
                parser.stop();
            }
        });

        scope.spawn({
            let parser = parser.clone();
            move || {
                let rt = Runtime::new().unwrap();
                rt.block_on(async {
                    assert!(parser.cursor().await.is_err_and(|e| e == error::Lexer::Cancelled.into()));
                });
            }
        });
    });
}

runtime_test!(empty_source);
async fn empty_source() {
    let source = "";
    assert!(
        Builder::new(source)
            .build()
            .is_err_and(|e| { e == error::Lexer::SourceIsEmpty.into() })
    );
}

runtime_test!(whitespace_source);
async fn whitespace_source() {
    let source = " \n \t ";
    assert!(Builder::new(source).build().is_err_and(|e| {
        e == error::Token::ExpectedButGot {
            expected: [JSON::Object, JSON::Array, JSON::String, JSON::Sequence]
                .into(),
            got: None,
            location: (Relation::After, Position::new(2, 3)),
        }
        .into()
    }));
}

runtime_test!(empty_string);
async fn empty_string() {
    {
        let source = " \n \t \"\" \n \t ";
        let parser = Builder::new(source).build().unwrap();
        let mut cursor = parser.cursor().await.unwrap();
        assert_eq!(*cursor.kind(), JSON::String);
        assert_eq!(cursor.content(), source);

        assert!(cursor.walk(Direction::Down(Target::First)).await.unwrap());
        assert_eq!(*cursor.kind(), JSON::String);
        assert_eq!(cursor.content(), "");

        assert!(cursor.walk(Direction::Up).await.unwrap());
        assert_eq!(*cursor.kind(), JSON::String);
        assert_eq!(cursor.content(), source);
    }

    {
        let source = r#""""#;
        let parser = Builder::new(source).build().unwrap();
        let mut cursor = parser.cursor().await.unwrap();
        assert_eq!(*cursor.kind(), JSON::String);
        assert_eq!(cursor.content(), source);

        assert!(cursor.walk(Direction::Down(Target::First)).await.unwrap());
        assert_eq!(*cursor.kind(), JSON::String);
        assert_eq!(cursor.content(), "");

        assert!(cursor.walk(Direction::Up).await.unwrap());
        assert_eq!(*cursor.kind(), JSON::String);
        assert_eq!(cursor.content(), source);
    }
}

runtime_test!(valid_string);
async fn valid_string() {
    {
        let source = " \n \t \"foo\" \n \t ";
        let parser = Builder::new(source).build().unwrap();
        let mut cursor = parser.cursor().await.unwrap();
        assert_eq!(*cursor.kind(), JSON::String);
        assert_eq!(cursor.content(), source);

        assert!(cursor.walk(Direction::Down(Target::First)).await.unwrap());
        assert_eq!(*cursor.kind(), JSON::String);
        assert_eq!(cursor.content(), "foo");

        assert!(!cursor.walk(Direction::Down(Target::First)).await.unwrap());

        assert!(cursor.walk(Direction::Up).await.unwrap());
        assert_eq!(*cursor.kind(), JSON::String);
        assert_eq!(cursor.content(), source);
    }

    {
        let source = r#""foo""#;
        let parser = Builder::new(source).build().unwrap();
        let mut cursor = parser.cursor().await.unwrap();
        assert_eq!(*cursor.kind(), JSON::String);
        assert_eq!(cursor.content(), source);

        assert!(cursor.walk(Direction::Down(Target::First)).await.unwrap());
        assert_eq!(*cursor.kind(), JSON::String);
        assert_eq!(cursor.content(), "foo");

        assert!(!cursor.walk(Direction::Down(Target::First)).await.unwrap());

        assert!(cursor.walk(Direction::Up).await.unwrap());
        assert_eq!(*cursor.kind(), JSON::String);
        assert_eq!(cursor.content(), source);
    }

    {
        let source = r#""   ""#;
        let parser = Builder::new(source).build().unwrap();
        let mut cursor = parser.cursor().await.unwrap();
        assert_eq!(*cursor.kind(), JSON::String);
        assert_eq!(cursor.content(), source);

        assert!(cursor.walk(Direction::Down(Target::First)).await.unwrap());
        assert_eq!(*cursor.kind(), JSON::String);
        assert_eq!(cursor.content(), "   ");

        assert!(!cursor.walk(Direction::Down(Target::First)).await.unwrap());

        assert!(cursor.walk(Direction::Up).await.unwrap());
        assert_eq!(*cursor.kind(), JSON::String);
        assert_eq!(cursor.content(), source);
    }

    {
        let source = r#"" foo bar ""#;
        let parser = Builder::new(source).build().unwrap();
        let mut cursor = parser.cursor().await.unwrap();
        assert_eq!(*cursor.kind(), JSON::String);
        assert_eq!(cursor.content(), source);

        assert!(cursor.walk(Direction::Down(Target::First)).await.unwrap());
        assert_eq!(*cursor.kind(), JSON::String);
        assert_eq!(cursor.content(), " foo bar ");

        assert!(!cursor.walk(Direction::Down(Target::First)).await.unwrap());

        assert!(cursor.walk(Direction::Up).await.unwrap());
        assert_eq!(*cursor.kind(), JSON::String);
        assert_eq!(cursor.content(), source);
    }

    {
        let source = r#""foo \" bar""#;
        let parser = Builder::new(source).build().unwrap();
        let mut cursor = parser.cursor().await.unwrap();
        assert_eq!(*cursor.kind(), JSON::String);
        assert_eq!(cursor.content(), source);

        assert!(cursor.walk(Direction::Down(Target::First)).await.unwrap());
        assert_eq!(*cursor.kind(), JSON::String);
        assert_eq!(cursor.content(), r#"foo \" bar"#);

        assert!(!cursor.walk(Direction::Down(Target::First)).await.unwrap());

        assert!(cursor.walk(Direction::Up).await.unwrap());
        assert_eq!(*cursor.kind(), JSON::String);
        assert_eq!(cursor.content(), source);
    }

    {
        let source = r#""foo \"  \" bar""#;
        let parser = Builder::new(source).build().unwrap();
        let mut cursor = parser.cursor().await.unwrap();
        assert_eq!(*cursor.kind(), JSON::String);
        assert_eq!(cursor.content(), source);

        assert!(cursor.walk(Direction::Down(Target::First)).await.unwrap());
        assert_eq!(*cursor.kind(), JSON::String);
        assert_eq!(cursor.content(), r#"foo \"  \" bar"#);

        assert!(!cursor.walk(Direction::Down(Target::First)).await.unwrap());

        assert!(cursor.walk(Direction::Up).await.unwrap());
        assert_eq!(*cursor.kind(), JSON::String);
        assert_eq!(cursor.content(), source);
    }

    {
        let source = r#""foo { bar""#;
        let parser = Builder::new(source).build().unwrap();
        let mut cursor = parser.cursor().await.unwrap();
        assert_eq!(*cursor.kind(), JSON::String);
        assert_eq!(cursor.content(), source);

        assert!(cursor.walk(Direction::Down(Target::First)).await.unwrap());
        assert_eq!(*cursor.kind(), JSON::String);
        assert_eq!(cursor.content(), r#"foo { bar"#);

        assert!(!cursor.walk(Direction::Down(Target::First)).await.unwrap());

        assert!(cursor.walk(Direction::Up).await.unwrap());
        assert_eq!(*cursor.kind(), JSON::String);
        assert_eq!(cursor.content(), source);
    }

    {
        let source = r#""foo } bar""#;
        let parser = Builder::new(source).build().unwrap();
        let mut cursor = parser.cursor().await.unwrap();
        assert_eq!(*cursor.kind(), JSON::String);
        assert_eq!(cursor.content(), source);

        assert!(cursor.walk(Direction::Down(Target::First)).await.unwrap());
        assert_eq!(*cursor.kind(), JSON::String);
        assert_eq!(cursor.content(), r#"foo } bar"#);

        assert!(!cursor.walk(Direction::Down(Target::First)).await.unwrap());

        assert!(cursor.walk(Direction::Up).await.unwrap());
        assert_eq!(*cursor.kind(), JSON::String);
        assert_eq!(cursor.content(), source);
    }

    {
        let source = r#""foo {} bar""#;
        let parser = Builder::new(source).build().unwrap();
        let mut cursor = parser.cursor().await.unwrap();
        assert_eq!(*cursor.kind(), JSON::String);
        assert_eq!(cursor.content(), source);

        assert!(cursor.walk(Direction::Down(Target::First)).await.unwrap());
        assert_eq!(*cursor.kind(), JSON::String);
        assert_eq!(cursor.content(), r#"foo {} bar"#);

        assert!(!cursor.walk(Direction::Down(Target::First)).await.unwrap());

        assert!(cursor.walk(Direction::Up).await.unwrap());
        assert_eq!(*cursor.kind(), JSON::String);
        assert_eq!(cursor.content(), source);
    }

    {
        let source = r#""foo [ bar""#;
        let parser = Builder::new(source).build().unwrap();
        let mut cursor = parser.cursor().await.unwrap();
        assert_eq!(*cursor.kind(), JSON::String);
        assert_eq!(cursor.content(), source);

        assert!(cursor.walk(Direction::Down(Target::First)).await.unwrap());
        assert_eq!(*cursor.kind(), JSON::String);
        assert_eq!(cursor.content(), r#"foo [ bar"#);

        assert!(!cursor.walk(Direction::Down(Target::First)).await.unwrap());

        assert!(cursor.walk(Direction::Up).await.unwrap());
        assert_eq!(*cursor.kind(), JSON::String);
        assert_eq!(cursor.content(), source);
    }

    {
        let source = r#""foo ] bar""#;
        let parser = Builder::new(source).build().unwrap();
        let mut cursor = parser.cursor().await.unwrap();
        assert_eq!(*cursor.kind(), JSON::String);
        assert_eq!(cursor.content(), source);

        assert!(cursor.walk(Direction::Down(Target::First)).await.unwrap());
        assert_eq!(*cursor.kind(), JSON::String);
        assert_eq!(cursor.content(), r#"foo ] bar"#);

        assert!(!cursor.walk(Direction::Down(Target::First)).await.unwrap());

        assert!(cursor.walk(Direction::Up).await.unwrap());
        assert_eq!(*cursor.kind(), JSON::String);
        assert_eq!(cursor.content(), source);
    }

    {
        let source = r#""foo [] bar""#;
        let parser = Builder::new(source).build().unwrap();
        let mut cursor = parser.cursor().await.unwrap();
        assert_eq!(*cursor.kind(), JSON::String);
        assert_eq!(cursor.content(), source);

        assert!(cursor.walk(Direction::Down(Target::First)).await.unwrap());
        assert_eq!(*cursor.kind(), JSON::String);
        assert_eq!(cursor.content(), r#"foo [] bar"#);

        assert!(!cursor.walk(Direction::Down(Target::First)).await.unwrap());

        assert!(cursor.walk(Direction::Up).await.unwrap());
        assert_eq!(*cursor.kind(), JSON::String);
        assert_eq!(cursor.content(), source);
    }

    {
        let source = r#""foo : bar""#;
        let parser = Builder::new(source).build().unwrap();
        let mut cursor = parser.cursor().await.unwrap();
        assert_eq!(*cursor.kind(), JSON::String);
        assert_eq!(cursor.content(), source);

        assert!(cursor.walk(Direction::Down(Target::First)).await.unwrap());
        assert_eq!(*cursor.kind(), JSON::String);
        assert_eq!(cursor.content(), r#"foo : bar"#);

        assert!(!cursor.walk(Direction::Down(Target::First)).await.unwrap());

        assert!(cursor.walk(Direction::Up).await.unwrap());
        assert_eq!(*cursor.kind(), JSON::String);
        assert_eq!(cursor.content(), source);
    }
}

runtime_test!(invalid_string);
async fn invalid_string() {
    let source = r#" " "#;
    let parser = Builder::new(source).build().unwrap();

    assert!(parser.cursor().await.is_err_and(|e| {
        e == Error::Token(error::Token::PairNotFound {
            opening: "\"".into(),
            closing: "\"".into(),
            location: (Relation::After, (1, 2).into()),
        })
    }));
}

runtime_test!(sequence);
async fn sequence() {
    {
        let source = "asdfgh";
        let parser = Builder::new(source).build().unwrap();
        let mut cursor = parser.cursor().await.unwrap();
        assert_eq!(*cursor.kind(), JSON::Sequence);
        assert_eq!(cursor.content(), source);

        assert!(cursor.walk(Direction::Down(Target::First)).await.unwrap());
        assert_eq!(*cursor.kind(), JSON::Sequence);
        assert_eq!(cursor.content(), "asdfgh");

        assert!(cursor.walk(Direction::Up).await.unwrap());
        assert_eq!(*cursor.kind(), JSON::Sequence);
        assert_eq!(cursor.content(), source);
    }

    {
        let source = "123456";
        let parser = Builder::new(source).build().unwrap();
        let mut cursor = parser.cursor().await.unwrap();
        assert_eq!(*cursor.kind(), JSON::Sequence);
        assert_eq!(cursor.content(), source);

        assert!(cursor.walk(Direction::Down(Target::First)).await.unwrap());
        assert_eq!(*cursor.kind(), JSON::Sequence);
        assert_eq!(cursor.content(), "123456");

        assert!(cursor.walk(Direction::Up).await.unwrap());
        assert_eq!(*cursor.kind(), JSON::Sequence);
        assert_eq!(cursor.content(), source);
    }

    {
        let source = "a1s2d3f4g5h6";
        let parser = Builder::new(source).build().unwrap();
        let mut cursor = parser.cursor().await.unwrap();
        assert_eq!(*cursor.kind(), JSON::Sequence);
        assert_eq!(cursor.content(), source);

        assert!(cursor.walk(Direction::Down(Target::First)).await.unwrap());
        assert_eq!(*cursor.kind(), JSON::Sequence);
        assert_eq!(cursor.content(), "a1s2d3f4g5h6");

        assert!(cursor.walk(Direction::Up).await.unwrap());
        assert_eq!(*cursor.kind(), JSON::Sequence);
        assert_eq!(cursor.content(), source);
    }

    {
        let source = r#"a{1s"2d[3f}4g"5h]6"#;
        let parser = Builder::new(source).build().unwrap();
        let mut cursor = parser.cursor().await.unwrap();
        assert_eq!(*cursor.kind(), JSON::Sequence);
        assert_eq!(cursor.content(), source);

        assert!(cursor.walk(Direction::Down(Target::First)).await.unwrap());
        assert_eq!(*cursor.kind(), JSON::Sequence);
        assert_eq!(cursor.content(), r#"a{1s"2d[3f}4g"5h]6"#);

        assert!(cursor.walk(Direction::Up).await.unwrap());
        assert_eq!(*cursor.kind(), JSON::Sequence);
        assert_eq!(cursor.content(), source);
    }
}

runtime_test!(empty_object);
async fn empty_object() {
    {
        let source = "{}";
        let parser = Builder::new(source).build().unwrap();
        let cursor = parser.cursor().await.unwrap();
        assert_eq!(*cursor.kind(), JSON::Object);
        assert_eq!(cursor.content(), source);
    }

    {
        let source = " \n \t {} \n \t";
        let parser = Builder::new(source).build().unwrap();
        let cursor = parser.cursor().await.unwrap();
        assert_eq!(*cursor.kind(), JSON::Object);
        assert_eq!(cursor.content(), source);
    }

    {
        let source = "{ \n \t }";
        let parser = Builder::new(source).build().unwrap();
        let cursor = parser.cursor().await.unwrap();
        assert_eq!(*cursor.kind(), JSON::Object);
        assert_eq!(cursor.content(), source);
    }

    {
        let source = " \n \t { \n \t } \n \t";
        let parser = Builder::new(source).build().unwrap();
        let cursor = parser.cursor().await.unwrap();
        assert_eq!(*cursor.kind(), JSON::Object);
        assert_eq!(cursor.content(), source);
    }
}

runtime_test!(valid_object);
async fn valid_object() {
    {
        let source =
            " \n \t { \n \t \"foo\" \n \t : \n \t \"bar\" \n \t } \n \t ";
        let parser = Builder::new(source).build().unwrap();
        let mut cursor = parser.cursor().await.unwrap();
        assert_eq!(*cursor.kind(), JSON::Object);
        assert_eq!(cursor.content(), source);

        assert!(cursor.walk(Direction::Down(Target::First)).await.unwrap());
        assert_eq!(*cursor.kind(), JSON::String);
        assert_eq!(cursor.content(), "foo");

        assert!(cursor.walk(Direction::Down(Target::First)).await.unwrap());
        assert_eq!(*cursor.kind(), JSON::String);
        assert_eq!(cursor.content(), "bar");

        assert!(cursor.walk(Direction::Up).await.unwrap());
        assert_eq!(*cursor.kind(), JSON::String);
        assert_eq!(cursor.content(), "foo");

        assert!(cursor.walk(Direction::Up).await.unwrap());
        assert_eq!(*cursor.kind(), JSON::Object);
        assert_eq!(cursor.content(), source);
    }

    {
        let source = r#"{"foo":"bar"}"#;
        let parser = Builder::new(source).build().unwrap();
        let mut cursor = parser.cursor().await.unwrap();
        assert_eq!(*cursor.kind(), JSON::Object);
        assert_eq!(cursor.content(), source);

        assert!(cursor.walk(Direction::Down(Target::First)).await.unwrap());
        assert_eq!(*cursor.kind(), JSON::String);
        assert_eq!(cursor.content(), "foo");

        assert!(cursor.walk(Direction::Down(Target::First)).await.unwrap());
        assert_eq!(*cursor.kind(), JSON::String);
        assert_eq!(cursor.content(), "bar");

        assert!(cursor.walk(Direction::Up).await.unwrap());
        assert_eq!(*cursor.kind(), JSON::String);
        assert_eq!(cursor.content(), "foo");

        assert!(cursor.walk(Direction::Up).await.unwrap());
        assert_eq!(*cursor.kind(), JSON::Object);
        assert_eq!(cursor.content(), source);
    }

    {
        let source = r#" { "foo": 42, "bar": "baz" } "#;
        let parser = Builder::new(source).build().unwrap();
        let mut cursor = parser.cursor().await.unwrap();
        assert_eq!(*cursor.kind(), JSON::Object);
        assert_eq!(cursor.content(), source);

        assert!(cursor.walk(Direction::Down(Target::First)).await.unwrap());
        assert_eq!(*cursor.kind(), JSON::Sequence);
        assert_eq!(cursor.content(), "foo");

        assert!(cursor.walk(Direction::Down(Target::First)).await.unwrap());
        assert_eq!(*cursor.kind(), JSON::Sequence);
        assert_eq!(cursor.content(), "42");

        assert!(cursor.walk(Direction::Up).await.unwrap());
        assert_eq!(*cursor.kind(), JSON::Sequence);
        assert_eq!(cursor.content(), "foo");

        assert!(cursor.walk(Direction::Up).await.unwrap());
        assert_eq!(*cursor.kind(), JSON::Object);
        assert_eq!(cursor.content(), source);
    }

    {
        let source = r#" { "foo": bar } "#;
        let parser = Builder::new(source).build().unwrap();
        let mut cursor = parser.cursor().await.unwrap();
        assert_eq!(*cursor.kind(), JSON::Object);
        assert_eq!(cursor.content(), source);

        assert!(cursor.walk(Direction::Down(Target::First)).await.unwrap());
        assert_eq!(*cursor.kind(), JSON::Sequence);
        assert_eq!(cursor.content(), "foo");

        assert!(cursor.walk(Direction::Down(Target::First)).await.unwrap());
        assert_eq!(*cursor.kind(), JSON::Sequence);
        assert_eq!(cursor.content(), "bar");

        assert!(cursor.walk(Direction::Up).await.unwrap());
        assert_eq!(*cursor.kind(), JSON::Sequence);
        assert_eq!(cursor.content(), "foo");

        assert!(cursor.walk(Direction::Up).await.unwrap());
        assert_eq!(*cursor.kind(), JSON::Object);
        assert_eq!(cursor.content(), source);
    }

    {
        let source = r#" { "foo": { not parsed yet } } "#;
        let parser = Builder::new(source).build().unwrap();
        let mut cursor = parser.cursor().await.unwrap();
        assert_eq!(*cursor.kind(), JSON::Object);
        assert_eq!(cursor.content(), source);

        assert!(
            cursor
                .walk(Direction::Down(Target::First))
                .await
                .is_err_and(|e| {
                    e == error::Token::ExpectedButGot {
                        expected: [JSON::String].into(),
                        got: Some(JSON::Sequence),
                        location: (Relation::At, Position::new(1, 13)),
                    }
                    .into()
                })
        );
    }

    {
        let source = r#" { "foo": [ not parsed yet ] } "#;
        let parser = Builder::new(source).build().unwrap();
        let mut cursor = parser.cursor().await.unwrap();
        assert_eq!(*cursor.kind(), JSON::Object);
        assert_eq!(cursor.content(), source);

        assert!(
            cursor
                .walk(Direction::Down(Target::First))
                .await
                .is_err_and(|e| {
                    e == error::Token::ExpectedButGot {
                        expected: [JSON::Comma].into(),
                        got: Some(JSON::Sequence),
                        location: (Relation::At, Position::new(1, 17)),
                    }
                    .into()
                })
        );
    }

    {
        let source = r#" { "": "" } "#;
        let parser = Builder::new(source).build().unwrap();
        let mut cursor = parser.cursor().await.unwrap();
        assert_eq!(*cursor.kind(), JSON::Object);
        assert_eq!(cursor.content(), source);

        assert!(cursor.walk(Direction::Down(Target::First)).await.unwrap());
        assert_eq!(*cursor.kind(), JSON::String);
        assert_eq!(cursor.content(), "");

        assert!(cursor.walk(Direction::Down(Target::First)).await.unwrap());
        assert_eq!(*cursor.kind(), JSON::String);
        assert_eq!(cursor.content(), "");

        assert!(cursor.walk(Direction::Up).await.unwrap());
        assert_eq!(*cursor.kind(), JSON::String);
        assert_eq!(cursor.content(), "");

        assert!(cursor.walk(Direction::Up).await.unwrap());
        assert_eq!(*cursor.kind(), JSON::Object);
        assert_eq!(cursor.content(), source);
    }
}

runtime_test!(invalid_object);
async fn invalid_object() {
    {
        let source = r#" { "foo": "bar"  "#;
        let parser = Builder::new(source).build().unwrap();

        assert!(parser.cursor().await.is_err_and(|e| {
            e == Error::Token(error::Token::PairNotFound {
                opening: "{".into(),
                closing: "}".into(),
                location: (Relation::After, (1, 2).into()),
            })
        }));
    }

    {
        let source = r#" { "foo": "bar", } "#;
        let parser = Builder::new(source).build().unwrap();

        assert!(parser.cursor().await.is_err_and(|e| {
            e == Error::Token(error::Token::ExpectedButGot {
                expected: [JSON::String].into(),
                got: None,
                location: (Relation::After, (1, 17).into()),
            })
        }));
    }

    {
        let source = r#" { foo: "bar" } "#;
        let parser = Builder::new(source).build().unwrap();

        assert!(parser.cursor().await.is_err_and(|e| {
            e == Error::Token(error::Token::ExpectedButGot {
                expected: [JSON::String].into(),
                got: Some(JSON::Sequence),
                location: (Relation::At, (1, 4).into()),
            })
        }));
    }

    {
        let source = r#" { "foo" } "#;
        let parser = Builder::new(source).build().unwrap();

        assert!(parser.cursor().await.is_err_and(|e| {
            e == Error::Token(error::Token::ExpectedButGot {
                expected: [JSON::Colon].into(),
                got: None,
                location: (Relation::After, (1, 9).into()),
            })
        }));
    }

    {
        let source = r#" { foo } "#;
        let parser = Builder::new(source).build().unwrap();

        assert!(parser.cursor().await.is_err_and(|e| {
            e == Error::Token(error::Token::ExpectedButGot {
                expected: [JSON::String].into(),
                got: Some(JSON::Sequence),
                location: (Relation::At, (1, 4).into()),
            })
        }));
    }

    {
        let source = r#" { "foo" "bar" } "#;
        let parser = Builder::new(source).build().unwrap();

        assert!(parser.cursor().await.is_err_and(|e| {
            e == Error::Token(error::Token::ExpectedButGot {
                expected: [JSON::Colon].into(),
                got: Some(JSON::String),
                location: (Relation::At, (1, 10).into()),
            })
        }));
    }
}

runtime_test!(empty_array);
async fn empty_array() {
    {
        let source = "[]";
        let parser = Builder::new(source).build().unwrap();
        let cursor = parser.cursor().await.unwrap();
        assert_eq!(*cursor.kind(), JSON::Array);
        assert_eq!(cursor.content(), source);
    }

    {
        let source = " \n \t [] \n \t";
        let parser = Builder::new(source).build().unwrap();
        let cursor = parser.cursor().await.unwrap();
        assert_eq!(*cursor.kind(), JSON::Array);
        assert_eq!(cursor.content(), source);
    }

    {
        let source = "[ \n \t ]";
        let parser = Builder::new(source).build().unwrap();
        let cursor = parser.cursor().await.unwrap();
        assert_eq!(*cursor.kind(), JSON::Array);
        assert_eq!(cursor.content(), source);
    }

    {
        let source = " \n \t [ \n \t ] \n \t ";
        let parser = Builder::new(source).build().unwrap();
        let cursor = parser.cursor().await.unwrap();
        assert_eq!(*cursor.kind(), JSON::Array);
        assert_eq!(cursor.content(), source);
    }
}

runtime_test!(valid_array);
async fn valid_array() {
    {
        let source = " \n \t [ \n \t \"foo\" \n \t , \n \t bar \n \t , \n \t [ \n \t ] \n \t , \n \t { \n \t } \n \t ]";
        let parser = Builder::new(source).build().unwrap();
        let mut cursor = parser.cursor().await.unwrap();
        assert_eq!(*cursor.kind(), JSON::Array);
        assert_eq!(cursor.content(), source);

        assert!(cursor.walk(Direction::Down(Target::First)).await.unwrap());
        assert_eq!(*cursor.kind(), JSON::String);
        assert_eq!(cursor.content(), "foo");

        assert!(cursor.walk(Direction::Up).await.unwrap());
        assert_eq!(*cursor.kind(), JSON::Array);
        assert_eq!(cursor.content(), source);
    }

    {
        let source = r#"["foo",bar,[],{}]"#;
        let parser = Builder::new(source).build().unwrap();
        let mut cursor = parser.cursor().await.unwrap();
        assert_eq!(*cursor.kind(), JSON::Array);
        assert_eq!(cursor.content(), source);

        assert!(cursor.walk(Direction::Down(Target::First)).await.unwrap());
        assert_eq!(*cursor.kind(), JSON::String);
        assert_eq!(cursor.content(), "foo");

        assert!(cursor.walk(Direction::Up).await.unwrap());
        assert_eq!(*cursor.kind(), JSON::Array);
        assert_eq!(cursor.content(), source);
    }

    {
        let source = r#" [ "foo", 42 ] "#;
        let parser = Builder::new(source).build().unwrap();
        let mut cursor = parser.cursor().await.unwrap();
        assert_eq!(*cursor.kind(), JSON::Array);
        assert_eq!(cursor.content(), source);

        assert!(cursor.walk(Direction::Down(Target::First)).await.unwrap());
        assert_eq!(*cursor.kind(), JSON::String);
        assert_eq!(cursor.content(), "foo");

        assert!(cursor.walk(Direction::Up).await.unwrap());
        assert_eq!(*cursor.kind(), JSON::Array);
        assert_eq!(cursor.content(), source);
    }

    {
        let source = r#" [ "foo", bar ] "#;
        let parser = Builder::new(source).build().unwrap();
        let mut cursor = parser.cursor().await.unwrap();
        assert_eq!(*cursor.kind(), JSON::Array);
        assert_eq!(cursor.content(), source);

        assert!(cursor.walk(Direction::Down(Target::First)).await.unwrap());
        assert_eq!(*cursor.kind(), JSON::String);
        assert_eq!(cursor.content(), "foo");

        assert!(cursor.walk(Direction::Up).await.unwrap());
        assert_eq!(*cursor.kind(), JSON::Array);
        assert_eq!(cursor.content(), source);
    }

    {
        let source = r#" [ "foo", { not parsed yet } ] "#;
        let parser = Builder::new(source).build().unwrap();
        let mut cursor = parser.cursor().await.unwrap();
        assert_eq!(*cursor.kind(), JSON::Array);
        assert_eq!(cursor.content(), source);

        assert!(cursor.walk(Direction::Down(Target::First)).await.unwrap());
        assert_eq!(*cursor.kind(), JSON::String);
        assert_eq!(cursor.content(), "foo");

        assert!(cursor.walk(Direction::Up).await.unwrap());
        assert_eq!(*cursor.kind(), JSON::Array);
        assert_eq!(cursor.content(), source);
    }

    {
        let source = r#" [ "foo", [ not parsed yet ] ] "#;
        let parser = Builder::new(source).build().unwrap();
        let mut cursor = parser.cursor().await.unwrap();
        assert_eq!(*cursor.kind(), JSON::Array);
        assert_eq!(cursor.content(), source);

        assert!(cursor.walk(Direction::Down(Target::First)).await.unwrap());
        assert_eq!(*cursor.kind(), JSON::String);
        assert_eq!(cursor.content(), "foo");

        assert!(cursor.walk(Direction::Up).await.unwrap());
        assert_eq!(*cursor.kind(), JSON::Array);
        assert_eq!(cursor.content(), source);
    }

    {
        let source = r#" [ [] ] "#;
        let parser = Builder::new(source).build().unwrap();
        let mut cursor = parser.cursor().await.unwrap();
        assert_eq!(*cursor.kind(), JSON::Array);
        assert_eq!(cursor.content(), source);

        assert!(cursor.walk(Direction::Down(Target::First)).await.unwrap());
        assert_eq!(*cursor.kind(), JSON::Array);
        assert_eq!(cursor.content(), "[...]");

        assert!(cursor.walk(Direction::Up).await.unwrap());
        assert_eq!(*cursor.kind(), JSON::Array);
        assert_eq!(cursor.content(), source);
    }
}

runtime_test!(invalid_array);
async fn invalid_array() {
    {
        let source = r#" [ "foo", bar "#;
        let parser = Builder::new(source).build().unwrap();

        assert!(parser.cursor().await.is_err_and(|e| {
            e == Error::Token(error::Token::PairNotFound {
                opening: "[".into(),
                closing: "]".into(),
                location: (Relation::After, (1, 2).into()),
            })
        }));
    }

    {
        let source = r#" [ "foo", bar, ] "#;
        let parser = Builder::new(source).build().unwrap();

        assert!(parser.cursor().await.is_err_and(|e| {
            e == Error::Token(error::Token::NotExpectedButGot {
                not_expected: [JSON::Comma, JSON::Colon].into(),
                got: None,
                location: (Relation::After, (1, 15).into()),
            })
        }));
    }

    {
        let source = r#" [ "foo" bar ] "#;
        let parser = Builder::new(source).build().unwrap();

        assert!(parser.cursor().await.is_err_and(|e| {
            e == Error::Token(error::Token::ExpectedButGot {
                expected: [JSON::Comma].into(),
                got: Some(JSON::Sequence),
                location: (Relation::At, (1, 10).into()),
            })
        }));
    }
}

runtime_test!(full_single_cursor);
async fn full_single_cursor() {
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

    let parser = Builder::new(source).build().unwrap();

    let mut cursor = parser.cursor().await.unwrap();
    assert_eq!(cursor.path(), &[0]);
    assert_eq!(*cursor.kind(), JSON::Object);
    assert_eq!(cursor.content(), source);

    assert!(cursor.go_to(&[0, 0]).await.unwrap());
    assert_eq!(cursor.path(), &[0, 0]);
    assert_eq!(*cursor.kind(), JSON::String);
    assert_eq!(cursor.content(), "first_name");

    assert!(cursor.go_to(&[0, 0, 0]).await.unwrap());
    assert_eq!(cursor.path(), &[0, 0, 0]);
    assert_eq!(*cursor.kind(), JSON::String);
    assert_eq!(cursor.content(), "John");

    assert!(!cursor.go_to(&[0, 0, 0, 0]).await.unwrap());

    assert!(cursor.go_to(&[0, 0]).await.unwrap());
    assert_eq!(cursor.path(), &[0, 0]);
    assert_eq!(*cursor.kind(), JSON::String);
    assert_eq!(cursor.content(), "first_name");

    assert!(cursor.go_to(&[0]).await.unwrap());
    assert_eq!(cursor.path(), &[0]);
    assert_eq!(*cursor.kind(), JSON::Object);
    assert_eq!(cursor.content(), source);

    assert!(cursor.go_to(&[0, 1]).await.unwrap());
    assert_eq!(cursor.path(), &[0, 1]);
    assert_eq!(*cursor.kind(), JSON::String);
    assert_eq!(cursor.content(), "last_name");

    assert!(cursor.go_to(&[0, 1, 0]).await.unwrap());
    assert_eq!(cursor.path(), &[0, 1, 0]);
    assert_eq!(*cursor.kind(), JSON::String);
    assert_eq!(cursor.content(), "Smith");

    assert!(!cursor.go_to(&[0, 1, 0, 0]).await.unwrap());

    assert!(cursor.go_to(&[0, 1]).await.unwrap());
    assert_eq!(cursor.path(), &[0, 1]);
    assert_eq!(*cursor.kind(), JSON::String);
    assert_eq!(cursor.content(), "last_name");

    assert!(cursor.go_to(&[0]).await.unwrap());
    assert_eq!(cursor.path(), &[0]);
    assert_eq!(*cursor.kind(), JSON::Object);
    assert_eq!(cursor.content(), source);

    assert!(cursor.go_to(&[0, 2]).await.unwrap());
    assert_eq!(cursor.path(), &[0, 2]);
    assert_eq!(*cursor.kind(), JSON::Sequence);
    assert_eq!(cursor.content(), "is_alive");

    assert!(cursor.go_to(&[0, 2, 0]).await.unwrap());
    assert_eq!(cursor.path(), &[0, 2, 0]);
    assert_eq!(*cursor.kind(), JSON::Sequence);
    assert_eq!(cursor.content(), "true");

    assert!(!cursor.go_to(&[0, 2, 0, 0]).await.unwrap());

    assert!(cursor.go_to(&[0, 2]).await.unwrap());
    assert_eq!(cursor.path(), &[0, 2]);
    assert_eq!(*cursor.kind(), JSON::Sequence);
    assert_eq!(cursor.content(), "is_alive");

    assert!(cursor.go_to(&[0]).await.unwrap());
    assert_eq!(cursor.path(), &[0]);
    assert_eq!(*cursor.kind(), JSON::Object);
    assert_eq!(cursor.content(), source);

    assert!(cursor.go_to(&[0, 3]).await.unwrap());
    assert_eq!(cursor.path(), &[0, 3]);
    assert_eq!(*cursor.kind(), JSON::Sequence);
    assert_eq!(cursor.content(), "age");

    assert!(cursor.go_to(&[0, 3, 0]).await.unwrap());
    assert_eq!(cursor.path(), &[0, 3, 0]);
    assert_eq!(*cursor.kind(), JSON::Sequence);
    assert_eq!(cursor.content(), "27");

    assert!(!cursor.go_to(&[0, 3, 0, 0]).await.unwrap());

    assert!(cursor.go_to(&[0, 3]).await.unwrap());
    assert_eq!(cursor.path(), &[0, 3]);
    assert_eq!(*cursor.kind(), JSON::Sequence);
    assert_eq!(cursor.content(), "age");

    assert!(cursor.go_to(&[0]).await.unwrap());
    assert_eq!(cursor.path(), &[0]);
    assert_eq!(*cursor.kind(), JSON::Object);
    assert_eq!(cursor.content(), source);

    assert!(cursor.go_to(&[0, 4]).await.unwrap());
    assert_eq!(cursor.path(), &[0, 4]);
    assert_eq!(*cursor.kind(), JSON::Object);
    assert_eq!(cursor.content(), "address");

    assert!(cursor.go_to(&[0, 4, 0]).await.unwrap());
    assert_eq!(cursor.path(), &[0, 4, 0]);
    assert_eq!(*cursor.kind(), JSON::String);
    assert_eq!(cursor.content(), "street_address");

    assert!(cursor.go_to(&[0, 4, 0, 0]).await.unwrap());
    assert_eq!(cursor.path(), &[0, 4, 0, 0]);
    assert_eq!(*cursor.kind(), JSON::String);
    assert_eq!(cursor.content(), "21 2nd Street");

    assert!(!cursor.go_to(&[0, 4, 0, 0, 0]).await.unwrap());

    assert!(cursor.go_to(&[0, 4, 0]).await.unwrap());
    assert_eq!(cursor.path(), &[0, 4, 0]);
    assert_eq!(*cursor.kind(), JSON::String);
    assert_eq!(cursor.content(), "street_address");

    assert!(cursor.go_to(&[0, 4]).await.unwrap());
    assert_eq!(cursor.path(), &[0, 4]);
    assert_eq!(*cursor.kind(), JSON::Object);
    assert_eq!(cursor.content(), "address");

    assert!(cursor.go_to(&[0, 4, 1]).await.unwrap());
    assert_eq!(cursor.path(), &[0, 4, 1]);
    assert_eq!(*cursor.kind(), JSON::String);
    assert_eq!(cursor.content(), "city");

    assert!(cursor.go_to(&[0, 4, 1, 0]).await.unwrap());
    assert_eq!(cursor.path(), &[0, 4, 1, 0]);
    assert_eq!(*cursor.kind(), JSON::String);
    assert_eq!(cursor.content(), "New York");

    assert!(!cursor.go_to(&[0, 4, 1, 0, 0]).await.unwrap());

    assert!(cursor.go_to(&[0, 4, 1]).await.unwrap());
    assert_eq!(cursor.path(), &[0, 4, 1]);
    assert_eq!(*cursor.kind(), JSON::String);
    assert_eq!(cursor.content(), "city");

    assert!(cursor.go_to(&[0, 4]).await.unwrap());
    assert_eq!(cursor.path(), &[0, 4]);
    assert_eq!(*cursor.kind(), JSON::Object);
    assert_eq!(cursor.content(), "address");

    assert!(cursor.go_to(&[0, 4, 2]).await.unwrap());
    assert_eq!(cursor.path(), &[0, 4, 2]);
    assert_eq!(*cursor.kind(), JSON::String);
    assert_eq!(cursor.content(), "state");

    assert!(cursor.go_to(&[0, 4, 2, 0]).await.unwrap());
    assert_eq!(cursor.path(), &[0, 4, 2, 0]);
    assert_eq!(*cursor.kind(), JSON::String);
    assert_eq!(cursor.content(), "NY");

    assert!(!cursor.go_to(&[0, 4, 2, 0, 0]).await.unwrap());

    assert!(cursor.go_to(&[0, 4, 2]).await.unwrap());
    assert_eq!(cursor.path(), &[0, 4, 2]);
    assert_eq!(*cursor.kind(), JSON::String);
    assert_eq!(cursor.content(), "state");

    assert!(cursor.go_to(&[0, 4]).await.unwrap());
    assert_eq!(cursor.path(), &[0, 4]);
    assert_eq!(*cursor.kind(), JSON::Object);
    assert_eq!(cursor.content(), "address");

    assert!(cursor.go_to(&[0, 4, 3]).await.unwrap());
    assert_eq!(cursor.path(), &[0, 4, 3]);
    assert_eq!(*cursor.kind(), JSON::String);
    assert_eq!(cursor.content(), "postal_code");

    assert!(cursor.go_to(&[0, 4, 3, 0]).await.unwrap());
    assert_eq!(cursor.path(), &[0, 4, 3, 0]);
    assert_eq!(*cursor.kind(), JSON::String);
    assert_eq!(cursor.content(), "10021-3100");

    assert!(!cursor.go_to(&[0, 4, 3, 0, 0]).await.unwrap());

    assert!(cursor.go_to(&[0, 4, 3]).await.unwrap());
    assert_eq!(cursor.path(), &[0, 4, 3]);
    assert_eq!(*cursor.kind(), JSON::String);
    assert_eq!(cursor.content(), "postal_code");

    assert!(cursor.go_to(&[0, 4]).await.unwrap());
    assert_eq!(cursor.path(), &[0, 4]);
    assert_eq!(*cursor.kind(), JSON::Object);
    assert_eq!(cursor.content(), "address");

    assert!(cursor.go_to(&[0]).await.unwrap());
    assert_eq!(cursor.path(), &[0]);
    assert_eq!(*cursor.kind(), JSON::Object);
    assert_eq!(cursor.content(), source);

    assert!(cursor.go_to(&[0, 5]).await.unwrap());
    assert_eq!(cursor.path(), &[0, 5]);
    assert_eq!(*cursor.kind(), JSON::Array);
    assert_eq!(cursor.content(), "phone_numbers");

    assert!(cursor.go_to(&[0, 5, 0]).await.unwrap());
    assert_eq!(cursor.path(), &[0, 5, 0]);
    assert_eq!(*cursor.kind(), JSON::Object);
    assert_eq!(cursor.content(), "{...}");

    assert!(cursor.go_to(&[0, 5, 0, 0]).await.unwrap());
    assert_eq!(cursor.path(), &[0, 5, 0, 0]);
    assert_eq!(*cursor.kind(), JSON::String);
    assert_eq!(cursor.content(), "type");

    assert!(cursor.go_to(&[0, 5, 0, 0, 0]).await.unwrap());
    assert_eq!(cursor.path(), &[0, 5, 0, 0, 0]);
    assert_eq!(*cursor.kind(), JSON::String);
    assert_eq!(cursor.content(), "home");

    assert!(!cursor.go_to(&[0, 5, 0, 0, 0, 0]).await.unwrap());

    assert!(cursor.go_to(&[0, 5, 0, 0]).await.unwrap());
    assert_eq!(cursor.path(), &[0, 5, 0, 0]);
    assert_eq!(*cursor.kind(), JSON::String);
    assert_eq!(cursor.content(), "type");

    assert!(cursor.go_to(&[0, 5, 0]).await.unwrap());
    assert_eq!(cursor.path(), &[0, 5, 0]);
    assert_eq!(*cursor.kind(), JSON::Object);
    assert_eq!(cursor.content(), "{...}");

    assert!(cursor.go_to(&[0, 5, 0, 1]).await.unwrap());
    assert_eq!(cursor.path(), &[0, 5, 0, 1]);
    assert_eq!(*cursor.kind(), JSON::String);
    assert_eq!(cursor.content(), "number");

    assert!(cursor.go_to(&[0, 5, 0, 1, 0]).await.unwrap());
    assert_eq!(cursor.path(), &[0, 5, 0, 1, 0]);
    assert_eq!(*cursor.kind(), JSON::String);
    assert_eq!(cursor.content(), "212 555-1234");

    assert!(!cursor.go_to(&[0, 5, 0, 1, 0, 0]).await.unwrap());

    assert!(cursor.go_to(&[0, 5, 0, 1]).await.unwrap());
    assert_eq!(cursor.path(), &[0, 5, 0, 1]);
    assert_eq!(*cursor.kind(), JSON::String);
    assert_eq!(cursor.content(), "number");

    assert!(cursor.go_to(&[0, 5, 0]).await.unwrap());
    assert_eq!(cursor.path(), &[0, 5, 0]);
    assert_eq!(*cursor.kind(), JSON::Object);
    assert_eq!(cursor.content(), "{...}");

    assert!(cursor.go_to(&[0, 5, 1]).await.unwrap());
    assert_eq!(cursor.path(), &[0, 5, 1]);
    assert_eq!(*cursor.kind(), JSON::Object);
    assert_eq!(cursor.content(), "{...}");

    assert!(cursor.go_to(&[0, 5, 1, 0]).await.unwrap());
    assert_eq!(cursor.path(), &[0, 5, 1, 0]);
    assert_eq!(*cursor.kind(), JSON::String);
    assert_eq!(cursor.content(), "type");

    assert!(cursor.go_to(&[0, 5, 1, 0, 0]).await.unwrap());
    assert_eq!(cursor.path(), &[0, 5, 1, 0, 0]);
    assert_eq!(*cursor.kind(), JSON::String);
    assert_eq!(cursor.content(), "office");

    assert!(!cursor.go_to(&[0, 5, 1, 0, 0, 0]).await.unwrap());

    assert!(cursor.go_to(&[0, 5, 1, 0]).await.unwrap());
    assert_eq!(cursor.path(), &[0, 5, 1, 0]);
    assert_eq!(*cursor.kind(), JSON::String);
    assert_eq!(cursor.content(), "type");

    assert!(cursor.go_to(&[0, 5, 1]).await.unwrap());
    assert_eq!(cursor.path(), &[0, 5, 1]);
    assert_eq!(*cursor.kind(), JSON::Object);
    assert_eq!(cursor.content(), "{...}");

    assert!(cursor.go_to(&[0, 5, 1, 1]).await.unwrap());
    assert_eq!(cursor.path(), &[0, 5, 1, 1]);
    assert_eq!(*cursor.kind(), JSON::String);
    assert_eq!(cursor.content(), "number");

    assert!(cursor.go_to(&[0, 5, 1, 1, 0]).await.unwrap());
    assert_eq!(cursor.path(), &[0, 5, 1, 1, 0]);
    assert_eq!(*cursor.kind(), JSON::String);
    assert_eq!(cursor.content(), "646 555-4567");

    assert!(!cursor.go_to(&[0, 5, 1, 1, 0, 0]).await.unwrap());

    assert!(cursor.go_to(&[0, 5, 1, 1]).await.unwrap());
    assert_eq!(cursor.path(), &[0, 5, 1, 1]);
    assert_eq!(*cursor.kind(), JSON::String);
    assert_eq!(cursor.content(), "number");

    assert!(cursor.go_to(&[0, 5, 1]).await.unwrap());
    assert_eq!(cursor.path(), &[0, 5, 1]);
    assert_eq!(*cursor.kind(), JSON::Object);
    assert_eq!(cursor.content(), "{...}");

    assert!(cursor.go_to(&[0, 5]).await.unwrap());
    assert_eq!(cursor.path(), &[0, 5]);
    assert_eq!(*cursor.kind(), JSON::Array);
    assert_eq!(cursor.content(), "phone_numbers");

    assert!(cursor.go_to(&[0]).await.unwrap());
    assert_eq!(cursor.path(), &[0]);
    assert_eq!(*cursor.kind(), JSON::Object);
    assert_eq!(cursor.content(), source);

    assert!(cursor.go_to(&[0, 6]).await.unwrap());
    assert_eq!(cursor.path(), &[0, 6]);
    assert_eq!(*cursor.kind(), JSON::Array);

    assert!(cursor.go_to(&[0, 6, 0]).await.unwrap());
    assert_eq!(cursor.path(), &[0, 6, 0]);
    assert_eq!(*cursor.kind(), JSON::String);
    assert_eq!(cursor.content(), "Catherine");

    assert!(!cursor.go_to(&[0, 6, 0, 0]).await.unwrap());

    assert!(cursor.go_to(&[0, 6]).await.unwrap());
    assert_eq!(cursor.path(), &[0, 6]);
    assert_eq!(*cursor.kind(), JSON::Array);

    assert!(cursor.go_to(&[0, 6, 1]).await.unwrap());
    assert_eq!(cursor.path(), &[0, 6, 1]);
    assert_eq!(*cursor.kind(), JSON::String);
    assert_eq!(cursor.content(), "Thomas");

    assert!(!cursor.go_to(&[0, 6, 1, 0]).await.unwrap());

    assert!(cursor.go_to(&[0, 6]).await.unwrap());
    assert_eq!(cursor.path(), &[0, 6]);
    assert_eq!(*cursor.kind(), JSON::Array);

    assert!(cursor.go_to(&[0, 6, 2]).await.unwrap());
    assert_eq!(cursor.path(), &[0, 6, 2]);
    assert_eq!(*cursor.kind(), JSON::String);
    assert_eq!(cursor.content(), "Trevor");

    assert!(!cursor.go_to(&[0, 6, 2, 0]).await.unwrap());

    assert!(cursor.go_to(&[0, 6]).await.unwrap());
    assert_eq!(cursor.path(), &[0, 6]);
    assert_eq!(*cursor.kind(), JSON::Array);

    assert!(cursor.go_to(&[0]).await.unwrap());
    assert_eq!(cursor.path(), &[0]);
    assert_eq!(*cursor.kind(), JSON::Object);
    assert_eq!(cursor.content(), source);

    assert!(cursor.go_to(&[0, 7]).await.unwrap());
    assert_eq!(cursor.path(), &[0, 7]);
    assert_eq!(*cursor.kind(), JSON::Sequence);
    assert_eq!(cursor.content(), "spouse");

    assert!(cursor.go_to(&[0, 7, 0]).await.unwrap());
    assert_eq!(cursor.path(), &[0, 7, 0]);
    assert_eq!(*cursor.kind(), JSON::Sequence);
    assert_eq!(cursor.content(), "null");

    assert!(!cursor.go_to(&[0, 7, 0, 0]).await.unwrap());

    assert!(cursor.go_to(&[0, 7]).await.unwrap());
    assert_eq!(cursor.path(), &[0, 7]);
    assert_eq!(*cursor.kind(), JSON::Sequence);
    assert_eq!(cursor.content(), "spouse");

    assert!(cursor.go_to(&[0, 7, 0]).await.unwrap());
    assert_eq!(cursor.path(), &[0, 7, 0]);
    assert_eq!(*cursor.kind(), JSON::Sequence);
    assert_eq!(cursor.content(), "null");

    assert!(cursor.jump(Direction::Right).await.unwrap());
    assert_eq!(cursor.path(), &[0, 0, 0]);
    assert_eq!(*cursor.kind(), JSON::String);
    assert_eq!(cursor.content(), "John");

    assert!(cursor.jump(Direction::Down(Target::First)).await.unwrap());
    assert_eq!(cursor.path(), &[0]);
    assert_eq!(*cursor.kind(), JSON::Object);
    assert_eq!(cursor.content(), source);

    drop(cursor);

    let mut cursor = parser.cursor().await.unwrap();
    assert_eq!(cursor.path(), &[0]);
    assert_eq!(*cursor.kind(), JSON::Object);
    assert_eq!(cursor.content(), source);

    assert!(cursor.go_to(&[0, 0]).await.unwrap());
    assert_eq!(cursor.path(), &[0, 0]);
    assert_eq!(*cursor.kind(), JSON::String);
    assert_eq!(cursor.content(), "first_name");

    assert!(cursor.walk(Direction::Right).await.unwrap());
    assert_eq!(cursor.path(), &[0, 1]);
    assert_eq!(*cursor.kind(), JSON::String);
    assert_eq!(cursor.content(), "last_name");
}

#[tokio::test]
#[serial]
async fn full_multiple_cursors() {
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

    let parser = Arc::new(Builder::new(source).build().unwrap());

    let task_0 = tokio::spawn({
        let parser = parser.clone();

        async move {
            let mut cursor = parser.cursor().await.unwrap();
            assert_eq!(cursor.path(), &[0]);
            assert_eq!(*cursor.kind(), JSON::Object);
            assert_eq!(cursor.content(), source);

            assert!(cursor.go_to(&[0, 7]).await.unwrap());
            assert_eq!(cursor.path(), &[0, 7]);
            assert_eq!(*cursor.kind(), JSON::Sequence);
            assert_eq!(cursor.content(), "spouse");
        }
    });

    let task_1 = tokio::spawn({
        let parser = parser.clone();

        async move {
            let mut cursor = parser.cursor().await.unwrap();
            assert_eq!(cursor.path(), &[0]);
            assert_eq!(*cursor.kind(), JSON::Object);
            assert_eq!(cursor.content(), source);

            assert!(cursor.go_to(&[0, 0]).await.unwrap());
            assert_eq!(cursor.path(), &[0, 0]);
            assert_eq!(*cursor.kind(), JSON::String);
            assert_eq!(cursor.content(), "first_name");
        }
    });

    let task_2 = tokio::spawn({
        let parser = parser.clone();

        async move {
            let mut cursor = parser.cursor().await.unwrap();
            assert_eq!(cursor.path(), &[0]);
            assert_eq!(*cursor.kind(), JSON::Object);
            assert_eq!(cursor.content(), source);

            assert!(cursor.go_to(&[0, 4]).await.unwrap());
            assert_eq!(cursor.path(), &[0, 4]);
            assert_eq!(*cursor.kind(), JSON::Object);
            assert_eq!(cursor.content(), "address");
        }
    });

    let (result_0, result_1, result_2) = tokio::join!(task_0, task_1, task_2);
    result_0.unwrap();
    result_1.unwrap();
    result_2.unwrap();
}
