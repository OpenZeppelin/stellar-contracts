# Stellar Governance

Stellar governance functionalities

This crate is part of the [OpenZeppelin Stellar Contracts](https://github.com/OpenZeppelin/stellar-contracts) library, which is published as separate crates on [crates.io](https://crates.io):

- [stellar-access](https://crates.io/crates/stellar-access): Role-based access controls and ownable
- [stellar-accounts](https://crates.io/crates/stellar-accounts): Smart accounts with custom authentication and authorization
- [stellar-contract-utils](https://crates.io/crates/stellar-contract-utils): Utilities for contracts (pausable, upgradeable, cryptography, etc.)
- [stellar-fee-abstraction](https://crates.io/crates/stellar-fee-abstraction): Fee abstraction utilities
- **[stellar-governance](https://crates.io/crates/stellar-governance)**: Governance utilities (governor, votes, timelock)
- [stellar-macros](https://crates.io/crates/stellar-macros): Proc macros (`#[only_owner]`, `#[when_not_paused]`, etc.)
- [stellar-tokens](https://crates.io/crates/stellar-tokens): Token types (fungible, non-fungible, real-world assets, vaults)

Refer to the [OpenZeppelin for Stellar Contracts](https://docs.openzeppelin.com/stellar-contracts) page for additional information.

## Overview

This package provides governance modules for Soroban smart contracts:

- **Governor**: On-chain governance with proposals, voting, counting, and execution
- **Votes**: Vote tracking with delegation and historical checkpointing
- **Timelock**: Time-delayed execution of operations

## Modules

### Governor

The `governor` module implements on-chain governance for Soroban contracts. It provides the core governance primitives for decentralized decision-making.

#### Core Concepts

- **Proposals**: Bundles of on-chain calls (targets, functions, arguments) paired with a description
- **Voting**: Token holders vote during a voting period using snapshot-based voting power
- **Counting**: Default simple counting (Against/For/Abstain) with pluggable alternatives
- **Execution**: Successful proposals can be executed, triggering on-chain calls
- **Queuing**: Optional queue step for integration with a timelock (disabled by default, enabled with a single override)

#### Key Features

- Snapshot-based voting power prevents flash loan attacks
- Proposal threshold prevents governance spam
- Dynamic quorum support (override `quorum()` for supply-relative quorum)
- Queue logic is built into the base trait but disabled by default — a single `proposals_need_queuing()` override activates the full flow
- `execute` and `cancel` have no default implementation, requiring explicit access control decisions

### Votes

The `votes` module provides vote tracking functionality with delegation and historical checkpointing for governance mechanisms.

#### Core Concepts

- **Voting Units**: The base unit of voting power, typically 1:1 with token balance
- **Delegation**: Accounts can delegate their voting power to another account (delegatee)
- **Checkpoints**: Historical snapshots of voting power at specific ledger sequence numbers
- **Clock Mode**: Uses ledger sequence numbers (`e.ledger().sequence()`) as the timepoint reference

#### Key Features

- Track voting power per account with historical checkpoints
- Support delegation (an account can delegate its voting power to another account)
- Provide historical vote queries at any past ledger sequence number
- Explicit delegation required (accounts must self-delegate to use their own voting power)
- Non-delegated voting units are not counted as votes

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
stellar-governance = "=0.7.0"
```

## Examples

See the following examples in the repository:
- [`examples/fungible-governor/`](https://github.com/OpenZeppelin/stellar-contracts/tree/main/examples/fungible-governor) - Governor with fungible token voting
- [`examples/fungible-governor-timelock/`](https://github.com/OpenZeppelin/stellar-contracts/tree/main/examples/fungible-governor-timelock) - Governor with timelock queue
- [`examples/fungible-votes/`](https://github.com/OpenZeppelin/stellar-contracts/tree/main/examples/fungible-votes) - Fungible token with voting extension
- [`examples/timelock-controller/`](https://github.com/OpenZeppelin/stellar-contracts/tree/main/examples/timelock-controller) - Timelock controller with role-based access control
