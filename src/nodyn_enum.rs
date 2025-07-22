use core::option::Option::None; // for analyzer
use std::collections::HashSet;

use proc_macro2::{Span, TokenStream};
use quote::quote;
use syn::{
    parse::{Parse, ParseStream},
    punctuated::Punctuated,
    spanned::Spanned,
    Attribute, FnArg, GenericParam, Generics, Ident, Meta, Token, Type, Visibility, WherePredicate,
};

use crate::{keyword, Features, ImplBlock, TraitBlock, Variant, WrapperStruct};

mod kw {
    syn::custom_keyword!(from);
    syn::custom_keyword!(str);
}

#[derive(Debug, Clone)]
pub(crate) struct NodynEnum {
    pub(crate) attrs: Vec<Attribute>,
    pub(crate) visibility: Visibility,
    pub(crate) _keyword: syn::token::Enum,
    pub(crate) ident: Ident,
    pub(crate) generics: Generics,
    pub(crate) variants: Vec<Variant>,
    pub(crate) impl_blocks: Vec<ImplBlock>,
    pub(crate) trait_blocks: Vec<TraitBlock>,
    pub(crate) features: Features,
    pub(crate) collection_structs: Vec<WrapperStruct>,
}

impl Parse for NodynEnum {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let mut wrapper = Self {
            attrs: input.call(Attribute::parse_outer)?,
            visibility: input.parse::<Visibility>()?,
            _keyword: input.parse::<syn::token::Enum>()?,
            ident: input.parse::<Ident>()?,
            generics: input.parse::<Generics>()?,
            variants: Vec::new(),
            impl_blocks: Vec::new(),
            trait_blocks: Vec::new(),
            features: Features::default(),
            collection_structs: Vec::new(),
        };

        let mut existing_types = HashSet::new();
        let content;
        let _brace_token = syn::braced!(content in input);
        let variants = Punctuated::<Variant, Token![,]>::parse_terminated(&content)?;
        for variant in variants {
            if existing_types.contains(&variant.ty) {
                return Err(syn::Error::new(
                    variant.ty.span(),
                    "Enum variant could not be generated, variant exists",
                ));
            }
            existing_types.insert(variant.ty.clone());
            wrapper.variants.push(variant);
        }
        let derive_attr = Self::derive_attr(&wrapper.attrs);
        // println!("Starting impls");
        loop {
            if input.peek(Token![impl]) {
                let _keyword = input.parse::<syn::token::Impl>()?;
                if input.peek(keyword::vec) {
                    // println!("Starting impl standard vec");
                    let _kw = input.parse::<keyword::vec>()?;
                    let ident = if input.peek(Ident) {
                        input.parse::<Ident>()?
                    } else {
                        Ident::new(&format!("{}Vec", wrapper.ident), Span::call_site())
                    };
                    wrapper
                        .collection_structs
                        .push(WrapperStruct::standard_vec_wrapper(
                            &ident,
                            &wrapper.visibility,
                            &wrapper.ident,
                            &wrapper.generics,
                            &derive_attr,
                        ));
                    if input.peek(Token![;]) {
                        let _ = input.parse::<syn::token::Semi>()?;
                    }
                } else if input.peek(keyword::From)
                    || input.peek(keyword::TryInto)
                    || input.peek(keyword::is_as)
                    || input.peek(keyword::introspection)
                {
                    wrapper.features.merge(input.parse::<Features>()?);
                } else if input.peek(Ident) {
                    wrapper.trait_blocks.push(input.parse::<TraitBlock>()?);
                } else {
                    wrapper.impl_blocks.push(input.parse::<ImplBlock>()?);
                }
            } else if let Ok(wrapper_struct) = input.parse::<WrapperStruct>() {
                // println!("Starting impl custom vec");
                wrapper.collection_structs.push(wrapper_struct);
            } else if !input.is_empty() {
                return Err(syn::Error::new(
                    input.lookahead1().error().span(),
                    "only 'impl' and struct items are supported",
                ));
            } else {
                break;
            }
        }
        // println!(
        //     "---------------------------------------------\n{wrapper:#?}\n---------------------------------------------"
        // );
        // println!("Parsed ok");
        Ok(wrapper)
    }
}

impl NodynEnum {
    pub(crate) fn derive_attr(attrs: &[Attribute]) -> Vec<Attribute> {
        let mut derive_attrs = Vec::new();
        let ident = Ident::new("derive", Span::call_site());
        for attr in attrs {
            if let Meta::List(list) = &attr.meta {
                if list.path.is_ident(&ident) {
                    derive_attrs.push(attr.clone());
                }
            }
        }
        derive_attrs
    }

    pub(crate) fn generic_params<'a>(&'a self, generics: &'a Generics) -> TokenStream {
        let generics = self
            .generics
            .params
            .iter()
            .chain(generics.params.iter())
            .collect::<Vec<_>>();

        if generics.is_empty() {
            TokenStream::new()
        } else {
            quote! { < #(#generics ,)* > }
        }
    }

    pub(crate) fn generic_params_extra<'a>(
        &'a self,
        generics: &'a Generics,
        extra: &GenericParam,
    ) -> TokenStream {
        let mut generics = self
            .generics
            .params
            .iter()
            .chain(generics.params.iter())
            .collect::<Vec<_>>();
        generics.push(&extra);

        if generics.is_empty() {
            TokenStream::new()
        } else {
            quote! { < #(#generics ,)* > }
        }
    }

    pub(crate) fn enum_generic_params(&self) -> TokenStream {
        let generics = self.generics.params.iter().collect::<Vec<_>>();
        if generics.is_empty() {
            TokenStream::new()
        } else {
            quote! { < #(#generics ,)* > }
        }
    }

    pub(crate) fn enum_generic_params_plus(&self, extra: &GenericParam) -> TokenStream {
        let mut generics = self.generics.params.iter().collect::<Vec<_>>();
        generics.push(&extra);
        if generics.is_empty() {
            TokenStream::new()
        } else {
            quote! { < #(#generics ,)* > }
        }
    }

    pub(crate) fn enum_where_plus(&self, extra: &WherePredicate) -> TokenStream {
        let mut where_clause = if let Some(clause) = &self.generics.where_clause {
            clause.predicates.iter().collect::<Vec<_>>()
        } else {
            Vec::new()
        };
        where_clause.push(&extra);
        if where_clause.is_empty() {
            TokenStream::new()
        } else {
            quote! { where #(#where_clause ,)*  }
        }
    }
    pub(crate) fn where_clause(&self, generics: &Generics) -> TokenStream {
        // (Option<&'static str>, Vec<&'a WherePredicate>) {
        let predicates = match (&self.generics.where_clause, &generics.where_clause) {
            (Some(w), None) | (None, Some(w)) => w.predicates.iter().collect::<Vec<_>>(),
            (Some(w1), Some(w2)) => w1
                .predicates
                .iter()
                .chain(w2.predicates.iter())
                .collect::<Vec<_>>(),

            (None, None) => return TokenStream::new(),
        };
        quote! {where #(#predicates ,)* }
    }

    pub(crate) fn where_clause_extra(
        &self,
        generics: &Generics,
        extra: &WherePredicate,
    ) -> TokenStream {
        let mut predicates = match (&self.generics.where_clause, &generics.where_clause) {
            (Some(w), None) | (None, Some(w)) => w.predicates.iter().collect::<Vec<_>>(),
            (Some(w1), Some(w2)) => w1
                .predicates
                .iter()
                .chain(w2.predicates.iter())
                .collect::<Vec<_>>(),

            (None, None) => Vec::new(),
        };
        predicates.push(&extra);
        quote! {where #(#predicates ,)* }
    }

    pub(crate) fn enum_variants(&self) -> Vec<TokenStream> {
        self.variants.iter().map(Variant::enum_variant).collect()
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
            .iter()
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
            .iter()
            .map(|outer| {
                let ty = &outer.ty;
                let branches: Vec<TokenStream> = self
                    .variants
                    .iter()
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
                                .iter()
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
                let trait_path = &b.path;
                let items = &b.block.items;
                let fns = b.block.expand_methods(self);
                quote! {
                    impl #lt #trait_path for #wrapper #lt {
                         #(#items)*
                         #(#fns)*
                    }
                }
            })
            .collect()
    }

    pub(crate) fn generate_features(&self) -> TokenStream {
        if self.features.none() {
            // depreciated feature flags only if no features are set

            #[cfg(feature = "from")]
            let from = self.generate_from();
            #[cfg(not(feature = "from"))]
            let from = Vec::<proc_macro2::TokenStream>::new();
            #[cfg(feature = "try_into")]
            let try_into = self.generate_try_from();
            #[cfg(not(feature = "try_into"))]
            let try_into = Vec::<proc_macro2::TokenStream>::new();

            #[cfg(feature = "introspection")]
            let type_fns = self.generate_type_to_str();
            #[cfg(not(feature = "introspection"))]
            let type_fns = proc_macro2::TokenStream::new();
            #[cfg(feature = "is_as")]
            let is_as_fn = self.generate_is_as().unwrap();
            #[cfg(not(feature = "is_as"))]
            let is_as_fn = proc_macro2::TokenStream::new();

            quote! {
                #(#from)*
                #(#try_into)*
                #type_fns
                #is_as_fn
            }
        } else {
            let from = if self.features.from {
                self.generate_from()
            } else {
                Vec::new()
            };

            let try_into = if self.features.try_into {
                self.generate_try_from()
            } else {
                Vec::new()
            };
            let type_fns = if self.features.introspection {
                self.generate_type_to_str()
            } else {
                proc_macro2::TokenStream::new()
            };
            let is_as_fn = if self.features.is_as {
                self.generate_is_as().unwrap()
            } else {
                proc_macro2::TokenStream::new()
            };
            quote! {
                #(#from)*
                #(#try_into)*
                #type_fns
                #is_as_fn
            }
        }
    }

    pub(crate) fn generate_type_to_str(&self) -> TokenStream {
        let wrapper = &self.ident;
        let lt = &self.generics;
        let vis = &self.visibility;
        let names = self
            .variants
            .iter()
            .map(Variant::type_as_string)
            .collect::<Vec<_>>();
        let count = names.len();
        let arms = self
            .variants
            .iter()
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
            .iter()
            .map(|v| {
                // let ident = &v.ident;
                let ty = &v.ty;
                let snake = v.ident_as_snake();
                let type_name = v.type_as_string();

                let is_a_arms = self
                    .variants
                    .iter()
                    .map(|i| i.is_a_arm(wrapper, ty))
                    .collect::<Vec<_>>();
                let is_fn = Ident::new(&format!("is_{snake}"), ty.span());
                let is_a_doc = &format!("Returns true if the variant is a {type_name}>");

                let as_arms = self
                    .variants
                    .iter()
                    .map(|i| i.as_arm(wrapper, ty))
                    .collect::<Vec<_>>();
                let as_fn = Ident::new(&format!("try_as_{snake}"), ty.span());
                let as_doc = &format!("Returns the variant as an Option<{type_name}>");

                let as_ref_mut = if let &Type::Reference(_) = ty {
                    quote! {}
                } else {
                    let as_ref_arms = self
                        .variants
                        .iter()
                        .map(|i| i.as_ref_arm(wrapper, ty))
                        .collect::<Vec<_>>();
                    let as_ref_fn = Ident::new(&format!("try_as_{snake}_ref"), ty.span());
                    let as_ref_doc = &format!("Returns the variant as an Option<&{type_name}>");

                    let as_mut_arms = self
                        .variants
                        .iter()
                        .map(|i| i.as_mut_arm(wrapper, ty))
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
