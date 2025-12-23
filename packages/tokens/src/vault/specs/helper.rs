use cvlr::clog;
use soroban_sdk::{Env, panic_with_error};

use crate::{
    fungible::FungibleToken,
    vault::{
        specs::vault::BasicVault,
        FungibleVault, Vault, VaultTokenError,
    },
};

pub fn effective_total_assets(e: &Env) -> i128 {
    let total_assets = BasicVault::total_assets(e);
    clog!(total_assets);
    let effective_total_assets = total_assets.checked_add(1_i128).unwrap_or_else(|| panic_with_error!(e, VaultTokenError::MathOverflow));
    clog!(effective_total_assets);
    effective_total_assets
}

pub fn virtual_offset(e: &Env) -> i128 {
    let decimals_offset = Vault::get_decimals_offset(e);
    clog!(decimals_offset);
    let virtual_offset = 10_i128.checked_pow(decimals_offset).unwrap_or_else(|| panic_with_error!(e, VaultTokenError::MathOverflow));
    clog!(virtual_offset);
    virtual_offset
}

pub fn effective_total_supply(e: &Env) -> i128 {
    let total_supply = BasicVault::total_supply(e);
    clog!(total_supply);
    let virtual_offset = virtual_offset(e);
    clog!(virtual_offset);
    let effective_total_supply = total_supply.checked_add(virtual_offset).unwrap_or_else(|| panic_with_error!(e, VaultTokenError::MathOverflow));
    clog!(effective_total_supply);
    effective_total_supply
}
