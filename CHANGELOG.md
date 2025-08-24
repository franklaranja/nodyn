# Changelog

All notable changes to `nodyn`, a Rust procedural macro for creating wrapper
enums that encapsulate a fixed set of types, will be documented in this
file.

See the [nodyn GitHub repository](https://github.com/franklaranja/nodyn) for more details, or contribute to this changelog via [GitHub issues](https://github.com/franklaranja/nodyn/issues).

## [Unreleased]

No changes yet.

## 0.2.1

### Bug fixes

- When generating code for the polymorphic `Vec`s the macro
  checked necessary traits against the Vec wrapper and not the enum
  this has been fixed. only `Default` is checked for the vec wrapper.
- Implementations of `Extend` for the vec wrapper are now always generated.

## [0.2.0] - 2025-08-01

### Added

- **Polymorphic Support**: Added a `vec` feature to generate a `Vec<Enum>` wrapper with
  delegated `Vec` methods and variant-specific utilities (e.g., `first_i32`, `count_string`).
  This allows direct insertion of wrapped types via `Into<Enum>` and provides a `vec!`-like
  macro for easy initialization. See the [Polymorphic Vec section](https://github.com/franklaranja/nodyn#polymorphic-vec)
  in the documentation for details.

  ```rust
  nodyn::nodyn! {
      #[derive(Debug, PartialEq, Clone)]
      pub enum Foo<'a> {
          &'a str,
          u32,
      }
      vec;
  }

  let mut inventory = foo_vec!["sword", 100u32, "shield"];
  inventory.push(50u32); // Directly push u32
  assert_eq!(inventory.count_str_ref(), 2); // Count &str variants
  for (i, s) in inventory.iter_str_ref().enumerate() {
      println!("String #{}: {}", i, s); // Iterate over &str variants
  }
  ```

- **Explicit Feature Selection with `impl`**: Introduced `impl` directives to enable
  features per enum (`TryInto`, `is_as`, `introspection`), providing fine-grained
  control over generated code. See the [Feature Flags section](https://github.com/franklaranja/nodyn#feature-flags)
  for usage.

  ```rust
  nodyn::nodyn! {
      enum Value { i32, String }
      impl TryInto is_as;
  }

  let value: Value = 42.into();
  assert!(value.is_i32());
  let num: i32 = i32::try_from(value).unwrap();
  ```

### Changed

- **Feature Selection Mechanism**: Replaced Cargo feature flags with `impl`
  directives for enabling features like `TryInto`, `is_as` and `introspection`.
  This allows explicit control per enum definition, improving clarity and
  flexibility.
- **Feature Naming**: Renamed the `try_into` feature to `TryInto` for
  consistency when using `impl` directives.
- **From Trait**: The `from` feature is no longer optional; `From<T>` is
  now always implemented for each variant type.

### Deprecated

- **Cargo Feature Flags**: The use of Cargo features (`try_into`, `is_as`, `introspection`)
  to control code generation is deprecated. These are still supported for backward
  compatibility if no `impl` directives are specified, but users should migrate
  to `impl` directives. See the [Feature Flags section](https://github.com/franklaranja/nodyn#feature-flags)
  for migration guidance.

## [0.1.0] - 2025-05-02

### Added

- `nodyn` provides a Rust procedural macro for creating wrapper enums that
  encapsulate a fixed set of types with automatic implementations
  for `From` and generation of delegating methods and traits.
- Automatic variant creation for specified types with CamelCase naming (e.g., `i32` becomes `I32`).
- Method and trait delegation to underlying types for seamless polymorphism.
- Type introspection methods (`count`, `types`, `type_name`) with the `introspection` feature.
- Variant-specific accessors (`is_*`, `try_as_*`) with the `is_as` feature.
- `TryFrom<Enum>` support for non-reference types with the `try_into` feature.
- Support for complex types, including references, arrays, and tuples.
- Released at [v0.1.0](https://github.com/franklaranja/nodyn/releases/tag/v0.1.0).

[Unreleased]: https://github.com/franklaranja/nodyn/compare/v0.2.0...HEAD
[0.2.0]: https://github.com/franklaranja/nodyn/releases/tag/v0.2.0
[0.1.0]: https://github.com/franklaranja/nodyn/releases/tag/v0.1.0
