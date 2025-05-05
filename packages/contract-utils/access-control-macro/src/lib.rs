use proc_macro::TokenStream;
use quote::quote;
use syn::{
    parse_macro_input, AttributeArgs, FnArg, ItemFn, Lit, Meta, NestedMeta, Pat, PatIdent, PatType,
    Type,
};

/// A procedural macro that ensures the parameter has the specified role.
///
/// # Usage
///
/// ```rust
/// #[has_role(account, "minter")]
/// pub fn mint_tokens(e: &Env, amount: u32, account: Address) {
///     // Function body
/// }
/// ```
///
/// This will expand to:
///
/// ```rust
/// pub fn mint_tokens(e: &Env, amount: u32, account: Address) {
///     stellar_access_control::ensure_role(e, &account, &soroban_sdk::Symbol::new(e, "minter"));
///     // Function body
/// }
/// ```
#[proc_macro_attribute]
pub fn has_role(args: TokenStream, input: TokenStream) -> TokenStream {
    let args = parse_macro_input!(args as AttributeArgs);
    let input_fn = parse_macro_input!(input as ItemFn);

    let (param_name, role_str) = parse_args(&args);
    let is_ref_param = validate_param_type(&input_fn, &param_name);

    let param_reference = if is_ref_param {
        quote! { #param_name }
    } else {
        quote! { &#param_name }
    };

    let fn_attrs = &input_fn.attrs;
    let fn_vis = &input_fn.vis;
    let fn_sig = &input_fn.sig;
    let fn_block = &input_fn.block;

    let expanded = quote! {
        #(#fn_attrs)*
        #fn_vis #fn_sig {
            stellar_access_control::ensure_role(e, #param_reference, &soroban_sdk::Symbol::new(e, #role_str));
            #fn_block
        }
    };

    TokenStream::from(expanded)
}

fn parse_args(args: &AttributeArgs) -> (&syn::Ident, String) {
    match args.as_slice() {
        [NestedMeta::Meta(Meta::Path(param_path)), NestedMeta::Lit(Lit::Str(role))] => {
            let param_name = param_path.get_ident().expect("Parameter name must be an identifier");
            (param_name, role.value())
        }
        _ => panic!("Expected #[has_role(param_name, \"role\")]"),
    }
}

fn validate_param_type(func: &ItemFn, param_name: &syn::Ident) -> bool {
    for arg in &func.sig.inputs {
        if let FnArg::Typed(PatType { pat, ty, .. }) = arg {
            if let Pat::Ident(PatIdent { ident, .. }) = &**pat {
                if ident == param_name {
                    return match_address_type(ty, param_name);
                }
            }
        }
    }
    panic!("Parameter `{}` not found in function signature", param_name);
}

fn match_address_type(ty: &Box<Type>, param_name: &syn::Ident) -> bool {
    match &**ty {
        Type::Reference(TypeReference { elem, .. }) =>
            match_path_is_address(elem, param_name, true),
        Type::Path(_) => match_path_is_address(ty, param_name, false),
        _ => panic_type(param_name),
    }
}

fn match_path_is_address(ty: &Type, param_name: &syn::Ident, is_ref: bool) -> bool {
    if let Type::Path(TypePath { path, .. }) = ty {
        if path.segments.last().map(|s| s.ident == "Address").unwrap_or(false) {
            return is_ref;
        }
    }
    panic_type(param_name);
}

fn panic_type(param_name: &syn::Ident) -> ! {
    panic!("Parameter `{}` must be of type `Address` or `&Address`", param_name);
}
