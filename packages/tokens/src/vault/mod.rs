pub mod storage;

#[cfg(test)]
mod test;

use soroban_sdk::{contracterror, contractevent, contracttrait, Address, Env};
pub use storage::{CappedVault, Vault, VaultOverrides};

use crate::fungible::{total_supply::FungibleTotalSupply, FungibleToken};

/// Vault Trait for Fungible Token
///
/// The `FungibleVault` trait implements the ERC-4626 tokenized vault standard,
/// enabling fungible tokens to represent shares in an underlying asset pool.
/// This extension allows users to deposit underlying assets in exchange for
/// vault shares, and later redeem those shares for the underlying assets.
///
/// The vault maintains a conversion rate between shares and assets based on
/// the total supply of shares and total assets held by the vault contract.
/// The `Vault` contract type is therefore inherently supply-aware, and
/// [`FungibleVault`] requires
/// [`crate::fungible::total_supply::FungibleTotalSupply`] to be implemented
/// alongside it (an empty `impl` block is enough), so that the supply of
/// shares is exposed by the contract. `TotalSupply` has to be listed in the
/// contract type as well, e.g. `Compose<(Vault, TotalSupply)>`.
///
/// For a maximum supply of shares, `Capped` is listed too:
/// `Compose<(Vault, Capped, TotalSupply)>` resolves to [`CappedVault`], which
/// enforces the cap on `deposit` and `mint` and reports it through
/// `max_deposit` and `max_mint`. Each method delegates to the contract type
/// through [`VaultOverrides`].
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
/// The trait methods that move assets or shares (`deposit`, `mint`,
/// `withdraw`, `redeem`) require authorization from `operator` through the
/// contract type. The low-level functions on [`Vault`] (e.g.
/// `Vault::deposit_internal`) bypass authorization checks by design. A
/// contract that calls them directly is responsible for adding the
/// appropriate access controls, typically by combining with Ownable or Access
/// Control patterns.
///
/// # Compatibility
///
/// This implementation follows the ERC-4626 standard for tokenized vaults,
/// providing familiar interfaces for Ethereum developers while leveraging
/// Stellar's unique capabilities.
#[contracttrait]
pub trait FungibleVault: FungibleTotalSupply + FungibleToken<ContractType: VaultOverrides> {
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
    fn query_asset(e: &Env) -> Address {
        Self::ContractType::query_asset(e)
    }

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
    fn total_assets(e: &Env) -> i128 {
        Self::ContractType::total_assets(e)
    }

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
    fn convert_to_shares(e: &Env, assets: i128) -> i128 {
        Self::ContractType::convert_to_shares(e, assets)
    }

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
    fn convert_to_assets(e: &Env, shares: i128) -> i128 {
        Self::ContractType::convert_to_assets(e, shares)
    }

    /// Returns the maximum amount of underlying assets that can be deposited
    /// for the given receiver address.
    ///
    /// This is `max_mint` converted to assets rounding down, so
    /// depositing this amount never mints more shares than the vault accepts.
    /// Returns `0` when the vault's totals are too large for any deposit to
    /// succeed.
    ///
    /// # Arguments
    ///
    /// * `e` - Access to the Soroban environment.
    /// * `receiver` - The address that would receive the vault shares.
    ///
    /// # Errors
    ///
    /// * [`crate::vault::VaultTokenError::VaultAssetAddressNotSet`] - When the
    ///   vault's underlying asset address has not been initialized.
    fn max_deposit(e: &Env, receiver: Address) -> i128 {
        Self::ContractType::max_deposit(e, receiver)
    }

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
    fn preview_deposit(e: &Env, assets: i128) -> i128 {
        Self::ContractType::preview_deposit(e, assets)
    }

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
    /// * [`crate::vault::VaultTokenError::VaultZeroShares`] - When a positive
    ///   amount of assets would mint zero shares.
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
    /// # Notes
    ///
    /// Authorization from `operator` is required.
    fn deposit(e: &Env, assets: i128, receiver: Address, from: Address, operator: Address) -> i128 {
        Self::ContractType::deposit(e, assets, receiver, from, operator)
    }

    /// Returns the maximum amount of vault shares that can be minted
    /// for the given receiver address.
    ///
    /// Two limits apply, and the smaller one is returned:
    ///
    /// * The share supply must stay below `i128::MAX - 10^offset`, because
    ///   every conversion computes `total_supply + 10^offset`. So at most
    ///   `i128::MAX - 10^offset - total_supply` more shares can be minted.
    /// * Minting has to be paid for in assets, and the payment cannot exceed
    ///   `i128::MAX`. Once a share is worth more than one asset base unit, this
    ///   caps the shares at what `i128::MAX` assets buy at the current price.
    ///
    /// Returns `0` when the share supply has reached its limit, or when the
    /// vault holds `i128::MAX` assets (no mint can succeed then, because
    /// `total_assets + 1` overflows).
    ///
    /// # Arguments
    ///
    /// * `e` - Access to the Soroban environment.
    /// * `receiver` - The address that would receive the vault shares.
    ///
    /// # Errors
    ///
    /// * [`crate::vault::VaultTokenError::VaultAssetAddressNotSet`] - When the
    ///   vault's underlying asset address has not been initialized.
    fn max_mint(e: &Env, receiver: Address) -> i128 {
        Self::ContractType::max_mint(e, receiver)
    }

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
    fn preview_mint(e: &Env, shares: i128) -> i128 {
        Self::ContractType::preview_mint(e, shares)
    }

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
    /// # Notes
    ///
    /// Authorization from `operator` is required.
    fn mint(e: &Env, shares: i128, receiver: Address, from: Address, operator: Address) -> i128 {
        Self::ContractType::mint(e, shares, receiver, from, operator)
    }

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
    fn max_withdraw(e: &Env, owner: Address) -> i128 {
        Self::ContractType::max_withdraw(e, owner)
    }

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
    fn preview_withdraw(e: &Env, assets: i128) -> i128 {
        Self::ContractType::preview_withdraw(e, assets)
    }

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
    /// # Notes
    ///
    /// Authorization from `operator` is required.
    fn withdraw(
        e: &Env,
        assets: i128,
        receiver: Address,
        owner: Address,
        operator: Address,
    ) -> i128 {
        Self::ContractType::withdraw(e, assets, receiver, owner, operator)
    }

    /// Returns the maximum amount of vault shares that can be redeemed
    /// by the given owner.
    ///
    /// This is the owner's vault share balance, or `0` when that balance is
    /// worth zero assets (see
    /// [`crate::vault::VaultTokenError::VaultZeroAssets`]).
    ///
    /// # Arguments
    ///
    /// * `e` - Access to the Soroban environment.
    /// * `owner` - The address that owns the vault shares.
    ///
    /// # Errors
    ///
    /// * [`crate::vault::VaultTokenError::MathOverflow`] - When mathematical
    ///   operations result in overflow.
    fn max_redeem(e: &Env, owner: Address) -> i128 {
        Self::ContractType::max_redeem(e, owner)
    }

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
    fn preview_redeem(e: &Env, shares: i128) -> i128 {
        Self::ContractType::preview_redeem(e, shares)
    }

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
    /// * [`crate::vault::VaultTokenError::VaultZeroAssets`] - When a positive
    ///   amount of shares would redeem zero assets while the owner's balance is
    ///   worth at least one unit of the asset. When the whole balance is worth
    ///   zero assets, `max_redeem` is `0` and the redemption fails with
    ///   [`crate::vault::VaultTokenError::VaultExceededMaxRedeem`] instead.
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
    /// # Notes
    ///
    /// Authorization from `operator` is required.
    fn redeem(e: &Env, shares: i128, receiver: Address, owner: Address, operator: Address) -> i128 {
        Self::ContractType::redeem(e, shares, receiver, owner, operator)
    }
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
    /// Attempted to deposit a positive amount of assets that would mint zero
    /// shares.
    VaultZeroShares = 411,
    /// Attempted to redeem a positive amount of shares that would return zero
    /// assets.
    VaultZeroAssets = 412,
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
