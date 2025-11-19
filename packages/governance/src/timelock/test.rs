use soroban_sdk::{
    contract, contractimpl, symbol_short, testutils::Ledger, vec, BytesN, Env, IntoVal, Symbol,
};

use crate::timelock::{
    cancel_operation, execute_operation, get_min_delay, get_operation_state, get_timestamp,
    hash_operation, is_operation_done, is_operation_pending, is_operation_ready, operation_exists,
    schedule_operation, set_min_delay, Operation, OperationState,
};

#[contract]
struct MockContract;

#[contract]
struct TargetContract;

#[contractimpl]
impl TargetContract {
    pub fn set_counter(e: &Env, counter: u32) {
        e.storage().instance().set(&symbol_short!("key"), &counter);
    }
}

fn create_operation(e: &Env) -> Operation {
    let target = e.register(TargetContract, ());
    Operation {
        target: target.clone(),
        function: Symbol::new(e, "set_counter"),
        args: vec![e, 1u32.into_val(e)],
        predecessor: empty(e),
        salt: empty(e),
    }
}

fn empty(e: &Env) -> BytesN<32> {
    BytesN::<32>::from_array(e, &[0u8; 32])
}

#[test]
fn set_and_get_min_delay() {
    let e = Env::default();
    let contract_address = e.register(MockContract, ());

    e.as_contract(&contract_address, || {
        set_min_delay(&e, 100);
        assert_eq!(get_min_delay(&e), 100);

        set_min_delay(&e, 200);
        assert_eq!(get_min_delay(&e), 200);
    });
}

#[test]
#[should_panic(expected = "Error(Contract, #4005)")]
fn get_min_delay_not_set() {
    let e = Env::default();
    let contract_address = e.register(MockContract, ());

    e.as_contract(&contract_address, || {
        get_min_delay(&e);
    });
}

#[test]
fn hash_operation_deterministic() {
    let e = Env::default();
    let operation = create_operation(&e);

    let hash1 = hash_operation(&e, &operation);
    let hash2 = hash_operation(&e, &operation);

    assert_eq!(hash1, hash2);
}

#[test]
fn hash_operation_different_with_salt() {
    let e = Env::default();
    let operation1 = create_operation(&e);
    let mut operation2 = operation1.clone();

    operation2.salt = BytesN::from_array(&e, &[1u8; 32]);

    let hash1 = hash_operation(&e, &operation1);
    let hash2 = hash_operation(&e, &operation2);

    assert_ne!(hash1, hash2);
}

#[test]
fn initial_operation_state() {
    let e = Env::default();
    let contract_address = e.register(MockContract, ());

    e.as_contract(&contract_address, || {
        let operation = create_operation(&e);
        let id = hash_operation(&e, &operation);

        assert_eq!(get_operation_state(&e, &id), OperationState::Unset);
        assert_eq!(get_timestamp(&e, &id), 0);
        assert!(!operation_exists(&e, &id));
        assert!(!is_operation_pending(&e, &id));
        assert!(!is_operation_ready(&e, &id));
        assert!(!is_operation_done(&e, &id));
    });
}

#[test]
fn schedule_operation_success() {
    let e = Env::default();
    let contract_address = e.register(MockContract, ());
    e.ledger().set_timestamp(1000);

    e.as_contract(&contract_address, || {
        set_min_delay(&e, 100);
        let operation = create_operation(&e);

        let id = schedule_operation(&e, &operation, 150);

        assert_eq!(id, hash_operation(&e, &operation));
        assert_eq!(get_operation_state(&e, &id), OperationState::Waiting);
        assert_eq!(get_timestamp(&e, &id), 1150); // 1000 + 150
        assert!(operation_exists(&e, &id));
        assert!(is_operation_pending(&e, &id));
        assert!(!is_operation_ready(&e, &id));
        assert!(!is_operation_done(&e, &id));
    });
}

#[test]
#[should_panic(expected = "Error(Contract, #4005)")]
fn schedule_operation_min_delay_not_set() {
    let e = Env::default();
    let contract_address = e.register(MockContract, ());

    e.as_contract(&contract_address, || {
        let operation = create_operation(&e);
        schedule_operation(&e, &operation, 100);
    });
}

#[test]
#[should_panic(expected = "Error(Contract, #4001)")]
fn schedule_operation_insufficient_delay() {
    let e = Env::default();
    let contract_address = e.register(MockContract, ());

    e.as_contract(&contract_address, || {
        set_min_delay(&e, 100);
        let operation = create_operation(&e);
        schedule_operation(&e, &operation, 50);
    });
}

#[test]
#[should_panic(expected = "Error(Contract, #4000)")]
fn schedule_operation_already_scheduled() {
    let e = Env::default();
    let contract_address = e.register(MockContract, ());

    e.as_contract(&contract_address, || {
        set_min_delay(&e, 100);
        let operation = create_operation(&e);
        schedule_operation(&e, &operation, 100);
        schedule_operation(&e, &operation, 100);
    });
}

#[test]
fn operation_state_transitions() {
    let e = Env::default();
    let contract_address = e.register(MockContract, ());
    e.ledger().set_timestamp(1000);

    e.as_contract(&contract_address, || {
        set_min_delay(&e, 100);
        let operation = create_operation(&e);

        let id = schedule_operation(&e, &operation, 100);

        // Initially waiting
        assert_eq!(get_operation_state(&e, &id), OperationState::Waiting);

        // Still waiting before delay
        e.ledger().set_timestamp(1050);
        assert_eq!(get_operation_state(&e, &id), OperationState::Waiting);

        // Ready after delay
        e.ledger().set_timestamp(1100);
        assert_eq!(get_operation_state(&e, &id), OperationState::Ready);
        assert!(is_operation_ready(&e, &id));

        // Execute
        execute_operation(&e, &operation);
        assert_eq!(get_operation_state(&e, &id), OperationState::Done);
        assert!(is_operation_done(&e, &id));
        assert!(!is_operation_pending(&e, &id));
    });
}

#[test]
fn execute_operation_success() {
    let e = Env::default();
    let contract_address = e.register(MockContract, ());
    e.ledger().set_timestamp(1000);

    let target = e.as_contract(&contract_address, || {
        set_min_delay(&e, 100);
        let operation = create_operation(&e);
        schedule_operation(&e, &operation, 100);
        e.ledger().set_timestamp(1100);

        execute_operation(&e, &operation);
        operation.target
    });

    e.as_contract(&target, || {
        assert_eq!(e.storage().instance().get(&symbol_short!("key")), Some(1u32))
    });
}

#[test]
#[should_panic(expected = "Error(Contract, #4002)")]
fn execute_operation_not_ready() {
    let e = Env::default();
    let contract_address = e.register(MockContract, ());
    e.ledger().set_timestamp(1000);

    e.as_contract(&contract_address, || {
        set_min_delay(&e, 100);
        let operation = create_operation(&e);
        schedule_operation(&e, &operation, 100);

        // Try to execute before delay
        execute_operation(&e, &operation);
    });
}

#[test]
#[should_panic(expected = "Error(Contract, #4002)")]
fn execute_operation_already_done() {
    let e = Env::default();
    let contract_address = e.register(MockContract, ());
    e.ledger().set_timestamp(1000);

    e.as_contract(&contract_address, || {
        set_min_delay(&e, 100);
        let operation = create_operation(&e);
        schedule_operation(&e, &operation, 100);
        e.ledger().set_timestamp(1100);

        execute_operation(&e, &operation);
        execute_operation(&e, &operation); // Try to execute again
    });
}

#[test]
fn cancel_operation_success() {
    let e = Env::default();
    let contract_address = e.register(MockContract, ());
    e.ledger().set_timestamp(1000);

    e.as_contract(&contract_address, || {
        set_min_delay(&e, 100);
        let operation = create_operation(&e);

        let id = schedule_operation(&e, &operation, 100);
        assert!(is_operation_pending(&e, &id));

        cancel_operation(&e, &id);

        assert_eq!(get_operation_state(&e, &id), OperationState::Unset);
        assert!(!operation_exists(&e, &id));
    });
}

#[test]
#[should_panic(expected = "Error(Contract, #4002)")]
fn cancel_operation_not_pending() {
    let e = Env::default();
    let contract_address = e.register(MockContract, ());

    e.as_contract(&contract_address, || {
        let operation = create_operation(&e);
        let id = hash_operation(&e, &operation);
        cancel_operation(&e, &id);
    });
}

#[test]
#[should_panic(expected = "Error(Contract, #4002)")]
fn cancel_operation_already_done() {
    let e = Env::default();
    let contract_address = e.register(MockContract, ());
    e.ledger().set_timestamp(1000);

    e.as_contract(&contract_address, || {
        set_min_delay(&e, 100);
        let operation = create_operation(&e);
        let id = schedule_operation(&e, &operation, 100);
        e.ledger().set_timestamp(1100);

        execute_operation(&e, &operation);
        cancel_operation(&e, &id);
    });
}

#[test]
fn predecessor_dependency() {
    let e = Env::default();
    let contract_address = e.register(MockContract, ());
    e.ledger().set_timestamp(1000);

    e.as_contract(&contract_address, || {
        set_min_delay(&e, 100);

        // Create first operation
        let operation1 = create_operation(&e);
        let id1 = schedule_operation(&e, &operation1, 100);

        // Create second operation that depends on first
        let mut operation2 = create_operation(&e);
        operation2.predecessor = id1.clone();
        operation2.salt = BytesN::from_array(&e, &[1u8; 32]); // Different salt

        let id2 = schedule_operation(&e, &operation2, 100);

        e.ledger().set_timestamp(1100);

        // Execute operation1 first
        execute_operation(&e, &operation1);

        // Now operation2 can be executed
        execute_operation(&e, &operation2);

        assert!(is_operation_done(&e, &id1));
        assert!(is_operation_done(&e, &id2));
    });
}

#[test]
#[should_panic(expected = "Error(Contract, #4003)")]
fn predecessor_dependency_fails_when_not_executed() {
    let e = Env::default();
    let contract_address = e.register(MockContract, ());
    e.ledger().set_timestamp(1000);

    e.as_contract(&contract_address, || {
        set_min_delay(&e, 100);

        // Create first operation
        let operation1 = create_operation(&e);
        let id1 = schedule_operation(&e, &operation1, 100);

        // Create second operation that depends on first
        let mut operation2 = create_operation(&e);
        operation2.predecessor = id1.clone();
        operation2.salt = BytesN::from_array(&e, &[1u8; 32]);

        schedule_operation(&e, &operation2, 100);

        e.ledger().set_timestamp(1100);

        // Try to execute operation2 before operation1 - should panic
        execute_operation(&e, &operation2);
    });
}
