//! # Confidential Token
//!
//! Core token contract that provides confidential balances and transfers for
//! SEP-41 assets. Balances are stored as Pedersen commitments on the Grumpkin
//! curve; every state-changing operation that consumes private state is
//! accompanied by an UltraHonk proof that the contract verifies via a separate
//! verifier contract. Auditor keys are read from a separate registry contract
//! and dual auditor ciphertexts are emitted in each transfer event. See
//! [`docs/DESIGN.md`] for the full specification.
//!
//! # ⚠️ Not Production Ready
//!
//! This module depends on
//! [`verifier::ConfidentialVerifier::verify_proof`], whose UltraHonk
//! backend (`rs-soroban-ultrahonk`) is still under development and has not been
//! audited. Do **not** deploy a contract built on this trait to mainnet or any
//! environment that handles real value. See the verifier module-level warning
//! for details.
//!
//! ## What This Crate Provides
//!
//! - The [`ConfidentialToken`] trait — eleven entry points (register, deposit,
//!   merge, withdraw, two transfer variants, two spender management methods,
//!   and three read methods) with default bodies that delegate to the matching
//!   free functions in [`storage`].
//! - The on-chain account state types [`ConfidentialAccount`] and
//!   [`SpenderDelegation`] (DESIGN §6).
//! - The six XDR payload types carried in the `data: Bytes` parameter (DESIGN
//!   §11).
//! - Storage helpers and operation-level orchestration under [`storage`].
//!
//! ## Public-Input Encoding
//!
//! The contract assembles the public-input blob handed to the verifier by
//! concatenating canonical 32-byte big-endian `Bn254Fr` representatives in
//! the order specified by each circuit's table in DESIGN §7. Grumpkin
//! points contribute two consecutive 32-byte limbs (`x` then `y`). Public
//! `i128` amounts are zero-padded to 32 bytes. The encoding is positional
//! (no length prefix); any divergence from the order the prover used will
//! cause verification to fail.
//!
//! ## Contract Binding
//!
//! Every owner-initiated proof references a `addr_f` field, computed once at
//! construction as `Poseidon2(δ_addr, lo, hi)` over the contract's own
//! address (DESIGN §2.7, §3.5) and stored in instance storage.
//!
//! ## Underlying Token Requirements
//!
//! The contract assumes the underlying SEP-41 token has **exact-transfer
//! semantics**: a successful `transfer(from, to, amount)` moves exactly
//! `amount` units between the two accounts, with no fees deducted in transit
//! and no rebasing applied. [`storage::deposit`] credits the
//! confidential receiving balance with `amount · G` after the SEP-41
//! transfer, and [`storage::withdraw`] debits the confidential
//! spendable balance by `amount` before transferring the same amount out;
//! neither call re-measures the contract's own balance. With a
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
//! Deploying this contract over any other token implementation is the
//! deployer's responsibility — verify that `transfer` does not skim a fee
//! or otherwise diverge from exact-transfer semantics before doing so.

pub mod auditor;
pub mod compliance;
pub mod storage;
pub mod verifier;

#[cfg(test)]
mod test;

use soroban_sdk::{
    contracterror, contractevent, contracttrait, Address, Bytes, BytesN, Env, IntoVal, Val,
};
pub use storage::{
    ConfidentialAccount, ConfidentialTokenStorageKey, RegisterData, RegisterPayload,
    RevokeSpenderData, RevokeSpenderPayload, SetSpenderData, SetSpenderPayload, SpenderDelegation,
    SpenderTransferData, SpenderTransferPayload, TransferData, TransferPayload, WithdrawData,
    WithdrawPayload,
};

/// Lifecycle hooks invoked by [`ConfidentialToken`] at each
/// state-changing entry point, after auth and payload decode, before the
/// state mutation runs.
///
/// Hooks are general-purpose extension points: compliance gates (freeze /
/// allowlist / sanctions screen via panic), audit mirroring to a separate
/// log contract, per-account rate limiting, or any other synchronous
/// concern that must happen atomically with the token op. For pure
/// observability, prefer subscribing to the token's events instead.
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
    /// `payload: Val` carries an [`SpenderTransferPayload`].
    fn on_spender_transfer(e: &Env, spender: &Address, from: &Address, to: &Address, payload: Val) {
    }

    /// Invoked after `set_spender`'s auth and decode. `payload: Val`
    /// carries a [`SetSpenderPayload`].
    fn on_set_spender(
        e: &Env,
        account: &Address,
        spender: &Address,
        live_until_ledger: u32,
        payload: Val,
    ) {
    }

    /// Invoked after `revoke_spender`'s auth and decode. `payload: Val`
    /// carries a [`RevokeSpenderPayload`].
    fn on_revoke_spender(e: &Env, account: &Address, spender: &Address, payload: Val) {}
}

/// Zero-cost [`Hooks`] implementation whose every callback is an empty
/// no-op. Wire this as `type Hooks = NoHooks;` for deployments that need
/// no extension behaviour.
pub struct NoHooks;

impl Hooks for NoHooks {}

/// Trait for the confidential token.
///
/// Each entry point has a default body that authorizes the caller,
/// XDR-decodes the `data` envelope, runs the matching [`Hooks`] callback,
/// then delegates to a matching free function in [`storage`]. The
/// storage layer loads every trusted-state public input from on-chain
/// state (never from caller-controlled bytes), assembles the public-input
/// blob in the prescribed order, calls
/// [`ConfidentialVerifier::verify_proof`] cross-contract, applies the
/// post-verification state mutation, and emits the event.
///
/// [`ConfidentialVerifier::verify_proof`]: verifier::ConfidentialVerifier::verify_proof
#[contracttrait]
pub trait ConfidentialToken {
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
    /// * refer to [`storage::register`] errors.
    ///
    /// # Events
    ///
    /// * topics - `["register", account: Address]`
    /// * data - `[auditor_id: u32]`
    fn register(e: &Env, account: Address, auditor_id: u32, data: Bytes) {
        account.require_auth();

        let decoded: RegisterData = storage::decode_data(e, &data);
        Self::Hooks::on_register(e, &account, decoded.payload.clone().into_val(e));
        storage::register(e, &account, auditor_id, &decoded.payload, &decoded.proof);
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
    /// * refer to [`storage::deposit`] errors.
    ///
    /// # Events
    ///
    /// * topics - `["deposit", from: Address, to: Address]`
    /// * data - `[amount: i128]`
    fn deposit(e: &Env, from: Address, to: Address, amount: i128) {
        from.require_auth();

        Self::Hooks::on_deposit(e, &from, &to, amount);
        storage::deposit(e, &from, &to, amount);
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
    /// * refer to [`storage::merge`] errors.
    ///
    /// # Events
    ///
    /// * topics - `["merge", account: Address]`
    /// * data - `[]`
    fn merge(e: &Env, account: Address) {
        account.require_auth();

        Self::Hooks::on_merge(e, &account);
        storage::merge(e, &account);
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
    /// * refer to [`storage::withdraw`] errors.
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
        storage::withdraw(e, &from, &to, amount, &decoded.payload, &decoded.proof);
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
    /// * refer to [`storage::confidential_transfer`] errors.
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
        storage::confidential_transfer(e, &from, &to, &decoded.payload, &decoded.proof);
    }

    /// Spends from `from`'s allowance escrowed to `spender`, transferring
    /// confidentially to `to`. The owner's authorization was
    /// granted at `set_spender` and persists in the on-chain delegation
    /// entry; only the spender authorizes this call.
    ///
    /// # Arguments
    ///
    /// * `e` - Access to the Soroban environment.
    /// * `spender` - The delegated spender (the auth principal).
    /// * `from` - The owner whose allowance is being spent.
    /// * `to` - The recipient.
    /// * `data` - XDR-encoded [`SpenderTransferData`].
    ///
    /// # Errors
    ///
    /// * refer to [`storage::decode_data`] errors.
    /// * refer to [`storage::confidential_transfer_from`] errors.
    ///
    /// # Events
    ///
    /// * topics - `["spender_transfer", spender: Address, from: Address, to:
    ///   Address]`
    /// * data - `[r_e, v_tilde, sigma_a, v_aud_r, r_aud_r, v_aud_s, a_aud_s]`
    fn confidential_transfer_from(
        e: &Env,
        spender: Address,
        from: Address,
        to: Address,
        data: Bytes,
    ) {
        spender.require_auth();

        let decoded: SpenderTransferData = storage::decode_data(e, &data);
        Self::Hooks::on_spender_transfer(
            e,
            &spender,
            &from,
            &to,
            decoded.payload.clone().into_val(e),
        );
        storage::confidential_transfer_from(
            e,
            &spender,
            &from,
            &to,
            &decoded.payload,
            &decoded.proof,
        );
    }

    /// Escrows an allowance from `account`'s spendable balance and delegates it
    /// to `spender`. Reverts if a delegation already exists for the `(account,
    /// spender)` pair.
    ///
    /// # Arguments
    ///
    /// * `e` - Access to the Soroban environment.
    /// * `account` - The delegating owner.
    /// * `spender` - The delegated spender. Must be a registered confidential
    ///   account so its spending key is available for the `dvk` escrow ECDH.
    /// * `live_until_ledger` - The ledger number at which the delegation
    ///   expires. Spending is authorized while `ledger.sequence() <=
    ///   live_until_ledger`. The escrowed value persists until
    ///   `revoke_spender`.
    /// * `data` - XDR-encoded [`SetSpenderData`].
    ///
    /// # Errors
    ///
    /// * refer to [`storage::decode_data`] errors.
    /// * refer to [`storage::set_spender`] errors.
    ///
    /// # Events
    ///
    /// * topics - `["set_spender", account: Address, spender: Address]`
    /// * data - `[live_until_ledger: u32, r_e, sigma, b_tilde, v_aud_s,
    ///   b_aud_s]`
    fn set_spender(
        e: &Env,
        account: Address,
        spender: Address,
        live_until_ledger: u32,
        data: Bytes,
    ) {
        account.require_auth();

        let decoded: SetSpenderData = storage::decode_data(e, &data);
        Self::Hooks::on_set_spender(
            e,
            &account,
            &spender,
            live_until_ledger,
            decoded.payload.clone().into_val(e),
        );
        storage::set_spender(
            e,
            &account,
            &spender,
            live_until_ledger,
            &decoded.payload,
            &decoded.proof,
        );
    }

    /// Revokes the `(account, spender)` delegation and folds the
    /// remaining escrowed allowance back into `account`'s spendable balance.
    /// Works for both active and expired-but-not-revoked delegations.
    ///
    /// # Arguments
    ///
    /// * `e` - Access to the Soroban environment.
    /// * `account` - The owner reclaiming the allowance.
    /// * `spender` - The previously-delegated spender.
    /// * `data` - XDR-encoded [`RevokeSpenderData`].
    ///
    /// # Errors
    ///
    /// * refer to [`storage::decode_data`] errors.
    /// * refer to [`storage::revoke_spender`] errors.
    ///
    /// # Events
    ///
    /// * topics - `["revoke_spender", account: Address, spender: Address]`
    /// * data - `[r_e, sigma, b_tilde, v_aud_s, b_aud_s]`
    fn revoke_spender(e: &Env, account: Address, spender: Address, data: Bytes) {
        account.require_auth();

        let decoded: RevokeSpenderData = storage::decode_data(e, &data);
        Self::Hooks::on_revoke_spender(e, &account, &spender, decoded.payload.clone().into_val(e));
        storage::revoke_spender(e, &account, &spender, &decoded.payload, &decoded.proof);
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

    /// Returns `true` iff a delegation exists for `(account, spender)`
    /// and is still live (`ledger.sequence() <= live_until_ledger`).
    ///
    /// # Arguments
    ///
    /// * `e` - Access to the Soroban environment.
    /// * `account` - The delegating account.
    /// * `spender` - The delegated spender.
    fn is_spender(e: &Env, account: Address, spender: Address) -> bool {
        storage::is_spender(e, &account, &spender)
    }

    /// Returns the [`SpenderDelegation`] stored under `(account,
    /// spender)`.
    ///
    /// # Arguments
    ///
    /// * `e` - Access to the Soroban environment.
    /// * `account` - The delegating account.
    /// * `spender` - The delegated spender.
    ///
    /// # Errors
    ///
    /// * refer to [`storage::get_spender_delegation`] errors.
    fn get_spender_delegation(e: &Env, account: Address, spender: Address) -> SpenderDelegation {
        storage::get_spender_delegation(e, &account, &spender)
    }
}

// ################## ERRORS ##################

#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq, PartialOrd, Ord)]
#[repr(u32)]
pub enum ConfidentialTokenError {
    /// Indicates `account` already has a confidential account registered.
    AccountAlreadyRegistered = 3500,
    /// Indicates the target account is not registered.
    AccountNotRegistered = 3501,
    /// Indicates a public amount argument is negative.
    NegativeAmount = 3502,
    /// Indicates a delegation already exists for `(account, spender)`.
    DelegationAlreadyExists = 3503,
    /// Indicates no delegation exists for `(account, spender)`.
    DelegationNotFound = 3504,
    /// Indicates the delegation has expired
    /// (`ledger.sequence() > live_until_ledger`).
    DelegationExpired = 3505,
    /// Indicates the verifier rejected the accompanying proof.
    InvalidProof = 3506,
    /// Indicates the `data` payload could not be decoded into the expected
    /// `…Payload` struct.
    InvalidData = 3507,
    /// Indicates the contract has not been constructed: the SEP-41 token
    /// address is missing.
    UnderlyingAssetNotSet = 3508,
    /// Indicates the contract has not been constructed: the verifier
    /// address is missing.
    VerifierNotSet = 3509,
    /// Indicates the contract has not been constructed: the auditor
    /// registry address is missing.
    AuditorNotSet = 3510,
    /// Indicates the contract has not been constructed: the `addr_f` field is
    /// missing.
    AddressAsFieldNotSet = 3511,
    /// Indicates the `addr_f` field has already been set; re-initialization is
    /// forbidden.
    AddressAsFieldAlreadySet = 3512,
    /// Indicates the SEP-41 token address has already been set;
    /// re-initialization is forbidden.
    UnderlyingAssetAlreadySet = 3513,
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

/// Event emitted on an spender transfer.
#[contractevent]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SpenderTransfer {
    #[topic]
    pub spender: Address,
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

/// Emits an `SpenderTransfer` event.
#[allow(clippy::too_many_arguments)]
pub fn emit_spender_transfer(
    e: &Env,
    spender: &Address,
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
    SpenderTransfer {
        spender: spender.clone(),
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

/// Event emitted when an spender is set up.
#[contractevent]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SetSpender {
    #[topic]
    pub account: Address,
    #[topic]
    pub spender: Address,
    pub live_until_ledger: u32,
    pub r_e: BytesN<64>,
    pub sigma: BytesN<32>,
    pub b_tilde: BytesN<32>,
    pub v_aud_s: BytesN<32>,
    pub b_aud_s: BytesN<32>,
}

/// Emits a `SetSpender` event.
#[allow(clippy::too_many_arguments)]
pub fn emit_set_spender(
    e: &Env,
    account: &Address,
    spender: &Address,
    live_until_ledger: u32,
    r_e: &BytesN<64>,
    sigma: &BytesN<32>,
    b_tilde: &BytesN<32>,
    v_aud_s: &BytesN<32>,
    b_aud_s: &BytesN<32>,
) {
    SetSpender {
        account: account.clone(),
        spender: spender.clone(),
        live_until_ledger,
        r_e: r_e.clone(),
        sigma: sigma.clone(),
        b_tilde: b_tilde.clone(),
        v_aud_s: v_aud_s.clone(),
        b_aud_s: b_aud_s.clone(),
    }
    .publish(e);
}

/// Event emitted when an spender is revoked.
#[contractevent]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RevokeSpender {
    #[topic]
    pub account: Address,
    #[topic]
    pub spender: Address,
    pub r_e: BytesN<64>,
    pub sigma: BytesN<32>,
    pub b_tilde: BytesN<32>,
    pub v_aud_s: BytesN<32>,
    pub b_aud_s: BytesN<32>,
}

/// Emits a `RevokeSpender` event.
#[allow(clippy::too_many_arguments)]
pub fn emit_revoke_spender(
    e: &Env,
    account: &Address,
    spender: &Address,
    r_e: &BytesN<64>,
    sigma: &BytesN<32>,
    b_tilde: &BytesN<32>,
    v_aud_s: &BytesN<32>,
    b_aud_s: &BytesN<32>,
) {
    RevokeSpender {
        account: account.clone(),
        spender: spender.clone(),
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
pub struct UnderlyingAssetSet {
    pub underlying_asset: Address,
}

/// Emits an `UnderlyingAssetSet` event.
pub fn emit_underlying_asset_set(e: &Env, underlying_asset: &Address) {
    UnderlyingAssetSet { underlying_asset: underlying_asset.clone() }.publish(e);
}

/// Event emitted when the verifier registry contract address is set or
/// rotated. May fire more than once over the lifetime of the contract.
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
/// rotated. May fire more than once over the lifetime of the contract.
#[contractevent]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AuditorSet {
    pub auditor: Address,
}

/// Emits an `AuditorSet` event.
pub fn emit_auditor_set(e: &Env, auditor: &Address) {
    AuditorSet { auditor: auditor.clone() }.publish(e);
}

/// Event emitted when the contract's compressed `addr_f` field is computed and
/// stored. Expected to fire exactly once, from the contract's constructor.
#[contractevent]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AddressAsFieldSet {
    pub address_as_field: BytesN<32>,
}

/// Emits an `AddressAsFieldSet` event.
pub fn emit_address_as_field_set(e: &Env, address_as_field: &BytesN<32>) {
    AddressAsFieldSet { address_as_field: address_as_field.clone() }.publish(e);
}
