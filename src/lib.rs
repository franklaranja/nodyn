//! *Easy polymorphism with enums*
//!
//! The `nodyn!` macro generates a Rust `enum` that wraps a fixed set
//! of types, providing automatic implementations for `From`, `TryFrom`,
//! and delegated methods or traits. This is useful when you need to store
//! values of different types in a type-safe way without the overhead of trait
//! objects.
//!
//! ## Example
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
//! ## Why Use Enum Wrappers?
//!
//! In Rust, when you need to handle values of different types, you have two
//! primary options:
//!
//! - **Trait Objects**: Allow dynamic dispatch but incur runtime overhead and
//!   type erasure.
//! - **Enum Wrappers**: Provide type safety and zero-cost abstractions for a
//!   fixed set of types known at compile time, as described in [The Rust Programming Language][book].
//!
//! The `nodyn::nodyn!` macro simplifies the creation of enum wrappers by
//! generating boilerplate code for variant creation, type conversions,
//! and method/trait delegation.
//!
//! ## Key Features
//!
//! - **Automatic Variant Creation**: Generates an enum with variants for each specified type.
//! - **Type Conversion**: Implements `From<T>` for each variant type and `TryFrom<Enum> for T`
//!   for non-reference types. (Available with the `from` and `try_into` features)
//! - **Method and Trait Delegation**: Delegates methods or entire traits to the underlying types.
//! - **Type Introspection**: Provides utility methods like `count()`, `types()`, and `ty()` to
//!   query variant information. (available the `introspection` feature)
//! - **Custom Variant Names**: Allows overriding default variant names for clarity.
//!
//! ## Example with Trait Delegation
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
//! let article = Article::NewsArticle(NewsArticle {
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
//! }
//!
//! let values = vec![
//!     ComplexEnum::from(42i32),
//!     ComplexEnum::from("hello"),
//!     ComplexEnum::from((1u8, 2u16)),
//!     ComplexEnum::from([true, false]),
//!     ComplexEnum::from(vec!["a".to_string()]),
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
//!     [pub] enum EnumName [<'lifetime>] {
//!         [VariantName(Type),]
//!         [Type,]
//!     }
//!
//!     [impl TraitName {
//!         fn method_name(&self, args) -> ReturnType;
//!     }]
//!
//!     [impl {
//!         fn method_name(&self, args) -> ReturnType;
//!     }]
//! }
//! ```
//!
//! - **Enum Definition**: Define the enum with optional visibility, derive attributes, and lifetimes.
//! - **Variants**: Specify types directly (e.g., `i32`, `String`) or with custom variant names (e.g., `Int(i32)`).
//! - **Trait Delegation**: Include trait `impl` blocks to delegate trait methods to wrapped types.
//! - **Method Delegation**: Include regular `impl` blocks to delegate custom methods.
//!
//! # Variant Types and Naming
//!
//! The macro supports various type categories and automatically generates variant names:
//!
//! ## Supported Types
//!
//! - **Path types**: `i32`, `String`, `Vec<T>`, `Option<T>`, etc.
//! - **Reference types**: `&T`, `&mut T`, `&'a str`
//! - **Array types**: `[T; N]`
//! - **Tuple types**: `(T1, T2, ...)`
//!
//! ## Automatic Variant Naming
//!
//! ```rust
//! nodyn::nodyn! {
//!     #[derive(Debug)]
//!     pub enum Example<'a> {
//!         i32,           // → I32(i32)
//!         String,        // → String(String)
//!         (u8, u16),     // → U8U16((u8, u16))
//!         [bool; 2],     // → BoolArray2([bool; 2])
//!         &'a str,       // → StrRef(&'a str)
//!     }
//! }
//! ```
//!
//! ## Custom Variant Names
//!
//! Override automatic names by specifying them explicitly:
//!
//! ```rust
//! nodyn::nodyn! {
//!     enum CustomNames {
//!         Text(String),           // Custom name: Text
//!         Numbers([i32; 3]),      // Custom name: Numbers
//!         i32,                    // Auto name: I32
//!     }
//! }
//! ```
//!
//! # Generated Methods
//!
//! ## Type Information Methods
//!
//! Available with the `introspection` feature.
//!
//! ```rust
//! nodyn::nodyn! {
//!     enum Value { i32, String, f64 }
//! }
//!
//! // Number of variants
//! assert_eq!(Value::count(), 3);
//!
//! // Array of type names
//! assert_eq!(Value::types(), [ "i32", "String", "f64"]);
//!
//! // Get type name of current value
//! let val: Value = 42.into();
//! assert_eq!(val.type_name(), "i32");
//! ```
//!
//! ## Type Checking and Conversion Methods
//!
//! Available with the `is_as` feature.
//!
//! For each variant, the following methods are generated (using snake case names):
//!
//! ```rust
//! nodyn::nodyn! {
//!     enum Container { String, Vec<u8> }
//! }
//!
//! let container: Container = "hello".to_string().into();
//!
//! // Type checking
//! assert!(container.is_string());
//! assert!(!container.is_vec_u8());
//!
//! // Value extraction (consumes self)
//! if let Some(s) = container.try_as_string() {
//!     println!("Got string: {}", s);
//! }
//!
//! // Reference extraction (doesn't consume self)
//! let container: Container = "hello".to_string().into();
//! if let Some(s_ref) = container.try_as_string_ref() {
//!     println!("String reference: {}", s_ref);
//! }
//!
//! // Mutable reference extraction
//! let mut container: Container = "hello".to_string().into();
//! if let Some(s_mut) = container.try_as_string_mut() {
//!     s_mut.push_str(" world");
//! }
//! ```
//!
//! Note: `*_ref()` and `*_mut()` methods are not generated for variants that wrap references.
//!
//! # Automatic Trait Implementations
//!
//! ## From Trait
//!
//! Automatic `From<T>` implementations for all variant types:
//!
//! Available with the `from` feature.
//!
//! ```rust
//! nodyn::nodyn! {
//!     enum Value { i32, String }
//! }
//!
//! let num: Value = 42.into();          // From<i32>
//! let text: Value = "hello".to_string().into(); // From<String>
//! ```
//!
//! ## `TryFrom` Trait
//!
//! Automatic `TryFrom<Wrapper>` implementations for extracting original types:
//!
//! Available with the `try_into` feature.
//!
//! ```rust
//! use std::convert::TryFrom;
//!
//! nodyn::nodyn! {
//!     enum Value { i32, String }
//! }
//!
//! let val: Value = 42.into();
//! let num: i32 = i32::try_from(val).unwrap();
//! assert_eq!(num, 42);
//!
//! let val: Value = "hello".to_string().into();
//! let text: String = String::try_from(val).unwrap();
//! assert_eq!(text, "hello");
//! ```
//!
//! **`#[into(T)]` Attribute**: Allows a variant to be converted into another
//! type `T` if a `From` implementation and variant exists.
//!
//! ```rust
//!   nodyn::nodyn! {
//!       pub enum Foo {
//!           i64,
//!           #[into(i64)]
//!           i32,
//!       }
//!   }
//!   let foo: Foo = 42.into();
//!   assert_eq!(i64::try_from(foo), Ok(42i64));
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
//! # Trait Implementation
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
//! }
//!
//! let values: Vec<Displayable> = vec![
//!     42.into(),
//!     "hello".to_string().into(),
//!     3.14.into(),
//! ];
//!
//! for val in values {
//!     println!("{}", val); // Uses delegated Display implementation
//! }
//! ```
//!
//! # Advanced Example
//!
//! ```rust
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
//! pub struct JsonArray(Vec<JsonValue>);
//!
//! impl fmt::Display for JsonArray {
//!     fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
//!         let s = self
//!             .0
//!             .iter()
//!             .map(ToString::to_string)
//!             .collect::<Vec<_>>()
//!             .join(", ");
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
//! let values: Vec<JsonValue> = vec![
//!     Null.into(),                // null
//!     true.into(),                // boolean
//!     42.0.into(),                // number
//!     "hello".to_string().into(), // string
//!     JsonArray(vec![
//!         Null.into(),
//!         false.into(),
//!         33.0.into(),
//!         "world".to_string().into(),
//!     ]) .into(),
//! ];
//!
//! for val in &values {
//!     println!("{}: {}", val.json_type_name(), val);
//! }
//!
//! // null: null
//! // boolean: true
//! // number: 42
//! // string: hello
//! // array: [null, false, 33, world]
//! ```
//!
//! # Features
//!
//! All features are enabled by default.
//!
//! | feature | enables |
//! |-------|-------|
//! | `from`          | automatic From trait implementation |
//! | `try_into`      | automatic TryFrom trait implementation |
//! | `introspection` | generation of type introspection functions |
//! | `is_as`         | generation of variant test and accessor functions |
use proc_macro::TokenStream;
use quote::quote;
use syn::parse_macro_input;

mod impl_block;
mod nodyn_enum;
mod trait_block;
mod variant;

pub(crate) use impl_block::ImplBlock;
pub(crate) use nodyn_enum::NodynEnum;
pub(crate) use trait_block::TraitBlock;
pub(crate) use variant::Variant;

/// Creates a wrapper `enum` for a set of types with automatic method and trait delegation.
#[allow(clippy::missing_panics_doc)]
#[proc_macro]
pub fn nodyn(input: TokenStream) -> TokenStream {
    let nodyn_enum = parse_macro_input!(input as NodynEnum);

    let e_num = nodyn_enum.generate_enum();
    #[cfg(feature = "from")]
    let from = nodyn_enum.generate_from();
    #[cfg(not(feature = "from"))]
    let from = Vec::<&str>::new();
    #[cfg(feature = "try_into")]
    let try_into = nodyn_enum.generate_try_from();
    #[cfg(not(feature = "try_into"))]
    let try_into = Vec::<&str>::new();
    let impl_blocks = nodyn_enum.generate_impl_blocks();
    let trait_blocks = nodyn_enum.generate_trait_blocks();
    #[cfg(feature = "introspection")]
    let type_fns = nodyn_enum.generate_type_to_str();
    #[cfg(not(feature = "introspection"))]
    let type_fns = "";
    #[cfg(feature = "is_as")]
    let is_as_fn = nodyn_enum.generate_is_as().unwrap();
    #[cfg(not(feature = "is_as"))]
    let is_as_fn = "";

    let expanded = quote! {
        #e_num
        #(#from)*
        #(#try_into)*
        #(#impl_blocks)*
        #(#trait_blocks)*
        #type_fns
        #is_as_fn
    };

    TokenStream::from(expanded)
}
