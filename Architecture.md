# OpenZeppelin Stellar Contracts Architecture

This document outlines the architectural design and structure of the OpenZeppelin Stellar Contracts library, a comprehensive collection of smart contracts for the Stellar network built using the Soroban SDK.

## Overview

The OpenZeppelin Stellar Contracts library follows a modular, trait-based architecture that promotes code reusability, extensibility, and maintainability. The architecture is designed to provide both high-level convenience functions and low-level granular control, allowing developers to choose the appropriate level of abstraction for their use cases.

## Project Structure

```
stellar-contracts/
├── packages/                    # Core library packages
│   ├── access/                  # Role-based access controls and ownable patterns
│   ├── contract-utils/          # Utilities (pausable, upgradeable, cryptography)
│   ├── macros/                  # Procedural and derive macros
│   ├── test-utils/              # Testing utilities and helpers
│   └── tokens/                  # Token implementations (fungible, non-fungible)
│       ├── src/
│       │   ├── fungible/        # Fungible token implementation
│       │   │   ├── extensions/  # Optional token extensions
│       │   │   ├── utils/       # Utility functions and helpers
│       │   │   ├── mod.rs       # Core trait definitions, constants, errors, and events
│       │   │   ├── storage.rs   # Storage management and state operations
│       │   │   └── test.rs      # Comprehensive test suite
│       │   └── non_fungible/    # Non-fungible token implementation
│       └── lib.rs
├── examples/                    # Example contract implementations
└── audits/                      # Security audit reports
```

## Core Architectural Principles

### 1. Trait-Based Design with Associated Types

The library extensively uses Rust traits to define standard interfaces and behaviors, with a sophisticated approach to enable method overriding, and enforce mutually exclusive extensions through associated types:

#### Enforcing Mutually Exclusive Extensions

One of the most sophisticated aspects of this architecture is how it prevents incompatible extensions from being used together. This is achieved through **associated types** and **trait bounds**:

```rust
// Core trait with associated type
trait NonFungibleToken {
    type ContractType: ContractOverrides;

    fn transfer(e: &Env, from: Address, to: Address, token_id: u32) {
        Self::ContractType::transfer(e, from, to, token_id);
    }
    // ... other methods
}

// Contract type markers
pub struct Base;        // Default implementation
pub struct Enumerable;  // For enumeration features
pub struct Consecutive; // For batch minting optimization
```

#### Extension Trait Constraints

Extensions are constrained to specific contract types using associated type bounds:

```rust
// Enumerable can only be used with Enumerable contract type
trait NonFungibleEnumerable: NonFungibleToken<ContractType = Enumerable> {
    fn total_supply(e: &Env) -> u32;
    fn get_owner_token_id(e: &Env, owner: Address, index: u32) -> u32;
    // ...
}

// Consecutive can only be used with Consecutive contract type
trait NonFungibleConsecutive: NonFungibleToken<ContractType = Consecutive> {
    // Batch minting functionality
}
```

#### Mutual Exclusivity Enforcement

This design makes it **impossible** to implement conflicting extensions:

```rust
// ✅ This works - using Enumerable
impl NonFungibleToken for MyContract {
    type ContractType = Enumerable;
    // ... implementations
}
impl NonFungibleEnumerable for MyContract {
    // ... enumerable methods
}

// ❌ This CANNOT compile - Consecutive requires different ContractType
// impl NonFungibleConsecutive for MyContract { ... }
//     ^^^ Error: expected `Consecutive`, found `Enumerable`
```

#### Override Mechanism Through ContractOverrides

The `ContractOverrides` trait provides the actual implementations that vary by contract type:

```rust
trait ContractOverrides {
    fn transfer(e: &Env, from: &Address, to: &Address, token_id: u32) {
        // Default implementation (used by Base)
        Base::transfer(e, from, to, token_id);
    }
    // ... other overridable methods
}

// Base uses default implementations
impl ContractOverrides for Base {}

// Enumerable overrides specific methods
impl ContractOverrides for Enumerable {
    fn transfer(e: &Env, from: &Address, to: &Address, token_id: u32) {
        // Custom enumerable transfer logic
        Enumerable::transfer(e, from, to, token_id);
    }
}

// Consecutive overrides different methods
impl ContractOverrides for Consecutive {
    fn owner_of(e: &Env, token_id: u32) -> Address {
        // Custom consecutive ownership lookup
        Consecutive::owner_of(e, token_id)
    }
}
```

#### Benefits of This Approach

1. **Compile-Time Safety**: Incompatible extensions cannot be combined
2. **Zero Runtime Overhead**: All dispatch is resolved at compile time
3. **Intuitive API**: Developers don't need to specify generics or complex types
4. **Automatic Behavior Override**: Methods automatically use the correct implementation based on contract type
5. **Modular Design**: Extensions can be developed and maintained independently

This pattern represents a novel solution to the challenge of providing both type safety and developer ergonomics in a trait-based extension system, avoiding the need for runtime checks or complex generic constraints.

### 2. Dual-Layer Architecture

The library provides two levels of abstraction:

#### High-Level Functions
- Include all necessary checks, verifications, and authorizations
- Handle state-changing logic and event emissions automatically
- Provide secure defaults and comprehensive error handling
- Ideal for standard use cases and rapid development

#### Low-Level Functions
- Offer granular control for custom workflows
- Require manual handling of verifications and authorizations
- Enable composition of complex business logic
- Suitable for advanced use cases requiring customization

### 3. Modular Extension System

The architecture supports optional extensions that can be mixed and matched. Below is the list of the extensions for the Fungible Token:

- **Burnable**: Token destruction capabilities
- **Capped**: Maximum supply limits
- **Allowlist**: Whitelist-based access control
- **Blocklist**: Blacklist-based access control
- **Metadata**: Enhanced token information (name, symbol, decimals)
- **Vault**: Asset deposit/withdrawal with share tokenization

## Storage Architecture

### Storage Key Design

The library uses a structured approach to storage keys:

```rust
#[contracttype]
pub enum StorageKey {
    TotalSupply,
    Balance(Address),
    Allowance(AllowanceKey),
}
```

### TTL Management:

This library handles extension of storage entries to prevent expiration, except from `instance` storage entry.
Extending the `instance` storage entries is the responsibility of the contract developer.

## Contract Implementation Patterns

### 1. Base Implementation Pattern

```rust
#[contract]
pub struct MyToken;

#[default_impl]
#[contractimpl]
impl FungibleToken for MyToken {
    ContractType = Base;
    // Custom overrides here (optional)
}
```

### 2. Extension Composition Pattern

```rust
#[default_impl]
#[contractimpl]
impl FungibleBurnable for MyToken {
    // Burning functionality
}

#[default_impl]
#[contractimpl]
impl Pausable for MyToken {
    // Pausable functionality
}
```

### 3. Macros As Helpers

The library provides macros to either:
- reduce boilerplate (i.e. `#[default_impl]`)
- improve clarity of the code by annotating the function instead of having the business logic inside the function as a regular code to improve the DevX (i.e. `#[only_owner]`, `#[when_not_paused]`)

#### Principles for Introducing New Macros

- The reduction in boilerplate must justify the added complexity of a new domain-specific language (DSL).
- Macros should be intuitive: developers should be able to adopt them with minimal reference to documentation; a couple of clear examples should suffice.
- Whenever possible, the logic abstracted by the macro should remain transparent and debuggable, for example by using cargo expand to inspect the generated code. To elaborate more on this: the newly introduced macro should preferably not work directly with other heavy proc macros, since the expanded code will be intertwined with the expanded code of the other macros, and will be much harder to inspect and debug.

## Integration Architecture

### SEP-41 Compliance

The library ensures full compatibility with SEP-41 (Stellar Enhancement Proposal 41) for fungible tokens:

- Standard interface implementation
- Required function signatures
- Event emission standards
- Error handling conventions

### Cross-Ecosystem Compatibility

Designed to mirror familiar standards:

- **ERC-20 Similarity**: Familiar interface for Ethereum developers
- **Stellar Asset Contract Compatibility**: Seamless integration with existing Stellar infrastructure

### NFT Standard Compatibility

The non-fungible token implementation is designed to be compatible with existing NFT standards while leveraging Stellar's unique capabilities:

- **ERC-721 Similarity**: Provides familiar interfaces and patterns for Ethereum developers working with NFTs
  - Core ownership and transfer functionality
  - Approval mechanisms (single token and operator approvals)
  - Metadata handling
  - Standard events (Transfer, Approval, ApprovalForAll)

- **SEP Extensions**: Incorporates Stellar-specific enhancements for NFT functionality
  - Optimized for Stellar's execution environment
  - Compatible with the broader Stellar ecosystem
  - Designed for cross-chain interoperability

## Performance Considerations

- **Read operations are free in Stellar**: this means when designing the contract, the main goal is to minimize the write operations. We can be generous with read operations.
- **Computation is generally cheap in Stellar**: having clean code, that is maintainable, readable, and optimized for the developer experience is a higher priority than squeezing every last bit of performance out of the contract. Optimization whenever possible, is still our goal, but not at the cost of developer experience. A good balance can be seen in `enumerable` and `consecutive` extensions designs. These extensions have taken costs into consideration, have optimal code that minimizes the gas usage, yet provides clean and maintainable code that is easy to understand and debug.

## Testing Architecture

### Comprehensive Test Coverage

- **Unit Tests**: Individual function testing
- **Integration Tests**: Cross-module interaction testing
- **Property-Based Testing**: Invariant verification
- **Fuzzing**: Edge case discovery

## Deployment Architecture

### WASM Compilation

- **Target**: `wasm32v1-none`
- **Optimization**: Release builds with size optimization
- **No-std Environment**: Minimal runtime footprint

## Code Conventions

- We are strictly following `cargo fmt` and `cargo clippy` rules
- We prefer to use declarative code over imperative code
- We aim for the most idiomatic Rust code

## AI Usage Guidelines

- Follow code conventions and folder structure
- Use existing types/functions when possible
- Avoid introducing new dependencies without justification
- Always prefer declarative over imperative code where applicable
