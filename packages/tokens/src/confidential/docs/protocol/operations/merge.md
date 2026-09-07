# Merge

The owner folds the receiving balance into the spendable balance.

## Contract logic (no proof)

```
require account.require_auth()
C_spend ← C_spend + C_receive
C_receive ← O
```

**Proposition 1** (Merge correctness). If $$C_{\text{spend}} = \text{Com}(v_s, r_s)$$ and $$C_{\text{receive}} = \text{Com}(v_r, r_r)$$ before merge, then after merge $$C_{\text{spend}} = \text{Com}(v_s + v_r, r_s + r_r)$$ and $$C_{\text{receive}} = \mathcal{O} = \text{Com}(0, 0)$$.

*Proof.* By the homomorphic property of Pedersen commitments:
$$C_{\text{spend}} + C_{\text{receive}} = (v_s \cdot G + r_s \cdot H) + (v_r \cdot G + r_r \cdot H) = (v_s + v_r) \cdot G + (r_s + r_r) \cdot H = \text{Com}(v_s + v_r, r_s + r_r)$$
No value is created or destroyed. $$\square$$

**Owner state update.** The owner knows the opening of the post-merge commitment: $$v_{\text{spend}}' = v_s + v_r$$, $$r_{\text{spend}}' = r_s + r_r$$. The owner knows $$v_r$$ and $$r_r$$ from processing incoming transfer and deposit events into $$W_{\text{receive}}$$ ([Update rules](../wallet-state.md#update-rules); the per-transfer derivation is Definition 1 in [ECDH-Derived Blinding](../keys-and-commitments.md#ecdh-derived-blinding)). The values $$v_s$$ and $$r_s$$ are known from the owner's last proof output.

**Griefing analysis.** Merge is not front-runnable, and incoming transfers cannot invalidate an in-flight spend proof (Proposition 2, [Griefing Resistance](../security.md#griefing-resistance); Proposition 3, [Merge Safety](../security.md#merge-safety)).

## Encrypted balance

Merge emits no $$\tilde{b}$$ (there is no proof to enforce consistency between $$\tilde{b}$$ and the post-merge $$C_{\text{spend}}$$). The next owner-initiated proof operation issues a fresh checkpoint. The auditor tracks incoming amounts independently from transfer events.
