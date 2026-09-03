//! # Confidential Token Compliance Extension
//!
//! Deployer-configurable controls layered on top of the [`ConfidentialToken`]:
//! per-account freezing, SAC `authorized()` passthrough, a pluggable external
//! authorization policy, and opt-in seizure. See `docs/COMPLIANCE.md` for the
//! specification.
//!
//! ## Surface
//!
//! 1. [`ComplianceHooks`] — a ready-made [`Hooks`] implementation that checks
//!    every token entry point against the active configuration. Wire it as
//!    `type Hooks = ComplianceHooks;` on a contract that implements
//!    [`ConfidentialToken`].
//! 2. [`ConfidentialCompliance`] — the admin-facing trait: freeze, unfreeze,
//!    and configuration rotation.
//! 3. [`ConfidentialClawback`] — the opt-in seizure trait. A deployment that
//!    does not implement it keeps freeze and policy checks but has no seizure
//!    capability.
//! 4. [`Policy`] — the cross-contract interface for an external allowlist /
//!    denylist / KYC / sanctions registry.
//! 5. Storage helpers in [`storage`].
//!
//! Deployments that never write a configuration pay one instance-storage read
//! per operation: [`ComplianceHooks`] returns early when
//! [`storage::compliance_config`] is `None`.

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

/// External authorization policy interface. A contract implementing this
/// trait serves as a pluggable allowlist, denylist, KYC, or sanctions
/// registry.
///
/// The token contract passes its own address as `token`, so one registry can
/// serve several tokens and apply per-token rules.
#[contractclient(name = "PolicyClient")]
pub trait Policy {
    /// Returns `true` if and only if `account` is authorized to interact with
    /// `token`.
    fn is_authorized(e: Env, account: Address, token: Address) -> bool;
}

// ################## COMPLIANCE TRAIT ##################

/// Admin-facing compliance interface layered on top of [`ConfidentialToken`]:
/// freeze, unfreeze, configuration rotation, and the matching read accessors.
///
/// ## Why the write methods have no default body
///
/// [`ConfidentialCompliance::freeze`], [`ConfidentialCompliance::unfreeze`],
/// and [`ConfidentialCompliance::set_compliance_config`] take an
/// `operator: Address` and have no default implementation. The choice of
/// access-control module belongs to the contract author, so the trait requires
/// an explicit override. The override:
///
/// 1. Authorizes the call — either with `operator.require_auth()` plus a check
///    that `operator` holds the required role, or with `#[only_owner]` /
///    `#[only_role]` on the override, in which case the macro performs the
///    check and `operator` is informational.
/// 2. Delegates to the matching helper in [`storage`].
#[contracttrait]
pub trait ConfidentialCompliance: ConfidentialToken {
    /// Marks `account` as frozen.
    ///
    /// # Arguments
    ///
    /// * `e` - Access to the Soroban environment.
    /// * `account` - The address to freeze.
    /// * `operator` - The address that must authorize the call.
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
    /// [`storage::freeze`]. The trait-level docstring explains why the method
    /// has no default body.
    fn freeze(e: &Env, account: Address, operator: Address);

    /// Clears the frozen flag on `account`.
    ///
    /// # Arguments
    ///
    /// * `e` - Access to the Soroban environment.
    /// * `account` - The address to unfreeze.
    /// * `operator` - The address that must authorize the call.
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
    /// [`storage::unfreeze`]. The trait-level docstring explains why the
    /// method has no default body.
    fn unfreeze(e: &Env, account: Address, operator: Address);

    /// Replaces the compliance configuration with `config`. The initial
    /// configuration is normally written from the contract's `__constructor`,
    /// which may call [`storage::set_compliance_config`] directly; later
    /// rotations go through this method.
    ///
    /// # Arguments
    ///
    /// * `e` - Access to the Soroban environment.
    /// * `config` - The new [`ComplianceConfig`].
    /// * `operator` - The address that must authorize the call.
    ///
    /// # Events
    ///
    /// * topics - `["compliance_config_changed"]`
    /// * data - `[policy: Option<Address>, sac_passthrough: bool]`
    ///
    /// # Security Warning
    ///
    /// Implementations MUST authorize `operator` before calling
    /// [`storage::set_compliance_config`]. The trait-level docstring explains
    /// why the method has no default body.
    fn set_compliance_config(e: &Env, config: ComplianceConfig, operator: Address);

    /// Returns whether `account` is frozen, or `false` when compliance has not
    /// been configured.
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

/// Opt-in seizure interface. A deployment that wants freeze and policy checks
/// but no seizure capability does not implement this trait.
///
/// Both methods follow the [`ConfidentialCompliance`] pattern: no default
/// body, so the contract author supplies the access-control check. Both
/// require `account` to be frozen, and neither runs the [`Hooks`] callbacks: a
/// freeze-aware `Hooks` impl would reject the frozen accounts these methods
/// act on.
///
/// # Security Warning
///
/// **The freeze precondition is only meaningful when the deployment's `Hooks`
/// impl rejects operations from frozen accounts, and the trait bounds do not
/// require such an impl.** `ConfidentialClawback: ConfidentialCompliance`
/// obliges the deployment to implement `freeze` / `unfreeze` but places no
/// constraint on `<Self as ConfidentialToken>::Hooks`. A contract that wires
/// [`NoHooks`](crate::confidential::NoHooks) alongside this trait gets a
/// `freeze` that writes the flag and an `is_frozen` that returns `true`, so
/// the precondition passes while every token operation remains unrestricted
/// and the target can move its balance before the seizure executes. The only
/// signal to the caller is an `InvalidProof` error once the commitment has
/// moved.
///
/// Wiring [`ComplianceHooks`], or a custom [`Hooks`] impl that rejects frozen
/// accounts in `on_withdraw`, `on_transfer`, and the other balance-moving
/// callbacks, is a **deployment obligation** of this trait.
#[contracttrait]
pub trait ConfidentialClawback: ConfidentialCompliance {
    /// Reduces `account`'s confidential claim by `amount` and settles the
    /// corresponding underlying according to `destination`.
    ///
    /// With `destination = None`, no underlying is transferred: the pool is
    /// left over-collateralized by `amount`, and the issuer extracts the
    /// surplus with the SAC's own `clawback` against this contract's address.
    /// With `destination = Some(d)`, exactly `amount` is transferred to `d` in
    /// the same invocation and the pool remains equal to the sum of
    /// confidential claims.
    ///
    /// `destination` is bound into the proof, so a proof built for one
    /// destination cannot be submitted with another. `Some(d)` with `d` equal
    /// to this contract's own address is rejected.
    ///
    /// `account` MUST be frozen: the freeze keeps `C_spend` and `C_receive`
    /// unchanged between proof construction and submission, which the proof's
    /// bindings rely on. The freeze only immobilizes the target if the
    /// deployment's [`Hooks`] impl rejects operations from frozen accounts —
    /// see the trait-level warning.
    ///
    /// # Arguments
    ///
    /// * `e` - Access to the Soroban environment.
    /// * `account` - The confidential account whose claim is reduced.
    /// * `amount` - The amount to seize; must be strictly positive.
    /// * `destination` - The recipient of the seized underlying, or `None` to
    ///   leave the underlying in the pool.
    /// * `data` - XDR-encoded [`ClawbackData`].
    /// * `operator` - The address that must authorize the call.
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
    /// [`storage::clawback`]. The [`ConfidentialCompliance`] trait-level
    /// docstring explains why the method has no default body.
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
    /// the owner's participation, so the escrowed value becomes reachable by
    /// [`ConfidentialClawback::clawback`].
    ///
    /// The fold is the same as the owner's
    /// [`revoke_spender`](crate::confidential::ConfidentialToken::revoke_spender);
    /// only the authorizing party differs.
    ///
    /// `account` MUST be frozen.
    ///
    /// # Arguments
    ///
    /// * `e` - Access to the Soroban environment.
    /// * `account` - The delegating owner.
    /// * `spender` - The delegated spender.
    /// * `operator` - The address that must authorize the call.
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
    /// [`storage::force_revoke_spender`]. The [`ConfidentialCompliance`]
    /// trait-level docstring explains why the method has no default body.
    fn force_revoke_spender(e: &Env, account: Address, spender: Address, operator: Address);
}

// ################## HOOKS IMPL ##################

/// [`Hooks`] implementation that checks every token callback against the
/// active [`ComplianceConfig`]. Wire it as `type Hooks = ComplianceHooks;` on
/// a contract that implements [`ConfidentialToken`].
///
/// When a configuration is present, each checked address passes up to three
/// gates:
///
/// 1. Freeze: fails with [`ComplianceError::AccountFrozen`] if the address is
///    frozen.
/// 2. Policy: when `config.policy = Some(p)`, calls `p.is_authorized` and fails
///    with [`ComplianceError::NotAuthorizedByPolicy`] on `false`.
/// 3. SAC: when `config.sac_passthrough = true`, calls the underlying SAC's
///    `authorized` view (a Stellar Asset Contract interface, not SEP-41; see
///    [`storage::check_sac`]) and fails with
///    [`ComplianceError::NotAuthorizedBySac`] on `false`.
///
/// Which parties pass which gates:
///
/// * [`on_deposit`](Hooks::on_deposit), [`on_withdraw`](Hooks::on_withdraw),
///   [`on_transfer`](Hooks::on_transfer): `from` and `to` pass all three gates.
///   [`on_merge`](Hooks::on_merge): `account` passes all three.
/// * [`on_register`](Hooks::on_register): `account` skips the freeze gate
///   (registration predates the account entry) but passes policy and SAC. The
///   caller-selected `auditor_id` is not restricted; deployments that must
///   limit which auditors an account may bind to override `on_register` with a
///   custom gate (see `docs/COMPLIANCE.md` §4.3).
/// * [`on_spender_transfer`](Hooks::on_spender_transfer): `from` and `to` pass
///   all three gates; `spender` passes only the policy gate.
/// * [`on_set_spender`](Hooks::on_set_spender): the delegating `account` passes
///   all three gates; `spender` passes only the policy gate, so a delegation to
///   a policy-denied spender fails at grant time rather than at spend time.
/// * [`on_revoke_spender`](Hooks::on_revoke_spender): `account` passes all
///   three gates; `spender` passes none. Blocking revocation once the spender
///   becomes non-compliant would trap the owner in the delegation.
///
/// The spender skips the freeze and SAC gates everywhere: both concern fund
/// ownership, and a spender holds no funds, as in the fungible and rwa
/// allowance models.
///
/// Deployments that need additional behaviour (audit mirroring, rate
/// limiting, or alternative deposit semantics — see `docs/COMPLIANCE.md` §4)
/// can write a custom `Hooks` impl that calls the same primitives.
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
    /// Indicates the clawback target is not frozen.
    AccountNotFrozen = 3604,
    /// Indicates the seize amount is not strictly positive.
    InvalidClawbackAmount = 3605,
    /// Indicates `destination` names this contract's own address.
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
/// seizure. `destination` is `None` when no underlying moved, and `Some(d)`
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
