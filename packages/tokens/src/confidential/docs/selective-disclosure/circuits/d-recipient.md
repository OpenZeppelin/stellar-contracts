# Circuit D-recipient: Holder Discloses an Inbound Transfer

The account holder is the recipient of an on-chain confidential transfer (either a `Transfer` to them or a `SpenderTransfer` whose `to` is them) and proves to a third party that the transfer was for amount $$v\_{\text{transfer}}$$. The same circuit covers both event families because the recipient-side ECDH constraint has identical shape in either case; only the value of the event nonce $$\sigma\_E$$ differs ($$\sigma$$ for `Transfer`, $$\sigma\_a'$$ for `SpenderTransfer`; see [T9](../../protocol/operations/transfer.md#constraints) and [O9](../../protocol/operations/spender-transfer.md#constraints)).

## Public inputs

| Symbol | Source |
|:---|:---|
| $$\text{addr\\\_f}$$ | compressed contract-address Field, loaded from instance storage ([Address-to-Field Encoding](../../protocol/primitives.md#address-to-field-encoding), [Governance and Upgradeability](../../protocol/system-model.md#governance-and-upgradeability)) |
| $$\text{PVK}\_A$$ | disclosing account's stored `viewing_public_key` ([Account Data Model](../../protocol/account-state.md#account-data-model)); $$A$$ is the address listed as the event's `to` |
| $$R\_e, \sigma\_E, \tilde{v}$$ | from the on-chain event being disclosed ([Event Schema](../../protocol/interface.md#event-schema)). $$\sigma\_E = \sigma$$ for `Transfer`, $$\sigma\_E = \sigma\_a'$$ for `SpenderTransfer`. |
| $$P\_R$$ | disclosure recipient's Grumpkin pubkey ([Disclosure Recipient](../README.md#disclosure-recipient)) |
| $$\nu$$ | recipient-supplied nonce ([Disclosure Recipient](../README.md#disclosure-recipient)) |
| $$R\_{\text{disc}}, \tilde{v}\_{\text{disc}}$$ | disclosure ciphertext to recipient ([Disclosure Ciphertext to Recipient](../protocol.md#disclosure-ciphertext-to-recipient)) |

## Private witnesses

$$sk\_A$$, $$vk\_A$$, $$v\_{\text{transfer}}$$, $$r\_{\text{disc}}$$.

## Constraints

| # | Constraint |
|:--|:---|
| D1 | $$vk\_A = \text{Poseidon}(\delta\_{\text{vk}}, sk\_A, \text{addr\\\_f})$$ (viewing key correctly derived, binds proof to contract; mirrors [R2/T2/W2/S2](../../protocol/operations/register.md#constraints)) |
| D2 | $$\text{PVK}\_A = vk\_A \cdot H$$ (binds proof to on-chain account) |
| D3 | $$s = \text{ECDH}(vk\_A, R\_e)$$ (recipient-side ECDH shared scalar, [Elliptic Curve Diffie-Hellman](../../protocol/primitives.md#elliptic-curve-diffie-hellman)) |
| D4 | $$v\_{\text{transfer}} = \tilde{v} - \text{Poseidon}(\delta\_{\text{transfer\\\_amount}}, s, \sigma\_E)$$ (correct decryption of event amount; matches [T9](../../protocol/operations/transfer.md#constraints) for `Transfer` and O9 for `SpenderTransfer`) |
| D5 | $$v\_{\text{transfer}} \in [0, 2^{127})$$ (range, [Integer Embedding and Range Proofs](../../protocol/primitives.md#integer-embedding-and-range-proofs)) |
| U1–U3 | Disclosure ciphertext to recipient ([Disclosure Ciphertext to Recipient](../protocol.md#disclosure-ciphertext-to-recipient)) |

D1 and D2 anchor the proof to the disclosing account's on-chain record without revealing $$sk\_A$$ or $$vk\_A$$. D3 and D4 recompute the standard recipient-side decryption that the holder would normally perform offline to learn the incoming amount. The result $$v\_{\text{transfer}}$$ then feeds the U-block, which encrypts it to the disclosure recipient.

## Verifier flow

Follow [Verifier Protocol](../protocol.md#verifier-protocol) with `circuit_id = D-recipient`. Step 2 resolves $$\text{PVK}\_A$$ at $$E.\text{to}$$ (the only account record this variant consults). On success, the recipient now knows that the named on-chain event paid the named account exactly $$v\_{\text{transfer}}$$ tokens, and learns nothing else.
