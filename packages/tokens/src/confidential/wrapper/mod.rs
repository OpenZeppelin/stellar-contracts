//! # Confidential Token Wrapper
//!
//! Core contract that wraps a SEP-41 token to provide confidential balances
//! and transfers. Balances are stored as Pedersen commitments on the Grumpkin
//! curve; every state-changing operation that consumes private state is
//! accompanied by an UltraHonk proof that the wrapper verifies via a separate
//! verifier contract. Auditor keys are read from a separate registry contract
//! and dual auditor ciphertexts are emitted in each transfer event. See
//! [`docs/DESIGN.github.md`] for the full specification.
//!
//! [`docs/DESIGN.github.md`]: https://github.com/OpenZeppelin/stellar-contracts/blob/main/packages/tokens/src/confidential/docs/DESIGN.github.md
//!
//! # ⚠️ Not Production Ready
//!
//! This module depends on
//! [`super::verifier::ConfidentialVerifier::verify_proof`], whose UltraHonk
//! backend (`rs-soroban-ultrahonk`) is still under development and has not been
//! audited. Do **not** deploy a contract built on this trait to mainnet or any
//! environment that handles real value. See the verifier module-level warning
//! for details.
//!
//! ## What This Crate Provides
//!
//! - The [`ConfidentialTokenWrapper`] trait — eleven entry points (register,
//!   deposit, merge, withdraw, two transfer variants, two operator management
//!   methods, and three read methods) with default bodies that delegate to the
//!   matching free functions in [`storage`].
//! - The on-chain account state types [`ConfidentialAccount`] and
//!   [`OperatorDelegation`] (DESIGN §6).
//! - The six XDR payload types carried in the `data: Bytes` parameter (DESIGN
//!   §11).
//! - Storage helpers and operation-level orchestration under [`storage`].
//!
//! ## Public-Input Encoding
//!
//! The wrapper assembles the public-input blob handed to the verifier by
//! concatenating canonical 32-byte big-endian `Bn254Fr` representatives in
//! the order specified by each circuit's table in DESIGN §7. Grumpkin
//! points contribute two consecutive 32-byte limbs (`x` then `y`). Public
//! `i128` amounts are zero-padded to 32 bytes. The encoding is positional
//! (no length prefix); any divergence from the order the prover used will
//! cause verification to fail.
//!
//! ## Wrapper Binding
//!
//! Every owner-initiated proof references a `wrap` field, computed once at
//! construction as `Poseidon2(δ_addr, lo, hi)` over the wrapper's own
//! address (DESIGN §2.7, §3.5) and stored in instance storage.
//!
//! ## Storage
//!
//! Singleton configuration (`token`, `verifier`, `auditor`, `wrap`) lives in
//! instance storage. Per-account state lives in persistent storage; reads
//! extend TTL, writes do not (CLAUDE.md storage convention).

pub mod storage;

#[cfg(test)]
mod test;

use soroban_sdk::{
    contracterror, contractevent, contracttrait, contracttype, Address, Bytes, BytesN, Env,
};
use stellar_contract_utils::crypto::grumpkin::Point;
pub use storage::{compute_wrap, WrapperStorageKey};

/// Trait for the confidential token wrapper.
///
/// All methods have default bodies that delegate to the corresponding free
/// function in [`storage`]. The storage functions perform the full
/// operation per DESIGN §7.X: authorize the caller, XDR-decode the `data`
/// payload, load every trusted-state public input from on-chain state
/// (never from caller-controlled bytes per DESIGN §7.1 *Trust-boundary
/// rule*), assemble the public-input blob in the prescribed order, call
/// [`ConfidentialVerifier::verify_proof`] cross-contract, apply the
/// post-verification state mutation, and emit the DESIGN §11.2 event.
///
/// [`ConfidentialVerifier::verify_proof`]: super::verifier::ConfidentialVerifier::verify_proof
#[contracttrait]
pub trait ConfidentialTokenWrapper {
    /// Registers a confidential account under `account` (DESIGN §7.2).
    ///
    /// `data` is the XDR encoding of [`RegisterPayload`] — the prover's
    /// spending public key `Y`, public viewing key `PVK`, and proof blob.
    /// The wrapper validates that `auditor_id` resolves to a registered
    /// auditor key and verifies the [`super::verifier::CircuitType::Register`]
    /// proof before writing `account`'s slot with `spendable_balance =
    /// receiving_balance = O` (identity).
    ///
    /// # Arguments
    ///
    /// * `e` - Access to the Soroban environment.
    /// * `account` - The owner address being registered.
    /// * `auditor_id` - The auditor key index this account commits to.
    /// * `data` - XDR-encoded [`RegisterPayload`].
    ///
    /// # Errors
    ///
    /// * refer to [`storage::register`] errors.
    ///
    /// # Events
    ///
    /// * topics - `["register", account: Address]`
    /// * data - `[auditor_id: u32]`
    fn register(e: &Env, account: Address, auditor_id: u32, data: Bytes) {
        storage::register(e, &account, auditor_id, &data);
    }

    /// Deposits `amount` units of the underlying SEP-41 token from `from`
    /// into `to`'s confidential receiving balance (DESIGN §7.3).
    ///
    /// No proof is required. The deposit commitment is `a · G` with zero
    /// blinding, added homomorphically to `to.receiving_balance`. The
    /// depositor `from` does not need a registered confidential account;
    /// only `to` does.
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
    /// * refer to [`storage::deposit`] errors.
    ///
    /// # Events
    ///
    /// * topics - `["deposit", from: Address, to: Address]`
    /// * data - `[amount: i128]`
    fn deposit(e: &Env, from: Address, to: Address, amount: i128) {
        storage::deposit(e, &from, &to, amount);
    }

    /// Folds `account.receiving_balance` into `account.spendable_balance`
    /// and resets the receiving slot to the identity (DESIGN §7.4).
    ///
    /// No proof is required; correctness follows from the homomorphic
    /// property of Pedersen commitments. Only the account holder can
    /// authorize a merge — third parties cannot weaponize it (DESIGN §9.2).
    ///
    /// # Arguments
    ///
    /// * `e` - Access to the Soroban environment.
    /// * `account` - The owner address.
    ///
    /// # Errors
    ///
    /// * refer to [`storage::merge`] errors.
    ///
    /// # Events
    ///
    /// * topics - `["merge", account: Address]`
    /// * data - `[]`
    fn merge(e: &Env, account: Address) {
        storage::merge(e, &account);
    }

    /// Withdraws `amount` units to the public SEP-41 address `to` from the
    /// confidential spendable balance of `from` (DESIGN §7.5).
    ///
    /// `data` is the XDR encoding of [`WithdrawPayload`]. The proof binds
    /// the new spendable commitment, the encrypted balance scalar, and the
    /// sender-auditor ciphertext.
    ///
    /// # Arguments
    ///
    /// * `e` - Access to the Soroban environment.
    /// * `from` - The confidential account spending the funds.
    /// * `to` - The SEP-41 recipient.
    /// * `amount` - The non-negative withdrawal amount.
    /// * `data` - XDR-encoded [`WithdrawPayload`].
    ///
    /// # Errors
    ///
    /// * refer to [`storage::withdraw`] errors.
    ///
    /// # Events
    ///
    /// * topics - `["withdraw", from: Address, to: Address]`
    /// * data - `[amount: i128, r_e: BytesN<64>, sigma: BytesN<32>, b_tilde:
    ///   BytesN<32>, b_aud_s: BytesN<32>]`
    fn withdraw(e: &Env, from: Address, to: Address, amount: i128, data: Bytes) {
        storage::withdraw(e, &from, &to, amount, &data);
    }

    /// Sends a confidential transfer from `from` to `to` (DESIGN §7.6).
    ///
    /// `data` is the XDR encoding of [`TransferPayload`]. The proof binds
    /// balance sufficiency, ECDH-derived recipient blinding, dual auditor
    /// ciphertexts, the new sender spendable commitment, and the encrypted
    /// balance scalar.
    ///
    /// # Arguments
    ///
    /// * `e` - Access to the Soroban environment.
    /// * `from` - The sender.
    /// * `to` - The recipient.
    /// * `data` - XDR-encoded [`TransferPayload`].
    ///
    /// # Errors
    ///
    /// * refer to [`storage::confidential_transfer`] errors.
    ///
    /// # Events
    ///
    /// * topics - `["transfer", from: Address, to: Address]`
    /// * data - `[r_e, v_tilde, sigma, b_tilde, v_aud_r, r_aud_r, v_aud_s,
    ///   b_aud_s]`
    fn confidential_transfer(e: &Env, from: Address, to: Address, data: Bytes) {
        storage::confidential_transfer(e, &from, &to, &data);
    }

    /// Spends from `from`'s allowance escrowed to `operator`, transferring
    /// confidentially to `to` (DESIGN §7.8). The owner's authorization was
    /// granted at `set_operator` and persists in the on-chain delegation
    /// entry; only the operator authorizes this call.
    ///
    /// # Arguments
    ///
    /// * `e` - Access to the Soroban environment.
    /// * `operator` - The delegated operator (the auth principal).
    /// * `from` - The owner whose allowance is being spent.
    /// * `to` - The recipient.
    /// * `data` - XDR-encoded [`OperatorTransferPayload`].
    ///
    /// # Errors
    ///
    /// * refer to [`storage::confidential_transfer_from`] errors.
    ///
    /// # Events
    ///
    /// * topics - `["operator_transfer", operator: Address, from: Address, to:
    ///   Address]`
    /// * data - `[r_e, v_tilde, sigma_a, v_aud_r, r_aud_r, v_aud_s, a_aud_s]`
    fn confidential_transfer_from(
        e: &Env,
        operator: Address,
        from: Address,
        to: Address,
        data: Bytes,
    ) {
        storage::confidential_transfer_from(e, &operator, &from, &to, &data);
    }

    /// Escrows an allowance from `account`'s spendable balance into a
    /// `(account, operator)` delegation slot (DESIGN §7.7). Reverts if a
    /// delegation already exists for the pair (single-slot semantics, DESIGN
    /// §6.2).
    ///
    /// # Arguments
    ///
    /// * `e` - Access to the Soroban environment.
    /// * `account` - The delegating owner.
    /// * `operator` - The delegated operator. Must be a registered confidential
    ///   account so its spending key is available for the `dvk` escrow ECDH.
    /// * `expiration_ledger` - Ledger sequence at which the delegation stops
    ///   authorizing spending. The escrowed value persists until
    ///   `revoke_operator`.
    /// * `data` - XDR-encoded [`SetOperatorPayload`].
    ///
    /// # Errors
    ///
    /// * refer to [`storage::set_operator`] errors.
    ///
    /// # Events
    ///
    /// * topics - `["set_operator", account: Address, operator: Address]`
    /// * data - `[expiration_ledger: u32, r_e, sigma, b_tilde, v_aud_s,
    ///   b_aud_s]`
    fn set_operator(
        e: &Env,
        account: Address,
        operator: Address,
        expiration_ledger: u32,
        data: Bytes,
    ) {
        storage::set_operator(e, &account, &operator, expiration_ledger, &data);
    }

    /// Revokes the `(account, operator)` delegation and folds the
    /// remaining escrowed allowance back into `account`'s spendable balance
    /// (DESIGN §7.9). Works for both active and expired-but-not-revoked
    /// delegations.
    ///
    /// # Arguments
    ///
    /// * `e` - Access to the Soroban environment.
    /// * `account` - The owner reclaiming the allowance.
    /// * `operator` - The previously-delegated operator.
    /// * `data` - XDR-encoded [`RevokeOperatorPayload`].
    ///
    /// # Errors
    ///
    /// * refer to [`storage::revoke_operator`] errors.
    ///
    /// # Events
    ///
    /// * topics - `["revoke_operator", account: Address, operator: Address]`
    /// * data - `[r_e, sigma, b_tilde, v_aud_s, b_aud_s]`
    fn revoke_operator(e: &Env, account: Address, operator: Address, data: Bytes) {
        storage::revoke_operator(e, &account, &operator, &data);
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
    /// * refer to [`storage::get_account`] errors.
    fn confidential_balance(e: &Env, account: Address) -> ConfidentialAccount {
        storage::get_account(e, &account)
    }

    /// Returns `true` iff a delegation exists for `(account, operator)`
    /// and the current ledger sequence does not exceed its
    /// `expiration_ledger` (DESIGN §11.3).
    ///
    /// # Arguments
    ///
    /// * `e` - Access to the Soroban environment.
    /// * `account` - The delegating account.
    /// * `operator` - The delegated operator.
    fn is_operator(e: &Env, account: Address, operator: Address) -> bool {
        storage::is_operator(e, &account, &operator)
    }

    /// Returns the [`OperatorDelegation`] stored under `(account,
    /// operator)`. Does not apply the expiry filter; callers that need
    /// spending-authority state should call [`Self::is_operator`].
    ///
    /// # Arguments
    ///
    /// * `e` - Access to the Soroban environment.
    /// * `account` - The delegating account.
    /// * `operator` - The delegated operator.
    ///
    /// # Errors
    ///
    /// * refer to [`storage::get_delegation`] errors.
    fn get_operator(e: &Env, account: Address, operator: Address) -> OperatorDelegation {
        storage::get_delegation(e, &account, &operator)
    }
}

// ################## ERRORS ##################

#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq, PartialOrd, Ord)]
#[repr(u32)]
pub enum WrapperError {
    /// Indicates `account` already has a confidential account registered.
    AccountAlreadyRegistered = 3500,
    /// Indicates the target account is not registered.
    AccountNotRegistered = 3501,
    /// Indicates a public amount argument is negative (DESIGN §3.4).
    NegativeAmount = 3502,
    /// Indicates a delegation already exists for `(account, operator)`
    /// (DESIGN §6.2 *Single-slot semantics*).
    DelegationAlreadyExists = 3503,
    /// Indicates no delegation exists for `(account, operator)`.
    DelegationNotFound = 3504,
    /// Indicates the delegation has expired
    /// (`ledger.sequence() > expiration_ledger`).
    DelegationExpired = 3505,
    /// Indicates the verifier rejected the accompanying proof.
    InvalidProof = 3506,
    /// Indicates the `data` payload could not be decoded into the expected
    /// `…Payload` struct.
    InvalidDataPayload = 3507,
    /// Indicates the wrapper has not been constructed: the SEP-41 token
    /// address is missing.
    TokenNotSet = 3508,
    /// Indicates the wrapper has not been constructed: the verifier
    /// address is missing.
    VerifierNotSet = 3509,
    /// Indicates the wrapper has not been constructed: the auditor
    /// registry address is missing.
    AuditorNotSet = 3510,
    /// Indicates the wrapper has not been constructed: the `wrap` field is
    /// missing.
    WrapNotSet = 3511,
    /// Indicates the `wrap` field has already been set; re-initialization is
    /// forbidden.
    WrapAlreadySet = 3512,
    /// Indicates the SEP-41 token address has already been set;
    /// re-initialization is forbidden.
    TokenAlreadySet = 3513,
}

// ################## CONSTANTS ##################

const DAY_IN_LEDGERS: u32 = 17280;
pub const ACCOUNT_EXTEND_AMOUNT: u32 = 30 * DAY_IN_LEDGERS;
pub const ACCOUNT_TTL_THRESHOLD: u32 = ACCOUNT_EXTEND_AMOUNT - DAY_IN_LEDGERS;
pub const DELEGATION_EXTEND_AMOUNT: u32 = 30 * DAY_IN_LEDGERS;
pub const DELEGATION_TTL_THRESHOLD: u32 = DELEGATION_EXTEND_AMOUNT - DAY_IN_LEDGERS;

/// Domain-separation tag for `address_to_field` (DESIGN §13, `δ_addr`).
pub const DELTA_ADDR: u32 = 1;

// ################## EVENTS ##################

/// Event emitted when a confidential account is registered.
#[contractevent]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Register {
    #[topic]
    pub account: Address,
    pub auditor_id: u32,
}

/// Emits a `Register` event.
pub fn emit_register(e: &Env, account: &Address, auditor_id: u32) {
    Register { account: account.clone(), auditor_id }.publish(e);
}

/// Event emitted when a deposit moves SEP-41 tokens into a confidential
/// receiving balance.
#[contractevent]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Deposit {
    #[topic]
    pub from: Address,
    #[topic]
    pub to: Address,
    pub amount: i128,
}

/// Emits a `Deposit` event.
pub fn emit_deposit(e: &Env, from: &Address, to: &Address, amount: i128) {
    Deposit { from: from.clone(), to: to.clone(), amount }.publish(e);
}

/// Event emitted when a confidential account merges its receiving balance
/// into its spendable balance.
#[contractevent]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Merge {
    #[topic]
    pub account: Address,
}

/// Emits a `Merge` event.
pub fn emit_merge(e: &Env, account: &Address) {
    Merge { account: account.clone() }.publish(e);
}

/// Event emitted on a confidential withdrawal.
#[contractevent]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Withdraw {
    #[topic]
    pub from: Address,
    #[topic]
    pub to: Address,
    pub amount: i128,
    pub r_e: BytesN<64>,
    pub sigma: BytesN<32>,
    pub b_tilde: BytesN<32>,
    pub b_aud_s: BytesN<32>,
}

/// Emits a `Withdraw` event.
#[allow(clippy::too_many_arguments)]
pub fn emit_withdraw(
    e: &Env,
    from: &Address,
    to: &Address,
    amount: i128,
    r_e: &BytesN<64>,
    sigma: &BytesN<32>,
    b_tilde: &BytesN<32>,
    b_aud_s: &BytesN<32>,
) {
    Withdraw {
        from: from.clone(),
        to: to.clone(),
        amount,
        r_e: r_e.clone(),
        sigma: sigma.clone(),
        b_tilde: b_tilde.clone(),
        b_aud_s: b_aud_s.clone(),
    }
    .publish(e);
}

/// Event emitted on a confidential transfer.
#[contractevent]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Transfer {
    #[topic]
    pub from: Address,
    #[topic]
    pub to: Address,
    pub r_e: BytesN<64>,
    pub v_tilde: BytesN<32>,
    pub sigma: BytesN<32>,
    pub b_tilde: BytesN<32>,
    pub v_aud_r: BytesN<32>,
    pub r_aud_r: BytesN<32>,
    pub v_aud_s: BytesN<32>,
    pub b_aud_s: BytesN<32>,
}

/// Emits a `Transfer` event.
#[allow(clippy::too_many_arguments)]
pub fn emit_transfer(
    e: &Env,
    from: &Address,
    to: &Address,
    r_e: &BytesN<64>,
    v_tilde: &BytesN<32>,
    sigma: &BytesN<32>,
    b_tilde: &BytesN<32>,
    v_aud_r: &BytesN<32>,
    r_aud_r: &BytesN<32>,
    v_aud_s: &BytesN<32>,
    b_aud_s: &BytesN<32>,
) {
    Transfer {
        from: from.clone(),
        to: to.clone(),
        r_e: r_e.clone(),
        v_tilde: v_tilde.clone(),
        sigma: sigma.clone(),
        b_tilde: b_tilde.clone(),
        v_aud_r: v_aud_r.clone(),
        r_aud_r: r_aud_r.clone(),
        v_aud_s: v_aud_s.clone(),
        b_aud_s: b_aud_s.clone(),
    }
    .publish(e);
}

/// Event emitted on an operator transfer.
#[contractevent]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct OperatorTransfer {
    #[topic]
    pub operator: Address,
    #[topic]
    pub from: Address,
    #[topic]
    pub to: Address,
    pub r_e: BytesN<64>,
    pub v_tilde: BytesN<32>,
    pub sigma_a: BytesN<32>,
    pub v_aud_r: BytesN<32>,
    pub r_aud_r: BytesN<32>,
    pub v_aud_s: BytesN<32>,
    pub a_aud_s: BytesN<32>,
}

/// Emits an `OperatorTransfer` event.
#[allow(clippy::too_many_arguments)]
pub fn emit_operator_transfer(
    e: &Env,
    operator: &Address,
    from: &Address,
    to: &Address,
    r_e: &BytesN<64>,
    v_tilde: &BytesN<32>,
    sigma_a: &BytesN<32>,
    v_aud_r: &BytesN<32>,
    r_aud_r: &BytesN<32>,
    v_aud_s: &BytesN<32>,
    a_aud_s: &BytesN<32>,
) {
    OperatorTransfer {
        operator: operator.clone(),
        from: from.clone(),
        to: to.clone(),
        r_e: r_e.clone(),
        v_tilde: v_tilde.clone(),
        sigma_a: sigma_a.clone(),
        v_aud_r: v_aud_r.clone(),
        r_aud_r: r_aud_r.clone(),
        v_aud_s: v_aud_s.clone(),
        a_aud_s: a_aud_s.clone(),
    }
    .publish(e);
}

/// Event emitted when an operator is set up.
#[contractevent]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SetOperator {
    #[topic]
    pub account: Address,
    #[topic]
    pub operator: Address,
    pub expiration_ledger: u32,
    pub r_e: BytesN<64>,
    pub sigma: BytesN<32>,
    pub b_tilde: BytesN<32>,
    pub v_aud_s: BytesN<32>,
    pub b_aud_s: BytesN<32>,
}

/// Emits a `SetOperator` event.
#[allow(clippy::too_many_arguments)]
pub fn emit_set_operator(
    e: &Env,
    account: &Address,
    operator: &Address,
    expiration_ledger: u32,
    r_e: &BytesN<64>,
    sigma: &BytesN<32>,
    b_tilde: &BytesN<32>,
    v_aud_s: &BytesN<32>,
    b_aud_s: &BytesN<32>,
) {
    SetOperator {
        account: account.clone(),
        operator: operator.clone(),
        expiration_ledger,
        r_e: r_e.clone(),
        sigma: sigma.clone(),
        b_tilde: b_tilde.clone(),
        v_aud_s: v_aud_s.clone(),
        b_aud_s: b_aud_s.clone(),
    }
    .publish(e);
}

/// Event emitted when an operator is revoked.
#[contractevent]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RevokeOperator {
    #[topic]
    pub account: Address,
    #[topic]
    pub operator: Address,
    pub r_e: BytesN<64>,
    pub sigma: BytesN<32>,
    pub b_tilde: BytesN<32>,
    pub v_aud_s: BytesN<32>,
    pub b_aud_s: BytesN<32>,
}

/// Emits a `RevokeOperator` event.
#[allow(clippy::too_many_arguments)]
pub fn emit_revoke_operator(
    e: &Env,
    account: &Address,
    operator: &Address,
    r_e: &BytesN<64>,
    sigma: &BytesN<32>,
    b_tilde: &BytesN<32>,
    v_aud_s: &BytesN<32>,
    b_aud_s: &BytesN<32>,
) {
    RevokeOperator {
        account: account.clone(),
        operator: operator.clone(),
        r_e: r_e.clone(),
        sigma: sigma.clone(),
        b_tilde: b_tilde.clone(),
        v_aud_s: v_aud_s.clone(),
        b_aud_s: b_aud_s.clone(),
    }
    .publish(e);
}

// ################## ACCOUNT STATE ##################

/// On-chain confidential account record (DESIGN §6.1).
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

/// On-chain operator delegation record (DESIGN §6.2).
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

/// `data` payload for [`ConfidentialTokenWrapper::register`].
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RegisterPayload {
    pub y: Point,
    pub pvk: Point,
    pub proof: Bytes,
}

/// `data` payload for [`ConfidentialTokenWrapper::withdraw`].
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct WithdrawPayload {
    pub c_spend_new: Point,
    pub b_tilde: BytesN<32>,
    pub r_e: Point,
    pub sigma: BytesN<32>,
    pub b_aud_s: BytesN<32>,
    pub proof: Bytes,
}

/// `data` payload for [`ConfidentialTokenWrapper::confidential_transfer`].
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
    pub proof: Bytes,
}

/// `data` payload for
/// [`ConfidentialTokenWrapper::confidential_transfer_from`].
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
    pub proof: Bytes,
}

/// `data` payload for [`ConfidentialTokenWrapper::set_operator`].
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
    pub proof: Bytes,
}

/// `data` payload for [`ConfidentialTokenWrapper::revoke_operator`].
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RevokeOperatorPayload {
    pub c_spend_new: Point,
    pub b_tilde: BytesN<32>,
    pub r_e: Point,
    pub sigma: BytesN<32>,
    pub v_aud_s: BytesN<32>,
    pub b_aud_s: BytesN<32>,
    pub proof: Bytes,
}
