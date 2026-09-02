//! # Smart Account Factory Example
//!
//! A permissionless factory that deploys the multisig smart account from
//! `examples/multisig-smart-account/account` at an address bound to the
//! account's initial configuration.
//!
//! Soroban derives a contract address from `(network, deployer, salt)` and
//! nothing else: neither the wasm hash nor the constructor arguments are part
//! of the preimage. A wallet that shows a user "your account will live at X"
//! before that account exists therefore has to make sure nobody else can
//! create a *different* contract at X. This factory closes that gap with two
//! properties:
//!
//! - **Namespace exclusivity.** Accounts are created with
//!   `deployer().with_current_contract(salt)`, so the deployer in the address
//!   preimage is the factory itself. Only the factory can create contracts in
//!   its namespace, and the factory has no `__check_auth`, so there is no key
//!   that could authorize a deployment from outside of it.
//! - **Configuration binding.** The 32-byte chain salt is the SHA-256 of the
//!   canonical XDR of `(version, signers, policies, salt)`, where `signers` and
//!   `policies` are exactly the arguments passed to the account's
//!   `__constructor`. A different configuration hashes to a different chain
//!   salt and lands at a different address, so "the victim's address with the
//!   attacker's signers" is not expressible.
//!
//! The extra `salt: u32` exists only so that several accounts can share one
//! configuration. It is never the chain salt, and the caller never supplies
//! the chain salt.
//!
//! `predict` and `deploy_account` take the same argument tuple. `predict` is
//! the only address authority a client needs: it is a pure computation, and
//! there is intentionally no view exposing the intermediate chain salt, so no
//! off-chain code is tempted to re-implement the derivation.
//!
//! ## Wasm pin
//!
//! The account wasm hash is pinned when the factory is constructed and is not
//! an argument of `predict` or `deploy_account`. A different account wasm
//! means a different factory instance, which is a different deployer and
//! therefore a disjoint address namespace. Nothing in the factory is mutable
//! after construction: there is no admin, no setter and no re-pinning.
//!
//! ## Canonicalization
//!
//! The signer list is rebuilt through a host `Map` before it is hashed and
//! before it is passed to the account constructor. Host map keys are sorted
//! and unique, so `[A, B]` and `[B, A]` are one configuration and one address,
//! and so are `[A, A]` and `[A]`. The policy map is a host `Map` already and
//! is canonical by construction.
//!
//! ## Collisions
//!
//! Deploying the same tuple twice fails: the host refuses to create a contract
//! at an occupied address, and the refusal reaches the caller as an untyped
//! `Error(Context, InvalidAction)`. Soroban offers no contract-callable
//! existence check and the host error cannot be caught inside the factory, so
//! the factory does not try to be idempotent. Clients probe the predicted
//! address off-chain before submitting. Because the chain salt binds the
//! configuration, a front-runner who submits your tuple first creates *your*
//! account, at your address, with your signers, at their own expense.
use soroban_sdk::{
    contract, contractevent, contractimpl, contracttype, xdr::ToXdr, Address, BytesN, Env, IntoVal,
    Map, Val, Vec,
};
use stellar_accounts::smart_account::Signer;

/// Storage keys for the factory.
#[contracttype]
pub enum FactoryStorageKey {
    /// Hash of the account wasm every deployment instantiates.
    AccountWasmHash,
}

// ################## CONSTANTS ##################

/// Domain-separation tag and first element of the salt preimage.
///
/// The factory has no upgrade path, so a change to the preimage layout is
/// necessarily a new factory instance and therefore a new namespace anyway.
/// The tag is kept so that the preimage is self-describing and cannot collide
/// with a future layout that hashes the same fields in a different shape.
pub const SALT_PREIMAGE_VERSION: u32 = 1;

// ################## EVENTS ##################

/// Event emitted when an account is deployed.
#[contractevent]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AccountDeployed {
    #[topic]
    pub account: Address,
    pub signers: Vec<Signer>,
    pub policies: Map<Address, Val>,
    pub salt: u32,
}

/// Emits an event indicating that an account was deployed.
///
/// # Arguments
///
/// * `e` - Access to the Soroban environment.
/// * `account` - The address of the deployed account.
/// * `signers` - The canonical signer list the account was constructed with.
/// * `policies` - The policy map the account was constructed with.
/// * `salt` - The caller-chosen extra salt.
pub fn emit_account_deployed(
    e: &Env,
    account: &Address,
    signers: &Vec<Signer>,
    policies: &Map<Address, Val>,
    salt: u32,
) {
    AccountDeployed {
        account: account.clone(),
        signers: signers.clone(),
        policies: policies.clone(),
        salt,
    }
    .publish(e);
}

#[contract]
pub struct AccountFactoryContract;

#[contractimpl]
impl AccountFactoryContract {
    /// Pins the hash of the account wasm that every deployment instantiates.
    ///
    /// # Arguments
    ///
    /// * `e` - Access to the Soroban environment.
    /// * `account_wasm_hash` - Hash of an account wasm that is already uploaded
    ///   on this network.
    ///
    /// # Notes
    ///
    /// The pin is immutable: there is no admin and no setter. To deploy a
    /// different account wasm, deploy a new factory. Nothing is validated
    /// here; pinning a hash whose code is not uploaded on this network yields a
    /// factory whose every `deploy_account` call fails.
    pub fn __constructor(e: &Env, account_wasm_hash: BytesN<32>) {
        e.storage().instance().set(&FactoryStorageKey::AccountWasmHash, &account_wasm_hash);
    }

    /// Returns the hash of the account wasm that every deployment
    /// instantiates.
    ///
    /// # Arguments
    ///
    /// * `e` - Access to the Soroban environment.
    pub fn pinned_account_wasm_hash(e: &Env) -> BytesN<32> {
        e.storage()
            .instance()
            .get(&FactoryStorageKey::AccountWasmHash)
            .expect("the account wasm hash is set in __constructor")
    }

    /// Returns the address `deploy_account` creates for this tuple, whether or
    /// not the account exists yet.
    ///
    /// # Arguments
    ///
    /// * `e` - Access to the Soroban environment.
    /// * `signers` - Signers of the account's default context rule, in any
    ///   order. Duplicates are removed.
    /// * `policies` - Policy contract addresses mapped to their install
    ///   parameters.
    /// * `salt` - Caller-chosen value that lets one configuration have several
    ///   accounts.
    ///
    /// # Notes
    ///
    /// This is a pure computation: it validates nothing and reads no storage.
    /// It answers for tuples that `deploy_account` can never create, such as
    /// an empty signer list, more than `MAX_SIGNERS` signers or key data over
    /// `MAX_EXTERNAL_KEY_SIZE` bytes. Such an address cannot be squatted,
    /// since the namespace is still exclusively the factory's, but funds sent
    /// to it are unrecoverable. Clients should validate a configuration
    /// against the account contract's limits before presenting the address as
    /// usable.
    pub fn predict(
        e: &Env,
        signers: Vec<Signer>,
        policies: Map<Address, Val>,
        salt: u32,
    ) -> Address {
        let signers = canonical_signers(e, &signers);
        let chain_salt = chain_salt(e, &signers, &policies, salt);

        e.deployer().with_current_contract(chain_salt).deployed_address()
    }

    /// Deploys the account for this tuple and returns its address, which is
    /// the address `predict` returns for the same tuple.
    ///
    /// # Arguments
    ///
    /// * `e` - Access to the Soroban environment.
    /// * `signers` - Signers of the account's default context rule, in any
    ///   order. Duplicates are removed.
    /// * `policies` - Policy contract addresses mapped to their install
    ///   parameters.
    /// * `salt` - Caller-chosen value that lets one configuration have several
    ///   accounts.
    ///
    /// # Errors
    ///
    /// * refer to the account contract's `__constructor` errors, which
    ///   propagate unchanged (for example `SmartAccountError::TooManySigners`
    ///   or `SmartAccountError::NoSignersAndPolicies`).
    /// * `Error(Context, InvalidAction)` - an account for this exact tuple
    ///   already exists. This is the host's refusal to create a contract at an
    ///   occupied address, escalated across the contract boundary, so it is not
    ///   distinguishable from other host failures.
    ///
    /// # Events
    ///
    /// * topics - `["account_deployed", account: Address]`
    /// * data - `[signers: Vec<Signer>, policies: Map<Address, Val>, salt:
    ///   u32]`
    ///
    /// # Notes
    ///
    /// The values that feed the chain salt are the values passed to the
    /// account constructor: the canonical signer list and the policy map as
    /// given. Passing anything else to the constructor would break the binding
    /// between address and configuration.
    ///
    /// # Security Warning
    ///
    /// This function requires no authorization on purpose. Anyone may create
    /// any account, and paying the fee for somebody else's account is a
    /// supported use. Because the address binds the configuration, a caller
    /// who submits another party's tuple can only create that party's account.
    pub fn deploy_account(
        e: &Env,
        signers: Vec<Signer>,
        policies: Map<Address, Val>,
        salt: u32,
    ) -> Address {
        let wasm_hash = Self::pinned_account_wasm_hash(e);
        let signers = canonical_signers(e, &signers);
        let chain_salt = chain_salt(e, &signers, &policies, salt);

        let account = e
            .deployer()
            .with_current_contract(chain_salt)
            .deploy_v2(wasm_hash, (signers.clone(), policies.clone()));

        emit_account_deployed(e, &account, &signers, &policies, salt);

        account
    }
}

/// Sorts and deduplicates the signer list through a host `Map`, whose keys are
/// unique and host-ordered. The result is both what gets hashed and what
/// reaches the account constructor.
fn canonical_signers(e: &Env, signers: &Vec<Signer>) -> Vec<Signer> {
    let mut set: Map<Signer, ()> = Map::new(e);
    for signer in signers.iter() {
        set.set(signer, ());
    }
    set.keys()
}

/// SHA-256 of the canonical XDR of `(SALT_PREIMAGE_VERSION, signers,
/// policies, salt)`.
///
/// XDR is length-prefixed and self-delimiting, so distinct tuples cannot share
/// a preimage. Concatenating the fields would not give that guarantee once the
/// signer list and the key material are variable-length.
fn chain_salt(
    e: &Env,
    signers: &Vec<Signer>,
    policies: &Map<Address, Val>,
    salt: u32,
) -> BytesN<32> {
    let preimage: Vec<Val> =
        (SALT_PREIMAGE_VERSION, signers.clone(), policies.clone(), salt).into_val(e);

    e.crypto().sha256(&preimage.to_xdr(e)).to_bytes()
}
