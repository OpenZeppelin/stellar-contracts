/// The token from "v1", built with the current library. It shows the storage
/// migration required when upgrading a token built with `stellar-tokens`
/// v0.7.x or older.
///
/// Up to v0.7.x, `Base` tracked the total supply in `instance` storage. The
/// current `Base` no longer tracks it: the opt-in `TotalSupply` contract type
/// does, in a `persistent` entry. Right after the upgrade, that entry does not
/// exist yet, so `total_supply()` returns `0` and burning panics, as the
/// burned amount exceeds the recorded supply. `migrate()` moves the stored
/// supply to its new place.
///
/// Balances, allowances, metadata, roles and the rest of the stored data keep
/// their layout and need no migration.
use soroban_sdk::{
    contract, contractimpl, Address, BytesN, Env, MuxedAddress, String, Symbol, Vec,
};
use stellar_access::access_control::AccessControl;
use stellar_contract_utils::upgradeable::{self as upgradeable, Upgradeable};
use stellar_macros::only_role;
use stellar_tokens::fungible::{
    burnable::FungibleBurnable,
    total_supply::{migrate_total_supply, FungibleTotalSupply, TotalSupply},
    Compose, FungibleToken,
};

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
    /// Moves the total supply from its v0.7.x `instance` entry to the
    /// `persistent` entry read by the `TotalSupply` contract type. A schema
    /// version prevents this from running twice.
    ///
    /// Tokens minted between the upgrade and this call stay accounted for,
    /// but burns in that window panic, so upgrading and migrating in the same
    /// transaction (see the "upgrader" example) is preferable.
    #[only_role(operator, "migrator")]
    pub fn migrate(e: &Env, operator: Address) {
        assert!(upgradeable::get_schema_version(e) < 2, "already migrated");

        migrate_total_supply(e);
        upgradeable::set_schema_version(e, 2);
    }

    #[only_role(caller, "minter")]
    pub fn mint(e: &Env, to: Address, amount: i128, caller: Address) {
        // Routed through the contract type so the total supply is tracked.
        <Self as FungibleToken>::ContractType::mint(e, &to, amount);
    }
}

#[contractimpl(contracttrait)]
impl FungibleToken for ExampleContract {
    type ContractType = Compose<(TotalSupply,)>;
}

#[contractimpl(contracttrait)]
impl FungibleTotalSupply for ExampleContract {}

#[contractimpl(contracttrait)]
impl FungibleBurnable for ExampleContract {}

#[contractimpl(contracttrait)]
impl AccessControl for ExampleContract {}
