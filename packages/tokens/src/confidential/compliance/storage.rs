use soroban_sdk::{contracttype, panic_with_error, token, Address, Bytes, BytesN, Env};
use stellar_contract_utils::crypto::grumpkin::Grumpkin;

use crate::confidential::{
    compliance::{
        emit_clawback, emit_compliance_config_changed, emit_frozen, emit_unfrozen, ComplianceError,
        PolicyClient, FROZEN_EXTEND_AMOUNT, FROZEN_TTL_THRESHOLD,
    },
    storage::{
        address_to_field, append_amount, append_field, append_point, get_account,
        get_address_as_field_element, get_underlying_asset, revoke_spender, set_commitments,
        verify,
    },
    verifier::CircuitType,
};

// ################## TYPES ##################

/// Compliance configuration written once at construction and rotatable under
/// admin auth thereafter. Stored as an instance storage entry.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ComplianceConfig {
    /// Optional external authorization policy (see
    /// [`crate::confidential::compliance::Policy`]). `None` disables the
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
/// [`crate::confidential::compliance::ConfidentialClawback::clawback`].
/// Carries the proof alone: the clawback circuit has no prover-supplied
/// public inputs, so there is no payload to accompany it.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ClawbackData {
    pub proof: Bytes,
}

/// Storage keys for the confidential token compliance extension.
#[contracttype]
pub enum ComplianceStorageKey {
    /// Singleton [`ComplianceConfig`]. Instance storage.
    Config,
    /// Per-account frozen flag. Persistent storage; only set when an account
    /// is frozen and removed on unfreeze.
    Frozen(Address),
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
/// * data - `[policy: Option<Address>, sac_passthrough: bool]`
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
/// balances: that the prover knows the Pedersen openings of `C_spend` and
/// `C_receive` (CB1, CB2), and that `amount <= v_spend + v_receive` (CB3).
/// The witness is producible by anyone holding the openings — the auditor, or
/// the owner — and not by the admin, which holds no blinding.
///
/// The post-verification update is the [`crate::confidential::storage::merge`]
/// rule plus a public debit — `C_spend <- C_spend + C_receive - amount * G`
/// and `C_receive <- O` — with no fresh randomness. The new opening is
/// `(v_s + v_r - amount, r_s + r_r)`, which both the owner and the auditor
/// recompute, so the seized account stays spendable.
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
/// * refer to [`crate::confidential::storage::get_account`] errors.
/// * [`crate::confidential::ConfidentialTokenError::NonCanonicalEncoding`] -
///   When a stored commitment coordinate is not a canonical `Bn254Fr` value.
/// * [`crate::confidential::ConfidentialTokenError::InvalidProof`] - When the
///   proof fails verification.
///
/// # Events
///
/// * topics - `["clawback", account: Address]`
/// * data - `[amount: i128, destination: Option<Address>]`
///
/// # Notes
///
/// Under `None` no underlying moves: the pool is left over-collateralized by
/// `amount`, extractable only by the underlying's own issuer through a SAC
/// `clawback` against this contract's address. That extraction must follow
/// this call.
///
/// Under `Some(d)`, exactly `amount` is transferred to `d` in this invocation,
/// so the pool and the sum of claims move together.
///
/// # Security Warning
///
/// **IMPORTANT**: This function bypasses authorization checks. The trait entry
/// point [`crate::confidential::compliance::ConfidentialClawback::clawback`]
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
    let addr_f = get_address_as_field_element(e);
    // Zero is an unambiguous `None` sentinel: `address_to_field` is a
    // Poseidon2 output.
    let dest_f = match destination {
        Some(d) => address_to_field(e, d),
        None => BytesN::from_array(e, &[0u8; 32]),
    };

    // PI order (COMPLIANCE §5.3):
    //   C_spend, C_receive, alpha, addr_f, acct_f, dest_f
    //
    // `addr_f`, `acct_f` and `dest_f` are referenced by no constraint; their
    // membership in the public-input set binds the proof to one contract, one
    // account, and one settlement destination, on the `register` precedent.
    let mut pi = Bytes::new(e);
    append_point(&mut pi, &data.spendable_commitment);
    append_point(&mut pi, &data.receiving_commitment);
    append_amount(&mut pi, e, amount);
    append_field(&mut pi, &addr_f);
    append_field(&mut pi, &address_to_field(e, account));
    append_field(&mut pi, &dest_f);

    verify(e, CircuitType::Clawback, &pi, proof);

    let seized = Grumpkin::mul(e, &Grumpkin::generator(e), amount as u128);
    let c_spend_new = Grumpkin::sub(
        e,
        &Grumpkin::add(e, &data.spendable_commitment, &data.receiving_commitment),
        &seized,
    );
    set_commitments(e, account, &c_spend_new, &Grumpkin::identity(e));

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
/// [`revoke_spender`](crate::confidential::ConfidentialToken::revoke_spender)
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
/// * refer to [`crate::confidential::storage::revoke_spender`] errors.
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
/// [`crate::confidential::compliance::ConfidentialClawback::force_revoke_spender`]
/// is responsible for authorizing `operator`.
pub fn force_revoke_spender(e: &Env, account: &Address, spender: &Address) {
    if !is_frozen(e, account) {
        panic_with_error!(e, ComplianceError::AccountNotFrozen);
    }
    revoke_spender(e, account, spender);
}

// ################## LOW-LEVEL HELPERS ##################

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
/// interface, not to generic SEP-41 (DESIGN §3.4). Enabling
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
