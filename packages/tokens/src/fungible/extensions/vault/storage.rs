use soroban_sdk::{contracttype, panic_with_error, token, Address, Env};

use crate::fungible::{
    vault::{emit_deposit, emit_withdraw},
    Base, ContractOverrides, FungibleTokenError,
};

use stellar_contract_utils::math::fixed_point::{muldiv, Rounding};

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

/// TODO: describe functions, arguments, errors
impl Vault {
    // ################## QUERY STATE ##################

    pub fn query_asset(e: &Env) -> Address {
        e.storage()
            .instance()
            .get(&VaultStorageKey::AssetAddress)
            .unwrap_or_else(|| panic_with_error!(e, FungibleTokenError::VaultAssetAddressNotSet))
    }

    pub fn total_assets(e: &Env) -> i128 {
        let token_client = token::Client::new(e, &Vault::query_asset(e));
        token_client.balance(&e.current_contract_address())
    }

    pub fn convert_to_shares(e: &Env, assets: i128) -> i128 {
        Vault::convert_to_shares_with_rounding(e, assets, Rounding::Floor)
    }

    pub fn convert_to_assets(e: &Env, shares: i128) -> i128 {
        Vault::convert_to_assets_with_rounding(e, shares, Rounding::Floor)
    }

    pub fn max_deposit(_e: &Env, _receiver: Address) -> i128 {
        i128::MAX
    }

    pub fn preview_deposit(e: &Env, assets: i128) -> i128 {
        Vault::convert_to_shares_with_rounding(e, assets, Rounding::Floor)
    }

    pub fn max_mint(_e: &Env, _receiver: Address) -> i128 {
        i128::MAX
    }

    pub fn preview_mint(e: &Env, shares: i128) -> i128 {
        Vault::convert_to_assets_with_rounding(e, shares, Rounding::Ceil)
    }

    pub fn max_withdraw(e: &Env, owner: Address) -> i128 {
        Vault::convert_to_assets_with_rounding(e, Vault::balance(e, &owner), Rounding::Floor)
    }

    pub fn preview_withdraw(e: &Env, assets: i128) -> i128 {
        Vault::convert_to_shares_with_rounding(e, assets, Rounding::Ceil)
    }

    pub fn max_redeem(e: &Env, owner: Address) -> i128 {
        Vault::balance(e, &owner)
    }

    pub fn preview_redeem(e: &Env, shares: i128) -> i128 {
        Vault::convert_to_assets_with_rounding(e, shares, Rounding::Floor)
    }

    // ################## CHANGE STATE ##################

    pub fn deposit(e: &Env, assets: i128, caller: Address, receiver: Address) -> i128 {
        caller.require_auth();
        let max_assets = Vault::max_deposit(e, receiver.clone());
        if assets > max_assets {
            panic_with_error!(e, FungibleTokenError::VaultExceededMaxDeposit);
        }
        let shares: i128 = Vault::preview_deposit(e, assets);
        Vault::deposit_no_auth(e, &caller, &receiver, assets, shares);
        shares
    }

    pub fn mint(e: &Env, shares: i128, caller: Address, receiver: Address) -> i128 {
        caller.require_auth();
        let max_shares = Vault::max_mint(e, receiver.clone());
        if shares > max_shares {
            panic_with_error!(e, FungibleTokenError::VaultExceededMaxMint);
        }
        let assets: i128 = Vault::preview_mint(e, shares);
        Vault::deposit_no_auth(e, &caller, &receiver, assets, shares);
        assets
    }

    pub fn withdraw(
        e: &Env,
        assets: i128,
        caller: Address,
        receiver: Address,
        owner: Address,
    ) -> i128 {
        caller.require_auth();
        let max_assets = Vault::max_withdraw(e, owner.clone());
        if assets > max_assets {
            panic_with_error!(e, FungibleTokenError::VaultExceededMaxWithdraw);
        }
        let shares: i128 = Vault::preview_withdraw(e, assets);
        Vault::withdraw_no_auth(e, &caller, &receiver, &owner, assets, shares);
        shares
    }

    pub fn redeem(
        e: &Env,
        shares: i128,
        caller: Address,
        receiver: Address,
        owner: Address,
    ) -> i128 {
        caller.require_auth();
        let max_shares = Vault::max_redeem(e, owner.clone());
        if shares > max_shares {
            panic_with_error!(e, FungibleTokenError::VaultExceededMaxRedeem);
        }
        let assets = Vault::preview_redeem(e, shares);
        Vault::withdraw_no_auth(e, &caller, &receiver, &owner, assets, shares);
        assets
    }

    // ################## OVERRIDDEN FUNCTIONS ##################

    /**
     * Decimals are computed by adding the decimal offset on top of the underlying asset's decimals.
     */
    pub fn decimals(e: &Env) -> u32 {
        Vault::get_underlying_asset_decimals(e)
            .checked_add(Vault::get_decimals_offset(e))
            .unwrap_or_else(|| panic_with_error!(e, FungibleTokenError::MathOverflow))
    }

    // ################## LOW-LEVEL HELPERS ##################

    /// **IMPORTANT**: This function bypasses authorization checks.
    /// We recommend using this function in the constructor of your smart contract.
    /// By design, the underlying asset address should be set once in the constructor
    /// and remain immutable thereafter. Consider combining with the Ownable admin pattern.
    pub fn set_asset(e: &Env, asset: Address) {
        // Check if asset is already set
        if e.storage().instance().has(&VaultStorageKey::AssetAddress) {
            panic_with_error!(e, FungibleTokenError::VaultAssetAddressAlreadySet);
        }
        // Used to panic if non-existent asset address
        let _ = token::Client::new(e, &asset);
        e.storage().instance().set(&VaultStorageKey::AssetAddress, &asset);
    }

    /// **IMPORTANT**: This function bypasses authorization checks.
    /// We recommend using this function in the constructor of your smart contract.
    /// By design, the decimals offset should be set once in the constructor
    /// and remain immutable thereafter. Consider combining with the Ownable admin pattern.
    pub fn set_decimals_offset(e: &Env, offset: u32) {
        // Check if virtual decimals offset is already set
        if e.storage().instance().has(&VaultStorageKey::VirtualDecimalsOffset) {
            panic_with_error!(e, FungibleTokenError::VaultVirtualDecimalsOffsetAlreadySet);
        }
        e.storage().instance().set(&VaultStorageKey::VirtualDecimalsOffset, &offset);
    }

    /**
     * Internal conversion function (from assets to shares) with support for rounding direction.
     * assets.mulDiv(totalSupply() + 10 ** _decimalsOffset(), totalAssets() + 1, rounding)
     */
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
        let y = Vault::total_supply(e)
            .checked_add(pow)
            .unwrap_or_else(|| panic_with_error!(e, FungibleTokenError::MathOverflow));
        let denominator = Vault::total_assets(e)
            .checked_add(1_i128)
            .unwrap_or_else(|| panic_with_error!(e, FungibleTokenError::MathOverflow));
        muldiv(e, x, y, denominator, rounding)
    }

    /**
     * Internal conversion function (from shares to assets) with support for rounding direction.
     * shares.mulDiv(totalAssets() + 1, totalSupply() + 10 ** _decimalsOffset(), rounding)
     */
    pub fn convert_to_assets_with_rounding(e: &Env, shares: i128, rounding: Rounding) -> i128 {
        if shares < 0 {
            panic_with_error!(e, FungibleTokenError::VaultInvalidSharesAmount);
        }
        if shares == 0 {
            return 0;
        }
        let x = shares;
        let y = Vault::total_assets(e)
            .checked_add(1_i128)
            .unwrap_or_else(|| panic_with_error!(e, FungibleTokenError::MathOverflow));
        let pow = 10_i128
            .checked_pow(Self::get_decimals_offset(e))
            .unwrap_or_else(|| panic_with_error!(e, FungibleTokenError::MathOverflow));
        let denominator = Vault::total_supply(e)
            .checked_add(pow)
            .unwrap_or_else(|| panic_with_error!(e, FungibleTokenError::MathOverflow));
        muldiv(e, x, y, denominator, rounding)
    }

    /**
     * Deposit/mint common workflow.
     */
    pub fn deposit_no_auth(
        e: &Env,
        caller: &Address,
        receiver: &Address,
        assets: i128,
        shares: i128,
    ) {
        // This function assumes prior authorization of the caller and validation of amounts.
        let token_client = token::Client::new(e, &Self::query_asset(e));
        // `safeTransfer` mechanism is not present in the base module, (will be provided as an extension)
        token_client.transfer(caller, &e.current_contract_address(), &assets);
        Base::mint(e, receiver, shares);
        emit_deposit(e, caller, receiver, assets, shares);
    }

    /**
     * Withdraw/redeem common workflow.
     */
    pub fn withdraw_no_auth(
        e: &Env,
        caller: &Address,
        receiver: &Address,
        owner: &Address,
        assets: i128,
        shares: i128,
    ) {
        // This function assumes prior authorization of the caller and validation of amounts.
        if caller != owner {
            Base::spend_allowance(e, owner, caller, shares);
        }
        Base::update(e, Some(owner), None, shares);
        let token_client = token::Client::new(e, &Vault::query_asset(e));
        // `safeTransfer` mechanism is not present in the base module, (will be provided as an extension)
        token_client.transfer(&e.current_contract_address(), receiver, &assets);
        emit_withdraw(e, caller, receiver, owner, assets, shares);
    }

    /// The following document explains the importance and necessity of virtual decimals offset:
    /// https://docs.openzeppelin.com/contracts/5.x/erc4626
    /// Default value is 0.
    pub fn get_decimals_offset(e: &Env) -> u32 {
        e.storage().instance().get(&VaultStorageKey::VirtualDecimalsOffset).unwrap_or(0)
    }

    pub fn get_underlying_asset_decimals(e: &Env) -> u32 {
        let token_client = token::Client::new(e, &Vault::query_asset(e));
        token_client.decimals()
    }
}
