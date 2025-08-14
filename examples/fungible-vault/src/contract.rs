//! Fungible Vault Example Contract.

use soroban_sdk::{contract, contractimpl, Address, Env, String};
use stellar_contract_utils::math::fixed_point::Rounding;
use stellar_macros::default_impl;
use stellar_tokens::fungible::{
    vault::{FungibleVault, FungibleVaultInternal, Vault},
    Base, FungibleToken,
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

    // While overriding is technically possible, most implementations should stick to the
    // pattern of setting the asset address in the constructor and returning it from
    // instance storage, as this maintains predictability and composability that
    // other contracts and users expect from vaults.
    // IMPORTANT: If overriding query_asset, you MUST also override all other functions that depend on it.
    // Failure to override these functions will result in inconsistent behavior.
    fn query_asset(e: &Env) -> Address {
        Vault::query_asset(e)
    }

    fn total_assets(e: &Env) -> i128 {
        Vault::total_assets(e)
    }

    fn convert_to_shares(e: &Env, assets: i128) -> i128 {
        Vault::convert_to_shares::<Self>(e, assets)
    }

    fn convert_to_assets(e: &Env, shares: i128) -> i128 {
        Vault::convert_to_assets::<Self>(e, shares)
    }

    fn max_deposit(e: &Env, receiver: Address) -> i128 {
        Vault::max_deposit(e, receiver)
    }

    fn preview_deposit(e: &Env, assets: i128) -> i128 {
        Vault::preview_deposit::<Self>(e, assets)
    }

    fn max_mint(e: &Env, receiver: Address) -> i128 {
        Vault::max_mint(e, receiver)
    }

    fn preview_mint(e: &Env, shares: i128) -> i128 {
        Vault::preview_mint::<Self>(e, shares)
    }

    fn max_withdraw(e: &Env, owner: Address) -> i128 {
        Vault::max_withdraw::<Self>(e, owner)
    }

    fn preview_withdraw(e: &Env, assets: i128) -> i128 {
        Vault::preview_withdraw::<Self>(e, assets)
    }

    fn max_redeem(e: &Env, owner: Address) -> i128 {
        Vault::max_redeem::<Self>(e, owner)
    }

    fn preview_redeem(e: &Env, shares: i128) -> i128 {
        Vault::preview_redeem::<Self>(e, shares)
    }

    fn deposit(e: &Env, assets: i128, caller: Address, receiver: Address) -> i128 {
        Vault::deposit::<Self>(e, assets, caller, receiver)
    }

    fn mint(e: &Env, shares: i128, caller: Address, receiver: Address) -> i128 {
        Vault::mint::<Self>(e, shares, caller, receiver)
    }

    fn withdraw(e: &Env, assets: i128, caller: Address, receiver: Address, owner: Address) -> i128 {
        Vault::withdraw::<Self>(e, assets, caller, receiver, owner)
    }

    fn redeem(e: &Env, shares: i128, caller: Address, receiver: Address, owner: Address) -> i128 {
        Vault::redeem::<Self>(e, shares, caller, receiver, owner)
    }
}

// Do not apply #[contractimpl] attribute to preserve internal scope.
impl FungibleVaultInternal for ExampleContract {
    // Allows override of internal vault functions.

    fn convert_to_shares_with_rounding(e: &Env, assets: i128, rounding: Rounding) -> i128 {
        Vault::convert_to_shares_with_rounding::<Self>(e, assets, rounding)
    }

    fn convert_to_assets_with_rounding(e: &Env, shares: i128, rounding: Rounding) -> i128 {
        Vault::convert_to_assets_with_rounding::<Self>(e, shares, rounding)
    }

    fn deposit_no_auth(e: &Env, caller: &Address, receiver: &Address, assets: i128, shares: i128) {
        Vault::deposit_no_auth(e, caller, receiver, assets, shares);
    }

    fn withdraw_no_auth(
        e: &Env,
        caller: &Address,
        receiver: &Address,
        owner: &Address,
        assets: i128,
        shares: i128,
    ) {
        Vault::withdraw_no_auth(e, caller, receiver, owner, assets, shares);
    }
}
