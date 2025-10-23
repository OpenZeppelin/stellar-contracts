extern crate std;

use soroban_sdk::{
    symbol_short,
    testutils::{Address as _, Ledger, MockAuth, MockAuthInvoke},
    vec, Address, Env, IntoVal, String, Symbol,
};

use crate::contract::{ExampleContract, ExampleContractClient};

fn create_client<'a>(e: &Env, admin: &Address) -> ExampleContractClient<'a> {
    let address = e.register(ExampleContract, (admin,));
    ExampleContractClient::new(e, &address)
}

pub struct TestAccounts {
    pub minter_admin: Address,
    pub burner_admin: Address,
    pub minter1: Address,
    pub minter2: Address,
    pub burner1: Address,
    pub burner2: Address,
    pub outsider: Address,
}

fn setup_roles(e: &Env, client: &ExampleContractClient, admin: &Address) -> TestAccounts {
    let minter_admin = Address::generate(e);
    let burner_admin = Address::generate(e);
    let minter1 = Address::generate(e);
    let minter2 = Address::generate(e);
    let burner1 = Address::generate(e);
    let burner2 = Address::generate(e);
    let outsider = Address::generate(e);

    // Set role admins
    client.set_role_admin(&Symbol::new(e, "minter"), &Symbol::new(e, "minter_admin"));
    client.set_role_admin(&Symbol::new(e, "burner"), &Symbol::new(e, "burner_admin"));

    // Grant admin roles
    client.grant_role(admin, &minter_admin, &Symbol::new(e, "minter_admin"));
    client.grant_role(admin, &burner_admin, &Symbol::new(e, "burner_admin"));

    // Admins grant operational roles
    client.grant_role(&minter_admin, &minter1, &Symbol::new(e, "minter"));
    client.grant_role(&minter_admin, &minter2, &Symbol::new(e, "minter"));
    client.grant_role(&burner_admin, &burner1, &Symbol::new(e, "burner"));
    client.grant_role(&burner_admin, &burner2, &Symbol::new(e, "burner"));

    TestAccounts { minter_admin, burner_admin, minter1, minter2, burner1, burner2, outsider }
}

#[test]
fn minters_can_mint() {
    let e = Env::default();
    let admin = Address::generate(&e);
    let client = create_client(&e, &admin);

    e.mock_all_auths();

    let accounts = setup_roles(&e, &client, &admin);

    client.mint(&accounts.minter1, &accounts.minter1, &1);
    client.mint(&accounts.minter2, &accounts.minter2, &2);
}

#[test]
#[should_panic(expected = "Error(Contract, #2000)")]
fn non_minters_cannot_mint() {
    let e = Env::default();
    let admin = Address::generate(&e);
    let client = create_client(&e, &admin);

    e.mock_all_auths();

    let accounts = setup_roles(&e, &client, &admin);

    client.mint(&accounts.outsider, &accounts.outsider, &3);
}

#[test]
fn burners_can_burn() {
    let e = Env::default();
    let admin = Address::generate(&e);
    let client = create_client(&e, &admin);

    e.mock_all_auths();

    let accounts = setup_roles(&e, &client, &admin);

    client.mint(&accounts.minter1, &accounts.burner1, &10);
    client.burn(&accounts.burner1, &10);
}

#[test]
#[should_panic(expected = "Error(Contract, #2000)")]
fn non_burners_cannot_burn() {
    let e = Env::default();
    let admin = Address::generate(&e);
    let client = create_client(&e, &admin);

    e.mock_all_auths();

    let accounts = setup_roles(&e, &client, &admin);

    client.mint(&accounts.minter1, &accounts.outsider, &11);
    client.burn(&accounts.outsider, &11);
}

#[test]
fn burners_can_burn_from() {
    let e = Env::default();
    let admin = Address::generate(&e);
    let client = create_client(&e, &admin);

    e.mock_all_auths();

    let accounts = setup_roles(&e, &client, &admin);

    // Mint to someone else
    client.mint(&accounts.minter1, &accounts.outsider, &20);
    client.approve(&accounts.outsider, &accounts.burner2, &20, &1000);

    // burner2 burns on behalf of outsider
    client.burn_from(&accounts.burner2, &accounts.outsider, &20);
}

#[test]
#[should_panic(expected = "Error(Contract, #2000)")]
fn non_burners_cannot_burn_from() {
    let e = Env::default();
    let admin = Address::generate(&e);
    let client = create_client(&e, &admin);

    e.mock_all_auths();

    let accounts = setup_roles(&e, &client, &admin);

    // Mint to burner1
    client.mint(&accounts.minter1, &accounts.burner1, &21);

    // Outsider tries to burn on behalf of burner1
    client.burn_from(&accounts.outsider, &accounts.burner1, &21);
}

#[test]
fn minter_admin_can_grant_role() {
    let e = Env::default();
    let admin = Address::generate(&e);
    let client = create_client(&e, &admin);

    e.mock_all_auths();

    let accounts = setup_roles(&e, &client, &admin);

    let new_minter = Address::generate(&e);
    client.grant_role(&accounts.minter_admin, &new_minter, &symbol_short!("minter"));

    // Mint with new_minter to verify
    client.mint(&new_minter, &new_minter, &100);
}

#[test]
#[should_panic(expected = "Error(Contract, #2000)")]
fn burner_admin_can_revoke_role() {
    let e = Env::default();
    let admin = Address::generate(&e);
    let client = create_client(&e, &admin);

    e.mock_all_auths();

    let accounts = setup_roles(&e, &client, &admin);

    // Revoke burner's role
    client.revoke_role(&accounts.burner_admin, &accounts.burner1, &symbol_short!("burner"));

    // burner1 should now panic if it tries to burn
    client.burn(&accounts.burner1, &10);
}

#[test]
#[should_panic(expected = "Error(Contract, #2000)")]
fn non_admin_cannot_grant_role() {
    let e = Env::default();
    let admin = Address::generate(&e);
    let client = create_client(&e, &admin);

    e.mock_all_auths();

    let accounts = setup_roles(&e, &client, &admin);

    let new_minter = Address::generate(&e);
    client.grant_role(&accounts.outsider, &new_minter, &symbol_short!("minter"));
}

#[test]
#[should_panic(expected = "Error(Contract, #2000)")]
fn non_admin_cannot_revoke_role() {
    let e = Env::default();
    let admin = Address::generate(&e);
    let client = create_client(&e, &admin);

    e.mock_all_auths();

    let accounts = setup_roles(&e, &client, &admin);

    client.revoke_role(&accounts.outsider, &accounts.burner1, &symbol_short!("burner"));
}

#[test]
fn admin_transfer_works() {
    let e = Env::default();

    let admin = Address::generate(&e);
    let client = create_client(&e, &admin);
    let new_admin = Address::generate(&e);
    let random_user = Address::generate(&e);

    e.mock_auths(&[MockAuth {
        address: &admin,
        invoke: &MockAuthInvoke {
            contract: &client.address,
            fn_name: "transfer_admin_role",
            args: (new_admin.clone(), 1000_u32).into_val(&e),
            sub_invokes: &[],
        },
    }]);

    // Current admin initiates the transfer
    client.transfer_admin_role(&new_admin, &1000);

    e.mock_auths(&[MockAuth {
        address: &new_admin,
        invoke: &MockAuthInvoke {
            contract: &client.address,
            fn_name: "accept_admin_transfer",
            args: ().into_val(&e),
            sub_invokes: &[],
        },
    }]);

    // New admin accepts
    client.accept_admin_transfer();

    e.mock_auths(&[MockAuth {
        address: &new_admin,
        invoke: &MockAuthInvoke {
            contract: &client.address,
            fn_name: "grant_role",
            args: (new_admin.clone(), random_user.clone(), symbol_short!("minter")).into_val(&e),
            sub_invokes: &[],
        },
    }]);

    // Sanity check: new admin can now grant a role
    client.grant_role(&new_admin, &random_user, &symbol_short!("minter"));
}

#[test]
#[should_panic(expected = "Error(Contract, #2200)")]
fn cannot_accept_after_admin_transfer_cancelled() {
    let e = Env::default();
    let admin = Address::generate(&e);
    let client = create_client(&e, &admin);
    let new_admin = Address::generate(&e);

    e.mock_all_auths();

    client.transfer_admin_role(&new_admin, &1000);

    // Now cancel
    client.transfer_admin_role(&new_admin, &0);

    // New admin tries to accept—should panic
    client.accept_admin_transfer();
}

#[test]
#[should_panic(expected = "Error(Auth, InvalidAction)")]
fn non_admin_cannot_initiate_transfer() {
    let e = Env::default();
    let admin = Address::generate(&e);
    let client = create_client(&e, &admin);
    let new_admin = Address::generate(&e);

    e.mock_auths(&[MockAuth {
        address: &admin,
        invoke: &MockAuthInvoke {
            contract: &client.address,
            fn_name: "transfer_admin_role",
            args: (new_admin.clone(), 1000_i128).into_val(&e),
            sub_invokes: &[],
        },
    }]);

    client.transfer_admin_role(&new_admin, &1000);
}

#[test]
#[should_panic(expected = "HostError: Error(Auth, InvalidAction)")]
fn non_recipient_cannot_accept_transfer() {
    let e = Env::default();
    let admin = Address::generate(&e);
    let client = create_client(&e, &admin);
    let new_admin = Address::generate(&e);
    let imposter = Address::generate(&e);

    e.mock_auths(&[MockAuth {
        address: &admin,
        invoke: &MockAuthInvoke {
            contract: &client.address,
            fn_name: "transfer_admin_role",
            args: (new_admin.clone(), 1000_i128).into_val(&e),
            sub_invokes: &[],
        },
    }]);

    e.mock_auths(&[MockAuth {
        address: &imposter,
        invoke: &MockAuthInvoke {
            contract: &client.address,
            fn_name: "accept_admin_transfer",
            args: (imposter.clone(),).into_val(&e),
            sub_invokes: &[],
        },
    }]);

    client.transfer_admin_role(&new_admin, &1000);

    // Imposter tries to accept
    client.accept_admin_transfer();
}

#[test]
#[should_panic(expected = "Error(Contract, #2200)")]
fn expired_admin_transfer_panics() {
    let e = Env::default();
    let admin = Address::generate(&e);
    let client = create_client(&e, &admin);
    let new_admin = Address::generate(&e);

    e.mock_all_auths();

    client.transfer_admin_role(&new_admin, &2000);

    // Move past the TTL for the admin transfer
    e.ledger().set_sequence_number(3000);

    client.accept_admin_transfer();
}

#[test]
#[should_panic(expected = "HostError: Error(Auth, InvalidAction)")]
fn non_admin_cannot_cancel_transfer_admin_role() {
    let e = Env::default();
    let admin = Address::generate(&e);
    let client = create_client(&e, &admin);
    let new_admin = Address::generate(&e);

    e.mock_auths(&[MockAuth {
        address: &admin,
        invoke: &MockAuthInvoke {
            contract: &client.address,
            fn_name: "transfer_admin_role",
            args: (new_admin.clone(), 1000_i128).into_val(&e),
            sub_invokes: &[],
        },
    }]);

    // Start a valid admin transfer
    client.transfer_admin_role(&new_admin, &1000);

    e.mock_auths(&[MockAuth {
        address: &new_admin,
        invoke: &MockAuthInvoke {
            contract: &client.address,
            fn_name: "transfer_admin_role",
            args: (new_admin.clone(), 0_i128).into_val(&e),
            sub_invokes: &[],
        },
    }]);

    // Non-admin attempts to cancel the admin transfer
    client.transfer_admin_role(&new_admin, &0);
}

#[test]
#[should_panic(expected = "HostError: Error(Auth, InvalidAction)")]
fn non_admin_cannot_set_role_admin() {
    let e = Env::default();
    let admin = Address::generate(&e);
    let client = create_client(&e, &admin);
    let non_admin = Address::generate(&e);

    e.mock_auths(&[MockAuth {
        address: &non_admin,
        invoke: &MockAuthInvoke {
            contract: &client.address,
            fn_name: "set_role_admin",
            args: (Symbol::new(&e, "minter"), Symbol::new(&e, "minter_admin")).into_val(&e),
            sub_invokes: &[],
        },
    }]);

    // Non-admin attempts to set a role admin
    client.set_role_admin(&Symbol::new(&e, "minter"), &Symbol::new(&e, "minter_admin"));
}

#[test]
fn admin_can_call_admin_restricted_function() {
    let e = Env::default();
    let admin = Address::generate(&e);
    let client = create_client(&e, &admin);

    e.mock_auths(&[MockAuth {
        address: &admin,
        invoke: &MockAuthInvoke {
            contract: &client.address,
            fn_name: "admin_restricted_function",
            args: ().into_val(&e),
            sub_invokes: &[],
        },
    }]);

    let secret = client.admin_restricted_function();
    assert_eq!(secret, vec![&e, String::from_str(&e, "seems sus")]);
}

#[test]
#[should_panic(expected = "HostError: Error(Auth, InvalidAction)")]
fn non_admin_cannot_call_admin_restricted_function() {
    let e = Env::default();
    let admin = Address::generate(&e);
    let non_admin = Address::generate(&e);
    let client = create_client(&e, &admin);

    e.mock_auths(&[MockAuth {
        address: &non_admin,
        invoke: &MockAuthInvoke {
            contract: &client.address,
            fn_name: "admin_restricted_function",
            args: ().into_val(&e),
            sub_invokes: &[],
        },
    }]);

    let _ = client.admin_restricted_function();
}

#[test]
fn minters_can_call_multi_role_action() {
    let e = Env::default();
    let admin = Address::generate(&e);
    let client = create_client(&e, &admin);

    e.mock_all_auths();

    let accounts = setup_roles(&e, &client, &admin);

    // Minters should be able to call the function
    let result = client.multi_role_action(&accounts.minter1);
    assert_eq!(result, String::from_str(&e, "multi_role_action_success"));

    let result = client.multi_role_action(&accounts.minter2);
    assert_eq!(result, String::from_str(&e, "multi_role_action_success"));
}

#[test]
fn burners_can_call_multi_role_action() {
    let e = Env::default();
    let admin = Address::generate(&e);
    let client = create_client(&e, &admin);

    e.mock_all_auths();

    let accounts = setup_roles(&e, &client, &admin);

    // Burners should be able to call the function
    let result = client.multi_role_action(&accounts.burner1);
    assert_eq!(result, String::from_str(&e, "multi_role_action_success"));

    let result = client.multi_role_action(&accounts.burner2);
    assert_eq!(result, String::from_str(&e, "multi_role_action_success"));
}

#[test]
#[should_panic(expected = "Account does not have any of the required roles")]
fn outsiders_cannot_call_multi_role_action() {
    let e = Env::default();
    let admin = Address::generate(&e);
    let client = create_client(&e, &admin);

    e.mock_all_auths();

    let accounts = setup_roles(&e, &client, &admin);

    // Outsider should not be able to call the function
    client.multi_role_action(&accounts.outsider);
}

#[test]
fn minters_can_call_multi_role_auth_action() {
    let e = Env::default();
    let admin = Address::generate(&e);
    let client = create_client(&e, &admin);

    e.mock_all_auths();

    let accounts = setup_roles(&e, &client, &admin);

    // Minter1 should be able to call the function with auth
    let result = client.multi_role_auth_action(&accounts.minter1);
    assert_eq!(result, String::from_str(&e, "multi_role_auth_action_success"));
}

#[test]
fn burners_can_call_multi_role_auth_action() {
    let e = Env::default();
    let admin = Address::generate(&e);
    let client = create_client(&e, &admin);

    e.mock_all_auths();

    let accounts = setup_roles(&e, &client, &admin);

    // Burner1 should be able to call the function with auth
    let result = client.multi_role_auth_action(&accounts.burner1);
    assert_eq!(result, String::from_str(&e, "multi_role_auth_action_success"));
}

#[test]
#[should_panic(expected = "Account does not have any of the required roles")]
fn outsiders_cannot_call_multi_role_auth_action() {
    let e = Env::default();
    let admin = Address::generate(&e);
    let client = create_client(&e, &admin);

    e.mock_all_auths();

    let accounts = setup_roles(&e, &client, &admin);

    // Outsider should not be able to call the function even with auth
    client.multi_role_auth_action(&accounts.outsider);
}
