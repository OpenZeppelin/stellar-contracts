# Circuit D-recipient: Holder Discloses an Inbound Transfer

The account holder is the recipient of an on-chain confidential transfer (either a `Transfer` to them or a `SpenderTransfer` whose `to` is them) and proves to a third party that the transfer was for amount $$v_{\text{transfer}}$$. The same circuit covers both event families because the recipient-side ECDH constraint has identical shape in either case; only the value of the event nonce $$\sigma_E$$ differs ($$\sigma$$ for `Transfer`, $$\sigma_a'$$ for `SpenderTransfer`; see [T9](../../protocol/operations/transfer.md#constraints) and [O9](../../protocol/operations/spender-transfer.md#constraints)).

## Public inputs

| Symbol | Source |
|:---|:---|
| $$\text{addr\\\_f}$$ | compressed contract-address Field, loaded from instance storage ([Address-to-Field Encoding](../../protocol/primitives.md#address-to-field-encoding), [Governance and Upgradeability](../../protocol/system-model.md#governance-and-upgradeability)) |
| $$\text{PVK}_A$$ | disclosing account's stored `viewing_public_key` ([Account Data Model](../../protocol/account-state.md#account-data-model)); $$A$$ is the address listed as the event's `to` |
| $$R_e, \sigma_E, \tilde{v}$$ | from the on-chain event being disclosed ([Event Schema](../../protocol/interface.md#event-schema)). $$\sigma_E = \sigma$$ for `Transfer`, $$\sigma_E = \sigma_a'$$ for `SpenderTransfer`. |
| $$P_R$$ | disclosure recipient's Grumpkin pubkey ([Disclosure Recipient](../README.md#disclosure-recipient)) |
| $$\nu$$ | recipient-supplied nonce ([Disclosure Recipient](../README.md#disclosure-recipient)) |
| $$R_{\text{disc}}, \tilde{v}_{\text{disc}}$$ | disclosure ciphertext to recipient ([Disclosure Ciphertext to Recipient](../protocol.md#disclosure-ciphertext-to-recipient)) |

## Private witnesses

$$sk_A$$, $$vk_A$$, $$v_{\text{transfer}}$$, $$r_{\text{disc}}$$.

## Constraints

| # | Constraint |
|:--|:---|
| D1 | $$vk_A = \text{Poseidon}(\delta_{\text{vk}}, sk_A, \text{addr\\\_f})$$ (viewing key correctly derived, binds proof to contract; mirrors [R2/T2/W2/S2](../../protocol/operations/register.md#constraints)) |
| D2 | $$\text{PVK}_A = vk_A \cdot H$$ (binds proof to on-chain account) |
| D3 | $$s = \text{ECDH}(vk_A, R_e)$$ (recipient-side ECDH shared scalar, [Elliptic Curve Diffie-Hellman](../../protocol/primitives.md#elliptic-curve-diffie-hellman)) |
| D4 | $$v_{\text{transfer}} = \tilde{v} - \text{Poseidon}(\delta_{\text{transfer\\\_amount}}, s, \sigma_E)$$ (correct decryption of event amount; matches [T9](../../protocol/operations/transfer.md#constraints) for `Transfer` and O9 for `SpenderTransfer`) |
| D5 | $$v_{\text{transfer}} \in [0, 2^{127})$$ (range, [Integer Embedding and Range Proofs](../../protocol/primitives.md#integer-embedding-and-range-proofs)) |
| U1–U3 | Disclosure ciphertext to recipient ([Disclosure Ciphertext to Recipient](../protocol.md#disclosure-ciphertext-to-recipient)) |

D1 and D2 anchor the proof to the disclosing account's on-chain record without revealing $$sk_A$$ or $$vk_A$$. D3 and D4 recompute the standard recipient-side decryption that the holder would normally perform offline to learn the incoming amount. The result $$v_{\text{transfer}}$$ then feeds the U-block, which encrypts it to the disclosure recipient.

## Verifier flow

Follow [Verifier Protocol](../protocol.md#verifier-protocol) with `circuit_id = D-recipient`. Step 2 resolves $$\text{PVK}_A$$ at $$E.\text{to}$$ (the only account record this variant consults). On success, the recipient now knows that the named on-chain event paid the named account exactly $$v_{\text{transfer}}$$ tokens, and learns nothing else.

---

Previous: [Disclosure Protocol](../protocol.md) · Up: [Index](../../README.md#companions) · Next: [D-sender](d-sender.md)
