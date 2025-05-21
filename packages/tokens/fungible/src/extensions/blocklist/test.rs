#![cfg(test)]

use soroban_sdk::{
    testutils::{Address as _, AuthorizedFunction, AuthorizedInvocation},
    Address, Env, IntoVal,
};

use crate::{
    extensions::blocklist::{BlockList, FungibleBlockList},
    FungibleToken,
};

#[test]
fn test_block_unblock_user() {
    let e = Env::default();
    let admin = Address::generate(&e);
    let user1 = Address::generate(&e);
    let user2 = Address::generate(&e);

    // Set admin
    BlockList::set_admin(&e, &admin);

    // Test initial state - users should not be blocked
    assert_eq!(BlockList::blocked(&e, &user1), false);
    assert_eq!(BlockList::blocked(&e, &user2), false);

    // Block user1
    let block_user_invocation = AuthorizedInvocation {
        function: AuthorizedFunction::Contract((
            e.current_contract_address(),
            symbol_short!("block_user"),
            (admin.clone(), user1.clone()).into_val(&e),
        )),
        sub_invocations: vec![],
    };

    admin.require_auth_for_testing(vec![block_user_invocation]);
    BlockList::block_user(&e, &admin, &user1);

    // Verify user1 is blocked, user2 is still not blocked
    assert_eq!(BlockList::blocked(&e, &user1), true);
    assert_eq!(BlockList::blocked(&e, &user2), false);

    // Unblock user1
    let unblock_user_invocation = AuthorizedInvocation {
        function: AuthorizedFunction::Contract((
            e.current_contract_address(),
            symbol_short!("unblock_user"),
            (admin.clone(), user1.clone()).into_val(&e),
        )),
        sub_invocations: vec![],
    };

    admin.require_auth_for_testing(vec![unblock_user_invocation]);
    BlockList::unblock_user(&e, &admin, &user1);

    // Verify user1 is no longer blocked
    assert_eq!(BlockList::blocked(&e, &user1), false);
}

struct TestContract;

impl FungibleToken for TestContract {
    type ContractType = BlockList;

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

impl FungibleBlockList for TestContract {
    fn blocked(e: &Env, account: Address) -> bool {
        BlockList::blocked(e, &account)
    }

    fn block_user(e: &Env, admin: Address, user: Address) {
        BlockList::block_user(e, &admin, &user);
    }

    fn unblock_user(e: &Env, admin: Address, user: Address) {
        BlockList::unblock_user(e, &admin, &user);
    }
}
