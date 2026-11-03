use soroban_sdk::{contracttype, panic_with_error, token, Address, Env};
use stellar_contract_utils::math::{
    i128_fixed_point::{checked_mul_div_with_rounding, mul_div_with_rounding},
    Rounding,
};

use crate::{
    fungible::{
        capped::{check_cap, query_cap, CappedContractType},
        total_supply::{
            decrease_total_supply, increase_total_supply, total_supply, TotalSupplyOverrides,
        },
        Base, ContractOverrides,
    },
    vault::{emit_deposit, emit_withdraw, VaultTokenError, MAX_DECIMALS_OFFSET},
};

pub struct Vault;

impl ContractOverrides for Vault {
    fn decimals(e: &Env) -> u32 {
        Vault::decimals(e)
    }
}

// The vault's share math requires the total supply of shares, so the vault
// contract type is inherently supply-aware.
impl TotalSupplyOverrides for Vault {}

/// Internal override hook for [`crate::vault::FungibleVault`].
///
/// # Why this trait exists
///
/// `FungibleVault` used to require `ContractType = Vault` exactly, so there
/// was only one possible vault behavior, and a contract type built on the
/// vault (such as [`CappedVault`]) could not change any of it. Every
/// `FungibleVault` method delegates to `Self::ContractType::{function_name}`,
/// the same routing `ContractOverrides` gives `FungibleToken`, and this trait
/// is what that call reaches. `FungibleVault` now requires it on the
/// contract's `ContractType`.
///
/// Every method defaults to the [`Vault`] function of the same name, so
/// [`Vault`] implements it with an empty body and behaves exactly as before.
/// A contract type built on the vault overrides only the methods it changes:
/// [`CappedVault`] overrides `max_deposit`, `deposit`, `max_mint` and `mint`,
/// and keeps the defaults for the rest.
///
/// # Note
///
/// Like `ContractOverrides`, this trait is internal plumbing of the library.
/// There is no need to implement or import it: implementing
/// [`crate::vault::FungibleVault`] with an empty body is enough, and the
/// behavior is picked based on the contract's `ContractType`.
#[diagnostic::on_unimplemented(
    message = "`{Self}` is not a vault contract type, so `FungibleVault` cannot be implemented",
    note = "list `Vault` in the `Compose` list, e.g. `Compose<(Vault, TotalSupply)>`"
)]
pub trait VaultOverrides {
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

    fn deposit(e: &Env, assets: i128, receiver: Address, from: Address, operator: Address) -> i128 {
        Vault::deposit(e, assets, receiver, from, operator)
    }

    fn max_mint(e: &Env, receiver: Address) -> i128 {
        Vault::max_mint(e, receiver)
    }

    fn preview_mint(e: &Env, shares: i128) -> i128 {
        Vault::preview_mint(e, shares)
    }

    fn mint(e: &Env, shares: i128, receiver: Address, from: Address, operator: Address) -> i128 {
        Vault::mint(e, shares, receiver, from, operator)
    }

    fn max_withdraw(e: &Env, owner: Address) -> i128 {
        Vault::max_withdraw(e, owner)
    }

    fn preview_withdraw(e: &Env, assets: i128) -> i128 {
        Vault::preview_withdraw(e, assets)
    }

    fn withdraw(
        e: &Env,
        assets: i128,
        receiver: Address,
        owner: Address,
        operator: Address,
    ) -> i128 {
        Vault::withdraw(e, assets, receiver, owner, operator)
    }

    fn max_redeem(e: &Env, owner: Address) -> i128 {
        Vault::max_redeem(e, owner)
    }

    fn preview_redeem(e: &Env, shares: i128) -> i128 {
        Vault::preview_redeem(e, shares)
    }

    fn redeem(e: &Env, shares: i128, receiver: Address, owner: Address, operator: Address) -> i128 {
        Vault::redeem(e, shares, receiver, owner, operator)
    }
}

impl VaultOverrides for Vault {}

/// Contract type enforcing a maximum supply of shares on top of [`Vault`],
/// resolved by `Compose<(Vault, Capped, TotalSupply)>`.
///
/// The cap (refer to [`crate::fungible::capped::Capped`]) bounds the number of
/// shares in circulation. It is enforced on both share-creating entry points,
/// `deposit` and `mint`, and reflected in `max_deposit` and `max_mint`, so the
/// limits the vault reports are the limits it enforces, as ERC-4626 requires.
/// Withdrawals, redemptions, conversions and previews are the [`Vault`] ones.
///
/// A deposit or mint that would exceed the cap fails with
/// [`crate::fungible::FungibleTokenError::ExceededCap`], as on every other
/// capped contract type.
pub struct CappedVault;

impl CappedContractType for CappedVault {}

// The share math of the vault requires the total supply of shares, and the
// capped variant reads the same counter.
impl TotalSupplyOverrides for CappedVault {}

impl ContractOverrides for CappedVault {
    fn decimals(e: &Env) -> u32 {
        Vault::decimals(e)
    }
}

// Only the four entry points the cap touches are overridden.
impl VaultOverrides for CappedVault {
    fn max_deposit(e: &Env, receiver: Address) -> i128 {
        CappedVault::max_deposit(e, receiver)
    }

    fn deposit(e: &Env, assets: i128, receiver: Address, from: Address, operator: Address) -> i128 {
        CappedVault::deposit(e, assets, receiver, from, operator)
    }

    fn max_mint(e: &Env, receiver: Address) -> i128 {
        CappedVault::max_mint(e, receiver)
    }

    fn mint(e: &Env, shares: i128, receiver: Address, from: Address, operator: Address) -> i128 {
        CappedVault::mint(e, shares, receiver, from, operator)
    }
}

impl CappedVault {
    /// Returns the amount of shares that can still be minted before the cap
    /// is reached, and never more than [`Vault::max_mint`]. Returns `0` when
    /// the supply is at or above the cap (the cap may have been lowered below
    /// the current supply, refer to [`crate::fungible::capped::set_cap`]).
    ///
    /// # Arguments
    ///
    /// * `e` - Access to the Soroban environment.
    /// * `receiver` - The address that would receive the vault shares.
    ///
    /// # Errors
    ///
    /// * refer to [`query_cap`] errors.
    /// * refer to [`Vault::max_mint`] errors.
    pub fn max_mint(e: &Env, receiver: Address) -> i128 {
        let remaining = query_cap(e).saturating_sub(total_supply(e)).max(0);
        remaining.min(Vault::max_mint(e, receiver))
    }

    /// Returns the amount of underlying assets that can still be deposited
    /// before the cap is reached, and never more than [`Vault::max_deposit`].
    ///
    /// This is the largest amount whose deposit mints at most the remaining
    /// shares ([`CappedVault::max_mint`]), computed as in
    /// [`Vault::max_deposit`]: depositing it never mints more shares than the
    /// cap allows, and never zero, and unless it is `i128::MAX`, depositing
    /// one more asset would exceed the cap. The edge cases are those of
    /// [`Vault::max_deposit`].
    ///
    /// # Arguments
    ///
    /// * `e` - Access to the Soroban environment.
    /// * `receiver` - The address that would receive the vault shares.
    ///
    /// # Errors
    ///
    /// * refer to [`query_cap`] errors.
    /// * refer to [`Vault::max_mint`] errors.
    pub fn max_deposit(e: &Env, receiver: Address) -> i128 {
        // `CappedVault::max_mint` is never above `Vault::max_mint`, and the
        // largest amount minting at most a number of shares grows with that
        // number, so this is never above `Vault::max_deposit`.
        Vault::max_assets_for_shares(e, CappedVault::max_mint(e, receiver))
    }

    /// Deposits `assets` if the shares it mints fit under the cap.
    ///
    /// The shares are computed with [`Vault::preview_deposit`], the same
    /// computation [`Vault::deposit`] performs against the same state, so the
    /// cap is checked against the exact amount that is minted.
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
    /// * refer to [`check_cap`] errors.
    /// * refer to [`Vault::deposit`] errors.
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
    pub fn deposit(
        e: &Env,
        assets: i128,
        receiver: Address,
        from: Address,
        operator: Address,
    ) -> i128 {
        check_cap(e, Vault::preview_deposit(e, assets), total_supply(e));
        Vault::deposit(e, assets, receiver, from, operator)
    }

    /// Mints `shares` if they fit under the cap.
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
    /// * refer to [`check_cap`] errors.
    /// * refer to [`Vault::mint`] errors.
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
    pub fn mint(
        e: &Env,
        shares: i128,
        receiver: Address,
        from: Address,
        operator: Address,
    ) -> i128 {
        check_cap(e, shares, total_supply(e));
        Vault::mint(e, shares, receiver, from, operator)
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

/// # Inflation Attack (Donation Attack) Mitigation
///
/// ## Vulnerability Overview
///
/// In empty (or nearly empty) vaults, deposits are at high risk of being stolen
/// through a "donation" to the vault that inflates the price of a share.
/// This is variously known as a **donation attack** or **inflation attack** and
/// is essentially a problem of slippage.
///
/// ## Attack Mechanism
///
/// 1. Attacker observes a pending deposit transaction in the mempool
/// 2. Attacker frontruns by directly transferring assets to the vault
///    (donation)
/// 3. This inflates the share price before the victim's deposit is processed
/// 4. Victim receives fewer shares than expected due to inflated price
/// 5. Attacker redeems their shares, capturing value from the victim's deposit
///
/// ## Mitigation Strategies
///
/// ### 1. Initial Deposit Protection
///
/// Vault deployers can protect against this attack by making an initial deposit
/// of a non-trivial amount of the asset, such that price manipulation becomes
/// infeasible. This "dead shares" approach makes the attack economically
/// unviable.
///
/// ### 2. Virtual Assets and Shares (Configurable Decimals Offset)
///
/// This implementation introduces configurable virtual assets and shares to
/// help developers mitigate the risk. The decimals offset (accessible via
/// [`Vault::get_decimals_offset()`]) corresponds to an offset in the decimal
/// representation between the underlying asset's decimals and the vault
/// decimals.
///
/// The virtual shares and assets do not fully prevent the attack. With the
/// default offset (0), diluting a single deposit costs the attacker at least
/// as much as that deposit, so a single victim is never profitable. The
/// rounding loss of each deposit, however, accrues to the existing
/// shareholders, and an attacker holding most of the real shares profits once
/// enough deposits land on the inflated price. With a larger offset, the
/// attack becomes orders of magnitude more expensive than it is profitable.
///
/// The drawback of this approach is that the virtual shares do capture (a very
/// small) part of the value being accrued to the vault. Also, if the vault
/// experiences losses, the users try to exit the vault, the virtual shares and
/// assets will cause the first user to exit to experience reduced losses in
/// detriment to the last users that will experience bigger losses.
///
/// The virtual shares also lower the ceiling of the share supply cap. Every
/// conversion computes `total_supply + 10^offset`, so the real supply has to
/// stay at least `10^offset` below `i128::MAX` for the vault to keep working.
/// [`Vault::deposit_internal()`] enforces this bound before minting. Contracts
/// that mint shares through any other path are expected to preserve it
/// themselves.
///
/// ### 3. Zero-Share Deposit Rejection
///
/// [`Vault::deposit()`] rejects a positive deposit that would mint zero
/// shares, so no depositor can lose their entire deposit to an inflated
/// price. Each deposit still loses up to one share's worth of assets to
/// rounding. Integrators that need a tighter bound should compare
/// [`Vault::preview_deposit()`] against a caller-supplied minimum before
/// depositing.
///
/// If this is not the preferred solution, implementers can still use the
/// default offset of 0 and implement their own safeguards.
///
/// ## References
///
/// <https://docs.openzeppelin.com/contracts/5.x/erc4626>
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
    /// * [`VaultTokenError::VaultAssetAddressNotSet`] - When the vault's
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
            .unwrap_or_else(|| panic_with_error!(e, VaultTokenError::VaultAssetAddressNotSet))
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
    /// for the given receiver address.
    ///
    /// This is the largest amount whose deposit mints at most the number of
    /// shares [`Self::max_mint`] returns: depositing it never mints more
    /// shares than the vault accepts, and never zero, and unless it is
    /// `i128::MAX`, depositing one more asset would mint more. As with any
    /// deposit, depositing this amount can pay up to one share's worth of
    /// assets more than the shares it mints cost (refer to the rounding notes
    /// on [`Vault`]); [`Self::preview_mint()`] of [`Self::max_mint`] is the
    /// exact cost of the remaining shares. Returns `0` when the vault's totals
    /// are too
    /// large for any deposit to succeed: `deposit` computes `total_assets + 1`
    /// and `total_supply + 10^offset`, and panics when either overflows.
    ///
    /// # Arguments
    ///
    /// * `e` - Access to the Soroban environment.
    /// * `receiver` - The address that would receive the vault shares.
    ///
    /// # Errors
    ///
    /// * refer to [`Self::total_assets()`] errors.
    pub fn max_deposit(e: &Env, receiver: Address) -> i128 {
        Self::max_assets_for_shares(e, Self::max_mint(e, receiver))
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
    /// for the given receiver address.
    ///
    /// Two limits apply, and the smaller one is returned:
    ///
    /// * The share supply must stay below `i128::MAX - 10^offset`, because
    ///   every conversion computes `total_supply + 10^offset` (refer to
    ///   [`Self::deposit_internal()`]). So at most `i128::MAX - 10^offset -
    ///   total_supply` more shares can be minted. In practice this only matters
    ///   for vaults holding close to `i128::MAX / 10^offset` asset base units.
    /// * Minting has to be paid for in assets, and the payment cannot exceed
    ///   `i128::MAX`. Once a share is worth more than one asset base unit, this
    ///   caps the shares at what `i128::MAX` assets buy at the current price,
    ///   so that [`Self::preview_mint()`] of the returned amount fits in an
    ///   `i128`.
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
    /// * refer to [`Self::total_assets()`] errors.
    pub fn max_mint(e: &Env, _receiver: Address) -> i128 {
        let supply_limit = Self::max_mint_under_supply_bound(e);
        if supply_limit == 0 {
            return 0;
        }

        // Every conversion prices shares with `total_assets + 1` (the `+ 1`
        // keeps the price defined in an empty vault). When the vault holds
        // `i128::MAX` assets, this sum overflows, and `mint` panics with
        // `MathOverflow` in `preview_mint` for any positive amount. No mint can
        // succeed, so this reports `0` instead of panicking.
        let Some(assets_plus_one) = Self::total_assets(e).checked_add(1) else {
            return 0;
        };

        // `supply_limit > 0` means `total_supply + 10^offset` fits in an
        // `i128`.
        let effective_supply = i128::MAX - supply_limit;
        let payable = checked_mul_div_with_rounding(
            e,
            i128::MAX,
            effective_supply,
            assets_plus_one,
            Rounding::Floor,
        )
        .unwrap_or(i128::MAX);
        supply_limit.min(payable)
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
    /// by the given owner.
    ///
    /// This is the owner's vault share balance, or `0` when that balance is
    /// worth zero assets (see [`VaultTokenError::VaultZeroAssets`]).
    ///
    /// # Arguments
    ///
    /// * `e` - Access to the Soroban environment.
    /// * `owner` - The address that owns the vault shares.
    ///
    /// # Errors
    ///
    /// * refer to [`Self::convert_to_assets()`] errors.
    pub fn max_redeem(e: &Env, owner: Address) -> i128 {
        let shares = Self::balance(e, &owner);
        // `redeem` rejects burning shares for zero assets, so a balance
        // worth less than one unit of the asset cannot be redeemed at all.
        if Self::convert_to_assets(e, shares) == 0 {
            return 0;
        }
        shares
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
    /// * [`VaultTokenError::VaultExceededMaxDeposit`] - When the deposit would
    ///   mint more shares than [`Self::max_mint`] allows, i.e. when depositing
    ///   more assets than [`Self::max_deposit`] (up to rounding).
    /// * [`VaultTokenError::VaultZeroShares`] - When a positive amount of
    ///   assets would mint zero shares.
    /// * also refer to [`Self::preview_deposit()`] errors.
    /// * also refer to [`Self::deposit_internal()`] errors.
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
    pub fn deposit(
        e: &Env,
        assets: i128,
        receiver: Address,
        from: Address,
        operator: Address,
    ) -> i128 {
        operator.require_auth();

        let shares: i128 = Self::preview_deposit(e, assets);
        // Both limits in `max_mint` count shares, so this checks the shares
        // the deposit mints, not the assets it pays.
        //
        // Only the supply limit is checked here. The payment limit (the most
        // shares that `i128::MAX` assets can buy) cannot be exceeded by a
        // deposit, because a deposit never pays more than `i128::MAX` assets.
        //
        // Checking `assets` against `max_deposit` would enforce the same
        // limit, but `max_deposit` reads `total_assets` again, after
        // `preview_deposit` already read it above. Depositing exactly
        // `max_deposit` still passes this check, because `max_deposit` is the
        // largest amount that mints at most `max_mint` shares.
        if shares > Self::max_mint_under_supply_bound(e) {
            panic_with_error!(e, VaultTokenError::VaultExceededMaxDeposit);
        }
        if shares == 0 && assets > 0 {
            panic_with_error!(e, VaultTokenError::VaultZeroShares);
        }
        Self::deposit_internal(e, &receiver, assets, shares, &from, &operator);
        emit_deposit(e, &operator, &from, &receiver, assets, shares);

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
    /// * [`VaultTokenError::VaultExceededMaxMint`] - When attempting to mint
    ///   more shares than the maximum allowed for the receiver.
    /// * also refer to [`Self::preview_mint()`] errors.
    /// * also refer to [`Self::deposit_internal()`] errors.
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
    pub fn mint(
        e: &Env,
        shares: i128,
        receiver: Address,
        from: Address,
        operator: Address,
    ) -> i128 {
        operator.require_auth();

        let max_shares = Self::max_mint(e, receiver.clone());
        if shares > max_shares {
            panic_with_error!(e, VaultTokenError::VaultExceededMaxMint);
        }
        let assets: i128 = Self::preview_mint(e, shares);
        Self::deposit_internal(e, &receiver, assets, shares, &from, &operator);
        emit_deposit(e, &operator, &from, &receiver, assets, shares);

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
    /// * [`VaultTokenError::VaultExceededMaxWithdraw`] - When attempting to
    ///   withdraw more assets than the maximum allowed for the owner.
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
    pub fn withdraw(
        e: &Env,
        assets: i128,
        receiver: Address,
        owner: Address,
        operator: Address,
    ) -> i128 {
        operator.require_auth();

        let max_assets = Self::max_withdraw(e, owner.clone());
        if assets > max_assets {
            panic_with_error!(e, VaultTokenError::VaultExceededMaxWithdraw);
        }
        let shares: i128 = Self::preview_withdraw(e, assets);
        Self::withdraw_internal(e, &receiver, &owner, assets, shares, &operator);
        emit_withdraw(e, &operator, &receiver, &owner, assets, shares);

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
    /// * [`VaultTokenError::VaultExceededMaxRedeem`] - When attempting to
    ///   redeem more shares than the maximum allowed for the owner.
    /// * [`VaultTokenError::VaultZeroAssets`] - When a positive amount of
    ///   shares would redeem zero assets while the owner's balance is worth at
    ///   least one unit of the asset. When the whole balance is worth zero
    ///   assets, [`Self::max_redeem`] is `0` and the redemption fails with
    ///   [`VaultTokenError::VaultExceededMaxRedeem`] instead.
    /// * also refer to [`Self::max_redeem()`] errors.
    /// * also refer to [`Self::preview_redeem()`] errors.
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
    pub fn redeem(
        e: &Env,
        shares: i128,
        receiver: Address,
        owner: Address,
        operator: Address,
    ) -> i128 {
        operator.require_auth();

        let max_shares = Self::max_redeem(e, owner.clone());
        if shares > max_shares {
            panic_with_error!(e, VaultTokenError::VaultExceededMaxRedeem);
        }
        let assets = Self::preview_redeem(e, shares);
        if assets == 0 && shares > 0 {
            panic_with_error!(e, VaultTokenError::VaultZeroAssets);
        }
        Self::withdraw_internal(e, &receiver, &owner, assets, shares, &operator);
        emit_withdraw(e, &operator, &receiver, &owner, assets, shares);

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
    /// * [`VaultTokenError::MathOverflow`] - When the sum of underlying asset
    ///   decimals and offset exceeds the maximum value.
    pub fn decimals(e: &Env) -> u32 {
        Self::get_underlying_asset_decimals(e)
            .checked_add(Self::get_decimals_offset(e))
            .unwrap_or_else(|| panic_with_error!(e, VaultTokenError::MathOverflow))
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
    /// * [`VaultTokenError::VaultAssetAddressAlreadySet`] - When attempting to
    ///   set the asset address after it has already been initialized.
    ///
    /// # Security Warning
    ///
    /// ⚠️ SECURITY RISK: This function has NO AUTHORIZATION CONTROLS ⚠️
    ///
    /// It is the responsibility of the implementer to establish appropriate
    /// access controls to ensure that only authorized accounts can set the
    /// asset address. This function is best used in the constructor of the
    /// smart contract or combined with the Ownable or Access Control pattern.
    pub fn set_asset(e: &Env, asset: Address) {
        // Check if asset is already set
        if e.storage().instance().has(&VaultStorageKey::AssetAddress) {
            panic_with_error!(e, VaultTokenError::VaultAssetAddressAlreadySet);
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
    /// To enforce a reasonable value that maximizes security and UX at the
    /// same time, this value is bounded to a maximum of 10.
    ///
    /// Any value higher than 10 is not recommended as it provides
    /// almost no practical benefits, and any value close to 30 may
    /// cause overflow errors depending on the base asset decimals, and
    /// amount of assets in the vault.
    ///
    /// If a value higher than 10 is needed, a custom copy of this function
    /// should be considered.
    ///
    /// # Arguments
    ///
    /// * `e` - Access to the Soroban environment.
    /// * `offset` - The number of additional decimal places to add.
    ///
    /// # Errors
    ///
    /// * [`VaultTokenError::VaultVirtualDecimalsOffsetAlreadySet`] - When
    ///   attempting to set the offset after it has already been initialized.
    /// * [`VaultTokenError::VaultMaxDecimalsOffsetExceeded`] - When attempting
    ///   to set the offset to a value higher than the suggested maximum
    ///   allowed.
    ///
    /// # Security Warning
    ///
    /// ⚠️ SECURITY RISK: This function has NO AUTHORIZATION CONTROLS ⚠️
    ///
    /// It is the responsibility of the implementer to establish appropriate
    /// access controls to ensure that only authorized accounts can set the
    /// decimals offset. This function is best used in the constructor of the
    /// smart contract or combined with the Ownable or Access Control pattern.
    pub fn set_decimals_offset(e: &Env, offset: u32) {
        if offset > MAX_DECIMALS_OFFSET {
            panic_with_error!(e, VaultTokenError::VaultMaxDecimalsOffsetExceeded);
        }
        // Check if virtual decimals offset is already set
        if e.storage().instance().has(&VaultStorageKey::VirtualDecimalsOffset) {
            panic_with_error!(e, VaultTokenError::VaultVirtualDecimalsOffsetAlreadySet);
        }
        e.storage().instance().set(&VaultStorageKey::VirtualDecimalsOffset, &offset);
    }

    /// Internal conversion function from assets to shares with support for
    /// rounding direction, returning the equivalent amount of vault shares.
    ///
    /// Implements the formula:
    /// shares = (assets × (totalSupply + 10^offset)) / (totalAssets + 1)
    ///
    /// # Arguments
    ///
    /// * `e` - Access to the Soroban environment.
    /// * `assets` - The amount of underlying assets to convert.
    /// * `rounding` - The rounding direction to use for the conversion.
    ///
    /// # Errors
    ///
    /// * [`VaultTokenError::VaultInvalidAssetsAmount`] - When `assets < 0`.
    /// * [`VaultTokenError::MathOverflow`] - When mathematical operations
    ///   result in overflow.
    pub fn convert_to_shares_with_rounding(e: &Env, assets: i128, rounding: Rounding) -> i128 {
        if assets < 0 {
            panic_with_error!(e, VaultTokenError::VaultInvalidAssetsAmount);
        }
        if assets == 0 {
            return 0;
        }

        // Assets being deposited
        let x = assets;

        // Virtual offset = 10^offset
        let pow = 10_i128
            .checked_pow(Self::get_decimals_offset(e))
            .unwrap_or_else(|| panic_with_error!(e, VaultTokenError::MathOverflow));

        // Effective total supply = totalSupply + virtual offset
        let y = Self::total_supply(e)
            .checked_add(pow)
            .unwrap_or_else(|| panic_with_error!(e, VaultTokenError::MathOverflow));

        // Effective total assets = totalAssets + 1 (prevents division by zero)
        let denominator = Self::total_assets(e)
            .checked_add(1_i128)
            .unwrap_or_else(|| panic_with_error!(e, VaultTokenError::MathOverflow));

        // (assets × (totalSupply + 10^offset)) / (totalAssets + 1)
        mul_div_with_rounding(e, x, y, denominator, rounding)
    }

    /// Internal conversion function from shares to assets with support for
    /// rounding direction, returning the equivalent amount of underlying
    /// assets.
    ///
    /// Implements the formula:
    /// assets = (shares × (totalAssets + 1)) / (totalSupply + 10^offset)
    ///
    /// # Arguments
    ///
    /// * `e` - Access to the Soroban environment.
    /// * `shares` - The amount of vault shares to convert.
    /// * `rounding` - The rounding direction to use for the conversion.
    ///
    /// # Errors
    ///
    /// * [`VaultTokenError::VaultInvalidSharesAmount`] - When `shares < 0`.
    /// * [`VaultTokenError::MathOverflow`] - When mathematical operations
    ///   result in overflow.
    pub fn convert_to_assets_with_rounding(e: &Env, shares: i128, rounding: Rounding) -> i128 {
        if shares < 0 {
            panic_with_error!(e, VaultTokenError::VaultInvalidSharesAmount);
        }
        if shares == 0 {
            return 0;
        }

        // Shares being redeemed
        let x = shares;

        // Effective total assets = totalAssets + 1 (prevents division by zero)
        let y = Self::total_assets(e)
            .checked_add(1_i128)
            .unwrap_or_else(|| panic_with_error!(e, VaultTokenError::MathOverflow));

        // Virtual offset = 10^offset
        let pow = 10_i128
            .checked_pow(Self::get_decimals_offset(e))
            .unwrap_or_else(|| panic_with_error!(e, VaultTokenError::MathOverflow));

        // Effective total supply = totalSupply + virtual offset
        let denominator = Self::total_supply(e)
            .checked_add(pow)
            .unwrap_or_else(|| panic_with_error!(e, VaultTokenError::MathOverflow));

        // (shares × (totalAssets + 1)) / (totalSupply + 10^offset)
        mul_div_with_rounding(e, x, y, denominator, rounding)
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
    /// # Errors
    ///
    /// * [`VaultTokenError::MathOverflow`] - When minting `shares` would push
    ///   the share supply plus the virtual shares (`10^offset`) past
    ///   `i128::MAX`.
    /// * refer to [`Self::query_asset()`] errors.
    /// * refer to [`increase_total_supply`] errors.
    /// * refer to [`Base::update`] errors.
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
        // This function assumes prior authorization of the operator and
        // validation of amounts.

        // Every conversion computes `total_supply + 10^offset`, so the real
        // share supply has to stay at least `10^offset` below `i128::MAX`.
        // `increase_total_supply` only guards `total_supply + shares`; a mint
        // that passes that check but breaks this stricter bound would leave
        // every later conversion, and therefore every exit, failing with
        // `MathOverflow`, locking the assets in the vault.
        let pow = 10_i128
            .checked_pow(Self::get_decimals_offset(e))
            .unwrap_or_else(|| panic_with_error!(e, VaultTokenError::MathOverflow));
        Self::total_supply(e)
            .checked_add(shares)
            .and_then(|new_supply| new_supply.checked_add(pow))
            .unwrap_or_else(|| panic_with_error!(e, VaultTokenError::MathOverflow));

        let token_client = token::Client::new(e, &Self::query_asset(e));
        // `safeTransfer` mechanism is not present in the base module, (will be
        // provided as an extension)

        if operator == from {
            // Direct transfer: `operator` is depositing their own assets
            token_client.transfer(from, e.current_contract_address(), &assets);
        } else {
            // Allowance-based transfer: `operator` is depositing on behalf of
            // `from` This requires that `from` has approved
            // `operator` on the underlying asset
            token_client.transfer_from(operator, from, &e.current_contract_address(), &assets);
        }

        increase_total_supply(e, shares);
        Base::update(e, None, Some(receiver), shares);
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
        // This function assumes prior authorization of the operator and
        // validation of amounts.
        if operator != owner {
            Base::spend_allowance(e, owner, operator, shares);
        }
        Base::update(e, Some(owner), None, shares);
        decrease_total_supply(e, shares);
        let token_client = token::Client::new(e, &Self::query_asset(e));
        // `safeTransfer` mechanism is not present in the base module, (will be
        // provided as an extension)
        token_client.transfer(&e.current_contract_address(), receiver, &assets);
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
    /// For more information about virtual decimals offset and its role in
    /// mitigating inflation attacks, see the implementation-level
    /// documentation: [Inflation Attack (Donation Attack)
    /// Mitigation](Vault)
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

    /// Returns the shares that can still be minted under the virtual supply
    /// bound, `i128::MAX - 10^offset - total_supply`, and `0` once the bound
    /// is reached.
    ///
    /// Every conversion computes `total_supply + 10^offset`, so the share
    /// supply has to stay at least `10^offset` below `i128::MAX`, and
    /// [`Self::deposit_internal()`] rejects any mint that would break this
    /// bound. Unlike [`Self::max_mint()`], this does not read `total_assets`.
    ///
    /// # Arguments
    ///
    /// * `e` - Access to the Soroban environment.
    fn max_mint_under_supply_bound(e: &Env) -> i128 {
        // `get_decimals_offset` is at most `MAX_DECIMALS_OFFSET` (10), so the
        // power always fits; the fallback only keeps this view panic-free.
        let Some(virtual_shares) = 10_i128.checked_pow(Self::get_decimals_offset(e)) else {
            return 0;
        };
        // Both terms are non-negative and at most `i128::MAX`, so the
        // subtraction cannot overflow; a negative result means the bound is
        // already reached.
        (i128::MAX - virtual_shares - Self::total_supply(e)).max(0)
    }

    /// Returns the largest amount of assets whose deposit mints at most
    /// `shares` shares, and never panics: this backs the `max_deposit` views,
    /// which report a limit instead of failing. A positive result always
    /// mints at least one share, so `deposit` accepts it.
    ///
    /// Returns `0` for `shares == 0`, and also when `total_assets + 1` or
    /// `total_supply + 10^offset` overflows: `deposit` computes these same two
    /// sums and panics when they overflow, so no deposit is possible in that
    /// state. Returns `i128::MAX` when the largest amount does not fit in an
    /// `i128`: a deposit cannot pay more than `i128::MAX` assets, and
    /// depositing `i128::MAX` still mints at most `shares` shares.
    ///
    /// # Arguments
    ///
    /// * `e` - Access to the Soroban environment.
    /// * `shares` - The amount of vault shares still allowed to be minted.
    fn max_assets_for_shares(e: &Env, shares: i128) -> i128 {
        if shares <= 0 {
            return 0;
        }

        // `effective_assets` is the vault's asset balance plus 1.
        // `effective_supply` is the number of shares that exist plus
        // `10^offset`. The added 1 and `10^offset` are virtual: nobody owns
        // them. They make the inflation attack expensive, and keep the price
        // defined when the vault is empty.
        //
        // If either sum overflows, `deposit` panics, so no
        // deposit is possible.
        let (Some(effective_assets), Some(effective_supply)) = (
            Self::total_assets(e).checked_add(1),
            10_i128
                .checked_pow(Self::get_decimals_offset(e))
                .and_then(|pow| Self::total_supply(e).checked_add(pow)),
        ) else {
            return 0;
        };

        // # The largest deposit that `deposit` accepts
        //
        // `M` is the amount returned here. Let
        // `P = effective_assets / effective_supply`, the price of one share in
        // assets. Depositing `a` assets mints `floor(a / P)` shares, and
        // `deposit` accepts it only if:
        //
        // * it mints at most `shares` shares (the limit), and
        // * it mints at least one share, when `a > 0` (a positive deposit that
        //   mints zero shares is rejected with `VaultZeroShares`).
        //
        // ## Depositing `M` mints at most `shares` shares
        //
        // A deposit only mints whole shares: depositing `a` assets mints
        // `a / P` shares with the fraction dropped.
        //
        // So the limit is exceeded only when a deposit pays the full cost of
        // `shares + 1` shares, which is `(shares + 1) * P`. Every deposit below
        // that cost mints at most `shares` shares.
        //
        // `M` is therefore the largest whole amount below `(shares + 1) * P`.
        // When that cost has a fraction, `M` should be the cost rounded down
        // (2.99 gives 2). When the cost is whole, `M` should be one
        // less (3 gives 2). In both cases, the cost can be rounded up,
        // and then minus one: `ceil((shares + 1) * P) - 1`, which is
        // what this function returns.
        //
        // For example, with one share left and a price of 1.495 assets: two
        // shares cost 2.99 assets. Rounding up gives 3, and minus one makes
        // `M = 2`. Depositing 2 assets mints 1 share.
        //
        // ## Depositing `M`, when positive, mints at least one share
        //
        // `deposit` rejects a positive deposit that mints zero shares, so a
        // positive `M` has to pay at least the price of one share. Here
        // `shares` is at least one, as `shares <= 0` already returned early.
        //
        // There are two cases, depending on the price of one share:
        //
        // ### Case 1 (trivial): one share costs less than one asset (`P < 1`)
        //
        // Even the smallest positive `M`, which is 1 asset, is already more
        // than the price of one share. So any positive `M` pays for at least
        // one share. For example, with a share price of 0.55 assets, 1 asset
        // pays for one share.
        //
        // ### Case 2: one share costs one asset or more (`P >= 1`)
        //
        // * Let `C = (shares + 1) * P`, the cost of `shares + 1` shares. `M` is
        //   `C` rounded up, minus one, so `M` is at most one asset below `C`.
        // * `shares + 1` is at least two, so `C` is at least the price of two
        //   shares.
        // * One asset is no more than the price of one share. So going at most
        //   one asset below `C` still leaves at least the price of one share.
        //
        // So `M` pays for at least one share. For example, with one share left
        // and a price of 1.495 assets: `C = 2 * 1.495 = 2.99`, and `M = 2` is
        // still more than the 1.495 that one share costs.
        //
        // ## Examples
        //
        // * `P = 151 / 101 ~ 1.495`, one share left: `M = ceil(2.99) - 1 = 2`.
        //   Depositing 2 mints 1 share, depositing 3 mints 2. Rounding the cost
        //   of one share down instead gives 1, which mints 0 shares.
        // * `P = 61 / 110 ~ 0.55`, one share left: `M = ceil(1.11) - 1 = 1`.
        //   Depositing 1 mints 1 share, depositing 2 mints 3. Rounding the cost
        //   of one share down instead gives 0, although 1 is accepted.
        // * `P = 1 / 10`, two shares left: `M = ceil(0.3) - 1 = 0`. The
        //   smallest deposit, 1 asset, already mints 10 shares.
        //
        // ## Overflow
        //
        // `shares + 1` does not overflow for the two callers. `shares` is at
        // most `Vault::max_mint` (`CappedVault::max_mint` never exceeds it),
        // which is at most `i128::MAX - 10^offset - total_supply`. As
        // `10^offset` is at least 1 and `total_supply` is not negative, that is
        // at most `i128::MAX - 1`.
        //
        // The addition still saturates, so this view keeps its promise not to
        // panic even if a future caller passes `i128::MAX`. A plain `+` would
        // panic there, as overflow checks are on. Saturating also gives a
        // correct result in that case: the sum stays at `i128::MAX`, so the
        // result is the largest deposit minting at most `i128::MAX - 1` shares,
        // which is still within the limit.
        //
        // `amount - 1` does not underflow, so it does not need to saturate.
        // `shares + 1` is at least 2, `effective_assets` is at least 1 (the
        // vault's asset balance is a token balance, never negative), and
        // `effective_supply` is at least 1. So the rounded-up cost `amount` is
        // at least 1, and `amount - 1` is at least `0`.
        //
        // When the cost of `shares + 1` shares does not fit in an `i128`, it is
        // above `i128::MAX`, so a deposit of `i128::MAX` is below it and mints
        // at most `shares` shares. It also mints at least one, as
        // `P <= effective_assets <= i128::MAX`. So `i128::MAX`, the largest
        // amount a deposit can pay, is returned.
        checked_mul_div_with_rounding(
            e,
            shares.saturating_add(1),
            effective_assets,
            effective_supply,
            Rounding::Ceil,
        )
        .map_or(i128::MAX, |amount| amount - 1)
    }
}
