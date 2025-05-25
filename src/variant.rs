use proc_macro2::TokenStream;
use quote::{quote, ToTokens};
use syn::{
    parenthesized, parse::Parse, punctuated::Punctuated, spanned::Spanned, token::Comma, Attribute,
    FnArg, Ident, Token, TypeArray, TypePath, TypeReference, TypeSlice, TypeTuple,
};

use crate::{ident_from_path, path_from_type, syn_to_ident};

#[derive(Debug, Clone)]
pub(crate) struct Variant {
    pub(crate) attrs: Vec<Attribute>,
    pub(crate) into: Vec<syn::Type>,
    pub(crate) ident: Ident,
    pub(crate) ty: syn::Type,
}

impl Variant {
    pub(crate) fn enum_variant(&self) -> TokenStream {
        // let variants = content.parse_terminated::<Punctuated<Variant, Token![,]>>()?;
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
                self.ty.to_token_stream(),
                other.ty.to_token_stream()
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
}

impl Parse for Variant {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let attrs_in = input.call(Attribute::parse_outer)?;
        let ty = input.parse::<syn::Type>()?;
        let (ident, ty) = if input.peek(syn::token::Paren) {
            let content;
            let _paren_token = parenthesized!(content in input);
            (ident_from_type(&ty)?, content.parse::<syn::Type>()?)
        } else {
            (ident_from_type(&ty)?, ty)
        };
        let mut into = Vec::new();
        let mut attrs = Vec::new();
        for a in attrs_in {
            if a.path().is_ident("into") {
                into = a
                    .parse_args_with(Punctuated::<syn::Type, Token![,]>::parse_terminated)?
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

fn ident_from_type(ty: &syn::Type) -> syn::Result<Ident> {
    match &ty {
        syn::Type::Path(TypePath { path, .. }) => Ok(ident_from_path(path, "")),
        syn::Type::Reference(TypeReference { elem, .. }) => {
            if let Some(path) = path_from_type(elem) {
                Ok(ident_from_path(path, "Ref"))
            } else {
                no_ident_err(ty)
            }
        }
        syn::Type::Array(TypeArray { elem, len, .. }) => {
            let ext = format!("Array{}", syn_to_ident(len));
            if let Some(path) = path_from_type(elem) {
                Ok(ident_from_path(path, &ext))
            } else {
                no_ident_err(ty)
            }
        }
        syn::Type::Slice(TypeSlice { elem, .. }) => {
            if let Some(path) = path_from_type(elem) {
                Ok(ident_from_path(path, "Slice"))
            } else {
                no_ident_err(ty)
            }
        }
        syn::Type::Tuple(TypeTuple { elems, .. }) => {
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

fn no_ident_err(ty: &syn::Type) -> syn::Result<Ident> {
    Err(syn::Error::new(
        ty.span(),
        "This type can't be used, try defining the variant name",
    ))
}
