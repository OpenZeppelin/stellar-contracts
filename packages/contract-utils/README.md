# Stellar Contract Utils

Utilities for Stellar contracts.

This crate is part of the [OpenZeppelin Stellar Contracts](https://github.com/OpenZeppelin/stellar-contracts) library, which is published as separate crates on [crates.io](https://crates.io):

- [stellar-access](https://crates.io/crates/stellar-access): Role-based access controls and ownable
- [stellar-accounts](https://crates.io/crates/stellar-accounts): Smart accounts with custom authentication and authorization
- **[stellar-contract-utils](https://crates.io/crates/stellar-contract-utils)**: Utilities for contracts (pausable, upgradeable, cryptography, etc.)
- [stellar-fee-abstraction](https://crates.io/crates/stellar-fee-abstraction): Fee abstraction utilities
- [stellar-governance](https://crates.io/crates/stellar-governance): Governance utilities (governor, votes, timelock)
- [stellar-macros](https://crates.io/crates/stellar-macros): Proc macros (`#[only_owner]`, `#[when_not_paused]`, etc.)
- [stellar-tokens](https://crates.io/crates/stellar-tokens): Token types (fungible, non-fungible, real-world assets, vaults)

Refer to the [OpenZeppelin for Stellar Contracts](https://docs.openzeppelin.com/stellar-contracts) page for additional information.

## Modules

### Pausable

The `pausable` module provides functionality to pause and unpause contract operations for emergency situations or maintenance.

#### Usage Examples

```rust
use soroban_sdk::{contract, contractimpl, Address, Env};
use stellar_contract_utils::pausable::{self as pausable, Pausable};
use stellar_access::ownable::{self as ownable, Ownable};
use stellar_macros::{only_owner, when_not_paused, when_paused};

#[contract]
pub struct MyContract;

#[contractimpl]
impl MyContract {
    // deploy this contract with the Stellar CLI:
    //
    // stellar contract deploy \
    // --wasm path/to/file.wasm \
    // -- \
    // --owner <owner_address>
    pub fn __constructor(e: &Env, owner: Address) {
        ownable::set_owner(e, &owner);
    }

    #[when_not_paused]
    pub fn normal_operation(e: &Env) {
        // This function can only be called when contract is not paused
    }

    #[when_paused]
    pub fn emergency_reset(e: &Env) {
        // This function can only be called when contract is paused
    }
}

#[contractimpl(contracttrait)]
impl Pausable for MyContract {
    #[only_owner]
    pub fn pause(e: &Env) {
        pausable::pause(e);
    }

    #[only_owner]
    pub fn unpause(e: &Env) {
        pausable::unpause(e);
    }
}

#[contractimpl(contracttrait)]
impl Ownable for MyContract {}
```

### Upgradeable

The `upgradeable` module provides a trait and helper for contract upgrades. Implementing the `Upgradeable` trait generates an `UpgradeableClient` that other contracts (e.g. a governance contract, upgrader helper, or multisig) can use to trigger upgrades.

For storage migration patterns (eager, lazy, and enum wrapper), see the module-level documentation in `src/upgradeable/mod.rs` and the `examples/upgradeable/` directory.

#### Usage Examples

```rust
use soroban_sdk::{contract, contractimpl, Address, BytesN, Env};
use stellar_access::ownable::{self as ownable, Ownable};
use stellar_contract_utils::upgradeable::{self as upgradeable, Upgradeable};
use stellar_macros::only_owner;

#[contract]
pub struct ExampleContract;

#[contractimpl]
impl ExampleContract {
    // deploy this contract with the Stellar CLI:
    //
    // stellar contract deploy \
    // --wasm path/to/file.wasm \
    // -- \
    // --owner <owner_address>
    pub fn __constructor(e: &Env, owner: Address) {
        ownable::set_owner(e, &owner);
    }
}

#[contractimpl]
impl Upgradeable for ExampleContract {
    #[only_owner]
    fn upgrade(e: &Env, new_wasm_hash: BytesN<32>, _operator: Address) {
        upgradeable::upgrade(e, &new_wasm_hash);
    }
}

#[contractimpl(contracttrait)]
impl Ownable for ExampleContract {}
```

### Crypto

The `crypto` module provides cryptographic utilities including hash functions and Merkle tree verification.

#### Usage Examples

```rust
use soroban_sdk::{Bytes, BytesN, Env};
use stellar_contract_utils::crypto::{hasher::Hasher, keccak::Keccak256};

pub fn hash_data(e: &Env, data: Bytes) -> BytesN<32> {
    let mut hasher = Keccak256::new(e);
    hasher.update(data);
    hasher.finalize()
}
```

#### Features

- **Hash Functions**: SHA-256 and Keccak-256 implementations
- **Merkle Verification**: Verify Merkle proofs for data integrity
- **Utility Functions**: Hash pairs and commutative hashing

### Merkle Distributor

The `merkle_distributor` module implements a Merkle-based claim distribution system for snapshot-based voting and token distributions.

#### Features

- **Indexed Claims**: Claims are indexed by position in the Merkle tree
- **Flexible Leaf Structure**: Support for custom claim data structures
- **Use Cases**: Token airdrops, NFT distributions, allowlists, snapshot voting

## Installation

Add this to your `Cargo.toml`:

```toml
[dependencies]
# We recommend pinning to a specific version, because rapid iterations are expected as the library is in an active development phase.
stellar-contract-utils = "=0.7.1"
# Add this if you want to use macros
stellar-macros = "=0.7.1"
```

## Examples

See the following examples in the repository:
- [`examples/pausable/`](https://github.com/OpenZeppelin/stellar-contracts/tree/main/examples/pausable) - Pausable contract functionality
- [`examples/upgradeable/`](https://github.com/OpenZeppelin/stellar-contracts/tree/main/examples/upgradeable) - Contract upgrade patterns
- [`examples/fungible-merkle-airdrop/`](https://github.com/OpenZeppelin/stellar-contracts/tree/main/examples/fungible-merkle-airdrop) - Merkle-based token distribution

## License

This package is part of the Stellar Contracts library and follows the same licensing terms.
