# Nodyn

[![Crates.io](https://img.shields.io/crates/v/nodyn.svg)](https://crates.io/crates/nodyn)
[![Docs.rs](https://docs.rs/nodyn/badge.svg)](https://docs.rs/nodyn)
[![CI](https://github.com/franklaranja/nodyn/actions/workflows/ci.yml/badge.svg)](https://github.com/franklaranja/nodyn/actions/workflows/ci.yml)
[![Maintenance](https://img.shields.io/badge/maintenance-actively%20developed-brightgreen.svg)](https://github.com/franklaranja/nodyn)
[![GitHub Issues](https://img.shields.io/github/issues/franklaranja/nodyn)](https://github.com/franklaranja/nodyn/issues)
[![License](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)

<div align="center">

*Easy polymorphism with enums*

</div>

`nodyn` provides a Rust macro for creating wrapper enums that
encapsulate a fixed set of types with automatic implementations
for `From`, `TryFrom`, and delegated methods or traits. This is
ideal for scenarios where you need to store values of different
types in a type-safe, zero-cost way, as an alternative to trait
objects.

> "This is a perfectly good solution when our interchangeable 
> items are a fixed set of types that we know when our code is compiled."\
> [The Rust Programming Language](http://doc.rust-lang.org/book/ch18-02-trait-objects.html)

## Quick Start

Create a simple enum wrapper for `i32`, `String`, and `f64`:

```rust
nodyn::nodyn! {
    #[derive(Debug, PartialEq)]
    pub enum Value {
        i32,
        String,
        f64,
    }
}

let values: Vec<Value> = vec![
    42.into(),                  // Converts i32 to Value::I32
    "hello".to_string().into(), // Converts String to Value::String
    3.14.into(),                // Converts f64 to Value::F64
];

for value in values {
    match value {
        Value::I32(n) => println!("Integer: {}", n),
        Value::String(s) => println!("String: {}", s),
        Value::F64(f) => println!("Float: {}", f),
    }
}
```

## Features

- **Automatic Variant Creation**: Generates enum variants for types
  like `i32`, `String`, or custom structs with CamelCase naming.
- **Type Conversions**: Implements `From<T>` for each variant and `TryFrom<Enum>`
  with the `impl TryInto` directive.
- **Method and Trait Delegation**: Delegates methods or entire traits to
  underlying types.
- **Type Introspection**: Provides `count`, `types`, and `type_name`
  methods with `impl introspection`.
- **Polymorphic Vec**: Generates a `Vec<Enum>` wrapper with a `vec!`-like macro
  and variant-specific methods (e.g., `first_i32`, `count_string`) via
  `vec`.
- **Customizable Variants**: Allows overriding default variant names.
- **Supported Types**: Handles path types, references, arrays, and tuples.

Use `impl` directives to enable features explicitly (e.g., `impl TryInto is_as`). Cargo features (`try_into`, `is_as`, `introspection`) are
deprecated but supported for backward compatibility. See the [Feature Flags section](https://github.com/franklaranja/nodyn/blob/main/src/lib.rs#feature-flags)
for details.

### Vec Wrapper Example

Use the `vec` feature to create a polymorphic `Vec` with variant-specific
methods:

```rust
nodyn::nodyn! {
    #[derive(Debug, Clone)]
    pub enum Item {
        i32,    // Gold coins
        String, // Weapon names
        f64,    // Health potions (liters)
    }
    vec Inventory;
}

let mut inventory = inventory![100, "sword".to_string(), 0.5, "axe".to_string()];
// Add more gold
inventory.push(50);
// Check for weapons in the inventory
assert!(inventory.any_string());
// Total gold coins
let total_gold = inventory.iter_i32().sum::<i32>();
assert_eq!(total_gold, 150);
// Get a potion
if let Some(potion) = inventory.first_f64() {
    println!("Found potion: {} liters", potion); // Prints: 0.5 liters
}
```

See the Polymorphic Vec section in the [Documentation](https://docs.rs/nodyn)

## Method Delegation Example

```rust
nodyn::nodyn! {
    enum Container { String, Vec<u8> }
    impl {
        fn len(&self) -> usize;
        fn is_empty(&self) -> bool;
        fn clear(&mut self);
    }
}

let mut container: Container = "hello".to_string().into();
assert_eq!(container.len(), 5);
assert!(!container.is_empty());
container.clear();
assert!(container.is_empty());
```

## Trait Implementation Example

Delegate entire traits when all wrapped types implement them:

```rust
use std::fmt::{self, Display};

// All wrapped types implement Display
nodyn::nodyn! {
    enum Displayable { i32, String, f64 }

    impl Display {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result;
    }

    vec Displayables;
}

let values = displayables![42, "hello".to_string(), 3.14];

for val in values {
    println!("{}", val); // Uses delegated Display implementation
}
```

### JSON Example

This example creates a JSON-like data structure with nested arrays,
showcasing trait delegation and Polymorphic Vec features:

```rust
use std::fmt;

#[derive(Debug, Clone)]
pub struct Null;

impl fmt::Display for Null {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "null")
    }
}

#[derive(Debug, Clone)]
pub struct JsonArray(JsonValueVec);

impl fmt::Display for JsonArray {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = self.0.iter().map(ToString::to_string).collect::<Vec<_>>().join(", ");
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
    
    vec;

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


let mut values = JsonValueVec::default();
values.push(Null);
values.push(true);
values.push(42.0);
values.push("hello".to_string());
values.push(JsonArray(json_value_vec![Null, false, 33.0]));

for val in &values {
    println!("{}: {}", val.json_type_name(), val);
}

## Installation

Add `nodyn` to your `Cargo.toml`:

```toml
[dependencies]
nodyn = "0.2.0"
```

## Comparison

| Feature | nodyn | enum_dispatch | sum_type | Box<dyn Trait> |
|---------|-------|---------------|----------|----------------|
| **Runtime Cost** | Zero | Zero | Zero | Heap allocation |
| **Trait Delegation** | ✅ Yes | ✅ Scoped only | ❌ No | ✅ Yes |
| **Method Delegation** | ✅ Yes | ❌ No | ❌ No | ❌ No |
| **Type Introspection** | ✅ Built-in | ❌ No | ❌ No | ❌ No |
| **Vec Wrapper** | ✅ Yes | ❌ No | ❌ No | ❌ No |
| **Compile-Time Known** | Required | Required | Required | Not required |
| **Memory Overhead** | Discriminant only | Discriminant only | Discriminant only | Pointer + vtable |

- **[enum_dispatch](https://crates.io/crates/enum_dispatch)**: Optimizes dynamic dispatch with zero-cost enums but lacks method delegation and Vec wrappers.
- **[sum_type](https://crates.io/crates/sum_type)**: Simplifies enum creation but lacks advanced features like delegation or introspection.

## Documentation

- [API Documentation](https://docs.rs/nodyn)
- [Changelog](https://github.com/franklaranja/nodyn/blob/main/CHANGELOG.md)

## Contributing

Contributions are welcome! Check out the [GitHub repository](https://github.com/franklaranja/nodyn) for issues, feature requests, or to submit pull requests.

## License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.

---

<div align="center">

**[Documentation](https://docs.rs/nodyn) | [Crates.io](https://crates.io/crates/nodyn) | [Repository](https://github.com/franklaranja/nodyn)**

</div>
