/// A fungible token built with `stellar-tokens` v0.7.2, the last release in
/// which `Base` tracked the total supply itself, in `instance` storage under
/// `FungibleStorageKey::TotalSupply`.
///
/// The contract in "v2" is the same token built with the current library,
/// where the supply is tracked by the opt-in `TotalSupply` contract type in
/// its own `persistent` entry. Upgrading to it requires migrating the stored
/// supply, see `migrate()` in "v2".
use soroban_sdk::{
    contract, contractimpl, Address, BytesN, Env, MuxedAddress, String, Symbol, Vec,
};
use stellar_access::access_control::{set_admin, AccessControl};
use stellar_contract_utils::upgradeable::{self as upgradeable, Upgradeable};
use stellar_macros::only_role;
use stellar_tokens::fungible::{burnable::FungibleBurnable, Base, FungibleToken};

#[contract]
pub struct ExampleContract;

#[contractimpl]
impl ExampleContract {
    pub fn __constructor(e: &Env, admin: Address, initial_supply: i128) {
        set_admin(e, &admin);
        Base::set_metadata(e, 7, String::from_str(e, "My Token"), String::from_str(e, "MTK"));
        Base::mint(e, &admin, initial_supply);
    }

    #[only_role(caller, "minter")]
    pub fn mint(e: &Env, to: Address, amount: i128, caller: Address) {
        Base::mint(e, &to, amount);
    }
}

#[contractimpl]
impl FungibleToken for ExampleContract {
    type ContractType = Base;

    fn total_supply(e: &Env) -> i128 {
        Self::ContractType::total_supply(e)
    }

    fn balance(e: &Env, account: Address) -> i128 {
        Self::ContractType::balance(e, &account)
    }

    fn allowance(e: &Env, owner: Address, spender: Address) -> i128 {
        Self::ContractType::allowance(e, &owner, &spender)
    }

    fn transfer(e: &Env, from: Address, to: MuxedAddress, amount: i128) {
        Self::ContractType::transfer(e, &from, &to, amount);
    }

    fn transfer_from(e: &Env, spender: Address, from: Address, to: Address, amount: i128) {
        Self::ContractType::transfer_from(e, &spender, &from, &to, amount);
    }

    fn approve(e: &Env, owner: Address, spender: Address, amount: i128, live_until_ledger: u32) {
        Self::ContractType::approve(e, &owner, &spender, amount, live_until_ledger);
    }

    fn decimals(e: &Env) -> u32 {
        Self::ContractType::decimals(e)
    }

    fn name(e: &Env) -> String {
        Self::ContractType::name(e)
    }

    fn symbol(e: &Env) -> String {
        Self::ContractType::symbol(e)
    }
}

#[contractimpl]
impl FungibleBurnable for ExampleContract {
    fn burn(e: &Env, from: Address, amount: i128) {
        Self::ContractType::burn(e, &from, amount)
    }

    fn burn_from(e: &Env, spender: Address, from: Address, amount: i128) {
        Self::ContractType::burn_from(e, &spender, &from, amount)
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
