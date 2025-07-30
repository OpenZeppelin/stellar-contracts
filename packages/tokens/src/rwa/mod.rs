pub mod identity_storage_registry;

use soroban_sdk::{Address, Env, Vec};

/// TODO: describe the trait
pub trait RWA {}

pub trait TokenBinder {
    fn linked_tokens(e: &Env) -> Vec<Address>;

    fn bind_token(e: &Env, token: Address, operator: Address);

    fn unbind_token(e: &Env, token: Address, operator: Address);
}
