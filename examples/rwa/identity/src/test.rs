extern crate std;

use soroban_sdk::{
    contract, contractimpl, symbol_short, testutils::Address as _, Address, Bytes, BytesN, Env,
    IntoVal, String, Symbol,
};

const NOT_VALID: Symbol = symbol_short!("not_valid");
use stellar_tokens::rwa::claim_issuer::ClaimIssuer;

use crate::contract::{IdentityContract, IdentityContractClient};

// ==================== Mock Claim Issuer ====================

mod mock_claim_issuer {
    use soroban_sdk::panic_with_error;
    use stellar_tokens::rwa::identity_claims::ClaimsError;

    use super::*;

    #[contract]
    pub struct Contract;

    #[contractimpl]
    impl ClaimIssuer for Contract {
        fn is_claim_valid(
            e: &Env,
            _identity: Address,
            _claim_topic: u32,
            _scheme: u32,
            _sig_data: Bytes,
            _claim_data: Bytes,
        ) {
            if e.storage().persistent().get(&NOT_VALID).unwrap_or(false) {
                panic_with_error!(e, ClaimsError::ClaimNotValid)
            }
        }
    }
}

// ==================== Test Helpers ====================

struct TestSetup<'a> {
    env: Env,
    client: IdentityContractClient<'a>,
    non_owner: Address,
    issuer: Address,
}

fn setup() -> TestSetup<'static> {
    let env = Env::default();
    env.mock_all_auths();

    let owner = Address::generate(&env);
    let non_owner = Address::generate(&env);
    let contract_id = env.register(IdentityContract, (&owner,));
    let client = IdentityContractClient::new(&env, &contract_id);
    let issuer = env.register(mock_claim_issuer::Contract, ());

    TestSetup { env, client, non_owner, issuer }
}

fn test_claim_data(e: &Env) -> (u32, u32, Bytes, Bytes, String) {
    let topic = 1u32;
    let scheme = 1u32;
    let signature = Bytes::from_array(e, &[1, 2, 3, 4]);
    let data = Bytes::from_array(e, &[5, 6, 7, 8]);
    let uri = String::from_str(e, "https://example.com");
    (topic, scheme, signature, data, uri)
}

// ==================== Tests ====================

#[test]
fn add_claim_by_owner_works() {
    let setup = setup();
    let (topic, scheme, signature, data, uri) = test_claim_data(&setup.env);

    let claim_id = setup.client.add_claim(&topic, &scheme, &setup.issuer, &signature, &data, &uri);

    let claim = setup.client.get_claim(&claim_id);
    assert_eq!(claim.topic, topic);
    assert_eq!(claim.scheme, scheme);
    assert_eq!(claim.issuer, setup.issuer);
    assert_eq!(claim.signature, signature);
    assert_eq!(claim.data, data);
    assert_eq!(claim.uri, uri);
}

#[test]
#[should_panic(expected = "Error(Auth, InvalidAction)")]
fn add_claim_by_non_owner_panics() {
    let setup = setup();
    let env = &setup.env;

    let (topic, scheme, signature, data, uri) = test_claim_data(env);

    // Only authenticate as non_owner — should fail ownership check
    env.mock_auths(&[soroban_sdk::testutils::MockAuth {
        address: &setup.non_owner,
        invoke: &soroban_sdk::testutils::MockAuthInvoke {
            contract: &setup.client.address,
            fn_name: "add_claim",
            args: (
                topic,
                scheme,
                setup.issuer.clone(),
                signature.clone(),
                data.clone(),
                uri.clone(),
            )
                .into_val(env),
            sub_invokes: &[],
        },
    }]);

    setup.client.add_claim(&topic, &scheme, &setup.issuer, &signature, &data, &uri);
}

#[test]
#[should_panic(expected = "Error(Contract, #341)")]
fn add_claim_invalid_issuer_panics() {
    let setup = setup();
    let (topic, scheme, signature, data, uri) = test_claim_data(&setup.env);

    // Set mock issuer to reject claims
    setup.env.as_contract(&setup.issuer, || {
        setup.env.storage().persistent().set(&NOT_VALID, &true);
    });

    setup.client.add_claim(&topic, &scheme, &setup.issuer, &signature, &data, &uri);
}

#[test]
fn get_claim_works() {
    let setup = setup();
    let (topic, scheme, signature, data, uri) = test_claim_data(&setup.env);

    let claim_id = setup.client.add_claim(&topic, &scheme, &setup.issuer, &signature, &data, &uri);

    let claim = setup.client.get_claim(&claim_id);
    assert_eq!(claim.topic, topic);
    assert_eq!(claim.issuer, setup.issuer);
}

#[test]
fn get_claim_ids_by_topic_works() {
    let setup = setup();
    let (topic, scheme, signature, data, uri) = test_claim_data(&setup.env);

    let claim_id = setup.client.add_claim(&topic, &scheme, &setup.issuer, &signature, &data, &uri);

    let ids = setup.client.get_claim_ids_by_topic(&topic);
    assert_eq!(ids.len(), 1);
    assert_eq!(ids.get(0).unwrap(), claim_id);

    // Different topic should be empty
    let ids_empty = setup.client.get_claim_ids_by_topic(&99u32);
    assert_eq!(ids_empty.len(), 0);
}

#[test]
#[should_panic(expected = "Error(Contract, #340)")]
fn get_nonexistent_claim_panics() {
    let setup = setup();
    let fake_id = BytesN::from_array(&setup.env, &[0u8; 32]);
    setup.client.get_claim(&fake_id);
}

#[test]
fn remove_claim_by_owner_works() {
    let setup = setup();
    let (topic, scheme, signature, data, uri) = test_claim_data(&setup.env);

    let claim_id = setup.client.add_claim(&topic, &scheme, &setup.issuer, &signature, &data, &uri);

    // Verify it exists
    let ids = setup.client.get_claim_ids_by_topic(&topic);
    assert_eq!(ids.len(), 1);

    setup.client.remove_claim(&claim_id);

    // Verify removal from topic index
    let ids_after = setup.client.get_claim_ids_by_topic(&topic);
    assert_eq!(ids_after.len(), 0);
}

#[test]
#[should_panic(expected = "Error(Auth, InvalidAction)")]
fn remove_claim_by_non_owner_panics() {
    let setup = setup();
    let env = &setup.env;
    let (topic, scheme, signature, data, uri) = test_claim_data(env);

    let claim_id = setup.client.add_claim(&topic, &scheme, &setup.issuer, &signature, &data, &uri);

    // Only authenticate as non_owner — should fail ownership check
    env.mock_auths(&[soroban_sdk::testutils::MockAuth {
        address: &setup.non_owner,
        invoke: &soroban_sdk::testutils::MockAuthInvoke {
            contract: &setup.client.address,
            fn_name: "remove_claim",
            args: (claim_id.clone(),).into_val(env),
            sub_invokes: &[],
        },
    }]);

    setup.client.remove_claim(&claim_id);
}
