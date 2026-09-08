# Circuit D-auditor: Auditor Discloses a Transfer

The auditor proves to a third party that an on-chain event corresponds to a transfer of amount $$v_{\text{transfer}}$$ for one of the accounts under the auditor's scope. Used when the holder is uncooperative or when the disclosure recipient requires a guarantee that the auditor (not just the holder) has attested.

**Which auditor.** Every transfer carries ciphertexts under *two* auditor keys ([Per-Transfer Auditor Ciphertexts](../../protocol/auditing.md#per-transfer-auditor-ciphertexts)): the recipient-side key $$K_{\text{aud,r}}$$ (channel $$\delta_{\text{aud\\\_r}}$$, two lanes yielding masks for $$v_{\text{transfer}}$$ and $$r_{\text{transfer}}$$) and the sender-side key $$K_{\text{aud,s}}$$ (channel $$\delta_{\text{aud\\\_s}}$$, three lanes yielding masks for $$v_{\text{transfer}}$$, the sender's post-transfer balance, and the blinding-escrow slot of [Poseidon2 Hash](../../protocol/primitives.md#poseidon2-hash)). Whichever auditor is disclosing reuses the same shared-secret derivation they perform to read events natively; the circuit additionally encrypts the result to the disclosure recipient.

The constraints below parameterize the channel as $$\delta_{\text{aud}} \in \\{\delta_{\text{aud\\\_r}}, \delta_{\text{aud\\\_s}}\\}$$ and the corresponding event ciphertext as $$\tilde{v}_{\text{aud}} \in \\{\tilde{v}_{\text{aud,r}}, \tilde{v}_{\text{aud,s}}\\}$$. In each case the amount mask is `lane[0]` of the channel's sponge; the remaining lanes ($$m_{r,r}$$ on the recipient channel; $$m_{b,s}$$ and $$m_{r,s}$$ on the sender channel) are computed and discarded for an amount disclosure, or used in place of `lane[0]` for the balance/randomness variants noted below. A3 squeezes each channel at the width [Poseidon2 Hash](../../protocol/primitives.md#poseidon2-hash) fixes for its tag — two lanes on $$\delta_{\text{aud\\\_r}}$$, three on $$\delta_{\text{aud\\\_s}}$$ — so neither tag is ever read at a second arity ([Mode exclusivity](../../protocol/primitives.md#mode-exclusivity)).

## Public inputs

| Symbol | Source |
|:---|:---|
| $$K_{\text{aud}}$$ | auditor's on-chain Grumpkin pubkey for the chosen channel ($$K_{\text{aud,r}}$$ or $$K_{\text{aud,s}}$$) ([Auditor Key Management and Rotation](../../protocol/auditing.md#auditor-key-management-and-rotation)) |
| $$R_e, \sigma_E, \tilde{v}_{\text{aud}}$$ | from the on-chain event ($$\tilde{v}_{\text{aud,r}}$$ for the recipient-side channel, $$\tilde{v}_{\text{aud,s}}$$ for the sender-side channel). $$\sigma_E = \sigma$$ for `Transfer`, $$\sigma_E = \sigma_a'$$ for `SpenderTransfer` ([Spender Transfer](../../protocol/operations/spender-transfer.md)). |
| $$P_R, \nu$$ | disclosure recipient pubkey and nonce |
| $$R_{\text{disc}}, \tilde{v}_{\text{disc}}$$ | disclosure ciphertext |

## Private witnesses

$$aud_{sk}$$, $$v_{\text{transfer}}$$, $$r_{\text{disc}}$$.

## Constraints

| # | Constraint |
|:--|:---|
| A1 | $$K_{\text{aud}} = aud_{sk} \cdot H$$ (auditor key ownership) |
| A2 | $$s_{\text{aud}} = \text{ECDH}(aud_{sk}, R_e)$$ (auditor-side ECDH shared scalar; mirrors the prover-side $$\text{ECDH}(r_e, K_{\text{aud}})$$ from [T_a1](../../protocol/operations/transfer.md#constraints) / T_a5) |
| A3 | $$(m_v, m_2, \ldots) = \text{SpongeSqueeze}_n(\delta_{\text{aud}}, s_{\text{aud}}, \sigma_E)$$, $$n = 2$$ on $$\delta_{\text{aud\\\_r}}$$ and $$n = 3$$ on $$\delta_{\text{aud\\\_s}}$$ (auditor channel sponge at the width [Poseidon2 Hash](../../protocol/primitives.md#poseidon2-hash) fixes per tag; same construction as [Per-Transfer Auditor Ciphertexts](../../protocol/auditing.md#per-transfer-auditor-ciphertexts)) |
| A4 | $$v_{\text{transfer}} = \tilde{v}_{\text{aud}} - m_v$$ (correct decryption of the channel's amount slot, the first squeeze) |
| D5 | $$v_{\text{transfer}} \in [0, 2^{127})$$ |
| U1–U3 | Disclosure ciphertext to recipient ([Disclosure Ciphertext to Recipient](../protocol.md#disclosure-ciphertext-to-recipient)) |

D-auditor does not bind to an account record; the auditor key already binds the proof. The disclosure recipient confirms which account the event concerns by reading the event's sender and recipient addresses directly.

## Verifier flow

Follow [Verifier Protocol](../protocol.md#verifier-protocol) with `circuit_id = D-auditor` (or the chosen balance / randomness variant). Step 2 is skipped — no $$\text{PVK}_A$$ lookup is needed. Step 3 resolves $$K_{\text{aud}}$$ at the event's ledger: $$K_{\text{aud,r}}$$ from the `auditor_id` on the event's `to` account when disclosing the recipient-side channel, or $$K_{\text{aud,s}}$$ from the `auditor_id` on the `from` account when disclosing the sender-side channel. `from` is the funds' owner in both `Transfer` and `SpenderTransfer`, since the sender-auditor channel always tracks the owner ([Spender Transfer](../../protocol/operations/spender-transfer.md)).

**Balance / randomness variants.** `lane[1]` of each channel carries a distinct datum: $$m_{b,s}$$ (sender's post-transfer balance checkpoint, channel $$\delta_{\text{aud\\\_s}}$$, recovered from $$\tilde{b}_{\text{aud,s}}$$; on a `SpenderTransfer` the same lane carries the post-transfer allowance, recovered from $$\tilde{a}_{\text{aud,s}}$$, [Spender Transfer](../../protocol/operations/spender-transfer.md) O\_a8) or $$m_{r,r}$$ (per-transfer Pedersen randomness, channel $$\delta_{\text{aud\\\_r}}$$, recovered from $$\tilde{r}_{\text{aud,r}}$$). A circuit that discloses either of these substitutes the corresponding event ciphertext for $$\tilde{v}_{\text{aud}}$$ in A4 and reads $$m_2$$ rather than $$m_v$$ from the sponge output. Range constraint D5 applies unchanged to a balance disclosure; for a randomness disclosure D5 is dropped since $$r_{\text{transfer}} \in \mathbb{F}_r$$ is not range-bounded. The balance variant has a blinding sibling on `lane[2]` of the sender channel: $$m_{r,s}$$ recovers the sender's post-transfer spendable blinding from $$\tilde{r}_{\text{aud,s}}$$ ([Confidential Transfer](../../protocol/operations/transfer.md) T\_a9), so an auditor can disclose the full opening of the sender's $$C_{\text{spend}}$$ *as of that transfer* rather than its value alone ([Per-Transfer Auditor Ciphertexts](../../protocol/auditing.md#per-transfer-auditor-ciphertexts)); the variant reads A3's third lane and D5 is dropped as for the randomness variant. On a `SpenderTransfer` the same lane carries the post-transfer allowance blinding $$r_a'$$ ([Spender Transfer](../../protocol/operations/spender-transfer.md) O\_a9), so the same variant discloses the opening of $$C_a'$$ rather than of $$C_{\text{spend}}'$$ ([Spender Allowance Auditing](../../protocol/auditing.md#spender-allowance-auditing)). These variants are not separately tabulated.

---

Previous: [D-sender](d-sender.md) · Up: [Index](../../README.md#companions) · Next: [D-balance](d-balance.md)
