//! Fungible BlockList Example Contract.

//! This contract showcases how to integrate the BlockList extension with a
//! SEP-41-compliant fungible token. It includes essential features such as
//! controlled token transfers by an admin who can block or unblock specific accounts.

use soroban_sdk::{contract, contracterror, contractimpl, panic_with_error, Address, Env, String};
use stellar_fungible::{
    self as fungible,
    blocklist::{BlockList, FungibleBlockList},
    burnable::FungibleBurnable,
    impl_token_interface,
    mintable::FungibleMintable,
    FungibleToken, FungibleTokenError,
};

#[contract]
pub struct ExampleContract;

#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq, PartialOrd, Ord)]
#[repr(u32)]
pub enum ExampleContractError {
    Unauthorized = 1,
}

#[contractimpl]
impl ExampleContract {
    pub fn __constructor(e: &Env, admin: Address, initial_supply: i128) {
        fungible::metadata::set_metadata(
            e,
            18,
            String::from_str(e, "BlockList Token"),
            String::from_str(e, "BLT"),
        );

        // Set the admin for the BlockList extension
        BlockList::set_admin(e, &admin);

        // Mint initial supply to the admin
        fungible::mintable::mint(e, &admin, initial_supply);
    }
}

#[contractimpl]
impl FungibleToken for ExampleContract {
    type ContractType = BlockList;

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
        // The BlockList extension will check if any account is blocked
        fungible::transfer(e, &from, &to, amount);
    }

    fn transfer_from(e: &Env, spender: Address, from: Address, to: Address, amount: i128) {
        // The BlockList extension will check if any account is blocked
        fungible::transfer_from(e, &spender, &from, &to, amount);
    }

    fn approve(e: &Env, owner: Address, spender: Address, amount: i128, live_until_ledger: u32) {
        // The BlockList extension will check if any account is blocked
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
        // The BlockList extension will check if the account is blocked
        fungible::burnable::burn(e, &from, amount)
    }

    fn burn_from(e: &Env, spender: Address, from: Address, amount: i128) {
        // The BlockList extension will check if any account is blocked
        fungible::burnable::burn_from(e, &spender, &from, amount)
    }
}

#[contractimpl]
impl FungibleMintable for ExampleContract {
    fn mint(e: &Env, account: Address, amount: i128) {
        // Verify admin authorization
        let admin = BlockList::get_admin(e);
        admin.require_auth();

        // Check if the account is blocked before minting
        if BlockList::blocked(e, &account) {
            panic_with_error!(e, FungibleTokenError::UserBlocked);
        }

        fungible::mintable::mint(e, &account, amount);
    }
}

#[contractimpl]
impl FungibleBlockList for ExampleContract {
    fn blocked(e: &Env, account: Address) -> bool {
        BlockList::blocked(e, &account)
    }

    fn block_user(e: &Env, user: Address) {
        BlockList::block_user(e, &user)
    }

    fn unblock_user(e: &Env, user: Address) {
        BlockList::unblock_user(e, &user)
    }
}

// Implement TokenInterface for SEP-41 compliance
impl_token_interface!(ExampleContract);
