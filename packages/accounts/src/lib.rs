//! # Soroban Smart Accounts
//!
//! A flexible and modular smart account framework for Soroban that enables
//! advanced authentication and authorization patterns through composable rules,
//! signers, and policies.
//!
//! ## Overview
//!
//! This crate provides a context-centric smart account implementation that
//! manages the composition of authorization intents from various sources. It
//! separates the "who" (signers), "what" (rules for specific actions), and
//! "how" (pluggable policies) of authorization, enabling flexible combinations
//! of authentication methods.
//!
//! Smart accounts can combine multiple authorization mechanisms seamlessly. For
//! example, a wallet might require both a session policy and a passkey with a
//! 24-hour lifespan, treating them as a single composite "key" for signing
//! actions.
//!
//! ## Core Components
//!
//! ### Context Rules
//!
//! Context Rules define the scope of authorization and compose signers with
//! policies:
//!
//! - Set authorization requirements for specific context types (contract calls,
//!   deployments)
//! - Must contain at least one signer or one policy
//! - Support optional expiration via ledger sequence (`valid_until`)
//! - Allow multiple rules for the same context with different configurations
//! - Evaluated in reverse chronological order (most recent first)
//!
//! Supported context types:
//! - `Default`: Applies to any context
//! - `CallContract(Address)`: Specific contract calls
//! - `CreateContract(BytesN<32>)`: Contract deployments
//!
//! ### Signers
//!
//! Two types of signers provide authentication:
//!
//! #### Native Signers
//! - Any Soroban `Address` that can be authorized
//! - Verified within the smart account using `require_auth_for_args()`
//! - Requires manual construction of authorization entries during simulation
//!
//! #### Delegated Signers
//! - Tuple of `(Address, Bytes)`: verifier contract + public key
//! - Offloads signature verification to trusted, immutable verifier contracts
//! - Supports multiple cryptographic schemes without modifying the account
//! - Enables future authentication methods (zk-based, email signing)
//!
//! ### Policies
//!
//! Policies customize and enforce signer behavior within context rules:
//!
//! - External contracts with standardized interfaces
//! - Logic owned solely by the smart account
//! - Support both per-account and shared configurations
//! - All policies in a rule must be enforceable for authorization
//!
//! Policy interface:
//! - `can_enforce()`: Pre-check without state changes
//! - `enforce()`: Execute with potential state changes (account-only
//!   invocation)
//! - `install()`: Store custom configuration for account/context
//! - `uninstall()`: Clean up stored configuration
//!
//! ## Authorization Flow
//!
//! The matching algorithm for authorization:
//!
//! 1. Retrieve all non-expired rules for the context type, plus defaults
//! 2. For each rule (newest first):
//!    - Filter authenticated signers from rule's signer set
//!    - If policies exist: verify all are enforceable
//!    - If no policies: require all signers authenticated
//!    - First matching rule succeeds
//! 3. If no rule matches, authorization fails
//! 4. Enforce all matched policies to trigger state changes
//!
//! ## Use Cases
//!
//! ### Session Logins (Web3 dApps)
//! Users grant temporary, scoped permissions to applications instead of
//! approving each action individually. Similar to OAuth tokens with limited
//! capabilities.
//!
//! ### Backend Automation
//! Enable safe automation for subscription payments and recurring tasks with
//! stateful policies enforcing strict parameters (amount limits, frequency).
//!
//! ### AI Agents
//! Grant controlled wallet access to AI contracts through policies that
//! strictly limit funds, functions, and conditions for autonomous operations.
//!
//! ### Multiparty Multisig
//! Support complex transactions with multiple signers of different types,
//! combining traditional cryptographic keys with policy-based programmable
//! signers.
//!
//! ## Example Configurations
//!
//! ```ignore
//! // Admin rule: 2 specific signers for all contract calls
//! Context Rule {
//!     name: "Admin",
//!     context_type: Default,
//!     signers: [delegated_signer_1, delegated_signer_2],
//!     policies: None,
//! }
//!
//! // Regular operations: 3-of-5 threshold with spending limits
//! Context Rule {
//!     name: "Regular",
//!     context_type: CallContract(token_address),
//!     signers: [signer_1, signer_2, signer_3, signer_4, signer_5],
//!     policies: [threshold_policy, spending_limit_policy],
//!     valid_until: Some(ledger_seq + 30_days),
//! }
//!
//! // Emergency: Single signer with additional constraints
//! Context Rule {
//!     name: "Emergency",
//!     context_type: CallContract(specific_contract),
//!     signers: [emergency_signer],
//!     policies: [emergency_policy],
//! }
//! ```
//!
//! ## Design Principles
//!
//! - **Context-centric**: Authorization rules are scoped to specific contexts
//! - **Composable**: Mix and match signers, policies, and rules
//! - **Extensible**: Add new signature schemes without account upgrades
//!
//! ## Implementation Notes
//!
//! This implementation takes advantage of Protocol 23 changes that make storage
//! reads marginal in cost and significantly reduce cross-contract call
//! expenses. The design prioritizes flexibility and security while maintaining
//! gas efficiency through careful state management and delegation to
//! specialized verifier contracts.
//!
//! ### Limits
//! - Maximum of 15 signers per context rule
//! - Maximum of 5 policies per context rule
#![no_std]

pub mod policies;
pub mod smart_account;
pub mod verifiers;
