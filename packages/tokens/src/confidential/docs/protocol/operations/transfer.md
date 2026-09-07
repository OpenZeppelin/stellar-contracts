# Confidential Transfer

The sender (account $$A$$, spending key $$sk_A$$) transfers a hidden amount $$v_{\text{transfer}}$$ to recipient $$B$$ (public viewing key $$\text{PVK}_B$$).

**Sender computation.** The constraint table below doubles as the specification of what the sender computes: every quantity the prover produces appears there as the equation the circuit enforces on it. Two prover-side steps are not constraints — the sender draws the salt $$\sigma$$ by the [Grumpkin-BN254 Cycle](../primitives.md#grumpkin-bn254-cycle) procedure and derives the ephemeral scalar from it per [ECDH-Derived Blinding](../keys-and-commitments.md#ecdh-derived-blinding).

## Constraints

| # | Constraint |
|:--|:---|
| T1 | $$Y_A = sk_A \cdot H$$ (sender key ownership) |
| T2 | $$vk_A = \text{Poseidon}(\delta_{\text{vk}}, sk_A, \text{addr\\\_f})$$ (binds proof to contract) |
| T3 | Prover knows opening $$(v_A, r_A)$$ of $$C_{\text{spend}}^A$$ |
| T4 | $$v_A \in [0, 2^{127})$$, $$v_{\text{transfer}} \in [0, 2^{127})$$, $$v_A - v_{\text{transfer}} \in [0, 2^{127})$$ (range validity, [Integer Embedding and Range Proofs](../primitives.md#integer-embedding-and-range-proofs)) |
| T5 | $$s = \text{ECDH}(r_e, \text{PVK}_B)$$ (recipient ECDH shared scalar correctly derived, [Elliptic Curve Diffie-Hellman](../primitives.md#elliptic-curve-diffie-hellman)) |
| T6 | $$R_e = r_e \cdot H$$ (ephemeral key well-formed) |
| T7 | $$r_{\text{transfer}} = \text{Poseidon}(\delta_{\text{transfer\\\_blind}}, s, \sigma)$$ (blinding correctly derived) |
| T8 | $$C_{\text{transfer}} = v_{\text{transfer}} \cdot G + r_{\text{transfer}} \cdot H$$ (transfer commitment well-formed) |
| T9 | $$\tilde{v} = v_{\text{transfer}} + \text{Poseidon}(\delta_{\text{transfer\\\_amount}}, s, \sigma)$$ (encrypted amount correct) |
| T10 | $$r_A' = \text{Poseidon}(\delta_{\text{spend\\\_r}}, vk_A, \sigma)$$ (deterministic randomness) |
| T11 | $$C_{\text{spend}}' = (v_A - v_{\text{transfer}}) \cdot G + r_A' \cdot H$$ (new sender balance) |
| T12 | $$\tilde{b} = (v_A - v_{\text{transfer}}) + \text{Poseidon}(\delta_{\text{enc\\\_bal}}, vk_A, \sigma)$$ (encrypted balance scalar) |
| T13 | $$r_e \neq 0$$ (rules out $$R_e = \mathcal{O}$$ and $$S, S_{a,r}, S_{a,s} = \mathcal{O}$$; otherwise every ECDH mask in this transfer collapses to a constant function of $$\sigma$$) |
| T\_a1 | $$s_{a,r} = \text{ECDH}(r_e, K_{\text{aud,r}})$$ (recipient-auditor ECDH shared scalar, reuses ephemeral scalar) |
| T\_a2 | $$(m_{v,r}, m_{r,r}) = \text{SpongeSqueeze}_2(\delta_{\text{aud\\\_r}}, s_{a,r}, \sigma)$$ (recipient-auditor channel masks) |
| T\_a3 | $$\tilde{v}_{\text{aud,r}} = v_{\text{transfer}} + m_{v,r}$$ (recipient-auditor encrypted transfer amount) |
| T\_a4 | $$\tilde{r}_{\text{aud,r}} = r_{\text{transfer}} + m_{r,r}$$ (recipient-auditor encrypted transfer randomness, enables Pedersen-opening reconstruction of $$C_{\text{receive}}$$, see [Per-Transfer Auditor Ciphertexts](../auditing.md#per-transfer-auditor-ciphertexts)) |
| T\_a5 | $$s_{a,s} = \text{ECDH}(r_e, K_{\text{aud,s}})$$ (sender-auditor ECDH shared scalar, reuses ephemeral scalar) |
| T\_a6 | $$(m_{v,s}, m_{b,s}, m_{r,s}) = \text{SpongeSqueeze}_3(\delta_{\text{aud\\\_s}}, s_{a,s}, \sigma)$$ (sender-auditor channel masks) |
| T\_a7 | $$\tilde{v}_{\text{aud,s}} = v_{\text{transfer}} + m_{v,s}$$ (sender-auditor encrypted transfer amount) |
| T\_a8 | $$\tilde{b}_{\text{aud,s}} = (v_A - v_{\text{transfer}}) + m_{b,s}$$ (sender-auditor encrypted balance checkpoint) |
| T\_a9 | $$\tilde{r}_{\text{aud,s}} = r_A' + m_{r,s}$$ (sender-auditor escrow of the new spendable blinding, over T10's $$r_A'$$) |

## Public inputs (25 fields, counting each Grumpkin point as two $$\mathbb{F}_r$$ coordinates)

| Input | Notes |
|:---|:---|
| $$C_{\text{spend}}^A$$ | Loaded from sender's `spendable_commitment` |
| $$Y_A$$ | Loaded from sender's `spending_public_key` |
| $$\text{PVK}_B$$ | Loaded from recipient's `viewing_public_key`. Recipient must be registered. |
| $$\text{addr\\\_f}$$ | Loaded from instance storage; set once at construction ([Governance and Upgradeability](../system-model.md#governance-and-upgradeability)) |
| $$K_{\text{aud,r}}$$ | Fetched from the auditor contract using recipient's `auditor_id` |
| $$K_{\text{aud,s}}$$ | Fetched from the auditor contract using sender's `auditor_id` |
| $$C_{\text{spend}}'$$, $$C_{\text{transfer}}$$, $$R_e$$, $$\tilde{v}$$, $$\tilde{b}$$, $$\sigma$$, $$\tilde{v}_{\text{aud,r}}$$, $$\tilde{r}_{\text{aud,r}}$$, $$\tilde{v}_{\text{aud,s}}$$, $$\tilde{b}_{\text{aud,s}}$$, $$\tilde{r}_{\text{aud,s}}$$ | Prover-supplied, in this order; $$C_{\text{spend}}'$$ written to sender's `spendable_commitment`, $$C_{\text{transfer}}$$ added to recipient's `receiving_commitment`, the rest emitted in event |

## Private witnesses

$$sk_A$$, $$vk_A$$, $$v_A$$, $$r_A$$, $$v_{\text{transfer}}$$, $$r_e$$.

## Post-verification

The contract verifies the proof, then:
- Sets $$A$$`.spendable_commitment` $$= C_{\text{spend}}'$$
- Adds to recipient: $$B$$`.receiving_commitment` $$\mathrel{+}= C_{\text{transfer}}$$
- Emits event with $$(R_e, \tilde{v}, \sigma, \tilde{b}, \tilde{v}_{\text{aud,r}}, \tilde{r}_{\text{aud,r}}, \tilde{v}_{\text{aud,s}}, \tilde{b}_{\text{aud,s}}, \tilde{r}_{\text{aud,s}})$$

## Recipient processing

Upon observing the event, the recipient computes $$s = \text{ECDH}(vk, R_e)$$, derives amount and blinding. The decryption flow is independent of whether the sender was the owner or a spender.

---

Previous: [Withdrawal](withdraw.md) · Up: [Operations](README.md) · Next: [Set Spender](set-spender.md)
