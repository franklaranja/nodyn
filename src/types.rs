use proc_macro2::TokenStream;
use quote::{ToTokens, quote};
use syn::{Attribute, FnArg, Ident, ReturnType, punctuated::Punctuated, token::Comma};

#[derive(Debug, Clone)]
pub(crate) struct Type {
    pub(crate) ident: Ident,
    pub(crate) ty: syn::Type,
    pub(crate) attrs: Vec<Attribute>,
    pub(crate) into: Vec<syn::Type>,
}

impl Type {
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
