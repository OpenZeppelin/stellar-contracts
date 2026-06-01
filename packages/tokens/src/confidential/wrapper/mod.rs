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
//! ## Underlying Token Requirements
//!
//! The wrapper assumes the underlying SEP-41 token has **exact-transfer
//! semantics**: a successful `transfer(from, to, amount)` moves exactly
//! `amount` units between the two accounts, with no fees deducted in transit
//! and no rebasing applied. [`storage::deposit_no_auth`] credits the
//! confidential receiving balance with `amount · G` after the SEP-41
//! transfer, and [`storage::withdraw_no_auth`] debits the confidential
//! spendable balance by `amount` before transferring the same amount out;
//! neither call re-measures the wrapper's own balance. With a
//! fee-on-transfer, rebasing, or otherwise malicious token implementation,
//! the confidential ledger would drift from the on-chain reserves —
//! credit a higher amount than was actually received, or pay out less than
//! was debited.
//!
//! Tokens that are SEP-41 compliant are supported, for example:
//!
//! - the Stellar Asset Contract (SAC), or
//! - OpenZeppelin's [`fungible`](crate::fungible) token.
//!
//! Deploying the wrapper over any other token implementation is the
//! deployer's responsibility — verify that `transfer` does not skim a fee
//! or otherwise diverge from exact-transfer semantics before doing so.

pub mod compliance;
pub mod storage;

#[cfg(test)]
mod test;

use soroban_sdk::{
    contracterror, contractevent, contracttrait, Address, Bytes, BytesN, Env, IntoVal, Val,
};
pub use storage::{
    compute_wrap, ConfidentialAccount, OperatorDelegation, OperatorTransferData,
    OperatorTransferPayload, RegisterData, RegisterPayload, RevokeOperatorData,
    RevokeOperatorPayload, SetOperatorData, SetOperatorPayload, TransferData, TransferPayload,
    WithdrawData, WithdrawPayload, WrapperStorageKey,
};

/// Lifecycle hooks invoked by [`ConfidentialTokenWrapper`] at each
/// state-changing entry point, after auth and payload decode, before the
/// state mutation runs.
///
/// Hooks are general-purpose extension points: compliance gates (freeze /
/// allowlist / sanctions screen via panic), audit mirroring to a separate
/// log contract, per-account rate limiting, or any other synchronous
/// concern that must happen atomically with the wrapper op. For pure
/// observability, prefer subscribing to the wrapper's events instead.
///
/// For ops that carry a `data: Bytes` argument, the last parameter
/// `payload` on the matching hook is the decoded operation payload as
/// `Val` (e.g. [`TransferPayload`] for [`Self::on_transfer`]); the hook
/// reconstructs the typed value via `T::from_val(e, &payload)`. The proof
/// is not forwarded. The proofless ops [`Self::on_deposit`] and
/// [`Self::on_merge`] receive no `payload`.
///
/// Default bodies are empty no-ops — overriding only the methods relevant
/// to a given deployment is the expected pattern.
#[allow(unused_variables)]
pub trait Hooks {
    /// Invoked after `register`'s auth and payload decode, before account
    /// creation. `payload: Val` carries a [`RegisterPayload`].
    fn on_register(e: &Env, account: &Address, payload: Val) {}

    /// Invoked after `deposit`'s auth, before SEP-41 transfer and balance
    /// update.
    fn on_deposit(e: &Env, from: &Address, to: &Address, amount: i128) {}

    /// Invoked after `merge`'s auth, before the receiving→spendable fold.
    fn on_merge(e: &Env, account: &Address) {}

    /// Invoked after `withdraw`'s auth and decode, before proof verification.
    /// `payload: Val` carries a [`WithdrawPayload`].
    fn on_withdraw(e: &Env, from: &Address, to: &Address, amount: i128, payload: Val) {}

    /// Invoked after `confidential_transfer`'s auth and decode. `payload:
    /// Val` carries a [`TransferPayload`].
    fn on_transfer(e: &Env, from: &Address, to: &Address, payload: Val) {}

    /// Invoked after `confidential_transfer_from`'s auth and decode.
    /// `payload: Val` carries an [`OperatorTransferPayload`].
    fn on_operator_transfer(
        e: &Env,
        operator: &Address,
        from: &Address,
        to: &Address,
        payload: Val,
    ) {
    }

    /// Invoked after `set_operator`'s auth and decode. `payload: Val`
    /// carries a [`SetOperatorPayload`].
    fn on_set_operator(
        e: &Env,
        account: &Address,
        operator: &Address,
        expiration_ledger: u32,
        payload: Val,
    ) {
    }

    /// Invoked after `revoke_operator`'s auth and decode. `payload: Val`
    /// carries a [`RevokeOperatorPayload`].
    fn on_revoke_operator(e: &Env, account: &Address, operator: &Address, payload: Val) {}
}

/// Zero-cost [`Hooks`] implementation whose every callback is an empty
/// no-op. Wire this as `type Hooks = NoHooks;` for deployments that need
/// no extension behaviour.
pub struct NoHooks;

impl Hooks for NoHooks {}

/// Trait for the confidential token wrapper.
///
/// Each entry point has a default body that authorizes the caller,
/// XDR-decodes the `data` envelope, runs the matching [`Hooks`] callback,
/// then delegates to a `*_no_auth` free function in [`storage`]. The
/// storage layer loads every trusted-state public input from on-chain
/// state (never from caller-controlled bytes), assembles the public-input
/// blob in the prescribed order, calls
/// [`ConfidentialVerifier::verify_proof`] cross-contract, applies the
/// post-verification state mutation, and emits the event.
///
/// [`ConfidentialVerifier::verify_proof`]: super::verifier::ConfidentialVerifier::verify_proof
#[contracttrait]
pub trait ConfidentialTokenWrapper {
    /// Lifecycle hook impl invoked at each state-changing entry point.
    /// Use [`NoHooks`] for deployments that need no extension behaviour.
    type Hooks: Hooks;

    /// Registers a confidential account under `account`.
    ///
    /// # Arguments
    ///
    /// * `e` - Access to the Soroban environment.
    /// * `account` - The owner address being registered.
    /// * `auditor_id` - The auditor key index this account commits to.
    /// * `data` - XDR-encoded [`RegisterData`].
    ///
    /// # Errors
    ///
    /// * refer to [`storage::decode_data`] errors.
    /// * refer to [`storage::register_no_auth`] errors.
    ///
    /// # Events
    ///
    /// * topics - `["register", account: Address]`
    /// * data - `[auditor_id: u32]`
    fn register(e: &Env, account: Address, auditor_id: u32, data: Bytes) {
        account.require_auth();

        let decoded: RegisterData = storage::decode_data(e, &data);
        Self::Hooks::on_register(e, &account, decoded.payload.clone().into_val(e));
        storage::register_no_auth(e, &account, auditor_id, &decoded.payload, &decoded.proof);
    }

    /// Deposits `amount` units of the underlying SEP-41 token from `from`
    /// into `to`'s confidential receiving balance.
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
    /// * refer to [`storage::deposit_no_auth`] errors.
    ///
    /// # Events
    ///
    /// * topics - `["deposit", from: Address, to: Address]`
    /// * data - `[amount: i128]`
    fn deposit(e: &Env, from: Address, to: Address, amount: i128) {
        from.require_auth();

        Self::Hooks::on_deposit(e, &from, &to, amount);
        storage::deposit_no_auth(e, &from, &to, amount);
    }

    /// Folds `account.receiving_balance` into `account.spendable_balance`
    /// and resets the receiving storage entry to the identity.
    ///
    /// No proof is required; correctness follows from the homomorphic
    /// property of Pedersen commitments. Only the account holder can
    /// authorize a merge — third parties cannot weaponize it.
    ///
    /// # Arguments
    ///
    /// * `e` - Access to the Soroban environment.
    /// * `account` - The owner address.
    ///
    /// # Errors
    ///
    /// * refer to [`storage::merge_no_auth`] errors.
    ///
    /// # Events
    ///
    /// * topics - `["merge", account: Address]`
    /// * data - `[]`
    fn merge(e: &Env, account: Address) {
        account.require_auth();

        Self::Hooks::on_merge(e, &account);
        storage::merge_no_auth(e, &account);
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
    /// * `data` - XDR-encoded [`WithdrawData`].
    ///
    /// # Errors
    ///
    /// * refer to [`storage::decode_data`] errors.
    /// * refer to [`storage::withdraw_no_auth`] errors.
    ///
    /// # Events
    ///
    /// * topics - `["withdraw", from: Address, to: Address]`
    /// * data - `[amount: i128, r_e: BytesN<64>, sigma: BytesN<32>, b_tilde:
    ///   BytesN<32>, b_aud_s: BytesN<32>]`
    fn withdraw(e: &Env, from: Address, to: Address, amount: i128, data: Bytes) {
        from.require_auth();

        let decoded: WithdrawData = storage::decode_data(e, &data);
        Self::Hooks::on_withdraw(e, &from, &to, amount, decoded.payload.clone().into_val(e));
        storage::withdraw_no_auth(e, &from, &to, amount, &decoded.payload, &decoded.proof);
    }

    /// Sends a confidential transfer from `from` to `to`.
    ///
    /// # Arguments
    ///
    /// * `e` - Access to the Soroban environment.
    /// * `from` - The sender.
    /// * `to` - The recipient.
    /// * `data` - XDR-encoded [`TransferData`].
    ///
    /// # Errors
    ///
    /// * refer to [`storage::decode_data`] errors.
    /// * refer to [`storage::confidential_transfer_no_auth`] errors.
    ///
    /// # Events
    ///
    /// * topics - `["transfer", from: Address, to: Address]`
    /// * data - `[r_e, v_tilde, sigma, b_tilde, v_aud_r, r_aud_r, v_aud_s,
    ///   b_aud_s]`
    fn confidential_transfer(e: &Env, from: Address, to: Address, data: Bytes) {
        from.require_auth();

        let decoded: TransferData = storage::decode_data(e, &data);
        Self::Hooks::on_transfer(e, &from, &to, decoded.payload.clone().into_val(e));
        storage::confidential_transfer_no_auth(e, &from, &to, &decoded.payload, &decoded.proof);
    }

    /// Spends from `from`'s allowance escrowed to `operator`, transferring
    /// confidentially to `to`. The owner's authorization was
    /// granted at `set_operator` and persists in the on-chain delegation
    /// entry; only the operator authorizes this call.
    ///
    /// # Arguments
    ///
    /// * `e` - Access to the Soroban environment.
    /// * `operator` - The delegated operator (the auth principal).
    /// * `from` - The owner whose allowance is being spent.
    /// * `to` - The recipient.
    /// * `data` - XDR-encoded [`OperatorTransferData`].
    ///
    /// # Errors
    ///
    /// * refer to [`storage::decode_data`] errors.
    /// * refer to [`storage::confidential_transfer_from_no_auth`] errors.
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
        operator.require_auth();

        let decoded: OperatorTransferData = storage::decode_data(e, &data);
        Self::Hooks::on_operator_transfer(
            e,
            &operator,
            &from,
            &to,
            decoded.payload.clone().into_val(e),
        );
        storage::confidential_transfer_from_no_auth(
            e,
            &operator,
            &from,
            &to,
            &decoded.payload,
            &decoded.proof,
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
    ///   authorizing spending. The escrowed value persists until
    ///   `revoke_operator`.
    /// * `data` - XDR-encoded [`SetOperatorData`].
    ///
    /// # Errors
    ///
    /// * refer to [`storage::decode_data`] errors.
    /// * refer to [`storage::set_operator_no_auth`] errors.
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
        account.require_auth();

        let decoded: SetOperatorData = storage::decode_data(e, &data);
        Self::Hooks::on_set_operator(
            e,
            &account,
            &operator,
            expiration_ledger,
            decoded.payload.clone().into_val(e),
        );
        storage::set_operator_no_auth(
            e,
            &account,
            &operator,
            expiration_ledger,
            &decoded.payload,
            &decoded.proof,
        );
    }

    /// Revokes the `(account, operator)` delegation and folds the
    /// remaining escrowed allowance back into `account`'s spendable balance.
    /// Works for both active and expired-but-not-revoked delegations.
    ///
    /// # Arguments
    ///
    /// * `e` - Access to the Soroban environment.
    /// * `account` - The owner reclaiming the allowance.
    /// * `operator` - The previously-delegated operator.
    /// * `data` - XDR-encoded [`RevokeOperatorData`].
    ///
    /// # Errors
    ///
    /// * refer to [`storage::decode_data`] errors.
    /// * refer to [`storage::revoke_operator_no_auth`] errors.
    ///
    /// # Events
    ///
    /// * topics - `["revoke_operator", account: Address, operator: Address]`
    /// * data - `[r_e, sigma, b_tilde, v_aud_s, b_aud_s]`
    fn revoke_operator(e: &Env, account: Address, operator: Address, data: Bytes) {
        account.require_auth();

        let decoded: RevokeOperatorData = storage::decode_data(e, &data);
        Self::Hooks::on_revoke_operator(
            e,
            &account,
            &operator,
            decoded.payload.clone().into_val(e),
        );
        storage::revoke_operator_no_auth(e, &account, &operator, &decoded.payload, &decoded.proof);
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
    /// `expiration_ledger`.
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
    /// operator)`.
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
    /// Indicates a public amount argument is negative.
    NegativeAmount = 3502,
    /// Indicates a delegation already exists for `(account, operator)`.
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
    InvalidData = 3507,
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

/// Domain-separation tag for `address_to_field`.
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

/// Event emitted when the SEP-41 token address is set. Expected to fire
/// exactly once, from the contract's constructor.
#[contractevent]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TokenSet {
    pub token: Address,
}

/// Emits a `TokenSet` event.
pub fn emit_token_set(e: &Env, token: &Address) {
    TokenSet { token: token.clone() }.publish(e);
}

/// Event emitted when the verifier registry contract address is set or
/// rotated. May fire more than once over the lifetime of the wrapper.
#[contractevent]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct VerifierSet {
    pub verifier: Address,
}

/// Emits a `VerifierSet` event.
pub fn emit_verifier_set(e: &Env, verifier: &Address) {
    VerifierSet { verifier: verifier.clone() }.publish(e);
}

/// Event emitted when the auditor registry contract address is set or
/// rotated. May fire more than once over the lifetime of the wrapper.
#[contractevent]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AuditorSet {
    pub auditor: Address,
}

/// Emits an `AuditorSet` event.
pub fn emit_auditor_set(e: &Env, auditor: &Address) {
    AuditorSet { auditor: auditor.clone() }.publish(e);
}

/// Event emitted when the wrapper's compressed `wrap` field is computed and
/// stored. Expected to fire exactly once, from the contract's constructor.
#[contractevent]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct WrapSet {
    pub wrap: BytesN<32>,
}

/// Emits a `WrapSet` event.
pub fn emit_wrap_set(e: &Env, wrap: &BytesN<32>) {
    WrapSet { wrap: wrap.clone() }.publish(e);
}
