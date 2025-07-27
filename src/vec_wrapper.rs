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
///
/// This struct generates a wrapper around a `Vec<Enum>` with methods that delegate to the underlying
/// `std::vec::Vec`, plus additional methods for variant-specific access (via `NodynEnum`).
/// It supports both standard (auto-generated) and custom structs with a `vec_wrapper` attribute.
///
#[derive(Debug, Clone)]
pub(crate) struct VecWrapper {
    /// The struct definition, including fields and attributes.
    pub(crate) definition: ItemStruct,
    /// The identifier of the `Vec` field (e.g., `inner` or `inner_vec`).
    pub(crate) vec_field: Ident,
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
        let vec_wrapper = Ident::new("vec", Span::call_site());
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

        if let Some(vec_field) = vec_field {
            Ok(Self {
                definition: wrapper,
                vec_field,
                is_custom: true,
                derived_traits: traits,
            })
        } else {
            Err(syn::Error::new(
                wrapper.span(),
                "Struct is missing #[vec] atrribute",
            ))
        }
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
        let stripped_attrs = strip_copy(derive_attr);
        let wrapper: ItemStruct = parse_quote! {
            #[derive(Default)]
            #(#stripped_attrs)*
            #visibility struct #ident #generics {
                inner: std::vec::Vec< #enum_ident #generics >,
            }
        };
        let mut traits = Self::extract_traits(derive_attr);
        traits.push("Default".to_string());
        Self {
            definition: wrapper,
            vec_field: Ident::new("inner", Span::call_site()),
            is_custom: false,
            derived_traits: traits,
        }
    }

    /// Generates the complete `TokenStream` for the wrapper struct and its implementations.
    pub(crate) fn to_token_stream(&self, nodyn: &NodynEnum) -> TokenStream {
        let wrapper_struct = self.struct_tokens(nodyn);
        let impls = self.impl_tokens(nodyn);
        let traits = &self.traits_tokens(nodyn);
        let clone = &self.with_clone_tokens(nodyn);
        let clone_and_default = &self.with_clone_and_default_tokens(nodyn);
        let default = &self.with_default_tokens(nodyn);
        let partial_ord = &self.with_partial_ord_tokens(nodyn);
        let ord = &self.with_ord_tokens(nodyn);
        let copy = &self.with_copy_tokens(nodyn);
        quote! {
            #wrapper_struct
            #impls
            #traits
            #default
            #clone
            #clone_and_default
            #partial_ord
            #ord
            #copy
        }
    }

    /// Generates the `TokenStream` for the wrapper struct definition.
    fn struct_tokens(&self, nodyn: &NodynEnum) -> TokenStream {
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
            let field = &self.vec_field;
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
    fn impl_tokens(&self, nodyn: &NodynEnum) -> TokenStream {
        let ident = &self.definition.ident;
        let generics = self.generics_tokens(nodyn);
        let where_clause = self.where_tokens(nodyn);
        let delegated_methods = &self.delegated_methods_tokens(nodyn);
        let modified_methods = &self.modified_methods_tokens(nodyn);
        let partial_eq_methods = &self.partial_eq_methods_tokens();
        let field = &self.vec_field;
        let variant_methods = nodyn.variant_vec_tokens(field);

        quote! {
            impl #generics #ident #generics #where_clause {
                #delegated_methods
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
    /// - [`clear`][std::vec::Vec::clear]
    /// - [`len`][std::vec::Vec::len]
    /// - [`is_empty`][std::vec::Vec::is_empty]
    /// - [`fill_with`][std::vec::Vec::fill_with]
    /// - [`rotate_left`][std::vec::Vec::rotate_left]
    /// - [`rotate_right`][std::vec::Vec::rotate_right]
    /// - [`is_sorted_by`][std::vec::Vec::is_sorted_by]
    /// - [`is_sorted_by_key`][std::vec::Vec::is_sorted_by_key]
    /// - [`sort_by`][std::vec::Vec::sort_by]
    /// - [`sort_by_key`][std::vec::Vec::sort_by_key]
    /// - [`sort_unstable_by`][std::vec::Vec::sort_unstable_by]
    /// - [`sort_unstable_by_key`][std::vec::Vec::sort_unstable_by_key]
    /// - [`binary_search_by`][std::vec::Vec::binary_search_by]
    /// - [`binary_search_by_key`][std::vec::Vec::binary_search_by_key]
    #[allow(clippy::too_many_lines)]
    fn delegated_methods_tokens(&self, nodyn: &NodynEnum) -> TokenStream {
        let field = &self.vec_field;
        let visibility = &self.definition.vis;
        let enum_ident = &nodyn.ident;
        let enum_generics = nodyn.to_generics();
        let nt = &nodyn.generics.new_types(2);
        let new_type = &nt[0];
        let new_type2 = &nt[1];
        let lt = &nodyn.generics.new_lifetime();

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

            // new

            // #visibility fn drain<#new_type>(&mut self, range: #new_type) -> ::std::vec::Drain<'_, <#enum_ident #enum_generics>>
            // where #new_type: ::core::ops::RangeBounds<usize>,
            // {
            //     self.#field.drain(range)
            // }

            /// Clears the vector, removing all values.
            /// See [`std::vec::Vec::clear`].
            #visibility fn clear(&mut self) {
                self.#field.clear();
            }

            /// Returns the number of elements in the vector.
            /// See [`std::vec::Vec::len`].
            #visibility const fn len(&self) -> usize {
                self.#field.len()
            }

            /// Returns `true` if the vector contains no elements.
            /// See [`std::vec::Vec::is_empty`].
            #visibility const fn is_empty(&self) -> bool {
                self.#field.is_empty()
            }

            /// Fills `self` with elements returned by calling a closure repeatedly.
            /// See [`std::vec::Vec::fill_with`].
            #visibility fn fill_with<#new_type>(&mut self, f: #new_type)
            where #new_type: ::core::ops::FnMut() -> #enum_ident #enum_generics {
                self.#field.fill_with(f);
            }

            /// Rotates the slice in-place such that the first mid elements of the slice move to the end while the last self.len() - mid elements move to the front.
            /// See [`std::vec::Vec::rotate_left`].
            #visibility fn rotate_left(&mut self, mid: usize) {
                self.#field.rotate_left(mid)
            }

            /// Rotates the slice in-place such that the first self.len() - k elements of the slice move to the end while the last k elements move to the front.
            /// See [`std::vec::Vec::rotate_right`].
            #visibility fn rotate_right(&mut self, k: usize) {
                self.#field.rotate_right(k)
            }

            /// Checks if the elements are sorted using the given comparator function.
            /// See [`std::vec::Vec::is_sorted_by`].
            #visibility fn is_sorted_by<#lt, #new_type>(&#lt self, f: #new_type) -> bool
            where #new_type: ::core::ops::FnMut(&#lt #enum_ident #enum_generics, &#lt #enum_ident #enum_generics) -> bool, {
                self.#field.is_sorted_by(f)
            }

            /// Checks if the elements are sorted using the given key extraction function.
            /// See [`std::vec::Vec::is_sorted_by_key`].
            #visibility fn is_sorted_by_key<#lt, #new_type, #new_type2>(&#lt self, f: #new_type) -> bool
            where #new_type: ::core::ops::FnMut(&#lt #enum_ident #enum_generics) -> #new_type2,
                  #new_type2: ::core::cmp::PartialOrd {
                self.#field.is_sorted_by_key(f)
            }

            /// Sorts the slice with a comparison function, preserving initial order of equal elements.
            /// See [`std::vec::Vec::sort_by`].
            #visibility fn sort_by<#new_type>(&mut self, f: #new_type)
            where #new_type: ::core::ops::FnMut(&#enum_ident #enum_generics, &#enum_ident #enum_generics) -> ::core::cmp::Ordering, {
                self.#field.sort_by(f);
            }

            /// Sorts the slice with a key extraction function, preserving initial order of equal elements.
            /// See [`std::vec::Vec::sort_by_key`].
            #visibility fn sort_by_key<#new_type, #new_type2>(&mut self, f: #new_type)
            where #new_type: ::core::ops::FnMut(&#enum_ident #enum_generics) -> #new_type2,
                  #new_type2: ::core::cmp::Ord {
                self.#field.sort_by_key(f);
            }

            /// Sorts the slice with a comparison function, without preserving the initial order of equal elements.
            /// See [`std::vec::Vec::sort_unstable_by`].
            #visibility fn sort_unstable_by<#new_type>(&mut self, f: #new_type)
            where #new_type: ::core::ops::FnMut(&#enum_ident #enum_generics, &#enum_ident #enum_generics) -> ::core::cmp::Ordering, {
                self.#field.sort_unstable_by(f);
            }

            /// Sorts the slice with a key extraction function, without preserving the initial order of equal elements.
            /// See [`std::vec::Vec::sort_unstable_by_key`].
            #visibility fn sort_unstable_by_key<#new_type, #new_type2>(&mut self, f: #new_type)
            where #new_type: ::core::ops::FnMut(&#enum_ident #enum_generics) -> #new_type2,
                  #new_type2: ::core::cmp::Ord {
                self.#field.sort_unstable_by_key(f);
            }

            /// Binary searches this slice with a comparator function.
            /// See [`std::vec::Vec::binary_search_by`].
            #visibility fn binary_search_by<#lt, #new_type>(&#lt self, f: #new_type) -> ::core::result::Result<usize, usize>
            where #new_type: ::core::ops::FnMut(&#lt #enum_ident #enum_generics) -> ::core::cmp::Ordering, {
                self.#field.binary_search_by(f)
            }

            /// Binary searches this slice with a key extraction function.
            /// See [`std::vec::Vec::binary_search_by_key`].
            #visibility fn binary_search_by_key<#lt, #new_type, #new_type2>(&#lt self, b: &#new_type2, f: #new_type) -> ::core::result::Result<usize, usize>
            where #new_type: ::core::ops::FnMut(&#lt #enum_ident #enum_generics) -> #new_type2,
                  #new_type2: ::core::cmp::Ord {
                self.#field.binary_search_by_key(b, f)
            }

        }
    }

    /// Generates methods that differ from `std::vec::Vec`.
    ///
    /// - [`insert`][std::vec::Vec::insert]: Accepts `Into<Enum>` for the element.
    /// - [`push`][std::vec::Vec::push]: Accepts `Into<Enum>` for the value.
    fn modified_methods_tokens(&self, nodyn: &NodynEnum) -> TokenStream {
        let field = &self.vec_field;
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
    fn partial_eq_methods_tokens(&self) -> TokenStream {
        if !self.derived_traits.contains(&"PartialEq".to_string()) {
            return TokenStream::new();
        }
        let field = &self.vec_field;
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
    fn traits_tokens(&self, nodyn: &NodynEnum) -> TokenStream {
        let ident = &self.definition.ident;
        let enum_ident = &nodyn.ident;
        let field = &self.vec_field;
        let generics = self.generics_tokens(nodyn);
        let where_clause = self.where_tokens(nodyn);
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
    /// - `From<Vec<T>> where T: Into<enum>` (all variants)
    /// - [`FromIterator<Enum>`][std::iter::FromIterator]
    /// - [`new`][std::vec::Vec::new]
    /// - [`with_capacity`][std::vec::Vec::with_capacity]
    /// - [`split_off`][std::vec::Vec::split_off]
    fn with_default_tokens(&self, nodyn: &NodynEnum) -> TokenStream {
        if !self.derived_traits.contains(&"Default".to_string()) {
            return TokenStream::new();
        }
        let field = &self.vec_field;
        let ident = &self.definition.ident;
        let enum_ident = &nodyn.ident;
        let visibility = &self.definition.vis;
        let generics = self.generics_tokens(nodyn);
        let where_clause = self.where_tokens(nodyn);
        let enum_generics = nodyn.to_generics();
        let new_type = &nodyn.generics.new_type();
        let variants = nodyn.variants.iter().map(|variant| {
            let ty = &variant.ty;
            quote!{
                impl #generics ::core::convert::From<::std::vec::Vec<#ty>> for #ident #generics #where_clause {
                    fn from(v: ::std::vec::Vec<#ty>) -> Self {
                        Self {
                            #field: v.into_iter().map(#enum_ident::from).collect(),
                            ..::core::default::Default::default()
                        }
                    }
                }
            }
        }).collect::<Vec<_>>();

        let vec_macro = self.macro_tokens(nodyn);

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

            #(#variants)*

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

            #vec_macro
        }
    }

    /// Generates traits and methods that require `Clone`.
    ///
    /// - [`Extend<Enum>`][std::iter::Extend] also for each variant
    /// - [`resize`][std::vec::Vec::resize]
    /// - [`extend_from_within`][std::vec::Vec::extend_from_within]
    /// - [`extend_from_slice`][std::vec::Vec::extend_from_slice]
    /// - [`clone_from_slice`][std::vec::Vec::clone_from_slice]
    /// - [`to_vec`][std::vec::Vec::to_vec]
    /// - [`fill`][std::vec::Vec::fill]
    fn with_clone_tokens(&self, nodyn: &NodynEnum) -> TokenStream {
        if !self.derived_traits.contains(&"Clone".to_string()) {
            return TokenStream::new();
        }
        let field = &self.vec_field;
        let ident = &self.definition.ident;
        let visibility = &self.definition.vis;
        let generics = self.generics_tokens(nodyn);
        let where_clause = self.where_tokens(nodyn);
        let enum_ident = &nodyn.ident;
        let enum_generics = nodyn.to_generics();
        let new_type = &nodyn.generics.new_type();

        let variants = nodyn.variants.iter().map(|variant| {
            let ty = &variant.ty;
            quote!{
                impl #generics ::core::iter::Extend<#ty> for #ident #generics #where_clause {
                    fn extend<#new_type: ::core::iter::IntoIterator<Item = #ty>>(&mut self, iter: #new_type) {
                        self.#field.extend(iter.into_iter().map(#enum_ident::from))
                    }
                }
            }
        }).collect::<Vec<_>>();

        quote! {
            impl #generics ::core::iter::Extend<#enum_ident #enum_generics> for #ident #generics #where_clause {
                fn extend<#new_type: ::core::iter::IntoIterator<Item = #enum_ident #enum_generics>>(&mut self, iter: #new_type) {
                    self.#field.extend(iter.into_iter())
                }
            }

            #(#variants)*

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

                /// Copies the elements from src into self.
                /// See [`std::vec::Vec::clone_from_slice`].
                #visibility fn clone_from_slice(&mut self, other: &[#enum_ident #enum_generics]) {
                    self.#field.clone_from_slice(other);
                }

                /// Copies self into a new Vec.
                /// See [`std::vec::Vec::clone_from_slice`].
                #visibility fn to_vec(&self) -> Vec<#enum_ident #enum_generics> {
                    self.#field.to_vec()
                }

                /// Fills self with elements by cloning value.
                /// Accepts `Into<Enum>` for the value.
                /// See [`std::vec::Vec::fill`].
                #visibility fn fill<#new_type: ::core::convert::Into<#enum_ident #enum_generics>>(&mut self, value: #new_type) {
                    self.#field.fill(value.into());
                }
            }
        }
    }

    /// Generates the macto for constructing the `Vec` like `vec!`.
    fn macro_tokens(&self, nodyn: &NodynEnum) -> TokenStream {
        let ident = &self.definition.ident;
        let field = &self.vec_field;
        let enum_ident = &nodyn.ident;
        let snake_ident = Ident::new(&camel_to_snake(&ident.to_string()), ident.span());
        let (macro_vec, macro_enum) = if let Some(path) = &nodyn.module_path {
            (quote! { ::#path::#ident }, quote! { ::#path::#enum_ident })
        } else {
            (quote! { #ident }, quote! { #enum_ident })
        };
        if self.is_custom {
            quote! {
                #[macro_export]
                macro_rules! #snake_ident {
                    () => ( #ident::new() );
                    ($elem:expr; $n:expr) => (
                        #macro_vec{ #field: ::std::vec![#macro_enum::from($elem);$n], .. ::core::default::Default::default() }
                    );
                    ($($x:expr),+ $(,)?) => (
                        #macro_vec{ #field: ::std::vec![$(#macro_enum::from($x)),+], .. ::core::default::Default::default() }
                    );
                }
            }
        } else {
            quote! {
                #[macro_export]
                macro_rules! #snake_ident {
                    () => ( #ident::new() );
                    ($elem:expr; $n:expr) => (
                        #macro_vec{ #field: ::std::vec![#macro_enum::from($elem);$n] }
                    );
                    ($($x:expr),+ $(,)?) => (
                        #macro_vec{ #field: ::std::vec![$(#macro_enum::from($x)),+] }
                    );
                }
            }
        }
    }

    /// Generates traits and methods that require both `Clone` and `Default`.
    ///
    /// - `From<&[Enum]>` also for each variant
    /// - `From<&mut [Enum]>` also for each variant
    fn with_clone_and_default_tokens(&self, nodyn: &NodynEnum) -> TokenStream {
        if !self.derived_traits.contains(&"Clone".to_string())
            || !self.derived_traits.contains(&"Default".to_string())
        {
            return TokenStream::new();
        }
        let field = &self.vec_field;
        let ident = &self.definition.ident;
        let enum_ident = &nodyn.ident;
        let generics = self.generics_tokens(nodyn);
        let where_clause = self.where_tokens(nodyn);
        let enum_generics = nodyn.to_generics();

        let variants = nodyn.variants.iter().map(|variant| {
            let ty = &variant.ty;
            quote!{
                impl #generics ::core::convert::From<&[#ty]> for #ident #generics #where_clause {
                    fn from(s: &[#ty]) -> #ident #generics {
                        Self {
                           #field: s.iter().cloned().map(#enum_ident::from).collect(),
                            ..::core::default::Default::default()
                        }
                    }
                }

                impl #generics ::core::convert::From<&mut [#ty]> for #ident #generics #where_clause {
                    fn from(s: &mut [#ty]) -> #ident #generics {
                        Self {
                           #field: s.iter().cloned().map(#enum_ident::from).collect(),
                            ..::core::default::Default::default()
                        }
                    }
                }
            }
        }).collect::<Vec<_>>();

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

            #(#variants)*
        }
    }

    /// Generates methods that require the enum to have `#[derive(PartialOrd)]`;
    /// - [`is_sorted`][std::vec::Vec::is_sorted]
    //
    //   fn copy_from_slice(&mut self, src: &[T]) where T: Copy,
    //   fn copy_within<R>(&mut self, src: R, dest: usize) where R: RangeBounds<usize>, T: Copy,
    fn with_partial_ord_tokens(&self, nodyn: &NodynEnum) -> TokenStream {
        if !self.derived_traits.contains(&"PartialOrd".to_string()) {
            return TokenStream::new();
        }
        let field = &self.vec_field;
        let ident = &self.definition.ident;
        let generics = self.generics_tokens(nodyn);
        let where_clause = self.where_tokens(nodyn);
        let visibility = &self.definition.vis;

        quote! {
            /// Checks if the elements of this slice are sorted.
            /// See [`std::vec::Vec::is_sorted`].
            impl #generics #ident #generics #where_clause {
                #visibility fn is_sorted(&self) -> bool {
                    self.#field.is_sorted()
                }
            }
        }
    }

    /// Generates methods that require the enum to have `#[derive(Ord)]`;
    ///
    /// - [`sort`][std::vec::Vec::sort]
    /// - [`sort_unstable`][std::vec::Vec::sort_unstable]
    /// - [`binary_search`][std::vec::Vec::binary_search]
    fn with_ord_tokens(&self, nodyn: &NodynEnum) -> TokenStream {
        if !self.derived_traits.contains(&"Ord".to_string()) {
            return TokenStream::new();
        }
        let field = &self.vec_field;
        let ident = &self.definition.ident;
        let generics = self.generics_tokens(nodyn);
        let where_clause = self.where_tokens(nodyn);
        let visibility = &self.definition.vis;
        let enum_ident = &nodyn.ident;
        let enum_generics = nodyn.to_generics();

        quote! {
            /// Sorts the slice, preserving initial order of equal elements.
            /// See [`std::vec::Vec::sort()`].
            impl #generics #ident #generics #where_clause {
                #visibility fn sort(&mut self) {
                    self.#field.sort();
                }
            }

            /// Sorts the slice without preserving the initial order of equal elements.
            /// See [`std::vec::Vec::sort_unstable()`].
            impl #generics #ident #generics #where_clause {
                #visibility fn sort_unstable(&mut self) {
                    self.#field.sort_unstable();
                }
            }

            /// Binary searches this slice for a given element.
            /// See [`std::vec::Vec::binary_search()`].
            impl #generics #ident #generics #where_clause {
                #visibility fn binary_search(&mut self, x: &#enum_ident #enum_generics) -> ::core::result::Result<usize, usize> {
                    self.#field.binary_search(x)
                }
            }
        }
    }

    /// Generates methods that require the enum to have `#[derive(Copy)]`;
    /// - [`copy_from_slice`][std::vec::Vec::copy_from_slice]
    /// - [`copy_within`][std::vec::Vec::copy_within]
    fn with_copy_tokens(&self, nodyn: &NodynEnum) -> TokenStream {
        if !self.derived_traits.contains(&"Copy".to_string()) {
            return TokenStream::new();
        }
        let field = &self.vec_field;
        let ident = &self.definition.ident;
        let generics = self.generics_tokens(nodyn);
        let where_clause = self.where_tokens(nodyn);
        let visibility = &self.definition.vis;
        let enum_ident = &nodyn.ident;
        let enum_generics = nodyn.to_generics();
        let new_type = &nodyn.generics.new_type();

        quote! {
            impl #generics #ident #generics #where_clause {
                /// Copies all elements from src into self, using a memcpy.
                /// See [`std::vec::Vec::copy_from_slice`].
                #visibility fn copy_from_slice(&mut self, src: &[#enum_ident #enum_generics]) {
                    self.#field.copy_from_slice(src);
                }

                /// Copies elements from one part of the slice to another part of itself, using a memmove.
                #visibility fn copy_within<#new_type>(&mut self, src: # new_type, dest: usize)
                where
                    #new_type: ::core::ops::RangeBounds<usize>, {
                       self.#field.copy_within(src, dest);
                }
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

    fn generics_tokens(&self, nodyn: &NodynEnum) -> TokenStream {
        if self.is_custom {
            nodyn.to_merged_generics(&self.definition.generics)
        } else {
            nodyn.to_generics()
        }
    }

    fn where_tokens(&self, nodyn: &NodynEnum) -> TokenStream {
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

fn strip_copy(attrs: &[Attribute]) -> Vec<Attribute> {
    let parser = Punctuated::<Ident, Token![,]>::parse_terminated;
    attrs
        .iter()
        .map(|attr| {
            if let Meta::List(list) = &attr.meta {
                if list.path.is_ident("derive") {
                    let ids: Vec<Ident> = parser
                        .parse(list.tokens.clone().into())
                        .ok()
                        .map(|idents| {
                            idents
                                .into_iter()
                                .filter_map(|id| if id == "Copy" { None } else { Some(id) })
                                .collect()
                        })
                        .unwrap();
                    let a: Attribute = parse_quote! { #[derive( #(#ids ,)* )]};
                    a
                } else {
                    attr.clone()
                }
            } else {
                attr.clone()
            }
        })
        .collect()
}
