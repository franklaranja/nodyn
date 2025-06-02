# Nodyn

Easy polymorphism with enums.

[![Crates.io](https://img.shields.io/crates/v/nodyn.svg)](https://crates.io/crates/nodyn)
[![Docs.rs](https://docs.rs/nodyn/badge.svg)](https://docs.rs/nodyn)
[![License](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)

`nodyn` provides a Rust macro for creating wrapper enums that
encapsulate a fixed set of types with automatic implementations
for `From`, `TryFrom`, and delegated methods or traits. This is
ideal for scenarios where you need to store values of different
types in a type-safe, zero-cost way, as an alternative to trait
objects.

> "This is a perfectly good solution when our interchangeable 
> items are a fixed set of types that we know when our code is compiled."\
> [The Rust Programming Language](http://doc.rust-lang.org/book/ch18-02-trait-objects.html)

## Features

- **Method and Trait Delegation**: Delegates methods or entire traits to the underlying types.
- **Automatic Variant Creation**: Generates an enum with variants for specified types (e.g., `i32`, `String`, custom structs).
- **Type Conversion**: Implements `From<T>` for each variant type and `TryFrom<Enum> for T` for non-reference types.
- **Type Introspection**: Provides utility methods like `count()`, `types()`, and `type_name()` to query variant information.
- **Custom Variant Names**: Allows overriding default variant names for clarity.
- **Supported Types**: Handles path types, references, arrays and tuples.

## Installation

Add `nodyn` to your `Cargo.toml`:

```toml
[dependencies]
nodyn = "0.1.0"
```

## Basic example

```rust
use nodyn::wrap;

wrap! {
    #[derive(Debug, PartialEq)]
    pub enum Value {
        i32,
        String,
        f64,
    }
}

fn main() {
    // Store different types in the same collection
    let values: Vec<Value> = vec![
        42.into(),                    // i32 → Value::I32(42)
        "hello".to_string().into(),   // String → Value::String("hello")
        3.14.into(),                  // f64 → Value::F64(3.14)
    ];

    // Pattern match or use generated methods
    for value in values {
        match value {
            Value::I32(n) => println!("Integer: {}", n),
            Value::String(s) => println!("String: {}", s),
            Value::F64(f) => println!("Float: {}", f),
        }
    }
}
```

## Method Delegation Example

```rust
nodyn::nodyn! {
    enum Container { String, Vec<u8> }

    impl {
        // Delegate methods that exist on all types
        fn len(&self) -> usize;
        fn is_empty(&self) -> bool;
        fn clear(&mut self);
    }
}

let mut container: Container = "hello".to_string().into();
assert_eq!(container.len(), 5);
assert!(!container.is_empty());
```

## Trait Implementation Example

```rust
use std::fmt::{self, Display};

// All wrapped types implement Display
nodyn::nodyn! {
    enum Displayable { i32, String, f64 }

    impl Display {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result;
    }
}

let values: Vec<Displayable> = vec![
    42.into(),
    "hello".to_string().into(),
    3.14.into(),
];

for val in values {
    println!("{}", val); // Uses delegated Display implementation
}
```

## Advanced Example

```rust
use std::fmt;

#[derive(Debug, Clone)]
pub struct Null;
//!
impl fmt::Display for Null {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "null")
    }
}

#[derive(Debug, Clone)]
pub struct JsonArray(Vec<JsonValue>);

impl fmt::Display for JsonArray {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = self
            .0
            .iter()
            .map(ToString::to_string)
            .collect::<Vec<_>>()
            .join(", ");
        write!(f, "[{s}]")
    }
}

nodyn::nodyn! {
    #[derive(Debug, Clone)]
    pub enum JsonValue {
        Null,
        bool,
        f64,
        String,
        JsonArray,
    }

    impl fmt::Display {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result;
    }

    impl {
        pub const fn json_type_name(&self) -> &'static str {
            match self {
                Self::Null(_) => "null",
                Self::Bool(_) => "boolean",
                Self::F64(_) => "number",
                Self::String(_) => "string",
                Self::JsonArray(_) => "array",
            }
        }
    }
}

let values: Vec<JsonValue> = vec![
    Null.into(),                // null
    true.into(),                // boolean
    42.0.into(),                // number
    "hello".to_string().into(), // string
    JsonArray(vec![
        Null.into(),
        false.into(),
        33.0.into(),
        "world".to_string().into(),
    ]) .into(),
];

for val in &values {
    println!("{}: {}", val.json_type_name(), val);
}

// null: null
// boolean: true
// number: 42
// string: hello
// array: [null, false, 33, world]

## Features

All features are enabled by default.

|feature|enables|
|-------|-------|
|`from`          | automatic From trait implementation |
|`try_into`      | automatic TryFrom trait implementation |
|`introspection` | generation of type introspection functions |
|`is_as`         | generation of variant test and accessor functions |
```

## 📚 Documentation

- [API Documentation](https://docs.rs/nodyn)
- [Changelog](https://github.com/franklaranja/nodyn/blob/main/CHANGELOG.md)

## 🆚 Comparison

| Feature | nodyn | enum_dispatch | sum_type | Box\<dyn Trait\> |
|---------|-------|---------------|----------|----------------|
| **Runtime cost** | Zero | Zero | Zero | Heap allocation |
| **Trait delegation** | ✅ Yes | ✅ Scoped only | ❌ No | ✅ Yes |
| **Method delegation** | ✅ Yes | ❌ No | ❌ No | ✅ Yes |
| **Type introspection** | ✅ Built-in | ❌ No | ❌ No | ❌ No |
| **Compile-time known** | Required | Required | Required | Not required |
| **Memory overhead** | Discriminant only | Discriminant only | Discriminant only | Pointer + vtable |

- **[enum_dispatch]**: Near drop-in replacement for dynamic-dispatched method
  calls with up to 10x the speed.
- **[sum_type]**: A convenience macro for creating a wrapper enum which
  may be one of several distinct types.

[enum_dispatch]: https://crates.io/crates/enum_dispatch
[sum_type]: https://crates.io/crates/sum_type

## License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.

---

<div align="center">

**[Documentation](https://docs.rs/nodyn) | [Crates.io](https://crates.io/crates/nodyn) | [Repository](https://github.com/yourusername/nodyn)**

</div>
