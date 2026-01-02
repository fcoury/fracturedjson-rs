# fracturedjson

A Rust port of [FracturedJson](https://github.com/j-brooke/FracturedJson) — a JSON formatter that produces human-readable output with smart line breaks, table-like alignment, and optional comment support.

## Overview

FracturedJson formats JSON data in a way that's easy for humans to read while remaining fairly compact. Arrays and objects are written on single lines when they're short and simple enough. When several lines have similar structure, their fields are aligned like a table. Long arrays are written with multiple items per line.

This crate is part of the FracturedJson family:

- [Home Page and Browser-based Formatter](https://j-brooke.github.io/FracturedJson/)
- [FracturedJson (C#)](https://github.com/j-brooke/FracturedJson)
- [FracturedJsonJs (JavaScript/npm)](https://github.com/j-brooke/FracturedJsonJs)
- [VS Code Extension](https://marketplace.visualstudio.com/items?itemName=j-brooke.fracturedjsonvsc)

## Installation

Add to your `Cargo.toml`:

```toml
[dependencies]
fracturedjson = "0.1"
```

## Usage

### Reformat JSON Text

```rust
use fracturedjson::Formatter;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let input = r#"{"name":"Alice","scores":[95,87,92],"active":true}"#;

    let mut formatter = Formatter::new();
    let output = formatter.reformat(input, 0)?;

    println!("{}", output);
    Ok(())
}
```

### Serialize Rust Values

```rust
use fracturedjson::Formatter;
use serde::Serialize;

#[derive(Serialize)]
struct Player {
    name: String,
    scores: Vec<i32>,
    active: bool,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let player = Player {
        name: "Alice".into(),
        scores: vec![95, 87, 92],
        active: true,
    };

    let mut formatter = Formatter::new();
    let output = formatter.serialize(&player, 0, 100)?;

    println!("{}", output);
    Ok(())
}
```

### Serialize `serde_json::Value`

```rust
use fracturedjson::Formatter;
use serde_json::json;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let value = json!({
        "name": "Alice",
        "scores": [95, 87, 92],
        "active": true
    });

    let mut formatter = Formatter::new();
    let output = formatter.serialize_value(&value, 0, 100)?;

    println!("{}", output);
    Ok(())
}
```

### Minify JSON

```rust
use fracturedjson::Formatter;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let input = r#"{
        "name": "Alice",
        "scores": [95, 87, 92]
    }"#;

    let mut formatter = Formatter::new();
    let output = formatter.minify(input)?;

    println!("{}", output);
    // Output: {"name":"Alice","scores":[95,87,92]}
    Ok(())
}
```

## Configuration

Customize formatting behavior via `FracturedJsonOptions`:

```rust
use fracturedjson::{
    Formatter, FracturedJsonOptions, EolStyle,
    NumberListAlignment, CommentPolicy,
};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut formatter = Formatter::new();

    formatter.options.max_total_line_length = 80;
    formatter.options.max_inline_complexity = 1;
    formatter.options.indent_spaces = 2;
    formatter.options.json_eol_style = EolStyle::Lf;
    formatter.options.number_list_alignment = NumberListAlignment::Decimal;

    // Enable comment support (JSON with comments)
    formatter.options.comment_policy = CommentPolicy::Preserve;

    let input = r#"{"values": [1, 2, 3]}"#;
    let output = formatter.reformat(input, 0)?;

    println!("{}", output);
    Ok(())
}
```

### Available Options

| Option | Type | Default | Description |
|--------|------|---------|-------------|
| `max_total_line_length` | `usize` | 120 | Maximum line length before wrapping |
| `max_inline_complexity` | `isize` | 2 | Max nesting depth for inline formatting |
| `max_compact_array_complexity` | `isize` | 2 | Max complexity for compact array rows |
| `max_table_row_complexity` | `isize` | 2 | Max complexity for table row formatting |
| `indent_spaces` | `usize` | 4 | Spaces per indentation level |
| `use_tab_to_indent` | `bool` | false | Use tabs instead of spaces |
| `json_eol_style` | `EolStyle` | `Lf` | Line ending style (`Lf` or `Crlf`) |
| `number_list_alignment` | `NumberListAlignment` | `Decimal` | Number alignment in arrays |
| `comment_policy` | `CommentPolicy` | `TreatAsError` | How to handle comments |
| `preserve_blank_lines` | `bool` | false | Keep blank lines from input |
| `allow_trailing_commas` | `bool` | false | Allow trailing commas in input |

## Example Output

```json
{
    "BasicObject"   : {
        "ModuleId"   : "armor",
        "Name"       : "",
        "Locations"  : [
            [11,  2], [11,  3], [11,  4], [11,  5], [11,  6], [11,  7], [11,  8], [11,  9],
            [11, 10], [11, 11], [11, 12], [11, 13], [11, 14], [ 1, 14], [ 1, 13], [ 1, 12],
            [ 1, 11], [ 1, 10], [ 1,  9], [ 1,  8], [ 1,  7], [ 1,  6], [ 1,  5], [ 1,  4],
            [ 1,  3], [ 1,  2], [ 4,  2], [ 5,  2], [ 6,  2], [ 7,  2], [ 8,  2], [ 8,  3],
            [ 7,  3], [ 6,  3], [ 5,  3], [ 4,  3], [ 0,  4], [ 0,  5], [ 0,  6], [ 0,  7],
            [ 0,  8], [12,  8], [12,  7], [12,  6], [12,  5], [12,  4]
        ],
        "Orientation": "Fore",
        "Seed"       : 272691529
    },
    "SimilarArrays" : {
        "Katherine": ["blue",       "lightblue", "black"       ],
        "Logan"    : ["yellow",     "blue",      "black", "red"],
        "Erik"     : ["red",        "purple"                   ],
        "Jean"     : ["lightgreen", "yellow",    "black"       ]
    },
    "SimilarObjects": [
        { "type": "turret",    "hp": 400, "loc": {"x": 47, "y":  -4}, "flags": "S"   },
        { "type": "assassin",  "hp":  80, "loc": {"x": 12, "y":   6}, "flags": "Q"   },
        { "type": "berserker", "hp": 150, "loc": {"x":  0, "y":   0}                 },
        { "type": "pittrap",              "loc": {"x": 10, "y": -14}, "flags": "S,I" }
    ]
}
```

With comment support enabled (`CommentPolicy::Preserve`):

```jsonc
{
    /*
     * Multi-line comments
     * are fun!
     */
    "NumbersWithHex": [
          254 /*00FE*/,  1450 /*5AA*/,      0 /*0000*/, 36000 /*8CA0*/,    10 /*000A*/,
          199 /*00C7*/, 15001 /*3A99*/,  6540 /*198C*/
    ],
    /* Elements are keen */
    "Elements"      : [
        { /*Carbon*/   "Symbol": "C",  "Number":  6, "Isotopes": [11, 12, 13, 14] },
        { /*Oxygen*/   "Symbol": "O",  "Number":  8, "Isotopes": [16, 18, 17    ] },
        { /*Hydrogen*/ "Symbol": "H",  "Number":  1, "Isotopes": [ 1,  2,  3    ] },
        { /*Iron*/     "Symbol": "Fe", "Number": 26, "Isotopes": [56, 54, 57, 58] }
        // Not a complete list...
    ]
}
```

## License

MIT
