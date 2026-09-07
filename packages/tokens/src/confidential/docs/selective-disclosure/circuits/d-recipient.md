# Circuit D-recipient: Holder Discloses an Inbound Transfer

The account holder is the recipient of an on-chain confidential transfer (either a `Transfer` to them or a `SpenderTransfer` whose `to` is them) and proves to a third party that the transfer was for amount $$v\_{\text{transfer}}$$. The same circuit covers both event families because the recipient-side ECDH constraint has identical shape in either case; only the value of the event nonce $$\sigma\_E$$ differs ($$\sigma$$ for `Transfer`, $$\sigma\_a'$$ for `SpenderTransfer`; see DESIGN.md §7.6 T9, §7.8 O9).

## Public inputs

| Symbol | Source |
|:---|:---|
| $$\text{addr\\\_f}$$ | compressed contract-address Field, loaded from instance storage (DESIGN.md §2.7, §3.5) |
| $$\text{PVK}\_A$$ | disclosing account's stored `viewing_public_key` (DESIGN.md §6.1); $$A$$ is the address listed as the event's `to` |
| $$R\_e, \sigma\_E, \tilde{v}$$ | from the on-chain event being disclosed (DESIGN_cont.md §11.2). $$\sigma\_E = \sigma$$ for `Transfer`, $$\sigma\_E = \sigma\_a'$$ for `SpenderTransfer`. |
| $$P\_R$$ | disclosure recipient's Grumpkin pubkey (§2.1) |
| $$\nu$$ | recipient-supplied nonce (§2.1) |
| $$R\_{\text{disc}}, \tilde{v}\_{\text{disc}}$$ | disclosure ciphertext to recipient (§4) |

## Private witnesses

$$sk\_A$$, $$vk\_A$$, $$v\_{\text{transfer}}$$, $$r\_{\text{disc}}$$.

## Constraints

| # | Constraint |
|:--|:---|
| D1 | $$vk\_A = \text{Poseidon}(\delta\_{\text{vk}}, sk\_A, \text{addr\\\_f})$$ (viewing key correctly derived, binds proof to contract; mirrors DESIGN.md R2/T2/W2/S2) |
| D2 | $$\text{PVK}\_A = vk\_A \cdot H$$ (binds proof to on-chain account) |
| D3 | $$s = \text{ECDH}(vk\_A, R\_e)$$ (recipient-side ECDH shared scalar, DESIGN.md §2.4) |
| D4 | $$v\_{\text{transfer}} = \tilde{v} - \text{Poseidon}(\delta\_{\text{transfer\\\_amount}}, s, \sigma\_E)$$ (correct decryption of event amount; matches DESIGN.md T9 for `Transfer` and O9 for `SpenderTransfer`) |
| D5 | $$v\_{\text{transfer}} \in [0, 2^{127})$$ (range, DESIGN.md §2.6) |
| U1–U3 | Disclosure ciphertext to recipient (§4) |

D1 and D2 anchor the proof to the disclosing account's on-chain record without revealing $$sk\_A$$ or $$vk\_A$$. D3 and D4 recompute the standard recipient-side decryption that the holder would normally perform offline to learn the incoming amount. The result $$v\_{\text{transfer}}$$ then feeds the U-block, which encrypts it to the disclosure recipient.

## Verifier flow

Follow §5.3 with `circuit_id = D-recipient`. Step 2 resolves $$\text{PVK}\_A$$ at $$E.\text{to}$$ (the only account record this variant consults). On success, the recipient now knows that the named on-chain event paid the named account exactly $$v\_{\text{transfer}}$$ tokens, and learns nothing else.
