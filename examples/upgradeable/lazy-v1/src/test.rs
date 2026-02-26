extern crate std;

use soroban_sdk::{testutils::Address as _, Address, BytesN, Env, Symbol};

use crate::contract::{LeaderboardContract, LeaderboardContractClient};

mod contract_v2 {
    soroban_sdk::contractimport!(file = "../testdata/upgradeable_lazy_v2_example.wasm");
}

fn install_new_wasm(e: &Env) -> BytesN<32> {
    e.deployer().upload_contract_wasm(contract_v2::WASM)
}

#[test]
fn test_lazy_migration() {
    let e = Env::default();
    e.mock_all_auths();

    let admin = Address::generate(&e);
    let manager = Address::generate(&e);
    let alice = Address::generate(&e);
    let bob = Address::generate(&e);

    // Deploy v1 and set scores for two users.
    let address = e.register(LeaderboardContract, (&admin,));
    let client_v1 = LeaderboardContractClient::new(&e, &address);

    client_v1.set_score(&alice, &100u32);
    client_v1.set_score(&bob, &200u32);
    assert_eq!(client_v1.get_score(&alice), 100);

    // Upgrade to v2.
    client_v1.grant_role(&manager, &Symbol::new(&e, "manager"), &admin);
    let new_wasm_hash = install_new_wasm(&e);
    client_v1.upgrade(&new_wasm_hash, &manager);

    let client_v2 = contract_v2::Client::new(&e, &address);

    // Alice's score was stored in v1 format (no `level` field, no version
    // marker). The first read triggers lazy migration: the v1 entry is read
    // as `ScoreV1`, converted to `Score { points, level: 0 }`, and written
    // back so all subsequent reads skip the conversion entirely.
    let alice_score = client_v2.get_score(&alice);
    assert_eq!(alice_score.points, 100);
    assert_eq!(alice_score.level, 0);

    // Bob's score: same lazy migration on first access.
    let bob_score = client_v2.get_score(&bob);
    assert_eq!(bob_score.points, 200);
    assert_eq!(bob_score.level, 0);

    // Charlie: new user after the upgrade, written directly in v2 format.
    let charlie = Address::generate(&e);
    client_v2.set_score(&charlie, &300u32, &5u32);
    let charlie_score = client_v2.get_score(&charlie);
    assert_eq!(charlie_score.points, 300);
    assert_eq!(charlie_score.level, 5);

    // Alice's second read: the entry is now in v2 format, no conversion needed.
    let alice_score_again = client_v2.get_score(&alice);
    assert_eq!(alice_score_again.points, 100);
    assert_eq!(alice_score_again.level, 0);
}
