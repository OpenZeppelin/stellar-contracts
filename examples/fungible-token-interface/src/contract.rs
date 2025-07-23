//! Fungible Pausable Example Contract.

//! This contract showcases how to integrate various OpenZeppelin modules to
//! build a fully SEP-41-compliant fungible token. It includes essential
//! features such as an emergency stop mechanism and controlled token minting by
//! the owner.
//!
//! This contract replicates the functionality of the contract in
//! "examples/fungible-pausable", offering the same features. The key difference
//! lies in how SEP-41 compliance is achieved. The contract in "contract.rs"
//! accomplishes this by implementing
//! [`stellar_tokens::fungible::FungibleToken`] and
//! [`stellar_tokens::fungible_burnable::FungibleBurnable`], whereas this
//! version directly implements [`soroban_sdk::token::TokenInterface`].
//!
//! Ultimately, it is up to the user to choose their preferred approach to
//! creating a SEP-41 token. We suggest the approach in
//! "examples/fungible-pausable" for better organization of the code,
//! consistency and ease of inspection/debugging.

use soroban_sdk::{
    contract, contracterror, contractimpl, panic_with_error, symbol_short, token::TokenInterface,
    Address, Env, String, Symbol,
};
use stellar_contract_utils::pausable::{self as pausable, Pausable};
use stellar_macros::when_not_paused;
use stellar_tokens::fungible::Base;

#[contract]

pub struct ExampleContract;

#[contractimpl]
impl ExampleContract {
    pub fn __constructor(e: &Env, owner: Address, initial_supply: i128) {
        Self::set_metadata(e, 18, String::from_str(e, "My Token"), String::from_str(e, "TKN"));
        Self::set_owner(e, &owner);
        Self::internal_mint(e, &owner, initial_supply);
    }

    pub fn mint(e: &Env, account: Address, amount: i128) {
        Self::when_not_paused(e);
        Self::enforce_owner_auth(e);
        Self::internal_mint(e, &account, amount);
    }
}

#[contracttrait]
impl Ownable for ExampleContract {}

#[contracttrait(ext = PausableExt)]
impl FungibleToken for ExampleContract {}

#[contracttrait(ext = PausableExt)]
impl FungibleBurnable for ExampleContract {}

#[contracttrait(ext = OwnableExt)]
impl Pausable for ExampleContract {}

// NOTE: if your contract implements `FungibleToken` and `FungibleBurnable`,
// and you also want your contract to implement
// `soroban_sdk::token::TokenInterface`, you can use the `impl_token_interface!`
// macro to generate the boilerplate implementation.
impl_token_interface!(ExampleContract);
