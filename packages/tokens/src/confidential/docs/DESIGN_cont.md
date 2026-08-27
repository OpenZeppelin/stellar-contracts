# Confidential Token (continued)

<!-- This specification is split into two files because GitHub stops rendering
     LaTeX math after ~750 expressions per page. Keep each part under that
     budget when adding content. -->

This document continues [DESIGN.md](./DESIGN.md), which covers Sections 1-7.
Section numbering is shared across both parts: references of the form
"DESIGN §N" resolve to [DESIGN.md](./DESIGN.md) for §1-§7 and to this document
for §8-§13.

---

## 8. Auditing

### 8.1 Per-Transfer Auditor Ciphertexts

Each confidential transfer produces ciphertexts under two auditor keys via ECDH, using the same ephemeral scalar $$r\_e$$ used for recipient ECDH. Each auditor channel runs Poseidon2 in sponge mode (Section 2.5), absorbing the channel's domain tag, the ECDH shared scalar, and $$\sigma$$; the recipient channel squeezes two masks, the sender channel three (Section 2.5 *Lane assignment*).

**Recipient's auditor** ($$K\_{\text{aud,r}}$$, from the recipient's `auditor_id`) receives the transfer amount and the per-transfer Pedersen randomness:

$$s\_{a,r} = \text{ECDH}(r\_e, K\_{\text{aud,r}}) \qquad \text{(DESIGN §2.4)}$$
$$(m\_{v,r}, m\_{r,r}) = \text{SpongeSqueeze}\_2(\delta\_{\text{aud\\\_r}}, s\_{a,r}, \sigma)$$
$$\tilde{v}\_{\text{aud,r}} = v\_{\text{transfer}} + m\_{v,r}, \qquad \tilde{r}\_{\text{aud,r}} = r\_{\text{transfer}} + m\_{r,r}$$

**Sender's auditor** ($$K\_{\text{aud,s}}$$, from the sender's `auditor_id`) receives the transfer amount, the sender's post-transfer balance, and -- in the lane-2 secret-escrow slot (DESIGN §2.5 *Lane assignment*) -- the sender's post-transfer spendable blinding $$r\_A'$$:

$$s\_{a,s} = \text{ECDH}(r\_e, K\_{\text{aud,s}}) \qquad \text{(DESIGN §2.4)}$$
$$(m\_{v,s}, m\_{b,s}, m\_{r,s}) = \text{SpongeSqueeze}\_3(\delta\_{\text{aud\\\_s}}, s\_{a,s}, \sigma)$$
$$\tilde{v}\_{\text{aud,s}} = v\_{\text{transfer}} + m\_{v,s}, \qquad \tilde{b}\_{\text{aud,s}} = (v\_A - v\_{\text{transfer}}) + m\_{b,s}, \qquad \tilde{r}\_{\text{aud,s}} = r\_A' + m\_{r,s}$$

The transfer circuit (constraints T\_a1--T\_a9) enforces correct computation. At operation time, the contract fetches both auditor keys from the auditor contract using the *stored* `auditor_id` field of each account; neither the sender nor the recipient can substitute a different key for the operation being proven. This guarantee is scoped to operation time: *which* auditor an account is bound to is chosen by the account owner at registration (DESIGN §7.2), subject only to existence in the auditor registry unless the deployment gates the selection in its `Hooks::on_register` implementation ([COMPLIANCE.md](./COMPLIANCE.md) §4.3).

Each auditor decrypts using their secret key $$k$$. For example, the sender's auditor:

$$S\_{a,s} = k \cdot R\_e, \qquad s\_{a,s} = \text{Poseidon}(\delta\_{\text{ecdh}}, S\_{a,s}.x, S\_{a,s}.y)$$
$$(m\_{v,s}, m\_{b,s}, m\_{r,s}) = \text{SpongeSqueeze}\_3(\delta\_{\text{aud\\\_s}}, s\_{a,s}, \sigma)$$
$$v\_{\text{transfer}} = \tilde{v}\_{\text{aud,s}} - m\_{v,s}, \qquad v\_{\text{new}} = \tilde{b}\_{\text{aud,s}} - m\_{b,s}, \qquad r\_{\text{new}} = \tilde{r}\_{\text{aud,s}} - m\_{r,s}$$

where $$R\_e$$ and $$\sigma$$ are published in the Transfer event. The recipient's auditor follows the same pattern with $$\delta\_{\text{aud\\\_r}}$$ to recover the pair $$(v\_{\text{transfer}}, r\_{\text{transfer}})$$.

**The lane-2 slot.** Lane 2 of the sender-auditor channel (DESIGN §2.5 *Lane assignment*) carries the new spendable blinding on the three checkpoint operations (W\_a5, T\_a9, S\_a6) and the delegation viewing key $$dvk\_i$$ on spender transfers (O\_a9, Section 8.4). Carrying two kinds of plaintext under one lane is not pad reuse: the pad is fixed by $$(s\_{a,s}, \sigma)$$ or $$(s\_{a,s}, \sigma\_a)$$, both fresh per operation.

**Recipient-auditor opening capability.** Because the recipient-auditor recovers $$r\_{\text{transfer}}$$ for every inbound transfer, and because deposits add to `receiving_commitment` with $$r = 0$$ (Section 7.3), the recipient-auditor can reconstruct the full Pedersen opening of $$C\_{\text{receive}}$$ between merges:

$$v\_r = \sum\_i v\_{\text{transfer},i} + \sum\_j a\_j, \qquad r\_r = \sum\_i r\_{\text{transfer},i}$$

where $$i$$ ranges over inbound transfers and spender-transfers since the last merge and $$j$$ ranges over deposits. This is a full Pedersen *opening* of $$C\_{\text{receive}}$$: both the value and the blinding are reconstructed by the auditor.

The capability is bounded in three ways:

- **Forward-only.** Only events emitted while the auditor key was active are decryptable.
- **Receiving-side only.** The reconstruction above covers `receiving_commitment`. It does not extend to $$C\_{\text{spend}}$$: the recipient-auditor cannot derive the spend-side blinding $$r\_s = \text{Poseidon}(\delta\_{\text{spend\\\_r}}, vk\_A, \sigma)$$, which depends on $$vk\_A$$. It knows the *value* $$v\_s$$ at every spend boundary via $$\tilde{b}\_{\text{aud,s}}$$ (Section 5.5), and can extend that with the known $$v\_r$$ contribution at each merge. The spend-side opening reaches the *sender*-auditor by a different route -- the lane-2 escrow, bounded separately below -- not by this reconstruction.
- **Reset by merge.** Merge folds $$r\_r$$ into the spendable-balance randomness ($$r\_{\text{spend}}' = r\_s + r\_r$$, Section 7.4) and emits no checkpoint, so the reconstruction above restarts from the next inbound flow.

**Sender-auditor opening capability.** The lane-2 escrow hands the sender-auditor the *blinding* of the account's post-operation spendable balance directly, without $$vk\_A$$: together with the value in $$\tilde{b}\_{\text{aud,s}}$$ it is a full Pedersen opening of $$C\_{\text{spend}}'$$. It is available at exactly the three checkpoint operations that escrow lane 2 -- withdrawal (W\_a5), outgoing transfer (T\_a9), and `set_spender` (S\_a6) -- and is likewise bounded:

- **Forward-only**, on the same grounds as the recipient side.
- **Maintained across merges.** An account binds a single `auditor_id` (Section 6.1), so the key that decrypts the lane-2 escrow is the same key that decrypts the recipient channel of every inbound flow to that account. Merge adds both the values and the blindings ($$v\_{\text{spend}}' = v\_s + v\_r$$, $$r\_{\text{spend}}' = r\_s + r\_r$$, Section 7.4), and the auditor holds each addend: $$(v\_{\text{transfer},i}, r\_{\text{transfer},i})$$ from the recipient-channel reconstruction above, and $$(a\_j, 0)$$ from the public deposits. It therefore carries the escrowed opening forward through every merge by the same addition the contract performs, rather than losing it at one.
- **Not renewed by `revoke_spender`.** V\_a3 stays two-lane (DESIGN §7.9), so a revoke rewrites $$C\_{\text{spend}}$$ under a blinding the auditor never receives and leaves it with the post-reclaim *value* alone. The opening is re-acquired at the next checkpoint (W\_a5, T\_a9, S\_a6).
- **Rotation-scoped.** A newly activated key cannot decrypt escrows published under the previous one (Section 8.3). An auditor that carries its accumulated opening across the rotation keeps it; one that bootstraps from the new key alone re-acquires it at the next checkpoint.

The recipient-side reconstruction is therefore event-scoped, while the sender-side escrow gives the account's auditor a **standing** opening of $$C\_{\text{spend}}$$, held continuously from its first checkpoint under the active key.

These bounded openings are what enable the clawback flow specified in [COMPLIANCE.md](./COMPLIANCE.md) §5: the recipient-auditor is the seize-enabling party for inbound flows while $$C\_{\text{receive}}$$ has not yet been merged, while the sender-auditor is the seize-enabling party for the spendable-balance side via $$\tilde{b}\_{\text{aud,s}}$$ and $$\tilde{r}\_{\text{aud,s}}$$.

### 8.2 Auditor Visibility Properties

**Transfer amounts.** Both auditors see the transfer amount in real time. The recipient's auditor decrypts $$v\_{\text{transfer}}$$ from $$\tilde{v}\_{\text{aud,r}}$$; the sender's auditor decrypts it from $$\tilde{v}\_{\text{aud,s}}$$.

**Balance checkpoints.** The sender's auditor receives an encrypted balance checkpoint at every owner-initiated operation that produces a proof:

- **Outgoing transfer**: auditor decrypts post-transfer balance $$(v\_A - v\_{\text{transfer}})$$ from $$\tilde{b}\_{\text{aud,s}}$$ and the matching blinding $$r\_A'$$ from $$\tilde{r}\_{\text{aud,s}}$$ (constraints T\_a5--T\_a9).
- **Withdrawal**: auditor decrypts post-withdrawal balance $$(v - a)$$ from $$\tilde{b}\_{\text{aud,s}}$$ and the matching blinding $$r'$$ from $$\tilde{r}\_{\text{aud,s}}$$ (constraints W\_a1--W\_a5). The withdrawal amount $$a$$ is also visible as a public input.
- **Set spender**: auditor decrypts escrowed amount $$v\_a$$ from $$\tilde{v}\_{\text{aud,s}}$$, post-escrow balance $$(v - v\_a)$$ from $$\tilde{b}\_{\text{aud,s}}$$, and the matching blinding $$r'$$ from $$\tilde{r}\_{\text{aud,s}}$$ (constraints S\_a1--S\_a6).
- **Revoke spender**: auditor decrypts reclaimed amount $$v\_a$$ from $$\tilde{v}\_{\text{aud,s}}$$ and post-reclaim balance $$(v\_s + v\_a)$$ from $$\tilde{b}\_{\text{aud,s}}$$ (constraints V\_a1--V\_a5). This is the one checkpoint operation that escrows no blinding: V\_a3 stays two-lane.

The three operations that also escrow the post-operation spendable blinding confer the opening capability bounded in Section 8.1. The recipient's auditor does not see the sender's balance in any of these operations.

**Per-transfer Pedersen randomness (recipient-auditor, not sender-auditor).** Beyond the transfer amount, the recipient's auditor also decrypts the per-transfer Pedersen blinding $$r\_{\text{transfer}}$$ from $$\tilde{r}\_{\text{aud,r}}$$ on every confidential transfer and spender-transfer; Section 8.1 states the opening capability this confers and its bounds (forward-only, receiving-side only, reset by merge). The sender's auditor does not see $$r\_{\text{transfer}}$$. The *originating account's own* $$vk$$ holder does, by recomputing $$r\_e$$ and hence $$s$$ (DESIGN.md §5.3); that path lies outside the auditor model and its consequences are stated in §9.4.

**Key rotation.** When the auditor contract sets a new key under the account's `auditor_id` (§8.3), the new key sees the balance checkpoint at the next owner-initiated operation, with no event replay or bootstrapping required. The balance checkpoint is self-contained: it depends only on the auditor's ECDH secret key and the published $$(R\_e, \sigma)$$. Note that `auditor_id` itself is immutable per account (§6.1); only the key under that `auditor_id` rotates.

**Not exclusive to the recipient-auditor.** Because $$r\_e$$ is derived from the originator's viewing key and the operation salt (DESIGN.md §5.3), each transfer's originator can recompute $$r\_e$$, hence $$s$$, hence $$r\_{\text{transfer}}$$ — so the opening of any single $$C\_{\text{transfer},i}$$ is held both by the recipient's auditor and by the originating account's $$vk$$ holder.

### 8.3 Auditor Key Management and Rotation

The auditor contract stores Grumpkin public keys as full affine points $$(x, y)$$ indexed by `auditor_id`. The contract validates that every inserted key is canonical, on-curve ($$y^2 \equiv x^3 - 17 \pmod{r}$$), and non-identity at insertion time (Section 3.1, Section 10.8). Each `auditor_id` MAY maintain a sequence of versions, each carrying its activation ledger, in which case rotation appends a new entry rather than overwriting the previous one. The reference registry shipped in this repository takes the simpler form: it keeps a single current key per `auditor_id`, which `rotate_key` overwrites in place; a versioned, activation-ledger registry is an optional production target.

When building public inputs for any operation that produces auditor ciphertexts (transfers, withdrawals, set/revoke spender), the contract fetches the relevant auditor keys for the recipient's and/or sender's `auditor_id`. The contract passes the full Grumpkin point as a public input; the circuit constrains the ECDH ciphertexts against that exact point. The contract and the circuit are version-agnostic: they verify against whichever key the auditor contract currently exposes.

**In-flight proofs across rotation.** A proof constructed against version $$v$$ becomes unverifiable the instant the auditor contract activates version $$v+1$$. The $$K\_{\text{aud}}$$ public input the contract fetches at verification no longer matches the value the prover committed to, so UltraHonk verification fails and the invocation **reverts at the proof-verification boundary**. The caller (sender, owner, or spender) reconstructs the proof against the new $$K\_{\text{aud}}$$ and resubmits. The rejection is benign: the contract's spendable balance, receiving balance, and delegation state are unchanged by the reverted call, $$\sigma$$ is freshly sampled on retry (Section 9.6), and an observer cannot correlate the rejected attempt with the resubmission.

**Auditor's off-chain obligation.** The auditor MUST retain the secret key for every historical version it has issued. To decrypt an event at ledger $$L$$, the auditor resolves the version from its own rotation records (with a versioned activation-ledger registry, the auditor instead queries the auditor contract for the version of its `auditor_id` whose activation ledger is the largest value not exceeding $$L$$), then uses the corresponding off-chain secret key against the $$R\_e$$ and $$\sigma$$ (or $$\sigma\_a$$) emitted in the event.

### 8.4 Spender Transfer Auditing

Each spender transfer produces auditor ciphertexts under two keys (constraints O\_a1--O\_a9), following the same dual-auditor sponge model as owner transfers. The recipient's auditor decrypts the transfer amount and the per-transfer Pedersen randomness:

$$(m\_{v,r}, m\_{r,r}) = \text{SpongeSqueeze}\_2(\delta\_{\text{aud\\\_r}}, s\_{a,r}, \sigma\_a)$$
$$v\_{\text{transfer}} = \tilde{v}\_{\text{aud,r}} - m\_{v,r}, \qquad r\_{\text{transfer}} = \tilde{r}\_{\text{aud,r}} - m\_{r,r}$$

The owner's auditor decrypts the transfer amount, the post-transfer allowance, and -- in the lane-2 secret-escrow slot -- the delegation viewing key $$dvk\_i$$ (O\_a9):

$$(m\_{v,s}, m\_{a,s}, m\_{r,s}) = \text{SpongeSqueeze}\_3(\delta\_{\text{aud\\\_s}}, s\_{a,s}, \sigma\_a)$$
$$v\_{\text{transfer}} = \tilde{v}\_{\text{aud,s}} - m\_{v,s}, \qquad v\_a' = \tilde{a}\_{\text{aud,s}} - m\_{a,s}, \qquad dvk\_i = \text{dvk\\\_cipher\\\_aud} - m\_{r,s}$$

where $$s\_{a,r}$$, $$s\_{a,s}$$, and $$\sigma\_a$$ are recovered from the event as in Section 8.1. The recipient-auditor opening capability stated in Section 8.1 extends to spender-transfer inbound flows: $$r\_{\text{transfer}}$$ from spender-transfers contributes to $$r\_r$$ in $$C\_{\text{receive}}$$ identically to owner-transfer inbound flows.

### 8.5 Spender Allowance Auditing

The auditor tracks each allowance's current value through the per-event ciphertexts produced at every state-changing operation: `set_spender` reveals the escrowed amount $$v\_a$$ (Section 8.2), `confidential_transfer_from` reveals the transfer amount and post-transfer allowance $$v\_a'$$ (Section 8.4), and `revoke_spender` reveals the reclaimed amount (Section 8.2).

**Allowance opening.** The owner's auditor also receives $$dvk\_i$$ itself: under $$\delta\_{\text{esc\\\_dvk\\\_aud}}$$ at `set_spender` (S14, §8.5 *Auditor-side delegation-key escrow*) and in lane 2 at every spender transfer (O\_a9, §8.4). Since the allowance blinding is $$r\_a = \text{Poseidon}(\delta\_{\text{allow\\\_r}}, dvk\_i, \sigma\_a)$$, and the salt that opens the commitment written by each event is published in that event -- $$\sigma\_a$$ on `SetSpender`, $$\sigma\_a'$$ on `SpenderTransfer` (§11.2) -- the auditor reconstructs the full Pedersen opening of $$C\_a$$ at each of those events, with no on-chain read. Unlike the spendable side there is no merge to fold in, since a delegation's only state transitions are the events themselves.

**Auditor-side delegation-key escrow.** At `set_spender` the owner escrows $$dvk\_i$$ to its own auditor as well as to the spender:

$$\text{dvk\\\_cipher\\\_aud} = dvk\_i + \text{Poseidon}(\delta\_{\text{esc\\\_dvk\\\_aud}}, s\_{a,s}, \text{op}\_i)$$

reusing the S\_a2 shared scalar $$s\_{a,s} = \text{ECDH}(r\_e, K\_{\text{aud,s}})$$ rather than opening a new ECDH channel, so it costs one Poseidon and no scalar multiplication (DESIGN S14). The auditor recovers $$s\_{a,s}$$ from $$k\_{\text{aud,s}}$$ and the event's $$R\_e$$, and $$\text{op}\_i$$ from the event's `spender` topic, then subtracts.

A single-output pad rather than a sponge lane, because lane 2 of this channel is already taken by the spendable blinding (S\_a6). Its separation from that channel is DESIGN §2.5 *Mode exclusivity*: $$\delta\_{\text{esc\\\_dvk\\\_aud}}$$ is a single-output tag and $$\delta\_{\text{aud\\\_s}}$$ a multi-lane one, so the two never produce the same field element from the same $$(s\_{a,s}, \cdot)$$. $$\text{op}\_i$$ is a per-$$(owner, spender)$$ constant that separates pads across delegations, not a nonce: the pad's freshness rests entirely on $$s\_{a,s}$$, hence on $$r\_e$$, hence on the salt (DESIGN §5.3, §9.6). Re-delegating to the same spender under a reused salt would republish a byte-identical ciphertext.

**Key rotation.** Visibility is forward-only at the event level, matching the spendable-balance model (§8.2). A new key under the account's existing `auditor_id` sees an allowance at the next state-changing operation, when a fresh ciphertext is produced under the new key.

---

## 9. Security Analysis

### 9.1 Griefing Resistance

**Proposition 2** (Spend-proof stability). *No third party can invalidate an honest owner's in-flight spend proof.*

*Proof.* A spend proof (Section 7.6) references $$C\_{\text{spend}}^A$$ as a public input. The contract modifies $$C\_{\text{spend}}^A$$ only through:
1. Owner-initiated operations (transfer, withdrawal, `set_spender`, `revoke_spender`) - all require `account.require_auth()`.
2. Merge - requires `account.require_auth()`.

Incoming transfers modify only $$C\_{\text{receive}}^A$$, which does not appear in the spend proof's public inputs. Therefore, between proof construction and submission, no third-party action can alter $$C\_{\text{spend}}^A$$. The proof remains valid. $$\square$$

**Corollary.** There is no counter cap on incoming transfers. An account can receive an unbounded number of transfers without any mandatory owner action. The receiving balance is a single point whose committed value grows monotonically; there is no chunk overflow because Pedersen commitments operate over the full scalar field ($$|\mathbb{F}\_q| \approx 2^{254}$$).

### 9.2 Merge Safety

**Proposition 3** (Merge cannot be weaponized). *A third party cannot invoke merge on another account.*

*Proof.* The `merge()` function requires `account.require_auth()`. Only the account holder can authorize it. $$\square$$

**Proposition 4** (Merge does not create or destroy value). *Follows directly from Proposition 1 and the homomorphic property of Pedersen commitments.*

### 9.3 Balance Conservation

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
- **Revoke** moves remaining $$v\_{\text{allowance}\_i}$$ back to $$v\_{\text{spend}}$$; enforced by V4–V7.

### 9.4 Privacy Properties

**Amount confidentiality.** Transfer amounts are hidden inside Pedersen commitments (computationally hiding under DL). The encrypted amount $$\tilde{v}$$ is masked by $$\text{Poseidon}(\delta\_{\text{transfer\\\_amount}}, s, \sigma)$$, which is pseudorandom to anyone who does not know $$s$$ (the ECDH shared secret).

**Balance confidentiality.** The spendable balance commitment hides both value and blinding. The encrypted balance scalar $$\tilde{b}$$ emitted in spend-boundary events is masked by $$\text{Poseidon}(\delta\_{\text{enc\\\_bal}}, vk, \sigma)$$, pseudorandom without $$vk$$.

**Sender-recipient linkage.** Sender and recipient addresses are visible on-chain. The system provides amount and balance confidentiality, not anonymity.

**Viewing key compromise.** Since $$vk$$ is contract-specific (Section 4.2), compromise of one contract's viewing key does not affect the owner's accounts in other deployments. Within the compromised contract, the attacker can: read all spendable balance snapshots (via $$\tilde{b}$$ emitted in spend-boundary events), decrypt all incoming transfer amounts (via ECDH with $$R\_e$$ from events), derive all $$dvk\_i$$ to read spender allowances, and recompute the ephemeral scalar $$r\_e$$ of every transfer the account *originated* (DESIGN.md §5.3), hence the recipient shared scalar $$s$$, hence a full Pedersen opening $$(v\_{\text{transfer}}, r\_{\text{transfer}})$$ of each such $$C\_{\text{transfer}}$$. The attacker **cannot** authorize any spending operation (requires $$sk$$, and $$vk$$ cannot recover $$sk$$ by Poseidon preimage resistance).

Two properties of that last capability deserve stating plainly. It is **retroactive**: the derivation is deterministic in $$(vk, \sigma)$$ and $$\sigma$$ is published, so a compromise today opens every transfer the account ever originated. And it **reaches outside the account's own state**: the commitments it opens were added to *recipients'* `receiving_commitment` values. The transfer *amounts* were already inferable from $$vk$$ alone, by differencing consecutive balance checkpoints and netting the inbound credits and merges between them, so amount visibility is not what changes; the openings are, and an opening is a self-verifying artifact that its holder can hand to any third party. This is why $$vk$$ cannot be treated as a safely shareable read-only credential ([SDK.md](./SDK.md) §13): a counterparty that needs visibility into an account's outbound transfers receives per-event disclosure proofs bound to it and to a nonce ([SELECTIVE_DISCLOSURE.md](./SELECTIVE_DISCLOSURE.md) §7), not the key.

**Auditor key compromise.** An account binds one key for both channels (Section 6.1), so a compromise is best read per account rather than per role. For every account that used the compromised key, the attacker acquires: all amounts and balance checkpoints; the standing opening of $$C\_{\text{spend}}$$ from the first post-activation checkpoint, maintained across merges exactly as Section 8.1 describes; the opening of $$C\_{\text{receive}}$$ between merges; and every $$dvk\_i$$, hence the opening of every $$C\_a$$ (Section 8.5). It cannot recover viewing keys, read data from before the key was active, or authorize any spending. After key rotation, new operations are protected by the new key.

As with $$vk$$ above, what the openings add over the amounts is that an opening is a self-verifying artifact its holder can hand to any third party. An auditor key is therefore custodied at the same level as a viewing key, not at the level of a read-only amount feed ([SDK.md](./SDK.md) §13).

### 9.5 State Recovery

The recovery procedure, the definition of the **checkpoint** it is built around, the replay window it runs over, and the durable-archive requirement it depends on are all specified in [DESIGN.md](./DESIGN.md) §5.2 *Recovery*. This section states the security properties that follow from it.

**Why the two anchors compose.** No `Merge` falls strictly between $$T\_0$$ and the checkpoint, by construction of $$T\_0$$ as the last merge at or before it. Within $$(T\_0, \text{checkpoint}]$$ the only spendable-affecting events are therefore checkpoint events, and the latest checkpoint's $$(\tilde{b}, \sigma)$$ is the final word on $$W\_{\text{spend}}$$ regardless of them — which is why the replay skips them ([DESIGN.md](./DESIGN.md) §5.2 *Recovery*, step 6). Any `Merge` the replay encounters necessarily sits after the checkpoint and is folded normally, against the receiving opening it actually consumed.

**Recoverability.** An owner who holds $$vk$$ reconstructs both openings deterministically from the account's event history, so the loss of local wallet state is not a loss of funds. The single-lookup shortcut on the spendable side is sound rather than merely convenient: each checkpoint's $$\tilde{b}$$ is bound to the $$C\_{\text{spend}}$$ it certifies by the emitting operation's own proof (W7, T12, S11, V8), so a checkpoint cannot misreport the balance the wallet restarts from. Beyond the event history and the on-chain commitments, recovery requires nothing from the auditor, from counterparties, or from the contract admin.

**Integrity fails closed.** The event archive is trusted for availability, not for integrity. Recovery terminates in a consistency check against the on-chain commitments ([DESIGN.md](./DESIGN.md) §5.2 *Recovery*, step 7) and Pedersen commitments are binding (Section 2.3), so a tampered or truncated history cannot yield an opening that verifies against a balance the account does not hold — it yields a detectable mismatch. The residual risk is therefore denial of recovery, a liveness failure, rather than silent corruption of the reconstructed balance. [INDEXER.md](./INDEXER.md) §7 specifies the archive's trust model and the client-side checks that follow from it.

**Incoming-transfer spam.** A third party can spam an account with confidential transfers (including zero-value transfers, see Section 9.1 Corollary) without invalidating the recipient's spend proofs. The cost to the spammer is the Soroban transaction fee per transfer, which bounds the rate. The cost to the recipient is per-event indexer storage and wallet replay work. Both costs are linear in the number of incoming transfers and bounded by the replay window; neither breaks correctness.

### 9.6 Revert Safety

Because $$\sigma$$ is sampled fresh via CSPRNG for every operation, a retry after a reverted transaction naturally uses a different $$\sigma$$. This means the deterministic randomness $$r = \text{Poseidon}(\delta\_{\text{spend\\\_r}}, vk, \sigma)$$ is always fresh, and an observer cannot correlate reverted and retried commitments.

**Retry procedure.** On revert, the wallet simply picks a new random $$\sigma$$ and recomputes the proof. No special-case logic is needed. The $$\sigma$$ is a public input and emitted in events so the auditor and owner can reconstruct randomness.

### 9.7 Replay Protection

**Proposition 5** (Proof non-replayability). *A valid proof cannot be replayed to execute the same operation twice.*

*Proof.* Every spending proof includes the current on-chain commitment ($$C\_{\text{spend}}$$ or $$C\_a$$) as a public input. Upon successful verification, the contract replaces this commitment with the proof's output commitment ($$C\_{\text{spend}}'$$ or $$C\_a'$$). A replayed proof references the old commitment, which no longer matches the stored state, so verification fails. The same argument applies to spender transfers via $$C\_a$$. $$\square$$

**Corollary.** No explicit nullifier or nonce is needed. State binding through commitment chaining provides replay protection as an inherent property of the protocol.

---

## 10. Proof System

### 10.1 Circuits

All proof logic is written in Noir, compiled to UltraHonk circuits. The system uses 6 circuits:

```rust
#[contracttype]
#[repr(u32)]
pub enum CircuitType {
    Register = 0,
    Withdraw = 1,
    Transfer = 2,
    SpenderTransfer = 3,
    SetSpender = 4,
    RevokeSpender = 5,
}
```

### 10.2 Circuit Summary

| Circuit | What it proves |
|:---|:---|
| `Register` | Spending key well-formedness; contract-bound viewing key derivation from $$sk$$; public viewing key consistency with the derived $$vk$$ |
| `Withdraw` | Balance sufficiency; new spendable commitment with deterministic randomness; encrypted balance scalar; sender-auditor ECDH ciphertexts (balance checkpoint + lane-2 escrow of the new spendable blinding); owner key ownership |
| `Transfer` | Balance conservation; ECDH-derived blinding and encrypted amount for recipient; dual-auditor channel sponges (recipient auditor: amount + per-transfer Pedersen randomness; sender auditor: amount + balance + lane-2 escrow of the new spendable blinding); deterministic randomness for new sender balance; encrypted balance scalar; sender key ownership; range validity (balance $$\in [0, 2^{127})$$, amount $$\in [0, 2^{127})$$) |
| `SpenderTransfer` | Allowance sufficiency; ECDH-derived blinding and encrypted amount for recipient; dual-auditor channel sponges (recipient auditor: amount + per-transfer Pedersen randomness; owner auditor: amount + allowance + lane-2 escrow of $$dvk\_i$$); deterministic randomness for new allowance; encrypted allowance scalar; spender key ownership; contract-bound indirectly via $$C\_a$$ chain (Section 7.8) |
| `SetSpender` | Balance split; $$dvk\_i$$ derivation; ECDH escrow of $$dvk\_i$$ to the spender and to the owner's auditor; allowance commitment with deterministic randomness; encrypted balance and allowance scalars; owner-auditor ECDH ciphertexts (escrow amount + balance checkpoint + lane-2 escrow of the new spendable blinding); owner key ownership; contract-bound via $$vk$$ derivation |
| `RevokeSpender` | Allowance decryption via $$dvk\_i$$; balance merge; deterministic randomness for new balance; encrypted balance scalar; owner-auditor ECDH ciphertexts (reclaimed amount + balance checkpoint); owner key ownership; contract-bound via $$vk$$ derivation |

### 10.3 Circuit Cost Analysis

The dominant cost in Noir circuits is elliptic curve scalar multiplication. With Barretenberg's native Grumpkin support via `multi_scalar_mul`, each scalar multiplication costs approximately 64 UltraPlonk-equivalent constraints (with ECC VM) or 4,700–6,250 without.

**Counting unit.** The totals below count `multi_scalar_mul` *call sites*, matching how the circuits are written (`circuits/lib/src/lib.nr`): a Pedersen commitment is one call over two scalars, and each `ecdh` is one call plus a Poseidon2 absorb (§2.4). A per-scalar count is higher, since every commitment contributes two. Each row lists every site, so the constraint identifiers are the audit trail.

| Circuit | Calls | Sites |
|:---|:--:|:---|
| `Register` | 2 | $$Y$$ (R1), $$\text{PVK}$$ (R3) |
| `Withdraw` | 5 | $$Y$$ (W1), $$C\_{\text{spend}}$$ opening (W3), $$C\_{\text{spend}}'$$ (W6), $$R\_e$$ (W\_a1), sender-auditor ECDH (W\_a2) |
| `Transfer` | 8 | $$Y\_A$$ (T1), $$C\_{\text{spend}}^A$$ opening (T3), recipient ECDH (T5), $$R\_e$$ (T6), $$C\_{\text{transfer}}$$ (T8), $$C\_{\text{spend}}'$$ (T11), recipient-auditor ECDH (T\_a1), sender-auditor ECDH (T\_a5) |
| `SpenderTransfer` | 8 | $$Y\_{\text{op}}$$ (O1), $$C\_a$$ opening (O2), recipient ECDH (O5), $$R\_e$$ (O6), $$C\_{\text{transfer}}$$ (O8), $$C\_a'$$ (O11), recipient-auditor ECDH (O\_a1), owner-auditor ECDH (O\_a5) |
| `SetSpender` | 7 | $$Y$$ (S1), $$C\_{\text{spend}}$$ opening (S3), $$C\_a$$ (S7), $$C\_{\text{spend}}'$$ (S10), $$R\_e$$ (S\_a1), $$dvk\_i$$ escrow ECDH (S12, §7.11), owner-auditor ECDH (S\_a2) |
| `RevokeSpender` | 6 | $$Y$$ (V1), $$C\_a$$ opening (V4), $$C\_{\text{spend}}$$ opening (V5), $$C\_{\text{spend}}'$$ (V7), $$R\_e$$ (V\_a1), owner-auditor ECDH (V\_a2) |

`SetSpender` is the one circuit with a third ECDH beyond the auditor channel: the $$dvk\_i$$ handoff of §7.11 reuses $$r\_e$$ but multiplies it against $$Y\_{\text{op}}$$, so it is a separate call, not a reuse of the S\_a2 shared secret. The auditor-side escrow of $$dvk\_i$$ (S14) reuses the S\_a2 shared scalar and adds a Poseidon evaluation, not a call. The lane-2 escrows (W\_a5, T\_a9, S\_a6, O\_a9) read a third lane of a permutation each circuit already computes and cost one field addition apiece. The ordering these totals imply is consistent with the committed ACIR opcode counts in `circuits/constraints.baseline`: `Register` 33, `Withdraw` 95, `RevokeSpender` 123, `Transfer` 134, `SetSpender` 135, `SpenderTransfer` 136.

The ECDH computations add scalar multiplications compared to a random-blinding scheme, but the unchunked design eliminates all per-chunk constraints (which, in a chunked scheme, would involve 8+ scalar multiplications for balance chunks and per-chunk range proofs).

### 10.4 Noir Primitives

```rust
use std::embedded_curve_ops::{EmbeddedCurvePoint, EmbeddedCurveScalar, multi_scalar_mul};

/// Barretenberg's Pedersen generator at index 0 of the "DEFAULT_DOMAIN_SEPARATOR"
/// domain. Equivalent to `derive_generators("DEFAULT_DOMAIN_SEPARATOR", 0)[0]`.
global G: EmbeddedCurvePoint = EmbeddedCurvePoint {
    x: 0x083e7911d835097629f0067531fc15cafd79a89beecb39903f69572c636f4a5a,
    y: 0x1a7f5efaad7f315c25a918f30cc8d7333fccab7ad7c90f14de81bcc528f9935d,
    is_infinite: false,
};

/// Barretenberg's Pedersen generator at index 1 of the "DEFAULT_DOMAIN_SEPARATOR"
/// domain. Equivalent to `derive_generators("DEFAULT_DOMAIN_SEPARATOR", 0)[1]`.
/// No known discrete-log relation to G (each is the output of Barretenberg's
/// hash-to-curve on the domain separator + index).
global H: EmbeddedCurvePoint = EmbeddedCurvePoint {
    x: 0x054aa86a73cb8a34525e5bbed6e43ba1198e860f5f3950268f71df4591bde402,
    y: 0x209dcfbf2cfb57f9f6046f44d71ac6faf87254afc7407c04eb621a6287cac126,
    is_infinite: false,
};

/// Pedersen commitment, used uniformly for every opening witnessed in any
/// circuit (input or output). Both scalars are encoded as single-limb F_r
/// `Field` values: Poseidon outputs or rejection-sampled CSPRNG draws for fresh
/// blindings, and (for the spend-side input opening of C_spend in W3/T3/S3 and
/// of both balance commitments in CB1/CB2)
/// the canonical F_q reduction of the wallet's post-merge integer blinding,
/// which lies in F_r with probability >= 1 - 2^-127 per merge. The complementary
/// case is acknowledged below in *Post-merge witness availability*.
fn commit(value: Field, randomness: Field) -> EmbeddedCurvePoint {
    multi_scalar_mul(
        [G, H],
        [EmbeddedCurveScalar::from_field(value),
         EmbeddedCurveScalar::from_field(randomness)]
    )
}

/// ECDH shared-secret scalar (Section 2.4): S = scalar * point, then
/// s = Poseidon2(delta_ecdh, S.x, S.y). Absorbing both coordinates binds
/// the full shared point, removing the (P, -P) negation invariance of an
/// x-only extraction. All ECDH scalars in this protocol are F_r-sampled
/// (sk, vk, r_e_point), so single-limb conversion is sound.
fn ecdh(scalar: Field, point: EmbeddedCurvePoint) -> Field {
    let s = multi_scalar_mul(
        [point],
        [EmbeddedCurveScalar::from_field(scalar)]
    );
    poseidon_with_domain(ECDH_SHARED_SECRET, [s.x, s.y])
}
```

**Post-merge witness availability.** Every opening witnessed in any circuit is encoded as a single $$\mathbb{F}\_r$$ `Field` via the same `commit` primitive. After a `Merge` (§7.4), the spendable-balance blinding is $$r\_s + r\_r$$ over $$\mathbb{F}\_q$$. Its canonical $$\mathbb{F}\_q$$ representative lies in $$[0, r)$$ -- representable as a Noir `Field` -- with probability $$\geq 1 - (q - r)/q \approx 1 - 2^{-127}$$ per merge (§2.3). With the complementary probability $$\approx 2^{-127}$$ it lies in $$[r, q)$$. In that case the on-chain state remains well-formed (the commitment is a valid Grumpkin point), but the wallet's local opening witness is unencodable as a `Field`, so no spend / transfer / set-spender / revoke-spender proof can be constructed against the affected $$C\_{\text{spend}}$$ until further accumulation shifts the blinding back into $$\mathbb{F}\_r$$.

**Soft recovery.** Every subsequent inbound confidential transfer or spender transfer, once merged, adds a fresh $$\mathbb{F}\_r$$-derived blinding. For each transfer-derived addend, the new canonical $$\mathbb{F}\_q$$ representative falls in $$[0, r)$$ with probability $$\geq 1 - 2^{-127}$$ regardless of the current stuck value (worst case: the current value sits at the lower edge of $$[r, q)$$, requiring the new $$\mathbb{F}\_r$$ addend to cross the mod $$q$$ boundary; the probability of failing to do so is bounded by $$(q - r)/r \approx 2^{-127}$$). For accounts that continue to receive confidential transfers, the unspendable window is self-resolving at the next merge with overwhelming probability; accounts whose only inflows are deposits remain stuck until a confidential transfer arrives.

### 10.5 Verification Flow

1. Contract reads on-chain state (commitments, public keys)
2. Encodes state as public inputs: Grumpkin point coordinates as 32-byte $$\mathbb{F}\_r$$ values
3. Cross-contract call: `verifier.verify_proof(circuit_type, public_inputs, proof)`
4. Verifier deserializes stored VK, runs UltraHonk verification (BN254 G1/G2 pairings, Fiat-Shamir, sumcheck)
5. Contract applies homomorphic balance updates (Grumpkin point arithmetic via $$\mathbb{F}\_r$$ ops)

### 10.6 Structured Reference String

UltraHonk is a PLONK-family proving system. Its knowledge soundness guarantee depends on a **Structured Reference String (SRS)** -- a sequence of BN254 G1 and G2 points derived from a secret scalar $$\tau$$ (the "toxic waste"):

$$\text{SRS} = \bigl([1]\_1, [\tau]\_1, [\tau^2]\_1, \ldots, [\tau^{N-1}]\_1, \\; [1]\_2, [\tau]\_2\bigr)$$

where $$[x]\_1 = x \cdot G\_1$$ and $$[x]\_2 = x \cdot G\_2$$ are BN254 group elements. The SRS is **universal**: a single SRS supports any circuit up to size $$N$$, and circuit-specific verification keys are derived from it deterministically. The SRS is used during both proof generation (client-side) and verification key derivation (one-time setup).

**Security requirement.** If $$\tau$$ is known to an attacker, they can forge proofs for arbitrary false statements: minting tokens, draining accounts, bypassing all circuit constraints. The knowledge soundness of the entire system reduces to the assumption that $$\tau$$ was destroyed after SRS generation.

**Multi-party ceremony.** The standard mitigation is a multi-party computation (MPC) ceremony in which $$N$$ participants each contribute randomness. The resulting SRS is secure if *at least one* participant honestly destroyed their contribution.

**SRS used in this system.** The Noir/Barretenberg toolchain uses the **Aztec Ignition SRS** by default. Barretenberg downloads the required SRS points from a public transcript on first use. The Ignition ceremony transcript, participant attestations, and verification code are publicly available. The SRS supports circuits up to $$2^{28}$$ gates, well above the expected circuit sizes for this system ($$< 2^{20}$$).

**Deployment considerations.** The verifier contract does not store or reference the full SRS. Circuit-specific verification keys are derived offline from the SRS during circuit compilation and embedded in the verifier contract at deployment. The correctness of these VKs can be independently verified by anyone with access to the circuit source code and the public SRS transcript.

**Risk assessment.** The Ignition ceremony had 176 independent participants across multiple jurisdictions, hardware platforms, and operating systems. Compromise requires collusion of *all* 176 participants.

### 10.7 Dependency: CAP-80

[CAP-80](https://github.com/stellar/stellar-protocol/blob/master/core/cap-0080.md) introduces the host functions required for efficient UltraHonk verification and on-chain Grumpkin point arithmetic. The rollout spans two protocols; both are required, so **protocol 26 is the effective minimum**.

- **Protocol 25:** `bn254_g1_{add, mul}`, `bn254_multi_pairing_check`.
- **Protocol 26:** `bn254_g1_msm`, `bn254_g1_is_on_curve`, `bn254_fr_{add, sub, mul, inv, pow}` -- the $$\mathbb{F}\_r$$ scalar arithmetic underpinning Grumpkin point operations.

### 10.8 On-Chain Point Arithmetic

The contract performs Grumpkin affine point addition and subtraction for homomorphic balance updates. Since Grumpkin coordinates are $$\mathbb{F}\_r^{\text{BN254}}$$ elements, these reduce to Fr field operations.

**Curve coefficients.** Grumpkin $$y^2 = x^3 - 17$$ (Section 2.2) is in short Weierstrass form $$y^2 = x^3 + a x + b$$ with $$a = 0$$ and $$b = -17$$. Only $$a$$ enters the point arithmetic slope formulas below; $$b$$ enters only the on-curve check.

The contract distinguishes the following cases when computing $$P\_3 = P\_1 + P\_2$$:

| Case | Condition | Result |
|:--|:--|:--|
| Left identity | $$P\_1 = \mathcal{O}$$ | $$P\_3 = P\_2$$ |
| Right identity | $$P\_2 = \mathcal{O}$$ | $$P\_3 = P\_1$$ |
| Inverse | $$P\_1, P\_2 \neq \mathcal{O}$$, $$x\_1 = x\_2$$, $$y\_1 = -y\_2 \bmod r$$ | $$P\_3 = \mathcal{O}$$ |
| Doubling | $$P\_1, P\_2 \neq \mathcal{O}$$, $$P\_1 = P\_2$$ (so $$y\_1 \neq 0$$) | slope formula with $$\lambda\_{\text{dbl}}$$ below |
| Generic | $$P\_1, P\_2 \neq \mathcal{O}$$, $$x\_1 \neq x\_2$$ | slope formula with $$\lambda\_{\text{add}}$$ below |

The inverse case must be detected and short-circuited before the generic slope formula, because $$x\_1 - x\_2 = 0$$ would otherwise force a division by zero in $$\mathbb{F}\_r$$.

**Slope.**

$$\lambda\_{\text{add}} = (y\_2 - y\_1)(x\_2 - x\_1)^{-1} \pmod{r}$$

$$\lambda\_{\text{dbl}} = (3 x\_1^2 + a)(2 y\_1)^{-1} = 3 x\_1^2 \cdot (2 y\_1)^{-1} \pmod{r} \qquad (a = 0 \text{ for Grumpkin})$$

**Resulting coordinates.** With $$\lambda$$ selected per the case above:

$$x\_3 = \lambda^2 - x\_1 - x\_2 \pmod{r}$$
$$y\_3 = \lambda (x\_1 - x\_3) - y\_1 \pmod{r}$$

Requires `bn254_fr_{add, sub, mul, inv}` host calls (CAP-80, Section 10.7).

**Point subtraction** $$P\_3 = P\_1 - P\_2$$: if $$P\_2 = \mathcal{O}$$ set $$-P\_2 = \mathcal{O}$$, else $$-P\_2 = (x\_2, -y\_2 \bmod r)$$; then apply the addition cases above. Subtraction of a point from itself yields $$\mathcal{O}$$ via the inverse case, never the doubling branch.

**Point validation.** Grumpkin points enter the system through three boundaries; on-curve and non-identity checks live at the boundary that owns each one. The contract itself performs no per-call on-curve check.

1. **Proof-constrained points (the dominant case).** Every public input that the corresponding circuit also derives via `multi_scalar_mul` is on-curve by construction -- Noir's embedded-curve operations cannot produce an off-curve Grumpkin point. This covers $$Y$$ (R1), $$\text{PVK}$$ (R3), $$R\_e$$ (T6, O6, W_a1, S_a1, V_a1), $$C\_{\text{transfer}}$$ (T8, O8), $$C\_{\text{spend}}'$$ (T11, W6, S10, V7), $$C\_a$$ / $$C\_a'$$ (S7, O11), and the ECDH shared secrets. Non-identity is enforced *in-circuit* by explicit nonzero-scalar constraints: $$sk \neq 0$$ and $$vk \neq 0$$ at registration (R4, R5), and $$r\_e \neq 0$$ in every circuit that produces an ephemeral key (W8, T13, S13, O13, V10). Without these constraints an adversary could publish $$Y = \mathcal{O}$$, $$\text{PVK} = \mathcal{O}$$, or $$R\_e = \mathcal{O}$$ and collapse ECDH (every shared secret becomes $$\mathcal{O}$$, every Poseidon mask becomes a constant function of $$\sigma$$, every ciphertext becomes trivially decryptable).
2. **Points read from prior on-chain state.** $$C\_{\text{spend}}$$, $$C\_{\text{receive}}$$, stored $$Y$$ / $$\text{PVK}$$, and allowance commitments were validated through path (1) when first written. The contract trusts them on subsequent reads.
3. **Auditor keys (the only proof-less entry point).** $$K\_{\text{aud}}$$ is registered in the auditor contract by the auditor itself, with no accompanying proof. The auditor contract performs canonical encoding, on-curve ($$y^2 \equiv x^3 - 17 \pmod{r}$$), and non-identity checks at insertion (Section 3.1); the contract trusts the fetched value.

**Canonical encoding** ($$x, y \in [0, r)$$ as 32-byte representatives) is enforced **by the contract** at the verifier boundary, not by the Soroban host, which reduces non-canonical inputs rather than rejecting them (§2.2 *Host deserialiser caveat*). Every prover-supplied scalar and coordinate that reaches the verifier — and therefore every byte string that gets persisted or emitted downstream — is the unique canonical representative of its field element.

---

## 11. Interface

Based on [EIP-7984](https://eips.ethereum.org/EIPS/eip-7984), adapted for Soroban. The `data: Bytes` parameter carries XDR-encoded proof payloads.

**Canonical encoding.** The `data` payloads are `#[contracttype]` structs and enums declared in the contract crate. Their on-chain byte representation is fixed by Soroban's XDR rules, which are canonical: every value has exactly one valid byte encoding. Named struct fields are serialised as an `ScMap` in declaration order; unnamed fields and tuple-enum variants as an `ScVec` in declaration order; map keys are host-enforced into a canonical sorted form. As a consequence, independent implementations that compile against the same `#[contracttype]` definitions produce byte-identical `data` payloads. The authoritative schemas for the underlying `ScVal`, `ScMap`, `ScVec` live in the [stellar/stellar-xdr](https://github.com/stellar/stellar-xdr) repository; the encoding rules are summarised in the [Stellar XDR documentation](https://developers.stellar.org/docs/learn/fundamentals/data-format/xdr) and the [`#[contracttype]` mapping reference](https://developers.stellar.org/docs/learn/fundamentals/contract-development/types/custom-types).

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

    fn revoke_spender(e: Env, account: Address, spender: Address,
                       data: Bytes);

    fn confidential_balance(e: Env, account: Address) -> ConfidentialAccount;

    fn is_spender(e: Env, account: Address, spender: Address) -> bool;

    fn get_spender_delegation(e: Env, account: Address, spender: Address) -> SpenderDelegation;
}
```

A deployment that enables the optional compliance extension adds `compliance: Option<ComplianceConfig>` to this constructor and the freeze, configuration-rotation, and read entry points of [COMPLIANCE.md](./COMPLIANCE.md) §6 to this interface; the admin-gated ones are authorized by the deployment's own access-control module.

This table is authoritative: every entry is exactly the set of prover-supplied public inputs from the corresponding Section 7 operation (the contract loads the remaining public inputs from trusted state per §7.1), plus the `proof` blob. Names map directly to the Section 7 symbols.

| Operation | `data` contents |
|:---|:---|
| `register` | $$Y$$, $$\text{PVK}$$, `proof` |
| `withdraw` | $$C\_{\text{spend}}'$$, $$\tilde{b}$$, $$R\_e$$, $$\sigma$$, $$\tilde{b}\_{\text{aud,s}}$$, $$\tilde{r}\_{\text{aud,s}}$$, `proof` |
| `confidential_transfer` | $$C\_{\text{spend}}'$$, $$C\_{\text{transfer}}$$, $$R\_e$$, $$\tilde{v}$$, $$\tilde{b}$$, $$\sigma$$, $$\tilde{v}\_{\text{aud,r}}$$, $$\tilde{r}\_{\text{aud,r}}$$, $$\tilde{v}\_{\text{aud,s}}$$, $$\tilde{b}\_{\text{aud,s}}$$, $$\tilde{r}\_{\text{aud,s}}$$, `proof` |
| `confidential_transfer_from` | $$C\_a'$$, $$C\_{\text{transfer}}$$, $$R\_e$$, $$\tilde{v}$$, $$\tilde{a}'$$, $$\sigma\_a'$$, $$\tilde{v}\_{\text{aud,r}}$$, $$\tilde{r}\_{\text{aud,r}}$$, $$\tilde{v}\_{\text{aud,s}}$$, $$\tilde{a}\_{\text{aud,s}}$$, $$\text{dvk\\\_cipher\\\_aud}$$, `proof` |
| `set_spender` | $$C\_{\text{spend}}'$$, $$C\_a$$, $$\text{escrowed\\\_dvk}$$, $$\tilde{b}$$, $$\tilde{a}$$, $$R\_e$$, $$\sigma$$, $$\sigma\_a$$, $$\tilde{v}\_{\text{aud,s}}$$, $$\tilde{b}\_{\text{aud,s}}$$, $$\tilde{r}\_{\text{aud,s}}$$, $$\text{dvk\\\_cipher\\\_aud}$$, `proof` |
| `revoke_spender` | $$C\_{\text{spend}}'$$, $$\tilde{b}$$, $$R\_e$$, $$\sigma$$, $$\tilde{v}\_{\text{aud,s}}$$, $$\tilde{b}\_{\text{aud,s}}$$, `proof` |

For `confidential_transfer_from`, the stored allowance salt $$\sigma\_a$$ is **not** carried in `data`: the contract loads it from the `(from, spender)` delegation entry (§7.8 public-input table). Only the prover-chosen replacement $$\sigma\_a'$$ travels in `data`, gets bound by constraint O10, and is then written back to the delegation entry as the new `allowance_salt` (§6.2). This keeps the trust-boundary rule of §7.1 intact: caller-controlled bytes never overwrite the live $$\sigma\_a$$ used to verify the proof. `set_spender`, by contrast, has no prior delegation entry to load from, so its $$\sigma\_a$$ is prover-supplied and bound by S6.

### 11.1 Authorization Model

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
| `revoke_spender(account, spender, data)` | `account` |
| `confidential_balance`, `is_spender`, `get_spender_delegation` | none (read-only) |

**`register` is single-use.** It reverts if `account` is already registered. Combined with `account.require_auth()`, this prevents a third party from binding attacker-controlled $$(Y, \text{PVK})$$ to `account`'s `ConfidentialAccount` storage entry.

**`set_spender` rejects replacement.** It reverts if a non-revoked delegation already exists for `(account, spender)` -- see §6.2.

**`confidential_transfer_from` is spender-authorized.** The owner's authorization was granted out-of-band at `set_spender` and persists in the on-chain delegation entry until expiry or revocation. The spender's `require_auth()` binds `from`, `to`, and `data`.

### 11.2 Event Schema

Each state-modifying operation emits a structured event. Events carry the data needed for recipient decryption, auditor decryption, and wallet recovery.

| Event | Fields |
|:---|:---|
| `Register` | `account`, `auditor_id` |
| `Deposit` | `from`, `to`, `amount` |
| `Merge` | `account` |
| `Withdraw` | `from`, `to`, `amount`, $$R\_e$$, $$\sigma$$, $$\tilde{b}$$, $$\tilde{b}\_{\text{aud,s}}$$, $$\tilde{r}\_{\text{aud,s}}$$ |
| `Transfer` | `from`, `to`, $$R\_e$$, $$\tilde{v}$$, $$\sigma$$, $$\tilde{b}$$, $$\tilde{v}\_{\text{aud,r}}$$, $$\tilde{r}\_{\text{aud,r}}$$, $$\tilde{v}\_{\text{aud,s}}$$, $$\tilde{b}\_{\text{aud,s}}$$, $$\tilde{r}\_{\text{aud,s}}$$ |
| `SpenderTransfer` | `spender`, `from`, `to`, $$R\_e$$, $$\tilde{v}$$, $$\sigma\_a$$, $$\sigma\_a'$$, $$\tilde{v}\_{\text{aud,r}}$$, $$\tilde{r}\_{\text{aud,r}}$$, $$\tilde{v}\_{\text{aud,s}}$$, $$\tilde{a}\_{\text{aud,s}}$$, $$\text{dvk\\\_cipher\\\_aud}$$ |
| `SetSpender` | `account`, `spender`, `live_until_ledger`, $$R\_e$$, $$\sigma$$, $$\sigma\_a$$, $$\tilde{b}$$, $$\tilde{v}\_{\text{aud,s}}$$, $$\tilde{b}\_{\text{aud,s}}$$, $$\tilde{r}\_{\text{aud,s}}$$, $$\text{dvk\\\_cipher\\\_aud}$$ |
| `RevokeSpender` | `account`, `spender`, $$R\_e$$, $$\sigma$$, $$\tilde{b}$$, $$\tilde{v}\_{\text{aud,s}}$$, $$\tilde{b}\_{\text{aud,s}}$$ |

Amount fields in `Deposit` and `Withdraw` are typed `i128`, matching SEP-41.

**Usage by consumers:**

- **Recipient wallet**: processes `Transfer` and `SpenderTransfer` events using $$(R\_e, \tilde{v}, \sigma)$$ to derive $$v\_{\text{transfer}}$$ and $$r\_{\text{transfer}}$$ (Section 5.3).
- **Owner wallet**: processes all events for recovery (Section 5.2). The $$(\tilde{b}, \sigma)$$ pair from the most recent owner-initiated event forms a checkpoint.
- **Auditor**: processes events containing $$R\_e$$ to compute ECDH shared secrets and decrypt amounts and balance checkpoints (Section 8.1, 8.2).

### 11.3 Read Methods

**`confidential_balance(account) -> ConfidentialAccount`.** Returns the `ConfidentialAccount` struct for the given account (§6.1), i.e. the tuple `(spending_public_key, viewing_public_key, spendable_commitment, receiving_commitment, auditor_id)`. Reverts if `account` is not registered. Wallets bootstrap from this call (single round-trip to obtain both Pedersen commitments plus the keys needed to identify the account and its bound auditor); indexers use it to verify consistency between their replayed accumulators and on-chain state (§5.2 "Consistency check").

**`is_spender(account, spender) -> bool`.** Returns `true` iff a delegation entry exists for `(account, spender)` **and** `ledger.sequence() <= live_until_ledger`. Returns `false` for:

- pairs with no delegation entry,
- pairs whose entry has `ledger.sequence() > live_until_ledger` (expired-but-not-yet-revoked: the spender can no longer spend, while the escrowed value stays on-chain until `revoke_spender` reclaims it, §6.2),
- pairs whose entry was revoked (deleted) by `revoke_spender`.

The function returns the *spending-authority* state, not the *escrow-existence* state. Consumers that need to distinguish "no delegation" from "expired delegation" inspect `get_spender_delegation` (below) or replay `SetSpender` / `RevokeSpender` events.

**`get_spender_delegation(account, spender) -> SpenderDelegation`.** Returns the `SpenderDelegation` struct (§6.2) for the `(account, spender)` pair, i.e. `(allowance_commitment, a_tilde, escrowed_dvk, allowance_salt, live_until_ledger)`. Reverts if no delegation entry exists for the pair. Unlike `is_spender`, this surfaces the raw on-chain delegation state without applying the expiry filter, so callers can separate an absent delegation (revert) from an active one and from an expired-but-not-yet-revoked one, by comparing `live_until_ledger` against `ledger.sequence()` as `is_spender` does. Primary consumers:

- **Spender wallet:** fetches `allowance_commitment`, `a_tilde`, `escrowed_dvk`, and `allowance_salt` to recover $$dvk\_i$$ via §7.11 decryption, then reads the current allowance via $$\tilde{a} = v\_a + \text{Poseidon}(\delta\_{\text{enc\\\_allow}}, dvk\_i, \sigma\_a)$$ to construct the next `confidential_transfer_from` witness.
- **Owner wallet:** reads the same fields after losing local state, or before calling `revoke_spender`, to confirm the on-chain entry matches its records.
- **Indexers:** verify their replayed delegation state against the live commitment, in the same way `confidential_balance` is used for account state (§5.2 "Consistency check").

The auditor's allowance tracking does **not** use this method: per-event allowance ciphertexts (§8.5) are the auditor's data path; `a_tilde` is keyed to $$dvk\_i$$ and is unreadable without it.

---

## 12. Void

---

## 13. Domain Separation Constants

Each $$\delta$$ is a small positive integer in $$\mathbb{F}\_r$$, fixed for the protocol version and used as a Poseidon2 leading-input domain tag. Numeric values are assigned sequentially from 1; the protocol version is implicit in the deployment, and any change to a circuit's constraint that uses these tags requires a new deployment with a fresh verification key (§3.5, §10.6).

| $$\delta$$ | Value | Context |
|:---|:---:|:---|
| $$\delta\_{\text{addr}}$$ | 1 | Soroban Address compression into a single $$\mathbb{F}\_r$$ Field (§2.7) |
| $$\delta\_{\text{vk}}$$ | 2 | Viewing key derivation from spending key and contract address (§4.2) |
| $$\delta\_{\text{dvk}}$$ | 3 | Delegation viewing key derivation (§4.4) |
| $$\delta\_{\text{spend\\\_r}}$$ | 4 | Deterministic randomness for spendable balance commitments (§5.2 *Update rules*) |
| $$\delta\_{\text{transfer\\\_blind}}$$ | 5 | ECDH-derived transfer blinding factor (§5.3 Definition 1) |
| $$\delta\_{\text{transfer\\\_amount}}$$ | 6 | ECDH-derived transfer amount encryption (§5.3 Definition 1) |
| $$\delta\_{\text{enc\\\_bal}}$$ | 7 | Encrypted balance scalar masking (§5.5) |
| $$\delta\_{\text{enc\\\_allow}}$$ | 8 | Encrypted allowance scalar masking (§6.2 *a\_tilde*) |
| $$\delta\_{\text{allow\\\_r}}$$ | 9 | Deterministic randomness for spender allowance commitments (§6.2 *allowance\_commitment*) |
| $$\delta\_{\text{esc\\\_dvk}}$$ | 10 | Delegation key escrow (spender ECDH) (§7.11) |
| $$\delta\_{\text{aud\\\_s}}$$ | 11 | Sender / owner-auditor channel sponge (§2.5, §8.1) |
| $$\delta\_{\text{aud\\\_r}}$$ | 12 | Recipient-auditor channel sponge (§2.5, §8.1) |
| $$\delta\_{\text{ecdh}}$$ | 13 | ECDH shared-secret scalar extraction (§2.4) |
| $$\delta\_{\text{eph}}$$ | 14 | Deterministic ephemeral-scalar derivation (§5.3) |
| $$\delta\_{\text{disc\\\_bind}}$$ | 15 | Disclosure ciphertext to the disclosure recipient, aggregate variant ([SELECTIVE_DISCLOSURE.md](./SELECTIVE_DISCLOSURE.md) §10) |
| $$\delta\_{\text{disc}}$$ | 16 | Disclosure ciphertext to the disclosure recipient ([SELECTIVE_DISCLOSURE.md](./SELECTIVE_DISCLOSURE.md) §4) |
| $$\delta\_{\text{esc\\\_dvk\\\_aud}}$$ | 17 | Delegation key escrow to the owner's auditor (S14, §8.5) |

This table assigns all seventeen values; no other document assigns them. Tags 14–16 are never absorbed inside a core circuit — 14 is derived off-circuit (DESIGN.md §5.3 makes its derivation normative for every operation whose originator holds a viewing key), and 15–16 belong to the off-chain selective-disclosure layer ([SELECTIVE_DISCLOSURE.md](./SELECTIVE_DISCLOSURE.md) §2.2) — so they are not part of the on-chain wire contract; `circuits/lib/src/lib.nr` accordingly implements 1–13 and 17. All seventeen values MUST still be distinct and each MUST be confined to a single sponge mode, so a deployment treats them as one namespace. Tag 17 is assigned out of sequence with its neighbours because it was added after 14–16. It takes its own tag rather than reusing $$\delta\_{\text{esc\\\_dvk}}$$ because the two escrows key off different shared scalars ($$Y\_{\text{op}}$$ vs $$K\_{\text{aud,s}}$$), which is the distinct-domain leg of §5.3's *Why reusing $$r\_e$$ is safe*. It shares its shared scalar with $$\delta\_{\text{aud\\\_s}}$$ instead, and is separated from it by mode exclusivity rather than by a distinct scalar (§8.5).

**Provenance.** Sequential small integers are the simplest assignment that satisfies the requirement of *distinctness* across all Poseidon2 invocations in this protocol -- §3.2 models Poseidon2 as a pseudorandom function, so evaluations whose leading input differs are computationally independent. Distinctness alone is not sufficient: each tag must also be confined to a single sponge mode, since the multi-lane forms of §2.5 share their first lane with the single-output form on the same inputs; $$\delta\_{\text{aud\\\_s}}$$ is the one tag ever squeezed three-wide, and the widths it is read at agree on their shared lanes (§2.5 *Mode exclusivity*). The values themselves carry no semantic meaning; the binding is purely positional and the table is the only authoritative source. Implementations MUST hardcode these exact numeric values.

**Cross-protocol collision.** Future protocols that share Grumpkin / BN254 / Poseidon2 with this protocol -- e.g. an unrelated payments protocol that uses small-integer Poseidon2 domains -- could in principle pick the same numeric values for unrelated purposes. The protocol assumes that the surrounding inputs to Poseidon2 (key material, structural witnesses) sufficiently disambiguate even in such a case; no Poseidon2 invocation in this protocol is keyed solely on a $$\delta$$ value. If stronger isolation is desired, implementers may instead use the alternate scheme $$\delta\_X = \text{Poseidon2}(0, \text{ASCII}(\text{"openzeppelin/confidential-token/v1:X"}))$$, but this is a deployment-time choice that must be applied uniformly and disclosed in the deployment's circuit-binding documentation.
