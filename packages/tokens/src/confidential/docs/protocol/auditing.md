# Auditing

## Per-Transfer Auditor Ciphertexts

Each confidential transfer produces ciphertexts under two auditor keys via ECDH, using the same ephemeral scalar $$r_e$$ used for recipient ECDH. Each auditor channel runs Poseidon2 in sponge mode ([Poseidon2 Hash](primitives.md#poseidon2-hash)), absorbing the channel's domain tag, the ECDH shared scalar, and $$\sigma$$; the recipient channel squeezes two masks, the sender channel three ([Lane assignment](primitives.md#lane-assignment)).

**Recipient's auditor** ($$K_{\text{aud,r}}$$, from the recipient's `auditor_id`) receives the transfer amount and the per-transfer Pedersen randomness:

$$s_{a,r} = \text{ECDH}(r_e, K_{\text{aud,r}}) \qquad \text{(Elliptic Curve Diffie-Hellman)}$$
$$(m_{v,r}, m_{r,r}) = \text{SpongeSqueeze}_2(\delta_{\text{aud\\\_r}}, s_{a,r}, \sigma)$$
$$\tilde{v}_{\text{aud,r}} = v_{\text{transfer}} + m_{v,r}, \qquad \tilde{r}_{\text{aud,r}} = r_{\text{transfer}} + m_{r,r}$$

**Sender's auditor** ($$K_{\text{aud,s}}$$, from the sender's `auditor_id`) receives the transfer amount, the sender's post-transfer balance, and the sender's post-transfer spendable blinding $$r_A'$$:

$$s_{a,s} = \text{ECDH}(r_e, K_{\text{aud,s}}) \qquad \text{(Elliptic Curve Diffie-Hellman)}$$
$$(m_{v,s}, m_{b,s}, m_{r,s}) = \text{SpongeSqueeze}_3(\delta_{\text{aud\\\_s}}, s_{a,s}, \sigma)$$
$$\tilde{v}_{\text{aud,s}} = v_{\text{transfer}} + m_{v,s}, \qquad \tilde{b}_{\text{aud,s}} = (v_A - v_{\text{transfer}}) + m_{b,s}, \qquad \tilde{r}_{\text{aud,s}} = r_A' + m_{r,s}$$

The transfer circuit (constraints T\_a1--T\_a9) enforces correct computation. Which auditor an account is bound to is chosen by the account owner at registration ([Registration](operations/register.md)), subject only to existence in the auditor registry unless the deployment gates the selection in its `Hooks::on_register` implementation ([Restrict Auditor Selection](../compliance.md#restrict-auditor-selection)).

Each auditor decrypts using their secret key $$k$$. For example, the sender's auditor:

$$S_{a,s} = k \cdot R_e, \qquad s_{a,s} = \text{Poseidon}(\delta_{\text{ecdh}}, S_{a,s}.x, S_{a,s}.y)$$
$$(m_{v,s}, m_{b,s}, m_{r,s}) = \text{SpongeSqueeze}_3(\delta_{\text{aud\\\_s}}, s_{a,s}, \sigma)$$
$$v_{\text{transfer}} = \tilde{v}_{\text{aud,s}} - m_{v,s}, \qquad v_{\text{new}} = \tilde{b}_{\text{aud,s}} - m_{b,s}, \qquad r_{\text{new}} = \tilde{r}_{\text{aud,s}} - m_{r,s}$$

where $$R_e$$ and $$\sigma$$ are published in the Transfer event. The recipient's auditor follows the same pattern with $$\delta_{\text{aud\\\_r}}$$ to recover the pair $$(v_{\text{transfer}}, r_{\text{transfer}})$$.

### Recipient-auditor opening capability

Because the recipient-auditor recovers $$r_{\text{transfer}}$$ for every inbound transfer, and because deposits add to `receiving_commitment` with $$r = 0$$ ([Deposit](operations/deposit.md)), the recipient-auditor can reconstruct the full Pedersen opening of $$C_{\text{receive}}$$ between merges:

$$v_r = \sum_i v_{\text{transfer},i} + \sum_j a_j, \qquad r_r = \sum_i r_{\text{transfer},i}$$

where $$i$$ ranges over inbound transfers and spender-transfers since the last merge and $$j$$ ranges over deposits. This is a full Pedersen *opening* of $$C_{\text{receive}}$$: both the value and the blinding are reconstructed by the auditor.

The capability is bounded in three ways:

- **Forward-only.** Only events emitted while the auditor key was active are decryptable.
- **Recipient-channel only.** The reconstruction above yields an opening of `receiving_commitment`: the spend-side blinding $$r_s = \text{Poseidon}(\delta_{\text{spend\\\_r}}, vk_A, \sigma)$$ depends on $$vk_A$$ and is not derivable from the recipient channel.
- **Reset by merge.** Merge folds $$r_r$$ into the spendable-balance randomness ($$r_{\text{spend}}' = r_s + r_r$$, [Merge](operations/merge.md)) and emits no checkpoint, so the reconstruction above restarts from the next inbound flow. A clawback resets it the same way ([Wallet and Auditor Consequences](../compliance.md#wallet-and-auditor-consequences)).

### Sender-auditor opening capability

The `lane[2]` escrow hands the sender-auditor the *blinding* of the account's post-operation spendable balance directly, without $$vk_A$$: together with the value in $$\tilde{b}_{\text{aud,s}}$$ it is a full Pedersen opening of $$C_{\text{spend}}'$$. It is available at the three checkpoint operations that escrow the spendable blinding -- withdrawal (W\_a5), outgoing transfer (T\_a9), and `set_spender` (S\_a6) -- and is likewise bounded:

- **Forward-only**, on the same grounds as the recipient side.
- **Maintained across merges.** An account binds a single `auditor_id` ([Account Data Model](account-state.md#account-data-model)), so the key that decrypts the `lane[2]` escrow is the same key that decrypts the recipient channel of every inbound flow to that account. Merge adds both the values and the blindings ($$v_{\text{spend}}' = v_s + v_r$$, $$r_{\text{spend}}' = r_s + r_r$$, [Merge](operations/merge.md)). An auditor that decrypted the inbound flows since the previous merge holds each addend -- $$(v_{\text{transfer},i}, r_{\text{transfer},i})$$ from the recipient-channel reconstruction above, and $$(a_j, 0)$$ from the public deposits -- and carries the escrowed opening forward by the same addition the contract performs.
- **Carried through `revoke_spender`.** The revoke is a public homomorphic fold, $$C_{\text{spend}}' = C_{\text{spend}} + C_a$$, emitting no new checkpoint ([Revoke Spender](operations/revoke-spender.md)). An auditor that decrypted the delegation's last state-changing event holds the opening of the $$C_a$$ being folded ([Spender Allowance Auditing](#spender-allowance-auditing)) and advances its own opening by the same addition the contract performs.
- **Rotation-scoped.** Both folds above depend on the auditor having decrypted the event that produced the addend; [Decryption capability across rotation](#decryption-capability-across-rotation) states what a rotated key can and cannot open.

The clawback flow specified in [Clawback](../compliance.md#clawback) draws its witness from these bounded openings.

## Auditor Visibility Properties

Per channel and lane, the auditor ciphertexts carry:

| Channel | `lane[0]` | `lane[1]` | `lane[2]` |
|:--|:--|:--|:--|
| Sender / owner ($$\delta_{\text{aud\\\_s}}$$) | Transfer amount, or the escrowed amount for `SetSpender`; unused on `Withdraw`, whose amount is public | Sender's post-operation balance, or post-operation allowance for a spender transfer | Post-operation spendable blinding on `Withdraw`, `Transfer`, and `SetSpender`; post-transfer allowance blinding $$r_a'$$ on `SpenderTransfer` |
| Recipient ($$\delta_{\text{aud\\\_r}}$$) | Transfer amount | Per-transfer Pedersen randomness $$r_{\text{transfer}}$$ | — (channel is two-lane) |

**Transfer amounts.** Both auditors see the transfer amount in real time. The recipient's auditor decrypts $$v_{\text{transfer}}$$ from $$\tilde{v}_{\text{aud,r}}$$; the sender's auditor decrypts it from $$\tilde{v}_{\text{aud,s}}$$.

**Balance checkpoints.** The sender's auditor receives an encrypted balance checkpoint at every owner-initiated operation that produces a proof:

- **Outgoing transfer**: auditor decrypts post-transfer balance $$(v_A - v_{\text{transfer}})$$ from $$\tilde{b}_{\text{aud,s}}$$ and the matching blinding $$r_A'$$ from $$\tilde{r}_{\text{aud,s}}$$ (constraints T\_a5--T\_a9).
- **Withdrawal**: auditor decrypts post-withdrawal balance $$(v - a)$$ from $$\tilde{b}_{\text{aud,s}}$$ and the matching blinding $$r'$$ from $$\tilde{r}_{\text{aud,s}}$$ (constraints W\_a1--W\_a5). The withdrawal amount $$a$$ is also visible as a public input.
- **Set spender**: auditor decrypts escrowed amount $$v_a$$ from $$\tilde{v}_{\text{aud,s}}$$, post-escrow balance $$(v - v_a)$$ from $$\tilde{b}_{\text{aud,s}}$$, and the matching blinding $$r'$$ from $$\tilde{r}_{\text{aud,s}}$$ (constraints S\_a1--S\_a6).

The recipient's auditor does not see the sender's balance in any of these operations.

**Per-transfer Pedersen randomness (recipient-auditor).** Beyond the transfer amount, the recipient's auditor also decrypts the per-transfer Pedersen blinding $$r_{\text{transfer}}$$ from $$\tilde{r}_{\text{aud,r}}$$ on every confidential transfer and spender-transfer. The sender's auditor does not see $$r_{\text{transfer}}$$.

**Key rotation.** When the auditor contract sets a new key under the account's `auditor_id` ([Auditor Key Management and Rotation](#auditor-key-management-and-rotation)), the new key reads the balance checkpoint at the next owner-initiated operation with no event replay: the checkpoint is self-contained, depending only on the auditor's ECDH secret key and the published $$(R_e, \sigma)$$. Reading one checkpoint is not the same as holding the account's accumulated opening; [Auditor Key Management and Rotation](#auditor-key-management-and-rotation) states what a rotated key can open. Note that `auditor_id` itself is immutable per account ([Account Data Model](account-state.md#account-data-model)); only the key under that `auditor_id` rotates.

## Auditor Key Management and Rotation

The auditor contract stores Grumpkin public keys as full affine points $$(x, y)$$ indexed by `auditor_id`. The contract validates that every inserted key is canonical, on-curve ($$y^2 \equiv x^3 - 17 \pmod{r}$$), and non-identity at insertion time ([Components](system-model.md#components), [On-Chain Point Arithmetic](proof-system.md#on-chain-point-arithmetic)). Each `auditor_id` MAY maintain a sequence of versions, each carrying its activation ledger, in which case rotation appends a new entry rather than overwriting the previous one. The reference registry shipped in this repository takes the simpler form: it keeps a single current key per `auditor_id`, which `rotate_key` overwrites in place; a versioned, activation-ledger registry is an optional production target.

When building public inputs for any operation that produces auditor ciphertexts (transfers, withdrawals, `set_spender`), the contract fetches the relevant auditor keys for the recipient's and/or sender's `auditor_id`. The contract passes the full Grumpkin point as a public input; the circuit constrains the ECDH ciphertexts against that exact point. The contract and the circuit are version-agnostic: they verify against whichever key the auditor contract currently exposes.

### In-flight proofs across rotation

A proof constructed against version $$v$$ becomes unverifiable the instant the auditor contract activates version $$v+1$$. The $$K_{\text{aud}}$$ public input the contract fetches at verification no longer matches the value the prover committed to, so UltraHonk verification fails and the invocation **reverts at the proof-verification boundary**. The caller (sender, owner, or spender) reconstructs the proof against the new $$K_{\text{aud}}$$ and resubmits. The rejection is benign: the contract's spendable balance, receiving balance, and delegation state are unchanged by the reverted call, the operation's salt is freshly sampled on retry ([Revert Safety](security.md#revert-safety)), and an observer cannot correlate the rejected attempt with the resubmission.

### Auditor's off-chain obligation

The auditor MUST retain the secret key for every historical version it has issued. To decrypt an event at ledger $$L$$, the auditor resolves the version from its own rotation records (with a versioned activation-ledger registry, the auditor instead queries the auditor contract for the version of its `auditor_id` whose activation ledger is the largest value not exceeding $$L$$), then uses the corresponding off-chain secret key against the $$R_e$$ and $$\sigma$$ (or $$\sigma_a'$$) emitted in the event.

### Decryption capability across rotation

A key decrypts an auditor ciphertext exactly when it was the active key at the moment that ciphertext was produced. Rotation therefore affects the two holders asymmetrically:

- **Retired key.** Its holder keeps what it already opened. Rotation does not revoke a held opening, and the proofless folds advance it exactly as [Per-Transfer Auditor Ciphertexts](#per-transfer-auditor-ciphertexts) specifies.
- **New key.** Its holder starts with nothing and re-anchors per commitment: at the next checkpoint operation for $$C_{\text{spend}}$$ ([Auditor Visibility Properties](#auditor-visibility-properties)), and at the delegation's next state-changing operation for $$C_a$$ ([Spender Allowance Auditing](#spender-allowance-auditing)).

Each proofless fold consumes an addend produced by an earlier event, and a holder that bootstrapped from the new key alone cannot open an addend that predates the rotation. Such a holder loses its opening at every fold that consumes one, and re-anchors only at the next checkpoint operation following it. A merge's addends reach back only as far as the previous merge; a delegation's last state change is bounded only by its entry TTL.

## Spender Transfer Auditing

Each spender transfer produces auditor ciphertexts under two keys (constraints O\_a1--O\_a9), following the same dual-auditor sponge model as owner transfers. The recipient's auditor decrypts the transfer amount and the per-transfer Pedersen randomness:

$$(m_{v,r}, m_{r,r}) = \text{SpongeSqueeze}_2(\delta_{\text{aud\\\_r}}, s_{a,r}, \sigma_a')$$
$$v_{\text{transfer}} = \tilde{v}_{\text{aud,r}} - m_{v,r}, \qquad r_{\text{transfer}} = \tilde{r}_{\text{aud,r}} - m_{r,r}$$

The owner's auditor decrypts the transfer amount, the post-transfer allowance, and the *new* allowance blinding $$r_a'$$, the one O11 commits $$C_a'$$ under (O\_a9):

$$(m_{v,s}, m_{a,s}, m_{r,s}) = \text{SpongeSqueeze}_3(\delta_{\text{aud\\\_s}}, s_{a,s}, \sigma_a')$$
$$v_{\text{transfer}} = \tilde{v}_{\text{aud,s}} - m_{v,s}, \qquad v_a' = \tilde{a}_{\text{aud,s}} - m_{a,s}, \qquad r_a' = \tilde{r}_{\text{aud,s}} - m_{r,s}$$

where $$s_{a,r}$$, $$s_{a,s}$$, and $$\sigma_a'$$ are recovered from the event as in [Per-Transfer Auditor Ciphertexts](#per-transfer-auditor-ciphertexts). The recipient-auditor opening capability stated in [Per-Transfer Auditor Ciphertexts](#per-transfer-auditor-ciphertexts) extends to spender-transfer inbound flows: $$r_{\text{transfer}}$$ from spender-transfers contributes to $$r_r$$ in $$C_{\text{receive}}$$ identically to owner-transfer inbound flows.

$$(\tilde{a}_{\text{aud,s}}, \tilde{r}_{\text{aud,s}})$$ open the post-transfer $$C_a'$$, the state left on-chain.

## Spender Allowance Auditing

The auditor tracks each allowance's current value from the per-event ciphertexts of its two state-changing operations, `set_spender` ([Auditor Visibility Properties](#auditor-visibility-properties)) and `confidential_transfer_from` ([Spender Transfer Auditing](#spender-transfer-auditing)).

### Allowance opening

The owner's auditor also receives the *blinding* of the allowance commitment each event writes: $$r_a$$ under $$\delta_{\text{esc\\\_allow\\\_r\\\_aud}}$$ at `set_spender` (S14, below) and $$r_a'$$ in `lane[2]` at every spender transfer (O\_a9, [Spender Transfer Auditing](#spender-transfer-auditing)). Paired with the value that event already publishes -- $$\tilde{v}_{\text{aud,s}}$$ on `SetSpender`, $$\tilde{a}_{\text{aud,s}}$$ on `SpenderTransfer` -- that is the full Pedersen opening of the $$C_a$$ left on-chain, reconstructed from the event alone with no storage read. Unlike the spendable side there is no merge to fold in, since a delegation's only state transitions are the events themselves. This is what `revoke_spender` folds against: it publishes no ciphertext of its own ([Revoke Spender](operations/revoke-spender.md)), and the auditor opens the $$C_a$$ from whichever event last wrote it.

### Auditor-side allowance-blinding escrow

At `set_spender` the owner escrows $$r_a$$ to its own auditor. A blinding is per state, so one leaked escrow ciphertext opens one allowance state.

$$\tilde{r}_{a,\text{aud,s}} = \text{Poseidon}(\delta_{\text{esc\\\_allow\\\_r\\\_aud}}, s_{a,s}, \text{op}_i) + r_a$$

reusing the S\_a2 shared scalar $$s_{a,s} = \text{ECDH}(r_e, K_{\text{aud,s}})$$ rather than opening a new ECDH channel; [Circuit Cost Analysis](proof-system.md#circuit-cost-analysis) gives the cost. The auditor recovers $$s_{a,s}$$ from $$k_{\text{aud,s}}$$ and the event's $$R_e$$, and $$\text{op}_i$$ from the event's `spender` topic, then subtracts. Because the escrowed value is pinned to S6, the opening the auditor holds necessarily matches the $$C_a$$ the same proof wrote.

It is a single-output pad because `lane[2]` of this channel is taken by the spendable blinding (S\_a6). Distinct tags separate the two despite the shared scalar, each confined to one sponge mode as [Mode exclusivity](primitives.md#mode-exclusivity) requires. Its pad freshness is that of $$r_e$$ ([Why reusing $$r_e$$ is safe](keys-and-commitments.md#why-reusing-r_e-is-safe)).

### Archive dependence

The auditor must have *observed* the event: the escrowed blinding lives nowhere in contract storage, so an auditor that missed a delegation event cannot recover that opening from `a_tilde` and $$\sigma_a$$ the way a $$dvk_i$$ holder could. [Wallet State and Recovery](wallet-state.md) already makes a durable event archive normative; [Auditor recovery](../indexer.md#auditor-recovery) states the operational consequences for auditor clients.

---

Previous: [Revoke Spender](operations/revoke-spender.md) · Up: [Index](../README.md#protocol) · Next: [Security Analysis](security.md)
