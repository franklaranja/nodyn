use core::option::Option::None;
use proc_macro2::{Ident, Span, TokenStream, TokenTree};
use quote::{ToTokens, quote};
use syn::spanned::Spanned;
use syn::{
    Attribute, Fields, GenericParam, Generics, ItemStruct, Meta, Token, Visibility, WherePredicate,
    parse::{Parse, ParseStream, Parser},
    parse_quote,
    punctuated::Punctuated,
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
        let clone_traits = &self.vec_clone_traits_and_functions(nodyn);
        let clone_default = &self.vec_clone_default_traits(nodyn);
        let default = &self.vec_default_traits_and_methods(nodyn);
        quote! {
            #wrapper_struct
            #general_impl
            #traits
            #default
            #clone_traits
            #clone_default
        }
    }

    pub(crate) fn vec_general_impl(&self, nodyn: &NodynEnum) -> TokenStream {
        if self.vec_field.is_none() {
            return TokenStream::new();
        }
        let generics = self.generic_params(nodyn);
        let where_clause = self.where_clause(nodyn);
        let ident = &self.wrapper.ident;
        let fns = &self.vec_standard_functions(nodyn);
        let changed_fns = &self.vec_changed_functions(nodyn);
        let partial_eq_fns = &self.vec_partial_eq_methods();
        let field = self.vec_field.as_ref().unwrap();
        let variant_fns = nodyn.generate_variant_vec_fns(field);

        quote! {
            impl #generics #ident #generics #where_clause {
                #fns
                #changed_fns
                #partial_eq_fns
                #variant_fns
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
    //         impl #generics ::core::ops::Deref for #ident #generics #where_clause {
    //             type Target = ::std::vec::Vec< #enum_ident #generics >;
    //             fn deref(&self) -> &Self::Target {
    //                 &self.#field
    //             }
    //         }
    //
    //
    //         impl #generics ::core::ops::DerefMut for #ident #generics #where_clause {
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

    /// - `From<Self> for Vec<enum>`
    /// - `Index` & `IndexMut`
    /// - 3x `IntoIterator`
    /// - `AsRef` & `AsMut` for `Self`, `Vec<enum>` and `&[enum]`
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
            #new_type: ::core::slice::SliceIndex<[#enum_ident #enum_generics]>
        };

        let (index_generics, index_where) = self.merge_generics(nodyn, &index_g, &index_w);

        let lt = &nodyn.generics.new_lifetime();
        let (lt_generics, _) = {
            let index_g: GenericParam = parse_quote! {#lt};
            let index_w: WherePredicate = parse_quote! {W: Clone};
            self.merge_generics(nodyn, &index_g, &index_w)
        };
        quote! {
            impl #generics From<#ident #generics> for Vec<#enum_ident #enum_generics> #where_clause {
                fn from(v: #ident #generics) -> Vec<#enum_ident #enum_generics> {
                    v.#field
                }
            }

            impl #index_generics ::core::ops::Index<#new_type> for #ident #generics #index_where {
                type Output = #new_type::Output;
                #[inline]
                fn index(&self, index: #new_type) -> &Self::Output {
                        &self.#field[index]
                }
            }

            impl #index_generics ::core::ops::IndexMut<#new_type> for #ident #generics #index_where {
                #[inline]
                fn index_mut(&mut self, index: #new_type) -> &mut Self::Output {
                        &mut self.#field[index]
                }
            }

            impl #lt_generics ::core::iter::IntoIterator for &#lt #ident #generics #where_clause {
                type Item = &#lt #enum_ident #enum_generics;
                type IntoIter = ::core::slice::Iter<#lt, #enum_ident #enum_generics>;
                fn into_iter(self) -> Self::IntoIter {
                    self.#field.iter()
                }
            }

            impl #lt_generics ::core::iter::IntoIterator for &#lt mut #ident #generics #where_clause {
                type Item = &#lt mut #enum_ident #enum_generics;
                type IntoIter = ::core::slice::IterMut<#lt, #enum_ident #enum_generics>;
                fn into_iter(self) -> Self::IntoIter {
                    self.#field.iter_mut()
                }
            }

            impl #generics ::core::iter::IntoIterator for #ident #generics #where_clause {
                type Item = #enum_ident #enum_generics;
                type IntoIter = ::std::vec::IntoIter<#enum_ident #enum_generics>;
                fn into_iter(self) -> Self::IntoIter {
                    self.#field.into_iter()
                }
            }

            impl #generics ::core::convert::AsRef<#ident #generics> for #ident #generics #where_clause  {
                fn as_ref(&self) -> &Self {
                    self
                }
            }

            impl #generics ::core::convert::AsMut<#ident #generics> for #ident #generics #where_clause {
                fn as_mut(&mut self) -> &mut Self {
                    self
                }
            }

            impl #generics ::core::convert::AsRef<Vec<#enum_ident #enum_generics>> for #ident #generics #where_clause  {
                fn as_ref(&self) -> &Vec<#enum_ident #enum_generics> {
                    &self.#field
                }
            }

            impl #generics ::core::convert::AsMut<Vec<#enum_ident #enum_generics>> for #ident #generics #where_clause {
                fn as_mut(&mut self) -> &mut Vec<#enum_ident #enum_generics> {
                    &mut self.#field
                }
            }

            impl #generics ::core::convert::AsRef<[#enum_ident #enum_generics]> for #ident #generics #where_clause {
                fn as_ref(&self) -> &[#enum_ident #enum_generics] {
                    &self.#field
                }
            }

            impl #generics ::core::convert::AsMut<[#enum_ident #enum_generics]> for #ident #generics #where_clause {
                fn as_mut(&mut self) -> &mut [#enum_ident #enum_generics] {
                    &mut self.#field
                }
            }
        }
    }

    /// Vec wrapper methods that depend on the `Default` trait:
    ///
    /// traits
    /// - 'From<Vec<enum>>'
    /// - `FromIterator<enum>`
    ///
    /// methods:
    /// - [`fn new() -> Self`](Vec::new())
    /// - [`fn with_capacity(capacity: usize) -> Self`](Vec::with_capacity())
    /// - `fn split_off(&mut self, at: usize) -> Self` the returned Self uses default to set other
    pub(crate) fn vec_default_traits_and_methods(&self, nodyn: &NodynEnum) -> TokenStream {
        if self.vec_field.is_none() || !self.traits.contains(&"Default".to_string()) {
            return TokenStream::new();
        }
        let field = self.vec_field.as_ref().unwrap();
        let wrapper_ident = &self.wrapper.ident;
        let enum_ident = &nodyn.ident;
        let vis = &self.wrapper.vis;
        let wrapper_generics = self.generic_params(nodyn);
        let where_clause = self.where_clause(nodyn);
        let enum_generics = nodyn.enum_generic_params();
        let new_type = &nodyn.generics.new_type();
        quote! {
            impl #wrapper_generics ::core::convert::From<::std::vec::Vec<#enum_ident #enum_generics>> for #wrapper_ident #wrapper_generics #where_clause {
                fn from(v: ::std::vec::Vec<#enum_ident #enum_generics>) -> #wrapper_ident #wrapper_generics {
                    Self {
                       #field: v,
                        ..::core::default::Default::default()
                    }
                }
            }

            impl #wrapper_generics ::core::iter::FromIterator<#enum_ident #enum_generics> for #wrapper_ident #wrapper_generics #where_clause {
                fn from_iter<#new_type: IntoIterator<Item = #enum_ident #enum_generics>>(iter: #new_type) -> #wrapper_ident #wrapper_generics {
                    Self {
                        #field: ::std::vec::Vec::from_iter(iter),
                        .. ::core::default::Default::default()
                    }
                }
            }

            impl #wrapper_generics #wrapper_ident #wrapper_generics #where_clause {
                #vis fn new() -> Self {
                    Self::default()
                }

                #vis fn with_capacity(capacity: usize) -> Self {
                    Self {
                        #field: ::std::vec::Vec::with_capacity(capacity),
                        ..::core::default::Default::default()
                    }
                }

                #vis fn split_off(&mut self, at: usize) -> Self {
                    Self {
                        #field: self.#field.split_off(at),
                        ..::core::default::Default::default()
                    }
                }
            }
        }
    }

    /// functions implemented when wrapper has `derive(PartialEq)`
    /// - `fn dedup(&mut self)`
    pub(crate) fn vec_partial_eq_methods(&self) -> TokenStream {
        if self.vec_field.is_none() || !self.traits.contains(&"PartialEq".to_string()) {
            return TokenStream::new();
        }
        let field = self.vec_field.as_ref().unwrap();
        let vis = &self.wrapper.vis;

        quote! {
            #vis fn dedup(&mut self) {
                self.#field.dedup()
            }
        }
    }

    /// - `From<&[enum]>`
    /// - `From<&mut [enum]>`
    pub(crate) fn vec_clone_default_traits(&self, nodyn: &NodynEnum) -> TokenStream {
        if self.vec_field.is_none()
            || (!self.traits.contains(&"Clone".to_string())
                && !self.traits.contains(&"Default".to_string()))
        {
            return TokenStream::new();
        }
        let field = self.vec_field.as_ref().unwrap();
        let wrapper_ident = &self.wrapper.ident;
        let enum_ident = &nodyn.ident;
        // let vis = &self.wrapper.vis;
        let wrapper_generics = self.generic_params(nodyn);
        let where_clause = self.where_clause(nodyn);
        let enum_generics = nodyn.enum_generic_params();
        quote! {
            impl #wrapper_generics ::core::convert::From<&[#enum_ident #enum_generics]> for #wrapper_ident #wrapper_generics #where_clause {
                fn from(s: &[#enum_ident #enum_generics]) -> #wrapper_ident #wrapper_generics {
                    Self {
                       #field: s.to_vec(),
                        ..::core::default::Default::default()
                    }
                }
            }

            impl #wrapper_generics ::core::convert::From<&mut [#enum_ident #enum_generics]> for #wrapper_ident #wrapper_generics #where_clause {
                fn from(s: &mut [#enum_ident #enum_generics]) -> #wrapper_ident #wrapper_generics {
                    Self {
                       #field: s.to_vec(),
                        ..::core::default::Default::default()
                    }
                }
            }
        }
    }
    /// - Extend trait
    ///
    /// functions:
    /// - `fn resize(&mut self, new_len: usize, value: Self)`
    /// - `fn extend_from_within<R>(&mut self, src: R)`
    ///
    pub(crate) fn vec_clone_traits_and_functions(&self, nodyn: &NodynEnum) -> TokenStream {
        if self.vec_field.is_none() || !self.traits.contains(&"Clone".to_string()) {
            return TokenStream::new();
        }
        let field = self.vec_field.as_ref().unwrap();
        let wrapper_ident = &self.wrapper.ident;
        let enum_ident = &nodyn.ident;
        let vis = &self.wrapper.vis;
        let wrapper_generics = self.generic_params(nodyn);
        let where_clause = self.where_clause(nodyn);
        let enum_generics = nodyn.enum_generic_params();
        // let enum_where = &nodyn.generics.where_clause;
        // let lt = &nodyn.generics.new_lifetime();
        // let (lt_generics, _) = {
        //     let index_g: GenericParam = parse_quote! {#lt};
        //     let index_w: WherePredicate = parse_quote! {W: Clone};
        //     self.merge_generics(nodyn, &index_g, &index_w)
        // };
        let new_type = &nodyn.generics.new_type();

        quote! {
            impl #wrapper_generics ::core::iter::Extend<#enum_ident #enum_generics> for #wrapper_ident #wrapper_generics #where_clause {
                fn extend<#new_type: ::core::iter::IntoIterator<Item = #enum_ident #enum_generics>>(&mut self, iter: #new_type) {
                    self.#field.extend(iter.into_iter())
                }
            }

            impl #wrapper_generics #wrapper_ident #wrapper_generics #where_clause {
                #vis fn resize<#new_type: ::core::convert::Into<#enum_ident #enum_generics>>(&mut self, new_len: usize, value: #new_type) {
                    self.resize(new_len, value)
                }

                #vis fn extend_from_within<R>(&mut self, src: R)
                where R: ::core::ops::RangeBounds<usize>, {
                    self.#field.extend_from_within(src)
                }

                #vis fn extend_from_slice(&mut self, other: &[#enum_ident #enum_generics]) {
                    self.#field.extend_from_slice(other)
                }

            }

        }
    }

    /// - `fn insert(&mut self, index: usize, element: T)` uses `Into`
    /// - `fn push(&mut self, value: T)` uses `Into`
    pub(crate) fn vec_changed_functions(&self, nodyn: &NodynEnum) -> TokenStream {
        if self.vec_field.is_none() {
            return TokenStream::new();
        }
        let field = self.vec_field.as_ref().unwrap();
        let vis = &self.wrapper.vis;
        let enum_ident = &nodyn.ident;
        // let generics = self.generic_params(nodyn);
        let enum_generics = nodyn.enum_generic_params();

        let new_type = &nodyn.generics.new_type();
        quote! {
            #vis fn insert<#new_type: ::core::convert::Into<#enum_ident #enum_generics>>(&mut self, index: usize, element: #new_type) {
                self.#field.insert(index, element.into())
            }

            #vis fn push<#new_type: ::core::convert::Into<#enum_ident #enum_generics>>(&mut self, value: #new_type) {
                self.#field.push(value.into());
            }



        }
    }

    /// implemented simple redirect:
    /// - `fn capacity(&self) -> usize`
    /// - `fn reserve(&mut self, additional: usize)`
    /// - `fn reserve_exact(&mut self, additional: usize)`
    /// - `fn try_reserve(&mut self, additional: usize) -> Result<(), TryReserveError>`
    /// - `fn try_reserve_exact(&mut self, additional: usize) -> Result<(), TryReserveError>`
    /// - `fn shrink_to_fit(&mut self)`
    /// - `fn shrink_to(&mut self, min_capacity: usize)`
    /// - `fn into_boxed_slice(self) -> Box<[T], A>`
    /// - `fn truncate(&mut self, len: usize)`
    /// - `fn as_slice(&self) -> &[T]`
    /// - `fn as_mut_slice(&mut self) -> &mut [T]`
    /// - `fn swap_remove(&mut self, index: usize) -> T`
    /// - `fn remove(&mut self, index: usize) -> T`
    /// - `fn retain<F>(&mut self, f: F)`
    /// - `fn retain_mut<F>(&mut self, f: F)`
    /// - `fn dedup_by_key<F, K>(&mut self, key: F)`
    /// - `fn dedup_by<F>(&mut self, same_bucket: F)`
    /// - `fn pop(&mut self) -> Option<T>`
    /// - `fn pop_if(&mut self, predicate: impl FnOnce(&mut T) -> bool) -> Option<T>`
    /// - `fn append(&mut self, other: &mut Vec<T, A>)`
    /// - `fn drain<R>(&mut self, range: R) -> Drain<'_, T, A>`
    /// - `fn clear(&mut self)`
    /// - `fn len(&self) -> usize`
    /// - `fn is_empty(&self) -> bool`
    /// - `fn extend_from_slice(&mut self, other: &[T])`
    /// - `fn splice<R, I>(&mut self, range: R, replace_with: I) -> Splice<'_, <I as IntoIterator>::IntoIter, A>`
    /// - `fn extract_if<F, R>( &mut self, range: R, filter: F,) -> ExtractIf<'_, T, F, A>`
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
    /// Not implemented:
    /// - All nightly-only experimental API.
    /// - `fn from_raw_parts(ptr: *mut T, length: usize, capacity: usize,) -> Vec<T>`
    /// - `fn as_ptr(&self) -> *const T`
    /// - `fn as_mut_ptr(&mut self) -> *mut T`
    /// - `fn set_len(&mut self, new_len: usize)`
    /// - `fn resize_with<F>(&mut self, new _len: usize, f: F)`
    /// - `fn leak<'a>(self) -> &'a mut [T]`
    /// - `fn spare_capacity_mut(&mut self) -> &mut [MaybeUninit<T>]`
    /// - `fn into_flattened(self) -> Vec<T, A>`
    /// - `fn first_chunk<const N: usize>(&self) -> Option<&[T; N]>`
    /// - `fn first_chunk_mut<const N: usize>(&mut self) -> Option<&mut [T; N]>`
    /// - `fn split_first_chunk<const N: usize>(&self) -> Option<(&[T; N], &[T])>`
    /// - `fn split_first_chunk_mut<const N: usize>(&mut self) -> Option<(&mut [T; N], &mut [T])>`
    /// - `fn split_last_chunk<const N: usize>(&self) -> Option<(&[T], &[T; N])>`
    /// - `fn split_last_chunk_mut<const N: usize>(&mut self) -> Option<(&mut [T], &mut [T; N])>`
    /// - `fn last_chunk<const N: usize>(&self) -> Option<&[T; N]>`
    /// - `fn last_chunk_mut<const N: usize>(&mut self) -> Option<&mut [T; N]>`
    /// - `fn get_unchecked<I>(&self, index: I,) -> &<I as SliceIndex<[T]>>::Output`
    /// - `fn get_unchecked_mut<I>(&mut self, index: I,) -> &mut <I as SliceIndex<[T]>>::Output`
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
        let nt = &nodyn.generics.new_types(2);
        let new_type = &nt[0];
        let new_type2 = &nt[1];

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

            #vis fn remove(&mut self, index: usize) -> #enum_ident #enum_generics {
                self.#field.remove(index)
            }

            #vis fn retain<#new_type>(&mut self, f: #new_type)
            where #new_type: ::core::ops::FnMut(&#enum_ident #enum_generics) -> bool {
                self.#field.retain(f);
            }

            #vis fn retain_mut<#new_type>(&mut self, f: #new_type)
            where #new_type: ::core::ops::FnMut(&mut #enum_ident #enum_generics) -> bool {
                self.#field.retain_mut(f);
            }

            #vis fn dedup_by_key<#new_type, #new_type2>(&mut self, key: #new_type)
            where
                #new_type: ::core::ops::FnMut(&mut #enum_ident #enum_generics) -> #new_type2,
                #new_type2: ::core::cmp::PartialEq,
            {
                self.#field.dedup_by_key(key);
            }

            #vis fn dedup_by<#new_type>(&mut self, same_bucket: #new_type)
            where #new_type: ::core::ops::FnMut(&mut #enum_ident #enum_generics, &mut #enum_ident #enum_generics) -> bool,
            {
                self.#field.dedup_by(same_bucket)
            }

            #vis fn pop(&mut self) -> ::core::option::Option<#enum_ident #enum_generics> {
                self.#field.pop()
            }

            #vis fn pop_if(&mut self, predicate: impl ::core::ops::FnOnce(&mut #enum_ident #enum_generics) -> bool) -> ::core::option::Option<#enum_ident #enum_generics> {
                self.#field.pop_if(predicate)
            }

            #vis fn append(&mut self, other: &mut Self) {
                self.#field.append(&mut other.#field)
            }

            // #vis fn drain<#new_type>(&mut self, range: #new_type) -> ::std::vec::Drain<'_, <#enum_ident #enum_generics>>
            // where #new_type: ::core::ops::RangeBounds<usize>,
            // {
            //     self.#field.drain(range)
            // }

            #vis fn clear(&mut self) {
                self.#field.clear();
            }

            #vis const fn len(&self) -> usize {
                self.#field.len()
            }

            #vis const fn is_empty(&self) -> bool {
                self.#field.is_empty()
            }

            #vis fn splice<#new_type, #new_type2>(&mut self, range: #new_type, replace_with: #new_type2)
            -> ::std::vec::Splice<'_, <#new_type2 as ::core::iter::IntoIterator>::IntoIter>
            where #new_type: ::core::ops::RangeBounds<usize>,
                  #new_type2: ::core::iter::IntoIterator<Item = #enum_ident #enum_generics>, {
                self.#field.splice(range, replace_with)
            }

            #vis fn extract_if<#new_type, #new_type2>(&mut self, range: #new_type2, filter: #new_type,) -> ::std::vec::ExtractIf<'_, #enum_ident #enum_generics, #new_type>
            where #new_type: ::core::ops::FnMut(&mut #enum_ident #enum_generics) -> bool,
                  #new_type2: ::core::ops::RangeBounds<usize>,
            {
                self.#field.extract_if(range, filter)
            }

            #vis fn first(&self) -> ::core::option::Option<&#enum_ident #enum_generics> {
                self.#field.first()
            }

            #vis fn first_mut(&mut self) -> ::core::option::Option<&mut #enum_ident #enum_generics> {
                self.#field.first_mut()
            }

            #vis fn last(&self) -> ::core::option::Option<&#enum_ident #enum_generics> {
                self.#field.last()
            }

            #vis fn last_mut(&mut self) -> ::core::option::Option<&mut #enum_ident #enum_generics> {
                self.#field.last_mut()
            }

            #vis fn split_first(&self) -> ::core::option::Option<(&#enum_ident #enum_generics, &[#enum_ident #enum_generics])> {
                self.#field.split_first()
            }

            #vis fn split_first_mut(&mut self) -> ::core::option::Option<(&mut #enum_ident #enum_generics, &mut [#enum_ident #enum_generics])> {
                self.#field.split_first_mut()
            }

            #vis fn split_last(&self) -> ::core::option::Option<(&#enum_ident #enum_generics, &[#enum_ident #enum_generics])> {
                self.#field.split_last()
            }

            #vis fn split_last_mut(&mut self) -> ::core::option::Option<(&mut #enum_ident #enum_generics, &mut [#enum_ident #enum_generics])> {
                self.#field.split_last_mut()
            }

            #vis fn get<#new_type>(&self, index: #new_type) -> ::core::option::Option<&<#new_type as ::core::slice::SliceIndex<[#enum_ident #enum_generics]>>::Output>
            where
                #new_type: ::core::slice::SliceIndex<[#enum_ident #enum_generics]>,
            {
                self.#field.get(index)
            }

            #vis fn get_mut<#new_type>(&mut self, index: #new_type) -> ::core::option::Option<&mut <#new_type as ::core::slice::SliceIndex<[#enum_ident #enum_generics]>>::Output>
            where
                #new_type: ::core::slice::SliceIndex<[#enum_ident #enum_generics]>,
            {
                self.#field.get_mut(index)
            }

            #vis fn swap(&mut self, a: usize, b: usize) {
                self.#field.swap(a, b);
            }

            #vis fn reverse(&mut self) {
                self.#field.reverse();
            }

            #vis fn iter(&self) -> ::core::slice::Iter<'_, #enum_ident #enum_generics> {
                self.#field.iter()
            }

            #vis fn iter_mut(&mut self) -> ::core::slice::IterMut<'_, #enum_ident #enum_generics> {
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
