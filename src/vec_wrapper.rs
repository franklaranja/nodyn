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

use crate::{GenericsExt, NodynEnum, camel_to_snake};

/// Represents a wrapper struct for a collection of enum variants in the `nodyn` crate.
/// Currently only `Vec` wrappers are supported.
///
/// This struct generates a wrapper around a `Vec<Enum>` with methods that delegate to the underlying
/// `std::vec::Vec`, plus additional methods for variant-specific access (via `NodynEnum`).
/// It supports both standard (auto-generated) and custom structs with a `vec_wrapper` attribute.
///
/// # Overview of Implemented Methods
///
/// The following table lists all methods implemented for the wrapper struct, their required traits,
/// and differences from their `std::vec::Vec` counterparts:
///
/// | Method | Required Traits | Differences from `std::vec::Vec` |
/// |--------|-----------------|-------------------------------|
/// | [`new`](#method.new) | `Default` | Initializes other fields with `Default::default()`. |
/// | [`with_capacity`](#method.with_capacity) | `Default` | Initializes other fields with `Default::default()`. |
/// | [`split_off`](#method.split_off) | `Default` | Initializes other fields with `Default::default()`. |
/// | [`dedup`](#method.dedup) | `PartialEq` | None; direct delegation to `Vec::dedup`. |
/// | [`resize`](#method.resize) | `Clone` | Accepts `Into<Enum>` for the value parameter. |
/// | [`extend_from_within`](#method.extend_from_within) | `Clone` | None; direct delegation to `Vec::extend_from_within`. |
/// | [`extend_from_slice`](#method.extend_from_slice) | `Clone` | None; direct delegation to `Vec::extend_from_slice`. |
/// | [`insert`](#method.insert) | None | Accepts `Into<Enum>` for the element parameter. |
/// | [`push`](#method.push) | None | Accepts `Into<Enum>` for the value parameter. |
/// | [`capacity`](#method.capacity) | None | None; direct delegation to `Vec::capacity`. |
/// | [`reserve`](#method.reserve) | None | None; direct delegation to `Vec::reserve`. |
/// | [`reserve_exact`](#method.reserve_exact) | None | None; direct delegation to `Vec::reserve_exact`. |
/// | [`try_reserve`](#method.try_reserve) | None | None; direct delegation to `Vec::try_reserve`. |
/// | [`try_reserve_exact`](#method.try_reserve_exact) | None | None; direct delegation to `Vec::try_reserve_exact`. |
/// | [`shrink_to_fit`](#method.shrink_to_fit) | None | None; direct delegation to `Vec::shrink_to_fit`. |
/// | [`shrink_to`](#method.shrink_to) | None | None; direct delegation to `Vec::shrink_to`. |
/// | [`into_boxed_slice`](#method.into_boxed_slice) | None | None; direct delegation to `Vec::into_boxed_slice`. |
/// | [`truncate`](#method.truncate) | None | None; direct delegation to `Vec::truncate`. |
/// | [`as_slice`](#method.as_slice) | None | None; direct delegation to `Vec::as_slice`. |
/// | [`as_mut_slice`](#method.as_mut_slice) | None | None; direct delegation to `Vec::as_mut_slice`. |
/// | [`swap_remove`](#method.swap_remove) | None | None; direct delegation to `Vec::swap_remove`. |
/// | [`remove`](#method.remove) | None | None; direct delegation to `Vec::remove`. |
/// | [`retain`](#method.retain) | None | None; direct delegation to `Vec::retain`. |
/// | [`retain_mut`](#method.retain_mut) | None | None; direct delegation to `Vec::retain_mut`. |
/// | [`dedup_by_key`](#method.dedup_by_key) | None | None; direct delegation to `Vec::dedup_by_key`. |
/// | [`dedup_by`](#method.dedup_by) | None | None; direct delegation to `Vec::dedup_by`. |
/// | [`pop`](#method.pop) | None | None; direct delegation to `Vec::pop`. |
/// | [`pop_if`](#method.pop_if) | None | None; direct delegation to `Vec::pop_if`. |
/// | [`append`](#method.append) | None | None; direct delegation to `Vec::append`. |
/// | [`splice`](#method.splice) | None | None; direct delegation to `Vec::splice`. |
/// | [`extract_if`](#method.extract_if) | None | None; direct delegation to `Vec::extract_if`. |
/// | [`first`](#method.first) | None | None; direct delegation to `Vec::first`. |
/// | [`first_mut`](#method.first_mut) | None | None; direct delegation to `Vec::first_mut`. |
/// | [`last`](#method.last) | None | None; direct delegation to `Vec::last`. |
/// | [`last_mut`](#method.last_mut) | None | None; direct delegation to `Vec::last_mut`. |
/// | [`split_first`](#method.split_first) | None | None; direct delegation to `Vec::split_first`. |
/// | [`split_first_mut`](#method.split_first_mut) | None | None; direct delegation to `Vec::split_first_mut`. |
/// | [`split_last`](#method.split_last) | None | None; direct delegation to `Vec::split_last`. |
/// | [`split_last_mut`](#method.split_last_mut) | None | None; direct delegation to `Vec::split_last_mut`. |
/// | [`get`](#method.get) | None | None; direct delegation to `Vec::get`. |
/// | [`get_mut`](#method.get_mut) | None | None; direct delegation to `Vec::get_mut`. |
/// | [`swap`](#method.swap) | None | None; direct delegation to `Vec::swap`. |
/// | [`reverse`](#method.reverse) | None | None; direct delegation to `Vec::reverse`. |
/// | [`iter`](#method.iter) | None | None; direct delegation to `Vec::iter`. |
/// | [`iter_mut`](#method.iter_mut) |   None | None; direct delegation to `Vec::iter_mut`. |
///
/// # Implemented Traits
///
/// | Trait | Required Traits | Differences from `std` |
/// |-------|-----------------|-----------------------|
/// | [`From<Self>`](#impl-From<Self>) | None | Converts to `Vec<Enum>`. |
/// | [`Index`](#impl-Index) | None | Delegates to `Vec::index`. |
/// | [`IndexMut`](#impl-IndexMut) | None | Delegates to `Vec::index_mut`. |
/// | [`IntoIterator`](#impl-IntoIterator) | None | Implements for `&Self`, `&mut Self`, and `Self`. |
/// | [`AsRef<Self>`](#impl-AsRef<Self>) | None | Returns `&Self`. |
/// | [`AsMut<Self>`](#impl-AsMut<Self>) | None | Returns `&mut Self`. |
/// | [`AsRef<Vec<Enum>>`](#impl-AsRef<Vec>) | None | Delegates to `Vec`. |
/// | [`AsMut<Vec<Enum>>`](#impl-AsMut<Vec>) | None | Delegates to `Vec`. |
/// | [`AsRef<[Enum]>`](#impl-AsRef<Slice>) | None | Delegates to `Vec`. |
/// | [`AsMut<[Enum]>`](#impl-AsMut<Slice>) | None | Delegates to `Vec`. |
/// | [`From<Vec<Enum>>`](#impl-From<Vec>) | `Default` | Initializes other fields with `Default::default()`. |
/// | [`FromIterator<Enum>`](#impl-FromIterator) | `Default` | Initializes other fields with `Default::default()`. |
/// | [`From<&[Enum]>`](#impl-From<Slice>) | `Clone`, `Default` | Initializes other fields with `Default::default()`. |
/// | [`From<&mut [Enum]>`](#impl-From<MutSlice>) | `Clone`, `Default` | Initializes other fields with `Default::default()`. |
/// | [`Extend<Enum>`](#impl-Extend) | `Clone` | Delegates to `Vec::extend`. |
///
#[derive(Debug, Clone)]
pub(crate) struct VecWrapper {
    /// The struct definition, including fields and attributes.
    pub(crate) definition: ItemStruct,
    /// The identifier of the `Vec` field (e.g., `inner` or `inner_vec`).
    pub(crate) vec_field: Option<Ident>,
    /// Whether the struct is custom (defined with `#[vec_wrapper]`).
    pub(crate) is_custom: bool,
    /// Derived traits (e.g., `Default`, `Clone`, `PartialEq`).
    pub(crate) derived_traits: Vec<String>,
}

impl Parse for VecWrapper {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let mut wrapper = input.parse::<ItemStruct>()?;
        if !matches!(wrapper.fields, Fields::Named(_)) {
            return Err(syn::Error::new(
                wrapper.span(),
                "Only structs with named fields are supported",
            ));
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
        let traits = Self::extract_traits(&attrs);
        wrapper.attrs = attrs;

        Ok(Self {
            // ident: wrapper.ident.clone(),
            definition: wrapper,
            vec_field,
            is_custom: true,
            derived_traits: traits,
        })
    }
}

impl VecWrapper {
    pub(crate) fn standard_vec_wrapper(
        ident: &Ident,
        visibility: &Visibility,
        enum_ident: &Ident,
        generics: &Generics,
        derive_attr: &[Attribute],
    ) -> Self {
        // let pound = syn::token::Pound::default();
        let wrapper: ItemStruct = parse_quote! {
            #[derive(Default)]
            #(#derive_attr)*
            #visibility struct #ident #generics {
                inner: std::vec::Vec< #enum_ident #generics >,
            }
        };
        let mut traits = Self::extract_traits(derive_attr);
        traits.push("Default".to_string());
        Self {
            definition: wrapper,
            vec_field: Some(Ident::new("inner", Span::call_site())),
            is_custom: false,
            derived_traits: traits,
        }
    }

    /// Generates the complete `TokenStream` for the wrapper struct and its implementations.
    pub(crate) fn to_token_stream(&self, nodyn: &NodynEnum) -> TokenStream {
        if self.vec_field.is_none() {
            return TokenStream::new();
        }
        let wrapper_struct = self.to_struct(nodyn);
        let general_impl = self.to_general_impl(nodyn);
        let trait_impls = &self.to_trait_impls(nodyn);
        let clone_impls = &self.to_impl_with_clone(nodyn);
        let clone_default_impls = &self.to_impl_with_clone_default(nodyn);
        let default_impls = &self.to_impl_with_default(nodyn);
        quote! {
            #wrapper_struct
            #general_impl
            #trait_impls
            #default_impls
            #clone_impls
            #clone_default_impls
        }
    }

    /// Generates the `TokenStream` for the wrapper struct definition.
    fn to_struct(&self, nodyn: &NodynEnum) -> TokenStream {
        if self.vec_field.is_none() {
            return TokenStream::new();
        }
        if self.is_custom {
            let enum_ident = &nodyn.ident;
            let enum_generics = nodyn.to_generics();
            let visibility = &self.definition.vis;
            let ident = &self.definition.ident;
            let generics = nodyn.to_merged_generics(&self.definition.generics);
            let where_clause = nodyn.to_merged_where(&self.definition.generics);

            let fields = if let Fields::Named(fields) = &self.definition.fields {
                fields.named.iter().collect::<Vec<_>>()
            } else {
                Vec::new()
            };
            let default_field = Ident::new("inner", Span::call_site());
            let field = self.vec_field.as_ref().unwrap_or(&default_field);
            let attrs = &self.definition.attrs;
            quote! {
                #(#attrs)*
                #visibility struct #ident #generics
                #where_clause {
                    #(#fields ,)*
                    #field: std::vec::Vec< #enum_ident #enum_generics >,
                }
            }
        } else {
            self.definition.to_token_stream()
        }
    }

    /// Generates the general implementation block, including standard methods and variant-specific methods.
    fn to_general_impl(&self, nodyn: &NodynEnum) -> TokenStream {
        if self.vec_field.is_none() {
            return TokenStream::new();
        }
        let ident = &self.definition.ident;
        let generics = self.to_generics(nodyn);
        let where_clause = self.to_where(nodyn);
        let standard_methods = &self.to_standard_methods(nodyn);
        let modified_methods = &self.to_modified_methods(nodyn);
        let partial_eq_methods = &self.to_partial_eq_methods();
        let field = self.vec_field.as_ref().unwrap();
        let variant_methods = nodyn.to_vec_methods(field);

        quote! {
            impl #generics #ident #generics #where_clause {
                #standard_methods
                #modified_methods
                #partial_eq_methods
                #variant_methods
            }
        }
    }

    /// Generates standard `Vec` methods that directly delegate to the underlying `Vec`.
    ///
    /// These methods match their `std::vec::Vec` counterparts exactly:
    /// - [`capacity`][std::vec::Vec::capacity]
    /// - [`reserve`][std::vec::Vec::reserve]
    /// - [`reserve_exact`][std::vec::Vec::reserve_exact]
    /// - [`try_reserve`][std::vec::Vec::try_reserve]
    /// - [`try_reserve_exact`][std::vec::Vec::try_reserve_exact]
    /// - [`shrink_to_fit`][std::vec::Vec::shrink_to_fit]
    /// - [`shrink_to`][std::vec::Vec::shrink_to]
    /// - [`into_boxed_slice`][std::vec::Vec::into_boxed_slice]
    /// - [`truncate`][std::vec::Vec::truncate]
    /// - [`as_slice`][std::vec::Vec::as_slice]
    /// - [`as_mut_slice`][std::vec::Vec::as_mut_slice]
    /// - [`swap_remove`][std::vec::Vec::swap_remove]
    /// - [`remove`][std::vec::Vec::remove]
    /// - [`retain`][std::vec::Vec::retain]
    /// - [`retain_mut`][std::vec::Vec::retain_mut]
    /// - [`dedup_by_key`][std::vec::Vec::dedup_by_key]
    /// - [`dedup_by`][std::vec::Vec::dedup_by]
    /// - [`pop`][std::vec::Vec::pop]
    /// - [`pop_if`][std::vec::Vec::pop_if]
    /// - [`append`][std::vec::Vec::append]
    /// - [`splice`][std::vec::Vec::splice]
    /// - [`extract_if`][std::vec::Vec::extract_if]
    /// - [`first`][std::vec::Vec::first]
    /// - [`first_mut`][std::vec::Vec::first_mut]
    /// - [`last`][std::vec::Vec::last]
    /// - [`last_mut`][std::vec::Vec::last_mut]
    /// - [`split_first`][std::vec::Vec::split_first]
    /// - [`split_first_mut`][std::vec::Vec::split_first_mut]
    /// - [`split_last`][std::vec::Vec::split_last]
    /// - [`split_last_mut`][std::vec::Vec::split_last_mut]
    /// - [`get`][std::vec::Vec::get]
    /// - [`get_mut`][std::vec::Vec::get_mut]
    /// - [`swap`][std::vec::Vec::swap]
    /// - [`reverse`][std::vec::Vec::reverse]
    /// - [`iter`][std::vec::Vec::iter]
    /// - [`iter_mut`][std::vec::Vec::iter_mut]
    #[allow(clippy::too_many_lines)]
    fn to_standard_methods(&self, nodyn: &NodynEnum) -> TokenStream {
        if self.vec_field.is_none() {
            return TokenStream::new();
        }
        let field = self.vec_field.as_ref().unwrap();
        let visibility = &self.definition.vis;
        let enum_ident = &nodyn.ident;
        let enum_generics = nodyn.to_generics();
        let nt = &nodyn.generics.new_types(2);
        let new_type = &nt[0];
        let new_type2 = &nt[1];

        quote! {
            /// Returns the total number of elements the vector can hold without reallocating.
            /// See [`std::vec::Vec::capacity`].
            #visibility fn capacity(&self) -> usize {
                self.#field.capacity()
            }

            /// Reserves capacity for at least `additional` more elements.
            /// See [`std::vec::Vec::reserve`].
            #visibility fn reserve(&mut self, additional: usize) {
                self.#field.reserve(additional);
            }

            /// Reserves the minimum capacity for exactly `additional` more elements.
            /// See [`std::vec::Vec::reserve_exact`].
            #visibility fn reserve_exact(&mut self, additional: usize) {
                self.#field.reserve_exact(additional);
            }

            /// Tries to reserve capacity for at least `additional` more elements.
            /// See [`std::vec::Vec::try_reserve`].
            #visibility fn try_reserve(&mut self, additional: usize) -> ::core::result::Result<(), ::std::collections::TryReserveError> {
                self.#field.try_reserve(additional)
            }

            /// Tries to reserve the minimum capacity for exactly `additional` more elements.
            /// See [`std::vec::Vec::try_reserve_exact`].
            #visibility fn try_reserve_exact(&mut self, additional: usize) -> ::core::result::Result<(), ::std::collections::TryReserveError> {
                self.#field.try_reserve_exact(additional)
            }

            /// Shrinks the capacity of the vector as much as possible.
            /// See [`std::vec::Vec::shrink_to_fit`].
            #visibility fn shrink_to_fit(&mut self) {
                self.#field.shrink_to_fit();
            }

            /// Shrinks the capacity of the vector with a lower bound.
            /// See [`std::vec::Vec::shrink_to`].
            #visibility fn shrink_to(&mut self, min_capacity: usize) {
                self.#field.shrink_to(min_capacity);
            }

            /// Converts the vector into a `Box<[Enum]>`.
            /// See [`std::vec::Vec::into_boxed_slice`].
            #visibility fn into_boxed_slice(self) -> ::std::boxed::Box<[#enum_ident #enum_generics]> {
                self.#field.into_boxed_slice()
            }

            /// Shortens the vector to the specified length.
            /// See [`std::vec::Vec::truncate`].
            #visibility fn truncate(&mut self, len: usize) {
                self.#field.truncate(len);
            }

            /// Returns a slice containing all elements.
            /// See [`std::vec::Vec::as_slice`].
            #visibility const fn as_slice(&self) -> &[#enum_ident #enum_generics] {
                self.#field.as_slice()
            }

            /// Returns a mutable slice containing all elements.
            /// See [`std::vec::Vec::as_mut_slice`].
            #visibility const fn as_mut_slice(&mut self) -> &mut [#enum_ident #enum_generics] {
                self.#field.as_mut_slice()
            }

            /// Removes and returns the element at `index`, swapping with the last element.
            /// See [`std::vec::Vec::swap_remove`].
            #visibility fn swap_remove(&mut self, index: usize) -> #enum_ident #enum_generics {
                self.#field.swap_remove(index)
            }

            /// Removes and returns the element at `index`.
            /// See [`std::vec::Vec::remove`].
            #visibility fn remove(&mut self, index: usize) -> #enum_ident #enum_generics {
                self.#field.remove(index)
            }

            /// Retains only the elements specified by the predicate.
            /// See [`std::vec::Vec::retain`].
            #visibility fn retain<#new_type>(&mut self, f: #new_type)
            where #new_type: ::core::ops::FnMut(&#enum_ident #enum_generics) -> bool {
                self.#field.retain(f);
            }

            /// Retains only the elements specified by the mutable predicate.
            /// See [`std::vec::Vec::retain_mut`].
            #visibility fn retain_mut<#new_type>(&mut self, f: #new_type)
            where #new_type: ::core::ops::FnMut(&mut #enum_ident #enum_generics) -> bool {
                self.#field.retain_mut(f);
            }

            /// Removes consecutive duplicate elements based on a key function.
            /// See [`std::vec::Vec::dedup_by_key`].
            #visibility fn dedup_by_key<#new_type, #new_type2>(&mut self, key: #new_type)
            where
                #new_type: ::core::ops::FnMut(&mut #enum_ident #enum_generics) -> #new_type2,
                #new_type2: ::core::cmp::PartialEq,
            {
                self.#field.dedup_by_key(key);
            }

            /// Removes consecutive duplicate elements based on a predicate.
            /// See [`std::vec::Vec::dedup_by`].
            #visibility fn dedup_by<#new_type>(&mut self, same_bucket: #new_type)
            where #new_type: ::core::ops::FnMut(&mut #enum_ident #enum_generics, &mut #enum_ident #enum_generics) -> bool {
                self.#field.dedup_by(same_bucket)
            }

            /// Removes and returns the last element, if any.
            /// See [`std::vec::Vec::pop`].
            #visibility fn pop(&mut self) -> ::core::option::Option<#enum_ident #enum_generics> {
                self.#field.pop()
            }

            /// Removes and returns the last element if it satisfies the predicate.
            /// See [`std::vec::Vec::pop_if`].
            #visibility fn pop_if(&mut self, predicate: impl ::core::ops::FnOnce(&mut #enum_ident #enum_generics) -> bool) -> ::core::option::Option<#enum_ident #enum_generics> {
                self.#field.pop_if(predicate)
            }

            /// Appends all elements from `other` to `self`, emptying `other`.
            /// See [`std::vec::Vec::append`].
            #visibility fn append(&mut self, other: &mut Self) {
                self.#field.append(&mut other.#field)
            }

            /// Replaces elements in the specified range with new ones.
            /// See [`std::vec::Vec::splice`].
            #visibility fn splice<#new_type, #new_type2>(&mut self, range: #new_type, replace_with: #new_type2)
            -> ::std::vec::Splice<'_, <#new_type2 as ::core::iter::IntoIterator>::IntoIter>
            where
                #new_type: ::core::ops::RangeBounds<usize>,
                #new_type2: ::core::iter::IntoIterator<Item = #enum_ident #enum_generics> {
                self.#field.splice(range, replace_with)
            }

            /// Removes elements that match the predicate in the specified range.
            /// See [`std::vec::Vec::extract_if`].
            #visibility fn extract_if<#new_type, #new_type2>(&mut self, range: #new_type2, filter: #new_type)
            -> ::std::vec::ExtractIf<'_, #enum_ident #enum_generics, #new_type>
            where
                #new_type: ::core::ops::FnMut(&mut #enum_ident #enum_generics) -> bool,
                #new_type2: ::core::ops::RangeBounds<usize> {
                self.#field.extract_if(range, filter)
            }

            /// Returns the first element, if any.
            /// See [`std::vec::Vec::first`].
            #visibility fn first(&self) -> ::core::option::Option<&#enum_ident #enum_generics> {
                self.#field.first()
            }

            /// Returns a mutable reference to the first element, if any.
            /// See [`std::vec::Vec::first_mut`].
            #visibility fn first_mut(&mut self) -> ::core::option::Option<&mut #enum_ident #enum_generics> {
                self.#field.first_mut()
            }

            /// Returns the last element, if any.
            /// See [`std::vec::Vec::last`].
            #visibility fn last(&self) -> ::core::option::Option<&#enum_ident #enum_generics> {
                self.#field.last()
            }

            /// Returns a mutable reference to the last element, if any.
            /// See [`std::vec::Vec::last_mut`].
            #visibility fn last_mut(&mut self) -> ::core::option::Option<&mut #enum_ident #enum_generics> {
                self.#field.last_mut()
            }

            /// Returns the first element and the rest of the slice, if any.
            /// See [`std::vec::Vec::split_first`].
            #visibility fn split_first(&self) -> ::core::option::Option<(&#enum_ident #enum_generics, &[#enum_ident #enum_generics])> {
                self.#field.split_first()
            }

            /// Returns a mutable first element and the rest of the slice, if any.
            /// See [`std::vec::Vec::split_first_mut`].
            #visibility fn split_first_mut(&mut self) -> ::core::option::Option<(&mut #enum_ident #enum_generics, &mut [#enum_ident #enum_generics])> {
                self.#field.split_first_mut()
            }

            /// Returns the last element and the rest of the slice, if any.
            /// See [`std::vec::Vec::split_last`].
            #visibility fn split_last(&self) -> ::core::option::Option<(&#enum_ident #enum_generics, &[#enum_ident #enum_generics])> {
                self.#field.split_last()
            }

            /// Returns a mutable last element and the rest of the slice, if any.
            /// See [`std::vec::Vec::split_last_mut`].
            #visibility fn split_last_mut(&mut self) -> ::core::option::Option<(&mut #enum_ident #enum_generics, &mut [#enum_ident #enum_generics])> {
                self.#field.split_last_mut()
            }

            /// Returns a reference to an element or subslice by index.
            /// See [`std::vec::Vec::get`].
            #visibility fn get<#new_type>(&self, index: #new_type) -> ::core::option::Option<&<#new_type as ::core::slice::SliceIndex<[#enum_ident #enum_generics]>>::Output>
            where
                #new_type: ::core::slice::SliceIndex<[#enum_ident #enum_generics]> {
                self.#field.get(index)
            }

            /// Returns a mutable reference to an element or subslice by index.
            /// See [`std::vec::Vec::get_mut`].
            #visibility fn get_mut<#new_type>(&mut self, index: #new_type) -> ::core::option::Option<&mut <#new_type as ::core::slice::SliceIndex<[#enum_ident #enum_generics]>>::Output>
            where
                #new_type: ::core::slice::SliceIndex<[#enum_ident #enum_generics]> {
                self.#field.get_mut(index)
            }

            /// Swaps two elements in the vector.
            /// See [`std::vec::Vec::swap`].
            #visibility fn swap(&mut self, a: usize, b: usize) {
                self.#field.swap(a, b);
            }

            /// Reverses the order of elements in the vector.
            /// See [`std::vec::Vec::reverse`].
            #visibility fn reverse(&mut self) {
                self.#field.reverse();
            }

            /// Returns an iterator over the vector's elements.
            /// See [`std::vec::Vec::iter`].
            #visibility fn iter(&self) -> ::core::slice::Iter<'_, #enum_ident #enum_generics> {
                self.#field.iter()
            }

            /// Returns a mutable iterator over the vector's elements.
            /// See [`std::vec::Vec::iter_mut`].
            #visibility fn iter_mut(&mut self) -> ::core::slice::IterMut<'_, #enum_ident #enum_generics> {
                self.#field.iter_mut()
            }
        }
    }

    /// Generates methods that differ from `std::vec::Vec`.
    ///
    /// - [`insert`][std::vec::Vec::insert]: Accepts `Into<Enum>` for the element.
    /// - [`push`][std::vec::Vec::push]: Accepts `Into<Enum>` for the value.
    fn to_modified_methods(&self, nodyn: &NodynEnum) -> TokenStream {
        if self.vec_field.is_none() {
            return TokenStream::new();
        }
        let field = self.vec_field.as_ref().unwrap();
        let visibility = &self.definition.vis;
        let enum_ident = &nodyn.ident;
        let enum_generics = nodyn.to_generics();
        let new_type = nodyn.generics.new_type();

        quote! {
            /// Inserts an element at the specified index, shifting elements as needed.
            /// Accepts `Into<Enum>` for the element.
            /// See [`std::vec::Vec::insert`].
            #visibility fn insert<#new_type: ::core::convert::Into<#enum_ident #enum_generics>>(&mut self, index: usize, element: #new_type) {
                self.#field.insert(index, element.into());
            }

            /// Appends an element to the end of the vector.
            /// Accepts `Into<Enum>` for the value.
            /// See [`std::vec::Vec::push`].
            #visibility fn push<#new_type: ::core::convert::Into<#enum_ident #enum_generics>>(&mut self, value: #new_type) {
                self.#field.push(value.into());
            }
        }
    }

    /// Generates methods that require the `PartialEq` trait.
    ///
    /// - [`dedup`][std::vec::Vec::dedup]: Removes consecutive duplicate elements.
    fn to_partial_eq_methods(&self) -> TokenStream {
        if self.vec_field.is_none() || !self.derived_traits.contains(&"PartialEq".to_string()) {
            return TokenStream::new();
        }
        let field = self.vec_field.as_ref().unwrap();
        let visibility = &self.definition.vis;

        quote! {
            /// Removes consecutive duplicate elements.
            /// Requires `PartialEq` on the wrapper struct.
            /// See [`std::vec::Vec::dedup`].
            #visibility fn dedup(&mut self) {
                self.#field.dedup();
            }
        }
    }

    /// Generates trait implementations not depended on other traits.
    ///
    /// - [`From<Self>`][std::vec::Vec]: Converts to `Vec<Enum>`.
    /// - [`Index`][std::ops::Index]
    /// - [`IndexMut`][std::ops::IndexMut]
    /// - [`IntoIterator`][std::iter::IntoIterator] (for `&Self`, `&mut Self`, `Self`)
    /// - [`AsRef<Self>`][std::convert::AsRef]
    /// - [`AsMut<Self>`][std::convert::AsMut]
    /// - [`AsRef<Vec<Enum>>`][std::convert::AsRef]
    /// - [`AsMut<Vec<Enum>>`][std::convert::AsMut]
    /// - [`AsRef<[Enum]>`][std::convert::AsRef]
    /// - [`AsMut<[Enum]>`][std::convert::AsMut]
    fn to_trait_impls(&self, nodyn: &NodynEnum) -> TokenStream {
        if self.vec_field.is_none() {
            return TokenStream::new();
        }
        let ident = &self.definition.ident;
        let enum_ident = &nodyn.ident;
        let field = self.vec_field.as_ref().unwrap();
        let generics = self.to_generics(nodyn);
        let where_clause = self.to_where(nodyn);
        let enum_generics = nodyn.to_generics();
        let new_type = nodyn.generics.new_type();
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
            impl #generics ::core::convert::From<#ident #generics> for ::std::vec::Vec<#enum_ident #enum_generics> #where_clause {
                fn from(v: #ident #generics) -> ::std::vec::Vec<#enum_ident #enum_generics> {
                    v.#field
                }
            }

            impl #index_generics ::core::ops::Index<#new_type> for #ident #generics #index_where {
                type Output = #new_type::Output;
                fn index(&self, index: #new_type) -> &Self::Output {
                    &self.#field[index]
                }
            }

            impl #index_generics ::core::ops::IndexMut<#new_type> for #ident #generics #index_where {
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

            impl #generics ::core::convert::AsRef<#ident #generics> for #ident #generics #where_clause {
                fn as_ref(&self) -> &Self {
                    self
                }
            }

            impl #generics ::core::convert::AsMut<#ident #generics> for #ident #generics #where_clause {
                fn as_mut(&mut self) -> &mut Self {
                    self
                }
            }

            impl #generics ::core::convert::AsRef<::std::vec::Vec<#enum_ident #enum_generics>> for #ident #generics #where_clause {
                fn as_ref(&self) -> &::std::vec::Vec<#enum_ident #enum_generics> {
                    &self.#field
                }
            }

            impl #generics ::core::convert::AsMut<::std::vec::Vec<#enum_ident #enum_generics>> for #ident #generics #where_clause {
                fn as_mut(&mut self) -> &mut ::std::vec::Vec<#enum_ident #enum_generics> {
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

    /// Generates methods and traits that require `Default`.
    ///
    /// - [`From<Vec<Enum>>`][std::vec::Vec]
    /// - [`FromIterator<Enum>`][std::iter::FromIterator]
    /// - [`new`][std::vec::Vec::new]
    /// - [`with_capacity`][std::vec::Vec::with_capacity]
    /// - [`split_off`][std::vec::Vec::split_off]
    fn to_impl_with_default(&self, nodyn: &NodynEnum) -> TokenStream {
        if self.vec_field.is_none() || !self.derived_traits.contains(&"Default".to_string()) {
            return TokenStream::new();
        }
        let field = self.vec_field.as_ref().unwrap();
        let ident = &self.definition.ident;
        let enum_ident = &nodyn.ident;
        let visibility = &self.definition.vis;
        let generics = self.to_generics(nodyn);
        let where_clause = self.to_where(nodyn);
        let enum_generics = nodyn.to_generics();
        let new_type = &nodyn.generics.new_type();

        quote! {
            impl #generics ::core::convert::From<::std::vec::Vec<#enum_ident #enum_generics>> for #ident #generics #where_clause {
                fn from(v: ::std::vec::Vec<#enum_ident #enum_generics>) -> Self {
                    Self {
                        #field: v,
                        ..::core::default::Default::default()
                    }
                }
            }

            impl #generics ::core::iter::FromIterator<#enum_ident #enum_generics> for #ident #generics #where_clause {
                fn from_iter<#new_type: ::core::iter::IntoIterator<Item = #enum_ident #enum_generics>>(iter: #new_type) -> Self {
                    Self {
                        #field: ::std::vec::Vec::from_iter(iter),
                        ..::core::default::Default::default()
                    }
                }
            }

            impl #generics #ident #generics #where_clause {
                /// Creates a new empty wrapper.
                /// See [`std::vec::Vec::new`].
                #visibility fn new() -> Self {
                    Self::default()
                }

                /// Creates a new wrapper with the specified capacity.
                /// See [`std::vec::Vec::with_capacity`].
                #visibility fn with_capacity(capacity: usize) -> Self {
                    Self {
                        #field: ::std::vec::Vec::with_capacity(capacity),
                        ..::core::default::Default::default()
                    }
                }

                /// Splits the wrapper at the given index, returning a new wrapper.
                /// See [`std::vec::Vec::split_off`].
                #visibility fn split_off(&mut self, at: usize) -> Self {
                    Self {
                        #field: self.#field.split_off(at),
                        ..::core::default::Default::default()
                    }
                }
            }
        }
    }

    /// Generates traits and methods that require `Clone`.
    ///
    /// - [`Extend<Enum>`][std::iter::Extend]
    /// - [`resize`][std::vec::Vec::resize]
    /// - [`extend_from_within`][std::vec::Vec::extend_from_within]
    /// - [`extend_from_slice`][std::vec::Vec::extend_from_slice]
    fn to_impl_with_clone(&self, nodyn: &NodynEnum) -> TokenStream {
        if self.vec_field.is_none() || !self.derived_traits.contains(&"Clone".to_string()) {
            return TokenStream::new();
        }
        let field = self.vec_field.as_ref().unwrap();
        let ident = &self.definition.ident;
        let visibility = &self.definition.vis;
        let generics = self.to_generics(nodyn);
        let where_clause = self.to_where(nodyn);
        let enum_ident = &nodyn.ident;
        let enum_generics = nodyn.to_generics();
        let new_type = &nodyn.generics.new_type();

        quote! {
            impl #generics ::core::iter::Extend<#enum_ident #enum_generics> for #ident #generics #where_clause {
                fn extend<#new_type: ::core::iter::IntoIterator<Item = #enum_ident #enum_generics>>(&mut self, iter: #new_type) {
                    self.#field.extend(iter.into_iter())
                }
            }

            impl #generics #ident #generics #where_clause {
                /// Resizes the vector to the new length, using the provided value.
                /// Accepts `Into<Enum>` for the value.
                /// See [`std::vec::Vec::resize`].
                #visibility fn resize<#new_type: ::core::convert::Into<#enum_ident #enum_generics>>(&mut self, new_len: usize, value: #new_type) {
                    self.#field.resize(new_len, value.into());
                }

                /// Copies elements from a range within the vector.
                /// See [`std::vec::Vec::extend_from_within`].
                #visibility fn extend_from_within<#new_type>(&mut self, src: #new_type)
                where #new_type: ::core::ops::RangeBounds<usize> {
                    self.#field.extend_from_within(src);
                }

                /// Extends the vector with a copy of the slice.
                /// See [`std::vec::Vec::extend_from_slice`].
                #visibility fn extend_from_slice(&mut self, other: &[#enum_ident #enum_generics]) {
                    self.#field.extend_from_slice(other);
                }
            }
        }
    }

    /// Generates traits and methods that require both `Clone` and `Default`.
    ///
    /// - [`From<&[Enum]>`][std::vec::Vec]
    /// - [`From<&mut [Enum]>`][std::vec::Vec]
    /// - A macro named after the wrapper in snake case for convenient construction.
    fn to_impl_with_clone_default(&self, nodyn: &NodynEnum) -> TokenStream {
        if self.vec_field.is_none()
            || !self.derived_traits.contains(&"Clone".to_string())
            || !self.derived_traits.contains(&"Default".to_string())
        {
            return TokenStream::new();
        }
        let field = self.vec_field.as_ref().unwrap();
        let ident = &self.definition.ident;
        let enum_ident = &nodyn.ident;
        let generics = self.to_generics(nodyn);
        let where_clause = self.to_where(nodyn);
        let enum_generics = nodyn.to_generics();
        let snake_ident = Ident::new(&camel_to_snake(&ident.to_string()), ident.span());
        // let pound = syn::token::Pound::default();

        // let new_type = &nodyn.generics.new_type();
        quote! {
            impl #generics ::core::convert::From<&[#enum_ident #enum_generics]> for #ident #generics #where_clause {
                fn from(s: &[#enum_ident #enum_generics]) -> #ident #generics {
                    Self {
                       #field: s.to_vec(),
                        ..::core::default::Default::default()
                    }
                }
            }

            impl #generics ::core::convert::From<&mut [#enum_ident #enum_generics]> for #ident #generics #where_clause {
                fn from(s: &mut [#enum_ident #enum_generics]) -> #ident #generics {
                    Self {
                       #field: s.to_vec(),
                        ..::core::default::Default::default()
                    }
                }
            }

            #[macro_export]
            macro_rules! #snake_ident {
                () => ( #ident::new() );
                ($elem:expr; $n:expr) => (
                    #ident::from( ::std::vec![#enum_ident::from($elem);$n])
                );
                ($($x:expr),+ $(,)?) => (
                    #ident::from( ::std::vec![$(#enum_ident::from($x)),+])
                );
            }
        }
    }

    /// Extracts derived traits from the struct's attributes.
    fn extract_traits(attrs: &[Attribute]) -> Vec<String> {
        let parser = Punctuated::<Ident, Token![,]>::parse_terminated;
        attrs
            .iter()
            .filter_map(|attr| {
                if let Meta::List(list) = &attr.meta {
                    if list.path.is_ident("derive") {
                        parser.parse(list.tokens.clone().into()).ok().map(|idents| {
                            idents
                                .into_iter()
                                .map(|id| id.to_string())
                                .collect::<Vec<_>>()
                        })
                    } else {
                        None
                    }
                } else {
                    None
                }
            })
            .flatten()
            .collect()
    }

    fn to_generics(&self, nodyn: &NodynEnum) -> TokenStream {
        if self.is_custom {
            nodyn.to_merged_generics(&self.definition.generics)
        } else {
            nodyn.to_generics()
        }
    }

    fn to_where(&self, nodyn: &NodynEnum) -> TokenStream {
        if self.is_custom {
            nodyn.to_merged_where(&self.definition.generics)
        } else {
            nodyn.generics.where_clause.to_token_stream()
        }
    }

    fn merge_generics(
        &self,
        nodyn: &NodynEnum,
        extra1: &GenericParam,
        extra2: &WherePredicate,
    ) -> (TokenStream, TokenStream) {
        if self.is_custom {
            (
                nodyn.to_merged_generics_and_param(&self.definition.generics, extra1),
                nodyn.to_merged_where_and_predicate(&self.definition.generics, extra2),
            )
        } else {
            (
                nodyn.to_generics_and_param(extra1),
                nodyn.to_where_and_predicate(extra2),
            )
        }
    }
}
