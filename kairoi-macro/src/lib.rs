use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, ItemFn};

#[proc_macro_attribute]
pub fn instrument(_attr: TokenStream, item: TokenStream) -> TokenStream {
    let input_fn = parse_macro_input!(item as ItemFn);

    let vis = &input_fn.vis;
    let sig = &input_fn.sig;
    let block = &input_fn.block;
    let fn_name = &sig.ident;
    let fn_name_str = fn_name.to_string();

    if sig.asyncness.is_none() {
        let msg = "instrument macro can only be used on async functions";
        return syn::Error::new_spanned(sig, msg).to_compile_error().into();
    }

    let output = quote! {
        #vis #sig {
            let span_data = kairoi::Span::default().with_name(#fn_name_str.to_string());
            kairoi::Span::scope(async move |scope| {
                scope.update(span_data);
                #block
            }).await
        }
    };

    TokenStream::from(output)
}
