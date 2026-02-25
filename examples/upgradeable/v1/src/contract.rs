/// A basic contract that demonstrates how to implement the `Upgradeable` trait
/// directly. It stores a `Config` struct that will change shape in "v2",
/// demonstrating a realistic storage migration scenario.
use soroban_sdk::{
    contract, contractimpl, contracttype, symbol_short, Address, BytesN, Env, Symbol, Vec,
};
use stellar_access::access_control::{set_admin, AccessControl};
use stellar_contract_utils::upgradeable::{self as upgradeable, Upgradeable};
use stellar_macros::only_role;

#[contracttype]
pub struct Config {
    pub rate: u32,
}

pub const CONFIG_KEY: Symbol = symbol_short!("CONFIG");

#[contract]
pub struct ExampleContract;

#[contractimpl]
impl ExampleContract {
    pub fn __constructor(e: &Env, admin: Address, rate: u32) {
        set_admin(e, &admin);
        e.storage().instance().set(&CONFIG_KEY, &Config { rate });
    }

    pub fn get_rate(e: &Env) -> u32 {
        e.storage().instance().get::<_, Config>(&CONFIG_KEY).unwrap().rate
    }
}

#[contractimpl]
impl Upgradeable for ExampleContract {
    #[only_role(operator, "manager")]
    fn upgrade(e: &Env, new_wasm_hash: BytesN<32>, operator: Address) {
        upgradeable::upgrade(e, &new_wasm_hash);
    }
}

#[contractimpl(contracttrait)]
impl AccessControl for ExampleContract {}
