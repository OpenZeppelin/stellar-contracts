use soroban_sdk::{Address, Env, Symbol, Val, Vec};

pub trait ModularCompliance {
    fn bind_token(e: &Env, token: Address);

    fn unbind_token(e: &Env, token: Address);

    fn add_module(e: &Env, module: Address);

    fn remove_module(e: &Env, module: Address);

    fn call_module_function(e: &Env, module: Address, module_fn: Symbol, params: Vec<Val>);

    fn transferred(e: &Env, from: Address, to: Address, amount: i128);

    fn created(e: &Env, to: Address, amount: i128);

    fn destroyed(e: &Env, from: Address, amount: i128);

    fn can_transfer(e: &Env, from: Address, to: Address, amount: i128) -> bool;

    fn is_module_bound(e: &Env, module: Address) -> bool;

    fn get_modules(e: &Env) -> Vec<Address>;

    fn get_token_bound(e: &Env) -> Address;
}
