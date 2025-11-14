#![cfg(test)]

use soroban_sdk::{
    contract, contractimpl, symbol_short,
    testutils::{Address as _, Ledger},
    vec, Address, BytesN, Env, IntoVal, Vec,
};

use crate::{TimelockController, TimelockControllerClient};

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
fn test_initialization() {
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

    // Check minimum delay is set
    assert_eq!(client.get_min_delay(), 10);

    // Check roles are granted
    assert!(client.has_role(&proposer, &symbol_short!("proposer")).is_some());
    assert!(client.has_role(&proposer, &symbol_short!("canceller")).is_some());
    assert!(client.has_role(&executor, &symbol_short!("executor")).is_some());
    assert_eq!(client.get_admin(), Some(admin));
}

#[test]
fn test_schedule_and_execute_operation() {
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

    // Schedule operation
    let args = vec![&e, 42u32.into_val(&e)];
    let operation_id = client.schedule_op(
        &target,
        &symbol_short!("set_value"),
        &args,
        &empty(&e),
        &empty(&e),
        &10,
        &proposer,
    );

    // Check operation is pending
    assert!(client.is_operation(&operation_id));
    assert!(client.is_operation_pending(&operation_id));
    assert!(!client.is_operation_ready(&operation_id));

    // Advance ledgers to make operation ready
    e.ledger().with_mut(|li| li.sequence_number += 10);

    // Check operation is ready
    assert!(client.is_operation_ready(&operation_id));

    // Execute operation
    client.execute_op(
        &target,
        &symbol_short!("set_value"),
        &args,
        &empty(&e),
        &empty(&e),
        &executor,
    );

    // Verify execution
    assert_eq!(target_client.get_value(), 42);
    assert!(client.is_operation_done(&operation_id));
}

#[test]
fn test_cancel_operation() {
    let e = Env::default();
    e.mock_all_auths();

    let proposer = Address::generate(&e);
    let target = e.register(TargetContract, ());

    let timelock = e.register(
        TimelockController,
        (10u32, vec![&e, proposer.clone()], Vec::<Address>::new(&e), None::<Address>),
    );

    let client = TimelockControllerClient::new(&e, &timelock);

    // Schedule operation
    let args = vec![&e, 42u32.into_val(&e)];
    let operation_id = client.schedule_op(
        &target,
        &symbol_short!("set_value"),
        &args,
        &empty(&e),
        &empty(&e),
        &10,
        &proposer,
    );

    // Check operation is pending
    assert!(client.is_operation_pending(&operation_id));

    // Cancel operation (proposers are also cancellers)
    client.cancel_op(&operation_id, &proposer);

    // Check operation is no longer pending
    assert!(!client.is_operation(&operation_id));
}

#[test]
#[should_panic(expected = "#4001")]
fn test_schedule_with_insufficient_delay() {
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
    client.schedule_op(
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
fn test_execute_before_ready() {
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
    client.schedule_op(
        &target,
        &symbol_short!("set_value"),
        &args,
        &empty(&e),
        &empty(&e),
        &10,
        &proposer,
    );

    // Try to execute before delay passes (should panic)
    client.execute_op(
        &target,
        &symbol_short!("set_value"),
        &args,
        &empty(&e),
        &empty(&e),
        &executor,
    );
}

#[test]
fn test_hash_operation_deterministic() {
    let e = Env::default();

    let target = Address::generate(&e);
    let timelock = e.register(
        TimelockController,
        (10u32, Vec::<Address>::new(&e), Vec::<Address>::new(&e), None::<Address>),
    );
    let client = TimelockControllerClient::new(&e, &timelock);

    let args = vec![&e, 42u32.into_val(&e)];
    let hash1 =
        client.hash_operation(&target, &symbol_short!("set_value"), &args, &empty(&e), &empty(&e));

    let hash2 =
        client.hash_operation(&target, &symbol_short!("set_value"), &args, &empty(&e), &empty(&e));

    assert_eq!(hash1, hash2);
}
