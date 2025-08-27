use core::option::Option::None; // for analyzer
use std::collections::HashSet;

use proc_macro2::TokenStream;
use quote::{ToTokens, format_ident, quote};
use syn::parse::Parse;
use syn::parse::ParseStream;
use syn::{
    Attribute, FnArg, Generics, Ident, Meta, Path, Token, Type, Visibility, WherePredicate,
    punctuated::Punctuated, spanned::Spanned,
};

use crate::vec_wrapper::StandardVecWrapper;
use crate::{MethodImpl, OptionalImpl, TraitImpl, Variant, VecWrapper, keyword};

/// Represents the input for the `nodyn` procedural macro, defining a nodyn enum.
#[derive(Debug, Clone)]
pub(crate) struct NodynEnum {
    /// Attributes applied to the enum (e.g., `#[derive(Debug)]`).
    pub(crate) attrs: Vec<Attribute>,
    /// Visibility of the enum (e.g., `pub`, `pub(crate)`).
    pub(crate) visibility: Visibility,
    /// The identifier of the enum (e.g., `MyEnum`).
    pub(crate) ident: Ident,
    /// Generic parameters of the enum.
    pub(crate) generics: Generics,
    /// Variants of the enum, each holding a type and optional `#[into]` attributes.
    pub(crate) variants: Vec<Variant>,
    /// Method implementations for the enum.
    pub(crate) method_impls: Vec<MethodImpl>,
    /// Trait implementations for the enum.
    pub(crate) trait_impls: Vec<TraitImpl>,
    /// Enabled features (`TryInto`, `is_as`, `introspection`).
    pub(crate) optional_impl: OptionalImpl,
    /// Wrapper structs for collections (e.g., `Vec`-based structs).
    pub(crate) vec_wrappers: Vec<VecWrapper>,
    /// module path to where the macro is invoked, used for vec wrapper macro
    pub(crate) module_path: Option<Path>,
}

impl Parse for NodynEnum {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let attrs = input.call(Attribute::parse_outer)?;
        let (module_path, attrs) = Self::extract_module_path(&attrs);
        let visibility = input.parse::<Visibility>()?;
        let _ = input.parse::<syn::token::Enum>()?;
        let ident = input.parse::<Ident>()?;
        let generics = input.parse::<Generics>()?;

        let content;
        syn::braced!(content in input);
        let variants = Punctuated::<Variant, Token![,]>::parse_terminated(&content)?
            .into_iter()
            .collect::<Vec<_>>();

        // Ensure unique variant types
        let mut existing_types = HashSet::new();
        for variant in &variants {
            if !existing_types.insert(variant.ty.clone()) {
                return Err(syn::Error::new(
                    variant.ty.span(),
                    "Duplicate variant type detected",
                ));
            }
        }

        let derive_attrs = Self::extract_derive_attrs(&attrs);
        let mut impl_blocks = Vec::new();
        let mut trait_blocks = Vec::new();
        let mut features = OptionalImpl::default();
        let mut collection_structs = Vec::new();

        // Parse additional impl blocks and wrapper structs
        while !input.is_empty() {
            if input.peek(Token![impl]) {
                input.parse::<syn::token::Impl>()?;
                if input.peek(keyword::TryInto)
                    || input.peek(keyword::is_as)
                    || input.peek(keyword::introspection)
                {
                    features.merge(input.parse::<OptionalImpl>()?);
                } else if input.peek(Ident) {
                    trait_blocks.push(input.parse::<TraitImpl>()?);
                } else {
                    impl_blocks.push(input.parse::<MethodImpl>()?);
                }
            } else if let Ok(wrapper_struct) = input.parse::<VecWrapper>() {
                collection_structs.push(wrapper_struct);
            } else if let Ok(standard_wrapper) = input.parse::<StandardVecWrapper>() {
                collection_structs.push(standard_wrapper.into_vec_wrapper(
                    &visibility,
                    &ident,
                    &generics,
                    &derive_attrs,
                ));
            } else {
                return Err(syn::Error::new(
                    input.span(),
                    "Expected 'impl' or struct item",
                ));
            }
        }

        Ok(Self {
            attrs,
            visibility,
            // enum_token,
            ident,
            generics,
            variants,
            method_impls: impl_blocks,
            trait_impls: trait_blocks,
            optional_impl: features,
            vec_wrappers: collection_structs,
            module_path,
        })
    }
}

impl NodynEnum {
    pub(crate) fn to_token_stream(&self) -> TokenStream {
        let enum_definition = self.enum_definition_tokens();
        let default = self.default_tokens();
        let optional = self.optional_tokens();
        let methods = self.method_tokens();
        let traits = self.trait_tokens();
        let vec_wrappers = self
            .vec_wrappers
            .iter()
            .map(|s| s.to_token_stream(self))
            .collect::<Vec<_>>();

        quote! {
            #enum_definition
            #default
            #optional
            #(#methods)*
            #(#traits)*
            #(#vec_wrappers)*
        }
    }

    /// Extract `nodyn_path` attribute from provide attributes.
    fn extract_module_path(attrs: &[Attribute]) -> (Option<Path>, Vec<Attribute>) {
        (attrs.iter()
            .find(|attr| matches!(&attr.meta, Meta::NameValue(meta) if meta.path.is_ident("module_path")))
            .map(|attr| {
                if let Meta::NameValue(meta) = &attr.meta {
                    syn::parse_str::<Path>(&meta.value.to_token_stream().to_string().replace([' ', '"'],"")).expect("nodyn_path value is no path")
                } else {
                    panic!("attribute is not #[module_path = \"some::module\"]");
                }
            }),

        attrs
            .iter()
            .filter(|attr| !matches!(&attr.meta, Meta::NameValue(meta) if meta.path.is_ident("module_path")))
            .cloned()
            .collect()
            )
    }

    /// Extracts `#[derive]` attributes from the provided attributes.
    fn extract_derive_attrs(attrs: &[Attribute]) -> Vec<Attribute> {
        attrs
            .iter()
            .filter(|attr| matches!(&attr.meta, Meta::List(list) if list.path.is_ident("derive")))
            .cloned()
            .collect()
    }

    /// Generates a `TokenStream` for the generic parameters, combining enum and additional generics.
    pub(crate) fn merged_generics_tokens(&self, other: &Generics) -> TokenStream {
        let params = self
            .generics
            .params
            .iter()
            .chain(other.params.iter())
            .collect::<Vec<_>>();
        if params.is_empty() {
            TokenStream::new()
        } else {
            quote! { <#(#params),*> }
        }
    }

    /// Generates a `TokenStream` for the enum's generic parameters.
    pub(crate) fn generics_tokens(&self) -> TokenStream {
        let generics = self.generics.params.iter().collect::<Vec<_>>();
        if generics.is_empty() {
            TokenStream::new()
        } else {
            quote! { <#(#generics,)*> }
        }
    }

    pub(crate) fn where_and_predicate_tokens(&self, predicate: &WherePredicate) -> TokenStream {
        let mut where_clause = if let Some(clause) = &self.generics.where_clause {
            clause.predicates.iter().collect::<Vec<_>>()
        } else {
            Vec::new()
        };
        where_clause.push(predicate);
        if where_clause.is_empty() {
            TokenStream::new()
        } else {
            quote! { where #(#where_clause,)*  }
        }
    }

    pub(crate) fn merged_where_tokens(&self, generics: &Generics) -> TokenStream {
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

    pub(crate) fn merged_where_and_predicate_tokens(
        &self,
        generics: &Generics,
        predicate: &WherePredicate,
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
        predicates.push(predicate);
        quote! {where #(#predicates ,)* }
    }

    fn enum_definition_tokens(&self) -> TokenStream {
        let variants = self.variants.iter().map(Variant::enum_variant_tokens);
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

    #[allow(clippy::wrong_self_convention)]
    fn from_tokens(&self) -> Vec<TokenStream> {
        let ident = &self.ident;
        let generics = &self.generics;
        self.variants
            .iter()
            .map(|variant| {
                let ty = &variant.ty;
                let variant_ident = &variant.ident;
                quote! {
                    impl #generics ::core::convert::From<#ty> for #ident #generics {
                        fn from(value: #ty) -> Self {
                            #ident::#variant_ident(value)
                        }
                    }
                }
            })
            .collect()
    }

    fn try_from_tokens(&self) -> Vec<TokenStream> {
        let ident = &self.ident;
        let generics = &self.generics;
        self.variants
            .iter()
            .map(|outer| {
                let ty = &outer.ty;
                let arms: Vec<TokenStream> = self
                    .variants
                    .iter()
                    .map(|inner| inner.try_from_arm_tokens(outer, ident))
                    .collect();
                quote! {
                    impl #generics ::core::convert::TryFrom<#ident #generics> for #ty {
                        type Error = &'static str;
                        fn try_from(other: #ident #generics) -> ::core::result::Result< Self, Self::Error >
                        {
                            match other {
                                #(#arms)*
                            }
                        }
                    }
                }
            })
            .collect()
    }

    /// Generate delegation methods for shared methods.
    fn method_tokens(&self) -> Vec<TokenStream> {
        let ident = &self.ident;
        let generics = &self.generics;
        self.method_impls
            .iter()
            .map(|block| {
                let items = &block.items;
                let methods = block
                    .functions
                    .iter()
                    .filter_map(|f| {
                        if let Some(FnArg::Receiver(_)) = f.sig.inputs.first() {
                            let arms = self
                                .variants
                                .iter()
                                .map(|v| v.fn_call_arm_tokens(ident, &f.sig.ident, &f.sig.inputs));
                            let attrs = &f.attrs;
                            let vis = &f.vis;
                            let signature = &f.sig;
                            Some(quote! {
                                #(#attrs)*
                                #vis #signature {
                                    match self {
                                        #(#arms)*
                                    }
                                }
                            })
                        } else {
                            None
                        }
                    })
                    .collect::<Vec<_>>();

                quote! {
                    impl #generics #ident #generics {
                        #(#items)*
                        #(#methods)*
                    }
                }
            })
            .collect()
    }

    fn trait_tokens(&self) -> Vec<TokenStream> {
        let wrapper = &self.ident;
        let lt = &self.generics;
        self.trait_impls
            .iter()
            .map(|b| {
                let trait_path = &b.path;
                let items = &b.block.items;
                let fns = b.block.expand_methods_tokens(self);
                quote! {
                    impl #lt #trait_path for #wrapper #lt {
                         #(#items)*
                         #(#fns)*
                    }
                }
            })
            .collect()
    }

    /// Generates type introspection methods (`count`, `types`, `type_name`).
    fn introspection_tokens(&self) -> TokenStream {
        let ident = &self.ident;
        let generics = &self.generics;
        let visibility = &self.visibility;
        let variant_count = self.variants.len();
        let type_names = self
            .variants
            .iter()
            .map(Variant::type_to_string)
            .collect::<Vec<_>>();
        let arms = self
            .variants
            .iter()
            .map(|v| v.type_as_str_arm_tokens(ident));

        quote! {
            impl #generics #ident #generics {
                /// Returns the number of variants in the enum.
                #visibility const fn count() -> usize {
                    #variant_count
                }

                /// Returns an array of variant type names as `&'static str`.
                #visibility const fn types() -> [&'static str; #variant_count] {
                    [#(#type_names),*]
                }

                /// Returns the type name of the current variant as `&'static str`.
                #visibility const fn type_name(&self) -> &'static str {
                    match self {
                        #(#arms)*
                    }
                }
            }
        }
    }

    /// Generates type checking and conversion methods (`is_`, `try_as_`, etc.).
    ///
    /// Skips `try_as_ref` and `try_as_mut` for reference types to avoid redundant implementations.
    fn is_as_tokens(&self) -> syn::Result<TokenStream> {
        let ident = &self.ident;
        let generics = &self.generics;
        let methods = self
            .variants
            .iter()
            .map(|variant| {
                let ty = &variant.ty;
                let snake = variant.ident_to_snake();
                let type_name = variant.type_to_string();

                let is_fn = format_ident!("is_{}", snake);
                let is_doc = format!("Returns `true` if the variant is `{type_name}`.");
                let is_arms = self
                    .variants
                    .iter()
                    .map(|v| v.is_type_arm_tokens(ident, ty));

                let as_fn = format_ident!("try_as_{}", snake);
                let as_doc = format!("Converts to `Option<{type_name}>` if possible.");
                let as_arms = self
                    .variants
                    .iter()
                    .map(|v| v.as_type_arm_tokens(ident, ty));

                let ref_mut_methods = if matches!(ty, Type::Reference(_)) {
                    quote! {}
                } else {
                    let as_ref_fn = format_ident!("try_as_{}_ref", snake);
                    let as_ref_doc =
                        format!("Returns `Option<&{type_name}>` if the variant is `{type_name}`.");
                    let as_ref_arms = self.variants.iter().map(|v| v.as_ref_arm_tokens(ident, ty));

                    let as_mut_fn = format_ident!("try_as_{}_mut", snake);
                    let as_mut_doc = format!(
                        "Returns `Option<&mut {type_name}>` if the variant is `{type_name}`."
                    );
                    let as_mut_arms = self.variants.iter().map(|v| v.as_mut_arm_tokens(ident, ty));

                    quote! {
                        #[doc = #as_ref_doc]
                        pub fn #as_ref_fn(&self) -> ::core::option::Option<&#ty> {
                            match self {
                                #(#as_ref_arms)*
                                _ => ::core::option::Option::None,
                            }
                        }

                        #[doc = #as_mut_doc]
                        pub fn #as_mut_fn(&mut self) -> ::core::option::Option<&mut #ty> {
                            match self {
                                #(#as_mut_arms)*
                                _ => ::core::option::Option::None,
                            }
                        }
                    }
                };

                Ok(quote! {
                    #[doc = #is_doc]
                    pub fn #is_fn(&self) -> bool {
                        match self {
                            #(#is_arms)*
                            _ => false,
                        }
                    }

                    #[doc = #as_doc]
                    pub fn #as_fn(self) -> ::core::option::Option<#ty> {
                        match self {
                            #(#as_arms)*
                            _ => ::core::option::Option::None,
                        }
                    }

                    #ref_mut_methods
                })
            })
            .collect::<syn::Result<Vec<_>>>()?;

        Ok(quote! {
            impl #generics #ident #generics {
                #(#methods)*
            }
        })
    }

    /// Generates vector accessor methods for a given `Vec` field in a vec wrapper.
    pub(crate) fn variant_vec_tokens(&self, vec_field: &Ident) -> TokenStream {
        let methods = self
            .variants
            .iter()
            .map(|v| v.vec_methods_tokens(&self.ident, vec_field));
        quote! { #(#methods)* }
    }

    /// returns a `TokenStream` that is always included
    fn default_tokens(&self) -> TokenStream {
        let from = self.from_tokens();
        quote! {
            #(#from)*
        }
    }

    fn optional_tokens(&self) -> TokenStream {
        if self.optional_impl.none() {
            // depreciated feature flags only if no features are set
            #[cfg(feature = "try_into")]
            eprintln!(
                "Warning: The `try_into` cargo feature is deprecated. Use `impl TryInto` in the nodyn! macro."
            );
            #[cfg(feature = "introspection")]
            eprintln!(
                "Warning: The `introspection` cargo feature is deprecated. Use `impl introspection` in the nodyn! macro."
            );
            #[cfg(feature = "is_as")]
            eprintln!(
                "Warning: The `is_as` cargo feature is deprecated. Use `impl is_as` in the nodyn! macro."
            );
            #[cfg(feature = "try_into")]
            let try_into = self.try_from_tokens();
            #[cfg(not(feature = "try_into"))]
            let try_into = Vec::<proc_macro2::TokenStream>::new();

            #[cfg(feature = "introspection")]
            let type_fns = self.introspection_tokens();
            #[cfg(not(feature = "introspection"))]
            let type_fns = proc_macro2::TokenStream::new();

            #[cfg(feature = "is_as")]
            let is_as_fn = self.is_as_tokens().unwrap();
            #[cfg(not(feature = "is_as"))]
            let is_as_fn = proc_macro2::TokenStream::new();

            quote! {
                #(#try_into)*
                #type_fns
                #is_as_fn
            }
        } else {
            let try_into = if self.optional_impl.try_into {
                self.try_from_tokens()
            } else {
                Vec::new()
            };
            let type_fns = if self.optional_impl.introspection {
                self.introspection_tokens()
            } else {
                proc_macro2::TokenStream::new()
            };
            let is_as_fn = if self.optional_impl.is_as {
                self.is_as_tokens().unwrap()
            } else {
                proc_macro2::TokenStream::new()
            };
            quote! {
                #(#try_into)*
                #type_fns
                #is_as_fn
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use syn::parse_str;

    #[test]
    fn test_nodyn_enum_parsing() {
        let input = parse_str::<NodynEnum>(
            "
            #[derive(Debug)]
            pub enum MyEnum {
                Number(i32),
                String,
            }
            ",
        )
        .unwrap();

        assert_eq!(input.ident.to_string(), "MyEnum");
        assert_eq!(input.variants.len(), 2);
        assert_eq!(input.variants[0].ident.to_string(), "Number");
        assert_eq!(input.variants[0].type_to_string(), "i32");
        assert_eq!(input.variants[1].ident.to_string(), "String");
        assert_eq!(input.variants[1].type_to_string(), "String");
        assert_eq!(input.method_impls.len(), 0);
        assert_eq!(input.trait_impls.len(), 0);
        assert_eq!(input.vec_wrappers.len(), 0);
        assert_eq!(input.attrs.len(), 1); // #[derive(Debug)]
    }

    #[test]
    fn test_duplicate_variant_types() {
        let result = parse_str::<NodynEnum>(
            "
            pub enum MyEnum {
                Number(i32),
                Another(i32),
            }
            ",
        );
        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("Duplicate variant type")
        );
    }

    #[test]
    fn test_vec_wrapper_parsing() {
        let input = parse_str::<NodynEnum>(
            "
            pub enum MyEnum {
                Number(i32),
            }
            vec MyEnumVec;
            ",
        )
        .unwrap();

        assert_eq!(input.vec_wrappers.len(), 1);
        assert_eq!(
            input.vec_wrappers[0].definition.ident.to_string(),
            "MyEnumVec"
        );
    }

    #[test]
    fn test_features_parsing() {
        let input = parse_str::<NodynEnum>(
            "
            pub enum MyEnum {
                Number(i32),
            }
            impl TryInto;
            ",
        )
        .unwrap();

        assert!(input.optional_impl.try_into);
        assert!(!input.optional_impl.is_as);
        assert!(!input.optional_impl.introspection);
    }
}
