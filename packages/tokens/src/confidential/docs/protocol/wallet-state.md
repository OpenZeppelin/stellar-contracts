# Wallet State and Recovery

A Pedersen commitment $$C = \text{Com}(v, r)$$ hides its opening $$(v, r)$$. The owner must know this opening to construct spend proofs so they must maintain $$(v, r)$$ as local wallet state, updated incrementally as balance-modifying events occur.

**Definition** (Wallet state). The owner's wallet maintains two running accumulators:

$$W_{\text{spend}} = (v_s, r_s) \quad \text{such that} \quad C_{\text{spend}} = v_s \cdot G + r_s \cdot H$$
$$W_{\text{receive}} = (v_r, r_r) \quad \text{such that} \quad C_{\text{receive}} = v_r \cdot G + r_r \cdot H$$

**Initialization.** At registration, $$C_{\text{spend}} = C_{\text{receive}} = \mathcal{O}$$. The wallet sets $$W_{\text{spend}} = W_{\text{receive}} = (0, 0)$$.

## Update rules

Each balance-modifying event updates the accumulators as follows:

| Event | Accumulator update |
|:---|:---|
| Deposit of public amount $$a$$ to this account | $$W_{\text{receive}} \mathrel{+}= (a, 0)$$ |
| Incoming transfer with event $$(R_e, \tilde{v}, \sigma)$$ | Compute $$s = \text{ECDH}(vk, R_e)$$ ([Elliptic Curve Diffie-Hellman](primitives.md#elliptic-curve-diffie-hellman)); derive $$v_{\text{transfer}} = \tilde{v} - \text{Poseidon}(\delta_{\text{transfer\\\_amount}}, s, \sigma)$$ and $$r_{\text{transfer}} = \text{Poseidon}(\delta_{\text{transfer\\\_blind}}, s, \sigma)$$. Then $$W_{\text{receive}} \mathrel{+}= (v_{\text{transfer}}, r_{\text{transfer}})$$ |
| Outgoing transfer/withdrawal of amount $$a$$ | Proof outputs new commitment with deterministic randomness. $$W_{\text{spend}} \leftarrow (v_s - a, \\; \text{Poseidon}(\delta_{\text{spend\\\_r}}, vk, \sigma))$$ |
| Merge | $$W_{\text{spend}} \leftarrow (v_s + v_r, \\; r_s + r_r)$$; $$W_{\text{receive}} \leftarrow (0, 0)$$ |
| Set spender (escrow amount $$a$$) | Proof outputs new commitment. $$W_{\text{spend}} \leftarrow (v_s - a, \\; \text{Poseidon}(\delta_{\text{spend\\\_r}}, vk, \sigma))$$ |
| Revoke spender | Proofless fold. $$W_{\text{spend}} \mathrel{+}= (v_a, r_a)$$, the escrow opening recovered from the event per [Revoke Spender](operations/revoke-spender.md) |
| Clawback (compliance extension) | The Merge row, then the event's public `amount` is subtracted from the spendable value ([Contract Flow](../compliance.md#contract-flow)) |

The Merge, Revoke spender, and Clawback rows fold openings the wallet already holds, leaving $$W_{\text{spend}}.r$$ a sum; [Post-merge witness availability](proof-system.md#post-merge-witness-availability) covers the case where no circuit can witness it.

After every owner-initiated operation that produces a proof, $$r_s$$ resets to a deterministic value. This is the **normalization** property: the spendable balance's blinding factor is always recoverable from $$(vk, \sigma)$$ at spend boundaries. Together with $$\tilde{b}$$, both emitted in the spend-boundary event, each spend boundary forms a **checkpoint** from which the spendable opening $$(v_s, r_s)$$ *as of that boundary* is recoverable via a single event lookup, with no replay of the spend history preceding it. $$W_{\text{receive}}$$ has no such anchor: a checkpoint leaves $$C_{\text{receive}}$$ untouched, which only `Merge` ([Merge](operations/merge.md)) and `Clawback` reset, so recovering it requires a replay (see Recovery below).

## Consistency check

At any time, the wallet can verify its state: $$C_{\text{spend}} \stackrel{?}{=} v_s \cdot G + r_s \cdot H$$ and $$C_{\text{receive}} \stackrel{?}{=} v_r \cdot G + r_r \cdot H$$, where $$C_{\text{spend}}$$ and $$C_{\text{receive}}$$ are read from on-chain state.

## Recovery

If the wallet loses local state, it recovers from **two anchors**: the account's last **checkpoint** for the spendable side (steps 1-4), and $$T_0$$ -- its most recent `Merge` or `Clawback` at or before that checkpoint -- for the receiving side (step 5). Since $$W_{\text{receive}}$$ restarts at $$(0, 0)$$ as of $$T_0$$, one replay window $$(T_0, \text{now}]$$ serves both: it rebuilds the receiving side and carries the post-checkpoint proofless folds onto the spendable one (step 6). [Why the two anchors compose](#why-the-two-anchors-compose) gives the argument that they do:

1. Fetch $$(\tilde{b}, \sigma)$$ from the most recent **checkpoint event** for this account: exactly one of `Withdraw`, `Transfer` (where the account is the `from`), or `SetSpender` -- the three event types carrying a proof-bound $$(\tilde{b}, \sigma)$$ for this account's spendable balance. `Merge`, `RevokeSpender`, and `Clawback` do change $$C_{\text{spend}}$$ but are not checkpoints: none binds a $$\tilde{b}$$ to the resulting commitment ([Encrypted balance](operations/merge.md#encrypted-balance), [Encrypted balance](operations/revoke-spender.md#encrypted-balance), [Wallet and Auditor Consequences](../compliance.md#wallet-and-auditor-consequences)), and their effect is absorbed into the next checkpoint. **No-checkpoint case:** if the account has no checkpoint event since `Register`, initialize $$W_{\text{spend}} \leftarrow (0, 0)$$ and skip to step 5 with $$T_0$$ = the `Register` event.
2. Recover the spendable balance value: $$v_s = \tilde{b} - \text{Poseidon}(\delta_{\text{enc\\\_bal}}, vk, \sigma)$$.
3. Recover the spendable balance blinding: $$r_s = \text{Poseidon}(\delta_{\text{spend\\\_r}}, vk, \sigma)$$.
4. Set $$W_{\text{spend}} \leftarrow (v_s, r_s)$$.
5. Locate $$T_0$$: the account's most recent `Merge` or `Clawback` event at or before the checkpoint of step 1, or its `Register` event if neither exists. Set $$W_{\text{receive}} \leftarrow (0, 0)$$ as of $$T_0$$.
6. Replay every event after $$T_0$$ in ledger order, applying the *Update rules* above with three replay-specific amendments:
   - **Checkpoint event**: skip the spendable side -- step 1 already captured it.
   - **Revoke spender**: skip if at or before the checkpoint of step 1, which absorbed it; otherwise fold as the table specifies.
   - **Self-transfer** (a `Transfer` whose `from` and `to` are both this account): simultaneously a checkpoint and an incoming transfer -- apply the incoming-transfer rule to $$W_{\text{receive}}$$ as for any other.
7. Verify consistency: $$C_{\text{spend}} \stackrel{?}{=} W_{\text{spend}}.v \cdot G + W_{\text{spend}}.r \cdot H$$ and $$C_{\text{receive}} \stackrel{?}{=} W_{\text{receive}}.v \cdot G + W_{\text{receive}}.r \cdot H$$.

The window's length is set by how recently the account merged before its latest checkpoint, not by how often it spends; the wallet merge policy that keeps it short in practice is [Merge policy](../sdk/wallet.md#merge-policy)'s. In the worst case (funds received but never merged), it extends back to registration.

## Event durability requirement

Recovery depends on the wallet being able to locate $$T_0$$ and retrieve every event after it, the checkpoint event included, in ledger order. Stellar RPC retains event history for a 7-days window only, so a wallet that loses local state after that window cannot recover from RPC alone. The protocol therefore assumes a durable event archive retaining, forever, the full per-account history of every event these rules read. That event set, along with the data model, ingestion contract, retention obligations, and recommended API surface for the archive, is specified in [Indexing and Off-Chain State Recovery](../indexer.md) ([Threat Model](system-model.md#threat-model) for the set itself). Wallets and SDKs MUST consume an indexer that meets that contract for recovery.

---

## Recovery Properties

The recovery procedure, the definition of the **checkpoint** it is built around, the replay window it runs over, and the durable-archive requirement it depends on are all specified in [Recovery](#recovery). This section states the security properties that follow from it.

### Why the two anchors compose

No `Merge` or `Clawback` falls strictly between $$T_0$$ and the checkpoint, by construction of $$T_0$$ as the last of either at or before it. Within $$(T_0, \text{checkpoint}]$$ the only spendable-affecting events are therefore checkpoint events and `RevokeSpender` folds, and the latest checkpoint's $$(\tilde{b}, \sigma)$$ supersedes both — which is why the replay skips them ([Recovery](#recovery), step 6). Any `Merge` the replay encounters necessarily sits after the checkpoint and is folded normally, against the receiving opening it actually consumed.

### Recoverability

An owner who holds $$vk$$ reconstructs both openings deterministically from the account's event history, so the loss of local wallet state is not a loss of funds. The single-lookup shortcut on the spendable side is sound: each checkpoint's $$\tilde{b}$$ is bound to the $$C_{\text{spend}}$$ it certifies by the emitting operation's own proof (W7, T12, S11), so a checkpoint cannot misreport the balance the wallet restarts from. Beyond the event history and the on-chain commitments, recovery requires nothing from the auditor, from counterparties, or from the contract admin.

### Integrity fails closed

The event archive is trusted for availability, not for integrity. Recovery terminates in a consistency check against the on-chain commitments ([Recovery](#recovery), step 7) and Pedersen commitments are binding ([Pedersen Commitments](primitives.md#pedersen-commitments)), so a tampered or truncated history cannot yield an opening that verifies against a balance the account does not hold — it yields a detectable mismatch. The residual risk is therefore denial of recovery, a liveness failure, rather than silent corruption of the reconstructed balance. [Trust Model and Client-Side Verification](../indexer.md#trust-model-and-client-side-verification) specifies the archive's trust model and the client-side checks that follow from it.

### Incoming-transfer spam

A third party can spam an account with confidential transfers (including zero-value transfers, see [Griefing Resistance](security.md#griefing-resistance) Corollary) without invalidating the recipient's spend proofs. The cost to the spammer is the Soroban transaction fee per transfer, which bounds the rate. The cost to the recipient is per-event indexer storage and wallet replay work. Both costs are linear in the number of incoming transfers and bounded by the replay window; neither breaks correctness.

---

Previous: [Key Hierarchy and Commitment Scheme](keys-and-commitments.md) · Up: [Index](../README.md#protocol) · Next: [Account State](account-state.md)
