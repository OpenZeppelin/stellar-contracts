//! Tokenized Vault Example Contract.

use soroban_sdk::{contract, contractimpl, Address, Env, String};
use stellar_macros::default_impl;
use stellar_tokens::{
    fungible::{Base, FungibleToken},
    vault::{FungibleVault, Vault},
};

#[contract]
pub struct ExampleContract;

#[contractimpl]
impl ExampleContract {
    pub fn __constructor(e: &Env, asset: Address, decimals_offset: u32) {
        // Asset and decimal offset should be configured once during initialization.
        Vault::set_asset(e, asset);
        Vault::set_decimals_offset(e, decimals_offset);
        // Vault overrides the decimals function by default.
        // Decimal offset must be set prior to metadata initialization.
        Base::set_metadata(
            e,
            Self::decimals(e),
            String::from_str(e, "Vault Token"),
            String::from_str(e, "VLT"),
        );
    }
}

#[default_impl]
#[contractimpl]
impl FungibleToken for ExampleContract {
    type ContractType = Vault;

    // Allows override of decimals and other base functions.

    fn decimals(e: &Env) -> u32 {
        Vault::decimals(e)
    }
}

#[contractimpl]
impl FungibleVault for ExampleContract {
    // Allows override of public vault functions.

    fn query_asset(e: &Env) -> Address {
        Vault::query_asset(e)
    }

    fn total_assets(e: &Env) -> i128 {
        Vault::total_assets(e)
    }

    fn convert_to_shares(e: &Env, assets: i128) -> i128 {
        Vault::convert_to_shares(e, assets)
    }

    fn convert_to_assets(e: &Env, shares: i128) -> i128 {
        Vault::convert_to_assets(e, shares)
    }

    fn max_deposit(e: &Env, receiver: Address) -> i128 {
        Vault::max_deposit(e, receiver)
    }

    fn preview_deposit(e: &Env, assets: i128) -> i128 {
        Vault::preview_deposit(e, assets)
    }

    fn max_mint(e: &Env, receiver: Address) -> i128 {
        Vault::max_mint(e, receiver)
    }

    fn preview_mint(e: &Env, shares: i128) -> i128 {
        Vault::preview_mint(e, shares)
    }

    fn max_withdraw(e: &Env, owner: Address) -> i128 {
        Vault::max_withdraw(e, owner)
    }

    fn preview_withdraw(e: &Env, assets: i128) -> i128 {
        Vault::preview_withdraw(e, assets)
    }

    fn max_redeem(e: &Env, owner: Address) -> i128 {
        Vault::max_redeem(e, owner)
    }

    fn preview_redeem(e: &Env, shares: i128) -> i128 {
        Vault::preview_redeem(e, shares)
    }

    fn deposit(e: &Env, assets: i128, receiver: Address, from: Address, operator: Address) -> i128 {
        operator.require_auth();
        Vault::deposit(e, assets, receiver, from, operator)
    }

    fn mint(e: &Env, shares: i128, receiver: Address, from: Address, operator: Address) -> i128 {
        operator.require_auth();
        Vault::mint(e, shares, receiver, from, operator)
    }

    fn withdraw(
        e: &Env,
        assets: i128,
        receiver: Address,
        owner: Address,
        operator: Address,
    ) -> i128 {
        operator.require_auth();
        Vault::withdraw(e, assets, receiver, owner, operator)
    }

    fn redeem(e: &Env, shares: i128, receiver: Address, owner: Address, operator: Address) -> i128 {
        operator.require_auth();
        Vault::redeem(e, shares, receiver, owner, operator)
    }
}
