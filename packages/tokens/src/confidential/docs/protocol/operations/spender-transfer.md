# Spender Transfer

The spender transfers from the owner's escrowed allowance to a recipient.

## Constraints

| # | Constraint |
|:--|:---|
| O1 | $$Y_{\text{op}} = sk_{\text{op}} \cdot H$$ (spender key ownership) |
| O2 | Prover knows $$dvk_i$$ and the opening $$(v_a, r_a)$$ of $$C_a$$ |
| O3 | $$r_a = \text{Poseidon}(\delta_{\text{allow\\\_r}}, dvk_i, \sigma_a)$$ (allowance randomness matches stored state) |
| O4 | $$v_a \in [0, 2^{127})$$, $$v_{\text{transfer}} \in [0, 2^{127})$$, $$v_a - v_{\text{transfer}} \in [0, 2^{127})$$ (range validity, [Integer Embedding and Range Proofs](../primitives.md#integer-embedding-and-range-proofs)) |
| O5 | $$s = \text{ECDH}(r_e, \text{PVK}_{\text{recipient}})$$ (recipient ECDH shared scalar, [Elliptic Curve Diffie-Hellman](../primitives.md#elliptic-curve-diffie-hellman)) |
| O6 | $$R_e = r_e \cdot H$$ |
| O7 | $$r_{\text{transfer}} = \text{Poseidon}(\delta_{\text{transfer\\\_blind}}, s, \sigma_a')$$ (transfer blinding) |
| O8 | $$C_{\text{transfer}} = v_{\text{transfer}} \cdot G + r_{\text{transfer}} \cdot H$$ |
| O9 | $$\tilde{v} = v_{\text{transfer}} + \text{Poseidon}(\delta_{\text{transfer\\\_amount}}, s, \sigma_a')$$ (encrypted amount) |
| O10 | $$r_a' = \text{Poseidon}(\delta_{\text{allow\\\_r}}, dvk_i, \sigma_a')$$ (new allowance randomness) |
| O11 | $$C_a' = (v_a - v_{\text{transfer}}) \cdot G + r_a' \cdot H$$ (new allowance) |
| O12 | $$\tilde{a}' = (v_a - v_{\text{transfer}}) + \text{Poseidon}(\delta_{\text{enc\\\_allow}}, dvk_i, \sigma_a')$$ (encrypted allowance) |
| O13 | $$r_e \neq 0$$ (rules out $$R_e = \mathcal{O}$$ and $$S, S_{a,r}, S_{a,s} = \mathcal{O}$$) |
| O14 | $$\sigma_a' \neq \sigma_a$$ (nonce rotation) |
| O\_a1 | $$s_{a,r} = \text{ECDH}(r_e, K_{\text{aud,r}})$$ (recipient-auditor ECDH shared scalar, reuses ephemeral scalar) |
| O\_a2 | $$(m_{v,r}, m_{r,r}) = \text{SpongeSqueeze}_2(\delta_{\text{aud\\\_r}}, s_{a,r}, \sigma_a')$$ (recipient-auditor channel masks) |
| O\_a3 | $$\tilde{v}_{\text{aud,r}} = v_{\text{transfer}} + m_{v,r}$$ (recipient-auditor encrypted transfer amount) |
| O\_a4 | $$\tilde{r}_{\text{aud,r}} = r_{\text{transfer}} + m_{r,r}$$ (recipient-auditor encrypted transfer randomness, enables Pedersen-opening reconstruction of $$C_{\text{receive}}$$, see [Per-Transfer Auditor Ciphertexts](../auditing.md#per-transfer-auditor-ciphertexts)) |
| O\_a5 | $$s_{a,s} = \text{ECDH}(r_e, K_{\text{aud,s}})$$ (owner-auditor ECDH shared scalar, reuses ephemeral scalar) |
| O\_a6 | $$(m_{v,s}, m_{a,s}, m_{r,s}) = \text{SpongeSqueeze}_3(\delta_{\text{aud\\\_s}}, s_{a,s}, \sigma_a')$$ (owner-auditor channel masks) |
| O\_a7 | $$\tilde{v}_{\text{aud,s}} = v_{\text{transfer}} + m_{v,s}$$ (owner-auditor encrypted transfer amount) |
| O\_a8 | $$\tilde{a}_{\text{aud,s}} = (v_a - v_{\text{transfer}}) + m_{a,s}$$ (owner-auditor encrypted post-transfer allowance) |
| O\_a9 | $$\tilde{r}_{\text{aud,s}} = r_a' + m_{r,s}$$ (owner-auditor escrow of the new allowance blinding, over O10's $$r_a'$$) |

## Public inputs (25 fields)

| Input | Notes |
|:---|:---|
| $$C_a$$, $$\sigma_a$$ | Loaded from the `(from, spender)` delegation entry |
| $$Y_{\text{op}}$$ | Loaded from spender's `spending_public_key`; matches the auth principal |
| $$\text{PVK}_{\text{recipient}}$$ | Loaded from recipient's `viewing_public_key` |
| $$K_{\text{aud,r}}$$ | Fetched from the auditor contract using recipient's `auditor_id` |
| $$K_{\text{aud,s}}$$ | Fetched from the auditor contract using **owner's** `auditor_id`, not spender's. The visibility model points balance- and allowance-checkpoint ciphertexts at the funds' owner. |
| $$C_a'$$, $$C_{\text{transfer}}$$, $$R_e$$, $$\tilde{v}$$, $$\tilde{a}'$$, $$\sigma_a'$$, $$\tilde{v}_{\text{aud,r}}$$, $$\tilde{r}_{\text{aud,r}}$$, $$\tilde{v}_{\text{aud,s}}$$, $$\tilde{a}_{\text{aud,s}}$$, $$\tilde{r}_{\text{aud,s}}$$ | Prover-supplied, in this order; allowance fields written to delegation storage, $$C_{\text{transfer}}$$ added to recipient's `receiving_commitment`, the rest emitted in event |

## Private witnesses

$$sk_{\text{op}}$$, $$dvk_i$$, $$v_a$$, $$r_a$$ (single-limb $$\mathbb{F}_r$$; pinned by O3 to $$\text{Poseidon}(\delta_{\text{allow\\\_r}}, dvk_i, \sigma_a)$$), $$v_{\text{transfer}}$$, $$r_e$$.

## Post-verification

The contract checks `ledger.sequence() <= live_until_ledger`, updates `allowance_commitment`, `a_tilde`, stores $$\sigma_a'$$ as the new `allowance_salt`, and adds $$C_{\text{transfer}}$$ to the recipient's `receiving_commitment`. Emits event with $$(R_e, \tilde{v}, \sigma_a', \tilde{v}_{\text{aud,r}}, \tilde{r}_{\text{aud,r}}, \tilde{v}_{\text{aud,s}}, \tilde{a}_{\text{aud,s}}, \tilde{r}_{\text{aud,s}})$$.

**Ephemeral scalar.** The spender derives $$r_e = \text{Poseidon}(\delta_{\text{eph}}, vk_{\text{op}}, \sigma_a')$$ ([ECDH-Derived Blinding](../keys-and-commitments.md#ecdh-derived-blinding), [Transfer nonce](../account-state.md#transfer-nonce)) from its *own* viewing key rather than the owner's, so that the spender can later disclose it ([D-sender](../../selective-disclosure/circuits/d-sender.md)). The circuit does not constrain the derivation; it does not constrain $$vk_{\text{op}}$$ at all, per *Contract binding* below. One consequence follows for the owner: since the owner does not hold $$vk_{\text{op}}$$, the owner cannot recompute $$r_e$$ for a spender transfer and cannot disclose it without the spender's cooperation ([Coverage asymmetry](../../selective-disclosure/circuits/d-sender.md#coverage-asymmetry-owner-cannot-d-sender-a-spendertransfer)).

**Recipient uniformity.** The recipient path is identical to the direct-transfer path of [Recipient processing](transfer.md#recipient-processing).

## Contract binding

Unlike owner-initiated circuits, the SpenderTransfer circuit does not constrain the $$vk$$ derivation (the spender has no access to the owner's $$sk$$). Contract binding is instead inherited indirectly through the allowance commitment chain: the SetSpender circuit derives $$dvk_i$$ from the contract-specific $$vk$$ (S2, S5), which determines $$r_a$$ (S6) and thus $$C_a$$ (S7). The SpenderTransfer circuit verifies $$dvk_i$$ against $$C_a$$ via $$\sigma_a$$ (O3). Since $$C_a$$ is a public input and was constructed with contract-specific randomness, a proof generated against one contract's $$C_a$$ cannot verify against another's.
