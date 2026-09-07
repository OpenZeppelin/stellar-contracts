# Holder Wallet

## State model

The wallet maintains the two accumulators of [Wallet State and Recovery](../protocol/wallet-state.md) — $$W_{\text{spend}}$$ and $$W_{\text{receive}}$$ — plus a sync position and any in-flight projection ([In-flight operations](#in-flight-operations)). Values and blindings alike accumulate as exact integers; a blinding is reduced modulo $$q$$ only where it leaves an accumulator for the curve ([Blinding accumulation](crypto-core.md#blinding-accumulation)).

Persistence MUST be pluggable, since the same core serves environments with very different storage. With RPC-only event access, discarding persisted state loses the receiving-side openings permanently ([Recovery](#recovery)), so it MUST NOT be treated as an evictable cache.

## Event application

Event application MUST be ordered, deduplicated, and idempotent in combination, and an implementation MUST state which layer discharges each obligation. The three are commonly split, with ordering and dedup at the event source and application left non-idempotent, which is conformant only if the split is explicit.

- **Ordering.** Events MUST be applied in emission order, because reconstruction is order-sensitive: a merge and a deposit in the same ledger produce different state depending on which is applied first. The canonical total order is [Ordering](../indexer.md#ordering)'s $$(\text{ledger\\\_seq}, \text{tx\\\_application\\\_order}, \text{event\\\_index})$$.
- **Deduplication.** Events MUST be deduplicated by event id, since a hybrid source ([The hybrid read path and its two failure modes](clients.md#the-hybrid-read-path-and-its-two-failure-modes)) can deliver the same event twice at its seam.
- **Idempotence.** Either application is idempotent, or dedup provably precedes it. Crediting rules accumulate, so a duplicate inflates a balance.

Application rules are [Wallet State and Recovery](../protocol/wallet-state.md)'s update table and are not restated here. Two properties worth making explicit:

- `Withdraw`, sender-side `Transfer`, and `SetSpender` **overwrite** $$W_{\text{spend}}$$ from the event's $$(\tilde{b}, \sigma)$$ rather than adjusting it, so a wallet that missed intervening events still converges on the spendable side.
- `RevokeSpender` is not a checkpoint — it carries no $$\tilde{b}$$ — and is **folded**, like `Merge`. `RevokeSpender` adds the reclaimed allowance opening to $$W_{\text{spend}}$$, with $$v_a$$ and $$r_a$$ recovered from the event's $$\tilde{a}$$ and `allowance_salt` under the owner-derived $$dvk_i$$ ([Revoke Spender](../protocol/operations/revoke-spender.md)). A wallet that misses either event diverges until the next checkpoint and MUST rely on [Consistency checking](#consistency-checking) to detect that.
- `Clawback` is likewise not a checkpoint and is **folded**: the `Merge` rule, then the event's public `amount` subtracted from $$W_{\text{spend}}$$'s value ([Wallet and Auditor Consequences](../compliance.md#wallet-and-auditor-consequences)). It occurs only in deployments that enable the compliance extension.
- A self-transfer — a `Transfer` whose `from` and `to` are the same account — MUST be applied in both roles: the sender side overwrites $$W_{\text{spend}}$$ from the event's $$(\tilde{b}, \sigma)$$, and the recipient side credits $$W_{\text{receive}}$$ from the same event's recipient-channel ciphertexts. The two roles act on different accumulators, so their relative order does not affect the result; applying only one loses the other accumulator's update.

## In-flight operations

A wallet MAY project the post-operation opening immediately on successful submission rather than waiting for the event, so that the balance a user sees reflects an action they just took.

The projection MUST still be reconciled against the event, and MUST NOT be treated as confirmed for the purpose of [Consistency checking](#consistency-checking).

## Salt freshness

A fresh salt MUST be sampled for every **attempt**, including retries after a reverted or dropped transaction. The salt that must be fresh is the one the operation's pads absorb: $$\sigma$$ for owner-initiated operations, and $$\sigma_a'$$ — the replacement allowance salt — for spender transfers ([Transfer nonce](../protocol/account-state.md#transfer-nonce)).

[Revert Safety](../protocol/security.md#revert-safety) motivates this as unlinkability: a fresh salt prevents an observer correlating a reverted attempt with its retry. It is equally a confidentiality requirement, because the salt is the sole freshness input to every derived pad in the operation, the ephemeral scalar included ([Poseidon2 Hash](../protocol/primitives.md#poseidon2-hash), [ECDH-Derived Blinding](../protocol/keys-and-commitments.md#ecdh-derived-blinding)). Reuse therefore repeats the ephemeral key and every channel mask that depends on it.

An implementation MUST NOT cache or reuse a salt across attempts, and MUST NOT derive it from anything an observer can predict.

**Freshness is not enforced on-chain.** Constraint O14 rejects only $$\sigma_a' = \sigma_a$$, the adjacent collision; a circuit cannot see a delegation's older salts. An implementation MUST therefore treat non-repetition of $$\sigma_a'$$ over the whole life of a delegation as its own obligation, and MUST NOT cycle salts through a bounded set.

## Deterministic ephemeral scalars

An implementation MUST derive the ephemeral scalar of every operation the holder or spender originates:

$$r_e = \text{poseidon\\\_with\\\_domain}(\delta_{\text{eph}}, [vk, \sigma_E])$$

where $$vk$$ is the originator's viewing key and $$\sigma_E$$ the operation's salt. [ECDH-Derived Blinding](../protocol/keys-and-commitments.md#ecdh-derived-blinding) is the normative source: it fixes which viewing key and salt each operation derives from, and the retry rule for the negligible case that the derivation yields zero.

**Three consequences an implementation MUST handle.**

First, **$$vk$$ carries more authority than balance decryption.** Recomputing $$r_e$$ yields the recipient shared scalar, hence $$r_{\text{transfer}}$$, hence a full Pedersen opening of every transfer commitment the account created — retroactively, and reaching commitments that sit inside recipients' receiving balances ([Privacy Properties](../protocol/security.md#privacy-properties), [Per-Transfer Auditor Ciphertexts](../protocol/auditing.md#per-transfer-auditor-ciphertexts)). An implementation MUST treat $$vk$$ accordingly in [Security Requirements](requirements.md#security-requirements) and MUST NOT export it as a read-only credential without stating this.

Second, **the salt requirement of [Salt freshness](#salt-freshness) is a confidentiality requirement**, not only an unlinkability one, the salt being the operation's sole freshness input.

Third, **disclosability is unverifiable from chain data.** No on-chain value distinguishes an ephemeral this derivation produced from one it did not, so an implementation MUST NOT infer disclosability from a stored per-transfer flag and MUST determine it by test ([Disclosure verification](clients.md#disclosure-verification)). Transfers predating this specification may not be disclosable by their sender.

## Consistency checking

The wallet MUST verify its openings against on-chain commitments by re-committing and comparing ([Consistency check](../protocol/wallet-state.md#consistency-check)), and MUST do so both after every sync and before constructing any proof. A missed event, a duplicate, an expired credit, or a defect then produces a mismatch rather than a plausible wrong balance, and a mismatched state MUST NOT be spent from.

Implementations MUST report which accumulator diverged, since the two have different causes and different remedies.

## The unspendable-blinding case

$$W_{\text{spend}}.r$$ is an exact integer, but a proof witnesses it as a single $$\mathbb{F}_r$$ `Field`. After any of the three proofless folds — `Merge`, `RevokeSpender`, `Clawback` — its canonical $$\mathbb{F}_q$$ representative can land in $$[r, q)$$, which no `Field` encodes, so no proof can be constructed against that commitment. The state itself is sound: the commitment is a well-formed Grumpkin point, and [Consistency checking](#consistency-checking)'s check recommits from the same representative and still passes ([Post-merge witness availability](../protocol/proof-system.md#post-merge-witness-availability)).

An implementation MUST test the reduction rather than the accumulator: before attempting a proof, reduce $$W_{\text{spend}}.r$$ modulo $$q$$ at [Blinding accumulation](crypto-core.md#blinding-accumulation)'s reduction point and compare the representative against $$r$$. The unreduced accumulator carries no signal: after a few folds an integer sum past $$r$$ is the ordinary case.

A failed comparison MUST surface as a distinct, named state rather than as a generic proof-construction failure. The recovery path MUST surface with it: the condition resolves at the next merge that folds in an inbound confidential transfer, and an account whose only inflows are deposits stays affected until one arrives ([Soft recovery](../protocol/proof-system.md#soft-recovery)).

## Merge policy

Received funds are spendable only after a merge ([Merge](../protocol/operations/merge.md)). A wallet SHOULD merge automatically, or prompt, ahead of a spend that the spendable balance alone cannot cover. Merging ahead of a spend also bounds the recovery replay window ([Recovery](../protocol/wallet-state.md#recovery)).

Merge is proof-less and owner-authorized, and neither a merge nor an in-flight spend proof can be disrupted by a third party ([Griefing Resistance](../protocol/security.md#griefing-resistance), [Merge Safety](../protocol/security.md#merge-safety)).

## Recovery

Recovery follows the procedure of [Recovery](../protocol/wallet-state.md#recovery), with the replayed events applied per [Event application](#event-application) and the reconstructed state verified per [Consistency checking](#consistency-checking).

Two further obligations follow from data availability:

- Recovery from a root alone depends on a conforming indexer ([Indexing and Off-Chain State Recovery](../indexer.md)). Without one, a client can see that funds exist but cannot reconstruct the opening needed to spend them.
- **With RPC-only event access, a client MUST sync at least once per RPC retention window**, and MUST warn when it has not. Any event that ages out before it is applied takes its opening with it permanently: a crediting event on the receiving side, which is a running sum from $$T_0$$, and a `RevokeSpender` after the latest checkpoint, whose $$\tilde{a}$$ and `allowance_salt` survive nowhere else once the delegation entry is deleted ([Revoke Spender](../protocol/operations/revoke-spender.md)). Only the spend history preceding the checkpoint is expendable, each checkpoint being self-contained.

## Spender-side wallet

A spender reconstructs its allowance state from the on-chain delegation entry rather than from event replay: it recovers $$dvk_i$$ from the escrowed value by ECDH ([Delegation Key Escrow](../protocol/operations/set-spender.md#delegation-key-escrow)), then reads the current allowance from the entry's encrypted allowance and salt ([Read Methods](../protocol/interface.md#read-methods)).

Implementations MUST surface the delegation's expiry ledger and SHOULD warn ahead of it. They MUST represent expired-but-unrevoked delegations as still holding escrowed value ([Spender Delegation](../protocol/account-state.md#spender-delegation)). A `RevokeSpender` event with the entry gone means the owner folded the delegation back.

A spender MUST NOT be able to reach the owner's spendable balance through any interface ([Roles and Capability Separation](README.md#roles-and-capability-separation)).

---

Previous: [Witness Assembly, Prover, and Chain Adapter](proving.md) · Up: [Documentation Index](../README.md) · Next: [Auditor Client](auditor-client.md)
