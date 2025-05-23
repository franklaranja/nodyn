use super::ident_from_path;
use super::path_from_type;
use crate::{Function, Trait, Type, syn_to_ident};
use quote::quote;
use std::collections::btree_map::{BTreeMap, Entry};
use syn::{parse::Parse, spanned::Spanned};

#[derive(Debug, Clone)]
pub(crate) struct Generator {
    pub(crate) attrs: Vec<syn::Attribute>,
    pub(crate) visibility: syn::Visibility,
    pub(crate) ident: syn::Ident,
    pub(crate) generics: syn::Generics,
    pub(crate) variants: BTreeMap<String, Type>,
    pub(crate) impls: Vec<Function>,
    pub(crate) traits: Vec<Trait>,
}

impl Parse for Generator {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let content;
        let mut wrapper = Self {
            attrs: input.call(syn::Attribute::parse_outer)?,
            visibility: input.parse::<syn::Visibility>()?,
            ident: input.parse::<syn::Ident>()?,
            generics: input.parse::<syn::Generics>()?,
            variants: BTreeMap::new(),
            impls: Vec::new(),
            traits: Vec::new(),
        };

        let _brace_token = syn::braced!(content in input);
        while !content.is_empty() {
            let attrs = content.call(syn::Attribute::parse_outer)?;
            let ty = content.parse::<syn::Type>()?;
            let _ = content.parse::<syn::Token![,]>();
            match &ty {
                syn::Type::Path(syn::TypePath { path, .. }) => {
                    wrapper.add_variant(ident_from_path(path, ""), ty, attrs)?
                }
                syn::Type::Reference(syn::TypeReference { elem, .. }) => {
                    if let Some(path) = path_from_type(elem) {
                        wrapper.add_variant(ident_from_path(path, "Ref"), ty, attrs)?
                    } else {
                        return Err(syn::Error::new(ty.span(), "This type can't be used"));
                    }
                }
                syn::Type::Array(syn::TypeArray { elem, len, .. }) => {
                    let ext = format!("Array{}", syn_to_ident(len));
                    if let Some(path) = path_from_type(elem) {
                        wrapper.add_variant(ident_from_path(path, &ext), ty, attrs)?
                    } else {
                        return Err(syn::Error::new(ty.span(), "This type can't be used"));
                    }
                }
                syn::Type::Slice(syn::TypeSlice { elem, .. }) => {
                    if let Some(path) = path_from_type(elem) {
                        wrapper.add_variant(ident_from_path(path, "Slice"), ty, attrs)?
                    } else {
                        return Err(syn::Error::new(ty.span(), "This type can't be used"));
                    }
                }
                syn::Type::Tuple(syn::TypeTuple { elems, .. }) => {
                    let ident = elems
                        .iter()
                        .map(|t| path_from_type(t).map(|p| ident_from_path(p, "").to_string()))
                        .collect::<Option<Vec<String>>>()
                        .map(|mut v| {
                            v.push("Tuple".to_string());
                            syn::Ident::new(&v.concat(), elems.span())
                        });

                    if let Some(i) = ident {
                        wrapper.add_variant(i, ty, attrs)?
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
            if input.peek(syn::Token![impl]) {
                wrapper.impls.push(input.parse::<Function>()?)
            } else if input.peek(syn::Token![trait]) {
                wrapper.traits.push(input.parse::<Trait>()?)
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

impl Generator {
    pub(crate) fn add_variant(
        &mut self,
        ident: syn::Ident,
        ty: syn::Type,
        attrs_in: Vec<syn::Attribute>,
    ) -> syn::Result<()> {
        if let Entry::Vacant(entry) = self.variants.entry(ident.to_string()) {
            let mut into = Vec::new();
            let mut attrs = Vec::new();
            for a in attrs_in {
                if a.path().is_ident("into") {
                    into = a.parse_args_with(
                        syn::punctuated::Punctuated::<syn::Type, syn::Token![,]>::parse_terminated,
                    )?.iter().cloned().collect();
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

    pub(crate) fn enum_variants(&self) -> Vec<proc_macro2::TokenStream> {
        self.variants.values().map(Type::enum_variant).collect()
    }

    pub(crate) fn generate_enum(&self) -> proc_macro2::TokenStream {
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

    pub(crate) fn generate_from(&self) -> Vec<proc_macro2::TokenStream> {
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

    pub(crate) fn generate_try_from(&self) -> Vec<proc_macro2::TokenStream> {
        let wrapper = &self.ident;
        let lt = &self.generics;
        self.variants
            .values()
            .map(|outer| {
                let ty = &outer.ty;
                let branches: Vec<proc_macro2::TokenStream> = self
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
}
