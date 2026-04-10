extern crate std;

use soroban_sdk::{
    auth::{Context, ContractContext},
    contract, contractimpl, symbol_short,
    testutils::{Address as _, BytesN as _, Ledger, MockAuth, MockAuthInvoke},
    vec, Address, BytesN, Env, IntoVal, Symbol, Vec,
};
use stellar_governance::timelock::{OperationState, TimelockError};

use crate::{OperationMeta, TimelockController, TimelockControllerClient};

// Helper function to create empty BytesN<32>
fn empty(e: &Env) -> BytesN<32> {
    BytesN::<32>::from_array(e, &[0u8; 32])
}

// Mock target contract for testing
#[contract]
pub struct TargetContract;

#[contractimpl]
impl TargetContract {
    pub fn set_value(e: &Env, value: u32) -> u32 {
        e.storage().instance().set(&symbol_short!("value"), &value);
        value
    }

    pub fn get_value(e: &Env) -> u32 {
        e.storage().instance().get(&symbol_short!("value")).unwrap_or(0)
    }
}

#[test]
fn initialization() {
    let e = Env::default();
    e.mock_all_auths();

    let proposer = Address::generate(&e);
    let executor = Address::generate(&e);
    let admin = Address::generate(&e);

    let timelock = e.register(
        TimelockController,
        (10u32, vec![&e, proposer.clone()], vec![&e, executor.clone()], Some(admin.clone())),
    );

    let client = TimelockControllerClient::new(&e, &timelock);

    assert_eq!(client.get_min_delay(), 10);

    // Check roles are granted
    assert!(client.has_role(&proposer, &symbol_short!("proposer")).is_some());
    assert!(client.has_role(&proposer, &symbol_short!("canceller")).is_some());
    assert!(client.has_role(&executor, &symbol_short!("executor")).is_some());
    assert_eq!(client.get_admin(), Some(admin));
}

#[test]
fn schedule_and_execute_operation() {
    let e = Env::default();
    e.mock_all_auths();

    let proposer = Address::generate(&e);
    let executor = Address::generate(&e);
    let target = e.register(TargetContract, ());

    let timelock = e.register(
        TimelockController,
        (10u32, vec![&e, proposer.clone()], vec![&e, executor.clone()], None::<Address>),
    );

    let client = TimelockControllerClient::new(&e, &timelock);
    let target_client = TargetContractClient::new(&e, &target);

    let args = vec![&e, 42u32.into_val(&e)];
    let operation_id = client.schedule(
        &target,
        &symbol_short!("set_value"),
        &args,
        &empty(&e),
        &empty(&e),
        &10,
        &proposer,
    );

    assert!(client.get_operation_state(&operation_id) != OperationState::Unset);
    assert!(matches!(
        client.get_operation_state(&operation_id),
        OperationState::Waiting | OperationState::Ready
    ));
    assert_ne!(client.get_operation_state(&operation_id), OperationState::Ready);

    // Advance ledgers to make operation ready
    e.ledger().with_mut(|li| li.sequence_number += 10);

    assert_eq!(client.get_operation_state(&operation_id), OperationState::Ready);

    client.execute(
        &target,
        &symbol_short!("set_value"),
        &args,
        &empty(&e),
        &empty(&e),
        &Some(executor),
    );

    assert_eq!(target_client.get_value(), 42);
    assert_eq!(client.get_operation_state(&operation_id), OperationState::Done);
}

#[test]
fn schedule_and_execute_operation_no_executors() {
    let e = Env::default();
    e.mock_all_auths();

    let proposer = Address::generate(&e);
    let target = e.register(TargetContract, ());

    let timelock = e.register(
        TimelockController,
        // no executors
        (10u32, vec![&e, proposer.clone()], Vec::<Address>::new(&e), None::<Address>),
    );

    let client = TimelockControllerClient::new(&e, &timelock);
    let target_client = TargetContractClient::new(&e, &target);

    let args = vec![&e, 42u32.into_val(&e)];
    let operation_id = client.schedule(
        &target,
        &symbol_short!("set_value"),
        &args,
        &empty(&e),
        &empty(&e),
        &10,
        &proposer,
    );

    assert!(client.get_operation_state(&operation_id) != OperationState::Unset);
    assert!(matches!(
        client.get_operation_state(&operation_id),
        OperationState::Waiting | OperationState::Ready
    ));
    assert_ne!(client.get_operation_state(&operation_id), OperationState::Ready);

    e.ledger().with_mut(|li| li.sequence_number += 10);

    assert_eq!(client.get_operation_state(&operation_id), OperationState::Ready);

    client.execute(
        &target,
        &symbol_short!("set_value"),
        &args,
        &empty(&e),
        &empty(&e),
        // any address
        &None,
    );

    assert_eq!(target_client.get_value(), 42);
    assert_eq!(client.get_operation_state(&operation_id), OperationState::Done);
}

#[test]
fn schedule_and_execute_self_admin_operation() {
    let e = Env::default();

    let proposer = Address::generate(&e);
    let executor = Address::generate(&e);

    let timelock = e.register(
        TimelockController,
        (10u32, vec![&e, proposer.clone()], vec![&e, executor.clone()], None::<Address>),
    );

    let client = TimelockControllerClient::new(&e, &timelock);

    let args = vec![&e, 42u32.into_val(&e)];
    let operation_id = client
        .mock_auths(&[MockAuth {
            address: &proposer,
            invoke: &MockAuthInvoke {
                contract: &timelock,
                fn_name: "schedule",
                args: (
                    timelock.clone(),
                    Symbol::new(&e, "update_delay"),
                    args.clone(),
                    empty(&e),
                    empty(&e),
                    10u32,
                    proposer.clone(),
                )
                    .into_val(&e),
                sub_invokes: &[],
            },
        }])
        .schedule(
            &timelock,
            &Symbol::new(&e, "update_delay"),
            &args,
            &empty(&e),
            &empty(&e),
            &10,
            &proposer,
        );

    // Check operation is pending
    assert!(client.get_operation_state(&operation_id) != OperationState::Unset);
    assert!(matches!(
        client.get_operation_state(&operation_id),
        OperationState::Waiting | OperationState::Ready
    ));
    assert_ne!(client.get_operation_state(&operation_id), OperationState::Ready);

    e.ledger().with_mut(|li| li.sequence_number += 10);

    assert_eq!(client.get_operation_state(&operation_id), OperationState::Ready);

    // Mock executor's require_auth_for_args() that's called in `__check_auth`
    e.mock_auths(&[MockAuth {
        address: &executor,
        invoke: &MockAuthInvoke {
            contract: &timelock,
            fn_name: "__check_auth",
            args: (
                Symbol::new(&e, "execute_op"),
                timelock.clone(),
                Symbol::new(&e, "update_delay"),
                args.clone(),
                empty(&e),
                empty(&e),
            )
                .into_val(&e),
            sub_invokes: &[],
        },
    }]);

    // `__check_auth` can't be called directly, hence we need to use
    // `try_invoke_contract_check_auth` testing utility that emulates being
    // called by the Soroban host during a `require_auth` call.
    e.try_invoke_contract_check_auth::<TimelockError>(
        &timelock,
        &BytesN::random(&e),
        vec![
            &e,
            OperationMeta {
                predecessor: empty(&e),
                salt: empty(&e),
                executor: Some(executor.clone()),
            },
        ]
        .into_val(&e),
        &vec![
            &e,
            Context::Contract(ContractContext {
                contract: timelock.clone(),
                fn_name: Symbol::new(&e, "update_delay"),
                args: args.clone(),
            }),
        ],
    )
    .unwrap();

    assert_eq!(client.get_operation_state(&operation_id), OperationState::Done);
}

#[test]
fn cancel_operation() {
    let e = Env::default();
    e.mock_all_auths();

    let proposer = Address::generate(&e);
    let target = e.register(TargetContract, ());

    let timelock = e.register(
        TimelockController,
        (10u32, vec![&e, proposer.clone()], Vec::<Address>::new(&e), None::<Address>),
    );

    let client = TimelockControllerClient::new(&e, &timelock);

    let args = vec![&e, 42u32.into_val(&e)];
    let operation_id = client.schedule(
        &target,
        &symbol_short!("set_value"),
        &args,
        &empty(&e),
        &empty(&e),
        &10,
        &proposer,
    );

    assert!(matches!(
        client.get_operation_state(&operation_id),
        OperationState::Waiting | OperationState::Ready
    ));

    client.cancel(&operation_id, &proposer);

    // Check operation is no longer existing
    assert_eq!(client.get_operation_state(&operation_id), OperationState::Unset);
}

#[test]
#[should_panic(expected = "#4001")]
fn schedule_with_insufficient_delay() {
    let e = Env::default();
    e.mock_all_auths();

    let proposer = Address::generate(&e);
    let target = e.register(TargetContract, ());

    let timelock = e.register(
        TimelockController,
        (10u32, vec![&e, proposer.clone()], Vec::<Address>::new(&e), None::<Address>),
    );

    let client = TimelockControllerClient::new(&e, &timelock);

    // Try to schedule with delay less than minimum
    let args = vec![&e, 42u32.into_val(&e)];
    client.schedule(
        &target,
        &symbol_short!("set_value"),
        &args,
        &empty(&e),
        &empty(&e),
        &5, // Less than min delay of 10
        &proposer,
    );
}

#[test]
#[should_panic(expected = "#4002")]
fn execute_before_ready() {
    let e = Env::default();
    e.mock_all_auths();

    let proposer = Address::generate(&e);
    let executor = Address::generate(&e);
    let target = e.register(TargetContract, ());

    let timelock = e.register(
        TimelockController,
        (10u32, vec![&e, proposer.clone()], vec![&e, executor.clone()], None::<Address>),
    );

    let client = TimelockControllerClient::new(&e, &timelock);

    // Schedule operation
    let args = vec![&e, 42u32.into_val(&e)];
    client.schedule(
        &target,
        &symbol_short!("set_value"),
        &args,
        &empty(&e),
        &empty(&e),
        &10,
        &proposer,
    );

    // Try to execute before delay passes (should panic)
    client.execute(
        &target,
        &symbol_short!("set_value"),
        &args,
        &empty(&e),
        &empty(&e),
        &Some(executor),
    );
}
