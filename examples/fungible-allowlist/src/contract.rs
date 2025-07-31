//! Fungible AllowList Example Contract.

//! This contract showcases how to integrate the AllowList extension with a
//! SEP-41-compliant fungible token. It includes essential features such as
//! controlled token transfers by an admin who can allow or disallow specific
//! accounts.

use soroban_sdk::{contract, contractimpl, contracttrait, symbol_short, Address, Env, String};
use stellar_access::AccessControl;
use stellar_macros::has_role;
use stellar_tokens::{FungibleAllowList, FungibleBurnable, FungibleToken};

#[contract]
pub struct ExampleContract;

#[contractimpl]
impl ExampleContract {
    pub fn __constructor(e: &Env, admin: Address, manager: Address, initial_supply: i128) {
        Self::set_metadata(
            e,
            18,
            String::from_str(e, "AllowList Token"),
            String::from_str(e, "ALT"),
        );

        Self::init_admin(e, &admin);

        // create a role "manager" and grant it to `manager`
        Self::grant_role_no_auth(e, &admin, &manager, &symbol_short!("manager"));

        // Allow the admin to transfer tokens
        <Self as FungibleAllowList>::allow_user(e, &admin, &manager);

        // Mint initial supply to the admin
        Self::internal_mint(e, &admin, initial_supply);
    }
}

#[contracttrait]
impl FungibleToken for ExampleContract {
    fn transfer(e: &Env, from: &Address, to: &Address, amount: i128) {
        Self::assert_allowed(e, &[from, to]);
        Self::Impl::transfer(e, from, to, amount);
    }

    fn transfer_from(e: &Env, spender: &Address, from: &Address, to: &Address, amount: i128) {
        Self::assert_allowed(e, &[from, to]);
        Self::Impl::transfer_from(e, spender, from, to, amount);
    }

    fn approve(e: &Env, owner: &Address, spender: &Address, amount: i128, live_until_ledger: u32) {
        Self::assert_allowed(e, &[owner]);
        Self::Impl::approve(e, owner, spender, amount, live_until_ledger);
    }
}

#[contracttrait]
impl FungibleBurnable for ExampleContract {
    fn burn(e: &Env, from: &Address, amount: i128) {
        Self::assert_allowed(e, &[from]);
        Self::Impl::burn(e, from, amount);
    }

    fn burn_from(e: &Env, spender: &Address, from: &Address, amount: i128) {
        Self::assert_allowed(e, &[from]);
        Self::Impl::burn_from(e, spender, from, amount);
    }
}

#[contracttrait]
impl AccessControl for ExampleContract {}

#[contracttrait]
impl FungibleAllowList for ExampleContract {
    #[has_role(operator, "manager")]
    fn allow_user(e: &Env, user: &Address, operator: &Address) {
        Self::Impl::allow_user(e, user, operator)
    }

    #[has_role(operator, "manager")]
    fn disallow_user(e: &Env, user: &Address, operator: &Address) {
        Self::Impl::disallow_user(e, user, operator)
    }
}
