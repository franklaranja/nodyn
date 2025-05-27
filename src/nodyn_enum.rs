use crate::Variant;
use crate::{ImplBlock, TraitBlock};
use proc_macro2::TokenStream;
use quote::quote;
use std::collections::btree_map::{BTreeMap, Entry};
use syn::punctuated::Punctuated;
use syn::FnArg;
use syn::{
    parse::{Parse, ParseStream},
    spanned::Spanned,
    Attribute, Generics, Ident, Token, Visibility,
};

#[derive(Debug, Clone)]
pub(crate) struct NodynEnum {
    pub(crate) attrs: Vec<Attribute>,
    pub(crate) visibility: Visibility,
    pub(crate) _keyword: syn::token::Enum,
    pub(crate) ident: Ident,
    pub(crate) generics: Generics,
    pub(crate) variants: BTreeMap<String, Variant>,
    pub(crate) impl_blocks: Vec<ImplBlock>,
    pub(crate) trait_blocks: Vec<TraitBlock>,
}

impl Parse for NodynEnum {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let mut wrapper = Self {
            attrs: input.call(Attribute::parse_outer)?,
            visibility: input.parse::<Visibility>()?,
            _keyword: input.parse::<syn::token::Enum>()?,
            ident: input.parse::<Ident>()?,
            generics: input.parse::<Generics>()?,
            variants: BTreeMap::new(),
            impl_blocks: Vec::new(),
            trait_blocks: Vec::new(),
        };

        let content;
        let _brace_token = syn::braced!(content in input);
        let variants = Punctuated::<Variant, Token![,]>::parse_terminated(&content)?;
        for v in variants {
            wrapper.add_variant(v)?;
        }
        loop {
            if input.peek(Token![impl]) {
                let _keyword = input.parse::<syn::token::Impl>()?;
                // impl of a trait if followed by an identi
                if input.peek(Ident) {
                    wrapper.trait_blocks.push(input.parse::<TraitBlock>()?);
                } else {
                    wrapper.impl_blocks.push(input.parse::<ImplBlock>()?);
                }
            } else if !input.is_empty() {
                return Err(syn::Error::new(
                    input.lookahead1().error().span(),
                    "only 'impl' items are supported",
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

impl NodynEnum {
    pub(crate) fn add_variant(&mut self, variant: Variant) -> syn::Result<()> {
        if let Entry::Vacant(entry) = self.variants.entry(variant.ident.to_string()) {
            entry.insert(variant);
            Ok(())
        } else {
            Err(syn::Error::new(
                variant.ty.span(),
                "Enum variant could not be generated, variant exists",
            ))
        }
    }

    pub(crate) fn enum_variants(&self) -> Vec<TokenStream> {
        self.variants.values().map(Variant::enum_variant).collect()
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

    pub(crate) fn generate_trait_blocks(&self) -> Vec<TokenStream> {
        let wrapper = &self.ident;
        let lt = &self.generics;
        self.trait_blocks
            .iter()
            .map(|b| {
                let trait_name = &b.ident;
                let trait_gen = &b.generics;
                let items = &b.block.items;
                let fns = b.block.expand_methods(self);
                quote! {
                    impl #lt #trait_name #trait_gen for #wrapper #lt {
                         #(#items)*
                         #(#fns)*
                    }
                }
            })
            .collect()
    }
}
