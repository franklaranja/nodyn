use proc_macro2::TokenStream;
use quote::quote;
use syn::Ident;

use crate::NodynEnum;

pub(crate) struct VecBuilder;

impl VecBuilder {
    pub(crate) fn wrapper_struct(nodyn: &NodynEnum, ident: &Ident) -> TokenStream {
        let pound = syn::token::Pound::default();
        let vis = &nodyn.visibility;
        let generics = &nodyn.generics;
        let enum_ident = &nodyn.ident;
        quote! {
            #pound [derive(Debug, Default)]
            #vis struct #ident #generics {
                inner: std::vec::Vec< #enum_ident #generics >,
            }
        }
    }
}
