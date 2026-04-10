extern crate std;

use soroban_sdk::{contract, testutils::Address as _, vec, Address, Env};

use super::storage::{
    allow_user, batch_allow_users, batch_disallow_users, can_transfer, disallow_user,
    is_user_allowed,
};
use crate::rwa::compliance::modules::storage::set_compliance_address;

#[contract]
struct TestModuleContract;

#[test]
fn can_transfer_allows_sender_or_recipient_when_allowlisted() {
    let e = Env::default();

    let module_id = e.register(TestModuleContract, ());
    let compliance = Address::generate(&e);
    let token = Address::generate(&e);
    let sender = Address::generate(&e);
    let recipient = Address::generate(&e);
    let outsider = Address::generate(&e);

    e.as_contract(&module_id, || {
        set_compliance_address(&e, &compliance);

        assert!(!can_transfer(&e, &sender, &recipient, &token));

        allow_user(&e, &token, &sender);
        assert!(can_transfer(&e, &sender, &outsider, &token));

        disallow_user(&e, &token, &sender);
        allow_user(&e, &token, &recipient);
        assert!(can_transfer(&e, &outsider, &recipient, &token));
    });
}

#[test]
fn batch_allow_and_disallow_update_allowlist_entries() {
    let e = Env::default();

    let module_id = e.register(TestModuleContract, ());
    let compliance = Address::generate(&e);
    let token = Address::generate(&e);
    let user_a = Address::generate(&e);
    let user_b = Address::generate(&e);

    e.as_contract(&module_id, || {
        set_compliance_address(&e, &compliance);

        batch_allow_users(&e, &token, &vec![&e, user_a.clone(), user_b.clone()]);

        assert!(is_user_allowed(&e, &token, &user_a));
        assert!(is_user_allowed(&e, &token, &user_b));

        batch_disallow_users(&e, &token, &vec![&e, user_a.clone(), user_b.clone()]);

        assert!(!is_user_allowed(&e, &token, &user_a));
        assert!(!is_user_allowed(&e, &token, &user_b));
    });
}
