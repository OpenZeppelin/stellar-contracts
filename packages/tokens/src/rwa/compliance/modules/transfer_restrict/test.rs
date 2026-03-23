extern crate std;

use soroban_sdk::{contract, contractimpl, testutils::Address as _, vec, Address, Env};

use super::*;
use crate::rwa::compliance::modules::storage::set_compliance_address;

#[contract]
struct TestTransferRestrictContract;

#[contractimpl(contracttrait)]
impl TransferRestrict for TestTransferRestrictContract {
    fn set_compliance_address(_e: &Env, _compliance: Address) {
        unreachable!("set_compliance_address is not used in these tests");
    }
}

#[test]
fn can_transfer_allows_sender_or_recipient_when_allowlisted() {
    let e = Env::default();
    e.mock_all_auths();

    let module_id = e.register(TestTransferRestrictContract, ());
    let compliance = Address::generate(&e);
    let token = Address::generate(&e);
    let sender = Address::generate(&e);
    let recipient = Address::generate(&e);
    let outsider = Address::generate(&e);
    let client = TestTransferRestrictContractClient::new(&e, &module_id);

    e.as_contract(&module_id, || {
        set_compliance_address(&e, &compliance);
    });

    assert!(!client.can_transfer(&sender.clone(), &recipient.clone(), &100, &token));

    client.allow_user(&token, &sender.clone());
    assert!(client.can_transfer(&sender.clone(), &outsider.clone(), &100, &token));

    client.disallow_user(&token, &sender.clone());
    client.allow_user(&token, &recipient.clone());
    assert!(client.can_transfer(&outsider, &recipient, &100, &token));
}

#[test]
fn batch_allow_and_disallow_update_allowlist_entries() {
    let e = Env::default();
    e.mock_all_auths();

    let module_id = e.register(TestTransferRestrictContract, ());
    let compliance = Address::generate(&e);
    let token = Address::generate(&e);
    let user_a = Address::generate(&e);
    let user_b = Address::generate(&e);
    let client = TestTransferRestrictContractClient::new(&e, &module_id);

    e.as_contract(&module_id, || {
        set_compliance_address(&e, &compliance);
    });

    client.batch_allow_users(&token, &vec![&e, user_a.clone(), user_b.clone()]);

    assert!(client.is_user_allowed(&token, &user_a.clone()));
    assert!(client.is_user_allowed(&token, &user_b.clone()));

    client.batch_disallow_users(&token, &vec![&e, user_a.clone(), user_b.clone()]);

    assert!(!client.is_user_allowed(&token, &user_a));
    assert!(!client.is_user_allowed(&token, &user_b));
}
