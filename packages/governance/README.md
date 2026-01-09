# Stellar Governance

Stellar governance functionalities

## Overview

This package provides governance modules for Soroban smart contracts:

- **Timelock**: Time-delayed execution of operations

## Modules

### Timelock

The `timelock` module provides functionality for time-delayed execution of operations, enabling governance mechanisms where actions must wait for a minimum delay before execution.

#### Core Concepts

- **Operations**: Actions to be executed on target contracts
- **Scheduling**: Proposing an operation with a delay period
- **Execution**: Running the operation after the delay has passed
- **Cancellation**: Removing a scheduled operation before execution
- **Predecessors**: Dependencies between operations (operation B requires operation A to be done first)

#### Usage Example

```rust
use soroban_sdk::{contract, contractimpl, Address, BytesN, Env, Symbol, Val, Vec};
use stellar_governance::timelock::{
    schedule_operation, execute_operation, cancel_operation,
    get_operation_state, set_min_delay, Operation, OperationState,
};

#[contract]
pub struct TimelockController;

#[contractimpl]
impl TimelockController {
    pub fn __constructor(e: &Env, min_delay: u32) {
        set_min_delay(e, min_delay);
    }

    pub fn schedule(
        e: &Env,
        target: Address,
        function: Symbol,
        args: Vec<Val>,
        predecessor: BytesN<32>,
        salt: BytesN<32>,
        delay: u32,
    ) -> BytesN<32> {
        // Add authorization checks here
        let operation = Operation {
            target,
            function,
            args,
            predecessor,
            salt,
        };
        schedule_operation(e, &operation, delay)
    }

    pub fn execute(
        e: &Env,
        target: Address,
        function: Symbol,
        args: Vec<Val>,
        predecessor: BytesN<32>,
        salt: BytesN<32>,
    ) {
        // Add authorization checks here
        let operation = Operation {
            target,
            function,
            args,
            predecessor,
            salt,
        };
        execute_operation(e, &operation);
    }

    pub fn cancel(e: &Env, id: BytesN<32>) {
        // Add authorization checks here
        cancel_operation(e, &id);
    }
}
```

## Installation

Add this to your `Cargo.toml`:

```toml
[dependencies]
# We recommend pinning to a specific version, because rapid iterations are expected as the library is in an active development phase.
stellar-governance = "=0.6.0"
```

## Examples

See the following examples in the repository:
- [`examples/timelock-controller/`](https://github.com/OpenZeppelin/stellar-contracts/tree/main/examples/timelock-controller) - Timelock controller with role-based access control
