//! Maximum-balance compliance module — Stellar port of T-REX
//! [`MaxBalanceModule.sol`][trex-src].
//!
//! Tracks effective balances per identity (not per wallet) and enforces a
//! per-token cap. When a transfer or mint would push an identity's
//! aggregate balance above the configured maximum, the operation is
//! blocked.
//!
//! Multiple wallets controlled by the same on-chain identity share a single
//! tracked balance: holding the same tokens across many wallets does not
//! sidestep the cap. The mapping from wallet to identity is resolved
//! through the token's Identity Registry Storage on every hook call.
//!
//! For migration scenarios — registering this module on a token whose
//! holders already own balances — the module exposes a preset phase. The
//! operator pre-seeds the tracked balances via [`storage::preset_id_balance`]
//! / [`storage::batch_preset_id_balances`], then finalizes the phase with
//! [`storage::mark_preset_completed`]. Subsequent preset calls panic with
//! [`crate::rwa::compliance::modules::ComplianceModuleError::PresetAlreadyCompleted`].
//!
//! [trex-src]: https://github.com/TokenySolutions/T-REX/blob/main/contracts/compliance/modular/modules/MaxBalanceModule.sol

pub mod storage;

#[cfg(test)]
mod test;

use soroban_sdk::{contractevent, contracttrait, Address, Env, Vec};

use crate::rwa::compliance::modules::ComplianceModule;

/// Maximum Balance Compliance Module Trait
///
/// The `MaxBalance` trait extends the [`ComplianceModule`] trait to enforce
/// a per-identity balance cap for a token. When this module is registered
/// on a token's modular compliance contract, transfers and mints are
/// blocked whenever the recipient's identity already holds, or would after
/// the operation hold, more than the configured maximum.
///
/// The check is performed against the identity returned by the Identity
/// Registry Storage, not the recipient wallet. All wallets owned by the
/// same identity share a single tracked balance — splitting holdings
/// across wallets does not raise the effective cap.
///
/// Unlike a stateless rule (such as a country allowlist), this module
/// **maintains its own state**: it credits and debits aggregate balances on
/// every transfer, mint, and burn. Correct accounting therefore requires
/// the module to be registered on **all** of
/// [`crate::rwa::compliance::ComplianceHook::Transferred`],
/// [`crate::rwa::compliance::ComplianceHook::Created`], and
/// [`crate::rwa::compliance::ComplianceHook::Destroyed`] in addition to the
/// validation hooks
/// [`crate::rwa::compliance::ComplianceHook::CanTransfer`] and
/// [`crate::rwa::compliance::ComplianceHook::CanCreate`]. Missing a
/// state-mutating hook causes the tracked balances to drift from reality.
///
/// For migration scenarios, the trait exposes preset functions
/// ([`MaxBalance::preset_id_balance`],
/// [`MaxBalance::batch_preset_id_balances`],
/// [`MaxBalance::mark_preset_completed`]) so the operator can seed the
/// module with the existing on-chain state before binding it to the token.
///
/// This trait is designed to be used in conjunction with the
/// [`ComplianceModule`] trait.
///
/// **NOTE**
///
/// All setter functions exposed in the `MaxBalance` trait are privileged
/// operations and intentionally omit an `operator: Address` parameter.
/// Access control for a compliance module is typically expressed in terms
/// of the module's admin and the compliance contract it is registered on
/// (e.g. requiring auth from the module admin, the compliance contract, or
/// both), rather than from a per-call operator. Implementors should
/// enforce the access policy that matches their deployment before
/// delegating to the corresponding `storage::*` helper.
#[contracttrait]
pub trait MaxBalance: ComplianceModule {
    /// Configures the Identity Registry Storage contract for `token`.
    ///
    /// # Arguments
    ///
    /// * `e` - Access to the Soroban environment.
    /// * `token` - The token whose IRS is being configured.
    /// * `irs` - The Identity Registry Storage contract address.
    ///
    /// # Notes
    ///
    /// No default implementation is provided because this is a privileged
    /// operation that requires custom access control. Access control should be
    /// enforced before calling
    /// [`crate::rwa::compliance::modules::storage::set_irs_address`] for the
    /// implementation.
    fn set_identity_registry_storage(e: &Env, token: Address, irs: Address);

    /// Sets the per-identity maximum balance for `token`.
    ///
    /// # Arguments
    ///
    /// * `e` - Access to the Soroban environment.
    /// * `token` - The token whose cap is being configured.
    /// * `max` - The maximum aggregate balance any identity may hold.
    ///
    /// # Errors
    ///
    /// * [`crate::rwa::compliance::modules::ComplianceModuleError::InvalidAmount`] -
    ///   When `max` is negative.
    ///
    /// # Events
    ///
    /// * topics - `["max_balance_set", token: Address]`
    /// * data - `[max: i128]`
    ///
    /// # Notes
    ///
    /// No default implementation is provided because this is a privileged
    /// operation that requires custom access control. Access control should be
    /// enforced before calling [`storage::set_max_balance`] for the
    /// implementation.
    fn set_max_balance(e: &Env, token: Address, max: i128);

    /// Pre-seeds the tracked aggregate balance for `identity` under `token`.
    ///
    /// # Arguments
    ///
    /// * `e` - Access to the Soroban environment.
    /// * `token` - The token whose tracked balance is being seeded.
    /// * `identity` - The identity address whose balance is being seeded.
    /// * `balance` - The balance to record for `identity`.
    ///
    /// # Errors
    ///
    /// * [`crate::rwa::compliance::modules::ComplianceModuleError::InvalidAmount`] -
    ///   When `balance` is negative.
    /// * [`crate::rwa::compliance::modules::ComplianceModuleError::PresetAlreadyCompleted`] -
    ///   When the preset phase has already been finalized.
    ///
    /// # Events
    ///
    /// * topics - `["id_balance_preset", token: Address, identity: Address]`
    /// * data - `[balance: i128]`
    ///
    /// # Notes
    ///
    /// * Intended for registering this module on a token that already has live
    ///   balances; only callable before [`MaxBalance::mark_preset_completed`].
    /// * No default implementation is provided because this is a privileged
    ///   operation that requires custom access control. Access control should
    ///   be enforced before calling [`storage::preset_id_balance`] for the
    ///   implementation.
    fn preset_id_balance(e: &Env, token: Address, identity: Address, balance: i128);

    /// Pre-seeds tracked balances for multiple identities in a single call.
    ///
    /// # Arguments
    ///
    /// * `e` - Access to the Soroban environment.
    /// * `token` - The token whose tracked balances are being seeded.
    /// * `identities` - The identity addresses to seed.
    /// * `balances` - The balances aligned positionally with `identities`. Must
    ///   have the same length as `identities`.
    ///
    /// # Errors
    ///
    /// * [`crate::rwa::compliance::modules::ComplianceModuleError::BatchSizeMismatch`] -
    ///   When `identities` and `balances` have different lengths.
    /// * [`crate::rwa::compliance::modules::ComplianceModuleError::InvalidAmount`] -
    ///   When any entry in `balances` is negative.
    /// * [`crate::rwa::compliance::modules::ComplianceModuleError::PresetAlreadyCompleted`] -
    ///   When the preset phase has already been finalized.
    ///
    /// # Events
    ///
    /// For each entry:
    /// * topics - `["id_balance_preset", token: Address, identity: Address]`
    /// * data - `[balance: i128]`
    ///
    /// # Notes
    ///
    /// No default implementation is provided because this is a privileged
    /// operation that requires custom access control. Access control should be
    /// enforced before calling [`storage::batch_preset_id_balances`] for the
    /// implementation.
    fn batch_preset_id_balances(
        e: &Env,
        token: Address,
        identities: Vec<Address>,
        balances: Vec<i128>,
    );

    /// Finalizes the preset phase for `token`.
    ///
    /// # Arguments
    ///
    /// * `e` - Access to the Soroban environment.
    /// * `token` - The token whose preset phase is being finalized.
    ///
    /// # Events
    ///
    /// * topics - `["preset_completed", token: Address]`
    /// * data - `[]`
    ///
    /// # Notes
    ///
    /// * After this call, any further preset attempts panic.
    /// * No default implementation is provided because this is a privileged
    ///   operation that requires custom access control. Access control should
    ///   be enforced before calling [`storage::mark_preset_completed`] for the
    ///   implementation.
    fn mark_preset_completed(e: &Env, token: Address);

    /// Returns the configured per-identity maximum balance for `token`.
    ///
    /// # Arguments
    ///
    /// * `e` - Access to the Soroban environment.
    /// * `token` - The token address.
    fn get_max_balance(e: &Env, token: Address) -> i128 {
        storage::get_max_balance(e, &token)
    }

    /// Returns the aggregate balance tracked for `identity` under `token`.
    ///
    /// # Arguments
    ///
    /// * `e` - Access to the Soroban environment.
    /// * `token` - The token address.
    /// * `identity` - The identity address.
    fn get_id_balance(e: &Env, token: Address, identity: Address) -> i128 {
        storage::get_id_balance(e, &token, &identity)
    }

    /// Returns `true` when the preset phase for `token` has been finalized.
    ///
    /// # Arguments
    ///
    /// * `e` - Access to the Soroban environment.
    /// * `token` - The token address.
    fn is_preset_completed(e: &Env, token: Address) -> bool {
        storage::is_preset_completed(e, &token)
    }
}

// ################## EVENTS ##################

/// Emitted when the per-identity maximum balance for a token is configured.
#[contractevent]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MaxBalanceSet {
    #[topic]
    pub token: Address,
    pub max: i128,
}

/// Emits a [`MaxBalanceSet`] event.
///
/// # Arguments
///
/// * `e` - Access to the Soroban environment.
/// * `token` - The token whose cap changed.
/// * `max` - The new per-identity maximum balance.
pub fn emit_max_balance_set(e: &Env, token: &Address, max: i128) {
    MaxBalanceSet { token: token.clone(), max }.publish(e);
}

/// Emitted when a tracked identity balance is pre-seeded during the
/// migration preset phase.
#[contractevent]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct IdBalancePreset {
    #[topic]
    pub token: Address,
    #[topic]
    pub identity: Address,
    pub balance: i128,
}

/// Emits an [`IdBalancePreset`] event.
///
/// # Arguments
///
/// * `e` - Access to the Soroban environment.
/// * `token` - The token whose tracked balance changed.
/// * `identity` - The identity whose balance was seeded.
/// * `balance` - The balance recorded for `identity`.
pub fn emit_id_balance_preset(e: &Env, token: &Address, identity: &Address, balance: i128) {
    IdBalancePreset { token: token.clone(), identity: identity.clone(), balance }.publish(e);
}

/// Emitted when the preset phase for a token is finalized.
#[contractevent]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PresetCompleted {
    #[topic]
    pub token: Address,
}

/// Emits a [`PresetCompleted`] event.
///
/// # Arguments
///
/// * `e` - Access to the Soroban environment.
/// * `token` - The token whose preset phase was finalized.
pub fn emit_preset_completed(e: &Env, token: &Address) {
    PresetCompleted { token: token.clone() }.publish(e);
}
