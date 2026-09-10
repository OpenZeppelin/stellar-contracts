# Interface

Based on [EIP-7984](https://eips.ethereum.org/EIPS/eip-7984), adapted for Soroban. The `data: Bytes` parameter carries XDR-encoded proof payloads.

**Canonical encoding.** The `data` payloads are `#[contracttype]` structs and enums declared in the contract crate. Their on-chain byte representation is fixed by Soroban's XDR rules, which are canonical: every value has exactly one valid byte encoding. Named struct fields are serialised as an `ScMap` keyed by field-name symbols in canonical sorted order, which the host enforces; unnamed fields and tuple-enum variants as an `ScVec` in declaration order. As a consequence, independent implementations that compile against the same `#[contracttype]` definitions produce byte-identical `data` payloads. The authoritative schemas for the underlying `ScVal`, `ScMap`, `ScVec` live in the [stellar/stellar-xdr](https://github.com/stellar/stellar-xdr) repository; the encoding rules are summarised in the [Stellar XDR documentation](https://developers.stellar.org/docs/learn/fundamentals/data-format/xdr) and the [`#[contracttype]` mapping reference](https://developers.stellar.org/docs/learn/fundamentals/contract-development/types/custom-types).

```rust
trait ConfidentialToken {
    fn __constructor(e: Env, admin: Address, token: Address,
                     verifier: Address, auditor: Address);

    fn register(e: Env, account: Address, auditor_id: u32, data: Bytes);

    fn deposit(e: Env, from: Address, to: Address, amount: i128);

    fn merge(e: Env, account: Address);

    fn withdraw(e: Env, from: Address, to: Address, amount: i128, data: Bytes);

    fn confidential_transfer(e: Env, from: Address, to: Address, data: Bytes);

    fn confidential_transfer_from(e: Env, spender: Address,
                                   from: Address, to: Address, data: Bytes);

    fn set_spender(e: Env, account: Address, spender: Address,
                    live_until_ledger: u32, data: Bytes);

    fn revoke_spender(e: Env, account: Address, spender: Address);

    fn confidential_balance(e: Env, account: Address) -> ConfidentialAccount;

    fn is_spender(e: Env, account: Address, spender: Address) -> bool;

    fn get_spender_delegation(e: Env, account: Address, spender: Address) -> SpenderDelegation;
}
```

A deployment that enables the optional compliance extension adds `compliance: Option<ComplianceConfig>` to this constructor and the freeze, configuration-rotation, read, and — when it opts into `ConfidentialClawback` — seizure entry points of [Interface Summary](../compliance.md#interface-summary) to this interface. The admin-gated ones are authorized by the deployment's own access-control module.

**Hooks.** Every entry point invokes the deployment's `Hooks` implementation after authorization and payload decoding and before the storage-layer operation. Hooks for operations that carry `data` receive the decoded payload by reference; the proofless `on_deposit`, `on_merge`, and `on_revoke_spender` receive none. The two seizure entry points, `clawback` and `force_revoke_spender`, invoke no hook, since a gating hook rejects exactly their precondition — a frozen target ([Clawback](../compliance.md#clawback)).

This table is authoritative: every entry is exactly the set of prover-supplied public inputs from the corresponding [Operations](operations/README.md) operation (the contract loads the remaining public inputs from trusted state per [Public Input Sources](operations/README.md#public-input-sources)), plus the `proof` blob. Names map directly to the [Operations](operations/README.md) symbols.

| Operation | `data` contents |
|:---|:---|
| `register` | $$Y$$, $$\text{PVK}$$, `proof` |
| `withdraw` | $$C\_{\text{spend}}'$$, $$\tilde{b}$$, $$R\_e$$, $$\sigma$$, $$\tilde{b}\_{\text{aud,s}}$$, $$\tilde{r}\_{\text{aud,s}}$$, `proof` |
| `confidential_transfer` | $$C\_{\text{spend}}'$$, $$C\_{\text{transfer}}$$, $$R\_e$$, $$\tilde{v}$$, $$\tilde{b}$$, $$\sigma$$, $$\tilde{v}\_{\text{aud,r}}$$, $$\tilde{r}\_{\text{aud,r}}$$, $$\tilde{v}\_{\text{aud,s}}$$, $$\tilde{b}\_{\text{aud,s}}$$, $$\tilde{r}\_{\text{aud,s}}$$, `proof` |
| `confidential_transfer_from` | $$C\_a'$$, $$C\_{\text{transfer}}$$, $$R\_e$$, $$\tilde{v}$$, $$\tilde{a}'$$, $$\sigma\_a'$$, $$\tilde{v}\_{\text{aud,r}}$$, $$\tilde{r}\_{\text{aud,r}}$$, $$\tilde{v}\_{\text{aud,s}}$$, $$\tilde{a}\_{\text{aud,s}}$$, $$\tilde{r}\_{\text{aud,s}}$$, `proof` |
| `set_spender` | $$C\_{\text{spend}}'$$, $$C\_a$$, $$\text{escrowed\\\_dvk}$$, $$\tilde{b}$$, $$\tilde{a}$$, $$R\_e$$, $$\sigma$$, $$\sigma\_a$$, $$\tilde{v}\_{\text{aud,s}}$$, $$\tilde{b}\_{\text{aud,s}}$$, $$\tilde{r}\_{\text{aud,s}}$$, $$\tilde{r}\_{a,\text{aud,s}}$$, `proof` |

For `confidential_transfer_from`, the stored allowance salt $$\sigma\_a$$ is **not** carried in `data`: the contract loads it from the `(from, spender)` delegation entry ([Spender Transfer](operations/spender-transfer.md) public-input table). Only the prover-chosen replacement $$\sigma\_a'$$ travels in `data`. It is bound as this transfer's channel nonce (O7, O9, O\_a2, O\_a6), as the new allowance state's salt (O10, O12), and against the salt it replaces (O14); it is then written back to the delegation entry as the new `allowance_salt` and emitted in the event ([Transfer nonce](account-state.md#transfer-nonce)). This keeps the trust-boundary rule of [Public Input Sources](operations/README.md#public-input-sources) intact: caller-controlled bytes never overwrite the live $$\sigma\_a$$ used to verify the proof. `set_spender`, by contrast, has no prior delegation entry to load from, so its $$\sigma\_a$$ is prover-supplied and bound by S6.

## Authorization Model

Soroban `address.require_auth()` proves that the named principal authorized the current invocation; it binds the full invocation (function name and all arguments) by default. ZK proof verification proves that the prover knows a witness satisfying the circuit's constraints over public inputs the contract itself supplies. The two are complementary: every state-changing operation requires **both** the appropriate `require_auth()` and (where applicable) a valid proof.

| Operation | `require_auth()` principal |
|:---|:---|
| `register(account, auditor_id, data)` | `account` |
| `deposit(from, to, amount)` | `from` |
| `merge(account)` | `account` |
| `withdraw(from, to, amount, data)` | `from` |
| `confidential_transfer(from, to, data)` | `from` |
| `confidential_transfer_from(spender, from, to, data)` | `spender` (not `from`) |
| `set_spender(account, spender, live_until_ledger, data)` | `account` |
| `revoke_spender(account, spender)` | `account` |
| `confidential_balance`, `is_spender`, `get_spender_delegation` | none (read-only) |

**`register` is single-use.** It reverts if `account` is already registered. Combined with `account.require_auth()`, this prevents a third party from binding attacker-controlled $$(Y, \text{PVK})$$ to `account`'s `ConfidentialAccount` storage entry.

**`set_spender` rejects replacement.** It reverts if a non-revoked delegation already exists for `(account, spender)` -- see [Spender Delegation](account-state.md#spender-delegation).

**`confidential_transfer_from` is spender-authorized.** The owner's authorization was granted out-of-band at `set_spender` and persists in the on-chain delegation entry until expiry or revocation. The spender's `require_auth()` binds `from`, `to`, and `data`.

## Event Schema

Each state-modifying operation emits a structured event. Events carry the data needed for recipient decryption, auditor decryption, and wallet recovery.

| Event | Fields |
|:---|:---|
| `Register` | `account`, `auditor_id` |
| `Deposit` | `from`, `to`, `amount` |
| `Merge` | `account` |
| `Withdraw` | `from`, `to`, `amount`, $$R\_e$$, $$\sigma$$, $$\tilde{b}$$, $$\tilde{b}\_{\text{aud,s}}$$, $$\tilde{r}\_{\text{aud,s}}$$ |
| `Transfer` | `from`, `to`, $$R\_e$$, $$\tilde{v}$$, $$\sigma$$, $$\tilde{b}$$, $$\tilde{v}\_{\text{aud,r}}$$, $$\tilde{r}\_{\text{aud,r}}$$, $$\tilde{v}\_{\text{aud,s}}$$, $$\tilde{b}\_{\text{aud,s}}$$, $$\tilde{r}\_{\text{aud,s}}$$ |
| `SpenderTransfer` | `spender`, `from`, `to`, $$R\_e$$, $$\tilde{v}$$, $$\sigma\_a'$$, $$\tilde{v}\_{\text{aud,r}}$$, $$\tilde{r}\_{\text{aud,r}}$$, $$\tilde{v}\_{\text{aud,s}}$$, $$\tilde{a}\_{\text{aud,s}}$$, $$\tilde{r}\_{\text{aud,s}}$$ |
| `SetSpender` | `account`, `spender`, `live_until_ledger`, $$R\_e$$, $$\sigma$$, $$\tilde{b}$$, $$\tilde{v}\_{\text{aud,s}}$$, $$\tilde{b}\_{\text{aud,s}}$$, $$\tilde{r}\_{\text{aud,s}}$$, $$\tilde{r}\_{a,\text{aud,s}}$$ |
| `RevokeSpender` | `account`, `spender`, $$\tilde{a}$$, $$\sigma\_a$$ |
| `Clawback` | `account`, `amount`, `destination` |

Amount fields in `Deposit`, `Withdraw`, and `Clawback` are typed `i128`, matching SEP-41.

`Clawback` is emitted only by deployments that enable the optional compliance extension ([Clawback](../compliance.md#clawback)).

**Usage by consumers:**

- **Recipient wallet**: processes `Transfer` and `SpenderTransfer` events using $$(R\_e, \tilde{v}, \sigma)$$ -- with $$\sigma\_a'$$ in place of $$\sigma$$ for `SpenderTransfer` -- to derive $$v\_{\text{transfer}}$$ and $$r\_{\text{transfer}}$$ ([ECDH-Derived Blinding](keys-and-commitments.md#ecdh-derived-blinding)).
- **Owner wallet**: processes all events for recovery ([Wallet State and Recovery](wallet-state.md)). The $$(\tilde{b}, \sigma)$$ pair from the most recent checkpoint event (`Withdraw`, `Transfer` as sender, `SetSpender`) anchors $$W\_{\text{spend}}$$; `Merge`, `RevokeSpender`, and `Clawback` are folded into it during replay.
- **Auditor**: processes events containing $$R\_e$$ to compute ECDH shared secrets and decrypt amounts, balance checkpoints, and the escrowed blindings ([Per-Transfer Auditor Ciphertexts](auditing.md#per-transfer-auditor-ciphertexts), [Auditor Visibility Properties](auditing.md#auditor-visibility-properties), [Spender Allowance Auditing](auditing.md#spender-allowance-auditing)); folds `Merge`, `RevokeSpender`, and `Clawback` into its accumulators as [Per-Transfer Auditor Ciphertexts](auditing.md#per-transfer-auditor-ciphertexts) and [Wallet and Auditor Consequences](../compliance.md#wallet-and-auditor-consequences) specify.
- **Indexer**: reconciles the pooled balance against the sum of claims ([Balance Conservation](security.md#balance-conservation), [Indexing and Off-Chain State Recovery](../indexer.md)).

## Read Methods

**`confidential_balance(account) -> ConfidentialAccount`.** Returns the `ConfidentialAccount` struct for the given account ([Account Data Model](account-state.md#account-data-model)), i.e. the tuple `(spending_public_key, viewing_public_key, spendable_commitment, receiving_commitment, auditor_id)`. Reverts if `account` is not registered. Wallets bootstrap from this call (single round-trip to obtain both Pedersen commitments plus the keys needed to identify the account and its bound auditor); indexers use it to verify consistency between their replayed accumulators and on-chain state ([Consistency check](wallet-state.md#consistency-check)).

**`is_spender(account, spender) -> bool`.** Returns `true` iff a delegation entry exists for `(account, spender)` **and** `ledger.sequence() <= live_until_ledger`. Returns `false` for:

- pairs with no delegation entry,
- pairs whose entry has `ledger.sequence() > live_until_ledger` (expired-but-not-yet-revoked: the spender can no longer spend, while the escrowed value stays on-chain until `revoke_spender` reclaims it, [Spender Delegation](account-state.md#spender-delegation)),
- pairs whose entry was revoked (deleted) by `revoke_spender`.

The function returns the *spending-authority* state, not the *escrow-existence* state. Consumers that need to distinguish "no delegation" from "expired delegation" inspect `get_spender_delegation` (below) or replay `SetSpender` / `RevokeSpender` events.

**`get_spender_delegation(account, spender) -> SpenderDelegation`.** Returns the `SpenderDelegation` struct ([Spender Delegation](account-state.md#spender-delegation)) for the `(account, spender)` pair, i.e. `(allowance_commitment, a_tilde, escrowed_dvk, allowance_salt, live_until_ledger)`. Reverts if no delegation entry exists for the pair. Unlike `is_spender`, this surfaces the raw on-chain delegation state without applying the expiry filter, so callers can separate an absent delegation (revert) from an active one and from an expired-but-not-yet-revoked one, by comparing `live_until_ledger` against `ledger.sequence()` as `is_spender` does. Primary consumers:

- **Spender wallet:** fetches `allowance_commitment`, `a_tilde`, `escrowed_dvk`, and `allowance_salt` to recover $$dvk\_i$$ via [Delegation Key Escrow](operations/set-spender.md#delegation-key-escrow) decryption, then reads the current allowance via $$\tilde{a} = v\_a + \text{Poseidon}(\delta\_{\text{enc\\\_allow}}, dvk\_i, \sigma\_a)$$ to construct the next `confidential_transfer_from` witness.
- **Owner wallet:** reads the same fields after losing local state, or before calling `revoke_spender`, to confirm the on-chain entry matches its records.
- **Indexers:** verify their replayed delegation state against the live commitment, in the same way `confidential_balance` is used for account state ([Consistency check](wallet-state.md#consistency-check)).

The auditor's allowance tracking does **not** use this method: per-event allowance ciphertexts ([Spender Allowance Auditing](auditing.md#spender-allowance-auditing)) are the auditor's data path. `a_tilde` is keyed to $$dvk\_i$$ and is unreadable without it, the owner's auditor included ([Spender Allowance Auditing](auditing.md#spender-allowance-auditing)).

---

Previous: [Proof System](proof-system.md) · Up: [Index](../README.md#protocol) · Next: [Domain Separation Constants](domain-separators.md)
