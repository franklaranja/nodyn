use proc_macro2::TokenStream;
use quote::{ToTokens, quote};
use syn::{
    Attribute, FnArg, GenericArgument, Ident, Path, PathArguments, Token, Type, TypeArray,
    TypePath, TypeReference, TypeTuple, parenthesized,
    parse::Parse,
    punctuated::Punctuated,
    spanned::Spanned,
    token::{Comma, Paren},
};

/// Represents a variant in an enum with its attributes, type, and conversion types.
///
/// A `Variant` encapsulates the metadata for a single enum variant, including its identifier,
/// the type it holds, any attributes, and types it can be converted into (via `#[into]`).
#[derive(Debug, Clone)]
pub(crate) struct Variant {
    /// Attributes applied to the variant (excluding `#[into]`).
    pub(crate) attrs: Vec<Attribute>,
    /// Types this variant's type can be converted into (via `#[into]`).
    pub(crate) into: Vec<Type>,
    /// The identifier of the variant (e.g., `String` for type `String`).
    pub(crate) ident: Ident,
    /// The type held by the variant (e.g., `String`, `i32`).
    pub(crate) ty: Type,
}

impl Variant {
    /// Generates the `TokenStream` for the enum variant definition.
    pub(crate) fn enum_variant_tokens(&self) -> TokenStream {
        let attrs = &self.attrs;
        let ty = &self.ty;
        let ident = &self.ident;
        quote! {
            #(#attrs)*
            #ident(#ty)
        }
    }

    /// Generates a match arm for `TryFrom` conversion between variants.
    ///
    /// If the variants are the same or the source type can be converted to the target type
    /// (via `into`), it generates a successful conversion arm. Otherwise, it generates an error.
    pub(crate) fn try_from_arm_tokens(&self, other: &Self, wrapper: &Ident) -> TokenStream {
        let ident = &self.ident;
        if self.ident == other.ident {
            quote! { #wrapper::#ident(value) => Ok(value), }
        } else if self.into.contains(&other.ty) {
            quote! { #wrapper::#ident(value) => Ok(value.into()),}
        } else {
            let message = format!(
                "No conversion from '{}' to {}",
                self.type_to_string(),
                other.type_to_string()
            );
            quote! { #wrapper::#ident(_) => Err(#message), }
        }
    }

    /// Generates a match arm for calling a function on the variant's value.
    #[allow(clippy::match_wildcard_for_single_variants)]
    pub(crate) fn fn_call_arm_tokens(
        &self,
        wrapper: &Ident,
        function: &Ident,
        inputs: &Punctuated<FnArg, Comma>,
    ) -> TokenStream {
        let ident = &self.ident;
        let args = inputs
            .iter()
            .filter_map(|arg| match arg {
                FnArg::Typed(typed) => Some(&typed.pat),
                _ => None,
            })
            .collect::<Punctuated<_, Comma>>();

        quote! { #wrapper::#ident(value) => value.#function(#args), }
    }

    /// Generates a match arm for retrieving the variant's type as a string.
    pub(crate) fn type_as_str_arm_tokens(&self, wrapper: &Ident) -> TokenStream {
        let type_string = self.type_to_string();
        let ident = &self.ident;
        quote! {
            #wrapper::#ident(_) => #type_string,
        }
    }

    /// Generates a match arm for checking if the variant matches a specific type.
    ///
    /// Returns `true` if the variant's type matches the provided type, otherwise an empty arm.
    pub(crate) fn is_type_arm_tokens(&self, wrapper: &Ident, ty: &Type) -> TokenStream {
        let ident = &self.ident;
        if &self.ty == ty {
            quote! { #wrapper::#ident(_) => true, }
        } else {
            quote! {}
        }
    }

    /// Generates a match arm for converting the variant to a specific type.
    ///
    /// Returns `Some(value)` if the variant's type matches or can be converted to the target type.
    pub(crate) fn as_type_arm_tokens(&self, wrapper: &Ident, ty: &Type) -> TokenStream {
        let ident = &self.ident;
        if &self.ty == ty {
            quote! { #wrapper::#ident(value) => Some(value), }
        } else if self.into.contains(ty) {
            quote! { #wrapper::#ident(value) => Some(value.into()),}
        } else {
            quote! {}
        }
    }

    /// Generates a match arm for borrowing the variant as a reference to a specific type.
    ///
    /// Returns `Some(value)` if the variant's type matches the target type.
    pub(crate) fn as_ref_arm_tokens(&self, wrapper: &Ident, ty: &Type) -> TokenStream {
        let ident = &self.ident;
        if &self.ty == ty {
            quote! { #wrapper::#ident(value) => Some(value), }
        } else {
            quote! {}
        }
    }

    /// Generates a match arm for mutably borrowing the variant as a specific type.
    ///
    /// Returns `Some(value)` if the variant's type matches the target type.
    pub(crate) fn as_mut_arm_tokens(&self, wrapper: &Ident, ty: &Type) -> TokenStream {
        let ident = &self.ident;
        if &self.ty == ty {
            quote! { #wrapper::#ident(value) => Some(value), }
        } else {
            quote! {}
        }
    }

    /// Generates methods for accessing and iterating over variants in a `Vec`.
    ///
    /// Generates methods:
    /// - `first_varient`,
    /// - `first_variant_mut`,
    /// - `last_variant`,
    /// - `last_variant_mut`,
    /// - `iter_variant`
    /// - `iter_variant_mut`
    /// - `enumerate_variant`
    /// - `enumerste_variant_mut`
    /// - `count_variant`
    /// - `all_variant`
    /// - `any_variant`
    ///
    // TODO: - From<Vec<Variant>> &Vec &[T] &mut  (Box/Array?)
    //       - append_type
    //       - extend_from_slice
    //       - AsRef<[T]> & AsMut Vecs to
    //       - Extend T, &T
    //       - FromIterator<T>
    //       - max & min (Ord)
    //
    #[allow(clippy::too_many_lines)]
    pub(crate) fn vec_methods_tokens(&self, enum_ident: &Ident, vec_field: &Ident) -> TokenStream {
        let ident = &self.ident;
        let ty = &self.ty;
        let snake = self.ident_to_snake();
        let type_name = self.type_to_string();

        let fn_first = Ident::new(&format!("first_{snake}"), ty.span());
        let fn_first_doc = format!("Returns the first `{ident}` as `Option<&{type_name}>`.");

        let fn_first_mut = Ident::new(&format!("first_{snake}_mut"), ty.span());
        let fn_first_mut_doc =
            format!("Returns the first `{ident}` as `Option<&mut {type_name}>`.");

        let fn_last = Ident::new(&format!("last_{snake}"), ty.span());
        let fn_last_doc = format!("Returns the last `{ident}` as `Option<&{type_name}>`.");

        let fn_last_mut = Ident::new(&format!("last_{snake}_mut"), ty.span());
        let fn_last_mut_doc = format!("Returns the last `{ident}` as `Option<&mut {type_name}>`.");

        let fn_iter = Ident::new(&format!("iter_{snake}"), ty.span());
        let fn_iter_doc = format!("Returns an iterator over `{ident}` as `&{type_name}`.");

        let fn_iter_mut = Ident::new(&format!("iter_{snake}_mut"), ty.span());
        let fn_iter_mut_doc =
            format!("Returns a mutable iterator over `{ident}` as `&mut {type_name}`.");

        let fn_enumerate = Ident::new(&format!("enumerate_{snake}"), ty.span());
        let fn_enumerate_doc =
            format!("Returns an iterator over `{ident}` as (index, `&{type_name}`).");

        let fn_enumerate_mut = Ident::new(&format!("enumerate_{snake}_mut"), ty.span());
        let fn_enumerate_mut_doc =
            format!("Returns a mutable iterator over `{ident}` as (index, `&mut {type_name}`).");

        let fn_count = Ident::new(&format!("count_{snake}"), ty.span());
        let fn_count_doc = format!("Counts the number of `{ident}` variants in `{enum_ident}`.");

        let fn_all = Ident::new(&format!("all_{snake}"), ty.span());
        let fn_all_doc =
            format!("Returns true if all variants are `{ident}` variants in `{enum_ident}`.");

        let fn_any = Ident::new(&format!("any_{snake}"), ty.span());
        let fn_any_doc = format!("Returns true there is a `{ident}` variants in `{enum_ident}`.");

        quote! {
            #[doc = #fn_first_doc]
            pub fn #fn_first(&self) -> ::core::option::Option<&#ty> {
                self.#vec_field.iter().find_map(|item| {
                    if let #enum_ident::#ident(value) = item {
                        Some(value)
                    } else {
                        None
                    }
                })
            }

            #[doc = #fn_first_mut_doc]
            pub fn #fn_first_mut(&mut self) -> ::core::option::Option<&mut #ty> {
                self.#vec_field.iter_mut().find_map(|item| {
                    if let #enum_ident::#ident(value) = item {
                        Some(value)
                    } else {
                        None
                    }
                })
            }

            #[doc = #fn_last_doc]
            pub fn #fn_last(&self) -> ::core::option::Option<&#ty> {
                self.#vec_field.iter().rev().find_map(|item| {
                    if let #enum_ident::#ident(value) = item {
                        Some(value)
                    } else {
                        None
                    }
                })
            }

            #[doc = #fn_last_mut_doc]
            pub fn #fn_last_mut(&mut self) -> ::core::option::Option<&mut #ty> {
                self.#vec_field.iter_mut().rev().find_map(|item| {
                    if let #enum_ident::#ident(value) = item {
                        Some(value)
                    } else {
                        None
                    }
                })
            }

            #[doc = #fn_iter_doc]
            pub fn #fn_iter(&self) -> impl ::core::iter::Iterator<Item = &#ty> {
                self.#vec_field.iter().filter_map(|item| {
                    if let #enum_ident::#ident(value) = item {
                        Some(value)
                    } else {
                        None
                    }
                })
            }

            #[doc = #fn_iter_mut_doc]
            pub fn #fn_iter_mut(&mut self) -> impl ::core::iter::Iterator<Item = &mut #ty> {
                self.#vec_field.iter_mut().filter_map(|item| {
                    if let #enum_ident::#ident(value) = item {
                        Some(value)
                    } else {
                        None
                    }
                })
            }

            #[doc = #fn_enumerate_doc]
            pub fn #fn_enumerate(&self) -> impl ::core::iter::Iterator<Item = (usize, &#ty)> {
                self.#vec_field.iter().enumerate().filter_map(|(i, item)| {
                    if let #enum_ident::#ident(value) = item {
                        Some((i, value))
                    } else {
                        None
                    }
                })
            }

            #[doc = #fn_enumerate_mut_doc]
            pub fn #fn_enumerate_mut(&mut self) -> impl ::core::iter::Iterator<Item = (usize, &mut #ty)> {
                self.#vec_field.iter_mut().enumerate().filter_map(|(i, item)| {
                    if let #enum_ident::#ident(value) = item {
                        Some((i, value))
                    } else {
                        None
                    }
                })
            }

            #[doc = #fn_count_doc]
            pub fn #fn_count(&self) -> usize {
                self.#vec_field.iter().filter(|item| matches!(item, #enum_ident::#ident(_))).count()
            }

            #[doc = #fn_all_doc]
            pub fn #fn_all(&self) -> bool {
                self.#vec_field.iter().all(|item| ::std::matches!(item, #enum_ident::#ident(_)))
            }

            #[doc = #fn_any_doc]
            pub fn #fn_any(&self) -> bool {
                self.#vec_field.iter().any(|item| ::std::matches!(item, #enum_ident::#ident(_)))
            }

        }
    }

    /// Converts the variant's type to a string representation.
    ///
    /// Cleans up the token stream to remove unnecessary spaces and format references.
    pub(crate) fn type_to_string(&self) -> String {
        self.ty
            .clone()
            .into_token_stream()
            .to_string()
            .replace("& ", "&")
            .replace("& '", "&'")
            .replace(" < ", "<")
            .replace(" > ", ">")
            .replace(" >", ">")
    }

    /// Converts the variant's identifier to `snake_case`.
    pub(crate) fn ident_to_snake(&self) -> String {
        camel_to_snake(&self.ident.to_string())
    }
}

impl Parse for Variant {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let attrs = input.call(Attribute::parse_outer)?;
        let ty = input.parse::<Type>()?;
        let (ident, ty) = if input.peek(Paren) {
            let content;
            parenthesized!(content in input);
            (ident_from_type(&ty)?, content.parse::<Type>()?)
        } else {
            (ident_from_type(&ty)?, ty)
        };

        let (into, other_attrs): (Vec<_>, Vec<_>) = attrs
            .into_iter()
            .partition(|attr| attr.path().is_ident("into"));
        let into_types = into
            .into_iter()
            .flat_map(|attr| {
                attr.parse_args_with(Punctuated::<Type, Token![,]>::parse_terminated)
                    .map(|p| p.into_iter().collect::<Vec<_>>())
                    .unwrap_or_default()
            })
            .collect::<Vec<_>>();

        Ok(Self {
            attrs: other_attrs,
            into: into_types,
            ident,
            ty,
        })
    }
}

pub(crate) fn camel_to_snake(camel: &str) -> String {
    let mut snake = String::new();
    let mut first = true;
    for c in camel.chars() {
        if c.is_uppercase() {
            if !first {
                snake.push('_');
            }
            snake.push_str(&c.to_lowercase().to_string());
        } else {
            snake.push(c);
        }
        first = false;
    }
    snake
}

/// Generates a `CamelCase` identifier from a `syn::Path` with an optional extension.
///
/// # Arguments
///
/// - `path`: The `syn::Path` to convert (e.g., `std::string::String`).
/// - `extension`: A string to append to the generated identifier (e.g., `Ref` for references).
///
/// # Returns
///
/// A `syn::Ident` representing the CamelCase name (e.g., `String` for `std::string::String`).
fn camel_case_ident(path: &Path, extension: &str) -> Ident {
    let idents = path
        .segments
        .iter()
        .map(|segment| {
            let ident = segment.ident.to_string();
            let extra_idents = match &segment.arguments {
                PathArguments::AngleBracketed(args) => args
                    .args
                    .iter()
                    .filter_map(|arg| match arg {
                        GenericArgument::Type(ty) => {
                            ident_from_type(ty).ok().map(|i| i.to_string())
                        }
                        GenericArgument::AssocType(assoc) => {
                            ident_from_type(&assoc.ty).ok().map(|i| i.to_string())
                        }
                        _ => None,
                    })
                    .collect::<String>(),
                _ => String::new(),
            };
            let mut chars = ident.chars();
            chars
                .next()
                .map(|first| {
                    format!(
                        "{}{}{extra_idents}{extension}",
                        first.to_uppercase(),
                        chars.as_str()
                    )
                })
                .unwrap_or_default()
        })
        .collect::<Vec<_>>();

    Ident::new(&idents.join(""), path.span())
}

/// Converts tokens to a `CamelCase` string representation.
///
/// # Arguments
///
/// - `tokens`: Something that implements `ToTokens`.
///
/// # Returns
///
/// A `String` in CamelCase (e.g., `I32` for `i32`).
fn camel_case_tokens<T: ToTokens>(tokens: T) -> String {
    tokens
        .to_token_stream()
        .to_string()
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
                .unwrap_or_default()
        })
        .collect::<String>()
}

/// Extracts a `syn::Path` from a `syn::Type`, if applicable.
///
/// # Arguments
///
/// - `ty`: The type to analyze (e.g., `syn::Type::Path`, `syn::Type::Reference`).
///
/// # Returns
///
/// An `Option<&syn::Path>` containing the path if the type is a path, reference, array, or slice.
fn extract_path(ty: &Type) -> Option<&syn::Path> {
    match ty {
        Type::Path(TypePath { path, .. }) => Some(path),
        Type::Reference(TypeReference { elem, .. }) | Type::Array(TypeArray { elem, .. }) => {
            extract_path(elem)
        }
        _ => None,
    }
}

/// Generates an `Ident` from a `Type`, used for variant naming.
///
/// # Arguments
///
/// - `ty`: The type to derive an identifier from.
///
/// # Returns
///
/// A `syn::Result<Ident>` containing the generated identifier or an error if the type is unsupported.
fn ident_from_type(ty: &Type) -> syn::Result<Ident> {
    match ty {
        Type::Path(TypePath { path, .. }) => Ok(camel_case_ident(path, "")),
        Type::Reference(TypeReference { elem, .. }) => extract_path(elem)
            .map(|path| camel_case_ident(path, "Ref"))
            .ok_or_else(|| syn::Error::new(ty.span(), "Unsupported reference type")),
        Type::Array(TypeArray { elem, len, .. }) => {
            let ext = format!("Array{}", camel_case_tokens(len));
            extract_path(elem)
                .map(|path| camel_case_ident(path, &ext))
                .ok_or_else(|| syn::Error::new(ty.span(), "Unsupported array type"))
        }
        Type::Tuple(TypeTuple { elems, .. }) => {
            let ident = elems
                .iter()
                .map(|t| extract_path(t).map(|p| camel_case_ident(p, "").to_string()))
                .collect::<Option<Vec<_>>>()
                .map(|mut names| {
                    names.push("Tuple".to_string());
                    Ident::new(&names.join(""), elems.span())
                });
            ident.ok_or_else(|| syn::Error::new(ty.span(), "Unsupported tuple type"))
        }
        _ => Err(syn::Error::new(
            ty.span(),
            "Unsupported type for variant identifier",
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use syn::parse_str;

    #[test]
    fn test_camel_to_snake() {
        assert_eq!(camel_to_snake("MyVariant"), "my_variant");
        assert_eq!(camel_to_snake("HTTPResponse"), "h_t_t_p_response");
        assert_eq!(camel_to_snake("lowercase"), "lowercase");
    }

    #[test]
    fn test_variant_parsing() {
        let input = parse_str::<Variant>("#[into(i32, f32)] String").unwrap();
        assert_eq!(input.ident.to_string(), "String");
        assert_eq!(input.into.len(), 2);
        assert_eq!(input.type_to_string(), "String");

        let input = parse_str::<Variant>("#[into(i32)] i32 (u32)").unwrap();
        assert_eq!(input.ident.to_string(), "I32");
        assert_eq!(input.type_to_string(), "u32");
        assert_eq!(input.into.len(), 1);
    }

    #[test]
    fn test_ident_from_type() {
        let ty: Type = parse_str("std::string::String").unwrap();
        let ident = ident_from_type(&ty).unwrap();
        assert_eq!(ident.to_string(), "StdStringString");

        let ty: Type = parse_str("&str").unwrap();
        let ident = ident_from_type(&ty).unwrap();
        assert_eq!(ident.to_string(), "StrRef");

        let ty: Type = parse_str("[i32; 4]").unwrap();
        let ident = ident_from_type(&ty).unwrap();
        assert_eq!(ident.to_string(), "I32Array4");

        let ty: Type = parse_str("(i32, String)").unwrap();
        let ident = ident_from_type(&ty).unwrap();
        assert_eq!(ident.to_string(), "I32StringTuple");
    }

    #[test]
    fn test_type_to_string() {
        let variant = Variant {
            attrs: vec![],
            into: vec![],
            ident: Ident::new("Test", proc_macro2::Span::call_site()),
            ty: parse_str::<Type>("&str").unwrap(),
        };
        assert_eq!(variant.type_to_string(), "&str");

        let variant = Variant {
            attrs: vec![],
            into: vec![],
            ident: Ident::new("Test", proc_macro2::Span::call_site()),
            ty: parse_str::<Type>("Vec<i32>").unwrap(),
        };
        assert_eq!(variant.type_to_string(), "Vec<i32>");
    }
}
