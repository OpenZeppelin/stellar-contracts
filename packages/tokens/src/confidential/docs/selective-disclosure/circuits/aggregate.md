# Aggregate Disclosures

For statements of the form "this account received at least $$X$$ from counterparty $$Y$$ during window $$W$$", the D-recipient circuit (or D-auditor) is vectorized over $$n$$ events.

## Public inputs

| Symbol | Source |
|:---|:---|
| Common: $$\text{addr\\\_f}$$, $$\text{PVK}\_A$$, $$P\_R$$, $$\nu$$, $$R\_{\text{disc}}, \tilde{V}\_{\text{disc}}$$ | as in [Circuit D-recipient: Holder Discloses an Inbound Transfer](d-recipient.md) |
| List: $$(R\_{e,i}, \sigma\_{E,i}, \tilde{v}\_i)$$ for $$i \in [1, n]$$ | from $$n$$ on-chain transfer-family events; $$\sigma\_{E,i} = \sigma$$ if event $$i$$ is a `Transfer`, $$\sigma\_a'$$ if `SpenderTransfer`. Each event MUST be identified by a $$\text{ref}\_{E,i}$$ in the proof bundle and resolved per [Verifier Protocol](../protocol.md#verifier-protocol). |
| Optional: $$V\_{\text{threshold}}$$ | aggregate threshold |

## Private witnesses

$$sk\_A$$, $$vk\_A$$, $$\\{v\_{\text{transfer},i}\\}\_{i=1}^n$$, $$r\_{\text{disc}}$$.

## Constraints

| # | Constraint |
|:--|:---|
| D1, D2 | As in [Circuit D-recipient: Holder Discloses an Inbound Transfer](d-recipient.md) |
| For each $$i$$: $$\text{D3}\_i$$ | $$s\_i = \text{ECDH}(vk\_A, R\_{e,i})$$ |
| For each $$i$$: $$\text{D4}\_i$$ | $$v\_{\text{transfer},i} = \tilde{v}\_i - \text{Poseidon}(\delta\_{\text{transfer\\\_amount}}, s\_i, \sigma\_{E,i})$$ |
| For each $$i$$: $$\text{D5}\_i$$ | $$v\_{\text{transfer},i} \in [0, 2^{127})$$ |
| AGG | $$V\_{\text{total}} = \sum\_{i=1}^n v\_{\text{transfer},i}$$ |
| AGG-R | $$V\_{\text{total}} \in [0, 2^{127})$$ (range, [Integer Embedding and Range Proofs](../../protocol/primitives.md#integer-embedding-and-range-proofs)) |
| THRESH (optional) | $$V\_{\text{total}} \geq V\_{\text{threshold}}$$, evaluated as $$V\_{\text{total}} - V\_{\text{threshold}} \in [0, 2^{127})$$ on the AGG-R-checked $$V\_{\text{total}}$$, with $$V\_{\text{threshold}} \in [0, 2^{127})$$ supplied by the recipient |
| U1–U3 | Encrypt the AGG-R-checked $$V\_{\text{total}}$$ (not the individual $$v\_{\text{transfer},i}$$) to the recipient, with $$\delta\_{\text{disc\\\_bind}}$$ replacing $$\delta\_{\text{disc}}$$ to separate domain |

Over $$\mathbb{F}\_r$$ the comparison in THRESH is undefined and decrypting $$\tilde{V}\_{\text{disc}}$$ recovers $$V\_{\text{total}}$$ only modulo $$r$$, so AGG-R MUST precede THRESH and U1–U3. The event count $$n$$ is fixed per aggregate `circuit_id` ([Circuits](../security.md#circuits)); every realizable $$n$$ satisfies $$n \cdot 2^{127} < r$$, so the field sum in AGG is the integer sum and AGG-R bounds that integer. An aggregate of $$2^{127}$$ or more exceeds the token's `i128` domain and is not disclosable by this circuit.

The recipient filters the $$n$$ events off-chain by the criteria they care about (sender address, block timestamp) before constructing the verifier's public inputs. They learn the aggregate $$V\_{\text{total}}$$ but not the individual amounts. If THRESH is included and the recipient does not need the aggregate value itself, U1–U3 can be omitted; the proof's mere validity asserts the threshold.

Aggregate disclosures over outbound transfers use the D-sender constraint block per event; aggregate auditor disclosures use the D-auditor block per event.

---

Previous: [D-balance](d-balance.md) · Up: [Index](../../README.md#companions) · Next: [Security and Implementation Notes](../security.md)
