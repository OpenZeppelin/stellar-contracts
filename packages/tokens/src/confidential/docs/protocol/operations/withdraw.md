# Withdrawal

The owner withdraws a public amount $$a$$ (typed `i128`) from their spendable balance. The W4 range constraint bounds $$a$$ at $$2^{127}$$ in-circuit; the contract additionally checks $$a \ge 0$$ at the entrypoint ([Underlying Token Assumptions](../system-model.md#underlying-token-assumptions)).

## Constraints

| # | Constraint |
|:--|:---|
| W1 | $$Y = sk \cdot H$$ (owner key ownership) |
| W2 | $$vk = \text{Poseidon}(\delta\_{\text{vk}}, sk, \text{addr\\\_f})$$ (binds proof to contract) |
| W3 | The prover knows the opening $$(v, r)$$ of $$C\_{\text{spend}}$$: $$C\_{\text{spend}} = v \cdot G + r \cdot H$$ |
| W4 | $$v \in [0, 2^{127})$$, $$a \in [0, 2^{127})$$, $$v - a \in [0, 2^{127})$$ (range validity, [Integer Embedding and Range Proofs](../primitives.md#integer-embedding-and-range-proofs)) |
| W5 | $$r' = \text{Poseidon}(\delta\_{\text{spend\\\_r}}, vk, \sigma)$$ (deterministic randomness for new balance) |
| W6 | $$C\_{\text{spend}}' = (v - a) \cdot G + r' \cdot H$$ (new spendable commitment) |
| W7 | $$\tilde{b} = (v - a) + \text{Poseidon}(\delta\_{\text{enc\\\_bal}}, vk, \sigma)$$ (encrypted balance scalar) |
| W8 | $$r\_e \neq 0$$ (rules out $$R\_e = \mathcal{O}$$ and $$S\_{a,s} = \mathcal{O}$$, which would reduce $$m\_b$$ to a constant function of $$\sigma$$) |
| W\_a1 | $$R\_e = r\_e \cdot H$$ (ephemeral key for auditor ECDH) |
| W\_a2 | $$s\_{a,s} = \text{ECDH}(r\_e, K\_{\text{aud,s}})$$ (sender-auditor ECDH shared scalar, [Elliptic Curve Diffie-Hellman](../primitives.md#elliptic-curve-diffie-hellman)) |
| W\_a3 | $$(\cdot, m\_b, m\_r) = \text{SpongeSqueeze}\_3(\delta\_{\text{aud\\\_s}}, s\_{a,s}, \sigma)$$ (sender-auditor channel masks; `lane[0]`, the amount slot, is unused) |
| W\_a4 | $$\tilde{b}\_{\text{aud,s}} = (v - a) + m\_b$$ (sender-auditor encrypted balance checkpoint) |
| W\_a5 | $$\tilde{r}\_{\text{aud,s}} = r' + m\_r$$ (sender-auditor escrow of the new spendable blinding, over W5's $$r'$$) |

## Public inputs (16 fields)

| Input | Notes |
|:---|:---|
| $$C\_{\text{spend}}$$ | Loaded from `from.spendable_commitment` |
| $$Y$$ | Loaded from `from.spending_public_key` |
| $$\text{addr\\\_f}$$ | Loaded from instance storage; set once at construction ([Governance and Upgradeability](../system-model.md#governance-and-upgradeability)) |
| $$K\_{\text{aud,s}}$$ | Fetched from the auditor contract using `from.auditor_id` |
| $$a$$ | Public withdrawal amount from invocation inputs |
| $$C\_{\text{spend}}'$$, $$\sigma$$, $$\tilde{b}$$, $$R\_e$$, $$\tilde{b}\_{\text{aud,s}}$$, $$\tilde{r}\_{\text{aud,s}}$$ | Prover-supplied, in this order; $$C\_{\text{spend}}'$$ written to `from.spendable_commitment`, the rest emitted in event |

$$\text{to}$$ is bound under `from.require_auth()` and does not appear in the proof.

## Private witnesses

$$sk$$, $$vk$$, $$v$$, $$r$$, $$r\_e$$.

## Post-verification

The contract verifies the proof, sets `from`.`spendable_commitment` $$= C\_{\text{spend}}'$$, and calls `token.transfer(self, to, a)`. Emits event with $$(R\_e, \sigma, \tilde{b}, \tilde{b}\_{\text{aud,s}}, \tilde{r}\_{\text{aud,s}})$$.
