# Auditing

## Per-Transfer Auditor Ciphertexts

Each confidential transfer produces ciphertexts under two auditor keys via ECDH, using the same ephemeral scalar $$r\_e$$ used for recipient ECDH. Each auditor channel runs Poseidon2 in sponge mode (Section 2.5), absorbing the channel's domain tag, the ECDH shared scalar, and $$\sigma$$; the recipient channel squeezes two masks, the sender channel three (Section 2.5 *Lane assignment*).

**Recipient's auditor** ($$K\_{\text{aud,r}}$$, from the recipient's `auditor_id`) receives the transfer amount and the per-transfer Pedersen randomness:

$$s\_{a,r} = \text{ECDH}(r\_e, K\_{\text{aud,r}}) \qquad \text{(DESIGN §2.4)}$$
$$(m\_{v,r}, m\_{r,r}) = \text{SpongeSqueeze}\_2(\delta\_{\text{aud\\\_r}}, s\_{a,r}, \sigma)$$
$$\tilde{v}\_{\text{aud,r}} = v\_{\text{transfer}} + m\_{v,r}, \qquad \tilde{r}\_{\text{aud,r}} = r\_{\text{transfer}} + m\_{r,r}$$

**Sender's auditor** ($$K\_{\text{aud,s}}$$, from the sender's `auditor_id`) receives the transfer amount, the sender's post-transfer balance, and the sender's post-transfer spendable blinding $$r\_A'$$:

$$s\_{a,s} = \text{ECDH}(r\_e, K\_{\text{aud,s}}) \qquad \text{(DESIGN §2.4)}$$
$$(m\_{v,s}, m\_{b,s}, m\_{r,s}) = \text{SpongeSqueeze}\_3(\delta\_{\text{aud\\\_s}}, s\_{a,s}, \sigma)$$
$$\tilde{v}\_{\text{aud,s}} = v\_{\text{transfer}} + m\_{v,s}, \qquad \tilde{b}\_{\text{aud,s}} = (v\_A - v\_{\text{transfer}}) + m\_{b,s}, \qquad \tilde{r}\_{\text{aud,s}} = r\_A' + m\_{r,s}$$

The transfer circuit (constraints T\_a1--T\_a9) enforces correct computation. Which auditor an account is bound to is chosen by the account owner at registration (DESIGN §7.2), subject only to existence in the auditor registry unless the deployment gates the selection in its `Hooks::on_register` implementation ([COMPLIANCE.md](./COMPLIANCE.md) §4.3).

Each auditor decrypts using their secret key $$k$$. For example, the sender's auditor:

$$S\_{a,s} = k \cdot R\_e, \qquad s\_{a,s} = \text{Poseidon}(\delta\_{\text{ecdh}}, S\_{a,s}.x, S\_{a,s}.y)$$
$$(m\_{v,s}, m\_{b,s}, m\_{r,s}) = \text{SpongeSqueeze}\_3(\delta\_{\text{aud\\\_s}}, s\_{a,s}, \sigma)$$
$$v\_{\text{transfer}} = \tilde{v}\_{\text{aud,s}} - m\_{v,s}, \qquad v\_{\text{new}} = \tilde{b}\_{\text{aud,s}} - m\_{b,s}, \qquad r\_{\text{new}} = \tilde{r}\_{\text{aud,s}} - m\_{r,s}$$

where $$R\_e$$ and $$\sigma$$ are published in the Transfer event. The recipient's auditor follows the same pattern with $$\delta\_{\text{aud\\\_r}}$$ to recover the pair $$(v\_{\text{transfer}}, r\_{\text{transfer}})$$.

### Recipient-auditor opening capability

Because the recipient-auditor recovers $$r\_{\text{transfer}}$$ for every inbound transfer, and because deposits add to `receiving_commitment` with $$r = 0$$ (Section 7.3), the recipient-auditor can reconstruct the full Pedersen opening of $$C\_{\text{receive}}$$ between merges:

$$v\_r = \sum\_i v\_{\text{transfer},i} + \sum\_j a\_j, \qquad r\_r = \sum\_i r\_{\text{transfer},i}$$

where $$i$$ ranges over inbound transfers and spender-transfers since the last merge and $$j$$ ranges over deposits. This is a full Pedersen *opening* of $$C\_{\text{receive}}$$: both the value and the blinding are reconstructed by the auditor.

The capability is bounded in three ways:

- **Forward-only.** Only events emitted while the auditor key was active are decryptable.
- **Recipient-channel only.** The reconstruction above yields an opening of `receiving_commitment`: the spend-side blinding $$r\_s = \text{Poseidon}(\delta\_{\text{spend\\\_r}}, vk\_A, \sigma)$$ depends on $$vk\_A$$ and is not derivable from the recipient channel.
- **Reset by merge.** Merge folds $$r\_r$$ into the spendable-balance randomness ($$r\_{\text{spend}}' = r\_s + r\_r$$, Section 7.4) and emits no checkpoint, so the reconstruction above restarts from the next inbound flow. A clawback resets it the same way ([COMPLIANCE.md](./COMPLIANCE.md) §5.7).

### Sender-auditor opening capability

The `lane[2]` escrow hands the sender-auditor the *blinding* of the account's post-operation spendable balance directly, without $$vk\_A$$: together with the value in $$\tilde{b}\_{\text{aud,s}}$$ it is a full Pedersen opening of $$C\_{\text{spend}}'$$. It is available at the three checkpoint operations that escrow the spendable blinding -- withdrawal (W\_a5), outgoing transfer (T\_a9), and `set_spender` (S\_a6) -- and is likewise bounded:

- **Forward-only**, on the same grounds as the recipient side.
- **Maintained across merges.** An account binds a single `auditor_id` (Section 6.1), so the key that decrypts the `lane[2]` escrow is the same key that decrypts the recipient channel of every inbound flow to that account. Merge adds both the values and the blindings ($$v\_{\text{spend}}' = v\_s + v\_r$$, $$r\_{\text{spend}}' = r\_s + r\_r$$, Section 7.4). An auditor that decrypted the inbound flows since the previous merge holds each addend -- $$(v\_{\text{transfer},i}, r\_{\text{transfer},i})$$ from the recipient-channel reconstruction above, and $$(a\_j, 0)$$ from the public deposits -- and carries the escrowed opening forward by the same addition the contract performs.
- **Carried through `revoke_spender`.** The revoke is a public homomorphic fold, $$C\_{\text{spend}}' = C\_{\text{spend}} + C\_a$$, emitting no new checkpoint (Section 7.9). An auditor that decrypted the delegation's last state-changing event holds the opening of the $$C\_a$$ being folded (Section 8.5) and advances its own opening by the same addition the contract performs.
- **Rotation-scoped.** Both folds above depend on the auditor having decrypted the event that produced the addend; Section 8.3 *Decryption capability across rotation* states what a rotated key can and cannot open.

The clawback flow specified in [COMPLIANCE.md](./COMPLIANCE.md) §5 draws its witness from these bounded openings.

## Auditor Visibility Properties

**Transfer amounts.** Both auditors see the transfer amount in real time. The recipient's auditor decrypts $$v\_{\text{transfer}}$$ from $$\tilde{v}\_{\text{aud,r}}$$; the sender's auditor decrypts it from $$\tilde{v}\_{\text{aud,s}}$$.

**Balance checkpoints.** The sender's auditor receives an encrypted balance checkpoint at every owner-initiated operation that produces a proof:

- **Outgoing transfer**: auditor decrypts post-transfer balance $$(v\_A - v\_{\text{transfer}})$$ from $$\tilde{b}\_{\text{aud,s}}$$ and the matching blinding $$r\_A'$$ from $$\tilde{r}\_{\text{aud,s}}$$ (constraints T\_a5--T\_a9).
- **Withdrawal**: auditor decrypts post-withdrawal balance $$(v - a)$$ from $$\tilde{b}\_{\text{aud,s}}$$ and the matching blinding $$r'$$ from $$\tilde{r}\_{\text{aud,s}}$$ (constraints W\_a1--W\_a5). The withdrawal amount $$a$$ is also visible as a public input.
- **Set spender**: auditor decrypts escrowed amount $$v\_a$$ from $$\tilde{v}\_{\text{aud,s}}$$, post-escrow balance $$(v - v\_a)$$ from $$\tilde{b}\_{\text{aud,s}}$$, and the matching blinding $$r'$$ from $$\tilde{r}\_{\text{aud,s}}$$ (constraints S\_a1--S\_a6).

The recipient's auditor does not see the sender's balance in any of these operations.

**Per-transfer Pedersen randomness (recipient-auditor).** Beyond the transfer amount, the recipient's auditor also decrypts the per-transfer Pedersen blinding $$r\_{\text{transfer}}$$ from $$\tilde{r}\_{\text{aud,r}}$$ on every confidential transfer and spender-transfer. The sender's auditor does not see $$r\_{\text{transfer}}$$.

**Key rotation.** When the auditor contract sets a new key under the account's `auditor_id` (§8.3), the new key reads the balance checkpoint at the next owner-initiated operation with no event replay: the checkpoint is self-contained, depending only on the auditor's ECDH secret key and the published $$(R\_e, \sigma)$$. Reading one checkpoint is not the same as holding the account's accumulated opening; Section 8.3 states what a rotated key can open. Note that `auditor_id` itself is immutable per account (§6.1); only the key under that `auditor_id` rotates.

## Auditor Key Management and Rotation

The auditor contract stores Grumpkin public keys as full affine points $$(x, y)$$ indexed by `auditor_id`. The contract validates that every inserted key is canonical, on-curve ($$y^2 \equiv x^3 - 17 \pmod{r}$$), and non-identity at insertion time (Section 3.1, Section 10.8). Each `auditor_id` MAY maintain a sequence of versions, each carrying its activation ledger, in which case rotation appends a new entry rather than overwriting the previous one. The reference registry shipped in this repository takes the simpler form: it keeps a single current key per `auditor_id`, which `rotate_key` overwrites in place; a versioned, activation-ledger registry is an optional production target.

When building public inputs for any operation that produces auditor ciphertexts (transfers, withdrawals, `set_spender`), the contract fetches the relevant auditor keys for the recipient's and/or sender's `auditor_id`. The contract passes the full Grumpkin point as a public input; the circuit constrains the ECDH ciphertexts against that exact point. The contract and the circuit are version-agnostic: they verify against whichever key the auditor contract currently exposes.

### In-flight proofs across rotation

A proof constructed against version $$v$$ becomes unverifiable the instant the auditor contract activates version $$v+1$$. The $$K\_{\text{aud}}$$ public input the contract fetches at verification no longer matches the value the prover committed to, so UltraHonk verification fails and the invocation **reverts at the proof-verification boundary**. The caller (sender, owner, or spender) reconstructs the proof against the new $$K\_{\text{aud}}$$ and resubmits. The rejection is benign: the contract's spendable balance, receiving balance, and delegation state are unchanged by the reverted call, the operation's salt is freshly sampled on retry (Section 9.6), and an observer cannot correlate the rejected attempt with the resubmission.

### Auditor's off-chain obligation

The auditor MUST retain the secret key for every historical version it has issued. To decrypt an event at ledger $$L$$, the auditor resolves the version from its own rotation records (with a versioned activation-ledger registry, the auditor instead queries the auditor contract for the version of its `auditor_id` whose activation ledger is the largest value not exceeding $$L$$), then uses the corresponding off-chain secret key against the $$R\_e$$ and $$\sigma$$ (or $$\sigma\_a'$$) emitted in the event.

### Decryption capability across rotation

A key decrypts an auditor ciphertext exactly when it was the active key at the moment that ciphertext was produced. Rotation therefore affects the two holders asymmetrically:

- **Retired key.** Its holder keeps what it already opened. Rotation does not revoke a held opening, and the proofless folds advance it exactly as Section 8.1 specifies.
- **New key.** Its holder starts with nothing and re-anchors per commitment: at the next checkpoint operation for $$C\_{\text{spend}}$$ (Section 8.2), and at the delegation's next state-changing operation for $$C\_a$$ (Section 8.5).

Each proofless fold consumes an addend produced by an earlier event, and a holder that bootstrapped from the new key alone cannot open an addend that predates the rotation. Such a holder loses its opening at every fold that consumes one, and re-anchors only at the next checkpoint operation following it. A merge's addends reach back only as far as the previous merge; a delegation's last state change is bounded only by its entry TTL.

## Spender Transfer Auditing

Each spender transfer produces auditor ciphertexts under two keys (constraints O\_a1--O\_a9), following the same dual-auditor sponge model as owner transfers. The recipient's auditor decrypts the transfer amount and the per-transfer Pedersen randomness:

$$(m\_{v,r}, m\_{r,r}) = \text{SpongeSqueeze}\_2(\delta\_{\text{aud\\\_r}}, s\_{a,r}, \sigma\_a')$$
$$v\_{\text{transfer}} = \tilde{v}\_{\text{aud,r}} - m\_{v,r}, \qquad r\_{\text{transfer}} = \tilde{r}\_{\text{aud,r}} - m\_{r,r}$$

The owner's auditor decrypts the transfer amount, the post-transfer allowance, and the *new* allowance blinding $$r\_a'$$, the one O11 commits $$C\_a'$$ under (O\_a9):

$$(m\_{v,s}, m\_{a,s}, m\_{r,s}) = \text{SpongeSqueeze}\_3(\delta\_{\text{aud\\\_s}}, s\_{a,s}, \sigma\_a')$$
$$v\_{\text{transfer}} = \tilde{v}\_{\text{aud,s}} - m\_{v,s}, \qquad v\_a' = \tilde{a}\_{\text{aud,s}} - m\_{a,s}, \qquad r\_a' = \tilde{r}\_{\text{aud,s}} - m\_{r,s}$$

where $$s\_{a,r}$$, $$s\_{a,s}$$, and $$\sigma\_a'$$ are recovered from the event as in Section 8.1. The recipient-auditor opening capability stated in Section 8.1 extends to spender-transfer inbound flows: $$r\_{\text{transfer}}$$ from spender-transfers contributes to $$r\_r$$ in $$C\_{\text{receive}}$$ identically to owner-transfer inbound flows.

$$(\tilde{a}\_{\text{aud,s}}, \tilde{r}\_{\text{aud,s}})$$ open the post-transfer $$C\_a'$$, the state left on-chain.

## Spender Allowance Auditing

The auditor tracks each allowance's current value from the per-event ciphertexts of its two state-changing operations, `set_spender` (Section 8.2) and `confidential_transfer_from` (Section 8.4).

### Allowance opening

The owner's auditor also receives the *blinding* of the allowance commitment each event writes: $$r\_a$$ under $$\delta\_{\text{esc\\\_allow\\\_r\\\_aud}}$$ at `set_spender` (S14, below) and $$r\_a'$$ in `lane[2]` at every spender transfer (O\_a9, §8.4). Paired with the value that event already publishes -- $$\tilde{v}\_{\text{aud,s}}$$ on `SetSpender`, $$\tilde{a}\_{\text{aud,s}}$$ on `SpenderTransfer` -- that is the full Pedersen opening of the $$C\_a$$ left on-chain, reconstructed from the event alone with no storage read. Unlike the spendable side there is no merge to fold in, since a delegation's only state transitions are the events themselves. This is what `revoke_spender` folds against: it publishes no ciphertext of its own (§7.9), and the auditor opens the $$C\_a$$ from whichever event last wrote it.

### Auditor-side allowance-blinding escrow

At `set_spender` the owner escrows $$r\_a$$ to its own auditor. A blinding is per state, so one leaked escrow ciphertext opens one allowance state.

$$\tilde{r}\_{a,\text{aud,s}} = \text{Poseidon}(\delta\_{\text{esc\\\_allow\\\_r\\\_aud}}, s\_{a,s}, \text{op}\_i) + r\_a$$

reusing the S\_a2 shared scalar $$s\_{a,s} = \text{ECDH}(r\_e, K\_{\text{aud,s}})$$ rather than opening a new ECDH channel; §10.3 gives the cost. The auditor recovers $$s\_{a,s}$$ from $$k\_{\text{aud,s}}$$ and the event's $$R\_e$$, and $$\text{op}\_i$$ from the event's `spender` topic, then subtracts. Because the escrowed value is pinned to S6, the opening the auditor holds necessarily matches the $$C\_a$$ the same proof wrote.

It is a single-output pad because `lane[2]` of this channel is taken by the spendable blinding (S\_a6). Distinct tags separate the two despite the shared scalar, each confined to one sponge mode as DESIGN §2.5 *Mode exclusivity* requires. Its pad freshness is that of $$r\_e$$ (DESIGN §5.3 *Why reusing $$r\_e$$ is safe*).

### Archive dependence

The auditor must have *observed* the event: the escrowed blinding lives nowhere in contract storage, so an auditor that missed a delegation event cannot recover that opening from `a_tilde` and $$\sigma\_a$$ the way a $$dvk\_i$$ holder could. DESIGN.md §5.2 already makes a durable event archive normative; [INDEXER.md](./INDEXER.md) §7.1 states the operational consequences for auditor clients.
