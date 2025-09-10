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
This proposal defines a standard contract interface for Real World Asset (RWA) tokens on Stellar. RWA tokens represent tokenized real-world assets such as securities, bonds, real estate, or other regulated financial instruments that require compliance with regulatory frameworks. The interface is based on the T-REX (Token for Regulated Exchanges) standard and extends fungible token functionality with comprehensive regulatory compliance features.

## Motivation
Real World Assets (RWAs) represent a significant opportunity for blockchain adoption, enabling the tokenization of traditional financial instruments and physical assets. However, unlike standard fungible tokens, RWAs must comply with complex regulatory requirements including:

- **Know Your Customer (KYC) and Anti-Money Laundering (AML)** compliance
- **Identity verification** and investor accreditation
- **Transfer restrictions** based on regulatory jurisdictions
- **Freezing capabilities** for regulatory enforcement
- **Recovery mechanisms** for lost or compromised wallets
- **Audit trails** and compliance reporting

Currently, while it is technically possible to create compliant tokens using existing fungible token standards, this approach lacks the sophisticated compliance infrastructure required for regulated securities. Standard fungible tokens cannot:

- **Enforce identity verification** - Standard tokens cannot verify that token holders meet regulatory requirements such as accredited investor status or jurisdictional compliance.
- **Implement transfer restrictions** - Regulatory compliance often requires restricting transfers based on identity verification, holding periods, or jurisdictional rules.
- **Provide freezing mechanisms** - Regulators may require the ability to freeze specific addresses or token amounts in case of suspicious activity or legal proceedings.
- **Enable wallet recovery** - For high-value securities, investors need mechanisms to recover tokens from lost or compromised wallets through proper identity verification.
- **Maintain compliance hooks** - Regulatory frameworks require hooks for compliance validation before transfers, minting, or burning operations.

The T-REX standard addresses these limitations by providing a comprehensive framework for compliant security tokens. This SEP adapts T-REX to the Stellar ecosystem, enabling:

- **Modular compliance framework** with pluggable compliance rules
- **Identity registry integration** for KYC/AML verification
- **Sophisticated freezing mechanisms** at both address and token levels
- **Administrative controls** with role-based access control (RBAC)
- **Recovery systems** for institutional-grade wallet management
- **Audit-ready event emission** for regulatory reporting

By establishing this standard, RWA tokens can interact seamlessly with DeFi protocols, custody solutions, and compliance infrastructure while maintaining full regulatory compliance.

## Interface
The RWA token interface extends the standard fungible token functionality with regulatory compliance features. The interface is designed to be modular, allowing implementations to plug in different compliance rules, identity verification systems, and administrative controls based on specific regulatory requirements.

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
pub trait RWAToken: Pausable + FungibleToken<ContractType = RWA> {
    // ################## CORE TOKEN FUNCTIONS ##################

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
    fn version(e: &Env) -> String;

    /// Returns the address of the onchain ID of the token.
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

    /// Sets the claim topics and issuers contract of the token.
    /// This function can only be called by the operator with necessary
    /// privileges. RBAC checks are expected to be enforced on the
    /// `operator`.
    ///
    /// # Arguments
    ///
    /// * `e` - Access to the Soroban environment.
    /// * `claim_topics_and_issuers` - The address of the claim topics and
    ///   issuers contract to set.
    /// * `operator` - The address of the operator.
    ///
    /// # Events
    ///
    /// * topics - `["claim_topics_issuers_set", claim_topics_and_issuers:
    ///   Address]`
    /// * data - `[]`
    fn set_claim_topics_and_issuers(e: &Env, claim_topics_and_issuers: Address, operator: Address);

    /// Sets the identity registry storage contract of the token.
    /// This function can only be called by the operator with necessary
    /// privileges. RBAC checks are expected to be enforced on the
    /// `operator`.
    ///
    /// # Arguments
    ///
    /// * `e` - Access to the Soroban environment.
    /// * `identity_registry_storage` - The address of the identity registry
    ///   storage contract to set.
    /// * `operator` - The address of the operator.
    ///
    /// # Events
    ///
    /// * topics - `["identity_registry_storage_set", identity_registry_storage:
    ///   Address]`
    /// * data - `[]`
    fn set_identity_registry_storage(
        e: &Env,
        identity_registry_storage: Address,
        operator: Address,
    );

    /// Returns the Compliance contract linked to the token.
    fn compliance(e: &Env) -> Address;

    /// Returns the Claim Topics and Issuers contract linked to the token.
    fn claim_topics_and_issuers(e: &Env) -> Address;

    /// Returns the Identity Registry Storage contract linked to the token.
    fn identity_registry_storage(e: &Env) -> Address;
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

### Claim Topics and Issuers Set Event
The claim topics and issuers set event is emitted when the claim topics and issuers contract is updated.

**Topics:**
- `Symbol` with value `"claim_topics_issuers_set"`
- `Address`: the address of the new claim topics and issuers contract

**Data:**
- Empty array `[]`

### Identity Registry Storage Set Event
The identity registry storage set event is emitted when the identity registry storage contract is updated.

**Topics:**
- `Symbol` with value `"identity_registry_storage_set"`
- `Address`: the address of the new identity registry storage contract

**Data:**
- Empty array `[]`

## Notes on Regulatory Compliance

### Identity Verification
RWA tokens require robust identity verification mechanisms to ensure compliance with KYC/AML regulations. The identity system consists of:

- **Identity Registry Storage**: Stores investor profiles, including personal information, jurisdiction, and investor type (individual/organization)
- **Claim Topics and Issuers**: Manages trusted claim issuers and defines claim topics (e.g., KYC=1, AML=2, Accredited Investor=3)
- **Identity Claims**: Validates cryptographic claims with support for multiple signature schemes (Ed25519, Secp256k1, Secp256r1)

### Compliance Framework
The modular compliance framework allows for pluggable compliance rules that can be customized based on regulatory requirements:

- **Transfer Validation**: Before any transfer, the compliance contract validates whether the transfer meets regulatory requirements
- **Compliance Hooks**: The system provides hooks for `Transferred`, `Created`, `Destroyed`, `CanTransfer`, and `CanCreate` events
- **Jurisdiction Rules**: Compliance rules can enforce jurisdiction-specific restrictions and requirements

### Freezing Mechanisms
RWA tokens support sophisticated freezing capabilities for regulatory enforcement:

- **Address-Level Freezing**: Completely freeze an address, preventing all token operations
- **Partial Token Freezing**: Freeze specific amounts of tokens while allowing operations on unfrozen balances
- **Regulatory Enforcement**: Freezing can be triggered by regulatory requirements, suspicious activity, or legal proceedings

### Recovery System
The recovery system enables institutional-grade wallet management:

- **Identity-Based Recovery**: Recovery requires verification of the investor's onchain identity
- **Operator Authorization**: Recovery operations must be performed by authorized operators with proper RBAC permissions
- **Audit Trail**: All recovery operations are logged with comprehensive event emission for regulatory reporting

## Notes on Role-Based Access Control (RBAC)

RWA tokens implement comprehensive RBAC to ensure that sensitive operations are only performed by authorized entities:

- **Operator Roles**: Different operator roles can be defined with specific permissions for minting, burning, freezing, recovery, and compliance management
- **Administrative Functions**: All administrative functions require proper operator authorization
- **Compliance Integration**: RBAC permissions can be integrated with compliance rules to ensure regulatory requirements are met

## Notes on Integration with Fungible Tokens

RWA tokens extend the standard fungible token interface while adding regulatory compliance features:

- **Base Functionality**: All standard fungible token operations (transfer, approve, allowance) are available
- **Compliance Overrides**: Standard operations are enhanced with compliance checks and identity verification
- **Pausable Operations**: The entire token can be paused for emergency situations while maintaining compliance
- **Event Compatibility**: RWA events are compatible with standard fungible token events, enabling integration with existing infrastructure

## Notes on T-REX Standard Compliance

This SEP implements the T-REX (Token for Regulated Exchanges) standard adapted for the Stellar ecosystem:

- **Regulatory Framework**: Provides a comprehensive framework for compliant security tokens
- **Modular Design**: Allows customization of compliance rules, identity verification, and administrative controls
- **Industry Adoption**: T-REX is widely adopted in the traditional finance industry for security token issuance
- **Audit-Ready**: Designed with regulatory reporting and audit requirements in mind

## Security Considerations

### Access Control
- All administrative functions must implement proper RBAC checks
- Operator permissions should be carefully managed and regularly audited
- Multi-signature schemes should be considered for high-privilege operations

### Identity Verification
- Identity claims must be cryptographically verified using trusted issuers
- Claim expiration and revocation mechanisms should be implemented
- Regular re-verification of investor identities may be required based on regulatory requirements

### Compliance Validation
- All token operations should be validated against current compliance rules
- Compliance contracts should be upgradeable to adapt to changing regulations
- Emergency pause mechanisms should be available for regulatory compliance

### Freezing and Recovery
- Freezing operations should be logged and auditable
- Recovery operations should require multiple levels of authorization
- Time-locked recovery mechanisms can provide additional security for high-value operations
