# Stellar Access Control

Access Control, Ownable, and Role Transfer utilities for Stellar contracts.

## Overview

This package provides three main modules for managing access control in Soroban smart contracts:

- **Access Control**: Role-based access control with hierarchical permissions
- **Ownable**: Simple single-owner access control pattern
- **Role Transfer**: Utility module for secure role and ownership transfers

## Modules

### Access Control

The `access_control` module provides comprehensive role-based access control functionality:

- **Admin Management**: Single overarching admin with full privileges
- **Role Hierarchy**: Roles can have admin roles that can grant/revoke permissions
- **Secure Transfers**: Two-step admin transfer process for security

#### Usage Examples

```rust
use soroban_sdk::{contract, contractimpl, symbol_short, Address, Env};
use stellar_access::access_control::{self as access_control, AccessControl};
use stellar_macros::default_impl;

#[contract]
pub struct MyContract;

#[contractimpl]
impl MyContract {
    pub fn __constructor(e: &Env, admin: Address) {
        access_control::set_admin(e, &admin);
    }

    pub fn admin_restricted_function(e: &Env) {
        access_control::enforce_admin_auth(e);
        // ...
    }

    pub fn mint(e: &Env, to: Address, token_id: u32, caller: Address) {
        access_control::ensure_role(e, &caller, &symbol_short!("minter"));
        caller.require_auth();
        // minting
    }
}

#[default_impl]
#[contractimpl]
impl AccessControl for MyContract {}
```

**With Macros** (requires `stellar-macros` dependency):

```rust
use stellar_macros::{only_admin, only_role};

#[only_admin]
pub fn admin_restricted_function(e: &Env) {
    // ...
}

#[only_role(caller, "minter")]
fn mint(e: &Env, to: Address, token_id: u32, caller: Address) {
    // ...
}
```

### Ownable

The `ownable` module implements a simple ownership pattern:

- **Single Owner**: Contract has one owner with exclusive access
- **Ownership Transfer**: Secure two-step ownership transfer
- **Ownership Renouncement**: Owner can renounce ownership

#### Usage Examples

```rust
use soroban_sdk::{contract, contractimpl, Address, Env};
use stellar_access::ownable::{self as ownable, Ownable};
use stellar_macros::default_impl;

#[contract]
pub struct MyContract;

#[contractimpl]
impl MyContract {
    pub fn __constructor(e: &Env, owner: Address) {
        ownable::set_owner(e, &owner);
    }

    pub fn owner_restricted_function(e: &Env) {
        ownable::enforce_owner_auth(e);
        // ...
    }
}

#[default_impl]
#[contractimpl]
impl Ownable for MyContract {}
```

**With Macros** (requires `stellar-macros` dependency):

```rust
use stellar_macros::only_owner;

#[only_owner]
pub fn owner_restricted_function(e: &Env) {
    // ...
}
```

### Role Transfer

The `role_transfer` module is a utility module that provides the underlying infrastructure for secure two-step role and ownership transfers used by both Access Control and Ownable modules.

## Security Model

Both Access Control and Ownable modules implement a **two-step transfer process** for critical role changes:

1. **Initiate Transfer**: Current admin/owner specifies the new recipient and expiration
2. **Accept Transfer**: Designated recipient must explicitly accept the transfer

This mechanism prevents accidental transfers to wrong addresses or loss of control due to typos or errors.

**Note**: Unlike OpenZeppelin's Solidity library where role transfers can be immediate, **all role transfers in this Stellar library are always two-step processes** for enhanced security. This applies to both ownership transfers and admin role transfers.

## Installation

Add this to your `Cargo.toml`:

```toml
[dependencies]
# We recommend pinning to a specific version, because rapid iterations are expected as the library is in an active development phase.
stellar-access = "=0.5.1"
# Add this if you want to use macros
stellar-macros = "=0.5.1"
```

## Examples

See the following examples in the repository:
- [`examples/ownable/`](https://github.com/OpenZeppelin/stellar-contracts/tree/main/examples/ownable) - Simple ownership pattern
- [`examples/nft-access-control/`](https://github.com/OpenZeppelin/stellar-contracts/tree/main/examples/nft-access-control) - Role-based access control

## License

This package is part of the Stellar Contracts library and follows the same licensing terms.
