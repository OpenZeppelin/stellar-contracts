# Stellar Macros

Procedural attribute macros for Stellar contracts, providing compile-time checks for access control and pausable patterns.

This crate is part of the [OpenZeppelin Stellar Contracts](https://github.com/OpenZeppelin/stellar-contracts) library, which is published as separate crates on [crates.io](https://crates.io):

- [stellar-access](https://crates.io/crates/stellar-access): Role-based access controls and ownable
- [stellar-accounts](https://crates.io/crates/stellar-accounts): Smart accounts with custom authentication and authorization
- [stellar-contract-utils](https://crates.io/crates/stellar-contract-utils): Utilities for contracts (pausable, upgradeable, cryptography, etc.)
- [stellar-fee-abstraction](https://crates.io/crates/stellar-fee-abstraction): Fee abstraction utilities
- [stellar-governance](https://crates.io/crates/stellar-governance): Governance utilities (governor, votes, timelock)
- **[stellar-macros](https://crates.io/crates/stellar-macros)**: Proc macros (`#[only_owner]`, `#[when_not_paused]`, etc.)
- [stellar-tokens](https://crates.io/crates/stellar-tokens): Token types (fungible, non-fungible, real-world assets, vaults)

Refer to the [OpenZeppelin for Stellar Contracts](https://docs.openzeppelin.com/stellar-contracts) page for additional information.

## Access Control Macros

Macros for role-based and ownership-based access control.

### Usage Examples

```rust
use soroban_sdk::{contract, contractimpl, Address, Env};
use stellar_macros::{only_admin, only_role, has_role, only_owner};

#[contract]
pub struct MyContract;

#[contractimpl]
impl MyContract {
    #[only_admin]
    pub fn admin_function(e: &Env) {
        // Only admin can call this
    }

    #[only_role(caller, "minter")]
    pub fn mint(e: &Env, amount: i128, caller: Address) {
        // Only accounts with "minter" role can call this
        // Includes both role check AND authorization
    }

    #[has_role(caller, "minter")]
    pub fn mint_with_auth(e: &Env, amount: i128, caller: Address) {
        caller.require_auth(); // Manual authorization required
        // Only role check, no automatic authorization
    }

    #[only_owner]
    pub fn owner_function(e: &Env) {
        // Only contract owner can call this
    }
}
```

### Available Macros

- `#[only_admin]`: Restricts access to admin only
- `#[only_role(account, "role")]`: Role check with authorization
- `#[has_role(account, "role")]`: Role check without authorization
- `#[has_any_role(account, ["role1", "role2"])]`: Multiple role check without authorization
- `#[only_any_role(account, ["role1", "role2"])]`: Multiple role check with authorization
- `#[only_owner]`: Restricts access to owner only

**Important**: Some macros perform role checking without authorization, while others include both:

- **Role Check Only** (`#[has_role]`, `#[has_any_role]`): Verify role membership but don't call `require_auth()`
- **Role Check + Auth** (`#[only_role]`, `#[only_any_role]`): Verify role membership AND call `require_auth()`

Use role-only macros when your function already contains `require_auth()` calls to avoid duplicate authorization panics.

## Pausable Macros

Macros for implementing pausable functionality in contracts.

### Usage Examples

```rust
use soroban_sdk::{contract, contractimpl, Env};
use stellar_macros::{when_not_paused, when_paused};

#[contract]
pub struct MyContract;

#[contractimpl]
impl MyContract {
    #[when_not_paused]
    pub fn normal_operation(e: &Env) {
        // This function only works when contract is not paused
    }

    #[when_paused]
    pub fn emergency_function(e: &Env) {
        // This function only works when contract is paused
    }
}
```

### Available Macros

- `#[when_not_paused]`: Function executes only when contract is not paused
- `#[when_paused]`: Function executes only when contract is paused

## Installation

Add this to your `Cargo.toml`:

```toml
[dependencies]
# We recommend pinning to a specific version, because rapid iterations are expected as the library is in an active development phase.
stellar-macros = "=0.7.1"
```

## Examples

See the following examples in the repository:
- [`examples/fungible-pausable/`](https://github.com/OpenZeppelin/stellar-contracts/tree/main/examples/fungible-pausable) - Pausable macros usage
- [`examples/nft-access-control/`](https://github.com/OpenZeppelin/stellar-contracts/tree/main/examples/nft-access-control) - Access control macros

## License

This package is part of the Stellar Contracts library and follows the same licensing terms.
