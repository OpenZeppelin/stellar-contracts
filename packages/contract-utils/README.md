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

### Math

The `math` module provides fixed-point arithmetic: a `Wad` decimal type with 18 decimal places, and free functions for `x * y / denominator` on both `i128` and `I256` with an explicit rounding direction.

Each operation comes in a panicking and a checked variant. The panicking variants raise `SorobanFixedPointError::Overflow` (1500); the checked variants return `None`.

#### Usage Examples

```rust
use soroban_sdk::{Env, I256};
use stellar_contract_utils::math::{i256_fixed_point, i128_fixed_point, wad::Wad, Rounding};

// Share conversion, rounding in the vault's favour.
pub fn to_shares(e: &Env, assets: i128, total_shares: i128, total_assets: i128) -> i128 {
    i128_fixed_point::mul_div_with_rounding(e, assets, total_shares, total_assets, Rounding::Floor)
}

// The same computation at 256-bit width. `I256` carries its own `Env`,
// so these signatures take no `&Env`.
pub fn to_shares_256(assets: &I256, total_shares: &I256, total_assets: &I256) -> I256 {
    i256_fixed_point::mul_div_floor(assets, total_shares, total_assets)
}

// 18-decimal fixed point, where 1.0 is 10^18.
pub fn apply_rate(e: &Env, principal: i128) -> Option<i128> {
    let rate = Wad::from_raw(1_050_000_000_000_000_000); // 1.05
    Wad::from_integer(e, principal).checked_mul(e, rate).map(|w| w.to_integer())
}
```

#### Phantom Overflow

Both widths recover from an intermediate `x * y` that overflows while the final result is representable, a case known as *phantom overflow*.

- **`i128`**: the operands are promoted to `I256`, and the result is scaled back if it fits `i128`.
- **`I256`**: there is no wider type to promote to, so the operands are split by the denominator and the division is distributed, which is an exact identity rather than an approximation. This holds for every denominator with `|denominator| <= 2^128` (roughly `3.4e38`, far above the fixed-point scales in practical use: `10^18`, `10^27`, `2^96`, `10^38`). Within that domain the result is bit-for-bit what a 512-bit intermediate would produce.

Above `2^128` the operation rejects rather than returning an incorrect value, and rejection is permitted rather than guaranteed: a large denominator whose remainders happen to be small still succeeds.

Phantom overflow handling does not extend to `Wad`'s operator implementations (`+`, `-`, `*`, `/`), because an operator cannot reach an `Env` to build the intermediate. Their narrower bounds are tabulated in the `# Overflow` section of `Wad`'s documentation, and `checked_mul` / `checked_div` should be preferred wherever an operand can plausibly exceed them.

Zero denominators and `MIN / -1` are left to the platform rather than mapped to a contract error, since these are plain arithmetic operations. Both surface a native or host arithmetic error from the panicking variants, and `None` from the checked ones.

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
