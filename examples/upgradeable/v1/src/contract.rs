/// A basic contract that demonstrates how to implement the `Upgradeable` trait
/// directly. The goal is to upgrade this "v1" contract with the contract in
/// "v2".
use soroban_sdk::{
    contract, contracterror, contractimpl, panic_with_error, symbol_short, Address, BytesN, Env,
    Symbol,
};
use stellar_contract_utils::upgradeable::{self as upgradeable, Upgradeable};

pub const OWNER: Symbol = symbol_short!("OWNER");

#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq, PartialOrd, Ord)]
#[repr(u32)]
pub enum ExampleContractError {
    Unauthorized = 1,
}

#[contract]
pub struct ExampleContract;

#[contractimpl]
impl ExampleContract {
    pub fn __constructor(e: &Env, admin: Address) {
        e.storage().instance().set(&OWNER, &admin);
    }
}

#[contractimpl]
impl Upgradeable for ExampleContract {
    fn upgrade(e: &Env, new_wasm_hash: BytesN<32>, operator: Address) {
        operator.require_auth();
        let owner = e.storage().instance().get::<_, Address>(&OWNER).unwrap();
        if operator != owner {
            panic_with_error!(e, ExampleContractError::Unauthorized)
        }
        upgradeable::upgrade(e, &new_wasm_hash);
    }
}
