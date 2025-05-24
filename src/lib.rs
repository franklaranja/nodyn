//! The `nodyn::wrap!` **procedural macro** creates an enum for a set
//! of types and generates method and trait implementations. It can
//! also generate collection types for the created `enum`.
//!
//! # Generated code
//!
//! ## Variant types and names
//!
//! Only Path, Reference, Array and Slice types are allowed. The
//! variant names are created from the full path given converted
//! to camel case. Reference types have 'Ref', Arrays 'Arrey'
//! and Slices 'Slice'.
//!
//! ## Automatic generated trait implementations
//!
//! 1. `From<T> for Wrapper` for all variant types
//! 2. `TryFrom<Wrapper> for T` automatic for all non reference types
//!
//! # Alternative crates
//!
//! - **`enum_dispatch`**
//!     - uses procedural macros
//!     - automatic `From` & `TryInto`
//!     - crate defined traits
//!     - only generate trait implementations for traits in scope
//!       in a very conveniant way.
//! - **`sum_type`**
//!     - declaritative macros
//!     - automatic `From` & `TryFrom`
//!     - no lifetimes
//!     - does not generate method or trait implementations
//!
//!

use proc_macro::TokenStream;
use quote::{ToTokens, quote};
use syn::spanned::Spanned;
use syn::{TypeArray, TypePath, TypeReference, TypeSlice, parse_macro_input};

mod impl_block;
mod multi_type;
mod trait_block;
mod types;

pub(crate) use impl_block::ImplBlock;
pub(crate) use multi_type::MultiType;
pub(crate) use trait_block::TraitBlock;
pub(crate) use types::Type;

#[proc_macro]
pub fn wrap(input: TokenStream) -> TokenStream {
    let multi_type = parse_macro_input!(input as MultiType);

    let e_num = multi_type.generate_enum();
    let from = multi_type.generate_from();
    let try_into = multi_type.generate_try_from();

    let expanded = quote! {
        #e_num
        #(#from)*
        #(#try_into)*
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
