//! # Smart Account Factory Example
//!
//! A permissionless factory that deploys the multisig smart account from
//! `examples/multisig-smart-account/account`. Each account's address is bound
//! to its initial configuration through the factory's namespace: only this
//! contract can create in that namespace, and the chain salt hashes the
//! constructor arguments.
use soroban_sdk::{
    contract, contractimpl, contracttype, xdr::ToXdr, Address, BytesN, Env, IntoVal, Map, Val, Vec,
};
use stellar_accounts::smart_account::Signer;

#[contracttype]
enum DataKey {
    AccountWasmHash,
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
        e.storage().instance().set(&DataKey::AccountWasmHash, &account_wasm_hash);
    }

    /// Returns the hash of the account wasm that every deployment
    /// instantiates.
    pub fn pinned_account_wasm_hash(e: &Env) -> BytesN<32> {
        e.storage()
            .instance()
            .get(&DataKey::AccountWasmHash)
            .expect("the account wasm hash is set in __constructor")
    }

    /// Returns the address `deploy` creates for this tuple, whether or
    /// not the account exists yet.
    ///
    /// # Arguments
    ///
    /// * `signers` - Signers of the account's default context rule, in any
    ///   order. Duplicates are removed.
    /// * `policies` - Policy contract addresses mapped to their install
    ///   parameters.
    /// * `salt` - Caller-chosen value that lets one configuration have several
    ///   accounts.
    pub fn predict_address(
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
    /// the address `predict_address` returns for the same tuple.
    ///
    /// # Arguments
    ///
    /// * `signers` - Signers of the account's default context rule, in any
    ///   order. Duplicates are removed.
    /// * `policies` - Policy contract addresses mapped to their install
    ///   parameters.
    /// * `salt` - Caller-chosen value that lets one configuration have several
    ///   accounts.
    pub fn deploy(
        e: &Env,
        signers: Vec<Signer>,
        policies: Map<Address, Val>,
        salt: u32,
    ) -> Address {
        let wasm_hash = Self::pinned_account_wasm_hash(e);
        let signers = canonical_signers(e, &signers);
        let chain_salt = chain_salt(e, &signers, &policies, salt);

        e.deployer().with_current_contract(chain_salt).deploy_v2(wasm_hash, (signers, policies))
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

/// SHA-256 of the canonical XDR of `(signers, policies, salt)`.
fn chain_salt(
    e: &Env,
    signers: &Vec<Signer>,
    policies: &Map<Address, Val>,
    salt: u32,
) -> BytesN<32> {
    let salt_data: Vec<Val> = (signers.clone(), policies.clone(), salt).into_val(e);

    e.crypto().sha256(&salt_data.to_xdr(e)).to_bytes()
}
