# Security Analysis

## Griefing Resistance

**Proposition 2** (Spend-proof stability). *No third party can invalidate an honest owner's in-flight spend proof.*

*Proof.* A spend proof (Section 7.6) references $$C\_{\text{spend}}^A$$ as a public input. The contract modifies $$C\_{\text{spend}}^A$$ only through:
1. Owner-initiated operations (transfer, withdrawal, `set_spender`, `revoke_spender`) - all require `account.require_auth()`.
2. Merge - requires `account.require_auth()`.

Incoming transfers modify only $$C\_{\text{receive}}^A$$, which does not appear in the spend proof's public inputs. Therefore, between proof construction and submission, no third-party action can alter $$C\_{\text{spend}}^A$$. The proof remains valid. $$\square$$

**Corollary.** There is no counter cap on incoming transfers. An account can receive an unbounded number of transfers without any mandatory owner action. The receiving balance is a single point whose committed value grows monotonically; there is no chunk overflow because Pedersen commitments operate over the full scalar field ($$|\mathbb{F}\_q| \approx 2^{254}$$).

## Merge Safety

**Proposition 3** (Merge cannot be weaponized). *A third party cannot invoke merge on another account.*

*Proof.* The `merge()` function requires `account.require_auth()`. Only the account holder can authorize it. $$\square$$

**Proposition 4** (Merge does not create or destroy value). *Follows directly from Proposition 1 and the homomorphic property of Pedersen commitments.*

## Balance Conservation

**Invariant.** For any account at any time:

$$\sum\_{j} d\_j - \sum\_{k} w\_k = v\_{\text{spend}} + v\_{\text{receive}} + \sum\_{i} v\_{\text{allowance}\_i}$$

where $$d\_j$$ are deposits, $$w\_k$$ are withdrawals, and the right-hand side sums committed values across the spendable balance, the receiving balance, and every stored (not-yet-revoked) spender allowance. Expired-but-not-revoked allowances are included: the escrowed value resides on-chain in $$C\_a$$ until `revoke_spender` reclaims it (Section 6.2).

This invariant is maintained by:
- **Deposits** increase $$v\_{\text{receive}}$$ by $$d\_j$$ (Section 7.3).
- **Withdrawals** decrease $$v\_{\text{spend}}$$ by $$w\_k$$, enforced by circuit constraint W4.
- **Transfers** decrease sender's $$v\_{\text{spend}}$$ and increase recipient's $$v\_{\text{receive}}$$ by the same $$v\_{\text{transfer}}$$, enforced by circuit constraints T3–T8.
- **Merge** moves value from $$v\_{\text{receive}}$$ to $$v\_{\text{spend}}$$ (Proposition 1); the sum is unchanged.
- **Set spender** moves value from $$v\_{\text{spend}}$$ to $$v\_{\text{allowance}\_i}$$; enforced by S3–S7.
- **Spender transfer** decreases $$v\_{\text{allowance}\_i}$$ and increases recipient's $$v\_{\text{receive}}$$ by $$v\_{\text{transfer}}$$; enforced by O2–O8.
- **Revoke** moves remaining $$v\_{\text{allowance}\_i}$$ back to $$v\_{\text{spend}}$$ (Proposition 1); the sum is unchanged.
- **Clawback** (compliance extension) removes a public amount from $$v\_{\text{spend}} + v\_{\text{receive}}$$ without a withdrawal, enforced by CB1–CB3; in a deployment that enables it, seized amounts count alongside $$w\_k$$ on the left-hand side ([COMPLIANCE.md](./COMPLIANCE.md) §5.4).

## Privacy Properties

**Amount confidentiality.** Transfer amounts are hidden inside Pedersen commitments (computationally hiding under DL). The encrypted amount $$\tilde{v}$$ is masked by $$\text{Poseidon}(\delta\_{\text{transfer\\\_amount}}, s, \sigma)$$, which is pseudorandom to anyone who does not know $$s$$ (the ECDH shared secret).

**Balance confidentiality.** The spendable balance commitment hides both value and blinding. The encrypted balance scalar $$\tilde{b}$$ emitted in spend-boundary events is masked by $$\text{Poseidon}(\delta\_{\text{enc\\\_bal}}, vk, \sigma)$$, pseudorandom without $$vk$$.

**Sender-recipient linkage.** Sender and recipient addresses are visible on-chain. The system provides amount and balance confidentiality, not anonymity.

**Viewing key compromise.** Since $$vk$$ is contract-specific (Section 4.2), compromise of one contract's viewing key does not affect the owner's accounts in other deployments. Within the compromised contract, the attacker can: read all spendable balance snapshots (via $$\tilde{b}$$ emitted in spend-boundary events), decrypt all incoming transfer amounts (via ECDH with $$R\_e$$ from events), derive all $$dvk\_i$$ to read spender allowances, and recompute the ephemeral scalar $$r\_e$$ of every transfer the account *originated* (DESIGN.md §5.3), hence the recipient shared scalar $$s$$, hence a full Pedersen opening $$(v\_{\text{transfer}}, r\_{\text{transfer}})$$ of each such $$C\_{\text{transfer}}$$. The attacker **cannot** authorize any spending operation (requires $$sk$$, and $$vk$$ cannot recover $$sk$$ by Poseidon preimage resistance).

Two properties of that last capability deserve stating plainly. It is **retroactive**: the derivation is deterministic in $$(vk, \sigma)$$ and $$\sigma$$ is published, so a compromise today opens every transfer the account ever originated. And it **reaches outside the account's own state**: the commitments it opens were added to *recipients'* `receiving_commitment` values. The transfer *amounts* were already inferable from $$vk$$ alone, by differencing consecutive balance checkpoints and netting the inbound credits and merges between them, so amount visibility is not what changes; the openings are, and an opening is a self-verifying artifact that its holder can hand to any third party. This is why $$vk$$ cannot be treated as a safely shareable read-only credential ([SDK.md](./SDK.md) §13): a counterparty that needs visibility into an account's outbound transfers receives per-event disclosure proofs bound to it and to a nonce ([SELECTIVE_DISCLOSURE.md](./SELECTIVE_DISCLOSURE.md) §7), not the key.

**Auditor key compromise.** An account binds one key for both channels (Section 6.1). For every account that used the compromised key, the attacker acquires: all amounts and balance checkpoints; the standing opening of $$C\_{\text{spend}}$$ from the first post-activation checkpoint, maintained across merges exactly as Section 8.1 describes; the opening of $$C\_{\text{receive}}$$ between merges; and the escrowed allowance blinding $$r\_a$$ from every delegation event it observed, hence the opening of each $$C\_a$$ those events wrote (Section 8.5). It cannot recover viewing keys, read data from before the key was active, or authorize any spending. After key rotation, new operations are protected by the new key.

As with $$vk$$ above, the openings are self-verifying artifacts a holder can hand to any third party, so an auditor key is custodied at the same level as a viewing key ([SDK.md](./SDK.md) §13).

## Revert Safety

Because $$\sigma$$ is sampled fresh via CSPRNG for every operation, a retry after a reverted transaction naturally uses a different $$\sigma$$. This means the deterministic randomness $$r = \text{Poseidon}(\delta\_{\text{spend\\\_r}}, vk, \sigma)$$ is always fresh, and an observer cannot correlate reverted and retried commitments.

**Retry procedure.** On revert, the wallet picks a new random salt -- $$\sigma$$, or $$\sigma\_a'$$ for a spender transfer -- and recomputes the proof. No special-case logic is needed. The salt is a public input and emitted in events so the auditor and owner can reconstruct randomness.

## Replay Protection

**Proposition 5** (Proof non-replayability). *A valid proof cannot be replayed to execute the same operation twice.*

*Proof.* Every spending proof includes the current on-chain commitment ($$C\_{\text{spend}}$$ or $$C\_a$$) as a public input. Upon successful verification, the contract replaces this commitment with the proof's output commitment ($$C\_{\text{spend}}'$$ or $$C\_a'$$). A replayed proof references the old commitment, which no longer matches the stored state, so verification fails. The same argument applies to spender transfers via $$C\_a$$. $$\square$$

**Corollary.** No explicit nullifier or nonce is needed. State binding through commitment chaining provides replay protection as an inherent property of the protocol.
