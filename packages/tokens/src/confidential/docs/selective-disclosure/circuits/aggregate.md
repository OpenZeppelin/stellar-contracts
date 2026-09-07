# Aggregate Disclosures

For statements of the form "this account received at least $$X$$ from counterparty $$Y$$ during window $$W$$", the D-recipient circuit (or D-auditor) is vectorized over $$n$$ events.

## Public inputs

| Symbol | Source |
|:---|:---|
| Common: $$\text{addr\\\_f}$$, $$\text{PVK}_A$$, $$P_R$$, $$\nu$$, $$R_{\text{disc}}, \tilde{V}_{\text{disc}}$$ | as in [Circuit D-recipient: Holder Discloses an Inbound Transfer](d-recipient.md) |
| List: $$(R_{e,i}, \sigma_{E,i}, \tilde{v}_i)$$ for $$i \in [1, n]$$ | from $$n$$ on-chain transfer-family events; $$\sigma_{E,i} = \sigma$$ if event $$i$$ is a `Transfer`, $$\sigma_a'$$ if `SpenderTransfer`. Each event MUST be identified by a $$\text{ref}_{E,i}$$ in the proof bundle and resolved per [Verifier Protocol](../protocol.md#verifier-protocol). |
| Optional: $$V_{\text{threshold}}$$ | aggregate threshold |

## Private witnesses

$$sk_A$$, $$vk_A$$, $$\\{v_{\text{transfer},i}\\}_{i=1}^n$$, $$r_{\text{disc}}$$.

## Constraints

| # | Constraint |
|:--|:---|
| D1, D2 | As in [Circuit D-recipient: Holder Discloses an Inbound Transfer](d-recipient.md) |
| For each $$i$$: D3$$_i$$ | $$s_i = \text{ECDH}(vk_A, R_{e,i})$$ |
| For each $$i$$: D4$$_i$$ | $$v_{\text{transfer},i} = \tilde{v}_i - \text{Poseidon}(\delta_{\text{transfer\\\_amount}}, s_i, \sigma_{E,i})$$ |
| For each $$i$$: D5$$_i$$ | $$v_{\text{transfer},i} \in [0, 2^{127})$$ |
| AGG | $$V_{\text{total}} = \sum_{i=1}^n v_{\text{transfer},i}$$ |
| THRESH (optional) | $$V_{\text{total}} \geq V_{\text{threshold}}$$ |
| U1–U3 | Encrypt $$V_{\text{total}}$$ (not the individual $$v_{\text{transfer},i}$$) to the recipient, with $$\delta_{\text{disc\\\_bind}}$$ replacing $$\delta_{\text{disc}}$$ to separate domain |

The recipient filters the $$n$$ events off-chain by the criteria they care about (sender address, block timestamp) before constructing the verifier's public inputs. They learn the aggregate $$V_{\text{total}}$$ but not the individual amounts. If THRESH is included and the recipient does not need the aggregate value itself, U1–U3 can be omitted; the proof's mere validity asserts the threshold.

Aggregate disclosures over outbound transfers use the D-sender constraint block per event; aggregate auditor disclosures use the D-auditor block per event.
