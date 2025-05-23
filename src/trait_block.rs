use syn::parse::Parse;

#[allow(dead_code)]
#[derive(Debug, Clone)]
pub(crate) struct TraitBlock {
    pub(crate) ident: syn::Ident,
    pub(crate) generics: syn::Generics,
    pub(crate) items: Vec<syn::TraitItem>,
}

impl TraitBlock {
    #[allow(dead_code)]
    pub(crate) fn new(
        ident: syn::Ident,
        generics: syn::Generics,
        items: Vec<syn::TraitItem>,
    ) -> Self {
        Self {
            ident,
            generics,
            items,
        }
    }
}

impl Parse for TraitBlock {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let _keyword = input.parse::<syn::token::Trait>()?;
        let ident = input.parse::<syn::Ident>()?;
        let generics = input.parse::<syn::Generics>()?;

        let content;

        let _brace_token = syn::braced!(content in input);

        let mut items = Vec::new();

        while !content.is_empty() {
            items.push(content.parse::<syn::TraitItem>()?);
        }
        Ok(Self {
            ident,
            generics,
            items,
        })
    }
}
