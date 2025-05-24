use syn::{Generics, Ident, parse::Parse};

use crate::ImplBlock;

#[allow(dead_code)]
#[derive(Debug, Clone)]
pub(crate) struct TraitBlock {
    pub(crate) ident: Ident,
    pub(crate) generics: Generics,
    pub(crate) block: ImplBlock,
}

impl TraitBlock {
    #[allow(dead_code)]
    pub(crate) const fn new(ident: Ident, generics: Generics, block: ImplBlock) -> Self {
        Self {
            ident,
            generics,
            block,
        }
    }
}

impl Parse for TraitBlock {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let _keyword = input.parse::<syn::token::Trait>()?;
        let ident = input.parse::<Ident>()?;
        let generics = input.parse::<Generics>()?;

        let block = input.parse::<ImplBlock>()?;
        Ok(Self {
            ident,
            generics,
            block,
        })
    }
}
