use syn::{
    Attribute, ImplItem, Signature, Token, Visibility,
    parse::{Parse, Parser},
    parse2,
};

#[allow(dead_code)]
#[derive(Debug, Clone)]
pub(crate) struct Function {
    pub(crate) items: Vec<syn::ImplItem>,
}

// pub enum ImplItem {
//     Const(ImplItemConst),
//     Fn(ImplItemFn),
//     Type(ImplItemType),
//     Macro(ImplItemMacro),
//     Verbatim(TokenStream),
// }

// pub enum TraitItem {
//     Const(TraitItemConst),
//     Fn(TraitItemFn),
//     Type(TraitItemType),
//     Macro(TraitItemMacro),
//     Verbatim(TokenStream),
// }

impl Parse for Function {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let _keyword = input.parse::<syn::token::Impl>()?;
        let content;

        let mut items = Vec::new();
        // Convert TokenStream to ParseBuffer
        let content;
        let _brace_token = syn::braced!(content in input);
        while !content.is_empty() {
            let item = content.parse::<syn::ImplItem>()?;
            if let ImplItem::Verbatim(ts) = item {
        let parse_buffer: syn::parse::ParseBuffer = parse2(ts)?;
                let parser = Attribute::parse_outer;
                // let attrs = parser.parse2(ts)?;
                // let vis = parse2::<Visibility>(ts)?;
                // let defaultness = parse2::<Token![default]>(ts)?;
                // let sig = parse2::<Signature>(ts)?;
                let mut ts = ts; // Clone or work with the token stream
                let attrs = Attribute::parse_outer(&mut ts)?; // Parse outer attributes
                let vis = parse2::<Visibility>(ts.clone())?; // Parse visibility
                let defaultness = parse2::<Token![default]>(ts.clone()).ok(); // Optional defaultness
                let sig = parse2::<Signature>(ts)?; // Parse signature
            }
        }
        println!("=========\n{items:#?}\n========");
        Ok(Self { items })
    }
}
