# Spender Transfer

The spender transfers from the owner's escrowed allowance to a recipient.

## Constraints

| # | Constraint |
|:--|:---|
| O1 | $$Y\_{\text{op}} = sk\_{\text{op}} \cdot H$$ (spender key ownership) |
| O2 | Prover knows $$dvk\_i$$ and the opening $$(v\_a, r\_a)$$ of $$C\_a$$ |
| O3 | $$r\_a = \text{Poseidon}(\delta\_{\text{allow\\\_r}}, dvk\_i, \sigma\_a)$$ (allowance randomness matches stored state) |
| O4 | $$v\_a \in [0, 2^{127})$$, $$v\_{\text{transfer}} \in [0, 2^{127})$$, $$v\_a - v\_{\text{transfer}} \in [0, 2^{127})$$ (range validity, Section 2.6) |
| O5 | $$s = \text{ECDH}(r\_e, \text{PVK}\_{\text{recipient}})$$ (recipient ECDH shared scalar, §2.4) |
| O6 | $$R\_e = r\_e \cdot H$$ |
| O7 | $$r\_{\text{transfer}} = \text{Poseidon}(\delta\_{\text{transfer\\\_blind}}, s, \sigma\_a')$$ (transfer blinding) |
| O8 | $$C\_{\text{transfer}} = v\_{\text{transfer}} \cdot G + r\_{\text{transfer}} \cdot H$$ |
| O9 | $$\tilde{v} = v\_{\text{transfer}} + \text{Poseidon}(\delta\_{\text{transfer\\\_amount}}, s, \sigma\_a')$$ (encrypted amount) |
| O10 | $$r\_a' = \text{Poseidon}(\delta\_{\text{allow\\\_r}}, dvk\_i, \sigma\_a')$$ (new allowance randomness) |
| O11 | $$C\_a' = (v\_a - v\_{\text{transfer}}) \cdot G + r\_a' \cdot H$$ (new allowance) |
| O12 | $$\tilde{a}' = (v\_a - v\_{\text{transfer}}) + \text{Poseidon}(\delta\_{\text{enc\\\_allow}}, dvk\_i, \sigma\_a')$$ (encrypted allowance) |
| O13 | $$r\_e \neq 0$$ (rules out $$R\_e = \mathcal{O}$$ and $$S, S\_{a,r}, S\_{a,s} = \mathcal{O}$$) |
| O14 | $$\sigma\_a' \neq \sigma\_a$$ (nonce rotation) |
| O\_a1 | $$s\_{a,r} = \text{ECDH}(r\_e, K\_{\text{aud,r}})$$ (recipient-auditor ECDH shared scalar, reuses ephemeral scalar) |
| O\_a2 | $$(m\_{v,r}, m\_{r,r}) = \text{SpongeSqueeze}\_2(\delta\_{\text{aud\\\_r}}, s\_{a,r}, \sigma\_a')$$ (recipient-auditor channel masks) |
| O\_a3 | $$\tilde{v}\_{\text{aud,r}} = v\_{\text{transfer}} + m\_{v,r}$$ (recipient-auditor encrypted transfer amount) |
| O\_a4 | $$\tilde{r}\_{\text{aud,r}} = r\_{\text{transfer}} + m\_{r,r}$$ (recipient-auditor encrypted transfer randomness, enables Pedersen-opening reconstruction of $$C\_{\text{receive}}$$, see Section 8.1) |
| O\_a5 | $$s\_{a,s} = \text{ECDH}(r\_e, K\_{\text{aud,s}})$$ (owner-auditor ECDH shared scalar, reuses ephemeral scalar) |
| O\_a6 | $$(m\_{v,s}, m\_{a,s}, m\_{r,s}) = \text{SpongeSqueeze}\_3(\delta\_{\text{aud\\\_s}}, s\_{a,s}, \sigma\_a')$$ (owner-auditor channel masks) |
| O\_a7 | $$\tilde{v}\_{\text{aud,s}} = v\_{\text{transfer}} + m\_{v,s}$$ (owner-auditor encrypted transfer amount) |
| O\_a8 | $$\tilde{a}\_{\text{aud,s}} = (v\_a - v\_{\text{transfer}}) + m\_{a,s}$$ (owner-auditor encrypted post-transfer allowance) |
| O\_a9 | $$\tilde{r}\_{\text{aud,s}} = r\_a' + m\_{r,s}$$ (owner-auditor escrow of the new allowance blinding, over O10's $$r\_a'$$) |

## Public inputs (25 fields)

| Input | Notes |
|:---|:---|
| $$C\_a$$, $$\sigma\_a$$ | Loaded from the `(from, spender)` delegation entry |
| $$Y\_{\text{op}}$$ | Loaded from spender's `spending_public_key`; matches the auth principal |
| $$\text{PVK}\_{\text{recipient}}$$ | Loaded from recipient's `viewing_public_key` |
| $$K\_{\text{aud,r}}$$ | Fetched from the auditor contract using recipient's `auditor_id` |
| $$K\_{\text{aud,s}}$$ | Fetched from the auditor contract using **owner's** `auditor_id`, not spender's. The visibility model points balance- and allowance-checkpoint ciphertexts at the funds' owner. |
| $$C\_a'$$, $$C\_{\text{transfer}}$$, $$R\_e$$, $$\tilde{v}$$, $$\tilde{a}'$$, $$\sigma\_a'$$, $$\tilde{v}\_{\text{aud,r}}$$, $$\tilde{r}\_{\text{aud,r}}$$, $$\tilde{v}\_{\text{aud,s}}$$, $$\tilde{a}\_{\text{aud,s}}$$, $$\tilde{r}\_{\text{aud,s}}$$ | Prover-supplied, in this order; allowance fields written to delegation storage, $$C\_{\text{transfer}}$$ added to recipient's `receiving_commitment`, the rest emitted in event |

## Private witnesses

$$sk\_{\text{op}}$$, $$dvk\_i$$, $$v\_a$$, $$r\_a$$ (single-limb $$\mathbb{F}\_r$$; pinned by O3 to $$\text{Poseidon}(\delta\_{\text{allow\\\_r}}, dvk\_i, \sigma\_a)$$), $$v\_{\text{transfer}}$$, $$r\_e$$.

## Post-verification

The contract checks `ledger.sequence() <= live_until_ledger`, updates `allowance_commitment`, `a_tilde`, stores $$\sigma\_a'$$ as the new `allowance_salt`, and adds $$C\_{\text{transfer}}$$ to the recipient's `receiving_commitment`. Emits event with $$(R\_e, \tilde{v}, \sigma\_a', \tilde{v}\_{\text{aud,r}}, \tilde{r}\_{\text{aud,r}}, \tilde{v}\_{\text{aud,s}}, \tilde{a}\_{\text{aud,s}}, \tilde{r}\_{\text{aud,s}})$$.

**Ephemeral scalar.** The spender derives $$r\_e = \text{Poseidon}(\delta\_{\text{eph}}, vk\_{\text{op}}, \sigma\_a')$$ (§5.3, §6.2 *Transfer nonce*) from its *own* viewing key rather than the owner's, so that the spender can later disclose it ([SELECTIVE_DISCLOSURE.md](./SELECTIVE_DISCLOSURE.md) §7). The circuit does not constrain the derivation; it does not constrain $$vk\_{\text{op}}$$ at all, per *Contract binding* below. One consequence follows for the owner: since the owner does not hold $$vk\_{\text{op}}$$, the owner cannot recompute $$r\_e$$ for a spender transfer and cannot disclose it without the spender's cooperation ([SELECTIVE_DISCLOSURE.md](./SELECTIVE_DISCLOSURE.md) §7, *Coverage asymmetry*).

**Recipient uniformity.** The recipient path is identical to the direct-transfer path of §7.6 *Recipient processing*.

## Contract binding

Unlike owner-initiated circuits, the SpenderTransfer circuit does not constrain the $$vk$$ derivation (the spender has no access to the owner's $$sk$$). Contract binding is instead inherited indirectly through the allowance commitment chain: the SetSpender circuit derives $$dvk\_i$$ from the contract-specific $$vk$$ (S2, S5), which determines $$r\_a$$ (S6) and thus $$C\_a$$ (S7). The SpenderTransfer circuit verifies $$dvk\_i$$ against $$C\_a$$ via $$\sigma\_a$$ (O3). Since $$C\_a$$ is a public input and was constructed with contract-specific randomness, a proof generated against one contract's $$C\_a$$ cannot verify against another's.
