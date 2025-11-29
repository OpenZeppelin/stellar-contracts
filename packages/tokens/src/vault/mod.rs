pub mod storage;

#[cfg(test)]
mod test;

#[cfg(not(feature = "certora"))]
use soroban_sdk::{contractevent};

#[cfg(feature = "certora")]
use cvlr_soroban_derive::contractevent;

use soroban_sdk::{contracterror, Address, Env};
pub use storage::Vault;

use crate::fungible::FungibleToken;

/// Vault Trait for Fungible Token
///
/// The `FungibleVault` trait implements the ERC-4626 tokenized vault standard,
/// enabling fungible tokens to represent shares in an underlying asset pool.
/// This extension allows users to deposit underlying assets in exchange for
/// vault shares, and later redeem those shares for the underlying assets.
///
/// The vault maintains a conversion rate between shares and assets based on
/// the total supply of shares and total assets held by the vault contract.
///
/// # Design Overview
///
/// This trait provides both high-level and low-level functions:
///
/// - **High-Level Functions**: Include necessary checks, validations, and event
///   emissions for secure vault operations.
/// - **Low-Level Functions**: Offer granular control for custom workflows
///   requiring manual authorization handling.
///
/// # Security Considerations
///
/// ⚠️ **IMPORTANT**: Most low-level functions for this trait bypass
/// authorization checks by design. It is the implementer's responsibility to
/// add appropriate access controls, typically by combining with Ownable or
/// Access Control patterns.
///
/// # Compatibility
///
/// This implementation follows the ERC-4626 standard for tokenized vaults,
/// providing familiar interfaces for Ethereum developers while leveraging
/// Stellar's unique capabilities.
pub trait FungibleVault: FungibleToken<ContractType = Vault> {
    /// Returns the address of the underlying asset that the vault manages.
    ///
    /// # Arguments
    ///
    /// * `e` - Access to the Soroban environment.
    ///
    /// # Errors
    ///
    /// * [`crate::vault::VaultTokenError::VaultAssetAddressNotSet`] - When the
    ///   vault's underlying asset address has not been initialized.
    fn query_asset(e: &Env) -> Address;

    /// Returns the total amount of underlying assets held by the vault.
    ///
    /// This represents the vault's balance of the underlying asset, which
    /// determines the conversion rate between shares and assets.
    ///
    /// # Arguments
    ///
    /// * `e` - Access to the Soroban environment.
    ///
    /// # Errors
    ///
    /// * [`crate::vault::VaultTokenError::VaultAssetAddressNotSet`] - When the
    ///   vault's underlying asset address has not been initialized.
    fn total_assets(e: &Env) -> i128;

    /// Converts an amount of underlying assets to the equivalent amount of
    /// vault shares (rounded down).
    ///
    /// # Arguments
    ///
    /// * `e` - Access to the Soroban environment.
    /// * `assets` - The amount of underlying assets to convert.
    ///
    /// # Errors
    ///
    /// * [`crate::vault::VaultTokenError::VaultInvalidAssetsAmount`] - When
    ///   assets < 0.
    /// * [`crate::vault::VaultTokenError::MathOverflow`] - When mathematical
    ///   operations result in overflow.
    fn convert_to_shares(e: &Env, assets: i128) -> i128;

    /// Converts an amount of vault shares to the equivalent amount of
    /// underlying assets (rounded down).
    ///
    /// # Arguments
    ///
    /// * `e` - Access to the Soroban environment.
    /// * `shares` - The amount of vault shares to convert.
    ///
    /// # Errors
    ///
    /// * [`crate::vault::VaultTokenError::VaultInvalidSharesAmount`] - When
    ///   shares < 0.
    /// * [`crate::vault::VaultTokenError::MathOverflow`] - When mathematical
    ///   operations result in overflow.
    fn convert_to_assets(e: &Env, shares: i128) -> i128;

    /// Returns the maximum amount of underlying assets that can be deposited
    /// for the given receiver address (currently `i128::MAX`).
    ///
    /// # Arguments
    ///
    /// * `e` - Access to the Soroban environment.
    /// * `receiver` - The address that would receive the vault shares.
    fn max_deposit(e: &Env, receiver: Address) -> i128;

    /// Simulates and returns the amount of vault shares that would be minted
    /// for a given deposit of underlying assets (rounded down).
    ///
    /// # Arguments
    ///
    /// * `e` - Access to the Soroban environment.
    /// * `assets` - The amount of underlying assets to simulate depositing.
    ///
    /// # Errors
    ///
    /// * [`crate::vault::VaultTokenError::VaultInvalidAssetsAmount`] - When
    ///   assets < 0.
    /// * [`crate::vault::VaultTokenError::MathOverflow`] - When mathematical
    ///   operations result in overflow.
    fn preview_deposit(e: &Env, assets: i128) -> i128;

    /// Deposits underlying assets into the vault and mints vault shares
    /// to the receiver, returning the amount of vault shares minted.
    ///
    /// # Arguments
    ///
    /// * `e` - Access to the Soroban environment.
    /// * `assets` - The amount of underlying assets to deposit.
    /// * `receiver` - The address that will receive the minted vault shares.
    /// * `from` - The address that will provide the underlying assets.
    /// * `operator` - The address performing the deposit operation.
    ///
    /// # Errors
    ///
    /// * [`crate::vault::VaultTokenError::VaultExceededMaxDeposit`] - When
    ///   attempting to deposit more assets than the maximum allowed for the
    ///   receiver.
    /// * [`crate::vault::VaultTokenError::VaultInvalidAssetsAmount`] - When
    ///   `assets < 0`.
    /// * [`crate::vault::VaultTokenError::MathOverflow`] - When mathematical
    ///   operations result in overflow.
    ///
    /// # Events
    ///
    /// * topics - `["deposit", operator: Address, from: Address, receiver:
    ///   Address]`
    /// * data - `[assets: i128, shares: i128]`
    ///
    /// # Security Warning
    ///
    /// ⚠️ SECURITY RISK: This function has NO AUTHORIZATION CONTROLS ⚠️
    ///
    /// Authorization for the operator must be handled at a higher level.
    fn deposit(e: &Env, assets: i128, receiver: Address, from: Address, operator: Address) -> i128;

    /// Returns the maximum amount of vault shares that can be minted
    /// for the given receiver address (currently `i128::MAX`).
    ///
    /// # Arguments
    ///
    /// * `e` - Access to the Soroban environment.
    /// * `receiver` - The address that would receive the vault shares.
    fn max_mint(e: &Env, receiver: Address) -> i128;

    /// Simulates and returns the amount of underlying assets required to mint
    /// a given amount of vault shares (rounded up).
    ///
    /// # Arguments
    ///
    /// * `e` - Access to the Soroban environment.
    /// * `shares` - The amount of vault shares to simulate minting.
    ///
    /// # Errors
    ///
    /// * [`crate::vault::VaultTokenError::VaultInvalidSharesAmount`] - When
    ///   shares < 0.
    /// * [`crate::vault::VaultTokenError::MathOverflow`] - When mathematical
    ///   operations result in overflow.
    fn preview_mint(e: &Env, shares: i128) -> i128;

    /// Mints a specific amount of vault shares to the receiver by depositing
    /// the required amount of underlying assets, returning the amount of assets
    /// deposited.
    ///
    /// # Arguments
    ///
    /// * `e` - Access to the Soroban environment.
    /// * `shares` - The amount of vault shares to mint.
    /// * `receiver` - The address that will receive the minted vault shares.
    /// * `from` - The address that will provide the underlying assets.
    /// * `operator` - The address performing the mint operation.
    ///
    /// # Errors
    ///
    /// * [`crate::vault::VaultTokenError::VaultExceededMaxMint`] - When
    ///   attempting to mint more shares than the maximum allowed for the
    ///   receiver.
    /// * [`crate::vault::VaultTokenError::VaultInvalidSharesAmount`] - When
    ///   `shares < 0`.
    /// * [`crate::vault::VaultTokenError::MathOverflow`] - When mathematical
    ///   operations result in overflow.
    ///
    /// # Events
    ///
    /// * topics - `["deposit", operator: Address, from: Address, receiver:
    ///   Address]`
    /// * data - `[assets: i128, shares: i128]`
    ///
    /// # Security Warning
    ///
    /// ⚠️ SECURITY RISK: This function has NO AUTHORIZATION CONTROLS ⚠️
    ///
    /// Authorization for the operator must be handled at a higher level.
    fn mint(e: &Env, shares: i128, receiver: Address, from: Address, operator: Address) -> i128;

    /// Returns the maximum amount of underlying assets that can be
    /// withdrawn by the given owner, limited by their vault share balance.
    ///
    /// # Arguments
    ///
    /// * `e` - Access to the Soroban environment.
    /// * `owner` - The address that owns the vault shares.
    ///
    /// # Errors
    ///
    /// * [`crate::vault::VaultTokenError::VaultInvalidSharesAmount`] - When
    ///   shares < 0.
    /// * [`crate::vault::VaultTokenError::MathOverflow`] - When mathematical
    ///   operations result in overflow.
    fn max_withdraw(e: &Env, owner: Address) -> i128;

    /// Simulates and returns the amount of vault shares that would be burned
    /// to withdraw a given amount of underlying assets (rounded up).
    ///
    /// # Arguments
    ///
    /// * `e` - Access to the Soroban environment.
    /// * `assets` - The amount of underlying assets to simulate withdrawing.
    ///
    /// # Errors
    ///
    /// * [`crate::vault::VaultTokenError::VaultInvalidAssetsAmount`] - When
    ///   assets < 0.
    /// * [`crate::vault::VaultTokenError::MathOverflow`] - When mathematical
    ///   operations result in overflow.
    fn preview_withdraw(e: &Env, assets: i128) -> i128;

    /// Withdraws a specific amount of underlying assets from the vault
    /// by burning the required amount of vault shares from the owner,
    /// returning the amount of vault shares burned.
    ///
    /// # Arguments
    ///
    /// * `e` - Access to the Soroban environment.
    /// * `assets` - The amount of underlying assets to withdraw.
    /// * `receiver` - The address that will receive the underlying assets.
    /// * `owner` - The address that owns the vault shares to be burned.
    /// * `operator` - The address performing the withdrawal operation.
    ///
    /// # Errors
    ///
    /// * [`crate::vault::VaultTokenError::VaultExceededMaxWithdraw`] - When
    ///   attempting to withdraw more assets than the maximum allowed for the
    ///   owner.
    ///
    /// # Events
    ///
    /// * topics - `["withdraw", operator: Address, receiver: Address, owner:
    ///   Address]`
    /// * data - `[assets: i128, shares: i128]`
    ///
    /// # Security Warning
    ///
    /// ⚠️ SECURITY RISK: This function has NO AUTHORIZATION CONTROLS ⚠️
    ///
    /// Authorization for the operator must be handled at a higher level.
    fn withdraw(
        e: &Env,
        assets: i128,
        receiver: Address,
        owner: Address,
        operator: Address,
    ) -> i128;

    /// Returns the maximum amount of vault shares that can be redeemed
    /// by the given owner (equal to their vault share balance).
    ///
    /// # Arguments
    ///
    /// * `e` - Access to the Soroban environment.
    /// * `owner` - The address that owns the vault shares.
    fn max_redeem(e: &Env, owner: Address) -> i128;

    /// Simulates and returns the amount of underlying assets that would be
    /// received for redeeming a given amount of vault shares (rounded down).
    ///
    /// # Arguments
    ///
    /// * `e` - Access to the Soroban environment.
    /// * `shares` - The amount of vault shares to simulate redeeming.
    ///
    /// # Errors
    ///
    /// * [`crate::vault::VaultTokenError::VaultInvalidSharesAmount`] - When
    ///   shares < 0.
    /// * [`crate::vault::VaultTokenError::MathOverflow`] - When mathematical
    ///   operations result in overflow.
    fn preview_redeem(e: &Env, shares: i128) -> i128;

    /// Redeems a specific amount of vault shares for underlying assets,
    /// returning the amount of underlying assets received.
    ///
    /// # Arguments
    ///
    /// * `e` - Access to the Soroban environment.
    /// * `shares` - The amount of vault shares to redeem.
    /// * `receiver` - The address that will receive the underlying assets.
    /// * `owner` - The address that owns the vault shares to be burned.
    /// * `operator` - The address performing the redemption operation.
    ///
    /// # Errors
    ///
    /// * [`crate::vault::VaultTokenError::VaultExceededMaxRedeem`] - When
    ///   attempting to redeem more shares than the maximum allowed for the
    ///   owner.
    /// * [`crate::vault::VaultTokenError::VaultInvalidSharesAmount`] - When
    ///   `shares < 0`.
    /// * [`crate::vault::VaultTokenError::MathOverflow`] - When mathematical
    ///   operations result in overflow.
    ///
    /// # Events
    ///
    /// * topics - `["withdraw", operator: Address, receiver: Address, owner:
    ///   Address]`
    /// * data - `[assets: i128, shares: i128]`
    ///
    /// # Security Warning
    ///
    /// ⚠️ SECURITY RISK: This function has NO AUTHORIZATION CONTROLS ⚠️
    ///
    /// Authorization for the operator must be handled at a higher level.
    fn redeem(e: &Env, shares: i128, receiver: Address, owner: Address, operator: Address) -> i128;
}

// ################## ERRORS ##################

#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq, PartialOrd, Ord)]
#[repr(u32)]
pub enum VaultTokenError {
    /// Indicates access to uninitialized vault asset address.
    VaultAssetAddressNotSet = 400,
    /// Indicates that vault asset address is already set.
    VaultAssetAddressAlreadySet = 401,
    /// Indicates that vault virtual decimals offset is already set.
    VaultVirtualDecimalsOffsetAlreadySet = 402,
    /// Indicates the amount is not a valid vault assets value.
    VaultInvalidAssetsAmount = 403,
    /// Indicates the amount is not a valid vault shares value.
    VaultInvalidSharesAmount = 404,
    /// Attempted to deposit more assets than the max amount for address.
    VaultExceededMaxDeposit = 405,
    /// Attempted to mint more shares than the max amount for address.
    VaultExceededMaxMint = 406,
    /// Attempted to withdraw more assets than the max amount for address.
    VaultExceededMaxWithdraw = 407,
    /// Attempted to redeem more shares than the max amount for address.
    VaultExceededMaxRedeem = 408,
    /// Maximum number of decimals offset exceeded
    VaultMaxDecimalsOffsetExceeded = 409,
    /// Indicates overflow due to mathematical operations
    MathOverflow = 410,
}

// ################## CONSTANTS ##################

// Suggested upper-bound for decimals to maximize both security and UX
pub const MAX_DECIMALS_OFFSET: u32 = 10;

// ################## EVENTS ##################

/// Event emitted when underlying assets are deposited into the vault.
#[contractevent]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Deposit {
    #[topic]
    pub operator: Address,
    #[topic]
    pub from: Address,
    #[topic]
    pub receiver: Address,
    pub assets: i128,
    pub shares: i128,
}

/// Emits an event when underlying assets are deposited into the vault in
/// exchange for shares.
///
/// # Arguments
///
/// * `e` - Access to Soroban environment.
/// * `operator` - The address that initiated the deposit transaction.
/// * `from` - The address that will provide the underlying assets.
/// * `receiver` - The address that will own the vault shares being minted.
/// * `assets` - The amount of underlying assets being deposited into the vault.
/// * `shares` - The amount of vault shares being minted in exchange for the
///   assets.
#[cfg(not(feature = "certora"))]
pub fn emit_deposit(
    e: &Env,
    operator: &Address,
    from: &Address,
    receiver: &Address,
    assets: i128,
    shares: i128,
) {
    Deposit {
        operator: operator.clone(),
        from: from.clone(),
        receiver: receiver.clone(),
        assets,
        shares,
    }
    .publish(e);
}

/// Event emitted when shares are exchanged back for underlying assets.
#[contractevent]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Withdraw {
    #[topic]
    pub operator: Address,
    #[topic]
    pub receiver: Address,
    #[topic]
    pub owner: Address,
    pub assets: i128,
    pub shares: i128,
}

/// Emits an event when shares are exchanged back for underlying assets and
/// assets are withdrawn from the vault.
///
/// # Arguments
///
/// * `e` - Access to Soroban environment.
/// * `operator` - The address that initiated the withdrawal transaction.
/// * `receiver` - The address that will receive the underlying assets being
///   withdrawn.
/// * `owner` - The address that owns the vault shares being burned.
/// * `assets` - The amount of underlying assets being withdrawn from the vault.
/// * `shares` - The amount of vault shares being burned in exchange for the
///   assets.
#[cfg(not(feature = "certora"))]
pub fn emit_withdraw(
    e: &Env,
    operator: &Address,
    receiver: &Address,
    owner: &Address,
    assets: i128,
    shares: i128,
) {
    Withdraw {
        operator: operator.clone(),
        receiver: receiver.clone(),
        owner: owner.clone(),
        assets,
        shares,
    }
    .publish(e);
}
