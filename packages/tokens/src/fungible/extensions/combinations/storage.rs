use soroban_sdk::{Address, Env, MuxedAddress};

use crate::fungible::{
    extensions::{
        allowlist::{AllowList, AllowListContractType},
        blocklist::{BlockList, BlockListContractType},
        total_supply::{decrease_total_supply, mint, total_supply, TotalSupplyOverrides},
    },
    overrides::BurnableOverrides,
    ContractOverrides,
};

/// Contract type combining the [`AllowList`] transfer policy with total
/// supply tracking.
pub struct TotalSupplyAllowList;

/// Contract type combining the [`BlockList`] transfer policy with total
/// supply tracking.
pub struct TotalSupplyBlockList;

impl TotalSupplyOverrides for TotalSupplyAllowList {}
impl TotalSupplyOverrides for TotalSupplyBlockList {}

// The combined contract types keep enforcing their respective list policy.
impl AllowListContractType for TotalSupplyAllowList {}
impl BlockListContractType for TotalSupplyBlockList {}

// Transfers and approvals never touch the total supply, so they are routed
// to the respective list policy unchanged.
impl ContractOverrides for TotalSupplyAllowList {
    fn transfer(e: &Env, from: &Address, to: &MuxedAddress, amount: i128) {
        AllowList::transfer(e, from, to, amount);
    }

    fn transfer_from(e: &Env, spender: &Address, from: &Address, to: &Address, amount: i128) {
        AllowList::transfer_from(e, spender, from, to, amount);
    }

    fn approve(e: &Env, owner: &Address, spender: &Address, amount: i128, live_until_ledger: u32) {
        AllowList::approve(e, owner, spender, amount, live_until_ledger);
    }
}

impl ContractOverrides for TotalSupplyBlockList {
    fn transfer(e: &Env, from: &Address, to: &MuxedAddress, amount: i128) {
        BlockList::transfer(e, from, to, amount);
    }

    fn transfer_from(e: &Env, spender: &Address, from: &Address, to: &Address, amount: i128) {
        BlockList::transfer_from(e, spender, from, to, amount);
    }

    fn approve(e: &Env, owner: &Address, spender: &Address, amount: i128, live_until_ledger: u32) {
        BlockList::approve(e, owner, spender, amount, live_until_ledger);
    }
}

impl BurnableOverrides for TotalSupplyAllowList {
    fn burn(e: &Env, from: &Address, amount: i128) {
        TotalSupplyAllowList::burn(e, from, amount);
    }

    fn burn_from(e: &Env, spender: &Address, from: &Address, amount: i128) {
        TotalSupplyAllowList::burn_from(e, spender, from, amount);
    }
}

impl BurnableOverrides for TotalSupplyBlockList {
    fn burn(e: &Env, from: &Address, amount: i128) {
        TotalSupplyBlockList::burn(e, from, amount);
    }

    fn burn_from(e: &Env, spender: &Address, from: &Address, amount: i128) {
        TotalSupplyBlockList::burn_from(e, spender, from, amount);
    }
}

impl TotalSupplyAllowList {
    /// Returns the total amount of tokens in circulation.
    ///
    /// refer to [`total_supply`] for the inline documentation.
    pub fn total_supply(e: &Env) -> i128 {
        total_supply(e)
    }

    /// Creates `amount` of tokens and assigns them to `to`, increasing the
    /// total supply accordingly.
    ///
    /// refer to [`mint`] for the inline documentation.
    pub fn mint(e: &Env, to: &Address, amount: i128) {
        mint(e, to, amount);
    }

    /// Destroys `amount` of tokens from `from` through the allowlist burn
    /// policy and decreases the total supply accordingly.
    ///
    /// refer to [`AllowList::burn`] and [`decrease_total_supply`] for the
    /// inline documentation.
    pub fn burn(e: &Env, from: &Address, amount: i128) {
        AllowList::burn(e, from, amount);
        decrease_total_supply(e, amount);
    }

    /// Destroys `amount` of tokens from `from` using the allowance mechanism,
    /// through the allowlist burn policy, and decreases the total supply
    /// accordingly.
    ///
    /// refer to [`AllowList::burn_from`] and [`decrease_total_supply`] for
    /// the inline documentation.
    pub fn burn_from(e: &Env, spender: &Address, from: &Address, amount: i128) {
        AllowList::burn_from(e, spender, from, amount);
        decrease_total_supply(e, amount);
    }
}

impl TotalSupplyBlockList {
    /// Returns the total amount of tokens in circulation.
    ///
    /// refer to [`total_supply`] for the inline documentation.
    pub fn total_supply(e: &Env) -> i128 {
        total_supply(e)
    }

    /// Creates `amount` of tokens and assigns them to `to`, increasing the
    /// total supply accordingly.
    ///
    /// refer to [`mint`] for the inline documentation.
    pub fn mint(e: &Env, to: &Address, amount: i128) {
        mint(e, to, amount);
    }

    /// Destroys `amount` of tokens from `from` through the blocklist burn
    /// policy and decreases the total supply accordingly.
    ///
    /// refer to [`BlockList::burn`] and [`decrease_total_supply`] for the
    /// inline documentation.
    pub fn burn(e: &Env, from: &Address, amount: i128) {
        BlockList::burn(e, from, amount);
        decrease_total_supply(e, amount);
    }

    /// Destroys `amount` of tokens from `from` using the allowance mechanism,
    /// through the blocklist burn policy, and decreases the total supply
    /// accordingly.
    ///
    /// refer to [`BlockList::burn_from`] and [`decrease_total_supply`] for
    /// the inline documentation.
    pub fn burn_from(e: &Env, spender: &Address, from: &Address, amount: i128) {
        BlockList::burn_from(e, spender, from, amount);
        decrease_total_supply(e, amount);
    }
}
