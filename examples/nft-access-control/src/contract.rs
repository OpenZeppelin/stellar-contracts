//! Non-Fungible with Access Control Example Contract.
//!
//! Demonstrates how can Access Control be utilized.

use soroban_sdk::{contract, contractimpl, vec, Address, Env, String, Vec};
use stellar_access::access_control::{set_admin, AccessControl};
use stellar_macros::{default_impl, has_any_role, has_role, only_admin, only_any_role, only_role};
use stellar_tokens::non_fungible::{burnable::NonFungibleBurnable, Base, NonFungibleToken};

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
        vec![&e, String::from_str(e, "seems sus")]
    }

    // we want `require_auth()` provided by the macro, since there is no
    // `require_auth()` in `Base::mint`.
    #[only_role(caller, "minter")]
    pub fn mint(e: &Env, caller: Address, to: Address, token_id: u32) {
        Base::mint(e, &to, token_id)
    }

    // allows either minter or burner role, does not enforce `require_auth` in the
    // macro
    #[has_any_role(caller, ["minter", "burner"])]
    pub fn multi_role_action(e: &Env, caller: Address) -> String {
        caller.require_auth();
        String::from_str(e, "multi_role_action_success")
    }

    // allows either minter or burner role AND enforces `require_auth` in the macro
    #[only_any_role(caller, ["minter", "burner"])]
    pub fn multi_role_auth_action(e: &Env, caller: Address) -> String {
        String::from_str(e, "multi_role_auth_action_success")
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
    // we DON'T want `require_auth()` provided by the macro, since there is already
    // `require_auth()` in `Base::burn`
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
