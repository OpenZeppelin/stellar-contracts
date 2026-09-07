# Aggregate Disclosures

For statements of the form "this account received at least $$X$$ from counterparty $$Y$$ during window $$W$$", the D-recipient circuit (or D-auditor) is vectorized over $$n$$ events.

## Public inputs

| Symbol | Source |
|:---|:---|
| Common: $$\text{addr\\\_f}$$, $$\text{PVK}\_A$$, $$P\_R$$, $$\nu$$, $$R\_{\text{disc}}, \tilde{V}\_{\text{disc}}$$ | as in §6 |
| List: $$(R\_{e,i}, \sigma\_{E,i}, \tilde{v}\_i)$$ for $$i \in [1, n]$$ | from $$n$$ on-chain transfer-family events; $$\sigma\_{E,i} = \sigma$$ if event $$i$$ is a `Transfer`, $$\sigma\_a'$$ if `SpenderTransfer`. Each event MUST be identified by a $$\text{ref}\_{E,i}$$ in the proof bundle and resolved per §5.3. |
| Optional: $$V\_{\text{threshold}}$$ | aggregate threshold |

## Private witnesses

$$sk\_A$$, $$vk\_A$$, $$\\{v\_{\text{transfer},i}\\}\_{i=1}^n$$, $$r\_{\text{disc}}$$.

## Constraints

| # | Constraint |
|:--|:---|
| D1, D2 | As in §6 |
| For each $$i$$: D3$$\_i$$ | $$s\_i = \text{ECDH}(vk\_A, R\_{e,i})$$ |
| For each $$i$$: D4$$\_i$$ | $$v\_{\text{transfer},i} = \tilde{v}\_i - \text{Poseidon}(\delta\_{\text{transfer\\\_amount}}, s\_i, \sigma\_{E,i})$$ |
| For each $$i$$: D5$$\_i$$ | $$v\_{\text{transfer},i} \in [0, 2^{127})$$ |
| AGG | $$V\_{\text{total}} = \sum\_{i=1}^n v\_{\text{transfer},i}$$ |
| THRESH (optional) | $$V\_{\text{total}} \geq V\_{\text{threshold}}$$ |
| U1–U3 | Encrypt $$V\_{\text{total}}$$ (not the individual $$v\_{\text{transfer},i}$$) to the recipient, with $$\delta\_{\text{disc\\\_bind}}$$ replacing $$\delta\_{\text{disc}}$$ to separate domain |

The recipient filters the $$n$$ events off-chain by the criteria they care about (sender address, block timestamp) before constructing the verifier's public inputs. They learn the aggregate $$V\_{\text{total}}$$ but not the individual amounts. If THRESH is included and the recipient does not need the aggregate value itself, U1–U3 can be omitted; the proof's mere validity asserts the threshold.

Aggregate disclosures over outbound transfers use the D-sender constraint block per event; aggregate auditor disclosures use the D-auditor block per event.
