use syn::{FnArg, ItemFn, PatType, Type};

pub fn check_env_arg(input_fn: &ItemFn) -> (syn::Ident, bool) {
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
