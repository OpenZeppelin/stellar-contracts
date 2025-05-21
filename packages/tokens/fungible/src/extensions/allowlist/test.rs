#![cfg(test)]

use soroban_sdk::{
    testutils::{Address as _, AuthorizedFunction, AuthorizedInvocation},
    Address, Env, IntoVal,
};

use crate::{
    extensions::allowlist::{AllowList, FungibleAllowList},
    FungibleToken,
};

#[test]
fn test_allow_disallow_user() {
    let e = Env::default();
    let admin = Address::generate(&e);
    let user1 = Address::generate(&e);
    let user2 = Address::generate(&e);

    // Set admin
    AllowList::set_admin(&e, &admin);

    // Test initial state - users should not be allowed
    assert_eq!(AllowList::allowed(&e, &user1), false);
    assert_eq!(AllowList::allowed(&e, &user2), false);

    // Allow user1
    let allow_user_invocation = AuthorizedInvocation {
        function: AuthorizedFunction::Contract((
            e.current_contract_address(),
            symbol_short!("allow_user"),
            (admin.clone(), user1.clone()).into_val(&e),
        )),
        sub_invocations: vec![],
    };

    admin.require_auth_for_testing(vec![allow_user_invocation]);
    AllowList::allow_user(&e, &admin, &user1);

    // Verify user1 is allowed, user2 is still not allowed
    assert_eq!(AllowList::allowed(&e, &user1), true);
    assert_eq!(AllowList::allowed(&e, &user2), false);

    // Disallow user1
    let disallow_user_invocation = AuthorizedInvocation {
        function: AuthorizedFunction::Contract((
            e.current_contract_address(),
            symbol_short!("disallow_user"),
            (admin.clone(), user1.clone()).into_val(&e),
        )),
        sub_invocations: vec![],
    };

    admin.require_auth_for_testing(vec![disallow_user_invocation]);
    AllowList::disallow_user(&e, &admin, &user1);

    // Verify user1 is no longer allowed
    assert_eq!(AllowList::allowed(&e, &user1), false);
}

struct TestContract;

impl FungibleToken for TestContract {
    type ContractType = AllowList;

    fn total_supply(e: &Env) -> i128 {
        Self::ContractType::total_supply(e)
    }

    fn balance(e: &Env, account: Address) -> i128 {
        Self::ContractType::balance(e, &account)
    }

    fn allowance(e: &Env, owner: Address, spender: Address) -> i128 {
        Self::ContractType::allowance(e, &owner, &spender)
    }

    fn transfer(e: &Env, from: Address, to: Address, amount: i128) {
        Self::ContractType::transfer(e, &from, &to, amount);
    }

    fn transfer_from(e: &Env, spender: Address, from: Address, to: Address, amount: i128) {
        Self::ContractType::transfer_from(e, &spender, &from, &to, amount);
    }

    fn approve(e: &Env, owner: Address, spender: Address, amount: i128, live_until_ledger: u32) {
        Self::ContractType::approve(e, &owner, &spender, amount, live_until_ledger);
    }

    fn decimals(e: &Env) -> u32 {
        Self::ContractType::decimals(e)
    }

    fn name(e: &Env) -> soroban_sdk::String {
        Self::ContractType::name(e)
    }

    fn symbol(e: &Env) -> soroban_sdk::String {
        Self::ContractType::symbol(e)
    }
}

impl FungibleAllowList for TestContract {
    fn allowed(e: &Env, account: Address) -> bool {
        AllowList::allowed(e, &account)
    }

    fn allow_user(e: &Env, admin: Address, user: Address) {
        AllowList::allow_user(e, &admin, &user);
    }

    fn disallow_user(e: &Env, admin: Address, user: Address) {
        AllowList::disallow_user(e, &admin, &user);
    }
}
