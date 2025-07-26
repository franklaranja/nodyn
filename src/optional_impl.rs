use syn::{Token, parse::Parse};

use crate::keyword;

// #[allow(clippy::struct_excessive_bools)]
#[derive(Debug, Copy, Clone, Default)]
pub(crate) struct OptionalImpl {
    pub(crate) try_into: bool,
    pub(crate) is_as: bool,
    pub(crate) introspection: bool,
}

impl OptionalImpl {
    pub(crate) const fn merge(&mut self, other: Self) {
        if other.try_into {
            self.try_into = true;
        }
        if other.is_as {
            self.is_as = true;
        }
        if other.introspection {
            self.introspection = true;
        }
    }

    pub(crate) const fn none(self) -> bool {
        !self.try_into && !self.is_as && !self.introspection
    }
}

impl Parse for OptionalImpl {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let mut optional = Self::default();
        loop {
            if input.peek(keyword::TryInto) {
                let _ = input.parse::<keyword::TryInto>()?;
                optional.try_into = true;
            } else if input.peek(keyword::is_as) {
                let _ = input.parse::<keyword::is_as>()?;
                optional.is_as = true;
            } else if input.peek(keyword::introspection) {
                let _ = input.parse::<keyword::introspection>()?;
                optional.introspection = true;
            } else {
                break;
            }
        }
        if input.peek(Token![;]) {
            let _ = input.parse::<syn::token::Semi>()?;
        }
        Ok(optional)
    }
}
