//! # Confidential Token Compliance Extension
//!
//! Deployer-configurable controls layered on top of the
//! [`ConfidentialToken`]: per-account freezing,
//! SAC `authorized()` passthrough, and a pluggable external
//! authorization policy. See `docs/COMPLIANCE.md` for the
//! specification.
//!
//! ## Surface
//!
//! 1. [`ComplianceHooks`] — a turnkey [`Hooks`] implementation that gates every
//!    token entry point against the active configuration. Wire as `type Hooks =
//!    ComplianceHooks;` on a contract that implements [`ConfidentialToken`].
//! 2. [`ConfidentialCompliance`] — the admin-facing trait.
//! 3. [`ConfidentialClawback`] — the opt-in seizure trait. Omitting its impl
//!    block is how a deployment ships freeze and policy gating without seizure
//!    capability.
//! 4. [`Policy`] — the cross-contract interface for an external allowlist /
//!    denylist / KYC / sanctions registry.
//! 5. Storage helpers in [`storage`].
//!
//! Deployments that never write a configuration pay only one instance-storage
//! probe per op: [`ComplianceHooks`] short-circuits when
//! [`storage::compliance_config`] returns `None`.

pub mod storage;

#[cfg(test)]
mod test;

use soroban_sdk::{
    contractclient, contracterror, contractevent, contracttrait, Address, Bytes, Env,
};
pub use storage::{ClawbackData, ComplianceConfig, ComplianceStorageKey};

use crate::confidential::{
    ConfidentialToken, Hooks, RegisterPayload, SetSpenderPayload, SpenderTransferPayload,
    TransferPayload, WithdrawPayload,
};

// ################## POLICY ##################

/// External authorization policy interface. Contracts implementing this
/// trait become pluggable allowlist / denylist / KYC / sanctions registries.
///
/// The token contract passes its own address as `token` so a single registry
/// can serve multiple tokens and apply per-token rules where needed.
#[contractclient(name = "PolicyClient")]
pub trait Policy {
    /// Returns `true` iff `account` is authorized to interact with
    /// `token`.
    fn is_authorized(e: Env, account: Address, token: Address) -> bool;
}

// ################## COMPLIANCE TRAIT ##################

/// Admin-facing compliance interface layered on top of
/// [`ConfidentialToken`]. Exposes freeze/unfreeze, configuration
/// rotation, and the matching read accessors.
///
/// ## Why the write methods have no default body
///
/// The write methods ([`ConfidentialCompliance::freeze`],
/// [`ConfidentialCompliance::unfreeze`],
/// [`ConfidentialCompliance::set_compliance_config`])
/// accept an `operator: Address` and intentionally ship without a default
/// implementation. Because the choice of access-control module is the contract
/// author's, the trait forces an explicit override. The override typically:
///
/// 1. Performs the authorization check — either via `operator.require_auth()`
///    plus a manual identity check, or by attaching `#[only_owner]` /
///    `#[only_role]` to the override (in which case the `operator` parameter is
///    passed through as the documented caller).
/// 2. Delegates to the matching helper in [`storage`].
#[contracttrait]
pub trait ConfidentialCompliance: ConfidentialToken {
    /// Marks `account` as frozen.
    ///
    /// # Arguments
    ///
    /// * `e` - Access to the Soroban environment.
    /// * `account` - The address to freeze.
    /// * `operator` - The address whose authorization gates this operation.
    ///
    /// # Errors
    ///
    /// * refer to [`storage::freeze`] errors.
    ///
    /// # Events
    ///
    /// * topics - `["frozen", account: Address]`
    /// * data - `[]`
    ///
    /// # Security Warning
    ///
    /// Implementations MUST authorize `operator` before calling
    /// [`storage::freeze`]. The trait cannot provide a default body
    /// — see the trait-level docstring for the rationale.
    fn freeze(e: &Env, account: Address, operator: Address);

    /// Clears the frozen flag on `account`.
    ///
    /// # Arguments
    ///
    /// * `e` - Access to the Soroban environment.
    /// * `account` - The address to unfreeze.
    /// * `operator` - The address whose authorization gates this operation.
    ///
    /// # Errors
    ///
    /// * refer to [`storage::unfreeze`] errors.
    ///
    /// # Events
    ///
    /// * topics - `["unfrozen", account: Address]`
    /// * data - `[]`
    ///
    /// # Security Warning
    ///
    /// Implementations MUST authorize `operator` before calling
    /// [`storage::unfreeze`]. The trait cannot provide a default
    /// body — see the trait-level docstring for the rationale.
    fn unfreeze(e: &Env, account: Address, operator: Address);

    /// Atomically replaces the compliance configuration with `config`. The
    /// intended deployment-time call is from the contract's
    /// `__constructor` (which may invoke
    /// [`storage::set_compliance_config`] directly); subsequent
    /// rotations flow through this method.
    ///
    /// # Arguments
    ///
    /// * `e` - Access to the Soroban environment.
    /// * `config` - The new [`ComplianceConfig`].
    /// * `operator` - The address whose authorization gates this operation.
    ///
    /// # Events
    ///
    /// * topics - `["compliance_config_changed"]`
    /// * data - `[policy: Option<Address>, sac_passthrough: bool]`
    ///
    /// # Security Warning
    ///
    /// Implementations MUST authorize `operator` before calling
    /// [`storage::set_compliance_config`]. The trait cannot provide a
    /// default body — see the trait-level docstring for the rationale.
    fn set_compliance_config(e: &Env, config: ComplianceConfig, operator: Address);

    /// Returns whether `account` is currently frozen. Returns `false` when
    /// compliance has not been configured.
    ///
    /// # Arguments
    ///
    /// * `e` - Access to the Soroban environment.
    /// * `account` - The address to check.
    fn is_frozen(e: &Env, account: Address) -> bool {
        storage::is_frozen(e, &account)
    }

    /// Returns the active [`ComplianceConfig`], or `None` when compliance
    /// has not been configured.
    ///
    /// # Arguments
    ///
    /// * `e` - Access to the Soroban environment.
    fn compliance_config(e: &Env) -> Option<ComplianceConfig> {
        storage::compliance_config(e)
    }
}

// ################## CLAWBACK TRAIT ##################

/// Opt-in seizure interface. A deployment that wants freeze and policy gating
/// but no seizure capability simply omits this impl block — a compile-time
/// choice with no storage, no configuration, and no misconfiguration mode.
///
/// Both methods follow the [`ConfidentialCompliance`] pattern: no default
/// body, so the contract author must supply the access-control check
/// explicitly. Both require the target to be frozen, and neither consults the
/// [`Hooks`] impl — gating them would be self-defeating, since the freeze gate
/// rejects exactly the accounts these methods exist to act on.
///
/// # Security Warning
///
/// **The freeze precondition is only meaningful when the deployment's `Hooks`
/// impl gates on it, and this trait's bounds do not force that.**
/// `ConfidentialClawback: ConfidentialCompliance` obliges the deployment to
/// implement `freeze` / `unfreeze`, but it places no constraint on
/// `<Self as ConfidentialToken>::Hooks`. A contract that wires
/// [`NoHooks`](crate::confidential::NoHooks) alongside this impl block gets a
/// `freeze` that writes the flag and an `is_frozen` that returns `true`, so
/// the precondition passes — while every token operation stays ungated and
/// the target spends its balance out before the seizure lands. The freeze is
/// then satisfied and meaningless, and the admin's only signal is an
/// `InvalidProof` once the commitment has moved.
///
/// Wiring [`ComplianceHooks`], or a custom [`Hooks`] impl that gates the same
/// seven positions (`on_deposit`, `on_merge`, `on_withdraw`, `on_transfer`,
/// `on_spender_transfer`, `on_set_spender`, `on_revoke_spender`), is a
/// **deployment obligation** of this trait.
#[contracttrait]
pub trait ConfidentialClawback: ConfidentialCompliance {
    /// Reduces `account`'s confidential claim by `amount` and settles the
    /// corresponding underlying according to `destination`.
    ///
    /// With `None`, no underlying is transferred: the pool is left
    /// over-collateralized by `amount`, and extraction is the issuer's own SAC
    /// `clawback` against this contract's address — safe in that order, and
    /// only in that order (see [`storage::clawback`]). With `Some(d)`, exactly
    /// `amount` is transferred to `d` in this invocation and the pool stays in
    /// step with the sum of confidential claims.
    ///
    /// `destination` is bound into the proof, so a proof built for one
    /// destination cannot be submitted against another, and `Some` naming this
    /// contract's own address is rejected.
    ///
    /// `account` MUST be frozen: the freeze is what holds `C_spend` and
    /// `C_receive` still between proof construction and submission, which is
    /// what the proof's bindings rely on. Note that the freeze only
    /// immobilizes the target if the deployment's [`Hooks`] impl gates on it —
    /// see the trait-level warning.
    ///
    /// # Arguments
    ///
    /// * `e` - Access to the Soroban environment.
    /// * `account` - The confidential account being seized from.
    /// * `amount` - The strictly positive seize amount.
    /// * `destination` - Where the underlying settles, or `None` for no
    ///   on-chain settlement from this contract.
    /// * `data` - XDR-encoded [`ClawbackData`].
    /// * `operator` - The address whose authorization gates this operation.
    ///
    /// # Errors
    ///
    /// * refer to [`crate::confidential::storage::decode_data`] errors.
    /// * refer to [`storage::clawback`] errors.
    ///
    /// # Events
    ///
    /// * topics - `["clawback", account: Address]`
    /// * data - `[amount: i128, destination: Option<Address>]`
    ///
    /// # Security Warning
    ///
    /// Implementations MUST authorize `operator` before calling
    /// [`storage::clawback`], which authorizes nobody. The trait cannot
    /// provide a default body — see [`ConfidentialCompliance`]'s trait-level
    /// docstring for the rationale.
    fn clawback(
        e: &Env,
        account: Address,
        amount: i128,
        destination: Option<Address>,
        data: Bytes,
        operator: Address,
    );

    /// Folds the `(account, spender)` delegation's escrowed allowance back
    /// into `account`'s spendable balance and deletes the delegation, without
    /// the owner's participation. This is what brings escrowed value into the
    /// reach of [`ConfidentialClawback::clawback`], which can only see
    /// `C_spend` and `C_receive`.
    ///
    /// Identical fold to the owner's
    /// [`revoke_spender`](crate::confidential::ConfidentialToken::revoke_spender);
    /// only the authorization gate differs. Works against expired delegations
    /// too — expiry blocks spending, never reclamation.
    ///
    /// `account` MUST be frozen.
    ///
    /// # Arguments
    ///
    /// * `e` - Access to the Soroban environment.
    /// * `account` - The delegating owner.
    /// * `spender` - The delegated spender.
    /// * `operator` - The address whose authorization gates this operation.
    ///
    /// # Errors
    ///
    /// * refer to [`storage::force_revoke_spender`] errors.
    ///
    /// # Events
    ///
    /// * topics - `["revoke_spender", account: Address, spender: Address]`
    /// * data - `[a_tilde: BytesN<32>, allowance_salt: BytesN<32>]`
    ///
    /// # Security Warning
    ///
    /// Implementations MUST authorize `operator` before calling
    /// [`storage::force_revoke_spender`], which authorizes nobody.
    fn force_revoke_spender(e: &Env, account: Address, spender: Address, operator: Address);
}

// ################## HOOKS IMPL ##################

/// [`Hooks`] implementation that gates every token callback against
/// the active [`ComplianceConfig`]. Wire as `type Hooks = ComplianceHooks;`
/// on a contract that implements [`ConfidentialToken`].
///
/// Each gated party is checked against up to three gates when a configuration
/// is present:
///
/// 1. Reverts [`ComplianceError::AccountFrozen`] if the address is frozen.
/// 2. When `config.policy = Some(p)`, calls `p.is_authorized` and reverts
///    [`ComplianceError::NotAuthorizedByPolicy`] on `false`.
/// 3. When `config.sac_passthrough = true`, calls the underlying SAC's
///    `authorized` view (a Stellar Asset Contract interface, not SEP-41; see
///    [`storage::check_sac`]) and reverts
///    [`ComplianceError::NotAuthorizedBySac`] on `false`.
///
/// Which parties pass which gates:
///
/// * [`on_deposit`](Hooks::on_deposit), [`on_withdraw`](Hooks::on_withdraw),
///   [`on_transfer`](Hooks::on_transfer): `from` and `to` pass all three gates.
///   [`on_merge`](Hooks::on_merge): `account` passes all three.
/// * [`on_register`](Hooks::on_register): `account` skips the freeze gate
///   (registration predates the account entry) but passes policy and SAC.
/// * [`on_register`](Hooks::on_register) does not restrict the caller-selected
///   `auditor_id`. Deployments that must limit which auditors an account may
///   bind to override it with a custom gate (see `docs/COMPLIANCE.md` §4.3).
/// * [`on_spender_transfer`](Hooks::on_spender_transfer): `from` and `to` pass
///   all three gates; `spender` passes only the policy gate.
/// * [`on_set_spender`](Hooks::on_set_spender): the delegating `account` passes
///   all three gates; `spender` passes only the policy gate, so a delegation to
///   a policy-denied spender fails at grant time rather than at spend time.
/// * [`on_revoke_spender`](Hooks::on_revoke_spender): `account` passes all
///   three gates; `spender` is not gated at all. Revocation is the owner's
///   escape hatch — blocking it once the spender turns non-compliant would
///   entrench the bad delegation.
///
/// The spender is exempt from the freeze and SAC gates everywhere: both
/// target fund ownership, and the spender holds no funds in this model,
/// mirroring the fungible and rwa allowance models.
///
/// Deployments that need additional behaviour (audit mirroring, rate
/// limiting, or alternative deposit semantics — see
/// `docs/COMPLIANCE.md` §4) can write a custom `Hooks` impl that
/// calls the same primitives.
pub struct ComplianceHooks;

impl Hooks for ComplianceHooks {
    fn on_register(e: &Env, account: &Address, _auditor_id: u32, _payload: &RegisterPayload) {
        let Some(config) = storage::compliance_config(e) else {
            return;
        };
        storage::check_policy(e, account, &config);
        storage::check_sac(e, account, &config);
    }

    fn on_deposit(e: &Env, from: &Address, to: &Address, _amount: i128) {
        let Some(config) = storage::compliance_config(e) else {
            return;
        };
        storage::gate_account(e, from, &config);
        storage::gate_account(e, to, &config);
    }

    fn on_merge(e: &Env, account: &Address) {
        let Some(config) = storage::compliance_config(e) else {
            return;
        };
        storage::gate_account(e, account, &config);
    }

    fn on_withdraw(
        e: &Env,
        from: &Address,
        to: &Address,
        _amount: i128,
        _payload: &WithdrawPayload,
    ) {
        let Some(config) = storage::compliance_config(e) else {
            return;
        };
        storage::gate_account(e, from, &config);
        storage::gate_account(e, to, &config);
    }

    fn on_transfer(e: &Env, from: &Address, to: &Address, _payload: &TransferPayload) {
        let Some(config) = storage::compliance_config(e) else {
            return;
        };
        storage::gate_account(e, from, &config);
        storage::gate_account(e, to, &config);
    }

    fn on_spender_transfer(
        e: &Env,
        spender: &Address,
        from: &Address,
        to: &Address,
        _payload: &SpenderTransferPayload,
    ) {
        let Some(config) = storage::compliance_config(e) else {
            return;
        };
        // `from` and `to` pass all three gates; the spender holds no funds, so
        // it skips freeze and SAC (consistent with the fungible and rwa
        // allowance models) but must still clear the external policy.
        storage::gate_account(e, from, &config);
        storage::gate_account(e, to, &config);
        storage::check_policy(e, spender, &config);
    }

    fn on_set_spender(
        e: &Env,
        account: &Address,
        spender: &Address,
        _live_until_ledger: u32,
        _payload: &SetSpenderPayload,
    ) {
        let Some(config) = storage::compliance_config(e) else {
            return;
        };
        storage::gate_account(e, account, &config);
        storage::check_policy(e, spender, &config);
    }

    fn on_revoke_spender(e: &Env, account: &Address, _spender: &Address) {
        let Some(config) = storage::compliance_config(e) else {
            return;
        };
        storage::gate_account(e, account, &config);
    }
}

// ################## ERRORS ##################

#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq, PartialOrd, Ord)]
#[repr(u32)]
pub enum ComplianceError {
    /// Indicates an admin operation was invoked before
    /// [`storage::set_compliance_config`] established a configuration.
    NotConfigured = 3600,
    /// Indicates the target account is frozen.
    AccountFrozen = 3601,
    /// Indicates the configured policy returned `false` for the target
    /// account.
    NotAuthorizedByPolicy = 3602,
    /// Indicates the underlying SAC's `authorized()` view returned `false`
    /// for the target account (only reachable when `sac_passthrough` is
    /// enabled).
    NotAuthorizedBySac = 3603,
    /// Indicates the clawback target is not frozen. The freeze is what holds
    /// the target's commitments still between proof construction and
    /// submission.
    AccountNotFrozen = 3604,
    /// Indicates the seize amount is not strictly positive. Load-bearing
    /// beyond diagnostics: it guards the `amount as u128` cast in
    /// post-verification, and it is what makes a clawback proof
    /// non-replayable.
    InvalidClawbackAmount = 3605,
    /// Indicates `destination` is `Some` naming this contract's own address,
    /// which would create pool surplus while reporting a settlement.
    InvalidClawbackDestination = 3606,
}

// ################## CONSTANTS ##################

const DAY_IN_LEDGERS: u32 = 17280;
pub const FROZEN_EXTEND_AMOUNT: u32 = 30 * DAY_IN_LEDGERS;
pub const FROZEN_TTL_THRESHOLD: u32 = FROZEN_EXTEND_AMOUNT - DAY_IN_LEDGERS;

// ################## EVENTS ##################

/// Event emitted when an account is frozen.
#[contractevent]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Frozen {
    #[topic]
    pub account: Address,
}

/// Emits a [`Frozen`] event.
pub fn emit_frozen(e: &Env, account: &Address) {
    Frozen { account: account.clone() }.publish(e);
}

/// Event emitted when an account is unfrozen.
#[contractevent]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Unfrozen {
    #[topic]
    pub account: Address,
}

/// Emits an [`Unfrozen`] event.
pub fn emit_unfrozen(e: &Env, account: &Address) {
    Unfrozen { account: account.clone() }.publish(e);
}

/// Event emitted when a confidential claim is reduced by a compliance
/// seizure. `destination` is `None` when no underlying moved — the contract's
/// pooled balance is then left over-collateralized by `amount` — and `Some(d)`
/// when exactly `amount` was transferred to `d` in the same invocation.
#[contractevent]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Clawback {
    #[topic]
    pub account: Address,
    pub amount: i128,
    pub destination: Option<Address>,
}

/// Emits a [`Clawback`] event.
pub fn emit_clawback(e: &Env, account: &Address, amount: i128, destination: &Option<Address>) {
    Clawback { account: account.clone(), amount, destination: destination.clone() }.publish(e);
}

/// Event emitted when the compliance configuration is set or rotated.
#[contractevent]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ComplianceConfigChanged {
    pub policy: Option<Address>,
    pub sac_passthrough: bool,
}

/// Emits a [`ComplianceConfigChanged`] event.
pub fn emit_compliance_config_changed(e: &Env, policy: &Option<Address>, sac_passthrough: bool) {
    ComplianceConfigChanged { policy: policy.clone(), sac_passthrough }.publish(e);
}
