# lazy_json

Quickly splits string data into `JSON` `tokens` and forms a `tree` of lazily-created `nodes` based on them.

## `Lexer`

![](https://github.com/mnmun/images/blob/main/baggage.png?raw=true)

Implemented using the [`a_bc`](https://github.com/mnmun/a_bc) crate.

Splits the sequence of raw bytes into tokens of the following `kinds`:

- `Comma` - comma `,` character;
- `Colon` - colon `:` character;
- `Object` - curly braces `{` `}` and all characters between them;
- `Array` - square brackets `[` `]` and all characters between them;
- `String` - double quotes `"` `"` and all characters between them;
- `Sequence` - a contiguous run of characters: numbers, `true`, `false`, `null`, etc.; whitespace characters as well as the `:` and `,` characters act as separators.

It is important to note the `sequence` `kind`: the `lexer` does not distinguish between numbers, `null` and the boolean values `true` and `false`. Instead, it groups them all under the `sequence` `kind`, leaving it up to the developer to determine whether a `token's` value is valid.

`Lexer` can be created using a `builder`, and `tokens` can be extracted using the `next()` method:

```rust
use lazy_json::{JSON, Lexer, lexer::Builder, a_bc::Token};

let source: &'static [u8] = br#" "name": "value" "#;

let mut lexer: Lexer = Builder::new(source).build().unwrap();

let token: Token<JSON> = lexer.next().unwrap().unwrap();
assert_eq!(token.kind(), &JSON::String);
assert_eq!(lexer.format(&token).unwrap(), r#""name""#);

let token: Token<JSON> = lexer.next().unwrap().unwrap();
assert_eq!(token.kind(), &JSON::Colon);
assert_eq!(lexer.format(&token).unwrap(), ":");

let token: Token<JSON> = lexer.next().unwrap().unwrap();
assert_eq!(token.kind(), &JSON::String);
assert_eq!(lexer.format(&token).unwrap(), r#""value""#);
```

## `Parser`

![](https://github.com/mnmun/images/blob/main/cart.png?raw=true)

Implemented using the `lexer` and the [`lazy_tree`](https://github.com/mnmun/lazy_tree) crate.

Represents string data as a `tree` of lazily-created `nodes`, based on the `JSON` structure contained within:

```text
   *Source data*                *Parsed tree*

{                               *Source data*
  "object": {                ┌────────┴────────┐
    "string": "foo",       object            array
    "number": 42         ┌───┴───┐       ┌─────┼─────┐
  },                   string  number   true false  null
  "array": [             │       │
    true,               foo      42
    false,
    null
  ]
}
```

The entire hierarchical structure of `nodes` is not built at once - `nodes` are created lazily when the `cursor` needs to access them and are deleted when they are no longer used.

`Parser` can be created using a `builder`, and the `tree` of lazily-created `nodes` can be traversed using a `cursor`:

```rust
use lazy_json::{
    JSON, Parser, Cursor,
    parser::{Builder, Bundle},
    a_bc::Error,
    lazy_tree::cursor::{Direction, Target},
};

//
//              source
//       ┌────────┴────────┐
//     object            array
//   ┌───┴───┐       ┌─────┼─────┐
// string  number   true false  null
//   │       │
//  foo      42
//
let source: &'static str = r#"
{
  "object": {
    "string": "foo",
    "number": 42
  },
  "array": [
    true,
    false,
    null
  ]
}
"#;

let parser: Parser = Builder::new(source).build().unwrap();

let mut cursor: Cursor<'_> = parser.cursor().await.unwrap();
assert_eq!(cursor.kind(), &JSON::Object);
assert_eq!(cursor.content(), source);

cursor.walk(Direction::Down(Target::First)).await.unwrap();
assert_eq!(cursor.kind(), &JSON::Object);
assert_eq!(cursor.content(), "object");

cursor.jump(Direction::Left).await.unwrap();
assert_eq!(cursor.kind(), &JSON::Array);
assert_eq!(cursor.content(), "array");

cursor.go_to(&[0, 0, 1, 0]).await.unwrap();
assert_eq!(cursor.kind(), &JSON::Sequence);
assert_eq!(cursor.content(), "42");
```

## License

[MIT](LICENSE)
