use proc_macro::TokenStream;
use proc_macro2::Span;
use quote::{quote, ToTokens};
use syn::{parse_macro_input, FnArg, ImplItem, ItemImpl, Pat, PatType, Visibility};

/// A procedural macro that adds a standard init function to a facet contract implementation.
///
/// This macro can be applied to the impl block of a facet contract.
///
/// When applied to an impl block, it adds standard initialization and storage access functions,
/// plus security validation to ensure only the diamond proxy can call admin functions.
///
/// # Example
///
/// ```ignore
/// use soroban_sdk::{contract, contractimpl, Address, Env, Symbol};
/// use stellar_diamond_proxy_core::{storage::Storage, Error};
/// use stellar_facet_macro::facet;
///
/// #[contract]
/// pub struct MyFacet;
///
/// #[facet] // This MUST go BEFORE #[contract_impl]
/// #[contractimpl]
/// impl MyFacet {
///     // The init function will be automatically generated here
///     
///     // Other contract methods...
/// }
/// ```
#[proc_macro_attribute]
pub fn facet(attr: TokenStream, item: TokenStream) -> TokenStream {
    // Check if attr is empty, we don't expect any attributes for this macro
    if !attr.is_empty() {
        return syn::Error::new(
            Span::call_site(),
            "The facet macro doesn't accept any arguments",
        )
        .to_compile_error()
        .into();
    }

    // If not a struct, parse as impl
    let mut input = parse_macro_input!(item as ItemImpl);

    if uses_reserved_function(&input) {
        return syn::Error::new(
            Span::call_site(),
            "Facets cannot have pre-made or custom init functions",
        )
        .to_compile_error()
        .into();
    }

    // Generate the modified init function with shared_storage_address parameter
    let init_fn = syn::parse2::<ImplItem>(quote! {
        pub fn init(
            env: soroban_sdk::Env,
            owner: soroban_sdk::Address,
            shared_storage_address: soroban_sdk::Address,
            diamond_proxy_address: soroban_sdk::Address,
        ) -> Result<(), stellar_diamond_proxy_core::Error> {
            let storage = stellar_diamond_proxy_core::storage::Storage::new(env.clone());
            storage.require_uninitialized();
            storage.set_initialized();
            storage.set_owner(&owner);
            storage.set_shared_storage_address(&shared_storage_address);

            // SECURITY: Store the diamond proxy address for authorization validation
            storage.set_diamond_proxy_address(&diamond_proxy_address);


            Ok(())
        }
    })
    .expect("Failed to parse generated init function");

    // Add security validation helper function
    let security_fn = syn::parse2::<ImplItem>(quote! {
        /// Security check: Ensure only the diamond proxy can call admin functions
        /// This prevents direct calls to facet contracts, which would bypass authorization
        pub fn require_diamond_proxy_caller(env: &soroban_sdk::Env) -> Result<(), stellar_diamond_proxy_core::Error> {
            let storage = stellar_diamond_proxy_core::storage::Storage::new(env.clone());
            // Get the diamond proxy address that was set during initialization
            let diamond_proxy = storage.get_diamond_proxy_address()
                .ok_or(stellar_diamond_proxy_core::Error::DiamondProxyNotSet)?;
            // Require auth on the diamond proxy with the current contract (facet) address as argument
            // This will only succeed if the diamond proxy's fallback function authorized this exact call
            let current_address = env.current_contract_address();
            let args: soroban_sdk::Vec<soroban_sdk::Val> = soroban_sdk::vec![env, current_address.to_val()];
            diamond_proxy.require_auth_for_args(args);
            Ok(())
        }
    })
    .expect("Failed to parse generated security function");

    let owner_fn = syn::parse2::<ImplItem>(quote! {
        fn owner(env: &soroban_sdk::Env) -> Result<soroban_sdk::Address, stellar_diamond_proxy_core::Error> {
            let storage = stellar_diamond_proxy_core::storage::Storage::new(env.clone());
            storage.get_owner().ok_or(stellar_diamond_proxy_core::Error::OwnerNotSet)
        }
    })
    .expect("Failed to parse generated owner function");

    // Add security checks to all public functions (except init and require_diamond_proxy_caller)
    modify_public_functions(&mut input);

    // Add the generated functions to the impl block
    input.items.insert(0, init_fn);
    input.items.insert(1, security_fn);
    input.items.insert(2, owner_fn);

    // Return the modified impl block
    TokenStream::from(input.to_token_stream())
}

// Helper function to modify all public functions to add security checks
fn modify_public_functions(impl_block: &mut ItemImpl) {
    let mut modified_items = Vec::new();

    for item in impl_block.items.iter() {
        if let ImplItem::Fn(method) = item {
            let fn_name = method.sig.ident.to_string();

            // Skip reserved functions and private functions
            if fn_name == "init"
                || fn_name == "require_diamond_proxy_caller"
                || fn_name == "owner"
                || !matches!(method.vis, Visibility::Public(_))
            {
                modified_items.push(item.clone());
                continue;
            }

            // For public functions, inject security check at the beginning
            let mut modified_method = method.clone();

            // Extract the environment parameter (usually the first parameter)
            let env_ident =
                if let Some(FnArg::Typed(PatType { pat, .. })) = method.sig.inputs.first() {
                    if let Pat::Ident(pat_ident) = &**pat {
                        Some(&pat_ident.ident)
                    } else {
                        None
                    }
                } else {
                    None
                };

            // Check if function returns a Result
            let returns_result = if let syn::ReturnType::Type(_, ref ty) = method.sig.output {
                if let syn::Type::Path(type_path) = &**ty {
                    type_path
                        .path
                        .segments
                        .first()
                        .map(|seg| seg.ident == "Result")
                        .unwrap_or(false)
                } else {
                    false
                }
            } else {
                false
            };

            // If we found an env parameter, inject the security check
            if let Some(env) = env_ident {
                let original_block = &method.block;

                // Different handling for Result vs non-Result return types
                if returns_result {
                    let security_check = quote! {
                        // SECURITY: Verify this call is authorized through the diamond proxy
                        Self::require_diamond_proxy_caller(&#env)?;
                    };

                    modified_method.block = syn::parse2(quote! {
                        {
                            #security_check
                            #original_block
                        }
                    })
                    .expect("Failed to parse modified function block");
                } else {
                    // For non-Result types, we need to handle the error differently
                    let security_check = quote! {
                        // SECURITY: Verify this call is authorized through the diamond proxy
                        if let Err(_) = Self::require_diamond_proxy_caller(&#env) {
                            #env.panic_with_error(stellar_diamond_proxy_core::Error::UnauthorizedDirectCall);
                        }
                    };

                    modified_method.block = syn::parse2(quote! {
                        {
                            #security_check
                            #original_block
                        }
                    })
                    .expect("Failed to parse modified function block");
                }
            }

            modified_items.push(ImplItem::Fn(modified_method));
        } else {
            modified_items.push(item.clone());
        }
    }

    impl_block.items = modified_items;
}

// Helper function to check if the impl block already has an init function
fn uses_reserved_function(impl_block: &ItemImpl) -> bool {
    const RESERVED_FUNCTIONS: [&str; 3] = ["init", "require_diamond_proxy_caller", "owner"];
    for item in &impl_block.items {
        if let ImplItem::Fn(method) = item {
            let ident = method.sig.ident.to_string();
            if RESERVED_FUNCTIONS.contains(&ident.as_str()) {
                return true;
            }
        }
    }
    false
}
