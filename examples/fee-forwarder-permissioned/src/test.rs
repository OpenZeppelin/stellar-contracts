extern crate std;

use soroban_sdk::{
    contract, contractimpl,
    testutils::{Address as _, MockAuth, MockAuthInvoke},
    vec, Address, Env, IntoVal, MuxedAddress, String, Symbol, TryIntoVal, Val, Vec,
};
use stellar_tokens::fungible::{Base, FungibleToken};

use crate::contract::{FeeForwarder, FeeForwarderClient};

#[contract]
pub struct MockToken;

#[contractimpl]
impl MockToken {
    pub fn __constructor(e: &Env, to: Address) {
        Base::set_metadata(e, 7, String::from_str(e, "Mock Token"), String::from_str(e, "MOCK"));
        Base::mint(e, &to, 1_000_000_000);
    }
}

#[contractimpl(contracttrait)]
impl FungibleToken for MockToken {
    type ContractType = Base;
}

#[contract]
pub struct MockTarget;

#[contractimpl]
impl MockTarget {
    pub fn greet(e: Env) -> String {
        String::from_str(&e, "hello")
    }

    pub fn require_auth_test(_e: Env, caller: Address) -> Address {
        caller.require_auth();
        caller
    }
}

fn setup<'a>(
    e: &Env,
) -> (FeeForwarderClient<'a>, MockTokenClient<'a>, MockTargetClient<'a>, Address, Address, i128, i128)
{
    let admin = Address::generate(e);
    let user = Address::generate(e);
    let manager = Address::generate(e);
    let relayer = Address::generate(e);

    let fee_forwarder_id = e.register(FeeForwarder, (admin, manager, vec![e, relayer.clone()]));
    let token_id = e.register(MockToken, (user.clone(),));
    let target_id = e.register(MockTarget, ());

    let fee_forwarder = FeeForwarderClient::new(e, &fee_forwarder_id);
    let token = MockTokenClient::new(e, &token_id);
    let target = MockTargetClient::new(e, &target_id);

    (fee_forwarder, token, target, user, relayer, 100_000, 150_000)
}

#[test]
fn forward_basic() {
    let e = Env::default();
    let (fee_forwarder, token, target, user, relayer, fee_amount, max_fee_amount) = setup(&e);

    let current_ledger = e.ledger().sequence();
    let fn_name = Symbol::new(&e, "greet");
    let fn_args: Vec<Val> = vec![&e];

    let initial_user_balance = token.balance(&user);
    let initial_contract_balance = token.balance(&fee_forwarder.address);

    e.mock_auths(&[
        // mock auth for user
        MockAuth {
            address: &user,
            invoke: &MockAuthInvoke {
                contract: &fee_forwarder.address,
                fn_name: "forward",
                args: (
                    token.address.clone(),
                    max_fee_amount,
                    current_ledger,
                    target.address.clone(),
                    &fn_name,
                    &fn_args,
                )
                    .into_val(&e),
                sub_invokes: &[],
            },
        },
        MockAuth {
            address: &user,
            invoke: &MockAuthInvoke {
                contract: &token.address,
                fn_name: "approve",
                args: (user.clone(), fee_forwarder.address.clone(), max_fee_amount, current_ledger)
                    .into_val(&e),
                sub_invokes: &[],
            },
        },
        MockAuth {
            // mock auth for relayer
            address: &relayer,
            invoke: &MockAuthInvoke {
                contract: &fee_forwarder.address,
                fn_name: "forward",
                args: (
                    token.address.clone(),
                    fee_amount,
                    max_fee_amount,
                    current_ledger,
                    target.address.clone(),
                    &fn_name,
                    &fn_args,
                    user.clone(),
                    relayer.clone(),
                )
                    .into_val(&e),
                sub_invokes: &[],
            },
        },
    ]);

    // `greet` should return "hello"
    let res: String = fee_forwarder
        .forward(
            &token.address,
            &fee_amount,
            &max_fee_amount,
            &current_ledger,
            &target.address,
            &fn_name,
            &fn_args,
            &user,
            &relayer,
        )
        .try_into_val(&e)
        .unwrap();

    assert_eq!(res, String::from_str(&e, "hello"));

    assert_eq!(token.balance(&user), initial_user_balance - fee_amount);
    assert_eq!(token.balance(&fee_forwarder.address), initial_contract_balance + fee_amount);
}

#[test]
fn forward_two_subinvokes() {
    let e = Env::default();
    let (fee_forwarder, token, target, user, relayer, fee_amount, max_fee_amount) = setup(&e);

    let current_ledger = e.ledger().sequence();
    let fn_name = Symbol::new(&e, "require_auth_test");
    let fn_args: Vec<Val> = vec![&e, user.into_val(&e)];

    let initial_user_balance = token.balance(&user);
    let initial_contract_balance = token.balance(&fee_forwarder.address);

    e.mock_auths(&[
        // mock auth for user
        MockAuth {
            address: &user,
            invoke: &MockAuthInvoke {
                contract: &fee_forwarder.address,
                fn_name: "forward",
                args: (
                    token.address.clone(),
                    max_fee_amount,
                    current_ledger,
                    target.address.clone(),
                    &fn_name,
                    &fn_args,
                )
                    .into_val(&e),
                sub_invokes: &[MockAuthInvoke {
                    contract: &target.address,
                    fn_name: "require_auth_test",
                    args: (user.clone(),).into_val(&e),
                    sub_invokes: &[],
                }],
            },
        },
        MockAuth {
            address: &user,
            invoke: &MockAuthInvoke {
                contract: &token.address,
                fn_name: "approve",
                args: (user.clone(), fee_forwarder.address.clone(), max_fee_amount, current_ledger)
                    .into_val(&e),
                sub_invokes: &[],
            },
        },
        MockAuth {
            // mock auth for relayer
            address: &relayer,
            invoke: &MockAuthInvoke {
                contract: &fee_forwarder.address,
                fn_name: "forward",
                args: (
                    token.address.clone(),
                    fee_amount,
                    max_fee_amount,
                    current_ledger,
                    target.address.clone(),
                    &fn_name,
                    &fn_args,
                    user.clone(),
                    relayer.clone(),
                )
                    .into_val(&e),
                sub_invokes: &[],
            },
        },
    ]);

    // `require_auth_test` should return user address
    let res: Address = fee_forwarder
        .forward(
            &token.address,
            &fee_amount,
            &max_fee_amount,
            &current_ledger,
            &target.address,
            &fn_name,
            &fn_args,
            &user,
            &relayer,
        )
        .try_into_val(&e)
        .unwrap();

    assert_eq!(res, user);

    assert_eq!(token.balance(&user), initial_user_balance - fee_amount);
    assert_eq!(token.balance(&fee_forwarder.address), initial_contract_balance + fee_amount);
}
