use super::ident_from_path;
use super::path_from_type;
use crate::{ImplBlock, TraitBlock, Type, syn_to_ident};
use proc_macro2::TokenStream;
use quote::quote;
use std::collections::btree_map::{BTreeMap, Entry};
use syn::FnArg;
use syn::TypeArray;
use syn::TypePath;
use syn::TypeReference;
use syn::TypeSlice;
use syn::TypeTuple;
use syn::punctuated::Punctuated;
use syn::{
    Attribute, Generics, Ident, Token, Visibility,
    parse::{Parse, ParseStream},
    spanned::Spanned,
};

#[derive(Debug, Clone)]
pub(crate) struct MultiType {
    pub(crate) attrs: Vec<Attribute>,
    pub(crate) visibility: Visibility,
    pub(crate) ident: Ident,
    pub(crate) generics: Generics,
    pub(crate) variants: BTreeMap<String, Type>,
    pub(crate) impl_blocks: Vec<ImplBlock>,
    pub(crate) trait_blocks: Vec<TraitBlock>,
}

impl Parse for MultiType {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let content;
        let mut wrapper = Self {
            attrs: input.call(Attribute::parse_outer)?,
            visibility: input.parse::<Visibility>()?,
            ident: input.parse::<Ident>()?,
            generics: input.parse::<Generics>()?,
            variants: BTreeMap::new(),
            impl_blocks: Vec::new(),
            trait_blocks: Vec::new(),
        };

        let _brace_token = syn::braced!(content in input);
        while !content.is_empty() {
            let attrs = content.call(Attribute::parse_outer)?;
            let ty = content.parse::<syn::Type>()?;
            let _ = content.parse::<Token![,]>();
            match &ty {
                syn::Type::Path(TypePath { path, .. }) => {
                    wrapper.add_variant(ident_from_path(path, ""), ty, attrs)?;
                }
                syn::Type::Reference(TypeReference { elem, .. }) => {
                    if let Some(path) = path_from_type(elem) {
                        wrapper.add_variant(ident_from_path(path, "Ref"), ty, attrs)?;
                    } else {
                        return Err(syn::Error::new(ty.span(), "This type can't be used"));
                    }
                }
                syn::Type::Array(TypeArray { elem, len, .. }) => {
                    let ext = format!("Array{}", syn_to_ident(len));
                    if let Some(path) = path_from_type(elem) {
                        wrapper.add_variant(ident_from_path(path, &ext), ty, attrs)?;
                    } else {
                        return Err(syn::Error::new(ty.span(), "This type can't be used"));
                    }
                }
                syn::Type::Slice(TypeSlice { elem, .. }) => {
                    if let Some(path) = path_from_type(elem) {
                        wrapper.add_variant(ident_from_path(path, "Slice"), ty, attrs)?;
                    } else {
                        return Err(syn::Error::new(ty.span(), "This type can't be used"));
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
                        wrapper.add_variant(i, ty, attrs)?;
                    } else {
                        return Err(syn::Error::new(ty.span(), "This type can't be used"));
                    }
                }
                _ => {
                    return Err(syn::Error::new(ty.span(), "This type can't be used"));
                }
            }
        }
        loop {
            if input.peek(Token![impl]) {
                wrapper.impl_blocks.push(input.parse::<ImplBlock>()?);
            } else if input.peek(Token![trait]) {
                wrapper.trait_blocks.push(input.parse::<TraitBlock>()?);
            } else if !input.is_empty() {
                return Err(syn::Error::new(
                    input.lookahead1().error().span(),
                    "only 'impl' and 'trait' itemd are supported",
                ));
            } else {
                break;
            }
        }
        // println!(
        //     "---------------------------------------------\n{wrapper:#?}\n---------------------------------------------"
        // );
        Ok(wrapper)
    }
}

impl MultiType {
    pub(crate) fn add_variant(
        &mut self,
        ident: Ident,
        ty: syn::Type,
        attrs_in: Vec<Attribute>,
    ) -> syn::Result<()> {
        if let Entry::Vacant(entry) = self.variants.entry(ident.to_string()) {
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

            entry.insert(Type {
                ident,
                ty,
                attrs,
                into,
            });
            Ok(())
        } else {
            Err(syn::Error::new(
                ty.span(),
                "Enum variant could not be generated, variant exists",
            ))
        }
    }

    pub(crate) fn enum_variants(&self) -> Vec<TokenStream> {
        self.variants.values().map(Type::enum_variant).collect()
    }

    pub(crate) fn generate_enum(&self) -> TokenStream {
        let variants = self.enum_variants();
        let attrs = &self.attrs;
        let visibility = &self.visibility;
        let ident = &self.ident;
        let generics = &self.generics;

        quote! {
            #(#attrs)*
            #visibility enum #ident #generics {
                #(#variants,)*
            }
        }
    }

    pub(crate) fn generate_from(&self) -> Vec<TokenStream> {
        let wrapper = &self.ident;
        let lt = &self.generics;
        self.variants
            .values()
            .map(|v| {
                let ident = &v.ident;
                let ty = &v.ty;
                quote! {
                    impl #lt ::core::convert::From<#ty> for #wrapper #lt {
                        fn from(value: #ty) -> Self {
                             #wrapper :: #ident(value)
                         }
                    }
                }
            })
            .collect()
    }

    pub(crate) fn generate_try_from(&self) -> Vec<TokenStream> {
        let wrapper = &self.ident;
        let lt = &self.generics;
        self.variants
            .values()
            .map(|outer| {
                let ty = &outer.ty;
                let branches: Vec<TokenStream> = self
                    .variants
                    .values()
                    .map(|inner| inner.try_from_arm(outer, wrapper))
                    .collect();
                quote! {
                    impl #lt ::core::convert::TryFrom<#wrapper #lt> for #ty {
                        type Error = &'static str;
                        fn try_from(other: #wrapper #lt) -> ::core::result::Result< Self, Self::Error >
                        {
                            match other {
                                #(#branches)*
                            }
                        }
                    }
                }
            })
            .collect()
    }

    pub(crate) fn generate_impl_blocks(&self) -> Vec<TokenStream> {
        let wrapper = &self.ident;
        let lt = &self.generics;
        self.impl_blocks
            .iter()
            .map(|block| {
                let items = &block.items;
                let fns = block
                    .functions
                    .iter()
                    .map(|f| {
                        if let Some(FnArg::Receiver(_)) = f.sig.inputs.first() {
                            let arms = self
                                .variants
                                .values()
                                .map(|v| v.fn_call(&self.ident, &f.sig.ident, &f.sig.inputs));
                            let attrs = &f.attrs;
                            let vis = &f.vis;
                            let signature = &f.sig;
                            quote! {
                                #(#attrs)*
                                #vis #signature {
                                    match self {
                                        #(#arms)*
                                    }
                                }
                            }
                        } else {
                            quote! {}
                        }
                    })
                    .collect::<Vec<_>>();

                quote! {
                    impl #lt #wrapper #lt {
                         #(#items)*
                         #(#fns)*
                    }
                }
            })
            .collect()
    }
}
