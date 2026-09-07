# Set Spender

The owner locks funds from their spendable balance into a per-spender escrow. The spender must be a registered account in the contract, so that $$Y_{\text{op}}$$ (needed for $$dvk_i$$ escrow) can be looked up from the spender's stored `spending_public_key`.

## Constraints

| # | Constraint |
|:--|:---|
| S1 | $$Y = sk \cdot H$$ (owner key ownership) |
| S2 | $$vk = \text{Poseidon}(\delta_{\text{vk}}, sk, \text{addr\\\_f})$$ (binds proof to contract) |
| S3 | Prover knows opening $$(v, r)$$ of $$C_{\text{spend}}$$ |
| S4 | $$v \in [0, 2^{127})$$, $$v_a \in [0, 2^{127})$$, $$v - v_a \in [0, 2^{127})$$ (range validity, [Integer Embedding and Range Proofs](../primitives.md#integer-embedding-and-range-proofs)) |
| S5 | $$dvk_i = \text{Poseidon}(\delta_{\text{dvk}}, vk, \text{op}_i)$$ (delegation key derivation; contract-bound via $$vk$$) |
| S6 | $$r_a = \text{Poseidon}(\delta_{\text{allow\\\_r}}, dvk_i, \sigma_a)$$ (allowance blinding) |
| S7 | $$C_a = v_a \cdot G + r_a \cdot H$$ (allowance commitment) |
| S8 | $$\tilde{a} = v_a + \text{Poseidon}(\delta_{\text{enc\\\_allow}}, dvk_i, \sigma_a)$$ (encrypted allowance) |
| S9 | $$r' = \text{Poseidon}(\delta_{\text{spend\\\_r}}, vk, \sigma)$$ (new balance randomness) |
| S10 | $$C_{\text{spend}}' = (v - v_a) \cdot G + r' \cdot H$$ (new spendable balance) |
| S11 | $$\tilde{b} = (v - v_a) + \text{Poseidon}(\delta_{\text{enc\\\_bal}}, vk, \sigma)$$ (encrypted balance) |
| S12 | Escrowed $$dvk_i$$ correctly encrypts under $$Y_{\text{op}}$$ via ECDH |
| S13 | $$r_e \neq 0$$ (rules out $$R_e = \mathcal{O}$$ and $$S_{a,s} = \mathcal{O}$$; the same $$r_e$$ is reused for the $$dvk_i$$ escrow ECDH in [Delegation Key Escrow](#delegation-key-escrow), so this also rules out a trivial escrow shared secret) |
| S14 | $$\tilde{r}_{a,\text{aud,s}} = \text{Poseidon}(\delta_{\text{esc\\\_allow\\\_r\\\_aud}}, s_{a,s}, \text{op}_i) + r_a$$ |
| S\_a1 | $$R_e = r_e \cdot H$$ (ephemeral key for auditor ECDH) |
| S\_a2 | $$s_{a,s} = \text{ECDH}(r_e, K_{\text{aud,s}})$$ (owner-auditor ECDH shared scalar, [Elliptic Curve Diffie-Hellman](../primitives.md#elliptic-curve-diffie-hellman)) |
| S\_a3 | $$(m_v, m_b, m_r) = \text{SpongeSqueeze}_3(\delta_{\text{aud\\\_s}}, s_{a,s}, \sigma)$$ (owner-auditor channel masks) |
| S\_a4 | $$\tilde{v}_{\text{aud,s}} = v_a + m_v$$ (owner-auditor encrypted escrow amount) |
| S\_a5 | $$\tilde{b}_{\text{aud,s}} = (v - v_a) + m_b$$ (owner-auditor encrypted balance checkpoint) |
| S\_a6 | $$\tilde{r}_{\text{aud,s}} = r' + m_r$$ (owner-auditor escrow of the new spendable blinding, over S9's $$r'$$) |

## Public inputs (26 fields)

| Input | Notes |
|:---|:---|
| $$C_{\text{spend}}$$ | Loaded from owner's `spendable_commitment` |
| $$Y$$ | Loaded from owner's `spending_public_key` |
| $$Y_{\text{op}}$$ | Loaded from spender account's `spending_public_key`. Spender must be registered. |
| $$\text{op}_i$$ | $$\text{address\\\_to\\\_field}$$(`spender` argument), computed per-call by the contract ([Address-to-Field Encoding](../primitives.md#address-to-field-encoding)) |
| $$\text{addr\\\_f}$$ | Loaded from instance storage; set once at construction ([Governance and Upgradeability](../system-model.md#governance-and-upgradeability)) |
| $$K_{\text{aud,s}}$$ | Fetched from the auditor contract using owner's `auditor_id` |
| $$C_{\text{spend}}'$$, $$C_a$$, escrowed\_dvk, $$\tilde{b}$$, $$\tilde{a}$$, $$\sigma$$, $$\sigma_a$$, $$R_e$$, $$\tilde{v}_{\text{aud,s}}$$, $$\tilde{b}_{\text{aud,s}}$$, $$\tilde{r}_{\text{aud,s}}$$, $$\tilde{r}_{a,\text{aud,s}}$$ | Prover-supplied, in this order; $$C_{\text{spend}}'$$ written to owner's `spendable_commitment`, the delegation fields written to storage, the rest emitted in event |

## Private witnesses

$$sk$$, $$vk$$, $$v$$, $$r$$, $$v_a$$, $$r_e$$.

## Post-verification

The contract verifies the proof, sets `spendable_commitment` $$= C_{\text{spend}}'$$ and stores the `SpenderDelegation`. Emits event with $$(R_e, \sigma, \tilde{b}, \tilde{v}_{\text{aud,s}}, \tilde{b}_{\text{aud,s}}, \tilde{r}_{\text{aud,s}}, \tilde{r}_{a,\text{aud,s}})$$.

---

## Delegation Key Escrow

At `set_spender`, the owner escrows $$dvk_i$$ to the spender on-chain via ECDH, eliminating off-chain key sharing:

1. Owner derives ephemeral $$r_e$$ per [ECDH-Derived Blinding](../keys-and-commitments.md#ecdh-derived-blinding) — the same scalar as the `set_spender` proof's outer ECDH; see [Why reusing $$r_e$$ is safe](../keys-and-commitments.md#why-reusing-r_e-is-safe) — and computes $$R = r_e \cdot H$$.
2. Shared secret: $$s = \text{ECDH}(r_e, Y_{\text{op}})$$ ([Elliptic Curve Diffie-Hellman](../primitives.md#elliptic-curve-diffie-hellman))
3. Escrowed key: $$\text{escrowed\\\_dvk} = (R.x, \\; \text{Poseidon}(\delta_{\text{esc\\\_dvk}}, s, \text{op}_i) + dvk_i)$$

**Encoding.** `escrowed_dvk` is a `BytesN<64>` consisting of two 32-byte $$\mathbb{F}_r$$ representatives: `R_x` (the $$x$$-coordinate of $$R$$) followed by `dvk_cipher` (the masked $$dvk_i$$). $$R.y$$ is **not** stored. Reconstructing the curve point from `R_x` alone is sign-ambiguous: the two roots of $$y^2 = R.x^3 - 17$$ in $$\mathbb{F}_r$$ are $$\pm R$$, and since the shared scalar binds $$S.y$$ ([Elliptic Curve Diffie-Hellman](../primitives.md#elliptic-curve-diffie-hellman)), the candidates $$sk_{\text{op}} \cdot R$$ and $$sk_{\text{op}} \cdot (-R) = -(sk_{\text{op}} \cdot R)$$ yield two *different* masks. The ambiguity resolves with one scalar multiplication and a trial decryption, still without storing $$R.y$$: the two candidate shared points are inverses of one another, so the spender computes $$S = sk_{\text{op}} \cdot R$$ for either root, forms both candidate scalars $$s_{\pm} = \text{Poseidon}(\delta_{\text{ecdh}}, S.x, \pm S.y)$$, decrypts a $$dvk_i$$ candidate from each, and keeps the one consistent with the on-chain delegation entry ([Spender Delegation](../account-state.md#spender-delegation), read via `get_spender_delegation`, [Read Methods](../interface.md#read-methods)): $$dvk = \text{dvk\\\_cipher} - \text{Poseidon}(\delta_{\text{esc\\\_dvk}}, s_{\pm}, \text{op}_i)$$ is correct iff $$C_a = \text{Com}(\tilde{a} - \text{Poseidon}(\delta_{\text{enc\\\_allow}}, dvk, \sigma_a), \\; \text{Poseidon}(\delta_{\text{allow\\\_r}}, dvk, \sigma_a))$$. The wrong candidate fails this check except with negligible probability.

The spender decrypts using $$sk_{\text{op}}$$. The `set_spender` proof enforces escrow correctness via constraint S12, which expands to three sub-constraints over the prover-supplied `escrowed_dvk = (R_x, dvk_cipher)`:

- $$R_x = (r_e \cdot H).x$$
- $$s_{\text{esc}} = \text{ECDH}(r_e, Y_{\text{op}})$$ ([Elliptic Curve Diffie-Hellman](../primitives.md#elliptic-curve-diffie-hellman))
- $$\text{dvk\\\_cipher} = \text{Poseidon}(\delta_{\text{esc\\\_dvk}}, s_{\text{esc}}, \text{op}_i) + dvk_i$$

The $$r_e$$ here is the same scalar S\_a1 commits to ($$R_e = r_e \cdot H$$), so the escrow's $$R_x$$ and the auditor channel's $$R_e.x$$ are forced equal.

The same proof also escrows the allowance blinding $$r_a$$ to the owner's auditor (S14), under its own domain tag over the auditor shared scalar; the construction and its decryption path are specified in [Spender Allowance Auditing](../auditing.md#spender-allowance-auditing).

---

Previous: [Confidential Transfer](transfer.md) · Up: [Operations](README.md) · Next: [Spender Transfer](spender-transfer.md)
