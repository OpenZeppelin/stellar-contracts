//! # Smart Account Factory Example
//!
//! A permissionless factory that deploys the multisig smart account from
//! `examples/multisig-smart-account/account`. Each account's address is bound
//! to its initial configuration through the factory's namespace: only this
//! contract can create in that namespace, and the chain salt hashes the
//! constructor arguments.
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

/// Domain-separation tag; first element of the salt preimage.
pub const SALT_PREIMAGE_VERSION: u32 = 1;

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

/// Emits an `AccountDeployed` event.
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
    /// * `account_wasm_hash` - Hash of an account wasm that is already uploaded
    ///   on this network.
    pub fn __constructor(e: &Env, account_wasm_hash: BytesN<32>) {
        e.storage().instance().set(&FactoryStorageKey::AccountWasmHash, &account_wasm_hash);
    }

    /// Returns the hash of the account wasm that every deployment
    /// instantiates.
    pub fn pinned_account_wasm_hash(e: &Env) -> BytesN<32> {
        e.storage()
            .instance()
            .get(&FactoryStorageKey::AccountWasmHash)
            .expect("the account wasm hash is set in __constructor")
    }

    /// Returns the address `deploy_account` creates for this tuple, whether or
    /// not the account exists yet.
    ///
    /// Does not validate account constructor limits. Funds sent to an address
    /// that `deploy_account` can never create are lost.
    ///
    /// # Arguments
    ///
    /// * `signers` - Signers of the account's default context rule, in any
    ///   order. Duplicates are removed.
    /// * `policies` - Policy contract addresses mapped to their install
    ///   parameters.
    /// * `salt` - Caller-chosen value that lets one configuration have several
    ///   accounts.
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

/// Sorts and deduplicates signers through a host `Map`.
fn canonical_signers(e: &Env, signers: &Vec<Signer>) -> Vec<Signer> {
    let mut set: Map<Signer, ()> = Map::new(e);
    for signer in signers.iter() {
        set.set(signer, ());
    }
    set.keys()
}

/// SHA-256 of the canonical XDR of `(SALT_PREIMAGE_VERSION, signers, policies,
/// salt)`.
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
