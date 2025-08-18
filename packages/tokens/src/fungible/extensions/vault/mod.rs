pub mod storage;

#[cfg(test)]
mod test;

use soroban_sdk::{symbol_short, Address, Env};

pub use storage::Vault;

use crate::fungible::FungibleToken;

use stellar_contract_utils::math::fixed_point::Rounding;

/// Vault Trait for Fungible Token
/// TODO: describe trait, functions, arguments
/// Public implementation details. Intended for external use.
/// May be overridden to customize default behavior.
pub trait FungibleVault: FungibleToken<ContractType = Vault> + FungibleVaultInternal {
    fn query_asset(e: &Env) -> Address;
    fn total_assets(e: &Env) -> i128;
    fn convert_to_shares(e: &Env, assets: i128) -> i128;
    fn convert_to_assets(e: &Env, shares: i128) -> i128;
    fn max_deposit(e: &Env, receiver: Address) -> i128;
    fn preview_deposit(e: &Env, assets: i128) -> i128;
    fn deposit(e: &Env, assets: i128, caller: Address, receiver: Address) -> i128;
    fn max_mint(e: &Env, receiver: Address) -> i128;
    fn preview_mint(e: &Env, shares: i128) -> i128;
    fn mint(e: &Env, shares: i128, caller: Address, receiver: Address) -> i128;
    fn max_withdraw(e: &Env, owner: Address) -> i128;
    fn preview_withdraw(e: &Env, assets: i128) -> i128;
    fn withdraw(e: &Env, assets: i128, caller: Address, receiver: Address, owner: Address) -> i128;
    fn max_redeem(e: &Env, owner: Address) -> i128;
    fn preview_redeem(e: &Env, shares: i128) -> i128;
    fn redeem(e: &Env, shares: i128, caller: Address, receiver: Address, owner: Address) -> i128;
}

/// Internal implementation details. Not intended for external use.
/// May be overridden to customize default behavior.
pub trait FungibleVaultInternal {
    fn convert_to_shares_with_rounding(e: &Env, assets: i128, rounding: Rounding) -> i128;
    fn convert_to_assets_with_rounding(e: &Env, shares: i128, rounding: Rounding) -> i128;
    fn deposit_no_auth(e: &Env, caller: &Address, receiver: &Address, assets: i128, shares: i128);
    fn withdraw_no_auth(
        e: &Env,
        caller: &Address,
        receiver: &Address,
        owner: &Address,
        assets: i128,
        shares: i128,
    );
}

// ################## EVENTS ##################

/// Emits an event when underlying assets are deposited into the vault in exchange for shares.
///
/// # Arguments
///
/// * `e` - Access to Soroban environment.
/// * `sender` - The address that initiated the deposit transaction.
/// * `owner` - The address that will own the vault shares being minted.
/// * `assets` - The amount of underlying assets being deposited into the vault.
/// * `shares` - The amount of vault shares being minted in exchange for the assets.
///
/// # Events
///
/// * topics - `["deposit", sender: Address, owner: Address]`
/// * data - `[assets: i128, shares: i128]`
pub fn emit_deposit(e: &Env, sender: &Address, owner: &Address, assets: i128, shares: i128) {
    let topics = (symbol_short!("deposit"), sender, owner);
    e.events().publish(topics, (assets, shares));
}

/// Emits an event when shares are exchanged back for underlying assets and assets are withdrawn from the vault.
///
/// # Arguments
///
/// * `e` - Access to Soroban environment.
/// * `sender` - The address that initiated the withdrawal transaction.
/// * `receiver` - The address that will receive the underlying assets being withdrawn.
/// * `owner` - The address that owns the vault shares being burned.
/// * `assets` - The amount of underlying assets being withdrawn from the vault.
/// * `shares` - The amount of vault shares being burned in exchange for the assets.
///
/// # Events
///
/// * topics - `["withdraw", sender: Address, receiver: Address, owner: Address]`
/// * data - `[assets: i128, shares: i128]`
pub fn emit_withdraw(
    e: &Env,
    sender: &Address,
    receiver: &Address,
    owner: &Address,
    assets: i128,
    shares: i128,
) {
    let topics = (symbol_short!("withdraw"), sender, receiver, owner);
    e.events().publish(topics, (assets, shares));
}
