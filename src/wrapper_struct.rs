use core::option::Option::None;
use proc_macro2::{Ident, Span, TokenStream, TokenTree};
use quote::{quote, ToTokens};
use syn::spanned::Spanned;
use syn::{
    parse::{Parse, ParseStream},
    parse_quote, Fields, Generics, ItemStruct, Meta, Visibility,
};

use crate::NodynEnum;

#[derive(Debug, Clone)]
pub(crate) struct WrapperStruct {
    pub(crate) wrapper: ItemStruct,
    // pub(crate) ident: Ident,
    pub(crate) vec_field: Option<Ident>,
    pub(crate) custom_struct: bool,
}

impl WrapperStruct {
    pub(crate) fn standard_vec_wrapper(
        ident: Ident,
        vis: &Visibility,
        enum_ident: &Ident,
        generics: &Generics,
    ) -> Self {
        let pound = syn::token::Pound::default();
        // let vis = &nodyn.visibility;
        // let generics = &nodyn.generics;
        // let enum_ident = &nodyn.ident;
        let wrapper: ItemStruct = parse_quote! {
            #pound [derive(Debug, Default)]
            #vis struct #ident #generics {
                inner: std::vec::Vec< #enum_ident #generics >,
            }
        };
        Self {
            wrapper,
            // ident,
            vec_field: Some(Ident::new("inner", Span::call_site())),
            custom_struct: false,
        }
    }

    pub(crate) fn vec_wrapper_struct(&self, nodyn: &NodynEnum) -> TokenStream {
        if self.vec_field.is_none() {
            return TokenStream::new();
        }
        if self.custom_struct {
            let enum_ident = &nodyn.ident;
            let generics = nodyn.enum_generic_params();
            let vis = &self.wrapper.vis;
            let ident = &self.wrapper.ident;
            let generic_params = nodyn.generic_params(&self.wrapper.generics);

            let where_clause = nodyn.where_clause(&self.wrapper.generics);
            let fields = if let Fields::Named(fields) = &self.wrapper.fields {
                fields.named.iter().collect::<Vec<_>>()
            } else {
                Vec::new()
            };
            let default_field = Ident::new("inner", Span::call_site());
            let inner = self.vec_field.as_ref().unwrap_or(&default_field);
            quote! {
                #vis struct #ident #generic_params
                #where_clause {
                    #(#fields ,)*
                    #inner: std::vec::Vec< #enum_ident #generics >,
                }
            }
        } else {
            self.wrapper.to_token_stream()
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

        Ok(Self {
            // ident: wrapper.ident.clone(),
            wrapper,
            vec_field,
            custom_struct: true,
        })
    }
}
