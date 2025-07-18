use crate::parse_env_arg;

use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, ItemFn};

pub fn generate_pause_check(item: TokenStream, check_fn: &str) -> TokenStream {
    let input_fn = parse_macro_input!(item as ItemFn);
    let env_arg = parse_env_arg(&input_fn);

    let fn_vis = &input_fn.vis;
    let fn_sig = &input_fn.sig;
    let fn_block = &input_fn.block;
    let fn_attrs = &input_fn.attrs;

    let check_ident = syn::Ident::new(check_fn, proc_macro2::Span::call_site());
    let output = quote! {
        #(#fn_attrs)* // retain other macros
        #fn_vis #fn_sig {
            stellar_pausable::#check_ident(#env_arg);

            #fn_block
        }
    };

    output.into()
}
