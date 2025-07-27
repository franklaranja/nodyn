//! *Easy polymorphism with enums*
//!
//! The `nodyn!` macro generates a Rust `enum` that wraps a fixed set of [types](#supported-types), providing automatic
//! implementations for type conversions, [method delegation](#method-delegation), and [trait delegation](#trait-delegation). This enables
//! type-safe storage of different types without the runtime overhead of trait objects, ideal for
//! scenarios requiring zero-cost abstractions for a known set of types at compile time.
//!
//! ## Why Use Enum Wrappers?
//!
//! In Rust, handling values of different types typically involves:
//! - **Trait Objects**: Enable dynamic dispatch but incur runtime overhead and type erasure.
//! - **Enum Wrappers**: Offer type safety and zero-cost abstractions for a fixed set of types,
//!   as described in [The Rust Programming Language][book].
//!
//! The `nodyn!` macro simplifies [creating enum wrappers](#syntax) by generating boilerplate for variant creation,
//! type conversions, method/trait delegation, and introspection utilities.
//! `nodyn!` can also generate a special [polymorphic `Vec`](#vec-wrapper) for your `enum`.
//!
//! ## Key Features
//!
//! - **Automatic Variant Creation**: Generates an enum with variants for specified types.
//! - **Type Conversion**: Implements `From<T>` for each variant type and optionally
//!   `TryFrom<Enum>` for `T` for non-reference types (with [`TryInto`](#tryfrom-trait-with-tryinto) feature).
//!   Additional type conversion tools can be generated with the [`as_is`](#type-checking-and-conversion-methods-with-is_as) feature
//! - **Method and Trait Delegation**: [Delegates methods](#method-delegation) or entire [traits](#trait-delegation) to underlying types.
//! - **Type Introspection**: Provides `count`, `types`, and `type_name` methods to query variant
//!   information (with [introspection](#type-information-methods-with-introspection) feature).
//! - **Custom Variant Names**: Allows overriding default variant names for clarity.
//! - **Vec Wrapper**: Generates a `Vec<Enum>` wrapper with [delegated `Vec` methods](#delegated-vec-methods-and-traits) and
//!   and [extra functionality](#variant-methods-and-traits) such as [construction macro](#a-vec-like-macro) to leverage the enums features.
//!
//! ## Basic Example
//!
//! ```rust
//! nodyn::nodyn! {
//!     #[derive(Debug, PartialEq)]
//!     pub enum Value {
//!         i32,
//!         String,
//!         f64,
//!     }
//! }
//!
//! let values: Vec<Value> = vec![
//!     42.into(),
//!     "hello".to_string().into(),
//!     3.14.into(),
//! ];
//!
//! for value in values {
//!     match value {
//!         Value::I32(n) => println!("Integer: {}", n),
//!         Value::String(s) => println!("String: {}", s),
//!         Value::F64(f) => println!("Float: {}", f),
//!     }
//! }
//! ```
//!
//! ## Trait Delegation Example
//!
//! Here's an example inspired by [Listing 10-13][listing-10-13] from [The Rust Programming Language][book], demonstrating trait delegation:
//!
//! ```rust
//! pub trait Summary {
//!     fn summarize(&self) -> String;
//! }
//!
//! #[derive(Debug)]
//! pub struct NewsArticle {
//!     pub headline: String,
//!     pub location: String,
//!     pub author: String,
//!     pub content: String,
//! }
//!
//! impl Summary for NewsArticle {
//!     fn summarize(&self) -> String {
//!         format!("{}, by {} ({})", self.headline, self.author, self.location)
//!     }
//! }
//!
//! #[derive(Debug)]
//! pub struct SocialPost {
//!     pub username: String,
//!     pub content: String,
//!     pub reply: bool,
//!     pub repost: bool,
//! }
//!
//! impl Summary for SocialPost {
//!     fn summarize(&self) -> String {
//!         format!("{}: {}", self.username, self.content)
//!     }
//! }
//!
//! nodyn::nodyn! {
//!     #[derive(Debug)]
//!     pub enum Article {
//!         NewsArticle,
//!         SocialPost,
//!     }
//!
//!     impl Summary {
//!         fn summarize(&self) -> String;
//!     }
//! }
//!
//! let article = Article::from(NewsArticle {
//!     headline: String::from("Rust 2.0 Released"),
//!     location: String::from("Internet"),
//!     author: String::from("Rust Team"),
//!     content: String::from("..."),
//! });
//!
//! assert_eq!(
//!     article.summarize(),
//!     "Rust 2.0 Released, by Rust Team (Internet)"
//! );
//! ```
//!
//! ## Supported Types
//!
//! The macro supports these type categories with automatic variant name generation:
//!
//! | Type Category | Example | Generated Variant | Notes |
//! |--------------|---------|-------------------|-------|
//! | **Path types** | `String`, `i32`, `Vec<T>` | `String`, `I32`, `VecT` | CamelCase conversion |
//! | **References** | `&str` | `StrRef` | Adds `Ref` suffix |
//! | **Arrays** | `[i32; 4]` | `I32Array4` | Adds `Array{len}` suffix |
//! | **Tuples** | `(i32, String)` | `I32String` | Concatenates types |
//!
//! ### Complex Types Example
//!
//! ```rust
//! nodyn::nodyn! {
//!     #[derive(Debug)]
//!     pub enum ComplexEnum<'a> {
//!         i32,                    // I32
//!         String,                 // String
//!         (u8, u16),              // U8U16
//!         [bool; 2],              // BoolArray2
//!         &'a str,                // StrRef
//!         Vec<String>,            // VecString
//!     }
//!     vec;
//! }
//!
//! let values = complex_enum_vec![
//!     42i32,
//!     "hello",
//!     (1u8, 2u16),
//!     [true, false],
//!     vec!["a".to_string()],
//! ];
//! ```
//!
//! [book]: https://doc.rust-lang.org/book/ch18-02-trait-objects.html#using-trait-objects-that-allow-for-values-of-different-types
//! [listing-10-13]: https://doc.rust-lang.org/book/ch10-02-traits.html#listing-10-13
//!
//! # Syntax
//!
//! ```ignore
//! nodyn::nodyn! {
//!     [ #[attribute] ]
//!     [ #[module_path = "full::module::path"]]
//!     [pub] enum EnumName [<'lifetime>] {
//!         [VariantName(Type),]
//!         [Type,]
//!     }
//!
//!     [impl TryInto | is_as | introspection]
//!
//!     [impl TraitName {
//!         fn method_name(&self, args) -> ReturnType;
//!     }]
//!
//!     [impl {
//!         fn method_name(&self, args) -> ReturnType;
//!     }]
//!
//!     [vec [VecName]]
//!
//!     [ #[vec] | #[vec(field)]
//!       [ #[attribute] ]
//!       [pub] struct CustomValues {
//!           [field: Type,]
//!       }
//!     ]
//! }
//! ```
//!
//! ## Generated Methods
//!
//! ### Type Information Methods (with `introspection`)
//!
//! ```rust
//! nodyn::nodyn! {
//!     enum Value { i32, String, f64 }
//!     impl introspection;
//! }
//!
//! assert_eq!(Value::count(), 3);
//! assert_eq!(Value::types(), ["i32", "String", "f64"]);
//! let val: Value = 42.into();
//! assert_eq!(val.type_name(), "i32");
//! ```
//!
//! ### Type Checking and Conversion Methods (with `is_as`)
//!
//! ```rust
//! nodyn::nodyn! {
//!     enum Container { String, Vec<u8> }
//!     impl is_as;
//! }
//!
//! let container: Container = "hello".to_string().into();
//! assert!(container.is_string());
//! assert!(!container.is_vec_u8());
//! if let Some(s) = container.try_as_string() {
//!     println!("Got string: {}", s);
//! }
//! let container: Container = "hello".to_string().into();
//! if let Some(s_ref) = container.try_as_string_ref() {
//!     println!("String reference: {}", s_ref);
//! }
//! ```
//!
//! Note: `*_ref()` and `*_mut()` methods are not generated for variants that wrap references.
//!
//! ## Automatic Trait Implementations
//!
//! ### From Trait
//!
//! ```rust
//! nodyn::nodyn! {
//!     enum Value { i32, String }
//! }
//!
//! let num: Value = 42.into();
//! let text: Value = "hello".to_string().into();
//! ```
//!
//! ### `TryFrom` Trait (with `TryInto`)
//!
//! ```rust
//! nodyn::nodyn! {
//!     enum Value { i32, String }
//!     impl TryInto;
//! }
//!
//! let val: Value = 42.into();
//! let num: i32 = i32::try_from(val).unwrap();
//! assert_eq!(num, 42);
//! ```
//!
//! ### `#[into(T)]` Attribute
//!
//! **`#[into(T)]` Attribute**: Allows a variant to be converted into another
//! type `T` if a `From` implementation and variant exists.
//!
//! ```rust
//! nodyn::nodyn! {
//!     pub enum Foo {
//!         i64,
//!         #[into(i64)]
//!         i32,
//!     }
//!     impl TryInto;
//! }
//!
//! let foo: Foo = 42.into();
//! assert_eq!(i64::try_from(foo), Ok(42i64));
//! ```
//!
//! # Method Delegation
//!
//! When all wrapped types implement a method with the same signature, you can
//! delegate it in an `impl` block:
//!
//! ```rust
//! nodyn::nodyn! {
//!     enum Container { String, Vec<u8> }
//!
//!     impl {
//!         // Delegate methods that exist on all types
//!         fn len(&self) -> usize;
//!         fn is_empty(&self) -> bool;
//!         fn clear(&mut self);
//!     }
//! }
//!
//! let mut container: Container = "hello".to_string().into();
//! assert_eq!(container.len(), 5);
//! assert!(!container.is_empty());
//! ```
//!
//! # Trait Delegation
//!
//! When all wrapped types implement a trait, you can implement it for the wrapper
//! by delegating the required methods:
//!
//! ```rust
//! use std::fmt::{self, Display};
//!
//! // All wrapped types implement Display
//! nodyn::nodyn! {
//!     enum Displayable { i32, String, f64 }
//!
//!     impl Display {
//!         fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result;
//!     }
//!
//!     vec Displayables;
//! }
//!
//! let values = displayables![42, "hello".to_string(), 3.14];
//!
//! for val in values {
//!     println!("{}", val); // Uses delegated Display implementation
//! }
//! ```
//!
//! ## `Vec` Wrapper
//!
//! The `vec` feature generates a wrapper around a [`std::vec::Vec<Enum>`][std::vec::Vec], implementing
//! many `Vec` methods and variant-specific accessors (e.g., `first_number` for an `i32` variant).
//! Methods like `push` and `insert` leverage `Into<Enum>`, allowing direct insertion of wrapped types.
//!
//! ### Example
//!
//! ```rust
//! nodyn::nodyn! {
//!     #[derive(Debug, Clone)]
//!     pub enum Value {
//!         i32,
//!         String,
//!         f64,
//!     }
//!     vec;
//! }
//!
//! let mut values = ValueVec::default();
//! values.push(42);
//! values.push("hello".to_string());
//! assert_eq!(values.first_i32(), Some(&42));
//! assert_eq!(values.len(), 2);
//! ```
//!
//! ### Simple Vec Wrappers
//!
//! Using `impl vec` generates a wrapper named after the enum with the extension Vec.
//! The generated wrapper has the same derive attributes as the enum, plus
//! `Default`. If the enum is `Copy` that won't be included in the derive
//! for the wrapper. The visibility of the wrapper and its methods is the
//! same as the enums.
//!
//! You can specify a name for the wrapper:
//!
//! ```rust
//! nodyn::nodyn! {
//!     #[derive(Debug, PartialEq, Clone)]
//!     pub enum Value {
//!         i32,
//!         String,
//!         f64,
//!     }
//!
//!     vec Values;
//! }
//! ```
//!
//! ### A `vec!` like macro
//!
//! Nodyn generates a macro for your vec wrapper with the name
//! of the wrapper changed to snake case. As the `nodyn!` macro
//! does not know where it is invoked, you have to tell it the
//! full module path including the crate name, so the generated
//! macro works correctly. If you don't specify the module path
//! it is assumed the vec wrapper and enum are in the local scope.
//!
//! The macro works like `vec!` but accepts any value within your
//! enum.
//!
//! The macro requires that the wrapper has the `#[derive('Default)]`
//! attribute (the standard vec wrapper always has this).
//!
//! #### Example
//!
//! ```ignore
//! // in src/my/awsome/foo.rs:
//! nodyn::nodyn! {
//!     #[module_path = "my::awsome::foo"]
//!     #[derive(Debug, Clone)]
//!     pub enum Value<'a> {
//!         i32,
//!         &'a str,
//!         f64,
//!     }
//!
//!     vec Values;
//! }
//!
//! // elsewhere after importing values, etc:
//! let my_values = values!["hello", 42, "world", 0.1];
//! ```
//!
//! ### Custom Vec Wrappers
//!
//! Define a custom wrapper struct with additional fields using the
//! `#[vec]` or `#[vec(field_name)]` attribute. Without
//! a field name 'inner' is used. `nodyn!` adds the field. Unlike the
//! standard vec wrapper, derive arguments are not copied from the enum.
//! `nodyn!` does implement neither `Deref` nor `DerefMut`, so you can!
//!
//! I recommend you put a `#[derive('Default)]` on your custom vec wrapper
//! so nodyn can generate the macro and implement the From trait.
//!
//! #### Example
//!
//! ```rust
//! nodyn::nodyn! {
//!     #[derive(Debug, Clone)]
//!     pub enum Value {
//!         i32,
//!         String,
//!     }
//!
//!     #[vec(inner_vec)]
//!     #[derive(Debug, Clone)]
//!     pub struct CustomValues {
//!         metadata: String,
//!     }
//! }
//!
//! let mut values = CustomValues {
//!     metadata: "example".to_string(),
//!     inner_vec: vec![],
//! };
//! values.push(42);
//! assert_eq!(values.metadata, "example");
//! assert_eq!(values.len(), 1);
//! ```
//!
//! ### Variant methods and traits
//!
//! For each variant the following methods are generated for the wrapper:
//!
//! | method            | required traits | notes       |
//! |-------------------|------|------------------------|
//! | `all_*`           | none |   |
//! | `any_*`           | none |   |
//! | `count_*`         | none | Counts all items of variant, iteratimg over whole vec |
//! | `enumerate_*_mut` | none | idem.                  |
//! | `enumerate_*`     | none | Enumerate with index from the wrapper |
//! | `first_*_mut`     | none | [`first_mut`][core::slice:first_mut()]|
//! | `first_*`         | none | [`first`][core::slice:first()]    |
//! | `iter_*_mut`      | none | [`std::slice:last_mut()`] |
//! | `iter_*`          | none | [`std::slice:last()`]     |
//! | `last_*_mut`      | none | [`std::slice:last_mut()`] |
//! | `last_*`          | none | [`std::slice:last()`]     |
//!
//! And the following traits for each variant with type V:
//!
//! | trait             | required trait      |
//! |-------------------|---------------------|
//! | `Extend<V>`       | `Clone`             |
//! | `From<&[V]>`      | `Default` & `Clone` |
//! | `From<&mut [V]>`  | `Default` & `Clone` |
//! | `From<Vec<V>>`    | `Default`           |
//!
//! ### Delegated `Vec` methods and traits
//!
//! the `vec` wrapper implements many [`std::vec::Vec`] methods and traits, with some modified to
//! leverage `nodyn` features. the following table summarizes them:
//!
//! | method | required traits | differences from [`std::vec::Vec`] |
//! |--------|-----------------|------------------------------------|
//! | [`append`][std::vec::Vec::append()]                     | none | none; direct delegation. |
//! | [`as_mut_slice`][std::vec::Vec::as_mut_slice()]         | none | none; direct delegation. |
//! | [`as_slice`][std::vec::Vec::as_slice()]                 | none | none; direct delegation. |
//! | [`binary_search_by_key`][std::vec::Vec::binary_search_key()] | none | none; direct delegation. |
//! | [`binary_search_by`][std::vec::Vec::binary_search()]    | none    | none; direct delegation. |
//! | [`binary_search`][std::vec::Vec::binary_search()]       | `Ord`   | none; direct delegation. |
//! | [`capacity`][std::vec::Vec::capacity()]                 | none | none; direct delegation. |
//! | [`clear`][std::vec::Vec::clear()]                       | none | none; direct delegation. |
//! | [`clone_from_slice`][std::vec::Vec::clone_from_slice()] | `Clone` | none; direct delegation. |
//! | [`copy_from_slice`][std::vec::Vec::copy_from_slice()]   | `Copy`  | none; direct delegation. |
//! | [`copy_within`][std::vec::Vec::copy_within()]           | `Copy`  | none; direct delegation. |
//! | [`dedup_by_key`][std::vec::Vec::dedup_by_key()]         | none | none; direct delegation. |
//! | [`dedup_by`][std::vec::Vec::dedup_by()]                 | none | none; direct delegation. |
//! | [`dedup`][std::vec::Vec::dedup()]                       | `PartialEq` | none; direct delegation. |
//! | [`extend_from_slice`][std::vec::Vec::extend_from_slice()]   | `Clone` | none; direct delegation. |
//! | [`extend_from_within`][std::vec::Vec::extend_from_within()] | `Clone` | none; direct delegation. |
//! | [`extract_if`][std::vec::Vec::extract_if()]             | none | none; direct delegation. |
//! | [`fill_with`][std::vec::Vec::fill_with()]               | none    | accepts `Into<enum>`. |
//! | [`fill`][std::vec::Vec::fill()]                         | `Clone` | none; direct delegation. |
//! | [`first_mut`][std::vec::Vec::first_mut()]               | none | none; direct delegation. |
//! | [`first`][std::vec::Vec::first()]                       | none | none; direct delegation. |
//! | [`get_mut`][std::vec::Vec::get_mut()]                   | none | none; direct delegation. |
//! | [`get`][std::vec::Vec::get()]                           | none | none; direct delegation. |
//! | [`insert`][std::vec::Vec::insert()]                     | none | accepts `Into<enum>`. |
//! | [`into_boxed_slice`][std::vec::Vec::into_boxed_slice()] | none | none; direct delegation. |
//! | [`is_empty`][std::vec::Vec::is_empty()]                 | none | none; direct delegation. |
//! | [`is_sorted_by_key`][std::vec::Vec::is_sorted_key()]    | none | none; direct delegation. |
//! | [`is_sorted_by`][std::vec::Vec::is_sorted()]            | none | none; direct delegation. |
//! | [`is_sorted`][std::vec::Vec::is_sorted()]               | `PartialOrd`    | none; direct delegation. |
//! | [`iter_mut`][std::vec::Vec::iter_mut()]                 | none | none; direct delegation. |
//! | [`iter`][std::vec::Vec::iter()]                         | none | none; direct delegation. |
//! | [`last_mut`][std::vec::Vec::last_mut()]                 | none | none; direct delegation. |
//! | [`last`][std::vec::Vec::last()]                         | none | none; direct delegation. |
//! | [`len`][std::vec::Vec::len()]                           | none | none; direct delegation. |
//! | [`new`][std::vec::Vec::new()]                           | `Default`   | initializes other fields with `Default::default()`. |
//! | [`pop_if`][std::vec::Vec::pop_if()]                     | none | none; direct delegation. |
//! | [`pop`][std::vec::Vec::pop()]                           | none | none; direct delegation. |
//! | [`push`][std::vec::Vec::push()]                         | none | accepts `Into<enum>`. |
//! | [`remove`][std::vec::Vec::remove()]                     | none | none; direct delegation. |
//! | [`reserve_exact`][std::vec::Vec::reserve_exact()]       | none | none; direct delegation. |
//! | [`reserve`][std::vec::Vec::reserve()]                   | none | none; direct delegation. |
//! | [`resize`][std::vec::Vec::resize()]                     | `Clone`     | accepts `Into<enum>`. |
//! | [`retain_mut`][std::vec::Vec::retain_mut()]             | none | none; direct delegation. |
//! | [`retain`][std::vec::Vec::retain()]                     | none | none; direct delegation. |
//! | [`reverse`][std::vec::Vec::reverse()]                   | none | none; direct delegation. |
//! | [`rotate_left`][std::vec::Vec::rotate_left()]           | none    | none; direct delegation. |
//! | [`rotate_right`][std::vec::Vec::rotate_right()]         | none    | none; direct delegation. |
//! | [`shrink_to_fit`][std::vec::Vec::shrink_to_fit()]       | none | none; direct delegation. |
//! | [`shrink_to`][std::vec::Vec::shrink_to()]               | none | none; direct delegation. |
//! | [`sort_by_key`][std::vec::Vec::sort_key()]              | none    | none; direct delegation. |
//! | [`sort_by`][std::vec::Vec::sort()]                      | none    | none; direct delegation. |
//! | [`sort_unstable_by_key`][std::vec::Vec::sort_unstable_key()] | none | none; direct delegation. |
//! | [`sort_unstable_by`][std::vec::Vec::sort_unstable()]    | none    | none; direct delegation. |
//! | [`sort_unstable`][std::vec::Vec::sort_unstable()]       | `Ord`   | none; direct delegation. |
//! | [`sort`][std::vec::Vec::sort()]                         | `Ord`   | none; direct delegation. |
//! | [`splice`][std::vec::Vec::splice()]                     | none | none; direct delegation. |
//! | [`split_first_mut`][std::vec::Vec::split_first_mut()]   | none | none; direct delegation. |
//! | [`split_first`][std::vec::Vec::split_first()]           | none | none; direct delegation. |
//! | [`split_last_mut`][std::vec::Vec::split_last_mut()]     | none | none; direct delegation. |
//! | [`split_last`][std::vec::Vec::split_last()]             | none | none; direct delegation. |
//! | [`split_off`][std::vec::Vec::split_off()]               | `Default`   | initializes other fields with `Default::default()`. |
//! | [`swap_remove`][std::vec::Vec::swap_remove()]           | none | none; direct delegation. |
//! | [`swap`][std::vec::Vec::swap()]                         | none | none; direct delegation. |
//! | [`to_vec`][std::vec::Vec::to_vec()]                     | `Clone` | none; direct delegation. |
//! | [`truncate`][std::vec::Vec::truncate()]                 | none | none; direct delegation. |
//! | [`try_reserve_exact`][std::vec::Vec::try_reserve_exact()]   | none | none; direct delegation. |
//! | [`try_reserve`][std::vec::Vec::try_reserve()]           | none | none; direct delegation. |
//! | [`with_capacity`][std::vec::Vec::with_capacity()]       | `Default`   | initializes other fields with `Default::default()`. |
//!
//! | trait | required traits | differences from [`std::vec::Vec`] |
//! |-------|-----------------|------------------------------------|
//! | [`AsMut<Self>`][std::convert::AsMut]            | none | returns `&mut self`. |
//! | [`AsMut<Vec<enum>>`][std::convert::AsMut]       | none | delegates to `vec`. |
//! | [`AsMut<[enum]>`][std::convert::AsMut]          | none | delegates to `vec`. |
//! | [`AsRef<Self>`][std::convert::AsRef]            | none | returns `&self`. |
//! | [`AsRef<Vec<enum>>`][std::convert::AsRef]       | none | delegates to `vec`. |
//! | [`AsRef<[enum]>`][std::convert::AsRef]          | none | delegates to `vec`. |
//! | [`Extend<enum>`][std::iter::Extend]             | `Clone` | delegates to `vec::extend`. |
//! | [`From<&[enum]>`][std::convert::From]           | `Clone`, `Default` | initializes other fields with `default::default()`. |
//! | [`From<&mut [enum]>`][std::convert::From]       | `Clone`, `Default` | initializes other fields with `default::default()`. |
//! | [`From<Self>`][std::convert::From]              | none | converts to `Vec<enum>`. |
//! | [`From<Vec<enum>>`][std::convert::From]         | `Default` | initializes other fields with `default::default()`. |
//! | [`Fromiterator<enum>`][std::iter::FromIterator] | `Default` | initializes other fields with `default::default()`. |
//! | [`IndexMut`][std::ops::IndexMut]                | none | delegates to `vec::index_mut`. |
//! | [`Index`][std::ops::Index]                      | none | delegates to `vec::index`. |
//! | [`IntoIterator`]       | none | implemented for `&self`, `&mut self`, and `self`. |
//!
//! ## Feature Flags
//!
//! ### Using `impl` (Recommended)
//!
//! Specify features within the macro using `impl TryInto`, `impl is_as`, `impl introspection`, or `impl vec`.
//! These are disabled by default, allowing explicit control.
//!
//! ### Using Cargo Features (Deprecated)
//!
//! If no `impl` features are specified, the macro falls back to Cargo feature flags for backward compatibility.
//! All Cargo features are enabled by default.
//!
//! | Cargo Feature   | `impl` Equivalent | Enables |
//! |-----------------|-------------------|---------|
//! | `from`          | None              | From 0.2.0 no longer optional |
//! | `try_into`      | `TryInto`         | Automatic `TryFrom` trait implementation |
//! | `introspection` | `introspection`   | Type introspection methods (`count`, `types`, `type_name`) |
//! | `is_as`         | `is_as`           | Variant test and accessor methods (`is_*`, `try_as_*`) |
//!
//! To transition from Cargo features, replace feature flags in `Cargo.toml` with `impl` directives in the macro.
//!
//! # Advanced Example
//!
//! ```
//! use std::fmt;
//!
//! #[derive(Debug, Clone)]
//! pub struct Null;
//!
//! impl fmt::Display for Null {
//!     fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
//!         write!(f, "null")
//!     }
//! }
//!
//! #[derive(Debug, Clone)]
//! pub struct JsonArray(JsonValueVec);
//!
//! impl fmt::Display for JsonArray {
//!     fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
//!         let s = self.0.iter().map(ToString::to_string).collect::<Vec<_>>().join(", ");
//!         write!(f, "[{s}]")
//!     }
//! }
//!
//! nodyn::nodyn! {
//!     #[derive(Debug, Clone)]
//!     pub enum JsonValue {
//!         Null,
//!         bool,
//!         f64,
//!         String,
//!         JsonArray,
//!     }
//!     
//!     vec;
//!
//!     impl fmt::Display {
//!         fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result;
//!     }
//!
//!     impl {
//!         pub const fn json_type_name(&self) -> &'static str {
//!             match self {
//!                 Self::Null(_) => "null",
//!                 Self::Bool(_) => "boolean",
//!                 Self::F64(_) => "number",
//!                 Self::String(_) => "string",
//!                 Self::JsonArray(_) => "array",
//!             }
//!         }
//!     }
//! }
//!
//!
//! let mut values = JsonValueVec::default();
//! values.push(Null);
//! values.push(true);
//! values.push(42.0);
//! values.push("hello".to_string());
//! values.push(JsonArray(json_value_vec![Null, false, 33.0]));
//!
//! for val in &values {
//!     println!("{}: {}", val.json_type_name(), val);
//! }

use proc_macro::TokenStream;
use quote::quote;
use syn::parse_macro_input;

mod method_impl;
mod nodyn_enum;
mod optional_impl;
mod trait_impl;
mod variant;
mod vec_wrapper;

pub(crate) use method_impl::MethodImpl;
pub(crate) use nodyn_enum::NodynEnum;
pub(crate) use optional_impl::OptionalImpl;
pub(crate) use trait_impl::TraitImpl;
pub(crate) use variant::{Variant, camel_to_snake};
pub(crate) use vec_wrapper::VecWrapper;

/// Creates a wrapper `enum` for a set of types with automatic method and trait delegation.
#[allow(clippy::missing_panics_doc)]
#[proc_macro]
pub fn nodyn(input: TokenStream) -> TokenStream {
    let nodyn_enum = parse_macro_input!(input as NodynEnum);

    let e_num = nodyn_enum.to_enum_definition();
    let standard_impl = nodyn_enum.to_standard_impl();
    let optional_impl = nodyn_enum.to_optional_impl();
    let method_impls = nodyn_enum.to_method_impls();
    let trait_impls = nodyn_enum.to_trait_impls();
    let vec_wrappers = nodyn_enum
        .vec_wrappers
        .iter()
        .map(|s| s.to_token_stream(&nodyn_enum))
        .collect::<Vec<_>>();

    let expanded = quote! {
        #e_num
        #standard_impl
        #optional_impl
        #(#method_impls)*
        #(#trait_impls)*
        #(#vec_wrappers)*
    };

    TokenStream::from(expanded)
}

pub(crate) mod keyword {
    syn::custom_keyword!(vec);
    // syn::custom_keyword!(From);
    syn::custom_keyword!(TryInto);
    syn::custom_keyword!(is_as);
    syn::custom_keyword!(introspection);
    // syn::custom_keyword!(from);
    // syn::custom_keyword!(str);
}

use proc_macro2::{Ident, Span};
use syn::{Generics, Lifetime};

/// Extension trait for managing generics in macro code generation.
pub(crate) trait GenericsExt {
    /// Checks if the generics include a specific lifetime.
    fn contains_lifetime(&self, other: &str) -> bool;
    /// Checks if the generics include a specific type parameter.
    fn contains_type(&self, other: &str) -> bool;
    /// Generates a new, unused lifetime.
    fn new_lifetime(&self) -> Lifetime;
    /// Generates a new, unused type identifier.
    fn new_type(&self) -> Ident;
    /// Generates multiple new, unused type identifiers.
    fn new_types(&self, count: u8) -> Vec<Ident>;
}

impl GenericsExt for Generics {
    fn contains_lifetime(&self, other: &str) -> bool {
        let other = Lifetime::new(other, Span::call_site());
        self.lifetimes().any(|l| l.lifetime == other)
    }

    fn contains_type(&self, other: &str) -> bool {
        let other = Ident::new(other, Span::call_site());
        self.type_params().any(|t| t.ident == other)
    }

    fn new_lifetime(&self) -> Lifetime {
        for c in ('a'..='z').rev() {
            let l = format!("'{c}");
            if !self.contains_lifetime(&l) {
                return Lifetime::new(&l, Span::call_site());
            }
        }
        panic!("no new lifetime available");
    }

    fn new_type(&self) -> Ident {
        for c in 'A'..='Z' {
            let l = c.to_string();
            if !self.contains_type(&l) {
                return Ident::new(&l, Span::call_site());
            }
        }
        panic!("no new lifetime available");
    }

    fn new_types(&self, mut count: u8) -> Vec<Ident> {
        let mut result = Vec::new();
        for c in 'A'..='Z' {
            let l = c.to_string();
            if !self.contains_type(&l) {
                result.push(Ident::new(&l, Span::call_site()));
                count -= 1;
                if count == 0 {
                    return result;
                }
            }
        }
        result
    }
}
