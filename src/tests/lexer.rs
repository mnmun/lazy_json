use pretty_assertions::assert_eq;
use std::{
    thread::{self, sleep},
    time::Duration,
};

use a_bc::{
    Token,
    error::{self, Position, Relation},
    lexer::Builder,
};

use crate::{JSON, lexer};

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

    let lexer = lexer::Builder::new(source).build().unwrap();
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

    let mut lexer = lexer::Builder::new(source).build().unwrap();

    let token = lexer.expect(&[JSON::Object]).unwrap();
    assert_eq!(token.kind(), &JSON::Object);

    {
        let mut lexer = lexer::Builder::new(source)
            .with_range(token.inner_range().unwrap())
            .build()
            .unwrap();

        let token = lexer
            .not_expect(&[JSON::Object, JSON::Array, JSON::Comma, JSON::Colon])
            .unwrap();
        assert_eq!(lexer.format(&token).unwrap(), r#""first_name""#);
        assert_eq!(token.kind(), &JSON::String);

        let token = lexer.expect(&[JSON::Colon]).unwrap();
        assert_eq!(lexer.format(&token).unwrap(), ":");
        assert_eq!(token.kind(), &JSON::Colon);

        let token = lexer.not_expect(&[JSON::Comma, JSON::Colon]).unwrap();
        assert_eq!(lexer.format(&token).unwrap(), r#""John""#);
        assert_eq!(token.kind(), &JSON::String);

        let token = lexer.expect(&[JSON::Comma]).unwrap();
        assert_eq!(lexer.format(&token).unwrap(), ",");
        assert_eq!(token.kind(), &JSON::Comma);

        // ---

        let token = lexer
            .not_expect(&[JSON::Object, JSON::Array, JSON::Comma, JSON::Colon])
            .unwrap();
        assert_eq!(lexer.format(&token).unwrap(), r#""last_name""#);
        assert_eq!(token.kind(), &JSON::String);

        let token = lexer.expect(&[JSON::Colon]).unwrap();
        assert_eq!(lexer.format(&token).unwrap(), ":");
        assert_eq!(token.kind(), &JSON::Colon);

        let token = lexer.not_expect(&[JSON::Comma, JSON::Colon]).unwrap();
        assert_eq!(lexer.format(&token).unwrap(), r#""Smith""#);
        assert_eq!(token.kind(), &JSON::String);

        let token = lexer.expect(&[JSON::Comma]).unwrap();
        assert_eq!(lexer.format(&token).unwrap(), ",");
        assert_eq!(token.kind(), &JSON::Comma);

        // ---

        let token = lexer
            .not_expect(&[JSON::Object, JSON::Array, JSON::Comma, JSON::Colon])
            .unwrap();
        assert_eq!(lexer.format(&token).unwrap(), r#""is_alive""#);
        assert_eq!(token.kind(), &JSON::String);

        let token = lexer.expect(&[JSON::Colon]).unwrap();
        assert_eq!(lexer.format(&token).unwrap(), ":");
        assert_eq!(token.kind(), &JSON::Colon);

        let token = lexer.not_expect(&[JSON::Comma, JSON::Colon]).unwrap();
        assert_eq!(lexer.format(&token).unwrap(), "true");
        assert_eq!(token.kind(), &JSON::Sequence);

        let token = lexer.expect(&[JSON::Comma]).unwrap();
        assert_eq!(lexer.format(&token).unwrap(), ",");
        assert_eq!(token.kind(), &JSON::Comma);

        // ---

        let token = lexer
            .not_expect(&[JSON::Object, JSON::Array, JSON::Comma, JSON::Colon])
            .unwrap();
        assert_eq!(lexer.format(&token).unwrap(), r#""age""#);
        assert_eq!(token.kind(), &JSON::String);

        let token = lexer.expect(&[JSON::Colon]).unwrap();
        assert_eq!(lexer.format(&token).unwrap(), ":");
        assert_eq!(token.kind(), &JSON::Colon);

        let token = lexer.not_expect(&[JSON::Comma, JSON::Colon]).unwrap();
        assert_eq!(lexer.format(&token).unwrap(), "27");
        assert_eq!(token.kind(), &JSON::Sequence);

        let token = lexer.expect(&[JSON::Comma]).unwrap();
        assert_eq!(lexer.format(&token).unwrap(), ",");
        assert_eq!(token.kind(), &JSON::Comma);

        // ---

        let token = lexer
            .not_expect(&[JSON::Object, JSON::Array, JSON::Comma, JSON::Colon])
            .unwrap();
        assert_eq!(lexer.format(&token).unwrap(), r#""address""#);
        assert_eq!(token.kind(), &JSON::String);

        let token = lexer.expect(&[JSON::Colon]).unwrap();
        assert_eq!(lexer.format(&token).unwrap(), ":");
        assert_eq!(token.kind(), &JSON::Colon);

        let token = lexer.expect(&[JSON::Object]).unwrap();
        assert_eq!(token.kind(), &JSON::Object);
        {
            let mut lexer = lexer::Builder::new(source)
                .with_range(token.inner_range().unwrap())
                .build()
                .unwrap();

            let token = lexer
                .not_expect(&[
                    JSON::Object,
                    JSON::Array,
                    JSON::Comma,
                    JSON::Colon,
                ])
                .unwrap();
            assert_eq!(lexer.format(&token).unwrap(), r#""street_address""#);
            assert_eq!(token.kind(), &JSON::String);

            let token = lexer.expect(&[JSON::Colon]).unwrap();
            assert_eq!(lexer.format(&token).unwrap(), ":");
            assert_eq!(token.kind(), &JSON::Colon);

            let token = lexer.not_expect(&[JSON::Comma, JSON::Colon]).unwrap();
            assert_eq!(lexer.format(&token).unwrap(), r#""21 2nd Street""#);
            assert_eq!(token.kind(), &JSON::String);

            let token = lexer.expect(&[JSON::Comma]).unwrap();
            assert_eq!(lexer.format(&token).unwrap(), ",");
            assert_eq!(token.kind(), &JSON::Comma);

            // ---

            let token = lexer
                .not_expect(&[
                    JSON::Object,
                    JSON::Array,
                    JSON::Comma,
                    JSON::Colon,
                ])
                .unwrap();
            assert_eq!(lexer.format(&token).unwrap(), r#""city""#);
            assert_eq!(token.kind(), &JSON::String);

            let token = lexer.expect(&[JSON::Colon]).unwrap();
            assert_eq!(lexer.format(&token).unwrap(), ":");
            assert_eq!(token.kind(), &JSON::Colon);

            let token = lexer.not_expect(&[JSON::Comma, JSON::Colon]).unwrap();
            assert_eq!(lexer.format(&token).unwrap(), r#""New York""#);
            assert_eq!(token.kind(), &JSON::String);

            let token = lexer.expect(&[JSON::Comma]).unwrap();
            assert_eq!(lexer.format(&token).unwrap(), ",");
            assert_eq!(token.kind(), &JSON::Comma);

            // ---

            let token = lexer
                .not_expect(&[
                    JSON::Object,
                    JSON::Array,
                    JSON::Comma,
                    JSON::Colon,
                ])
                .unwrap();
            assert_eq!(lexer.format(&token).unwrap(), r#""state""#);
            assert_eq!(token.kind(), &JSON::String);

            let token = lexer.expect(&[JSON::Colon]).unwrap();
            assert_eq!(lexer.format(&token).unwrap(), ":");
            assert_eq!(token.kind(), &JSON::Colon);

            let token = lexer.not_expect(&[JSON::Comma, JSON::Colon]).unwrap();
            assert_eq!(lexer.format(&token).unwrap(), r#""NY""#);
            assert_eq!(token.kind(), &JSON::String);

            let token = lexer.expect(&[JSON::Comma]).unwrap();
            assert_eq!(lexer.format(&token).unwrap(), ",");
            assert_eq!(token.kind(), &JSON::Comma);

            // ---

            let token = lexer
                .not_expect(&[
                    JSON::Object,
                    JSON::Array,
                    JSON::Comma,
                    JSON::Colon,
                ])
                .unwrap();
            assert_eq!(lexer.format(&token).unwrap(), r#""postal_code""#);
            assert_eq!(token.kind(), &JSON::String);

            let token = lexer.expect(&[JSON::Colon]).unwrap();
            assert_eq!(lexer.format(&token).unwrap(), ":");
            assert_eq!(token.kind(), &JSON::Colon);

            let token = lexer.not_expect(&[JSON::Comma, JSON::Colon]).unwrap();
            assert_eq!(lexer.format(&token).unwrap(), r#""10021-3100""#);
            assert_eq!(token.kind(), &JSON::String);
        }

        let token = lexer.expect(&[JSON::Comma]).unwrap();
        assert_eq!(lexer.format(&token).unwrap(), ",");
        assert_eq!(token.kind(), &JSON::Comma);

        // ---

        let token = lexer
            .not_expect(&[JSON::Object, JSON::Array, JSON::Comma, JSON::Colon])
            .unwrap();
        assert_eq!(lexer.format(&token).unwrap(), r#""phone_numbers""#);
        assert_eq!(token.kind(), &JSON::String);

        let token = lexer.expect(&[JSON::Colon]).unwrap();
        assert_eq!(lexer.format(&token).unwrap(), ":");
        assert_eq!(token.kind(), &JSON::Colon);

        let token = lexer.expect(&[JSON::Array]).unwrap();
        assert_eq!(token.kind(), &JSON::Array);

        {
            let mut lexer = lexer::Builder::new(source)
                .with_range(token.inner_range().unwrap())
                .build()
                .unwrap();

            let token = lexer.expect(&[JSON::Object]).unwrap();
            assert_eq!(token.kind(), &JSON::Object);

            {
                let mut lexer = lexer::Builder::new(source)
                    .with_range(token.inner_range().unwrap())
                    .build()
                    .unwrap();

                let token = lexer
                    .not_expect(&[
                        JSON::Object,
                        JSON::Array,
                        JSON::Comma,
                        JSON::Colon,
                    ])
                    .unwrap();
                assert_eq!(lexer.format(&token).unwrap(), r#""type""#);
                assert_eq!(token.kind(), &JSON::String);

                let token = lexer.expect(&[JSON::Colon]).unwrap();
                assert_eq!(lexer.format(&token).unwrap(), ":");
                assert_eq!(token.kind(), &JSON::Colon);

                let token =
                    lexer.not_expect(&[JSON::Comma, JSON::Colon]).unwrap();
                assert_eq!(lexer.format(&token).unwrap(), r#""home""#);
                assert_eq!(token.kind(), &JSON::String);

                let token = lexer.expect(&[JSON::Comma]).unwrap();
                assert_eq!(lexer.format(&token).unwrap(), ",");
                assert_eq!(token.kind(), &JSON::Comma);

                // ---

                let token = lexer
                    .not_expect(&[
                        JSON::Object,
                        JSON::Array,
                        JSON::Comma,
                        JSON::Colon,
                    ])
                    .unwrap();
                assert_eq!(lexer.format(&token).unwrap(), r#""number""#);
                assert_eq!(token.kind(), &JSON::String);

                let token = lexer.expect(&[JSON::Colon]).unwrap();
                assert_eq!(lexer.format(&token).unwrap(), ":");
                assert_eq!(token.kind(), &JSON::Colon);

                let token =
                    lexer.not_expect(&[JSON::Comma, JSON::Colon]).unwrap();
                assert_eq!(lexer.format(&token).unwrap(), r#""212 555-1234""#);
                assert_eq!(token.kind(), &JSON::String);
            }

            let token = lexer.expect(&[JSON::Comma]).unwrap();
            assert_eq!(lexer.format(&token).unwrap(), ",");
            assert_eq!(token.kind(), &JSON::Comma);

            // ---

            let token = lexer.expect(&[JSON::Object]).unwrap();
            assert_eq!(token.kind(), &JSON::Object);

            {
                let mut lexer = lexer::Builder::new(source)
                    .with_range(token.inner_range().unwrap())
                    .build()
                    .unwrap();

                let token = lexer
                    .not_expect(&[
                        JSON::Object,
                        JSON::Array,
                        JSON::Comma,
                        JSON::Colon,
                    ])
                    .unwrap();
                assert_eq!(lexer.format(&token).unwrap(), r#""type""#);
                assert_eq!(token.kind(), &JSON::String);

                let token = lexer.expect(&[JSON::Colon]).unwrap();
                assert_eq!(lexer.format(&token).unwrap(), ":");
                assert_eq!(token.kind(), &JSON::Colon);

                let token =
                    lexer.not_expect(&[JSON::Comma, JSON::Colon]).unwrap();
                assert_eq!(lexer.format(&token).unwrap(), r#""office""#);
                assert_eq!(token.kind(), &JSON::String);

                let token = lexer.expect(&[JSON::Comma]).unwrap();
                assert_eq!(lexer.format(&token).unwrap(), ",");
                assert_eq!(token.kind(), &JSON::Comma);

                // ---

                let token = lexer
                    .not_expect(&[
                        JSON::Object,
                        JSON::Array,
                        JSON::Comma,
                        JSON::Colon,
                    ])
                    .unwrap();
                assert_eq!(lexer.format(&token).unwrap(), r#""number""#);
                assert_eq!(token.kind(), &JSON::String);

                let token = lexer.expect(&[JSON::Colon]).unwrap();
                assert_eq!(lexer.format(&token).unwrap(), ":");
                assert_eq!(token.kind(), &JSON::Colon);

                let token =
                    lexer.not_expect(&[JSON::Comma, JSON::Colon]).unwrap();
                assert_eq!(lexer.format(&token).unwrap(), r#""646 555-4567""#);
                assert_eq!(token.kind(), &JSON::String);
            }
        }

        let token = lexer.expect(&[JSON::Comma]).unwrap();
        assert_eq!(lexer.format(&token).unwrap(), ",");
        assert_eq!(token.kind(), &JSON::Comma);

        // ---

        let token = lexer
            .not_expect(&[JSON::Object, JSON::Array, JSON::Comma, JSON::Colon])
            .unwrap();
        assert_eq!(lexer.format(&token).unwrap(), r#""children""#);
        assert_eq!(token.kind(), &JSON::String);

        let token = lexer.expect(&[JSON::Colon]).unwrap();
        assert_eq!(lexer.format(&token).unwrap(), ":");
        assert_eq!(token.kind(), &JSON::Colon);

        let token = lexer.expect(&[JSON::Array]).unwrap();
        assert_eq!(token.kind(), &JSON::Array);

        {
            let mut lexer = lexer::Builder::new(source)
                .with_range(token.inner_range())
                .build()
                .unwrap();

            let token = lexer.not_expect(&[JSON::Colon, JSON::Comma]).unwrap();
            assert_eq!(lexer.format(&token).unwrap(), r#""Catherine""#);
            assert_eq!(token.kind(), &JSON::String);

            let token = lexer.expect(&[JSON::Comma]).unwrap();
            assert_eq!(lexer.format(&token).unwrap(), ",");
            assert_eq!(token.kind(), &JSON::Comma);

            // ---

            let token = lexer.not_expect(&[JSON::Colon, JSON::Comma]).unwrap();
            assert_eq!(lexer.format(&token).unwrap(), r#""Thomas""#);
            assert_eq!(token.kind(), &JSON::String);

            let token = lexer.expect(&[JSON::Comma]).unwrap();
            assert_eq!(lexer.format(&token).unwrap(), ",");
            assert_eq!(token.kind(), &JSON::Comma);

            // ---

            let token = lexer.not_expect(&[JSON::Colon, JSON::Comma]).unwrap();
            assert_eq!(lexer.format(&token).unwrap(), r#""Trevor""#);
            assert_eq!(token.kind(), &JSON::String);
        }

        let token = lexer.expect(&[JSON::Comma]).unwrap();
        assert_eq!(lexer.format(&token).unwrap(), ",");
        assert_eq!(token.kind(), &JSON::Comma);

        // ---

        let token = lexer
            .not_expect(&[JSON::Object, JSON::Array, JSON::Comma, JSON::Colon])
            .unwrap();
        assert_eq!(lexer.format(&token).unwrap(), r#""spouse""#);
        assert_eq!(token.kind(), &JSON::String);

        let token = lexer.expect(&[JSON::Colon]).unwrap();
        assert_eq!(lexer.format(&token).unwrap(), ":");
        assert_eq!(token.kind(), &JSON::Colon);

        let token = lexer.not_expect(&[JSON::Comma, JSON::Colon]).unwrap();
        assert_eq!(lexer.format(&token).unwrap(), "null");
        assert_eq!(token.kind(), &JSON::Sequence);
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
    let source = b"{123}  ";

    let mut lexer = lexer::Builder::new(source).build().unwrap();
    let block_range = lexer.find_block(b'{', b'}').unwrap();
    assert_eq!(block_range, 0..5);

    let source = b"  {123}";

    let mut lexer = lexer::Builder::new(source).build().unwrap();
    assert!(lexer.find_block(b'{', b'}').is_err_and(|e| {
        e == a_bc::error::Token::PairNotFound {
            opening: "{".into(),
            closing: "}".into(),
            location: (Relation::After, Position::new(1, 1)),
        }
        .into()
    }));
}

#[test]
fn find_string() {
    let source = br#""123"  "#;

    let mut lexer = lexer::Builder::new(source).build().unwrap();
    let string_range = lexer.find_string().unwrap();
    assert_eq!(string_range, 0..5);

    let source = br#"  "123""#;

    let mut lexer = lexer::Builder::new(source).build().unwrap();
    let string_range = lexer.find_string().unwrap();
    assert_eq!(string_range, 0..3);
}

#[test]
fn find_sequence() {
    let source = br#"123  "#;

    let mut lexer = lexer::Builder::new(source).build().unwrap();
    let sequence_range = lexer.find_sequence().unwrap();
    assert_eq!(sequence_range, 0..3);

    let source = br#"  123"#;

    let mut lexer = lexer::Builder::new(source).build().unwrap();
    let sequence_range = lexer.find_sequence().unwrap();
    assert_eq!(sequence_range, 0..1);
}

#[test]
fn find_complicated_block() {
    let source = br#"{ "} {" }"#;

    let mut lexer = lexer::Builder::new(source).build().unwrap();
    let block_range = lexer.find_block(b'{', b'}').unwrap();
    assert_eq!(block_range, 0..9);
}

#[test]
fn colon() {
    let source = b":";

    let mut lexer = lexer::Builder::new(source).build().unwrap();
    let token = lexer.next();
    assert_eq!(token, Some(Ok(Token::new(JSON::Colon, 0..1))));

    let token = lexer.next();
    assert_eq!(token, None);
    assert_eq!(*lexer.cursor().position(), 1);
}

#[test]
fn comma() {
    let source = b",";

    let mut lexer = lexer::Builder::new(source).build().unwrap();
    let token = lexer.next();
    assert_eq!(token, Some(Ok(Token::new(JSON::Comma, 0..1))));

    let token = lexer.next();
    assert_eq!(token, None);
    assert_eq!(*lexer.cursor().position(), 1);
}

#[test]
fn empty_object() {
    let source = b"{}";

    let mut lexer = lexer::Builder::new(source).build().unwrap();
    let token = lexer.next();
    assert_eq!(token, Some(Ok(Token::new(JSON::Object, 0..2))));

    let token = lexer.next();
    assert_eq!(token, None);
    assert_eq!(*lexer.cursor().position(), 2);
}

#[test]
fn empty_array() {
    let source = b"[]";

    let mut lexer = lexer::Builder::new(source).build().unwrap();
    let token = lexer.next();
    assert_eq!(token, Some(Ok(Token::new(JSON::Array, 0..2))));

    let token = lexer.next();
    assert_eq!(token, None);
    assert_eq!(*lexer.cursor().position(), 2);
}

#[test]
fn empty_string() {
    let source = br#""""#;

    let mut lexer = lexer::Builder::new(source).build().unwrap();
    let token = lexer.next();
    assert_eq!(token, Some(Ok(Token::new(JSON::String, 0..2))));

    let token = lexer.next();
    assert_eq!(token, None);
    assert_eq!(*lexer.cursor().position(), 2);
}

#[test]
fn filled_object() {
    let source = br#"{"name":"value"}"#;

    let mut lexer = lexer::Builder::new(source).build().unwrap();
    let token = lexer.next();
    assert_eq!(token, Some(Ok(Token::new(JSON::Object, 0..16))));

    let token = lexer.next();
    assert_eq!(token, None);
    assert_eq!(*lexer.cursor().position(), 16);
}

#[test]
fn incomplete_object() {
    let source = br#"{"name":"value""#;

    let mut lexer = lexer::Builder::new(source).build().unwrap();
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

    let mut lexer = lexer::Builder::new(source).build().unwrap();
    let token = lexer.next();
    assert_eq!(token, Some(Ok(Token::new(JSON::Array, 0..21))));

    let token = lexer.next();
    assert_eq!(token, None);
    assert_eq!(*lexer.cursor().position(), 21);
}

#[test]
fn incomplete_array() {
    let source = b"[1,2,3,4,5,6,7,8,9,0";

    let mut lexer = lexer::Builder::new(source).build().unwrap();
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

    let mut lexer = lexer::Builder::new(source).build().unwrap();
    let token = lexer.next();
    assert_eq!(token, Some(Ok(Token::new(JSON::String, 0..6))));

    let token = lexer.next();
    assert_eq!(token, None);
    assert_eq!(*lexer.cursor().position(), 6);
}

#[test]
fn escaped_string() {
    let source = br#""a\"b""#;

    let mut lexer = lexer::Builder::new(source).build().unwrap();
    let token = lexer.next();
    assert_eq!(token, Some(Ok(Token::new(JSON::String, 0..6))));

    let token = lexer.next();
    assert_eq!(token, None);
    assert_eq!(*lexer.cursor().position(), 6);
}

#[test]
fn incomplete_string() {
    let source = br#""John"#;

    let mut lexer = lexer::Builder::new(source).build().unwrap();
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

    let mut lexer = lexer::Builder::new(source).build().unwrap();
    let token = lexer.next();
    assert_eq!(token, Some(Ok(Token::new(JSON::Sequence, 0..12))));

    let token = lexer.next();
    assert_eq!(token, None);
}

#[test]
fn error_print() {
    assert_eq!(
        format!(
            "{}",
            a_bc::Error::Token(a_bc::error::Token::<JSON>::PairNotFound {
                opening: "{".into(),
                closing: "}".into(),
                location: (Relation::At, Position::new(1, 1))
            })
        )
        .as_str(),
        "Token error: Could not find '}' at [1:1]"
    );

    assert_eq!(
        format!(
            "{}",
            a_bc::Error::Token(a_bc::error::Token::<JSON>::ExpectedButGot {
                expected: [JSON::String, JSON::Sequence].into(),
                got: None,
                location: (Relation::After, Position::new(1, 1))
            })
        )
        .as_str(),
        "Token error: Expected one of these tokens: [String, Sequence] after [1:1], but got nothing"
    );

    assert_eq!(
        format!(
            "{}",
            a_bc::Error::Token(a_bc::error::Token::<JSON>::ExpectedButGot {
                expected: [JSON::String, JSON::Sequence].into(),
                got: Some(JSON::Comma),
                location: (Relation::After, Position::new(1, 1))
            })
        )
        .as_str(),
        "Token error: Expected one of these tokens: [String, Sequence] after [1:1], but got Comma"
    );

    assert_eq!(
        format!(
            "{}",
            a_bc::Error::Token(a_bc::error::Token::<JSON>::NotExpectedButGot {
                not_expected: [JSON::String, JSON::Sequence].into(),
                got: None,
                location: (Relation::After, Position::new(1, 1))
            })
        )
        .as_str(),
        "Token error: Not expected these tokens: [String, Sequence] after [1:1], but got nothing"
    );

    assert_eq!(
        format!(
            "{}",
            a_bc::Error::Token(a_bc::error::Token::<JSON>::NotExpectedButGot {
                not_expected: [JSON::String, JSON::Sequence].into(),
                got: Some(JSON::String),
                location: (Relation::After, Position::new(1, 1))
            })
        )
        .as_str(),
        "Token error: Not expected these tokens: [String, Sequence] after [1:1], but got String"
    );

    assert_eq!(
        format!(
            "{}",
            a_bc::Error::<JSON>::Lexer(a_bc::error::Lexer::SourceIsEmpty)
        )
        .as_str(),
        "Lexer error: The source data is empty"
    );

    assert_eq!(
        format!(
            "{}",
            a_bc::Error::<JSON>::Lexer(a_bc::error::Lexer::Cancelled)
        )
        .as_str(),
        "Lexer error: Lexing was manually cancelled"
    );
}
