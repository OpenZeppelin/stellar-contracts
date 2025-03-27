use soroban_sdk::{contract, contracterror, contractimpl, contracttype, Address, Env, String};
use stellar_non_fungible::{
    consecutive::{overrides::Consecutive, storage::batch_mint},
    Balance, ContractOverrides, NonFungibleToken, TokenId,
};

#[contracttype]
pub enum DataKey {
    Owner,
}

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
    pub fn __constructor(e: &Env, owner: Address, recipient: Address, amount: Balance) {
        e.storage().instance().set(&DataKey::Owner, &owner);
        batch_mint(e, &recipient, amount);
    }
}

#[contractimpl]
impl NonFungibleToken for ExampleContract {
    type ContractType = Consecutive;

    fn name(e: &Env) -> String {
        String::from_str(e, "My Collection")
    }

    fn symbol(e: &Env) -> String {
        String::from_str(e, "MC NFT")
    }

    fn token_uri(e: &Env, _token_id: TokenId) -> String {
        String::from_str(e, "https:://smth.com/")
    }

    fn balance(e: &Env, owner: Address) -> Balance {
        stellar_non_fungible::balance(e, &owner)
    }

    fn owner_of(e: &Env, token_id: TokenId) -> Address {
        Self::ContractType::owner_of(e, token_id)
    }

    fn transfer(e: &Env, from: Address, to: Address, token_id: TokenId) {
        Self::ContractType::transfer(e, from, to, token_id);
    }

    fn transfer_from(e: &Env, spender: Address, from: Address, to: Address, token_id: TokenId) {
        Self::ContractType::transfer_from(e, spender, from, to, token_id);
    }

    fn approve(
        e: &Env,
        approver: Address,
        approved: Address,
        token_id: TokenId,
        live_until_ledger: u32,
    ) {
        Self::ContractType::approve(e, approver, approved, token_id, live_until_ledger);
    }

    fn approve_for_all(e: &Env, owner: Address, operator: Address, live_until_ledger: u32) {
        stellar_non_fungible::approve_for_all(e, &owner, &operator, live_until_ledger);
    }

    fn get_approved(e: &Env, token_id: TokenId) -> Option<Address> {
        stellar_non_fungible::get_approved(e, token_id)
    }

    fn is_approved_for_all(e: &Env, owner: Address, operator: Address) -> bool {
        stellar_non_fungible::is_approved_for_all(e, &owner, &operator)
    }
}
