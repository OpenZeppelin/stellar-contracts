//! Fungible AllowList Example Contract.

//! This contract showcases how to integrate the AllowList extension with a
//! SEP-41-compliant fungible token. It includes essential features such as
//! controlled token transfers by an admin who can allow or disallow specific accounts.

use soroban_sdk::{contract, contractimpl, panic_with_error, Address, Env, String};
use stellar_fungible::{
    self as fungible,
    allowlist::{AllowList, FungibleAllowList},
    burnable::FungibleBurnable,
    impl_token_interface,
    mintable::FungibleMintable,
    FungibleToken, FungibleTokenError,
};

#[contract]
pub struct ExampleContract;

#[contractimpl]
impl ExampleContract {
    pub fn __constructor(e: &Env, admin: Address, initial_supply: i128) {
        fungible::metadata::set_metadata(
            e,
            18,
            String::from_str(e, "AllowList Token"),
            String::from_str(e, "ALT"),
        );

        // Set the admin for the AllowList extension
        AllowList::set_admin(e, &admin);

        // Allow the admin to transfer tokens
        AllowList::allow_user(e, &admin);

        // Mint initial supply to the admin
        fungible::mintable::mint(e, &admin, initial_supply);
    }
}

#[contractimpl]
impl FungibleToken for ExampleContract {
    type ContractType = AllowList;

    fn total_supply(e: &Env) -> i128 {
        fungible::total_supply(e)
    }

    fn balance(e: &Env, account: Address) -> i128 {
        fungible::balance(e, &account)
    }

    fn allowance(e: &Env, owner: Address, spender: Address) -> i128 {
        fungible::allowance(e, &owner, &spender)
    }

    fn transfer(e: &Env, from: Address, to: Address, amount: i128) {
        // The AllowList extension will check if both accounts are allowed
        fungible::transfer(e, &from, &to, amount);
    }

    fn transfer_from(e: &Env, spender: Address, from: Address, to: Address, amount: i128) {
        // The AllowList extension will check if all accounts are allowed
        fungible::transfer_from(e, &spender, &from, &to, amount);
    }

    fn approve(e: &Env, owner: Address, spender: Address, amount: i128, live_until_ledger: u32) {
        // The AllowList extension will check if both accounts are allowed
        fungible::approve(e, &owner, &spender, amount, live_until_ledger);
    }

    fn decimals(e: &Env) -> u32 {
        fungible::metadata::decimals(e)
    }

    fn name(e: &Env) -> String {
        fungible::metadata::name(e)
    }

    fn symbol(e: &Env) -> String {
        fungible::metadata::symbol(e)
    }
}

#[contractimpl]
impl FungibleBurnable for ExampleContract {
    fn burn(e: &Env, from: Address, amount: i128) {
        // The AllowList extension will check if the account is allowed
        fungible::burnable::burn(e, &from, amount)
    }

    fn burn_from(e: &Env, spender: Address, from: Address, amount: i128) {
        // The AllowList extension will check if both accounts are allowed
        fungible::burnable::burn_from(e, &spender, &from, amount)
    }
}

#[contractimpl]
impl FungibleMintable for ExampleContract {
    fn mint(e: &Env, account: Address, amount: i128) {
        // Verify admin authorization
        let admin: Address = AllowList::get_admin(e);
        admin.require_auth();

        // Check if the account is allowed before minting
        if !AllowList::allowed(e, &account) {
            panic_with_error!(e, FungibleTokenError::UserNotAllowed);
        }

        fungible::mintable::mint(e, &account, amount);
    }
}

#[contractimpl]
impl FungibleAllowList for ExampleContract {
    fn allowed(e: &Env, account: Address) -> bool {
        AllowList::allowed(e, &account)
    }

    fn allow_user(e: &Env, user: Address) {
        AllowList::allow_user(e, &user)
    }

    fn disallow_user(e: &Env, user: Address) {
        AllowList::disallow_user(e, &user)
    }
}

// Implement TokenInterface for SEP-41 compliance
impl_token_interface!(ExampleContract);
