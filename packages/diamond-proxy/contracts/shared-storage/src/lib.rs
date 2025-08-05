#![no_std]
use soroban_sdk::{contract, contractimpl, Address, Bytes, Env, Val, Vec};
use stellar_diamond_proxy_core::utils::keys::SorobanConcat;
use stellar_diamond_proxy_core::{storage::Storage, Error};

#[contract]
pub struct SharedStorageLayer;

const INSTANCE_SHARED_STORAGE_KEY: &str = "__INSTANCE_SHARED_STORAGE";
const PERSISTENT_SHARED_STORAGE_KEY: &str = "__PERSISTENT_SHARED_STORAGE";
const TEMPORARY_SHARED_STORAGE_KEY: &str = "__TEMPORARY_SHARED_STORAGE";

#[contractimpl]
impl SharedStorageLayer {
    pub fn init(env: Env, owner: Address) -> Result<(), Error> {
        let storage = Storage::new(env.clone());
        storage.require_uninitialized();
        storage.set_owner(&owner);
        storage.set_initialized();

        // The owner should be the diamond proxy
        storage.set_diamond_proxy_address(&owner);

        Ok(())
    }

    /// Security check: Ensure only the diamond proxy can call storage functions
    /// This prevents direct calls to shared storage, which would bypass authorization
    fn require_diamond_proxy_caller(env: &Env) -> Result<(), Error> {
        let storage = Storage::new(env.clone());

        // Get the diamond proxy address that was set during initialization
        let diamond_proxy = storage
            .get_diamond_proxy_address()
            .ok_or(Error::DiamondProxyNotSet)?;

        // Require auth on the diamond proxy with the current contract (shared storage) address as argument
        // This will only succeed if the diamond proxy's fallback function authorized this exact call
        let current_address = env.current_contract_address();
        let args: Vec<Val> = soroban_sdk::vec![env, current_address.to_val()];
        diamond_proxy.require_auth_for_args(args);

        Ok(())
    }

    /// Security check: Verify authorization
    /// This ensures calls only come through the diamond proxy
    fn validate_authorization(env: &Env) -> Result<(), Error> {
        // Verify this call is authorized through the diamond proxy
        Self::require_diamond_proxy_caller(env)?;

        Ok(())
    }

    pub fn get_instance_shared_storage_at(env: Env, key: Bytes) -> Option<Bytes> {
        // Security check
        if Self::validate_authorization(&env).is_err() {
            return None;
        }

        env.storage()
            .instance()
            .get(&key.concat(&env, INSTANCE_SHARED_STORAGE_KEY))
    }

    pub fn set_instance_shared_storage_at(env: Env, key: Bytes, value: Bytes) -> Result<(), Error> {
        // Security check
        Self::validate_authorization(&env)?;

        env.storage()
            .instance()
            .set(&key.concat(&env, INSTANCE_SHARED_STORAGE_KEY), &value);
        Ok(())
    }

    pub fn del_instance_shared_storage_at(env: Env, key: Bytes) -> Result<(), Error> {
        // Security check
        Self::validate_authorization(&env)?;

        env.storage()
            .instance()
            .remove(&key.concat(&env, INSTANCE_SHARED_STORAGE_KEY));
        Ok(())
    }

    pub fn get_persistent_shared_storage_at(env: Env, key: Bytes) -> Option<Bytes> {
        // Security check
        if Self::validate_authorization(&env).is_err() {
            return None;
        }

        env.storage()
            .persistent()
            .get(&key.concat(&env, PERSISTENT_SHARED_STORAGE_KEY))
    }

    pub fn set_persistent_shared_storage_at(
        env: Env,
        key: Bytes,
        value: Bytes,
    ) -> Result<(), Error> {
        // Security check
        Self::validate_authorization(&env)?;

        env.storage()
            .persistent()
            .set(&key.concat(&env, PERSISTENT_SHARED_STORAGE_KEY), &value);
        Ok(())
    }

    pub fn del_persistent_shared_storage_at(env: Env, key: Bytes) -> Result<(), Error> {
        // Security check
        Self::validate_authorization(&env)?;

        env.storage()
            .persistent()
            .remove(&key.concat(&env, PERSISTENT_SHARED_STORAGE_KEY));
        Ok(())
    }

    pub fn get_temporary_shared_storage_at(env: Env, key: Bytes) -> Option<Bytes> {
        // Security check
        if Self::validate_authorization(&env).is_err() {
            return None;
        }

        env.storage()
            .temporary()
            .get(&key.concat(&env, TEMPORARY_SHARED_STORAGE_KEY))
    }

    pub fn set_temporary_shared_storage_at(
        env: Env,
        key: Bytes,
        value: Bytes,
    ) -> Result<(), Error> {
        // Security check
        Self::validate_authorization(&env)?;

        env.storage()
            .temporary()
            .set(&key.concat(&env, TEMPORARY_SHARED_STORAGE_KEY), &value);
        Ok(())
    }

    pub fn del_temporary_shared_storage_at(env: Env, key: Bytes) -> Result<(), Error> {
        // Security check
        Self::validate_authorization(&env)?;

        env.storage()
            .temporary()
            .remove(&key.concat(&env, TEMPORARY_SHARED_STORAGE_KEY));
        Ok(())
    }
}
