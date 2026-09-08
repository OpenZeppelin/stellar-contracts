# Confidential Transfer

The sender (account $$A$$, spending key $$sk\_A$$) transfers a hidden amount $$v\_{\text{transfer}}$$ to recipient $$B$$ (public viewing key $$\text{PVK}\_B$$).

**Sender computation.** The constraint table below doubles as the specification of what the sender computes: every quantity the prover produces appears there as the equation the circuit enforces on it. Two prover-side steps are not constraints — the sender draws the salt $$\sigma$$ by the [Grumpkin-BN254 Cycle](../primitives.md#grumpkin-bn254-cycle) procedure and derives the ephemeral scalar from it per [ECDH-Derived Blinding](../keys-and-commitments.md#ecdh-derived-blinding).

## Constraints

| # | Constraint |
|:--|:---|
| T1 | $$Y\_A = sk\_A \cdot H$$ (sender key ownership) |
| T2 | $$vk\_A = \text{Poseidon}(\delta\_{\text{vk}}, sk\_A, \text{addr\\\_f})$$ (binds proof to contract) |
| T3 | Prover knows opening $$(v\_A, r\_A)$$ of $$C\_{\text{spend}}^A$$ |
| T4 | $$v\_A \in [0, 2^{127})$$, $$v\_{\text{transfer}} \in [0, 2^{127})$$, $$v\_A - v\_{\text{transfer}} \in [0, 2^{127})$$ (range validity, [Integer Embedding and Range Proofs](../primitives.md#integer-embedding-and-range-proofs)) |
| T5 | $$s = \text{ECDH}(r\_e, \text{PVK}\_B)$$ (recipient ECDH shared scalar correctly derived, [Elliptic Curve Diffie-Hellman](../primitives.md#elliptic-curve-diffie-hellman)) |
| T6 | $$R\_e = r\_e \cdot H$$ (ephemeral key well-formed) |
| T7 | $$r\_{\text{transfer}} = \text{Poseidon}(\delta\_{\text{transfer\\\_blind}}, s, \sigma)$$ (blinding correctly derived) |
| T8 | $$C\_{\text{transfer}} = v\_{\text{transfer}} \cdot G + r\_{\text{transfer}} \cdot H$$ (transfer commitment well-formed) |
| T9 | $$\tilde{v} = v\_{\text{transfer}} + \text{Poseidon}(\delta\_{\text{transfer\\\_amount}}, s, \sigma)$$ (encrypted amount correct) |
| T10 | $$r\_A' = \text{Poseidon}(\delta\_{\text{spend\\\_r}}, vk\_A, \sigma)$$ (deterministic randomness) |
| T11 | $$C\_{\text{spend}}' = (v\_A - v\_{\text{transfer}}) \cdot G + r\_A' \cdot H$$ (new sender balance) |
| T12 | $$\tilde{b} = (v\_A - v\_{\text{transfer}}) + \text{Poseidon}(\delta\_{\text{enc\\\_bal}}, vk\_A, \sigma)$$ (encrypted balance scalar) |
| T13 | $$r\_e \neq 0$$ (rules out $$R\_e = \mathcal{O}$$ and $$S, S\_{a,r}, S\_{a,s} = \mathcal{O}$$; otherwise every ECDH mask in this transfer collapses to a constant function of $$\sigma$$) |
| T\_a1 | $$s\_{a,r} = \text{ECDH}(r\_e, K\_{\text{aud,r}})$$ (recipient-auditor ECDH shared scalar, reuses ephemeral scalar) |
| T\_a2 | $$(m\_{v,r}, m\_{r,r}) = \text{SpongeSqueeze}\_2(\delta\_{\text{aud\\\_r}}, s\_{a,r}, \sigma)$$ (recipient-auditor channel masks) |
| T\_a3 | $$\tilde{v}\_{\text{aud,r}} = v\_{\text{transfer}} + m\_{v,r}$$ (recipient-auditor encrypted transfer amount) |
| T\_a4 | $$\tilde{r}\_{\text{aud,r}} = r\_{\text{transfer}} + m\_{r,r}$$ (recipient-auditor encrypted transfer randomness, enables Pedersen-opening reconstruction of $$C\_{\text{receive}}$$, see [Per-Transfer Auditor Ciphertexts](../auditing.md#per-transfer-auditor-ciphertexts)) |
| T\_a5 | $$s\_{a,s} = \text{ECDH}(r\_e, K\_{\text{aud,s}})$$ (sender-auditor ECDH shared scalar, reuses ephemeral scalar) |
| T\_a6 | $$(m\_{v,s}, m\_{b,s}, m\_{r,s}) = \text{SpongeSqueeze}\_3(\delta\_{\text{aud\\\_s}}, s\_{a,s}, \sigma)$$ (sender-auditor channel masks) |
| T\_a7 | $$\tilde{v}\_{\text{aud,s}} = v\_{\text{transfer}} + m\_{v,s}$$ (sender-auditor encrypted transfer amount) |
| T\_a8 | $$\tilde{b}\_{\text{aud,s}} = (v\_A - v\_{\text{transfer}}) + m\_{b,s}$$ (sender-auditor encrypted balance checkpoint) |
| T\_a9 | $$\tilde{r}\_{\text{aud,s}} = r\_A' + m\_{r,s}$$ (sender-auditor escrow of the new spendable blinding, over T10's $$r\_A'$$) |

## Public inputs (25 fields, counting each Grumpkin point as two $$\mathbb{F}\_r$$ coordinates)

| Input | Notes |
|:---|:---|
| $$C\_{\text{spend}}^A$$ | Loaded from sender's `spendable_commitment` |
| $$Y\_A$$ | Loaded from sender's `spending_public_key` |
| $$\text{PVK}\_B$$ | Loaded from recipient's `viewing_public_key`. Recipient must be registered. |
| $$\text{addr\\\_f}$$ | Loaded from instance storage; set once at construction ([Governance and Upgradeability](../system-model.md#governance-and-upgradeability)) |
| $$K\_{\text{aud,r}}$$ | Fetched from the auditor contract using recipient's `auditor_id` |
| $$K\_{\text{aud,s}}$$ | Fetched from the auditor contract using sender's `auditor_id` |
| $$C\_{\text{spend}}'$$, $$C\_{\text{transfer}}$$, $$R\_e$$, $$\tilde{v}$$, $$\tilde{b}$$, $$\sigma$$, $$\tilde{v}\_{\text{aud,r}}$$, $$\tilde{r}\_{\text{aud,r}}$$, $$\tilde{v}\_{\text{aud,s}}$$, $$\tilde{b}\_{\text{aud,s}}$$, $$\tilde{r}\_{\text{aud,s}}$$ | Prover-supplied, in this order; $$C\_{\text{spend}}'$$ written to sender's `spendable_commitment`, $$C\_{\text{transfer}}$$ added to recipient's `receiving_commitment`, the rest emitted in event |

## Private witnesses

$$sk\_A$$, $$vk\_A$$, $$v\_A$$, $$r\_A$$, $$v\_{\text{transfer}}$$, $$r\_e$$.

## Post-verification

The contract verifies the proof, then:
- Sets $$A$$`.spendable_commitment` $$= C\_{\text{spend}}'$$
- Adds to recipient: $$B$$`.receiving_commitment` $$\mathrel{+}= C\_{\text{transfer}}$$
- Emits event with $$(R\_e, \tilde{v}, \sigma, \tilde{b}, \tilde{v}\_{\text{aud,r}}, \tilde{r}\_{\text{aud,r}}, \tilde{v}\_{\text{aud,s}}, \tilde{b}\_{\text{aud,s}}, \tilde{r}\_{\text{aud,s}})$$

## Recipient processing

Upon observing the event, the recipient computes $$s = \text{ECDH}(vk, R\_e)$$, derives amount and blinding. The decryption flow is independent of whether the sender was the owner or a spender.

---

Previous: [Withdrawal](withdraw.md) · Up: [Operations](README.md) · Next: [Set Spender](set-spender.md)
