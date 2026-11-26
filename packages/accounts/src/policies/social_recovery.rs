//! # Social Recovery Policy Module
//!
//! This policy lets a quorum of guardians restore access to a smart account
//! whose owner lost their keys. One deployment serves any number of smart
//! accounts; all state is keyed by smart account and context rule ID.
//!
//! ## Setup
//!
//! The owner adds a recovery rule scoped to the smart account itself, with no
//! signers and this policy as its only policy:
//!
//! ```rust,ignore
//! account.add_context_rule(
//!     ContextRuleType::CallContract(account),
//!     "recovery",
//!     None,
//!     vec![],
//!     map![(policy, SocialRecoveryAccountParams {
//!         guardians: vec![bob, carol, dave],
//!         threshold: 2,
//!         delay_ledgers: 17_280,   // ~1 day
//!         expiry_ledgers: 120_960, // ~7 days
//!     })],
//! );
//! ```
//!
//! A recovery adds a `Default` rule named [`RECOVERED_RULE_NAME`], with no
//! `valid_until` and no policies, whose signers the guardians approve. With no
//! policies, the smart account requires every signer of that rule to sign
//! through it.
//!
//! ## Recovery Flow
//!
//! 1. A guardian calls [`approve`] with the current epoch and a new signer set.
//!    The first approval opens an attempt for that exact, ordered set, and the
//!    approvals must reach `threshold` within `expiry_ledgers` of it.
//! 2. The approval that reaches `threshold` starts the delay. The attempt is
//!    executable from `delay_ledgers` after that approval, for
//!    `expiry_ledgers`.
//! 3. Anyone calls `add_context_rule(Default, RECOVERED_RULE_NAME, None,
//!    signers, {})` on the smart account with the approved signers, selecting
//!    the recovery rule in the `AuthPayload` with no signatures. The smart
//!    account defers to [`enforce`], which admits exactly that call, removes
//!    the attempt and increments the epoch.
//! 4. The new owner removes the old rule through the new one.
//!
//! The epoch of a recovery rule increases on [`install`], [`set_config`],
//! [`cancel`] and every completion. Approvals name the epoch, so each increment
//! invalidates every open attempt and every signed approval not yet submitted.
//!
//! ## Notes
//!
//! - Attempts are keyed by the ordered signer set. Guardians approve the
//!   `signers` emitted by the opening `RecoveryApprovalRecorded` event.
//! - A completion adds one rule and removes nothing. The old rule stays live
//!   until the new owner removes it.
//! - A `RecoveryAuthorized` event means this policy authorized a completion,
//!   not that a rule was added. A direct [`enforce`] call under the smart
//!   account's authorization retires the attempt without adding a rule. A
//!   recovery is complete only when the smart account emits
//!   `context_rule_added` in the same transaction.
//! - A `valid_until` on the recovery rule turns recovery off after that ledger.
//!
//! # Security Warning
//!
//! - **Lost keys only.** Anyone holding the owner's keys can uninstall the
//!   policy, cancel any attempt and replace the guardians immediately. This
//!   policy does not protect against key theft.
//! - **Veto authority.** [`cancel`] requires the smart account's authorization
//!   for a context on this policy contract, which any rule admitting such a
//!   context can give. Among the shipped policies, that is a `Default` rule
//!   without policies or with a threshold policy, or a `CallContract` rule for
//!   this policy contract without `spending_limit`. A `spending_limit` session
//!   key cannot cancel: that policy installs only on `CallContract` rules and
//!   rejects every context except `transfer`. The cancel has no deadline, but
//!   an attacker can complete and remove the old rule in one transaction, so in
//!   practice the veto window is `delay_ledgers`.
//! - **Rules on this policy contract.** The authorization that passes
//!   [`cancel`] also passes [`set_config`] and [`uninstall`]. A rule admitting
//!   calls on this policy contract can replace the guardians and then recover
//!   the account, so it holds the owner's full authority unless a policy on
//!   that rule limits it to [`cancel`].
//! - **Guardians.** A guardian quorum acting together is indistinguishable from
//!   the owner, except through the owner's cancel.
//! - **Signers.** This policy cannot tell a genuine signer from a malicious
//!   verifier, or from a delegated signer that authorizes back into the smart
//!   account, and either one lets anyone use the recovered rule. Guardians
//!   check each signer before approving: an `External` verifier must be a known
//!   deployment and the key must parse in its format, and a `Delegated` address
//!   must be a G-address or a known smart account build.
//! - **Upgrades.** Whoever can upgrade a deployment of this policy controls
//!   recovery for every smart account that uses it.
use soroban_sdk::{
    auth::{Context, ContractContext},
    contracterror, contractevent, contracttype, panic_with_error,
    xdr::ToXdr,
    Address, BytesN, Env, Map, String, Symbol, TryFromVal, Val, Vec,
};

use crate::smart_account::{
    validate_no_canonical_duplicates, ContextRule, ContextRuleType, Signer, MAX_EXTERNAL_KEY_SIZE,
    MAX_SIGNERS,
};

/// Installation parameters for the social recovery policy.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SocialRecoveryAccountParams {
    /// Addresses allowed to approve a recovery.
    pub guardians: Vec<Address>,
    /// Number of distinct guardian approvals a recovery needs.
    pub threshold: u32,
    /// Ledgers between the approval that reaches `threshold` and the earliest
    /// completion.
    pub delay_ledgers: u32,
    /// Ledgers an attempt has to reach `threshold` after it opens, and to
    /// complete after it becomes executable.
    pub expiry_ledgers: u32,
}

/// A recovery attempt for one signer set at one epoch.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RecoveryAttempt {
    /// Ledger of the approval that opened the attempt.
    pub opened_at: u32,
    /// Guardians that approved the attempt.
    pub approvals: Vec<Address>,
    /// First ledger at which the attempt can complete. Set by the approval
    /// that reaches `threshold`, and cleared by a revoke that drops the
    /// approvals below it.
    pub executable_after: Option<u32>,
}

/// Storage keys for social recovery policy data.
#[contracttype]
pub enum SocialRecoveryStorageKey {
    /// Configuration of a smart account's recovery rule.
    Config(Address, u32), // Persistent
    /// Epoch of a smart account's recovery rule. Never removed.
    Epoch(Address, u32), // Persistent
    /// Attempt of a smart account's recovery rule, keyed by epoch and the
    /// SHA-256 hash of the XDR-encoded signer set.
    Attempt(Address, u32, u32, BytesN<32>), // Persistent
}

/// Lifecycle state of a stored attempt at the current ledger.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum AttemptState {
    /// Fewer than `threshold` approvals, within `expiry_ledgers` of opening.
    Collecting,
    /// At least `threshold` approvals, before `executable_after +
    /// expiry_ledgers`.
    Ready,
    /// Past the deadline of its current state.
    Expired,
}

// ################## ERRORS ##################

/// Error codes for social recovery policy operations.
#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq, PartialOrd, Ord)]
#[repr(u32)]
pub enum SocialRecoveryError {
    /// The policy is not installed for the smart account and context rule.
    NotInstalled = 3230,
    /// The policy is already installed for the smart account and context rule.
    AlreadyInstalled = 3231,
    /// The configuration or the recovery rule is invalid.
    InvalidConfig = 3232,
    /// The proposed signer set is invalid.
    InvalidSigners = 3233,
    /// The address is not a guardian of the recovery rule.
    NotGuardian = 3234,
    /// The epoch does not match the recovery rule's current epoch.
    StaleEpoch = 3235,
    /// The guardian has already approved the attempt.
    AlreadyApproved = 3236,
    /// The guardian has no approval on a live attempt to revoke.
    ApprovalNotFound = 3237,
    /// The context is not an executable recovery completion.
    NotAllowed = 3238,
    /// A ledger or epoch computation overflowed.
    MathOverflow = 3239,
}

// ################## CONSTANTS ##################

const DAY_IN_LEDGERS: u32 = 17280;
/// TTL, in ledgers, that a read extends a social recovery entry to.
pub const SOCIAL_RECOVERY_EXTEND_AMOUNT: u32 = 30 * DAY_IN_LEDGERS;
/// TTL, in ledgers, below which a read extends a social recovery entry.
pub const SOCIAL_RECOVERY_TTL_THRESHOLD: u32 = SOCIAL_RECOVERY_EXTEND_AMOUNT - DAY_IN_LEDGERS;

/// Maximum number of guardians per recovery rule.
pub const MAX_GUARDIANS: u32 = 32;

/// Name of the rule a completed recovery adds.
pub const RECOVERED_RULE_NAME: &str = "recovered";

// ################## EVENTS ##################

/// Event emitted when the social recovery policy is installed.
#[contractevent]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RecoveryInstalled {
    #[topic]
    pub smart_account: Address,
    #[topic]
    pub context_rule_id: u32,
    pub config: SocialRecoveryAccountParams,
    pub epoch: u32,
}

/// Emits an event indicating the social recovery policy has been installed.
///
/// # Arguments
///
/// * `e` - Access to the Soroban environment.
/// * `smart_account` - The address of the smart account.
/// * `context_rule_id` - The ID of the recovery rule.
/// * `config` - The installed configuration.
/// * `epoch` - The epoch after installation.
///
/// # Events
///
/// * topics - `["recovery_installed", smart_account: Address, context_rule_id:
///   u32]`
/// * data - `[config: SocialRecoveryAccountParams, epoch: u32]`
pub fn emit_recovery_installed(
    e: &Env,
    smart_account: &Address,
    context_rule_id: u32,
    config: &SocialRecoveryAccountParams,
    epoch: u32,
) {
    RecoveryInstalled {
        smart_account: smart_account.clone(),
        context_rule_id,
        config: config.clone(),
        epoch,
    }
    .publish(e);
}

/// Event emitted when the social recovery configuration is replaced.
#[contractevent]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RecoveryConfigUpdated {
    #[topic]
    pub smart_account: Address,
    #[topic]
    pub context_rule_id: u32,
    pub config: SocialRecoveryAccountParams,
    pub epoch: u32,
}

/// Emits an event indicating the social recovery configuration has been
/// replaced.
///
/// # Arguments
///
/// * `e` - Access to the Soroban environment.
/// * `smart_account` - The address of the smart account.
/// * `context_rule_id` - The ID of the recovery rule.
/// * `config` - The new configuration.
/// * `epoch` - The epoch after the update.
///
/// # Events
///
/// * topics - `["recovery_config_updated", smart_account: Address,
///   context_rule_id: u32]`
/// * data - `[config: SocialRecoveryAccountParams, epoch: u32]`
pub fn emit_recovery_config_updated(
    e: &Env,
    smart_account: &Address,
    context_rule_id: u32,
    config: &SocialRecoveryAccountParams,
    epoch: u32,
) {
    RecoveryConfigUpdated {
        smart_account: smart_account.clone(),
        context_rule_id,
        config: config.clone(),
        epoch,
    }
    .publish(e);
}

/// Event emitted when the social recovery policy is uninstalled.
#[contractevent]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RecoveryUninstalled {
    #[topic]
    pub smart_account: Address,
    #[topic]
    pub context_rule_id: u32,
}

/// Emits an event indicating the social recovery policy has been uninstalled.
///
/// # Arguments
///
/// * `e` - Access to the Soroban environment.
/// * `smart_account` - The address of the smart account.
/// * `context_rule_id` - The ID of the recovery rule.
///
/// # Events
///
/// * topics - `["recovery_uninstalled", smart_account: Address,
///   context_rule_id: u32]`
/// * data - `[]`
pub fn emit_recovery_uninstalled(e: &Env, smart_account: &Address, context_rule_id: u32) {
    RecoveryUninstalled { smart_account: smart_account.clone(), context_rule_id }.publish(e);
}

/// Event emitted when a guardian approval is recorded on an attempt.
#[contractevent]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RecoveryApprovalRecorded {
    #[topic]
    pub smart_account: Address,
    #[topic]
    pub context_rule_id: u32,
    pub epoch: u32,
    pub signers_hash: BytesN<32>,
    pub signers: Vec<Signer>,
    pub guardian: Address,
    pub approvals: u32,
    pub quorum_deadline: u32,
}

/// Emits an event indicating a guardian approval has been recorded.
///
/// # Arguments
///
/// * `e` - Access to the Soroban environment.
/// * `smart_account` - The address of the smart account.
/// * `context_rule_id` - The ID of the recovery rule.
/// * `epoch` - The epoch of the attempt.
/// * `signers_hash` - The hash identifying the attempt within the epoch.
/// * `signers` - The approved signer set.
/// * `guardian` - The approving guardian.
/// * `approvals` - The number of approvals on the attempt.
/// * `quorum_deadline` - The first ledger at which the attempt can no longer
///   reach the threshold.
///
/// # Events
///
/// * topics - `["recovery_approval_recorded", smart_account: Address,
///   context_rule_id: u32]`
/// * data - `[epoch: u32, signers_hash: BytesN<32>, signers: Vec<Signer>,
///   guardian: Address, approvals: u32, quorum_deadline: u32]`
#[allow(clippy::too_many_arguments)]
pub fn emit_recovery_approval_recorded(
    e: &Env,
    smart_account: &Address,
    context_rule_id: u32,
    epoch: u32,
    signers_hash: &BytesN<32>,
    signers: &Vec<Signer>,
    guardian: &Address,
    approvals: u32,
    quorum_deadline: u32,
) {
    RecoveryApprovalRecorded {
        smart_account: smart_account.clone(),
        context_rule_id,
        epoch,
        signers_hash: signers_hash.clone(),
        signers: signers.clone(),
        guardian: guardian.clone(),
        approvals,
        quorum_deadline,
    }
    .publish(e);
}

/// Event emitted when an attempt reaches the threshold and its delay starts.
#[contractevent]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RecoveryQuorumReached {
    #[topic]
    pub smart_account: Address,
    #[topic]
    pub context_rule_id: u32,
    pub epoch: u32,
    pub signers_hash: BytesN<32>,
    pub executable_after: u32,
    pub expires_at: u32,
}

/// Emits an event indicating an attempt has reached the threshold.
///
/// # Arguments
///
/// * `e` - Access to the Soroban environment.
/// * `smart_account` - The address of the smart account.
/// * `context_rule_id` - The ID of the recovery rule.
/// * `epoch` - The epoch of the attempt.
/// * `signers_hash` - The hash identifying the attempt within the epoch.
/// * `executable_after` - The first ledger at which the attempt can complete.
/// * `expires_at` - The first ledger at which the attempt can no longer
///   complete.
///
/// # Events
///
/// * topics - `["recovery_quorum_reached", smart_account: Address,
///   context_rule_id: u32]`
/// * data - `[epoch: u32, signers_hash: BytesN<32>, executable_after: u32,
///   expires_at: u32]`
pub fn emit_recovery_quorum_reached(
    e: &Env,
    smart_account: &Address,
    context_rule_id: u32,
    epoch: u32,
    signers_hash: &BytesN<32>,
    executable_after: u32,
    expires_at: u32,
) {
    RecoveryQuorumReached {
        smart_account: smart_account.clone(),
        context_rule_id,
        epoch,
        signers_hash: signers_hash.clone(),
        executable_after,
        expires_at,
    }
    .publish(e);
}

/// Event emitted when a guardian revokes an approval.
#[contractevent]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RecoveryApprovalRevoked {
    #[topic]
    pub smart_account: Address,
    #[topic]
    pub context_rule_id: u32,
    pub epoch: u32,
    pub signers_hash: BytesN<32>,
    pub guardian: Address,
    pub approvals: u32,
    pub executable_after: Option<u32>,
}

/// Emits an event indicating a guardian has revoked an approval.
///
/// # Arguments
///
/// * `e` - Access to the Soroban environment.
/// * `smart_account` - The address of the smart account.
/// * `context_rule_id` - The ID of the recovery rule.
/// * `epoch` - The epoch of the attempt.
/// * `signers_hash` - The hash identifying the attempt within the epoch.
/// * `guardian` - The revoking guardian.
/// * `approvals` - The number of approvals left on the attempt.
/// * `executable_after` - The first ledger at which the attempt can complete,
///   or `None` when the approvals are below the threshold.
///
/// # Events
///
/// * topics - `["recovery_approval_revoked", smart_account: Address,
///   context_rule_id: u32]`
/// * data - `[epoch: u32, signers_hash: BytesN<32>, guardian: Address,
///   approvals: u32, executable_after: Option<u32>]` (`executable_after` is
///   left out of the data when it is `None`)
#[allow(clippy::too_many_arguments)]
pub fn emit_recovery_approval_revoked(
    e: &Env,
    smart_account: &Address,
    context_rule_id: u32,
    epoch: u32,
    signers_hash: &BytesN<32>,
    guardian: &Address,
    approvals: u32,
    executable_after: Option<u32>,
) {
    RecoveryApprovalRevoked {
        smart_account: smart_account.clone(),
        context_rule_id,
        epoch,
        signers_hash: signers_hash.clone(),
        guardian: guardian.clone(),
        approvals,
        executable_after,
    }
    .publish(e);
}

/// Event emitted when the smart account cancels every open attempt.
#[contractevent]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RecoveryCancelled {
    #[topic]
    pub smart_account: Address,
    #[topic]
    pub context_rule_id: u32,
    pub epoch: u32,
}

/// Emits an event indicating the smart account has cancelled every open
/// attempt.
///
/// # Arguments
///
/// * `e` - Access to the Soroban environment.
/// * `smart_account` - The address of the smart account.
/// * `context_rule_id` - The ID of the recovery rule.
/// * `epoch` - The epoch after the cancellation.
///
/// # Events
///
/// * topics - `["recovery_cancelled", smart_account: Address, context_rule_id:
///   u32]`
/// * data - `[epoch: u32]`
pub fn emit_recovery_cancelled(e: &Env, smart_account: &Address, context_rule_id: u32, epoch: u32) {
    RecoveryCancelled { smart_account: smart_account.clone(), context_rule_id, epoch }.publish(e);
}

/// Event emitted when the policy authorizes a recovery completion. The
/// recovery is complete only when the smart account emits `context_rule_added`
/// in the same transaction.
#[contractevent]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RecoveryAuthorized {
    #[topic]
    pub smart_account: Address,
    #[topic]
    pub context_rule_id: u32,
    pub epoch: u32,
    pub signers_hash: BytesN<32>,
}

/// Emits an event indicating the policy has authorized a recovery completion.
///
/// # Arguments
///
/// * `e` - Access to the Soroban environment.
/// * `smart_account` - The address of the smart account.
/// * `context_rule_id` - The ID of the recovery rule.
/// * `epoch` - The epoch of the consumed attempt.
/// * `signers_hash` - The hash identifying the consumed attempt within the
///   epoch.
///
/// # Events
///
/// * topics - `["recovery_authorized", smart_account: Address, context_rule_id:
///   u32]`
/// * data - `[epoch: u32, signers_hash: BytesN<32>]`
pub fn emit_recovery_authorized(
    e: &Env,
    smart_account: &Address,
    context_rule_id: u32,
    epoch: u32,
    signers_hash: &BytesN<32>,
) {
    RecoveryAuthorized {
        smart_account: smart_account.clone(),
        context_rule_id,
        epoch,
        signers_hash: signers_hash.clone(),
    }
    .publish(e);
}

// ################## QUERY STATE ##################

/// Retrieves the social recovery configuration of a smart account's recovery
/// rule.
///
/// # Arguments
///
/// * `e` - Access to the Soroban environment.
/// * `context_rule_id` - The ID of the recovery rule.
/// * `smart_account` - The address of the smart account.
///
/// # Errors
///
/// * [`SocialRecoveryError::NotInstalled`] - When the policy is not installed
///   for the smart account and context rule.
pub fn get_config(
    e: &Env,
    context_rule_id: u32,
    smart_account: &Address,
) -> SocialRecoveryAccountParams {
    let key = SocialRecoveryStorageKey::Config(smart_account.clone(), context_rule_id);
    e.storage()
        .persistent()
        .get(&key)
        .inspect(|_| {
            e.storage().persistent().extend_ttl(
                &key,
                SOCIAL_RECOVERY_TTL_THRESHOLD,
                SOCIAL_RECOVERY_EXTEND_AMOUNT,
            );
        })
        .unwrap_or_else(|| panic_with_error!(e, SocialRecoveryError::NotInstalled))
}

/// Returns the current epoch of a smart account's recovery rule, or `0` when
/// the policy was never installed on it.
///
/// # Arguments
///
/// * `e` - Access to the Soroban environment.
/// * `context_rule_id` - The ID of the recovery rule.
/// * `smart_account` - The address of the smart account.
pub fn get_epoch(e: &Env, context_rule_id: u32, smart_account: &Address) -> u32 {
    let key = SocialRecoveryStorageKey::Epoch(smart_account.clone(), context_rule_id);
    if let Some(epoch) = e.storage().persistent().get::<_, u32>(&key) {
        e.storage().persistent().extend_ttl(
            &key,
            SOCIAL_RECOVERY_TTL_THRESHOLD,
            SOCIAL_RECOVERY_EXTEND_AMOUNT,
        );
        epoch
    } else {
        0
    }
}

/// Returns the stored attempt for a signer set at an epoch, or `None` when
/// there is none. The attempt may be expired; its state follows from its
/// fields, the configuration's `expiry_ledgers` and the current ledger.
///
/// # Arguments
///
/// * `e` - Access to the Soroban environment.
/// * `epoch` - The epoch of the attempt.
/// * `signers_hash` - The SHA-256 hash of the XDR-encoded signer set.
/// * `context_rule_id` - The ID of the recovery rule.
/// * `smart_account` - The address of the smart account.
pub fn get_attempt(
    e: &Env,
    epoch: u32,
    signers_hash: &BytesN<32>,
    context_rule_id: u32,
    smart_account: &Address,
) -> Option<RecoveryAttempt> {
    let key = SocialRecoveryStorageKey::Attempt(
        smart_account.clone(),
        context_rule_id,
        epoch,
        signers_hash.clone(),
    );
    e.storage().persistent().get(&key).inspect(|_| {
        e.storage().persistent().extend_ttl(
            &key,
            SOCIAL_RECOVERY_TTL_THRESHOLD,
            SOCIAL_RECOVERY_EXTEND_AMOUNT,
        )
    })
}

// ################## CHANGE STATE ##################

/// Enforces the social recovery policy. Admits only the completion of an
/// executable attempt, then removes the attempt and increments the epoch.
/// Requires authorization from the smart account.
///
/// The context must be a call to `add_context_rule(Default,
/// RECOVERED_RULE_NAME, None, signers, {})` on the smart account, where the
/// hash of `signers` identifies an attempt at the current epoch that has
/// reached the threshold, passed its delay and not expired.
///
/// # Arguments
///
/// * `e` - Access to the Soroban environment.
/// * `context` - The authorization context.
/// * `_authenticated_signers` - The list of authenticated signers. Ignored, as
///   the recovery rule has no signers.
/// * `context_rule` - The recovery rule.
/// * `smart_account` - The address of the smart account.
///
/// # Errors
///
/// * [`SocialRecoveryError::NotAllowed`] - When the context is not a completion
///   of that shape, or no attempt for its signers is executable at the current
///   epoch.
/// * [`SocialRecoveryError::MathOverflow`] - When the epoch or a deadline
///   overflows.
/// * refer to [`get_config`] errors.
///
/// # Events
///
/// * topics - `["recovery_authorized", smart_account: Address, context_rule_id:
///   u32]`
/// * data - `[epoch: u32, signers_hash: BytesN<32>]`
///
/// # Notes
///
/// * Authorization for `smart_account` is required.
pub fn enforce(
    e: &Env,
    context: &Context,
    _authenticated_signers: &Vec<Signer>,
    context_rule: &ContextRule,
    smart_account: &Address,
) {
    smart_account.require_auth();

    let config = get_config(e, context_rule.id, smart_account);

    let signers = completion_signers(e, context, smart_account)
        .unwrap_or_else(|| panic_with_error!(e, SocialRecoveryError::NotAllowed));
    let signers_hash = hash_signers(e, &signers);
    let epoch = get_epoch(e, context_rule.id, smart_account);

    let attempt = get_attempt(e, epoch, &signers_hash, context_rule.id, smart_account)
        .unwrap_or_else(|| panic_with_error!(e, SocialRecoveryError::NotAllowed));
    let executable =
        match (attempt_state(e, &attempt, config.expiry_ledgers), attempt.executable_after) {
            (AttemptState::Ready, Some(executable_after)) => {
                executable_after <= e.ledger().sequence()
            }
            _ => false,
        };
    if !executable {
        panic_with_error!(e, SocialRecoveryError::NotAllowed)
    }

    e.storage().persistent().remove(&SocialRecoveryStorageKey::Attempt(
        smart_account.clone(),
        context_rule.id,
        epoch,
        signers_hash.clone(),
    ));
    increment_epoch(e, context_rule.id, smart_account);

    emit_recovery_authorized(e, smart_account, context_rule.id, epoch, &signers_hash);
}

/// Installs the social recovery policy on a smart account's recovery rule and
/// increments its epoch. Requires authorization from the smart account.
///
/// # Arguments
///
/// * `e` - Access to the Soroban environment.
/// * `params` - The recovery configuration.
/// * `context_rule` - The recovery rule, which must be scoped to
///   `CallContract(smart_account)`.
/// * `smart_account` - The address of the smart account.
///
/// # Errors
///
/// * [`SocialRecoveryError::InvalidConfig`] - When the context rule is not
///   scoped to `CallContract(smart_account)`.
/// * [`SocialRecoveryError::AlreadyInstalled`] - When the policy is already
///   installed for the smart account and context rule.
/// * [`SocialRecoveryError::MathOverflow`] - When the epoch overflows.
/// * refer to [`validate_config`] errors.
///
/// # Events
///
/// * topics - `["recovery_installed", smart_account: Address, context_rule_id:
///   u32]`
/// * data - `[config: SocialRecoveryAccountParams, epoch: u32]`
///
/// # Notes
///
/// * Authorization for `smart_account` is required.
/// * The rule is expected to have no signers and this policy as its only
///   policy, but only its scope is checked: later `add_signer` or `add_policy`
///   calls on the rule are not observed.
pub fn install(
    e: &Env,
    params: &SocialRecoveryAccountParams,
    context_rule: &ContextRule,
    smart_account: &Address,
) {
    smart_account.require_auth();

    if context_rule.context_type != ContextRuleType::CallContract(smart_account.clone()) {
        panic_with_error!(e, SocialRecoveryError::InvalidConfig)
    }

    let key = SocialRecoveryStorageKey::Config(smart_account.clone(), context_rule.id);
    if e.storage().persistent().has(&key) {
        panic_with_error!(e, SocialRecoveryError::AlreadyInstalled)
    }

    validate_config(e, params, smart_account);

    let epoch = increment_epoch(e, context_rule.id, smart_account);
    e.storage().persistent().set(&key, params);

    emit_recovery_installed(e, smart_account, context_rule.id, params, epoch);
}

/// Uninstalls the social recovery policy from a smart account's recovery rule.
/// Removes the configuration and keeps the epoch, so approvals signed before
/// the uninstall stay invalid after a reinstall. Requires authorization from
/// the smart account.
///
/// # Arguments
///
/// * `e` - Access to the Soroban environment.
/// * `context_rule` - The recovery rule.
/// * `smart_account` - The address of the smart account.
///
/// # Errors
///
/// * [`SocialRecoveryError::NotInstalled`] - When the policy is not installed
///   for the smart account and context rule.
///
/// # Events
///
/// * topics - `["recovery_uninstalled", smart_account: Address,
///   context_rule_id: u32]`
/// * data - `[]`
///
/// # Notes
///
/// * Authorization for `smart_account` is required.
pub fn uninstall(e: &Env, context_rule: &ContextRule, smart_account: &Address) {
    smart_account.require_auth();

    let key = SocialRecoveryStorageKey::Config(smart_account.clone(), context_rule.id);
    if !e.storage().persistent().has(&key) {
        panic_with_error!(e, SocialRecoveryError::NotInstalled)
    }

    e.storage().persistent().remove(&key);

    emit_recovery_uninstalled(e, smart_account, context_rule.id);
}

/// Records a guardian's approval of a signer set at the current epoch.
/// Requires authorization from the guardian, which binds every argument.
///
/// The first approval of a signer set opens an attempt, and so does an
/// approval of an expired attempt, which discards the earlier approvals. The
/// approval that reaches the threshold sets `executable_after` to the current
/// ledger plus `delay_ledgers`. Later approvals are recorded and never move
/// it.
///
/// # Arguments
///
/// * `e` - Access to the Soroban environment.
/// * `guardian` - The approving guardian.
/// * `epoch` - The current epoch of the recovery rule.
/// * `signers` - The signers of the rule a completion adds. Their order
///   identifies the attempt.
/// * `context_rule_id` - The ID of the recovery rule.
/// * `smart_account` - The address of the smart account.
///
/// # Errors
///
/// * [`SocialRecoveryError::NotGuardian`] - When `guardian` is not a guardian
///   of the recovery rule.
/// * [`SocialRecoveryError::StaleEpoch`] - When `epoch` is not the current
///   epoch.
/// * [`SocialRecoveryError::AlreadyApproved`] - When the guardian has already
///   approved the live attempt.
/// * [`SocialRecoveryError::MathOverflow`] - When a deadline overflows.
/// * refer to [`get_config`] errors.
/// * refer to [`validate_signers`] errors.
///
/// # Events
///
/// * topics - `["recovery_approval_recorded", smart_account: Address,
///   context_rule_id: u32]`
/// * data - `[epoch: u32, signers_hash: BytesN<32>, signers: Vec<Signer>,
///   guardian: Address, approvals: u32, quorum_deadline: u32]`
///
/// When the approval reaches the threshold, also:
/// * topics - `["recovery_quorum_reached", smart_account: Address,
///   context_rule_id: u32]`
/// * data - `[epoch: u32, signers_hash: BytesN<32>, executable_after: u32,
///   expires_at: u32]`
///
/// # Notes
///
/// * Authorization for `guardian` is required.
/// * Validating `signers` calls `batch_canonicalize_key` once on each verifier
///   the set names. A verifier that fails blocks approvals of the sets that
///   name it and of no other set.
pub fn approve(
    e: &Env,
    guardian: &Address,
    epoch: u32,
    signers: &Vec<Signer>,
    context_rule_id: u32,
    smart_account: &Address,
) {
    guardian.require_auth();

    let config = get_config(e, context_rule_id, smart_account);

    if !config.guardians.contains(guardian) {
        panic_with_error!(e, SocialRecoveryError::NotGuardian)
    }

    if epoch != get_epoch(e, context_rule_id, smart_account) {
        panic_with_error!(e, SocialRecoveryError::StaleEpoch)
    }

    validate_signers(e, signers, smart_account);

    let signers_hash = hash_signers(e, signers);
    let current_ledger = e.ledger().sequence();

    let mut attempt = match get_attempt(e, epoch, &signers_hash, context_rule_id, smart_account) {
        Some(mut attempt)
            if attempt_state(e, &attempt, config.expiry_ledgers) != AttemptState::Expired =>
        {
            if attempt.approvals.contains(guardian) {
                panic_with_error!(e, SocialRecoveryError::AlreadyApproved)
            }
            attempt.approvals.push_back(guardian.clone());
            attempt
        }
        _ => RecoveryAttempt {
            opened_at: current_ledger,
            approvals: Vec::from_array(e, [guardian.clone()]),
            executable_after: None,
        },
    };

    let reached_quorum =
        attempt.executable_after.is_none() && attempt.approvals.len() >= config.threshold;
    if reached_quorum {
        attempt.executable_after = Some(add_ledgers(e, current_ledger, config.delay_ledgers));
    }
    let quorum_deadline = add_ledgers(e, attempt.opened_at, config.expiry_ledgers);

    e.storage().persistent().set(
        &SocialRecoveryStorageKey::Attempt(
            smart_account.clone(),
            context_rule_id,
            epoch,
            signers_hash.clone(),
        ),
        &attempt,
    );

    emit_recovery_approval_recorded(
        e,
        smart_account,
        context_rule_id,
        epoch,
        &signers_hash,
        signers,
        guardian,
        attempt.approvals.len(),
        quorum_deadline,
    );

    if let (true, Some(executable_after)) = (reached_quorum, attempt.executable_after) {
        emit_recovery_quorum_reached(
            e,
            smart_account,
            context_rule_id,
            epoch,
            &signers_hash,
            executable_after,
            add_ledgers(e, executable_after, config.expiry_ledgers),
        );
    }
}

/// Removes a guardian's approval from a live attempt at the current epoch.
/// Requires authorization from the guardian, which binds every argument.
///
/// When the remaining approvals fall below the threshold, `executable_after`
/// is cleared and the next approval that reaches the threshold starts a new
/// delay; the quorum deadline stays `opened_at + expiry_ledgers`. When no
/// approval remains, the attempt is removed.
///
/// # Arguments
///
/// * `e` - Access to the Soroban environment.
/// * `guardian` - The revoking guardian.
/// * `epoch` - The current epoch of the recovery rule.
/// * `signers` - The signer set of the attempt.
/// * `context_rule_id` - The ID of the recovery rule.
/// * `smart_account` - The address of the smart account.
///
/// # Errors
///
/// * [`SocialRecoveryError::NotGuardian`] - When `guardian` is not a guardian
///   of the recovery rule.
/// * [`SocialRecoveryError::StaleEpoch`] - When `epoch` is not the current
///   epoch.
/// * [`SocialRecoveryError::ApprovalNotFound`] - When the attempt does not
///   exist, has expired, or holds no approval from `guardian`.
/// * [`SocialRecoveryError::MathOverflow`] - When a deadline overflows.
/// * refer to [`get_config`] errors.
///
/// # Events
///
/// * topics - `["recovery_approval_revoked", smart_account: Address,
///   context_rule_id: u32]`
/// * data - `[epoch: u32, signers_hash: BytesN<32>, guardian: Address,
///   approvals: u32, executable_after: Option<u32>]` (`executable_after` is
///   left out of the data when it is `None`)
///
/// # Notes
///
/// * Authorization for `guardian` is required.
/// * `signers` only identifies the attempt. It is not validated and no verifier
///   is called, so a revoke succeeds whatever the verifiers it names do.
pub fn revoke_approval(
    e: &Env,
    guardian: &Address,
    epoch: u32,
    signers: &Vec<Signer>,
    context_rule_id: u32,
    smart_account: &Address,
) {
    guardian.require_auth();

    let config = get_config(e, context_rule_id, smart_account);

    if !config.guardians.contains(guardian) {
        panic_with_error!(e, SocialRecoveryError::NotGuardian)
    }

    if epoch != get_epoch(e, context_rule_id, smart_account) {
        panic_with_error!(e, SocialRecoveryError::StaleEpoch)
    }

    let signers_hash = hash_signers(e, signers);
    let key = SocialRecoveryStorageKey::Attempt(
        smart_account.clone(),
        context_rule_id,
        epoch,
        signers_hash.clone(),
    );

    let mut attempt = get_attempt(e, epoch, &signers_hash, context_rule_id, smart_account)
        .filter(|attempt| attempt_state(e, attempt, config.expiry_ledgers) != AttemptState::Expired)
        .unwrap_or_else(|| panic_with_error!(e, SocialRecoveryError::ApprovalNotFound));
    let index = attempt
        .approvals
        .first_index_of(guardian)
        .unwrap_or_else(|| panic_with_error!(e, SocialRecoveryError::ApprovalNotFound));
    attempt.approvals.remove(index);

    if attempt.approvals.len() < config.threshold {
        attempt.executable_after = None;
    }

    if attempt.approvals.is_empty() {
        e.storage().persistent().remove(&key);
    } else {
        e.storage().persistent().set(&key, &attempt);
    }

    emit_recovery_approval_revoked(
        e,
        smart_account,
        context_rule_id,
        epoch,
        &signers_hash,
        guardian,
        attempt.approvals.len(),
        attempt.executable_after,
    );
}

/// Cancels every open attempt and signed approval of a smart account's
/// recovery rule by incrementing its epoch. Requires authorization from the
/// smart account.
///
/// # Arguments
///
/// * `e` - Access to the Soroban environment.
/// * `context_rule_id` - The ID of the recovery rule.
/// * `smart_account` - The address of the smart account.
///
/// # Errors
///
/// * [`SocialRecoveryError::NotInstalled`] - When the policy is not installed
///   for the smart account and context rule.
/// * [`SocialRecoveryError::MathOverflow`] - When the epoch overflows.
///
/// # Events
///
/// * topics - `["recovery_cancelled", smart_account: Address, context_rule_id:
///   u32]`
/// * data - `[epoch: u32]`
///
/// # Notes
///
/// * Authorization for `smart_account` is required.
/// * The cancellation has no deadline or precondition, and works with no
///   attempt open.
pub fn cancel(e: &Env, context_rule_id: u32, smart_account: &Address) {
    smart_account.require_auth();

    let key = SocialRecoveryStorageKey::Config(smart_account.clone(), context_rule_id);
    if !e.storage().persistent().has(&key) {
        panic_with_error!(e, SocialRecoveryError::NotInstalled)
    }
    e.storage().persistent().extend_ttl(
        &key,
        SOCIAL_RECOVERY_TTL_THRESHOLD,
        SOCIAL_RECOVERY_EXTEND_AMOUNT,
    );

    let epoch = increment_epoch(e, context_rule_id, smart_account);

    emit_recovery_cancelled(e, smart_account, context_rule_id, epoch);
}

/// Replaces the social recovery configuration of a smart account's recovery
/// rule and increments its epoch, which cancels every open attempt. Requires
/// authorization from the smart account.
///
/// # Arguments
///
/// * `e` - Access to the Soroban environment.
/// * `params` - The new recovery configuration.
/// * `context_rule_id` - The ID of the recovery rule.
/// * `smart_account` - The address of the smart account.
///
/// # Errors
///
/// * [`SocialRecoveryError::NotInstalled`] - When the policy is not installed
///   for the smart account and context rule.
/// * [`SocialRecoveryError::MathOverflow`] - When the epoch overflows.
/// * refer to [`validate_config`] errors.
///
/// # Events
///
/// * topics - `["recovery_config_updated", smart_account: Address,
///   context_rule_id: u32]`
/// * data - `[config: SocialRecoveryAccountParams, epoch: u32]`
///
/// # Notes
///
/// * Authorization for `smart_account` is required.
/// * The new configuration takes effect immediately.
pub fn set_config(
    e: &Env,
    params: &SocialRecoveryAccountParams,
    context_rule_id: u32,
    smart_account: &Address,
) {
    smart_account.require_auth();

    let key = SocialRecoveryStorageKey::Config(smart_account.clone(), context_rule_id);
    if !e.storage().persistent().has(&key) {
        panic_with_error!(e, SocialRecoveryError::NotInstalled)
    }
    e.storage().persistent().extend_ttl(
        &key,
        SOCIAL_RECOVERY_TTL_THRESHOLD,
        SOCIAL_RECOVERY_EXTEND_AMOUNT,
    );

    validate_config(e, params, smart_account);

    let epoch = increment_epoch(e, context_rule_id, smart_account);
    e.storage().persistent().set(&key, params);

    emit_recovery_config_updated(e, smart_account, context_rule_id, params, epoch);
}

// ################## LOW-LEVEL HELPERS ##################

/// Validates a social recovery configuration for a smart account.
///
/// # Arguments
///
/// * `e` - Access to the Soroban environment.
/// * `params` - The recovery configuration.
/// * `smart_account` - The address of the smart account.
///
/// # Errors
///
/// * [`SocialRecoveryError::InvalidConfig`] - When the guardian set is empty,
///   exceeds [`MAX_GUARDIANS`], holds a duplicate, the smart account or this
///   policy contract; when the threshold is `0` or exceeds the guardian count;
///   when `delay_ledgers` or `expiry_ledgers` is `0`, or the current ledger
///   plus `delay_ledgers` plus twice `expiry_ledgers` exceeds `u32::MAX`.
pub fn validate_config(e: &Env, params: &SocialRecoveryAccountParams, smart_account: &Address) {
    let policy = e.current_contract_address();
    let SocialRecoveryAccountParams { guardians, threshold, delay_ledgers, expiry_ledgers } =
        params;

    let guardian_count = guardians.len();
    if guardian_count > MAX_GUARDIANS {
        panic_with_error!(e, SocialRecoveryError::InvalidConfig)
    }

    let has_duplicate =
        guardians.iter().any(|g| guardians.first_index_of(&g) != guardians.last_index_of(&g));
    let ledger_headroom = e
        .ledger()
        .sequence()
        .checked_add(*delay_ledgers)
        .and_then(|ledger| ledger.checked_add(*expiry_ledgers))
        .and_then(|ledger| ledger.checked_add(*expiry_ledgers));
    if has_duplicate
        || guardians.contains(smart_account)
        || guardians.contains(&policy)
        || *threshold == 0
        || *threshold > guardian_count
        || *delay_ledgers == 0
        || *expiry_ledgers == 0
        || ledger_headroom.is_none()
    {
        panic_with_error!(e, SocialRecoveryError::InvalidConfig)
    }
}

/// Validates a proposed signer set for a smart account, then checks it for
/// canonical duplicates, which calls `batch_canonicalize_key` once on each
/// verifier the set names.
///
/// # Arguments
///
/// * `e` - Access to the Soroban environment.
/// * `signers` - The proposed signer set.
/// * `smart_account` - The address of the smart account.
///
/// # Errors
///
/// * [`SocialRecoveryError::InvalidSigners`] - When the set is empty, exceeds
///   `MAX_SIGNERS`, holds an external key over `MAX_EXTERNAL_KEY_SIZE` bytes, a
///   delegated signer that is the smart account or this policy contract, or two
///   equal signers.
/// * refer to [`validate_no_canonical_duplicates`] errors.
pub fn validate_signers(e: &Env, signers: &Vec<Signer>, smart_account: &Address) {
    let policy = e.current_contract_address();
    let invalid = signers.is_empty()
        || signers.len() > MAX_SIGNERS
        || signers.iter().any(|signer| {
            let invalid_signer = match &signer {
                Signer::External(_, key_data) => key_data.len() > MAX_EXTERNAL_KEY_SIZE,
                Signer::Delegated(address) => address == smart_account || *address == policy,
            };
            invalid_signer || signers.first_index_of(&signer) != signers.last_index_of(&signer)
        });
    if invalid {
        panic_with_error!(e, SocialRecoveryError::InvalidSigners)
    }

    validate_no_canonical_duplicates(e, signers);
}

/// Returns the SHA-256 hash of the XDR-encoded signer set, which identifies
/// an attempt within an epoch. The hash depends on the order of the signers.
///
/// # Arguments
///
/// * `e` - Access to the Soroban environment.
/// * `signers` - The signer set.
pub fn hash_signers(e: &Env, signers: &Vec<Signer>) -> BytesN<32> {
    e.crypto().sha256(&signers.clone().to_xdr(e)).to_bytes()
}

/// Returns the signers of a recovery completion, or `None` when `context` is
/// not a call to `add_context_rule` on the smart account with a `Default`
/// context type, the name [`RECOVERED_RULE_NAME`], no `valid_until` and no
/// policies.
///
/// # Arguments
///
/// * `e` - Access to the Soroban environment.
/// * `context` - The authorization context.
/// * `smart_account` - The address of the smart account.
fn completion_signers(e: &Env, context: &Context, smart_account: &Address) -> Option<Vec<Signer>> {
    let Context::Contract(ContractContext { contract, fn_name, args }) = context else {
        return None;
    };
    if contract != smart_account
        || *fn_name != Symbol::new(e, "add_context_rule")
        || args.len() != 5
    {
        return None;
    }

    let context_type = ContextRuleType::try_from_val(e, &args.get(0)?).ok()?;
    let name = String::try_from_val(e, &args.get(1)?).ok()?;
    let valid_until = Option::<u32>::try_from_val(e, &args.get(2)?).ok()?;
    let signers = Vec::<Signer>::try_from_val(e, &args.get(3)?).ok()?;
    let policies = Map::<Address, Val>::try_from_val(e, &args.get(4)?).ok()?;

    (context_type == ContextRuleType::Default
        && name == String::from_str(e, RECOVERED_RULE_NAME)
        && valid_until.is_none()
        && policies.is_empty())
    .then_some(signers)
}

/// Returns the state of a stored attempt at the current ledger.
///
/// # Arguments
///
/// * `e` - Access to the Soroban environment.
/// * `attempt` - The stored attempt.
/// * `expiry_ledgers` - The configured expiry window.
///
/// # Errors
///
/// * [`SocialRecoveryError::MathOverflow`] - When a deadline overflows.
fn attempt_state(e: &Env, attempt: &RecoveryAttempt, expiry_ledgers: u32) -> AttemptState {
    let current_ledger = e.ledger().sequence();
    match attempt.executable_after {
        None if current_ledger < add_ledgers(e, attempt.opened_at, expiry_ledgers) => {
            AttemptState::Collecting
        }
        Some(executable_after)
            if current_ledger < add_ledgers(e, executable_after, expiry_ledgers) =>
        {
            AttemptState::Ready
        }
        _ => AttemptState::Expired,
    }
}

/// Increments the epoch of a smart account's recovery rule and returns the new
/// epoch.
///
/// # Arguments
///
/// * `e` - Access to the Soroban environment.
/// * `context_rule_id` - The ID of the recovery rule.
/// * `smart_account` - The address of the smart account.
///
/// # Errors
///
/// * [`SocialRecoveryError::MathOverflow`] - When the epoch is `u32::MAX`.
fn increment_epoch(e: &Env, context_rule_id: u32, smart_account: &Address) -> u32 {
    let epoch = get_epoch(e, context_rule_id, smart_account)
        .checked_add(1)
        .unwrap_or_else(|| panic_with_error!(e, SocialRecoveryError::MathOverflow));
    e.storage()
        .persistent()
        .set(&SocialRecoveryStorageKey::Epoch(smart_account.clone(), context_rule_id), &epoch);
    epoch
}

/// Returns `ledger + ledgers`.
///
/// # Arguments
///
/// * `e` - Access to the Soroban environment.
/// * `ledger` - The ledger sequence.
/// * `ledgers` - The number of ledgers to add.
///
/// # Errors
///
/// * [`SocialRecoveryError::MathOverflow`] - When the sum exceeds `u32::MAX`.
fn add_ledgers(e: &Env, ledger: u32, ledgers: u32) -> u32 {
    ledger
        .checked_add(ledgers)
        .unwrap_or_else(|| panic_with_error!(e, SocialRecoveryError::MathOverflow))
}
