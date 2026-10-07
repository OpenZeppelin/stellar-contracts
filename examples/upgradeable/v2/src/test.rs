extern crate std;

use soroban_sdk::{contracttype, testutils::Address as _, Address, BytesN, Env, Symbol};

use crate::contract::ExampleContractClient;

// v1 is built with soroban-sdk 26, whose test host cannot run the v2 wasm, so
// the upgrade is tested from here.
mod contract_v1 {
    soroban_sdk::contractimport!(file = "../testdata/upgradeable_v1_example.wasm");
}

/// The key under which v1 stores the total supply in `instance` storage.
#[contracttype]
enum LegacyStorageKey {
    TotalSupply,
}

const V2_WASM: &[u8] = include_bytes!("../../testdata/upgradeable_v2_example.wasm");

fn install_new_wasm(e: &Env) -> BytesN<32> {
    e.deployer().upload_contract_wasm(V2_WASM)
}

fn has_legacy_supply(e: &Env, address: &Address) -> bool {
    e.as_contract(address, || e.storage().instance().has(&LegacyStorageKey::TotalSupply))
}

struct Setup {
    address: Address,
    admin: Address,
    minter: Address,
    manager: Address,
    migrator: Address,
    user: Address,
}

fn setup(e: &Env) -> Setup {
    let admin = Address::generate(e);
    let minter = Address::generate(e);
    let manager = Address::generate(e);
    let migrator = Address::generate(e);
    let user = Address::generate(e);

    let address = e.register(contract_v1::WASM, (&admin, &1000i128));
    let client_v1 = contract_v1::Client::new(e, &address);
    client_v1.grant_role(&minter, &Symbol::new(e, "minter"), &admin);
    client_v1.grant_role(&manager, &Symbol::new(e, "manager"), &admin);
    client_v1.grant_role(&migrator, &Symbol::new(e, "migrator"), &admin);

    Setup { address, admin, minter, manager, migrator, user }
}

#[test]
fn test_upgrade_and_migrate() {
    let e = Env::default();
    e.mock_all_auths();
    let Setup { address, admin, minter, manager, migrator, user } = setup(&e);

    // v1 keeps the supply in instance storage: 1000 + 500 - 100
    let client_v1 = contract_v1::Client::new(&e, &address);
    client_v1.mint(&user, &500, &minter);
    client_v1.burn(&user, &100);
    assert_eq!(client_v1.total_supply(), 1400);
    assert!(has_legacy_supply(&e, &address));

    client_v1.upgrade(&install_new_wasm(&e), &manager);

    // before the migration, v2 reads the supply from a persistent entry that
    // does not exist yet, and any burn exceeds the recorded supply
    let client_v2 = ExampleContractClient::new(&e, &address);
    assert_eq!(client_v2.total_supply(), 0);
    assert!(client_v2.try_burn(&user, &1).is_err());

    // balances need no migration
    assert_eq!(client_v2.balance(&admin), 1000);
    assert_eq!(client_v2.balance(&user), 400);

    // migrate: moves the supply to the persistent entry
    client_v2.migrate(&migrator);
    assert_eq!(client_v2.total_supply(), 1400);
    assert!(!has_legacy_supply(&e, &address));

    // mints and burns keep the migrated supply up to date
    client_v2.mint(&user, &200, &minter);
    client_v2.burn(&user, &50);
    assert_eq!(client_v2.total_supply(), 1550);

    // ensure migrate can't be invoked again (schema version guard)
    assert!(client_v2.try_migrate(&migrator).is_err());
}

#[test]
fn test_mint_between_upgrade_and_migrate() {
    let e = Env::default();
    e.mock_all_auths();
    let Setup { address, minter, manager, migrator, user, .. } = setup(&e);

    contract_v1::Client::new(&e, &address).upgrade(&install_new_wasm(&e), &manager);

    // a mint lands in the new entry before the migration runs
    let client_v2 = ExampleContractClient::new(&e, &address);
    client_v2.mint(&user, &300, &minter);
    assert_eq!(client_v2.total_supply(), 300);

    // the migration adds the old supply on top instead of overwriting it
    client_v2.migrate(&migrator);
    assert_eq!(client_v2.total_supply(), 1300);
}
