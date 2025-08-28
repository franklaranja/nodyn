use core::option::Option::None;
use proc_macro2::{Span, TokenStream, TokenTree};
use quote::{ToTokens, format_ident, quote};
use syn::spanned::Spanned;
use syn::{
    Attribute, Fields, Generics, Ident, ItemStruct, Meta, Token, Visibility, WherePredicate,
    parse::{Parse, ParseStream, Parser},
    parse_quote,
    punctuated::Punctuated,
};

use crate::{GenericsExt, NodynEnum, camel_to_snake};

#[derive(Debug, Clone)]
pub(crate) struct StandardVecWrapper {
    pub(crate) attrs: Vec<Attribute>,
    ident: Option<Ident>,
}

impl Parse for StandardVecWrapper {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        // let attrs = input.call(Attribute::parse_outer)?;
        // println!("vec attrs: {attrs:?}");
        let attrs = Vec::new();
        input.parse::<crate::keyword::vec>()?;
        let ident = if input.peek(Ident) {
            Some(input.parse::<Ident>()?)
        } else {
            None
        };
        if input.peek(Token![;]) {
            input.parse::<syn::token::Semi>()?;
        }
        Ok(Self { attrs, ident })
    }
}

impl StandardVecWrapper {
    pub(crate) fn into_vec_wrapper(
        self,
        visibility: &Visibility,
        enum_ident: &Ident,
        generics: &Generics,
        derive_attr: &[Attribute],
    ) -> VecWrapper {
        let ident = self
            .ident
            .unwrap_or_else(|| format_ident!("{}Vec", enum_ident));
        let defined_attrs = self.attrs;
        let stripped_attrs = strip_copy(derive_attr);
        let wrapper: ItemStruct = parse_quote! {
            #[derive(Default)]
            #(#defined_attrs)*
            #(#stripped_attrs)*
            #visibility struct #ident #generics {
                #visibility inner: std::vec::Vec< #enum_ident #generics >,
            }
        };
        VecWrapper {
            definition: wrapper,
            vec_field: Ident::new("inner", Span::call_site()),
            is_custom: false,
        }
    }
}

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
}

impl Parse for VecWrapper {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let wrapper = input.parse::<ItemStruct>()?;
        if !matches!(wrapper.fields, Fields::Named(_)) {
            return Err(syn::Error::new(
                wrapper.span(),
                "Only structs with named fields are supported",
            ));
        }
        let vec_field = format_ident!("temp");
        Ok(Self {
            definition: wrapper,
            vec_field,
            is_custom: true,
        })
    }
}

impl VecWrapper {
    pub(crate) fn parse_custom_attrs(attrs: Vec<Attribute>) -> (Vec<Attribute>, Option<Ident>) {
        let mut result_attrs = Vec::new();
        let vec_wrapper = Ident::new("vec", Span::call_site());
        let mut vec_field = None;
        for attribute in attrs {
            if let Meta::Path(path) = &attribute.meta {
                if path.is_ident(&vec_wrapper) {
                    vec_field = Some(Ident::new("inner_vec", Span::call_site()));
                } else {
                    result_attrs.push(attribute);
                }
            } else if let Meta::List(list) = &attribute.meta {
                if list.path.is_ident(&vec_wrapper) {
                    if let Some(TokenTree::Ident(ident)) = list.tokens.clone().into_iter().next() {
                        vec_field = Some(ident);
                    } else {
                        vec_field = Some(Ident::new("inner_vec", Span::call_site()));
                    }
                } else {
                    result_attrs.push(attribute);
                }
            } else {
                result_attrs.push(attribute);
            }
        }
        (result_attrs, vec_field)
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
            let enum_generics = nodyn.generics_tokens();
            let visibility = &self.definition.vis;
            let ident = &self.definition.ident;
            let generics = nodyn.merged_generics_tokens(&self.definition.generics);
            let where_clause = nodyn.merged_where_tokens(&self.definition.generics);

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
                    #visibility #field: std::vec::Vec< #enum_ident #enum_generics >,
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
        let slice_methods = if self.is_custom {
            self.slice_methods_tokens(nodyn)
        } else {
            TokenStream::new()
        };
        let modified_methods = &self.modified_methods_tokens(nodyn);
        let partial_eq_methods = &self.partial_eq_methods_tokens(nodyn);
        let field = &self.vec_field;
        let variant_methods = nodyn.variant_vec_tokens(field);
        let type_generics = self.merged_type_generics_tokens(nodyn);

        quote! {
            impl #generics #ident #type_generics #where_clause {
                #delegated_methods
                #slice_methods
                #modified_methods
                #partial_eq_methods
                #variant_methods
            }
        }
    }

    /// Generates standard `Vec` methods that directly delegate to the underlying `Vec`.
    ///
    /// These methods match their `Vec` counterparts exactly:
    /// - [`capacity`][Vec::capacity]
    /// - [`reserve`][Vec::reserve]
    /// - [`reserve_exact`][Vec::reserve_exact]
    /// - [`try_reserve`][Vec::try_reserve]
    /// - [`try_reserve_exact`][Vec::try_reserve_exact]
    /// - [`shrink_to_fit`][Vec::shrink_to_fit]
    /// - [`shrink_to`][Vec::shrink_to]
    /// - [`into_boxed_slice`][Vec::into_boxed_slice]
    /// - [`truncate`][Vec::truncate]
    /// - [`as_slice`][Vec::as_slice]
    /// - [`as_mut_slice`][Vec::as_mut_slice]
    /// - [`swap_remove`][Vec::swap_remove]
    /// - [`remove`][Vec::remove]
    /// - [`retain`][Vec::retain]
    /// - [`retain_mut`][Vec::retain_mut]
    /// - [`dedup_by_key`][Vec::dedup_by_key]
    /// - [`dedup_by`][Vec::dedup_by]
    /// - [`pop`][Vec::pop]
    /// - [`pop_if`][Vec::pop_if]
    /// - [`append`][Vec::append]
    /// - [`splice`][Vec::splice]
    /// - [`extract_if`][Vec::extract_if]
    /// - [`clear`][Vec::clear]
    /// - [`len`][Vec::len]
    /// - [`is_empty`][Vec::is_empty]
    fn delegated_methods_tokens(&self, nodyn: &NodynEnum) -> TokenStream {
        let field = &self.vec_field;
        let visibility = &self.definition.vis;
        let enum_ident = &nodyn.ident;
        let enum_generics = nodyn.generics_tokens();
        let nt = &nodyn.generics.new_types(2);
        let new_type = &nt[0];
        let new_type2 = &nt[1];

        quote! {
            /// Returns the total number of elements the vector can hold without reallocating.
            /// See [`Vec::capacity`].
            #visibility fn capacity(&self) -> usize {
                self.#field.capacity()
            }

            /// Reserves capacity for at least `additional` more elements.
            /// See [`Vec::reserve`].
            #visibility fn reserve(&mut self, additional: usize) {
                self.#field.reserve(additional);
            }

            /// Reserves the minimum capacity for exactly `additional` more elements.
            /// See [`Vec::reserve_exact`].
            #visibility fn reserve_exact(&mut self, additional: usize) {
                self.#field.reserve_exact(additional);
            }

            /// Tries to reserve capacity for at least `additional` more elements.
            /// See [`Vec::try_reserve`].
            #visibility fn try_reserve(&mut self, additional: usize) -> ::core::result::Result<(), ::std::collections::TryReserveError> {
                self.#field.try_reserve(additional)
            }

            /// Tries to reserve the minimum capacity for exactly `additional` more elements.
            /// See [`Vec::try_reserve_exact`].
            #visibility fn try_reserve_exact(&mut self, additional: usize) -> ::core::result::Result<(), ::std::collections::TryReserveError> {
                self.#field.try_reserve_exact(additional)
            }

            /// Shrinks the capacity of the vector as much as possible.
            /// See [`Vec::shrink_to_fit`].
            #visibility fn shrink_to_fit(&mut self) {
                self.#field.shrink_to_fit();
            }

            /// Shrinks the capacity of the vector with a lower bound.
            /// See [`Vec::shrink_to`].
            #visibility fn shrink_to(&mut self, min_capacity: usize) {
                self.#field.shrink_to(min_capacity);
            }

            /// Converts the vector into a `Box<[Enum]>`.
            /// See [`Vec::into_boxed_slice`].
            #visibility fn into_boxed_slice(self) -> ::std::boxed::Box<[#enum_ident #enum_generics]> {
                self.#field.into_boxed_slice()
            }

            /// Shortens the vector to the specified length.
            /// See [`Vec::truncate`].
            #visibility fn truncate(&mut self, len: usize) {
                self.#field.truncate(len);
            }

            /// Returns a slice containing all elements.
            /// See [`Vec::as_slice`].
            #visibility const fn as_slice(&self) -> &[#enum_ident #enum_generics] {
                self.#field.as_slice()
            }

            /// Returns a mutable slice containing all elements.
            /// See [`Vec::as_mut_slice`].
            #visibility const fn as_mut_slice(&mut self) -> &mut [#enum_ident #enum_generics] {
                self.#field.as_mut_slice()
            }

            /// Removes and returns the element at `index`, swapping with the last element.
            /// See [`Vec::swap_remove`].
            #visibility fn swap_remove(&mut self, index: usize) -> #enum_ident #enum_generics {
                self.#field.swap_remove(index)
            }

            /// Removes and returns the element at `index`.
            /// See [`Vec::remove`].
            #visibility fn remove(&mut self, index: usize) -> #enum_ident #enum_generics {
                self.#field.remove(index)
            }

            /// Retains only the elements specified by the predicate.
            /// See [`Vec::retain`].
            #visibility fn retain<#new_type>(&mut self, f: #new_type)
            where #new_type: ::core::ops::FnMut(&#enum_ident #enum_generics) -> bool {
                self.#field.retain(f);
            }

            /// Retains only the elements specified by the mutable predicate.
            /// See [`Vec::retain_mut`].
            #visibility fn retain_mut<#new_type>(&mut self, f: #new_type)
            where #new_type: ::core::ops::FnMut(&mut #enum_ident #enum_generics) -> bool {
                self.#field.retain_mut(f);
            }

            /// Removes consecutive duplicate elements based on a key function.
            /// See [`Vec::dedup_by_key`].
            #visibility fn dedup_by_key<#new_type, #new_type2>(&mut self, key: #new_type)
            where
                #new_type: ::core::ops::FnMut(&mut #enum_ident #enum_generics) -> #new_type2,
                #new_type2: ::core::cmp::PartialEq,
            {
                self.#field.dedup_by_key(key);
            }

            /// Removes consecutive duplicate elements based on a predicate.
            /// See [`Vec::dedup_by`].
            #visibility fn dedup_by<#new_type>(&mut self, same_bucket: #new_type)
            where #new_type: ::core::ops::FnMut(&mut #enum_ident #enum_generics, &mut #enum_ident #enum_generics) -> bool {
                self.#field.dedup_by(same_bucket)
            }

            /// Removes and returns the last element, if any.
            /// See [`Vec::pop`].
            #visibility fn pop(&mut self) -> ::core::option::Option<#enum_ident #enum_generics> {
                self.#field.pop()
            }

            /// Removes and returns the last element if it satisfies the predicate.
            /// See [`Vec::pop_if`].
            #visibility fn pop_if(&mut self, predicate: impl ::core::ops::FnOnce(&mut #enum_ident #enum_generics) -> bool) -> ::core::option::Option<#enum_ident #enum_generics> {
                self.#field.pop_if(predicate)
            }

            /// Appends all elements from `other` to `self`, emptying `other`.
            /// See [`Vec::append`].
            #visibility fn append(&mut self, other: &mut Self) {
                self.#field.append(&mut other.#field)
            }

            /// Replaces elements in the specified range with new ones.
            /// See [`Vec::splice`].
            #visibility fn splice<#new_type, #new_type2>(&mut self, range: #new_type, replace_with: #new_type2)
            -> ::std::vec::Splice<'_, <#new_type2 as ::core::iter::IntoIterator>::IntoIter>
            where
                #new_type: ::core::ops::RangeBounds<usize>,
                #new_type2: ::core::iter::IntoIterator<Item = #enum_ident #enum_generics> {
                self.#field.splice(range, replace_with)
            }

            /// Removes elements that match the predicate in the specified range.
            /// See [`Vec::extract_if`].
            #visibility fn extract_if<#new_type, #new_type2>(&mut self, range: #new_type2, filter: #new_type)
            -> ::std::vec::ExtractIf<'_, #enum_ident #enum_generics, #new_type>
            where
                #new_type: ::core::ops::FnMut(&mut #enum_ident #enum_generics) -> bool,
                #new_type2: ::core::ops::RangeBounds<usize> {
                self.#field.extract_if(range, filter)
            }

            // #visibility fn drain<#new_type>(&mut self, range: #new_type) -> ::std::vec::Drain<'_, <#enum_ident #enum_generics>>
            // where #new_type: ::core::ops::RangeBounds<usize>,
            // {
            //     self.#field.drain(range)
            // }

            /// Clears the vector, removing all values.
            /// See [`Vec::clear`].
            #visibility fn clear(&mut self) {
                self.#field.clear();
            }

            /// Returns the number of elements in the vector.
            /// See [`Vec::len`].
            #visibility const fn len(&self) -> usize {
                self.#field.len()
            }

            /// Returns `true` if the vector contains no elements.
            /// See [`Vec::is_empty`].
            #visibility const fn is_empty(&self) -> bool {
                self.#field.is_empty()
            }
        }
    }

    /// Generates standard slice methods that directly delegate to the underlying `Vec`.
    ///
    /// These methods match their slice counterparts exactly:
    ///
    /// - [`first`][slice::first]
    /// - [`first_mut`][slice::first_mut]
    /// - [`last`][slice::last]
    /// - [`last_mut`][slice::last_mut]
    /// - [`split_first`][slice::split_first]
    /// - [`split_first_mut`][slice::split_first_mut]
    /// - [`split_last`][slice::split_last]
    /// - [`split_last_mut`][slice::split_last_mut]
    /// - [`get`][slice::get]
    /// - [`get_mut`][slice::get_mut]
    /// - [`swap`][slice::swap]
    /// - [`reverse`][slice::reverse]
    /// - [`iter`][slice::iter]
    /// - [`iter_mut`][slice::iter_mut]
    /// - [`fill_with`][slice::fill_with]
    /// - [`rotate_left`][slice::rotate_left]
    /// - [`rotate_right`][slice::rotate_right]
    /// - [`is_sorted_by`][slice::is_sorted_by]
    /// - [`is_sorted_by_key`][slice::is_sorted_by_key]
    /// - [`sort_by`][slice::sort_by]
    /// - [`sort_by_key`][slice::sort_by_key]
    /// - [`sort_unstable_by`][slice::sort_unstable_by]
    /// - [`sort_unstable_by_key`][slice::sort_unstable_by_key]
    /// - [`binary_search_by`][slice::binary_search_by]
    /// - [`binary_search_by_key`][slice::binary_search_by_key]
    #[allow(clippy::too_many_lines)]
    fn slice_methods_tokens(&self, nodyn: &NodynEnum) -> TokenStream {
        let field = &self.vec_field;
        let visibility = &self.definition.vis;
        let enum_ident = &nodyn.ident;
        let enum_generics = nodyn.generics_tokens();
        let nt = &nodyn.generics.new_types(2);
        let new_type = &nt[0];
        let new_type2 = &nt[1];
        let lt = &nodyn.generics.new_lifetime();

        quote! {
            /// Returns the first element, if any.
            /// See [`slice`::first`].
            #visibility fn first(&self) -> ::core::option::Option<&#enum_ident #enum_generics> {
                self.#field.first()
            }

            /// Returns a mutable reference to the first element, if any.
            /// See [`slice::first_mut`].
            #visibility fn first_mut(&mut self) -> ::core::option::Option<&mut #enum_ident #enum_generics> {
                self.#field.first_mut()
            }

            /// Returns the last element, if any.
            /// See [`slice::last`].
            #visibility fn last(&self) -> ::core::option::Option<&#enum_ident #enum_generics> {
                self.#field.last()
            }

            /// Returns a mutable reference to the last element, if any.
            /// See [`slice::last_mut`].
            #visibility fn last_mut(&mut self) -> ::core::option::Option<&mut #enum_ident #enum_generics> {
                self.#field.last_mut()
            }

            /// Returns the first element and the rest of the slice, if any.
            /// See [`slice::split_first`].
            #visibility fn split_first(&self) -> ::core::option::Option<(&#enum_ident #enum_generics, &[#enum_ident #enum_generics])> {
                self.#field.split_first()
            }

            /// Returns a mutable first element and the rest of the slice, if any.
            /// See [`slice::split_first_mut`].
            #visibility fn split_first_mut(&mut self) -> ::core::option::Option<(&mut #enum_ident #enum_generics, &mut [#enum_ident #enum_generics])> {
                self.#field.split_first_mut()
            }

            /// Returns the last element and the rest of the slice, if any.
            /// See [`slice::split_last`].
            #visibility fn split_last(&self) -> ::core::option::Option<(&#enum_ident #enum_generics, &[#enum_ident #enum_generics])> {
                self.#field.split_last()
            }

            /// Returns a mutable last element and the rest of the slice, if any.
            /// See [`slice::split_last_mut`].
            #visibility fn split_last_mut(&mut self) -> ::core::option::Option<(&mut #enum_ident #enum_generics, &mut [#enum_ident #enum_generics])> {
                self.#field.split_last_mut()
            }

            /// Returns a reference to an element or subslice by index.
            /// See [`slice::get`].
            #visibility fn get<#new_type>(&self, index: #new_type) -> ::core::option::Option<&<#new_type as ::core::slice::SliceIndex<[#enum_ident #enum_generics]>>::Output>
            where
                #new_type: ::core::slice::SliceIndex<[#enum_ident #enum_generics]> {
                self.#field.get(index)
            }

            /// Returns a mutable reference to an element or subslice by index.
            /// See [`slice::get_mut`].
            #visibility fn get_mut<#new_type>(&mut self, index: #new_type) -> ::core::option::Option<&mut <#new_type as ::core::slice::SliceIndex<[#enum_ident #enum_generics]>>::Output>
            where
                #new_type: ::core::slice::SliceIndex<[#enum_ident #enum_generics]> {
                self.#field.get_mut(index)
            }

            /// Swaps two elements in the vector.
            /// See [`slice::swap`].
            #visibility fn swap(&mut self, a: usize, b: usize) {
                self.#field.swap(a, b);
            }

            /// Reverses the order of elements in the vector.
            /// See [`slice::reverse`].
            #visibility fn reverse(&mut self) {
                self.#field.reverse();
            }

            /// Returns an iterator over the vector's elements.
            /// See [`slice::iter`].
            #visibility fn iter(&self) -> ::core::slice::Iter<'_, #enum_ident #enum_generics> {
                self.#field.iter()
            }

            /// Returns a mutable iterator over the vector's elements.
            /// See [`slice::iter_mut`].
            #visibility fn iter_mut(&mut self) -> ::core::slice::IterMut<'_, #enum_ident #enum_generics> {
                self.#field.iter_mut()
            }

            /// Fills `self` with elements returned by calling a closure repeatedly.
            /// See [`slice::fill_with`].
            #visibility fn fill_with<#new_type>(&mut self, f: #new_type)
            where #new_type: ::core::ops::FnMut() -> #enum_ident #enum_generics {
                self.#field.fill_with(f);
            }

            /// Rotates the slice in-place such that the first mid elements of the slice move to the end while the last self.len() - mid elements move to the front.
            /// See [`slice::rotate_left`].
            #visibility fn rotate_left(&mut self, mid: usize) {
                self.#field.rotate_left(mid)
            }

            /// Rotates the slice in-place such that the first self.len() - k elements of the slice move to the end while the last k elements move to the front.
            /// See [`slice::rotate_right`].
            #visibility fn rotate_right(&mut self, k: usize) {
                self.#field.rotate_right(k)
            }

            /// Checks if the elements are sorted using the given comparator function.
            /// See [`slice::is_sorted_by`].
            #visibility fn is_sorted_by<#lt, #new_type>(&#lt self, f: #new_type) -> bool
            where #new_type: ::core::ops::FnMut(&#lt #enum_ident #enum_generics, &#lt #enum_ident #enum_generics) -> bool, {
                self.#field.is_sorted_by(f)
            }

            /// Checks if the elements are sorted using the given key extraction function.
            /// See [`slice::is_sorted_by_key`].
            #visibility fn is_sorted_by_key<#lt, #new_type, #new_type2>(&#lt self, f: #new_type) -> bool
            where #new_type: ::core::ops::FnMut(&#lt #enum_ident #enum_generics) -> #new_type2,
                  #new_type2: ::core::cmp::PartialOrd {
                self.#field.is_sorted_by_key(f)
            }

            /// Sorts the slice with a comparison function, preserving initial order of equal elements.
            /// See [`slice::sort_by`].
            #visibility fn sort_by<#new_type>(&mut self, f: #new_type)
            where #new_type: ::core::ops::FnMut(&#enum_ident #enum_generics, &#enum_ident #enum_generics) -> ::core::cmp::Ordering, {
                self.#field.sort_by(f);
            }

            /// Sorts the slice with a key extraction function, preserving initial order of equal elements.
            /// See [`slice::sort_by_key`].
            #visibility fn sort_by_key<#new_type, #new_type2>(&mut self, f: #new_type)
            where #new_type: ::core::ops::FnMut(&#enum_ident #enum_generics) -> #new_type2,
                  #new_type2: ::core::cmp::Ord {
                self.#field.sort_by_key(f);
            }

            /// Sorts the slice with a comparison function, without preserving the initial order of equal elements.
            /// See [`slice::sort_unstable_by`].
            #visibility fn sort_unstable_by<#new_type>(&mut self, f: #new_type)
            where #new_type: ::core::ops::FnMut(&#enum_ident #enum_generics, &#enum_ident #enum_generics) -> ::core::cmp::Ordering, {
                self.#field.sort_unstable_by(f);
            }

            /// Sorts the slice with a key extraction function, without preserving the initial order of equal elements.
            /// See [`slice::sort_unstable_by_key`].
            #visibility fn sort_unstable_by_key<#new_type, #new_type2>(&mut self, f: #new_type)
            where #new_type: ::core::ops::FnMut(&#enum_ident #enum_generics) -> #new_type2,
                  #new_type2: ::core::cmp::Ord {
                self.#field.sort_unstable_by_key(f);
            }

            /// Binary searches this slice with a comparator function.
            /// See [`slice::binary_search_by`].
            #visibility fn binary_search_by<#lt, #new_type>(&#lt self, f: #new_type) -> ::core::result::Result<usize, usize>
            where #new_type: ::core::ops::FnMut(&#lt #enum_ident #enum_generics) -> ::core::cmp::Ordering, {
                self.#field.binary_search_by(f)
            }

            /// Binary searches this slice with a key extraction function.
            /// See [`slice::binary_search_by_key`].
            #visibility fn binary_search_by_key<#lt, #new_type, #new_type2>(&#lt self, b: &#new_type2, f: #new_type) -> ::core::result::Result<usize, usize>
            where #new_type: ::core::ops::FnMut(&#lt #enum_ident #enum_generics) -> #new_type2,
                  #new_type2: ::core::cmp::Ord {
                self.#field.binary_search_by_key(b, f)
            }
        }
    }

    /// Generates methods that differ from `Vec`.
    ///
    /// - [`insert`][Vec::insert]: Accepts `Into<Enum>` for the element.
    /// - [`push`][Vec::push]: Accepts `Into<Enum>` for the value.
    fn modified_methods_tokens(&self, nodyn: &NodynEnum) -> TokenStream {
        let field = &self.vec_field;
        let visibility = &self.definition.vis;
        let enum_ident = &nodyn.ident;
        let enum_generics = nodyn.generics_tokens();
        let new_type = nodyn.generics.new_type();

        quote! {
            /// Inserts an element at the specified index, shifting elements as needed.
            /// Accepts `Into<Enum>` for the element.
            /// See [`Vec::insert`].
            #visibility fn insert<#new_type: ::core::convert::Into<#enum_ident #enum_generics>>(&mut self, index: usize, element: #new_type) {
                self.#field.insert(index, element.into());
            }

            /// Appends an element to the end of the vector.
            /// Accepts `Into<Enum>` for the value.
            /// See [`Vec::push`].
            #visibility fn push<#new_type: ::core::convert::Into<#enum_ident #enum_generics>>(&mut self, value: #new_type) {
                self.#field.push(value.into());
            }
        }
    }

    /// Generates methods that require the `PartialEq` trait.
    ///
    /// - [`dedup`][Vec::dedup]: Removes consecutive duplicate elements.
    fn partial_eq_methods_tokens(&self, nodyn: &NodynEnum) -> TokenStream {
        if !is_trait_derived(&nodyn.attrs, "PartialEq") {
            return TokenStream::new();
        }
        let field = &self.vec_field;
        let visibility = &self.definition.vis;

        quote! {
            /// Removes consecutive duplicate elements.
            /// Requires `PartialEq` on the wrapper struct.
            /// See [`Vec::dedup`].
            #visibility fn dedup(&mut self) {
                self.#field.dedup();
            }
        }
    }

    /// Generates trait implementations not depended on other traits.
    ///
    /// - [`From<Self>`][Vec]: Converts to `Vec<Enum>`.
    /// - [`Index`]
    /// - [`IndexMut`]
    /// - [`IntoIterator`] (for `&Self`, `&mut Self`, `Self`)
    /// - [`AsRef<Self>`][AsRef]
    /// - [`AsMut<Self>`][AsMut]
    /// - [`AsRef<Vec<Enum>>`][AsRef]
    /// - [`AsMut<Vec<Enum>>`][AsMut]
    /// - [`AsRef<[Enum]>`][AsRef]
    /// - [`AsMut<[Enum]>`][AsMut]
    /// - [`Extend<Enum>`][Extend] also for each variant
    #[allow(clippy::too_many_lines)]
    fn traits_tokens(&self, nodyn: &NodynEnum) -> TokenStream {
        let ident = &self.definition.ident;
        let enum_ident = &nodyn.ident;
        let field = &self.vec_field;
        let generics = self.generics_tokens(nodyn);
        let where_clause = self.where_tokens(nodyn);
        let enum_generics = nodyn.generics_tokens();
        let new_type = nodyn.generics.new_type();
        let index_g: Generics = parse_quote! {<#new_type>};
        let index_w: WherePredicate = parse_quote! {
            #new_type: ::core::slice::SliceIndex<[#enum_ident #enum_generics]>
        };

        let (index_generics, index_where) = self.merge_generics(nodyn, &index_g, &index_w);

        let lt = &nodyn.generics.new_lifetime();
        let (lt_generics, _) = {
            let index_g: Generics = parse_quote! {<#lt>};
            let index_w: WherePredicate = parse_quote! {W: Clone};
            self.merge_generics(nodyn, &index_g, &index_w)
        };
        let type_generics = self.merged_type_generics_tokens(nodyn);

        let variants = nodyn.variants.iter().map(|variant| {
            let ty = &variant.ty;
            quote!{
                impl #generics ::core::iter::Extend<#ty> for #ident #type_generics #where_clause {
                    fn extend<#new_type: ::core::iter::IntoIterator<Item = #ty>>(&mut self, iter: #new_type) {
                        self.#field.extend(iter.into_iter().map(#enum_ident::from))
                    }
                }
            }
        }).collect::<Vec<_>>();

        let deref = if self.is_custom {
            TokenStream::new()
        } else {
            quote! {
                impl #generics ::core::ops::Deref for #ident #type_generics #where_clause {
                    type Target = [#enum_ident #enum_generics];
                    fn deref(&self) -> &[#enum_ident #enum_generics] {
                        self.as_slice()
                    }
                }

                impl #generics ::core::ops::DerefMut for #ident #type_generics #where_clause {
                    fn deref_mut(&mut self) -> &mut [#enum_ident #enum_generics] {
                        self.as_mut_slice()
                    }
                }
            }
        };

        quote! {
            #deref

            impl #generics ::core::iter::Extend<#enum_ident #enum_generics> for #ident #type_generics #where_clause {
                fn extend<#new_type: ::core::iter::IntoIterator<Item = #enum_ident #enum_generics>>(&mut self, iter: #new_type) {
                    self.#field.extend(iter.into_iter())
                }
            }

            #(#variants)*

            impl #generics ::core::convert::From<#ident #type_generics> for ::std::vec::Vec<#enum_ident #enum_generics> #where_clause {
                fn from(v: #ident #type_generics) -> ::std::vec::Vec<#enum_ident #enum_generics> {
                    v.#field
                }
            }

            impl #index_generics ::core::ops::Index<#new_type> for #ident #type_generics #index_where {
                type Output = #new_type::Output;
                fn index(&self, index: #new_type) -> &Self::Output {
                    &self.#field[index]
                }
            }

            impl #index_generics ::core::ops::IndexMut<#new_type> for #ident #type_generics #index_where {
                fn index_mut(&mut self, index: #new_type) -> &mut Self::Output {
                    &mut self.#field[index]
                }
            }

            impl #lt_generics ::core::iter::IntoIterator for &#lt #ident #type_generics #where_clause {
                type Item = &#lt #enum_ident #enum_generics;
                type IntoIter = ::core::slice::Iter<#lt, #enum_ident #enum_generics>;
                fn into_iter(self) -> Self::IntoIter {
                    self.#field.iter()
                }
            }

            impl #lt_generics ::core::iter::IntoIterator for &#lt mut #ident #type_generics #where_clause {
                type Item = &#lt mut #enum_ident #enum_generics;
                type IntoIter = ::core::slice::IterMut<#lt, #enum_ident #enum_generics>;
                fn into_iter(self) -> Self::IntoIter {
                    self.#field.iter_mut()
                }
            }

            impl #generics ::core::iter::IntoIterator for #ident #type_generics #where_clause {
                type Item = #enum_ident #enum_generics;
                type IntoIter = ::std::vec::IntoIter<#enum_ident #enum_generics>;
                fn into_iter(self) -> Self::IntoIter {
                    self.#field.into_iter()
                }
            }

            impl #generics ::core::convert::AsRef<#ident #type_generics> for #ident #type_generics #where_clause {
                fn as_ref(&self) -> &Self {
                    self
                }
            }

            impl #generics ::core::convert::AsMut<#ident #type_generics> for #ident #type_generics #where_clause {
                fn as_mut(&mut self) -> &mut Self {
                    self
                }
            }

            impl #generics ::core::convert::AsRef<::std::vec::Vec<#enum_ident #enum_generics>> for #ident #type_generics #where_clause {
                fn as_ref(&self) -> &::std::vec::Vec<#enum_ident #enum_generics> {
                    &self.#field
                }
            }

            impl #generics ::core::convert::AsMut<::std::vec::Vec<#enum_ident #enum_generics>> for #ident #type_generics #where_clause {
                fn as_mut(&mut self) -> &mut ::std::vec::Vec<#enum_ident #enum_generics> {
                    &mut self.#field
                }
            }

            impl #generics ::core::convert::AsRef<[#enum_ident #enum_generics]> for #ident #type_generics #where_clause {
                fn as_ref(&self) -> &[#enum_ident #enum_generics] {
                    &self.#field
                }
            }

            impl #generics ::core::convert::AsMut<[#enum_ident #enum_generics]> for #ident #type_generics #where_clause {
                fn as_mut(&mut self) -> &mut [#enum_ident #enum_generics] {
                    &mut self.#field
                }
            }
        }
    }

    /// Generates methods and traits that require `Default`.
    ///
    /// - [`From<Vec<Enum>>`][Vec]
    /// - `From<Vec<T>> where T: Into<enum>` (all variants)
    /// - [`FromIterator<Enum>`][FromIterator]
    /// - [`new`][Vec::new]
    /// - [`with_capacity`][Vec::with_capacity]
    /// - [`split_off`][Vec::split_off]
    fn with_default_tokens(&self, nodyn: &NodynEnum) -> TokenStream {
        if !is_trait_derived(&self.definition.attrs, "Default") {
            return TokenStream::new();
        }
        let field = &self.vec_field;
        let ident = &self.definition.ident;
        let enum_ident = &nodyn.ident;
        let visibility = &self.definition.vis;
        let generics = self.generics_tokens(nodyn);
        let where_clause = self.where_tokens(nodyn);
        let enum_generics = nodyn.generics_tokens();
        let new_type = &nodyn.generics.new_type();
        let default_fields = self.default_fields();
        let type_generics = self.merged_type_generics_tokens(nodyn);
        let variants = nodyn.variants.iter().map(|variant| {
            let ty = &variant.ty;
            quote!{
                impl #generics ::core::convert::From<::std::vec::Vec<#ty>> for #ident #type_generics #where_clause {
                    fn from(v: ::std::vec::Vec<#ty>) -> Self {
                        Self {
                            #field: v.into_iter().map(#enum_ident::from).collect(),
                            #default_fields
                        }
                    }
                }
            }
        }).collect::<Vec<_>>();

        let vec_macro = self.macro_tokens(nodyn);

        quote! {
            impl #generics ::core::convert::From<::std::vec::Vec<#enum_ident #enum_generics>> for #ident #type_generics #where_clause {
                fn from(v: ::std::vec::Vec<#enum_ident #enum_generics>) -> Self {
                    Self {
                        #field: v,
                        #default_fields
                    }
                }
            }

            impl #generics ::core::iter::FromIterator<#enum_ident #enum_generics> for #ident #type_generics #where_clause {
                fn from_iter<#new_type: ::core::iter::IntoIterator<Item = #enum_ident #enum_generics>>(iter: #new_type) -> Self {
                    Self {
                        #field: ::std::vec::Vec::from_iter(iter),
                        #default_fields
                    }
                }
            }

            #(#variants)*

            impl #generics #ident #type_generics #where_clause {
                /// Creates a new empty wrapper.
                /// See [`Vec::new`].
                #visibility fn new() -> Self {
                    Self::default()
                }

                /// Creates a new wrapper with the specified capacity.
                /// See [`Vec::with_capacity`].
                #visibility fn with_capacity(capacity: usize) -> Self {
                    Self {
                        #field: ::std::vec::Vec::with_capacity(capacity),
                        #default_fields
                    }
                }

                /// Splits the wrapper at the given index, returning a new wrapper.
                /// See [`Vec::split_off`].
                #visibility fn split_off(&mut self, at: usize) -> Self {
                    Self {
                        #field: self.#field.split_off(at),
                        #default_fields
                    }
                }
            }

            #vec_macro
        }
    }

    /// Generates traits and methods that require `Clone`.
    ///
    /// - [`resize`][Vec::resize]
    /// - [`extend_from_within`][Vec::extend_from_within]
    /// - [`extend_from_slice`][Vec::extend_from_slice]
    /// - [`clone_from_slice`][Vec::clone_from_slice]
    /// - [`to_vec`][Vec::to_vec]
    /// - [`fill`][Vec::fill]
    fn with_clone_tokens(&self, nodyn: &NodynEnum) -> TokenStream {
        if !is_trait_derived(&nodyn.attrs, "Clone") {
            return TokenStream::new();
        }
        let field = &self.vec_field;
        let ident = &self.definition.ident;
        let visibility = &self.definition.vis;
        let generics = self.generics_tokens(nodyn);
        let where_clause = self.where_tokens(nodyn);
        let enum_ident = &nodyn.ident;
        let enum_generics = nodyn.generics_tokens();
        let new_type = &nodyn.generics.new_type();
        let type_generics = self.merged_type_generics_tokens(nodyn);

        quote! {
            impl #generics #ident #type_generics #where_clause {
                /// Resizes the vector to the new length, using the provided value.
                /// Accepts `Into<Enum>` for the value.
                /// See [`Vec::resize`].
                #visibility fn resize<#new_type: ::core::convert::Into<#enum_ident #enum_generics>>(&mut self, new_len: usize, value: #new_type) {
                    self.#field.resize(new_len, value.into());
                }

                /// Copies elements from a range within the vector.
                /// See [`Vec::extend_from_within`].
                #visibility fn extend_from_within<#new_type>(&mut self, src: #new_type)
                where #new_type: ::core::ops::RangeBounds<usize> {
                    self.#field.extend_from_within(src);
                }

                /// Extends the vector with a copy of the slice.
                /// See [`Vec::extend_from_slice`].
                #visibility fn extend_from_slice(&mut self, other: &[#enum_ident #enum_generics]) {
                    self.#field.extend_from_slice(other);
                }

                /// Copies the elements from src into self.
                /// See [`Vec::clone_from_slice`].
                #visibility fn clone_from_slice(&mut self, other: &[#enum_ident #enum_generics]) {
                    self.#field.clone_from_slice(other);
                }

                /// Copies self into a new Vec.
                /// See [`Vec::clone_from_slice`].
                #visibility fn to_vec(&self) -> Vec<#enum_ident #enum_generics> {
                    self.#field.to_vec()
                }

                /// Fills self with elements by cloning value.
                /// Accepts `Into<Enum>` for the value.
                /// See [`Vec::fill`].
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
        if !is_trait_derived(&nodyn.attrs, "Clone")
            || !is_trait_derived(&self.definition.attrs, "Default")
        {
            return TokenStream::new();
        }
        let field = &self.vec_field;
        let ident = &self.definition.ident;
        let enum_ident = &nodyn.ident;
        let generics = self.generics_tokens(nodyn);
        let where_clause = self.where_tokens(nodyn);
        let enum_generics = nodyn.generics_tokens();
        let default_fields = self.default_fields();
        let type_generics = self.merged_type_generics_tokens(nodyn);

        let variants = nodyn.variants.iter().map(|variant| {
            let ty = &variant.ty;
            quote!{
                impl #generics ::core::convert::From<&[#ty]> for #ident #type_generics #where_clause {
                    fn from(s: &[#ty]) -> #ident #type_generics {
                        Self {
                           #field: s.iter().cloned().map(#enum_ident::from).collect(),
                           #default_fields
                        }
                    }
                }

                impl #generics ::core::convert::From<&mut [#ty]> for #ident #type_generics #where_clause {
                    fn from(s: &mut [#ty]) -> #ident #type_generics {
                        Self {
                           #field: s.iter().cloned().map(#enum_ident::from).collect(),
                           #default_fields
                        }
                    }
                }
            }
        }).collect::<Vec<_>>();

        quote! {
            impl #generics ::core::convert::From<&[#enum_ident #enum_generics]> for #ident #type_generics #where_clause {
                fn from(s: &[#enum_ident #enum_generics]) -> #ident #type_generics {
                    Self {
                       #field: s.to_vec(),
                        #default_fields
                    }
                }
            }

            impl #generics ::core::convert::From<&mut [#enum_ident #enum_generics]> for #ident #type_generics #where_clause {
                fn from(s: &mut [#enum_ident #enum_generics]) -> #ident #type_generics {
                    Self {
                       #field: s.to_vec(),
                       #default_fields
                    }
                }
            }

            #(#variants)*
        }
    }

    /// Generates methods that require the enum to have `#[derive(PartialOrd)]`;
    /// - [`is_sorted`][Vec::is_sorted]
    fn with_partial_ord_tokens(&self, nodyn: &NodynEnum) -> TokenStream {
        if !is_trait_derived(&nodyn.attrs, "PartialOrd") {
            return TokenStream::new();
        }
        let field = &self.vec_field;
        let ident = &self.definition.ident;
        let generics = self.generics_tokens(nodyn);
        let where_clause = self.where_tokens(nodyn);
        let visibility = &self.definition.vis;
        let type_generics = self.merged_type_generics_tokens(nodyn);

        quote! {
            /// Checks if the elements of this slice are sorted.
            /// See [`Vec::is_sorted`].
            impl #generics #ident #type_generics #where_clause {
                #visibility fn is_sorted(&self) -> bool {
                    self.#field.is_sorted()
                }
            }
        }
    }

    /// Generates methods that require the enum to have `#[derive(Ord)]`;
    ///
    /// - [`sort`][Vec::sort]
    /// - [`sort_unstable`][Vec::sort_unstable]
    /// - [`binary_search`][Vec::binary_search]
    fn with_ord_tokens(&self, nodyn: &NodynEnum) -> TokenStream {
        if !is_trait_derived(&nodyn.attrs, "Ord") {
            return TokenStream::new();
        }
        let field = &self.vec_field;
        let ident = &self.definition.ident;
        let generics = self.generics_tokens(nodyn);
        let where_clause = self.where_tokens(nodyn);
        let visibility = &self.definition.vis;
        let enum_ident = &nodyn.ident;
        let enum_generics = nodyn.generics_tokens();
        let type_generics = self.merged_type_generics_tokens(nodyn);

        quote! {
            /// Sorts the slice, preserving initial order of equal elements.
            /// See [`Vec::sort()`].
            impl #generics #ident #type_generics #where_clause {
                #visibility fn sort(&mut self) {
                    self.#field.sort();
                }
            }

            /// Sorts the slice without preserving the initial order of equal elements.
            /// See [`Vec::sort_unstable()`].
            impl #generics #ident #type_generics #where_clause {
                #visibility fn sort_unstable(&mut self) {
                    self.#field.sort_unstable();
                }
            }

            /// Binary searches this slice for a given element.
            /// See [`Vec::binary_search()`].
            impl #generics #ident #type_generics #where_clause {
                #visibility fn binary_search(&mut self, x: &#enum_ident #enum_generics) -> ::core::result::Result<usize, usize> {
                    self.#field.binary_search(x)
                }
            }
        }
    }

    /// Generates methods that require the enum to have `#[derive(Copy)]`;
    /// - [`copy_from_slice`][Vec::copy_from_slice]
    /// - [`copy_within`][Vec::copy_within]
    fn with_copy_tokens(&self, nodyn: &NodynEnum) -> TokenStream {
        if !is_trait_derived(&nodyn.attrs, "Copy") {
            return TokenStream::new();
        }
        let field = &self.vec_field;
        let ident = &self.definition.ident;
        let generics = self.generics_tokens(nodyn);
        let where_clause = self.where_tokens(nodyn);
        let visibility = &self.definition.vis;
        let enum_ident = &nodyn.ident;
        let enum_generics = nodyn.generics_tokens();
        let new_type = &nodyn.generics.new_type();
        let type_generics = self.merged_type_generics_tokens(nodyn);

        quote! {
            impl #generics #ident #type_generics #where_clause {
                /// Copies all elements from src into self, using a memcpy.
                /// See [`Vec::copy_from_slice`].
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

    fn generics_tokens(&self, nodyn: &NodynEnum) -> TokenStream {
        if self.is_custom {
            self.definition
                .generics
                .merged_generics_tokens(&nodyn.generics)
        } else {
            nodyn.generics_tokens()
        }
    }

    fn merged_type_generics_tokens(&self, nodyn: &NodynEnum) -> TokenStream {
        if self.is_custom {
            self.definition
                .generics
                .merged_type_generics_tokens(&nodyn.generics)
        } else {
            nodyn.generics.type_generics_tokens()
        }
    }

    fn where_tokens(&self, nodyn: &NodynEnum) -> TokenStream {
        if self.is_custom {
            nodyn.merged_where_tokens(&self.definition.generics)
        } else {
            nodyn.generics.where_clause.to_token_stream()
        }
    }

    fn merge_generics(
        &self,
        nodyn: &NodynEnum,
        extra1: &Generics,
        extra2: &WherePredicate,
    ) -> (TokenStream, TokenStream) {
        if self.is_custom {
            (
                // nodyn.merged_generics_and_param_tokens(&self.definition.generics, extra1),
                self.definition
                    .generics
                    .merged2_generics_tokens(&nodyn.generics, extra1),
                nodyn.merged_where_and_predicate_tokens(&self.definition.generics, extra2),
            )
        } else {
            (
                nodyn.generics.merged_generics_tokens(extra1),
                nodyn.where_and_predicate_tokens(extra2),
            )
        }
    }

    fn default_fields(&self) -> TokenStream {
        if self.is_custom {
            quote! { .. ::core::default::Default::default() }
        } else {
            TokenStream::new()
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

fn is_trait_derived(attributes: &[Attribute], trait_name: &str) -> bool {
    let parser = Punctuated::<Ident, Token![,]>::parse_terminated;
    for attr in attributes {
        if let Meta::List(list) = &attr.meta {
            if list.path.is_ident("derive") {
                if let Ok(idents) = parser.parse(list.tokens.clone().into()) {
                    for id in idents {
                        if id == trait_name {
                            return true;
                        }
                    }
                }
            }
        }
    }
    false
}
