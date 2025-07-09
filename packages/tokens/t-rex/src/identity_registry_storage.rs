use soroban_sdk::{Address, Env, Vec};

use crate::TokenBinder;

pub trait IdentityRegistryStorage: TokenBinder {
    fn add_identity(e: &Env, account: Address, identity: Address, operator: Address);

    fn remove_identity(e: &Env, account: Address, identity: Address, operator: Address);

    fn modify_identity(e: &Env, account: Address, identity: Address, operator: Address);

    fn linked_tokens(e: &Env) -> Vec<Address>;

    fn stored_identity(e: &Env, account: Address) -> Address;
}
