use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, ItemImpl};

fn get_default_methods(trait_name: &str) -> Vec<syn::ImplItem> {
    match trait_name {
        "AccessControl" => vec![
            syn::parse_quote! {
                fn has_role(e: &soroban_sdk::Env, account: soroban_sdk::Address, role: soroban_sdk::Symbol) -> Option<u32> {
                    stellar_access::access_control::has_role(e, &account, &role)
                }
            },
            syn::parse_quote! {
                fn get_existing_roles(e: &soroban_sdk::Env) -> soroban_sdk::Vec<soroban_sdk::Symbol> {
                    stellar_access::access_control::get_existing_roles(e)
                }
            },
            syn::parse_quote! {
                fn get_role_member_count(e: &soroban_sdk::Env, role: soroban_sdk::Symbol) -> u32 {
                    stellar_access::access_control::get_role_member_count(e, &role)
                }
            },
            syn::parse_quote! {
                fn get_role_member(e: &soroban_sdk::Env, role: soroban_sdk::Symbol, index: u32) -> soroban_sdk::Address {
                    stellar_access::access_control::get_role_member(e, &role, index)
                }
            },
            syn::parse_quote! {
                fn get_role_admin(e: &soroban_sdk::Env, role: soroban_sdk::Symbol) -> Option<soroban_sdk::Symbol> {
                    stellar_access::access_control::get_role_admin(e, &role)
                }
            },
            syn::parse_quote! {
                fn get_admin(e: &soroban_sdk::Env) -> Option<soroban_sdk::Address> {
                    stellar_access::access_control::get_admin(e)
                }
            },
            syn::parse_quote! {
                fn grant_role(e: &soroban_sdk::Env, account: soroban_sdk::Address, role: soroban_sdk::Symbol, caller: soroban_sdk::Address) {
                    stellar_access::access_control::grant_role(e, &account, &role, &caller);
                }
            },
            syn::parse_quote! {
                fn revoke_role(e: &soroban_sdk::Env, account: soroban_sdk::Address, role: soroban_sdk::Symbol, caller: soroban_sdk::Address ) {
                    stellar_access::access_control::revoke_role(e, &account, &role, &caller);
                }
            },
            syn::parse_quote! {
                fn renounce_role(e: &soroban_sdk::Env, role: soroban_sdk::Symbol, caller: soroban_sdk::Address) {
                    stellar_access::access_control::renounce_role(e, &role, &caller);
                }
            },
            syn::parse_quote! {
                fn renounce_admin(e: &soroban_sdk::Env) {
                    stellar_access::access_control::renounce_admin(e);
                }
            },
            syn::parse_quote! {
                fn transfer_admin_role(e: &soroban_sdk::Env, new_admin: soroban_sdk::Address, live_until_ledger: u32) {
                    stellar_access::access_control::transfer_admin_role(e, &new_admin, live_until_ledger);
                }
            },
            syn::parse_quote! {
                fn accept_admin_transfer(e: &soroban_sdk::Env) {
                    stellar_access::access_control::accept_admin_transfer(e);
                }
            },
            syn::parse_quote! {
                fn set_role_admin(e: &soroban_sdk::Env, role: soroban_sdk::Symbol, admin_role: soroban_sdk::Symbol) {
                    stellar_access::access_control::set_role_admin(e, &role, &admin_role);
                }
            },
        ],
        "FungibleToken" => vec![
            syn::parse_quote! {
                fn total_supply(e: &soroban_sdk::Env) -> i128 {
                    Self::ContractType::total_supply(e)
                }
            },
            syn::parse_quote! {
                fn balance(e: &soroban_sdk::Env, account: soroban_sdk::Address) -> i128 {
                    Self::ContractType::balance(e, &account)
                }
            },
            syn::parse_quote! {
                fn allowance(e: &soroban_sdk::Env, owner: soroban_sdk::Address, spender: soroban_sdk::Address) -> i128 {
                    Self::ContractType::allowance(e, &owner, &spender)
                }
            },
            syn::parse_quote! {
                fn transfer(e: &soroban_sdk::Env, from: soroban_sdk::Address, to: soroban_sdk::MuxedAddress, amount: i128) {
                    Self::ContractType::transfer(e, &from, &to, amount);
                }
            },
            syn::parse_quote! {
                fn transfer_from(e: &soroban_sdk::Env, spender: soroban_sdk::Address, from: soroban_sdk::Address, to: soroban_sdk::Address, amount: i128) {
                    Self::ContractType::transfer_from(e, &spender, &from, &to, amount);
                }
            },
            syn::parse_quote! {
                fn approve(e: &soroban_sdk::Env, owner: soroban_sdk::Address, spender: soroban_sdk::Address, amount: i128, live_until_ledger: u32) {
                    Self::ContractType::approve(e, &owner, &spender, amount, live_until_ledger);
                }
            },
            syn::parse_quote! {
                fn decimals(e: &soroban_sdk::Env) -> u32 {
                    Self::ContractType::decimals(e)
                }
            },
            syn::parse_quote! {
                fn name(e: &soroban_sdk::Env) -> soroban_sdk::String {
                    Self::ContractType::name(e)
                }
            },
            syn::parse_quote! {
                fn symbol(e: &soroban_sdk::Env) -> soroban_sdk::String {
                    Self::ContractType::symbol(e)
                }
            },
        ],
        "FungibleBurnable" => vec![
            syn::parse_quote! {
                fn burn(e: &soroban_sdk::Env, from: soroban_sdk::Address, amount: i128) {
                    Self::ContractType::burn(e, &from, amount);
                }
            },
            syn::parse_quote! {
                fn burn_from(e: &soroban_sdk::Env, spender: soroban_sdk::Address, from: soroban_sdk::Address, amount: i128) {
                    Self::ContractType::burn_from(e, &spender, &from, amount);
                }
            },
        ],
        "NonFungibleToken" => vec![
            syn::parse_quote! {
                fn balance(e: &soroban_sdk::Env, account: soroban_sdk::Address) -> u32 {
                    Self::ContractType::balance(e, &account)
                }
            },
            syn::parse_quote! {
                fn owner_of(e: &soroban_sdk::Env, token_id: u32) -> soroban_sdk::Address {
                    Self::ContractType::owner_of(e, token_id)
                }
            },
            syn::parse_quote! {
                fn transfer(e: &soroban_sdk::Env, from: soroban_sdk::Address, to: soroban_sdk::Address, token_id: u32) {
                    Self::ContractType::transfer(e, &from, &to, token_id);
                }
            },
            syn::parse_quote! {
                fn transfer_from(e: &soroban_sdk::Env, spender: soroban_sdk::Address, from: soroban_sdk::Address, to: soroban_sdk::Address, token_id: u32) {
                    Self::ContractType::transfer_from(e, &spender, &from, &to, token_id);
                }
            },
            syn::parse_quote! {
                fn approve(e: &soroban_sdk::Env, approver: soroban_sdk::Address, approved: soroban_sdk::Address, token_id: u32, live_until_ledger: u32) {
                    Self::ContractType::approve(e, &approver, &approved, token_id, live_until_ledger);
                }
            },
            syn::parse_quote! {
                fn approve_for_all(e: &soroban_sdk::Env, owner: soroban_sdk::Address, operator: soroban_sdk::Address, live_until_ledger: u32) {
                    Self::ContractType::approve_for_all(e, &owner, &operator, live_until_ledger);
                }
            },
            syn::parse_quote! {
                fn get_approved(e: &soroban_sdk::Env, token_id: u32) -> Option<soroban_sdk::Address> {
                    Self::ContractType::get_approved(e, token_id)
                }
            },
            syn::parse_quote! {
                fn is_approved_for_all(e: &soroban_sdk::Env, owner: soroban_sdk::Address, operator: soroban_sdk::Address) -> bool {
                    Self::ContractType::is_approved_for_all(e, &owner, &operator)
                }
            },
            syn::parse_quote! {
                fn token_uri(e: &soroban_sdk::Env, token_id: u32) -> soroban_sdk::String {
                    Self::ContractType::token_uri(e, token_id)
                }
            },
            syn::parse_quote! {
                fn name(e: &soroban_sdk::Env) -> soroban_sdk::String {
                    Self::ContractType::name(e)
                }
            },
            syn::parse_quote! {
                fn symbol(e: &soroban_sdk::Env) -> soroban_sdk::String {
                    Self::ContractType::symbol(e)
                }
            },
        ],
        "NonFungibleBurnable" => vec![
            syn::parse_quote! {
                fn burn(e: &soroban_sdk::Env, from: soroban_sdk::Address, token_id: u32) {
                    Self::ContractType::burn(e, &from, token_id);
                }
            },
            syn::parse_quote! {
                fn burn_from(e: &soroban_sdk::Env, spender: soroban_sdk::Address, from: soroban_sdk::Address, token_id: u32) {
                    Self::ContractType::burn_from(e, &spender, &from, token_id);
                }
            },
        ],
        "NonFungibleEnumerable" => vec![
            syn::parse_quote! {
                fn total_supply(e: &soroban_sdk::Env) -> u32 {
                    Enumerable::total_supply(e)
                }
            },
            syn::parse_quote! {
                fn get_owner_token_id(e: &soroban_sdk::Env, owner: soroban_sdk::Address, index: u32) -> u32 {
                    Enumerable::get_owner_token_id(e, &owner, index)
                }
            },
            syn::parse_quote! {
                fn get_token_id(e: &soroban_sdk::Env, index: u32) -> u32 {
                    Enumerable::get_token_id(e, index)
                }
            },
        ],
        "Ownable" => vec![
            syn::parse_quote! {
                fn get_owner(e: &soroban_sdk::Env) -> Option<soroban_sdk::Address> {
                    stellar_access::ownable::get_owner(e)
                }
            },
            syn::parse_quote! {
                fn transfer_ownership(e: &soroban_sdk::Env, new_owner: soroban_sdk::Address, live_until_ledger: u32) {
                    stellar_access::ownable::transfer_ownership(e, &new_owner, live_until_ledger);
                }
            },
            syn::parse_quote! {
                fn accept_ownership(e: &soroban_sdk::Env) {
                    stellar_access::ownable::accept_ownership(e);
                }
            },
            syn::parse_quote! {
                fn renounce_ownership(e: &soroban_sdk::Env) {
                    stellar_access::ownable::renounce_ownership(e);
                }
            },
        ],
        "FungibleVault" => vec![
            syn::parse_quote! {
                fn query_asset(e: &soroban_sdk::Env) -> soroban_sdk::Address {
                    Self::ContractType::query_asset(e)
                }
            },
            syn::parse_quote! {
                fn total_assets(e: &soroban_sdk::Env) -> i128 {
                    Self::ContractType::total_assets(e)
                }
            },
            syn::parse_quote! {
                fn convert_to_shares(e: &soroban_sdk::Env, assets: i128) -> i128 {
                    Self::ContractType::convert_to_shares(e, assets)
                }
            },
            syn::parse_quote! {
                fn convert_to_assets(e: &soroban_sdk::Env, shares: i128) -> i128 {
                    Self::ContractType::convert_to_assets(e, shares)
                }
            },
            syn::parse_quote! {
                fn max_deposit(e: &soroban_sdk::Env, receiver: soroban_sdk::Address) -> i128 {
                    Self::ContractType::max_deposit(e, receiver)
                }
            },
            syn::parse_quote! {
                fn preview_deposit(e: &soroban_sdk::Env, assets: i128) -> i128 {
                    Self::ContractType::preview_deposit(e, assets)
                }
            },
            syn::parse_quote! {
                fn deposit(e: &soroban_sdk::Env, assets: i128, receiver: soroban_sdk::Address, operator: soroban_sdk::Address) -> i128 {
                    Self::ContractType::deposit(e, assets, receiver, operator)
                }
            },
            syn::parse_quote! {
                fn max_mint(e: &soroban_sdk::Env, receiver: soroban_sdk::Address) -> i128 {
                    Self::ContractType::max_mint(e, receiver)
                }
            },
            syn::parse_quote! {
                fn preview_mint(e: &soroban_sdk::Env, shares: i128) -> i128 {
                    Self::ContractType::preview_mint(e, shares)
                }
            },
            syn::parse_quote! {
                fn mint(e: &soroban_sdk::Env, shares: i128, receiver: soroban_sdk::Address, operator: soroban_sdk::Address) -> i128 {
                    Self::ContractType::mint(e, shares, receiver, operator)
                }
            },
            syn::parse_quote! {
                fn max_withdraw(e: &soroban_sdk::Env, owner: soroban_sdk::Address) -> i128 {
                    Self::ContractType::max_withdraw(e, owner)
                }
            },
            syn::parse_quote! {
                fn preview_withdraw(e: &soroban_sdk::Env, assets: i128) -> i128 {
                    Self::ContractType::preview_withdraw(e, assets)
                }
            },
            syn::parse_quote! {
                fn withdraw(e: &soroban_sdk::Env, assets: i128, receiver: soroban_sdk::Address, owner: soroban_sdk::Address, operator: soroban_sdk::Address) -> i128 {
                    Self::ContractType::withdraw(e, assets, receiver, owner, operator)
                }
            },
            syn::parse_quote! {
                fn max_redeem(e: &soroban_sdk::Env, owner: soroban_sdk::Address) -> i128 {
                    Self::ContractType::max_redeem(e, owner)
                }
            },
            syn::parse_quote! {
                fn preview_redeem(e: &soroban_sdk::Env, shares: i128) -> i128 {
                    Self::ContractType::preview_redeem(e, shares)
                }
            },
            syn::parse_quote! {
                fn redeem(e: &soroban_sdk::Env, shares: i128, receiver: soroban_sdk::Address, owner: soroban_sdk::Address, operator: soroban_sdk::Address) -> i128 {
                    Self::ContractType::redeem(e, shares, receiver, owner, operator)
                }
            },
        ],

        not_supported => {
            panic!("Trait {not_supported} is not supported by #[default_impl]")
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

    // Import the necessary trait if the trait is `NonFungibleToken` or
    // `FungibleToken`
    let expanded = if trait_name == "NonFungibleToken" {
        quote! {
            use stellar_tokens::non_fungible::ContractOverrides;
            #new_impl
        }
    } else if trait_name == "FungibleToken" {
        quote! {
            use stellar_tokens::fungible::ContractOverrides;
            #new_impl
        }
    } else {
        quote! { #new_impl }
    };

    TokenStream::from(quote! { #expanded })
}
