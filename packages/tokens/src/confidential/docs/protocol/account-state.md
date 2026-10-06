# Account State

## Account Data Model

Each registered account stores a `ConfidentialAccount` in persistent storage, keyed by `Address`:

```rust
ConfidentialAccount {
    spending_public_key:    BytesN<64>,   // Y = sk · H
    viewing_public_key:     BytesN<64>,   // PVK = vk · H
    spendable_commitment:   BytesN<64>,   // C_spend: single Pedersen commitment
    receiving_commitment:   BytesN<64>,   // C_receive: single Pedersen commitment
    auditor_id:             u32,
}
```

### `spending_public_key`

$$Y = sk \cdot H$$. Set once at registration. Authorizes all spending operations.

### `viewing_public_key`

$$\text{PVK} = vk \cdot H$$. Set once at registration. Used by senders for ECDH key agreement. The registration proof enforces derivation from the same $$sk$$ as $$Y$$.

### `spendable_commitment`

The commitment the owner can spend from. Modified by owner-authorized operations and, in a deployment that enables the optional compliance extension, by its clawback ([Contract Flow](../compliance.md#contract-flow)). Encoded as a single Grumpkin affine point (64 bytes).

### `receiving_commitment`

Accumulates incoming deposits and transfers via homomorphic addition. The contract adds to this without any proof from the recipient. Reset to $$\mathcal{O}$$ on merge ([Merge](operations/merge.md)) and on clawback ([Contract Flow](../compliance.md#contract-flow)). Encoded as a single Grumpkin affine point (64 bytes).

### `auditor_id`

Index into the auditor contract's key store. Set once at registration. Used by the contract to fetch the correct auditor public key when building transfer public inputs. For incoming transfers, the recipient's `auditor_id` determines the key under which the transfer amount is encrypted. For outgoing transfers (and spender transfers), the sender's (or owner's) `auditor_id` determines the key under which the transfer amount and post-transfer balance (or allowance) are encrypted.

## Spender Delegation

Spender delegations are stored in persistent storage, keyed by `(owner, spender)`:

```rust
SpenderDelegation {
    allowance_commitment:   BytesN<64>,   // Single Pedersen commitment
    a_tilde:                BytesN<32>,   // Poseidon-encrypted allowance scalar
    escrowed_dvk:           BytesN<64>,   // ECDH escrow of dvk_i under spender key
    allowance_salt:         BytesN<32>,
    live_until_ledger:      u32,
}
```

### `allowance_commitment`

The spender's remaining escrowed allowance, a single Pedersen commitment: $$C\_a = \text{Com}(v\_a, r\_a)$$ where $$r\_a = \text{Poseidon}(\delta\_{\text{allow\\\_r}}, dvk\_i, \sigma\_a)$$. One Grumpkin point (64 bytes).

### `a_tilde`

Poseidon-encrypted allowance scalar: $$\tilde{a} = v\_a + \text{Poseidon}(\delta\_{\text{enc\\\_allow}}, dvk\_i, \sigma\_a)$$. Enables the spender (who holds $$dvk\_i$$ via `escrowed_dvk`) to read the current allowance without DLP when constructing an `SpenderTransfer` witness. The owner can also read it via $$vk \rightarrow dvk\_i$$. It is emitted alongside `allowance_salt` in the `RevokeSpender` event ([Revoke Spender](operations/revoke-spender.md)).

### `escrowed_dvk`

$$dvk\_i$$ encrypted under the spender's spending key via ECDH. (64 bytes)

### `allowance_salt`

Per-delegation salt for allowance randomness derivation, encoded as `BytesN<32>` (canonical $$\mathbb{F}\_r$$ representative). $$\sigma\_a$$ is sampled by the rejection sampling procedure of [Grumpkin-BN254 Cycle](primitives.md#grumpkin-bn254-cycle) (same as $$\sigma$$), and it opens the current allowance: the stored `allowance_commitment` and `a_tilde` are both derived under it. It is not the freshness input for a spender transfer's pads -- its replacement is (*Transfer nonce* below). Set by the owner at `set_spender` and replaced by the spender on every `confidential_transfer_from`. The salt is bound to the current commitment: when the commitment changes, the salt changes with it. It is stored on-chain so the owner can decrypt the allowance at revocation without depending on event history, and it is emitted in the `RevokeSpender` event ([Revoke Spender](operations/revoke-spender.md)).

### Transfer nonce

Unlike $$\sigma$$, which an owner samples afresh for every operation, $$\sigma\_a$$ is stored state: a reverted transfer leaves the delegation entry untouched, so the retry is forced to reuse it. Pads keyed to it would repeat $$r\_e$$ and every mask across the two attempts, and a retry that changed the amount would publish the difference in the clear ([Poseidon2 Hash](primitives.md#poseidon2-hash)).

A spender transfer avoids this by splitting the salt's two jobs: opening the stored allowance must be deterministic, since the value has to reproduce the randomness the commitment on-chain was built under, while keying pads must be fresh on every attempt. The stored $$\sigma\_a$$ does only the first (O3); every pad the transfer derives, and its ephemeral scalar, absorbs a prover-chosen replacement $$\sigma\_a'$$ instead ([Spender Transfer](operations/spender-transfer.md)).

### `live_until_ledger`

The ledger number at which the delegation expires. The delegation is live while `ledger.sequence() <= live_until_ledger` and expired once `ledger.sequence() > live_until_ledger`. Checked on every `confidential_transfer_from`. The entry lives in persistent, not temporary, storage: automatic cleanup would destroy escrowed funds.

The `(owner, spender)` storage entry holds at most one delegation. `set_spender` ([Set Spender](operations/set-spender.md)) reverts if a delegation already exists for that pair, regardless of whether the existing delegation is past `live_until_ledger`. Expiry only prevents the spender from spending; the escrowed value persists on-chain until `revoke_spender` ([Revoke Spender](operations/revoke-spender.md)) folds it back into the owner's spendable balance. Re-delegating to the same spender therefore requires the sequence: `revoke_spender` then `set_spender`. This rule is what keeps the balance-conservation invariant ([Balance Conservation](security.md#balance-conservation)) well-defined over stored delegations: every delegation is either active, expired-pending-revoke, or absent, and the escrowed value is never silently dropped.

---

Previous: [Wallet State and Recovery](wallet-state.md) · Up: [Index](../README.md#protocol) · Next: [Operations](operations/README.md)
