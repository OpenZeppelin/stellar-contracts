# Set Spender

The owner locks funds from their spendable balance into a per-spender escrow. The spender must be a registered account in the contract, so that $$Y\_{\text{op}}$$ (needed for $$dvk\_i$$ escrow) can be looked up from the spender's stored `spending_public_key`.

## Constraints

| # | Constraint |
|:--|:---|
| S1 | $$Y = sk \cdot H$$ (owner key ownership) |
| S2 | $$vk = \text{Poseidon}(\delta\_{\text{vk}}, sk, \text{addr\\\_f})$$ (binds proof to contract) |
| S3 | Prover knows opening $$(v, r)$$ of $$C\_{\text{spend}}$$ |
| S4 | $$v \in [0, 2^{127})$$, $$v\_a \in [0, 2^{127})$$, $$v - v\_a \in [0, 2^{127})$$ (range validity, [Integer Embedding and Range Proofs](../primitives.md#integer-embedding-and-range-proofs)) |
| S5 | $$dvk\_i = \text{Poseidon}(\delta\_{\text{dvk}}, vk, \text{op}\_i)$$ (delegation key derivation; contract-bound via $$vk$$) |
| S6 | $$r\_a = \text{Poseidon}(\delta\_{\text{allow\\\_r}}, dvk\_i, \sigma\_a)$$ (allowance blinding) |
| S7 | $$C\_a = v\_a \cdot G + r\_a \cdot H$$ (allowance commitment) |
| S8 | $$\tilde{a} = v\_a + \text{Poseidon}(\delta\_{\text{enc\\\_allow}}, dvk\_i, \sigma\_a)$$ (encrypted allowance) |
| S9 | $$r' = \text{Poseidon}(\delta\_{\text{spend\\\_r}}, vk, \sigma)$$ (new balance randomness) |
| S10 | $$C\_{\text{spend}}' = (v - v\_a) \cdot G + r' \cdot H$$ (new spendable balance) |
| S11 | $$\tilde{b} = (v - v\_a) + \text{Poseidon}(\delta\_{\text{enc\\\_bal}}, vk, \sigma)$$ (encrypted balance) |
| S12 | Escrowed $$dvk\_i$$ correctly encrypts under $$Y\_{\text{op}}$$ via ECDH |
| S13 | $$r\_e \neq 0$$ (rules out $$R\_e = \mathcal{O}$$ and $$S\_{a,s} = \mathcal{O}$$; the same $$r\_e$$ is reused for the $$dvk\_i$$ escrow ECDH in [Delegation Key Escrow](#delegation-key-escrow), so this also rules out a trivial escrow shared secret) |
| S14 | $$\tilde{r}\_{a,\text{aud,s}} = \text{Poseidon}(\delta\_{\text{esc\\\_allow\\\_r\\\_aud}}, s\_{a,s}, \text{op}\_i) + r\_a$$ |
| S\_a1 | $$R\_e = r\_e \cdot H$$ (ephemeral key for auditor ECDH) |
| S\_a2 | $$s\_{a,s} = \text{ECDH}(r\_e, K\_{\text{aud,s}})$$ (owner-auditor ECDH shared scalar, [Elliptic Curve Diffie-Hellman](../primitives.md#elliptic-curve-diffie-hellman)) |
| S\_a3 | $$(m\_v, m\_b, m\_r) = \text{SpongeSqueeze}\_3(\delta\_{\text{aud\\\_s}}, s\_{a,s}, \sigma)$$ (owner-auditor channel masks) |
| S\_a4 | $$\tilde{v}\_{\text{aud,s}} = v\_a + m\_v$$ (owner-auditor encrypted escrow amount) |
| S\_a5 | $$\tilde{b}\_{\text{aud,s}} = (v - v\_a) + m\_b$$ (owner-auditor encrypted balance checkpoint) |
| S\_a6 | $$\tilde{r}\_{\text{aud,s}} = r' + m\_r$$ (owner-auditor escrow of the new spendable blinding, over S9's $$r'$$) |

## Public inputs (26 fields)

| Input | Notes |
|:---|:---|
| $$C\_{\text{spend}}$$ | Loaded from owner's `spendable_commitment` |
| $$Y$$ | Loaded from owner's `spending_public_key` |
| $$Y\_{\text{op}}$$ | Loaded from spender account's `spending_public_key`. Spender must be registered. |
| $$\text{op}\_i$$ | $$\text{address\\\_to\\\_field}$$(`spender` argument), computed per-call by the contract ([Address-to-Field Encoding](../primitives.md#address-to-field-encoding)) |
| $$\text{addr\\\_f}$$ | Loaded from instance storage; set once at construction ([Governance and Upgradeability](../system-model.md#governance-and-upgradeability)) |
| $$K\_{\text{aud,s}}$$ | Fetched from the auditor contract using owner's `auditor_id` |
| $$C\_{\text{spend}}'$$, $$C\_a$$, escrowed\_dvk, $$\tilde{b}$$, $$\tilde{a}$$, $$\sigma$$, $$\sigma\_a$$, $$R\_e$$, $$\tilde{v}\_{\text{aud,s}}$$, $$\tilde{b}\_{\text{aud,s}}$$, $$\tilde{r}\_{\text{aud,s}}$$, $$\tilde{r}\_{a,\text{aud,s}}$$ | Prover-supplied, in this order; $$C\_{\text{spend}}'$$ written to owner's `spendable_commitment`, the delegation fields written to storage, the rest emitted in event |

## Private witnesses

$$sk$$, $$vk$$, $$v$$, $$r$$, $$v\_a$$, $$r\_e$$.

## Post-verification

The contract verifies the proof, sets `spendable_commitment` $$= C\_{\text{spend}}'$$ and stores the `SpenderDelegation`. Emits event with $$(R\_e, \sigma, \tilde{b}, \tilde{v}\_{\text{aud,s}}, \tilde{b}\_{\text{aud,s}}, \tilde{r}\_{\text{aud,s}}, \tilde{r}\_{a,\text{aud,s}})$$.

---

## Delegation Key Escrow

At `set_spender`, the owner escrows $$dvk\_i$$ to the spender on-chain via ECDH, eliminating off-chain key sharing:

1. Owner derives ephemeral $$r\_e$$ per [ECDH-Derived Blinding](../keys-and-commitments.md#ecdh-derived-blinding) — the same scalar as the `set_spender` proof's outer ECDH; see [Why reusing $$r\_e$$ is safe](../keys-and-commitments.md#why-reusing-r_e-is-safe) — and computes $$R = r\_e \cdot H$$.
2. Shared secret: $$s = \text{ECDH}(r\_e, Y\_{\text{op}})$$ ([Elliptic Curve Diffie-Hellman](../primitives.md#elliptic-curve-diffie-hellman))
3. Escrowed key: $$\text{escrowed\\\_dvk} = (R.x, \\; \text{Poseidon}(\delta\_{\text{esc\\\_dvk}}, s, \text{op}\_i) + dvk\_i)$$

**Encoding.** `escrowed_dvk` is a `BytesN<64>` consisting of two 32-byte $$\mathbb{F}\_r$$ representatives: `R_x` (the $$x$$-coordinate of $$R$$) followed by `dvk_cipher` (the masked $$dvk\_i$$). $$R.y$$ is **not** stored. Reconstructing the curve point from `R_x` alone is sign-ambiguous: the two roots of $$y^2 = R.x^3 - 17$$ in $$\mathbb{F}\_r$$ are $$\pm R$$, and since the shared scalar binds $$S.y$$ ([Elliptic Curve Diffie-Hellman](../primitives.md#elliptic-curve-diffie-hellman)), the candidates $$sk\_{\text{op}} \cdot R$$ and $$sk\_{\text{op}} \cdot (-R) = -(sk\_{\text{op}} \cdot R)$$ yield two *different* masks. The ambiguity resolves with one scalar multiplication and a trial decryption, still without storing $$R.y$$: the two candidate shared points are inverses of one another, so the spender computes $$S = sk\_{\text{op}} \cdot R$$ for either root, forms both candidate scalars $$s\_{\pm} = \text{Poseidon}(\delta\_{\text{ecdh}}, S.x, \pm S.y)$$, decrypts a $$dvk\_i$$ candidate from each, and keeps the one consistent with the on-chain delegation entry ([Spender Delegation](../account-state.md#spender-delegation), read via `get_spender_delegation`, [Read Methods](../interface.md#read-methods)): $$dvk = \text{dvk\\\_cipher} - \text{Poseidon}(\delta\_{\text{esc\\\_dvk}}, s\_{\pm}, \text{op}\_i)$$ is correct iff $$C\_a = \text{Com}(\tilde{a} - \text{Poseidon}(\delta\_{\text{enc\\\_allow}}, dvk, \sigma\_a), \\; \text{Poseidon}(\delta\_{\text{allow\\\_r}}, dvk, \sigma\_a))$$. The wrong candidate fails this check except with negligible probability.

The spender decrypts using $$sk\_{\text{op}}$$. The `set_spender` proof enforces escrow correctness via constraint S12, which expands to three sub-constraints over the prover-supplied `escrowed_dvk = (R_x, dvk_cipher)`:

- $$R\_x = (r\_e \cdot H).x$$
- $$s\_{\text{esc}} = \text{ECDH}(r\_e, Y\_{\text{op}})$$ ([Elliptic Curve Diffie-Hellman](../primitives.md#elliptic-curve-diffie-hellman))
- $$\text{dvk\\\_cipher} = \text{Poseidon}(\delta\_{\text{esc\\\_dvk}}, s\_{\text{esc}}, \text{op}\_i) + dvk\_i$$

The $$r\_e$$ here is the same scalar S\_a1 commits to ($$R\_e = r\_e \cdot H$$), so the escrow's $$R\_x$$ and the auditor channel's $$R\_e.x$$ are forced equal.

The same proof also escrows the allowance blinding $$r\_a$$ to the owner's auditor (S14), under its own domain tag over the auditor shared scalar; the construction and its decryption path are specified in [Spender Allowance Auditing](../auditing.md#spender-allowance-auditing).

---

