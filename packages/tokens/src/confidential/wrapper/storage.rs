use soroban_poseidon::poseidon2_hash;
use soroban_sdk::{
    contracttype, crypto::bn254::Bn254Fr, panic_with_error, token, xdr::FromXdr, Address, Bytes,
    BytesN, Env, TryFromVal, Val, U256,
};
use stellar_contract_utils::crypto::grumpkin::{Grumpkin, Point};

use crate::confidential::{
    auditor::ConfidentialAuditorClient,
    verifier::{CircuitType, ConfidentialVerifierClient},
    wrapper::{
        emit_auditor_set, emit_deposit, emit_merge, emit_operator_transfer, emit_register,
        emit_revoke_operator, emit_set_operator, emit_token_set, emit_transfer, emit_verifier_set,
        emit_withdraw, emit_wrap_set, WrapperError, ACCOUNT_EXTEND_AMOUNT, ACCOUNT_TTL_THRESHOLD,
        DELEGATION_EXTEND_AMOUNT, DELEGATION_TTL_THRESHOLD, DELTA_ADDR,
    },
};

/// Storage keys for the confidential token wrapper.
#[contracttype]
pub enum WrapperStorageKey {
    /// SEP-41 token whose balances back the wrapper. Instance storage.
    Token,
    /// Confidential verifier contract used for `verify_proof`. Instance
    /// storage.
    Verifier,
    /// Confidential auditor contract used for `get_key`. Instance storage.
    Auditor,
    /// `wrap = address_to_field(env.current_contract_address())` (DESIGN §2.7,
    /// §3.5), bound into every owner-initiated circuit's `vk` derivation.
    /// Stored as a canonical 32-byte big-endian `Bn254Fr` representative.
    /// Instance storage.
    Wrap,
    /// Per-account `ConfidentialAccount` entry, keyed by the owner address.
    /// Persistent storage.
    Account(Address),
    /// Per-`(owner, operator)` `OperatorDelegation` entry. Persistent storage.
    /// Persists until explicitly revoked even when `expiration_ledger` has
    /// passed.
    Delegation(Address, Address),
}

/// Length of the Stellar strkey ASCII encoding for both Account (G…) and
/// Contract (C…) addresses: 1 version byte + 32 payload bytes + 2 CRC bytes
/// = 35 bytes, base32-encoded to 56 ASCII characters.
const STRKEY_LEN: usize = 56;

/// 28 bytes per limb in the `address_to_field` decomposition.
const STRKEY_LIMB_LEN: usize = 28;

// ################## ACCOUNT STATE ##################

/// On-chain confidential account record.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ConfidentialAccount {
    /// `Y = sk · H`, the Grumpkin spending public key.
    pub spending_key: Point,
    /// `PVK = vk · H`, the Grumpkin viewing public key.
    pub viewing_public_key: Point,
    /// Spendable balance commitment `C_spend`.
    pub spendable_balance: Point,
    /// Receiving balance commitment `C_receive`.
    pub receiving_balance: Point,
    /// Index of the auditor key in the auditor registry.
    pub auditor_id: u32,
}

/// On-chain operator delegation record.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct OperatorDelegation {
    /// Allowance commitment `C_a = Com(v_a, r_a)`.
    pub allowance_commitment: Point,
    /// Poseidon-encrypted allowance scalar `ã`.
    pub encrypted_allowance: BytesN<32>,
    /// ECDH escrow of `dvk_i` under the operator's spending key.
    pub escrowed_dvk: Point,
    /// Per-delegation salt `σ_a`.
    pub allowance_salt: BytesN<32>,
    /// Ledger sequence after which the delegation no longer authorizes
    /// spending.
    pub expiration_ledger: u32,
}

// ################## DATA PAYLOADS ##################
//
// Each `*Payload` struct mirrors DESIGN §11.1 minus the proof field. The
// matching `*Data` envelope is what the trait-entry-point `data: Bytes`
// argument decodes into: `{ payload, proof }`. Splitting at the storage
// boundary lets storage fns take `(payload, proof)` as two distinct args
// while keeping a single `data: Bytes` wire-format parameter at the trait
// surface.

/// Payload for [`super::ConfidentialTokenWrapper::register`].
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RegisterPayload {
    pub y: Point,
    pub pvk: Point,
}

/// Envelope decoded from the `data: Bytes` argument of
/// [`super::ConfidentialTokenWrapper::register`].
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RegisterData {
    pub payload: RegisterPayload,
    pub proof: Bytes,
}

/// Payload for [`super::ConfidentialTokenWrapper::withdraw`].
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct WithdrawPayload {
    pub c_spend_new: Point,
    pub b_tilde: BytesN<32>,
    pub r_e: Point,
    pub sigma: BytesN<32>,
    pub b_aud_s: BytesN<32>,
}

/// Envelope decoded from the `data: Bytes` argument of
/// [`super::ConfidentialTokenWrapper::withdraw`].
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct WithdrawData {
    pub payload: WithdrawPayload,
    pub proof: Bytes,
}

/// Payload for [`super::ConfidentialTokenWrapper::confidential_transfer`].
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TransferPayload {
    pub c_spend_new: Point,
    pub c_tx: Point,
    pub r_e: Point,
    pub v_tilde: BytesN<32>,
    pub b_tilde: BytesN<32>,
    pub sigma: BytesN<32>,
    pub v_aud_r: BytesN<32>,
    pub r_aud_r: BytesN<32>,
    pub v_aud_s: BytesN<32>,
    pub b_aud_s: BytesN<32>,
}

/// Envelope decoded from the `data: Bytes` argument of
/// [`super::ConfidentialTokenWrapper::confidential_transfer`].
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TransferData {
    pub payload: TransferPayload,
    pub proof: Bytes,
}

/// Payload for
/// [`super::ConfidentialTokenWrapper::confidential_transfer_from`].
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct OperatorTransferPayload {
    pub c_a_new: Point,
    pub c_tx: Point,
    pub r_e: Point,
    pub v_tilde: BytesN<32>,
    pub a_tilde_new: BytesN<32>,
    pub sigma_a_new: BytesN<32>,
    pub v_aud_r: BytesN<32>,
    pub r_aud_r: BytesN<32>,
    pub v_aud_s: BytesN<32>,
    pub a_aud_s: BytesN<32>,
}

/// Envelope decoded from the `data: Bytes` argument of
/// [`super::ConfidentialTokenWrapper::confidential_transfer_from`].
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct OperatorTransferData {
    pub payload: OperatorTransferPayload,
    pub proof: Bytes,
}

/// Payload for [`super::ConfidentialTokenWrapper::set_operator`].
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SetOperatorPayload {
    pub c_spend_new: Point,
    pub c_a: Point,
    pub escrowed_dvk: Point,
    pub b_tilde: BytesN<32>,
    pub a_tilde: BytesN<32>,
    pub r_e: Point,
    pub sigma: BytesN<32>,
    pub sigma_a: BytesN<32>,
    pub v_aud_s: BytesN<32>,
    pub b_aud_s: BytesN<32>,
}

/// Envelope decoded from the `data: Bytes` argument of
/// [`super::ConfidentialTokenWrapper::set_operator`].
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SetOperatorData {
    pub payload: SetOperatorPayload,
    pub proof: Bytes,
}

/// Payload for [`super::ConfidentialTokenWrapper::revoke_operator`].
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RevokeOperatorPayload {
    pub c_spend_new: Point,
    pub b_tilde: BytesN<32>,
    pub r_e: Point,
    pub sigma: BytesN<32>,
    pub v_aud_s: BytesN<32>,
    pub b_aud_s: BytesN<32>,
}

/// Envelope decoded from the `data: Bytes` argument of
/// [`super::ConfidentialTokenWrapper::revoke_operator`].
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RevokeOperatorData {
    pub payload: RevokeOperatorPayload,
    pub proof: Bytes,
}

// ################## QUERY STATE ##################

/// Returns the SEP-41 token address bound at construction.
///
/// # Arguments
///
/// * `e` - Access to the Soroban environment.
///
/// # Errors
///
/// * [`WrapperError::TokenNotSet`] - When the wrapper has not been constructed.
pub fn get_token(e: &Env) -> Address {
    e.storage()
        .instance()
        .get(&WrapperStorageKey::Token)
        .unwrap_or_else(|| panic_with_error!(e, WrapperError::TokenNotSet))
}

/// Returns the confidential verifier contract address bound at construction.
///
/// # Arguments
///
/// * `e` - Access to the Soroban environment.
///
/// # Errors
///
/// * [`WrapperError::VerifierNotSet`] - When the wrapper has not been
///   constructed.
pub fn get_verifier(e: &Env) -> Address {
    e.storage()
        .instance()
        .get(&WrapperStorageKey::Verifier)
        .unwrap_or_else(|| panic_with_error!(e, WrapperError::VerifierNotSet))
}

/// Returns the auditor registry contract address bound at construction.
///
/// # Arguments
///
/// * `e` - Access to the Soroban environment.
///
/// # Errors
///
/// * [`WrapperError::AuditorNotSet`] - When the wrapper has not been
///   constructed.
pub fn get_auditor(e: &Env) -> Address {
    e.storage()
        .instance()
        .get(&WrapperStorageKey::Auditor)
        .unwrap_or_else(|| panic_with_error!(e, WrapperError::AuditorNotSet))
}

/// Returns the wrapper's compressed `wrap` Field, the
/// `address_to_field(env.current_contract_address())` value computed once at
/// construction.
///
/// # Arguments
///
/// * `e` - Access to the Soroban environment.
///
/// # Errors
///
/// * [`WrapperError::WrapNotSet`] - When the wrapper has not been constructed.
pub fn get_wrap(e: &Env) -> BytesN<32> {
    e.storage()
        .instance()
        .get(&WrapperStorageKey::Wrap)
        .unwrap_or_else(|| panic_with_error!(e, WrapperError::WrapNotSet))
}

/// Returns the [`ConfidentialAccount`] stored under `account`.
///
/// # Arguments
///
/// * `e` - Access to the Soroban environment.
/// * `account` - The owner address.
///
/// # Errors
///
/// * [`WrapperError::AccountNotRegistered`] - When no account is stored under
///   `account`.
pub fn get_account(e: &Env, account: &Address) -> ConfidentialAccount {
    get_persistent_entry::<ConfidentialAccount>(e, &WrapperStorageKey::Account(account.clone()))
        .unwrap_or_else(|| panic_with_error!(e, WrapperError::AccountNotRegistered))
}

/// Returns the [`OperatorDelegation`] stored under `(owner, operator)`.
///
/// # Arguments
///
/// * `e` - Access to the Soroban environment.
/// * `owner` - The delegating account.
/// * `operator` - The delegated operator.
///
/// # Errors
///
/// * [`WrapperError::DelegationNotFound`] - When no delegation exists for the
///   pair.
pub fn get_delegation(e: &Env, owner: &Address, operator: &Address) -> OperatorDelegation {
    get_persistent_entry::<OperatorDelegation>(
        e,
        &WrapperStorageKey::Delegation(owner.clone(), operator.clone()),
    )
    .unwrap_or_else(|| panic_with_error!(e, WrapperError::DelegationNotFound))
}

/// Returns `true` iff a delegation exists for `(owner, operator)` and the
/// current ledger sequence does not exceed its `expiration_ledger`.
///
/// Returns `false` for missing entries and for expired-but-not-revoked
/// entries.
///
/// # Arguments
///
/// * `e` - Access to the Soroban environment.
/// * `owner` - The delegating account.
/// * `operator` - The delegated operator.
pub fn is_operator(e: &Env, owner: &Address, operator: &Address) -> bool {
    match get_persistent_entry::<OperatorDelegation>(
        e,
        &WrapperStorageKey::Delegation(owner.clone(), operator.clone()),
    ) {
        Some(d) => e.ledger().sequence() <= d.expiration_ledger,
        None => false,
    }
}

/// Returns whether an account is registered, without panicking.
///
/// # Arguments
///
/// * `e` - Access to the Soroban environment.
/// * `account` - The owner address.
pub fn account_exists(e: &Env, account: &Address) -> bool {
    e.storage().persistent().has(&WrapperStorageKey::Account(account.clone()))
}

/// Returns whether a delegation entry exists for `(owner, operator)`,
/// without applying the expiry filter.
///
/// # Arguments
///
/// * `e` - Access to the Soroban environment.
/// * `owner` - The delegating account.
/// * `operator` - The delegated operator.
pub fn delegation_exists(e: &Env, owner: &Address, operator: &Address) -> bool {
    e.storage().persistent().has(&WrapperStorageKey::Delegation(owner.clone(), operator.clone()))
}

// ################## CHANGE STATE ##################

/// Registers a confidential account under `account`.
///
/// # Arguments
///
/// * `e` - Access to the Soroban environment.
/// * `account` - The owner address being registered.
/// * `auditor_id` - The auditor key index this account commits to.
/// * `payload` - The decoded [`RegisterPayload`].
/// * `proof` - The raw UltraHonk proof bytes.
///
/// # Errors
///
/// * [`WrapperError::AccountAlreadyRegistered`] - When `account` is already
///   registered.
/// * [`WrapperError::InvalidProof`] - When the proof fails verification.
/// * refer to [`crate::confidential::auditor::ConfidentialAuditor::get_key`]
///   errors.
///
/// # Events
///
/// * topics - `["register", account: Address]`
/// * data - `[auditor_id: u32]`
///
/// # Security Warning
///
/// **IMPORTANT**: This function bypasses authorization checks. Use only from
/// trait-level entry points that have already called `account.require_auth()`,
/// or from admin paths with their own gating.
pub fn register_no_auth(
    e: &Env,
    account: &Address,
    auditor_id: u32,
    payload: &RegisterPayload,
    proof: &Bytes,
) {
    let auditor = ConfidentialAuditorClient::new(e, &get_auditor(e));
    let _k_aud = auditor.get_key(&auditor_id);
    let wrap = get_wrap(e);

    // PI order (DESIGN §7.2): Y, PVK, wrap.
    let mut pi = Bytes::new(e);
    append_point(&mut pi, &payload.y);
    append_point(&mut pi, &payload.pvk);
    append_field(&mut pi, &wrap);

    verify(e, CircuitType::Register, &pi, proof);

    let key = WrapperStorageKey::Account(account.clone());
    if e.storage().persistent().has(&key) {
        panic_with_error!(e, WrapperError::AccountAlreadyRegistered);
    }
    e.storage().persistent().set(
        &key,
        &ConfidentialAccount {
            spending_key: payload.y.clone(),
            viewing_public_key: payload.pvk.clone(),
            spendable_balance: Grumpkin::identity(e),
            receiving_balance: Grumpkin::identity(e),
            auditor_id,
        },
    );
    emit_register(e, account, auditor_id);
}

/// Deposits `amount` units of the underlying SEP-41 token from `from` into
/// `to`'s confidential receiving balance.
///
/// No proof is required. The deposit commitment is `a · G` with zero
/// blinding, added homomorphically to `to.receiving_balance`. The depositor
/// `from` does not need a registered confidential account; only `to` does.
///
/// # Arguments
///
/// * `e` - Access to the Soroban environment.
/// * `from` - The depositor (SEP-41 holder).
/// * `to` - The confidential recipient.
/// * `amount` - The non-negative deposit amount.
///
/// # Errors
///
/// * [`WrapperError::NegativeAmount`] - When `amount < 0`.
/// * [`WrapperError::AccountNotRegistered`] - When `to` is not a registered
///   confidential account.
///
/// # Events
///
/// * topics - `["deposit", from: Address, to: Address]`
/// * data - `[amount: i128]`
///
/// # Notes
///
/// This function credits `amount · G` to `to`'s confidential receiving
/// balance immediately after invoking `transfer` on the underlying token,
/// without re-measuring the wrapper's balance. The underlying token MUST
/// therefore have exact-transfer semantics (no fee-on-transfer, no
/// rebasing). See the [module-level constraint](super) for the list of
/// supported token implementations.
///
/// # Security Warning
///
/// **IMPORTANT**: This function bypasses authorization checks. The trait
/// entry point is responsible for calling `from.require_auth()`.
pub fn deposit_no_auth(e: &Env, from: &Address, to: &Address, amount: i128) {
    if amount < 0 {
        panic_with_error!(e, WrapperError::NegativeAmount);
    }
    if !account_exists(e, to) {
        panic_with_error!(e, WrapperError::AccountNotRegistered);
    }

    let token = token::TokenClient::new(e, &get_token(e));
    token.transfer(from, e.current_contract_address(), &amount);

    let c_dep = Grumpkin::mul(e, &Grumpkin::generator(e), amount as u128);
    add_to_receiving(e, to, &c_dep);
    emit_deposit(e, from, to, amount);
}

/// Folds `account.receiving_balance` into `account.spendable_balance` and
/// resets the receiving storage entry to the identity.
///
/// # Arguments
///
/// * `e` - Access to the Soroban environment.
/// * `account` - The owner address.
///
/// # Errors
///
/// * [`WrapperError::AccountNotRegistered`] - When `account` is not registered.
///
/// # Events
///
/// * topics - `["merge", account: Address]`
/// * data - `[]`
///
/// # Security Warning
///
/// **IMPORTANT**: This function bypasses authorization checks. The trait
/// entry point is responsible for calling `account.require_auth()`.
pub fn merge_no_auth(e: &Env, account: &Address) {
    let mut data = get_account(e, account);
    data.spendable_balance = Grumpkin::add(e, &data.spendable_balance, &data.receiving_balance);
    data.receiving_balance = Grumpkin::identity(e);
    e.storage().persistent().set(&WrapperStorageKey::Account(account.clone()), &data);

    emit_merge(e, account);
}

/// Withdraws `amount` units to the public SEP-41 address `to` from the
/// confidential spendable balance of `from`.
///
/// # Arguments
///
/// * `e` - Access to the Soroban environment.
/// * `from` - The confidential account spending the funds.
/// * `to` - The SEP-41 recipient.
/// * `amount` - The non-negative withdrawal amount.
/// * `payload` - The decoded [`WithdrawPayload`].
/// * `proof` - The raw UltraHonk proof bytes.
///
/// # Errors
///
/// * [`WrapperError::NegativeAmount`] - When `amount < 0`.
/// * [`WrapperError::AccountNotRegistered`] - When `from` is not registered.
/// * [`WrapperError::InvalidProof`] - When the proof fails verification.
/// * refer to [`crate::confidential::auditor::ConfidentialAuditor::get_key`]
///   errors.
///
/// # Events
///
/// * topics - `["withdraw", from: Address, to: Address]`
/// * data - `[amount: i128, r_e: BytesN<64>, sigma: BytesN<32>, b_tilde:
///   BytesN<32>, b_aud_s: BytesN<32>]`
///
/// # Notes
///
/// The proof binds the confidential balance debit to exactly `amount`, and
/// this function then invokes `transfer` on the underlying token for the
/// same `amount` without re-measuring the recipient's balance. The
/// underlying token MUST therefore have exact-transfer semantics (no
/// fee-on-transfer, no rebasing) — otherwise the recipient would receive
/// less than was debited from the confidential balance. See the
/// [module-level constraint](super) for the list of supported token
/// implementations.
///
/// # Security Warning
///
/// **IMPORTANT**: This function bypasses authorization checks. The trait
/// entry point is responsible for calling `from.require_auth()`.
pub fn withdraw_no_auth(
    e: &Env,
    from: &Address,
    to: &Address,
    amount: i128,
    payload: &WithdrawPayload,
    proof: &Bytes,
) {
    if amount < 0 {
        panic_with_error!(e, WrapperError::NegativeAmount);
    }

    let account = get_account(e, from);
    let auditor = ConfidentialAuditorClient::new(e, &get_auditor(e));
    let k_aud_s = auditor.get_key(&account.auditor_id);
    let wrap = get_wrap(e);

    // PI order (DESIGN §7.5):
    //   C_spend, Y, wrap, K_aud_s, a,
    //   C_spend', sigma, b_tilde, R_e, b_aud_s
    let mut pi = Bytes::new(e);
    append_point(&mut pi, &account.spendable_balance);
    append_point(&mut pi, &account.spending_key);
    append_field(&mut pi, &wrap);
    append_point(&mut pi, &k_aud_s);
    append_amount(&mut pi, e, amount);
    append_point(&mut pi, &payload.c_spend_new);
    append_field(&mut pi, &payload.sigma);
    append_field(&mut pi, &payload.b_tilde);
    append_point(&mut pi, &payload.r_e);
    append_field(&mut pi, &payload.b_aud_s);

    verify(e, CircuitType::Withdraw, &pi, proof);

    set_spendable(e, from, &payload.c_spend_new);

    let token = token::TokenClient::new(e, &get_token(e));
    token.transfer(&e.current_contract_address(), to, &amount);

    emit_withdraw(
        e,
        from,
        to,
        amount,
        &payload.r_e,
        &payload.sigma,
        &payload.b_tilde,
        &payload.b_aud_s,
    );
}

/// Sends a confidential transfer from `from` to `to`.
///
/// # Arguments
///
/// * `e` - Access to the Soroban environment.
/// * `from` - The sender.
/// * `to` - The recipient.
/// * `payload` - The decoded [`TransferPayload`].
/// * `proof` - The raw UltraHonk proof bytes.
///
/// # Errors
///
/// * [`WrapperError::AccountNotRegistered`] - When `from` or `to` is not
///   registered.
/// * [`WrapperError::InvalidProof`] - When the proof fails verification.
/// * refer to [`crate::confidential::auditor::ConfidentialAuditor::get_key`]
///   errors.
///
/// # Events
///
/// * topics - `["transfer", from: Address, to: Address]`
/// * data - `[r_e, v_tilde, sigma, b_tilde, v_aud_r, r_aud_r, v_aud_s,
///   b_aud_s]`
///
/// # Security Warning
///
/// **IMPORTANT**: This function bypasses authorization checks. The trait
/// entry point is responsible for calling `from.require_auth()`.
pub fn confidential_transfer_no_auth(
    e: &Env,
    from: &Address,
    to: &Address,
    payload: &TransferPayload,
    proof: &Bytes,
) {
    let sender = get_account(e, from);
    let recipient = get_account(e, to);
    let auditor = ConfidentialAuditorClient::new(e, &get_auditor(e));
    let k_aud_r = auditor.get_key(&recipient.auditor_id);
    let k_aud_s = auditor.get_key(&sender.auditor_id);
    let wrap = get_wrap(e);

    // PI order (DESIGN §7.6):
    //   C_spend_A, Y_A, PVK_B, wrap, K_aud_r, K_aud_s,
    //   C_spend', C_tx, R_e, v_tilde, b_tilde, sigma,
    //   v_aud_r, r_aud_r, v_aud_s, b_aud_s
    let mut pi = Bytes::new(e);
    append_point(&mut pi, &sender.spendable_balance);
    append_point(&mut pi, &sender.spending_key);
    append_point(&mut pi, &recipient.viewing_public_key);
    append_field(&mut pi, &wrap);
    append_point(&mut pi, &k_aud_r);
    append_point(&mut pi, &k_aud_s);
    append_point(&mut pi, &payload.c_spend_new);
    append_point(&mut pi, &payload.c_tx);
    append_point(&mut pi, &payload.r_e);
    append_field(&mut pi, &payload.v_tilde);
    append_field(&mut pi, &payload.b_tilde);
    append_field(&mut pi, &payload.sigma);
    append_field(&mut pi, &payload.v_aud_r);
    append_field(&mut pi, &payload.r_aud_r);
    append_field(&mut pi, &payload.v_aud_s);
    append_field(&mut pi, &payload.b_aud_s);

    verify(e, CircuitType::Transfer, &pi, proof);

    set_spendable(e, from, &payload.c_spend_new);
    add_to_receiving(e, to, &payload.c_tx);

    emit_transfer(
        e,
        from,
        to,
        &payload.r_e,
        &payload.v_tilde,
        &payload.sigma,
        &payload.b_tilde,
        &payload.v_aud_r,
        &payload.r_aud_r,
        &payload.v_aud_s,
        &payload.b_aud_s,
    );
}

/// Spends from `from`'s allowance escrowed to `operator`, transferring
/// confidentially to `to`.
///
/// # Arguments
///
/// * `e` - Access to the Soroban environment.
/// * `operator` - The delegated operator (the auth principal).
/// * `from` - The owner whose allowance is being spent.
/// * `to` - The recipient.
/// * `payload` - The decoded [`OperatorTransferPayload`].
/// * `proof` - The raw UltraHonk proof bytes.
///
/// # Errors
///
/// * [`WrapperError::AccountNotRegistered`] - When `from`, `operator`, or `to`
///   is not registered.
/// * [`WrapperError::DelegationNotFound`] - When `(from, operator)` has no
///   delegation.
/// * [`WrapperError::DelegationExpired`] - When the delegation has expired.
/// * [`WrapperError::InvalidProof`] - When the proof fails verification.
/// * refer to [`crate::confidential::auditor::ConfidentialAuditor::get_key`]
///   errors.
///
/// # Events
///
/// * topics - `["operator_transfer", operator: Address, from: Address, to:
///   Address]`
/// * data - `[r_e, v_tilde, sigma_a, v_aud_r, r_aud_r, v_aud_s, a_aud_s]`
///
/// # Security Warning
///
/// **IMPORTANT**: This function bypasses authorization checks. The trait
/// entry point is responsible for calling `operator.require_auth()`.
pub fn confidential_transfer_from_no_auth(
    e: &Env,
    operator: &Address,
    from: &Address,
    to: &Address,
    payload: &OperatorTransferPayload,
    proof: &Bytes,
) {
    let delegation = get_delegation(e, from, operator);
    if e.ledger().sequence() > delegation.expiration_ledger {
        panic_with_error!(e, WrapperError::DelegationExpired);
    }

    let owner = get_account(e, from);
    let operator_account = get_account(e, operator);
    let recipient = get_account(e, to);
    let auditor = ConfidentialAuditorClient::new(e, &get_auditor(e));
    let k_aud_r = auditor.get_key(&recipient.auditor_id);
    // Sender-auditor key is the OWNER's auditor, not the operator's (DESIGN
    // §7.8 — visibility points balance/allowance ciphertexts at the funds'
    // owner).
    let k_aud_s = auditor.get_key(&owner.auditor_id);

    // PI order (DESIGN §7.8):
    //   C_a, sigma_a, Y_op, PVK_recipient, K_aud_r, K_aud_s,
    //   C_a', C_tx, R_e, v_tilde, a_tilde', sigma_a',
    //   v_aud_r, r_aud_r, v_aud_s, a_aud_s
    let mut pi = Bytes::new(e);
    append_point(&mut pi, &delegation.allowance_commitment);
    append_field(&mut pi, &delegation.allowance_salt);
    append_point(&mut pi, &operator_account.spending_key);
    append_point(&mut pi, &recipient.viewing_public_key);
    append_point(&mut pi, &k_aud_r);
    append_point(&mut pi, &k_aud_s);
    append_point(&mut pi, &payload.c_a_new);
    append_point(&mut pi, &payload.c_tx);
    append_point(&mut pi, &payload.r_e);
    append_field(&mut pi, &payload.v_tilde);
    append_field(&mut pi, &payload.a_tilde_new);
    append_field(&mut pi, &payload.sigma_a_new);
    append_field(&mut pi, &payload.v_aud_r);
    append_field(&mut pi, &payload.r_aud_r);
    append_field(&mut pi, &payload.v_aud_s);
    append_field(&mut pi, &payload.a_aud_s);

    verify(e, CircuitType::OperatorTransfer, &pi, proof);

    update_delegation(
        e,
        from,
        operator,
        &payload.c_a_new,
        &payload.a_tilde_new,
        &payload.sigma_a_new,
    );
    add_to_receiving(e, to, &payload.c_tx);

    emit_operator_transfer(
        e,
        operator,
        from,
        to,
        &payload.r_e,
        &payload.v_tilde,
        &payload.sigma_a_new,
        &payload.v_aud_r,
        &payload.r_aud_r,
        &payload.v_aud_s,
        &payload.a_aud_s,
    );
}

/// Escrows an allowance from `account`'s spendable balance and delegates it
/// to `operator`. Reverts if a delegation already exists for the `(account,
/// operator)` pair.
///
/// # Arguments
///
/// * `e` - Access to the Soroban environment.
/// * `account` - The delegating owner.
/// * `operator` - The delegated operator. Must be a registered confidential
///   account so its spending key is available for the `dvk` escrow ECDH.
/// * `expiration_ledger` - Ledger sequence at which the delegation stops
///   authorizing spending.
/// * `payload` - The decoded [`SetOperatorPayload`].
/// * `proof` - The raw UltraHonk proof bytes.
///
/// # Errors
///
/// * [`WrapperError::AccountNotRegistered`] - When `account` or `operator` is
///   not registered.
/// * [`WrapperError::DelegationAlreadyExists`] - When a delegation already
///   exists for the `(account, operator)` pair.
/// * [`WrapperError::InvalidProof`] - When the proof fails verification.
/// * refer to [`crate::confidential::auditor::ConfidentialAuditor::get_key`]
///   errors.
///
/// # Events
///
/// * topics - `["set_operator", account: Address, operator: Address]`
/// * data - `[expiration_ledger: u32, r_e, sigma, b_tilde, v_aud_s, b_aud_s]`
///
/// # Security Warning
///
/// **IMPORTANT**: This function bypasses authorization checks. The trait
/// entry point is responsible for calling `account.require_auth()`.
pub fn set_operator_no_auth(
    e: &Env,
    account: &Address,
    operator: &Address,
    expiration_ledger: u32,
    payload: &SetOperatorPayload,
    proof: &Bytes,
) {
    let owner = get_account(e, account);
    let operator_account = get_account(e, operator);
    let auditor = ConfidentialAuditorClient::new(e, &get_auditor(e));
    let k_aud_s = auditor.get_key(&owner.auditor_id);
    let wrap = get_wrap(e);
    let op_i = address_to_field(e, operator);

    // PI order (DESIGN §7.7):
    //   C_spend, Y, Y_op, op_i, wrap, K_aud_s,
    //   C_spend', C_a, escrowed_dvk, b_tilde, a_tilde,
    //   sigma, sigma_a, R_e, v_aud_s, b_aud_s
    let mut pi = Bytes::new(e);
    append_point(&mut pi, &owner.spendable_balance);
    append_point(&mut pi, &owner.spending_key);
    append_point(&mut pi, &operator_account.spending_key);
    append_field(&mut pi, &op_i);
    append_field(&mut pi, &wrap);
    append_point(&mut pi, &k_aud_s);
    append_point(&mut pi, &payload.c_spend_new);
    append_point(&mut pi, &payload.c_a);
    append_point(&mut pi, &payload.escrowed_dvk);
    append_field(&mut pi, &payload.b_tilde);
    append_field(&mut pi, &payload.a_tilde);
    append_field(&mut pi, &payload.sigma);
    append_field(&mut pi, &payload.sigma_a);
    append_point(&mut pi, &payload.r_e);
    append_field(&mut pi, &payload.v_aud_s);
    append_field(&mut pi, &payload.b_aud_s);

    verify(e, CircuitType::SetOperator, &pi, proof);

    set_spendable(e, account, &payload.c_spend_new);
    set_delegation(
        e,
        account,
        operator,
        &OperatorDelegation {
            allowance_commitment: payload.c_a.clone(),
            encrypted_allowance: payload.a_tilde.clone(),
            escrowed_dvk: payload.escrowed_dvk.clone(),
            allowance_salt: payload.sigma_a.clone(),
            expiration_ledger,
        },
    );

    emit_set_operator(
        e,
        account,
        operator,
        expiration_ledger,
        &payload.r_e,
        &payload.sigma,
        &payload.b_tilde,
        &payload.v_aud_s,
        &payload.b_aud_s,
    );
}

/// Revokes the `(account, operator)` delegation and folds the remaining
/// escrowed allowance back into `account`'s spendable balance.
/// Works for both active and expired-but-not-revoked delegations.
///
/// # Arguments
///
/// * `e` - Access to the Soroban environment.
/// * `account` - The owner reclaiming the allowance.
/// * `operator` - The previously-delegated operator.
/// * `payload` - The decoded [`RevokeOperatorPayload`].
/// * `proof` - The raw UltraHonk proof bytes.
///
/// # Errors
///
/// * [`WrapperError::AccountNotRegistered`] - When `account` is not registered.
/// * [`WrapperError::DelegationNotFound`] - When `(account, operator)` has no
///   delegation.
/// * [`WrapperError::InvalidProof`] - When the proof fails verification.
/// * refer to [`crate::confidential::auditor::ConfidentialAuditor::get_key`]
///   errors.
///
/// # Events
///
/// * topics - `["revoke_operator", account: Address, operator: Address]`
/// * data - `[r_e, sigma, b_tilde, v_aud_s, b_aud_s]`
///
/// # Security Warning
///
/// **IMPORTANT**: This function bypasses authorization checks. The trait
/// entry point is responsible for calling `account.require_auth()`.
pub fn revoke_operator_no_auth(
    e: &Env,
    account: &Address,
    operator: &Address,
    payload: &RevokeOperatorPayload,
    proof: &Bytes,
) {
    let owner = get_account(e, account);
    let delegation = get_delegation(e, account, operator);
    let auditor = ConfidentialAuditorClient::new(e, &get_auditor(e));
    let k_aud_s = auditor.get_key(&owner.auditor_id);
    let wrap = get_wrap(e);
    let op_i = address_to_field(e, operator);

    // PI order (DESIGN §7.9):
    //   C_spend, C_a, sigma_a, Y, op_i, wrap, K_aud_s,
    //   C_spend', b_tilde, sigma, R_e, v_aud_s, b_aud_s
    let mut pi = Bytes::new(e);
    append_point(&mut pi, &owner.spendable_balance);
    append_point(&mut pi, &delegation.allowance_commitment);
    append_field(&mut pi, &delegation.allowance_salt);
    append_point(&mut pi, &owner.spending_key);
    append_field(&mut pi, &op_i);
    append_field(&mut pi, &wrap);
    append_point(&mut pi, &k_aud_s);
    append_point(&mut pi, &payload.c_spend_new);
    append_field(&mut pi, &payload.b_tilde);
    append_field(&mut pi, &payload.sigma);
    append_point(&mut pi, &payload.r_e);
    append_field(&mut pi, &payload.v_aud_s);
    append_field(&mut pi, &payload.b_aud_s);

    verify(e, CircuitType::RevokeOperator, &pi, proof);

    set_spendable(e, account, &payload.c_spend_new);
    delete_delegation(e, account, operator);

    emit_revoke_operator(
        e,
        account,
        operator,
        &payload.r_e,
        &payload.sigma,
        &payload.b_tilde,
        &payload.v_aud_s,
        &payload.b_aud_s,
    );
}

/// Sets the SEP-41 token address.
///
/// This function is **single-shot**: it reverts on any call after the first.
/// The token address is the reserve identity backing every confidential
/// balance; changing it after construction would break the link between the
/// on-chain reserves and the credits issued by [`deposit_no_auth`] /
/// [`withdraw_no_auth`]. The intended caller is the contract's
/// `__constructor`.
///
/// # Arguments
///
/// * `e` - Access to the Soroban environment.
/// * `token` - The SEP-41 token contract.
///
/// # Errors
///
/// * [`WrapperError::TokenAlreadySet`] - When the token address has already
///   been set.
///
/// # Events
///
/// * topics - `["token_set"]`
/// * data - `[token: Address]`
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
pub fn set_token_no_auth(e: &Env, token: &Address) {
    if e.storage().instance().has(&WrapperStorageKey::Token) {
        panic_with_error!(e, WrapperError::TokenAlreadySet);
    }
    e.storage().instance().set(&WrapperStorageKey::Token, token);
    emit_token_set(e, token);
}

/// Sets the confidential verifier contract address.
///
/// Unlike [`set_token_no_auth`] and [`set_wrap_no_auth`], this function has
/// no single-shot guard: rotating the verifier is a legitimate operation
/// (e.g. when a new circuit version ships or the verifier contract is
/// patched). Contract authors who need to rotate the verifier post-deployment
/// should expose the operation behind an admin-gated entry point — typically
/// owned by a multisig or timelock — since changing the verifier changes the
/// set of proofs the wrapper will accept.
///
/// # Arguments
///
/// * `e` - Access to the Soroban environment.
/// * `verifier` - The verifier registry contract.
///
/// # Events
///
/// * topics - `["verifier_set"]`
/// * data - `[verifier: Address]`
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
pub fn set_verifier_no_auth(e: &Env, verifier: &Address) {
    e.storage().instance().set(&WrapperStorageKey::Verifier, verifier);
    emit_verifier_set(e, verifier);
}

/// Sets the auditor registry contract address.
///
/// Unlike [`set_token_no_auth`] and [`set_wrap_no_auth`], this function has
/// no single-shot guard: rotating the auditor registry is a legitimate
/// operation (e.g. when auditor key custody changes or the registry contract
/// is patched). Contract authors who need to rotate the auditor
/// post-deployment should expose the operation behind an admin-gated entry
/// point — typically owned by a multisig or timelock — since the registry
/// controls which keys auditor ciphertexts are encrypted under.
///
/// # Arguments
///
/// * `e` - Access to the Soroban environment.
/// * `auditor` - The auditor registry contract.
///
/// # Events
///
/// * topics - `["auditor_set"]`
/// * data - `[auditor: Address]`
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
pub fn set_auditor_no_auth(e: &Env, auditor: &Address) {
    e.storage().instance().set(&WrapperStorageKey::Auditor, auditor);
    emit_auditor_set(e, auditor);
}

/// Stores the wrapper's compressed `wrap` Field in instance storage.
///
/// `wrap` is the Poseidon2 compression of the wrapper's own address. It is
/// bound into every owner-initiated circuit's viewing-key derivation, so it
/// must be computed once over `env.current_contract_address()` at
/// construction and never re-derived.
///
/// This function is **single-shot**: it reverts on any call after the first.
/// Changing `wrap` after construction would invalidate every previously
/// registered account, since their `vk` derivations are bound to the
/// original value. The intended caller is the contract's `__constructor`.
///
/// # Arguments
///
/// * `e` - Access to the Soroban environment.
///
/// # Errors
///
/// * [`WrapperError::WrapAlreadySet`] - When the `wrap` field has already been
///   set.
///
/// # Events
///
/// * topics - `["wrap_set"]`
/// * data - `[wrap: BytesN<32>]`
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
pub fn set_wrap_no_auth(e: &Env) {
    if e.storage().instance().has(&WrapperStorageKey::Wrap) {
        panic_with_error!(e, WrapperError::WrapAlreadySet);
    }
    let computed = compute_wrap(e, &e.current_contract_address());
    e.storage().instance().set(&WrapperStorageKey::Wrap, &computed);
    emit_wrap_set(e, &computed);
}

// ################## LOW-LEVEL HELPERS ##################

/// Computes `wrap = Poseidon2(δ_addr, lo, hi)` over `addr`'s 56-byte strkey,
/// as a canonical 32-byte big-endian `Bn254Fr` representative.
///
/// Intended for use in `__constructor` against
/// `env.current_contract_address()` and then stored via [`set_wrap`].
///
/// # Arguments
///
/// * `e` - Access to the Soroban environment.
/// * `addr` - The address to compress.
pub fn compute_wrap(e: &Env, addr: &Address) -> BytesN<32> {
    address_to_field(e, addr)
}

/// Overwrites `account.spendable_balance` with `c_spend_new`.
///
/// # Arguments
///
/// * `e` - Access to the Soroban environment.
/// * `account` - The owner address.
/// * `c_spend_new` - The new commitment.
///
/// # Errors
///
/// * [`WrapperError::AccountNotRegistered`] - When `account` is not registered.
fn set_spendable(e: &Env, account: &Address, c_spend_new: &Point) {
    let mut data = get_account(e, account);
    data.spendable_balance = c_spend_new.clone();
    e.storage().persistent().set(&WrapperStorageKey::Account(account.clone()), &data);
}

/// Adds `c_tx` to `account.receiving_balance` via Grumpkin homomorphic
/// addition. Used by deposits and incoming transfers.
///
/// # Arguments
///
/// * `e` - Access to the Soroban environment.
/// * `account` - The recipient address.
/// * `c_tx` - The commitment to fold in.
///
/// # Errors
///
/// * [`WrapperError::AccountNotRegistered`] - When `account` is not registered.
fn add_to_receiving(e: &Env, account: &Address, c_tx: &Point) {
    let mut data = get_account(e, account);
    data.receiving_balance = Grumpkin::add(e, &data.receiving_balance, c_tx);
    e.storage().persistent().set(&WrapperStorageKey::Account(account.clone()), &data);
}

/// Stores a fresh [`OperatorDelegation`] under `(owner, operator)`.
///
/// # Arguments
///
/// * `e` - Access to the Soroban environment.
/// * `owner` - The delegating account.
/// * `operator` - The delegated operator.
/// * `delegation` - The delegation entry.
///
/// # Errors
///
/// * [`WrapperError::DelegationAlreadyExists`] - When a delegation already
///   exists for the `(owner, operator)` pair.
fn set_delegation(e: &Env, owner: &Address, operator: &Address, delegation: &OperatorDelegation) {
    let key = WrapperStorageKey::Delegation(owner.clone(), operator.clone());
    if e.storage().persistent().has(&key) {
        panic_with_error!(e, WrapperError::DelegationAlreadyExists);
    }
    e.storage().persistent().set(&key, delegation);
}

/// Updates a delegation's allowance commitment, encrypted allowance, and
/// salt after an operator transfer.
///
/// # Arguments
///
/// * `e` - Access to the Soroban environment.
/// * `owner` - The delegating account.
/// * `operator` - The delegated operator.
/// * `c_a_new` - New allowance commitment.
/// * `a_tilde_new` - New encrypted allowance scalar.
/// * `sigma_a_new` - New allowance salt.
///
/// # Errors
///
/// * [`WrapperError::DelegationNotFound`] - When no delegation exists for the
///   pair.
fn update_delegation(
    e: &Env,
    owner: &Address,
    operator: &Address,
    c_a_new: &Point,
    a_tilde_new: &BytesN<32>,
    sigma_a_new: &BytesN<32>,
) {
    let mut delegation = get_delegation(e, owner, operator);
    delegation.allowance_commitment = c_a_new.clone();
    delegation.encrypted_allowance = a_tilde_new.clone();
    delegation.allowance_salt = sigma_a_new.clone();
    e.storage()
        .persistent()
        .set(&WrapperStorageKey::Delegation(owner.clone(), operator.clone()), &delegation);
}

/// Deletes the `(owner, operator)` delegation entry (revoke path).
///
/// # Arguments
///
/// * `e` - Access to the Soroban environment.
/// * `owner` - The delegating account.
/// * `operator` - The delegated operator.
///
/// # Errors
///
/// * [`WrapperError::DelegationNotFound`] - When no delegation exists for the
///   pair.
fn delete_delegation(e: &Env, owner: &Address, operator: &Address) {
    let key = WrapperStorageKey::Delegation(owner.clone(), operator.clone());
    if !e.storage().persistent().has(&key) {
        panic_with_error!(e, WrapperError::DelegationNotFound);
    }
    e.storage().persistent().remove(&key);
}

/// Tries to retrieve a persistent storage value and extend its TTL if the
/// entry exists.
fn get_persistent_entry<T: TryFromVal<Env, Val>>(e: &Env, key: &WrapperStorageKey) -> Option<T> {
    e.storage().persistent().get::<_, T>(key).inspect(|_| {
        let (threshold, extend) = match key {
            WrapperStorageKey::Account(_) => (ACCOUNT_TTL_THRESHOLD, ACCOUNT_EXTEND_AMOUNT),
            WrapperStorageKey::Delegation(_, _) => {
                (DELEGATION_TTL_THRESHOLD, DELEGATION_EXTEND_AMOUNT)
            }
            _ => return,
        };
        e.storage().persistent().extend_ttl(key, threshold, extend);
    })
}

/// Decodes a `data: Bytes` as the corresponding XDR-encoded `#[contracttype]`
/// struct.
///
/// # Errors
///
/// * [`WrapperError::InvalidData`] - When `data` is not a valid XDR encoding of
///   `T`.
pub fn decode_data<T>(e: &Env, data: &Bytes) -> T
where
    T: FromXdr,
{
    T::from_xdr(e, data).unwrap_or_else(|_| panic_with_error!(e, WrapperError::InvalidData))
}

/// Cross-contract proof verification through [`ConfidentialVerifierClient`].
/// Panics with [`WrapperError::InvalidProof`] if the verifier returns `false`.
fn verify(e: &Env, circuit_type: CircuitType, public_inputs: &Bytes, proof: &Bytes) {
    let verifier = ConfidentialVerifierClient::new(e, &get_verifier(e));
    if !verifier.verify_proof(&circuit_type, public_inputs, proof) {
        panic_with_error!(e, WrapperError::InvalidProof);
    }
}

/// Appends a Grumpkin point (`x || y`, 64 bytes) to the public-input blob.
fn append_point(pi: &mut Bytes, p: &Point) {
    pi.append(&Bytes::from(p));
}

/// Appends a 32-byte field element to the public-input blob.
fn append_field(pi: &mut Bytes, f: &BytesN<32>) {
    pi.append(&Bytes::from(f));
}

/// Appends a non-negative `i128` amount as a canonical 32-byte big-endian
/// `Bn254Fr` representative.
fn append_amount(pi: &mut Bytes, e: &Env, amount: i128) {
    let mut buf = [0u8; 32];
    buf[16..].copy_from_slice(&amount.to_be_bytes());
    pi.append(&Bytes::from_array(e, &buf));
}

/// Compresses an address into a canonical 32-byte big-endian `Bn254Fr`
/// representative via `Poseidon2(δ_addr, lo, hi)` over its 56-byte stellar
/// strkey.
fn address_to_field(e: &Env, addr: &Address) -> BytesN<32> {
    let strkey = addr.to_string();
    let mut buf = [0u8; STRKEY_LEN];
    strkey.copy_into_slice(&mut buf);

    let lo = le_bytes_to_u256(e, &buf[..STRKEY_LIMB_LEN]);
    let hi = le_bytes_to_u256(e, &buf[STRKEY_LIMB_LEN..]);

    let inputs = soroban_sdk::vec![e, U256::from_u32(e, DELTA_ADDR), lo, hi,];
    let hash: U256 = poseidon2_hash::<4, Bn254Fr>(e, &inputs);
    u256_to_bytes_n_32(e, &hash)
}

/// Interprets a 28-byte slice in little-endian order as a `U256`. The caller
/// passes the lower or upper 28 bytes of the 56-byte strkey;
/// both fit in `Bn254Fr` since each limb is bounded by `2^224 ≪ r`.
fn le_bytes_to_u256(e: &Env, le: &[u8]) -> U256 {
    let mut be = [0u8; 32];
    for (i, b) in le.iter().enumerate() {
        be[31 - i] = *b;
    }
    U256::from_be_bytes(e, &Bytes::from_array(e, &be))
}

/// Returns the canonical 32-byte big-endian encoding of a `U256`.
fn u256_to_bytes_n_32(e: &Env, u: &U256) -> BytesN<32> {
    BytesN::<32>::try_from(u.to_be_bytes())
        .unwrap_or_else(|_| panic_with_error!(e, WrapperError::InvalidData))
}
