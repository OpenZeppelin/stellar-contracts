use soroban_sdk::{Address, Env, Vec};

mod storage;

pub trait TokenBinder {
    fn linked_tokens(e: &Env) -> Vec<Address>;

    fn bind_token(e: &Env, token: Address, operator: Address);

    fn unbind_token(e: &Env, token: Address, operator: Address);
}

pub use storage::*;
