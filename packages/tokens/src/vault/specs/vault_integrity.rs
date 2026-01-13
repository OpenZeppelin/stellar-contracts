use cvlr::{cvlr_assert, cvlr_satisfy, nondet::*};
use cvlr::clog;
use cvlr_soroban::nondet_address;
use cvlr_soroban_derive::rule;
use soroban_sdk::{Address, Env};
use stellar_contract_utils::math::fixed_point::Rounding;

use crate::{
    fungible::FungibleToken,
    vault::{
        specs::{asset_token::AssetToken, vault::BasicVault},
        FungibleVault, Vault,
    },
};

use super::vault_invariants::safe_assumptions;
// integrity rules for all functions of the vault.

#[rule]
// set assets sets the asset adress in storage
// status: verified
pub fn set_asset_integrity(e: Env) {
    let new_asset_address: Address = nondet_address();
    clog!(cvlr_soroban::Addr(&new_asset_address.clone()));
    Vault::set_asset(&e, new_asset_address.clone());
    let asset_address_post = Vault::query_asset(&e);
    clog!(cvlr_soroban::Addr(&asset_address_post));
    cvlr_assert!(asset_address_post == new_asset_address);
}

#[rule]
// set_decimals_offset sets the decimals offset in storage
// status: verified
pub fn set_decimals_offset_integrity(e: Env) {
    let new_decimals_offset: u32 = nondet();
    clog!(new_decimals_offset);
    Vault::set_decimals_offset(&e, new_decimals_offset.clone());
    let decimals_offset_post = Vault::get_decimals_offset(&e);
    clog!(decimals_offset_post);
    cvlr_assert!(decimals_offset_post == new_decimals_offset);
}
