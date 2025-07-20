use proc_macro2::TokenStream;
use syn::{Generics, Ident, Visibility};

pub(crate) struct VecBuilder;

impl VecBuilder {
    pub(crate) fn wrapper_struct(
        vis: &Visibility,
        ident: &Ident,
        enum_ident: &Ident,
        generics: &Generics,
    ) -> TokenStream {
        quote::quote! {
            #vis struct #ident #generics {
                inner: std::vec::Vec< #enum_ident #generics >,
            }
        }
    }
}
