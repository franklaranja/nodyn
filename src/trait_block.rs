use syn::{Path, parse::Parse};

use crate::ImplBlock;

#[allow(dead_code)]
#[derive(Debug, Clone)]
pub(crate) struct TraitBlock {
    pub(crate) path: Path,
    pub(crate) block: ImplBlock,
}

impl Parse for TraitBlock {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        Ok(Self {
            path: input.parse::<Path>()?,
            block: input.parse::<ImplBlock>()?,
        })
    }
}
