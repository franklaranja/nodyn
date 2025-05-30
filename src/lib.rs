//! The Rust `nodyn::`[`wrap!`] macro creates a wrapper enum for a set of
//! types and can generate method and trait delegation.
//!
//! # Values of different Types in Rust
//!
//! When we want to have values of different types in Rust there are
//! two possible solutions: Trait Objects or Enum Wrappers. The second
//! option is a "good solution when our interchangeable items are a
//! fixed set of types that we know when our code is compiled"[^book].
// /html/book/ch18-02-trait-objects.html#using-trait-objects-that-allow-for-values-of-different-types
//!
//! ## Example
//!
//! [Listing 8-9][Listing_8-9] from the book[^book]:
// /html/book/ch08-01-vectors.html#using-an-enum-to-store-multiple-types
//!
//! ```rust
//!     enum SpreadsheetCell {
//!         Int(i32),
//!         Float(f64),
//!         Text(String),
//!     }
//!
//!     let row = vec![
//!         SpreadsheetCell::Int(3),
//!         SpreadsheetCell::Text(String::from("blue")),
//!         SpreadsheetCell::Float(10.12),
//!     ];
//! ```
//!
//! With nodyn, which implements `From` for each wrapped type:
//!
//! ```rust
//!     nodyn::wrap! {
//!         enum SpreadsheetCell { i32, f64, String }
//!     }
//!
//!     let row: Vec<SpreadsheetCell> = vec![
//!         3.into(),
//!         String::from("blue").into(),
//!         10.12.into(),
//!     ];
//! ```
//!
//! # Downsides of Enum Wrappers
//!
//! However, using an Enum Wrapper requires extra code to delegate
//! function calls. Adding types or functions requires a lot of changes
//! to the Enum Wrapper, bigger changes in comparison to Trait Objects.
//! The [`wrap!`] generates the delegation for you.
//!
//! ## Example
//!
//! Here is [Listing 10-13][Listing_10-13] from the book[^book]:
//!
//! ```rust
//!    pub trait Summary {
//!        fn summarize(&self) -> String;
//!    }
//!    
//!    pub struct NewsArticle {
//!        pub headline: String,
//!        pub location: String,
//!        pub author: String,
//!        pub content: String,
//!    }
//!    
//!    impl Summary for NewsArticle {
//!        fn summarize(&self) -> String {
//!            format!("{}, by {} ({})", self.headline, self.author, self.location)
//!        }
//!    }
//!    
//!    pub struct SocialPost {
//!        pub username: String,
//!        pub content: String,
//!        pub reply: bool,
//!        pub repost: bool,
//!    }
//!    
//!    impl Summary for SocialPost {
//!        fn summarize(&self) -> String {
//!            format!("{}: {}", self.username, self.content)
//!        }
//!    }
//! ```
//!
//! We can create an enum Wrapper `Article` that implements `Summery`
//! by delegating to `NewsArticle` or `SocialPost`:
//!
//! ```rust
//! # pub trait Summary {
//! #     fn summarize(&self) -> String;
//! # }
//! #
//! # pub struct NewsArticle {
//! #     pub headline: String,
//! #     pub location: String,
//! #     pub author: String,
//! #     pub content: String,
//! # }
//! #
//! # impl Summary for NewsArticle {
//! #     fn summarize(&self) -> String {
//! #         format!("{}, by {} ({})", self.headline, self.author, self.location)
//! #     }
//! # }
//! #
//! # pub struct SocialPost {
//! #     pub username: String,
//! #     pub content: String,
//! #     pub reply: bool,
//! #     pub repost: bool,
//! # }
//! #
//! # impl Summary for SocialPost {
//! #     fn summarize(&self) -> String {
//! #         format!("{}: {}", self.username, self.content)
//! #     }
//! # }
//!
//! nodyn::wrap! {
//!     enum Article {NewsArticle, SocialPost}
//!
//!     impl Summary {
//!         fn summarize(&self) -> String;
//!     }
//! }
//! ```
//!
//! See the documentation of the [`wrap!`] macro for details.
//!
//! # Alternative crates
//!
//! - **[enum_dispatch]**
//!     - can only generate delegation for traits in scope
//!       (but in a very convenient way).
//! - **[sum_type]**
//!     - very limited to the type of types being wrapped (e.g. no lifetimes)
//!     - no delegation
//!
//! [enum_dispatch]: https://crates.io/crates/enum_dispatch
//! [sum_type]: https://crates.io/crates/sum_type
//! # To do
//!
//! - [ ] strum like `EnumCount`
//!   ```ignore
//!        pub trait EnumCount {
//!            const COUNT: usize;
//!        }
//!    ```
//! - [ ] strum like `VariantArray`
//!   ```ignore
//!        pub trait VariantArray: Sized + 'static {
//!            const VARIANTS: &'static [Self];
//!        }
//!    ```
//! - [ ] strum like `VariantNames`
//!   ```ignore
//!        pub trait VariantNames {
//!            const VARIANTS: &'static [&'static str];
//!        }
//!    ```
//! - [ ] strum like `EnumIs`: Generated `is_*()` methods for each variant.
//! - [ ] strum like `TryAs`: Generated `try_as_*()` methods for all variants.
//!
//! [^book]: "The Rust Programming Language" by Steve Klabnik, Carol Nichols, and Chris Krycho, with contributions from the Rust Community
//!
//! [Listing_8-9]: http://localhost:3000/share/rust/html/book/ch08-01-vectors.html#listing-8-9
//! [Listing_10-13]: http://localhost:3000/share/rust/html/book/ch10-02-traits.html#listing-10-13

use proc_macro::TokenStream;
use quote::{quote, ToTokens};
use syn::spanned::Spanned;
use syn::{parse_macro_input, TypeArray, TypePath, TypeReference, TypeSlice};

mod impl_block;
mod nodyn_enum;
mod trait_block;
mod variant;

pub(crate) use impl_block::ImplBlock;
pub(crate) use nodyn_enum::NodynEnum;
pub(crate) use trait_block::TraitBlock;
pub(crate) use variant::Variant;

/// Creates a wrapper `enum` for a set of types and can generate method and
/// trait delegation.
///
/// # Variant types and names
///
/// Only Path, Reference, Array, Slice and Tuple types are allowed. The
/// variant names are created from the full path given converted
/// to camel case. Reference types have 'Ref', Arrays 'Array' + length
/// and Slices 'Slice' added.
///
/// ```
/// nodyn::wrap!{
///    #[derive(Debug)]
///    pub enum Foo<'a> {
///        i32,
///        String,
///        (u8, u8, u16),
///        [bool;2],
///        &'a str
///     }
/// }
///
/// let array: Foo = [true, false].into();
/// if let Foo::BoolArray2(inner) = array {
///     assert_eq!(inner, [true, false]);
/// } else {
///     unreachable!();
/// }
/// ```
///
/// You can define variant names just like in a regular `enum` definition,
/// to override the variant name:
///
/// ```
/// nodyn::wrap!{
///    #[derive(Debug)]
///    pub enum Foo {
///        String,
///        Tuple((u8, u8, u16)),
///        Bools([bool;2]),
///     }
/// }
///
/// let array: Foo = [true, false].into();
/// if let Foo::Bools(inner) = array {
///     assert_eq!(inner, [true, false]);
/// } else {
///     unreachable!();
/// }
/// ```
///
/// # Automatic generated functions
///
/// - `const fn count() -> usize`
///
///   Returns the number of types (variants).
///
///   TODO: example
///
/// - `const fn types() -> [&'static str;N]`
///
///   Returns all the type names
///
///   TODO: example
///
/// - `const fn ty(&self) -> &'static str`
///
///   Returns the name if the type.
///
///   TODO: example
///
/// The following functions are generated for each
/// varient using the snake_cased variant(=type) name.
///
/// - `fn as_\[variant](self) -> Option<T>`
///
///   Returns the wrapped value as T. When you annotate a
///   varient with #[into(T)], where T is another varient,
///   the varient will be returned by both `as_variant()` and
///   and `as_other_varient()`.
///
///   TODO: example
///
/// -`as_\[variant]_ref(&self) -> Option<&T>`
///
///   Returns a reference to the wrapped value.
///
///   TODO: example
///   
///  - `as_\[variant]_mut(&mut self) -> Option<&T>`
///
///   Returns a mutable reference to the wrapped value.
///
///   TODO: example
///
/// # Automatic generated implementated traits
///
/// - `From<T> for Wrapper` for all variant types
///
///   TODO: example
///
/// - `TryFrom<Wrapper> for T` automatic for all non reference types
///
///    When `T` has a `From<O>` implementation then you can add the
///    attribute `#[into(T)]` to `O` and it will return a `Some(T)` for it.
///
///    ```
///    use core::convert::TryFrom;
///
///    nodyn::wrap! {
///        #[derive(PartialEq, Debug)]
///        pub enum Foo<'a> {
///            i64,
///            &'a str,
///            u32,
///            [u8;4],
///        }
///    }
///
///    let t2: Foo = 42u32.into();
///    assert_eq!(t2, Foo::U32(42));
///    let r2 = u32::try_from(t2);
///    assert_eq!(r2, Ok(42u32));
///    ```
///
/// # Function delegation
///
/// When all types included implement a method, a delegation method
/// can be generated by including the method signature as in a trait definition:
/// with the block replaced by a semicolon in `impl`.
///
/// In addition `impl` blocks accept all other types of impl items,
/// those are passed unchanged.
///
/// TODO: example
///
/// # Trait implementation
///
/// When all types included implement a trait. The trait can be implemented
/// for the wrapper by providing the signatures of the required functions.
/// A trait block starts with `impl` followed by the trait name.
/// Other impl items are passed unchanged.
///
/// TODO: example

#[proc_macro]
pub fn wrap(input: TokenStream) -> TokenStream {
    let nodyn_enum = parse_macro_input!(input as NodynEnum);

    let e_num = nodyn_enum.generate_enum();
    let from = nodyn_enum.generate_from();
    let try_into = nodyn_enum.generate_try_from();
    let impl_blocks = nodyn_enum.generate_impl_blocks();
    let trait_blocks = nodyn_enum.generate_trait_blocks();
    let type_fns = nodyn_enum.generate_type_to_str();
    let is_as_fn = nodyn_enum.generate_is_as().unwrap();

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

fn ident_from_path(p: &syn::Path, extension: &str) -> syn::Ident {
    let idents: Option<Vec<String>> = p
        .segments
        .iter()
        .map(|p| {
            let ident = p.ident.to_string();
            let mut chars = ident.chars();
            chars
                .next()
                .map(|first| format!("{}{}{extension}", first.to_uppercase(), chars.as_str()))
        })
        .collect();
    idents
        .map(|s| syn::Ident::new(&s.concat(), p.span()))
        .expect("Could not generate ident")
}

fn syn_to_ident<T: ToTokens>(t: T) -> String {
    let input = t.to_token_stream().to_string();
    input
        .split_whitespace()
        .map(|word| {
            let filtered = word
                .chars()
                .filter(|c| c.is_alphanumeric())
                .collect::<String>();
            let mut chars = filtered.chars();
            chars
                .next()
                .map(|first| format!("{}{}", first.to_uppercase(), chars.as_str()))
                .expect("Could not uppercase first letter")
        })
        .collect::<Vec<String>>()
        .concat()
}

fn path_from_type(ty: &syn::Type) -> Option<&syn::Path> {
    match ty {
        syn::Type::Path(TypePath { path, .. }) => Some(path),
        syn::Type::Reference(TypeReference { elem, .. })
        | syn::Type::Array(TypeArray { elem, .. })
        | syn::Type::Slice(TypeSlice { elem, .. }) => path_from_type(elem),
        _ => None,
    }
}
