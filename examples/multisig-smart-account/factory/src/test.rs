extern crate std;

use soroban_sdk::{
    auth::Context,
    contract, contractimpl, contracttype, map,
    testutils::Address as _,
    vec,
    xdr::{ScErrorCode, ScErrorType},
    Address, Bytes, BytesN, Env, Error, IntoVal, Map, String, TryFromVal, Val, Vec,
};
use stellar_accounts::{
    policies::Policy,
    smart_account::{
        ContextRule, Signer, SmartAccountError, MAX_EXTERNAL_KEY_SIZE, MAX_NAME_SIZE, MAX_POLICIES,
        MAX_SIGNERS,
    },
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

/// Lets tests build two External keys that are not exactly equal, but still
/// count as the same signer — so we can check the factory rejects that as a
/// duplicate. Identity in this mock is the first 32 bytes.
#[contract]
struct MockCanonicalizingVerifierContract;

#[contractimpl]
impl MockCanonicalizingVerifierContract {
    pub fn verify(_e: &Env, _hash: Bytes, _key_data: Val, _sig_data: Val) -> bool {
        true
    }

    pub fn canonicalize_key(e: &Env, key_data: Val) -> Bytes {
        let key = Bytes::try_from_val(e, &key_data).unwrap();
        key.slice(..32)
    }

    pub fn batch_canonicalize_key(e: &Env, key_data: Vec<Val>) -> Vec<Bytes> {
        Vec::from_iter(e, key_data.iter().map(|key| Self::canonicalize_key(e, key)))
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

#[contract]
struct MockRejectingPolicyContract;

#[contractimpl]
impl Policy for MockRejectingPolicyContract {
    type AccountParams = Val;

    fn enforce(
        _e: &Env,
        _context: Context,
        _authenticated_signers: Vec<Signer>,
        _rule: ContextRule,
        _smart_account: Address,
    ) {
    }

    fn install(_e: &Env, _install_params: Val, _rule: ContextRule, _smart_account: Address) {
        panic!("install parameters rejected");
    }

    fn uninstall(_e: &Env, _rule: ContextRule, _smart_account: Address) {}
}

fn setup(e: &Env) -> (Address, Address, Address) {
    let wasm_hash = e.deployer().upload_contract_wasm(account::WASM);
    let factory = e.register(AccountFactoryContract, (wasm_hash,));
    let verifier = e.register(MockVerifierContract, ());
    let policy = e.register(MockPolicyContract, ());
    (factory, verifier, policy)
}

fn name(e: &Env) -> String {
    String::from_str(e, "multisig")
}

fn external(e: &Env, verifier: &Address, key: u8) -> Signer {
    Signer::External(verifier.clone(), Bytes::from_array(e, &[key; 32]))
}

fn no_policies(e: &Env) -> Map<Address, Val> {
    Map::new(e)
}

fn mock_policies(e: &Env, count: u32) -> Map<Address, Val> {
    let mut policies = Map::new(e);
    for _ in 0..count {
        policies.set(e.register(MockPolicyContract, ()), 1u32.into_val(e));
    }
    policies
}

fn deployed_signers(e: &Env, account: &Address) -> Vec<Signer> {
    let rule = account::Client::new(e, account).get_context_rule(&0);
    Vec::<Signer>::try_from_val(e, &rule.signers.to_val()).unwrap()
}

fn deployed_policies(e: &Env, account: &Address) -> Vec<Address> {
    account::Client::new(e, account).get_context_rule(&0).policies
}

fn assert_duplicate_signer(
    result: Result<
        Result<Address, soroban_sdk::ConversionError>,
        Result<Error, soroban_sdk::InvokeError>,
    >,
) {
    assert_eq!(
        result,
        Err(Ok(Error::from_contract_error(SmartAccountError::DuplicateSigner as u32)))
    );
}

fn assert_predict_and_deploy_reject(
    client: &AccountFactoryContractClient,
    name: &String,
    signers: &Vec<Signer>,
    policies: &Map<Address, Val>,
    error: SmartAccountError,
) {
    let expected = Err(Ok(Error::from_contract_error(error as u32)));
    assert_eq!(client.try_predict_address(name, signers, policies, &0), expected);
    assert_eq!(client.try_deploy(name, signers, policies, &0), expected);
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

    let predicted = client.predict_address(&name(&e), &signers, &no_policies(&e), &0);
    let deployed = client.deploy(&name(&e), &signers, &no_policies(&e), &0);

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

    let treasury = String::from_str(&e, "treasury");

    let deployed = client.deploy(&treasury, &vec![&e, b.clone(), a.clone()], &policies, &0);

    assert_eq!(account::Client::new(&e, &deployed).get_context_rule(&0).name, treasury);
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
fn signer_order_does_not_change_the_address() {
    let e = Env::default();
    let (factory, verifier, _) = setup(&e);
    let client = AccountFactoryContractClient::new(&e, &factory);
    let a = external(&e, &verifier, 1);
    let b = external(&e, &verifier, 2);

    let order_first_second =
        client.predict_address(&name(&e), &vec![&e, a.clone(), b.clone()], &no_policies(&e), &0);
    let order_second_first =
        client.predict_address(&name(&e), &vec![&e, b.clone(), a.clone()], &no_policies(&e), &0);
    assert_eq!(order_first_second, order_second_first);

    let deployed = client.deploy(&name(&e), &vec![&e, b.clone(), a.clone()], &no_policies(&e), &0);
    assert_eq!(deployed, order_first_second);
    let signers = deployed_signers(&e, &deployed);
    assert_eq!(signers.len(), 2);
    assert!(signers.contains(&a));
    assert!(signers.contains(&b));
}

#[test]
fn exact_duplicate_signers_are_rejected() {
    let e = Env::default();
    let (factory, verifier, _) = setup(&e);
    let client = AccountFactoryContractClient::new(&e, &factory);
    let a = external(&e, &verifier, 1);
    let b = external(&e, &verifier, 2);

    assert_duplicate_signer(client.try_predict_address(
        &name(&e),
        &vec![&e, a.clone(), a.clone()],
        &no_policies(&e),
        &0,
    ));
    assert_duplicate_signer(client.try_deploy(
        &name(&e),
        &vec![&e, a.clone(), a.clone()],
        &no_policies(&e),
        &0,
    ));
    assert_duplicate_signer(client.try_predict_address(
        &name(&e),
        &vec![&e, a.clone(), b.clone(), a.clone()],
        &no_policies(&e),
        &0,
    ));
}

#[test]
fn canonical_duplicate_external_signers_are_rejected() {
    let e = Env::default();
    let wasm_hash = e.deployer().upload_contract_wasm(account::WASM);
    let factory = e.register(AccountFactoryContract, (wasm_hash,));
    let verifier = e.register(MockCanonicalizingVerifierContract, ());
    let client = AccountFactoryContractClient::new(&e, &factory);

    // Same first 32 bytes, different suffixes — not equal, still a duplicate.
    let mut key_a = Bytes::from_array(&e, &[1u8; 32]);
    key_a.extend_from_array(&[0xAA; 8]);
    let mut key_b = Bytes::from_array(&e, &[1u8; 32]);
    key_b.extend_from_array(&[0xBB; 8]);

    let signers =
        vec![&e, Signer::External(verifier.clone(), key_a), Signer::External(verifier, key_b)];

    assert_duplicate_signer(client.try_predict_address(&name(&e), &signers, &no_policies(&e), &0));
    assert_duplicate_signer(client.try_deploy(&name(&e), &signers, &no_policies(&e), &0));
}

#[test]
fn names_longer_than_max_size_are_rejected() {
    let e = Env::default();
    let (factory, verifier, _) = setup(&e);
    let client = AccountFactoryContractClient::new(&e, &factory);
    let name_too_long = String::from_bytes(&e, &[b'a'; MAX_NAME_SIZE as usize + 1]);

    assert_predict_and_deploy_reject(
        &client,
        &name_too_long,
        &vec![&e, external(&e, &verifier, 1)],
        &no_policies(&e),
        SmartAccountError::NameTooLong,
    );
}

#[test]
fn external_keys_larger_than_max_size_are_rejected() {
    let e = Env::default();
    let (factory, verifier, _) = setup(&e);
    let client = AccountFactoryContractClient::new(&e, &factory);
    let key_too_large = Bytes::from_array(&e, &[1u8; MAX_EXTERNAL_KEY_SIZE as usize + 1]);

    assert_predict_and_deploy_reject(
        &client,
        &name(&e),
        &vec![&e, Signer::External(verifier, key_too_large)],
        &no_policies(&e),
        SmartAccountError::KeyDataTooLarge,
    );
}

#[test]
fn more_than_max_signers_are_rejected() {
    let e = Env::default();
    let (factory, verifier, _) = setup(&e);
    let client = AccountFactoryContractClient::new(&e, &factory);
    let signers =
        Vec::from_iter(&e, (0..=MAX_SIGNERS as u8).map(|key| external(&e, &verifier, key)));

    assert_predict_and_deploy_reject(
        &client,
        &name(&e),
        &signers,
        &no_policies(&e),
        SmartAccountError::TooManySigners,
    );
}

#[test]
fn more_than_max_policies_are_rejected() {
    let e = Env::default();
    let (factory, verifier, _) = setup(&e);
    let client = AccountFactoryContractClient::new(&e, &factory);

    assert_predict_and_deploy_reject(
        &client,
        &name(&e),
        &vec![&e, external(&e, &verifier, 1)],
        &mock_policies(&e, MAX_POLICIES + 1),
        SmartAccountError::TooManyPolicies,
    );
}

#[test]
fn rule_without_signers_or_policies_is_rejected() {
    let e = Env::default();
    let (factory, _, _) = setup(&e);
    let client = AccountFactoryContractClient::new(&e, &factory);

    assert_predict_and_deploy_reject(
        &client,
        &name(&e),
        &Vec::new(&e),
        &no_policies(&e),
        SmartAccountError::NoSignersAndPolicies,
    );
}

#[test]
fn tuple_at_every_limit_deploys_at_its_predicted_address() {
    let e = Env::default();
    let (factory, verifier, _) = setup(&e);
    let client = AccountFactoryContractClient::new(&e, &factory);
    let longest_name = String::from_bytes(&e, &[b'a'; MAX_NAME_SIZE as usize]);
    let mut signers =
        Vec::from_iter(&e, (1..MAX_SIGNERS as u8).map(|key| external(&e, &verifier, key)));
    signers.push_back(Signer::External(
        verifier.clone(),
        Bytes::from_array(&e, &[0u8; MAX_EXTERNAL_KEY_SIZE as usize]),
    ));
    let policies = mock_policies(&e, MAX_POLICIES);

    let predicted = client.predict_address(&longest_name, &signers, &policies, &0);
    let deployed = client.deploy(&longest_name, &signers, &policies, &0);

    assert_eq!(deployed, predicted);
    assert_eq!(deployed_signers(&e, &deployed).len(), MAX_SIGNERS);
    assert_eq!(deployed_policies(&e, &deployed).len(), MAX_POLICIES);
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
        client.predict_address(&name(&e), &signers, &one_way, &0),
        client.predict_address(&name(&e), &signers, &other_way, &0)
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
        client.predict_address(&name(&e), &vec![&e, a.clone()], &no_policies(&e), &0),
        client.predict_address(&name(&e), &vec![&e, b.clone()], &no_policies(&e), &0),
        client.predict_address(&name(&e), &vec![&e, a.clone(), b.clone()], &no_policies(&e), &0),
        client.predict_address(
            &name(&e),
            &vec![&e, a.clone()],
            &map![&e, (policy.clone(), 1u32.into_val(&e))],
            &0,
        ),
        client.predict_address(
            &name(&e),
            &vec![&e, a.clone()],
            &map![&e, (policy.clone(), 2u32.into_val(&e))],
            &0,
        ),
        client.predict_address(&name(&e), &vec![&e, a.clone()], &no_policies(&e), &1),
        client.predict_address(
            &String::from_str(&e, "treasury"),
            &vec![&e, a.clone()],
            &no_policies(&e),
            &0,
        ),
        client.predict_address(&name(&e), &vec![&e, delegated], &no_policies(&e), &0),
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

    let first = client.deploy(&name(&e), &signers, &no_policies(&e), &0);
    let second = client.deploy(&name(&e), &signers, &no_policies(&e), &1);

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

    let deployed = client.deploy(&name(&e), &signers, &no_policies(&e), &0);

    let again = client.try_deploy(&name(&e), &signers, &no_policies(&e), &0);
    assert_eq!(
        again,
        Err(Ok(Error::from_type_and_code(ScErrorType::Context, ScErrorCode::InvalidAction)))
    );
    assert_eq!(client.predict_address(&name(&e), &signers, &no_policies(&e), &0), deployed);
}

#[test]
fn policy_install_rejection_fails_deployment_but_not_prediction() {
    let e = Env::default();
    let (factory, verifier, _) = setup(&e);
    let client = AccountFactoryContractClient::new(&e, &factory);
    let signers = vec![&e, external(&e, &verifier, 1)];
    let policies = map![&e, (e.register(MockRejectingPolicyContract, ()), 1u32.into_val(&e))];

    assert!(client.try_predict_address(&name(&e), &signers, &policies, &0).is_ok());
    assert_eq!(
        client.try_deploy(&name(&e), &signers, &policies, &0),
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

    let other_predicted = other_client.predict_address(&name(&e), &signers, &no_policies(&e), &0);
    assert!(other_client.try_deploy(&name(&e), &signers, &no_policies(&e), &0).is_err());

    assert_ne!(client.predict_address(&name(&e), &signers, &no_policies(&e), &0), other_predicted);
}
