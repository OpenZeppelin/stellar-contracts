/// Helper contract to perform upgrade+migrate in a single transaction.
use soroban_sdk::{contract, contractimpl, Address, BytesN, Env};
use stellar_access::Ownable;
use stellar_contract_utils::upgradeable::UpgradeableClient;
use stellar_macros::only_owner;

#[contract]
pub struct Upgrader;

#[soroban_sdk::contracttrait]
impl Ownable for Upgrader {}

#[contractimpl]
impl Upgrader {
    pub fn __constructor(e: &Env, owner: Address) {
        Self::set_owner(e, &owner);
    }

    #[only_owner]
    pub fn upgrade(e: &Env, contract_address: Address, wasm_hash: BytesN<32>) {
        UpgradeableClient::new(e, &contract_address).upgrade(&wasm_hash);
    }
}
