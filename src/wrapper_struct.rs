use core::option::Option::None;
use proc_macro2::{Ident, Span, TokenStream, TokenTree};
use quote::{quote, ToTokens};
use syn::spanned::Spanned;
use syn::{
    parse::{Parse, ParseStream, Parser},
    parse_quote,
    punctuated::Punctuated,
    Attribute, Fields, GenericParam, Generics, ItemStruct, Meta, Token, Visibility, WherePredicate,
};

use crate::{GenericsExt, NodynEnum};

#[derive(Debug, Clone)]
pub(crate) struct WrapperStruct {
    pub(crate) wrapper: ItemStruct,
    // pub(crate) ident: Ident,
    pub(crate) vec_field: Option<Ident>,
    pub(crate) custom_struct: bool,
    pub(crate) traits: Vec<String>,
}

impl WrapperStruct {
    pub(crate) fn traits(attrs: &[Attribute]) -> Vec<String> {
        let mut derive_attrs = Vec::new();
        let parser = Punctuated::<Ident, Token![,]>::parse_terminated;
        let ident = Ident::new("derive", Span::call_site());
        for attr in attrs {
            if let Meta::List(list) = &attr.meta {
                if list.path.is_ident(&ident) {
                    if let Ok(traits) = parser.parse(list.tokens.clone().into()) {
                        let mut traits = traits
                            .iter()
                            .map(ToString::to_string)
                            .collect::<Vec<String>>();
                        derive_attrs.append(&mut traits);
                    }
                }
            }
        }
        derive_attrs
    }

    pub(crate) fn standard_vec_wrapper(
        ident: &Ident,
        vis: &Visibility,
        enum_ident: &Ident,
        generics: &Generics,
        derive_attr: &[Attribute],
    ) -> Self {
        let pound = syn::token::Pound::default();
        let wrapper: ItemStruct = parse_quote! {
            #pound[derive(Default)]
            #(#derive_attr)*
            #vis struct #ident #generics {
                inner: std::vec::Vec< #enum_ident #generics >,
            }
        };
        let traits = Self::traits(derive_attr);
        Self {
            wrapper,
            // ident,
            vec_field: Some(Ident::new("inner", Span::call_site())),
            custom_struct: false,
            traits,
        }
    }

    pub(crate) fn vec_general_code(&self, nodyn: &NodynEnum) -> TokenStream {
        if self.vec_field.is_none() {
            return TokenStream::new();
        }

        // let pound = syn::token::Pound::default();
        let wrapper_struct = self.vec_wrapper_struct(nodyn);
        // let deref_impl = self.vec_deref_deref_mut(nodyn);
        let general_impl = self.vec_general_impl(nodyn);
        // println!("{}", deref_impl.to_string());
        let traits = &self.vec_traits(nodyn);
        quote! {
            #wrapper_struct
            #general_impl
            #traits
        }
    }

    pub(crate) fn vec_general_impl(&self, nodyn: &NodynEnum) -> TokenStream {
        if self.vec_field.is_none() {
            return TokenStream::new();
        }
        // let enum_ident = &nodyn.ident;
        let generics = self.generic_params(nodyn);
        let where_clause = self.where_clause(nodyn);
        // let vis = &self.wrapper.vis;
        let ident = &self.wrapper.ident;
        let type_assoc = &self.vec_default_methods();
        let fns = &self.vec_standard_functions(nodyn);

        quote! {
            impl #generics #ident #generics #where_clause {
                #type_assoc
                #fns
            }
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
            let attrs = &self.wrapper.attrs;
            quote! {
                #(#attrs)*
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

    pub fn generic_params(&self, nodyn: &NodynEnum) -> TokenStream {
        if self.custom_struct {
            nodyn.generic_params(&self.wrapper.generics)
        } else {
            nodyn.enum_generic_params()
        }
    }

    pub fn where_clause(&self, nodyn: &NodynEnum) -> TokenStream {
        if self.custom_struct {
            nodyn.where_clause(&self.wrapper.generics)
        } else {
            nodyn.generics.where_clause.to_token_stream()
        }
    }

    // /// `Deref` and `DerefMut` are only implemented for
    // /// standard wrappers
    // pub(crate) fn vec_deref_deref_mut(&self, nodyn: &NodynEnum) -> TokenStream {
    //     if self.custom_struct || self.vec_field.is_none() {
    //         return TokenStream::new();
    //     }
    //     let ident = &self.wrapper.ident;
    //     let generics = &nodyn.enum_generic_params();
    //     let where_clause = &nodyn.generics.where_clause;
    //     let enum_ident = &nodyn.ident;
    //     let field = self.vec_field.as_ref().unwrap();
    //
    //     quote! {
    //         impl #generics ::std::ops::Deref for #ident #generics #where_clause {
    //             type Target = ::std::vec::Vec< #enum_ident #generics >;
    //             fn deref(&self) -> &Self::Target {
    //                 &self.#field
    //             }
    //         }
    //
    //
    //         impl #generics ::std::ops::DerefMut for #ident #generics #where_clause {
    //             fn deref_mut(&mut self) -> &mut Self::Target {
    //                 &mut self.#field
    //             }
    //         }
    //
    //     }
    //     // - `fn iter(&self) -> Iter<'_, T>`
    // }

    pub(crate) fn merge_generics(
        &self,
        nodyn: &NodynEnum,
        extra1: &GenericParam,
        extra2: &WherePredicate,
    ) -> (TokenStream, TokenStream) {
        if self.custom_struct {
            (
                nodyn.generic_params_extra(&self.wrapper.generics, extra1),
                nodyn.where_clause_extra(&self.wrapper.generics, extra2),
            )
        } else {
            (
                nodyn.enum_generic_params_plus(extra1),
                nodyn.enum_where_plus(extra2),
            )
        }
    }

    pub(crate) fn vec_traits(&self, nodyn: &NodynEnum) -> TokenStream {
        if self.vec_field.is_none() {
            return TokenStream::new();
        }
        let ident = &self.wrapper.ident;
        // let where_clause = &self.where_clause(nodyn);
        let enum_ident = &nodyn.ident;
        let field = self.vec_field.as_ref().unwrap();
        // let vis = &self.wrapper.vis;
        let generics = self.generic_params(nodyn);
        let where_clause = self.where_clause(nodyn);
        let enum_generics = nodyn.enum_generic_params();
        let new_type = &nodyn.generics.new_type();

        let index_g: GenericParam = parse_quote! {#new_type};
        let index_w: WherePredicate = parse_quote! {
            #new_type: ::std::slice::SliceIndex<[#enum_ident #enum_generics]>
        };

        let (index_generics, index_where) = self.merge_generics(nodyn, &index_g, &index_w);

        let lt = &nodyn.generics.new_lifetime();
        let (lt_generics, _) = {
            let index_g: GenericParam = parse_quote! {#lt};
            let index_w: WherePredicate = parse_quote! {W: Clone};
            self.merge_generics(nodyn, &index_g, &index_w)
        };
        quote! {
            impl #index_generics ::std::ops::Index<#new_type> for #ident #generics #index_where {
                type Output = #new_type::Output;
                #[inline]
                fn index(&self, index: #new_type) -> &Self::Output {
                        &self.#field[index]
                }
            }

            impl #index_generics ::std::ops::IndexMut<#new_type> for #ident #generics #index_where {
                #[inline]
                fn index_mut(&mut self, index: #new_type) -> &mut Self::Output {
                        &mut self.#field[index]
                }
            }

            impl #lt_generics ::std::iter::IntoIterator for &#lt #ident #generics #where_clause {
                type Item = &#lt #enum_ident #enum_generics;
                type IntoIter = ::std::slice::Iter<#lt, #enum_ident #enum_generics>;
                fn into_iter(self) -> Self::IntoIter {
                    self.#field.iter()
                }
            }

            impl #lt_generics ::std::iter::IntoIterator for &#lt mut #ident #generics #where_clause {
                type Item = &#lt mut #enum_ident #enum_generics;
                type IntoIter = ::std::slice::IterMut<#lt, #enum_ident #enum_generics>;
                fn into_iter(self) -> Self::IntoIter {
                    self.#field.iter_mut()
                }
            }

            impl #generics ::std::iter::IntoIterator for #ident #generics #where_clause {
                type Item = #enum_ident #enum_generics;
                type IntoIter = ::std::vec::IntoIter<#enum_ident #enum_generics>;
                fn into_iter(self) -> Self::IntoIter {
                    self.#field.into_iter()
                }
            }

        }
    }

    /// Vec wrapper methods that depend on the `Default` trait:
    ///
    /// - [`fn new() -> Self`](Vec::new())
    /// - [`fn with_capacity(capacity: usize) -> Self`](Vec::with_capacity())
    pub(crate) fn vec_default_methods(&self) -> TokenStream {
        if self.vec_field.is_none() || !self.traits.contains(&"Default".to_string()) {
            return TokenStream::new();
        }
        let field = self.vec_field.as_ref().unwrap();
        let vis = &self.wrapper.vis;

        quote! {
            #vis fn new() -> Self {
                Self::default()
            }

            #vis fn with_capacity(capacity: usize) -> Self {
                Self {
                    #field: std::vec::Vec::with_capacity(capacity),
                    ..::std::default::Default::default()
                }
            }
        }
    }

    pub(crate) fn vec_copy_traits(&self, nodyn: &NodynEnum) -> TokenStream {
        if self.vec_field.is_none() || !self.traits.contains(&"Copy".to_string()) {
            return TokenStream::new();
        }
        let field = self.vec_field.as_ref().unwrap();
        let vis = &self.wrapper.vis;
        let wrapper_ident = &self.wrapper.ident;
        // let where_clause = &self.where_clause(nodyn);
        let enum_ident = &nodyn.ident;
        // let vis = &self.wrapper.vis;
        let wrapper_generics = self.generic_params(nodyn);
        let where_clause = self.where_clause(nodyn);
        let enum_generics = nodyn.enum_generic_params();
        let lt = &nodyn.generics.new_lifetime();
        let (lt_generics, _) = {
            let index_g: GenericParam = parse_quote! {#lt};
            let index_w: WherePredicate = parse_quote! {W: Clone};
            self.merge_generics(nodyn, &index_g, &index_w)
        };

        quote! {
            impl #wrapper_generics Extend<#enum_ident #enum_generics> for #wrapper_ident #wrapper_generics #where_clause {
                fn extend<I: ::std::vec::IntoIterator<Item = #enum_ident #enum_generics>>(self, iter: I) {
            impl #lt_generics Extend<&#lt #enum_ident #enum_generics> for #wrapper_ident #wrapper_generics #where_clause {
                fn extend<I: ::std::vec::IntoIterator<Item = &#lt #enum_ident #enum_generics>>(&mut self, iter: I) {
                    self.#field.extend(iter.into_iter())
                }
            }
                    self.#field.extend(iter.into_iter())
                }
            }

            impl #lt_generics Extend<&#lt #enum_ident #enum_generics> for #wrapper_ident #wrapper_generics #where_clause {
                fn extend<I: ::std::vec::IntoIterator<Item = &#lt #enum_ident #enum_generics>>(&mut self, iter: I) {
                    self.#field.extend(iter.into_iter())
                }
            }
        }
    }

    /// implemented:
    /// - `fn capacity(&self) -> usize`
    /// - `fn reserve(&mut self, additional: usize)`
    /// - `fn reserve_exact(&mut self, additional: usize)`
    /// - `fn try_reserve(&mut self, additional: usize) -> Result<(), TryReserveError>`
    /// - `fn try_reserve_exact(&mut self, additional: usize) -> Result<(), TryReserveError>`
    /// - `fn shrink_to_fit(&mut self)`
    /// - `fn shrink_to(&mut self, min_capacity: usize)`
    /// - `fn into_boxed_slice(self) -> Box<[T], A>`
    /// - `fn truncate(&mut self, len: usize)`
    /// - `const fn as_slice(&self) -> &[T]`
    /// - `const fn as_mut_slice(&mut self) -> &mut [T]`
    /// - `fn swap_remove(&mut self, index: usize) -> T`
    /// - `fn remove(&mut self, index: usize) -> T`
    /// - `fn retain<F>(&mut self, f: F)`
    /// - `fn retain_mut<F>(&mut self, f: F)`
    /// - `fn pop(&mut self) -> Option<T>`
    /// - `fn pop_if(&mut self, predicate: impl FnOnce(&mut T) -> bool) -> Option<T>`
    /// - `fn append(&mut self, other: &mut Vec<T, A>)`
    /// - `fn clear(&mut self)`
    /// - `const fn len(&self) -> usize`
    /// - `const fn is_empty(&self) -> bool`
    /// - `fn first(&self) -> Option<&T>`
    /// - `fn first_mut(&mut self) -> Option<&mut T>`
    /// - `fn last(&self) -> Option<&T>`
    /// - `fn last_mut(&mut self) -> Option<&mut T>`
    /// - `fn split_first(&self) -> Option<(&T, &[T])>`
    /// - `fn split_first_mut(&mut self) -> Option<(&mut T, &mut [T])>`
    /// - `fn split_last(&self) -> Option<(&T, &[T])>`
    /// - `fn split_last_mut(&mut self) -> Option<(&mut T, &mut [T])>`
    /// - `fn get<I>(&self, index: I) -> Option<&<I as SliceIndex<[T]>>::Output>`
    /// - `fn get_mut<I>(&mut self, index: I,) -> Option<&mut <I as SliceIndex<[T]>>::Output>`
    /// - `fn swap(&mut self, a: usize, b: usize)`
    /// - `fn reverse(&mut self)`
    /// - `fn iter(&self) -> Iter<'_, T>`
    /// - `fn iter_mut(&mut self) -> IterMut<'_, T>`
    ///
    /// implemented using Into:
    /// - `fn insert(&mut self, index: usize, element: T)`
    /// - `fn push(&mut self, value: T)`
    ///
    /// Not implemented:
    /// - `fn dedup_by_key<F, K>(&mut self, key: F)`
    /// - `fn dedup_by<F>(&mut self, same_bucket: F)`
    /// - `fn drain<R>(&mut self, range: R) -> Drain<'_, T, A>`
    /// - `fn split_off(&mut self, at: usize) -> Vec<T, A>`
    /// - `fn resize_with<F>(&mut self, new_len: usize, f: F)`
    /// - `fn leak<'a>(self) -> &'a mut [T]`
    /// - `fn spare_capacity_mut(&mut self) -> &mut [MaybeUninit<T>]`
    /// - `fn first_chunk<const N: usize>(&self) -> Option<&[T; N]>`
    /// - `fn first_chunk_mut<const N: usize>(&mut self) -> Option<&mut [T; N]>`
    /// - `fn split_first_chunk<const N: usize>(&self) -> Option<(&[T; N], &[T])>`
    /// - `fn split_first_chunk_mut<const N: usize>(&mut self) -> Option<(&mut [T; N], &mut [T])>`
    /// - `fn split_last_chunk<const N: usize>(&self) -> Option<(&[T], &[T; N])>`
    /// - `fn split_last_chunk_mut<const N: usize>(&mut self) -> Option<(&mut [T], &mut [T; N])>`
    /// - `fn last_chunk<const N: usize>(&self) -> Option<&[T; N]>`
    /// - `fn last_chunk_mut<const N: usize>(&mut self) -> Option<&mut [T; N]>`
    /// - `unsafe fn get_unchecked<I>(&self, index: I,) -> &<I as SliceIndex<[T]>>::Output`
    /// - `unsafe fn get_unchecked_mut<I>(&mut self, index: I,) -> &mut <I as SliceIndex<[T]>>::Output`
    /// - `fn as_ptr(&self) -> *const T`
    /// - `fn as_mut_ptr(&mut self) -> *mut T`
    /// - `fn as_ptr_range(&self) -> Range<*const T>`
    /// - `fn as_mut_ptr_range(&mut self) -> Range<*mut T>`
    ///
    ///
    /// ```
    // did until windows
    #[allow(clippy::too_many_lines)]
    pub(crate) fn vec_standard_functions(&self, nodyn: &NodynEnum) -> TokenStream {
        if self.vec_field.is_none() {
            return TokenStream::new();
        }
        let field = self.vec_field.as_ref().unwrap();
        let vis = &self.wrapper.vis;
        let enum_ident = &nodyn.ident;
        // let generics = self.generic_params(nodyn);
        let enum_generics = nodyn.enum_generic_params();

        quote! {
            #vis fn capacity(&self) -> usize {
                self.#field.capacity()
            }

            #vis fn reserve(&mut self, additional: usize) {
                self.#field.reserve(additional);
            }

            #vis fn reserve_exact(&mut self, additional: usize) {
                self.#field.reserve_exact(additional);
            }

            #vis fn try_reserve(&mut self, additional: usize) -> ::core::result::Result<(), ::std::collections::TryReserveError> {
                self.#field.try_reserve(additional)
            }

            #vis fn try_reserve_exact(&mut self, additional: usize) -> ::core::result::Result<(), ::std::collections::TryReserveError> {
                self.#field.try_reserve_exact(additional)
            }

            #vis fn shrink_to_fit(&mut self) {
                self.#field.shrink_to_fit();
            }

            #vis fn shrink_to(&mut self, min_capacity: usize) {
                self.#field.shrink_to(min_capacity);
            }

            #vis fn into_boxed_slice(self) -> ::std::boxed::Box<[#enum_ident #enum_generics]> {
                self.#field.into_boxed_slice()
            }

            #vis fn truncate(&mut self, len: usize) {
                self.#field.truncate(len);
            }

            #vis const fn as_slice(&self) -> &[#enum_ident #enum_generics] {
                self.#field.as_slice()
            }

            #vis const fn as_mut_slice(&mut self) -> &mut [#enum_ident #enum_generics] {
                self.#field.as_mut_slice()
            }

            #vis fn swap_remove(&mut self, index: usize) -> #enum_ident #enum_generics {
                self.#field.swap_remove(index)
            }

            #vis fn insert<I: Into<#enum_ident #enum_generics>>(&mut self, index: usize, element: I) {
                self.#field.insert(index, element.into())
            }

            #vis fn remove(&mut self, index: usize) -> #enum_ident #enum_generics {
                self.#field.remove(index)
            }

            #vis fn retain<F>(&mut self, f: F)
            where F: FnMut(&#enum_ident #enum_generics) -> bool {
                self.#field.retain(f);
            }

            #vis fn retain_mut<F>(&mut self, f: F)
            where F: FnMut(&mut #enum_ident #enum_generics) -> bool {
                self.#field.retain_mut(f);
            }

            #vis fn push<I: Into<#enum_ident #enum_generics>>(&mut self, value: I) {
                self.#field.push(value. into());
            }

            #vis fn pop(&mut self) -> Option<#enum_ident #enum_generics> {
                self.#field.pop()
            }

            #vis fn pop_if(&mut self, predicate: impl FnOnce(&mut #enum_ident #enum_generics) -> bool) -> Option<#enum_ident #enum_generics> {
                self.#field.pop_if(predicate)
            }

            #vis fn append(&mut self, other: &mut Self) {
                self.#field.append(&mut other.#field);
            }

            #vis fn clear(&mut self) {
                self.#field.clear();
            }

            #vis const fn len(&self) -> usize {
                self.#field.len()
            }

            #vis const fn is_empty(&self) -> bool {
                self.#field.is_empty()
            }

            #vis fn first(&self) -> Option<&#enum_ident #enum_generics> {
                self.#field.first()
            }

            #vis fn first_mut(&mut self) -> Option<&mut #enum_ident #enum_generics> {
                self.#field.first_mut()
            }

            #vis fn last(&self) -> Option<&#enum_ident #enum_generics> {
                self.#field.last()
            }

            #vis fn last_mut(&mut self) -> Option<&mut #enum_ident #enum_generics> {
                self.#field.last_mut()
            }

            #vis fn split_first(&self) -> Option<(&#enum_ident #enum_generics, &[#enum_ident #enum_generics])> {
                self.#field.split_first()
            }

            #vis fn split_first_mut(&mut self) -> Option<(&mut #enum_ident #enum_generics, &mut [#enum_ident #enum_generics])> {
                self.#field.split_first_mut()
            }

            #vis fn split_last(&self) -> Option<(&#enum_ident #enum_generics, &[#enum_ident #enum_generics])> {
                self.#field.split_last()
            }

            #vis fn split_last_mut(&mut self) -> Option<(&mut #enum_ident #enum_generics, &mut [#enum_ident #enum_generics])> {
                self.#field.split_last_mut()
            }

            #vis fn get<I>(&self, index: I) -> Option<&<I as ::std::slice::SliceIndex<[#enum_ident #enum_generics]>>::Output>
            where
                I: ::std::slice::SliceIndex<[#enum_ident #enum_generics]>,
            {
                self.#field.get(index)
            }

            #vis fn get_mut<I>(&mut self, index: I) -> Option<&mut <I as ::std::slice::SliceIndex<[#enum_ident #enum_generics]>>::Output>
            where
                I: ::std::slice::SliceIndex<[#enum_ident #enum_generics]>,
            {
                self.#field.get_mut(index)
            }

            #vis fn swap(&mut self, a: usize, b: usize) {
                self.#field.swap(a, b);
            }

            #vis fn reverse(&mut self) {
                self.#field.reverse();
            }

            #vis fn iter(&self) -> ::std::slice::Iter<'_, #enum_ident #enum_generics> {
                self.#field.iter()
            }

            #vis fn iter_mut(&mut self) -> ::std::slice::IterMut<'_, #enum_ident #enum_generics> {
                self.#field.iter_mut()
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
                    vec_field = Some(Ident::new("inner_vec", Span::call_site()));
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
        let traits = Self::traits(&attrs);
        wrapper.attrs = attrs;

        Ok(Self {
            // ident: wrapper.ident.clone(),
            wrapper,
            vec_field,
            custom_struct: true,
            traits,
        })
    }
}
