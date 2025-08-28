//! <div align="center">
//!
//! *Easy polymorphism with enums*
//!
//! </div>
//!
//! The `nodyn!` macro generates a Rust `enum` that wraps a fixed set of [types](#supported-types), providing automatic
//! implementations for [type conversions](#type-conversions-and-introspection), [method delegation](#method-delegation), and [trait delegation](#trait-delegation). This enables
//! type-safe storage of different types without the runtime overhead of trait objects, ideal for
//! scenarios requiring zero-cost abstractions for a known set of types at compile time.
//!
//! ## Why use Nodyn?
//!
//! In Rust, handling values of different types typically involves:
//! - **Trait Objects**: Enable dynamic dispatch but incur runtime overhead and type erasure.
//! - **Enum Wrappers**: Offer type safety and zero-cost abstractions for a fixed set of types,
//!   as described in [The Rust Programming Language][book].
//!
//! The `nodyn!` macro simplifies [creating enum wrappers](#syntax) by generating boilerplate for variant creation,
//! type conversions, method/trait delegation, and introspection utilities.
//! `nodyn!` can also generate a special [polymorphic `Vec`](#polymorphic-vec) for your `enum`.
//!
//! ## Quick Start
//!
//! Create a simple enum wrapper for `i32`, `String`, and `f64`:
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
//!     42.into(),                  // Converts i32 to Value::I32
//!     "hello".to_string().into(), // Converts String to Value::String
//!     3.14.into(),                // Converts f64 to Value::F64
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
//! ## Key Features
//!
//! - **Automatic Variant Creation**: Generates an enum with variants for specified types.
//! - **Type Conversion**: Implements `From<T>` for each variant type and optionally
//!   `TryFrom<Enum>` for `T` for non-reference types (with [`TryInto`](#from-and-tryfrom) feature).
//!   Additional type conversion tools can be generated with the [`as_is`](#type-checking-and-conversion-methods-with-is_as) feature
//! - **Method and Trait Delegation**: [Delegates methods](#method-delegation) or entire [traits](#trait-delegation) to underlying types.
//! - **Type Introspection**: Provides `count`, `types`, and `type_name` methods to query variant
//!   information (with [introspection](#introspection-methods-with-introspection) feature).
//! - **Custom Variant Names**: Allows overriding default variant names for clarity.
//! - **Polymorphic `Vec`s**: Generates a `Vec<Enum>` wrapper with [delegated `Vec` methods](#delegated-vec-methods-and-traits) and
//!   and [extra functionality](#variant-methods-and-traits) such as [construction macro](#a-vec-like-macro) to leverage the enums features.
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
//! [book]: https://doc.rust-lang.org/book/ch18-02-trait-objects.html#using-trait-objects-that-allow-for-values-of-different-types
//! [listing-10-13]: https://doc.rust-lang.org/book/ch10-02-traits.html#listing-10-13
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
//! ## Type Conversions and Introspection
//!
//! ### `From` and `TryFrom`
//!
//! The macro automatically implements `From<T>` for each variant type.
//! With the `TryInto` feature, it also implements `TryFrom<Enum>`
//! for non-reference types.
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
//! ### Introspection Methods (with `introspection`)
//!
//! Enable type introspection with the `introspection` feature to query variant information:
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
//! The `is_as` feature generates methods like `is_*` and `try_as_*`
//! for variant-specific checks and conversions:
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
//! ## Method and Trait Delegation
//!
//! ### Method Delegation
//!
//! Delegate methods that exist on all wrapped types with the same signature:
//!
//! ```rust
//! nodyn::nodyn! {
//!     enum Container { String, Vec<u8> }
//!     impl {
//!         fn len(&self) -> usize;
//!         fn is_empty(&self) -> bool;
//!         fn clear(&mut self);
//!     }
//! }
//!
//! let mut container: Container = "hello".to_string().into();
//! assert_eq!(container.len(), 5);
//! assert!(!container.is_empty());
//! container.clear();
//! assert!(container.is_empty());
//! ```
//!
//! # Trait Delegation
//!
//! Delegate entire traits when all wrapped types implement them:
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
//! See the [JSON Example](#json-example) for a practical application of trait delegation.
//!
//! ## Polymorphic `Vec`
//!
//! The `vec` feature generates a `Vec<Enum>` wrapper with delegated `Vec`
//! methods and variant-specific utilities. It supports flexible insertion
//! via `Into<Enum>` and provides methods like `first_*`, `count_*`,
//! and `all_*` for variant-specific access.
//!
//! A `vec!`-like macro is also generated for easy initialization.
//!
//! ### Basic Polymorphic Vec
//!
//! Using `vec` generates a wrapper named after the enum with the suffix `Vec`.
//! The generated wrapper has the same derive attributes as the enum, plus
//! `Default`. If the enum is `Copy`, that won't be included in the derive
//! for the wrapper. The visibility of the wrapper and its methods matches the enum's.
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
//! values.push(42);                    // Accepts i32
//! values.push("hello".to_string());   // Accepts String
//! values.push(3.14);                  // Accepts f64
//! assert_eq!(values.first_i32(), Some(&42));
//! assert_eq!(values.len(), 3);
//! assert_eq!(values.count_string(), 1);
//! ```
//!
//! You can specify a custom name for the wrapper:
//!
//! ```rust
//! nodyn::nodyn! {
//!     #[derive(Debug, Clone)]
//!     pub enum Value {
//!         i32,
//!         String,
//!     }
//!
//!     /// A polymorphic vec wrapper around `Vec<Value>`.
//!     vec Values;
//! }
//!
//! let values = values![42, "hello".to_string()];
//! ```
//!
//! ## Polymorphic Vec Features
//!
//! ### Filtering and Iteration
//!
//! ```rust
//! nodyn::nodyn! {
//!     #[derive(Debug, Clone, PartialEq)]
//!     pub enum Data<'a> {
//!         i32,
//!         &'a str,
//!         bool,
//!     }
//!     vec;
//! }
//!
//! let mut data = data_vec![42, "hello", true, 99, "world", false];
//!
//! for number in data.iter_i32() {
//!     println!("Number: {}", number);  // Prints: 42, 99
//! }
//!
//! assert_eq!(data.count_str_ref(), 2);
//! assert_eq!(data.count_bool(), 2);
//!
//! assert!(!data.all_i32());  // Not all items are i32
//! assert!(data.any_str_ref()); // At least one string exists
//! ```
//!
//! ### Construction from Slices
//!
//! ```rust
//! nodyn::nodyn! {
//!     #[derive(Debug, Clone)]
//!     pub enum Number {
//!         i32,
//!         f64,
//!     }
//!     vec Numbers;
//! }
//!
//! // Construct from homogeneous slices
//! let integers: &[i32] = &[1, 2, 3, 4, 5];
//! let mut numbers = Numbers::from(integers);
//!
//! let floats = vec![1.1, 2.2, 3.3];
//! numbers.extend(floats);  // Extends with Vec<f64> via Into
//!
//! assert_eq!(numbers.count_i32(), 5);
//! assert_eq!(numbers.count_f64(), 3);
//! ```
//!
//!
//! ## A `vec!`-like Macro
//!
//! Nodyn generates a macro for your polymorphic vec with the name
//! of the wrapper changed to snake case. As the `nodyn!` macro
//! does not know where it is invoked, you have to tell it the
//! full module path including the crate name, so the generated
//! macro works correctly. If you don't specify the module path,
//! it is assumed the polymorphic vec and enum are in the local scope.
//!
//! The macro works like `vec!` but accepts any value within your
//! enum and uses `Into` for automatic conversion.
//!
//! The macro requires that the wrapper has `#[derive(Default)]`
//! (the standard polymorphic vec always has this).
//!
//! ### Example
//!
//! ```ignore
//! // in my_crate/src/foo/bar.rs:
//! nodyn::nodyn! {
//!     #[module_path = "my_crate::foo::bar"]
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
//! ## Custom Polymorphic Vecs
//!
//! Define a custom wrapper struct with additional fields using the
//! `#[vec]` or `#[vec(field_name)]` attribute. Without
//! a field name, 'inner' is used. `nodyn!` adds the field. Unlike the
//! standard polymorphic vec, derive arguments are not copied from the enum.
//! `nodyn!` does not implement `Deref` or `DerefMut` for custom wrappers,
//! so you can! (but you have to call `as_slice` yourself).
//!
//! I recommend you put `#[derive(Default)]` on your custom polymorphic vec
//! so nodyn can generate the macro and implement the `From` trait.
//!
//! ### Example
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
//!     #[derive(Debug, Clone, Default)]
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
//! ## Variant Methods and Traits
//!
//! For each variant, the following methods are generated for the wrapper:
//!
//! | Method            | Required Traits | Description |
//! |-------------------|-----------------|-------------|
//! | `all_*`           | none | Returns `true` if all items are of this variant |
//! | `any_*`           | none | Returns `true` if any item is of this variant |
//! | `count_*`         | none | Counts all items of this variant |
//! | `enumerate_*`     | none | Enumerate items of this variant with their indices |
//! | `enumerate_*_mut` | none | Enumerate mutable items of this variant with their indices |
//! | `first_*`         | none | Returns reference to first item of this variant |
//! | `first_*_mut`     | none | Returns mutable reference to first item of this variant |
//! | `iter_*`          | none | Iterator over items of this variant |
//! | `iter_*_mut`      | none | Mutable iterator over items of this variant |
//! | `last_*`          | none | Returns reference to last item of this variant |
//! | `last_*_mut`      | none | Returns mutable reference to last item of this variant |
//!
//! And the following traits for each variant with type `V`:
//!
//! | Trait             | Required Trait(*)  | Description |
//! |-------------------|-----------------|-------------|
//! | `Extend<V>`       |                 | Extend wrapper with items of this variant |
//! | `From<&[V]>`      | `Default` & `Clone` | Create wrapper from slice of this variant |
//! | `From<&mut [V]>`  | `Default` & `Clone` | Create wrapper from mutable slice |
//! | `From<Vec<V>>`    | `Default`       | Create wrapper from `Vec` of this variant |
//!
//! (*) Default is required for the `Vec` wrapper, other traits are required for the enum.
//!
//! ### Game Inventory Example
//!
//! This example demonstrates the power of the Polymorphic Vec in a game
//! inventory context:
//!
//! ```rust
//! nodyn::nodyn! {
//!     #[derive(Debug, Clone)]
//!     pub enum Item {
//!         i32,    // Gold coins
//!         String, // Weapon names
//!         f64,    // Health potions (liters)
//!     }
//!     vec Inventory;
//! }
//!
//! let mut inventory = inventory![100, "sword".to_string(), 0.5, "axe".to_string()];
//! // Add more gold
//! inventory.push(50);
//! // Check for weapons in the inventory
//! assert!(inventory.any_string());
//! // Total gold coins
//! let total_gold = inventory.iter_i32().sum::<i32>();
//! assert_eq!(total_gold, 150);
//! // Get a potion
//! if let Some(potion) = inventory.first_f64() {
//!     println!("Found potion: {} liters", potion); // Prints: 0.5 liters
//! }
//! ```
//!
//! ## Delegated `Vec` Methods and Traits
//!
//! The `vec` wrapper implements many [`Vec`] methods and traits, with some modified to
//! leverage `nodyn` features. Methods that directly delegate to slice methods are
//! only implemented for custom wrappers as standard wrappers handle this using `Deref` and
//! `DerefMut`.
//!
//! | Method | Required Traits(*) | Differences from [`Vec`] |
//! |--------|-----------------|----------------------------------------|
//! | [`append`][Vec::append] | none | none; direct delegation |
//! | [`as_mut_slice`][Vec::as_mut_slice] | none | none; direct delegation |
//! | [`as_slice`][Vec::as_slice] | none | none; direct delegation |
//! | [`binary_search_by_key`][slice::binary_search_by_key] | none | none; direct delegation |
//! | [`binary_search_by`][slice::binary_search_by] | none | none; direct delegation |
//! | [`binary_search`][slice::binary_search] | `Ord` | none; direct delegation |
//! | [`capacity`][Vec::capacity] | none | none; direct delegation |
//! | [`clear`][Vec::clear] | none | none; direct delegation |
//! | [`clone_from_slice`][slice::clone_from_slice] | `Clone` | none; direct delegation |
//! | [`copy_from_slice`][slice::copy_from_slice] | `Copy` | none; direct delegation |
//! | [`copy_within`][slice::copy_within] | `Copy` | none; direct delegation |
//! | [`dedup_by_key`][Vec::dedup_by_key] | none | none; direct delegation |
//! | [`dedup_by`][Vec::dedup_by] | none | none; direct delegation |
//! | [`dedup`][Vec::dedup] | `PartialEq` | none; direct delegation |
//! | [`extend_from_slice`][Vec::extend_from_slice] | `Clone` | none; direct delegation |
//! | [`extend_from_within`][Vec::extend_from_within] | `Clone` | none; direct delegation |
//! | [`extract_if`][Vec::extract_if] | none | none; direct delegation |
//! | [`fill_with`][slice::fill_with] | none | accepts `Into<enum>` |
//! | [`fill`][slice::fill] | `Clone` | none; direct delegation |
//! | [`first_mut`][slice::first_mut] | none | none; direct delegation |
//! | [`first`][slice::first] | none | none; direct delegation |
//! | [`get_mut`][slice::get_mut] | none | none; direct delegation |
//! | [`get`][slice::get] | none | none; direct delegation |
//! | [`insert`][Vec::insert] | none | accepts `Into<enum>` |
//! | [`into_boxed_slice`][Vec::into_boxed_slice] | none | none; direct delegation |
//! | [`is_empty`][Vec::is_empty] | none | none; direct delegation |
//! | [`is_sorted_by_key`][slice::is_sorted_by_key] | none | none; direct delegation |
//! | [`is_sorted_by`][slice::is_sorted_by] | none | none; direct delegation |
//! | [`is_sorted`][slice::is_sorted] | `PartialOrd` | none; direct delegation |
//! | [`iter_mut`][slice::iter_mut] | none | none; direct delegation |
//! | [`iter`][slice::iter] | none | none; direct delegation |
//! | [`last_mut`][slice::last_mut] | none | none; direct delegation |
//! | [`last`][slice::last] | none | none; direct delegation |
//! | [`len`][Vec::len] | none | none; direct delegation |
//! | [`new`][Vec::new] | `Default` | initializes other fields with `Default::default()` |
//! | [`pop_if`][Vec::pop_if] | none | none; direct delegation |
//! | [`pop`][Vec::pop] | none | none; direct delegation |
//! | [`push`][Vec::push] | none | accepts `Into<enum>` |
//! | [`remove`][Vec::remove] | none | none; direct delegation |
//! | [`reserve_exact`][Vec::reserve_exact] | none | none; direct delegation |
//! | [`reserve`][Vec::reserve] | none | none; direct delegation |
//! | [`resize`][Vec::resize] | `Clone` | accepts `Into<enum>` |
//! | [`retain_mut`][Vec::retain_mut] | none | none; direct delegation |
//! | [`retain`][Vec::retain] | none | none; direct delegation |
//! | [`reverse`][slice::reverse] | none | none; direct delegation |
//! | [`rotate_left`][slice::rotate_left] | none | none; direct delegation |
//! | [`rotate_right`][slice::rotate_right] | none | none; direct delegation |
//! | [`shrink_to_fit`][Vec::shrink_to_fit] | none | none; direct delegation |
//! | [`shrink_to`][Vec::shrink_to] | none | none; direct delegation |
//! | [`sort_by_key`][slice::sort_by_key] | none | none; direct delegation |
//! | [`sort_by`][slice::sort_by] | none | none; direct delegation |
//! | [`sort_unstable_by_key`][slice::sort_unstable_by_key] | none | none; direct delegation |
//! | [`sort_unstable_by`][slice::sort_unstable_by] | none | none; direct delegation |
//! | [`sort_unstable`][slice::sort_unstable] | `Ord` | none; direct delegation |
//! | [`sort`][slice::sort] | `Ord` | none; direct delegation |
//! | [`splice`][Vec::splice] | none | none; direct delegation |
//! | [`split_first_mut`][slice::split_first_mut] | none | none; direct delegation |
//! | [`split_first`][slice::split_first] | none | none; direct delegation |
//! | [`split_last_mut`][slice::split_last_mut] | none | none; direct delegation |
//! | [`split_last`][slice::split_last] | none | none; direct delegation |
//! | [`split_off`][Vec::split_off] | `Default` | initializes other fields with `Default::default()` |
//! | [`swap_remove`][Vec::swap_remove] | none | none; direct delegation |
//! | [`swap`][slice::swap] | none | none; direct delegation |
//! | [`to_vec`][slice::to_vec] | `Clone` | none; direct delegation |
//! | [`truncate`][Vec::truncate] | none | none; direct delegation |
//! | [`try_reserve_exact`][Vec::try_reserve_exact] | none | none; direct delegation |
//! | [`try_reserve`][Vec::try_reserve] | none | none; direct delegation |
//! | [`with_capacity`][Vec::with_capacity] | `Default` | initializes other fields with `Default::default()` |
//!
//! (*) Default is required for the `Vec` wrapper, other traits are required for the enum.
//!
//! | trait | required traits(*) | differences from [`std::vec::Vec`] |
//! |-------|-----------------|------------------------------------|
//! | [`AsMut<Self>`][std::convert::AsMut]            | none | returns `&mut self`. |
//! | [`AsMut<Vec<enum>>`][std::convert::AsMut]       | none | delegates to `vec`. |
//! | [`AsMut<[enum]>`][std::convert::AsMut]          | none | delegates to `vec`. |
//! | [`AsRef<Self>`][std::convert::AsRef]            | none | returns `&self`. |
//! | [`AsRef<Vec<enum>>`][std::convert::AsRef]       | none | delegates to `vec`. |
//! | [`AsRef<[enum]>`][std::convert::AsRef]          | none | delegates to `vec`. |
//! | [`Deref`][std::ops::Deref](**)                  | none | |
//! | [`DerefMut`][std::ops::DerefMut](**)                  | none | |
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
//!
//! (*) Default is required for the `Vec` wrapper, other traits are required for the enum.
//!
//! (**) Only implemented for standard vec wrappers.
//!
//! ## Feature Flags
//!
//! ### Using `impl` (Recommended)
//!
//! Specify features within the macro using `impl TryInto`, `impl is_as`, `impl introspection`, or `vec`.
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
//! The cargo features will be removed in version 0.3.0.
//!
//! ### JSON Example
//!
//! This example creates a JSON-like data structure with nested arrays,
//! showcasing trait delegation and Polymorphic Vec features:
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
use proc_macro2::{Ident, Span};
use syn::{GenericParam, Generics, Lifetime, parse_macro_input};

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
    TokenStream::from(nodyn_enum.to_token_stream())
}

pub(crate) mod keyword {
    syn::custom_keyword!(vec);
    syn::custom_keyword!(TryInto);
    syn::custom_keyword!(is_as);
    syn::custom_keyword!(introspection);
}

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
    fn type_generics_tokens(&self) -> proc_macro2::TokenStream;
    fn merged_type_generics_tokens(&self, other: &Self) -> proc_macro2::TokenStream;
    fn merged_generics_tokens(&self, other: &Self) -> proc_macro2::TokenStream;
    fn merged2_generics_tokens(&self, other1: &Self, other2: &Self) -> proc_macro2::TokenStream;
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

    fn type_generics_tokens(&self) -> proc_macro2::TokenStream {
        let (_, type_generics, _) = self.split_for_impl();
        quote::quote! { #type_generics }
    }

    fn merged_type_generics_tokens(&self, other: &Self) -> proc_macro2::TokenStream {
        let lifetimes = self
            .params
            .iter()
            .filter_map(|parameter| {
                if let GenericParam::Lifetime(lifetime) = parameter {
                    Some(&lifetime.lifetime)
                } else {
                    None
                }
            })
            .chain(other.params.iter().filter_map(|parameter| {
                if let GenericParam::Lifetime(lifetime) = parameter {
                    Some(&lifetime.lifetime)
                } else {
                    None
                }
            }))
            .collect::<Vec<_>>();
        let types = self
            .params
            .iter()
            .filter_map(|parameter| {
                if let GenericParam::Type(ty) = parameter {
                    Some(&ty.ident)
                } else {
                    None
                }
            })
            .chain(other.params.iter().filter_map(|parameter| {
                if let GenericParam::Type(ty) = parameter {
                    Some(&ty.ident)
                } else {
                    None
                }
            }))
            .collect::<Vec<_>>();
        if lifetimes.is_empty() && types.is_empty() {
            proc_macro2::TokenStream::new()
        } else if lifetimes.is_empty() {
            quote::quote! { < #(#types,)* >}
        } else if types.is_empty() {
            quote::quote! { < #(#lifetimes,)* >}
        } else {
            quote::quote! { < #(#lifetimes,)* #(#types,)* > }
        }
    }

    fn merged_generics_tokens(&self, other: &Self) -> proc_macro2::TokenStream {
        let lifetimes = self
            .params
            .iter()
            .filter(|p| matches!(p, GenericParam::Lifetime(_)))
            .chain(
                other
                    .params
                    .iter()
                    .filter(|p| matches!(p, GenericParam::Lifetime(_))),
            )
            .collect::<Vec<_>>();
        let types = self
            .params
            .iter()
            .filter(|p| matches!(p, GenericParam::Type(_)))
            .chain(
                other
                    .params
                    .iter()
                    .filter(|p| matches!(p, GenericParam::Type(_))),
            )
            .collect::<Vec<_>>();
        if lifetimes.is_empty() && types.is_empty() {
            proc_macro2::TokenStream::new()
        } else if lifetimes.is_empty() {
            quote::quote! { < #(#types,)* >}
        } else if types.is_empty() {
            quote::quote! { < #(#lifetimes,)* >}
        } else {
            quote::quote! { < #(#lifetimes,)* #(#types,)* > }
        }
    }

    fn merged2_generics_tokens(&self, other1: &Self, other2: &Self) -> proc_macro2::TokenStream {
        let lifetimes = self
            .params
            .iter()
            .filter(|p| matches!(p, GenericParam::Lifetime(_)))
            .chain(
                other1
                    .params
                    .iter()
                    .filter(|p| matches!(p, GenericParam::Lifetime(_))),
            )
            .chain(
                other2
                    .params
                    .iter()
                    .filter(|p| matches!(p, GenericParam::Lifetime(_))),
            )
            .collect::<Vec<_>>();
        let types = self
            .params
            .iter()
            .filter(|p| matches!(p, GenericParam::Type(_)))
            .chain(
                other1
                    .params
                    .iter()
                    .filter(|p| matches!(p, GenericParam::Type(_))),
            )
            .chain(
                other2
                    .params
                    .iter()
                    .filter(|p| matches!(p, GenericParam::Type(_))),
            )
            .collect::<Vec<_>>();
        if lifetimes.is_empty() && types.is_empty() {
            proc_macro2::TokenStream::new()
        } else if lifetimes.is_empty() {
            quote::quote! { < #(#types,)* >}
        } else if types.is_empty() {
            quote::quote! { < #(#lifetimes,)* >}
        } else {
            quote::quote! { < #(#lifetimes,)* #(#types,)* > }
        }
    }
}

// #[cfg(test)]
// mod tests {
//     use super::*;
//     use syn::parse_str;
//
//     #[test]
//     fn test_trait_derived() {
//         let input = parse_str::<NodynEnum>(
//             "
//             #[derive(Debug)]
//             #[derive(Default, Clone)]
//             pub enum MyEnum {
//                 Number(i32),
//                 String,
//             }
//             ",
//         )
//         .unwrap();
//         assert!(trait_derived(&input.attrs, "Default"));
//         assert!(trait_derived(&input.attrs, "Clone"));
//         assert!(trait_derived(&input.attrs, "Debug"));
//         assert!(!trait_derived(&input.attrs, "Copy"));
//         assert!(!trait_derived(&input.attrs, "Foo"));
//     }
// }
