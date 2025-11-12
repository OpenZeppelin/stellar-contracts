# Soroban Smart Accounts

This package provides a comprehensive smart account framework for Soroban, enabling flexible, programmable authorization. Instead of hard‑coding signature checks, smart accounts organize authorization as a composition of context rules, signers, and policies. The result is a system that reads naturally, scales to complex requirements, and remains auditable.

## Overview

Smart accounts in Soroban implement `CustomAccountInterface` and define authorization as data and behavior that can be evolved over time. The framework is context‑centric:

It distinguishes who is allowed to act (signers), what they are allowed to do (context rules), and how those permissions are enforced (policies). Under the hood, Protocol 23 improvements make this design practical, with marginal storage read costs and substantially cheaper cross‑contract calls, so composing multiple checks is efficient enough for production.

In practical terms, a smart account is a contract that manages the composition of authorization intents coming from multiple sources. Those sources can be policies (for example, a spending limit) and signing keys that may use different cryptographic curves. The goal is to enable flexible combinations of authentication methods by allowing several authorization mechanisms to work together seamlessly. For instance, a wallet might require both a session policy and a passkey that expires in 24 hours, and treat this combination as a single composite “key” that the client uses to authorize actions.

## Core Components

![context rule](./docs/SmartAccount-ContextRules.png "Context Rule Structure")

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

Context rules function like routing tables for authorization: for each context, they specify scope, lifetime, and the conditions, signers and policies, that must match before execution proceeds:

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

In addition, a single smart account can hold multiple context rules across its contexts. The maximum number of context rules per smart account is 15.

#### Key Properties

- Each rule must contain at least one signer OR one policy
- Multiple rules can exist for the same context type
- Rules are evaluated in reverse chronological order (newest first)
- Expired rules are automatically filtered out

### 3. Signers

Signers define who can authorize operations. There are two variants:

#### Delegated Signers

```rust
Signer::Delegated(Address)
```

- Any Soroban address (contract or account)
- Verification uses `require_auth_for_args(payload)`
- This model requires manual authorization entry crafting, because it is not returned in a simulation mode.

#### External Signers

```rust
Signer::External(Address, Bytes)
```
- External verifier contract + public key data
- Offloads signature verification to specialized contracts
This model scales to diverse cryptographic schemes, is flexible enough to accommodate new authentication methods (from passkeys to zk-proofs), and minimizes setup cost by allowing many accounts to reuse the same verifier contracts.

![external signers with verifying contracts](./docs/ExternalSigner.png "External Signers with Verifier Contracts")

### 4. Verifiers

Verifiers serve as cryptographic oracles for signature validation: specialized, trusted contracts that validate signatures on behalf of smart accounts. Drawing inspiration from EIP‑7913, a single verifier contract can validate signatures for any number of keys. Each key is represented as a `(verifier_address, public_key)` pair (ref. the section above), where the verifier address points to shared verification logic and the public key identifies the specific signer.

This architecture offers several advantages. Once a verifier is deployed, new keys can be used immediately without any on‑chain setup or deployment expenses. The model supports diverse cryptographic schemes: secp256r1 for mobile devices, secp256k1, ed25519, BLS, and RSA for institutional keys, as well as emerging authentication methods like zero‑knowledge proofs and email‑based signing. Keys remain address‑less, maintaining clear boundaries between accounts (which hold assets) and the keys that control them. Because verification logic is centralized in well‑audited, immutable contracts, the ecosystem shares both security guarantees and deployment costs. Well‑known verifier addresses build trust, and the entire network benefits from reduced overhead.

Verifiers should be implemented as pure verification functions with no internal state and shouldn't be upgradeable once deployed, ensuring trustlessness.

```rust
pub trait Verifier {
    type KeyData: FromVal<Env, Val>;
    type SigData: FromVal<Env, Val>;

    fn verify(e: &Env, hash: Bytes, key_data: Self::KeyData, sig_data: Self::SigData) -> bool;
}
```

### 5. Policies

Policies act as enforcement modules attached to context rules: they perform read‑only prechecks and, when authorized, can update state to enforce limits or workflows.

```rust
pub trait Policy {
    type AccountParams: FromVal<Env, Val>;

    // Read-only pre-check, no state changes
    fn can_enforce(/* ... */) -> bool;

    // State-changing hook, requires smart account authorization
    fn enforce(/* ... */);

    // Initialize policy-specific storage and configuration
    fn install(/* ... */);

    // Clean up policy data for an account and context rule
    fn uninstall(/* ... */);
}
```

#### Lifecycle

Policies follow a well-defined lifecycle that integrates with context rule management and the authorization matching algorithm.

**Installation** occurs when a new context rule is created with attached policies. The smart account calls `install()` on each policy contract, passing account-specific and context-specific parameters. This initialization step allows policies to configure their logic (for example, a threshold policy might define the required number of signatures for that particular account and context rule, while a spending limit policy might set daily or per-transaction caps). Installation ensures that each policy has the necessary state and configuration ready before authorization checks begin.

**Pre-check validation** happens during authorization. When the matching algorithm iterates over context rules and their associated policies, it calls `can_enforce()` on each policy as a read-only pre-check. This function examines the current state without modifying it, for instance, verifying that a spending limit has not been exceeded or that enough signers are present. Policies that fail this check cause the algorithm to move to the next context rule.

**Enforcement** is triggered when a context rule successfully matches. Once all policies in the matched rule pass their `can_enforce()` checks, the smart account calls `enforce()` on each policy. This state-changing hook allows policies to update counters, emit events, record timestamps, or perform other mutations that track authorization activity. For example, a spending limit policy might deduct from the available balance and emit an event documenting the transaction.

**Uninstallation** occurs when a context rule is removed from the smart account. The account calls `uninstall()` on each attached policy, allowing them to clean up any stored data associated with that specific account and context rule pairing. This ensures that policies do not leave orphaned state in storage.

#### Policy Examples

- Admin Access: Elevated permissions for account management
- Spending Limits: Time-based or token-based restrictions
- Multisig: Threshold-based authorization
- Session Policies: Temporary, scoped permissions
- Recovery Policies: Account recovery mechanisms

#### Caveats

**Signer Set Divergence in Threshold Policies**

Threshold policies (both simple and weighted) store authorization requirements that are validated at installation time. However, policies are not automatically notified when signers are added to or removed from their parent context rule. This creates a state divergence that can lead to operational issues.

If signers are removed after policy installation, the total available signatures or weight may fall below the stored threshold, making it impossible to meet the authorization requirement and permanently blocking actions governed by that policy. For example, a 5-of-5 multisig where two signers are removed leaves only three signers, making the threshold of five unreachable.

Conversely, if signers are added without updating the threshold, the security guarantee silently weakens. A strict 3-of-3 multisig becomes a 3-of-5 multisig after adding two signers, reducing the required approval from 100% to 60% without any explicit warning.

Administrators must manually update thresholds and weights when modifying signer sets. Before removing signers, verify that the threshold remains achievable. After adding signers, adjust thresholds or assign weights to maintain the desired security level. Ideally, these updates should occur in the same transaction as the signer modifications.

**Pre-check Constraints**

The `can_enforce()` function must be idempotent, side-effect free, and efficient. It may read from storage but must not modify it. This constraint exists because the matching algorithm may call `can_enforce()` multiple times during rule evaluation, and failed checks should not leave any persistent changes.

### 6. Execution Entry Point

The `ExecutionEntryPoint` trait enables secure contract-to-contract calls:

```rust
pub trait ExecutionEntryPoint {
    fn execute(e: &Env, target: Address, target_fn: Symbol, target_args: Vec<Val>);
}
```

This trait provides a secure mechanism for updating policy configuration after installation. As noted in the caveats above, administrators must manually adjust thresholds and weights when modifying signer sets. The execution entry point allows the smart account to call policy update functions (such as `set_threshold()` or `set_signer_weight()`) in a controlled manner, ensuring that configuration changes are properly authorized by the account itself. This enables administrators to maintain security invariants when adding or removing signers, ideally bundling signer modifications and threshold adjustments into a single authorized transaction.

## Authorization Flow

Authorization is determined by matching the current call context against the account’s context rules. Rules are gathered, ordered by recency, and evaluated until one satisfies the requirements. If a matching rule is found, its policies (if any) are enforced. Otherwise, authorization fails.

The evaluation proceeds as follows:

### 1. Rule Collection

- Retrieve all non-expired rules for the specific context type
- Include default rules that apply to any context
- Sort by creation time (newest first)

### 2. Rule Evaluation

For each rule in order:

1. Signer Filtering: Extract authenticated signers from the context rule's signer list
2. Policy Validation: If policies exist, verify all can be enforced via `can_enforce()`
3. Authorization Check:
   - With policies: Success if all policies are enforceable
   - Without policies: Success if all signers are authenticated
4. Rule Precedence: First matching rule wins (newest takes precedence)

### 3. Policy Enforcement

- If authorization succeeds, call `enforce()` on all matched policies
- This triggers any necessary state changes (spending tracking, etc.)

### 4. Result

- **Success**: Authorization granted, transaction proceeds
- **Failure**: Authorization denied, transaction reverts

#### Example

Consider a call in the `CallContract(dex_address)` context. The client presents two authorization entries: an ed25519 key and a passkey signature. The smart account maintains two relevant rules:

- A default rule with three ed25519 keys.
- A newer `CallContract(dex_address)` rule that requires both the ed25519 key and the passkey, along with a daily spend policy.

During evaluation, non‑expired rules are gathered and ordered by recency. The specific `CallContract(dex_address)` rule is evaluated first. The account authenticates the ed25519 key and the passkey. Because the rule includes policies, the account verifies that all policies can be enforced. Since checks pass, the policies are enforced (e.g., spending counters are updated) and authorization succeeds. If the specific rule did not match, evaluation would continue to the default rule; if no rule matched, authorization would fail.

## Use Cases

### 1. Session Logins (Web3 dApps)

```rust,ignore
// Create a session policy for a DeFi app
create_context_rule(
    context_type: CallContract(defi_app_address),
    name: "DeFi Session",
    valid_until: Some(current_ledger + 24_hours),
    signers: vec![&e, ed25519_key],
    policies: map![&e, (spending_limit_policy, spending_params)]
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
    signers: vec![&e, ai_agent_key],
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
        &e,
        Signer::External(ed25519_verifier, alice_pubkey),
        Signer::External(secp256k1_verifier, bob_pubkey),
        Signer::Delegated(carol_contract)
    ],
    policies: map![&e, (threshold_policy, two_of_three)]
)
```

## Getting Started

### 1. Installation

Add this to your `Cargo.toml`:

```toml
[dependencies]
# We recommend pinning to a specific version, because rapid iterations are expected as the library is in an active development phase.
stellar-accounts = "=0.5.1"
```

### 2. Implement the Smart Account Trait

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

        add_context_rule(e, &context_type, &name, &valid_until, &signers, &policies)
    }
    // Implement all other methods
}

#[contractimpl]
impl CustomAccountInterface for MySmartAccount {
    type Signature = Signatures;

    fn __check_auth(
        e: Env,
        signature_payload: Hash<32>,
        signatures: Signatures,
        auth_context: Vec<Context>,
    ) -> Result<(), SmartAccountError> {
        do_check_auth(e, signature_payload, signatures, auth_contexts)

        Ok(())
    }
}
```

### 3. Create Context Rules

```rust
// Create an admin rule
add_context_rule(
    &e,
    ContextRuleType::Default,
    String::from_str(&e, "Admin Access"),
    None, // No expiration
    vec![&e, admin_signer],
    map![&e]
);
```

### 4. Add Policies (Optional)

For policies, there are two options:

**Option A: Use Ecosystem Policies (Recommended)**
- Use pre-deployed, audited policy contracts for common use cases that are trusted by the ecosystem and cover standard scenarios
- Examples: Simple threshold, weighted threshold, spending limits, time-based restrictions

**Option B: Create Custom Policies**
- Implement your own policy contracts by implementing the `Policy` trait
- Useful for specialized business logic or unique authorization requirements

```rust
// Add a spending limit policy
add_policy(
    &e,
    admin_rule.id,
    spending_policy_address,
    spending_limit_params
);
```

### 5. Choose or Deploy Verifier Contracts (For External Signers)

For external signers, there are two options:

**Option A: Use Ecosystem Verifiers (Recommended)**

- Use pre-deployed, audited verifier contracts for common signature schemes
- These are trusted by the ecosystem and are immediately available
- Examples: Standard ed25519, secp256k1, secp256r1, bls12-381 verifiers

**Option B: Deploy Custom Verifiers**

- Deploy your own verifier contracts for application-specific requirements
- Useful for custom cryptographic schemes or specialized verification logic
- Requires thorough security auditing and testing

## Caveats

- Multiple context rules for the same context can co‑exist; the most recently added one takes precedence.
- For simple cases like threshold‑based multisig, using a policy may feel verbose compared to embedding logic directly in the account, but keeping business rules in policies preserves separation of concerns and allows for a greater flexibility.
- Authorization composes independent contracts (the smart account, verifiers, and policies). Protocol 23 makes cross‑contract calls cheap, but not free, so the framework sets explicit limits to keep costs predictable:
  - Maximum signers per context rule: 15
  - Maximum policies per context rule: 5
  - Maximum context rules per smart account: 15

## Crate Structure

This crate is organized into three submodules that provide building blocks for implementing smart accounts. These submodules can be used independently or together, allowing developers to implement only the components they need, create custom smart account architectures, mix and match different authentication methods, and build specialized authorization policies.

1. **smart_account**
- context rule management, signer/policy storage functions for implementing the `SmartAccount` trait

2. **verifiers**
- `ed25519` and `webauthn` (passkey authentication) utility functions for implementing the `Verifier` trait

3. **policies**
- `simple_threshold`, `weighted_threshold` and `spending_limit` utility functions for implementing the `Policy` trait

