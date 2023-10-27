use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, parse_quote, ImplItem, ImplItemFn, ItemImpl, Stmt};

#[proc_macro_attribute]
pub fn near_bindgen(args: TokenStream, input: TokenStream) -> TokenStream {
    let mut out = TokenStream::new();
    let bindgen: TokenStream = quote! { #[near_sdk::near_bindgen] }.into();
    out.extend(bindgen);
    if cfg!(coverage) {
        let coverage: TokenStream = quote! { #[hacken_cov::near_coverage] }.into();
        out.extend(coverage);
    }
    out.extend(input);
    out
}

#[proc_macro_attribute]
pub fn near_coverage(attr: TokenStream, item: TokenStream) -> TokenStream {
    let mut input = parse_macro_input!(item as ItemImpl);

    let code_block: Vec<Stmt> = parse_quote! {
        let mut coverage = vec![];
        unsafe {
            // Note that this function is not thread-safe! Use a lock if needed.
            minicov::capture_coverage(&mut coverage).unwrap();
        };
        near_sdk::env::log_str();
    };

    for item in &mut input.items {
        if let ImplItem::Fn(ImplItemFn { block, .. }) = item {
            let temp = block.stmts.pop();
            block.stmts.extend(code_block.clone());
            block.stmts.push(temp.unwrap());
        }
    }

    TokenStream::from(quote! { #input })
}
