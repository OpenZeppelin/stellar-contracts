#![allow(dead_code)]

use soroban_sdk::{
    contract, contractimpl, contracttype, testutils::Address as _, Address, Env, Vec,
};

use crate::rwa::identity_registry_storage::CountryData;

// ---------------------------------------------------------------------------
// MockIRS — minimal identity-registry-storage stub for module tests.
//
// Methods `stored_identity` and `get_country_data_entries` match the ABI
// that `IRSReadClient` calls cross-contract, so any module configured
// with a MockIRS address resolves identity/country transparently.
// ---------------------------------------------------------------------------

#[contract]
pub struct MockIRS;

#[contracttype]
#[derive(Clone)]
enum MockIRSKey {
    Identity(Address),
    Countries(Address),
}

#[contractimpl]
impl MockIRS {
    /// Returns the stored identity for `account`, defaulting to `account`
    /// itself (identity == wallet) when no explicit mapping exists.
    pub fn stored_identity(e: Env, account: Address) -> Address {
        e.storage()
            .persistent()
            .get(&MockIRSKey::Identity(account.clone()))
            .unwrap_or(account)
    }

    /// Returns country data entries for `account`, defaulting to empty vec.
    pub fn get_country_data_entries(e: Env, account: Address) -> Vec<CountryData> {
        e.storage()
            .persistent()
            .get(&MockIRSKey::Countries(account))
            .unwrap_or_else(|| Vec::new(&e))
    }

    // -- test helpers (not part of the IRS ABI, used by test setup) --

    pub fn mock_set_identity(e: Env, account: Address, identity: Address) {
        e.storage()
            .persistent()
            .set(&MockIRSKey::Identity(account), &identity);
    }

    pub fn mock_set_countries(e: Env, account: Address, data: Vec<CountryData>) {
        e.storage()
            .persistent()
            .set(&MockIRSKey::Countries(account), &data);
    }
}

// ---------------------------------------------------------------------------
// MockToken — supports configurable balances for modules that call
// `TokenBalanceViewClient::balance()` cross-contract (e.g., lockup module).
// ---------------------------------------------------------------------------

#[contract]
pub struct MockToken;

#[contracttype]
#[derive(Clone)]
enum MockTokenKey {
    Balance(Address),
}

#[contractimpl]
impl MockToken {
    pub fn balance(e: Env, id: Address) -> i128 {
        e.storage()
            .persistent()
            .get(&MockTokenKey::Balance(id))
            .unwrap_or_default()
    }

    pub fn mock_set_balance(e: Env, id: Address, amount: i128) {
        e.storage()
            .persistent()
            .set(&MockTokenKey::Balance(id), &amount);
    }
}

// ---------------------------------------------------------------------------
// Shared address factory
// ---------------------------------------------------------------------------

pub fn test_addresses(e: &Env) -> (Address, Address, Address) {
    (Address::generate(e), Address::generate(e), Address::generate(e))
}
