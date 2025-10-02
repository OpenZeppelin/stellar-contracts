use soroban_sdk::{contracttype, panic_with_error, token, Address, Env};
use stellar_contract_utils::math::fixed_point::{muldiv, Rounding};

use crate::fungible::{
    vault::{emit_deposit, emit_withdraw},
    Base, ContractOverrides, FungibleTokenError,
};

pub struct Vault;

impl ContractOverrides for Vault {
    fn decimals(e: &Env) -> u32 {
        Vault::decimals(e)
    }
}

/// Storage keys for the data associated with the vault extension
#[contracttype]
pub enum VaultStorageKey {
    /// Stores the address of the vault's underlying asset
    AssetAddress,
    /// Stores the virtual decimals offset of the vault
    VirtualDecimalsOffset,
}

impl Vault {
    // ################## QUERY STATE ##################

    /// Returns the contract address of the underlying asset that the vault
    /// manages.
    ///
    /// # Arguments
    ///
    /// * `e` - Access to the Soroban environment.
    ///
    /// # Errors
    ///
    /// * [`FungibleTokenError::VaultAssetAddressNotSet`] - When the vault's
    ///   underlying asset address has not been initialized.
    ///
    /// # ERC-4626 Compliance Note
    ///
    /// ⚠️ **DEVIATION FROM ERC-4626 SPECIFICATION** ⚠️
    ///
    /// The ERC-4626 standard requires that `asset()` MUST NOT revert. However,
    /// this implementation will panic if the underlying asset address has not
    /// been set during vault initialization.
    ///
    /// **Rationale**: Unlike EVM which has a "zero address" (0x0) concept,
    /// Soroban's type system does not provide a natural sentinel value for
    /// uninitialized addresses. Returning an `Option<Address>` would break
    /// ERC-4626 compatibility, while using an arbitrary sentinel address is
    /// not idiomatic in Soroban.
    ///
    /// **Mitigation**: Implementers MUST ensure that [`Self::set_asset()`] is
    /// called during contract initialization (typically in the constructor)
    /// before any vault operations are performed. Once properly initialized,
    /// this function will never revert during normal vault operations.
    ///
    /// **Impact**: This deviation affects [`Self::total_assets()`] and all
    /// conversion functions that depend on it. All these functions will panic
    /// if called before the vault is properly initialized.
    pub fn query_asset(e: &Env) -> Address {
        e.storage()
            .instance()
            .get(&VaultStorageKey::AssetAddress)
            .unwrap_or_else(|| panic_with_error!(e, FungibleTokenError::VaultAssetAddressNotSet))
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
    /// * refer to [`Self::query_asset()`] errors.
    ///
    /// # ERC-4626 Compliance Note
    ///
    /// This function inherits the revert behavior from [`Self::query_asset()`].
    /// See the ERC-4626 Compliance Note in that function's documentation for
    /// details on the deviation from the standard.
    pub fn total_assets(e: &Env) -> i128 {
        let token_client = token::Client::new(e, &Self::query_asset(e));
        token_client.balance(&e.current_contract_address())
    }

    /// Converts an amount of underlying assets to the equivalent amount of
    /// vault shares (rounded down) using an idealized, fee-neutral conversion
    /// rate.
    ///
    /// This function provides the theoretical conversion rate without
    /// considering fees or other conditions that might affect actual
    /// deposit outcomes.
    ///
    /// # Arguments
    ///
    /// * `e` - Access to the Soroban environment.
    /// * `assets` - The amount of underlying assets to convert.
    ///
    /// # Errors
    ///
    /// * refer to [`Self::convert_to_shares_with_rounding()`] errors.
    pub fn convert_to_shares(e: &Env, assets: i128) -> i128 {
        Self::convert_to_shares_with_rounding(e, assets, Rounding::Floor)
    }

    /// Converts an amount of vault shares to the equivalent amount of
    /// underlying assets (rounded down) using an idealized, fee-neutral
    /// conversion rate.
    ///
    /// This function provides the theoretical conversion rate without
    /// considering fees or other conditions that might affect actual
    /// redemption outcomes.
    ///
    /// # Arguments
    ///
    /// * `e` - Access to the Soroban environment.
    /// * `shares` - The amount of vault shares to convert.
    ///
    /// # Errors
    ///
    /// * refer to [`Self::convert_to_assets_with_rounding()`] errors.
    pub fn convert_to_assets(e: &Env, shares: i128) -> i128 {
        Self::convert_to_assets_with_rounding(e, shares, Rounding::Floor)
    }

    /// Returns the maximum amount of underlying assets that can be deposited
    /// for the given receiver address (currently `i64::MAX`).
    ///
    /// # Arguments
    ///
    /// * `e` - Access to the Soroban environment.
    /// * `receiver` - The address that would receive the vault shares.
    pub fn max_deposit(_e: &Env, _receiver: Address) -> i128 {
        i64::MAX as i128
    }

    /// Simulates and returns the amount of vault shares that would be minted
    /// for a given deposit of underlying assets (rounded down).
    ///
    /// This function provides the exact outcome of a deposit operation under
    /// current conditions, including any fees or other conditions that might
    /// reduce the shares received compared to the idealized conversion rate.
    ///
    /// # Arguments
    ///
    /// * `e` - Access to the Soroban environment.
    /// * `assets` - The amount of underlying assets to simulate depositing.
    ///
    /// # Errors
    ///
    /// * refer to [`Self::convert_to_shares_with_rounding()`] errors.
    pub fn preview_deposit(e: &Env, assets: i128) -> i128 {
        Self::convert_to_shares_with_rounding(e, assets, Rounding::Floor)
    }

    /// Returns the maximum amount of vault shares that can be minted
    /// for the given receiver address (currently `i64::MAX`).
    ///
    /// # Arguments
    ///
    /// * `e` - Access to the Soroban environment.
    /// * `receiver` - The address that would receive the vault shares.
    pub fn max_mint(_e: &Env, _receiver: Address) -> i128 {
        i64::MAX as i128
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
    /// * refer to [`Self::convert_to_assets_with_rounding()`] errors.
    pub fn preview_mint(e: &Env, shares: i128) -> i128 {
        Self::convert_to_assets_with_rounding(e, shares, Rounding::Ceil)
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
    /// * refer to [`Self::convert_to_assets_with_rounding()`] errors.
    pub fn max_withdraw(e: &Env, owner: Address) -> i128 {
        Self::convert_to_assets_with_rounding(e, Self::balance(e, &owner), Rounding::Floor)
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
    /// * refer to [`Self::convert_to_shares_with_rounding()`] errors.
    pub fn preview_withdraw(e: &Env, assets: i128) -> i128 {
        Self::convert_to_shares_with_rounding(e, assets, Rounding::Ceil)
    }

    /// Returns the maximum amount of vault shares that can be redeemed
    /// by the given owner (equal to their vault share balance).
    ///
    /// # Arguments
    ///
    /// * `e` - Access to the Soroban environment.
    /// * `owner` - The address that owns the vault shares.
    pub fn max_redeem(e: &Env, owner: Address) -> i128 {
        Self::balance(e, &owner)
    }

    /// Simulates and returns the amount of underlying assets that would be
    /// received for redeeming a given amount of vault shares (rounded down).
    ///
    /// This function provides the exact outcome of a redemption operation under
    /// current conditions, including any fees or other conditions that might
    /// reduce the assets received compared to the idealized conversion rate.
    ///
    /// # Arguments
    ///
    /// * `e` - Access to the Soroban environment.
    /// * `shares` - The amount of vault shares to simulate redeeming.
    ///
    /// # Errors
    ///
    /// * refer to [`Self::convert_to_assets_with_rounding()`] errors.
    pub fn preview_redeem(e: &Env, shares: i128) -> i128 {
        Self::convert_to_assets_with_rounding(e, shares, Rounding::Floor)
    }

    // ################## CHANGE STATE ##################

    /// Deposits underlying assets from the `from` address into the vault and
    /// mints vault shares to the receiver, returning the amount of vault
    /// shares minted.
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
    /// * [`FungibleTokenError::VaultExceededMaxDeposit`] - When attempting to
    ///   deposit more assets than the maximum allowed for the receiver.
    /// * also refer to [`Self::preview_deposit()`] errors.
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
    /// It is the responsibility of the implementer to establish appropriate
    /// access controls to ensure that only authorized accounts can execute
    /// deposit operations. Consider combining with the Ownable or Access
    /// Control pattern, or using `operator.require_auth()`.
    pub fn deposit(
        e: &Env,
        assets: i128,
        receiver: Address,
        from: Address,
        operator: Address,
    ) -> i128 {
        let max_assets = Self::max_deposit(e, receiver.clone());
        if assets > max_assets {
            panic_with_error!(e, FungibleTokenError::VaultExceededMaxDeposit);
        }
        let shares: i128 = Self::preview_deposit(e, assets);
        Self::deposit_internal(e, &receiver, assets, shares, &from, &operator);
        shares
    }

    /// Mints a specific amount of vault shares to the receiver by depositing
    /// the required amount of underlying assets from the `from` address,
    /// returning the amount of assets deposited.
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
    /// * [`FungibleTokenError::VaultExceededMaxMint`] - When attempting to mint
    ///   more shares than the maximum allowed for the receiver.
    /// * also refer to [`Self::preview_mint()`] errors.
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
    /// It is the responsibility of the implementer to establish appropriate
    /// access controls to ensure that only authorized accounts can execute
    /// mint operations. Consider combining with the Ownable or Access
    /// Control pattern, or using `operator.require_auth()`.
    pub fn mint(
        e: &Env,
        shares: i128,
        receiver: Address,
        from: Address,
        operator: Address,
    ) -> i128 {
        let max_shares = Self::max_mint(e, receiver.clone());
        if shares > max_shares {
            panic_with_error!(e, FungibleTokenError::VaultExceededMaxMint);
        }
        let assets: i128 = Self::preview_mint(e, shares);
        Self::deposit_internal(e, &receiver, assets, shares, &from, &operator);
        assets
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
    /// * [`FungibleTokenError::VaultExceededMaxWithdraw`] - When attempting to
    ///   withdraw more assets than the maximum allowed for the owner.
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
    /// It is the responsibility of the implementer to establish appropriate
    /// access controls to ensure that only authorized accounts can execute
    /// withdrawal operations. Consider combining with the Ownable or Access
    /// Control pattern, or using `operator.require_auth()`.
    pub fn withdraw(
        e: &Env,
        assets: i128,
        receiver: Address,
        owner: Address,
        operator: Address,
    ) -> i128 {
        let max_assets = Self::max_withdraw(e, owner.clone());
        if assets > max_assets {
            panic_with_error!(e, FungibleTokenError::VaultExceededMaxWithdraw);
        }
        let shares: i128 = Self::preview_withdraw(e, assets);
        Self::withdraw_internal(e, &receiver, &owner, assets, shares, &operator);
        shares
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
    /// * [`FungibleTokenError::VaultExceededMaxRedeem`] - When attempting to
    ///   redeem more shares than the maximum allowed for the owner.
    /// * also refer to [`Self::preview_redeem()`] errors.
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
    /// It is the responsibility of the implementer to establish appropriate
    /// access controls to ensure that only authorized accounts can execute
    /// redemption operations. Consider combining with the Ownable or Access
    /// Control pattern, or using `operator.require_auth()`.
    pub fn redeem(
        e: &Env,
        shares: i128,
        receiver: Address,
        owner: Address,
        operator: Address,
    ) -> i128 {
        let max_shares = Self::max_redeem(e, owner.clone());
        if shares > max_shares {
            panic_with_error!(e, FungibleTokenError::VaultExceededMaxRedeem);
        }
        let assets = Self::preview_redeem(e, shares);
        Self::withdraw_internal(e, &receiver, &owner, assets, shares, &operator);
        assets
    }

    // ################## OVERRIDDEN FUNCTIONS ##################

    /// Returns the number of decimals used to represent vault shares.
    ///
    /// Decimals are computed by adding the decimal offset on top of the
    /// underlying asset's decimals. This provides additional precision for
    /// share calculations and helps prevent rounding errors in vault
    /// operations.
    ///
    /// # Arguments
    ///
    /// * `e` - Access to the Soroban environment.
    ///
    /// # Errors
    ///
    /// * [`FungibleTokenError::MathOverflow`] - When the sum of underlying
    ///   asset decimals and offset exceeds the maximum value.
    pub fn decimals(e: &Env) -> u32 {
        Self::get_underlying_asset_decimals(e)
            .checked_add(Self::get_decimals_offset(e))
            .unwrap_or_else(|| panic_with_error!(e, FungibleTokenError::MathOverflow))
    }

    // ################## LOW-LEVEL HELPERS ##################

    /// Sets the address of the underlying asset that the vault will manage.
    ///
    /// Address of the asset contract is not checked here. It is the
    /// responsibility of the implementer to ensure that the asset address
    /// is valid and present.
    ///
    /// This function should typically be called once during contract
    /// initialization and the asset address should remain immutable thereafter.
    ///
    /// # Arguments
    ///
    /// * `e` - Access to the Soroban environment.
    /// * `asset` - The address of the underlying asset contract.
    ///
    /// # Errors
    ///
    /// * [`FungibleTokenError::VaultAssetAddressAlreadySet`] - When attempting
    ///   to set the asset address after it has already been initialized.
    ///
    /// # Security Warning
    ///
    /// ⚠️ SECURITY RISK: This function has NO AUTHORIZATION CONTROLS ⚠️
    ///
    /// It is the responsibility of the implementer to establish appropriate
    /// access controls to ensure that only authorized accounts can set the
    /// asset address. We recommend using this function in the constructor of
    /// your smart contract or combining with the Ownable or Access Control
    /// pattern.
    pub fn set_asset(e: &Env, asset: Address) {
        // Check if asset is already set
        if e.storage().instance().has(&VaultStorageKey::AssetAddress) {
            panic_with_error!(e, FungibleTokenError::VaultAssetAddressAlreadySet);
        }

        e.storage().instance().set(&VaultStorageKey::AssetAddress, &asset);
    }

    /// Sets the virtual decimals offset for the vault.
    ///
    /// The decimals offset adds extra precision to vault share calculations,
    /// helping to prevent rounding errors and improve the accuracy of
    /// share-to-asset conversions. This should typically be set once during
    /// contract initialization and remain immutable thereafter.
    ///
    /// # Arguments
    ///
    /// * `e` - Access to the Soroban environment.
    /// * `offset` - The number of additional decimal places to add.
    ///
    /// # Errors
    ///
    /// * [`FungibleTokenError::VaultVirtualDecimalsOffsetAlreadySet`] - When
    ///   attempting to set the offset after it has already been initialized.
    ///
    /// # Security Warning
    ///
    /// ⚠️ SECURITY RISK: This function has NO AUTHORIZATION CONTROLS ⚠️
    ///
    /// It is the responsibility of the implementer to establish appropriate
    /// access controls to ensure that only authorized accounts can set the
    /// decimals offset. We recommend using this function in the constructor
    /// of your smart contract or combining with the Ownable or Access Control
    /// pattern.
    pub fn set_decimals_offset(e: &Env, offset: u32) {
        // Check if virtual decimals offset is already set
        if e.storage().instance().has(&VaultStorageKey::VirtualDecimalsOffset) {
            panic_with_error!(e, FungibleTokenError::VaultVirtualDecimalsOffsetAlreadySet);
        }
        e.storage().instance().set(&VaultStorageKey::VirtualDecimalsOffset, &offset);
    }

    /// Internal conversion function from assets to shares with support for
    /// rounding direction, returning the equivalent amount of vault shares.
    ///
    /// Implements the formula:
    /// `assets.mulDiv(totalSupply() + 10 ** decimalsOffset(), totalAssets() +
    /// 1, rounding)`
    ///
    /// # Arguments
    ///
    /// * `e` - Access to the Soroban environment.
    /// * `assets` - The amount of underlying assets to convert.
    /// * `rounding` - The rounding direction to use for the conversion.
    ///
    /// # Errors
    ///
    /// * [`FungibleTokenError::VaultInvalidAssetsAmount`] - When `assets < 0`.
    /// * [`FungibleTokenError::MathOverflow`] - When mathematical operations
    ///   result in overflow.
    pub fn convert_to_shares_with_rounding(e: &Env, assets: i128, rounding: Rounding) -> i128 {
        if assets < 0 {
            panic_with_error!(e, FungibleTokenError::VaultInvalidAssetsAmount);
        }
        if assets == 0 {
            return 0;
        }
        let x = assets;
        let pow = 10_i128
            .checked_pow(Self::get_decimals_offset(e))
            .unwrap_or_else(|| panic_with_error!(e, FungibleTokenError::MathOverflow));
        let y = Self::total_supply(e)
            .checked_add(pow)
            .unwrap_or_else(|| panic_with_error!(e, FungibleTokenError::MathOverflow));
        let denominator = Self::total_assets(e)
            .checked_add(1_i128)
            .unwrap_or_else(|| panic_with_error!(e, FungibleTokenError::MathOverflow));
        muldiv(e, x, y, denominator, rounding)
    }

    /// Internal conversion function from shares to assets with support for
    /// rounding direction, returning the equivalent amount of underlying
    /// assets.
    ///
    /// Implements the formula:
    /// `shares.mulDiv(totalAssets() + 1, totalSupply() + 10 **
    /// decimalsOffset(), rounding)`
    ///
    /// # Arguments
    ///
    /// * `e` - Access to the Soroban environment.
    /// * `shares` - The amount of vault shares to convert.
    /// * `rounding` - The rounding direction to use for the conversion.
    ///
    /// # Errors
    ///
    /// * [`FungibleTokenError::VaultInvalidSharesAmount`] - When `shares < 0`.
    /// * [`FungibleTokenError::MathOverflow`] - When mathematical operations
    ///   result in overflow.
    pub fn convert_to_assets_with_rounding(e: &Env, shares: i128, rounding: Rounding) -> i128 {
        if shares < 0 {
            panic_with_error!(e, FungibleTokenError::VaultInvalidSharesAmount);
        }
        if shares == 0 {
            return 0;
        }
        let x = shares;
        let y = Self::total_assets(e)
            .checked_add(1_i128)
            .unwrap_or_else(|| panic_with_error!(e, FungibleTokenError::MathOverflow));
        let pow = 10_i128
            .checked_pow(Self::get_decimals_offset(e))
            .unwrap_or_else(|| panic_with_error!(e, FungibleTokenError::MathOverflow));
        let denominator = Self::total_supply(e)
            .checked_add(pow)
            .unwrap_or_else(|| panic_with_error!(e, FungibleTokenError::MathOverflow));
        muldiv(e, x, y, denominator, rounding)
    }

    /// Internal deposit/mint workflow without authorization checks.
    ///
    /// This function handles the core logic for depositing assets and minting
    /// shares, including transferring assets to the vault and emitting events.
    ///
    /// # Arguments
    ///
    /// * `e` - Access to the Soroban environment.
    /// * `receiver` - The address that will receive the minted vault shares.
    /// * `assets` - The amount of underlying assets being deposited.
    /// * `shares` - The amount of vault shares being minted.
    /// * `from` - The address that will provide the underlying assets.
    /// * `operator` - The address performing the deposit operation.
    ///
    /// # Events
    ///
    /// * topics - `["deposit", operator: Address, from: Address, receiver:
    ///   Address]`
    /// * data - `[assets: i128, shares: i128]`
    ///
    /// # Notes
    ///
    /// This function assumes prior authorization of the operator and validation
    /// of amounts. When `operator != from`, the operator must have sufficient
    /// allowance from `from` on the underlying asset contract. When `operator
    /// == from`, the transfer is direct. It should only be called from
    /// higher-level functions that handle authorization concerns.
    pub fn deposit_internal(
        e: &Env,
        receiver: &Address,
        assets: i128,
        shares: i128,
        from: &Address,
        operator: &Address,
    ) {
        // This function assumes prior authorization of the operator and validation of
        // amounts.
        let token_client = token::Client::new(e, &Self::query_asset(e));
        // `safeTransfer` mechanism is not present in the base module, (will be provided
        // as an extension)

        if operator == from {
            // Direct transfer: `operator` is depositing their own assets
            token_client.transfer(from, e.current_contract_address(), &assets);
        } else {
            // Allowance-based transfer: `operator` is depositing on behalf of `from`
            // This requires that `from` has approved `operator` on the underlying asset
            token_client.transfer_from(operator, from, &e.current_contract_address(), &assets);
        }

        Base::mint(e, receiver, shares);
        emit_deposit(e, operator, from, receiver, assets, shares);
    }

    /// Internal withdraw/redeem workflow without authorization checks.
    ///
    /// This function handles the core logic for burning shares and withdrawing
    /// assets, including managing allowances and emitting events.
    ///
    /// # Arguments
    ///
    /// * `e` - Access to the Soroban environment.
    /// * `receiver` - The address that will receive the underlying assets.
    /// * `owner` - The address that owns the vault shares being burned.
    /// * `assets` - The amount of underlying assets being withdrawn.
    /// * `shares` - The amount of vault shares being burned.
    /// * `operator` - The address performing the withdrawal operation.
    ///
    /// # Events
    ///
    /// * topics - `["withdraw", operator: Address, receiver: Address, owner:
    ///   Address]`
    /// * data - `[assets: i128, shares: i128]`
    ///
    /// # Notes
    ///
    /// This function assumes prior authorization of the operator and validation
    /// of amounts. It automatically handles allowance spending when the
    /// operator is different from the owner. It should only be called from
    /// higher-level functions that handle authorization concerns.
    pub fn withdraw_internal(
        e: &Env,
        receiver: &Address,
        owner: &Address,
        assets: i128,
        shares: i128,
        operator: &Address,
    ) {
        // This function assumes prior authorization of the operator and validation of
        // amounts.
        if operator != owner {
            Base::spend_allowance(e, owner, operator, shares);
        }
        Base::update(e, Some(owner), None, shares);
        let token_client = token::Client::new(e, &Self::query_asset(e));
        // `safeTransfer` mechanism is not present in the base module, (will be provided
        // as an extension)
        token_client.transfer(&e.current_contract_address(), receiver, &assets);
        emit_withdraw(e, operator, receiver, owner, assets, shares);
    }

    /// Returns the virtual decimals offset for the vault (defaults to 0 if not
    /// set).
    ///
    /// The decimals offset adds extra precision to vault share calculations,
    /// helping to prevent rounding errors and improve the accuracy of
    /// share-to-asset conversions.
    ///
    /// # Arguments
    ///
    /// * `e` - Access to the Soroban environment.
    ///
    /// # Notes
    ///
    /// For more information about virtual decimals offset, see:
    /// https://docs.openzeppelin.com/contracts/5.x/erc4626
    pub fn get_decimals_offset(e: &Env) -> u32 {
        e.storage().instance().get(&VaultStorageKey::VirtualDecimalsOffset).unwrap_or(0)
    }

    /// Returns the number of decimals used by the underlying asset.
    ///
    /// This is queried from the underlying asset contract and used in
    /// calculating the vault's total decimals.
    ///
    /// # Arguments
    ///
    /// * `e` - Access to the Soroban environment.
    ///
    /// # Errors
    ///
    /// * refer to [`Self::query_asset()`] errors.
    pub fn get_underlying_asset_decimals(e: &Env) -> u32 {
        let token_client = token::Client::new(e, &Self::query_asset(e));
        token_client.decimals()
    }
}
