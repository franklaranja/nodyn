use proc_macro2::{Delimiter, Group, TokenStream, TokenTree};
use syn::{ImplItem, parse::Parse, parse2};

#[allow(dead_code)]
#[derive(Debug, Clone)]
pub(crate) struct ImplBlock {
    pub(crate) items: Vec<syn::ImplItem>,
    pub(crate) functions: Vec<syn::ImplItemFn>,
}

impl Parse for ImplBlock {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let _keyword = input.parse::<syn::token::Impl>()?;

        let mut items = Vec::new();
        let mut functions = Vec::new();
        let content;
        let _brace_token = syn::braced!(content in input);
        while !content.is_empty() {
            let item = content.parse::<syn::ImplItem>()?;

            // Verbatim items are assumed to be trait like
            // functions without a body and ending with a semicolon
            if let ImplItem::Verbatim(ts) = item {
                // replace the semicolon with braces so it can
                // be parsed as an ImplItemFn
                let ts: TokenStream = ts
                    .into_iter()
                    .map(|tt| {
                        if &tt.to_string() == ";" {
                            TokenTree::Group(Group::new(Delimiter::Brace, TokenStream::new()))
                        } else {
                            tt
                        }
                    })
                    .collect();
                functions.push(parse2::<syn::ImplItemFn>(ts)?);
            } else {
                items.push(item);
            }
        }
        Ok(Self { items, functions })
    }
}
