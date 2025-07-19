use syn::{parse::Parse, Token};

pub(crate) mod keyword {
    syn::custom_keyword!(From);
    syn::custom_keyword!(TryInto);
    syn::custom_keyword!(is_as);
    syn::custom_keyword!(introspection);
}

#[allow(clippy::struct_excessive_bools)]
#[derive(Debug, Copy, Clone, Default)]
pub(crate) struct Features {
    pub(crate) from: bool,
    pub(crate) try_into: bool,
    pub(crate) is_as: bool,
    pub(crate) introspection: bool,
}

impl Features {
    pub(crate) const fn merge(&mut self, other: Self) {
        if other.from {
            self.from = true;
        }
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
        !self.from && !self.try_into && !self.is_as && !self.introspection
    }
}

impl Parse for Features {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let mut features = Self::default();
        loop {
            if input.peek(keyword::From) {
                let _ = input.parse::<keyword::From>()?;
                features.from = true;
            } else if input.peek(keyword::TryInto) {
                let _ = input.parse::<keyword::TryInto>()?;
                features.try_into = true;
            } else if input.peek(keyword::is_as) {
                let _ = input.parse::<keyword::is_as>()?;
                features.is_as = true;
            } else if input.peek(keyword::introspection) {
                let _ = input.parse::<keyword::introspection>()?;
                features.introspection = true;
            } else {
                break;
            }
        }
        if input.peek(Token![;]) {
            let _ = input.parse::<syn::token::Semi>()?;
        }
        Ok(features)
    }
}
