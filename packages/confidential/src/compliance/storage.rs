use soroban_sdk::{
    contracttype, panic_with_error, token, Address, Bytes, BytesN, Env, IntoVal, Val, Vec,
};
use stellar_contract_utils::crypto::grumpkin::Grumpkin;

use crate::{
    auditor::ConfidentialAuditorClient,
    compliance::{
        emit_clawback, emit_compliance_config_changed, emit_frozen, emit_recovered,
        emit_rwa_config_changed, emit_unfrozen, ComplianceError, PolicyClient,
        CLAWBACK_NONCE_EXTEND_AMOUNT, CLAWBACK_NONCE_TTL_THRESHOLD, FROZEN_EXTEND_AMOUNT,
        FROZEN_TTL_THRESHOLD,
    },
    storage::{
        address_to_field, append_amount, append_field, append_point, get_account,
        get_address_as_field_element, get_auditor, get_underlying_asset, revoke_spender,
        set_commitments, verify,
    },
    verifier::CircuitType,
    SpenderTransferPayload, TransferPayload, WithdrawPayload,
};

// ################## TYPES ##################

/// Compliance configuration written once at construction and rotatable under
/// admin auth thereafter. Stored as an instance storage entry.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ComplianceConfig {
    /// Optional external authorization policy (see
    /// [`crate::compliance::Policy`]). `None` disables the
    /// policy gate.
    pub policy: Option<Address>,
    /// When `true`, the gates additionally consult the underlying SAC's
    /// `authorized()` view. Requires the underlying token to be a Stellar
    /// Asset Contract — `authorized` is not part of SEP-41, and enabling
    /// this flag over a non-SAC underlying makes every gated operation trap
    /// (see [`check_sac`]).
    pub sac_passthrough: bool,
}

/// Envelope decoded from the `data: Bytes` argument of
/// [`crate::compliance::ConfidentialClawback::clawback`].
/// Carries the proof alone: the clawback circuit has no prover-supplied
/// public inputs, so there is no payload to accompany it.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ClawbackData {
    pub proof: Bytes,
}

/// Prover-supplied public inputs of
/// [`crate::compliance::ConfidentialRwa::recover`]
/// (`docs/compliance.md#recovery-circuit`). Every field except `c_transfer`
/// is emitted in the [`crate::compliance::Recovered`] event.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RecoverPayload {
    pub c_transfer: BytesN<64>,
    pub r_e_point: BytesN<64>,
    pub v_tilde: BytesN<32>,
    pub sigma: BytesN<32>,
    pub v_tilde_aud_r: BytesN<32>,
    pub r_tilde_aud_r: BytesN<32>,
}

/// Envelope decoded from the `data: Bytes` argument of
/// [`crate::compliance::ConfidentialRwa::recover`].
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RecoverData {
    pub payload: RecoverPayload,
    pub proof: Bytes,
}

/// RWA profile configuration (`docs/compliance.md#rwa-profile`). Stored as
/// an instance storage entry under its own key; [`ComplianceConfig`] is
/// unchanged.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RwaConfig {
    /// The only auditor id accounts of this token may be bound to. Set by
    /// the first [`set_rwa_config`] call and immutable afterwards.
    pub auditor_id: u32,
    /// Address whose authorization every value-moving operation
    /// additionally requires. `None` disables the approval gate.
    pub approver: Option<Address>,
}

/// Storage keys for the confidential token compliance extension.
#[contracttype]
pub enum ComplianceStorageKey {
    /// Singleton [`ComplianceConfig`]. Instance storage.
    Config,
    /// Per-account frozen flag. Persistent storage; only set when an account
    /// is frozen and removed on unfreeze.
    Frozen(Address),
    /// Number of seizures and recoveries executed against the account.
    /// Persistent storage; absent until the first.
    ClawbackNonce(Address),
    /// Singleton [`RwaConfig`]. Instance storage.
    RwaConfig,
}

// ################## QUERY STATE ##################

/// Returns the active [`ComplianceConfig`], or `None` when compliance has not
/// been configured for this deployment.
///
/// # Arguments
///
/// * `e` - Access to the Soroban environment.
pub fn compliance_config(e: &Env) -> Option<ComplianceConfig> {
    e.storage().instance().get(&ComplianceStorageKey::Config)
}

/// Returns whether `account` is currently frozen.
///
/// Returns `false` when compliance is not configured, ignoring any stale
/// `Frozen` entry left over from a prior configuration.
///
/// # Arguments
///
/// * `e` - Access to the Soroban environment.
/// * `account` - The address to query.
pub fn is_frozen(e: &Env, account: &Address) -> bool {
    if compliance_config(e).is_none() {
        return false;
    }
    let key = ComplianceStorageKey::Frozen(account.clone());
    if e.storage().persistent().has(&key) {
        e.storage().persistent().extend_ttl(&key, FROZEN_TTL_THRESHOLD, FROZEN_EXTEND_AMOUNT);
        true
    } else {
        false
    }
}

/// Returns the number of seizures and recoveries executed against `account`,
/// or `0` when there has been none. The value is a public input of the next
/// clawback proof (`docs/compliance.md#circuit`).
///
/// # Arguments
///
/// * `e` - Access to the Soroban environment.
/// * `account` - The address to query.
pub fn clawback_nonce(e: &Env, account: &Address) -> u32 {
    let key = ComplianceStorageKey::ClawbackNonce(account.clone());
    if let Some(nonce) = e.storage().persistent().get::<_, u32>(&key) {
        e.storage().persistent().extend_ttl(
            &key,
            CLAWBACK_NONCE_TTL_THRESHOLD,
            CLAWBACK_NONCE_EXTEND_AMOUNT,
        );
        nonce
    } else {
        0
    }
}

/// Returns the active [`RwaConfig`], or `None` when the RWA profile has not
/// been configured for this deployment.
///
/// # Arguments
///
/// * `e` - Access to the Soroban environment.
pub fn rwa_config(e: &Env) -> Option<RwaConfig> {
    e.storage().instance().get(&ComplianceStorageKey::RwaConfig)
}

// ################## CHANGE STATE ##################

/// Writes `config` into instance storage, overwriting any prior value. The
/// function does not guard against re-initialization: it is the single setter
/// used both for the initial deployment-time write and for subsequent
/// rotations.
///
/// # Arguments
///
/// * `e` - Access to the Soroban environment.
/// * `config` - The new [`ComplianceConfig`].
///
/// # Events
///
/// * topics - `["compliance_config_changed"]`
/// * data - `[policy: Option<Address>, sac_passthrough: bool]` (`policy` is
///   left out of the data when it is `None`)
///
/// # Security Warning
///
/// **IMPORTANT**: This function bypasses authorization checks and should
/// only be used:
/// - During contract initialization/construction
/// - In admin functions that implement their own authorization logic
///
/// Using this function in public-facing methods may create significant
/// security risks as it could allow unauthorized modifications.
pub fn set_compliance_config(e: &Env, config: &ComplianceConfig) {
    e.storage().instance().set(&ComplianceStorageKey::Config, config);
    emit_compliance_config_changed(e, &config.policy, config.sac_passthrough);
}

/// Marks `account` as frozen.
///
/// # Arguments
///
/// * `e` - Access to the Soroban environment.
/// * `account` - The address to freeze.
///
/// # Errors
///
/// * [`ComplianceError::NotConfigured`] - When [`compliance_config`] returns
///   `None`.
///
/// # Events
///
/// * topics - `["frozen", account: Address]`
/// * data - `[]`
///
/// # Security Warning
///
/// **IMPORTANT**: This function bypasses authorization checks and should
/// only be used:
/// - During contract initialization/construction
/// - In admin functions that implement their own authorization logic
///
/// Using this function in public-facing methods may create significant
/// security risks as it could allow unauthorized modifications.
pub fn freeze(e: &Env, account: &Address) {
    if compliance_config(e).is_none() {
        panic_with_error!(e, ComplianceError::NotConfigured);
    }
    e.storage().persistent().set(&ComplianceStorageKey::Frozen(account.clone()), &true);
    emit_frozen(e, account);
}

/// Clears the frozen flag on `account`.
///
/// # Arguments
///
/// * `e` - Access to the Soroban environment.
/// * `account` - The address to unfreeze.
///
/// # Errors
///
/// * [`ComplianceError::NotConfigured`] - When [`compliance_config`] returns
///   `None`.
///
/// # Events
///
/// * topics - `["unfrozen", account: Address]`
/// * data - `[]`
///
/// # Security Warning
///
/// **IMPORTANT**: This function bypasses authorization checks and should
/// only be used:
/// - During contract initialization/construction
/// - In admin functions that implement their own authorization logic
///
/// Using this function in public-facing methods may create significant
/// security risks as it could allow unauthorized modifications.
pub fn unfreeze(e: &Env, account: &Address) {
    if compliance_config(e).is_none() {
        panic_with_error!(e, ComplianceError::NotConfigured);
    }
    e.storage().persistent().remove(&ComplianceStorageKey::Frozen(account.clone()));
    emit_unfrozen(e, account);
}

/// Reduces `account`'s confidential claim by `amount` and settles the
/// corresponding underlying according to `destination`.
///
/// The proof establishes what the contract cannot check against committed
/// balances: that the prover holds the secret key of the auditor `account` is
/// bound to (CB1), that it knows the Pedersen openings of `C_spend` and
/// `C_receive` (CB2, CB3), and that `amount <= v_spend + v_receive` (CB4).
/// The openings alone are not a secret — registration, deposits, and merges
/// leave them public — so the witness is producible only by the auditor.
///
/// The post-verification update is the [`crate::storage::merge`]
/// rule plus a public debit — `C_spend <- C_spend + C_receive - amount * G`
/// and `C_receive <- O` — with no fresh randomness. The new opening is
/// `(v_s + v_r - amount, r_s + r_r)`, which both the owner and the auditor
/// recompute, so the seized account stays spendable.
///
/// The account's clawback nonce is a public input and advances on every
/// seizure and every [`recover`], so a proof executes at most once and never
/// after a recovery, even if the commitments are later restored to the values
/// it was built against (`docs/compliance.md#anti-replay-and-the-freeze`).
///
/// # Arguments
///
/// * `e` - Access to the Soroban environment.
/// * `account` - The confidential account being seized from.
/// * `amount` - The strictly positive seize amount.
/// * `destination` - Where the underlying settles, or `None`.
/// * `proof` - The raw UltraHonk proof bytes, from the decoded
///   [`ClawbackData`].
///
/// # Errors
///
/// * [`ComplianceError::AccountNotFrozen`] - When `account` is not frozen.
/// * [`ComplianceError::InvalidClawbackAmount`] - When `amount <= 0`.
/// * [`ComplianceError::InvalidClawbackDestination`] - When `destination` is
///   `Some` naming this contract's own address.
/// * refer to [`crate::storage::get_account`] errors.
/// * refer to [`crate::auditor::ConfidentialAuditor::get_key`] errors.
/// * [`crate::ConfidentialTokenError::NonCanonicalEncoding`] - When a stored
///   commitment or auditor key coordinate is not a canonical `Bn254Fr` value.
/// * [`crate::ConfidentialTokenError::InvalidProof`] - When the proof fails
///   verification.
///
/// # Events
///
/// * topics - `["clawback", account: Address]`
/// * data - `[amount: i128, destination: Option<Address>]` (`destination` is
///   left out of the data when it is `None`)
///
/// # Notes
///
/// Under `None` no underlying moves: the pool is left over-collateralized by
/// `amount`, extractable only by the underlying's own issuer through a SAC
/// `clawback` against this contract's address. That extraction must follow
/// this call.
///
/// Under `Some(d)`, exactly `amount` is transferred to `d` in this invocation,
/// so the pool and the sum of claims move together. `d` passes no compliance
/// gate (`docs/compliance.md#contract-flow`).
///
/// # Security Warning
///
/// **IMPORTANT**: This function bypasses authorization checks. The trait entry
/// point [`crate::compliance::ConfidentialClawback::clawback`]
/// is responsible for authorizing `operator`.
pub fn clawback(
    e: &Env,
    account: &Address,
    amount: i128,
    destination: &Option<Address>,
    proof: &Bytes,
) {
    if !is_frozen(e, account) {
        panic_with_error!(e, ComplianceError::AccountNotFrozen);
    }
    if amount <= 0 {
        panic_with_error!(e, ComplianceError::InvalidClawbackAmount);
    }
    if destination.as_ref() == Some(&e.current_contract_address()) {
        panic_with_error!(e, ComplianceError::InvalidClawbackDestination);
    }

    let data = get_account(e, account);
    let auditor = ConfidentialAuditorClient::new(e, &get_auditor(e));
    let k_aud = auditor.get_key(&data.auditor_id);
    let addr_f = get_address_as_field_element(e);
    // Zero is an unambiguous `None` sentinel: `address_to_field` is a
    // Poseidon2 output.
    let dest_f = match destination {
        Some(d) => address_to_field(e, d),
        None => BytesN::from_array(e, &[0u8; 32]),
    };
    let nonce = clawback_nonce(e, account);

    // PI order (docs/compliance.md#circuit):
    //   C_spend, C_receive, K_aud, alpha, addr_f, acct_f, dest_f, nonce
    //
    // `addr_f`, `acct_f`, `dest_f` and `nonce` are referenced by no
    // constraint; their membership in the public-input set binds the proof to
    // one contract, one account, one settlement destination, and one
    // execution, on the `register` precedent.
    let mut pi = Bytes::new(e);
    append_point(&mut pi, &data.spendable_commitment);
    append_point(&mut pi, &data.receiving_commitment);
    append_point(&mut pi, &k_aud);
    append_amount(&mut pi, e, amount);
    append_field(&mut pi, &addr_f);
    append_field(&mut pi, &address_to_field(e, account));
    append_field(&mut pi, &dest_f);
    append_amount(&mut pi, e, i128::from(nonce));

    verify(e, CircuitType::Clawback, &pi, proof);

    let seized = Grumpkin::mul(e, &Grumpkin::generator(e), amount as u128);
    let c_spend_new = Grumpkin::sub(
        e,
        &Grumpkin::add(e, &data.spendable_commitment, &data.receiving_commitment),
        &seized,
    );
    set_commitments(e, account, &c_spend_new, &Grumpkin::identity(e));
    e.storage()
        .persistent()
        .set(&ComplianceStorageKey::ClawbackNonce(account.clone()), &(nonce + 1));

    if let Some(d) = destination {
        let token = token::TokenClient::new(e, &get_underlying_asset(e));
        token.transfer(&e.current_contract_address(), d, &amount);
    }

    emit_clawback(e, account, amount, destination);
}

/// Folds the `(account, spender)` delegation's escrowed allowance back into
/// `account`'s spendable balance and deletes the delegation, without the
/// owner's participation.
///
/// Escrowed value is invisible to [`clawback`], which sees only `C_spend` and
/// `C_receive`; this moves it into reach. The fold itself is the same
/// proofless primitive the owner's
/// [`revoke_spender`](crate::ConfidentialToken::revoke_spender)
/// uses — only the authorization gate differs.
///
/// # Arguments
///
/// * `e` - Access to the Soroban environment.
/// * `account` - The delegating owner.
/// * `spender` - The delegated spender.
///
/// # Errors
///
/// * [`ComplianceError::AccountNotFrozen`] - When `account` is not frozen.
/// * refer to [`crate::storage::revoke_spender`] errors.
///
/// # Events
///
/// * topics - `["revoke_spender", account: Address, spender: Address]`
/// * data - `[a_tilde: BytesN<32>, allowance_salt: BytesN<32>]`
///
/// # Security Warning
///
/// **IMPORTANT**: This function bypasses authorization checks. The trait entry
/// point
/// [`crate::compliance::ConfidentialClawback::force_revoke_spender`]
/// is responsible for authorizing `operator`.
pub fn force_revoke_spender(e: &Env, account: &Address, spender: &Address) {
    if !is_frozen(e, account) {
        panic_with_error!(e, ComplianceError::AccountNotFrozen);
    }
    revoke_spender(e, account, spender);
}

/// Writes the RWA profile configuration.
///
/// The first call fixes the designated auditor; later calls may only change
/// the approver.
///
/// # Arguments
///
/// * `e` - Access to the Soroban environment.
/// * `auditor_id` - The only auditor id accounts of this token may be bound to.
/// * `approver` - The address whose authorization every value-moving operation
///   additionally requires, or `None` to disable the approval gate.
///
/// # Errors
///
/// * [`ComplianceError::NotConfigured`] - When [`compliance_config`] returns
///   `None`.
/// * [`ComplianceError::EligibilityRequired`] - When the compliance
///   configuration has neither a policy nor SAC passthrough.
/// * [`ComplianceError::AuditorAlreadySet`] - When an [`RwaConfig`] exists and
///   `auditor_id` differs from the stored one.
/// * [`ComplianceError::InvalidApprover`] - When `approver` is this contract's
///   own address.
/// * refer to [`crate::auditor::ConfidentialAuditor::get_key`] errors (first
///   call only).
///
/// # Events
///
/// * topics - `["rwa_config_changed"]`
/// * data - `[auditor_id: u32, approver: Option<Address>]` (`approver` is left
///   out of the data when it is `None`)
///
/// # Security Warning
///
/// **IMPORTANT**: This function bypasses authorization checks and should
/// only be used:
/// - During contract initialization/construction
/// - In admin functions that implement their own authorization logic
///
/// Using this function in public-facing methods may create significant
/// security risks as it could allow unauthorized modifications.
pub fn set_rwa_config(e: &Env, auditor_id: u32, approver: &Option<Address>) {
    let Some(config) = compliance_config(e) else {
        panic_with_error!(e, ComplianceError::NotConfigured);
    };
    if !has_eligibility_gate(&config) {
        panic_with_error!(e, ComplianceError::EligibilityRequired);
    }
    match rwa_config(e) {
        Some(current) => {
            if current.auditor_id != auditor_id {
                panic_with_error!(e, ComplianceError::AuditorAlreadySet);
            }
        }
        // The designated auditor must resolve to a registered key.
        None => {
            ConfidentialAuditorClient::new(e, &get_auditor(e)).get_key(&auditor_id);
        }
    }
    if approver.as_ref() == Some(&e.current_contract_address()) {
        panic_with_error!(e, ComplianceError::InvalidApprover);
    }

    let rwa = RwaConfig { auditor_id, approver: approver.clone() };
    e.storage().instance().set(&ComplianceStorageKey::RwaConfig, &rwa);
    emit_rwa_config_changed(e, auditor_id, approver);
}

/// Moves the whole committed balance of the frozen `from` to `to` without
/// revealing it, resets `from` to the identity on both sides, and advances
/// `from`'s clawback nonce.
///
/// The nonce advance voids every clawback proof built against `from` before
/// the recovery, which a public deposit could otherwise bring back into
/// validity (`docs/compliance.md#anti-replay-and-the-freeze`).
///
/// `to` is credited with `C_transfer`, which the proof shows holds `from`'s
/// committed total under a blinding `to` derives as for an incoming transfer
/// (`docs/compliance.md#recovery`).
///
/// # Arguments
///
/// * `e` - Access to the Soroban environment.
/// * `from` - The frozen account whose balance moves.
/// * `to` - The registered account credited with it.
/// * `payload` - The decoded [`RecoverPayload`].
/// * `proof` - The raw UltraHonk proof bytes.
///
/// # Errors
///
/// * [`ComplianceError::RecoveryRequiresApprover`] - When the [`RwaConfig`] has
///   no approver.
/// * [`ComplianceError::AccountNotFrozen`] - When `from` is not frozen.
/// * refer to [`rwa_context`] errors.
/// * refer to [`gate_account`] errors.
/// * refer to [`require_approval`] errors.
/// * refer to [`crate::storage::get_account`] errors.
/// * refer to [`crate::auditor::ConfidentialAuditor::get_key`] errors.
/// * [`crate::ConfidentialTokenError::NonCanonicalEncoding`] - When a point
///   coordinate or field is not a canonical `Bn254Fr` value.
/// * [`crate::ConfidentialTokenError::InvalidProof`] - When the proof fails
///   verification.
///
/// # Events
///
/// * topics - `["recovered", from: Address, to: Address]`
/// * data - `[r_e_point, v_tilde, sigma, v_tilde_aud_r, r_tilde_aud_r]`
///
/// # Security Warning
///
/// **IMPORTANT**: This function bypasses the operator's authorization. The
/// trait entry point [`crate::compliance::ConfidentialRwa::recover`] is
/// responsible for authorizing `operator`.
pub fn recover(e: &Env, from: &Address, to: &Address, payload: &RecoverPayload, proof: &Bytes) {
    let (config, rwa) = rwa_context(e);
    if rwa.approver.is_none() {
        panic_with_error!(e, ComplianceError::RecoveryRequiresApprover);
    }
    if !is_frozen(e, from) {
        panic_with_error!(e, ComplianceError::AccountNotFrozen);
    }
    // Also rejects `to == from`, which is frozen.
    gate_account(e, to, &config);
    require_approval(e, &rwa, &[from, to], recover_approval_args(e, from, to, payload));

    let source = get_account(e, from);
    let recipient = get_account(e, to);
    let auditor = ConfidentialAuditorClient::new(e, &get_auditor(e));
    let k_aud_r = auditor.get_key(&recipient.auditor_id);
    let addr_f = get_address_as_field_element(e);

    // PI order (docs/compliance.md#recovery-circuit):
    //   C_spend, C_receive, PVK_B, addr_f, K_aud_r, C_transfer, R_e, v_tilde,
    //   sigma, v_tilde_aud_r, r_tilde_aud_r
    let mut pi = Bytes::new(e);
    append_point(&mut pi, &source.spendable_commitment);
    append_point(&mut pi, &source.receiving_commitment);
    append_point(&mut pi, &recipient.viewing_public_key);
    append_field(&mut pi, &addr_f);
    append_point(&mut pi, &k_aud_r);
    append_point(&mut pi, &payload.c_transfer);
    append_point(&mut pi, &payload.r_e_point);
    append_field(&mut pi, &payload.v_tilde);
    append_field(&mut pi, &payload.sigma);
    append_field(&mut pi, &payload.v_tilde_aud_r);
    append_field(&mut pi, &payload.r_tilde_aud_r);

    verify(e, CircuitType::Recover, &pi, proof);

    let identity = Grumpkin::identity(e);
    set_commitments(e, from, &identity, &identity);
    e.storage()
        .persistent()
        .set(&ComplianceStorageKey::ClawbackNonce(from.clone()), &(clawback_nonce(e, from) + 1));
    set_commitments(
        e,
        to,
        &recipient.spendable_commitment,
        &Grumpkin::add(e, &recipient.receiving_commitment, &payload.c_transfer),
    );

    emit_recovered(e, from, to, payload);
}

// ################## LOW-LEVEL HELPERS ##################

/// Returns both configurations of an RWA deployment, reverting unless the
/// deployment is fully configured. Every [`crate::compliance::RwaHooks`]
/// callback starts here, so an unconfigured deployment rejects every
/// operation instead of passing it ungated.
///
/// # Arguments
///
/// * `e` - Access to the Soroban environment.
///
/// # Errors
///
/// * [`ComplianceError::RwaNotConfigured`] - When [`rwa_config`] returns
///   `None`.
/// * [`ComplianceError::NotConfigured`] - When [`compliance_config`] returns
///   `None`.
/// * [`ComplianceError::EligibilityRequired`] - When the compliance
///   configuration has neither a policy nor SAC passthrough.
pub fn rwa_context(e: &Env) -> (ComplianceConfig, RwaConfig) {
    let Some(rwa) = rwa_config(e) else {
        panic_with_error!(e, ComplianceError::RwaNotConfigured);
    };
    let Some(config) = compliance_config(e) else {
        panic_with_error!(e, ComplianceError::NotConfigured);
    };
    if !has_eligibility_gate(&config) {
        panic_with_error!(e, ComplianceError::EligibilityRequired);
    }
    (config, rwa)
}

/// Requires the approver's authorization for the current invocation with
/// `args`. A no-op when `rwa.approver` is `None`.
///
/// The host binds the authorization to this contract and to the entry point
/// being executed; `args` bind the parties and the operation's payload
/// (`docs/compliance.md#approval-gate`).
///
/// # Arguments
///
/// * `e` - Access to the Soroban environment.
/// * `rwa` - The active [`RwaConfig`].
/// * `parties` - Every address the operation names (`from`, `to`, and `spender`
///   for a delegated transfer).
/// * `args` - The approval arguments.
///
/// # Errors
///
/// * [`ComplianceError::ApproverIsParty`] - When the approver is one of
///   `parties`. A party cannot approve an operation it takes part in; this also
///   keeps one address from authorizing the same invocation twice.
pub fn require_approval(e: &Env, rwa: &RwaConfig, parties: &[&Address], args: Vec<Val>) {
    if let Some(approver) = &rwa.approver {
        if parties.contains(&approver) {
            panic_with_error!(e, ComplianceError::ApproverIsParty);
        }
        approver.require_auth_for_args(args);
    }
}

/// Approval arguments of a `deposit`: `(from, to, amount)`.
///
/// The tuple is a wire contract with the approver, which signs exactly
/// these values (`docs/compliance.md#approval-gate`).
///
/// # Arguments
///
/// * `e` - Access to the Soroban environment.
/// * `from` - The depositor.
/// * `to` - The credited confidential account.
/// * `amount` - The public amount.
pub fn deposit_approval_args(e: &Env, from: &Address, to: &Address, amount: i128) -> Vec<Val> {
    (from.clone(), to.clone(), amount).into_val(e)
}

/// Approval arguments of a `withdraw`: `(from, to, amount, payload)`.
/// See [`deposit_approval_args`].
///
/// # Arguments
///
/// * `e` - Access to the Soroban environment.
/// * `from` - The debited confidential account.
/// * `to` - The address receiving the underlying asset.
/// * `amount` - The public amount.
/// * `payload` - The decoded withdraw payload.
pub fn withdraw_approval_args(
    e: &Env,
    from: &Address,
    to: &Address,
    amount: i128,
    payload: &WithdrawPayload,
) -> Vec<Val> {
    (from.clone(), to.clone(), amount, payload.clone()).into_val(e)
}

/// Approval arguments of a `confidential_transfer`:
/// `(from, to, payload)`. See [`deposit_approval_args`].
///
/// # Arguments
///
/// * `e` - Access to the Soroban environment.
/// * `from` - The sender.
/// * `to` - The recipient.
/// * `payload` - The decoded transfer payload.
pub fn transfer_approval_args(
    e: &Env,
    from: &Address,
    to: &Address,
    payload: &TransferPayload,
) -> Vec<Val> {
    (from.clone(), to.clone(), payload.clone()).into_val(e)
}

/// Approval arguments of a `confidential_transfer_from`:
/// `(spender, from, to, payload)`. See [`deposit_approval_args`].
///
/// # Arguments
///
/// * `e` - Access to the Soroban environment.
/// * `spender` - The delegated spender.
/// * `from` - The owner whose allowance is spent.
/// * `to` - The recipient.
/// * `payload` - The decoded spender-transfer payload.
pub fn spender_transfer_approval_args(
    e: &Env,
    spender: &Address,
    from: &Address,
    to: &Address,
    payload: &SpenderTransferPayload,
) -> Vec<Val> {
    (spender.clone(), from.clone(), to.clone(), payload.clone()).into_val(e)
}

/// Approval arguments of a `recover`: `(from, to, payload)`. See
/// [`deposit_approval_args`].
///
/// # Arguments
///
/// * `e` - Access to the Soroban environment.
/// * `from` - The frozen account whose balance moves.
/// * `to` - The credited account.
/// * `payload` - The decoded recover payload.
pub fn recover_approval_args(
    e: &Env,
    from: &Address,
    to: &Address,
    payload: &RecoverPayload,
) -> Vec<Val> {
    (from.clone(), to.clone(), payload.clone()).into_val(e)
}

/// Returns whether `config` keeps an eligibility gate: an external policy or
/// the SAC `authorized()` passthrough.
fn has_eligibility_gate(config: &ComplianceConfig) -> bool {
    config.policy.is_some() || config.sac_passthrough
}

/// Asserts that `account` passes every configured compliance gate against the
/// given `config`: not frozen, authorized by the policy (when one is set),
/// and authorized by the SAC (when `sac_passthrough` is enabled).
///
/// # Arguments
///
/// * `e` - Access to the Soroban environment.
/// * `account` - The address to check.
/// * `config` - The active [`ComplianceConfig`].
///
/// # Errors
///
/// * [`ComplianceError::AccountFrozen`] - When `account` is frozen.
/// * refer to [`check_policy`] errors.
/// * refer to [`check_sac`] errors.
pub fn gate_account(e: &Env, account: &Address, config: &ComplianceConfig) {
    if is_frozen(e, account) {
        panic_with_error!(e, ComplianceError::AccountFrozen);
    }
    check_policy(e, account, config);
    check_sac(e, account, config);
}

/// Asserts that the configured external policy authorizes `account` for the
/// current token contract. A no-op when `config.policy` is `None`.
///
/// # Arguments
///
/// * `e` - Access to the Soroban environment.
/// * `account` - The address to check.
/// * `config` - The active [`ComplianceConfig`].
///
/// # Errors
///
/// * [`ComplianceError::NotAuthorizedByPolicy`] - When the configured policy
///   returns `false` for `account`.
pub fn check_policy(e: &Env, account: &Address, config: &ComplianceConfig) {
    if let Some(policy_addr) = &config.policy {
        let policy = PolicyClient::new(e, policy_addr);
        if !policy.is_authorized(account, &e.current_contract_address()) {
            panic_with_error!(e, ComplianceError::NotAuthorizedByPolicy);
        }
    }
}

/// Asserts that the underlying token's `authorized` view returns `true`
/// for `account`. A no-op when `config.sac_passthrough` is `false`.
///
/// The `authorized` view belongs to the Stellar Asset Contract admin
/// interface, not to generic SEP-41
/// (`docs/protocol/system-model.md#underlying-token-assumptions`). Enabling
/// `sac_passthrough` over a non-SAC underlying (e.g. a plain SEP-41 token)
/// makes this call — and with it every gated operation — trap on the
/// missing function.
///
/// # Arguments
///
/// * `e` - Access to the Soroban environment.
/// * `account` - The address to check.
/// * `config` - The active [`ComplianceConfig`].
///
/// # Errors
///
/// * [`ComplianceError::NotAuthorizedBySac`] - When the SAC's `authorized` view
///   returns `false` for `account`.
pub fn check_sac(e: &Env, account: &Address, config: &ComplianceConfig) {
    if config.sac_passthrough {
        let sac = token::StellarAssetClient::new(e, &get_underlying_asset(e));
        if !sac.authorized(account) {
            panic_with_error!(e, ComplianceError::NotAuthorizedBySac);
        }
    }
}
