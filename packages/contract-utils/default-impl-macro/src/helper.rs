use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, ItemImpl};

fn get_default_methods(trait_name: &str) -> Vec<syn::ImplItem> {
    match trait_name {
        "FungibleToken" => vec![
            syn::parse_quote! { fn total_supply(e: &Env) -> i128 { stellar_fungible::total_supply(e) } },
            syn::parse_quote! { fn balance(e: &Env, account: Address) -> i128 { stellar_fungible::balance(e, &account) } },
            syn::parse_quote! { fn allowance(e: &Env, owner: Address, spender: Address) -> i128 { stellar_fungible::allowance(e, &owner, &spender) } },
            syn::parse_quote! { fn transfer(e: &Env, from: Address, to: Address, amount: i128) { stellar_fungible::transfer(e, &from, &to, amount); } },
            syn::parse_quote! { fn transfer_from(e: &Env, spender: Address, from: Address, to: Address, amount: i128) { stellar_fungible::transfer_from(e, &spender, &from, &to, amount); } },
            syn::parse_quote! { fn approve(e: &Env, owner: Address, spender: Address, amount: i128, live_until_ledger: u32) { stellar_fungible::approve(e, &owner, &spender, amount, live_until_ledger); } },
            syn::parse_quote! { fn decimals(e: &Env) -> u32 { stellar_fungible::metadata::decimals(e) } },
            syn::parse_quote! { fn name(e: &Env) -> String { stellar_fungible::metadata::name(e) } },
            syn::parse_quote! { fn symbol(e: &Env) -> String { stellar_fungible::metadata::symbol(e) } },
        ],
        "FungibleBurnable" => vec![
            syn::parse_quote! { fn burn(e: &Env, from: Address, amount: i128) { stellar_fungible::burnable::burn(e, &from, amount); } },
            syn::parse_quote! { fn burn_from(e: &Env, spender: Address, from: Address, amount: i128) { stellar_fungible::burnable::burn_from(e, &spender, &from, amount); } },
        ],
        "NonFungibleToken" => vec![
            syn::parse_quote! { fn balance(e: &Env, owner: Address) -> Balance { Self::ContractType::balance(e, &owner) } },
            syn::parse_quote! { fn owner_of(e: &Env, token_id: TokenId) -> Address { Self::ContractType::owner_of(e, token_id) } },
            syn::parse_quote! { fn transfer(e: &Env, from: Address, to: Address, token_id: TokenId) { Self::ContractType::transfer(e, &from, &to, token_id); } },
            syn::parse_quote! { fn transfer_from(e: &Env, spender: Address, from: Address, to: Address, token_id: TokenId) { Self::ContractType::transfer_from(e, &spender, &from, &to, token_id); } },
            syn::parse_quote! { fn approve(e: &Env, approver: Address, approved: Address, token_id: TokenId, live_until_ledger: u32) { Self::ContractType::approve(e, &approver, &approved, token_id, live_until_ledger); } },
            syn::parse_quote! { fn approve_for_all(e: &Env, owner: Address, operator: Address, live_until_ledger: u32) { Self::ContractType::approve_for_all(e, &owner, &operator, live_until_ledger); } },
            syn::parse_quote! { fn get_approved(e: &Env, token_id: TokenId) -> Option<Address> { Self::ContractType::get_approved(e, token_id) } },
            syn::parse_quote! { fn is_approved_for_all(e: &Env, owner: Address, operator: Address) -> bool { Self::ContractType::is_approved_for_all(e, &owner, &operator) } },
            syn::parse_quote! { fn token_uri(e: &Env, token_id: TokenId) -> String { Self::ContractType::token_uri(e, token_id) } },
            syn::parse_quote! { fn name(e: &Env) -> String { Self::ContractType::name(e) } },
            syn::parse_quote! { fn symbol(e: &Env) -> String { Self::ContractType::symbol(e) } },
        ],
        "NonFungibleBurnable" => vec![
            syn::parse_quote! { fn burn(e: &Env, from: Address, token_id: TokenId) { Base::burn(e, &from, token_id); } },
            syn::parse_quote! { fn burn_from(e: &Env, spender: Address, from: Address, token_id: TokenId) { Base::burn_from(e, &spender, &from, token_id); } },
        ],
        "NonFungibleEnumerable" => vec![
            syn::parse_quote! { fn total_supply(e: &Env) -> Balance { Enumerable::total_supply(e) } },
            syn::parse_quote! { fn get_owner_token_id(e: &Env, owner: Address, index: TokenId) -> TokenId { Enumerable::get_owner_token_id(e, &owner, index) } },
            syn::parse_quote! { fn get_token_id(e: &Env, index: TokenId) -> TokenId { Enumerable::get_token_id(e, index) } },
        ],
        not_supported => {
            panic!("Trait {} is not supported by #[default_impl]", not_supported)
        }
    }
}

pub fn generate_default_impl(item: TokenStream) -> TokenStream {
    let input = parse_macro_input!(item as ItemImpl);

    // Extract the trait name
    let trait_name = match &input.trait_ {
        Some((_, path, _)) => path.segments.last().unwrap().ident.to_string(),
        None => panic!("#[default_impl] must be used on a trait implementation"),
    };

    let mut user_methods = std::collections::HashSet::new();
    for item in &input.items {
        if let syn::ImplItem::Fn(method) = item {
            user_methods.insert(method.sig.ident.to_string());
        }
    }

    // Get default methods for the trait
    let mut default_methods = get_default_methods(&trait_name);

    // Remove overridden methods
    default_methods.retain(|item| {
        if let syn::ImplItem::Fn(method) = item {
            !user_methods.contains(&method.sig.ident.to_string())
        } else {
            true
        }
    });

    // Merge default methods with user-defined ones
    let mut existing_items = input.items.clone();
    existing_items.extend(default_methods);

    // `existing_items` now contains the merged items
    let new_impl = ItemImpl { items: existing_items, ..input };

    // Import the necessary trait if the trait is `NonFungibleToken`
    let expanded = if trait_name == "NonFungibleToken" {
        quote! {
            use stellar_non_fungible::ContractOverrides;
            #new_impl
        }
    } else {
        quote! { #new_impl }
    };

    TokenStream::from(quote! { #expanded })
}
