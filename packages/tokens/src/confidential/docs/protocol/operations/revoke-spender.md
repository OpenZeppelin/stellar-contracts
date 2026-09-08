# Revoke Spender

The owner reclaims the remaining escrowed allowance. Like merge ([Merge](merge.md)), revocation is a homomorphic fold and carries no proof: nothing is re-randomized and no private value is asserted. The escrowed value was range-proven at S4 and re-bounded by O4 on every spender transfer, and the next spend re-bounds the result under W4 / T4 -- the posture [Integer Embedding and Range Proofs](../primitives.md#integer-embedding-and-range-proofs) already accepts for the receiving balance.

## Contract logic (no proof)

```
require account.require_auth()
C_spend ← C_spend + C_a
delete Delegation(account, spender)
emit RevokeSpender(account, spender, a_tilde, allowance_salt)
```

The fold works for both active and expired-but-not-revoked delegations ([Spender Delegation](../account-state.md#spender-delegation)).

**Owner state update.** By Proposition 1 applied to the allowance commitment, the post-revoke opening is $$(v\_s + v\_a, \\; r\_s + r\_a)$$. The owner derives $$dvk\_i$$ from $$vk$$ ([Delegation Viewing Key](../keys-and-commitments.md#delegation-viewing-key)), recovers $$v\_a = \tilde{a} - \text{Poseidon}(\delta\_{\text{enc\\\_allow}}, dvk\_i, \sigma\_a)$$ and $$r\_a = \text{Poseidon}(\delta\_{\text{allow\\\_r}}, dvk\_i, \sigma\_a)$$ from the event, and applies $$W\_{\text{spend}} \mathrel{+}= (v\_a, r\_a)$$ ([Wallet State and Recovery](../wallet-state.md)).

**Why the event carries $$\tilde{a}$$ and $$\sigma\_a$$.** The fold deletes the delegation entry in the same invocation, so neither field is readable from storage afterwards. `SpenderTransfer` writes its $$\tilde{a}'$$ to storage only. $$\sigma\_a$$ reaches events only through `SpenderTransfer`, which emits the replacement $$\sigma\_a'$$ it writes and not the salt it consumed ([Transfer nonce](../account-state.md#transfer-nonce)): the salt this fold consumes is therefore the `sigma_a_new` of the most recent `SpenderTransfer`, recoverable only by scanning back to it, and for a delegation revoked without ever being spent from it is published nowhere, since `SetSpender` emits the owner's spendable salt $$\sigma$$ and no allowance salt.

## Encrypted balance

Revocation emits no $$\tilde{b}$$ and is not a checkpoint ([Wallet State and Recovery](../wallet-state.md)); the next owner-initiated proof operation issues a fresh checkpoint. The auditor carries its opening of $$C\_{\text{spend}}$$ through the fold as [Per-Transfer Auditor Ciphertexts](../auditing.md#per-transfer-auditor-ciphertexts) specifies, using the opening of $$C\_a$$ it holds from the delegation's last state change ([Spender Allowance Auditing](../auditing.md#spender-allowance-auditing)).

---

Previous: [Spender Transfer](spender-transfer.md) · Up: [Operations](README.md) · Next: [Auditing](../auditing.md)
