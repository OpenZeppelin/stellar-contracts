use soroban_sdk::{contracttype, Address, Env};

#[contracttype]
#[derive(Clone)]
pub enum MaxBalanceStorageKey {
    /// Per-token maximum allowed identity balance.
    MaxBalance(Address),
    /// Balance keyed by (token, identity) — not by wallet.
    IDBalance(Address, Address),
}

pub fn get_max_balance(e: &Env, token: &Address) -> i128 {
    e.storage()
        .persistent()
        .get(&MaxBalanceStorageKey::MaxBalance(token.clone()))
        .unwrap_or_default()
}

pub fn set_max_balance(e: &Env, token: &Address, value: i128) {
    e.storage().persistent().set(&MaxBalanceStorageKey::MaxBalance(token.clone()), &value);
}

pub fn get_id_balance(e: &Env, token: &Address, identity: &Address) -> i128 {
    e.storage()
        .persistent()
        .get(&MaxBalanceStorageKey::IDBalance(token.clone(), identity.clone()))
        .unwrap_or_default()
}

pub fn set_id_balance(e: &Env, token: &Address, identity: &Address, balance: i128) {
    e.storage()
        .persistent()
        .set(&MaxBalanceStorageKey::IDBalance(token.clone(), identity.clone()), &balance);
}
