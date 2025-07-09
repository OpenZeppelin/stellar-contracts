use soroban_sdk::{Address, Env};

pub trait TokenBinder {
    fn bind_token(e: &Env, token: Address, operator: Address);

    fn unbind_token(e: &Env, token: Address, operator: Address);
}
