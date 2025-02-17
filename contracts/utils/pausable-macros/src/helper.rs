use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, FnArg, ItemFn, PatType, Type};

pub fn generate_pause_check(item: TokenStream, check_fn: &str) -> TokenStream {
    let input_fn = parse_macro_input!(item as ItemFn);
    let (env_ident, is_ref) = check_env_arg(&input_fn);

    let fn_vis = &input_fn.vis;
    let fn_sig = &input_fn.sig;
    let fn_block = &input_fn.block;
    let fn_attrs = &input_fn.attrs;

    let env_arg = if is_ref {
        quote! { #env_ident }
    } else {
        quote! { &#env_ident }
    };

    let check_ident = syn::Ident::new(check_fn, proc_macro2::Span::call_site());
    let output = quote! {
        #(#fn_attrs)* // retain other macros
        #fn_vis #fn_sig {
            openzeppelin_pausable::#check_ident(#env_arg);

            #fn_block
        }
    };

    output.into()
}

fn check_env_arg(input_fn: &ItemFn) -> (syn::Ident, bool) {
    // Get the first argument
    let first_arg = input_fn.sig.inputs.first().unwrap_or_else(|| {
        panic!("function '{}' must have at least one argument", input_fn.sig.ident)
    });

    // Extract the pattern and type from the argument
    let (pat, ty) = match first_arg {
        FnArg::Typed(PatType { pat, ty, .. }) => (pat, ty),
        _ =>
            panic!("first argument of function '{}' must be a typed parameter", input_fn.sig.ident),
    };

    // Get the identifier from the pattern
    let ident = match &**pat {
        syn::Pat::Ident(pat_ident) => pat_ident.ident.clone(),
        _ => panic!("first argument of function '{}' must be an identifier", input_fn.sig.ident),
    };

    // Check if the type is Env or &Env
    let is_ref = match &**ty {
        Type::Reference(type_ref) => match &*type_ref.elem {
            Type::Path(path) => {
                check_is_env(path, &input_fn.sig.ident);
                true
            }
            _ => panic!("first argument of function '{}' must be Env or &Env", input_fn.sig.ident),
        },
        Type::Path(path) => {
            check_is_env(path, &input_fn.sig.ident);
            false
        }
        _ => panic!("first argument of function '{}' must be Env or &Env", input_fn.sig.ident),
    };

    (ident, is_ref)
}

fn check_is_env(path: &syn::TypePath, fn_name: &syn::Ident) {
    let is_env = path.path.segments.last().map(|seg| seg.ident == "Env").unwrap_or(false);

    if !is_env {
        panic!("first argument of function '{}' must be Env or &Env", fn_name);
    }
}
