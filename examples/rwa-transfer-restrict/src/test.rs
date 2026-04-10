extern crate std;

use soroban_sdk::{testutils::Address as _, vec, Address, Env, String};

use crate::contract::{TransferRestrictContract, TransferRestrictContractClient};

fn create_client<'a>(e: &Env, admin: &Address) -> (Address, TransferRestrictContractClient<'a>) {
    let address = e.register(TransferRestrictContract, (admin,));
    (address.clone(), TransferRestrictContractClient::new(e, &address))
}

#[test]
fn allowlist_methods_and_can_transfer_work() {
    let e = Env::default();
    e.mock_all_auths();
    let admin = Address::generate(&e);
    let token = Address::generate(&e);
    let sender = Address::generate(&e);
    let recipient = Address::generate(&e);
    let other = Address::generate(&e);
    let (_address, client) = create_client(&e, &admin);

    assert!(!client.is_user_allowed(&token, &sender));
    assert!(!client.can_transfer(&sender, &recipient, &1, &token));

    client.allow_user(&token, &sender);
    assert!(client.is_user_allowed(&token, &sender));
    assert!(client.can_transfer(&sender, &other, &1, &token));

    client.disallow_user(&token, &sender);
    assert!(!client.is_user_allowed(&token, &sender));

    client.batch_allow_users(&token, &vec![&e, recipient.clone(), other.clone()]);
    assert!(client.is_user_allowed(&token, &recipient));
    assert!(client.is_user_allowed(&token, &other));
    assert!(client.can_transfer(&sender, &recipient, &1, &token));

    client.batch_disallow_users(&token, &vec![&e, recipient.clone(), other.clone()]);
    assert!(!client.is_user_allowed(&token, &recipient));
    assert!(!client.is_user_allowed(&token, &other));
    assert!(!client.can_transfer(&sender, &recipient, &1, &token));
}

#[test]
fn name_and_compliance_address_work() {
    let e = Env::default();
    e.mock_all_auths();
    let admin = Address::generate(&e);
    let compliance = Address::generate(&e);
    let (_address, client) = create_client(&e, &admin);

    assert_eq!(client.name(), String::from_str(&e, "TransferRestrictModule"));

    client.set_compliance_address(&compliance);
    assert_eq!(client.get_compliance_address(), compliance);
}

#[test]
fn allow_user_uses_admin_auth_before_compliance_bind() {
    let e = Env::default();
    e.mock_all_auths();
    let admin = Address::generate(&e);
    let token = Address::generate(&e);
    let user = Address::generate(&e);
    let (_address, client) = create_client(&e, &admin);

    client.allow_user(&token, &user);

    let auths = e.auths();
    assert_eq!(auths.len(), 1);
    let (addr, _) = &auths[0];
    assert_eq!(addr, &admin);
}

#[test]
fn allow_user_uses_compliance_auth_after_bind() {
    let e = Env::default();
    e.mock_all_auths();
    let admin = Address::generate(&e);
    let compliance = Address::generate(&e);
    let token = Address::generate(&e);
    let user = Address::generate(&e);
    let (_address, client) = create_client(&e, &admin);

    client.set_compliance_address(&compliance);
    let auths = e.auths();
    assert_eq!(auths.len(), 1);
    let (addr, _) = &auths[0];
    assert_eq!(addr, &admin);

    client.allow_user(&token, &user);

    let auths = e.auths();
    assert_eq!(auths.len(), 1);
    let (addr, _) = &auths[0];
    assert_eq!(addr, &compliance);
}
