use proc_macro2::TokenStream;
use quote::{quote, ToTokens};
use syn::{
    parenthesized,
    parse::Parse,
    punctuated::Punctuated,
    spanned::Spanned,
    token::{Comma, Paren},
    Attribute, FnArg, GenericArgument, Ident, PathArguments, Token, Type, TypeArray, TypePath,
    TypeReference, TypeSlice, TypeTuple,
};

#[derive(Debug, Clone)]
pub(crate) struct Variant {
    pub(crate) attrs: Vec<Attribute>,
    pub(crate) into: Vec<Type>,
    pub(crate) ident: Ident,
    pub(crate) ty: Type,
}

impl Variant {
    pub(crate) fn enum_variant(&self) -> TokenStream {
        let attrs = &self.attrs;
        let ty = &self.ty;
        let ident = &self.ident;
        quote! {
            #(#attrs)*
            #ident ( #ty )
        }
    }

    pub(crate) fn try_from_arm(&self, other: &Self, wrapper: &Ident) -> TokenStream {
        let ident = &self.ident;
        if self.ident == other.ident {
            quote! { #wrapper::#ident(value) => Ok(value), }
        } else if self.into.contains(&other.ty) {
            quote! { #wrapper::#ident(value) => Ok(value.into()),}
        } else {
            let message = format!(
                "No conversion from '{}' to {}",
                self.type_as_string(),
                other.type_as_string()
            );
            quote! { #wrapper::#ident(_) => Err(#message), }
        }
    }

    pub(crate) fn fn_call(
        &self,
        wrapper: &Ident,
        function: &Ident,
        inputs: &Punctuated<FnArg, Comma>,
    ) -> TokenStream {
        let ident = &self.ident;
        let args = inputs
            .iter()
            .filter_map(|arg| {
                if let FnArg::Typed(typed) = arg {
                    Some(&typed.pat)
                } else {
                    None
                }
            })
            .collect::<Punctuated<_, Comma>>();
        quote! {
            #wrapper::#ident(value) =>  value.#function( #args ),
        }
    }

    pub(crate) fn type_as_str_arm(&self, wrapper: &Ident) -> TokenStream {
        let type_string = self.type_as_string();
        let ident = &self.ident;
        quote! {
            #wrapper::#ident(_) => #type_string,
        }
    }

    pub(crate) fn is_a_arm(&self, wrapper: &Ident, ty: &Type) -> TokenStream {
        let ident = &self.ident;
        if &self.ty == ty {
            quote! { #wrapper::#ident(_) => true, }
        } else {
            quote! {}
        }
    }

    pub(crate) fn as_arm(&self, wrapper: &Ident, ty: &Type) -> TokenStream {
        let ident = &self.ident;
        if &self.ty == ty {
            quote! { #wrapper::#ident(value) => Some(value), }
        } else if self.into.contains(ty) {
            quote! { #wrapper::#ident(value) => Some(value.into()),}
        } else {
            quote! {}
        }
    }

    pub(crate) fn as_ref_arm(&self, wrapper: &Ident, ty: &Type) -> TokenStream {
        let ident = &self.ident;
        if &self.ty == ty {
            quote! { #wrapper::#ident(value) => Some(value), }
        } else {
            quote! {}
        }
    }

    pub(crate) fn as_mut_arm(&self, wrapper: &Ident, ty: &Type) -> TokenStream {
        let ident = &self.ident;
        if &self.ty == ty {
            quote! { #wrapper::#ident(value) => Some(value), }
        } else {
            quote! {}
        }
    }

    pub(crate) fn type_as_string(&self) -> String {
        self.ty
            .clone()
            .into_token_stream()
            .to_string()
            .replace("& '", "&'")
            .replace(" < ", "<")
            .replace(" > ", ">")
            .replace(" >", ">")
    }

    pub(crate) fn ident_as_snake(&self) -> String {
        camel_to_snake(&self.ident.to_string())
    }
}

impl Parse for Variant {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let attrs_in = input.call(Attribute::parse_outer)?;
        let ty = input.parse::<Type>()?;
        let (ident, ty) = if input.peek(Paren) {
            let content;
            let _paren_token = parenthesized!(content in input);
            (ident_from_type(&ty)?, content.parse::<Type>()?)
        } else {
            (ident_from_type(&ty)?, ty)
        };
        let mut into = Vec::new();
        let mut attrs = Vec::new();
        for a in attrs_in {
            if a.path().is_ident("into") {
                into = a
                    .parse_args_with(Punctuated::<Type, Token![,]>::parse_terminated)?
                    .iter()
                    .cloned()
                    .collect();
            } else {
                attrs.push(a);
            }
        }
        Ok(Self {
            attrs,
            into,
            ident,
            ty,
        })
    }
}

fn ident_from_type(ty: &Type) -> syn::Result<Ident> {
    match &ty {
        Type::Path(TypePath { path, .. }) => Ok(ident_from_path(path, "")),
        Type::Reference(TypeReference { elem, .. }) => {
            if let Some(path) = path_from_type(elem) {
                Ok(ident_from_path(path, "Ref"))
            } else {
                no_ident_err(ty)
            }
        }
        Type::Array(TypeArray { elem, len, .. }) => {
            let ext = format!("Array{}", syn_to_ident(len));
            if let Some(path) = path_from_type(elem) {
                Ok(ident_from_path(path, &ext))
            } else {
                no_ident_err(ty)
            }
        }
        Type::Slice(TypeSlice { elem, .. }) => {
            if let Some(path) = path_from_type(elem) {
                Ok(ident_from_path(path, "Slice"))
            } else {
                no_ident_err(ty)
            }
        }
        Type::Tuple(TypeTuple { elems, .. }) => {
            let ident = elems
                .iter()
                .map(|t| path_from_type(t).map(|p| ident_from_path(p, "").to_string()))
                .collect::<Option<Vec<String>>>()
                .map(|mut v| {
                    v.push("Tuple".to_string());
                    Ident::new(&v.concat(), elems.span())
                });

            if let Some(i) = ident {
                Ok(i)
            } else {
                no_ident_err(ty)
            }
        }
        _ => Err(syn::Error::new(ty.span(), "This type can't be used")),
    }
}

fn no_ident_err(ty: &Type) -> syn::Result<Ident> {
    Err(syn::Error::new(
        ty.span(),
        "This type can't be used, try defining the variant name",
    ))
}

fn camel_to_snake(camel: &str) -> String {
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

fn ident_from_path(p: &syn::Path, extension: &str) -> Ident {
    let idents: Option<Vec<String>> = p
        .segments
        .iter()
        .map(|p| {
            let ident = p.ident.to_string();
            let extra_idents = if let PathArguments::AngleBracketed(args) = &p.arguments {
                let idents = args
                    .args
                    .iter()
                    .filter_map(|a| match a {
                        GenericArgument::Type(t) => Some(ident_from_type(t).map(|i| i.to_string())),
                        GenericArgument::AssocType(t) => {
                            Some(ident_from_type(&t.ty).map(|i| i.to_string()))
                        }
                        _ => None,
                    })
                    .collect::<syn::Result<Vec<_>>>();
                if let Ok(vec) = idents {
                    vec.concat()
                } else {
                    String::new()
                }
            } else {
                String::new()
            };
            let mut chars = ident.chars();
            chars.next().map(|first| {
                format!(
                    "{}{}{extra_idents}{extension}",
                    first.to_uppercase(),
                    chars.as_str()
                )
            })
        })
        .collect();
    idents
        .map(|s| Ident::new(&s.concat(), p.span()))
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

fn path_from_type(ty: &Type) -> Option<&syn::Path> {
    match ty {
        Type::Path(TypePath { path, .. }) => Some(path),
        Type::Reference(TypeReference { elem, .. })
        | Type::Array(TypeArray { elem, .. })
        | Type::Slice(TypeSlice { elem, .. }) => path_from_type(elem),
        _ => None,
    }
}
