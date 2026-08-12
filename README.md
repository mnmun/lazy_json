![logo](https://github.com/mnmun/json/blob/main/logo.png?raw=true)

# JSON lexer built on top of [a_bc](https://github.com/mnmun/a_bc)

A ready-made lexer that splits a JSON byte input into `Token`s tagged with `Json` kinds.

## Example

```rust
use pretty_assertions::assert_eq;

use a_bc::{lexer::Builder, token::Token};
use json::{Json, JsonLexer};

let source = b"
 {
     \"first_name\": \"John\",
     \"second_name\": \"Smith\"
 }
";

let mut lexer = JsonLexer::new(Builder::new(source).build().unwrap());

let token = lexer.next().unwrap().unwrap();
assert_eq!(token.kind(), &Json::Object);

let mut lexer = JsonLexer::new(
 Builder::new(source)
     .with_range(token.inner_range().unwrap())
     .build()
     .unwrap()
);

let token = lexer.expect(&[Json::String]).unwrap().unwrap();
assert_eq!(token.kind(), &Json::String);
assert_eq!(lexer.format(&token), "\"first_name\"");

let token = lexer.expect(&[Json::Colon]).unwrap().unwrap();
assert_eq!(token.kind(), &Json::Colon);
assert_eq!(lexer.format(&token), ":");

let token = lexer.not_expect(&[Json::Comma]).unwrap().unwrap();
assert_eq!(token.kind(), &Json::String);
assert_eq!(lexer.format(&token), "\"John\"");

let token = lexer.expect(&[Json::Comma]).unwrap().unwrap();
assert_eq!(token.kind(), &Json::Comma);
assert_eq!(lexer.format(&token), ",");

let token = lexer
 .not_expect(&[
     Json::Object,
     Json::Array,
     Json::Comma,
     Json::Colon,
 ])
 .unwrap()
 .unwrap();
assert_eq!(token.kind(), &Json::String);
assert_eq!(lexer.format(&token), "\"second_name\"");

let token = lexer.expect(&[Json::Colon]).unwrap().unwrap();
assert_eq!(token.kind(), &Json::Colon);
assert_eq!(lexer.format(&token), ":");

let token = lexer.not_expect(&[Json::Comma]).unwrap().unwrap();
assert_eq!(token.kind(), &Json::String);
assert_eq!(lexer.format(&token), "\"Smith\"");

let token = lexer.next();
assert_eq!(token, None);
```

## License

[MIT](LICENSE)
