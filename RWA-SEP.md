
## Preamble
```
SEP: 0051
Title: Real World Asset (RWA) Tokens
Author: OpenZeppelin, Boyan Barakov <@brozorec>, Özgün Özerk <@ozgunozerk>
Status: Draft
Created: 2025-01-10
Updated: 2025-01-10
Version: 0.1.0
Discussion: TBD
```

## Summary
This proposal defines a standard contract interface for Real World Asset (RWA) tokens on Stellar. RWA tokens represent tokenized real-world assets such as securities, bonds, real estate, or other regulated financial instruments that require compliance with regulatory frameworks.

This standard is based on the T-REX (Token for Regulated Exchanges) framework, as implemented in ERC-3643 (https://github.com/ERC-3643/ERC-3643), but introduces significant architectural improvements for flexibility and modularity.

## Motivation
Real World Assets (RWAs) represent a significant opportunity for blockchain adoption, enabling the tokenization of traditional financial instruments and physical assets. However, unlike standard fungible tokens, RWAs must comply with complex regulatory requirements including but not limited to:

- **Know Your Customer (KYC) and Anti-Money Laundering (AML)** compliance
- **Identity verification** and investor accreditation
- **Freezing capabilities** for regulatory enforcement
- **Recovery mechanisms** for lost or compromised wallets
- **Compliance hooks** for regulatory reporting

The T-REX standard provides a comprehensive framework for compliant security tokens. This SEP adapts T-REX to the Stellar ecosystem, enabling:

- **Modular compliance framework** with pluggable compliance rules
- **Flexible identity verification** supporting multiple approaches (claim-based, Merkle tree, zero-knowledge, etc.)
- **Sophisticated freezing mechanisms** at both address and token levels
- **Administrative controls** with role-based access control (RBAC)
- **Recovery systems** for institutional-grade wallet management
- **Compliance hooks** for regulatory reporting

## Architecture Overview

Based on extensive research and collaboration with industry experts, this RWA standard introduces an approach built around **loose coupling** and **implementation abstraction**, addressing key limitations identified in existing standards.

### Core Design Principles

1. **Separation of Concerns**: Core token functionality is cleanly separated from compliance and identity verification
2. **Implementation Flexibility**: Compliance and identity systems are treated as pluggable implementation details
3. **Shared Infrastructure**: Components can be shared across multiple token contracts to reduce deployment and management costs
4. **Regulatory Adaptability**: The system can adapt to different regulatory frameworks without core changes

### Component Architecture

The Stellar T-REX consists of several interconnected but loosely coupled components:

```
┌─────────────────┐    ┌──────────────────┐    ┌─────────────────────┐
│   RWA Token     │───▶│   Compliance     │───▶│  Compliance Modules │
│   (Core)        │    │   Contract       │    │  (Pluggable Rules)  │
└─────────────────┘    └──────────────────┘    └─────────────────────┘
         │
         ▼
┌─────────────────┐    ┌──────────────────┐    ┌──────────────────┐
│ Identity        │───▶│ Claim Topics &   │    │ Custom Identity  │
│ Verifier        │    │ Issuers          │───▶│ Registry         │
└─────────────────┘    └──────────────────┘    └──────────────────┘
```

### Flexibility Through Abstraction

The architecture supports multiple implementation strategies:

- **Identity Verification**: Merkle trees, Zero-Knowledge proofs, claim-based systems, or custom approaches
- **Compliance Rules**: Modular hook-based system supporting diverse regulatory requirements
- **Access Control**: Integration with external RBAC systems

This design enables the same core RWA interface to work with vastly different regulatory and technical requirements.

## Interface

The RWA token interface extends the **fungible token** ([SEP-41](https://github.com/stellar/stellar-protocol/blob/master/ecosystem/sep-0041.md)) functionality with regulatory compliance features.

### Architecture Overview

The RWA Token contract requires only **two external functions** to operate:

```rust
// Compliance validation - returns true if transfer is allowed
fn can_transfer(e: &Env, from: Address, to: Address, amount: i128) -> bool;

// Identity verification - panics if user is not verified
fn verify_identity(e: &Env, user_address: &Address);
```

- `can_transfer()` is expected to be exposed from a compliance contract.
- `verify_identity()` is expected to be exposed from an identity verifier contract.

These functions are **deliberately abstracted** as implementation details, enabling:
- **Regulatory Flexibility**: Different jurisdictions can implement different compliance logic
- **Technical Flexibility**: Various identity verification approaches (ZK, Merkle trees, claims)
- **Cost Optimization**: Shared contracts across multiple tokens
- **Future-Proofing**: New compliance approaches without interface changes

In other words, the only thing required by this RWA token design, is that the RWA token should be able to call these expected functions made available by the compliance and identity verification contracts.

### Contract Connection Interface

The RWA Token provides simple setter/getter functions for external contracts:

```rust
// Compliance Contract Management
fn set_compliance(e: &Env, compliance: Address, operator: Address);
fn compliance(e: &Env) -> Address;

// Identity Verifier Contract Management
fn set_identity_verifier(e: &Env, identity_verifier: Address, operator: Address);
fn identity_verifier(e: &Env) -> Address;
```

### Integration Pattern

To deploy a compliant RWA token and make it functional:

1. **Deploy Core RWA Token**
2. **Deploy/Connect Compliance Contract**
3. **Deploy/Connect Identity Verifier**
4. **Configure Connections**
5. **Configure Rules** (Set up compliance modules and identity requirements)

```rust
use soroban_sdk::{Address, Env, String};
use stellar_contract_utils::pausable::Pausable;
use crate::fungible::FungibleToken;

/// Real World Asset Token Trait
///
/// The `RWAToken` trait defines the core functionality for Real World Asset
/// tokens, implementing the T-REX standard for regulated securities. It
/// provides a comprehensive interface for managing compliant token transfers,
/// identity verification, compliance rules, and administrative controls.
///
/// This trait extends basic fungible token functionality with regulatory
/// features required for security tokens.
pub trait RWAToken: TokenInterface {
    // ################## CORE TOKEN FUNCTIONS ##################

    /// Returns the total amount of tokens in circulation.
    ///
    /// # Arguments
    ///
    /// * `e` - Access to the Soroban environment.
    fn total_supply(e: &Env) -> i128;

    /// Forces a transfer of tokens between two whitelisted wallets.
    /// This function can only be called by the operator with necessary
    /// privileges. RBAC checks are expected to be enforced on the
    /// `operator`.
    ///
    /// # Arguments
    ///
    /// * `e` - Access to the Soroban environment.
    /// * `from` - The address of the sender.
    /// * `to` - The address of the receiver.
    /// * `amount` - The number of tokens to transfer.
    /// * `operator` - The address of the operator.
    ///
    /// # Events
    ///
    /// * topics - `["transfer", from: Address, to: Address]`
    /// * data - `[amount: i128]`
    fn forced_transfer(e: &Env, from: Address, to: Address, amount: i128, operator: Address);

    /// Mints tokens to a wallet. Tokens can only be minted to verified
    /// addresses. This function can only be called by the operator with
    /// necessary privileges. RBAC checks are expected to be enforced on the
    /// `operator`.
    ///
    /// # Arguments
    ///
    /// * `e` - Access to the Soroban environment.
    /// * `to` - Address to mint the tokens to.
    /// * `amount` - Amount of tokens to mint.
    /// * `operator` - The address of the operator.
    ///
    /// # Events
    ///
    /// * topics - `["mint", to: Address]`
    /// * data - `[amount: i128]`
    fn mint(e: &Env, to: Address, amount: i128, operator: Address);

    /// Burns tokens from a wallet.
    /// This function can only be called by the operator with necessary
    /// privileges. RBAC checks are expected to be enforced on the
    /// `operator`.
    ///
    /// # Arguments
    ///
    /// * `e` - Access to the Soroban environment.
    /// * `user_address` - Address to burn the tokens from.
    /// * `amount` - Amount of tokens to burn.
    /// * `operator` - The address of the operator.
    ///
    /// # Events
    ///
    /// * topics - `["burn", user_address: Address]`
    /// * data - `[amount: i128]`
    fn burn(e: &Env, user_address: Address, amount: i128, operator: Address);

    /// Recovery function used to force transfer tokens from a lost wallet
    /// to a new wallet for an investor.
    /// This function can only be called by the operator with necessary
    /// privileges. RBAC checks are expected to be enforced on the
    /// `operator`.
    ///
    /// # Arguments
    ///
    /// * `e` - Access to the Soroban environment.
    /// * `lost_wallet` - The wallet that the investor lost.
    /// * `new_wallet` - The newly provided wallet for token transfer.
    /// * `investor_onchain_id` - The onchain ID of the investor asking for
    ///   recovery.
    /// * `operator` - The address of the operator.
    ///
    /// # Events
    ///
    /// * topics - `["transfer", lost_wallet: Address, new_wallet: Address]`
    /// * data - `[amount: i128]`
    /// * topics - `["recovery", lost_wallet: Address, new_wallet: Address,
    ///   investor_onchain_id: Address]`
    /// * data - `[]`
    fn recovery_address(
        e: &Env,
        lost_wallet: Address,
        new_wallet: Address,
        investor_onchain_id: Address,
        operator: Address,
    ) -> bool;

    /// Sets the frozen status for an address. Frozen addresses cannot send or
    /// receive tokens. This function can only be called by the operator
    /// with necessary privileges. RBAC checks are expected to be enforced
    /// on the `operator`.
    ///
    /// # Arguments
    ///
    /// * `e` - Access to the Soroban environment.
    /// * `user_address` - The address for which to update frozen status.
    /// * `freeze` - Frozen status of the address.
    /// * `operator` - The address of the operator.
    ///
    /// # Events
    ///
    /// * topics - `["address_frozen", user_address: Address, is_frozen: bool,
    ///   operator: Address]`
    /// * data - `[]`
    fn set_address_frozen(e: &Env, user_address: Address, freeze: bool, operator: Address);

    /// Freezes a specified amount of tokens for a given address.
    /// This function can only be called by the operator with necessary
    /// privileges. RBAC checks are expected to be enforced on the
    /// `operator`.
    ///
    /// # Arguments
    ///
    /// * `e` - Access to the Soroban environment.
    /// * `user_address` - The address for which to freeze tokens.
    /// * `amount` - Amount of tokens to be frozen.
    /// * `operator` - The address of the operator.
    ///
    /// # Events
    ///
    /// * topics - `["tokens_frozen", user_address: Address]`
    /// * data - `[amount: i128]`
    fn freeze_partial_tokens(e: &Env, user_address: Address, amount: i128, operator: Address);

    /// Unfreezes a specified amount of tokens for a given address.
    /// This function can only be called by the operator with necessary
    /// privileges. RBAC checks are expected to be enforced on the
    /// `operator`.
    ///
    /// # Arguments
    ///
    /// * `e` - Access to the Soroban environment.
    /// * `user_address` - The address for which to unfreeze tokens.
    /// * `amount` - Amount of tokens to be unfrozen.
    /// * `operator` - The address of the operator.
    ///
    /// # Events
    ///
    /// * topics - `["tokens_unfrozen", user_address: Address]`
    /// * data - `[amount: i128]`
    fn unfreeze_partial_tokens(e: &Env, user_address: Address, amount: i128, operator: Address);

    /// Returns the freezing status of a wallet.
    ///
    /// # Arguments
    ///
    /// * `e` - Access to the Soroban environment.
    /// * `user_address` - The address of the wallet to check.
    fn is_frozen(e: &Env, user_address: Address) -> bool;

    /// Returns the amount of tokens that are partially frozen on a wallet.
    ///
    /// # Arguments
    ///
    /// * `e` - Access to the Soroban environment.
    /// * `user_address` - The address of the wallet to check.
    fn get_frozen_tokens(e: &Env, user_address: Address) -> i128;

    // ################## METADATA FUNCTIONS ##################

    /// Returns the version of the token (T-REX version).
    ///
    /// # Arguments
    ///
    /// * `e` - Access to the Soroban environment.
    fn version(e: &Env) -> String;

    /// Returns the address of the onchain ID of the token.
    ///
    /// # Arguments
    ///
    /// * `e` - Access to the Soroban environment.
    fn onchain_id(e: &Env) -> Address;

    // ################## COMPLIANCE AND IDENTITY FUNCTIONS ##################

    /// Sets the compliance contract of the token.
    /// This function can only be called by the operator with necessary
    /// privileges. RBAC checks are expected to be enforced on the
    /// `operator`.
    ///
    /// # Arguments
    ///
    /// * `e` - Access to the Soroban environment.
    /// * `compliance` - The address of the compliance contract to set.
    /// * `operator` - The address of the operator.
    ///
    /// # Events
    ///
    /// * topics - `["compliance_set", compliance: Address]`
    /// * data - `[]`
    fn set_compliance(e: &Env, compliance: Address, operator: Address);

    /// Sets the identity verifier contract of the token.
    ///
    /// This function can only be called by the operator with necessary
    /// privileges. RBAC checks are expected to be enforced on the
    /// `operator`.
    ///
    /// # Arguments
    ///
    /// * `e` - Access to the Soroban environment.
    /// * `identity_verifier` - The address of the identity verifier contract to
    ///   set.
    /// * `operator` - The address of the operator.
    ///
    /// # Events
    ///
    /// * topics - `["identity_verifier_set", identity_verifier: Address]`
    /// * data - `[]`
    fn set_identity_verifier(e: &Env, identity_verifier: Address, operator: Address);

    /// Returns the Compliance contract linked to the token.
    ///
    /// # Arguments
    ///
    /// * `e` - Access to the Soroban environment.
    fn compliance(e: &Env) -> Address;

    /// Returns the Identity Verifier contract linked to the token.
    ///
    /// # Arguments
    ///
    /// * `e` - Access to the Soroban environment.
    fn identity_verifier(e: &Env) -> Address;

    // ################## PAUSABLE FUNCTIONS ##################

    /// Returns true if the contract is paused, and false otherwise.
    ///
    /// # Arguments
    ///
    /// * `e` - Access to Soroban environment.
    /// * `caller` - The address of the caller.
    fn pause(e: &Env, caller: Address);

    /// Triggers `Unpaused` state.
    ///
    /// # Arguments
    ///
    /// * `e` - Access to Soroban environment.
    /// * `caller` - The address of the caller.
    fn unpause(e: &Env, caller: Address);

}
```

## Events

### Transfer Event
The transfer event is emitted when RWA tokens are transferred from one address to another, including forced transfers and recovery operations.

**Topics:**
- `Symbol` with value `"transfer"`
- `Address`: the address holding the tokens that were transferred
- `Address`: the address that received the tokens

**Data:**
- `i128`: the amount of tokens transferred

### Mint Event
The mint event is emitted when RWA tokens are minted to a verified address.

**Topics:**
- `Symbol` with value `"mint"`
- `Address`: the address receiving the newly minted tokens

**Data:**
- `i128`: the amount of tokens minted

### Burn Event
The burn event is emitted when RWA tokens are burned from an address.

**Topics:**
- `Symbol` with value `"burn"`
- `Address`: the address from which tokens were burned

**Data:**
- `i128`: the amount of tokens burned

### Recovery Event
The recovery event is emitted when tokens are successfully recovered from a lost wallet to a new wallet.

**Topics:**
- `Symbol` with value `"recovery"`
- `Address`: the lost wallet address
- `Address`: the new wallet address
- `Address`: the investor's onchain ID

**Data:**
- Empty array `[]`

### Address Frozen Event
The address frozen event is emitted when an address is frozen or unfrozen.

**Topics:**
- `Symbol` with value `"address_frozen"`
- `Address`: the address that was frozen/unfrozen
- `bool`: the frozen status (true for frozen, false for unfrozen)
- `Address`: the operator who performed the action

**Data:**
- Empty array `[]`

### Tokens Frozen Event
The tokens frozen event is emitted when a specific amount of tokens is frozen for an address.

**Topics:**
- `Symbol` with value `"tokens_frozen"`
- `Address`: the address for which tokens were frozen

**Data:**
- `i128`: the amount of tokens frozen

### Tokens Unfrozen Event
The tokens unfrozen event is emitted when a specific amount of tokens is unfrozen for an address.

**Topics:**
- `Symbol` with value `"tokens_unfrozen"`
- `Address`: the address for which tokens were unfrozen

**Data:**
- `i128`: the amount of tokens unfrozen

### Compliance Set Event
The compliance set event is emitted when the compliance contract is updated.

**Topics:**
- `Symbol` with value `"compliance_set"`
- `Address`: the address of the new compliance contract

**Data:**
- Empty array `[]`

### Identity Verifier Set Event
The identity verifier set event is emitted when the identity verifier contract is updated.

**Topics:**
- `Symbol` with value `"identity_verifier_set"`
- `Address`: the address of the new identity verifier contract

**Data:**
- Empty array `[]`

## Component Deep Dive

### 1. Identity Verification System

**Philosophy**: The entire identity stack is treated as an implementation detail, enabling maximum regulatory and technical flexibility.

#### The IdentityVerifier Trait

```rust
pub trait IdentityVerifier {
    /// Core verification function - panics if user is not verified
    fn verify_identity(e: &Env, user_address: &Address);

    // Setters and getters for the claim topics and issuers contract
    fn set_claim_topics_and_issuers(e: &Env, contract: Address, operator: Address);
    fn claim_topics_and_issuers(e: &Env) -> Address;
}
```

#### Implementation Strategies

Different regulatory environments may require different approaches. Here are some examples:

**1. Claim-Based Verification (Reference Implementation)**
- **Use Case**: Traditional KYC/AML with trusted issuers
- **Components**: ClaimTopicsAndIssuers + IdentityRegistryStorage + IdentityClaims
- **Benefits**: Familiar to regulators, rich metadata support

**2. Merkle Tree Verification**
- **Use Case**: Privacy-focused compliance with efficient proofs
- **Components**: ClaimTopicsAndIssuers + Merkle root storage + proof validation
- **Benefits**: Minimal storage, efficient verification

**3. Zero-Knowledge Verification**
- **Use Case**: Privacy-preserving compliance
- **Components**: ClaimTopicsAndIssuers + ZK circuit + proof verification
- **Benefits**: Maximum privacy, selective disclosure

**4. Custom Approaches**
- **Use Case**: Jurisdiction-specific requirements
- **Components**: ClaimTopicsAndIssuers + Custom verification logic
- **Benefits**: Tailored to specific regulatory needs

#### Reference Implementation Architecture

Our claim-based reference implementation demonstrates the full complexity of traditional RWA compliance:

```
┌─────────────────────┐
│  Identity Verifier  │
│  (Orchestrator)     │
│                     │
└─────────────────────┘
              │
              ├────▶┌───────────────────────────┐
              │     │ Claim Topics & Issuers    │
              │     │ (Shared Registry)         │
              │     └───────────────────────────┘
              │
              ├────▶┌───────────────────────────┐
              │     │ Identity Registry Storage │
              │     │ (User Profiles)           │
              │     └───────────────────────────┘
              │
              ├────▶┌───────────────────────────┐
              │     │ Identity Claims           │
              │     │ (Claim Validation)        │
              │     └───────────────────────────┘
              │
              └────▶┌───────────────────────────┐
                    │ Claim Issuer              │
                    │ (Signature Validation)    │
                    └───────────────────────────┘
```

**Key Components:**

- **ClaimTopicsAndIssuers**: Merged registry managing both trusted issuers and required claim types (KYC=1, AML=2, etc.)
- **IdentityRegistryStorage**: Component storing identity profiles with country relations and metadata
- **IdentityClaims**: Validates cryptographic claims using multiple signature schemes (Ed25519, Secp256k1, Secp256r1), with an emphasis on interoperability with the evolving OnchainID specification (https://github.com/ERC-3643/ERCs/blob/erc-oid/ERCS/erc-xxxx.md).
- **ClaimIssuer**: Builds and validates cryptographic claims about user attributes

### 2. Compliance System

Modular hook-based architecture supporting diverse regulatory requirements through pluggable compliance modules.

#### The Compliance Trait

```rust
pub trait Compliance {
    // Module management
    fn add_module_to(e: &Env, hook: ComplianceHook, module: Address, operator: Address);
    fn remove_module_from(e: &Env, hook: ComplianceHook, module: Address, operator: Address);

    // Validation hooks (READ-only)
    fn can_transfer(e: &Env, from: Address, to: Address, amount: i128) -> bool;
    fn can_create(e: &Env, to: Address, amount: i128) -> bool;

    // State update hooks (called after successful operations)
    fn transferred(e: &Env, from: Address, to: Address, amount: i128);
    fn created(e: &Env, to: Address, amount: i128);
    fn destroyed(e: &Env, from: Address, amount: i128);
}
```

#### Hook-Based Architecture

The compliance system uses a sophisticated hook mechanism:

```rust
pub enum ComplianceHook {
    CanTransfer,    // Pre-validation: Check if transfer meets compliance rules
    CanCreate,      // Pre-validation: Check if mint operation is compliant
    Transferred,    // Post-event: Update state after successful transfer
    Created,        // Post-event: Update state after successful mint
    Destroyed,      // Post-event: Update state after successful burn
}
```

#### Compliance Module Examples

**Transfer Limits Module**:
- Hook: `CanTransfer` + `Transferred`
- Logic: Enforce daily/monthly transfer limits per user

**Jurisdiction Restrictions Module**:
- Hook: `CanTransfer`
- Logic: Block transfers between incompatible jurisdictions

**Holding Period Module**:
- Hook: `CanTransfer` + `Created`
- Logic: Enforce minimum holding periods for newly minted tokens

**Investor Accreditation Module**:
- Hook: `CanCreate`
- Logic: Verify investor accreditation before minting

#### Shared Compliance Infrastructure

Compliance contracts are designed to be **shared across multiple RWA tokens**, reducing deployment costs and ensuring consistent regulatory enforcement:

```
┌─────────────┐    ┌─────────────────────┐    ┌──────────────────┐
│ RWA Token A │───▶│                     │───▶│ Transfer Limits  │
├─────────────┤    │   Shared Compliance │    │ Module           │
│ RWA Token B │───▶│   Contract          │───▶├──────────────────┤
├─────────────┤    │                     │    │ Jurisdiction     │
│ RWA Token C │───▶│                     │    │ Module           │
└─────────────┘    └─────────────────────┘    └──────────────────┘
```


### 3. Advanced Token Controls

#### Freezing Mechanisms

Based on regulatory requirements research, the system supports multiple freezing strategies:

**Address-Level Freezing**:
```rust
fn set_address_frozen(e: &Env, user_address: Address, freeze: bool, operator: Address);
fn is_frozen(e: &Env, user_address: Address) -> bool;
```
- **Use Case**: Complete account suspension for regulatory investigations
- **Effect**: Prevents all token operations (send/receive)

**Partial Token Freezing**:
```rust
fn freeze_partial_tokens(e: &Env, user_address: Address, amount: i128, operator: Address);
fn unfreeze_partial_tokens(e: &Env, user_address: Address, amount: i128, operator: Address);
fn get_frozen_tokens(e: &Env, user_address: Address) -> i128;
```
- **Use Case**: Escrow scenarios, disputed transactions
- **Effect**: Freezes specific token amounts while allowing operations on remaining balance

#### Recovery System

Institutional-grade wallet recovery for high-value securities:

```rust
fn recovery_address(
    e: &Env,
    lost_wallet: Address,
    new_wallet: Address,
    investor_onchain_id: Address,
    operator: Address
) -> bool;
```

**Recovery Process**:
1. **Identity Verification**: Verify investor's onchain identity matches the lost wallet
2. **Authorization Check**: Ensure operator has recovery permissions
3. **Balance Transfer**: Move all tokens from lost wallet to new wallet
4. **Identity Update**: Update identity registry to map new wallet to existing identity
5. **Audit Trail**: Emit comprehensive events for regulatory reporting

#### Forced Transfers

For regulatory compliance (court orders, sanctions):

```rust
fn forced_transfer(e: &Env, from: Address, to: Address, amount: i128, operator: Address);
```

- **Use Case**: Court-ordered asset transfers, regulatory seizures
- **Authorization**: Requires operator with forced transfer permissions
- **Compliance**: Bypasses normal compliance validation checks (operator responsibility)

### 4. Access Control & Governance

RWA tokens require proper access control to ensure that sensitive operations are only performed by authorized entities:

- **Operator Authorization**: All administrative functions require proper operator authorization using Soroban's built-in authorization mechanisms
- **Flexible Access Control**: While the RWA interface itself doesn't prescribe a specific access control model, implementations can integrate with external access control systems as needed
- **Compliance Integration**: Access control permissions should be integrated with compliance rules to ensure regulatory requirements are met

## Research-Driven Design Decisions

### Addressing ERC-3643 Limitations

Through our (OpenZeppelin) collaboration with Tokeny, Stellar, we identified key limitations in existing RWA standards:

**1. Tight Coupling Issues**
- **Problem**: ERC-3643 tightly couples identity verification with specific contract structures
- **Solution**: Abstract identity verification as pluggable implementation detail

**2. Inflexible Identity Models**
- **Problem**: Hardcoded ERC-734/735 identity contracts don't translate to all blockchain architectures
- **Solution**: Support multiple identity verification approaches (Merkle, ZK, claims, custom)

**3. Redundant Contract Hierarchies**
- **Problem**: Complex IdentityRegistry ↔ IdentityRegistryStorage relationships
- **Solution**: Direct access patterns

**4. Limited Compliance Flexibility**
- **Problem**: Monolithic compliance validation
- **Solution**: Modular hook-based compliance system

## Upgrade and Migration Strategies

**Compliance Evolution**:
- Modular compliance system supports rule updates without token redeployment
- New compliance modules can be added for evolving regulations

**Identity System Migration**:
- Abstract identity verification enables migration between verification approaches
- Gradual migration strategies for existing user bases
