//! Non-Fungible with Access Control Example Contract.
//!
//! Demonstrates how can Access Control be utilized.

use soroban_sdk::{contract, contractimpl, vec, Address, Env, String, Vec};
use stellar_access_control::{set_admin, AccessControl};
use stellar_access_control_macros::{has_role, only_admin};
use stellar_default_impl_macro::default_impl;
use stellar_non_fungible::{burnable::NonFungibleBurnable, Base, NonFungibleToken};

#[contract]
pub struct ExampleContract;

#[contractimpl]
impl ExampleContract {
    pub fn __constructor(e: &Env, admin: Address) {
        set_admin(e, &admin);
        Base::set_metadata(
            e,
            String::from_str(e, "www.mytoken.com"),
            String::from_str(e, "My Token"),
            String::from_str(e, "TKN"),
        );
    }

    #[only_admin]
    pub fn admin_restricted_function(e: &Env) -> Vec<String> {
        vec![&e, String::from_str(&e, "seems sus")]
    }

    #[has_role(caller, "minter")]
    pub fn mint(e: &Env, caller: Address, to: Address, token_id: u32) {
        Base::mint(e, &to, token_id)
    }
}

#[default_impl]
#[contractimpl]
impl NonFungibleToken for ExampleContract {
    type ContractType = Base;
}

// for this contract, the `burn*` functions are only meant to be called by
// specific people with the `burner` role
#[contractimpl]
impl NonFungibleBurnable for ExampleContract {
    #[has_role(from, "burner")]
    fn burn(e: &Env, from: Address, token_id: u32) {
        Base::burn(e, &from, token_id);
    }

    #[has_role(spender, "burner")]
    fn burn_from(e: &Env, spender: Address, from: Address, token_id: u32) {
        Base::burn_from(e, &spender, &from, token_id);
    }
}

#[default_impl]
#[contractimpl]
impl AccessControl for ExampleContract {}
