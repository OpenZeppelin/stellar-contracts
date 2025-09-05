# Soroban Smart Accounts

A comprehensive smart account framework for Soroban that enables flexible, programmable authorization through context rules, signers, and policies.

## Overview

Smart accounts in Soroban are contracts that implement the `CustomAccountInterface`, allowing them to define custom authorization logic beyond simple signature verification. This framework provides a modular, context-centric approach to authorization that separates:

- **Who** can authorize (signers)
- **What** they can authorize (context rules)
- **How** authorization is enforced (policies)

The design leverages Protocol 23 improvements for marginal storage read costs and significantly reduced cross-contract call costs.


## Core Components

### 1. Smart Account Trait

The `SmartAccount` trait extends `CustomAccountInterface` from `soroban_sdk` with context rule management capabilities:

```rust
pub trait SmartAccount: CustomAccountInterface {
    fn get_context_rule(e: &Env, context_rule_id: u32) -> ContextRule;
    fn get_context_rules(e: &Env, context_rule_type: ContextRuleType) -> Vec<ContextRule>;
    fn create_context_rule(/* ... */) -> ContextRule;
    fn update_context_rule_name(/* ... */) -> ContextRule;
    fn update_context_rule_valid_until(/* ... */) -> ContextRule;
    fn remove_context_rule(/* ... */);
    fn add_signer(/* ... */);
    fn remove_signer(/* ... */);
    fn add_policy(/* ... */);
    fn remove_policy(/* ... */);
}
```

### 2. Context Rules

Context rules are the fundamental building blocks that define authorization requirements for specific operations:

#### Structure

- **ID**: Unique identifier
- **Name**: Human-readable description
- **Context Type**: Scope of the rule
  - `Default`: Applies to any context
  - `CallContract(Address)`: Specific contract calls
  - `CreateContract(BytesN<32>)`: Contract deployments
- **Valid Until**: Optional expiration (ledger sequence)
- **Signers**: List of authorized signers (max: 15)
- **Policies**: Map of policy contracts and their parameters (max: 5)

#### Key Properties

- Each rule must contain at least one signer OR one policy
- Multiple rules can exist for the same context type
- Rules are evaluated in reverse chronological order (newest first)
- Expired rules are automatically filtered out

### 3. Signers

Signers define who can authorize operations. There are two variants:

#### Native Signers

```rust
Signer::Native(Address)
```

- Any Soroban address (contract or account)
- Verification uses `require_auth_for_args(payload)`
- **Caveat**: Requires manual authorization entry crafting, because it can be detected in a simulation mode

#### Delegated Signers

```rust
Signer::Delegated(Address, Bytes)
```

- External verifier contract + public key data
- Offloads signature verification to specialized contracts
- **Advantages**:
  - **Scalability**: Support for any cryptographic scheme
  - **Flexibility**: Can easily adapt to emerging authentication methods (zk-proofs, email signing)
  - **Low setup cost**: Reuse verifier contracts across accounts

### 4. Verifiers

Verifiers are specialized contracts that handle cryptographic signature verification. The main idea is to offload signature verification to trusted, immutable verifier contracts, similar to the mechanism described in EIP-7913.

```rust
pub trait Verifier {
    type SigData: FromVal<Env, Val> + FromXdr;

    fn verify(e: &Env, hash: Bytes, key_data: Bytes, sig_data: Self::SigData) -> bool;
}
```

#### Implementation Requirements

- **Stateless**: No internal state, pure verification functions
- **Immutable**: Cannot be upgraded once deployed (trustless operation)

#### Benefits of Shared Verifiers

1. **Security**: Centralized, audited verification logic reduces attack surface
2. **Efficiency**: Shared deployment costs across the ecosystem
3. **Scalability**: Easy adoption of new cryptographic schemes
4. **Trust**: Well-known, immutable contract addresses build ecosystem confidence

### 5. Policies

Policies provide programmable authorization logic that can be attached to context rules:

```rust
pub trait Policy {
    type AccountParams: FromVal<Env, Val>;

    fn can_enforce(/* ... */) -> bool;
    fn enforce(/* ... */);
    fn install(/* ... */);
    fn uninstall(/* ... */);
}
```

#### Lifecycle

1. **Installation**: Configure and attach to context rule
2. **Enforcement**: Validate authorization attempts
3. **Uninstallation**: Clean up and remove

#### Functions

- **`can_enforce()`**: Read-only pre-check, no state changes
- **`enforce()`**: State-changing hook, requires smart account authorization
- **`install()`**: Initialize policy-specific storage and configuration
- **`uninstall()`**: Clean up policy data

#### Policy Examples

- **Admin Access**: Elevated permissions for account management
- **Spending Limits**: Time-based or token-based restrictions
- **Multisig**: Threshold-based authorization
- **Session Policies**: Temporary, scoped permissions
- **Recovery Policies**: Account recovery mechanisms

### 6. Execution Entry Point

The `ExecutionEntryPoint` trait enables secure contract-to-contract calls:

```rust
pub trait ExecutionEntryPoint {
    fn execute(e: &Env, target: Address, target_fn: Symbol, target_args: Vec<Val>);
}
```

This prevents re-entry issues when policies need to authenticate back to their owner smart account.

## Authorization Flow

The smart account uses the following matching algorithm to determine authorization:

### 1. Rule Collection

- Retrieve all non-expired rules for the specific context type
- Include default rules that apply to any context
- Sort by creation time (newest first)

### 2. Rule Evaluation

For each rule in order:

1. **Signer Filtering**: Extract authenticated signers from the rule's signer list
2. **Policy Validation**: If policies exist, verify all can be enforced via `can_enforce()`
3. **Authorization Check**:
   - With policies: Success if all policies are enforceable
   - Without policies: Success if all signers are authenticated
4. **Rule Precedence**: First matching rule wins (newest takes precedence)

### 3. Policy Enforcement

- If authorization succeeds, call `enforce()` on all matched policies
- This triggers any necessary state changes (spending tracking, etc.)

### 4. Result

- **Success**: Authorization granted, transaction proceeds
- **Failure**: Authorization denied, transaction reverts

## Use Cases

### 1. Session Logins (Web3 dApps)

```rust,ignore
// Create a session policy for a DeFi app
create_context_rule(
    context_type: CallContract(defi_app_address),
    name: "DeFi Session",
    valid_until: Some(current_ledger + 24_hours),
    signers: vec![&e, session_key],
    policies: map![&E, (spending_limit_policy, spending_params)]
)
```

### 2. Backend Automation

```rust
// Recurring payment authorization
create_context_rule(
    context_type: CallContract(payment_processor),
    name: "Monthly Subscription",
    valid_until: None,
    signers: vec![&e, automation_key],
    policies: map![
        &e,
        (frequency_policy, monthly_params),
        (amount_policy, max_50_dollars)
    ]
)
```

### 3. AI Agents

```rust
// Controlled AI agent access
create_context_rule(
    context_type: Default,
    name: "Portfolio AI",
    valid_until: Some(current_ledger + 7_days),
    signers: vec![ai_agent_key],
    policies: map![
        &e,
        (whitelist_policy, allowed_functions),
        (balance_policy, max_percentage)
    ]
)
```

### 4. Multisig

```rust
// Complex multisig with mixed signer types
create_context_rule(
    context_type: Default,
    name: "Treasury Operations",
    valid_until: None,
    signers: vec![
        Signer::Delegated(ed25519_verifier, alice_pubkey),
        Signer::Delegated(secp256k1_verifier, bob_pubkey),
        Signer::Native(carol_contract)
    ],
    policies: map![&e, (threshold_policy, two_of_three)]
)
```

## Getting Started

### 1. Implement the Smart Account Trait

```rust
use stellar_accounts::smart_account::{
    add_context_rule, do_check_auth, ContextRule, ContextRuleType,
    Signatures, Signer, SmartAccount, SmartAccountError,
};
#[contract]
pub struct MySmartAccount;

#[contractimpl]
impl SmartAccount for MySmartAccount {
    fn add_context_rule(
        e: &Env,
        context_type: ContextRuleType,
        name: String,
        valid_until: Option<u32>,
        signers: Vec<Signer>,
        policies: Map<Address, Val>,
    ) -> ContextRule {
        e.current_contract_address().require_auth();

        add_context_rule(e, &context_type, name, valid_until, signers, policies)
    }
    // Implement all other methods
}

#[contractimpl]
impl CustomAccountInterface for MySmartAccount {
    type Signature = Signatures;

    fn __check_auth(
        env: Env,
        signature_payload: Hash<32>,
        signatures: Signatures,
        auth_context: Vec<Context>,
    ) -> Result<(), SmartAccountError> {
        do_check_auth(e, signature_payload, signatures, auth_contexts)
    }
}
```

### 2. Create Context Rules

```rust
// Create an admin rule
add_context_rule(
    &env,
    ContextRuleType::Default,
    String::from_str(&env, "Admin Access"),
    None, // No expiration
    vec![&env, admin_signer],
    Map::new(&env)
);
```

### 3. Add Policies (Optional)

For policies, you have two options:

**Option A: Use Ecosystem Policies (Recommended)**
- Use pre-deployed, audited policy contracts for common use cases
- These are trusted by the ecosystem and cover standard scenarios
- Examples: Simple threshold, weighted threshold, spending limits, time-based restrictions

**Option B: Create Custom Policies**
- Implement your own policy contracts by implementing the `Policy` trait
- Useful for specialized business logic or unique authorization requirements

```rust
// Add a spending limit policy
add_policy(
    &env,
    admin_rule.id,
    spending_policy_address,
    spending_limit_params
);
```

### 4. Choose or Deploy Verifier Contracts (For Delegated Signers)

For delegated signers, you have two options:

**Option A: Use Ecosystem Verifiers (Recommended)**

- Use pre-deployed, audited verifier contracts for common signature schemes
- These are trusted by the ecosystem and are immediately available
- Examples: Standard ed25519, secp256k1, secp256r1, bls12-381 verifiers

**Option B: Deploy Custom Verifiers**

- Deploy your own verifier contracts for application-specific requirements
- Useful for custom cryptographic schemes or specialized verification logic
- Requires thorough security auditing and testing

## Caveats

- Multiple context rules for the same context can co-exist, in which case the one added most recently takes precedence.
- For straightforward applications like a threshold-based multisig, the entire policy is a simple check (e.g., number of signers > threshold). While this might seem excessive, embedding business logic within the smart account would compromise the separation of concerns.
- The smart account framework's design assumes that its components are independent contracts that interact with each other, leading to multiple cross-contract calls during the authorization lifecycle. Although Protocol 23 significantly reduced the cost of these calls, some overhead in terms of cost and resources still exists. Consequently, the implementation imposes maximum limits on the number of signers and policies per context rule.

## Crate Structure

This crate is organized into three main submodules that provide building blocks for implementing smart accounts. These submodules can be used independently or together, allowing developers to implement only the components they need, create custom smart account architectures, mix and match different authentication methods, and build specialized authorization policies.

### `smart_account`
- **Core traits**: `SmartAccount` and `ExecutionEntryPoint`
- **Storage utilities**: Context rule management, signer/policy storage functions

### `verifiers` 
- **Core trait**: `Verifier` for cryptographic signature verification
- **Verifiers**: `ed25519` and `webauthn` (passkey authentication) utilities

### `policies`
- **Core trait**: `Policy` for programmable authorization logic
- **Policies**: `simple_threshold` and `weighted_threshold` utilities

