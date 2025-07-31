//! Fungibe BlockList Example Contract.

//! This contract showcases how to integrate the BlockList extension with a
//! SEP-41-compliant fungible token. It includes essential features such as
//! controlled token transfers by an admin who can block or unblock specific
//! accounts.

use soroban_sdk::{contract, contractimpl, contracttrait, symbol_short, Address, Env, String};
use stellar_access::AccessControl;
use stellar_macros::only_role;
use stellar_tokens::{FungibleBlockList, FungibleToken};

#[contract]
pub struct ExampleContract;

#[contractimpl]
impl ExampleContract {
    pub fn __constructor(e: &Env, admin: Address, manager: Address, initial_supply: i128) {
        Self::set_metadata(
            e,
            18,
            String::from_str(e, "BlockList Token"),
            String::from_str(e, "BLT"),
        );

        Self::init_admin(e, &admin);
        // create a role "manager" and grant it to `manager`
        Self::grant_role_no_auth(e, &admin, &manager, &symbol_short!("manager"));
        // Mint initial supply to the admin
        Self::internal_mint(e, &admin, initial_supply);
    }
}

#[contracttrait]
impl FungibleToken for ExampleContract {
    fn transfer(e: &Env, from: &Address, to: &Address, amount: i128) {
        Self::assert_not_blocked(e, &[from, to]);
        Self::Impl::transfer(e, from, to, amount);
    }

    fn transfer_from(e: &Env, spender: &Address, from: &Address, to: &Address, amount: i128) {
        Self::assert_not_blocked(e, &[from, to]);
        Self::Impl::transfer_from(e, spender, from, to, amount);
    }

    fn approve(e: &Env, owner: &Address, spender: &Address, amount: i128, live_until_ledger: u32) {
        Self::assert_not_blocked(e, &[owner]);
        Self::Impl::approve(e, owner, spender, amount, live_until_ledger);
    }
}

#[contracttrait]
impl AccessControl for ExampleContract {}

#[contracttrait]
impl FungibleBlockList for ExampleContract {
    #[only_role(operator, "manager")]
    fn block_user(e: &Env, user: &Address, operator: &Address) {
        Self::Impl::block_user(e, user, operator)
    }

    #[only_role(operator, "manager")]
    fn unblock_user(e: &Env, user: &Address, operator: &Address) {
        Self::Impl::unblock_user(e, user, operator)
    }
}
