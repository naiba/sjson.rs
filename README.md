# sjson.rs

Set JSON values quickly in Rust.

inspired by https://github.com/tidwall/sjson

## Getting Started

### Installation

Add this to your `Cargo.toml`:

```toml
[dependencies]
sjson = "0.1.0"
```

### Quick Start

```rust
use sjson::{set, set_bool, set_int, delete};

fn main() {
    // Set a simple value
    let json = r#"{"name":"Tom","age":37}"#;
    let result = set(json, "name", "Jerry").unwrap();
    println!("{}", result);
    // Output: {"name":"Jerry","age":37}

    // Set a nested value
    let json = r#"{"name":{"first":"Tom","last":"Anderson"}}"#;
    let result = set(json, "name.first", "Jerry").unwrap();
    println!("{}", result);
    // Output: {"name":{"first":"Jerry","last":"Anderson"}}

    // Set an array element
    let json = r#"{"children":["Sara","Alex","Jack"]}"#;
    let result = set(json, "children.1", "Jerry").unwrap();
    println!("{}", result);
    // Output: {"children":["Sara","Jerry","Jack"]}

    // Set a boolean value
    let json = r#"{"name":"Tom"}"#;
    let result = set_bool(json, "active", true, None).unwrap();
    println!("{}", result);
    // Output: {"name":"Tom","active":true}

    // Set an integer value
    let json = r#"{"name":"Tom"}"#;
    let result = set_int(json, "age", 37, None).unwrap();
    println!("{}", result);
    // Output: {"name":"Tom","age":37}

    // Delete a value
    let json = r#"{"name":"Tom","age":37}"#;
    let result = delete(json, "age").unwrap();
    println!("{}", result);
    // Output: {"name":"Tom"}
}
```

## API Reference

### Functions

#### `set(json: &str, path: &str, value: &str) -> Result<String, SjsonError>`

Sets a string value for the specified path.

#### `set_bool(json: &str, path: &str, value: bool, opts: Option<&Options>) -> Result<String, SjsonError>`

Sets a boolean value for the specified path.

#### `set_int<T: std::fmt::Display>(json: &str, path: &str, value: T, opts: Option<&Options>) -> Result<String, SjsonError>`

Sets an integer value for the specified path.

#### `set_float<T: std::fmt::Display>(json: &str, path: &str, value: T, opts: Option<&Options>) -> Result<String, SjsonError>`

Sets a float value for the specified path.

#### `set_value<T: serde::Serialize>(json: &str, path: &str, value: &T, opts: Option<&Options>) -> Result<String, SjsonError>`

Sets any serializable value for the specified path.

#### `set_raw(json: &str, path: &str, value: &str) -> Result<String, SjsonError>`

Sets a raw JSON value for the specified path.

#### `delete(json: &str, path: &str) -> Result<String, SjsonError>`

Deletes a value from JSON for the specified path.

### Options

```rust
use sjson::Options;

let mut opts = Options::default();
opts.optimistic = true;        // Hint that value likely exists
```

#### Optimistic Mode

When `optimistic` is set to `true`, sjson will attempt to perform a fast string-based replacement instead of full JSON parsing. This can provide significant performance improvements (up to 10x faster) for simple operations where the path exists and the value can be found directly in the JSON string.

```rust
use sjson::{set_options, Options};

let json = r#"{"name":"Tom","age":37}"#;
let mut opts = Options::default();
opts.optimistic = true;

// This will use fast string replacement
let result = set_options(json, "name", "Jerry", Some(&opts)).unwrap();
```

### Path Syntax

A path is a series of keys separated by a dot. For example:

```json
{
  "name": {"first": "Tom", "last": "Anderson"},
  "age": 37,
  "children": ["Sara","Alex","Jack"],
  "friends": [
    {"first": "James", "last": "Murphy"},
    {"first": "Roger", "last": "Craig"}
  ]
}
```

- `"name.last"` → `"Anderson"`
- `"age"` → `37`
- `"children.1"` → `"Alex"`
- `"friends.0.first"` → `"James"`

### Error Handling

```rust
use sjson::{set, SjsonError};

match set(json, "invalid.path", "value") {
    Ok(result) => println!("Success: {}", result),
    Err(SjsonError::EmptyPath) => println!("Path cannot be empty"),
    Err(SjsonError::InvalidPath) => println!("Invalid path"),
    Err(SjsonError::NoChange) => println!("No change made"),
    Err(e) => println!("Other error: {}", e),
}
```

## Performance

sjson.rs is designed for high performance JSON manipulation. It uses the serde_json library for fast JSON parsing and provides efficient string manipulation for setting values.

## License

MIT License - see LICENSE file for details.
