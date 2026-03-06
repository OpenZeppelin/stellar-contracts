use soroban_sdk::{contracttype, panic_with_error, Address, Env};
use stellar_compliance_common::ModuleError;

#[contracttype]
#[derive(Clone)]
pub enum SupplyLimitStorageKey {
    /// Per-token supply cap.
    SupplyLimit(Address),
    /// Per-token internal supply counter (updated via hooks).
    InternalSupply(Address),
}

pub fn get_supply_limit(e: &Env, token: &Address) -> i128 {
    e.storage()
        .persistent()
        .get(&SupplyLimitStorageKey::SupplyLimit(token.clone()))
        .unwrap_or_default()
}

pub fn get_supply_limit_or_panic(e: &Env, token: &Address) -> i128 {
    e.storage()
        .persistent()
        .get(&SupplyLimitStorageKey::SupplyLimit(token.clone()))
        .unwrap_or_else(|| panic_with_error!(e, ModuleError::MissingLimit))
}

pub fn set_supply_limit(e: &Env, token: &Address, limit: i128) {
    e.storage().persistent().set(&SupplyLimitStorageKey::SupplyLimit(token.clone()), &limit);
}

pub fn get_internal_supply(e: &Env, token: &Address) -> i128 {
    e.storage()
        .persistent()
        .get(&SupplyLimitStorageKey::InternalSupply(token.clone()))
        .unwrap_or_default()
}

pub fn set_internal_supply(e: &Env, token: &Address, supply: i128) {
    e.storage().persistent().set(&SupplyLimitStorageKey::InternalSupply(token.clone()), &supply);
}
