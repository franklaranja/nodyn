use syn::{Path, parse::Parse};

use crate::MethodImpl;

#[allow(dead_code)]
#[derive(Debug, Clone)]
pub(crate) struct TraitImpl {
    pub(crate) path: Path,
    pub(crate) block: MethodImpl,
}

impl Parse for TraitImpl {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        Ok(Self {
            path: input.parse::<Path>()?,
            block: input.parse::<MethodImpl>()?,
        })
    }
}
