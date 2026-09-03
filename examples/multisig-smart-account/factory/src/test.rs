extern crate std;

use soroban_sdk::{
    auth::Context,
    contract, contractimpl, contracttype, map,
    testutils::Address as _,
    vec,
    xdr::{ScErrorCode, ScErrorType},
    Address, Bytes, BytesN, Env, Error, IntoVal, Map, TryFromVal, Val, Vec,
};
use stellar_accounts::{
    policies::Policy,
    smart_account::{ContextRule, Signer},
};

use crate::contract::{AccountFactoryContract, AccountFactoryContractClient};

mod account {
    soroban_sdk::contractimport!(file = "testdata/multisig_account_example.wasm");
}

#[contract]
struct MockVerifierContract;

#[contractimpl]
impl MockVerifierContract {
    pub fn verify(_e: &Env, _hash: Bytes, _key_data: Val, _sig_data: Val) -> bool {
        true
    }

    pub fn canonicalize_key(e: &Env, key_data: Val) -> Bytes {
        Bytes::try_from_val(e, &key_data).unwrap()
    }

    pub fn batch_canonicalize_key(e: &Env, key_data: Vec<Val>) -> Vec<Bytes> {
        Vec::from_iter(e, key_data.iter().map(|key| Bytes::try_from_val(e, &key).unwrap()))
    }
}

#[contracttype]
enum MockPolicyStorageKey {
    Installed(Address, u32),
}

#[contract]
struct MockPolicyContract;

#[contractimpl]
impl Policy for MockPolicyContract {
    type AccountParams = Val;

    fn enforce(
        _e: &Env,
        _context: Context,
        _authenticated_signers: Vec<Signer>,
        _rule: ContextRule,
        _smart_account: Address,
    ) {
    }

    fn install(e: &Env, install_params: Val, rule: ContextRule, smart_account: Address) {
        smart_account.require_auth();
        e.storage()
            .persistent()
            .set(&MockPolicyStorageKey::Installed(smart_account, rule.id), &install_params);
    }

    fn uninstall(_e: &Env, _rule: ContextRule, _smart_account: Address) {}
}

#[contractimpl]
impl MockPolicyContract {
    pub fn install_params(e: &Env, smart_account: Address, rule_id: u32) -> Val {
        e.storage()
            .persistent()
            .get(&MockPolicyStorageKey::Installed(smart_account, rule_id))
            .unwrap()
    }
}

fn setup(e: &Env) -> (Address, Address, Address) {
    let wasm_hash = e.deployer().upload_contract_wasm(account::WASM);
    let factory = e.register(AccountFactoryContract, (wasm_hash,));
    let verifier = e.register(MockVerifierContract, ());
    let policy = e.register(MockPolicyContract, ());
    (factory, verifier, policy)
}

fn external(e: &Env, verifier: &Address, key: u8) -> Signer {
    Signer::External(verifier.clone(), Bytes::from_array(e, &[key; 32]))
}

fn no_policies(e: &Env) -> Map<Address, Val> {
    Map::new(e)
}

fn deployed_signers(e: &Env, account: &Address) -> Vec<Signer> {
    let rule = account::Client::new(e, account).get_context_rule(&0);
    Vec::<Signer>::try_from_val(e, &rule.signers.to_val()).unwrap()
}

fn deployed_policies(e: &Env, account: &Address) -> Vec<Address> {
    account::Client::new(e, account).get_context_rule(&0).policies
}

#[test]
fn pinned_account_wasm_hash_returns_the_constructor_argument() {
    let e = Env::default();
    let wasm_hash = e.deployer().upload_contract_wasm(account::WASM);
    let factory = e.register(AccountFactoryContract, (wasm_hash.clone(),));
    let client = AccountFactoryContractClient::new(&e, &factory);

    assert_eq!(client.pinned_account_wasm_hash(), wasm_hash);
}

#[test]
fn predict_address_and_deploy_agree() {
    let e = Env::default();
    let (factory, verifier, _) = setup(&e);
    let client = AccountFactoryContractClient::new(&e, &factory);
    let signers = vec![&e, external(&e, &verifier, 1)];

    let predicted = client.predict_address(&signers, &no_policies(&e), &0);
    let deployed = client.deploy(&signers, &no_policies(&e), &0);

    assert_eq!(deployed, predicted);
    assert_eq!(deployed_signers(&e, &deployed), signers);
}

#[test]
fn deployed_account_holds_exactly_the_requested_configuration() {
    let e = Env::default();
    let (factory, verifier, policy) = setup(&e);
    let client = AccountFactoryContractClient::new(&e, &factory);
    let a = external(&e, &verifier, 1);
    let b = external(&e, &verifier, 2);
    let policies = map![&e, (policy.clone(), 2u32.into_val(&e))];

    let deployed = client.deploy(&vec![&e, b.clone(), a.clone()], &policies, &0);

    let signers = deployed_signers(&e, &deployed);
    assert_eq!(signers.len(), 2);
    assert!(signers.contains(&a));
    assert!(signers.contains(&b));
    assert_eq!(deployed_policies(&e, &deployed), vec![&e, policy.clone()]);

    let installed: u32 =
        MockPolicyContractClient::new(&e, &policy).install_params(&deployed, &0).into_val(&e);
    assert_eq!(installed, 2);
}

#[test]
fn signer_order_and_duplicates_do_not_change_the_address() {
    let e = Env::default();
    let (factory, verifier, _) = setup(&e);
    let client = AccountFactoryContractClient::new(&e, &factory);
    let a = external(&e, &verifier, 1);
    let b = external(&e, &verifier, 2);

    let ab = client.predict_address(&vec![&e, a.clone(), b.clone()], &no_policies(&e), &0);
    let ba = client.predict_address(&vec![&e, b.clone(), a.clone()], &no_policies(&e), &0);
    assert_eq!(ab, ba);

    let aa = client.predict_address(&vec![&e, a.clone(), a.clone()], &no_policies(&e), &0);
    let a_only = client.predict_address(&vec![&e, a.clone()], &no_policies(&e), &0);
    assert_eq!(aa, a_only);

    let deployed = client.deploy(&vec![&e, a.clone(), a.clone()], &no_policies(&e), &0);
    assert_eq!(deployed, a_only);
    assert_eq!(deployed_signers(&e, &deployed), vec![&e, a]);
}

#[test]
fn policy_map_order_does_not_change_the_address() {
    let e = Env::default();
    let (factory, verifier, policy) = setup(&e);
    let client = AccountFactoryContractClient::new(&e, &factory);
    let other_policy = e.register(MockPolicyContract, ());
    let signers = vec![&e, external(&e, &verifier, 1)];

    let one_way =
        map![&e, (policy.clone(), 1u32.into_val(&e)), (other_policy.clone(), 2u32.into_val(&e))];
    let other_way = map![&e, (other_policy, 2u32.into_val(&e)), (policy, 1u32.into_val(&e))];

    assert_eq!(
        client.predict_address(&signers, &one_way, &0),
        client.predict_address(&signers, &other_way, &0)
    );
}

#[test]
fn every_part_of_the_tuple_changes_the_address() {
    let e = Env::default();
    let (factory, verifier, policy) = setup(&e);
    let client = AccountFactoryContractClient::new(&e, &factory);
    let a = external(&e, &verifier, 1);
    let b = external(&e, &verifier, 2);
    let delegated = Signer::Delegated(Address::generate(&e));

    let addresses = [
        client.predict_address(&vec![&e, a.clone()], &no_policies(&e), &0),
        client.predict_address(&vec![&e, b.clone()], &no_policies(&e), &0),
        client.predict_address(&vec![&e, a.clone(), b.clone()], &no_policies(&e), &0),
        client.predict_address(
            &vec![&e, a.clone()],
            &map![&e, (policy.clone(), 1u32.into_val(&e))],
            &0,
        ),
        client.predict_address(
            &vec![&e, a.clone()],
            &map![&e, (policy.clone(), 2u32.into_val(&e))],
            &0,
        ),
        client.predict_address(&vec![&e, a.clone()], &no_policies(&e), &1),
        client.predict_address(&vec![&e, delegated], &no_policies(&e), &0),
    ];

    for (i, address) in addresses.iter().enumerate() {
        for other in &addresses[i + 1..] {
            assert_ne!(address, other);
        }
    }
}

#[test]
fn extra_salt_gives_one_configuration_several_accounts() {
    let e = Env::default();
    let (factory, verifier, _) = setup(&e);
    let client = AccountFactoryContractClient::new(&e, &factory);
    let signers = vec![&e, external(&e, &verifier, 1)];

    let first = client.deploy(&signers, &no_policies(&e), &0);
    let second = client.deploy(&signers, &no_policies(&e), &1);

    assert_ne!(first, second);
    assert_eq!(deployed_signers(&e, &first), signers);
    assert_eq!(deployed_signers(&e, &second), signers);
}

#[test]
fn deploying_the_same_tuple_twice_traps() {
    let e = Env::default();
    let (factory, verifier, _) = setup(&e);
    let client = AccountFactoryContractClient::new(&e, &factory);
    let signers = vec![&e, external(&e, &verifier, 1)];

    let deployed = client.deploy(&signers, &no_policies(&e), &0);

    let again = client.try_deploy(&signers, &no_policies(&e), &0);
    assert_eq!(
        again,
        Err(Ok(Error::from_type_and_code(ScErrorType::Context, ScErrorCode::InvalidAction)))
    );
    assert_eq!(client.predict_address(&signers, &no_policies(&e), &0), deployed);
}

#[test]
fn account_constructor_errors_fail_deployment() {
    let e = Env::default();
    let (factory, _, _) = setup(&e);
    let client = AccountFactoryContractClient::new(&e, &factory);

    // Empty signers and policies fails account construction
    let res = client.try_deploy(&Vec::new(&e), &no_policies(&e), &0);
    assert_eq!(
        res,
        Err(Ok(Error::from_type_and_code(ScErrorType::Context, ScErrorCode::InvalidAction)))
    );
}

#[test]
fn every_deploy_uses_the_pinned_wasm() {
    let e = Env::default();
    let (factory, verifier, _) = setup(&e);
    let client = AccountFactoryContractClient::new(&e, &factory);
    let signers = vec![&e, external(&e, &verifier, 1)];

    let bogus_hash = BytesN::from_array(&e, &[7u8; 32]);
    let other_factory = e.register(AccountFactoryContract, (bogus_hash.clone(),));
    let other_client = AccountFactoryContractClient::new(&e, &other_factory);
    assert_eq!(other_client.pinned_account_wasm_hash(), bogus_hash);

    let other_predicted = other_client.predict_address(&signers, &no_policies(&e), &0);
    assert!(other_client.try_deploy(&signers, &no_policies(&e), &0).is_err());

    assert_ne!(client.predict_address(&signers, &no_policies(&e), &0), other_predicted);
}
