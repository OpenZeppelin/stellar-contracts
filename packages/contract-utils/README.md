# Stellar Contract Utils

Utilities for Stellar contracts.

## Modules

### Pausable

The `pausable` module provides functionality to pause and unpause contract operations for emergency situations or maintenance.

#### Usage Examples

```rust
use soroban_sdk::{contract, contractimpl, Address, Env};
use stellar_contract_utils::pausable::{self as pausable, Pausable};
use stellar_access::ownable::{self as ownable, Ownable};
use stellar_macros::{default_impl, only_owner, when_not_paused, when_paused};

#[contract]
pub struct MyContract;

#[contractimpl]
impl MyContract {
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

#[default_impl]
#[contractimpl]
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

#[default_impl]
#[contractimpl]
impl Ownable for MyContract {}
```

### Upgradeable

The `upgradeable` module provides a framework for safe contract upgrades and migrations with version control.

#### Usage Examples

**Simple Upgrade (Upgradeable)**:

```rust
use soroban_sdk::{
    contract, contractimpl, Address, Env,
};
use stellar_access::ownable::{self as ownable, Ownable};
use stellar_contract_utils::upgradeable::UpgradeableInternal;
use stellar_macros::{default_impl, only_owner, Upgradeable};

#[derive(Upgradeable)]
#[contract]
pub struct ExampleContract;

#[contractimpl]
impl ExampleContract {
    pub fn __constructor(e: &Env, owner: Address) {
        ownable::set_owner(e, &owner);
    }
}

impl UpgradeableInternal for ExampleContract {
    fn _require_auth(e: &Env, _operator: &Address) {
        ownable::enforce_owner_auth(e);
    }
}

#[default_impl]
#[contractimpl]
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
stellar-contract-utils = "=0.5.1"
# Add this if you want to use macros
stellar-macros = "=0.5.1"
```

## Examples

See the following examples in the repository:
- [`examples/pausable/`](https://github.com/OpenZeppelin/stellar-contracts/tree/main/examples/pausable) - Pausable contract functionality
- [`examples/upgradeable/`](https://github.com/OpenZeppelin/stellar-contracts/tree/main/examples/upgradeable) - Contract upgrade patterns
- [`examples/fungible-merkle-airdrop/`](https://github.com/OpenZeppelin/stellar-contracts/tree/main/examples/fungible-merkle-airdrop) - Merkle-based token distribution

## License

This package is part of the Stellar Contracts library and follows the same licensing terms.
