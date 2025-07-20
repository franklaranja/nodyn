use core::option::Option::None;
use proc_macro2::{Ident, Span, TokenStream, TokenTree};
use quote::quote;
use syn::spanned::Spanned;
use syn::{
    parse::{Parse, ParseStream},
    Fields, Generics, ItemStruct, Meta,
};

#[derive(Debug, Clone)]
pub(crate) struct WrapperStruct {
    pub(crate) wrapper: ItemStruct,
    pub(crate) vec_field: Option<Ident>,
}

impl WrapperStruct {
    pub(crate) fn vec_wrapper_struct(
        &self,
        enum_ident: &Ident,
        generics: &Generics,
    ) -> TokenStream {
        let vis = &self.wrapper.vis;
        let ident = &self.wrapper.ident;
        let generic_params = &self
            .wrapper
            .generics
            .params
            .iter()
            .chain(generics.params.iter())
            // .cloned()
            // .map(|g| g.to_token_stream())
            .collect::<Vec<_>>();

        let (where_kw, where_clauses) =
            match (&generics.where_clause, &self.wrapper.generics.where_clause) {
                (Some(w), None) | (None, Some(w)) => (
                    Some("where: "),
                    w.predicates.iter().cloned().collect::<Vec<_>>(),
                ),
                (Some(w1), Some(w2)) => (
                    Some("where:"),
                    w1.predicates
                        .iter()
                        .chain(w2.predicates.iter())
                        .cloned()
                        .collect::<Vec<_>>(),
                ),
                (None, None) => (None, Vec::new()),
            };
        let fields = if let Fields::Named(fields) = &self.wrapper.fields {
            fields.named.iter().collect::<Vec<_>>()
        } else {
            Vec::new()
        };
        let default_field = Ident::new("inner", Span::call_site());
        let inner = self.vec_field.as_ref().unwrap_or(&default_field);
        quote! {
            #vis struct #ident < #(#generic_params ,)* >
            #where_kw #(#where_clauses ,)* {
                #(#fields ,)*
                #inner: std::vec::Vec< #enum_ident #generics >,
            }
        }
    }
}

impl Parse for WrapperStruct {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let mut wrapper = input.parse::<ItemStruct>()?;
        match wrapper.fields {
            Fields::Unit | Fields::Unnamed(_) => {
                return Err(syn::Error::new(
                    wrapper.span(),
                    "only structs with named fields are supported",
                ));
            }
            Fields::Named(_) => {} // ok
        }
        let mut attrs = Vec::new();
        let vec_wrapper = Ident::new("vec_wrapper", Span::call_site());
        let mut vec_field = None;
        for attribute in wrapper.attrs {
            if let Meta::Path(path) = &attribute.meta {
                if path.is_ident(&vec_wrapper) {
                } else {
                    attrs.push(attribute);
                }
            } else if let Meta::List(list) = &attribute.meta {
                if list.path.is_ident(&vec_wrapper) {
                    if let Some(TokenTree::Ident(ident)) = list.tokens.clone().into_iter().next() {
                        vec_field = Some(ident);
                    } else {
                        vec_field = Some(Ident::new("inner_vec", Span::call_site()));
                    }
                } else {
                    attrs.push(attribute);
                }
            } else {
                attrs.push(attribute);
            }
        }
        wrapper.attrs = attrs;

        Ok(Self { wrapper, vec_field })
    }
}
