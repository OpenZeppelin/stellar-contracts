//! Max balance compliance module — Stellar port of T-REX
//! [`MaxBalanceModule.sol`][trex-src].
//!
//! Tracks effective balances per **identity** (not per wallet), enforcing a
//! per-token cap.
//!
//! [trex-src]: https://github.com/TokenySolutions/T-REX/blob/main/contracts/compliance/modular/modules/MaxBalanceModule.sol

pub mod storage;
#[cfg(test)]
mod test;

use soroban_sdk::{contractevent, Address};

/// Emitted when a token's per-identity balance cap is configured.
#[contractevent]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MaxBalanceSet {
    #[topic]
    pub token: Address,
    pub max_balance: i128,
}

/// Emits a [`MaxBalanceSet`] event.
///
/// # Arguments
///
/// * `e` - Access to the Soroban environment.
/// * `token` - The token whose max balance was configured.
/// * `max_balance` - The configured per-identity balance cap.
pub fn emit_max_balance_set(e: &soroban_sdk::Env, token: &Address, max_balance: i128) {
    MaxBalanceSet { token: token.clone(), max_balance }.publish(e);
}

/// Emitted when an identity balance is pre-seeded via `pre_set_module_state`.
#[contractevent]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct IDBalancePreSet {
    #[topic]
    pub token: Address,
    pub identity: Address,
    pub balance: i128,
}

/// Emits an [`IDBalancePreSet`] event.
///
/// # Arguments
///
/// * `e` - Access to the Soroban environment.
/// * `token` - The token whose identity balance was pre-seeded.
/// * `identity` - The identity whose balance was pre-seeded.
/// * `balance` - The pre-seeded balance.
pub fn emit_id_balance_pre_set(
    e: &soroban_sdk::Env,
    token: &Address,
    identity: &Address,
    balance: i128,
) {
    IDBalancePreSet { token: token.clone(), identity: identity.clone(), balance }.publish(e);
}
