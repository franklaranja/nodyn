use crate::Variant;
use crate::{ImplBlock, TraitBlock};
use proc_macro2::TokenStream;
use quote::quote;
use std::collections::btree_map::{BTreeMap, Entry};
use syn::Type;
use syn::{
    parse::{Parse, ParseStream},
    punctuated::Punctuated,
    spanned::Spanned,
    Attribute, FnArg, Generics, Ident, Token, Visibility,
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

    pub(crate) fn generate_type_to_str(&self) -> TokenStream {
        let wrapper = &self.ident;
        let lt = &self.generics;
        let vis = &self.visibility;
        let names = self
            .variants
            .values()
            .map(Variant::type_as_string)
            .collect::<Vec<_>>();
        let count = names.len();
        let arms = self
            .variants
            .values()
            .map(|v| v.type_as_str_arm(&self.ident))
            .collect::<Vec<_>>();
        quote! {
            impl #lt #wrapper #lt {
                /// Returns the number of types (variants)
                #vis const fn count() -> usize { #count }

                /// Returns an array with the types as `&str`s.
                /// If the type is a reference there will be a space
                /// between the & and '.
                #vis const fn types() -> [&'static str;#count] {
                    [ #(#names ,)*]
                }

                /// returns the type as a `&str` of the variant.
                /// If the type is a reference there will be a space
                /// between the & and '.
                #vis const fn  type_name(&self) -> &'static str {
                    match self {
                        #(#arms)*
                    }
                }
            }
        }
    }

    // TODO: skip as_ref for reference types
    // - function as try_as_   _ref/mut
    pub(crate) fn generate_is_as(&self) -> syn::Result<TokenStream> {
        let wrapper = &self.ident;
        let lt = &self.generics;
        let fns = self
            .variants
            .values()
            .map(|v| {
                // let ident = &v.ident;
                let ty = &v.ty;
                let snake = v.ident_as_snake();
                let type_name = v.type_as_string();

                let is_a_arms = self
                    .variants
                    .values()
                    .map(|i| i.is_a_arm(&wrapper, &ty))
                    .collect::<Vec<_>>();
                let is_fn = Ident::new(&format!("is_{snake}"), ty.span());
                let is_a_doc = &format!("Returns true if the variant is a {type_name}>");

                let as_arms = self
                    .variants
                    .values()
                    .map(|i| i.as_arm(&wrapper, &ty))
                    .collect::<Vec<_>>();
                let as_fn = Ident::new(&format!("try_as_{snake}"), ty.span());
                let as_doc = &format!("Returns the variant as an Option<{type_name}>");

                let as_ref_mut = if let &Type::Reference(_) = ty {
                    quote! {}
                } else {
                    let as_ref_arms = self
                        .variants
                        .values()
                        .map(|i| i.as_ref_arm(&wrapper, &ty))
                        .collect::<Vec<_>>();
                    let as_ref_fn = Ident::new(&format!("try_as_{snake}_ref"), ty.span());
                    let as_ref_doc = &format!("Returns the variant as an Option<&{type_name}>");

                    let as_mut_arms = self
                        .variants
                        .values()
                        .map(|i| i.as_mut_arm(&wrapper, &ty))
                        .collect::<Vec<_>>();
                    let as_mut_fn = Ident::new(&format!("try_as_{snake}_mut"), ty.span());
                    let as_mut_doc = &format!("Returns the variant as an Option<&mut {type_name}>");
                    quote! {
                    #[doc = #as_ref_doc]
                    fn #as_ref_fn(&self) -> Option<&#ty> {
                        match self {
                            #(#as_ref_arms)*
                            _ => None,
                        }
                    }
                    #[doc = #as_mut_doc]
                    fn #as_mut_fn(&mut self) -> Option<&mut #ty> {
                        match self {
                            #(#as_mut_arms)*
                            _ => None,
                        }
                    }

                    }
                };

                Ok(quote! {
                    #[doc = #is_a_doc]
                    fn #is_fn(&self) -> bool {
                        match self {
                            #(#is_a_arms)*
                            _ => false,
                        }
                    }
                    #[doc = #as_doc]
                    fn #as_fn(self) -> Option< #ty > {
                        match self {
                            #(#as_arms)*
                            _ => None,
                        }
                    }
                    #as_ref_mut
                })
            })
            .collect::<syn::Result<Vec<_>>>()?;
        Ok(quote! {
            impl #lt #wrapper #lt {
                #(#fns)*
            }
        })
    }
}
