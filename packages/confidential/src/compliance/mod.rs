//! # Confidential Token Compliance Extension
//!
//! Deployer-configurable controls layered on top of the [`ConfidentialToken`]:
//! per-account freezing, SAC `authorized()` passthrough, a pluggable external
//! authorization policy, and opt-in seizure. See `docs/compliance.md` for the
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
//! ## RWA profile
//!
//! A regulated deployment wires [`RwaHooks`] instead of [`ComplianceHooks`]
//! and implements [`ConfidentialRwa`]. The profile adds a designated auditor
//! every account must bind to, an optional approver whose authorization
//! every value-moving operation additionally requires, and — through the
//! opt-in [`ConfidentialForcedTransfer`] — a forced transfer between
//! confidential accounts. See `docs/compliance.md#rwa-profile` and
//! `docs/rwa.md`.
//!
//! Deployments that never write a configuration pay one instance-storage read
//! per operation: [`ComplianceHooks`] returns early when
//! [`storage::compliance_config`] is `None`.

pub mod storage;

#[cfg(test)]
mod test;

use soroban_sdk::{
    contractclient, contracterror, contractevent, contracttrait, panic_with_error, Address, Bytes,
    Env,
};
pub use storage::{ClawbackData, ComplianceConfig, ComplianceStorageKey, RwaConfig};

use crate::{
    storage::get_account, ConfidentialToken, Hooks, RegisterPayload, SetSpenderPayload,
    SpenderTransferPayload, TransferPayload, WithdrawPayload,
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
    /// * data - `[policy: Option<Address>, sac_passthrough: bool]` (`policy` is
    ///   left out of the data when it is `None`)
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
/// [`NoHooks`](crate::NoHooks) alongside this trait gets a
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
    /// The proof is producible only by the auditor `account` is bound to, and
    /// `destination` is bound into it, so a proof built for one destination
    /// cannot be submitted with another. `Some(d)` with `d` equal to this
    /// contract's own address is rejected; any other `d` is accepted without
    /// passing the freeze, policy, or SAC gates. The account's clawback nonce
    /// is bound into it as well and advances on every seizure, so a proof
    /// executes at most once.
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
    /// * refer to [`crate::storage::decode_data`] errors.
    /// * refer to [`storage::clawback`] errors.
    ///
    /// # Events
    ///
    /// * topics - `["clawback", account: Address]`
    /// * data - `[amount: i128, destination: Option<Address>]` (`destination`
    ///   is left out of the data when it is `None`)
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
    /// [`revoke_spender`](crate::ConfidentialToken::revoke_spender);
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

    /// Returns the number of seizures executed against `account`, which the
    /// auditor binds into the next clawback proof.
    ///
    /// # Arguments
    ///
    /// * `e` - Access to the Soroban environment.
    /// * `account` - The confidential account to query.
    fn clawback_nonce(e: &Env, account: Address) -> u32 {
        storage::clawback_nonce(e, &account)
    }
}

// ################## RWA TRAITS ##################

/// Admin and read surface of the RWA profile
/// (`docs/compliance.md#rwa-profile`).
///
/// A contract that implements this trait MUST wire `type Hooks = RwaHooks;`
/// on its [`ConfidentialToken`] impl: the configuration written here is
/// enforced only by [`RwaHooks`].
///
/// The implementor supplies the bodies of the two admin entry points and
/// thereby the access-control model, as for [`ConfidentialCompliance`].
#[contracttrait]
pub trait ConfidentialRwa: ConfidentialCompliance {
    /// Sets the designated auditor and the approver, and advances the
    /// approval epoch. The designated auditor is fixed by the first call.
    ///
    /// # Arguments
    ///
    /// * `e` - Access to the Soroban environment.
    /// * `auditor_id` - The only auditor id accounts may be bound to.
    /// * `approver` - The address whose authorization every value-moving
    ///   operation additionally requires, or `None` to disable the approval
    ///   gate.
    /// * `operator` - The address authorizing this call.
    ///
    /// # Errors
    ///
    /// * refer to [`storage::set_rwa_config`] errors.
    ///
    /// # Events
    ///
    /// * topics - `["rwa_config_changed"]`
    /// * data - `[auditor_id: u32, approver: Option<Address>, epoch: u32]`
    ///
    /// # Notes
    ///
    /// A contract approver is authorized automatically whenever it invokes
    /// the token itself, without its `__check_auth` running. It must expose
    /// no path through which a third party can make it call the token, and
    /// any such path of its own must check the approval epoch
    /// (`docs/compliance.md#approval-gate`).
    ///
    /// # Security Warning
    ///
    /// Implementations MUST authorize `operator` before calling
    /// [`storage::set_rwa_config`]. The method has no default body because
    /// the trait does not prescribe an access-control model.
    fn set_rwa_config(e: &Env, auditor_id: u32, approver: Option<Address>, operator: Address);

    /// Advances the approval epoch, voiding every outstanding approver
    /// authorization.
    ///
    /// # Arguments
    ///
    /// * `e` - Access to the Soroban environment.
    /// * `operator` - The address authorizing this call.
    ///
    /// # Errors
    ///
    /// * refer to [`storage::bump_approval_epoch`] errors.
    ///
    /// # Events
    ///
    /// * topics - `["approval_epoch_bumped"]`
    /// * data - `[epoch: u32]`
    ///
    /// # Security Warning
    ///
    /// Implementations MUST authorize `operator` before calling
    /// [`storage::bump_approval_epoch`].
    fn bump_approval_epoch(e: &Env, operator: Address);

    /// Returns the active [`RwaConfig`], or `None` when the profile has not
    /// been configured.
    ///
    /// # Arguments
    ///
    /// * `e` - Access to the Soroban environment.
    fn rwa_config(e: &Env) -> Option<RwaConfig> {
        storage::rwa_config(e)
    }

    /// Returns the current approval epoch.
    ///
    /// # Arguments
    ///
    /// * `e` - Access to the Soroban environment.
    fn approval_epoch(e: &Env) -> u32 {
        storage::approval_epoch(e)
    }

    /// Returns whether `account` is publicly eligible to send. Mirrors
    /// ERC-8424 `canSend`: an eligibility predicate that encodes no
    /// quantitative rule and raises no contract error. See
    /// [`storage::can_transact`].
    ///
    /// # Arguments
    ///
    /// * `e` - Access to the Soroban environment.
    /// * `account` - The confidential account to query.
    fn can_send(e: &Env, account: Address) -> bool {
        storage::can_transact(e, &account)
    }

    /// Returns whether `account` is publicly eligible to receive. Mirrors
    /// ERC-8424 `canReceive`. See [`storage::can_transact`].
    ///
    /// # Arguments
    ///
    /// * `e` - Access to the Soroban environment.
    /// * `account` - The confidential account to query.
    fn can_receive(e: &Env, account: Address) -> bool {
        storage::can_transact(e, &account)
    }
}

/// Opt-in forced transfer between confidential accounts
/// (`docs/compliance.md#forced-transfer`). Recovery of a lost account is a
/// forced transfer of its full balance.
///
/// # Deployment obligation
///
/// As for [`ConfidentialClawback`], the source must be frozen and the freeze
/// must actually gate it: a contract implementing this trait MUST wire
/// [`RwaHooks`].
///
/// # Proof-system restriction
///
/// The proof carries the auditor's secret key as a witness. Until the
/// verifier accepts zero-knowledge proofs
/// (`packages/confidential/README.md`), each proof may leak bits of that
/// key; do not use this entry point, or [`ConfidentialClawback::clawback`],
/// on a public network before then.
#[contracttrait]
pub trait ConfidentialForcedTransfer: ConfidentialClawback + ConfidentialRwa {
    /// Moves the public `amount` from the frozen account `from` to the
    /// registered account `to` inside the pool, against a clawback proof
    /// built by the auditor `from` is bound to.
    ///
    /// # Arguments
    ///
    /// * `e` - Access to the Soroban environment.
    /// * `from` - The frozen confidential account being debited.
    /// * `to` - The confidential account being credited.
    /// * `amount` - The strictly positive amount.
    /// * `data` - XDR-encoded [`ClawbackData`].
    /// * `operator` - The address authorizing this call.
    ///
    /// # Errors
    ///
    /// * refer to [`crate::storage::decode_data`] errors.
    /// * refer to [`storage::forced_transfer`] errors.
    ///
    /// # Events
    ///
    /// * topics - `["forced_transfer", from: Address, to: Address]`
    /// * data - `[amount: i128, nonce: u32]`
    ///
    /// # Security Warning
    ///
    /// Implementations MUST authorize `operator` before decoding `data` into
    /// [`ClawbackData`] and calling [`storage::forced_transfer`]. The call
    /// moves funds without the owner's authorization.
    fn forced_transfer(
        e: &Env,
        from: Address,
        to: Address,
        amount: i128,
        data: Bytes,
        operator: Address,
    );
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
///   custom gate (see `docs/compliance.md#restrict-auditor-selection`).
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
/// limiting, or alternative deposit semantics — see
/// `docs/compliance.md#customizing-the-hooks-trait`) can write a custom `Hooks`
/// impl that calls the same primitives.
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

/// [`Hooks`] implementation of the RWA profile
/// (`docs/compliance.md#rwa-profile`). Wire it as `type Hooks = RwaHooks;`
/// on a contract that implements [`ConfidentialToken`] and
/// [`ConfidentialRwa`].
///
/// It differs from [`ComplianceHooks`] in five ways:
///
/// 1. **Fails closed.** Every callback reverts unless both the
///    [`ComplianceConfig`] and the [`RwaConfig`] are set and the former keeps
///    an eligibility gate (a policy or SAC passthrough).
/// 2. **Designated auditor.** `on_register` accepts only the designated auditor
///    id. Sending, receiving and delegating require the confidential accounts
///    involved to be bound to it. `on_merge`, `on_revoke_spender` and
///    `on_withdraw` do not, so an account bound to another auditor (registered
///    before the profile was adopted) can still exit.
/// 3. **Approval gate.** When an approver is configured, `on_deposit`,
///    `on_withdraw`, `on_transfer` and `on_spender_transfer` additionally
///    require its authorization over the approval epoch, the parties and the
///    operation's payload (see [`storage::require_approval`]).
/// 4. **Spender.** The spender of a delegated flow passes the freeze, policy
///    and SAC gates, not the policy gate alone: a frozen or unauthorized party
///    must not direct regulated value. `on_revoke_spender` still gates only the
///    owner.
/// 5. **Deposits.** The recipient of a deposit must be a registered account.
///
/// The hooks enforce no balance or volume rule themselves; such rules are the
/// approver's (`docs/rwa.md`).
pub struct RwaHooks;

impl Hooks for RwaHooks {
    fn on_register(e: &Env, account: &Address, auditor_id: u32, _payload: &RegisterPayload) {
        let (config, rwa) = storage::rwa_context(e);
        if auditor_id != rwa.auditor_id {
            panic_with_error!(e, ComplianceError::AuditorNotAllowed);
        }
        storage::check_policy(e, account, &config);
        storage::check_sac(e, account, &config);
    }

    fn on_deposit(e: &Env, from: &Address, to: &Address, amount: i128) {
        let (config, rwa) = storage::rwa_context(e);
        storage::gate_account(e, from, &config);
        storage::gate_account(e, to, &config);
        storage::check_auditor(e, &get_account(e, to), &rwa);
        storage::require_approval(
            e,
            &rwa,
            &[from, to],
            storage::deposit_approval_args(e, from, to, amount),
        );
    }

    fn on_merge(e: &Env, account: &Address) {
        let (config, _) = storage::rwa_context(e);
        storage::gate_account(e, account, &config);
    }

    fn on_withdraw(e: &Env, from: &Address, to: &Address, amount: i128, payload: &WithdrawPayload) {
        let (config, rwa) = storage::rwa_context(e);
        storage::gate_account(e, from, &config);
        storage::gate_account(e, to, &config);
        storage::require_approval(
            e,
            &rwa,
            &[from, to],
            storage::withdraw_approval_args(e, from, to, amount, payload),
        );
    }

    fn on_transfer(e: &Env, from: &Address, to: &Address, payload: &TransferPayload) {
        let (config, rwa) = storage::rwa_context(e);
        storage::gate_account(e, from, &config);
        storage::gate_account(e, to, &config);
        storage::check_auditor(e, &get_account(e, from), &rwa);
        storage::check_auditor(e, &get_account(e, to), &rwa);
        storage::require_approval(
            e,
            &rwa,
            &[from, to],
            storage::transfer_approval_args(e, from, to, payload),
        );
    }

    fn on_spender_transfer(
        e: &Env,
        spender: &Address,
        from: &Address,
        to: &Address,
        payload: &SpenderTransferPayload,
    ) {
        let (config, rwa) = storage::rwa_context(e);
        storage::gate_account(e, from, &config);
        storage::gate_account(e, to, &config);
        storage::gate_account(e, spender, &config);
        storage::check_auditor(e, &get_account(e, from), &rwa);
        storage::check_auditor(e, &get_account(e, to), &rwa);
        storage::require_approval(
            e,
            &rwa,
            &[spender, from, to],
            storage::spender_transfer_approval_args(e, spender, from, to, payload),
        );
    }

    fn on_set_spender(
        e: &Env,
        account: &Address,
        spender: &Address,
        _live_until_ledger: u32,
        _payload: &SetSpenderPayload,
    ) {
        let (config, rwa) = storage::rwa_context(e);
        storage::gate_account(e, account, &config);
        storage::gate_account(e, spender, &config);
        storage::check_auditor(e, &get_account(e, account), &rwa);
    }

    fn on_revoke_spender(e: &Env, account: &Address, _spender: &Address) {
        let (config, _) = storage::rwa_context(e);
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
    /// Indicates an RWA operation was invoked before
    /// [`storage::set_rwa_config`] established an [`RwaConfig`].
    RwaNotConfigured = 3607,
    /// Indicates a registration named an auditor other than the designated
    /// one.
    AuditorNotAllowed = 3608,
    /// Indicates an operation named a confidential account bound to an
    /// auditor other than the designated one.
    AuditorMismatch = 3609,
    /// Indicates an attempt to change the designated auditor.
    AuditorAlreadySet = 3610,
    /// Indicates the approver is this contract's own address.
    InvalidApprover = 3611,
    /// Indicates the approver is a party to the operation it would approve.
    ApproverIsParty = 3612,
    /// Indicates the forced-transfer amount is not strictly positive.
    InvalidForcedTransferAmount = 3613,
    /// Indicates the forced-transfer destination is the source, this
    /// contract's own address, or maps to the zero field.
    InvalidForcedTransferDestination = 3614,
    /// Indicates a compliance configuration of an RWA deployment has neither
    /// a policy nor SAC passthrough.
    EligibilityRequired = 3615,
    /// Indicates the approval epoch cannot advance past `u32::MAX`.
    ApprovalEpochOverflow = 3616,
    /// Indicates the account's clawback nonce cannot advance past `u32::MAX`.
    NonceOverflow = 3617,
}

// ################## CONSTANTS ##################

const DAY_IN_LEDGERS: u32 = 17280;
pub const FROZEN_EXTEND_AMOUNT: u32 = 30 * DAY_IN_LEDGERS;
pub const FROZEN_TTL_THRESHOLD: u32 = FROZEN_EXTEND_AMOUNT - DAY_IN_LEDGERS;
pub const CLAWBACK_NONCE_EXTEND_AMOUNT: u32 = 30 * DAY_IN_LEDGERS;
pub const CLAWBACK_NONCE_TTL_THRESHOLD: u32 = CLAWBACK_NONCE_EXTEND_AMOUNT - DAY_IN_LEDGERS;

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

/// Event emitted when the RWA profile configuration is set or changed.
/// `approver` is `None` when the approval gate is disabled. `epoch` is the
/// approval epoch in force after the change.
#[contractevent]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RwaConfigChanged {
    pub auditor_id: u32,
    pub approver: Option<Address>,
    pub epoch: u32,
}

/// Emits an [`RwaConfigChanged`] event.
pub fn emit_rwa_config_changed(e: &Env, auditor_id: u32, approver: &Option<Address>, epoch: u32) {
    RwaConfigChanged { auditor_id, approver: approver.clone(), epoch }.publish(e);
}

/// Event emitted when the approval epoch advances without a change of the
/// [`RwaConfig`]. Authorizations issued for an earlier epoch are void.
#[contractevent]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ApprovalEpochBumped {
    pub epoch: u32,
}

/// Emits an [`ApprovalEpochBumped`] event.
pub fn emit_approval_epoch_bumped(e: &Env, epoch: u32) {
    ApprovalEpochBumped { epoch }.publish(e);
}

/// Event emitted when a public amount is moved between two confidential
/// accounts by a forced transfer. `nonce` is the source's clawback nonce
/// consumed by the call.
#[contractevent]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ForcedTransfer {
    #[topic]
    pub from: Address,
    #[topic]
    pub to: Address,
    pub amount: i128,
    pub nonce: u32,
}

/// Emits a [`ForcedTransfer`] event.
pub fn emit_forced_transfer(e: &Env, from: &Address, to: &Address, amount: i128, nonce: u32) {
    ForcedTransfer { from: from.clone(), to: to.clone(), amount, nonce }.publish(e);
}
