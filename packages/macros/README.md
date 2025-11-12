# Stellar Macros

Macros for Stellar contracts.

## Modules

### Default Implementation

The `#[default_impl]` macro generates missing default implementations for traits provided by the Stellar library.

#### Usage Examples

```rust
use soroban_sdk::{contract, contractimpl, Address, Env};
use stellar_tokens::fungible::{Base, FungibleToken};
use stellar_macros::default_impl;

#[contract]
pub struct MyContract;

#[default_impl]
#[contractimpl]
impl FungibleToken for MyContract {
    type ContractType = Base;

    // Only provide overrides here, default implementations are auto-generated
}
```

#### Supported Traits

- `FungibleToken`
- `FungibleBurnable`
- `NonFungibleToken`
- `NonFungibleBurnable`
- `NonFungibleEnumerable`
- `AccessControl`
- `Ownable`

### Access Control Macros

Macros for role-based and ownership-based access control.

#### Usage Examples

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

#### Available Macros

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

### Pausable Macros

Macros for implementing pausable functionality in contracts.

#### Usage Examples

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

#### Available Macros

- `#[when_not_paused]`: Function executes only when contract is not paused
- `#[when_paused]`: Function executes only when contract is paused

### Upgradeable Macros

Derive macros for implementing contract upgradeability.

#### Usage Examples

```rust
use soroban_sdk::{contract, contractimpl, Address, Env};
use stellar_contract_utils::upgradeable::UpgradeableInternal;
use stellar_macros::Upgradeable;

#[derive(Upgradeable)]
#[contract]
pub struct MyContract;

#[contractimpl]
impl MyContract {
    pub fn __constructor(e: &Env, owner: Address) {
        // Contract initialization
    }
}

impl UpgradeableInternal for MyContract {
    fn _require_auth(e: &Env, operator: &Address) {
        // Define who can upgrade the contract
        operator.require_auth();
        // Add your access controls: role-based or ownable
    }
}
```

#### Available Derives

- `#[derive(Upgradeable)]`: Basic contract upgradeability
- `#[derive(UpgradeableMigratable)]`: Upgradeability with migration support

## Installation

Add this to your `Cargo.toml`:

```toml
[dependencies]
# We recommend pinning to a specific version, because rapid iterations are expected as the library is in an active development phase.
stellar-macros = "=0.5.1"
```

## Examples

See the following examples in the repository:
- [`examples/fungible-pausable/`](https://github.com/OpenZeppelin/stellar-contracts/tree/main/examples/fungible-pausable) - Pausable macros usage
- [`examples/nft-access-control/`](https://github.com/OpenZeppelin/stellar-contracts/tree/main/examples/nft-access-control) - Access control macros
- [`examples/upgradeable/`](https://github.com/OpenZeppelin/stellar-contracts/tree/main/examples/upgradeable) - Upgradeable derive macros

## License

This package is part of the Stellar Contracts library and follows the same licensing terms.
