/// The contract in "v1" needs to be upgraded with this one. We demonstrate how
/// to implement `Upgradeable` directly and add a `migrate` function that uses
/// `upgradeable::run_migration()`. The `migrate()` function and its arguments
/// are completely customizable.
use soroban_sdk::{
    contract, contractimpl, contracttype, symbol_short, Address, BytesN, Env, Symbol, Vec,
};
use stellar_access::access_control::AccessControl;
use stellar_contract_utils::upgradeable::{self as upgradeable, Upgradeable};
use stellar_macros::only_role;

pub const DATA_KEY: Symbol = symbol_short!("DATA_KEY");

#[contracttype]
pub struct Data {
    pub num1: u32,
    pub num2: u32,
}

#[contract]
pub struct ExampleContract;

#[contractimpl]
impl Upgradeable for ExampleContract {
    #[only_role(operator, "manager")]
    fn upgrade(e: &Env, new_wasm_hash: BytesN<32>, operator: Address) {
        upgradeable::upgrade(e, &new_wasm_hash);
    }
}

#[contractimpl]
impl ExampleContract {
    #[only_role(operator, "migrator")]
    pub fn migrate(e: &Env, migration_data: Data, operator: Address) {
        upgradeable::run_migration(e, || {
            e.storage().instance().set(&DATA_KEY, &migration_data);
        });
    }
}

#[contractimpl(contracttrait)]
impl AccessControl for ExampleContract {}
