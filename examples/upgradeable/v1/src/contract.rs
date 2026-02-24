/// A basic contract that demonstrates how to implement the `Upgradeable` trait
/// directly. The goal is to upgrade this "v1" contract with the contract in
/// "v2".
use soroban_sdk::{contract, contractimpl, Address, BytesN, Env, Symbol, Vec};
use stellar_access::access_control::{set_admin, AccessControl};
use stellar_contract_utils::upgradeable::{self as upgradeable, Upgradeable};
use stellar_macros::only_role;

#[contract]
pub struct ExampleContract;

#[contractimpl]
impl ExampleContract {
    pub fn __constructor(e: &Env, admin: Address) {
        set_admin(e, &admin);
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
